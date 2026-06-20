use std::collections::BTreeSet;

use crate::error::{Result, TvmError};
use crate::field::{self, Elem, MODULUS};
use crate::ir::{TensorGraph, canonical_linear_training_step_graph, canonical_matmul_graph};
use crate::jobs::{LinearTrainingStepJob, MatmulJob};
use crate::tensor::{DType, Tensor, rescale_signed_elem_half_even};
use crate::types::{Hash, hash_bytes};
use crate::vm;

const SUITE_VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceVector {
    pub id: &'static str,
    pub op_name: &'static str,
    pub tier: &'static str,
    pub dtype: DType,
    pub input_dtypes: Vec<DType>,
    pub input_scales: Vec<i64>,
    pub input_shapes: Vec<Vec<usize>>,
    pub params: Vec<(&'static str, u64)>,
    pub input_data: Vec<Vec<Elem>>,
    pub expected_dtype: DType,
    pub expected_scale: i64,
    pub expected_data: Vec<Elem>,
    pub expected_shape: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceProfile {
    pub suite_hash: Hash,
    pub passed_ops: BTreeSet<&'static str>,
}

impl ConformanceProfile {
    pub fn empty_for_testing() -> Self {
        Self {
            suite_hash: [0; 32],
            passed_ops: BTreeSet::new(),
        }
    }

    pub fn passes(&self, op_name: &str) -> bool {
        self.suite_hash == conformance_suite_hash()
            && self.passed_ops.iter().any(|passed| *passed == op_name)
    }
}

pub fn conformance_vectors() -> Vec<ConformanceVector> {
    let p = MODULUS;
    vec![
        vector(
            "field-add-wraparound-v1",
            "add",
            "B",
            &[&[2, 3], &[2, 3]],
            &[],
            &[&[0, 1, p - 1, p - 2, 7, 11], &[0, p - 1, 2, 5, p - 7, 13]],
            &[0, 0, 1, 3, 0, 24],
            &[2, 3],
        ),
        vector(
            "field-sub-wraparound-v1",
            "sub",
            "B",
            &[&[2, 3], &[2, 3]],
            &[],
            &[&[0, 1, p - 1, p - 2, 7, 11], &[0, p - 1, 2, 5, p - 7, 13]],
            &[0, 2, p - 3, p - 7, 14, p - 2],
            &[2, 3],
        ),
        vector(
            "field-mul-wraparound-v1",
            "mul",
            "B",
            &[&[2, 3], &[2, 3]],
            &[],
            &[&[0, 1, p - 1, p - 2, 7, 11], &[3, p - 1, 2, p - 3, 6, 13]],
            &[0, p - 1, p - 2, 6, 42, 143],
            &[2, 3],
        ),
        vector(
            "field-scalar-mul-wraparound-v1",
            "scalar_mul",
            "B",
            &[&[2, 3]],
            &[("scalar", p + 2)],
            &[&[0, 1, p - 1, p - 2, 7, 11]],
            &[0, 2, p - 2, p - 4, 14, 22],
            &[2, 3],
        ),
        vector(
            "field-identity-unary-v1",
            "identity",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5],
            &[7],
        ),
        vector(
            "field-neg-unary-v1",
            "neg",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, p - 1, 1, p.div_ceil(2), (p - 1) / 2, p - 5, 5],
            &[7],
        ),
        vector(
            "field-abs-signed-unary-v1",
            "abs",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, 1, 1, (p - 1) / 2, (p - 1) / 2, 5, 5],
            &[7],
        ),
        vector(
            "field-sign-signed-unary-v1",
            "sign",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, 1, p - 1, 1, p - 1, 1, p - 1],
            &[7],
        ),
        vector(
            "field-round-identity-unary-v1",
            "round",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5],
            &[7],
        ),
        vector(
            "field-relu-signed-unary-v1",
            "relu",
            "B",
            &[&[7]],
            &[],
            &[&[0, 1, p - 1, (p - 1) / 2, p.div_ceil(2), 5, p - 5]],
            &[0, 1, 0, (p - 1) / 2, 0, 5, 0],
            &[7],
        ),
        scaled_vector(
            "fixed32-round-half-even-scale1-to-scale0-v1",
            "round",
            "B",
            &[&[8]],
            &[DType::Fixed32],
            &[1],
            &[],
            &[&[1, 3, p - 1, p - 3, 5, 7, p - 5, p - 7]],
            DType::Fixed32,
            0,
            &[0, 2, 0, p - 2, 2, 4, p - 2, p - 4],
            &[8],
        ),
        scaled_vector(
            "fixed32-cast-half-even-scale1-to-scale0-v1",
            "cast",
            "B",
            &[&[8]],
            &[DType::Fixed32],
            &[1],
            &[("scale", 0)],
            &[&[1, 3, p - 1, p - 3, 5, 7, p - 5, p - 7]],
            DType::Fixed32,
            0,
            &[0, 2, 0, p - 2, 2, 4, p - 2, p - 4],
            &[8],
        ),
        scaled_vector(
            "fixed32-cast-scale0-to-scale2-v1",
            "cast",
            "B",
            &[&[4]],
            &[DType::Fixed32],
            &[0],
            &[("scale", 2)],
            &[&[0, 2, p - 2, 5]],
            DType::Fixed32,
            2,
            &[0, 8, p - 8, 20],
            &[4],
        ),
        vector(
            "field-transpose-row-major-v1",
            "transpose",
            "B",
            &[&[2, 3]],
            &[],
            &[&[1, 2, 3, 4, 5, 6]],
            &[1, 4, 2, 5, 3, 6],
            &[3, 2],
        ),
        vector(
            "field-reshape-row-major-v1",
            "reshape",
            "B",
            &[&[2, 3]],
            &[("rows", 3), ("cols", 2)],
            &[&[1, 2, 3, 4, 5, 6]],
            &[1, 2, 3, 4, 5, 6],
            &[3, 2],
        ),
        vector(
            "field-broadcast-row-major-v1",
            "broadcast",
            "B",
            &[&[2, 1]],
            &[("rows", 2), ("cols", 3)],
            &[&[7, 9]],
            &[7, 7, 7, 9, 9, 9],
            &[2, 3],
        ),
        vector(
            "field-reduce-sum-axis0-v1",
            "reduce_sum",
            "B",
            &[&[2, 3]],
            &[("axis", 0)],
            &[&[p - 1, 2, 3, 4, p - 2, 6]],
            &[3, 0, 9],
            &[3],
        ),
        vector(
            "field-reduce-sum-axis1-v1",
            "reduce_sum",
            "B",
            &[&[2, 3]],
            &[("axis", 1)],
            &[&[p - 1, 2, 3, 4, p - 2, 6]],
            &[4, 8],
            &[2],
        ),
        vector(
            "field-mean-axis1-v1",
            "mean",
            "B",
            &[&[2, 3]],
            &[("axis", 1)],
            &[&[1, 2, 3, 4, 5, 6]],
            &[2, 5],
            &[2],
        ),
        vector(
            "field-concat-axis0-v1",
            "concat",
            "B",
            &[&[2, 2], &[2, 2]],
            &[("axis", 0)],
            &[&[1, 2, 3, 4], &[5, 6, 7, 8]],
            &[1, 2, 3, 4, 5, 6, 7, 8],
            &[4, 2],
        ),
        vector(
            "field-stack-axis1-v1",
            "stack",
            "B",
            &[&[2], &[2]],
            &[("axis", 1)],
            &[&[1, 2], &[3, 4]],
            &[1, 3, 2, 4],
            &[2, 2],
        ),
        vector(
            "field-matmul-wraparound-v1",
            "matmul",
            "A",
            &[&[2, 3], &[3, 2]],
            &[],
            &[&[p - 1, 2, 3, 4, p - 2, 6], &[7, 8, p - 3, 10, 11, p - 4]],
            &[20, 0, 100, p - 12],
            &[2, 2],
        ),
        vector(
            "field-mse-loss-wraparound-v1",
            "mse_loss",
            "B",
            &[&[2, 2], &[2, 2]],
            &[],
            &[&[0, 1, p - 1, 9], &[p - 1, 3, 1, p - 4]],
            &mse_loss_expected(&[0, 1, p - 1, 9], &[p - 1, 3, 1, p - 4]),
            &[32],
        ),
        vector(
            "field-full-v1",
            "full",
            "B",
            &[],
            &[("rows", 2), ("cols", 3), ("value", p + 5)],
            &[],
            &[5, 5, 5, 5, 5, 5],
            &[2, 3],
        ),
        vector(
            "field-arange-v1",
            "arange",
            "B",
            &[],
            &[("start", 3), ("end", 10), ("step", 2)],
            &[],
            &[3, 5, 7, 9],
            &[4],
        ),
    ]
}

pub fn conformance_suite_hash() -> Hash {
    let vectors = conformance_vectors();
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&SUITE_VERSION.to_le_bytes());
    encoded.extend_from_slice(&(vectors.len() as u64).to_le_bytes());
    for vector in &vectors {
        encode_vector(vector, &mut encoded);
    }
    hash_bytes(b"tensor-vm-fp-conformance-suite-v1", &[&encoded])
}

pub fn cpu_reference_conformance_profile() -> Result<ConformanceProfile> {
    let vectors = conformance_vectors();
    let mut passed_ops = BTreeSet::new();
    for vector in &vectors {
        if execute_vector(vector)? != expected_tensor(vector)? {
            return Err(TvmError::VerificationFailed("conformance vector mismatch"));
        }
        passed_ops.insert(vector.op_name);
    }
    Ok(ConformanceProfile {
        suite_hash: conformance_suite_hash(),
        passed_ops,
    })
}

pub fn ensure_tensor_op_job_conformance(
    job: &MatmulJob,
    profile: &ConformanceProfile,
) -> Result<()> {
    ensure_graph_conformance(&job.tensor_ir_graph())?;
    ensure_ops(profile, &["matmul"])
}

pub fn ensure_linear_training_step_conformance(
    job: &LinearTrainingStepJob,
    profile: &ConformanceProfile,
) -> Result<()> {
    ensure_graph_conformance(&job.tensor_ir_graph())?;
    ensure_ops(
        profile,
        &["matmul", "sub", "scalar_mul", "transpose", "mse_loss"],
    )
}

pub fn current_tensor_op_graphs() -> Vec<TensorGraph> {
    vec![canonical_matmul_graph(2, 3, 2, DType::FieldElement)]
}

pub fn current_linear_training_graphs() -> Vec<TensorGraph> {
    vec![canonical_linear_training_step_graph(
        &[2, 3],
        &[3, 2],
        &[2, 2],
        DType::FieldElement,
    )]
}

fn ensure_graph_conformance(graph: &TensorGraph) -> Result<()> {
    graph.validate_for_consensus().map(|_| ())
}

fn ensure_ops(profile: &ConformanceProfile, required_ops: &[&'static str]) -> Result<()> {
    if profile.suite_hash != conformance_suite_hash() {
        return Err(TvmError::InvalidReceipt("conformance suite unavailable"));
    }
    for op in required_ops {
        if !profile.passes(op) {
            return Err(TvmError::InvalidReceipt("required op conformance missing"));
        }
    }
    Ok(())
}

fn execute_vector(vector: &ConformanceVector) -> Result<Tensor> {
    let tensors = vector.input_tensors()?;
    match vector.op_name {
        "add" => tensors[0].add(&tensors[1]),
        "sub" => tensors[0].sub(&tensors[1]),
        "mul" => tensors[0].mul(&tensors[1]),
        "scalar_mul" => tensors[0].scalar_mul(param(vector, "scalar")?),
        "identity" => Ok(tensors[0].clone()),
        "neg" => unary_tensor(&tensors[0], |value| field::sub(0, value)),
        "abs" => unary_tensor(&tensors[0], signed_abs),
        "sign" => unary_tensor(&tensors[0], signed_sign),
        "round" => round_tensor(&tensors[0]),
        "relu" => unary_tensor(&tensors[0], signed_relu),
        "transpose" => tensors[0].transpose(),
        "reshape" => Tensor::from_vec_with_scale(
            vec![
                param(vector, "rows")? as usize,
                param(vector, "cols")? as usize,
            ],
            tensors[0].dtype(),
            tensors[0].scale(),
            tensors[0].as_slice().to_vec(),
        ),
        "cast" => cast_tensor(&tensors[0], vector.expected_dtype, vector.expected_scale),
        "broadcast" => {
            let rows = param(vector, "rows")? as usize;
            let cols = param(vector, "cols")? as usize;
            if tensors[0].shape() != [rows, 1] {
                return Err(TvmError::InvalidReceipt("invalid conformance broadcast"));
            }
            let mut data = Vec::with_capacity(rows * cols);
            for row in 0..rows {
                for _ in 0..cols {
                    data.push(tensors[0].as_slice()[row]);
                }
            }
            Tensor::from_vec_with_scale(
                vec![rows, cols],
                tensors[0].dtype(),
                tensors[0].scale(),
                data,
            )
        }
        "reduce_sum" => tensors[0].reduce_sum(param(vector, "axis")? as usize),
        "mean" => mean_tensor(&tensors[0], param(vector, "axis")? as usize),
        "concat" => concat_tensors(&tensors, param(vector, "axis")? as usize),
        "stack" => stack_tensors(&tensors, param(vector, "axis")? as usize),
        "matmul" => tensors[0].matmul(&tensors[1]),
        "mse_loss" => {
            let loss = vm::mse_loss(&tensors[0], &tensors[1])?;
            Tensor::from_vec(
                vec![32],
                DType::FieldElement,
                loss.iter().map(|byte| *byte as Elem).collect(),
            )
        }
        "full" => Tensor::from_vec_with_scale(
            vec![
                param(vector, "rows")? as usize,
                param(vector, "cols")? as usize,
            ],
            vector.expected_dtype,
            vector.expected_scale,
            vec![
                field::normalize(param(vector, "value")?);
                (param(vector, "rows")? * param(vector, "cols")?) as usize
            ],
        ),
        "arange" => {
            let mut data = Vec::new();
            let mut value = param(vector, "start")?;
            let end = param(vector, "end")?;
            let step = param(vector, "step")?;
            while value < end {
                data.push(field::normalize(value));
                value += step;
            }
            Tensor::from_vec_with_scale(
                vec![data.len()],
                vector.expected_dtype,
                vector.expected_scale,
                data,
            )
        }
        _ => Err(TvmError::InvalidReceipt("unknown conformance op")),
    }
}

fn unary_tensor(tensor: &Tensor, op: impl Fn(Elem) -> Elem) -> Result<Tensor> {
    Tensor::from_vec_with_scale(
        tensor.shape().to_vec(),
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().iter().map(|value| op(*value)).collect(),
    )
}

fn cast_tensor(tensor: &Tensor, dtype: DType, scale: i64) -> Result<Tensor> {
    if dtype != DType::Fixed32 && scale != 0 {
        return Err(TvmError::InvalidReceipt("invalid conformance cast scale"));
    }
    let data = tensor
        .as_slice()
        .iter()
        .map(|value| rescale_signed_elem_half_even(*value, tensor.scale(), scale))
        .collect::<Result<Vec<_>>>()?;
    Tensor::from_vec_with_scale(tensor.shape().to_vec(), dtype, scale, data)
}

fn round_tensor(tensor: &Tensor) -> Result<Tensor> {
    if tensor.dtype() != DType::Fixed32 {
        return Ok(tensor.clone());
    }
    cast_tensor(tensor, tensor.dtype(), 0)
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
        MODULUS - 1
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
    field::normalize(value) > MODULUS / 2
}

fn mean_tensor(tensor: &Tensor, axis: usize) -> Result<Tensor> {
    if tensor.shape().len() != 2 {
        return Err(TvmError::InvalidReceipt("invalid conformance mean"));
    }
    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
    match axis {
        0 => {
            let inv = field_inverse(rows as Elem)?;
            let mut out = vec![0; cols];
            for row in 0..rows {
                for (col, value) in out.iter_mut().enumerate() {
                    *value = field::add(*value, tensor.as_slice()[row * cols + col]);
                }
            }
            for value in &mut out {
                *value = field::mul(*value, inv);
            }
            Tensor::from_vec_with_scale(vec![cols], tensor.dtype(), tensor.scale(), out)
        }
        1 => {
            let inv = field_inverse(cols as Elem)?;
            let mut out = Vec::with_capacity(rows);
            for row in 0..rows {
                let mut acc = 0;
                for col in 0..cols {
                    acc = field::add(acc, tensor.as_slice()[row * cols + col]);
                }
                out.push(field::mul(acc, inv));
            }
            Tensor::from_vec_with_scale(vec![rows], tensor.dtype(), tensor.scale(), out)
        }
        _ => Err(TvmError::InvalidReceipt("invalid conformance mean")),
    }
}

fn concat_tensors(tensors: &[Tensor], axis: usize) -> Result<Tensor> {
    if tensors.len() != 2 || tensors[0].shape().len() != 2 || tensors[1].shape().len() != 2 {
        return Err(TvmError::InvalidReceipt("invalid conformance concat"));
    }
    if tensors[0].dtype() != tensors[1].dtype() || tensors[0].scale() != tensors[1].scale() {
        return Err(TvmError::InvalidReceipt("invalid conformance concat"));
    }
    let [a_rows, a_cols] = [tensors[0].shape()[0], tensors[0].shape()[1]];
    let [b_rows, b_cols] = [tensors[1].shape()[0], tensors[1].shape()[1]];
    match axis {
        0 if a_cols == b_cols => {
            let mut out = tensors[0].as_slice().to_vec();
            out.extend_from_slice(tensors[1].as_slice());
            Tensor::from_vec_with_scale(
                vec![a_rows + b_rows, a_cols],
                tensors[0].dtype(),
                tensors[0].scale(),
                out,
            )
        }
        1 if a_rows == b_rows => {
            let mut out = Vec::with_capacity((a_cols + b_cols) * a_rows);
            for row in 0..a_rows {
                out.extend_from_slice(&tensors[0].as_slice()[row * a_cols..(row + 1) * a_cols]);
                out.extend_from_slice(&tensors[1].as_slice()[row * b_cols..(row + 1) * b_cols]);
            }
            Tensor::from_vec_with_scale(
                vec![a_rows, a_cols + b_cols],
                tensors[0].dtype(),
                tensors[0].scale(),
                out,
            )
        }
        _ => Err(TvmError::InvalidReceipt("invalid conformance concat")),
    }
}

fn stack_tensors(tensors: &[Tensor], axis: usize) -> Result<Tensor> {
    if tensors.len() != 2
        || tensors[0].shape().len() != 1
        || tensors[0].shape() != tensors[1].shape()
    {
        return Err(TvmError::InvalidReceipt("invalid conformance stack"));
    }
    if tensors[0].dtype() != tensors[1].dtype() || tensors[0].scale() != tensors[1].scale() {
        return Err(TvmError::InvalidReceipt("invalid conformance stack"));
    }
    let len = tensors[0].shape()[0];
    match axis {
        0 => {
            let mut out = tensors[0].as_slice().to_vec();
            out.extend_from_slice(tensors[1].as_slice());
            Tensor::from_vec_with_scale(vec![2, len], tensors[0].dtype(), tensors[0].scale(), out)
        }
        1 => {
            let mut out = Vec::with_capacity(len * 2);
            for index in 0..len {
                out.push(tensors[0].as_slice()[index]);
                out.push(tensors[1].as_slice()[index]);
            }
            Tensor::from_vec_with_scale(vec![len, 2], tensors[0].dtype(), tensors[0].scale(), out)
        }
        _ => Err(TvmError::InvalidReceipt("invalid conformance stack")),
    }
}

fn field_inverse(value: Elem) -> Result<Elem> {
    let value = field::normalize(value);
    if value == 0 {
        return Err(TvmError::InvalidReceipt("invalid conformance mean"));
    }
    Ok(field_pow(value, MODULUS - 2))
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

fn expected_tensor(vector: &ConformanceVector) -> Result<Tensor> {
    Tensor::from_vec_with_scale(
        vector.expected_shape.clone(),
        vector.expected_dtype,
        vector.expected_scale,
        vector.expected_data.clone(),
    )
}

fn param(vector: &ConformanceVector, name: &str) -> Result<u64> {
    vector
        .params
        .iter()
        .find_map(|(key, value)| (*key == name).then_some(*value))
        .ok_or(TvmError::InvalidReceipt("missing conformance param"))
}

impl ConformanceVector {
    fn input_tensors(&self) -> Result<Vec<Tensor>> {
        self.input_shapes
            .iter()
            .cloned()
            .zip(self.input_dtypes.iter().copied())
            .zip(self.input_scales.iter().copied())
            .zip(self.input_data.iter().cloned())
            .map(|(((shape, dtype), scale), data)| {
                Tensor::from_vec_with_scale(shape, dtype, scale, data)
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn vector(
    id: &'static str,
    op_name: &'static str,
    tier: &'static str,
    input_shapes: &[&[usize]],
    params: &[(&'static str, u64)],
    input_data: &[&[Elem]],
    expected_data: &[Elem],
    expected_shape: &[usize],
) -> ConformanceVector {
    ConformanceVector {
        id,
        op_name,
        tier,
        dtype: DType::FieldElement,
        input_dtypes: vec![DType::FieldElement; input_shapes.len()],
        input_scales: vec![0; input_shapes.len()],
        input_shapes: input_shapes.iter().map(|shape| shape.to_vec()).collect(),
        params: params.to_vec(),
        input_data: input_data.iter().map(|data| data.to_vec()).collect(),
        expected_dtype: DType::FieldElement,
        expected_scale: 0,
        expected_data: expected_data.to_vec(),
        expected_shape: expected_shape.to_vec(),
    }
}

#[allow(clippy::too_many_arguments)]
fn scaled_vector(
    id: &'static str,
    op_name: &'static str,
    tier: &'static str,
    input_shapes: &[&[usize]],
    input_dtypes: &[DType],
    input_scales: &[i64],
    params: &[(&'static str, u64)],
    input_data: &[&[Elem]],
    expected_dtype: DType,
    expected_scale: i64,
    expected_data: &[Elem],
    expected_shape: &[usize],
) -> ConformanceVector {
    ConformanceVector {
        id,
        op_name,
        tier,
        dtype: expected_dtype,
        input_dtypes: input_dtypes.to_vec(),
        input_scales: input_scales.to_vec(),
        input_shapes: input_shapes.iter().map(|shape| shape.to_vec()).collect(),
        params: params.to_vec(),
        input_data: input_data.iter().map(|data| data.to_vec()).collect(),
        expected_dtype,
        expected_scale,
        expected_data: expected_data.to_vec(),
        expected_shape: expected_shape.to_vec(),
    }
}

fn mse_loss_expected(lhs: &[Elem], rhs: &[Elem]) -> Vec<Elem> {
    let mut acc = 0_u128;
    for (left, right) in lhs.iter().zip(rhs) {
        let diff = field::sub(*left, *right);
        acc += diff as u128 * diff as u128;
    }
    hash_bytes(
        b"tensor-vm-mse-loss-v1",
        &[
            &field::reduce_u128(acc).to_le_bytes(),
            &(lhs.len() as u64).to_le_bytes(),
        ],
    )
    .iter()
    .map(|byte| *byte as Elem)
    .collect()
}

fn encode_vector(vector: &ConformanceVector, out: &mut Vec<u8>) {
    encode_str(vector.id, out);
    encode_str(vector.op_name, out);
    encode_str(vector.tier, out);
    out.push(vector.dtype.tag());
    out.extend_from_slice(&(vector.input_shapes.len() as u64).to_le_bytes());
    for ((shape, dtype), scale) in vector
        .input_shapes
        .iter()
        .zip(vector.input_dtypes.iter())
        .zip(vector.input_scales.iter())
    {
        out.push(dtype.tag());
        out.extend_from_slice(&scale.to_le_bytes());
        encode_shape(shape, out);
    }
    out.extend_from_slice(&(vector.params.len() as u64).to_le_bytes());
    for (key, value) in &vector.params {
        encode_str(key, out);
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&(vector.input_data.len() as u64).to_le_bytes());
    for data in &vector.input_data {
        encode_field_slice(data, out);
    }
    out.push(vector.expected_dtype.tag());
    out.extend_from_slice(&vector.expected_scale.to_le_bytes());
    encode_field_slice(&vector.expected_data, out);
    encode_shape(&vector.expected_shape, out);
}

fn encode_str(value: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn encode_shape(shape: &[usize], out: &mut Vec<u8>) {
    out.extend_from_slice(&(shape.len() as u64).to_le_bytes());
    for dim in shape {
        out.extend_from_slice(&(*dim as u64).to_le_bytes());
    }
}

fn encode_field_slice(values: &[Elem], out: &mut Vec<u8>) {
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conformance_vectors_are_stable_and_cover_current_ops() {
        let vectors = conformance_vectors();
        let op_names = vectors
            .iter()
            .map(|vector| vector.op_name)
            .collect::<BTreeSet<_>>();
        assert!(op_names.contains("matmul"));
        assert!(op_names.contains("sub"));
        assert!(op_names.contains("scalar_mul"));
        assert!(op_names.contains("identity"));
        assert!(op_names.contains("neg"));
        assert!(op_names.contains("abs"));
        assert!(op_names.contains("sign"));
        assert!(op_names.contains("round"));
        assert!(op_names.contains("cast"));
        assert!(op_names.contains("relu"));
        assert!(op_names.contains("transpose"));
        assert!(op_names.contains("reshape"));
        assert!(op_names.contains("broadcast"));
        assert!(op_names.contains("mean"));
        assert!(op_names.contains("concat"));
        assert!(op_names.contains("stack"));
        assert!(op_names.contains("full"));
        assert!(op_names.contains("arange"));
        assert!(op_names.contains("mse_loss"));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-round-half-even-scale1-to-scale0-v1"
                && vector.input_dtypes == vec![DType::Fixed32]
                && vector.input_scales == vec![1]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 0
        }));
        assert_eq!(conformance_suite_hash(), conformance_suite_hash());
    }

    #[test]
    fn cpu_reference_passes_all_vectors() {
        let profile = cpu_reference_conformance_profile().unwrap();
        assert_eq!(profile.suite_hash, conformance_suite_hash());
        for op in [
            "add",
            "sub",
            "mul",
            "scalar_mul",
            "identity",
            "neg",
            "abs",
            "sign",
            "round",
            "cast",
            "relu",
            "transpose",
            "reshape",
            "broadcast",
            "reduce_sum",
            "mean",
            "concat",
            "stack",
            "matmul",
            "full",
            "arange",
            "mse_loss",
        ] {
            assert!(profile.passes(op), "missing conformance pass for {op}");
        }
    }

    #[test]
    fn required_conformance_gates_current_jobs() {
        let profile = cpu_reference_conformance_profile().unwrap();
        let beacon = hash_bytes(b"test", &[b"conformance-job"]);
        let matmul = MatmulJob::synthetic(0, 0, 2, 3, 2, &beacon, 10);
        ensure_tensor_op_job_conformance(&matmul, &profile).unwrap();

        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let linear = LinearTrainingStepJob::from_spec(crate::jobs::LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![2, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![2, 2],
            lr: 2,
            deadline_block: 10,
        });
        ensure_linear_training_step_conformance(&linear, &profile).unwrap();

        let empty = ConformanceProfile::empty_for_testing();
        assert_eq!(
            ensure_tensor_op_job_conformance(&matmul, &empty),
            Err(TvmError::InvalidReceipt("conformance suite unavailable"))
        );
        assert_eq!(
            ensure_linear_training_step_conformance(&linear, &empty),
            Err(TvmError::InvalidReceipt("conformance suite unavailable"))
        );
    }
}
