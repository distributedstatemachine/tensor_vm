use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Result, TvmError};
use crate::field::{self, Elem};
use crate::merkle::{MerkleProof, build_proof, merkle_root, verify_proof};
use crate::tensor::{
    DType, Tensor, add_elem_for_dtype, divide_elem_for_dtype, multiply_elem_for_dtype,
    packed_int8_payload_len, rescale_signed_elem_half_even, signed_elem_to_i128,
    signed_i128_to_elem, sub_elem_for_dtype,
};
use crate::types::{Hash, hash_bytes, parse_hash_hex};
use serde_json::Value as JsonValue;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrOutputCount {
    Exact(usize),
    KwargListLen(&'static str),
}

impl IrOutputCount {
    fn expected(self, kwargs: &BTreeMap<String, IrValue>) -> Result<usize> {
        match self {
            IrOutputCount::Exact(count) => Ok(count),
            IrOutputCount::KwargListLen(key) => literal_list_len_kwarg(kwargs, key),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpSpec {
    pub name: &'static str,
    pub tier: IrOpTier,
    pub arity: IrArity,
    pub output_count: IrOutputCount,
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
pub struct IrExecutionInputs {
    pub tensors: BTreeMap<String, Tensor>,
    pub field_params: BTreeMap<String, Elem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstBlobSpec {
    pub uri: String,
    pub shape: Vec<i64>,
    pub dtype: DType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrOpTrace {
    pub op_id: usize,
    pub input_roots: Vec<Hash>,
    pub output_roots: Vec<Hash>,
}

impl IrOpTrace {
    pub fn leaf_hash(&self) -> Hash {
        trace_op_leaf(self.op_id, &self.input_roots, &self.output_roots)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrExecution {
    pub graph_id: GraphId,
    pub outputs: BTreeMap<String, Tensor>,
    pub op_traces: Vec<IrOpTrace>,
    pub trace_root: Hash,
}

impl IrExecution {
    pub fn trace_leaves(&self) -> Vec<Hash> {
        self.op_traces.iter().map(IrOpTrace::leaf_hash).collect()
    }

    pub fn trace_opening(&self, op_index: u64) -> Result<IrTraceOpening> {
        let op_trace = self
            .op_traces
            .get(op_index as usize)
            .ok_or(TvmError::InvalidChunk {
                chunk_index: op_index,
            })?
            .clone();
        let leaves = self.trace_leaves();
        let proof = build_proof(&leaves, op_index)?;
        Ok(IrTraceOpening {
            trace_root: self.trace_root,
            op_index,
            op_trace,
            proof,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrTraceOpening {
    pub trace_root: Hash,
    pub op_index: u64,
    pub op_trace: IrOpTrace,
    pub proof: MerkleProof,
}

impl IrTraceOpening {
    pub fn verify(&self) -> bool {
        self.proof.leaf_index == self.op_index
            && self.op_trace.op_id as u64 == self.op_index
            && verify_proof(&self.trace_root, self.op_trace.leaf_hash(), &self.proof)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValueShape {
    shape: Vec<i64>,
    dtype: DType,
    scale: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RuntimeValue {
    Tensor(Tensor),
    Field(Elem),
}

impl From<IrOpWitnessValue> for RuntimeValue {
    fn from(value: IrOpWitnessValue) -> Self {
        match value {
            IrOpWitnessValue::Tensor(tensor) => Self::Tensor(tensor),
            IrOpWitnessValue::Field(value) => Self::Field(value),
        }
    }
}

impl From<RuntimeValue> for IrOpWitnessValue {
    fn from(value: RuntimeValue) -> Self {
        match value {
            RuntimeValue::Tensor(tensor) => Self::Tensor(tensor),
            RuntimeValue::Field(value) => Self::Field(value),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrOpWitnessValue {
    Tensor(Tensor),
    Field(Elem),
}

impl IrOpWitnessValue {
    pub fn commitment_root(&self) -> Hash {
        match self {
            Self::Tensor(tensor) => tensor.commitment_root(),
            Self::Field(value) => field_value_root(*value),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrOpRefereeWitness {
    pub op_index: u64,
    pub input_values: Vec<IrOpWitnessValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrOpRefereeVerdict {
    pub op_index: u64,
    pub input_roots: Vec<Hash>,
    pub canonical_output_roots: Vec<Hash>,
}

pub fn frozen_op_registry() -> &'static [OpSpec] {
    &FROZEN_OP_REGISTRY
}

pub fn op_spec(name: &str) -> Option<&'static OpSpec> {
    frozen_op_registry().iter().find(|spec| spec.name == name)
}

impl TensorGraph {
    pub fn from_canonical_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value: JsonValue = serde_json::from_slice(bytes)
            .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir graph json"))?;
        parse_tensor_graph_json(&value)
    }

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

    pub fn execute_exact(&self, inputs: &IrExecutionInputs) -> Result<IrExecution> {
        let graph_id = self.validate_for_consensus()?;
        validate_execution_inputs(self, inputs)?;

        let mut op_outputs = Vec::<Vec<RuntimeValue>>::new();
        let mut op_traces = Vec::with_capacity(self.ops.len());
        let mut trace_leaves = Vec::with_capacity(self.ops.len());

        for op in &self.ops {
            let args = op
                .args
                .iter()
                .map(|arg| {
                    resolve_runtime_ref(arg, &inputs.tensors, &inputs.field_params, &op_outputs)
                })
                .collect::<Result<Vec<_>>>()?;
            let input_roots = args.iter().map(runtime_value_root).collect::<Vec<_>>();
            let outputs = execute_op(op, args)?;
            let output_roots = outputs
                .iter()
                .map(|value| match value {
                    RuntimeValue::Tensor(tensor) => Ok(tensor.commitment_root()),
                    RuntimeValue::Field(_) => Err(TvmError::InvalidReceipt(
                        "tensor ir op produced scalar output",
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            trace_leaves.push(trace_op_leaf(op.id, &input_roots, &output_roots));
            op_traces.push(IrOpTrace {
                op_id: op.id,
                input_roots,
                output_roots,
            });
            op_outputs.push(outputs);
        }

        let mut outputs = BTreeMap::new();
        for output in &self.outputs {
            let value = resolve_runtime_ref(
                &output.value,
                &inputs.tensors,
                &inputs.field_params,
                &op_outputs,
            )?;
            match value {
                RuntimeValue::Tensor(tensor) => {
                    outputs.insert(output.name.clone(), tensor);
                }
                RuntimeValue::Field(_) => {
                    return Err(TvmError::InvalidReceipt(
                        "tensor ir graph output resolves to scalar",
                    ));
                }
            }
        }

        Ok(IrExecution {
            graph_id,
            outputs,
            op_traces,
            trace_root: merkle_root(&trace_leaves),
        })
    }

    pub fn referee_op(&self, witness: &IrOpRefereeWitness) -> Result<IrOpRefereeVerdict> {
        self.validate_for_consensus()?;
        let op = self
            .ops
            .get(witness.op_index as usize)
            .ok_or(TvmError::InvalidChunk {
                chunk_index: witness.op_index,
            })?;
        if op.id as u64 != witness.op_index {
            return Err(TvmError::InvalidReceipt("tensor ir op id mismatch"));
        }
        let input_roots = witness
            .input_values
            .iter()
            .map(IrOpWitnessValue::commitment_root)
            .collect::<Vec<_>>();
        let args = witness
            .input_values
            .iter()
            .cloned()
            .map(RuntimeValue::from)
            .collect::<Vec<_>>();
        let outputs = execute_op(op, args)?;
        let canonical_output_roots = outputs
            .iter()
            .map(|value| match value {
                RuntimeValue::Tensor(tensor) => Ok(tensor.commitment_root()),
                RuntimeValue::Field(_) => Err(TvmError::InvalidReceipt(
                    "tensor ir referee op produced scalar output",
                )),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(IrOpRefereeVerdict {
            op_index: witness.op_index,
            input_roots,
            canonical_output_roots,
        })
    }

    pub fn referee_witness(
        &self,
        inputs: &IrExecutionInputs,
        op_index: u64,
    ) -> Result<IrOpRefereeWitness> {
        self.validate_for_consensus()?;
        validate_execution_inputs(self, inputs)?;
        let target = op_index as usize;
        if target >= self.ops.len() {
            return Err(TvmError::InvalidChunk {
                chunk_index: op_index,
            });
        }

        let mut op_outputs = Vec::<Vec<RuntimeValue>>::new();
        for (index, op) in self.ops.iter().enumerate() {
            let args = op
                .args
                .iter()
                .map(|arg| {
                    resolve_runtime_ref(arg, &inputs.tensors, &inputs.field_params, &op_outputs)
                })
                .collect::<Result<Vec<_>>>()?;
            if index == target {
                return Ok(IrOpRefereeWitness {
                    op_index,
                    input_values: args.into_iter().map(IrOpWitnessValue::from).collect(),
                });
            }
            op_outputs.push(execute_op(op, args)?);
        }

        Err(TvmError::InvalidChunk {
            chunk_index: op_index,
        })
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
            if op.out.len() != spec.output_count.expected(&op.kwargs)? {
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

    pub fn const_blob_specs(&self) -> Result<BTreeMap<String, ConstBlobSpec>> {
        let mut specs = BTreeMap::new();
        for op in &self.ops {
            for arg in &op.args {
                collect_const_blob_ref(arg, &mut specs)?;
            }
            for value in op.kwargs.values() {
                collect_const_blob_value(value, &mut specs)?;
            }
        }
        for output in &self.outputs {
            collect_const_blob_ref(&output.value, &mut specs)?;
        }
        Ok(specs)
    }
}

fn validate_execution_inputs(graph: &TensorGraph, inputs: &IrExecutionInputs) -> Result<()> {
    for spec in &graph.inputs {
        let tensor = inputs
            .tensors
            .get(&spec.name)
            .ok_or(TvmError::InvalidReceipt(
                "missing tensor ir execution input",
            ))?;
        if tensor.dtype() != spec.dtype
            || tensor.scale() != spec.scale
            || tensor_shape_matches_spec(tensor.shape(), &spec.shape).is_err()
        {
            return Err(TvmError::InvalidReceipt(
                "tensor ir execution input mismatch",
            ));
        }
    }
    let const_blob_specs = graph.const_blob_specs()?;
    for name in inputs.tensors.keys() {
        if !graph.inputs.iter().any(|spec| spec.name == *name)
            && !const_blob_specs.contains_key(name)
        {
            return Err(TvmError::InvalidReceipt(
                "unknown tensor ir execution input",
            ));
        }
    }

    for (uri, spec) in &const_blob_specs {
        let tensor = inputs
            .tensors
            .get(uri)
            .ok_or(TvmError::InvalidReceipt("missing tensor ir const_blob"))?;
        validate_const_blob_tensor(spec, tensor)?;
    }

    for spec in &graph.params {
        if spec.type_name != "field_scalar" {
            return Err(TvmError::InvalidReceipt(
                "unsupported tensor ir execution param type",
            ));
        }
        if !inputs.field_params.contains_key(&spec.name) {
            return Err(TvmError::InvalidReceipt(
                "missing tensor ir execution param",
            ));
        }
    }
    for name in inputs.field_params.keys() {
        if !graph.params.iter().any(|spec| spec.name == *name) {
            return Err(TvmError::InvalidReceipt(
                "unknown tensor ir execution param",
            ));
        }
    }
    Ok(())
}

fn collect_const_blob_value(
    value: &IrValue,
    specs: &mut BTreeMap<String, ConstBlobSpec>,
) -> Result<()> {
    if let IrValue::Ref(reference) = value {
        collect_const_blob_ref(reference, specs)?;
    }
    Ok(())
}

fn collect_const_blob_ref(
    reference: &IrRef,
    specs: &mut BTreeMap<String, ConstBlobSpec>,
) -> Result<()> {
    if let IrRef::ConstBlob { uri, shape, dtype } = reference {
        let spec = ConstBlobSpec {
            uri: uri.clone(),
            shape: shape.clone(),
            dtype: *dtype,
        };
        if let Some(existing) = specs.get(uri) {
            if existing != &spec {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir const_blob spec mismatch",
                ));
            }
        } else {
            specs.insert(uri.clone(), spec);
        }
    }
    Ok(())
}

fn validate_const_blob_tensor(spec: &ConstBlobSpec, tensor: &Tensor) -> Result<()> {
    let expected_root = parse_hash_hex(&spec.uri)
        .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir const_blob uri"))?;
    if tensor.commitment_root() != expected_root
        || tensor.dtype() != spec.dtype
        || tensor.scale() != 0
        || tensor_shape_matches_spec(tensor.shape(), &spec.shape).is_err()
    {
        return Err(TvmError::InvalidReceipt("tensor ir const_blob mismatch"));
    }
    Ok(())
}

fn tensor_shape_matches_spec(shape: &[usize], spec: &[i64]) -> Result<()> {
    if shape.len() != spec.len() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir execution shape mismatch",
        ));
    }
    for (actual, declared) in shape.iter().zip(spec) {
        if *declared >= 0 && *actual as i64 != *declared {
            return Err(TvmError::InvalidReceipt(
                "tensor ir execution shape mismatch",
            ));
        }
    }
    Ok(())
}

fn resolve_runtime_ref(
    value: &IrRef,
    inputs: &BTreeMap<String, Tensor>,
    params: &BTreeMap<String, Elem>,
    op_outputs: &[Vec<RuntimeValue>],
) -> Result<RuntimeValue> {
    match value {
        IrRef::Input { name } => {
            inputs
                .get(name)
                .cloned()
                .map(RuntimeValue::Tensor)
                .ok_or(TvmError::InvalidReceipt(
                    "unknown tensor ir execution input",
                ))
        }
        IrRef::Op { id, idx } => op_outputs
            .get(*id)
            .and_then(|outputs| outputs.get(*idx))
            .cloned()
            .ok_or(TvmError::InvalidReceipt(
                "unknown tensor ir execution op ref",
            )),
        IrRef::Param { name } => params
            .get(name)
            .copied()
            .map(|value| RuntimeValue::Field(field::normalize(value)))
            .ok_or(TvmError::InvalidReceipt(
                "unknown tensor ir execution param",
            )),
        IrRef::Const { value } => literal_runtime_value(value),
        IrRef::ConstBlob { uri, .. } => inputs
            .get(uri)
            .cloned()
            .map(RuntimeValue::Tensor)
            .ok_or(TvmError::InvalidReceipt("missing tensor ir const_blob")),
    }
}

fn literal_runtime_value(value: &IrLiteral) -> Result<RuntimeValue> {
    match value {
        IrLiteral::Field(value) => Ok(RuntimeValue::Field(*value)),
        IrLiteral::Int(value) if *value >= 0 => Ok(RuntimeValue::Field(*value as Elem)),
        IrLiteral::Uint(value) => Ok(RuntimeValue::Field(*value as Elem)),
        IrLiteral::List(values) => values
            .iter()
            .map(literal_field)
            .collect::<Result<Vec<_>>>()
            .and_then(|data| Tensor::from_vec(vec![data.len()], DType::FieldElement, data))
            .map(RuntimeValue::Tensor),
        _ => Err(TvmError::InvalidReceipt(
            "unsupported tensor ir execution literal",
        )),
    }
}

fn literal_field(value: &IrLiteral) -> Result<Elem> {
    match value {
        IrLiteral::Field(value) => Ok(*value),
        IrLiteral::Int(value) if *value >= 0 => Ok(*value as Elem),
        IrLiteral::Uint(value) => Ok(*value as Elem),
        _ => Err(TvmError::InvalidReceipt(
            "unsupported tensor ir tensor literal",
        )),
    }
}

fn execute_op(op: &OpNode, args: Vec<RuntimeValue>) -> Result<Vec<RuntimeValue>> {
    if op.op == "quantize_int8_per_channel" {
        return quantize_int8_per_channel(one_tensor_value(&args)?, &op.kwargs);
    }
    if op.op == "split" {
        return split_tensor(one_tensor_value(&args)?, &op.kwargs)
            .map(|outputs| outputs.into_iter().map(RuntimeValue::Tensor).collect());
    }

    let output = match op.op.as_str() {
        "matmul" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            lhs.matmul(rhs)?
        }
        "einsum" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            einsum_tensor(lhs, rhs, &op.kwargs)?
        }
        "add" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            binary_add_sub_tensor(lhs, rhs, add_elem_for_dtype)?
        }
        "sub" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            binary_add_sub_tensor(lhs, rhs, sub_elem_for_dtype)?
        }
        "mul" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            binary_mul_tensor(lhs, rhs)?
        }
        "div" => {
            let [lhs, rhs] = two_tensor_values(&args)?;
            div_tensor(lhs, rhs)?
        }
        "scalar_mul" => {
            let (tensor, scalar) = tensor_and_scalar_values(&args)?;
            tensor.scalar_mul(scalar)?
        }
        "transpose" => one_tensor_value(&args)?.transpose()?,
        "sum" | "reduce_sum" => reduce_tensor(one_tensor_value(&args)?, &op.kwargs, false)?,
        "mean" => reduce_tensor(one_tensor_value(&args)?, &op.kwargs, true)?,
        "identity" => one_tensor_value(&args)?.clone(),
        "abs" => unary_tensor(one_tensor_value(&args)?, signed_abs)?,
        "sign" => unary_tensor(one_tensor_value(&args)?, signed_sign)?,
        "round" => round_tensor(one_tensor_value(&args)?)?,
        "relu" => unary_tensor(one_tensor_value(&args)?, signed_relu)?,
        "reshape" => reshape_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "broadcast" => broadcast_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "squeeze" => squeeze_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "unsqueeze" => unsqueeze_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "slice" => slice_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "tril" => triangular_tensor(one_tensor_value(&args)?, &op.kwargs, true)?,
        "triu" => triangular_tensor(one_tensor_value(&args)?, &op.kwargs, false)?,
        "neg" => {
            let tensor = one_tensor_value(&args)?;
            unary_tensor(tensor, |value| field::sub(0, value))?
        }
        "gt" => compare_tensors(&args, |lhs, rhs| lhs > rhs)?,
        "lt" => compare_tensors(&args, |lhs, rhs| lhs < rhs)?,
        "ge" => compare_tensors(&args, |lhs, rhs| lhs >= rhs)?,
        "le" => compare_tensors(&args, |lhs, rhs| lhs <= rhs)?,
        "eq" => compare_tensors(&args, |lhs, rhs| lhs == rhs)?,
        "where" => where_tensor(&args)?,
        "clamp" => clamp_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "cast" => cast_tensor(one_tensor_value(&args)?, &op.kwargs)?,
        "concat" => concat_tensors(&args, &op.kwargs)?,
        "stack" => stack_tensors(&args, &op.kwargs)?,
        "full" => full_tensor(&op.kwargs)?,
        "arange" => arange_tensor(&op.kwargs)?,
        "dequantize_int8_per_channel" => dequantize_int8_per_channel(&args)?,
        "quantize_pack_int8" => quantize_pack_int8(one_tensor_value(&args)?, &op.kwargs)?,
        "unpack_dequantize_int8" => unpack_dequantize_int8(one_tensor_value(&args)?, &op.kwargs)?,
        "exp" => crate::tensor::fixed_point_exp(one_tensor_value(&args)?)?,
        "log" => crate::tensor::fixed_point_log(one_tensor_value(&args)?)?,
        "sqrt" => crate::tensor::fixed_point_sqrt(one_tensor_value(&args)?)?,
        "sigmoid" => crate::tensor::fixed_point_sigmoid(one_tensor_value(&args)?)?,
        "tanh" => crate::tensor::fixed_point_tanh(one_tensor_value(&args)?)?,
        "silu" => crate::tensor::fixed_point_silu(one_tensor_value(&args)?)?,
        "gelu" => crate::tensor::fixed_point_gelu(one_tensor_value(&args)?)?,
        "softmax" => {
            let dim = optional_usize_kwarg(&op.kwargs, "dim")?
                .ok_or(TvmError::InvalidReceipt("softmax requires dim"))?;
            crate::tensor::fixed_point_softmax(one_tensor_value(&args)?, dim)?
        }
        _ => {
            return Err(TvmError::InvalidReceipt(
                "tensor ir op is not executable by exact interpreter",
            ));
        }
    };
    Ok(vec![RuntimeValue::Tensor(output)])
}

fn reduce_tensor(
    tensor: &Tensor,
    kwargs: &BTreeMap<String, IrValue>,
    mean: bool,
) -> Result<Tensor> {
    let dim = optional_usize_kwarg(kwargs, "dim")?;
    let keepdim = optional_bool_kwarg(kwargs, "keepdim")?.unwrap_or(false);
    let input_shape = tensor.shape();
    let mut output_shape = input_shape.to_vec();
    let reduce_count = if let Some(dim) = dim {
        if dim >= input_shape.len() {
            return Err(TvmError::InvalidReceipt("tensor ir reduction dim mismatch"));
        }
        let count = input_shape[dim];
        if keepdim {
            output_shape[dim] = 1;
        } else {
            output_shape.remove(dim);
        }
        count
    } else {
        if keepdim {
            output_shape.fill(1);
        } else {
            output_shape = vec![1];
        }
        tensor.len()
    };
    if mean && reduce_count == 0 {
        return Err(TvmError::InvalidReceipt("tensor ir mean over empty axis"));
    }

    let output_len = checked_usize_product(&output_shape)?;
    let mut data = vec![0; output_len];
    for (input_index, value) in tensor.as_slice().iter().enumerate() {
        let output_index =
            reduction_output_index(input_shape, &output_shape, dim, keepdim, input_index)?;
        data[output_index] = field::add(data[output_index], *value);
    }
    if mean {
        let inverse = field_inverse(reduce_count as Elem)?;
        for value in &mut data {
            *value = field::mul(*value, inverse);
        }
    }
    Tensor::from_vec_with_scale(output_shape, tensor.dtype(), tensor.scale(), data)
}

fn einsum_tensor(lhs: &Tensor, rhs: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let equation = matrix_contraction_einsum_equation(kwargs)?;
    if lhs.dtype() != rhs.dtype() || lhs.scale() != rhs.scale() {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    if lhs.shape().len() != 2 || rhs.shape().len() != 2 {
        return Err(TvmError::InvalidReceipt("tensor ir einsum rank mismatch"));
    }
    if lhs.shape()[equation.lhs_shared_axis] != rhs.shape()[equation.rhs_shared_axis] {
        return Err(TvmError::InvalidReceipt("tensor ir einsum shape mismatch"));
    }

    let lhs_matrix = if equation.lhs_shared_axis == 0 {
        lhs.transpose()?
    } else {
        lhs.clone()
    };
    let rhs_matrix = if equation.rhs_shared_axis == 1 {
        rhs.transpose()?
    } else {
        rhs.clone()
    };
    let product = lhs_matrix.matmul(&rhs_matrix)?;
    if equation.output_reversed {
        product.transpose()
    } else {
        Ok(product)
    }
}

fn reduction_output_index(
    input_shape: &[usize],
    output_shape: &[usize],
    dim: Option<usize>,
    keepdim: bool,
    input_index: usize,
) -> Result<usize> {
    let input_coords = unravel_index(input_shape, input_index)?;
    let output_coords = match dim {
        Some(dim) if keepdim => {
            let mut coords = input_coords;
            coords[dim] = 0;
            coords
        }
        Some(dim) => input_coords
            .into_iter()
            .enumerate()
            .filter_map(|(axis, coord)| (axis != dim).then_some(coord))
            .collect(),
        None if keepdim => vec![0; input_shape.len()],
        None => vec![0],
    };
    ravel_index(output_shape, &output_coords)
}

fn reshape_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let shape = concrete_shape_kwarg(kwargs, "shape")?;
    if checked_usize_product(&shape)? != tensor.len() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir reshape element mismatch",
        ));
    }
    Tensor::from_vec_with_scale(
        shape,
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().to_vec(),
    )
}

fn broadcast_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let shape = concrete_shape_kwarg(kwargs, "shape")?;
    let expected = broadcast_shape_usize(&[tensor.shape().to_vec(), shape.clone()])?;
    if expected != shape {
        return Err(TvmError::InvalidReceipt(
            "tensor ir broadcast shape mismatch",
        ));
    }
    let mut data = Vec::with_capacity(checked_usize_product(&shape)?);
    for index in 0..checked_usize_product(&shape)? {
        data.push(broadcast_value(tensor, &shape, index)?);
    }
    Tensor::from_vec_with_scale(shape, tensor.dtype(), tensor.scale(), data)
}

fn squeeze_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir squeeze requires explicit dim",
    ))?;
    let mut shape = tensor.shape().to_vec();
    if dim >= shape.len() || shape[dim] != 1 || shape.len() == 1 {
        return Err(TvmError::InvalidReceipt("tensor ir squeeze dim mismatch"));
    }
    shape.remove(dim);
    Tensor::from_vec_with_scale(
        shape,
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().to_vec(),
    )
}

fn unsqueeze_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir unsqueeze requires explicit dim",
    ))?;
    let mut shape = tensor.shape().to_vec();
    if dim > shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir unsqueeze dim mismatch"));
    }
    shape.insert(dim, 1);
    Tensor::from_vec_with_scale(
        shape,
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().to_vec(),
    )
}

fn slice_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir slice requires explicit dim",
    ))?;
    let start = optional_usize_kwarg(kwargs, "start")?
        .ok_or(TvmError::InvalidReceipt("tensor ir slice requires start"))?;
    let end = optional_usize_kwarg(kwargs, "end")?
        .ok_or(TvmError::InvalidReceipt("tensor ir slice requires end"))?;
    if dim >= tensor.shape().len() || start > end || end > tensor.shape()[dim] || start == end {
        return Err(TvmError::InvalidReceipt("tensor ir slice bounds mismatch"));
    }
    let mut shape = tensor.shape().to_vec();
    shape[dim] = end - start;
    let mut data = Vec::with_capacity(checked_usize_product(&shape)?);
    for output_index in 0..checked_usize_product(&shape)? {
        let mut coords = unravel_index(&shape, output_index)?;
        coords[dim] += start;
        data.push(tensor.as_slice()[ravel_index(tensor.shape(), &coords)?]);
    }
    Tensor::from_vec_with_scale(shape, tensor.dtype(), tensor.scale(), data)
}

fn split_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Vec<Tensor>> {
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir split requires explicit dim",
    ))?;
    if dim >= tensor.shape().len() {
        return Err(TvmError::InvalidReceipt("tensor ir split dim mismatch"));
    }
    let sizes = split_sizes_kwarg(kwargs, "sizes")?;
    let total = sizes.iter().try_fold(0usize, |acc, size| {
        acc.checked_add(*size)
            .ok_or(TvmError::InvalidReceipt("tensor ir split size mismatch"))
    })?;
    if total != tensor.shape()[dim] {
        return Err(TvmError::InvalidReceipt("tensor ir split size mismatch"));
    }

    let mut outputs = Vec::with_capacity(sizes.len());
    let mut offset = 0usize;
    for size in sizes {
        let mut shape = tensor.shape().to_vec();
        shape[dim] = size;
        let output_len = checked_usize_product(&shape)?;
        let mut data = Vec::with_capacity(output_len);
        for output_index in 0..output_len {
            let mut coords = unravel_index(&shape, output_index)?;
            coords[dim] += offset;
            data.push(tensor.as_slice()[ravel_index(tensor.shape(), &coords)?]);
        }
        outputs.push(Tensor::from_vec_with_scale(
            shape,
            tensor.dtype(),
            tensor.scale(),
            data,
        )?);
        offset = offset
            .checked_add(size)
            .ok_or(TvmError::InvalidReceipt("tensor ir split size mismatch"))?;
    }
    Ok(outputs)
}

fn triangular_tensor(
    tensor: &Tensor,
    kwargs: &BTreeMap<String, IrValue>,
    lower: bool,
) -> Result<Tensor> {
    if tensor.shape().len() != 2 {
        return Err(TvmError::InvalidReceipt(
            "tensor ir triangular rank mismatch",
        ));
    }
    let diagonal = integer_kwarg(kwargs, "diagonal")?;
    let [rows, cols] = [tensor.shape()[0], tensor.shape()[1]];
    let mut data = Vec::with_capacity(tensor.len());
    for row in 0..rows {
        for col in 0..cols {
            let keep = if lower {
                (col as i64) <= (row as i64).saturating_add(diagonal)
            } else {
                (col as i64) >= (row as i64).saturating_add(diagonal)
            };
            data.push(if keep {
                tensor.as_slice()[row * cols + col]
            } else {
                0
            });
        }
    }
    Tensor::from_vec_with_scale(
        tensor.shape().to_vec(),
        tensor.dtype(),
        tensor.scale(),
        data,
    )
}

fn binary_add_sub_tensor(
    lhs: &Tensor,
    rhs: &Tensor,
    op: impl Fn(DType, i64, i64, Elem, Elem) -> Result<Elem>,
) -> Result<Tensor> {
    if lhs.dtype() != rhs.dtype() || (lhs.dtype() != DType::Fixed32 && lhs.scale() != rhs.scale()) {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    let shape = broadcast_shape_usize(&[lhs.shape().to_vec(), rhs.shape().to_vec()])?;
    let len = checked_usize_product(&shape)?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        data.push(op(
            lhs.dtype(),
            lhs.scale(),
            rhs.scale(),
            broadcast_value(lhs, &shape, index)?,
            broadcast_value(rhs, &shape, index)?,
        )?);
    }
    Tensor::from_vec_with_scale(shape, lhs.dtype(), lhs.scale(), data)
}

fn binary_mul_tensor(lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
    if lhs.dtype() != rhs.dtype() || (lhs.dtype() != DType::Fixed32 && lhs.scale() != rhs.scale()) {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    let shape = broadcast_shape_usize(&[lhs.shape().to_vec(), rhs.shape().to_vec()])?;
    let len = checked_usize_product(&shape)?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        data.push(multiply_elem_for_dtype(
            lhs.dtype(),
            lhs.scale(),
            rhs.scale(),
            lhs.scale(),
            broadcast_value(lhs, &shape, index)?,
            broadcast_value(rhs, &shape, index)?,
        )?);
    }
    Tensor::from_vec_with_scale(shape, lhs.dtype(), lhs.scale(), data)
}

fn div_tensor(lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
    let valid = match lhs.dtype() {
        DType::FieldElement => {
            rhs.dtype() == DType::FieldElement && lhs.scale() == 0 && rhs.scale() == 0
        }
        DType::Fixed32 => rhs.dtype() == DType::Fixed32,
        _ => false,
    };
    if !valid {
        return Err(TvmError::InvalidReceipt("tensor ir div dtype mismatch"));
    }
    let shape = broadcast_shape_usize(&[lhs.shape().to_vec(), rhs.shape().to_vec()])?;
    let len = checked_usize_product(&shape)?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        let numerator = broadcast_value(lhs, &shape, index)?;
        let divisor = broadcast_value(rhs, &shape, index)?;
        data.push(divide_elem_for_dtype(
            lhs.dtype(),
            lhs.scale(),
            rhs.scale(),
            lhs.scale(),
            numerator,
            divisor,
        )?);
    }
    Tensor::from_vec_with_scale(shape, lhs.dtype(), lhs.scale(), data)
}

fn unary_tensor(tensor: &Tensor, op: impl Fn(Elem) -> Elem) -> Result<Tensor> {
    Tensor::from_vec_with_scale(
        tensor.shape().to_vec(),
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().iter().map(|value| op(*value)).collect(),
    )
}

fn signed_abs(value: Elem) -> Elem {
    if signed_field_is_negative(value) {
        field::sub(0, value)
    } else {
        field::normalize(value)
    }
}

fn signed_sign(value: Elem) -> Elem {
    let value = field::normalize(value);
    if value == 0 {
        0
    } else if signed_field_is_negative(value) {
        field::MODULUS - 1
    } else {
        1
    }
}

fn signed_relu(value: Elem) -> Elem {
    if signed_field_is_negative(value) {
        0
    } else {
        field::normalize(value)
    }
}

fn signed_field_is_negative(value: Elem) -> bool {
    field::normalize(value) > field::MODULUS / 2
}

fn compare_tensors(
    values: &[RuntimeValue],
    predicate: impl Fn(Elem, Elem) -> bool,
) -> Result<Tensor> {
    let [lhs, rhs] = two_tensor_values(values)?;
    if lhs.dtype() != rhs.dtype() || lhs.scale() != rhs.scale() {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    let shape = broadcast_shape_usize(&[lhs.shape().to_vec(), rhs.shape().to_vec()])?;
    let len = checked_usize_product(&shape)?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        let value = if predicate(
            broadcast_value(lhs, &shape, index)?,
            broadcast_value(rhs, &shape, index)?,
        ) {
            1
        } else {
            0
        };
        data.push(value);
    }
    Tensor::from_vec(shape, DType::Int32, data)
}

fn where_tensor(values: &[RuntimeValue]) -> Result<Tensor> {
    let [cond, when_true, when_false] = match values {
        [
            RuntimeValue::Tensor(cond),
            RuntimeValue::Tensor(when_true),
            RuntimeValue::Tensor(when_false),
        ] => [cond, when_true, when_false],
        _ => {
            return Err(TvmError::InvalidReceipt(
                "tensor ir expected tensor arguments",
            ));
        }
    };
    if cond.dtype() != DType::Int32
        || cond.scale() != 0
        || when_true.dtype() != when_false.dtype()
        || when_true.scale() != when_false.scale()
    {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    let shape = broadcast_shape_usize(&[
        cond.shape().to_vec(),
        when_true.shape().to_vec(),
        when_false.shape().to_vec(),
    ])?;
    let len = checked_usize_product(&shape)?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        let selected = if broadcast_value(cond, &shape, index)? == 0 {
            when_false
        } else {
            when_true
        };
        data.push(broadcast_value(selected, &shape, index)?);
    }
    Tensor::from_vec_with_scale(shape, when_true.dtype(), when_true.scale(), data)
}

fn clamp_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let min = field_kwarg(kwargs, "min")?;
    let max = field_kwarg(kwargs, "max")?;
    if min > max {
        return Err(TvmError::InvalidReceipt("tensor ir clamp bounds mismatch"));
    }
    Tensor::from_vec_with_scale(
        tensor.shape().to_vec(),
        tensor.dtype(),
        tensor.scale(),
        tensor
            .as_slice()
            .iter()
            .map(|value| field::normalize(*value).clamp(min, max))
            .collect(),
    )
}

fn full_tensor(kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let shape = concrete_shape_kwarg(kwargs, "shape")?;
    let dtype = dtype_kwarg(kwargs, "dtype")?;
    let value = field_kwarg(kwargs, "value")?;
    Tensor::from_vec_with_scale(
        shape.clone(),
        dtype,
        scale_kwarg(kwargs, "scale")?.unwrap_or(0),
        vec![value; checked_usize_product(&shape)?],
    )
}

fn arange_tensor(kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let start = integer_kwarg(kwargs, "start")?;
    let end = integer_kwarg(kwargs, "end")?;
    let step = integer_kwarg(kwargs, "step")?;
    if step <= 0 || end < start {
        return Err(TvmError::InvalidReceipt("invalid tensor ir arange bounds"));
    }
    let len = arange_len(start, end, step)?;
    let dtype = dtype_kwarg(kwargs, "dtype")?;
    let mut data = Vec::with_capacity(len);
    let mut value = start;
    while value < end {
        data.push(signed_field(value));
        value = value
            .checked_add(step)
            .ok_or(TvmError::InvalidReceipt("invalid tensor ir arange bounds"))?;
    }
    Tensor::from_vec_with_scale(
        vec![len],
        dtype,
        scale_kwarg(kwargs, "scale")?.unwrap_or(0),
        data,
    )
}

fn cast_tensor(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let dtype = dtype_kwarg(kwargs, "dtype")?;
    let target_scale = scale_kwarg(kwargs, "scale")?.unwrap_or_else(|| {
        if dtype == DType::Fixed32 {
            tensor.scale()
        } else {
            0
        }
    });
    if dtype != DType::Fixed32 && target_scale != 0 {
        return Err(TvmError::InvalidReceipt(
            "tensor ir non-fixed scale mismatch",
        ));
    }
    let data = tensor
        .as_slice()
        .iter()
        .map(|value| rescale_signed_elem_half_even(*value, tensor.scale(), target_scale))
        .collect::<Result<Vec<_>>>()?;
    Tensor::from_vec_with_scale(tensor.shape().to_vec(), dtype, target_scale, data)
}

fn round_tensor(tensor: &Tensor) -> Result<Tensor> {
    if tensor.dtype() != DType::Fixed32 {
        return Ok(tensor.clone());
    }
    let data = tensor
        .as_slice()
        .iter()
        .map(|value| rescale_signed_elem_half_even(*value, tensor.scale(), 0))
        .collect::<Result<Vec<_>>>()?;
    Tensor::from_vec_with_scale(tensor.shape().to_vec(), tensor.dtype(), 0, data)
}

fn quantize_int8_per_channel(
    tensor: &Tensor,
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<Vec<RuntimeValue>> {
    if tensor.dtype() != DType::Fixed32 {
        return Err(TvmError::InvalidReceipt(
            "tensor ir quantize requires fixed32 input",
        ));
    }
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir quantize requires explicit dim",
    ))?;
    if dim >= tensor.shape().len() {
        return Err(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"));
    }
    let channels = tensor.shape()[dim];
    let mut max_abs = vec![0i128; channels];
    for (index, value) in tensor.as_slice().iter().enumerate() {
        let channel = unravel_index(tensor.shape(), index)?[dim];
        max_abs[channel] = max_abs[channel].max(signed_elem_to_i128(*value).abs());
    }
    let scales = max_abs
        .iter()
        .map(|value| ((*value + 126) / 127).max(1))
        .collect::<Vec<_>>();
    let mut quantized = Vec::with_capacity(tensor.len());
    for (index, value) in tensor.as_slice().iter().enumerate() {
        let channel = unravel_index(tensor.shape(), index)?[dim];
        let raw = signed_elem_to_i128(*value);
        let rounded = div_round_half_even_i128(raw, scales[channel])?.clamp(-128, 127);
        quantized.push(signed_i128_to_elem(rounded));
    }
    let q = Tensor::from_vec(tensor.shape().to_vec(), DType::Int8, quantized)?;
    let scale = Tensor::from_vec_with_scale(
        vec![channels],
        DType::Fixed32,
        tensor.scale(),
        scales
            .into_iter()
            .map(signed_i128_to_elem)
            .collect::<Vec<_>>(),
    )?;
    Ok(vec![RuntimeValue::Tensor(q), RuntimeValue::Tensor(scale)])
}

fn dequantize_int8_per_channel(values: &[RuntimeValue]) -> Result<Tensor> {
    let [quantized, scale] = two_tensor_values(values)?;
    if quantized.dtype() != DType::Int8
        || quantized.scale() != 0
        || scale.dtype() != DType::Fixed32
        || scale.shape().len() != 1
    {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    let channel_dim = dequantize_channel_dim(quantized.shape(), scale.len())?;
    let mut data = Vec::with_capacity(quantized.len());
    for (index, value) in quantized.as_slice().iter().enumerate() {
        let channel = if scale.len() == 1 {
            0
        } else {
            unravel_index(quantized.shape(), index)?[channel_dim]
        };
        let raw = signed_elem_to_i128(*value)
            .checked_mul(signed_elem_to_i128(scale.as_slice()[channel]))
            .ok_or(TvmError::InvalidReceipt("tensor ir quantize overflow"))?;
        data.push(signed_i128_to_elem(raw));
    }
    Tensor::from_vec_with_scale(
        quantized.shape().to_vec(),
        DType::Fixed32,
        scale.scale(),
        data,
    )
}

fn dequantize_channel_dim(shape: &[usize], scale_len: usize) -> Result<usize> {
    if scale_len == 1 {
        return Ok(0);
    }
    let mut matches = shape
        .iter()
        .enumerate()
        .filter_map(|(axis, dim)| (*dim == scale_len).then_some(axis));
    let dim = matches.next().ok_or(TvmError::InvalidReceipt(
        "tensor ir dequantize scale mismatch",
    ))?;
    if matches.next().is_some() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir dequantize scale ambiguous",
        ));
    }
    Ok(dim)
}

fn quantize_pack_int8(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let [quantized, scale] = quantize_int8_per_channel(tensor, kwargs)?
        .try_into()
        .map_err(|_| TvmError::InvalidReceipt("tensor ir quantize output mismatch"))?;
    let RuntimeValue::Tensor(quantized) = quantized else {
        return Err(TvmError::InvalidReceipt(
            "tensor ir quantize output mismatch",
        ));
    };
    let RuntimeValue::Tensor(scale) = scale else {
        return Err(TvmError::InvalidReceipt(
            "tensor ir quantize output mismatch",
        ));
    };
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir quantize requires explicit dim",
    ))?;
    Tensor::from_packed_int8_payload(
        quantized.shape().to_vec(),
        dim,
        scale.scale(),
        scale.as_slice(),
        quantized.as_slice(),
    )
}

fn unpack_dequantize_int8(tensor: &Tensor, kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let expected_shape = concrete_shape_kwarg(kwargs, "shape")?;
    let expected_dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir packed quantize dim mismatch",
    ))?;
    let expected_scale = integer_kwarg(kwargs, "scale_dim")?;
    let decoded = tensor.packed_int8_payload()?;
    if decoded.shape != expected_shape
        || decoded.axis != expected_dim
        || decoded.output_scale != expected_scale
    {
        return Err(TvmError::InvalidReceipt(
            "tensor ir packed quantize metadata mismatch",
        ));
    }
    let q = Tensor::from_vec(decoded.shape.clone(), DType::Int8, decoded.quantized)?;
    let scale = Tensor::from_vec_with_scale(
        vec![decoded.scales.len()],
        DType::Fixed32,
        decoded.output_scale,
        decoded.scales,
    )?;
    dequantize_int8_per_channel(&[RuntimeValue::Tensor(q), RuntimeValue::Tensor(scale)])
}

fn div_round_half_even_i128(value: i128, divisor: i128) -> Result<i128> {
    if divisor <= 0 {
        return Err(TvmError::InvalidReceipt(
            "tensor ir quantize scale mismatch",
        ));
    }
    let sign = if value < 0 { -1 } else { 1 };
    let abs = value.abs();
    let quotient = abs / divisor;
    let remainder = abs % divisor;
    let twice = remainder
        .checked_mul(2)
        .ok_or(TvmError::InvalidReceipt("tensor ir quantize overflow"))?;
    let rounded_abs = if twice > divisor || (twice == divisor && quotient % 2 == 1) {
        quotient
            .checked_add(1)
            .ok_or(TvmError::InvalidReceipt("tensor ir quantize overflow"))?
    } else {
        quotient
    };
    Ok(if sign < 0 { -rounded_abs } else { rounded_abs })
}

fn concat_tensors(values: &[RuntimeValue], kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let tensors = tensor_values(values)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir concat requires explicit dim",
    ))?;
    let output_shape = infer_concat_shape_from_tensors(&tensors, dim)?;
    let output_len = checked_usize_product(&output_shape)?;
    let mut data = Vec::with_capacity(output_len);
    let mut dim_starts = Vec::with_capacity(tensors.len());
    let mut next_start = 0usize;
    for tensor in &tensors {
        dim_starts.push(next_start);
        next_start = next_start
            .checked_add(tensor.shape()[dim])
            .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
    }
    for output_index in 0..output_len {
        let coords = unravel_index(&output_shape, output_index)?;
        let dim_coord = coords[dim];
        let source_index = tensors
            .iter()
            .enumerate()
            .find_map(|(index, tensor)| {
                let start = dim_starts[index];
                let end = start + tensor.shape()[dim];
                (dim_coord >= start && dim_coord < end).then_some(index)
            })
            .ok_or(TvmError::InvalidReceipt("tensor ir concat index mismatch"))?;
        let source = tensors[source_index];
        let mut source_coords = coords;
        source_coords[dim] -= dim_starts[source_index];
        data.push(source.as_slice()[ravel_index(source.shape(), &source_coords)?]);
    }
    Tensor::from_vec_with_scale(output_shape, tensors[0].dtype(), tensors[0].scale(), data)
}

fn stack_tensors(values: &[RuntimeValue], kwargs: &BTreeMap<String, IrValue>) -> Result<Tensor> {
    let tensors = tensor_values(values)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?.ok_or(TvmError::InvalidReceipt(
        "tensor ir stack requires explicit dim",
    ))?;
    let output_shape = infer_stack_shape_from_tensors(&tensors, dim)?;
    let output_len = checked_usize_product(&output_shape)?;
    let mut data = Vec::with_capacity(output_len);
    for output_index in 0..output_len {
        let coords = unravel_index(&output_shape, output_index)?;
        let source_index = coords[dim];
        let source = tensors[source_index];
        let source_coords = coords
            .into_iter()
            .enumerate()
            .filter_map(|(axis, coord)| (axis != dim).then_some(coord))
            .collect::<Vec<_>>();
        data.push(source.as_slice()[ravel_index(source.shape(), &source_coords)?]);
    }
    Tensor::from_vec_with_scale(output_shape, tensors[0].dtype(), tensors[0].scale(), data)
}

fn infer_concat_shape_from_tensors(tensors: &[&Tensor], dim: usize) -> Result<Vec<usize>> {
    if tensors.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir variadic op requires args",
        ));
    }
    let first = tensors[0];
    if dim >= first.shape().len() {
        return Err(TvmError::InvalidReceipt("tensor ir concat dim mismatch"));
    }
    let mut shape = first.shape().to_vec();
    for tensor in &tensors[1..] {
        if tensor.dtype() != first.dtype() || tensor.shape().len() != first.shape().len() {
            return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
        }
        for (axis, shape_dim) in shape.iter_mut().enumerate() {
            if axis == dim {
                *shape_dim = shape_dim
                    .checked_add(tensor.shape()[axis])
                    .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
            } else if tensor.shape()[axis] != first.shape()[axis] {
                return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
            }
        }
    }
    Ok(shape)
}

fn infer_stack_shape_from_tensors(tensors: &[&Tensor], dim: usize) -> Result<Vec<usize>> {
    if tensors.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir variadic op requires args",
        ));
    }
    let first = tensors[0];
    if dim > first.shape().len() {
        return Err(TvmError::InvalidReceipt("tensor ir stack dim mismatch"));
    }
    for tensor in &tensors[1..] {
        if tensor.dtype() != first.dtype() || tensor.shape() != first.shape() {
            return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
        }
    }
    let mut shape = first.shape().to_vec();
    shape.insert(dim, tensors.len());
    Ok(shape)
}

fn broadcast_value(tensor: &Tensor, output_shape: &[usize], output_index: usize) -> Result<Elem> {
    let mut coords = vec![0usize; output_shape.len()];
    let mut remainder = output_index;
    for axis in (0..output_shape.len()).rev() {
        let dim = output_shape[axis];
        if dim == 0 {
            return Err(TvmError::InvalidReceipt("tensor ir zero-dim broadcast"));
        }
        coords[axis] = remainder % dim;
        remainder /= dim;
    }
    let rank_offset =
        output_shape
            .len()
            .checked_sub(tensor.shape().len())
            .ok_or(TvmError::InvalidReceipt(
                "tensor ir broadcast rank mismatch",
            ))?;
    let mut flat = 0usize;
    for (axis, dim) in tensor.shape().iter().enumerate() {
        let coord = if *dim == 1 {
            0
        } else {
            coords[rank_offset + axis]
        };
        if coord >= *dim {
            return Err(TvmError::InvalidReceipt(
                "tensor ir broadcast shape mismatch",
            ));
        }
        flat = flat
            .checked_mul(*dim)
            .and_then(|value| value.checked_add(coord))
            .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
    }
    tensor
        .as_slice()
        .get(flat)
        .copied()
        .ok_or(TvmError::InvalidReceipt(
            "tensor ir broadcast index mismatch",
        ))
}

fn unravel_index(shape: &[usize], mut index: usize) -> Result<Vec<usize>> {
    if shape.is_empty() {
        if index == 0 {
            return Ok(Vec::new());
        }
        return Err(TvmError::InvalidReceipt("tensor ir index mismatch"));
    }
    let mut coords = vec![0usize; shape.len()];
    for axis in (0..shape.len()).rev() {
        let dim = shape[axis];
        if dim == 0 {
            return Err(TvmError::InvalidReceipt("tensor ir zero-dim index"));
        }
        coords[axis] = index % dim;
        index /= dim;
    }
    Ok(coords)
}

fn ravel_index(shape: &[usize], coords: &[usize]) -> Result<usize> {
    if shape.len() != coords.len() {
        return Err(TvmError::InvalidReceipt("tensor ir index mismatch"));
    }
    let mut index = 0usize;
    for (dim, coord) in shape.iter().zip(coords) {
        if *coord >= *dim {
            return Err(TvmError::InvalidReceipt("tensor ir index mismatch"));
        }
        index = index
            .checked_mul(*dim)
            .and_then(|value| value.checked_add(*coord))
            .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
    }
    Ok(index)
}

fn one_tensor_value(values: &[RuntimeValue]) -> Result<&Tensor> {
    match values {
        [RuntimeValue::Tensor(tensor)] => Ok(tensor),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor argument",
        )),
    }
}

fn tensor_values(values: &[RuntimeValue]) -> Result<Vec<&Tensor>> {
    values
        .iter()
        .map(|value| match value {
            RuntimeValue::Tensor(tensor) => Ok(tensor),
            RuntimeValue::Field(_) => Err(TvmError::InvalidReceipt(
                "tensor ir expected tensor arguments",
            )),
        })
        .collect()
}

fn two_tensor_values(values: &[RuntimeValue]) -> Result<[&Tensor; 2]> {
    match values {
        [RuntimeValue::Tensor(lhs), RuntimeValue::Tensor(rhs)] => Ok([lhs, rhs]),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor arguments",
        )),
    }
}

fn tensor_and_scalar_values(values: &[RuntimeValue]) -> Result<(&Tensor, Elem)> {
    match values {
        [RuntimeValue::Tensor(tensor), RuntimeValue::Field(scalar)] => Ok((tensor, *scalar)),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor and scalar arguments",
        )),
    }
}

fn runtime_value_root(value: &RuntimeValue) -> Hash {
    match value {
        RuntimeValue::Tensor(tensor) => tensor.commitment_root(),
        RuntimeValue::Field(value) => field_value_root(*value),
    }
}

fn field_value_root(value: Elem) -> Hash {
    hash_bytes(b"tensor-vm-ir-field-value-v1", &[&value.to_le_bytes()])
}

fn trace_op_leaf(op_id: usize, input_roots: &[Hash], output_roots: &[Hash]) -> Hash {
    let mut encoded = Vec::with_capacity(24 + input_roots.len() * 32 + output_roots.len() * 32);
    encoded.extend_from_slice(&(op_id as u64).to_le_bytes());
    encoded.extend_from_slice(&(input_roots.len() as u64).to_le_bytes());
    for root in input_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(&(output_roots.len() as u64).to_le_bytes());
    for root in output_roots {
        encoded.extend_from_slice(root);
    }
    hash_bytes(b"tensor-vm-ir-trace-op-v1", &[&encoded])
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
            same_matmul_dtype(lhs, rhs)?;
            ValueShape {
                shape: vec![lhs.shape[0], rhs.shape[1]],
                dtype: lhs.dtype,
                scale: lhs.scale,
            }
        }
        "einsum" => infer_einsum(args, kwargs)?,
        "add" | "sub" => {
            let [lhs, rhs] = two_args(args)?;
            same_add_sub_dtype(lhs, rhs)?;
            ValueShape {
                shape: broadcast_shape_i64(&[lhs.shape.clone(), rhs.shape.clone()])?,
                dtype: lhs.dtype,
                scale: lhs.scale,
            }
        }
        "mul" => {
            let [lhs, rhs] = two_args(args)?;
            same_mul_dtype(lhs, rhs)?;
            ValueShape {
                shape: broadcast_shape_i64(&[lhs.shape.clone(), rhs.shape.clone()])?,
                dtype: lhs.dtype,
                scale: lhs.scale,
            }
        }
        "div" => {
            let [lhs, rhs] = two_args(args)?;
            same_div_dtype(lhs, rhs)?;
            ValueShape {
                shape: broadcast_shape_i64(&[lhs.shape.clone(), rhs.shape.clone()])?,
                dtype: lhs.dtype,
                scale: if lhs.dtype == DType::FieldElement {
                    0
                } else {
                    lhs.scale
                },
            }
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
            if op == "reshape" && shape_element_count(&arg.shape)? != shape_element_count(&shape)? {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir reshape element mismatch",
                ));
            }
            if op == "broadcast"
                && broadcast_shape_i64(&[arg.shape.clone(), shape.clone()])? != shape
            {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir broadcast shape mismatch",
                ));
            }
            ValueShape {
                shape,
                dtype: arg.dtype,
                scale: arg.scale,
            }
        }
        "squeeze" => infer_squeeze(args, kwargs)?,
        "unsqueeze" => infer_unsqueeze(args, kwargs)?,
        "slice" => infer_slice(args, kwargs)?,
        "split" => return infer_split_shapes(args, kwargs),
        "tril" | "triu" => {
            let arg = one_arg(args)?;
            if arg.shape.len() != 2 {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir triangular rank mismatch",
                ));
            }
            integer_kwarg(kwargs, "diagonal")?;
            arg.clone()
        }
        "identity" | "neg" | "abs" | "sign" | "relu" => one_arg(args)?.clone(),
        "round" => {
            let arg = one_arg(args)?;
            ValueShape {
                shape: arg.shape.clone(),
                dtype: arg.dtype,
                scale: if arg.dtype == DType::Fixed32 {
                    0
                } else {
                    arg.scale
                },
            }
        }
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
            same_dtype(lhs, rhs)?;
            ValueShape {
                shape: broadcast_shape_i64(&[lhs.shape.clone(), rhs.shape.clone()])?,
                dtype: DType::Int32,
                scale: 0,
            }
        }
        "where" => {
            if args.len() != 3 {
                return Err(TvmError::InvalidReceipt("tensor ir where arity mismatch"));
            }
            if args[0].dtype != DType::Int32 {
                return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
            }
            same_dtype(&args[1], &args[2])?;
            ValueShape {
                shape: broadcast_shape_i64(&[
                    args[0].shape.clone(),
                    args[1].shape.clone(),
                    args[2].shape.clone(),
                ])?,
                dtype: args[1].dtype,
                scale: args[1].scale,
            }
        }
        "clamp" => {
            let arg = one_arg(args)?;
            let min = field_kwarg(kwargs, "min")?;
            let max = field_kwarg(kwargs, "max")?;
            if min > max {
                return Err(TvmError::InvalidReceipt("tensor ir clamp bounds mismatch"));
            }
            arg.clone()
        }
        "cast" => {
            let arg = one_arg(args)?;
            let dtype = dtype_kwarg(kwargs, "dtype")?;
            let scale = scale_kwarg(kwargs, "scale")?.unwrap_or_else(|| {
                if dtype == DType::Fixed32 {
                    arg.scale
                } else {
                    0
                }
            });
            if dtype != DType::Fixed32 && scale != 0 {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir non-fixed scale mismatch",
                ));
            }
            ValueShape {
                shape: arg.shape.clone(),
                dtype,
                scale,
            }
        }
        "concat" => infer_concat_shape(args, kwargs)?,
        "stack" => infer_stack_shape(args, kwargs)?,
        "full" => ValueShape {
            shape: shape_kwarg(kwargs, "shape")?,
            dtype: dtype_kwarg(kwargs, "dtype")?,
            scale: scale_kwarg(kwargs, "scale")?.unwrap_or(0),
        },
        "arange" => ValueShape {
            shape: vec![arange_len(
                integer_kwarg(kwargs, "start")?,
                integer_kwarg(kwargs, "end")?,
                integer_kwarg(kwargs, "step")?,
            )? as i64],
            dtype: dtype_kwarg(kwargs, "dtype")?,
            scale: scale_kwarg(kwargs, "scale")?.unwrap_or(0),
        },
        "quantize_int8_per_channel" => {
            let arg = one_arg(args)?;
            if arg.dtype != DType::Fixed32 {
                return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
            }
            let dim = optional_usize_kwarg(kwargs, "dim")?
                .ok_or(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"))?;
            if dim >= arg.shape.len() {
                return Err(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"));
            }
            return Ok(vec![
                ValueShape {
                    shape: arg.shape.clone(),
                    dtype: DType::Int8,
                    scale: 0,
                },
                ValueShape {
                    shape: vec![arg.shape[dim]],
                    dtype: DType::Fixed32,
                    scale: arg.scale,
                },
            ]);
        }
        "dequantize_int8_per_channel" => {
            let [quantized, scale] = two_args(args)?;
            if quantized.dtype != DType::Int8
                || quantized.scale != 0
                || scale.dtype != DType::Fixed32
                || scale.shape.len() != 1
            {
                return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
            }
            let scale_len = usize::try_from(scale.shape[0])
                .map_err(|_| TvmError::InvalidReceipt("tensor ir dequantize scale mismatch"))?;
            let q_shape = quantized
                .shape
                .iter()
                .map(|dim| {
                    usize::try_from(*dim).map_err(|_| {
                        TvmError::InvalidReceipt("tensor ir dequantize scale mismatch")
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            dequantize_channel_dim(&q_shape, scale_len)?;
            ValueShape {
                shape: quantized.shape.clone(),
                dtype: DType::Fixed32,
                scale: scale.scale,
            }
        }
        "quantize_pack_int8" => {
            let arg = one_arg(args)?;
            if arg.dtype != DType::Fixed32 {
                return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
            }
            let dim = optional_usize_kwarg(kwargs, "dim")?
                .ok_or(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"))?;
            let shape = arg
                .shape
                .iter()
                .map(|dim| {
                    usize::try_from(*dim)
                        .map_err(|_| TvmError::InvalidReceipt("tensor ir shape mismatch"))
                })
                .collect::<Result<Vec<_>>>()?;
            let packed_len = packed_int8_payload_len(&shape, dim)?;
            ValueShape {
                shape: vec![packed_len as i64],
                dtype: DType::Uint8,
                scale: 0,
            }
        }
        "unpack_dequantize_int8" => {
            let arg = one_arg(args)?;
            if arg.dtype != DType::Uint8 || arg.scale != 0 || arg.shape.len() != 1 {
                return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
            }
            let shape = concrete_shape_kwarg(kwargs, "shape")?;
            let dim = optional_usize_kwarg(kwargs, "dim")?
                .ok_or(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"))?;
            if dim >= shape.len() {
                return Err(TvmError::InvalidReceipt("tensor ir quantize dim mismatch"));
            }
            let scale = integer_kwarg(kwargs, "scale_dim")?;
            let packed_len = packed_int8_payload_len(&shape, dim)?;
            if arg.shape[0] != packed_len as i64 {
                return Err(TvmError::InvalidReceipt(
                    "tensor ir packed quantize length mismatch",
                ));
            }
            ValueShape {
                shape: shape.into_iter().map(|dim| dim as i64).collect(),
                dtype: DType::Fixed32,
                scale,
            }
        }
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

fn infer_concat_shape(
    args: &[ValueShape],
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<ValueShape> {
    if args.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir variadic op requires args",
        ));
    }
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir concat dim mismatch"))?;
    if dim >= args[0].shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir concat dim mismatch"));
    }
    let mut output = args[0].clone();
    for arg in &args[1..] {
        same_dtype(&args[0], arg)?;
        if arg.shape.len() != args[0].shape.len() {
            return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
        }
        for axis in 0..output.shape.len() {
            if axis == dim {
                output.shape[axis] = output.shape[axis]
                    .checked_add(arg.shape[axis])
                    .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
            } else if arg.shape[axis] != args[0].shape[axis] {
                return Err(TvmError::InvalidReceipt("tensor ir shape mismatch"));
            }
        }
    }
    Ok(output)
}

fn infer_stack_shape(
    args: &[ValueShape],
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<ValueShape> {
    if args.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "tensor ir variadic op requires args",
        ));
    }
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir stack dim mismatch"))?;
    if dim > args[0].shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir stack dim mismatch"));
    }
    for arg in &args[1..] {
        same_tensor(&args[0], arg)?;
    }
    let mut output = args[0].clone();
    output.shape.insert(dim, args.len() as i64);
    Ok(output)
}

fn infer_squeeze(args: &[ValueShape], kwargs: &BTreeMap<String, IrValue>) -> Result<ValueShape> {
    let arg = one_arg(args)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir squeeze dim mismatch"))?;
    if dim >= arg.shape.len() || arg.shape[dim] != 1 || arg.shape.len() == 1 {
        return Err(TvmError::InvalidReceipt("tensor ir squeeze dim mismatch"));
    }
    let mut output = arg.clone();
    output.shape.remove(dim);
    Ok(output)
}

fn infer_unsqueeze(args: &[ValueShape], kwargs: &BTreeMap<String, IrValue>) -> Result<ValueShape> {
    let arg = one_arg(args)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir unsqueeze dim mismatch"))?;
    if dim > arg.shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir unsqueeze dim mismatch"));
    }
    let mut output = arg.clone();
    output.shape.insert(dim, 1);
    Ok(output)
}

fn infer_slice(args: &[ValueShape], kwargs: &BTreeMap<String, IrValue>) -> Result<ValueShape> {
    let arg = one_arg(args)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir slice dim mismatch"))?;
    let start = optional_usize_kwarg(kwargs, "start")?
        .ok_or(TvmError::InvalidReceipt("tensor ir slice bounds mismatch"))?;
    let end = optional_usize_kwarg(kwargs, "end")?
        .ok_or(TvmError::InvalidReceipt("tensor ir slice bounds mismatch"))?;
    if dim >= arg.shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir slice dim mismatch"));
    }
    let dim_len = usize::try_from(arg.shape[dim])
        .map_err(|_| TvmError::InvalidReceipt("tensor ir slice bounds mismatch"))?;
    if start > end || end > dim_len || start == end {
        return Err(TvmError::InvalidReceipt("tensor ir slice bounds mismatch"));
    }
    let mut output = arg.clone();
    output.shape[dim] = (end - start) as i64;
    Ok(output)
}

fn infer_einsum(args: &[ValueShape], kwargs: &BTreeMap<String, IrValue>) -> Result<ValueShape> {
    let [lhs, rhs] = two_args(args)?;
    same_dtype(lhs, rhs)?;
    if lhs.shape.len() != 2 || rhs.shape.len() != 2 {
        return Err(TvmError::InvalidReceipt("tensor ir einsum rank mismatch"));
    }
    let equation = matrix_contraction_einsum_equation(kwargs)?;
    if lhs.shape[equation.lhs_shared_axis] != rhs.shape[equation.rhs_shared_axis] {
        return Err(TvmError::InvalidReceipt("tensor ir einsum shape mismatch"));
    }
    let lhs_free = lhs.shape[1 - equation.lhs_shared_axis];
    let rhs_free = rhs.shape[1 - equation.rhs_shared_axis];
    let shape = if equation.output_reversed {
        vec![rhs_free, lhs_free]
    } else {
        vec![lhs_free, rhs_free]
    };
    Ok(ValueShape {
        shape,
        dtype: lhs.dtype,
        scale: lhs.scale,
    })
}

fn infer_split_shapes(
    args: &[ValueShape],
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<Vec<ValueShape>> {
    let arg = one_arg(args)?;
    let dim = optional_usize_kwarg(kwargs, "dim")?
        .ok_or(TvmError::InvalidReceipt("tensor ir split dim mismatch"))?;
    if dim >= arg.shape.len() {
        return Err(TvmError::InvalidReceipt("tensor ir split dim mismatch"));
    }
    let dim_len = usize::try_from(arg.shape[dim])
        .map_err(|_| TvmError::InvalidReceipt("tensor ir split size mismatch"))?;
    let sizes = split_sizes_kwarg(kwargs, "sizes")?;
    let total = sizes.iter().try_fold(0usize, |acc, size| {
        acc.checked_add(*size)
            .ok_or(TvmError::InvalidReceipt("tensor ir split size mismatch"))
    })?;
    if total != dim_len {
        return Err(TvmError::InvalidReceipt("tensor ir split size mismatch"));
    }
    sizes
        .into_iter()
        .map(|size| {
            let mut output = arg.clone();
            output.shape[dim] = size as i64;
            Ok(output)
        })
        .collect()
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

fn same_add_sub_dtype(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    if lhs.dtype != rhs.dtype || (lhs.dtype != DType::Fixed32 && lhs.scale != rhs.scale) {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    Ok(())
}

fn same_mul_dtype(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    if lhs.dtype != rhs.dtype || (lhs.dtype != DType::Fixed32 && lhs.scale != rhs.scale) {
        return Err(TvmError::InvalidReceipt("tensor ir dtype mismatch"));
    }
    Ok(())
}

fn same_matmul_dtype(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    let valid = match lhs.dtype {
        DType::FieldElement => rhs.dtype == DType::FieldElement && lhs.scale == 0 && rhs.scale == 0,
        DType::Fixed32 => rhs.dtype == DType::Fixed32,
        _ => false,
    };
    if !valid {
        return Err(TvmError::InvalidReceipt("tensor ir matmul dtype mismatch"));
    }
    Ok(())
}

fn same_div_dtype(lhs: &ValueShape, rhs: &ValueShape) -> Result<()> {
    let valid = match lhs.dtype {
        DType::FieldElement => rhs.dtype == DType::FieldElement && lhs.scale == 0 && rhs.scale == 0,
        DType::Fixed32 => rhs.dtype == DType::Fixed32,
        _ => false,
    };
    if !valid {
        return Err(TvmError::InvalidReceipt("tensor ir div dtype mismatch"));
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MatrixContractionEinsum {
    lhs_shared_axis: usize,
    rhs_shared_axis: usize,
    output_reversed: bool,
}

fn matrix_contraction_einsum_equation(
    kwargs: &BTreeMap<String, IrValue>,
) -> Result<MatrixContractionEinsum> {
    let equation = string_kwarg(kwargs, "equation")?;
    parse_matrix_contraction_einsum(equation)
}

fn parse_matrix_contraction_einsum(equation: &str) -> Result<MatrixContractionEinsum> {
    let (inputs, output) = equation.split_once("->").ok_or(TvmError::InvalidReceipt(
        "tensor ir einsum equation mismatch",
    ))?;
    let (lhs, rhs) = inputs.split_once(',').ok_or(TvmError::InvalidReceipt(
        "tensor ir einsum equation mismatch",
    ))?;
    let lhs = einsum_labels(lhs)?;
    let rhs = einsum_labels(rhs)?;
    let output = einsum_labels(output)?;
    if lhs.len() != 2 || rhs.len() != 2 || output.len() != 2 {
        return Err(TvmError::InvalidReceipt(
            "tensor ir einsum equation mismatch",
        ));
    }
    if lhs[0] == lhs[1] || rhs[0] == rhs[1] || output[0] == output[1] {
        return Err(TvmError::InvalidReceipt(
            "tensor ir einsum equation mismatch",
        ));
    }
    let shared = lhs
        .iter()
        .copied()
        .find(|label| rhs.contains(label))
        .ok_or(TvmError::InvalidReceipt(
            "tensor ir einsum equation mismatch",
        ))?;
    let lhs_shared_axis =
        lhs.iter()
            .position(|label| *label == shared)
            .ok_or(TvmError::InvalidReceipt(
                "tensor ir einsum equation mismatch",
            ))?;
    let rhs_shared_axis =
        rhs.iter()
            .position(|label| *label == shared)
            .ok_or(TvmError::InvalidReceipt(
                "tensor ir einsum equation mismatch",
            ))?;
    let lhs_free = lhs[1 - lhs_shared_axis];
    let rhs_free = rhs[1 - rhs_shared_axis];
    if output == [lhs_free, rhs_free] {
        Ok(MatrixContractionEinsum {
            lhs_shared_axis,
            rhs_shared_axis,
            output_reversed: false,
        })
    } else if output == [rhs_free, lhs_free] {
        Ok(MatrixContractionEinsum {
            lhs_shared_axis,
            rhs_shared_axis,
            output_reversed: true,
        })
    } else {
        Err(TvmError::InvalidReceipt(
            "tensor ir einsum equation mismatch",
        ))
    }
}

fn einsum_labels(value: &str) -> Result<Vec<char>> {
    value
        .chars()
        .map(|label| {
            if label.is_ascii_alphabetic() {
                Ok(label)
            } else {
                Err(TvmError::InvalidReceipt(
                    "tensor ir einsum equation mismatch",
                ))
            }
        })
        .collect()
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

fn string_kwarg<'a>(kwargs: &'a BTreeMap<String, IrValue>, key: &str) -> Result<&'a str> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::String(value))) => Ok(value),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir string kwarg")),
    }
}

fn literal_list_len_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<usize> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::List(values))) if !values.is_empty() => Ok(values.len()),
        Some(IrValue::Literal(IrLiteral::List(_))) => {
            Err(TvmError::InvalidReceipt("empty tensor ir list kwarg"))
        }
        _ => Err(TvmError::InvalidReceipt("missing tensor ir list kwarg")),
    }
}

fn split_sizes_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Vec<usize>> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::List(values))) if !values.is_empty() => values
            .iter()
            .map(|value| match value {
                IrLiteral::Int(size) if *size > 0 => Ok(*size as usize),
                IrLiteral::Uint(size) if *size > 0 => usize::try_from(*size)
                    .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir split size")),
                _ => Err(TvmError::InvalidReceipt("invalid tensor ir split size")),
            })
            .collect(),
        Some(IrValue::Literal(IrLiteral::List(_))) => {
            Err(TvmError::InvalidReceipt("invalid tensor ir split size"))
        }
        _ => Err(TvmError::InvalidReceipt("missing tensor ir split sizes")),
    }
}

fn concrete_shape_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Vec<usize>> {
    shape_kwarg(kwargs, key)?
        .into_iter()
        .map(|dim| {
            usize::try_from(dim)
                .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir shape kwarg"))
        })
        .collect()
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

fn integer_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<i64> {
    match kwargs.get(key) {
        Some(IrValue::Literal(IrLiteral::Int(value))) => Ok(*value),
        Some(IrValue::Literal(IrLiteral::Uint(value))) => i64::try_from(*value)
            .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir integer kwarg")),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir integer kwarg")),
    }
}

fn scale_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Option<i64>> {
    match kwargs.get(key) {
        None => Ok(None),
        Some(IrValue::Literal(IrLiteral::Int(value))) => Ok(Some(*value)),
        Some(IrValue::Literal(IrLiteral::Uint(value))) => i64::try_from(*value)
            .map(Some)
            .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir integer kwarg")),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir integer kwarg")),
    }
}

fn field_kwarg(kwargs: &BTreeMap<String, IrValue>, key: &str) -> Result<Elem> {
    match kwargs.get(key) {
        Some(IrValue::Literal(value)) => literal_field(value),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir field kwarg")),
    }
}

fn signed_field(value: i64) -> Elem {
    let modulus = field::MODULUS as i128;
    let reduced = (value as i128).rem_euclid(modulus);
    reduced as Elem
}

fn field_inverse(value: Elem) -> Result<Elem> {
    let value = field::normalize(value);
    if value == 0 {
        return Err(TvmError::InvalidReceipt("tensor ir division by zero"));
    }
    Ok(field_pow(value, field::MODULUS - 2))
}

fn field_pow(mut base: Elem, mut exponent: Elem) -> Elem {
    let mut acc = 1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            acc = field::mul(acc, base);
        }
        base = field::mul(base, base);
        exponent >>= 1;
    }
    acc
}

fn arange_len(start: i64, end: i64, step: i64) -> Result<usize> {
    if step <= 0 || end < start {
        return Err(TvmError::InvalidReceipt("invalid tensor ir arange bounds"));
    }
    let span = end - start;
    usize::try_from((span + step - 1) / step)
        .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir arange bounds"))
}

fn shape_element_count(shape: &[i64]) -> Result<i128> {
    let mut product = 1i128;
    for dim in shape {
        if *dim < 0 {
            return Err(TvmError::InvalidReceipt("invalid tensor ir shape kwarg"));
        }
        product = product
            .checked_mul(*dim as i128)
            .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))?;
    }
    Ok(product)
}

fn checked_usize_product(shape: &[usize]) -> Result<usize> {
    shape.iter().try_fold(1usize, |product, dim| {
        product
            .checked_mul(*dim)
            .ok_or(TvmError::InvalidReceipt("tensor ir shape overflow"))
    })
}

fn broadcast_shape_i64(shapes: &[Vec<i64>]) -> Result<Vec<i64>> {
    let concrete = shapes
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|dim| {
                    usize::try_from(*dim)
                        .map_err(|_| TvmError::InvalidReceipt("tensor ir broadcast shape mismatch"))
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    broadcast_shape_usize(&concrete).map(|shape| shape.into_iter().map(|dim| dim as i64).collect())
}

fn broadcast_shape_usize(shapes: &[Vec<usize>]) -> Result<Vec<usize>> {
    let rank = shapes.iter().map(Vec::len).max().unwrap_or(0);
    let mut output = vec![1usize; rank];
    for shape in shapes {
        for (offset, dim) in shape.iter().rev().enumerate() {
            let output_index = rank - 1 - offset;
            match (output[output_index], *dim) {
                (1, value) => output[output_index] = value,
                (_, 1) => {}
                (current, value) if current == value => {}
                _ => {
                    return Err(TvmError::InvalidReceipt(
                        "tensor ir broadcast shape mismatch",
                    ));
                }
            }
        }
    }
    Ok(output)
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
        "int8" => Some(DType::Int8),
        "uint8" => Some(DType::Uint8),
        "bool" => Some(DType::Bool),
        _ => None,
    }
}

fn dtype_name(dtype: DType) -> &'static str {
    match dtype {
        DType::Int32 => "int32",
        DType::Int64 => "int64",
        DType::Fixed32 => "fixed32",
        DType::FieldElement => "field",
        DType::Int8 => "int8",
        DType::Uint8 => "uint8",
        DType::Bool => "bool",
    }
}

fn parse_tensor_graph_json(value: &JsonValue) -> Result<TensorGraph> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir graph json"))?;
    Ok(TensorGraph {
        ir_version: json_u64(object.get("ir_version"), "invalid tensor ir version")?,
        inputs: json_array(object.get("inputs"), "invalid tensor ir inputs")?
            .iter()
            .map(parse_tensor_spec_json)
            .collect::<Result<Vec<_>>>()?,
        params: json_array(object.get("params"), "invalid tensor ir params")?
            .iter()
            .map(parse_param_spec_json)
            .collect::<Result<Vec<_>>>()?,
        ops: json_array(object.get("ops"), "invalid tensor ir ops")?
            .iter()
            .map(parse_op_json)
            .collect::<Result<Vec<_>>>()?,
        outputs: json_array(object.get("outputs"), "invalid tensor ir outputs")?
            .iter()
            .map(parse_graph_output_json)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn parse_tensor_spec_json(value: &JsonValue) -> Result<TensorSpec> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir tensor spec"))?;
    Ok(TensorSpec {
        name: json_string(object.get("name"), "invalid tensor ir tensor name")?.to_owned(),
        shape: json_array(object.get("shape"), "invalid tensor ir tensor shape")?
            .iter()
            .map(|value| json_i64(Some(value), "invalid tensor ir tensor shape"))
            .collect::<Result<Vec<_>>>()?,
        dtype: dtype_from_name(json_string(
            object.get("dtype"),
            "invalid tensor ir tensor dtype",
        )?)
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir tensor dtype"))?,
        scale: json_i64(object.get("scale"), "invalid tensor ir tensor scale")?,
    })
}

fn parse_param_spec_json(value: &JsonValue) -> Result<ParamSpec> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir param"))?;
    Ok(ParamSpec {
        name: json_string(object.get("name"), "invalid tensor ir param name")?.to_owned(),
        type_name: json_string(object.get("type"), "invalid tensor ir param type")?.to_owned(),
    })
}

fn parse_op_json(value: &JsonValue) -> Result<OpNode> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir op"))?;
    let kwargs = object
        .get("kwargs")
        .and_then(JsonValue::as_object)
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir kwargs"))?
        .iter()
        .map(|(key, value)| Ok((key.clone(), parse_value_json(value)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(OpNode {
        id: json_u64(object.get("id"), "invalid tensor ir op id")? as usize,
        op: json_string(object.get("op"), "invalid tensor ir op name")?.to_owned(),
        args: json_array(object.get("args"), "invalid tensor ir op args")?
            .iter()
            .map(parse_ref_json)
            .collect::<Result<Vec<_>>>()?,
        kwargs,
        out: json_array(object.get("out"), "invalid tensor ir op outputs")?
            .iter()
            .map(parse_tensor_spec_json)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn parse_graph_output_json(value: &JsonValue) -> Result<GraphOutput> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir graph output"))?;
    Ok(GraphOutput {
        name: json_string(object.get("name"), "invalid tensor ir output name")?.to_owned(),
        value: parse_ref_json(
            object
                .get("ref")
                .ok_or(TvmError::InvalidReceipt("invalid tensor ir output ref"))?,
        )?,
    })
}

fn parse_value_json(value: &JsonValue) -> Result<IrValue> {
    if value
        .as_object()
        .and_then(|object| object.get("kind"))
        .is_some()
    {
        Ok(IrValue::Ref(parse_ref_json(value)?))
    } else {
        Ok(IrValue::Literal(parse_literal_json(value)?))
    }
}

fn parse_ref_json(value: &JsonValue) -> Result<IrRef> {
    let object = value
        .as_object()
        .ok_or(TvmError::InvalidReceipt("invalid tensor ir ref"))?;
    match json_string(object.get("kind"), "invalid tensor ir ref kind")? {
        "input" => Ok(IrRef::Input {
            name: json_string(object.get("name"), "invalid tensor ir input ref")?.to_owned(),
        }),
        "op" => Ok(IrRef::Op {
            id: json_u64(object.get("id"), "invalid tensor ir op ref")? as usize,
            idx: json_u64(object.get("idx"), "invalid tensor ir op ref")? as usize,
        }),
        "param" => Ok(IrRef::Param {
            name: json_string(object.get("name"), "invalid tensor ir param ref")?.to_owned(),
        }),
        "const" => Ok(IrRef::Const {
            value: parse_literal_json(
                object
                    .get("value")
                    .ok_or(TvmError::InvalidReceipt("invalid tensor ir const ref"))?,
            )?,
        }),
        "const_blob" => Ok(IrRef::ConstBlob {
            uri: json_string(object.get("uri"), "invalid tensor ir const blob")?.to_owned(),
            shape: json_array(object.get("shape"), "invalid tensor ir const blob")?
                .iter()
                .map(|value| json_i64(Some(value), "invalid tensor ir const blob"))
                .collect::<Result<Vec<_>>>()?,
            dtype: dtype_from_name(json_string(
                object.get("dtype"),
                "invalid tensor ir const blob",
            )?)
            .ok_or(TvmError::InvalidReceipt("invalid tensor ir const blob"))?,
        }),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir ref kind")),
    }
}

fn parse_literal_json(value: &JsonValue) -> Result<IrLiteral> {
    match value {
        JsonValue::Bool(value) => Ok(IrLiteral::Bool(*value)),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(IrLiteral::Int(value))
            } else if let Some(value) = value.as_u64() {
                Ok(IrLiteral::Uint(value))
            } else {
                Err(TvmError::InvalidReceipt("invalid tensor ir literal"))
            }
        }
        JsonValue::String(value) => {
            if value.len() == 16 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
                u64::from_str_radix(value, 16)
                    .map(IrLiteral::Field)
                    .map_err(|_| TvmError::InvalidReceipt("invalid tensor ir field literal"))
            } else {
                Ok(IrLiteral::String(value.clone()))
            }
        }
        JsonValue::Array(values) => values
            .iter()
            .map(parse_literal_json)
            .collect::<Result<Vec<_>>>()
            .map(IrLiteral::List),
        _ => Err(TvmError::InvalidReceipt("invalid tensor ir literal")),
    }
}

fn json_array<'a>(value: Option<&'a JsonValue>, error: &'static str) -> Result<&'a Vec<JsonValue>> {
    value
        .and_then(JsonValue::as_array)
        .ok_or(TvmError::InvalidReceipt(error))
}

fn json_string<'a>(value: Option<&'a JsonValue>, error: &'static str) -> Result<&'a str> {
    value
        .and_then(JsonValue::as_str)
        .ok_or(TvmError::InvalidReceipt(error))
}

fn json_i64(value: Option<&JsonValue>, error: &'static str) -> Result<i64> {
    value
        .and_then(JsonValue::as_i64)
        .ok_or(TvmError::InvalidReceipt(error))
}

fn json_u64(value: Option<&JsonValue>, error: &'static str) -> Result<u64> {
    value
        .and_then(JsonValue::as_u64)
        .ok_or(TvmError::InvalidReceipt(error))
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

const FROZEN_OP_REGISTRY: [OpSpec; 53] = [
    OpSpec {
        name: "matmul",
        tier: IrOpTier::A,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::FullFreivalds,
        consensus_admitted: true,
    },
    OpSpec {
        name: "einsum",
        tier: IrOpTier::A,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["equation"],
        required_kwargs: &["equation"],
        verification: IrVerificationClass::FullFreivalds,
        consensus_admitted: true,
    },
    OpSpec {
        name: "add",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sub",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "mul",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "div",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "scalar_mul",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "transpose",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dims"],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sum",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "reduce_sum",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "mean",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim", "keepdim"],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "reshape",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["shape"],
        required_kwargs: &["shape"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "broadcast",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["shape"],
        required_kwargs: &["shape"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "squeeze",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "unsqueeze",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "slice",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim", "start", "end"],
        required_kwargs: &["dim", "start", "end"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "tril",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["diagonal"],
        required_kwargs: &["diagonal"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "triu",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["diagonal"],
        required_kwargs: &["diagonal"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "identity",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "neg",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::RandomLinear,
        consensus_admitted: true,
    },
    OpSpec {
        name: "abs",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "sign",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "round",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "relu",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "gt",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "lt",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "ge",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "le",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "eq",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "where",
        tier: IrOpTier::B,
        arity: IrArity::Exact(3),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "clamp",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["min", "max"],
        required_kwargs: &["min", "max"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "cast",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dtype", "scale"],
        required_kwargs: &["dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "concat",
        tier: IrOpTier::B,
        arity: IrArity::Variadic,
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "stack",
        tier: IrOpTier::B,
        arity: IrArity::Variadic,
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "split",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::KwargListLen("sizes"),
        allowed_kwargs: &["sizes", "dim"],
        required_kwargs: &["sizes", "dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "full",
        tier: IrOpTier::B,
        arity: IrArity::Exact(0),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["shape", "value", "dtype", "scale"],
        required_kwargs: &["shape", "value", "dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "arange",
        tier: IrOpTier::B,
        arity: IrArity::Exact(0),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["start", "end", "step", "dtype", "scale"],
        required_kwargs: &["start", "end", "step", "dtype"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "exp",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "log",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "sqrt",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "softmax",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "sigmoid",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "tanh",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "silu",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "gelu",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "gather",
        tier: IrOpTier::C,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "scatter",
        tier: IrOpTier::C,
        arity: IrArity::Exact(3),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "embedding",
        tier: IrOpTier::C,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::IndexConsistencyRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "topk",
        tier: IrOpTier::C,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(2),
        allowed_kwargs: &["k", "dim"],
        required_kwargs: &["k", "dim"],
        verification: IrVerificationClass::CanonicalReferenceRequired,
        consensus_admitted: false,
    },
    OpSpec {
        name: "quantize_int8_per_channel",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(2),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "dequantize_int8_per_channel",
        tier: IrOpTier::B,
        arity: IrArity::Exact(2),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &[],
        required_kwargs: &[],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "quantize_pack_int8",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim"],
        required_kwargs: &["dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
    },
    OpSpec {
        name: "unpack_dequantize_int8",
        tier: IrOpTier::B,
        arity: IrArity::Exact(1),
        output_count: IrOutputCount::Exact(1),
        allowed_kwargs: &["dim", "shape", "scale_dim"],
        required_kwargs: &["dim", "shape", "scale_dim"],
        verification: IrVerificationClass::ExactDeterministicReplay,
        consensus_admitted: true,
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
    fn graph_json_roundtrips_narrow_integer_dtypes() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("x", vec![3], DType::Int8, 0),
                tensor_spec("mask", vec![3], DType::Bool, 0),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "cast".to_owned(),
                args: vec![input_ref("x")],
                kwargs: BTreeMap::from([(
                    "dtype".to_owned(),
                    IrValue::Literal(IrLiteral::String("uint8".to_owned())),
                )]),
                out: vec![tensor_spec("y", vec![3], DType::Uint8, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: op_ref(0),
            }],
        };
        let canonical = graph.canonical_json();
        assert!(canonical.contains("\"dtype\":\"int8\""));
        assert!(canonical.contains("\"dtype\":\"uint8\""));
        assert!(canonical.contains("\"dtype\":\"bool\""));
        let parsed = TensorGraph::from_canonical_json_bytes(canonical.as_bytes()).unwrap();
        assert_eq!(parsed, graph);
        let mut changed = graph.clone();
        changed.inputs[0].dtype = DType::Uint8;
        assert_ne!(changed.graph_id(), graph.graph_id());
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
    fn quantization_vocabulary_admits_exact_quantization_ops() {
        let expected = [
            (
                "quantize_int8_per_channel",
                IrArity::Exact(1),
                IrOutputCount::Exact(2),
                &["dim"][..],
                IrVerificationClass::ExactDeterministicReplay,
                true,
            ),
            (
                "dequantize_int8_per_channel",
                IrArity::Exact(2),
                IrOutputCount::Exact(1),
                &[][..],
                IrVerificationClass::ExactDeterministicReplay,
                true,
            ),
            (
                "quantize_pack_int8",
                IrArity::Exact(1),
                IrOutputCount::Exact(1),
                &["dim"][..],
                IrVerificationClass::ExactDeterministicReplay,
                true,
            ),
            (
                "unpack_dequantize_int8",
                IrArity::Exact(1),
                IrOutputCount::Exact(1),
                &["dim", "shape", "scale_dim"][..],
                IrVerificationClass::ExactDeterministicReplay,
                true,
            ),
        ];
        for (name, arity, output_count, kwargs, verification, admitted) in expected {
            let spec = op_spec(name).expect("quantization op vocabulary must be present");
            assert_eq!(spec.tier, IrOpTier::B);
            assert_eq!(spec.arity, arity);
            assert_eq!(spec.output_count, output_count);
            assert_eq!(spec.allowed_kwargs, kwargs);
            assert_eq!(spec.required_kwargs, kwargs);
            assert_eq!(spec.verification, verification);
            assert_eq!(spec.consensus_admitted, admitted);
        }

        let mut graph = canonical_matmul_graph(2, 3, 4, DType::Fixed32);
        graph.ops[0].op = "quantize_pack_int8".to_owned();
        graph.ops[0].args = vec![input_ref("a")];
        graph.ops[0]
            .kwargs
            .insert("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(0)));
        graph.ops[0].out = vec![tensor_spec("packed", vec![54], DType::Uint8, 0)];
        graph.outputs = vec![GraphOutput {
            name: "packed".to_owned(),
            value: IrRef::Op { id: 0, idx: 0 },
        }];
        assert!(graph.validate_for_consensus().is_ok());
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
    fn exact_interpreter_executes_hand_built_graph_and_commits_trace() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::FieldElement, 0),
                tensor_spec("rhs", vec![2, 2], DType::FieldElement, 0),
                tensor_spec("bias", vec![2, 2], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "matmul".to_owned(),
                    args: vec![input_ref("lhs"), input_ref("rhs")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("product", vec![2, 2], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 1,
                    op: "add".to_owned(),
                    args: vec![op_ref(0), input_ref("bias")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("biased", vec![2, 2], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 2,
                    op: "reduce_sum".to_owned(),
                    args: vec![op_ref(1)],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(1)),
                    )]),
                    out: vec![tensor_spec("row_sum", vec![2], DType::FieldElement, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "biased".to_owned(),
                    value: op_ref(1),
                },
                GraphOutput {
                    name: "row_sum".to_owned(),
                    value: op_ref(2),
                },
            ],
        };
        let lhs = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let rhs = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
        let bias = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![9, 10, 11, 12]).unwrap();
        let expected_biased = lhs.matmul(&rhs).unwrap().add(&bias).unwrap();
        let expected_row_sum = expected_biased.reduce_sum(1).unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    ("lhs".to_owned(), lhs),
                    ("rhs".to_owned(), rhs),
                    ("bias".to_owned(), bias.clone()),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(execution.graph_id, graph.graph_id());
        assert_eq!(execution.outputs["biased"], expected_biased);
        assert_eq!(execution.outputs["row_sum"], expected_row_sum);
        assert_eq!(execution.op_traces.len(), 3);
        assert_eq!(
            execution.op_traces[1].output_roots[0],
            expected_biased.commitment_root()
        );
        assert_eq!(
            execution.op_traces[2].output_roots[0],
            expected_row_sum.commitment_root()
        );
        let trace_leaves: Vec<_> = execution
            .op_traces
            .iter()
            .map(|trace| trace_op_leaf(trace.op_id, &trace.input_roots, &trace.output_roots))
            .collect();
        assert_eq!(execution.trace_root, merkle_root(&trace_leaves));
        assert_eq!(execution.trace_leaves(), trace_leaves);
        let opening = execution.trace_opening(1).unwrap();
        assert_eq!(opening.trace_root, execution.trace_root);
        assert_eq!(opening.op_index, 1);
        assert_eq!(opening.op_trace.op_id, 1);
        assert_eq!(
            opening.op_trace.input_roots,
            vec![
                execution.op_traces[0].output_roots[0],
                bias.commitment_root()
            ]
        );
        assert_eq!(
            opening.op_trace.output_roots[0],
            expected_biased.commitment_root()
        );
        assert!(opening.verify());
        let witness = graph
            .referee_witness(
                &IrExecutionInputs {
                    tensors: BTreeMap::from([
                        (
                            "lhs".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4])
                                .unwrap(),
                        ),
                        (
                            "rhs".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8])
                                .unwrap(),
                        ),
                        (
                            "bias".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![9, 10, 11, 12])
                                .unwrap(),
                        ),
                    ]),
                    field_params: BTreeMap::new(),
                },
                1,
            )
            .unwrap();
        let verdict = graph.referee_op(&witness).unwrap();
        assert_eq!(witness.op_index, 1);
        assert_eq!(verdict.input_roots, opening.op_trace.input_roots);
        assert_eq!(
            verdict.canonical_output_roots,
            opening.op_trace.output_roots
        );
        let mut tampered_input = opening.clone();
        tampered_input.op_trace.input_roots[0] = expected_row_sum.commitment_root();
        assert!(!tampered_input.verify());
        let mut tampered = opening.clone();
        tampered.op_trace.output_roots[0] = expected_row_sum.commitment_root();
        assert!(!tampered.verify());
        assert_eq!(
            execution.trace_opening(99),
            Err(TvmError::InvalidChunk { chunk_index: 99 })
        );
        assert_eq!(
            execution.trace_root,
            graph
                .execute_exact(&IrExecutionInputs {
                    tensors: BTreeMap::from([
                        (
                            "lhs".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4])
                                .unwrap()
                        ),
                        (
                            "rhs".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8])
                                .unwrap()
                        ),
                        (
                            "bias".to_owned(),
                            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![9, 10, 11, 12])
                                .unwrap()
                        ),
                    ]),
                    field_params: BTreeMap::new(),
                })
                .unwrap()
                .trace_root
        );
    }

    #[test]
    fn exact_interpreter_executes_const_blob_by_content_uri() {
        let input = Tensor::from_vec(vec![2], DType::FieldElement, vec![1, 2]).unwrap();
        let blob = Tensor::from_vec(vec![2], DType::FieldElement, vec![10, 20]).unwrap();
        let uri = crate::hash::hex(&blob.commitment_root());
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "add".to_owned(),
                args: vec![
                    input_ref("x"),
                    IrRef::ConstBlob {
                        uri: uri.clone(),
                        shape: vec![2],
                        dtype: DType::FieldElement,
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("y", vec![2], DType::FieldElement, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: op_ref(0),
            }],
        };

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input), (uri.clone(), blob)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();
        assert_eq!(
            execution.outputs["y"],
            Tensor::from_vec(vec![2], DType::FieldElement, vec![11, 22]).unwrap()
        );

        assert_eq!(
            graph.execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([(
                    "x".to_owned(),
                    Tensor::from_vec(vec![2], DType::FieldElement, vec![1, 2]).unwrap()
                )]),
                field_params: BTreeMap::new(),
            }),
            Err(TvmError::InvalidReceipt("missing tensor ir const_blob"))
        );

        let wrong_blob = Tensor::from_vec(vec![2], DType::FieldElement, vec![10, 21]).unwrap();
        assert_eq!(
            graph.execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "x".to_owned(),
                        Tensor::from_vec(vec![2], DType::FieldElement, vec![1, 2]).unwrap(),
                    ),
                    (uri, wrong_blob),
                ]),
                field_params: BTreeMap::new(),
            }),
            Err(TvmError::InvalidReceipt("tensor ir const_blob mismatch"))
        );
    }

    #[test]
    fn exact_interpreter_executes_fixed32_matmul_with_mixed_scales() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::Fixed32, 0),
                tensor_spec("rhs", vec![2, 2], DType::Fixed32, 1),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "matmul".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("product", vec![2, 2], DType::Fixed32, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "product".to_owned(),
                value: op_ref(0),
            }],
        };
        graph.validate_for_consensus().unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![2, 2],
                            DType::Fixed32,
                            0,
                            vec![1, 1, 3, p - 3],
                        )
                        .unwrap(),
                    ),
                    (
                        "rhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![2, 2],
                            DType::Fixed32,
                            1,
                            vec![1, 2, 0, 4],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["product"],
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 0, vec![0, 3, 2, p - 3])
                .unwrap()
        );
    }

    #[test]
    fn graph_validation_rejects_unsupported_matmul_dtype() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::Int32, 0),
                tensor_spec("rhs", vec![2, 2], DType::Int32, 0),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "matmul".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("product", vec![2, 2], DType::Int32, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "product".to_owned(),
                value: op_ref(0),
            }],
        };

        assert_eq!(
            graph.validate_for_consensus(),
            Err(TvmError::InvalidReceipt("tensor ir matmul dtype mismatch"))
        );
    }

    #[test]
    fn exact_interpreter_executes_field_div() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::FieldElement, 0),
                tensor_spec("rhs", vec![2], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "div".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("quotient", vec![2, 2], DType::FieldElement, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "quotient".to_owned(),
                value: op_ref(0),
            }],
        };
        let lhs = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![2, 8, 4, 12]).unwrap();
        let rhs = Tensor::from_vec(vec![2], DType::FieldElement, vec![2, 4]).unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("lhs".to_owned(), lhs), ("rhs".to_owned(), rhs)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["quotient"],
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 2, 3]).unwrap()
        );

        let zero_rhs = Tensor::from_vec(vec![2], DType::FieldElement, vec![2, 0]).unwrap();
        assert_eq!(
            graph.execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![2, 8, 4, 12])
                            .unwrap()
                    ),
                    ("rhs".to_owned(), zero_rhs),
                ]),
                field_params: BTreeMap::new(),
            }),
            Err(TvmError::InvalidReceipt("tensor ir division by zero"))
        );
    }

    #[test]
    fn exact_interpreter_executes_fixed32_div_with_scale_rescale() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::Fixed32, 0),
                tensor_spec("rhs", vec![2], DType::Fixed32, 1),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "div".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("quotient", vec![2, 2], DType::Fixed32, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "quotient".to_owned(),
                value: op_ref(0),
            }],
        };
        graph.validate_for_consensus().unwrap();
        let lhs =
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 0, vec![9, 7, p - 9, p - 7])
                .unwrap();
        let rhs = Tensor::from_vec_with_scale(vec![2], DType::Fixed32, 1, vec![4, 4]).unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("lhs".to_owned(), lhs), ("rhs".to_owned(), rhs)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["quotient"],
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 0, vec![4, 4, p - 4, p - 4])
                .unwrap()
        );

        let zero_rhs = Tensor::from_vec_with_scale(vec![2], DType::Fixed32, 1, vec![4, 0]).unwrap();
        assert_eq!(
            graph.execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![2, 2],
                            DType::Fixed32,
                            0,
                            vec![9, 7, p - 9, p - 7],
                        )
                        .unwrap()
                    ),
                    ("rhs".to_owned(), zero_rhs),
                ]),
                field_params: BTreeMap::new(),
            }),
            Err(TvmError::InvalidReceipt("tensor fixed division by zero"))
        );
    }

    #[test]
    fn graph_validation_rejects_unsupported_div_dtype() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 2], DType::Int32, 0),
                tensor_spec("rhs", vec![2, 2], DType::Int32, 0),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "div".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("quotient", vec![2, 2], DType::Int32, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "quotient".to_owned(),
                value: op_ref(0),
            }],
        };

        assert_eq!(
            graph.validate_for_consensus(),
            Err(TvmError::InvalidReceipt("tensor ir div dtype mismatch"))
        );
    }

    #[test]
    fn exact_interpreter_executes_einsum_matrix_contraction() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 3], DType::FieldElement, 0),
                tensor_spec("rhs", vec![3, 2], DType::FieldElement, 0),
                tensor_spec("lhs_t", vec![3, 2], DType::FieldElement, 0),
                tensor_spec("rhs_t", vec![2, 3], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "einsum".to_owned(),
                    args: vec![input_ref("lhs"), input_ref("rhs")],
                    kwargs: BTreeMap::from([(
                        "equation".to_owned(),
                        IrValue::Literal(IrLiteral::String("ik,kj->ij".to_owned())),
                    )]),
                    out: vec![tensor_spec(
                        "contracted",
                        vec![2, 2],
                        DType::FieldElement,
                        0,
                    )],
                },
                OpNode {
                    id: 1,
                    op: "einsum".to_owned(),
                    args: vec![input_ref("lhs"), input_ref("rhs")],
                    kwargs: BTreeMap::from([(
                        "equation".to_owned(),
                        IrValue::Literal(IrLiteral::String("ik,kj->ji".to_owned())),
                    )]),
                    out: vec![tensor_spec("reversed", vec![2, 2], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 2,
                    op: "einsum".to_owned(),
                    args: vec![input_ref("lhs_t"), input_ref("rhs_t")],
                    kwargs: BTreeMap::from([(
                        "equation".to_owned(),
                        IrValue::Literal(IrLiteral::String("ki,jk->ij".to_owned())),
                    )]),
                    out: vec![tensor_spec(
                        "transposed_inputs",
                        vec![2, 2],
                        DType::FieldElement,
                        0,
                    )],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "contracted".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "reversed".to_owned(),
                    value: op_ref(1),
                },
                GraphOutput {
                    name: "transposed_inputs".to_owned(),
                    value: op_ref(2),
                },
            ],
        };
        let lhs =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let rhs =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![7, 8, 9, 10, 11, 12]).unwrap();
        let expected = lhs.matmul(&rhs).unwrap();
        let lhs_t = lhs.transpose().unwrap();
        let rhs_t = rhs.transpose().unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    ("lhs".to_owned(), lhs),
                    ("rhs".to_owned(), rhs),
                    ("lhs_t".to_owned(), lhs_t),
                    ("rhs_t".to_owned(), rhs_t),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(execution.outputs["contracted"], expected);
        assert_eq!(execution.outputs["reversed"], expected.transpose().unwrap());
        assert_eq!(execution.outputs["transposed_inputs"], expected);
        assert_eq!(execution.op_traces.len(), 3);
    }

    #[test]
    fn graph_validation_rejects_unsupported_einsum_equations() {
        let mut graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 3], DType::FieldElement, 0),
                tensor_spec("rhs", vec![3, 2], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "einsum".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::from([(
                    "equation".to_owned(),
                    IrValue::Literal(IrLiteral::String("ik,kj->ij".to_owned())),
                )]),
                out: vec![tensor_spec(
                    "contracted",
                    vec![2, 2],
                    DType::FieldElement,
                    0,
                )],
            }],
            outputs: vec![GraphOutput {
                name: "contracted".to_owned(),
                value: op_ref(0),
            }],
        };
        assert!(graph.validate_for_consensus().is_ok());

        for equation in ["ii,jk->ik", "ik,kj->ik", "ik,kj->ijk", "ik,kj"] {
            graph.ops[0].kwargs.insert(
                "equation".to_owned(),
                IrValue::Literal(IrLiteral::String(equation.to_owned())),
            );
            assert!(
                graph.validate_for_consensus().is_err(),
                "unsupported equation should reject: {equation}"
            );
        }

        graph.ops[0].kwargs.insert(
            "equation".to_owned(),
            IrValue::Literal(IrLiteral::String("ik,kj->ij".to_owned())),
        );
        graph.ops[0].out[0].shape = vec![2, 3];
        assert!(graph.validate_for_consensus().is_err());
    }

    #[test]
    fn exact_interpreter_executes_split_multi_output_structural_op() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2, 4], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "split".to_owned(),
                args: vec![input_ref("x")],
                kwargs: BTreeMap::from([
                    (
                        "sizes".to_owned(),
                        IrValue::Literal(IrLiteral::List(vec![
                            IrLiteral::Uint(1),
                            IrLiteral::Uint(3),
                        ])),
                    ),
                    ("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1))),
                ]),
                out: vec![
                    tensor_spec("left", vec![2, 1], DType::FieldElement, 0),
                    tensor_spec("right", vec![2, 3], DType::FieldElement, 0),
                ],
            }],
            outputs: vec![
                GraphOutput {
                    name: "left".to_owned(),
                    value: IrRef::Op { id: 0, idx: 0 },
                },
                GraphOutput {
                    name: "right".to_owned(),
                    value: IrRef::Op { id: 0, idx: 1 },
                },
            ],
        };
        let input = Tensor::from_vec(
            vec![2, 4],
            DType::FieldElement,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        )
        .unwrap();

        let graph_id = graph.validate_for_consensus().unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        let expected_left = Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![1, 5]).unwrap();
        let expected_right =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![2, 3, 4, 6, 7, 8]).unwrap();
        assert_eq!(execution.graph_id, graph_id);
        assert_eq!(execution.outputs["left"], expected_left);
        assert_eq!(execution.outputs["right"], expected_right);
        assert_eq!(execution.op_traces.len(), 1);
        assert_eq!(execution.op_traces[0].output_roots.len(), 2);
        assert_eq!(
            execution.op_traces[0].output_roots,
            vec![
                expected_left.commitment_root(),
                expected_right.commitment_root()
            ]
        );
    }

    #[test]
    fn graph_validation_rejects_split_size_mismatch() {
        let mut graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2, 4], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "split".to_owned(),
                args: vec![input_ref("x")],
                kwargs: BTreeMap::from([
                    (
                        "sizes".to_owned(),
                        IrValue::Literal(IrLiteral::List(vec![
                            IrLiteral::Uint(1),
                            IrLiteral::Uint(2),
                        ])),
                    ),
                    ("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1))),
                ]),
                out: vec![
                    tensor_spec("left", vec![2, 1], DType::FieldElement, 0),
                    tensor_spec("right", vec![2, 2], DType::FieldElement, 0),
                ],
            }],
            outputs: vec![GraphOutput {
                name: "left".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        assert!(graph.validate_for_consensus().is_err());

        graph.ops[0].kwargs.insert(
            "sizes".to_owned(),
            IrValue::Literal(IrLiteral::List(vec![
                IrLiteral::Uint(1),
                IrLiteral::Uint(3),
            ])),
        );
        graph.ops[0].out.pop();
        assert!(graph.validate_for_consensus().is_err());
    }

    #[test]
    fn exact_interpreter_supports_field_scalar_params() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2, 2], DType::FieldElement, 0)],
            params: vec![ParamSpec {
                name: "scale".to_owned(),
                type_name: "field_scalar".to_owned(),
            }],
            ops: vec![OpNode {
                id: 0,
                op: "scalar_mul".to_owned(),
                args: vec![input_ref("x"), param_ref("scale")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("scaled", vec![2, 2], DType::FieldElement, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "scaled".to_owned(),
                value: op_ref(0),
            }],
        };
        let tensor = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let expected = tensor.scalar_mul(7).unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), tensor)]),
                field_params: BTreeMap::from([("scale".to_owned(), 7)]),
            })
            .unwrap();

        assert_eq!(execution.outputs["scaled"], expected);

        let missing_param = graph.execute_exact(&IrExecutionInputs {
            tensors: BTreeMap::from([(
                "x".to_owned(),
                Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap(),
            )]),
            field_params: BTreeMap::new(),
        });
        assert!(missing_param.is_err());
    }

    #[test]
    fn exact_interpreter_rejects_deferred_ops() {
        let mut softmax = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        softmax.ops[0].op = "softmax".to_owned();
        softmax.ops[0].args = vec![input_ref("a")];
        softmax.ops[0]
            .kwargs
            .insert("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)));
        softmax.ops[0].out[0] = tensor_spec("softmax", vec![2, 3], DType::FieldElement, 0);
        softmax.outputs[0].value = op_ref(0);
        let err = softmax
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "a".to_owned(),
                        Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
                            .unwrap(),
                    ),
                    (
                        "b".to_owned(),
                        Tensor::from_vec(
                            vec![3, 4],
                            DType::FieldElement,
                            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap_err();
        assert_eq!(
            err,
            TvmError::InvalidReceipt("tensor ir op is not consensus admitted")
        );
    }

    #[test]
    fn exact_interpreter_executes_unary_tier_b_ops() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![7], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "identity".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("identity", vec![7], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 1,
                    op: "neg".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("neg", vec![7], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 2,
                    op: "abs".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("abs", vec![7], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 3,
                    op: "sign".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("sign", vec![7], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 4,
                    op: "round".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("round", vec![7], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 5,
                    op: "relu".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("relu", vec![7], DType::FieldElement, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "identity".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "neg".to_owned(),
                    value: op_ref(1),
                },
                GraphOutput {
                    name: "abs".to_owned(),
                    value: op_ref(2),
                },
                GraphOutput {
                    name: "sign".to_owned(),
                    value: op_ref(3),
                },
                GraphOutput {
                    name: "round".to_owned(),
                    value: op_ref(4),
                },
                GraphOutput {
                    name: "relu".to_owned(),
                    value: op_ref(5),
                },
            ],
        };
        let data = vec![0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5];
        let input = Tensor::from_vec(vec![7], DType::FieldElement, data.clone()).unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        let expected =
            |values: Vec<Elem>| Tensor::from_vec(vec![7], DType::FieldElement, values).unwrap();
        assert_eq!(execution.outputs["identity"], expected(data.clone()));
        assert_eq!(
            execution.outputs["neg"],
            expected(vec![0, p - 1, 1, p.div_ceil(2), (p - 1) / 2, p - 5, 5])
        );
        assert_eq!(
            execution.outputs["abs"],
            expected(vec![0, 1, 1, (p - 1) / 2, (p - 1) / 2, 5, 5])
        );
        assert_eq!(
            execution.outputs["sign"],
            expected(vec![0, 1, p - 1, 1, p - 1, 1, p - 1])
        );
        assert_eq!(execution.outputs["round"], expected(data));
        assert_eq!(
            execution.outputs["relu"],
            expected(vec![0, 1, 0, (p - 1) / 2, 0, 5, 0])
        );
        assert_eq!(execution.op_traces.len(), 6);
        for (trace, output_name) in execution
            .op_traces
            .iter()
            .zip(["identity", "neg", "abs", "sign", "round", "relu"])
        {
            assert_eq!(
                trace.output_roots[0],
                execution.outputs[output_name].commitment_root()
            );
        }
        assert_eq!(
            execution.trace_root,
            graph
                .execute_exact(&IrExecutionInputs {
                    tensors: BTreeMap::from([(
                        "x".to_owned(),
                        Tensor::from_vec(
                            vec![7],
                            DType::FieldElement,
                            vec![0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5],
                        )
                        .unwrap()
                    )]),
                    field_params: BTreeMap::new(),
                })
                .unwrap()
                .trace_root
        );
    }

    #[test]
    fn exact_interpreter_enforces_scale_and_executes_fixed_rounding() {
        let p = field::MODULUS;
        let mut cast_kwargs = BTreeMap::new();
        cast_kwargs.insert(
            "dtype".to_owned(),
            IrValue::Literal(IrLiteral::String("fixed32".to_owned())),
        );
        cast_kwargs.insert("scale".to_owned(), IrValue::Literal(IrLiteral::Int(0)));
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![8], DType::Fixed32, 1)],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "cast".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: cast_kwargs,
                    out: vec![tensor_spec("cast", vec![8], DType::Fixed32, 0)],
                },
                OpNode {
                    id: 1,
                    op: "round".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("round", vec![8], DType::Fixed32, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "cast".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "round".to_owned(),
                    value: op_ref(1),
                },
            ],
        };
        graph.validate_for_consensus().unwrap();
        let data = vec![1, 3, p - 1, p - 3, 5, 7, p - 5, p - 7];
        let input = Tensor::from_vec_with_scale(vec![8], DType::Fixed32, 1, data.clone()).unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();
        let expected = Tensor::from_vec_with_scale(
            vec![8],
            DType::Fixed32,
            0,
            vec![0, 2, 0, p - 2, 2, 4, p - 2, p - 4],
        )
        .unwrap();
        assert_eq!(execution.outputs["cast"], expected);
        assert_eq!(execution.outputs["round"], expected);
        assert_eq!(execution.outputs["cast"].scale(), 0);
        assert_eq!(execution.outputs["round"].scale(), 0);

        let mismatched_input = Tensor::from_vec(vec![8], DType::Fixed32, data).unwrap();
        assert_eq!(
            graph.execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), mismatched_input)]),
                field_params: BTreeMap::new(),
            }),
            Err(TvmError::InvalidReceipt(
                "tensor ir execution input mismatch"
            ))
        );
    }

    #[test]
    fn exact_interpreter_executes_fixed32_mul_with_scale_rescale() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 1], DType::Fixed32, 2),
                tensor_spec("rhs", vec![1, 3], DType::Fixed32, 2),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "mul".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("product", vec![2, 3], DType::Fixed32, 2)],
            }],
            outputs: vec![GraphOutput {
                name: "product".to_owned(),
                value: op_ref(0),
            }],
        };
        graph.validate_for_consensus().unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec_with_scale(vec![2, 1], DType::Fixed32, 2, vec![6, p - 7])
                            .unwrap(),
                    ),
                    (
                        "rhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![1, 3],
                            DType::Fixed32,
                            2,
                            vec![6, 7, p - 6],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["product"],
            Tensor::from_vec_with_scale(
                vec![2, 3],
                DType::Fixed32,
                2,
                vec![9, 10, p - 9, p - 10, p - 12, 10],
            )
            .unwrap()
        );
    }

    #[test]
    fn exact_interpreter_executes_fixed32_mul_with_mixed_scales() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![2, 1], DType::Fixed32, 0),
                tensor_spec("rhs", vec![1, 3], DType::Fixed32, 1),
            ],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "mul".to_owned(),
                args: vec![input_ref("lhs"), input_ref("rhs")],
                kwargs: BTreeMap::new(),
                out: vec![tensor_spec("product", vec![2, 3], DType::Fixed32, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "product".to_owned(),
                value: op_ref(0),
            }],
        };
        graph.validate_for_consensus().unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec_with_scale(vec![2, 1], DType::Fixed32, 0, vec![3, p - 3])
                            .unwrap(),
                    ),
                    (
                        "rhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![1, 3],
                            DType::Fixed32,
                            1,
                            vec![3, 2, p - 3],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["product"],
            Tensor::from_vec_with_scale(
                vec![2, 3],
                DType::Fixed32,
                0,
                vec![4, 3, p - 4, p - 4, p - 3, 4]
            )
            .unwrap()
        );
    }

    #[test]
    fn exact_interpreter_executes_fixed32_add_sub_with_mixed_scales() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("lhs", vec![5], DType::Fixed32, 2),
                tensor_spec("rhs", vec![5], DType::Fixed32, 0),
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "add".to_owned(),
                    args: vec![input_ref("lhs"), input_ref("rhs")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("sum", vec![5], DType::Fixed32, 2)],
                },
                OpNode {
                    id: 1,
                    op: "sub".to_owned(),
                    args: vec![input_ref("lhs"), input_ref("rhs")],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("diff", vec![5], DType::Fixed32, 2)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "sum".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "diff".to_owned(),
                    value: op_ref(1),
                },
            ],
        };
        graph.validate_for_consensus().unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "lhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![5],
                            DType::Fixed32,
                            2,
                            vec![6, p - 7, 3, p - 3, 5],
                        )
                        .unwrap(),
                    ),
                    (
                        "rhs".to_owned(),
                        Tensor::from_vec_with_scale(
                            vec![5],
                            DType::Fixed32,
                            0,
                            vec![2, p - 2, 1, p - 1, 0],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["sum"],
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![14, p - 15, 7, p - 7, 5])
                .unwrap()
        );
        assert_eq!(
            execution.outputs["diff"],
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![p - 2, 1, p - 1, 1, 5])
                .unwrap()
        );
    }

    #[test]
    fn exact_interpreter_executes_per_channel_int8_quantize_dequantize() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2, 3], DType::Fixed32, 0)],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "quantize_int8_per_channel".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(1)),
                    )]),
                    out: vec![
                        tensor_spec("q", vec![2, 3], DType::Int8, 0),
                        tensor_spec("scale", vec![3], DType::Fixed32, 0),
                    ],
                },
                OpNode {
                    id: 1,
                    op: "dequantize_int8_per_channel".to_owned(),
                    args: vec![IrRef::Op { id: 0, idx: 0 }, IrRef::Op { id: 0, idx: 1 }],
                    kwargs: BTreeMap::new(),
                    out: vec![tensor_spec("dq", vec![2, 3], DType::Fixed32, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "q".to_owned(),
                    value: IrRef::Op { id: 0, idx: 0 },
                },
                GraphOutput {
                    name: "scale".to_owned(),
                    value: IrRef::Op { id: 0, idx: 1 },
                },
                GraphOutput {
                    name: "dq".to_owned(),
                    value: op_ref(1),
                },
            ],
        };
        graph.validate_for_consensus().unwrap();

        let input = Tensor::from_vec_with_scale(
            vec![2, 3],
            DType::Fixed32,
            0,
            vec![0, 64, 128, p - 64, p - 128, 127],
        )
        .unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["q"],
            Tensor::from_vec(vec![2, 3], DType::Int8, vec![0, 32, 64, p - 64, p - 64, 64]).unwrap()
        );
        assert_eq!(
            execution.outputs["scale"],
            Tensor::from_vec(vec![3], DType::Fixed32, vec![1, 2, 2]).unwrap()
        );
        assert_eq!(
            execution.outputs["dq"],
            Tensor::from_vec(
                vec![2, 3],
                DType::Fixed32,
                vec![0, 64, 128, p - 64, p - 128, 128]
            )
            .unwrap()
        );
        assert_eq!(execution.op_traces[0].output_roots.len(), 2);
        assert_eq!(execution.op_traces[1].output_roots.len(), 1);

        let mut ambiguous = graph.clone();
        ambiguous.inputs[0].shape = vec![2, 2];
        ambiguous.ops[0].out[0].shape = vec![2, 2];
        ambiguous.ops[0].out[1].shape = vec![2];
        ambiguous.ops[1].out[0].shape = vec![2, 2];
        assert_eq!(
            ambiguous.validate_for_consensus(),
            Err(TvmError::InvalidReceipt(
                "tensor ir dequantize scale ambiguous"
            ))
        );
    }

    #[test]
    fn exact_interpreter_executes_packed_int8_quantize_dequantize() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![2, 3], DType::Fixed32, 0)],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "quantize_pack_int8".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(1)),
                    )]),
                    out: vec![tensor_spec("packed", vec![62], DType::Uint8, 0)],
                },
                OpNode {
                    id: 1,
                    op: "unpack_dequantize_int8".to_owned(),
                    args: vec![op_ref(0)],
                    kwargs: BTreeMap::from([
                        ("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1))),
                        (
                            "shape".to_owned(),
                            IrValue::Literal(IrLiteral::List(vec![
                                IrLiteral::Uint(2),
                                IrLiteral::Uint(3),
                            ])),
                        ),
                        ("scale_dim".to_owned(), IrValue::Literal(IrLiteral::Int(0))),
                    ]),
                    out: vec![tensor_spec("dq", vec![2, 3], DType::Fixed32, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "packed".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "dq".to_owned(),
                    value: op_ref(1),
                },
            ],
        };
        graph.validate_for_consensus().unwrap();

        let input = Tensor::from_vec_with_scale(
            vec![2, 3],
            DType::Fixed32,
            0,
            vec![0, 64, 128, p - 64, p - 128, 127],
        )
        .unwrap();
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([("x".to_owned(), input)]),
                field_params: BTreeMap::new(),
            })
            .unwrap();
        let expected_bytes = vec![
            84, 86, 81, 56, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
            32, 64, 192, 192, 64,
        ];
        assert_eq!(
            execution.outputs["packed"],
            Tensor::from_vec(
                vec![62],
                DType::Uint8,
                expected_bytes.iter().map(|value| *value as Elem).collect(),
            )
            .unwrap()
        );
        assert_eq!(
            execution.outputs["dq"],
            Tensor::from_vec(
                vec![2, 3],
                DType::Fixed32,
                vec![0, 64, 128, p - 64, p - 128, 128]
            )
            .unwrap()
        );

        let mut wrong_shape = graph.clone();
        wrong_shape.ops[1].kwargs.insert(
            "shape".to_owned(),
            IrValue::Literal(IrLiteral::List(vec![
                IrLiteral::Uint(3),
                IrLiteral::Uint(2),
            ])),
        );
        assert!(wrong_shape.validate_for_consensus().is_err());

        let mut bad_payload = execution.outputs["packed"].clone();
        bad_payload.as_mut_slice()[0] = 0;
        let mut unpack_only = graph.clone();
        unpack_only.inputs[0] = tensor_spec("x", vec![62], DType::Uint8, 0);
        unpack_only.ops.remove(0);
        unpack_only.ops[0].id = 0;
        unpack_only.ops[0].args = vec![input_ref("x")];
        unpack_only.outputs = vec![GraphOutput {
            name: "dq".to_owned(),
            value: op_ref(0),
        }];
        assert_eq!(
            unpack_only
                .execute_exact(&IrExecutionInputs {
                    tensors: BTreeMap::from([("x".to_owned(), bad_payload)]),
                    field_params: BTreeMap::new(),
                })
                .unwrap_err(),
            TvmError::InvalidReceipt("packed int8 header mismatch")
        );
    }

    #[test]
    fn exact_interpreter_executes_mean_cast_concat_and_stack() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("a", vec![2, 3], DType::FieldElement, 0),
                tensor_spec("b", vec![2, 3], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "mean".to_owned(),
                    args: vec![input_ref("a")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(1)),
                    )]),
                    out: vec![tensor_spec("row_mean", vec![2], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 1,
                    op: "cast".to_owned(),
                    args: vec![op_ref(0)],
                    kwargs: BTreeMap::from([(
                        "dtype".to_owned(),
                        IrValue::Literal(IrLiteral::String("int64".to_owned())),
                    )]),
                    out: vec![tensor_spec("row_mean_i64", vec![2], DType::Int64, 0)],
                },
                OpNode {
                    id: 2,
                    op: "concat".to_owned(),
                    args: vec![input_ref("a"), input_ref("b")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(0)),
                    )]),
                    out: vec![tensor_spec(
                        "rows_concat",
                        vec![4, 3],
                        DType::FieldElement,
                        0,
                    )],
                },
                OpNode {
                    id: 3,
                    op: "stack".to_owned(),
                    args: vec![input_ref("a"), input_ref("b")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(1)),
                    )]),
                    out: vec![tensor_spec(
                        "paired_rows",
                        vec![2, 2, 3],
                        DType::FieldElement,
                        0,
                    )],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "row_mean_i64".to_owned(),
                    value: op_ref(1),
                },
                GraphOutput {
                    name: "rows_concat".to_owned(),
                    value: op_ref(2),
                },
                GraphOutput {
                    name: "paired_rows".to_owned(),
                    value: op_ref(3),
                },
            ],
        };

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "a".to_owned(),
                        Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
                            .unwrap(),
                    ),
                    (
                        "b".to_owned(),
                        Tensor::from_vec(
                            vec![2, 3],
                            DType::FieldElement,
                            vec![7, 8, 9, 10, 11, 12],
                        )
                        .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["row_mean_i64"],
            Tensor::from_vec(vec![2], DType::Int64, vec![2, 5]).unwrap()
        );
        assert_eq!(
            execution.outputs["rows_concat"],
            Tensor::from_vec(
                vec![4, 3],
                DType::FieldElement,
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
            )
            .unwrap()
        );
        assert_eq!(
            execution.outputs["paired_rows"],
            Tensor::from_vec(
                vec![2, 2, 3],
                DType::FieldElement,
                vec![1, 2, 3, 7, 8, 9, 4, 5, 6, 10, 11, 12]
            )
            .unwrap()
        );
        assert_eq!(execution.op_traces.len(), 4);
    }

    #[test]
    fn exact_interpreter_executes_shaping_comparison_generators_and_where() {
        let mut graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                tensor_spec("x", vec![2, 1], DType::FieldElement, 0),
                tensor_spec("y", vec![1, 3], DType::FieldElement, 0),
            ],
            params: Vec::new(),
            ops: Vec::new(),
            outputs: vec![
                GraphOutput {
                    name: "sum".to_owned(),
                    value: op_ref(0),
                },
                GraphOutput {
                    name: "selected".to_owned(),
                    value: op_ref(7),
                },
            ],
        };
        graph.ops.push(OpNode {
            id: 0,
            op: "add".to_owned(),
            args: vec![input_ref("x"), input_ref("y")],
            kwargs: BTreeMap::new(),
            out: vec![tensor_spec("sum", vec![2, 3], DType::FieldElement, 0)],
        });
        graph.ops.push(OpNode {
            id: 1,
            op: "broadcast".to_owned(),
            args: vec![input_ref("x")],
            kwargs: BTreeMap::from([(
                "shape".to_owned(),
                IrValue::Literal(IrLiteral::List(vec![
                    IrLiteral::Uint(2),
                    IrLiteral::Uint(3),
                ])),
            )]),
            out: vec![tensor_spec("wide", vec![2, 3], DType::FieldElement, 0)],
        });
        graph.ops.push(OpNode {
            id: 2,
            op: "reshape".to_owned(),
            args: vec![op_ref(1)],
            kwargs: BTreeMap::from([(
                "shape".to_owned(),
                IrValue::Literal(IrLiteral::List(vec![
                    IrLiteral::Uint(3),
                    IrLiteral::Uint(2),
                ])),
            )]),
            out: vec![tensor_spec("reshaped", vec![3, 2], DType::FieldElement, 0)],
        });
        graph.ops.push(OpNode {
            id: 3,
            op: "full".to_owned(),
            args: Vec::new(),
            kwargs: BTreeMap::from([
                (
                    "shape".to_owned(),
                    IrValue::Literal(IrLiteral::List(vec![
                        IrLiteral::Uint(3),
                        IrLiteral::Uint(2),
                    ])),
                ),
                ("value".to_owned(), IrValue::Literal(IrLiteral::Field(3))),
                (
                    "dtype".to_owned(),
                    IrValue::Literal(IrLiteral::String("field".to_owned())),
                ),
            ]),
            out: vec![tensor_spec("threshold", vec![3, 2], DType::FieldElement, 0)],
        });
        graph.ops.push(OpNode {
            id: 4,
            op: "gt".to_owned(),
            args: vec![op_ref(2), op_ref(3)],
            kwargs: BTreeMap::new(),
            out: vec![tensor_spec("mask", vec![3, 2], DType::Int32, 0)],
        });
        graph.ops.push(OpNode {
            id: 5,
            op: "arange".to_owned(),
            args: Vec::new(),
            kwargs: BTreeMap::from([
                ("start".to_owned(), IrValue::Literal(IrLiteral::Int(10))),
                ("end".to_owned(), IrValue::Literal(IrLiteral::Int(16))),
                ("step".to_owned(), IrValue::Literal(IrLiteral::Int(1))),
                (
                    "dtype".to_owned(),
                    IrValue::Literal(IrLiteral::String("field".to_owned())),
                ),
            ]),
            out: vec![tensor_spec("range", vec![6], DType::FieldElement, 0)],
        });
        graph.ops.push(OpNode {
            id: 6,
            op: "reshape".to_owned(),
            args: vec![op_ref(5)],
            kwargs: BTreeMap::from([(
                "shape".to_owned(),
                IrValue::Literal(IrLiteral::List(vec![
                    IrLiteral::Uint(3),
                    IrLiteral::Uint(2),
                ])),
            )]),
            out: vec![tensor_spec(
                "range_matrix",
                vec![3, 2],
                DType::FieldElement,
                0,
            )],
        });
        graph.ops.push(OpNode {
            id: 7,
            op: "where".to_owned(),
            args: vec![op_ref(4), op_ref(6), op_ref(2)],
            kwargs: BTreeMap::new(),
            out: vec![tensor_spec("selected", vec![3, 2], DType::FieldElement, 0)],
        });

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([
                    (
                        "x".to_owned(),
                        Tensor::from_vec(vec![2, 1], DType::FieldElement, vec![1, 4]).unwrap(),
                    ),
                    (
                        "y".to_owned(),
                        Tensor::from_vec(vec![1, 3], DType::FieldElement, vec![10, 20, 30])
                            .unwrap(),
                    ),
                ]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["sum"],
            Tensor::from_vec(
                vec![2, 3],
                DType::FieldElement,
                vec![11, 21, 31, 14, 24, 34]
            )
            .unwrap()
        );
        assert_eq!(
            execution.outputs["selected"],
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 1, 1, 13, 14, 15]).unwrap()
        );
        assert_eq!(execution.op_traces.len(), 8);
    }

    #[test]
    fn exact_interpreter_executes_clamp() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![6], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "clamp".to_owned(),
                args: vec![input_ref("x")],
                kwargs: BTreeMap::from([
                    ("min".to_owned(), IrValue::Literal(IrLiteral::Field(2))),
                    ("max".to_owned(), IrValue::Literal(IrLiteral::Field(5))),
                ]),
                out: vec![tensor_spec("clamped", vec![6], DType::FieldElement, 0)],
            }],
            outputs: vec![GraphOutput {
                name: "clamped".to_owned(),
                value: op_ref(0),
            }],
        };
        graph.validate_for_consensus().unwrap();

        let p = field::MODULUS;
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([(
                    "x".to_owned(),
                    Tensor::from_vec(vec![6], DType::FieldElement, vec![0, 2, 4, 5, 7, p - 1])
                        .unwrap(),
                )]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["clamped"],
            Tensor::from_vec(vec![6], DType::FieldElement, vec![2, 2, 4, 5, 5, 5]).unwrap()
        );
        assert_eq!(execution.op_traces.len(), 1);

        let mut bad_bounds = graph;
        bad_bounds.ops[0].kwargs = BTreeMap::from([
            ("min".to_owned(), IrValue::Literal(IrLiteral::Field(5))),
            ("max".to_owned(), IrValue::Literal(IrLiteral::Field(2))),
        ]);
        assert_eq!(
            bad_bounds.validate_for_consensus(),
            Err(TvmError::InvalidReceipt("tensor ir clamp bounds mismatch"))
        );
    }

    #[test]
    fn exact_interpreter_executes_single_output_structural_ops() {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![tensor_spec("x", vec![3, 3], DType::FieldElement, 0)],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "unsqueeze".to_owned(),
                    args: vec![input_ref("x")],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(0)),
                    )]),
                    out: vec![tensor_spec(
                        "expanded",
                        vec![1, 3, 3],
                        DType::FieldElement,
                        0,
                    )],
                },
                OpNode {
                    id: 1,
                    op: "squeeze".to_owned(),
                    args: vec![op_ref(0)],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        IrValue::Literal(IrLiteral::Uint(0)),
                    )]),
                    out: vec![tensor_spec("restored", vec![3, 3], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 2,
                    op: "slice".to_owned(),
                    args: vec![op_ref(1)],
                    kwargs: BTreeMap::from([
                        ("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(0))),
                        ("start".to_owned(), IrValue::Literal(IrLiteral::Uint(0))),
                        ("end".to_owned(), IrValue::Literal(IrLiteral::Uint(2))),
                    ]),
                    out: vec![tensor_spec("top_rows", vec![2, 3], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 3,
                    op: "triu".to_owned(),
                    args: vec![op_ref(1)],
                    kwargs: BTreeMap::from([(
                        "diagonal".to_owned(),
                        IrValue::Literal(IrLiteral::Int(0)),
                    )]),
                    out: vec![tensor_spec("upper", vec![3, 3], DType::FieldElement, 0)],
                },
                OpNode {
                    id: 4,
                    op: "tril".to_owned(),
                    args: vec![op_ref(3)],
                    kwargs: BTreeMap::from([(
                        "diagonal".to_owned(),
                        IrValue::Literal(IrLiteral::Int(0)),
                    )]),
                    out: vec![tensor_spec("diagonal", vec![3, 3], DType::FieldElement, 0)],
                },
            ],
            outputs: vec![
                GraphOutput {
                    name: "top_rows".to_owned(),
                    value: op_ref(2),
                },
                GraphOutput {
                    name: "diagonal".to_owned(),
                    value: op_ref(4),
                },
            ],
        };
        graph.validate_for_consensus().unwrap();

        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: BTreeMap::from([(
                    "x".to_owned(),
                    Tensor::from_vec(
                        vec![3, 3],
                        DType::FieldElement,
                        vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
                    )
                    .unwrap(),
                )]),
                field_params: BTreeMap::new(),
            })
            .unwrap();

        assert_eq!(
            execution.outputs["top_rows"],
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap()
        );
        assert_eq!(
            execution.outputs["diagonal"],
            Tensor::from_vec(
                vec![3, 3],
                DType::FieldElement,
                vec![1, 0, 0, 0, 5, 0, 0, 0, 9]
            )
            .unwrap()
        );
        assert_eq!(execution.op_traces.len(), 5);

        let mut bad_squeeze = graph;
        bad_squeeze.ops[1].kwargs =
            BTreeMap::from([("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)))]);
        assert_eq!(
            bad_squeeze.validate_for_consensus(),
            Err(TvmError::InvalidReceipt("tensor ir squeeze dim mismatch"))
        );
    }

    #[test]
    fn graph_validation_rejects_inconsistent_exact_tier_b_shapes() {
        let mut reshape = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        reshape.ops[0].op = "reshape".to_owned();
        reshape.ops[0].args = vec![input_ref("a")];
        reshape.ops[0].kwargs.insert(
            "shape".to_owned(),
            IrValue::Literal(IrLiteral::List(vec![IrLiteral::Uint(5)])),
        );
        reshape.ops[0].out[0] = tensor_spec("bad", vec![5], DType::FieldElement, 0);
        assert_eq!(
            reshape.validate_for_consensus(),
            Err(TvmError::InvalidReceipt(
                "tensor ir reshape element mismatch"
            ))
        );

        let mut arange = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        arange.ops[0].op = "arange".to_owned();
        arange.ops[0].args = Vec::new();
        arange.ops[0].kwargs = BTreeMap::from([
            ("start".to_owned(), IrValue::Literal(IrLiteral::Int(0))),
            ("end".to_owned(), IrValue::Literal(IrLiteral::Int(5))),
            ("step".to_owned(), IrValue::Literal(IrLiteral::Int(2))),
            (
                "dtype".to_owned(),
                IrValue::Literal(IrLiteral::String("field".to_owned())),
            ),
        ]);
        arange.ops[0].out[0] = tensor_spec("bad_range", vec![5], DType::FieldElement, 0);
        assert!(arange.validate_for_consensus().is_err());

        let mut concat = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        concat.ops[0].op = "concat".to_owned();
        concat.ops[0].args = vec![input_ref("a"), input_ref("a")];
        concat.ops[0].kwargs =
            BTreeMap::from([("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(0)))]);
        concat.ops[0].out[0] = tensor_spec("bad_concat", vec![2, 3], DType::FieldElement, 0);
        assert!(concat.validate_for_consensus().is_err());

        let mut stack = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
        stack.ops[0].op = "stack".to_owned();
        stack.ops[0].args = vec![input_ref("a"), input_ref("a")];
        stack.ops[0].kwargs =
            BTreeMap::from([("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(3)))]);
        stack.ops[0].out[0] = tensor_spec("bad_stack", vec![2, 2, 3], DType::FieldElement, 0);
        assert!(stack.validate_for_consensus().is_err());
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
