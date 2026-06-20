use crate::chain::{Chain, HardwareClass, JobState};
use crate::hash::hex;
use crate::jobs::PrimitiveType;
use crate::types::Address;
use tensor_vm_explorer::{
    ExplorerAccount, ExplorerBlock, ExplorerJob, ExplorerMiner, ExplorerOverview, ExplorerReceipt,
    ExplorerSummary, ExplorerValidator,
};

pub(super) fn explorer_summary(chain: &Chain) -> ExplorerSummary {
    ExplorerSummary {
        height: chain.state().height(),
        epoch: chain.state().epoch(),
        block_count: chain.blocks().len(),
        miner_count: chain.state().miners().len(),
        validator_count: chain.state().validators().len(),
        job_count: chain.state().jobs().len(),
        model_count: chain.state().model_states().len(),
        attestation_count: chain.state().attestations().values().map(Vec::len).sum(),
        receipt_count: chain.state().receipts().len(),
        settled_receipt_count: chain.state().settled_receipts().len(),
        data_unavailable_receipt_count: chain.state().data_unavailable_receipts().len(),
        data_unavailability_slash_count: chain.state().data_unavailability_slashes().len(),
        data_unavailability_slashed_amount_total: chain
            .state()
            .data_unavailability_slashes()
            .values()
            .map(|slash| slash.amount)
            .sum(),
        finalized_block_count: chain.state().finalized_blocks().len(),
        treasury_balance: chain.state().rewards().treasury(),
        pending_receipt_reward_count: chain.state().pending_receipt_rewards().len(),
        pending_challenge_reward_count: chain.state().pending_challenge_rewards().len(),
        total_reward_balance: chain.state().rewards().total_balance(),
    }
}

pub(super) fn explorer_account(chain: &Chain, address: &Address) -> ExplorerAccount {
    let state = chain.state();
    let miner = state.miners().get(address);
    let validator = state.validators().get(address);
    let balance = state
        .accounts()
        .get(address)
        .map(|account| account.balance)
        .unwrap_or_default();
    ExplorerAccount {
        address: hex(address),
        is_miner: miner.is_some(),
        is_validator: validator.is_some(),
        balance,
        reward_balance: state.rewards().balance(address),
        stake: miner
            .map(|miner| miner.stake)
            .or_else(|| validator.map(|validator| validator.stake))
            .unwrap_or_default(),
        reputation: miner
            .map(|miner| miner.reputation)
            .or_else(|| validator.map(|validator| validator.reputation))
            .unwrap_or_default(),
        settled_tensor_work: miner
            .map(|miner| miner.settled_tensor_work)
            .unwrap_or_default(),
        pending_tensor_work: miner
            .map(|miner| miner.pending_tensor_work)
            .unwrap_or_default(),
    }
}

pub(super) fn explorer_blocks(chain: &Chain, limit: usize) -> Vec<ExplorerBlock> {
    chain
        .blocks()
        .iter()
        .rev()
        .take(limit)
        .map(|block| ExplorerBlock {
            height: block.height,
            epoch: block.epoch,
            hash: hex(&block.hash()),
            proposer: hex(&block.proposer),
            state_root: hex(&block.state_root),
            timestamp: block.timestamp,
        })
        .collect()
}

pub(super) fn explorer_miners(chain: &Chain) -> Vec<ExplorerMiner> {
    let state = chain.state();
    state
        .miners()
        .values()
        .map(|miner| ExplorerMiner {
            address: hex(&miner.address),
            operator_id: hex(&miner.operator_id),
            stake: miner.stake,
            reputation: miner.reputation,
            settled_tensor_work: miner.settled_tensor_work,
            pending_tensor_work: miner.pending_tensor_work,
            hardware_class: hardware_class_label(miner.hardware_class).to_owned(),
            gpu_utilization_bps: miner.gpu_utilization_bps,
            reward_balance: state.rewards().balance(&miner.address),
        })
        .collect()
}

pub(super) fn explorer_validators(chain: &Chain) -> Vec<ExplorerValidator> {
    let state = chain.state();
    state
        .validators()
        .values()
        .map(|validator| ExplorerValidator {
            address: hex(&validator.address),
            stake: validator.stake,
            reputation: validator.reputation,
            valid_attestations: validator.valid_attestations,
            missed_assignments: validator.missed_assignments,
            reward_balance: state.rewards().balance(&validator.address),
        })
        .collect()
}

pub(super) fn explorer_receipts(chain: &Chain, limit: usize) -> Vec<ExplorerReceipt> {
    let state = chain.state();
    state
        .receipts()
        .iter()
        .rev()
        .take(limit)
        .map(|(receipt_id, receipt)| {
            let validator_attestations: Vec<_> = chain
                .state()
                .attestations()
                .get(receipt_id)
                .into_iter()
                .flat_map(|attestations| attestations.iter())
                .map(|attestation| hex(&attestation.validator))
                .collect();
            ExplorerReceipt {
                receipt_id: hex(receipt_id),
                job_id: hex(&receipt.job_id()),
                primitive_type: primitive_label(receipt.primitive_type()).to_owned(),
                miner: hex(&receipt.miner()),
                tensor_work_units: receipt.tensor_work_units(),
                attestation_count: validator_attestations.len(),
                validator_attestations,
                settled: state.settled_receipts().contains(receipt_id),
            }
        })
        .collect()
}

pub(super) fn explorer_jobs(chain: &Chain, limit: usize) -> Vec<ExplorerJob> {
    chain
        .state()
        .jobs()
        .values()
        .rev()
        .take(limit)
        .map(|job| match job {
            JobState::TensorOp(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "tensor_op".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("matmul {}x{}x{}", job.m, job.k, job.n),
            },
            JobState::LinearTrainingStep(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "linear_training_step".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("model step {} input {:?}", job.step, job.input_shape),
            },
        })
        .collect()
}

pub(super) fn explorer_overview(
    chain: &Chain,
    block_limit: usize,
    receipt_limit: usize,
    job_limit: usize,
) -> ExplorerOverview {
    ExplorerOverview {
        summary: explorer_summary(chain),
        blocks: explorer_blocks(chain, block_limit),
        miners: explorer_miners(chain),
        validators: explorer_validators(chain),
        receipts: explorer_receipts(chain, receipt_limit),
        jobs: explorer_jobs(chain, job_limit),
    }
}

pub(super) fn primitive_label(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::TensorOp => "tensor_op",
        PrimitiveType::LinearTrainingStep => "linear_training_step",
    }
}

pub(super) fn hardware_class_label(hardware_class: HardwareClass) -> &'static str {
    match hardware_class {
        HardwareClass::Cpu => "cpu",
        HardwareClass::ConsumerGpu => "consumer_gpu",
        HardwareClass::DatacenterGpu => "datacenter_gpu",
        HardwareClass::Other => "other",
    }
}
