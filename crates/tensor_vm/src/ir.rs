use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Result, TvmError};
use crate::field::Elem;
use crate::tensor::DType;
use crate::types::{Hash, hash_bytes};

pub type GraphId = Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrOpTier {
    A,
    B,
    C,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrArity {
    Exact(usize),
    Variadic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrVerificationClass {
    FullFreivalds,
    RandomLinear,
    ExactDeterministicReplay,
    IndexConsistencyRequired,
    CanonicalReferenceRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpSpec {
    pub name: &'static str,
    pub tier: IrOpTier,
    pub arity: IrArity,
    pub output_count: usize,
    pub allowed_kwargs: &'static [&'static str],
    pub required_kwargs: &'static [&'static str],
    pub verification: IrVerificationClass,
    pub consensus_admitted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorSpec {
    pub name: String,
    pub shape: Vec<i64>,
    pub dtype: DType,
    pub scale: i64,
}

impl TensorSpec {
    pub fn field(name: impl Into<String>, shape: Vec<usize>) -> Self {
        Self {
            name: name.into(),
            shape: shape.into_iter().map(|dim| dim as i64).collect(),
            dtype: DType::FieldElement,
            scale: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamSpec {
    pub name: String,
    pub type_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrLiteral {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Field(Elem),
    String(String),
    List(Vec<IrLiteral>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrValue {
    Ref(IrRef),
    Literal(IrLiteral),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrRef {
    Input {
        name: String,
    },
    Op {
        id: usize,
        idx: usize,
    },
    Param {
        name: String,
    },
    Const {
        value: IrLiteral,
    },
    ConstBlob {
        uri: String,
        shape: Vec<i64>,
        dtype: DType,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpNode {
    pub id: usize,
    pub op: String,
    pub args: Vec<IrRef>,
    pub kwargs: BTreeMap<String, IrValue>,
    pub out: Vec<TensorSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphOutput {
    pub name: String,
    pub value: IrRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorGraph {
    pub ir_version: u64,
    pub inputs: Vec<TensorSpec>,
    pub params: Vec<ParamSpec>,
    pub ops: Vec<OpNode>,
    pub outputs: Vec<GraphOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValueShape {
    shape: Vec<i64>,
    dtype: DType,
    scale: i64,
}

pub fn frozen_op_registry() -> &'static [OpSpec] {
    &FROZEN_OP_REGISTRY
}

pub fn op_spec(name: &str) -> Option<&'static OpSpec> {
    frozen_op_registry().iter().find(|spec| spec.name == name)
}

impl TensorGraph {
    pub fn canonical_json(&self) -> String {
        let inputs = join_json(self.inputs.iter().map(canonical_tensor_spec_json));
        let params = join_json(self.params.iter().map(canonical_param_spec_json));
        let ops = join_json(self.ops.iter().map(canonical_op_json));
        let outputs = join_json(self.outputs.iter().map(canonical_graph_output_json));
        format!(
            "{{\"inputs\":[{}],\"ir_version\":{},\"ops\":[{}],\"outputs\":[{}],\"params\":[{}]}}",
            inputs, self.ir_version, ops, outputs, params
        )
    }

    pub fn graph_id(&self) -> GraphId {
        let canonical = self.canonical_json();
        hash_bytes(b"tensor-vm-ir-graph-v1", &[canonical.as_bytes()])
    }

    pub fn validate_for_consensus(&self) -> Result<GraphId> {
        self.validate(true)?;
        Ok(self.graph_id())
    }

    pub fn validate(&self, require_consensus_admitted: bool) -> Result<()> {
        if self.ir_version != 1 {
            return Err(TvmError::InvalidReceipt("unsupported tensor ir version"));
        }
        let input_names = unique_tensor_names(&self.inputs)?;
        let param_names = unique_param_names(&self.params)?;
        let mut op_outputs = Vec::<Vec<ValueShape>>::new();

        for (index, op) in self.ops.iter().enumerate() {
            if op.id != index {
                return Err(TvmError::InvalidReceipt("non-dense tensor ir op ids"));
            }
            let spec = op_spec(&op.op).ok_or(TvmError::InvalidReceipt("unknown tensor ir op"))?;
            if require_consensus_admitted && !spec.consensus_admitted {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir op is not consensus admitted",
                ));
            }
            validate_arity(spec, op.args.len())?;
            validate_kwargs(spec, &op.kwargs)?;
            if op.out.len() != spec.output_count {
                return Err(TvmError::InvalidReceipt("tensor ir output count mismatch"));
            }
            unique_local_output_names(&op.out)?;
            let arg_shapes = op
                .args
                .iter()
                .map(|arg| {
                    resolve_ref(
                        arg,
                        index,
                        &input_names,
                        &param_names,
                        &self.inputs,
                        &op_outputs,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let inferred = infer_outputs(&op.op, &arg_shapes, &op.kwargs)?;
            if inferred.len() != op.out.len() {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir inferred output count mismatch",
                ));
            }
            for (declared, expected) in op.out.iter().zip(inferred.iter()) {
                validate_tensor_spec(declared, false)?;
                if declared.shape != expected.shape
                    || declared.dtype != expected.dtype
                    || declared.scale != expected.scale
                {
                    return Err(TvmError::InvalidReceipt("tensor ir output type mismatch"));
                }
            }
            op_outputs.push(inferred);
        }

        let mut output_names = BTreeSet::new();
        for output in &self.outputs {
            if !output_names.insert(output.name.as_str()) {
                return Err(TvmError::InvalidReceipt("duplicate tensor ir output name"));
            }
            resolve_ref(
                &output.value,
                self.ops.len(),
                &input_names,
                &param_names,
                &self.inputs,
                &op_outputs,
            )?;
        }
        Ok(())
    }
}

pub fn canonical_matmul_graph(m: usize, k: usize, n: usize, dtype: DType) -> TensorGraph {
    TensorGraph {
        ir_version: 1,
        inputs: vec![
            tensor_spec("a", vec![m, k], dtype, 0),
            tensor_spec("b", vec![k, n], dtype, 0),
        ],
        params: Vec::new(),
        ops: vec![OpNode {
            id: 0,
            op: "matmul".to_owned(),
            args: vec![input_ref("a"), input_ref("b")],
            kwargs: BTreeMap::new(),
            out: vec![tensor_spec("c", vec![m, n], dtype, 0)],
        }],
        outputs: vec![GraphOutput {
            name: "c".to_owned(),
            value: IrRef::Op { id: 0, idx: 0 },
        }],
    }
}

pub fn canonical_linear_training_step_graph(
    input_shape: &[usize],
    weight_shape: &[usize],
    target_shape: &[usize],
    dtype: DType,
) -> TensorGraph {
    let y_shape = vec![input_shape[0], weight_shape[1]];
    TensorGraph {
        ir_version: 1,
        inputs: vec![
            tensor_spec("x", input_shape.to_vec(), dtype, 0),
            tensor_spec("w", weight_shape.to_vec(), dtype, 0),
            tensor_spec("target", target_shape.to_vec(), dtype, 0),
        ],
        params: vec![ParamSpec {
            name: "lr".to_owned(),
            type_name: "field_scalar".to_owned(),
        }],
        ops: vec![
            OpNode {
                id: 0,
                op: "matmul".to_owned(),
                args: vec![input_ref("x"), input_ref("w")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("y", y_shape.clone(), dtype, 0)],
            },
            OpNode {
                id: 1,
                op: "sub".to_owned(),
                args: vec![op_ref(0), input_ref("target")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("dy", target_shape.to_vec(), dtype, 0)],
            },
            OpNode {
                id: 2,
                op: "transpose".to_owned(),
                args: vec![input_ref("x")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec(
                    "x_t",
                    vec![input_shape[1], input_shape[0]],
                    dtype,
                    0,
                )],
            },
            OpNode {
                id: 3,
                op: "matmul".to_owned(),
                args: vec![op_ref(2), op_ref(1)],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("grad_w", weight_shape.to_vec(), dtype, 0)],
            },
            OpNode {
                id: 4,
                op: "scalar_mul".to_owned(),
                args: vec![op_ref(3), param_ref("lr")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec(
                    "scaled_grad_w",
                    weight_shape.to_vec(),
                    dtype,
                    0,
                )],
            },
            OpNode {
                id: 5,
                op: "sub".to_owned(),
                args: vec![input_ref("w"), op_ref(4)],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("weight_after", weight_shape.to_vec(), dtype, 0)],
            },
        ],
        outputs: vec![
            GraphOutput {
                name: "y".to_owned(),
                value: op_ref(0),
            },
            GraphOutput {
                name: "dy".to_owned(),
                value: op_ref(1),
            },
            GraphOutput {
                name: "grad_w".to_owned(),
                value: op_ref(3),
            },
            GraphOutput {
                name: "weight_after".to_owned(),
                value: op_ref(5),
            },
        ],
    }
}

fn input_ref(name: &str) -> IrRef {
    IrRef::Input {
        name: name.to_owned(),
    }
}

fn param_ref(name: &str) -> IrRef {
    IrRef::Param {
        name: name.to_owned(),
    }
}

fn op_ref(id: usize) -> IrRef {
    IrRef::Op { id, idx: 0 }
}

fn tensor_spec(name: &str, shape: Vec<usize>, dtype: DType, scale: i64) -> TensorSpec {
    TensorSpec {
        name: name.to_owned(),
        shape: shape.into_iter().map(|dim| dim as i64).collect(),
        dtype,
        scale,
    }
}

fn validate_arity(spec: &OpSpec, actual: usize) -> Result<()> {
    match spec.arity {
        IrArity::Exact(expected) if expected == actual => Ok(()),
        IrArity::Variadic if actual >= 1 => Ok(()),
        _ => Err(TvmError::InvalidReceipt("tensor ir op arity mismatch")),
    }
}

fn validate_kwargs(spec: &OpSpec, kwargs: &BTreeMap<String, IrValue>) -> Result<()> {
    for key in kwargs.keys() {
        if !spec.allowed_kwargs.contains(&key.as_str()) {
            return Err(TvmError::InvalidReceipt("unknown tensor ir kwarg"));
        }
    }
    for required in spec.required_kwargs {
        if !kwargs.contains_key(*required) {
            return Err(TvmError::InvalidReceipt("missing tensor ir kwarg"));
        }
    }
    Ok(())
}

fn unique_tensor_names(specs: &[TensorSpec]) -> Result<BTreeSet<&str>> {
    let mut names = BTreeSet::new();
    for spec in specs {
        validate_tensor_spec(spec, true)?;
        if !names.insert(spec.name.as_str()) {
            return Err(TvmError::InvalidReceipt("duplicate tensor ir input name"));
        }
    }
    Ok(names)
}

fn unique_param_names(specs: &[ParamSpec]) -> Result<BTreeSet<&str>> {
    let mut names = BTreeSet::new();
    for spec in specs {
        if spec.name.is_empty() || spec.type_name.is_empty() {
            return Err(TvmError::InvalidReceipt("invalid tensor ir param"));
        }
        if !names.insert(spec.name.as_str()) {
            return Err(TvmError::InvalidReceipt("duplicate tensor ir param name"));
        }
    }
    Ok(names)
}

fn unique_local_output_names(specs: &[TensorSpec]) -> Result<()> {
    let mut names = BTreeSet::new();
    for spec in specs {
        if !names.insert(spec.name.as_str()) {
            return Err(TvmError::InvalidReceipt(
                "duplicate tensor ir op output name",
            ));
        }
    }
    Ok(())
}

fn validate_tensor_spec(spec: &TensorSpec, allow_unbound_input_dims: bool) -> Result<()> {
    if spec.name.is_empty() || spec.shape.is_empty() {
        return Err(TvmError::InvalidReceipt("invalid tensor ir tensor spec"));
    }
    for dim in &spec.shape {
        if *dim == -1 && allow_unbound_input_dims {
            continue;
        }
        if *dim < 0 {
            return Err(TvmError::InvalidReceipt("invalid tensor ir tensor shape"));
        }
    }
    Ok(())
}

fn resolve_ref(
    value: &IrRef,
    current_op: usize,
    input_names: &BTreeSet<&str>,
    param_names: &BTreeSet<&str>,
    inputs: &[TensorSpec],
    op_outputs: &[Vec<ValueShape>],
) -> Result<ValueShape> {
    match value {
        IrRef::Input { name } => {
            if !input_names.contains(name.as_str()) {
                return Err(TvmError::InvalidReceipt("unknown tensor ir input ref"));
            }
            let spec = inputs
                .iter()
                .find(|spec| spec.name == *name)
                .ok_or(TvmError::InvalidReceipt("unknown tensor ir input ref"))?;
            Ok(ValueShape {
                shape: spec.shape.clone(),
                dtype: spec.dtype,
                scale: spec.scale,
            })
        }
        IrRef::Op { id, idx } => {
            if *id >= current_op {
                return Err(TvmError::InvalidReceipt("forward tensor ir op ref"));
            }
            let outputs = op_outputs
                .get(*id)
                .ok_or(TvmError::InvalidReceipt("unknown tensor ir op ref"))?;
            outputs
                .get(*idx)
                .cloned()
                .ok_or(TvmError::InvalidReceipt("bad tensor ir op output ref"))
        }
        IrRef::Param { name } => {
            if !param_names.contains(name.as_str()) {
                return Err(TvmError::InvalidReceipt("unknown tensor ir param ref"));
            }
            Ok(ValueShape {
                shape: Vec::new(),
                dtype: DType::FieldElement,
                scale: 0,
            })
        }
        IrRef::Const { value } => Ok(ValueShape {
            shape: literal_shape(value),
            dtype: DType::FieldElement,
            scale: 0,
        }),
        IrRef::ConstBlob { shape, dtype, .. } => Ok(ValueShape {
            shape: shape.clone(),
            dtype: *dtype,
            scale: 0,
        }),
    }
}

fn infer_outputs(
    op: &str,
    args: &[ValueShape],
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<Vec<ValueShape>> {
    let output = match op {
        "matmul" => {
            let [lhs, rhs] = two_args(args)?;
            if lhs.shape.len() != 2 || rhs.shape.len() != 2 || lhs.shape[1] != rhs.shape[0] {
                return Err(TvmError::InvalidReceipt("tensor ir matmul shape mismatch"));
            }
            same_dtype(lhs, rhs)?;
            ValueShape {
                shape: vec![lhs.shape[0], rhs.shape[1]],
                dtype: lhs.dtype,
                scale: lhs.scale,
            }
        }
        "add" | "sub" | "mul" => {
            let [lhs, rhs] = two_args(args)?;
            same_tensor(lhs, rhs)?;
            lhs.clone()
        }
        "scalar_mul" => {
            if args.len() != 2 || args[0].shape.is_empty() || !args[1].shape.is_empty() {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir scalar_mul shape mismatch",
                ));
            }
            args[0].clone()
        }
        "transpose" => {
            let arg = one_arg(args)?;
            if arg.shape.len() != 2 {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir transpose rank mismatch",
                ));
            }
            ValueShape {
                shape: vec![arg.shape[1], arg.shape[0]],
                dtype: arg.dtype,
                scale: arg.scale,
            }
        }
        "sum" | "reduce_sum" => infer_sum(args, kwargs)?,
        "mean" => infer_sum(args, kwargs)?,
        "reshape" | "broadcast" => {
            let arg = one_arg(args)?;
            let shape = shape_kwarg(kwargs, "shape")?;
            ValueShape {
                shape,
                dtype: arg.dtype,
                scale: arg.scale,
            }
        }
        "identity" | "neg" | "abs" | "sign" | "round" | "relu" => one_arg(args)?.clone(),
        "exp" | "log" | "sqrt" | "softmax" => one_arg(args)?.clone(),
        "gather" | "embedding" => {
            if args.len() != 2 {
                return Err(TvmError::InvalidReceipt("tensor ir op arity mismatch"));
            }
            args[0].clone()
        }
        "scatter" => {
            if args.len() != 3 {
                return Err(TvmError::InvalidReceipt("tensor ir op arity mismatch"));
            }
            args[0].clone()
        }
        "gt" | "lt" | "ge" | "le" | "eq" => {
            let [lhs, rhs] = two_args(args)?;
            same_tensor(lhs, rhs)?;
            ValueShape {
                shape: lhs.shape.clone(),
                dtype: DType::Int32,
                scale: 0,
            }
        }
        "where" => {
            if args.len() != 3 {
                return Err(TvmError::InvalidReceipt("tensor ir where arity mismatch"));
            }
            same_tensor(&args[1], &args[2])?;
            ValueShape {
                shape: args[1].shape.clone(),
                dtype: args[1].dtype,
                scale: args[1].scale,
            }
        }
        "cast" => {
            let arg = one_arg(args)?;
            let dtype = dtype_kwarg(kwargs, "dtype")?;
            ValueShape {
                shape: arg.shape.clone(),
                dtype,
                scale: arg.scale,
            }
        }
        "concat" | "stack" => infer_variadic_same(args)?,
        "full" | "arange" => ValueShape {
            shape: shape_kwarg(kwargs, "shape").unwrap_or_else(|_| vec![1]),
            dtype: dtype_kwarg(kwargs, "dtype").unwrap_or(DType::FieldElement),
            scale: 0,
        },
        "topk" => {
            let arg = one_arg(args)?;
            return Ok(vec![
                arg.clone(),
                ValueShape {
                    shape: arg.shape.clone(),
                    dtype: DType::Int64,
                    scale: 0,
                },
            ]);
        }
        _ => {
            return Err(TvmError::InvalidReceipt(
                "unsupported tensor ir typing rule",
            ));
        }
    };
    Ok(vec![output])
}

fn infer_sum(args: &[ValueShape], kwargs: &BTreeMap<String, IrValue>) -> Result<ValueShape> {
    let arg = one_arg(args)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?;
    let keepdim = optional_bool_kwarg(kwargs, "keepdim")?.unwrap_or(false);
    let mut shape = arg.shape.clone();
    if let Some(dim) = dim {
        if dim >= shape.len() {
            return Err(TvmError::InvalidReceipt("tensor ir reduction dim mismatch"));
        }
        if keepdim {
            shape[dim] = 1;
        } else {
            shape.remove(dim);
        }
    } else if keepdim {
        shape.fill(1);
    } else {
        shape = vec![1];
    }
    Ok(ValueShape {
        shape,
        dtype: arg.dtype,
        scale: arg.scale,
    })
}

fn infer_variadic_same(args: &[ValueShape]) -> Result<ValueShape> {
    if args.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir variadic op requires args",
        ));
    }
    for arg in &args[1..] {
        same_tensor(&args[0], arg)?;
    }
    Ok(args[0].clone())
}

fn one_arg(args: &[ValueShape]) -> Result<&ValueShape> {
    args.first()
        .filter(|_| args.len() == 1)
        .ok_or(TvmError::InvalidReceipt("tensor ir op arity mismatch"))
}

fn two_args(args: &[ValueShape]) -> Result<[&ValueShape; 2]> {
    if args.len() == 2 {
        Ok([&args[0], &args[1]])
    } else {
        Err(TvmError::InvalidReceipt("tensor ir op arity mismatch"))
    }
}

fn same_dtype(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    if lhs.dtype != rhs.dtype || lhs.scale != rhs.scale {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    Ok(())
}

fn same_tensor(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    same_dtype(lhs, rhs)?;
    if lhs.shape != rhs.shape {
        return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
    }
    Ok(())
}

fn shape_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Vec<i64>> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::List(values))) => values
            .iter()
            .map(|value| match value {
                IrLiteral::Int(dim) if *dim >= 0 => Ok(*dim),
                IrLiteral::Uint(dim) => Ok(*dim as i64),
                _ => Err(TvmError::InvalidReceipt("invalid tensor ir shape kwarg")),
            })
            .collect(),
        _ => Err(TvmError::InvalidReceipt("missing tensor ir shape kwarg")),
    }
}

fn dtype_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<DType> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::String(value))) => {
            dtype_from_name(value).ok_or(TvmError::InvalidReceipt("invalid tensor ir dtype kwarg"))
        }
        _ => Err(TvmError::InvalidReceipt("missing tensor ir dtype kwarg")),
    }
}

fn optional_usize_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Option<usize>> {
    match kwargs.get(key) {
        None => Ok(None),
        Some(IrValue::Literal(IrLiteral::Int(value))) if *value >= 0 => Ok(Some(*value as usize)),
        Some(IrValue::Literal(IrLiteral::Uint(value))) => Ok(Some(*value as usize)),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir integer kwarg")),
    }
}

fn optional_bool_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Option<bool>> {
    match kwargs.get(key) {
        None => Ok(None),
        Some(IrValue::Literal(IrLiteral::Bool(value))) => Ok(Some(*value)),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir bool kwarg")),
    }
}

fn literal_shape(value: &IrLiteral) -> Vec<i64> {
    match value {
        IrLiteral::List(values) => vec![values.len() as i64],
        _ => Vec::new(),
    }
}

fn dtype_from_name(value: &str) -> Option<DType> {
    match value {
        "int32" => Some(DType::Int32),
        "int64" => Some(DType::Int64),
        "fixed32" => Some(DType::Fixed32),
        "field" | "field_element" => Some(DType::FieldElement),
        _ => None,
    }
}

fn dtype_name(dtype: DType) -> &'static str {
    match dtype {
        DType::Int32 => "int32",
        DType::Int64 => "int64",
        DType::Fixed32 => "fixed32",
        DType::FieldElement => "field",
    }
}

fn canonical_tensor_spec_json(spec: &TensorSpec) -> String {
    format!(
        "{{\"dtype\":\"{}\",\"name\":\"{}\",\"scale\":{},\"shape\":[{}]}}",
        dtype_name(spec.dtype),
        escape_json(&spec.name),
        spec.scale,
        join_json(spec.shape.iter().map(|dim| dim.to_string()))
    )
}

fn canonical_param_spec_json(spec: &ParamSpec) -> String {
    format!(
        "{{\"name\":\"{}\",\"type\":\"{}\"}}",
        escape_json(&spec.name),
        escape_json(&spec.type_name)
    )
}

fn canonical_op_json(op: &OpNode) -> String {
    let args = join_json(op.args.iter().map(canonical_ref_json));
    let kwargs =
        join_json(op.kwargs.iter().map(|(key, value)| {
            format!("\"{}\":{}", escape_json(key), canonical_value_json(value))
        }));
    let out = join_json(op.out.iter().map(canonical_tensor_spec_json));
    format!(
        "{{\"args\":[{}],\"id\":{},\"kwargs\":{{{}}},\"op\":\"{}\",\"out\":[{}]}}",
        args,
        op.id,
        kwargs,
        escape_json(&op.op),
        out
    )
}

fn canonical_graph_output_json(output: &GraphOutput) -> String {
    format!(
        "{{\"name\":\"{}\",\"ref\":{}}}",
        escape_json(&output.name),
        canonical_ref_json(&output.value)
    )
}

fn canonical_value_json(value: &IrValue) -> String {
    match value {
        IrValue::Ref(value) => canonical_ref_json(value),
        IrValue::Literal(value) => canonical_literal_json(value),
    }
}

fn canonical_ref_json(value: &IrRef) -> String {
    match value {
        IrRef::Input { name } => {
            format!("{{\"kind\":\"input\",\"name\":\"{}\"}}", escape_json(name))
        }
        IrRef::Op { id, idx } => format!("{{\"id\":{},\"idx\":{},\"kind\":\"op\"}}", id, idx),
        IrRef::Param { name } => {
            format!("{{\"kind\":\"param\",\"name\":\"{}\"}}", escape_json(name))
        }
        IrRef::Const { value } => {
            format!(
                "{{\"kind\":\"const\",\"value\":{}}}",
                canonical_literal_json(value)
            )
        }
        IrRef::ConstBlob { uri, shape, dtype } => format!(
            "{{\"dtype\":\"{}\",\"kind\":\"const_blob\",\"shape\":[{}],\"uri\":\"{}\"}}",
            dtype_name(*dtype),
            join_json(shape.iter().map(|dim| dim.to_string())),
            escape_json(uri)
        ),
    }
}

fn canonical_literal_json(value: &IrLiteral) -> String {
    match value {
        IrLiteral::Bool(value) => value.to_string(),
        IrLiteral::Int(value) => value.to_string(),
        IrLiteral::Uint(value) => value.to_string(),
        IrLiteral::Field(value) => format!("\"{:016x}\"", value),
        IrLiteral::String(value) => format!("\"{}\"", escape_json(value)),
        IrLiteral::List(values) => join_json_wrapped(values.iter().map(canonical_literal_json)),
    }
}

fn join_json(values: impl Iterator<Item = String>) -> String {
    values.collect::<Vec<_>>().join(",")
}

fn join_json_wrapped(values: impl Iterator<Item = String>) -> String {
    format!("[{}]", join_json(values))
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch <= '\u{1f}' => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out
}

const FROZEN_OP_REGISTRY: [OpSpec; 38] = [
    OpSpec {
        name: "matmul",
        tier: IrOpTier::A,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::FullFreivalds,
        consensus_admitted: true,
    },
    OpSpec {
        name: "einsum",
        tier: IrOpTier::A,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &["equation"],
        required_kwargs: &["equation"],
        verification: IrVerificationClass::FullFreivalds,
        consensus_admitted: false,
    },
    OpSpec {
        name: "add",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sub",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "mul",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "div",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "scalar_mul",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "transpose",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dims"],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sum",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "reduce_sum",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "mean",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "reshape",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["shape"],
        required_kwargs: &["shape"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "broadcast",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["shape"],
        required_kwargs: &["shape"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "identity",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "neg",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "abs",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sign",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "round",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "relu",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "gt",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "lt",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "ge",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "le",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "eq",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "where",
        tier: IrOpTier::B,
        arity: IrArity::Exact(3),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "cast",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dtype"],
        required_kwargs: &["dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "concat",
        tier: IrOpTier::B,
        arity: IrArity::Variadic,
        output_count: 1,
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "stack",
        tier: IrOpTier::B,
        arity: IrArity::Variadic,
        output_count: 1,
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "full",
        tier: IrOpTier::B,
        arity: IrArity::Exact(0),
        output_count: 1,
        allowed_kwargs: &["shape", "value", "dtype"],
        required_kwargs: &["shape", "value", "dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "arange",
        tier: IrOpTier::B,
        arity: IrArity::Exact(0),
        output_count: 1,
        allowed_kwargs: &["start", "end", "step", "dtype"],
        required_kwargs: &["start", "end", "step", "dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "exp",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "log",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "sqrt",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "softmax",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: 1,
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "gather",
        tier: IrOpTier::C,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "scatter",
        tier: IrOpTier::C,
        arity: IrArity::Exact(3),
        output_count: 1,
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "embedding",
        tier: IrOpTier::C,
        arity: IrArity::Exact(2),
        output_count: 1,
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "topk",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: 2,
        allowed_kwargs: &["k", "dim"],
        required_kwargs: &["k", "dim"],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matmul_graph_has_stable_canonical_json_and_graph_id() {
        let graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph.validate_for_consensus().unwrap();
        assert_eq!(
            graph.canonical_json(),
            "{\"inputs\":[{\"dtype\":\"field\",\"name\":\"a\",\"scale\":0,\"shape\":[2,3]},{\"dtype\":\"field\",\"name\":\"b\",\"scale\":0,\"shape\":[3,4]}],\"ir_version\":1,\"ops\":[{\"args\":[{\"kind\":\"input\",\"name\":\"a\"},{\"kind\":\"input\",\"name\":\"b\"}],\"id\":0,\"kwargs\":{},\"op\":\"matmul\",\"out\":[{\"dtype\":\"field\",\"name\":\"c\",\"scale\":0,\"shape\":[2,4]}]}],\"outputs\":[{\"name\":\"c\",\"ref\":{\"id\":0,\"idx\":0,\"kind\":\"op\"}}],\"params\":[]}"
        );
        assert_eq!(graph.graph_id(), graph.validate_for_consensus().unwrap());
        let mut changed = graph.clone();
        changed.inputs[0].shape = vec![2, 4];
        assert_ne!(graph.graph_id(), changed.graph_id());
    }

    #[test]
    fn graph_validation_rejects_bad_structure() {
        let graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);

        let mut bad_id = graph.clone();
        bad_id.ops[0].id = 1;
        assert!(bad_id.validate_for_consensus().is_err());

        let mut bad_ref = graph.clone();
        bad_ref.ops[0].args[0] = IrRef::Op { id: 0, idx: 0 };
        assert!(bad_ref.validate_for_consensus().is_err());

        let mut bad_output = graph.clone();
        bad_output.outputs[0].value = IrRef::Op { id: 0, idx: 1 };
        assert!(bad_output.validate_for_consensus().is_err());

        let mut duplicate_input = graph.clone();
        duplicate_input.inputs[1].name = "a".to_owned();
        assert!(duplicate_input.validate_for_consensus().is_err());
    }

    #[test]
    fn graph_validation_rejects_op_metadata_mismatches() {
        let mut graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph.ops[0].out[0].shape = vec![2, 5];
        assert!(graph.validate_for_consensus().is_err());

        let mut graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph.ops[0].op = "unknown".to_owned();
        assert!(graph.validate_for_consensus().is_err());

        let mut graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph.ops[0]
            .kwargs
            .insert("bad".to_owned(), IrValue::Literal(IrLiteral::Bool(true)));
        assert!(graph.validate_for_consensus().is_err());
    }

    #[test]
    fn tier_c_vocabulary_is_carried_but_not_consensus_admitted() {
        let mut graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph.ops[0].op = "softmax".to_owned();
        graph.ops[0].args = vec![input_ref("a")];
        graph.ops[0]
            .kwargs
            .insert("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)));
        graph.ops[0].out[0] = tensor_spec("softmax", vec![2, 3], DType::FieldElement, 0);
        graph.outputs[0].value = op_ref(0);

        assert!(op_spec("softmax").is_some());
        assert!(graph.validate(false).is_ok());
        assert!(graph.validate_for_consensus().is_err());
    }

    #[test]
    fn frozen_registry_declares_verifier_class_for_every_op() {
        let mut names = BTreeSet::new();
        for spec in frozen_op_registry() {
            assert!(names.insert(spec.name), "duplicate op {}", spec.name);
            if spec.consensus_admitted {
                assert!(
                    !matches!(
                        spec.verification,
                        IrVerificationClass::CanonicalReferenceRequired
                            | IrVerificationClass::IndexConsistencyRequired
                    ),
                    "admitted op {} must have an implemented verifier class",
                    spec.name
                );
            }
            if spec.tier == IrOpTier::A && spec.consensus_admitted {
                assert_eq!(spec.verification, IrVerificationClass::FullFreivalds);
            }
        }

        for name in ["add", "sub", "scalar_mul", "sum", "reduce_sum", "mean"] {
            assert_eq!(
                op_spec(name).unwrap().verification,
                IrVerificationClass::RandomLinear,
                "{name} should have random-linear coverage"
            );
        }
        assert_eq!(
            op_spec("mul").unwrap().verification,
            IrVerificationClass::ExactDeterministicReplay
        );
    }

    #[test]
    fn index_ops_require_index_consistency_and_are_not_consensus_admitted() {
        for name in ["gather", "scatter", "embedding"] {
            let spec = op_spec(name).expect("index op vocabulary must be present");
            assert_eq!(
                spec.verification,
                IrVerificationClass::IndexConsistencyRequired
            );
            assert!(!spec.consensus_admitted);
        }

        let mut graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        graph
            .inputs
            .push(tensor_spec("index", vec![2, 3], DType::Int64, 0));
        graph.ops[0].op = "gather".to_owned();
        graph.ops[0].args = vec![input_ref("a"), input_ref("index")];
        graph.ops[0]
            .kwargs
            .insert("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)));
        graph.ops[0].out[0] = tensor_spec("gathered", vec![2, 3], DType::FieldElement, 0);
        graph.outputs[0].value = op_ref(0);

        assert!(graph.validate(false).is_ok());
        assert!(graph.validate_for_consensus().is_err());
    }

    #[test]
    fn linear_training_step_graph_validates_and_commits_shapes() {
        let graph =
            canonical_linear_training_step_graph(&[3, 2], &[2, 4], &[3, 4], DType::FieldElement);
        graph.validate_for_consensus().unwrap();
        assert_eq!(graph.ops.len(), 6);
        assert_eq!(graph.outputs.len(), 4);

        let mut changed = graph.clone();
        changed.params[0].name = "learning_rate".to_owned();
        assert_ne!(graph.graph_id(), changed.graph_id());
        assert!(changed.validate_for_consensus().is_err());
    }
}
