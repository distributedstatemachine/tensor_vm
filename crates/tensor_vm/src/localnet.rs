#[cfg(test)]
use crate::chain::{BlockVote, TensorBlock};
use crate::chain::{Chain, ChainCommand, ChainEngine, JobState, ReceiptState};
use crate::error::Result;
use crate::jobs::{LinearTrainingStepJob, MatmulJob};
use crate::profile::ChainProfile;
use crate::roles::{
    CpuReferenceMinerRole, ReferenceValidatorRole, RoleReceiptBundle, validator_stake,
};
use crate::scheduler::{JobScheduler, JobSource, SyntheticLocalJobSource};
use crate::tensor::Tensor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticCpuRoundResult {
    pub height: u64,
    pub tensors: Vec<Tensor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntheticCpuWorkResult {
    pub tensors: Vec<Tensor>,
}

pub fn produce_synthetic_cpu_round(chain: &mut Chain) -> Result<Option<u64>> {
    Ok(produce_synthetic_cpu_round_with_tensors(chain)?.map(|round| round.height))
}

pub fn produce_synthetic_cpu_round_with_tensors(
    chain: &mut Chain,
) -> Result<Option<SyntheticCpuRoundResult>> {
    produce_synthetic_cpu_round_with_profile(chain, &ChainProfile::local_cpu())
}

pub fn produce_synthetic_cpu_round_with_profile(
    chain: &mut Chain,
    profile: &ChainProfile,
) -> Result<Option<SyntheticCpuRoundResult>> {
    let Some(mut job_source) = profile.synthetic_job_source() else {
        return Ok(None);
    };
    produce_synthetic_cpu_round_from_source(chain, &mut job_source)
}

pub fn produce_synthetic_cpu_work_with_profile(
    chain: &mut Chain,
    profile: &ChainProfile,
) -> Result<Option<SyntheticCpuWorkResult>> {
    let Some(mut job_source) = profile.synthetic_job_source() else {
        return Ok(None);
    };
    produce_synthetic_cpu_work_from_source(chain, &mut job_source)
}

fn produce_synthetic_cpu_round_from_source(
    chain: &mut Chain,
    job_source: &mut impl JobSource,
) -> Result<Option<SyntheticCpuRoundResult>> {
    let Some(work) = produce_synthetic_cpu_work_from_source(chain, job_source)? else {
        return Ok(None);
    };
    let beacon = chain.state().finalized_randomness();
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap_or_default();
    let timestamp = next_block_timestamp(chain);
    chain.apply_command(ChainCommand::ProduceBlock {
        proposer,
        timestamp,
    })?;
    Ok(Some(SyntheticCpuRoundResult {
        height: chain.state().height(),
        tensors: work.tensors,
    }))
}

fn produce_synthetic_cpu_work_from_source(
    chain: &mut Chain,
    job_source: &mut impl JobSource,
) -> Result<Option<SyntheticCpuWorkResult>> {
    if chain.state().miners().is_empty() || chain.state().validators().is_empty() {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    match job_source.next_job(chain) {
        Some(JobState::TensorOp(job)) => produce_synthetic_matmul_work(chain, &scheduler, job),
        Some(JobState::LinearTrainingStep(job)) => {
            produce_synthetic_linear_training_work(chain, &scheduler, job)
        }
        None => Ok(None),
    }
}

fn produce_synthetic_matmul_work(
    chain: &mut Chain,
    scheduler: &JobScheduler,
    job: MatmulJob,
) -> Result<Option<SyntheticCpuWorkResult>> {
    let beacon = chain.state().finalized_randomness();
    let job_state = JobState::TensorOp(job.clone());
    chain.apply_command(ChainCommand::SubmitJob(job_state.clone()))?;
    let miner_assignment = scheduler.assign_miners(chain, job.job_id, &beacon);
    let mut receipts = Vec::new();
    for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
        let receipt = CpuReferenceMinerRole::new(miner_address).execute_job(
            &job_state,
            chain.state().height(),
            1 + index as u64,
        )?;
        chain.apply_command(ChainCommand::SubmitReceipt(receipt.receipt.clone()))?;
        receipts.push(receipt);
    }
    attest_receipt_bundles(chain, scheduler, &job_state, &receipts, &beacon)?;
    let Some(canonical_receipt) = receipts.first() else {
        return Ok(None);
    };
    if !chain.has_attestation_quorum(&canonical_receipt.receipt_id())
        || !chain.has_redundant_agreement(&canonical_receipt.receipt_id())
    {
        return Ok(None);
    }
    chain.apply_command(ChainCommand::SettleEpoch {
        miner_reward_pool: 1_000,
        validator_reward_pool: 500,
    })?;
    Ok(Some(SyntheticCpuWorkResult {
        tensors: canonical_receipt.served_tensors(),
    }))
}

fn produce_synthetic_linear_training_work(
    chain: &mut Chain,
    scheduler: &JobScheduler,
    job: LinearTrainingStepJob,
) -> Result<Option<SyntheticCpuWorkResult>> {
    let beacon = chain.state().finalized_randomness();
    let weights = SyntheticLocalJobSource::linear_training_weights();
    register_synthetic_linear_model(chain, &job, &weights)?;
    let job_state = JobState::LinearTrainingStep(job.clone());
    chain.apply_command(ChainCommand::SubmitJob(job_state.clone()))?;
    let miner_assignment = scheduler.assign_miners(chain, job.job_id, &beacon);
    let mut receipts = Vec::new();
    for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
        let receipt = CpuReferenceMinerRole::new(miner_address).execute_job(
            &job_state,
            chain.state().height(),
            1 + index as u64,
        )?;
        chain.apply_command(ChainCommand::SubmitReceipt(receipt.receipt.clone()))?;
        receipts.push(receipt);
    }
    attest_receipt_bundles(chain, scheduler, &job_state, &receipts, &beacon)?;
    let Some(canonical_receipt) = receipts.first() else {
        return Ok(None);
    };
    if !chain.has_attestation_quorum(&canonical_receipt.receipt_id())
        || !chain.has_redundant_agreement(&canonical_receipt.receipt_id())
    {
        return Ok(None);
    }
    chain.apply_command(ChainCommand::SettleEpoch {
        miner_reward_pool: 1_000,
        validator_reward_pool: 500,
    })?;
    let weight_root_after = match &canonical_receipt.receipt {
        ReceiptState::LinearTrainingStep(receipt) => receipt.weight_root_after,
        ReceiptState::TensorOp(_) => unreachable!("linear round must produce linear receipts"),
    };
    chain.apply_command(ChainCommand::ApplyModelTransition {
        model_id: job.model_id,
        step: job.step,
        weight_root_before: job.weight_root_before,
        weight_root_after,
    })?;
    Ok(Some(SyntheticCpuWorkResult {
        tensors: canonical_receipt.served_tensors(),
    }))
}

fn next_block_timestamp(chain: &Chain) -> u64 {
    chain
        .blocks()
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params().block_time_seconds)
        })
        .unwrap_or(0)
}

fn attest_receipt_bundles(
    chain: &mut Chain,
    scheduler: &JobScheduler,
    job: &JobState,
    receipts: &[RoleReceiptBundle],
    beacon: &crate::types::Hash,
) -> Result<()> {
    for receipt in receipts {
        let receipt_id = receipt.receipt_id();
        let validation_seed = chain.validation_seed(&receipt_id);
        let validator_assignment = scheduler.assign_validators(chain, receipt_id, beacon);
        for validator_address in validator_assignment.validators {
            let validator = ReferenceValidatorRole::new(
                validator_address,
                validator_stake(chain, &validator_address),
            );
            let attestation = validator.verify_receipt(
                job,
                receipt,
                &validation_seed,
                &chain.params().freivalds,
            )?;
            chain.apply_command(ChainCommand::SubmitAttestation(attestation))?;
        }
    }
    Ok(())
}

fn register_synthetic_linear_model(
    chain: &mut Chain,
    job: &LinearTrainingStepJob,
    weights: &Tensor,
) -> Result<()> {
    if !chain.state().model_states().contains_key(&job.model_id) {
        chain.apply_command(ChainCommand::RegisterModel {
            model_id: job.model_id,
            architecture_hash: SyntheticLocalJobSource::linear_training_architecture_hash(),
            weight_root: weights.commitment_root(),
            config_hash: SyntheticLocalJobSource::linear_training_config_hash(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
pub fn finalize_local_cpu_block(chain: &mut Chain, block: &TensorBlock) -> Result<()> {
    for validator_address in chain
        .state()
        .validators()
        .keys()
        .copied()
        .collect::<Vec<_>>()
    {
        let stake = chain
            .state()
            .validators()
            .get(&validator_address)
            .map(|validator| validator.stake)
            .unwrap_or_default();
        chain.apply_command(ChainCommand::SubmitBlockVote(BlockVote::new(
            validator_address,
            stake,
            block,
        )))?;
        if chain.is_block_finalized(&block.hash()) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainParams;
    use crate::types::{address, hash_bytes};
    use crate::verify::FreivaldsParams;

    #[test]
    fn synthetic_cpu_round_settles_work_and_advances_finalized_chain() {
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"localnet-round"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("localnet-miner-{index}").as_bytes()),
                    chain.params().miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("localnet-validator-{index}").as_bytes()),
                    chain.params().validator_min_stake,
                )
                .unwrap();
        }

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let first_block = chain.blocks().last().unwrap().clone();

        assert_eq!(height, Some(1));
        assert_eq!(chain.state().height(), 1);
        assert_eq!(chain.state().settled_receipts().len(), 2);
        assert!(
            !chain
                .state()
                .block_votes()
                .contains_key(&first_block.hash())
        );
        assert!(!chain.is_block_finalized(&first_block.hash()));
        finalize_local_cpu_block(&mut chain, &first_block).unwrap();
        assert!(chain.is_block_finalized(&first_block.hash()));

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let second_block = chain.blocks().last().unwrap().clone();

        assert_eq!(height, Some(2));
        assert_eq!(
            second_block.timestamp,
            first_block
                .timestamp
                .saturating_add(chain.params().block_time_seconds)
        );
        assert!(
            !chain
                .state()
                .block_votes()
                .contains_key(&second_block.hash())
        );
        assert!(!chain.is_block_finalized(&second_block.hash()));
        finalize_local_cpu_block(&mut chain, &second_block).unwrap();
        assert!(chain.is_block_finalized(&second_block.hash()));
        assert!(
            chain
                .state()
                .jobs()
                .values()
                .any(|job| matches!(job, JobState::LinearTrainingStep(_)))
        );
        assert_eq!(chain.state().model_states().len(), 1);
        assert_eq!(
            chain.state().model_states().values().next().unwrap().step,
            1
        );

        let height = produce_synthetic_cpu_round(&mut chain).unwrap();
        let third_block = chain.blocks().last().unwrap();

        assert_eq!(height, Some(3));
        assert_eq!(
            third_block.timestamp,
            second_block
                .timestamp
                .saturating_add(chain.params().block_time_seconds)
        );
        assert!(
            !chain
                .state()
                .block_votes()
                .contains_key(&third_block.hash())
        );
        assert!(!chain.is_block_finalized(&third_block.hash()));
        let third_block = third_block.clone();
        finalize_local_cpu_block(&mut chain, &third_block).unwrap();
        assert!(chain.is_block_finalized(&third_block.hash()));
    }

    #[test]
    fn synthetic_cpu_round_waits_for_registered_roles() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"localnet-empty"]));
        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
    }

    #[test]
    fn synthetic_cpu_round_uses_profile_configured_jobs() {
        let mut profile = ChainProfile::local_cpu();
        profile.synthetic_job_scheduler = Some(JobScheduler::with_small_shape((2, 3, 4)));
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 2,
                minimum_validators: 2,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"profile-localnet"]));
        for index in 0..2 {
            chain
                .register_miner(
                    address(format!("profile-localnet-miner-{index}").as_bytes()),
                    chain.params().miner_min_stake,
                )
                .unwrap();
            chain
                .register_validator(
                    address(format!("profile-localnet-validator-{index}").as_bytes()),
                    chain.params().validator_min_stake,
                )
                .unwrap();
        }

        let round = produce_synthetic_cpu_round_with_profile(&mut chain, &profile)
            .unwrap()
            .expect("profile-enabled localnet should produce a round");

        assert_eq!(round.height, 1);
        assert!(chain.state().jobs().values().any(
            |job| matches!(job, JobState::TensorOp(job) if (job.m, job.k, job.n) == (2, 3, 4))
        ));
    }

    #[test]
    fn synthetic_cpu_round_waits_when_profile_disables_synthetic_jobs() {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"profile-no-synthetic"]));
        chain
            .register_miner(
                address(b"profile-no-synthetic-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"profile-no-synthetic-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();

        assert_eq!(
            produce_synthetic_cpu_round_with_profile(&mut chain, &ChainProfile::public_testnet())
                .unwrap(),
            None
        );
        assert!(chain.blocks().is_empty());
        assert!(chain.state().jobs().is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_job_source() {
        struct EmptyJobSource;

        impl JobSource for EmptyJobSource {
            fn next_job(&mut self, _chain: &Chain) -> Option<JobState> {
                None
            }
        }

        let mut chain = Chain::new(hash_bytes(b"test", &[b"localnet-no-job"]));
        chain
            .register_miner(
                address(b"localnet-no-job-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-no-job-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();
        let mut job_source = EmptyJobSource;

        assert_eq!(
            produce_synthetic_cpu_round_from_source(&mut chain, &mut job_source).unwrap(),
            None
        );
        assert!(chain.blocks().is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_miner_assignment() {
        let params = ChainParams {
            replication_factor: 0,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"localnet-no-miners"]));
        chain
            .register_miner(
                address(b"localnet-assignment-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-assignment-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks().is_empty());
    }

    #[test]
    fn synthetic_cpu_round_waits_for_redundant_agreement() {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain =
            Chain::with_params(params, hash_bytes(b"test", &[b"localnet-no-agreement"]));
        chain
            .register_miner(
                address(b"localnet-agreement-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-agreement-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks().is_empty());
        assert!(chain.state().settled_receipts().is_empty());
    }

    #[test]
    fn synthetic_linear_round_waits_for_miner_assignment() {
        let params = ChainParams {
            replication_factor: 0,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain =
            Chain::with_params(params, hash_bytes(b"test", &[b"localnet-linear-no-miners"]));
        chain.set_position_for_testing(1, 0);
        chain
            .register_miner(
                address(b"localnet-linear-assignment-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-linear-assignment-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks().is_empty());
        assert_eq!(chain.state().model_states().len(), 1);
        assert_eq!(
            chain.state().model_states().values().next().unwrap().step,
            0
        );
    }

    #[test]
    fn synthetic_linear_round_waits_for_redundant_agreement() {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(
            params,
            hash_bytes(b"test", &[b"localnet-linear-no-agreement"]),
        );
        chain.set_position_for_testing(1, 0);
        chain
            .register_miner(
                address(b"localnet-linear-agreement-miner"),
                chain.params().miner_min_stake,
            )
            .unwrap();
        chain
            .register_validator(
                address(b"localnet-linear-agreement-validator"),
                chain.params().validator_min_stake,
            )
            .unwrap();

        assert_eq!(produce_synthetic_cpu_round(&mut chain).unwrap(), None);
        assert!(chain.blocks().is_empty());
        assert!(chain.state().settled_receipts().is_empty());
        assert_eq!(
            chain.state().model_states().values().next().unwrap().step,
            0
        );
    }
}
