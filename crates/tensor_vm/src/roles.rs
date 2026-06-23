use crate::chain::{Chain, JobState, ReceiptState};
use crate::error::{Result, TvmError};
use crate::ir::TensorGraph;
use crate::jobs::{GraphJob, GraphReceipt, LinearTrainingStepOutput, PrimitiveType};
use crate::miner::MinerNode;
use crate::runtime::{CpuReferenceBackend, ExecutionBackend};
use crate::scheduler::SyntheticLocalJobSource;
use crate::tensor::Tensor;
use crate::types::{Address, Hash};
use crate::validator::{MatmulVerificationInput, ValidatorNode};
use crate::verify::{FreivaldsParams, ValidatorAttestation};

#[derive(Clone, Debug)]
pub enum RoleReceiptArtifacts {
    TensorOp {
        a: Tensor,
        b: Tensor,
        c: Tensor,
    },
    LinearTrainingStep {
        weights_before: Tensor,
        output: Box<LinearTrainingStepOutput>,
    },
    GraphExecution {
        graph: TensorGraph,
        inputs: std::collections::BTreeMap<String, Tensor>,
        const_blobs: std::collections::BTreeMap<String, Tensor>,
        outputs: std::collections::BTreeMap<String, Tensor>,
    },
}

#[derive(Clone, Debug)]
pub struct RoleReceiptBundle {
    pub receipt: ReceiptState,
    pub artifacts: RoleReceiptArtifacts,
}

#[derive(Clone, Copy)]
pub struct GraphJobExecution<'a> {
    pub job: &'a GraphJob,
    pub graph: &'a TensorGraph,
    pub inputs: &'a std::collections::BTreeMap<String, Tensor>,
    pub const_blobs: &'a std::collections::BTreeMap<String, Tensor>,
    pub submitted_at_block: u64,
    pub execution_time_ms: u64,
}

impl RoleReceiptBundle {
    pub fn receipt_id(&self) -> Hash {
        self.receipt.receipt_id()
    }

    pub fn served_tensors(&self) -> Vec<Tensor> {
        match &self.artifacts {
            RoleReceiptArtifacts::TensorOp { a, b, c } => vec![a.clone(), b.clone(), c.clone()],
            RoleReceiptArtifacts::LinearTrainingStep { output, .. } => vec![
                output.x.clone(),
                output.target.clone(),
                output.y.clone(),
                output.dy.clone(),
                output.grad_w.clone(),
                output.weight_after.clone(),
            ],
            RoleReceiptArtifacts::GraphExecution {
                inputs,
                const_blobs,
                outputs,
                ..
            } => inputs
                .values()
                .chain(const_blobs.values())
                .chain(outputs.values())
                .cloned()
                .collect::<Vec<_>>(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuReferenceMinerRole {
    pub address: Address,
}

impl CpuReferenceMinerRole {
    pub fn new(address: Address) -> Self {
        Self { address }
    }

    pub fn execute_job(
        &self,
        job: &JobState,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<RoleReceiptBundle> {
        execute_job_with_backend(
            self.address,
            CpuReferenceBackend,
            job,
            submitted_at_block,
            execution_time_ms,
        )
    }

    pub fn execute_graph_job(
        &self,
        job: &GraphJob,
        graph: &TensorGraph,
        inputs: &std::collections::BTreeMap<String, Tensor>,
        const_blobs: &std::collections::BTreeMap<String, Tensor>,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<RoleReceiptBundle> {
        execute_graph_job_with_backend(
            self.address,
            CpuReferenceBackend,
            GraphJobExecution {
                job,
                graph,
                inputs,
                const_blobs,
                submitted_at_block,
                execution_time_ms,
            },
        )
    }
}

pub fn execute_job_with_backend<B: ExecutionBackend>(
    address: Address,
    backend: B,
    job: &JobState,
    submitted_at_block: u64,
    execution_time_ms: u64,
) -> Result<RoleReceiptBundle> {
    let mut miner = MinerNode::new(address, backend);
    match job {
        JobState::TensorOp(job) => {
            let (receipt, a, b, c) =
                miner.solve_matmul_job(job, submitted_at_block, execution_time_ms)?;
            Ok(RoleReceiptBundle {
                receipt: ReceiptState::TensorOp(receipt),
                artifacts: RoleReceiptArtifacts::TensorOp { a, b, c },
            })
        }
        JobState::LinearTrainingStep(job) => {
            let weights_before = SyntheticLocalJobSource::linear_training_weights();
            let (receipt, output) = miner.solve_linear_training_step(
                job,
                &weights_before,
                submitted_at_block,
                execution_time_ms,
            )?;
            Ok(RoleReceiptBundle {
                receipt: ReceiptState::LinearTrainingStep(receipt),
                artifacts: RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output: Box::new(output),
                },
            })
        }
        JobState::GraphExecution(_) => Err(TvmError::InvalidReceipt(
            "graph execution requires explicit graph inputs",
        )),
    }
}

pub fn execute_graph_job_with_backend<B: ExecutionBackend>(
    address: Address,
    backend: B,
    execution: GraphJobExecution<'_>,
) -> Result<RoleReceiptBundle> {
    let ir_execution = backend.execute_graph_exact(
        execution.job,
        execution.graph,
        execution.inputs,
        execution.const_blobs,
    )?;
    let (receipt, outputs) = GraphReceipt::from_ir_execution(
        execution.job,
        address,
        ir_execution,
        execution.submitted_at_block,
        execution.execution_time_ms,
    )?;
    Ok(RoleReceiptBundle {
        receipt: ReceiptState::GraphExecution(receipt),
        artifacts: RoleReceiptArtifacts::GraphExecution {
            graph: execution.graph.clone(),
            inputs: execution.inputs.clone(),
            const_blobs: execution.const_blobs.clone(),
            outputs,
        },
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceValidatorRole {
    pub address: Address,
    pub stake: u64,
}

impl ReferenceValidatorRole {
    pub fn new(address: Address, stake: u64) -> Self {
        Self { address, stake }
    }

    pub fn verify_receipt(
        &self,
        job: &JobState,
        bundle: &RoleReceiptBundle,
        validation_seed: &Hash,
        params: &FreivaldsParams,
    ) -> Result<ValidatorAttestation> {
        let validator = ValidatorNode::new(self.address, self.stake);
        match (job, &bundle.receipt, &bundle.artifacts) {
            (
                JobState::TensorOp(job),
                ReceiptState::TensorOp(receipt),
                RoleReceiptArtifacts::TensorOp { a, b, c },
            ) => validator.verify_matmul(MatmulVerificationInput {
                job,
                receipt,
                a,
                b,
                c,
                validation_seed,
                params,
            }),
            (
                JobState::LinearTrainingStep(job),
                ReceiptState::LinearTrainingStep(receipt),
                RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output,
                },
            ) => validator.verify_linear_training_step(
                job,
                receipt,
                weights_before,
                output.as_ref(),
                validation_seed,
                params,
            ),
            (
                JobState::GraphExecution(job),
                ReceiptState::GraphExecution(receipt),
                RoleReceiptArtifacts::GraphExecution {
                    graph,
                    inputs,
                    const_blobs,
                    ..
                },
            ) => {
                let report = crate::verify::verify_graph_execution_with_const_blobs(
                    job,
                    receipt,
                    graph,
                    inputs,
                    const_blobs,
                    validation_seed,
                )?;
                Ok(ValidatorAttestation::new(
                    self.address,
                    self.stake,
                    crate::verify::AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::GraphExecution,
                        result: report.result,
                        checks_root: report.checks_root,
                        data_availability_passed: report.data_availability_passed,
                    },
                ))
            }
            _ => Err(TvmError::InvalidReceipt(
                "job and receipt primitive mismatch",
            )),
        }
    }
}

pub fn validator_stake(chain: &Chain, validator: &Address) -> u64 {
    chain
        .state()
        .validators()
        .get(validator)
        .map(|validator| validator.stake)
        .unwrap_or_default()
}

pub fn primitive_type(job: &JobState) -> PrimitiveType {
    match job {
        JobState::TensorOp(_) => PrimitiveType::TensorOp,
        JobState::LinearTrainingStep(_) => PrimitiveType::LinearTrainingStep,
        JobState::GraphExecution(_) => PrimitiveType::GraphExecution,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainParams;
    use crate::ir::{GraphOutput, IrRef, OpNode, TensorSpec, canonical_matmul_graph};
    use crate::jobs::{GraphJob, LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob};
    use crate::tensor::DType;
    use crate::types::{address, hash_bytes};
    use crate::verify::VerificationResult;
    use std::collections::BTreeMap;

    #[test]
    fn cpu_reference_miner_role_executes_tensor_op_jobs() {
        let beacon = hash_bytes(b"test", &[b"role-miner-matmul"]);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let miner = CpuReferenceMinerRole::new(address(b"role-miner"));

        let bundle = miner.execute_job(&job, 7, 11).unwrap();

        assert_eq!(primitive_type(&job), PrimitiveType::TensorOp);
        assert_eq!(bundle.served_tensors().len(), 3);
        assert!(matches!(bundle.receipt, ReceiptState::TensorOp(_)));
    }

    #[test]
    fn cpu_reference_miner_role_executes_linear_training_jobs() {
        let weights = SyntheticLocalJobSource::linear_training_weights();
        let job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"role-linear-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"role-linear-batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let miner = CpuReferenceMinerRole::new(address(b"role-linear-miner"));

        let bundle = miner.execute_job(&job, 3, 5).unwrap();

        assert_eq!(primitive_type(&job), PrimitiveType::LinearTrainingStep);
        assert_eq!(bundle.served_tensors().len(), 6);
        assert!(matches!(
            bundle.receipt,
            ReceiptState::LinearTrainingStep(_)
        ));
    }

    #[test]
    fn reference_validator_role_attests_matching_receipt_artifacts() {
        let params = ChainParams::default();
        let beacon = hash_bytes(b"test", &[b"role-validator"]);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let miner = CpuReferenceMinerRole::new(address(b"role-validator-miner"));
        let bundle = miner.execute_job(&job, 0, 1).unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-validator"), 10_000);

        let attestation = validator
            .verify_receipt(
                &job,
                &bundle,
                &hash_bytes(b"test", &[b"role-validator-seed"]),
                &params.freivalds,
            )
            .unwrap();

        assert_eq!(attestation.result, VerificationResult::Valid);
        assert_eq!(attestation.receipt_id, bundle.receipt_id());
        assert!(attestation.verify_signature());
    }

    #[test]
    fn cpu_roles_execute_and_verify_graph_jobs() {
        let params = ChainParams::default();
        let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
        let graph_id = graph.validate_for_consensus().unwrap();
        let a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
        let inputs = BTreeMap::from([("a".to_owned(), a.clone()), ("b".to_owned(), b.clone())]);
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);
        let miner = CpuReferenceMinerRole::new(address(b"role-graph-miner"));

        let bundle = miner
            .execute_graph_job(&job, &graph, &inputs, &BTreeMap::new(), 0, 2)
            .unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-graph-validator"), 10_000);
        let attestation = validator
            .verify_receipt(
                &JobState::GraphExecution(job.clone()),
                &bundle,
                &hash_bytes(b"test", &[b"role-graph-seed"]),
                &params.freivalds,
            )
            .unwrap();

        assert_eq!(
            primitive_type(&JobState::GraphExecution(job)),
            PrimitiveType::GraphExecution
        );
        assert_eq!(bundle.served_tensors().len(), 3);
        assert_eq!(attestation.result, VerificationResult::Valid);
        assert!(attestation.verify_signature());
    }

    #[test]
    fn cpu_roles_execute_and_verify_graph_jobs_with_const_blob() {
        let params = ChainParams::default();
        let input = Tensor::from_vec(vec![2], DType::FieldElement, vec![5, 6]).unwrap();
        let blob = Tensor::from_vec(vec![2], DType::FieldElement, vec![1, 2]).unwrap();
        let blob_uri = crate::hash::hex(&blob.commitment_root());
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec::field("x", vec![2])],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "add".to_owned(),
                args: vec![
                    IrRef::Input {
                        name: "x".to_owned(),
                    },
                    IrRef::ConstBlob {
                        uri: blob_uri.clone(),
                        shape: vec![2],
                        dtype: DType::FieldElement,
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("y", vec![2])],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let const_blobs = BTreeMap::from([(blob_uri, blob)]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 2);
        let miner = CpuReferenceMinerRole::new(address(b"role-graph-blob-miner"));

        let bundle = miner
            .execute_graph_job(&job, &graph, &inputs, &const_blobs, 0, 2)
            .unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-graph-blob-validator"), 10_000);
        let attestation = validator
            .verify_receipt(
                &JobState::GraphExecution(job),
                &bundle,
                &hash_bytes(b"test", &[b"role-graph-blob-seed"]),
                &params.freivalds,
            )
            .unwrap();

        assert_eq!(bundle.served_tensors().len(), 3);
        assert_eq!(attestation.result, VerificationResult::Valid);
        assert!(attestation.verify_signature());
    }

    #[test]
    fn reference_validator_role_rejects_mismatched_artifacts() {
        let params = ChainParams::default();
        let beacon = hash_bytes(b"test", &[b"role-validator-mismatch"]);
        let matmul_job = JobState::TensorOp(MatmulJob::synthetic(0, 0, 2, 3, 4, &beacon, 10));
        let linear_job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"mismatch-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"mismatch-batch"]),
                weight_root_before: SyntheticLocalJobSource::linear_training_weights()
                    .commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let miner = CpuReferenceMinerRole::new(address(b"role-mismatch-miner"));
        let bundle = miner.execute_job(&matmul_job, 0, 1).unwrap();
        let validator = ReferenceValidatorRole::new(address(b"role-mismatch-validator"), 10_000);

        let error = validator
            .verify_receipt(
                &linear_job,
                &bundle,
                &hash_bytes(b"test", &[b"role-mismatch-seed"]),
                &params.freivalds,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TvmError::InvalidReceipt("job and receipt primitive mismatch")
        );
    }

    #[test]
    fn validator_stake_defaults_to_zero_for_unknown_validator() {
        let chain = Chain::new(hash_bytes(b"test", &[b"role-stake"]));

        assert_eq!(validator_stake(&chain, &address(b"missing-validator")), 0);
    }

    #[test]
    fn linear_training_role_artifacts_expose_weight_before() {
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = JobState::LinearTrainingStep(LinearTrainingStepJob::from_spec(
            LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"role-artifact-model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"role-artifact-batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            },
        ));
        let bundle = CpuReferenceMinerRole::new(address(b"role-artifact-miner"))
            .execute_job(&job, 0, 1)
            .unwrap();

        let RoleReceiptArtifacts::LinearTrainingStep { weights_before, .. } = bundle.artifacts
        else {
            panic!("linear job must return linear artifacts");
        };
        assert_eq!(weights_before.commitment_root(), weights.commitment_root());
    }
}
