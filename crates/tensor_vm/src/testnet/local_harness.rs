use super::{
    PublicNodeRole, PublicTestnetCriteria, PublicTestnetEvidence, TestnetConfig,
    ratio_parts_to_bps, ratio_to_bps, required_blocks_for_days, required_duration_seconds_for_days,
};
use crate::ExplorerSummary;
use crate::chain::{
    BlockVote, Chain, ChainCommand, ChainEngine, ChainParams, JobState, ReceiptState, TensorBlock,
    Transaction,
};
use crate::faucet::Faucet;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec, MatmulJob, TensorOpReceipt};
use crate::miner::MinerNode;
use crate::profile::ChainProfile;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::JobScheduler;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DType, Tensor};
use crate::tensor_server::TensorServer;
use crate::txpool::TxPool;
use crate::types::{Address, Hash, address, hash_bytes};
use crate::validator::ValidatorNode;
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: Chain,
    pub faucet: Faucet,
    pub miners: Vec<Address>,
    pub validators: Vec<Address>,
    pub participant_endpoints: Vec<LocalParticipantEndpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalParticipantEndpoint {
    pub role: PublicNodeRole,
    pub address: Address,
    pub operator_id: Hash,
    pub node_endpoint: String,
}

impl LocalParticipantEndpoint {
    pub fn has_mandatory_libp2p_node_path(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && local_libp2p_multiaddr_has_tcp_node_path(&self.node_endpoint)
    }
}

fn local_participant_tcp_port(base: u16, index: usize) -> u16 {
    base.saturating_add(u16::try_from(index).unwrap_or(u16::MAX.saturating_sub(base)))
}

pub(super) fn local_libp2p_multiaddr_has_tcp_node_path(endpoint: &str) -> bool {
    let Ok(address) = endpoint.parse::<Multiaddr>() else {
        return false;
    };
    let mut has_node_address = false;
    let mut has_tcp_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(_)
            | Protocol::Ip6(_)
            | Protocol::Dns(_)
            | Protocol::Dns4(_)
            | Protocol::Dns6(_) => has_node_address = true,
            Protocol::Tcp(port) if port != 0 => has_tcp_port = true,
            Protocol::Tcp(_) => return false,
            _ => {}
        }
    }
    has_node_address && has_tcp_port
}

impl LocalTestnet {
    pub fn new(config: TestnetConfig, finalized_randomness: Hash) -> Self {
        Self::with_chain_params(config, ChainParams::default(), finalized_randomness)
    }

    pub fn from_profile(profile: &ChainProfile, finalized_randomness: Hash) -> Self {
        Self::with_chain_params(
            TestnetConfig::from_profile(profile),
            profile.chain_params.clone(),
            finalized_randomness,
        )
    }

    pub fn with_chain_params(
        config: TestnetConfig,
        params: ChainParams,
        finalized_randomness: Hash,
    ) -> Self {
        let mut chain = Chain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        let mut participant_endpoints =
            Vec::with_capacity(config.miner_count + config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain
                .apply_command(ChainCommand::RegisterMiner {
                    address: miner,
                    stake: config.miner_stake,
                })
                .unwrap();
            miners.push(miner);
            let index = (i as u64).to_le_bytes();
            participant_endpoints.push(LocalParticipantEndpoint {
                role: PublicNodeRole::Miner,
                address: miner,
                operator_id: hash_bytes(b"tensor-vm-local-operator-v1", &[b"miner", &index]),
                node_endpoint: format!(
                    "/ip4/127.0.0.1/tcp/{}",
                    local_participant_tcp_port(4_001, i)
                ),
            });
        }
        for i in 0..config.validator_count {
            let validator = address(format!("testnet-validator-{i}").as_bytes());
            chain
                .apply_command(ChainCommand::RegisterValidator {
                    address: validator,
                    stake: config.validator_stake,
                })
                .unwrap();
            validators.push(validator);
            let index = (i as u64).to_le_bytes();
            participant_endpoints.push(LocalParticipantEndpoint {
                role: PublicNodeRole::Validator,
                address: validator,
                operator_id: hash_bytes(b"tensor-vm-local-operator-v1", &[b"validator", &index]),
                node_endpoint: format!(
                    "/ip4/127.0.0.1/tcp/{}",
                    local_participant_tcp_port(5_001, i)
                ),
            });
        }
        Self {
            chain,
            faucet: Faucet::new(config.faucet_balance, config.faucet_drip),
            miners,
            validators,
            participant_endpoints,
        }
    }

    pub fn has_mandatory_libp2p_participant_paths(&self) -> bool {
        if self.participant_endpoints.len() != self.miners.len() + self.validators.len() {
            return false;
        }
        let mut node_endpoints = BTreeSet::new();
        let mut operator_ids = BTreeSet::new();
        let mut miner_endpoints = 0;
        let mut validator_endpoints = 0;
        for participant in &self.participant_endpoints {
            if !participant.has_mandatory_libp2p_node_path()
                || !node_endpoints.insert(participant.node_endpoint.clone())
                || !operator_ids.insert(participant.operator_id)
            {
                return false;
            }
            match participant.role {
                PublicNodeRole::Miner => miner_endpoints += 1,
                PublicNodeRole::Validator => validator_endpoints += 1,
            }
        }
        miner_endpoints == self.miners.len() && validator_endpoints == self.validators.len()
    }

    pub fn run_blocks(&mut self, count: u64) {
        for i in 0..count {
            let beacon = self.chain.state().finalized_randomness();
            let proposer = self
                .chain
                .proposer_for_next_epoch(&beacon)
                .or_else(|| self.validators.first().copied())
                .unwrap_or([0; 32]);
            let timestamp = i.saturating_mul(self.chain.params().block_time_seconds);
            let block = self.produce_block_with_command(proposer, timestamp);
            self.finalize_block(&block);
        }
    }

    pub fn run_matmul_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state().finalized_randomness();
        let job = scheduler.generate_small_matmul(
            self.chain.state().epoch(),
            self.chain.state().height(),
            &beacon,
            self.chain.state().height() + self.chain.params().receipt_submission_window,
        );
        let mut txpool = TxPool::default();
        self.chain
            .apply_command(ChainCommand::SubmitJob(JobState::TensorOp(job.clone())))
            .expect("generated tensor job should be accepted");
        let miner_assignment_seed = self.chain.miner_assignment_seed(&job.job_id);
        let miner_assignment =
            scheduler.assign_miners(&self.chain, job.job_id, &miner_assignment_seed);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, _a, _b, _c) = miner
                .solve_matmul_job(&job, self.chain.state().height(), 1 + index as u64)
                .expect("reference miner should solve generated job");
            assert!(txpool.submit(Transaction::SubmitTensorOpReceipt(receipt.receipt_id)));
            self.chain
                .apply_command(ChainCommand::SubmitReceipt(ReceiptState::TensorOp(
                    receipt.clone(),
                )))
                .expect("registered miner receipt should be accepted");
            receipts.push((receipt, miner.tensor_server.clone()));
        }

        self.attest_matmul_receipts(scheduler, &job, &receipts, &beacon, &mut txpool);

        self.chain
            .apply_command(ChainCommand::SettleEpoch {
                miner_reward_pool: 1_000,
                validator_reward_pool: 500,
            })
            .expect("verified receipts should settle");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.validators[0]);
        let block = self.produce_block_with_command(
            proposer,
            self.chain.state().height() * self.chain.params().block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn run_linear_training_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state().finalized_randomness();
        let model_id = hash_bytes(b"tensor-vm-testnet-model-v1", &[&beacon]);
        let architecture = hash_bytes(b"tensor-vm-testnet-architecture-v1", &[]);
        let config = hash_bytes(b"tensor-vm-testnet-config-v1", &[]);
        let weights = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static weights should be valid");
        self.chain
            .apply_command(ChainCommand::RegisterModel {
                model_id,
                architecture_hash: architecture,
                weight_root: weights.commitment_root(),
                config_hash: config,
            })
            .expect("testnet linear model should be registered");
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-testnet-batch-v1", &[&beacon]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: self.chain.state().height()
                + self.chain.params().receipt_submission_window,
        });
        let mut txpool = TxPool::default();
        self.chain
            .apply_command(ChainCommand::SubmitJob(JobState::LinearTrainingStep(
                job.clone(),
            )))
            .expect("generated linear training job should be accepted");
        let miner_assignment_seed = self.chain.miner_assignment_seed(&job.job_id);
        let miner_assignment =
            scheduler.assign_miners(&self.chain, job.job_id, &miner_assignment_seed);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, output) = miner
                .solve_linear_training_step(
                    &job,
                    &weights,
                    self.chain.state().height(),
                    1 + index as u64,
                )
                .expect("reference miner should solve generated training step");
            assert!(txpool.submit(Transaction::SubmitLinearTrainingStepReceipt(
                receipt.receipt_id
            )));
            self.chain
                .apply_command(ChainCommand::SubmitReceipt(
                    ReceiptState::LinearTrainingStep(receipt.clone()),
                ))
                .expect("registered miner linear receipt should be accepted");
            receipts.push((receipt, output));
        }

        for (receipt, output) in &receipts {
            let assignment_seed = self.chain.validator_assignment_seed(&receipt.receipt_id);
            let assignment =
                scheduler.assign_validators(&self.chain, receipt.receipt_id, &assignment_seed);
            for validator_address in assignment.validators {
                let validation_seed = self
                    .chain
                    .validation_seed(&receipt.receipt_id, &validator_address);
                let stake = self
                    .chain
                    .state()
                    .validators()
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_linear_training_step(
                        &job,
                        receipt,
                        &weights,
                        output,
                        &validation_seed,
                        &self.chain.params().freivalds,
                    )
                    .expect("reference validator should verify generated training step");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .apply_command(ChainCommand::SubmitAttestation(attestation))
                    .expect("registered validator attestation should be accepted");
            }
        }

        let canonical_receipt = &receipts[0].0;
        assert!(
            self.chain
                .has_attestation_quorum(&canonical_receipt.receipt_id)
        );
        assert!(
            self.chain
                .has_redundant_agreement(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_command(ChainCommand::SettleEpoch {
                miner_reward_pool: 1_000,
                validator_reward_pool: 500,
            })
            .expect("verified linear receipts should settle");
        assert!(
            self.chain
                .state()
                .settled_receipts()
                .contains(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_command(ChainCommand::ApplyModelTransition {
                model_id,
                step: 0,
                weight_root_before: weights.commitment_root(),
                weight_root_after: canonical_receipt.weight_root_after,
            })
            .expect("verified training receipt should advance model state");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.validators[0]);
        let block = self.produce_block_with_command(
            proposer,
            self.chain.state().height() * self.chain.params().block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn expected_blocks_for_days(&self, days: u64) -> u64 {
        required_blocks_for_days(days, self.chain.params().block_time_seconds.max(1))
    }

    pub fn telemetry(&self) -> TelemetrySnapshot {
        TelemetrySnapshot::from_chain(&self.chain)
    }

    pub fn explorer_summary(&self) -> ExplorerSummary {
        let state = self.chain.state();
        ExplorerSummary {
            height: state.height(),
            epoch: state.epoch(),
            block_count: self.chain.blocks().len(),
            miner_count: state.miners().len(),
            validator_count: state.validators().len(),
            job_count: state.jobs().len(),
            model_count: state.model_states().len(),
            model_step_total: state.model_states().values().map(|model| model.step).sum(),
            attestation_count: state.attestations().values().map(Vec::len).sum(),
            receipt_count: state.receipts().len(),
            settled_receipt_count: state.settled_receipts().len(),
            data_unavailable_receipt_count: state.data_unavailable_receipts().len(),
            data_unavailability_slash_count: state.data_unavailability_slashes().len(),
            data_unavailability_slashed_amount_total: state
                .data_unavailability_slashes()
                .values()
                .map(|slash| slash.amount)
                .sum(),
            validator_audit_assignment_count: state.validator_audit_assignments().len(),
            validator_audit_result_count: state.validator_audit_results().len(),
            validator_audit_slash_count: state.validator_audit_slashes().len(),
            validator_audit_slashed_amount_total: state
                .validator_audit_slashes()
                .values()
                .map(|slash| slash.amount)
                .sum(),
            finalized_block_count: state.finalized_blocks().len(),
            treasury_balance: state.rewards().treasury(),
            pending_receipt_reward_count: state.pending_receipt_rewards().len(),
            pending_proposer_reward_count: state.pending_proposer_rewards().len(),
            pending_challenge_reward_count: state.pending_challenge_rewards().len(),
            pending_credit_reward_count: state.pending_credit_rewards().len(),
            total_reward_balance: state.rewards().total_balance(),
        }
    }

    pub fn public_testnet_evidence(
        &self,
        criteria: &PublicTestnetCriteria,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let telemetry = self.telemetry();
        let required_blocks = self.expected_blocks_for_days(criteria.duration_days);
        let required_duration_seconds = required_duration_seconds_for_days(criteria.duration_days);
        let observed_blocks = self.chain.blocks().len() as u64;
        let run_started_at_unix_seconds = self
            .chain
            .blocks()
            .first()
            .map(|block| block.timestamp)
            .unwrap_or(0);
        let run_ended_at_unix_seconds = self
            .chain
            .blocks()
            .last()
            .map(|block| {
                block
                    .timestamp
                    .saturating_add(self.chain.params().block_time_seconds)
            })
            .unwrap_or(run_started_at_unix_seconds);
        let observed_duration_seconds =
            run_ended_at_unix_seconds.saturating_sub(run_started_at_unix_seconds);
        let finality_rate_bps = ratio_to_bps(telemetry.block_finality_rate);
        let data_availability_bps = ratio_to_bps(telemetry.data_availability_rate);
        let invalid_receipts_submitted = telemetry.invalid_receipts_submitted as u64;
        let invalid_receipts_rejected =
            invalid_receipts_submitted.saturating_sub(telemetry.invalid_receipts_accepted);
        let invalid_work_rejection_rate_bps =
            ratio_parts_to_bps(invalid_receipts_rejected, invalid_receipts_submitted);
        let reward_settlement_records = telemetry.settled_receipt_count as u64;
        let has_required_miners = self.miners.len() >= criteria.min_miners;
        let has_required_validators = self.validators.len() >= criteria.min_validators;
        let has_required_run_duration = observed_duration_seconds >= required_duration_seconds;
        let has_required_block_count = observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected <= invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            reward_settlement_records >= criteria.min_reward_settlement_records;
        let has_production_libp2p_runtime = false;
        let has_deployed_rpc_service = false;
        let has_deployed_explorer_service = false;
        let has_deployed_faucet_service = false;
        let has_deployed_telemetry_service = false;
        let has_deployed_public_service_content = false;
        let has_deployed_public_services = false;
        let public_criterion_met = false;
        PublicTestnetEvidence {
            miner_count: self.miners.len(),
            validator_count: self.validators.len(),
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_duration_seconds,
            required_duration_seconds,
            observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted,
            invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records,
            external_operator_evidence,
            has_production_libp2p_runtime,
            has_deployed_rpc_service,
            has_deployed_explorer_service,
            has_deployed_faucet_service,
            has_deployed_telemetry_service,
            has_deployed_public_service_content,
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_run_duration,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            cuda_verified_miner_count: 0,
            has_cuda_verified_miners: false,
            cuda_graph_execution_receipts: 0,
            has_cuda_graph_execution_evidence: false,
            validator_vrf_lifecycle_records: 0,
            has_validator_vrf_lifecycle_evidence: false,
            public_criterion_met,
        }
    }

    fn attest_matmul_receipts(
        &mut self,
        scheduler: &JobScheduler,
        job: &MatmulJob,
        receipts: &[(TensorOpReceipt, TensorServer)],
        _beacon: &Hash,
        txpool: &mut TxPool,
    ) {
        for (receipt, tensor_server) in receipts {
            let assignment_seed = self.chain.validator_assignment_seed(&receipt.receipt_id);
            let assignment =
                scheduler.assign_validators(&self.chain, receipt.receipt_id, &assignment_seed);
            for validator_address in assignment.validators {
                let validation_seed = self
                    .chain
                    .validation_seed(&receipt.receipt_id, &validator_address);
                let stake = self
                    .chain
                    .state()
                    .validators()
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_matmul_from_server(
                        job,
                        receipt,
                        tensor_server,
                        &validation_seed,
                        &self.chain.params().freivalds,
                    )
                    .expect("reference validator should verify generated job");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .apply_command(ChainCommand::SubmitAttestation(attestation))
                    .expect("registered validator attestation should be accepted");
            }
        }
    }

    fn produce_block_with_command(&mut self, proposer: Address, timestamp: u64) -> TensorBlock {
        let timestamp = self
            .chain
            .blocks()
            .last()
            .map(|parent| {
                timestamp.max(
                    parent.timestamp.saturating_add(
                        self.chain
                            .params()
                            .pow_timeout_blocks
                            .max(1)
                            .saturating_mul(self.chain.params().block_time_seconds.max(1)),
                    ),
                )
            })
            .unwrap_or(timestamp);
        self.chain
            .apply_command(ChainCommand::ProduceBlock {
                proposer,
                timestamp,
            })
            .expect("registered validator should produce a useful-verification block");
        self.chain
            .blocks()
            .last()
            .cloned()
            .expect("producing a block should append to the chain")
    }

    fn finalize_block(&mut self, block: &TensorBlock) {
        for validator in self.validators.clone() {
            let stake = self
                .chain
                .state()
                .validators()
                .get(&validator)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            self.chain
                .apply_command(ChainCommand::SubmitBlockVote(BlockVote::new(
                    validator, stake, block,
                )))
                .expect("registered validator vote should finalize local block");
            if self.chain.is_block_finalized(&block.hash()) {
                break;
            }
        }
    }
}
