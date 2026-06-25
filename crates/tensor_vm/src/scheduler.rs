use crate::chain::{Chain, JobState};
use crate::ir::{GraphOutput, IrRef, OpNode, TensorGraph, TensorSpec};
use crate::jobs::{GraphJob, LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob};
use crate::tensor::{DType, Tensor};
use crate::types::{Address, Hash, hash_bytes, hash_to_u128};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAssignment {
    pub receipt_id: Hash,
    pub validators: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerAssignment {
    pub job_id: Hash,
    pub miners: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobScheduler {
    pub small_matmul: (usize, usize, usize),
    pub medium_matmul: (usize, usize, usize),
}

pub trait JobSource {
    fn next_job(&mut self, chain: &Chain) -> Option<JobState>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticLocalJobSource {
    scheduler: JobScheduler,
}

impl SyntheticLocalJobSource {
    pub fn new(scheduler: JobScheduler) -> Self {
        Self { scheduler }
    }

    pub fn next_matmul_job(&mut self, chain: &Chain) -> MatmulJob {
        self.scheduler.generate_small_matmul(
            chain.state().epoch(),
            chain.state().height(),
            &chain.state().finalized_randomness(),
            chain
                .state()
                .height()
                .saturating_add(chain.params().receipt_submission_window),
        )
    }

    pub fn next_linear_training_job(&mut self, chain: &Chain) -> LinearTrainingStepJob {
        let weights = Self::linear_training_weights();
        let height = chain.state().height().to_le_bytes();
        LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(
                b"tensor-vm-local-linear-model-v1",
                &[&chain.state().finalized_randomness(), &height],
            ),
            step: 0,
            batch_seed: hash_bytes(
                b"tensor-vm-local-linear-batch-v1",
                &[&chain.state().finalized_randomness(), &height],
            ),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: chain
                .state()
                .height()
                .saturating_add(chain.params().receipt_submission_window),
        })
    }

    pub fn next_graph_job(&mut self, chain: &Chain) -> GraphJob {
        let graph = Self::graph_execution_graph();
        let inputs = Self::graph_execution_inputs();
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        GraphJob::new(
            chain.state().epoch(),
            graph.graph_id(),
            input_roots,
            BTreeMap::new(),
            chain
                .state()
                .height()
                .saturating_add(chain.params().receipt_submission_window),
            1,
            12,
        )
    }

    /// Monotonic-nonce job production for the live synthetic producer. Unlike the
    /// height-keyed `next_job`, the job's uniqueness is keyed on `nonce` (a value
    /// the producer advances every round, e.g. the chain job count), so the
    /// producer keeps minting distinct jobs and never stalls when the chain head
    /// is briefly static (the height-keyed path would re-emit the same job id and
    /// freeze the work pipeline). Existing height-keyed `next_*_job` are unchanged.
    pub fn next_job_with_nonce(&mut self, chain: &Chain, nonce: u64) -> Option<JobState> {
        match nonce % 3 {
            0 => Some(JobState::TensorOp(self.matmul_job_for_nonce(chain, nonce))),
            1 => Some(JobState::LinearTrainingStep(
                self.linear_job_for_nonce(chain, nonce),
            )),
            _ => Some(JobState::GraphExecution(
                self.graph_job_for_nonce(chain, nonce),
            )),
        }
    }

    fn synthetic_deadline(chain: &Chain) -> u64 {
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window)
    }

    fn matmul_job_for_nonce(&self, chain: &Chain, nonce: u64) -> MatmulJob {
        self.scheduler.generate_small_matmul(
            chain.state().epoch(),
            nonce,
            &chain.state().finalized_randomness(),
            Self::synthetic_deadline(chain),
        )
    }

    fn linear_job_for_nonce(&self, chain: &Chain, nonce: u64) -> LinearTrainingStepJob {
        let weights = Self::linear_training_weights();
        let nonce_bytes = nonce.to_le_bytes();
        LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(
                b"tensor-vm-local-linear-model-v1",
                &[&chain.state().finalized_randomness(), &nonce_bytes],
            ),
            step: 0,
            batch_seed: hash_bytes(
                b"tensor-vm-local-linear-batch-v1",
                &[&chain.state().finalized_randomness(), &nonce_bytes],
            ),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: Self::synthetic_deadline(chain),
        })
    }

    fn graph_job_for_nonce(&self, chain: &Chain, nonce: u64) -> GraphJob {
        let graph = Self::graph_execution_graph();
        let inputs = Self::graph_execution_inputs();
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        // The graph body and inputs are fixed (so every node agrees on them); the
        // job's uniqueness comes from `nonce` carried in the epoch field, which the
        // chain does not validate against the live epoch for graph jobs.
        GraphJob::new(
            nonce,
            graph.graph_id(),
            input_roots,
            BTreeMap::new(),
            Self::synthetic_deadline(chain),
            1,
            12,
        )
    }

    pub fn graph_execution_graph() -> TensorGraph {
        TensorGraph {
            ir_version: 1,
            inputs: vec![
                TensorSpec::field("a", vec![2, 3]),
                TensorSpec::field("b", vec![2, 3]),
            ],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "add".to_owned(),
                    args: vec![
                        IrRef::Input {
                            name: "a".to_owned(),
                        },
                        IrRef::Input {
                            name: "b".to_owned(),
                        },
                    ],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec::field("sum", vec![2, 3])],
                },
                OpNode {
                    id: 1,
                    op: "relu".to_owned(),
                    args: vec![IrRef::Op { id: 0, idx: 0 }],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec::field("activated", vec![2, 3])],
                },
            ],
            outputs: vec![GraphOutput {
                name: "activated".to_owned(),
                value: IrRef::Op { id: 1, idx: 0 },
            }],
        }
    }

    pub fn graph_execution_inputs() -> BTreeMap<String, Tensor> {
        BTreeMap::from([
            (
                "a".to_owned(),
                Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
                    .expect("static synthetic graph input a must be valid"),
            ),
            (
                "b".to_owned(),
                Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![7, 8, 9, 10, 11, 12])
                    .expect("static synthetic graph input b must be valid"),
            ),
        ])
    }

    pub fn linear_training_weights() -> Tensor {
        Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static synthetic linear weights must be valid")
    }

    pub fn linear_training_architecture_hash() -> Hash {
        hash_bytes(b"tensor-vm-local-linear-architecture-v1", &[])
    }

    pub fn linear_training_config_hash() -> Hash {
        hash_bytes(b"tensor-vm-local-linear-config-v1", &[])
    }
}

impl Default for SyntheticLocalJobSource {
    fn default() -> Self {
        Self::new(JobScheduler::with_small_shape((8, 8, 8)))
    }
}

impl JobSource for SyntheticLocalJobSource {
    fn next_job(&mut self, chain: &Chain) -> Option<JobState> {
        match chain.state().height() % 3 {
            0 => Some(JobState::TensorOp(self.next_matmul_job(chain))),
            1 => Some(JobState::LinearTrainingStep(
                self.next_linear_training_job(chain),
            )),
            _ => Some(JobState::GraphExecution(self.next_graph_job(chain))),
        }
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self {
            small_matmul: (1024, 1024, 1024),
            medium_matmul: (4096, 4096, 4096),
        }
    }
}

impl JobScheduler {
    pub fn with_small_shape(shape: (usize, usize, usize)) -> Self {
        Self {
            small_matmul: shape,
            ..Self::default()
        }
    }

    pub fn generate_small_matmul(
        &self,
        epoch: u64,
        nonce: u64,
        beacon: &Hash,
        deadline_block: u64,
    ) -> MatmulJob {
        let (m, k, n) = self.small_matmul;
        MatmulJob::synthetic(epoch, nonce, m, k, n, beacon, deadline_block)
    }

    pub fn assign_validators(
        &self,
        chain: &Chain,
        receipt_id: Hash,
        seed: &Hash,
    ) -> ValidatorAssignment {
        let mut validators: Vec<_> = chain.state().validators().keys().copied().collect();
        validators.sort_by_key(|validator| {
            let draw = hash_bytes(
                b"tensor-vm-validator-assignment-v1",
                &[seed, &receipt_id, validator],
            );
            hash_to_u128(&draw)
        });
        validators.truncate(
            chain
                .params()
                .freivalds
                .validators_per_job
                .min(validators.len()),
        );
        ValidatorAssignment {
            receipt_id,
            validators,
        }
    }

    pub fn assign_miners(&self, chain: &Chain, job_id: Hash, seed: &Hash) -> MinerAssignment {
        let mut candidates: Vec<_> = chain.state().miners().values().collect();
        candidates.sort_by_key(|miner| {
            let draw = hash_bytes(
                b"tensor-vm-miner-assignment-v1",
                &[seed, &job_id, &miner.address],
            );
            hash_to_u128(&draw)
        });

        let target = chain.params().replication_factor.min(candidates.len());
        let mut miners = Vec::with_capacity(target);
        let mut selected_addresses = BTreeSet::new();
        let mut selected_operators = BTreeSet::new();

        for miner in &candidates {
            if miners.len() == target {
                break;
            }
            if selected_operators.insert(miner.operator_id) {
                selected_addresses.insert(miner.address);
                miners.push(miner.address);
            }
        }

        for miner in &candidates {
            if miners.len() == target {
                break;
            }
            if selected_addresses.insert(miner.address) {
                miners.push(miner.address);
            }
        }

        MinerAssignment { job_id, miners }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{ChainCommand, ChainEngine};
    use crate::types::{address, hash_bytes};

    fn register_validator(chain: &mut Chain, validator: Address) {
        let stake = chain.params().validator_min_stake;
        chain
            .apply_command(ChainCommand::RegisterValidator {
                address: validator,
                stake,
            })
            .unwrap();
    }

    fn register_miner(chain: &mut Chain, miner: Address) {
        let stake = chain.params().miner_min_stake;
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake,
            })
            .unwrap();
    }

    #[test]
    fn scheduler_generates_deterministic_jobs() {
        let scheduler = JobScheduler::with_small_shape((4, 5, 6));
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let a = scheduler.generate_small_matmul(1, 2, &beacon, 10);
        let b = scheduler.generate_small_matmul(1, 2, &beacon, 10);
        assert_eq!(a.job_id, b.job_id);
        assert_eq!((a.m, a.k, a.n), (4, 5, 6));
    }

    #[test]
    fn synthetic_job_source_uses_chain_epoch_height_and_deadline() {
        let beacon = hash_bytes(b"test", &[b"synthetic-source"]);
        let mut chain = Chain::new(beacon);
        chain.set_position_for_testing(12, 7);
        chain.set_receipt_submission_window_for_testing(13);
        let mut source = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((2, 3, 4)));

        let Some(JobState::TensorOp(job)) = source.next_job(&chain) else {
            panic!("synthetic local job source must emit TensorOp jobs");
        };

        assert_eq!(job.epoch, 7);
        assert_eq!((job.m, job.k, job.n), (2, 3, 4));
        assert_eq!(job.deadline_block, 25);
        assert_eq!(
            job.job_id,
            JobScheduler::with_small_shape((2, 3, 4))
                .generate_small_matmul(7, 12, &beacon, 25)
                .job_id
        );
    }

    #[test]
    fn synthetic_job_source_default_matches_local_cpu_profile_shape() {
        let beacon = hash_bytes(b"test", &[b"synthetic-default-source"]);
        let chain = Chain::new(beacon);
        let mut source = SyntheticLocalJobSource::default();

        let Some(JobState::TensorOp(job)) = source.next_job(&chain) else {
            panic!("default synthetic local job source must emit TensorOp jobs first");
        };

        assert_eq!((job.m, job.k, job.n), (8, 8, 8));
    }

    #[test]
    fn synthetic_job_source_emits_linear_training_steps_on_second_slot() {
        let beacon = hash_bytes(b"test", &[b"synthetic-linear-source"]);
        let mut chain = Chain::new(beacon);
        chain.set_position_for_testing(13, 3);
        chain.set_receipt_submission_window_for_testing(13);
        let mut source = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((2, 3, 4)));
        let weights = SyntheticLocalJobSource::linear_training_weights();

        let Some(JobState::LinearTrainingStep(job)) = source.next_job(&chain) else {
            panic!("synthetic local job source must emit LinearTrainingStep jobs on odd heights");
        };

        assert_eq!(job.step, 0);
        assert_eq!(job.weight_root_before, weights.commitment_root());
        assert_eq!(job.input_shape, vec![4, 3]);
        assert_eq!(job.weight_shape, vec![3, 2]);
        assert_eq!(job.target_shape, vec![4, 2]);
        assert_eq!(job.deadline_block, 26);
        assert_eq!(
            job.model_id,
            hash_bytes(
                b"tensor-vm-local-linear-model-v1",
                &[&beacon, &13u64.to_le_bytes()]
            )
        );
    }

    #[test]
    fn synthetic_job_source_emits_graph_jobs_on_third_slot() {
        let beacon = hash_bytes(b"test", &[b"synthetic-graph-source"]);
        let mut chain = Chain::new(beacon);
        chain.set_position_for_testing(14, 4);
        chain.set_receipt_submission_window_for_testing(13);
        let mut source = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((2, 3, 4)));
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let inputs = SyntheticLocalJobSource::graph_execution_inputs();

        let Some(JobState::GraphExecution(job)) = source.next_job(&chain) else {
            panic!("synthetic local job source must emit GraphExecution jobs on third slots");
        };

        assert_eq!(job.epoch, 4);
        assert_eq!(job.graph_id, graph.validate_for_consensus().unwrap());
        assert_eq!(job.deadline_block, 27);
        for (name, tensor) in inputs {
            assert_eq!(job.input_roots.get(&name), Some(&tensor.commitment_root()));
        }
    }

    #[test]
    fn validator_assignment_is_deterministic_and_bounded() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        for i in 0..12 {
            register_validator(&mut chain, address(format!("validator-{i}").as_bytes()));
        }
        let scheduler = JobScheduler::default();
        let receipt = hash_bytes(b"test", &[b"receipt"]);
        let first = scheduler.assign_validators(&chain, receipt, &beacon);
        let second = scheduler.assign_validators(&chain, receipt, &beacon);
        assert_eq!(first, second);
        assert_eq!(
            first.validators.len(),
            chain.params().freivalds.validators_per_job
        );
        assert_eq!(first.receipt_id, receipt);
    }

    #[test]
    fn validator_assignment_is_bound_to_receipt_id() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        chain.set_validators_per_job_for_testing(4);
        for i in 0..32 {
            register_validator(&mut chain, address(format!("validator-{i}").as_bytes()));
        }
        let scheduler = JobScheduler::default();
        let first =
            scheduler.assign_validators(&chain, hash_bytes(b"test", &[b"receipt-0"]), &beacon);
        let mut receipt_bound_assignment = None;
        for i in 1..100 {
            let receipt = hash_bytes(b"test", &[format!("receipt-{i}").as_bytes()]);
            let assignment = scheduler.assign_validators(&chain, receipt, &beacon);
            if assignment.validators != first.validators {
                receipt_bound_assignment = Some(assignment);
                break;
            }
        }

        assert!(
            receipt_bound_assignment.is_some(),
            "receipt id must affect validator assignment ordering"
        );
    }

    #[test]
    fn miner_assignment_uses_replication_factor() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        for i in 0..10 {
            register_miner(&mut chain, address(format!("miner-{i}").as_bytes()));
        }
        chain.set_replication_factor_for_testing(4);
        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"job"]);
        let first = scheduler.assign_miners(&chain, job, &beacon);
        let second = scheduler.assign_miners(&chain, job, &beacon);
        assert_eq!(first, second);
        assert_eq!(first.miners.len(), 4);
    }

    #[test]
    fn miner_assignment_prefers_operator_separation() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let shared_operator = address(b"shared-operator");
        for i in 0..4 {
            chain
                .register_miner_with_operator(
                    address(format!("shared-miner-{i}").as_bytes()),
                    100,
                    shared_operator,
                )
                .unwrap();
        }
        for i in 0..4 {
            chain
                .register_miner_with_operator(
                    address(format!("distinct-miner-{i}").as_bytes()),
                    100,
                    address(format!("distinct-operator-{i}").as_bytes()),
                )
                .unwrap();
        }
        chain.set_replication_factor_for_testing(4);

        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"operator-separated-job"]);
        let assignment = scheduler.assign_miners(&chain, job, &beacon);
        let operators: BTreeSet<_> = assignment
            .miners
            .iter()
            .map(|miner| chain.state().miners().get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 4);
        assert_eq!(operators.len(), 4);
    }

    #[test]
    fn miner_assignment_falls_back_when_operator_diversity_is_insufficient() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let shared_operator = address(b"only-operator");
        for i in 0..5 {
            chain
                .register_miner_with_operator(
                    address(format!("same-operator-miner-{i}").as_bytes()),
                    100,
                    shared_operator,
                )
                .unwrap();
        }
        chain.set_replication_factor_for_testing(3);

        let scheduler = JobScheduler::default();
        let job = hash_bytes(b"test", &[b"fallback-job"]);
        let assignment = scheduler.assign_miners(&chain, job, &beacon);
        let operators: BTreeSet<_> = assignment
            .miners
            .iter()
            .map(|miner| chain.state().miners().get(miner).unwrap().operator_id)
            .collect();

        assert_eq!(assignment.miners.len(), 3);
        assert_eq!(operators.len(), 1);
        assert_eq!(operators.into_iter().next(), Some(shared_operator));
    }
}
