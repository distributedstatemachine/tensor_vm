use crate::conformance::{
    ConformanceProfile, conformance_suite_hash, cpu_reference_conformance_profile,
    ensure_linear_training_step_conformance, ensure_tensor_op_job_conformance,
};
use crate::error::{Result, TvmError};
use crate::field::{self, Elem};
use crate::ir::TensorGraph;
use crate::jobs::{
    GraphJob, GraphReceipt, LinearTrainingStepJob, LinearTrainingStepOutput,
    LinearTrainingStepReceipt, MatmulJob, PrimitiveType, TensorOpReceipt,
};
use crate::tensor::{Tensor, random_field_vector};
use crate::types::{Address, Hash, Signature, hash_bytes, sign, verify_signature};
use crate::vm;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreivaldsParams {
    pub full_rounds: usize,
    pub audit_rows: usize,
    pub validators_per_job: usize,
    pub minimum_validators: usize,
    pub minimum_stake_numerator: u64,
    pub minimum_stake_denominator: u64,
}

impl Default for FreivaldsParams {
    fn default() -> Self {
        Self {
            full_rounds: 1,
            audit_rows: 16,
            validators_per_job: 8,
            minimum_validators: 5,
            minimum_stake_numerator: 2,
            minimum_stake_denominator: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationResult {
    Valid,
    Invalid,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorOpVerificationReport {
    pub result: VerificationResult,
    pub full_freivalds_passed: bool,
    pub sampled_rows_checked: usize,
    pub data_availability_passed: bool,
    pub conformance_suite_hash: Hash,
    pub checks_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearVerificationReport {
    pub result: VerificationResult,
    pub forward_passed: bool,
    pub error_relation_passed: bool,
    pub backward_passed: bool,
    pub optimizer_passed: bool,
    pub data_availability_passed: bool,
    pub conformance_suite_hash: Hash,
    pub checks_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphVerificationReport {
    pub result: VerificationResult,
    pub exact_replay_passed: bool,
    pub data_availability_passed: bool,
    pub conformance_suite_hash: Hash,
    pub checks_root: Hash,
}

pub struct TensorOpConformanceVerification<'a> {
    pub job: &'a MatmulJob,
    pub receipt: &'a TensorOpReceipt,
    pub a: &'a Tensor,
    pub b: &'a Tensor,
    pub c: &'a Tensor,
    pub validation_seed: &'a Hash,
    pub params: &'a FreivaldsParams,
    pub conformance_profile: &'a ConformanceProfile,
}

pub struct LinearConformanceVerification<'a> {
    pub job: &'a LinearTrainingStepJob,
    pub receipt: &'a LinearTrainingStepReceipt,
    pub weights_before: &'a Tensor,
    pub output: &'a LinearTrainingStepOutput,
    pub validation_seed: &'a Hash,
    pub params: &'a FreivaldsParams,
    pub conformance_profile: &'a ConformanceProfile,
}

pub struct GraphConformanceVerification<'a> {
    pub job: &'a GraphJob,
    pub receipt: &'a GraphReceipt,
    pub graph: &'a TensorGraph,
    pub tensors: &'a std::collections::BTreeMap<String, Tensor>,
    pub validation_seed: &'a Hash,
    pub conformance_profile: &'a ConformanceProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAttestation {
    pub validator: Address,
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub primitive_type: PrimitiveType,
    pub result: VerificationResult,
    pub checks_root: Hash,
    pub data_availability_passed: bool,
    pub stake: u64,
    pub signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationStatement {
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub primitive_type: PrimitiveType,
    pub result: VerificationResult,
    pub checks_root: Hash,
    pub data_availability_passed: bool,
}

impl ValidatorAttestation {
    pub fn new(validator: Address, stake: u64, statement: AttestationStatement) -> Self {
        let message = attestation_digest(&validator, stake, &statement);
        Self {
            validator,
            receipt_id: statement.receipt_id,
            job_id: statement.job_id,
            primitive_type: statement.primitive_type,
            result: statement.result,
            checks_root: statement.checks_root,
            data_availability_passed: statement.data_availability_passed,
            stake,
            signature: sign(&validator, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        let statement = AttestationStatement {
            receipt_id: self.receipt_id,
            job_id: self.job_id,
            primitive_type: self.primitive_type,
            result: self.result,
            checks_root: self.checks_root,
            data_availability_passed: self.data_availability_passed,
        };
        let message = attestation_digest(&self.validator, self.stake, &statement);
        verify_signature(&self.validator, &message, &self.signature)
    }
}

pub fn full_freivalds(
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    seed: &Hash,
    rounds: usize,
) -> Result<bool> {
    if a.rows()? != c.rows()? || b.cols()? != c.cols()? || a.cols()? != b.rows()? {
        return Err(TvmError::DimensionMismatch {
            left: a.shape().to_vec(),
            right: b.shape().to_vec(),
        });
    }

    for round in 0..rounds.max(1) {
        let round_seed = hash_bytes(
            b"tensor-vm-full-freivalds-round-v1",
            &[seed, &(round as u64).to_le_bytes()],
        );
        let r = random_field_vector(&round_seed, b"tensor-vm-freivalds-vector-v1", c.cols()?);
        let br = b.dot_vector(&r)?;
        let abr = a.dot_vector(&br)?;
        let cr = c.dot_vector(&r)?;
        if abr != cr {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn row_sampled_freivalds(
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    seed: &Hash,
    rows_to_check: usize,
) -> Result<bool> {
    if rows_to_check == 0 {
        return Ok(true);
    }
    let rows = c.rows()?;
    let r = random_field_vector(seed, b"tensor-vm-row-freivalds-vector-v1", c.cols()?);
    let br = b.dot_vector(&r)?;
    let sample_rows = sample_distinct_rows(seed, rows, rows_to_check.min(rows));
    for row in sample_rows {
        let lhs = c.row_dot(row, &r)?;
        let rhs = a.row_dot(row, &br)?;
        if lhs != rhs {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn row_sample_detection_probability(
    total_rows: usize,
    corrupted_rows: usize,
    sampled_rows: usize,
) -> f64 {
    if total_rows == 0 || corrupted_rows == 0 || sampled_rows == 0 {
        return 0.0;
    }
    if corrupted_rows >= total_rows || sampled_rows >= total_rows {
        return 1.0;
    }
    let mut miss = 1.0_f64;
    for draw in 0..sampled_rows {
        let clean_remaining = total_rows - corrupted_rows - draw.min(total_rows - corrupted_rows);
        let total_remaining = total_rows - draw;
        if clean_remaining == 0 {
            return 1.0;
        }
        miss *= clean_remaining as f64 / total_remaining as f64;
    }
    1.0 - miss
}

pub fn verify_tensor_op(
    job: &MatmulJob,
    receipt: &TensorOpReceipt,
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    validation_seed: &Hash,
    params: &FreivaldsParams,
) -> Result<TensorOpVerificationReport> {
    let profile = cpu_reference_conformance_profile()?;
    verify_tensor_op_with_conformance_profile(TensorOpConformanceVerification {
        job,
        receipt,
        a,
        b,
        c,
        validation_seed,
        params,
        conformance_profile: &profile,
    })
}

pub fn verify_tensor_op_with_conformance_profile(
    input: TensorOpConformanceVerification<'_>,
) -> Result<TensorOpVerificationReport> {
    let TensorOpConformanceVerification {
        job,
        receipt,
        a,
        b,
        c,
        validation_seed,
        params,
        conformance_profile,
    } = input;
    ensure_tensor_op_job_conformance(job, conformance_profile)?;
    if receipt.job_id != job.job_id {
        return Err(TvmError::InvalidReceipt("job id mismatch"));
    }
    if receipt.submitted_at_block > job.deadline_block {
        return Err(TvmError::InvalidReceipt("receipt submitted after deadline"));
    }
    if receipt.receipt_id != receipt.recompute_receipt_id() {
        return Err(TvmError::InvalidReceipt("receipt digest mismatch"));
    }
    if !verify_signature(&receipt.miner, &receipt.receipt_id, &receipt.signature) {
        return Err(TvmError::InvalidReceipt("bad receipt signature"));
    }
    if receipt.program_hash != job.program_hash() {
        return Err(TvmError::InvalidReceipt("program hash mismatch"));
    }
    if receipt.input_roots != vec![a.commitment_root(), b.commitment_root()] {
        return Err(TvmError::InvalidReceipt("input roots mismatch"));
    }
    if receipt.output_roots != vec![c.commitment_root()] {
        return Err(TvmError::InvalidReceipt("output root mismatch"));
    }
    if a.shape() != [job.m, job.k] || b.shape() != [job.k, job.n] {
        return Err(TvmError::InvalidReceipt("input shape mismatch"));
    }
    if c.shape() != [job.m, job.n] {
        return Err(TvmError::InvalidReceipt("output shape mismatch"));
    }

    let data_availability_passed = true;
    let full_freivalds_passed = full_freivalds(a, b, c, validation_seed, params.full_rounds)?;
    let sampled_passed = row_sampled_freivalds(a, b, c, validation_seed, params.audit_rows)?;
    let result = if data_availability_passed && full_freivalds_passed && sampled_passed {
        VerificationResult::Valid
    } else {
        VerificationResult::Invalid
    };
    if result == VerificationResult::Valid {
        let execution = job.exact_ir_execution(a, b)?;
        if execution.outputs.get("c") != Some(c) || receipt.trace_root != execution.trace_root {
            return Err(TvmError::InvalidReceipt("trace root mismatch"));
        }
    }
    let checks_root = hash_bytes(
        b"tensor-vm-tensorop-checks-v1",
        &[
            validation_seed,
            &[full_freivalds_passed as u8],
            &[sampled_passed as u8],
            &(params.audit_rows as u64).to_le_bytes(),
            &conformance_suite_hash(),
        ],
    );
    Ok(TensorOpVerificationReport {
        result,
        full_freivalds_passed,
        sampled_rows_checked: params.audit_rows.min(job.m),
        data_availability_passed,
        conformance_suite_hash: conformance_suite_hash(),
        checks_root,
    })
}

pub fn verify_linear_training_step(
    job: &LinearTrainingStepJob,
    receipt: &LinearTrainingStepReceipt,
    weights_before: &Tensor,
    output: &LinearTrainingStepOutput,
    validation_seed: &Hash,
    params: &FreivaldsParams,
) -> Result<LinearVerificationReport> {
    let profile = cpu_reference_conformance_profile()?;
    verify_linear_training_step_with_conformance_profile(LinearConformanceVerification {
        job,
        receipt,
        weights_before,
        output,
        validation_seed,
        params,
        conformance_profile: &profile,
    })
}

pub fn verify_linear_training_step_with_conformance_profile(
    input: LinearConformanceVerification<'_>,
) -> Result<LinearVerificationReport> {
    let LinearConformanceVerification {
        job,
        receipt,
        weights_before,
        output,
        validation_seed,
        params,
        conformance_profile,
    } = input;
    ensure_linear_training_step_conformance(job, conformance_profile)?;
    if receipt.job_id != job.job_id {
        return Err(TvmError::InvalidReceipt("job id mismatch"));
    }
    if receipt.submitted_at_block > job.deadline_block {
        return Err(TvmError::InvalidReceipt("receipt submitted after deadline"));
    }
    if receipt.receipt_id != receipt.recompute_receipt_id(&job.program_hash()) {
        return Err(TvmError::InvalidReceipt("receipt digest mismatch"));
    }
    if !verify_signature(&receipt.miner, &receipt.receipt_id, &receipt.signature) {
        return Err(TvmError::InvalidReceipt("bad receipt signature"));
    }
    if weights_before.commitment_root() != job.weight_root_before {
        return Err(TvmError::InvalidReceipt("weight root mismatch"));
    }
    if receipt.weight_root_before != job.weight_root_before
        || receipt.y_root != output.y.commitment_root()
        || receipt.grad_w_root != output.grad_w.commitment_root()
        || receipt.weight_root_after != output.weight_after.commitment_root()
        || receipt.loss_commitment != output.loss_commitment
    {
        return Err(TvmError::InvalidReceipt("linear output root mismatch"));
    }

    let (expected_x, expected_target) = job.batch_tensors()?;
    if output.x != expected_x || output.target != expected_target {
        return Err(TvmError::InvalidReceipt("batch tensor mismatch"));
    }
    let expected_batch_root = hash_bytes(
        b"tensor-vm-linear-batch-root-v1",
        &[
            &output.x.commitment_root(),
            &output.target.commitment_root(),
        ],
    );
    if receipt.batch_root != expected_batch_root {
        return Err(TvmError::InvalidReceipt("batch root mismatch"));
    }
    let forward_passed = full_freivalds(
        &output.x,
        weights_before,
        &output.y,
        &hash_bytes(b"tensor-vm-linear-forward-seed-v1", &[validation_seed]),
        params.full_rounds,
    )?;
    let expected_dy = output.y.sub(&output.target)?;
    let error_relation_passed = random_linear_equal(
        &output.dy,
        &expected_dy,
        &hash_bytes(b"tensor-vm-linear-error-seed-v1", &[validation_seed]),
    )?;
    let x_t = output.x.transpose()?;
    let backward_passed = full_freivalds(
        &x_t,
        &output.dy,
        &output.grad_w,
        &hash_bytes(b"tensor-vm-linear-backward-seed-v1", &[validation_seed]),
        params.full_rounds,
    )?;
    let expected_weight = weights_before.sub(&output.grad_w.scalar_mul(job.lr)?)?;
    let optimizer_passed = random_linear_equal(
        &output.weight_after,
        &expected_weight,
        &hash_bytes(b"tensor-vm-linear-optimizer-seed-v1", &[validation_seed]),
    )?;
    let loss_passed = vm::mse_loss(&output.y, &output.target)? == output.loss_commitment;
    let data_availability_passed = true;
    let result = if forward_passed
        && error_relation_passed
        && backward_passed
        && optimizer_passed
        && loss_passed
        && data_availability_passed
    {
        VerificationResult::Valid
    } else {
        VerificationResult::Invalid
    };
    if result == VerificationResult::Valid {
        let execution = job.exact_ir_execution(weights_before, output)?;
        if execution.outputs.get("y") != Some(&output.y)
            || execution.outputs.get("dy") != Some(&output.dy)
            || execution.outputs.get("grad_w") != Some(&output.grad_w)
            || execution.outputs.get("weight_after") != Some(&output.weight_after)
            || receipt.trace_root != execution.trace_root
        {
            return Err(TvmError::InvalidReceipt("trace root mismatch"));
        }
    }
    let checks_root = hash_bytes(
        b"tensor-vm-linear-checks-v1",
        &[
            validation_seed,
            &[forward_passed as u8],
            &[error_relation_passed as u8],
            &[backward_passed as u8],
            &[optimizer_passed as u8],
            &[loss_passed as u8],
            &conformance_suite_hash(),
        ],
    );
    Ok(LinearVerificationReport {
        result,
        forward_passed,
        error_relation_passed,
        backward_passed,
        optimizer_passed,
        data_availability_passed,
        conformance_suite_hash: conformance_suite_hash(),
        checks_root,
    })
}

pub fn verify_graph_execution(
    job: &GraphJob,
    receipt: &GraphReceipt,
    graph: &TensorGraph,
    tensors: &std::collections::BTreeMap<String, Tensor>,
    validation_seed: &Hash,
) -> Result<GraphVerificationReport> {
    let profile = cpu_reference_conformance_profile()?;
    verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
        job,
        receipt,
        graph,
        tensors,
        validation_seed,
        conformance_profile: &profile,
    })
}

pub fn verify_graph_execution_with_conformance_profile(
    input: GraphConformanceVerification<'_>,
) -> Result<GraphVerificationReport> {
    let GraphConformanceVerification {
        job,
        receipt,
        graph,
        tensors,
        validation_seed,
        conformance_profile,
    } = input;
    if graph.validate_for_consensus()? != job.graph_id {
        return Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"));
    }
    for op in &graph.ops {
        if !conformance_profile.passes(&op.op) {
            return Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted",
            ));
        }
    }
    if receipt.job_id != job.job_id {
        return Err(TvmError::InvalidReceipt("job id mismatch"));
    }
    if receipt.submitted_at_block > job.deadline_block {
        return Err(TvmError::InvalidReceipt("receipt submitted after deadline"));
    }
    if receipt.graph_id != job.graph_id {
        return Err(TvmError::InvalidReceipt("graph id mismatch"));
    }
    if receipt.input_roots != job.input_roots {
        return Err(TvmError::InvalidReceipt("input roots mismatch"));
    }
    if receipt.tensor_work_units != job.tensor_work_units() {
        return Err(TvmError::InvalidReceipt("tensor work mismatch"));
    }
    if receipt.receipt_id != receipt.recompute_receipt_id() {
        return Err(TvmError::InvalidReceipt("receipt digest mismatch"));
    }
    if !verify_signature(&receipt.miner, &receipt.receipt_id, &receipt.signature) {
        return Err(TvmError::InvalidReceipt("bad receipt signature"));
    }
    let execution = job.exact_ir_execution(graph, tensors)?;
    let output_roots = execution
        .outputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let exact_replay_passed =
        output_roots == receipt.output_roots && execution.trace_root == receipt.trace_root;
    let data_availability_passed = true;
    let result = if exact_replay_passed && data_availability_passed {
        VerificationResult::Valid
    } else {
        VerificationResult::Invalid
    };
    let mut encoded_outputs = Vec::new();
    for (name, root) in &receipt.output_roots {
        encoded_outputs.extend_from_slice(&(name.len() as u64).to_le_bytes());
        encoded_outputs.extend_from_slice(name.as_bytes());
        encoded_outputs.extend_from_slice(root);
    }
    let checks_root = hash_bytes(
        b"tensor-vm-graph-checks-v1",
        &[
            validation_seed,
            &job.graph_id,
            &receipt.receipt_id,
            &receipt.trace_root,
            &encoded_outputs,
            &conformance_suite_hash(),
        ],
    );
    Ok(GraphVerificationReport {
        result,
        exact_replay_passed,
        data_availability_passed,
        conformance_suite_hash: conformance_suite_hash(),
        checks_root,
    })
}

fn random_linear_equal(left: &Tensor, right: &Tensor, seed: &Hash) -> Result<bool> {
    if left.shape() != right.shape() {
        return Err(TvmError::ShapeMismatch {
            left: left.shape().to_vec(),
            right: right.shape().to_vec(),
        });
    }
    let q = random_field_vector(seed, b"tensor-vm-random-linear-v1", left.len());
    Ok(left.linear_combination(&q)? == right.linear_combination(&q)?)
}

fn sample_distinct_rows(seed: &Hash, rows: usize, count: usize) -> Vec<usize> {
    let mut selected = Vec::with_capacity(count);
    let mut cursor = 0_u64;
    while selected.len() < count {
        let h = hash_bytes(
            b"tensor-vm-sample-row-v1",
            &[seed, &cursor.to_le_bytes(), &(rows as u64).to_le_bytes()],
        );
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&h[..8]);
        let row = (u64::from_le_bytes(bytes) as usize) % rows;
        if !selected.contains(&row) {
            selected.push(row);
        }
        cursor += 1;
    }
    selected
}

fn attestation_digest(validator: &Address, stake: u64, statement: &AttestationStatement) -> Hash {
    let primitive = match statement.primitive_type {
        PrimitiveType::TensorOp => 1_u8,
        PrimitiveType::LinearTrainingStep => 2_u8,
        PrimitiveType::GraphExecution => 3_u8,
    };
    let result = match statement.result {
        VerificationResult::Valid => 1_u8,
        VerificationResult::Invalid => 2_u8,
        VerificationResult::Unavailable => 3_u8,
    };
    hash_bytes(
        b"tensor-vm-attestation-v1",
        &[
            validator,
            &statement.receipt_id,
            &statement.job_id,
            &[primitive, result, statement.data_availability_passed as u8],
            &statement.checks_root,
            &stake.to_le_bytes(),
        ],
    )
}

#[allow(dead_code)]
fn linear_relation(left: Elem, right: Elem) -> bool {
    field::sub(left, right) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{GraphOutput, IrLiteral, IrRef, IrValue, OpNode, TensorSpec};
    use crate::jobs::{
        GraphJob, GraphReceipt, LinearTrainingStepJob, LinearTrainingStepReceipt,
        LinearTrainingStepSpec, TensorOpReceipt,
    };
    use crate::tensor::DType;
    use crate::types::{address, hash_bytes};
    use std::collections::BTreeMap;

    #[test]
    fn full_freivalds_accepts_honest_and_rejects_corruption() {
        let a = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![7, 8, 9, 10]).unwrap();
        let mut c = a.matmul(&b).unwrap();
        let seed = hash_bytes(b"test", &[b"freivalds"]);
        assert!(full_freivalds(&a, &b, &c, &seed, 2).unwrap());
        let bad = field::add(c.get2(1, 1).unwrap(), 1);
        c.set2(1, 1, bad).unwrap();
        assert!(!full_freivalds(&a, &b, &c, &seed, 2).unwrap());
    }

    #[test]
    fn row_sampling_probability_exposes_sparse_weakness() {
        let p = row_sample_detection_probability(1024, 1, 16);
        assert!((p - 16.0 / 1024.0).abs() < 1e-12);
        assert!(row_sample_detection_probability(1024, 1024, 16) == 1.0);
        assert_eq!(row_sample_detection_probability(10, 9, 2), 1.0);
        assert!(linear_relation(7, 7));
        assert!(!linear_relation(7, 8));
    }

    #[test]
    fn tensor_op_verifier_rejects_bad_output() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 8, 4, 4, &beacon, 10);
        let miner = address(b"miner");
        let (receipt, a, b, mut c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"validation"]);
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &seed,
            &FreivaldsParams::default(),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        c.set2(0, 0, field::add(c.get2(0, 0).unwrap(), 1)).unwrap();
        let mut bad = receipt.clone();
        bad.output_roots = vec![c.commitment_root()];
        bad.trace_root = hash_bytes(b"test", &[b"bad-tensorop-trace"]);
        bad.receipt_id = bad.recompute_receipt_id();
        bad.signature = sign(&bad.miner, &bad.receipt_id);
        let report =
            verify_tensor_op(&job, &bad, &a, &b, &c, &seed, &FreivaldsParams::default()).unwrap();
        assert_eq!(report.result, VerificationResult::Invalid);
    }

    #[test]
    fn tensor_op_verifier_requires_conformance_profile() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 3, 2, &beacon, 10);
        let miner = address(b"miner");
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"validation"]);
        let params = FreivaldsParams::default();
        let report = verify_tensor_op(&job, &receipt, &a, &b, &c, &seed, &params).unwrap();
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let empty = ConformanceProfile::empty_for_testing();
        assert_eq!(
            verify_tensor_op_with_conformance_profile(TensorOpConformanceVerification {
                job: &job,
                receipt: &receipt,
                a: &a,
                b: &b,
                c: &c,
                validation_seed: &seed,
                params: &params,
                conformance_profile: &empty,
            }),
            Err(TvmError::InvalidReceipt("conformance suite unavailable"))
        );

        let mut missing_matmul = cpu_reference_conformance_profile().unwrap();
        missing_matmul.passed_ops.remove("matmul");
        assert_eq!(
            verify_tensor_op_with_conformance_profile(TensorOpConformanceVerification {
                job: &job,
                receipt: &receipt,
                a: &a,
                b: &b,
                c: &c,
                validation_seed: &seed,
                params: &params,
                conformance_profile: &missing_matmul,
            }),
            Err(TvmError::InvalidReceipt("required op conformance missing"))
        );
    }

    #[test]
    fn tensor_op_verifier_rejects_deadline_and_signature_failures() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let miner = address(b"miner");
        let (mut receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 11, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"validation"]);
        assert_eq!(
            verify_tensor_op(
                &job,
                &receipt,
                &a,
                &b,
                &c,
                &seed,
                &FreivaldsParams::default()
            ),
            Err(TvmError::InvalidReceipt("receipt submitted after deadline"))
        );

        receipt = TensorOpReceipt::from_output(&job, miner, 1, 5, &a, &b, &c).unwrap();
        receipt.signature = [9; 32];
        assert_eq!(
            verify_tensor_op(
                &job,
                &receipt,
                &a,
                &b,
                &c,
                &seed,
                &FreivaldsParams::default()
            ),
            Err(TvmError::InvalidReceipt("bad receipt signature"))
        );
    }

    #[test]
    fn tensor_op_verifier_rejects_digest_and_trace_mismatch() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let miner = address(b"miner");
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"validation"]);

        let mut bad_digest = receipt.clone();
        bad_digest.tensor_work_units += 1;
        assert_eq!(
            verify_tensor_op(
                &job,
                &bad_digest,
                &a,
                &b,
                &c,
                &seed,
                &FreivaldsParams::default()
            ),
            Err(TvmError::InvalidReceipt("receipt digest mismatch"))
        );

        let mut bad_trace = receipt.clone();
        bad_trace.trace_root = hash_bytes(b"test", &[b"bad-trace"]);
        bad_trace.receipt_id = bad_trace.recompute_receipt_id();
        bad_trace.signature = sign(&bad_trace.miner, &bad_trace.receipt_id);
        assert_eq!(
            verify_tensor_op(
                &job,
                &bad_trace,
                &a,
                &b,
                &c,
                &seed,
                &FreivaldsParams::default()
            ),
            Err(TvmError::InvalidReceipt("trace root mismatch"))
        );
    }

    #[test]
    fn graph_verifier_accepts_unary_tier_b_graph_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![4],
                dtype: DType::FieldElement,
                scale: 0,
            }],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "relu".to_owned(),
                args: vec![IrRef::Input {
                    name: "x".to_owned(),
                }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "y".to_owned(),
                    shape: vec![4],
                    dtype: DType::FieldElement,
                    scale: 0,
                }],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let input =
            Tensor::from_vec(vec![4], DType::FieldElement, vec![0, 3, p - 2, p - 1]).unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 4);
        let (receipt, outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"graph-unary-miner"),
            &inputs,
            1,
            2,
        )
        .unwrap();

        assert_eq!(
            outputs["y"],
            Tensor::from_vec(vec![4], DType::FieldElement, vec![0, 3, 0, 0]).unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-unary-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-unary-validation"]),
                conformance_profile: &ConformanceProfile::empty_for_testing(),
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn graph_verifier_accepts_fixed_point_rescale_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![8],
                dtype: DType::Fixed32,
                scale: 1,
            }],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "round".to_owned(),
                args: vec![IrRef::Input {
                    name: "x".to_owned(),
                }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "y".to_owned(),
                    shape: vec![8],
                    dtype: DType::Fixed32,
                    scale: 0,
                }],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let data = vec![1, 3, p - 1, p - 3, 5, 7, p - 5, p - 7];
        let input = Tensor::from_vec_with_scale(vec![8], DType::Fixed32, 1, data).unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);
        let (receipt, outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"graph-fixed-round-miner"),
            &inputs,
            1,
            2,
        )
        .unwrap();

        assert_eq!(
            outputs["y"],
            Tensor::from_vec_with_scale(
                vec![8],
                DType::Fixed32,
                0,
                vec![0, 2, 0, p - 2, 2, 4, p - 2, p - 4]
            )
            .unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-fixed-round-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-fixed-round-validation"]),
                conformance_profile: &ConformanceProfile::empty_for_testing(),
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn graph_verifier_accepts_sum_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![2, 3],
                dtype: DType::FieldElement,
                scale: 0,
            }],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "sum".to_owned(),
                args: vec![IrRef::Input {
                    name: "x".to_owned(),
                }],
                kwargs: BTreeMap::from([("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)))]),
                out: vec![TensorSpec {
                    name: "y".to_owned(),
                    shape: vec![2],
                    dtype: DType::FieldElement,
                    scale: 0,
                }],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let input = Tensor::from_vec(
            vec![2, 3],
            DType::FieldElement,
            vec![p - 1, 2, 3, 4, p - 2, 6],
        )
        .unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 6);
        let (receipt, outputs) =
            GraphReceipt::from_execution(&job, &graph, address(b"graph-sum-miner"), &inputs, 1, 2)
                .unwrap();

        assert_eq!(
            outputs["y"],
            Tensor::from_vec(vec![2], DType::FieldElement, vec![4, 8]).unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-sum-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let mut missing_sum = cpu_reference_conformance_profile().unwrap();
        missing_sum.passed_ops.remove("sum");
        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-sum-validation"]),
                conformance_profile: &missing_sum,
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn graph_verifier_accepts_quantize_dequantize_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![2, 3],
                dtype: DType::Fixed32,
                scale: 0,
            }],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "quantize_int8_per_channel".to_owned(),
                    args: vec![IrRef::Input {
                        name: "x".to_owned(),
                    }],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        crate::ir::IrValue::Literal(crate::ir::IrLiteral::Uint(1)),
                    )]),
                    out: vec![
                        TensorSpec {
                            name: "q".to_owned(),
                            shape: vec![2, 3],
                            dtype: DType::Int8,
                            scale: 0,
                        },
                        TensorSpec {
                            name: "scale".to_owned(),
                            shape: vec![3],
                            dtype: DType::Fixed32,
                            scale: 0,
                        },
                    ],
                },
                OpNode {
                    id: 1,
                    op: "dequantize_int8_per_channel".to_owned(),
                    args: vec![IrRef::Op { id: 0, idx: 0 }, IrRef::Op { id: 0, idx: 1 }],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec {
                        name: "dq".to_owned(),
                        shape: vec![2, 3],
                        dtype: DType::Fixed32,
                        scale: 0,
                    }],
                },
            ],
            outputs: vec![GraphOutput {
                name: "dq".to_owned(),
                value: IrRef::Op { id: 1, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let input = Tensor::from_vec(
            vec![2, 3],
            DType::Fixed32,
            vec![0, 64, 128, p - 64, p - 128, 127],
        )
        .unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 12);
        let (receipt, outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"graph-quant-miner"),
            &inputs,
            1,
            2,
        )
        .unwrap();

        assert_eq!(
            outputs["dq"],
            Tensor::from_vec(
                vec![2, 3],
                DType::Fixed32,
                vec![0, 64, 128, p - 64, p - 128, 128]
            )
            .unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-quant-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let mut missing_quantize = cpu_reference_conformance_profile().unwrap();
        missing_quantize
            .passed_ops
            .remove("quantize_int8_per_channel");
        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-quant-validation"]),
                conformance_profile: &missing_quantize,
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn graph_verifier_accepts_packed_quantize_dequantize_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![2, 3],
                dtype: DType::Fixed32,
                scale: 0,
            }],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "quantize_pack_int8".to_owned(),
                    args: vec![IrRef::Input {
                        name: "x".to_owned(),
                    }],
                    kwargs: BTreeMap::from([(
                        "dim".to_owned(),
                        crate::ir::IrValue::Literal(crate::ir::IrLiteral::Uint(1)),
                    )]),
                    out: vec![TensorSpec {
                        name: "packed".to_owned(),
                        shape: vec![62],
                        dtype: DType::Uint8,
                        scale: 0,
                    }],
                },
                OpNode {
                    id: 1,
                    op: "unpack_dequantize_int8".to_owned(),
                    args: vec![IrRef::Op { id: 0, idx: 0 }],
                    kwargs: BTreeMap::from([
                        (
                            "dim".to_owned(),
                            crate::ir::IrValue::Literal(crate::ir::IrLiteral::Uint(1)),
                        ),
                        (
                            "shape".to_owned(),
                            crate::ir::IrValue::Literal(crate::ir::IrLiteral::List(vec![
                                crate::ir::IrLiteral::Uint(2),
                                crate::ir::IrLiteral::Uint(3),
                            ])),
                        ),
                        (
                            "scale_dim".to_owned(),
                            crate::ir::IrValue::Literal(crate::ir::IrLiteral::Int(0)),
                        ),
                    ]),
                    out: vec![TensorSpec {
                        name: "dq".to_owned(),
                        shape: vec![2, 3],
                        dtype: DType::Fixed32,
                        scale: 0,
                    }],
                },
            ],
            outputs: vec![GraphOutput {
                name: "dq".to_owned(),
                value: IrRef::Op { id: 1, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let input = Tensor::from_vec(
            vec![2, 3],
            DType::Fixed32,
            vec![0, 64, 128, p - 64, p - 128, 127],
        )
        .unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 12);
        let (receipt, outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"graph-pack-quant-miner"),
            &inputs,
            1,
            2,
        )
        .unwrap();

        assert_eq!(
            outputs["dq"],
            Tensor::from_vec(
                vec![2, 3],
                DType::Fixed32,
                vec![0, 64, 128, p - 64, p - 128, 128]
            )
            .unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-pack-quant-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let mut missing_unpack = cpu_reference_conformance_profile().unwrap();
        missing_unpack.passed_ops.remove("unpack_dequantize_int8");
        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-pack-quant-validation"]),
                conformance_profile: &missing_unpack,
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn graph_verifier_accepts_comparison_where_receipt() {
        let p = field::MODULUS;
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![
                TensorSpec {
                    name: "x".to_owned(),
                    shape: vec![2, 1],
                    dtype: DType::Fixed32,
                    scale: 1,
                },
                TensorSpec {
                    name: "y".to_owned(),
                    shape: vec![1, 3],
                    dtype: DType::Fixed32,
                    scale: 1,
                },
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "gt".to_owned(),
                    args: vec![
                        IrRef::Input {
                            name: "x".to_owned(),
                        },
                        IrRef::Input {
                            name: "y".to_owned(),
                        },
                    ],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec {
                        name: "mask".to_owned(),
                        shape: vec![2, 3],
                        dtype: DType::Int32,
                        scale: 0,
                    }],
                },
                OpNode {
                    id: 1,
                    op: "where".to_owned(),
                    args: vec![
                        IrRef::Op { id: 0, idx: 0 },
                        IrRef::Input {
                            name: "x".to_owned(),
                        },
                        IrRef::Input {
                            name: "y".to_owned(),
                        },
                    ],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec {
                        name: "selected".to_owned(),
                        shape: vec![2, 3],
                        dtype: DType::Fixed32,
                        scale: 1,
                    }],
                },
            ],
            outputs: vec![GraphOutput {
                name: "selected".to_owned(),
                value: IrRef::Op { id: 1, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let x = Tensor::from_vec_with_scale(vec![2, 1], DType::Fixed32, 1, vec![4, p - 6]).unwrap();
        let y =
            Tensor::from_vec_with_scale(vec![1, 3], DType::Fixed32, 1, vec![1, p - 1, 8]).unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), x.clone()), ("y".to_owned(), y.clone())]);
        let input_roots = BTreeMap::from([
            ("x".to_owned(), x.commitment_root()),
            ("y".to_owned(), y.commitment_root()),
        ]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 12);
        let (receipt, outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"graph-where-miner"),
            &inputs,
            1,
            2,
        )
        .unwrap();

        assert_eq!(
            outputs["selected"],
            Tensor::from_vec_with_scale(
                vec![2, 3],
                DType::Fixed32,
                1,
                vec![4, p - 1, 8, p - 6, p - 1, p - 6],
            )
            .unwrap()
        );
        let report = verify_graph_execution(
            &job,
            &receipt,
            &graph,
            &inputs,
            &hash_bytes(b"test", &[b"graph-where-validation"]),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let mut missing_where = cpu_reference_conformance_profile().unwrap();
        missing_where.passed_ops.remove("where");
        assert_eq!(
            verify_graph_execution_with_conformance_profile(GraphConformanceVerification {
                job: &job,
                receipt: &receipt,
                graph: &graph,
                tensors: &inputs,
                validation_seed: &hash_bytes(b"test", &[b"graph-where-validation"]),
                conformance_profile: &missing_where,
            }),
            Err(TvmError::InvalidReceipt(
                "graph op not conformance admitted"
            ))
        );
    }

    #[test]
    fn tensor_op_verifier_rejects_metadata_and_shape_mismatches() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let miner = address(b"miner");
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"validation"]);
        let params = FreivaldsParams::default();

        let mut bad_job = receipt.clone();
        bad_job.job_id = hash_bytes(b"test", &[b"other-job"]);
        assert_eq!(
            verify_tensor_op(&job, &bad_job, &a, &b, &c, &seed, &params),
            Err(TvmError::InvalidReceipt("job id mismatch"))
        );

        let mut bad_program = receipt.clone();
        bad_program.program_hash = hash_bytes(b"test", &[b"bad-program"]);
        bad_program.receipt_id = bad_program.recompute_receipt_id();
        bad_program.signature = sign(&bad_program.miner, &bad_program.receipt_id);
        assert_eq!(
            verify_tensor_op(&job, &bad_program, &a, &b, &c, &seed, &params),
            Err(TvmError::InvalidReceipt("program hash mismatch"))
        );

        let mut bad_inputs = receipt.clone();
        bad_inputs.input_roots[0] = hash_bytes(b"test", &[b"bad-input"]);
        bad_inputs.receipt_id = bad_inputs.recompute_receipt_id();
        bad_inputs.signature = sign(&bad_inputs.miner, &bad_inputs.receipt_id);
        assert_eq!(
            verify_tensor_op(&job, &bad_inputs, &a, &b, &c, &seed, &params),
            Err(TvmError::InvalidReceipt("input roots mismatch"))
        );

        let mut bad_outputs = receipt.clone();
        bad_outputs.output_roots[0] = hash_bytes(b"test", &[b"bad-output"]);
        bad_outputs.receipt_id = bad_outputs.recompute_receipt_id();
        bad_outputs.signature = sign(&bad_outputs.miner, &bad_outputs.receipt_id);
        assert_eq!(
            verify_tensor_op(&job, &bad_outputs, &a, &b, &c, &seed, &params),
            Err(TvmError::InvalidReceipt("output root mismatch"))
        );

        let wrong_a = Tensor::from_vec(
            vec![2, 4],
            DType::FieldElement,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        )
        .unwrap();
        let mut bad_input_shape = receipt.clone();
        bad_input_shape.input_roots = vec![wrong_a.commitment_root(), b.commitment_root()];
        bad_input_shape.receipt_id = bad_input_shape.recompute_receipt_id();
        bad_input_shape.signature = sign(&bad_input_shape.miner, &bad_input_shape.receipt_id);
        assert_eq!(
            verify_tensor_op(&job, &bad_input_shape, &wrong_a, &b, &c, &seed, &params),
            Err(TvmError::InvalidReceipt("input shape mismatch"))
        );

        let wrong_c = Tensor::from_vec(
            vec![4, 3],
            DType::FieldElement,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        )
        .unwrap();
        let mut bad_output_shape = receipt.clone();
        bad_output_shape.output_roots = vec![wrong_c.commitment_root()];
        bad_output_shape.receipt_id = bad_output_shape.recompute_receipt_id();
        bad_output_shape.signature = sign(&bad_output_shape.miner, &bad_output_shape.receipt_id);
        assert_eq!(
            verify_tensor_op(&job, &bad_output_shape, &a, &b, &wrong_c, &seed, &params),
            Err(TvmError::InvalidReceipt("output shape mismatch"))
        );

        assert!(row_sampled_freivalds(&a, &b, &c, &seed, 0).unwrap());
        assert_eq!(row_sample_detection_probability(0, 1, 1), 0.0);
        assert!(matches!(
            full_freivalds(&a, &b, &wrong_c, &seed, 1),
            Err(TvmError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn linear_training_verifier_rejects_sparse_weight_poisoning() {
        let seed = hash_bytes(b"test", &[b"batch"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: seed,
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 3,
            deadline_block: 10,
        });
        let (receipt, mut output) =
            LinearTrainingStepReceipt::from_job(&job, address(b"miner"), &weights, 1, 5).unwrap();
        let validation_seed = hash_bytes(b"test", &[b"validation"]);
        let report = verify_linear_training_step(
            &job,
            &receipt,
            &weights,
            &output,
            &validation_seed,
            &FreivaldsParams::default(),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Valid);

        output
            .weight_after
            .set2(0, 0, field::add(output.weight_after.get2(0, 0).unwrap(), 1))
            .unwrap();
        let mut bad_receipt = receipt.clone();
        bad_receipt.weight_root_after = output.weight_after.commitment_root();
        bad_receipt.trace_root = hash_bytes(b"test", &[b"sparse-weight-trace"]);
        bad_receipt.receipt_id = bad_receipt.recompute_receipt_id(&job.program_hash());
        bad_receipt.signature = sign(&bad_receipt.miner, &bad_receipt.receipt_id);
        let report = verify_linear_training_step(
            &job,
            &bad_receipt,
            &weights,
            &output,
            &validation_seed,
            &FreivaldsParams::default(),
        )
        .unwrap();
        assert_eq!(report.result, VerificationResult::Invalid);
        assert!(!report.optimizer_passed);
    }

    #[test]
    fn linear_training_verifier_requires_conformance_profile() {
        let seed = hash_bytes(b"test", &[b"batch"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: seed,
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 3,
            deadline_block: 10,
        });
        let (receipt, output) =
            LinearTrainingStepReceipt::from_job(&job, address(b"miner"), &weights, 1, 5).unwrap();
        let validation_seed = hash_bytes(b"test", &[b"validation"]);
        let params = FreivaldsParams::default();
        let report = verify_linear_training_step(
            &job,
            &receipt,
            &weights,
            &output,
            &validation_seed,
            &params,
        )
        .unwrap();
        assert_eq!(report.conformance_suite_hash, conformance_suite_hash());

        let empty = ConformanceProfile::empty_for_testing();
        assert_eq!(
            verify_linear_training_step_with_conformance_profile(LinearConformanceVerification {
                job: &job,
                receipt: &receipt,
                weights_before: &weights,
                output: &output,
                validation_seed: &validation_seed,
                params: &params,
                conformance_profile: &empty,
            }),
            Err(TvmError::InvalidReceipt("conformance suite unavailable"))
        );

        let mut missing_sub = cpu_reference_conformance_profile().unwrap();
        missing_sub.passed_ops.remove("sub");
        assert_eq!(
            verify_linear_training_step_with_conformance_profile(LinearConformanceVerification {
                job: &job,
                receipt: &receipt,
                weights_before: &weights,
                output: &output,
                validation_seed: &validation_seed,
                params: &params,
                conformance_profile: &missing_sub,
            }),
            Err(TvmError::InvalidReceipt("required op conformance missing"))
        );
    }

    #[test]
    fn linear_training_verifier_rejects_sparse_error_poisoning() {
        let seed = hash_bytes(b"test", &[b"batch"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: seed,
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 3,
            deadline_block: 10,
        });
        let (receipt, mut output) =
            LinearTrainingStepReceipt::from_job(&job, address(b"miner"), &weights, 1, 5).unwrap();
        output
            .dy
            .set2(0, 0, field::add(output.dy.get2(0, 0).unwrap(), 1))
            .unwrap();
        let mut bad_receipt = receipt.clone();
        bad_receipt.trace_root = hash_bytes(b"test", &[b"sparse-error-trace"]);
        bad_receipt.receipt_id = bad_receipt.recompute_receipt_id(&job.program_hash());
        bad_receipt.signature = sign(&bad_receipt.miner, &bad_receipt.receipt_id);

        let report = verify_linear_training_step(
            &job,
            &bad_receipt,
            &weights,
            &output,
            &hash_bytes(b"test", &[b"validation"]),
            &FreivaldsParams::default(),
        )
        .unwrap();

        assert_eq!(report.result, VerificationResult::Invalid);
        assert!(!report.error_relation_passed);
    }

    #[test]
    fn linear_training_verifier_rejects_metadata_and_commitment_mismatches() {
        let seed = hash_bytes(b"test", &[b"batch"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: seed,
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 3,
            deadline_block: 10,
        });
        let (receipt, output) =
            LinearTrainingStepReceipt::from_job(&job, address(b"miner"), &weights, 1, 5).unwrap();
        let validation_seed = hash_bytes(b"test", &[b"validation"]);
        let params = FreivaldsParams::default();

        let mut bad_job = receipt.clone();
        bad_job.job_id = hash_bytes(b"test", &[b"wrong-linear-job"]);
        assert_eq!(
            verify_linear_training_step(
                &job,
                &bad_job,
                &weights,
                &output,
                &validation_seed,
                &params
            ),
            Err(TvmError::InvalidReceipt("job id mismatch"))
        );

        let mut late =
            LinearTrainingStepReceipt::from_output(&job, receipt.miner, &weights, &output, 11, 5)
                .unwrap();
        assert_eq!(
            verify_linear_training_step(&job, &late, &weights, &output, &validation_seed, &params),
            Err(TvmError::InvalidReceipt("receipt submitted after deadline"))
        );

        late.submitted_at_block = 1;
        assert_eq!(
            verify_linear_training_step(&job, &late, &weights, &output, &validation_seed, &params),
            Err(TvmError::InvalidReceipt("receipt digest mismatch"))
        );

        let mut bad_signature = receipt.clone();
        bad_signature.signature = [7; 32];
        assert_eq!(
            verify_linear_training_step(
                &job,
                &bad_signature,
                &weights,
                &output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("bad receipt signature"))
        );

        let wrong_weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![6, 5, 4, 3, 2, 1]).unwrap();
        assert_eq!(
            verify_linear_training_step(
                &job,
                &receipt,
                &wrong_weights,
                &output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("weight root mismatch"))
        );

        let mut bad_output_root = receipt.clone();
        bad_output_root.y_root = hash_bytes(b"test", &[b"wrong-y"]);
        bad_output_root.receipt_id = bad_output_root.recompute_receipt_id(&job.program_hash());
        bad_output_root.signature = sign(&bad_output_root.miner, &bad_output_root.receipt_id);
        assert_eq!(
            verify_linear_training_step(
                &job,
                &bad_output_root,
                &weights,
                &output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("linear output root mismatch"))
        );

        let mut bad_batch_output = output.clone();
        bad_batch_output.x.set2(0, 0, 99).unwrap();
        assert_eq!(
            verify_linear_training_step(
                &job,
                &receipt,
                &weights,
                &bad_batch_output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("batch tensor mismatch"))
        );

        let mut bad_batch_root = receipt.clone();
        bad_batch_root.batch_root = hash_bytes(b"test", &[b"wrong-batch-root"]);
        bad_batch_root.receipt_id = bad_batch_root.recompute_receipt_id(&job.program_hash());
        bad_batch_root.signature = sign(&bad_batch_root.miner, &bad_batch_root.receipt_id);
        assert_eq!(
            verify_linear_training_step(
                &job,
                &bad_batch_root,
                &weights,
                &output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("batch root mismatch"))
        );

        let mut bad_trace = receipt.clone();
        bad_trace.trace_root = hash_bytes(b"test", &[b"wrong-linear-trace"]);
        bad_trace.receipt_id = bad_trace.recompute_receipt_id(&job.program_hash());
        bad_trace.signature = sign(&bad_trace.miner, &bad_trace.receipt_id);
        assert_eq!(
            verify_linear_training_step(
                &job,
                &bad_trace,
                &weights,
                &output,
                &validation_seed,
                &params,
            ),
            Err(TvmError::InvalidReceipt("trace root mismatch"))
        );

        let short = Tensor::from_vec(vec![1], DType::FieldElement, vec![1]).unwrap();
        assert!(matches!(
            random_linear_equal(&output.y, &short, &validation_seed),
            Err(TvmError::ShapeMismatch { .. })
        ));
    }

    #[test]
    fn attestation_signatures_verify() {
        let validator = address(b"validator");
        let att = ValidatorAttestation::new(
            validator,
            100,
            AttestationStatement {
                receipt_id: hash_bytes(b"test", &[b"receipt"]),
                job_id: hash_bytes(b"test", &[b"job"]),
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );
        assert!(att.verify_signature());
    }
}
