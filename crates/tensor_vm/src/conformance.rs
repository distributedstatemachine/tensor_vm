use std::collections::BTreeSet;

use crate::error::{Result, TvmError};
use crate::field::{self, Elem, MODULUS};
use crate::ir::{TensorGraph, canonical_linear_training_step_graph, canonical_matmul_graph};
use crate::jobs::{LinearTrainingStepJob, MatmulJob};
use crate::tensor::{
    DType, Tensor, divide_elem_for_dtype, rescale_signed_elem_half_even, signed_elem_to_i128,
    signed_i128_to_elem,
};
use crate::types::{Hash, hash_bytes};
use crate::vm;

const SUITE_VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceOutput {
    pub dtype: DType,
    pub scale: i64,
    pub data: Vec<Elem>,
    pub shape: Vec<usize>,
}

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
    pub expected_outputs: Vec<ConformanceOutput>,
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
        scaled_vector(
            "fixed32-mul-same-scale-half-even-v1",
            "mul",
            "B",
            &[&[6], &[6]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 2],
            &[],
            &[&[6, 7, p - 6, p - 7, 3, 5], &[6, 6, 6, 6, 6, p - 6]],
            DType::Fixed32,
            2,
            &[9, 10, p - 9, p - 10, 4, p - 8],
            &[6],
        ),
        scaled_vector(
            "fixed32-mul-mixed-scale-rhs-to-lhs-half-even-v1",
            "mul",
            "B",
            &[&[5], &[5]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 0],
            &[],
            &[&[6, p - 7, 3, p - 3, 5], &[2, p - 2, 1, p - 1, 0]],
            DType::Fixed32,
            2,
            &[12, 14, 3, 3, 0],
            &[5],
        ),
        scaled_vector(
            "fixed32-mul-mixed-scale-half-even-rounding-v1",
            "mul",
            "B",
            &[&[4], &[4]],
            &[DType::Fixed32, DType::Fixed32],
            &[0, 1],
            &[],
            &[&[2, 3, p - 3, p - 2], &[3, 3, 3, 3]],
            DType::Fixed32,
            0,
            &[3, 4, p - 4, p - 3],
            &[4],
        ),
        scaled_vector(
            "fixed32-add-mixed-scale-rhs-to-lhs-half-even-v1",
            "add",
            "B",
            &[&[5], &[5]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 0],
            &[],
            &[&[6, p - 7, 3, p - 3, 5], &[2, p - 2, 1, p - 1, 0]],
            DType::Fixed32,
            2,
            &[14, p - 15, 7, p - 7, 5],
            &[5],
        ),
        scaled_vector(
            "fixed32-sub-mixed-scale-rhs-to-lhs-half-even-v1",
            "sub",
            "B",
            &[&[5], &[5]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 0],
            &[],
            &[&[6, p - 7, 3, p - 3, 5], &[2, p - 2, 1, p - 1, 0]],
            DType::Fixed32,
            2,
            &[p - 2, 1, p - 1, 1, 5],
            &[5],
        ),
        scaled_vector(
            "field-div-broadcast-v1",
            "div",
            "B",
            &[&[2, 2], &[2]],
            &[DType::FieldElement, DType::FieldElement],
            &[0, 0],
            &[],
            &[&[2, 8, 4, 12], &[2, 4]],
            DType::FieldElement,
            0,
            &[1, 2, 2, 3],
            &[2, 2],
        ),
        scaled_vector(
            "fixed32-div-same-scale-half-even-v1",
            "div",
            "B",
            &[&[6], &[6]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 2],
            &[],
            &[&[12, p - 12, 7, p - 7, 10, p - 10], &[4, 4, 2, 2, 4, p - 4]],
            DType::Fixed32,
            2,
            &[12, p - 12, 14, p - 14, 10, 10],
            &[6],
        ),
        scaled_vector(
            "fixed32-div-mixed-scale-half-even-rounding-v1",
            "div",
            "B",
            &[&[4], &[4]],
            &[DType::Fixed32, DType::Fixed32],
            &[0, 1],
            &[],
            &[&[9, 7, p - 9, p - 7], &[4, 4, 4, 4]],
            DType::Fixed32,
            0,
            &[4, 4, p - 4, p - 4],
            &[4],
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
        multi_output_vector(
            "fixed32-quantize-int8-per-channel-axis1-v1",
            "quantize_int8_per_channel",
            "B",
            &[&[2, 3]],
            &[DType::Fixed32],
            &[0],
            &[("axis", 1)],
            &[&[0, 64, 128, p - 64, p - 128, 127]],
            vec![
                ConformanceOutput {
                    dtype: DType::Int8,
                    scale: 0,
                    data: vec![0, 32, 64, p - 64, p - 64, 64],
                    shape: vec![2, 3],
                },
                ConformanceOutput {
                    dtype: DType::Fixed32,
                    scale: 0,
                    data: vec![1, 2, 2],
                    shape: vec![3],
                },
            ],
        ),
        scaled_vector(
            "int8-dequantize-per-channel-axis1-v1",
            "dequantize_int8_per_channel",
            "B",
            &[&[2, 3], &[3]],
            &[DType::Int8, DType::Fixed32],
            &[0, 0],
            &[],
            &[&[0, 32, 64, p - 64, p - 64, 64], &[1, 2, 2]],
            DType::Fixed32,
            0,
            &[0, 64, 128, p - 64, p - 128, 128],
            &[2, 3],
        ),
        scaled_vector(
            "fixed32-quantize-pack-int8-axis1-v1",
            "quantize_pack_int8",
            "B",
            &[&[2, 3]],
            &[DType::Fixed32],
            &[0],
            &[("axis", 1)],
            &[&[0, 64, 128, p - 64, p - 128, 127]],
            DType::Uint8,
            0,
            &[
                84, 86, 81, 56, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0,
                0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                0, 0, 0, 32, 64, 192, 192, 64,
            ],
            &[62],
        ),
        scaled_vector(
            "uint8-unpack-dequantize-int8-axis1-v1",
            "unpack_dequantize_int8",
            "B",
            &[&[62]],
            &[DType::Uint8],
            &[0],
            &[("axis", 1), ("scale", 0)],
            &[&[
                84, 86, 81, 56, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0,
                0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                0, 0, 0, 32, 64, 192, 192, 64,
            ]],
            DType::Fixed32,
            0,
            &[0, 64, 128, p - 64, p - 128, 128],
            &[2, 3],
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
            "field-sum-axis1-v1",
            "sum",
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
        multi_output_vector(
            "field-split-axis1-sizes1-3-v1",
            "split",
            "B",
            &[&[2, 4]],
            &[DType::FieldElement],
            &[0],
            &[("axis", 1), ("size0", 1), ("size1", 3)],
            &[&[1, 2, 3, 4, 5, 6, 7, 8]],
            vec![
                ConformanceOutput {
                    dtype: DType::FieldElement,
                    scale: 0,
                    data: vec![1, 5],
                    shape: vec![2, 1],
                },
                ConformanceOutput {
                    dtype: DType::FieldElement,
                    scale: 0,
                    data: vec![2, 3, 4, 6, 7, 8],
                    shape: vec![2, 3],
                },
            ],
        ),
        scaled_vector(
            "field-gt-broadcast-mask-v1",
            "gt",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::FieldElement, DType::FieldElement],
            &[0, 0],
            &[],
            &[&[1, 4], &[0, 4, 5]],
            DType::Int32,
            0,
            &[1, 0, 0, 1, 0, 0],
            &[2, 3],
        ),
        scaled_vector(
            "field-lt-broadcast-mask-v1",
            "lt",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::FieldElement, DType::FieldElement],
            &[0, 0],
            &[],
            &[&[1, 4], &[0, 4, 5]],
            DType::Int32,
            0,
            &[0, 1, 1, 0, 0, 1],
            &[2, 3],
        ),
        scaled_vector(
            "field-ge-broadcast-mask-v1",
            "ge",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::FieldElement, DType::FieldElement],
            &[0, 0],
            &[],
            &[&[1, 4], &[0, 4, 5]],
            DType::Int32,
            0,
            &[1, 0, 0, 1, 1, 0],
            &[2, 3],
        ),
        scaled_vector(
            "field-le-broadcast-mask-v1",
            "le",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::FieldElement, DType::FieldElement],
            &[0, 0],
            &[],
            &[&[1, 4], &[0, 4, 5]],
            DType::Int32,
            0,
            &[0, 1, 1, 0, 1, 1],
            &[2, 3],
        ),
        scaled_vector(
            "fixed32-gt-same-scale-broadcast-mask-v1",
            "gt",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 2],
            &[],
            &[&[8, p - 4], &[0, 8, 9]],
            DType::Int32,
            0,
            &[1, 0, 0, 1, 1, 1],
            &[2, 3],
        ),
        scaled_vector(
            "fixed32-le-same-scale-broadcast-mask-v1",
            "le",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::Fixed32, DType::Fixed32],
            &[2, 2],
            &[],
            &[&[8, p - 4], &[0, 8, 9]],
            DType::Int32,
            0,
            &[0, 1, 1, 0, 0, 0],
            &[2, 3],
        ),
        scaled_vector(
            "bool-eq-broadcast-mask-v1",
            "eq",
            "B",
            &[&[2, 1], &[1, 3]],
            &[DType::Bool, DType::Bool],
            &[0, 0],
            &[],
            &[&[1, 0], &[1, 0, 1]],
            DType::Int32,
            0,
            &[1, 0, 1, 0, 1, 0],
            &[2, 3],
        ),
        scaled_vector(
            "fixed32-where-mask-broadcast-v1",
            "where",
            "B",
            &[&[2, 3], &[2, 1], &[1, 3]],
            &[DType::Int32, DType::Fixed32, DType::Fixed32],
            &[0, 1, 1],
            &[],
            &[&[1, 0, 1, 0, 1, 0], &[4, p - 6], &[1, p - 1, 8]],
            DType::Fixed32,
            1,
            &[4, p - 1, 4, 1, p - 6, 8],
            &[2, 3],
        ),
        scaled_vector(
            "int8-where-mask-broadcast-v1",
            "where",
            "B",
            &[&[2, 3], &[2, 1], &[1, 3]],
            &[DType::Int32, DType::Int8, DType::Int8],
            &[0, 0, 0],
            &[],
            &[&[1, 0, 1, 0, 1, 0], &[4, p - 6], &[1, p - 1, 8]],
            DType::Int8,
            0,
            &[4, p - 1, 4, 1, p - 6, 8],
            &[2, 3],
        ),
        vector(
            "field-clamp-field-order-v1",
            "clamp",
            "B",
            &[&[6]],
            &[("min", 2), ("max", 5)],
            &[&[0, 2, 4, 5, 7, p - 1]],
            &[2, 2, 4, 5, 5, 5],
            &[6],
        ),
        vector(
            "field-squeeze-dim1-v1",
            "squeeze",
            "B",
            &[&[2, 1, 3]],
            &[("dim", 1)],
            &[&[1, 2, 3, 4, 5, 6]],
            &[1, 2, 3, 4, 5, 6],
            &[2, 3],
        ),
        vector(
            "field-unsqueeze-dim1-v1",
            "unsqueeze",
            "B",
            &[&[2, 3]],
            &[("dim", 1)],
            &[&[1, 2, 3, 4, 5, 6]],
            &[1, 2, 3, 4, 5, 6],
            &[2, 1, 3],
        ),
        vector(
            "field-slice-dim0-v1",
            "slice",
            "B",
            &[&[3, 4]],
            &[("dim", 0), ("start", 1), ("end", 3)],
            &[&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]],
            &[5, 6, 7, 8, 9, 10, 11, 12],
            &[2, 4],
        ),
        vector(
            "field-tril-main-diagonal-v1",
            "tril",
            "B",
            &[&[3, 3]],
            &[("diagonal", 0)],
            &[&[1, 2, 3, 4, 5, 6, 7, 8, 9]],
            &[1, 0, 0, 4, 5, 0, 7, 8, 9],
            &[3, 3],
        ),
        vector(
            "field-triu-main-diagonal-v1",
            "triu",
            "B",
            &[&[3, 3]],
            &[("diagonal", 0)],
            &[&[1, 2, 3, 4, 5, 6, 7, 8, 9]],
            &[1, 2, 3, 0, 5, 6, 0, 0, 9],
            &[3, 3],
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
        scaled_vector(
            "fixed32-matmul-mixed-scale-accumulate-half-even-v1",
            "matmul",
            "A",
            &[&[2, 2], &[2, 2]],
            &[DType::Fixed32, DType::Fixed32],
            &[0, 1],
            &[],
            &[&[1, 1, 3, p - 3], &[1, 2, 0, 4]],
            DType::Fixed32,
            0,
            &[0, 3, 2, p - 3],
            &[2, 2],
        ),
        vector(
            "field-einsum-matrix-contraction-v1",
            "einsum",
            "A",
            &[&[2, 3], &[3, 2]],
            &[("equation", 0)],
            &[&[1, 2, 3, 4, 5, 6], &[7, 8, 9, 10, 11, 12]],
            &[58, 64, 139, 154],
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
        if execute_vector_outputs(vector)? != expected_tensors(vector)? {
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

fn execute_vector_outputs(vector: &ConformanceVector) -> Result<Vec<Tensor>> {
    let tensors = vector.input_tensors()?;
    let output = match vector.op_name {
        "add" => tensors[0].add(&tensors[1]),
        "sub" => tensors[0].sub(&tensors[1]),
        "mul" => tensors[0].mul(&tensors[1]),
        "div" => field_div_tensor(&tensors[0], &tensors[1]),
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
        "sum" | "reduce_sum" => tensors[0].reduce_sum(param(vector, "axis")? as usize),
        "mean" => mean_tensor(&tensors[0], param(vector, "axis")? as usize),
        "concat" => concat_tensors(&tensors, param(vector, "axis")? as usize),
        "stack" => stack_tensors(&tensors, param(vector, "axis")? as usize),
        "split" => {
            return split_tensor(
                &tensors[0],
                param(vector, "axis")? as usize,
                split_sizes(vector)?,
            );
        }
        "gt" => compare_tensors(&tensors[0], &tensors[1], |lhs, rhs| lhs > rhs),
        "lt" => compare_tensors(&tensors[0], &tensors[1], |lhs, rhs| lhs < rhs),
        "ge" => compare_tensors(&tensors[0], &tensors[1], |lhs, rhs| lhs >= rhs),
        "le" => compare_tensors(&tensors[0], &tensors[1], |lhs, rhs| lhs <= rhs),
        "eq" => compare_tensors(&tensors[0], &tensors[1], |lhs, rhs| lhs == rhs),
        "where" => where_tensor(&tensors[0], &tensors[1], &tensors[2]),
        "clamp" => clamp_tensor(&tensors[0], param(vector, "min")?, param(vector, "max")?),
        "squeeze" => squeeze_tensor(&tensors[0], param(vector, "dim")? as usize),
        "unsqueeze" => unsqueeze_tensor(&tensors[0], param(vector, "dim")? as usize),
        "slice" => slice_tensor(
            &tensors[0],
            param(vector, "dim")? as usize,
            param(vector, "start")? as usize,
            param(vector, "end")? as usize,
        ),
        "tril" => triangular_tensor(&tensors[0], param(vector, "diagonal")? as i64, true),
        "triu" => triangular_tensor(&tensors[0], param(vector, "diagonal")? as i64, false),
        "matmul" => tensors[0].matmul(&tensors[1]),
        "einsum" => match param(vector, "equation")? {
            0 => tensors[0].matmul(&tensors[1]),
            _ => Err(TvmError::InvalidReceipt("invalid conformance einsum")),
        },
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
        "quantize_int8_per_channel" => {
            let scale = quantize_scales(&tensors[0], param(vector, "axis")? as usize)?;
            let quantized = quantize_tensor(&tensors[0], param(vector, "axis")? as usize, &scale)?;
            let scale_tensor = Tensor::from_vec_with_scale(
                vec![scale.len()],
                DType::Fixed32,
                tensors[0].scale(),
                scale
                    .into_iter()
                    .map(crate::tensor::signed_i128_to_elem)
                    .collect(),
            )?;
            return Ok(vec![quantized, scale_tensor]);
        }
        "dequantize_int8_per_channel" => dequantize_tensor(&tensors[0], &tensors[1]),
        "quantize_pack_int8" => quantize_pack_tensor(&tensors[0], param(vector, "axis")? as usize),
        "unpack_dequantize_int8" => unpack_dequantize_tensor(
            &tensors[0],
            param(vector, "axis")? as usize,
            param(vector, "scale")? as i64,
            &vector.expected_shape,
        ),
        _ => Err(TvmError::InvalidReceipt("unknown conformance op")),
    }?;
    Ok(vec![output])
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

fn compare_tensors(
    lhs: &Tensor,
    rhs: &Tensor,
    predicate: impl Fn(Elem, Elem) -> bool,
) -> Result<Tensor> {
    if lhs.dtype() != rhs.dtype() || lhs.scale() != rhs.scale() {
        return Err(TvmError::InvalidReceipt("invalid conformance comparison"));
    }
    let shape = broadcast_shape(&[lhs.shape(), rhs.shape()])?;
    let len = shape.iter().try_fold(1usize, |product, dim| {
        product
            .checked_mul(*dim)
            .ok_or(TvmError::InvalidReceipt("invalid conformance comparison"))
    })?;
    let mut data = Vec::with_capacity(len);
    for index in 0..len {
        data.push(
            if predicate(
                broadcast_value(lhs, &shape, index)?,
                broadcast_value(rhs, &shape, index)?,
            ) {
                1
            } else {
                0
            },
        );
    }
    Tensor::from_vec(shape, DType::Int32, data)
}

fn field_div_tensor(lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
    let valid = match lhs.dtype() {
        DType::FieldElement => {
            rhs.dtype() == DType::FieldElement && lhs.scale() == 0 && rhs.scale() == 0
        }
        DType::Fixed32 => rhs.dtype() == DType::Fixed32,
        _ => false,
    };
    if !valid {
        return Err(TvmError::InvalidReceipt("invalid conformance div"));
    }
    let shape = broadcast_shape(&[lhs.shape(), rhs.shape()])?;
    let len = shape.iter().try_fold(1usize, |product, dim| {
        product
            .checked_mul(*dim)
            .ok_or(TvmError::InvalidReceipt("invalid conformance div"))
    })?;
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

fn where_tensor(cond: &Tensor, when_true: &Tensor, when_false: &Tensor) -> Result<Tensor> {
    if cond.dtype() != DType::Int32
        || cond.scale() != 0
        || when_true.dtype() != when_false.dtype()
        || when_true.scale() != when_false.scale()
    {
        return Err(TvmError::InvalidReceipt("invalid conformance where"));
    }
    let shape = broadcast_shape(&[cond.shape(), when_true.shape(), when_false.shape()])?;
    let len = shape.iter().try_fold(1usize, |product, dim| {
        product
            .checked_mul(*dim)
            .ok_or(TvmError::InvalidReceipt("invalid conformance where"))
    })?;
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

fn clamp_tensor(tensor: &Tensor, min: Elem, max: Elem) -> Result<Tensor> {
    if min > max {
        return Err(TvmError::InvalidReceipt("invalid conformance clamp"));
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

fn squeeze_tensor(tensor: &Tensor, dim: usize) -> Result<Tensor> {
    let mut shape = tensor.shape().to_vec();
    if dim >= shape.len() || shape[dim] != 1 || shape.len() == 1 {
        return Err(TvmError::InvalidReceipt("invalid conformance squeeze"));
    }
    shape.remove(dim);
    Tensor::from_vec_with_scale(
        shape,
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().to_vec(),
    )
}

fn unsqueeze_tensor(tensor: &Tensor, dim: usize) -> Result<Tensor> {
    let mut shape = tensor.shape().to_vec();
    if dim > shape.len() {
        return Err(TvmError::InvalidReceipt("invalid conformance unsqueeze"));
    }
    shape.insert(dim, 1);
    Tensor::from_vec_with_scale(
        shape,
        tensor.dtype(),
        tensor.scale(),
        tensor.as_slice().to_vec(),
    )
}

fn slice_tensor(tensor: &Tensor, dim: usize, start: usize, end: usize) -> Result<Tensor> {
    if dim >= tensor.shape().len() || start > end || end > tensor.shape()[dim] || start == end {
        return Err(TvmError::InvalidReceipt("invalid conformance slice"));
    }
    let mut shape = tensor.shape().to_vec();
    shape[dim] = end - start;
    let mut data = Vec::with_capacity(shape.iter().product());
    for index in 0..shape.iter().product() {
        let mut coords = unravel_index(&shape, index)?;
        coords[dim] += start;
        data.push(tensor.as_slice()[ravel_index(tensor.shape(), &coords)?]);
    }
    Tensor::from_vec_with_scale(shape, tensor.dtype(), tensor.scale(), data)
}

fn split_tensor(tensor: &Tensor, axis: usize, sizes: Vec<usize>) -> Result<Vec<Tensor>> {
    if axis >= tensor.shape().len() || sizes.is_empty() {
        return Err(TvmError::InvalidReceipt("invalid conformance split"));
    }
    let total = sizes.iter().try_fold(0usize, |acc, size| {
        acc.checked_add(*size)
            .ok_or(TvmError::InvalidReceipt("invalid conformance split"))
    })?;
    if sizes.contains(&0) || total != tensor.shape()[axis] {
        return Err(TvmError::InvalidReceipt("invalid conformance split"));
    }
    let mut outputs = Vec::with_capacity(sizes.len());
    let mut offset = 0usize;
    for size in sizes {
        let mut shape = tensor.shape().to_vec();
        shape[axis] = size;
        let output_len = shape.iter().try_fold(1usize, |product, dim| {
            product
                .checked_mul(*dim)
                .ok_or(TvmError::InvalidReceipt("invalid conformance split"))
        })?;
        let mut data = Vec::with_capacity(output_len);
        for index in 0..output_len {
            let mut coords = unravel_index(&shape, index)?;
            coords[axis] += offset;
            data.push(tensor.as_slice()[ravel_index(tensor.shape(), &coords)?]);
        }
        outputs.push(Tensor::from_vec_with_scale(
            shape,
            tensor.dtype(),
            tensor.scale(),
            data,
        )?);
        offset += size;
    }
    Ok(outputs)
}

fn triangular_tensor(tensor: &Tensor, diagonal: i64, lower: bool) -> Result<Tensor> {
    if tensor.shape().len() != 2 {
        return Err(TvmError::InvalidReceipt("invalid conformance triangular"));
    }
    let rows = tensor.shape()[0];
    let cols = tensor.shape()[1];
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

fn broadcast_shape(shapes: &[&[usize]]) -> Result<Vec<usize>> {
    let rank = shapes.iter().map(|shape| shape.len()).max().unwrap_or(0);
    let mut out = Vec::with_capacity(rank);
    for offset in 0..rank {
        let mut dim = 1usize;
        for shape in shapes {
            let value = shape
                .len()
                .checked_sub(1 + offset)
                .map_or(1, |index| shape[index]);
            if value != 1 && dim != 1 && value != dim {
                return Err(TvmError::InvalidReceipt("invalid conformance broadcast"));
            }
            dim = dim.max(value);
        }
        out.push(dim);
    }
    out.reverse();
    Ok(out)
}

fn broadcast_value(tensor: &Tensor, output_shape: &[usize], output_index: usize) -> Result<Elem> {
    let output_coords = unravel_index(output_shape, output_index)?;
    let rank_offset = output_shape
        .len()
        .checked_sub(tensor.shape().len())
        .ok_or(TvmError::InvalidReceipt("invalid conformance broadcast"))?;
    let mut coords = Vec::with_capacity(tensor.shape().len());
    for (axis, dim) in tensor.shape().iter().enumerate() {
        coords.push(if *dim == 1 {
            0
        } else {
            output_coords[rank_offset + axis]
        });
    }
    let input_index = ravel_index(tensor.shape(), &coords)?;
    Ok(tensor.as_slice()[input_index])
}

fn quantize_scales(tensor: &Tensor, axis: usize) -> Result<Vec<i128>> {
    if tensor.dtype() != DType::Fixed32 || axis >= tensor.shape().len() {
        return Err(TvmError::InvalidReceipt("invalid conformance quantize"));
    }
    let mut max_abs = vec![0i128; tensor.shape()[axis]];
    for (index, value) in tensor.as_slice().iter().enumerate() {
        let channel = unravel_index(tensor.shape(), index)?[axis];
        max_abs[channel] = max_abs[channel].max(signed_elem_to_i128(*value).abs());
    }
    Ok(max_abs
        .into_iter()
        .map(|value| ((value + 126) / 127).max(1))
        .collect())
}

fn quantize_tensor(tensor: &Tensor, axis: usize, scales: &[i128]) -> Result<Tensor> {
    let mut data = Vec::with_capacity(tensor.len());
    for (index, value) in tensor.as_slice().iter().enumerate() {
        let channel = unravel_index(tensor.shape(), index)?[axis];
        let rounded = div_round_half_even_i128(signed_elem_to_i128(*value), scales[channel])?
            .clamp(-128, 127);
        data.push(signed_i128_to_elem(rounded));
    }
    Tensor::from_vec(tensor.shape().to_vec(), DType::Int8, data)
}

fn dequantize_tensor(quantized: &Tensor, scale: &Tensor) -> Result<Tensor> {
    if quantized.dtype() != DType::Int8
        || quantized.scale() != 0
        || scale.dtype() != DType::Fixed32
        || scale.shape().len() != 1
    {
        return Err(TvmError::InvalidReceipt("invalid conformance dequantize"));
    }
    let channel_dim = dequantize_channel_dim(quantized.shape(), scale.len())?;
    let mut data = Vec::with_capacity(quantized.len());
    for (index, value) in quantized.as_slice().iter().enumerate() {
        let channel = if scale.len() == 1 {
            0
        } else {
            unravel_index(quantized.shape(), index)?[channel_dim]
        };
        data.push(signed_i128_to_elem(
            signed_elem_to_i128(*value) * signed_elem_to_i128(scale.as_slice()[channel]),
        ));
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
    let dim = matches
        .next()
        .ok_or(TvmError::InvalidReceipt("invalid conformance dequantize"))?;
    if matches.next().is_some() {
        return Err(TvmError::InvalidReceipt("invalid conformance dequantize"));
    }
    Ok(dim)
}

fn quantize_pack_tensor(tensor: &Tensor, axis: usize) -> Result<Tensor> {
    let scales = quantize_scales(tensor, axis)?;
    let quantized = quantize_tensor(tensor, axis, &scales)?;
    let scale_elems = scales
        .iter()
        .map(|value| signed_i128_to_elem(*value))
        .collect::<Vec<_>>();
    Tensor::from_packed_int8_payload(
        tensor.shape().to_vec(),
        axis,
        tensor.scale(),
        &scale_elems,
        quantized.as_slice(),
    )
}

fn unpack_dequantize_tensor(
    tensor: &Tensor,
    axis: usize,
    output_scale: i64,
    expected_shape: &[usize],
) -> Result<Tensor> {
    let decoded = tensor.packed_int8_payload()?;
    if decoded.shape != expected_shape
        || decoded.axis != axis
        || decoded.output_scale != output_scale
    {
        return Err(TvmError::InvalidReceipt(
            "invalid conformance packed dequantize",
        ));
    }
    let q = Tensor::from_vec(decoded.shape, DType::Int8, decoded.quantized)?;
    let scale = Tensor::from_vec_with_scale(
        vec![decoded.scales.len()],
        DType::Fixed32,
        decoded.output_scale,
        decoded.scales,
    )?;
    dequantize_tensor(&q, &scale)
}

fn div_round_half_even_i128(value: i128, divisor: i128) -> Result<i128> {
    if divisor <= 0 {
        return Err(TvmError::InvalidReceipt("invalid conformance quantize"));
    }
    let sign = if value < 0 { -1 } else { 1 };
    let abs = value.abs();
    let quotient = abs / divisor;
    let remainder = abs % divisor;
    let twice = remainder
        .checked_mul(2)
        .ok_or(TvmError::InvalidReceipt("invalid conformance quantize"))?;
    let rounded_abs = if twice > divisor || (twice == divisor && quotient % 2 == 1) {
        quotient
            .checked_add(1)
            .ok_or(TvmError::InvalidReceipt("invalid conformance quantize"))?
    } else {
        quotient
    };
    Ok(if sign < 0 { -rounded_abs } else { rounded_abs })
}

fn unravel_index(shape: &[usize], mut index: usize) -> Result<Vec<usize>> {
    let mut coords = vec![0; shape.len()];
    for axis in (0..shape.len()).rev() {
        let dim = shape[axis];
        if dim == 0 {
            return Err(TvmError::InvalidReceipt("invalid conformance shape"));
        }
        coords[axis] = index % dim;
        index /= dim;
    }
    Ok(coords)
}

fn ravel_index(shape: &[usize], coords: &[usize]) -> Result<usize> {
    if shape.len() != coords.len() {
        return Err(TvmError::InvalidReceipt("invalid conformance shape"));
    }
    let mut index = 0usize;
    for (dim, coord) in shape.iter().zip(coords.iter()) {
        if *coord >= *dim {
            return Err(TvmError::InvalidReceipt("invalid conformance shape"));
        }
        index = index
            .checked_mul(*dim)
            .and_then(|value| value.checked_add(*coord))
            .ok_or(TvmError::InvalidReceipt("invalid conformance shape"))?;
    }
    Ok(index)
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

fn expected_tensors(vector: &ConformanceVector) -> Result<Vec<Tensor>> {
    if vector.expected_outputs.is_empty() {
        return Ok(vec![Tensor::from_vec_with_scale(
            vector.expected_shape.clone(),
            vector.expected_dtype,
            vector.expected_scale,
            vector.expected_data.clone(),
        )?]);
    }
    vector
        .expected_outputs
        .iter()
        .map(|output| {
            Tensor::from_vec_with_scale(
                output.shape.clone(),
                output.dtype,
                output.scale,
                output.data.clone(),
            )
        })
        .collect()
}

fn param(vector: &ConformanceVector, name: &str) -> Result<u64> {
    vector
        .params
        .iter()
        .find_map(|(key, value)| (*key == name).then_some(*value))
        .ok_or(TvmError::InvalidReceipt("missing conformance param"))
}

fn split_sizes(vector: &ConformanceVector) -> Result<Vec<usize>> {
    let mut sizes = Vec::new();
    for index in 0.. {
        let key = format!("size{index}");
        match vector
            .params
            .iter()
            .find_map(|(param_key, value)| (*param_key == key.as_str()).then_some(*value))
        {
            Some(value) if value > 0 => sizes.push(value as usize),
            Some(_) => return Err(TvmError::InvalidReceipt("invalid conformance split")),
            None => break,
        }
    }
    if sizes.is_empty() {
        return Err(TvmError::InvalidReceipt("invalid conformance split"));
    }
    Ok(sizes)
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
        expected_outputs: Vec::new(),
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
        expected_outputs: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn multi_output_vector(
    id: &'static str,
    op_name: &'static str,
    tier: &'static str,
    input_shapes: &[&[usize]],
    input_dtypes: &[DType],
    input_scales: &[i64],
    params: &[(&'static str, u64)],
    input_data: &[&[Elem]],
    expected_outputs: Vec<ConformanceOutput>,
) -> ConformanceVector {
    let first = expected_outputs
        .first()
        .expect("multi-output conformance vectors need outputs");
    ConformanceVector {
        id,
        op_name,
        tier,
        dtype: first.dtype,
        input_dtypes: input_dtypes.to_vec(),
        input_scales: input_scales.to_vec(),
        input_shapes: input_shapes.iter().map(|shape| shape.to_vec()).collect(),
        params: params.to_vec(),
        input_data: input_data.iter().map(|data| data.to_vec()).collect(),
        expected_dtype: first.dtype,
        expected_scale: first.scale,
        expected_data: first.data.clone(),
        expected_shape: first.shape.clone(),
        expected_outputs,
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
    out.extend_from_slice(&(vector.expected_outputs.len() as u64).to_le_bytes());
    for output in &vector.expected_outputs {
        out.push(output.dtype.tag());
        out.extend_from_slice(&output.scale.to_le_bytes());
        encode_field_slice(&output.data, out);
        encode_shape(&output.shape, out);
    }
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
    use crate::ir::frozen_op_registry;

    fn consensus_admitted_ops() -> BTreeSet<&'static str> {
        frozen_op_registry()
            .iter()
            .filter(|spec| spec.consensus_admitted)
            .map(|spec| spec.name)
            .collect()
    }

    fn auxiliary_conformance_ops() -> BTreeSet<&'static str> {
        BTreeSet::from(["mse_loss"])
    }

    #[test]
    fn conformance_vectors_are_stable_and_cover_current_ops() {
        let vectors = conformance_vectors();
        let mut vector_ids = BTreeSet::new();
        for vector in &vectors {
            assert!(
                vector_ids.insert(vector.id),
                "duplicate conformance vector id {}",
                vector.id
            );
        }
        let op_names = vectors
            .iter()
            .map(|vector| vector.op_name)
            .collect::<BTreeSet<_>>();
        assert!(op_names.contains("matmul"));
        assert!(op_names.contains("sub"));
        assert!(op_names.contains("div"));
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
        assert!(op_names.contains("sum"));
        assert!(op_names.contains("mean"));
        assert!(op_names.contains("concat"));
        assert!(op_names.contains("stack"));
        assert!(op_names.contains("split"));
        assert!(op_names.contains("full"));
        assert!(op_names.contains("arange"));
        assert!(op_names.contains("einsum"));
        assert!(op_names.contains("quantize_int8_per_channel"));
        assert!(op_names.contains("dequantize_int8_per_channel"));
        assert!(op_names.contains("mse_loss"));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-round-half-even-scale1-to-scale0-v1"
                && vector.input_dtypes == vec![DType::Fixed32]
                && vector.input_scales == vec![1]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 0
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-add-mixed-scale-rhs-to-lhs-half-even-v1"
                && vector.input_scales == vec![2, 0]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 2
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-mul-mixed-scale-rhs-to-lhs-half-even-v1"
                && vector.input_scales == vec![2, 0]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 2
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-div-mixed-scale-half-even-rounding-v1"
                && vector.input_scales == vec![0, 1]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 0
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-gt-same-scale-broadcast-mask-v1"
                && vector.input_dtypes == vec![DType::Fixed32, DType::Fixed32]
                && vector.input_scales == vec![2, 2]
                && vector.expected_dtype == DType::Int32
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-le-same-scale-broadcast-mask-v1"
                && vector.input_dtypes == vec![DType::Fixed32, DType::Fixed32]
                && vector.input_scales == vec![2, 2]
                && vector.expected_dtype == DType::Int32
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-matmul-mixed-scale-accumulate-half-even-v1"
                && vector.input_scales == vec![0, 1]
                && vector.expected_dtype == DType::Fixed32
                && vector.expected_scale == 0
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "fixed32-quantize-int8-per-channel-axis1-v1"
                && vector.expected_outputs.len() == 2
                && vector.expected_outputs[0].dtype == DType::Int8
                && vector.expected_outputs[1].dtype == DType::Fixed32
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "field-split-axis1-sizes1-3-v1"
                && vector.expected_outputs.len() == 2
                && vector.expected_outputs[0].shape == vec![2, 1]
                && vector.expected_outputs[1].shape == vec![2, 3]
        }));
        assert!(vectors.iter().any(|vector| {
            vector.id == "int8-where-mask-broadcast-v1"
                && vector.input_dtypes == vec![DType::Int32, DType::Int8, DType::Int8]
                && vector.expected_dtype == DType::Int8
        }));
        assert_eq!(conformance_suite_hash(), conformance_suite_hash());
    }

    #[test]
    fn conformance_vectors_cover_every_consensus_admitted_op() {
        let op_names = conformance_vectors()
            .iter()
            .map(|vector| vector.op_name)
            .collect::<BTreeSet<_>>();
        let missing = consensus_admitted_ops()
            .difference(&op_names)
            .copied()
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing conformance vectors for admitted ops: {missing:?}"
        );
    }

    #[test]
    fn conformance_vectors_only_cover_admitted_or_auxiliary_ops() {
        let admitted = consensus_admitted_ops();
        let auxiliary = auxiliary_conformance_ops();
        let op_names = conformance_vectors()
            .iter()
            .map(|vector| vector.op_name)
            .collect::<BTreeSet<_>>();
        let unexpected = op_names
            .difference(&admitted)
            .filter(|op| !auxiliary.contains(**op))
            .copied()
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty(),
            "conformance vectors for non-admitted ops need explicit auxiliary status: {unexpected:?}"
        );
    }

    #[test]
    fn cpu_reference_passes_all_vectors() {
        let profile = cpu_reference_conformance_profile().unwrap();
        assert_eq!(profile.suite_hash, conformance_suite_hash());
        for op in [
            "add",
            "sub",
            "mul",
            "div",
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
            "sum",
            "reduce_sum",
            "mean",
            "concat",
            "stack",
            "split",
            "matmul",
            "einsum",
            "full",
            "arange",
            "quantize_int8_per_channel",
            "dequantize_int8_per_channel",
            "quantize_pack_int8",
            "unpack_dequantize_int8",
            "gt",
            "lt",
            "ge",
            "le",
            "eq",
            "where",
            "mse_loss",
        ] {
            assert!(profile.passes(op), "missing conformance pass for {op}");
        }
    }

    #[test]
    fn cpu_reference_passes_all_admitted_ops() {
        let profile = cpu_reference_conformance_profile().unwrap();
        let missing = consensus_admitted_ops()
            .into_iter()
            .filter(|op| !profile.passes(op))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "CPU reference profile missing admitted ops: {missing:?}"
        );
    }

    #[test]
    fn cpu_reference_profile_matches_registry_and_auxiliary_boundary() {
        let profile = cpu_reference_conformance_profile().unwrap();
        let vector_ops = conformance_vectors()
            .iter()
            .map(|vector| vector.op_name)
            .collect::<BTreeSet<_>>();
        assert_eq!(profile.passed_ops, vector_ops);

        let admitted = consensus_admitted_ops();
        let auxiliary = auxiliary_conformance_ops();
        let unexpected = profile
            .passed_ops
            .difference(&admitted)
            .filter(|op| !auxiliary.contains(**op))
            .copied()
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty(),
            "CPU reference profile passed non-admitted ops without auxiliary status: {unexpected:?}"
        );
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
