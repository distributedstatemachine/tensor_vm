use std::collections::BTreeSet;

use crate::error::{Result, TvmError};
use crate::field::{self, Elem, MODULUS};
use crate::ir::{TensorGraph, canonical_linear_training_step_graph, canonical_matmul_graph};
use crate::jobs::{LinearTrainingStepJob, MatmulJob};
use crate::tensor::{DType, Tensor};
use crate::types::{Hash, hash_bytes};
use crate::vm;

const SUITE_VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceVector {
    pub id: &'static str,
    pub op_name: &'static str,
    pub tier: &'static str,
    pub dtype: DType,
    pub input_shapes: Vec<Vec<usize>>,
    pub params: Vec<(&'static str, u64)>,
    pub input_data: Vec<Vec<Elem>>,
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
        "transpose" => tensors[0].transpose(),
        "reduce_sum" => tensors[0].reduce_sum(param(vector, "axis")? as usize),
        "matmul" => tensors[0].matmul(&tensors[1]),
        "mse_loss" => {
            let loss = vm::mse_loss(&tensors[0], &tensors[1])?;
            Tensor::from_vec(
                vec![32],
                DType::FieldElement,
                loss.iter().map(|byte| *byte as Elem).collect(),
            )
        }
        _ => Err(TvmError::InvalidReceipt("unknown conformance op")),
    }
}

fn expected_tensor(vector: &ConformanceVector) -> Result<Tensor> {
    Tensor::from_vec(
        vector.expected_shape.clone(),
        vector.dtype,
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
            .zip(self.input_data.iter().cloned())
            .map(|(shape, data)| Tensor::from_vec(shape, self.dtype, data))
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
        input_shapes: input_shapes.iter().map(|shape| shape.to_vec()).collect(),
        params: params.to_vec(),
        input_data: input_data.iter().map(|data| data.to_vec()).collect(),
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
    for shape in &vector.input_shapes {
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
        assert!(op_names.contains("transpose"));
        assert!(op_names.contains("mse_loss"));
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
            "transpose",
            "reduce_sum",
            "matmul",
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
