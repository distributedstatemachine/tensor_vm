use super::{Chain, ChainEvent, PendingReceiptReward, ReceiptRewardKind, ReceiptState};
use crate::jobs::LinearTrainingStepReceipt;
use crate::types::{Address, Hash, hash_bytes};
use crate::verify::VerificationResult;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn redundant_agreement_count(chain: &Chain, receipt_id: &Hash) -> usize {
    let Some(receipt) = chain.state.receipts.get(receipt_id) else {
        return 0;
    };
    let mut agreeing_miners = BTreeSet::new();
    for (other_id, other) in &chain.state.receipts {
        if chain.has_attestation_quorum(other_id) && receipts_agree(receipt, other) {
            agreeing_miners.insert(other.miner());
        }
    }
    agreeing_miners.len()
}

pub(super) fn has_redundant_agreement(chain: &Chain, receipt_id: &Hash) -> bool {
    if !chain.state.receipts.contains_key(receipt_id) {
        return false;
    }
    if chain.params.agreement_quorum <= 1 {
        return true;
    }
    redundant_agreement_count(chain, receipt_id) >= chain.params.agreement_quorum
}

pub(super) fn settle_epoch(chain: &mut Chain, miner_reward_pool: u64, validator_reward_pool: u64) {
    let mut newly_settled = Vec::new();
    for (receipt_id, receipt) in &chain.state.receipts {
        if chain.state.settled_receipts.contains(receipt_id) {
            continue;
        }
        if chain.state.challenged_receipts.contains(receipt_id) {
            continue;
        }
        if chain.has_attestation_quorum(receipt_id) {
            if !has_redundant_agreement(chain, receipt_id) {
                continue;
            }
            if let ReceiptState::LinearTrainingStep(receipt) = receipt
                && has_conflicting_linear_receipt(chain, *receipt_id, receipt)
            {
                continue;
            }
            newly_settled.push((*receipt_id, receipt.clone()));
        }
    }

    let total_work: u64 = newly_settled
        .iter()
        .map(|(_, receipt)| receipt.tensor_work_units())
        .sum();
    let newly_settled_ids: BTreeSet<Hash> = newly_settled
        .iter()
        .map(|(receipt_id, _)| *receipt_id)
        .collect();
    let claimable_at_height = receipt_reward_claimable_height(chain);
    for (receipt_id, receipt) in newly_settled {
        chain.state.settled_receipts.insert(receipt_id);
        let mut miner_claim = None;
        if let Some(miner) = chain.state.miners.get_mut(&receipt.miner()) {
            miner.pending_tensor_work = miner
                .pending_tensor_work
                .saturating_add(receipt.tensor_work_units());
            miner.settled_tensor_work = miner
                .settled_tensor_work
                .saturating_add(receipt.tensor_work_units());
            if let Some(reward) = miner_reward_pool
                .saturating_mul(receipt.tensor_work_units())
                .checked_div(total_work)
            {
                miner_claim = Some((miner.address, reward));
            }
        }
        if let Some((beneficiary, reward)) = miner_claim {
            enqueue_pending_receipt_reward(
                chain,
                receipt_id,
                beneficiary,
                reward,
                ReceiptRewardKind::Miner,
                claimable_at_height,
            );
        }
    }

    let valid_attestations: Vec<_> = chain
        .state
        .attestations
        .iter()
        .filter(|(receipt_id, _)| newly_settled_ids.contains(*receipt_id))
        .flat_map(|(_, items)| items.iter())
        .filter(|att| att.result == VerificationResult::Valid && att.data_availability_passed)
        .cloned()
        .collect();
    let total_valid = valid_attestations.len() as u64;
    if let Some(validator_reward) = validator_reward_pool.checked_div(total_valid) {
        for attestation in valid_attestations {
            enqueue_pending_receipt_reward(
                chain,
                attestation.receipt_id,
                attestation.validator,
                validator_reward,
                ReceiptRewardKind::Validator,
                claimable_at_height,
            );
        }
    }
}

pub(super) fn events(
    chain: &Chain,
    settled_before: &BTreeSet<Hash>,
    pending_rewards_before: &BTreeMap<Hash, PendingReceiptReward>,
) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    for receipt_id in chain.state.settled_receipts.difference(settled_before) {
        events.push(ChainEvent::ReceiptSettled(*receipt_id));
    }
    for (claim_id, reward) in &chain.state.pending_receipt_rewards {
        if !pending_rewards_before.contains_key(claim_id) {
            events.push(ChainEvent::ReceiptRewardPending {
                claim_id: *claim_id,
                receipt_id: reward.receipt_id,
                beneficiary: reward.beneficiary,
                amount: reward.amount,
                claimable_at_height: reward.claimable_at_height,
            });
        }
    }
    events
}

pub(super) fn receipt_reward_claimable_height(chain: &Chain) -> u64 {
    chain
        .state
        .height
        .saturating_add(chain.params.reward_maturity_delay_blocks())
}

fn enqueue_pending_receipt_reward(
    chain: &mut Chain,
    receipt_id: Hash,
    beneficiary: Address,
    amount: u64,
    kind: ReceiptRewardKind,
    claimable_at_height: u64,
) {
    if amount == 0 {
        return;
    }
    let claim_id = receipt_reward_claim_id(&receipt_id, &beneficiary, kind);
    chain
        .state
        .pending_receipt_rewards
        .entry(claim_id)
        .or_insert(PendingReceiptReward {
            claim_id,
            receipt_id,
            beneficiary,
            amount,
            kind,
            claimable_at_height,
            voided_by_challenge: false,
        });
}

fn receipt_reward_claim_id(
    receipt_id: &Hash,
    beneficiary: &Address,
    kind: ReceiptRewardKind,
) -> Hash {
    hash_bytes(
        b"tensor-vm-receipt-reward-claim-id-v1",
        &[receipt_id, beneficiary, &[kind.tag()]],
    )
}

pub(super) fn receipts_agree(left: &ReceiptState, right: &ReceiptState) -> bool {
    match (left, right) {
        (ReceiptState::TensorOp(left), ReceiptState::TensorOp(right)) => {
            left.job_id == right.job_id
                && left.program_hash == right.program_hash
                && left.input_roots == right.input_roots
                && left.output_roots == right.output_roots
                && left.trace_root == right.trace_root
        }
        (ReceiptState::LinearTrainingStep(left), ReceiptState::LinearTrainingStep(right)) => {
            left.job_id == right.job_id
                && left.model_id == right.model_id
                && left.step == right.step
                && left.weight_root_before == right.weight_root_before
                && left.batch_root == right.batch_root
                && left.y_root == right.y_root
                && left.loss_commitment == right.loss_commitment
                && left.grad_w_root == right.grad_w_root
                && left.weight_root_after == right.weight_root_after
                && left.trace_root == right.trace_root
        }
        (ReceiptState::GraphExecution(left), ReceiptState::GraphExecution(right)) => {
            left.job_id == right.job_id
                && left.graph_id == right.graph_id
                && left.input_roots == right.input_roots
                && left.output_roots == right.output_roots
                && left.trace_root == right.trace_root
        }
        _ => false,
    }
}

pub(super) fn has_conflicting_linear_receipt(
    chain: &Chain,
    receipt_id: Hash,
    receipt: &LinearTrainingStepReceipt,
) -> bool {
    chain
        .state
        .receipts
        .iter()
        .any(|(other_id, other)| match other {
            ReceiptState::LinearTrainingStep(other) => {
                *other_id != receipt_id
                    && other.model_id == receipt.model_id
                    && other.step == receipt.step
                    && other.weight_root_before == receipt.weight_root_before
                    && other.weight_root_after != receipt.weight_root_after
                    && chain.has_attestation_quorum(other_id)
            }
            ReceiptState::TensorOp(_) | ReceiptState::GraphExecution(_) => false,
        })
}
