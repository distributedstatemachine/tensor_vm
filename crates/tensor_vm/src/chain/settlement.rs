use super::{
    Chain, ChainEvent, ChainState, PendingReceiptReward, RECEIPT_REWARD_AWAITING_INCLUSION_HEIGHT,
    ReceiptRewardKind, ReceiptState, RedundantSettlementDelayRecord,
};
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
    let mut delayed_records = Vec::new();
    let mut clear_delay_ids = Vec::new();
    for (receipt_id, receipt) in &chain.state.receipts {
        if chain.state.settled_receipts.contains(receipt_id) {
            continue;
        }
        if chain.state.challenged_receipts.contains(receipt_id) {
            clear_delay_ids.push(*receipt_id);
            continue;
        }
        if chain.has_attestation_quorum(receipt_id) {
            let agreeing_miners = redundant_agreement_count(chain, receipt_id);
            let conflicting_quorum_receipts = conflicting_quorum_receipt_count(chain, *receipt_id);
            if agreeing_miners < chain.params.agreement_quorum.max(1) {
                delayed_records.push(redundant_settlement_delay_record(
                    chain,
                    *receipt_id,
                    receipt,
                    agreeing_miners,
                    conflicting_quorum_receipts,
                    "awaiting redundant miner agreement quorum",
                ));
                continue;
            }
            if let ReceiptState::LinearTrainingStep(receipt) = receipt
                && has_conflicting_linear_receipt(chain, *receipt_id, receipt)
            {
                delayed_records.push(redundant_settlement_delay_record(
                    chain,
                    *receipt_id,
                    &ReceiptState::LinearTrainingStep(receipt.clone()),
                    agreeing_miners,
                    conflicting_quorum_receipts,
                    "conflicting quorum-backed linear training transition",
                ));
                continue;
            }
            let reward_delay_until_height = chain
                .state
                .redundant_settlement_delays
                .get(receipt_id)
                .map(|record| record.reward_delay_until_height)
                .unwrap_or(RECEIPT_REWARD_AWAITING_INCLUSION_HEIGHT);
            newly_settled.push((*receipt_id, receipt.clone(), reward_delay_until_height));
        }
    }

    for receipt_id in clear_delay_ids {
        chain.state.redundant_settlement_delays.remove(&receipt_id);
    }
    for record in delayed_records {
        chain
            .state
            .redundant_settlement_delays
            .insert(record.receipt_id, record);
    }

    let miner_reward_inputs = newly_settled
        .iter()
        .map(|(receipt_id, receipt, _)| (*receipt_id, receipt.clone()))
        .collect::<Vec<_>>();
    let miner_rewards = miner_reward_allocations(&miner_reward_inputs, miner_reward_pool);
    let newly_settled_ids: BTreeSet<Hash> = newly_settled
        .iter()
        .map(|(receipt_id, _, _)| *receipt_id)
        .collect();
    let reward_delay_by_receipt = newly_settled
        .iter()
        .map(|(receipt_id, _, reward_delay_until_height)| (*receipt_id, *reward_delay_until_height))
        .collect::<BTreeMap<_, _>>();
    for (receipt_id, receipt, reward_delay_until_height) in newly_settled {
        chain.state.settled_receipts.insert(receipt_id);
        chain.state.redundant_settlement_delays.remove(&receipt_id);
        let mut miner_claim = None;
        let miner_reward = miner_rewards.get(&receipt_id).copied();
        if let Some(miner) = chain.state.miners.get_mut(&receipt.miner()) {
            if miner_reward.is_some() {
                miner.pending_tensor_work = miner
                    .pending_tensor_work
                    .saturating_add(receipt.tensor_work_units());
            }
            if let Some(reward) = miner_reward {
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
                reward_delay_until_height,
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
                reward_delay_by_receipt
                    .get(&attestation.receipt_id)
                    .copied()
                    .unwrap_or(RECEIPT_REWARD_AWAITING_INCLUSION_HEIGHT),
            );
        }
    }
}

fn redundant_settlement_delay_record(
    chain: &Chain,
    receipt_id: Hash,
    receipt: &ReceiptState,
    observed_agreeing_miners: usize,
    conflicting_quorum_receipts: usize,
    reason: &str,
) -> RedundantSettlementDelayRecord {
    RedundantSettlementDelayRecord {
        receipt_id,
        job_id: receipt.job_id(),
        primitive_type: receipt.primitive_type(),
        observed_agreeing_miners,
        required_agreement_quorum: chain.params.agreement_quorum.max(1),
        conflicting_quorum_receipts,
        recorded_at_height: chain.state.height,
        reward_delay_until_height: chain
            .state
            .height
            .saturating_add(chain.params.reward_maturity_delay_blocks()),
        reason: reason.to_owned(),
    }
}

fn conflicting_quorum_receipt_count(chain: &Chain, receipt_id: Hash) -> usize {
    let Some(receipt) = chain.state.receipts.get(&receipt_id) else {
        return 0;
    };
    chain
        .state
        .receipts
        .iter()
        .filter(|(other_id, other)| {
            **other_id != receipt_id
                && chain.has_attestation_quorum(other_id)
                && !receipts_agree(receipt, other)
                && comparable_for_redundant_agreement(receipt, other)
        })
        .count()
}

fn comparable_for_redundant_agreement(left: &ReceiptState, right: &ReceiptState) -> bool {
    match (left, right) {
        (ReceiptState::TensorOp(left), ReceiptState::TensorOp(right)) => {
            left.job_id == right.job_id
                && left.program_hash == right.program_hash
                && left.input_roots == right.input_roots
        }
        (ReceiptState::LinearTrainingStep(left), ReceiptState::LinearTrainingStep(right)) => {
            left.job_id == right.job_id
                && left.model_id == right.model_id
                && left.step == right.step
                && left.weight_root_before == right.weight_root_before
                && left.batch_root == right.batch_root
        }
        (ReceiptState::GraphExecution(left), ReceiptState::GraphExecution(right)) => {
            left.job_id == right.job_id && left.graph_id == right.graph_id
        }
        _ => false,
    }
}

fn miner_reward_allocations(
    newly_settled: &[(Hash, ReceiptState)],
    miner_reward_pool: u64,
) -> BTreeMap<Hash, u64> {
    let receipt_scores = newly_settled
        .iter()
        .map(|(receipt_id, receipt)| (*receipt_id, receipt.tensor_work_units()))
        .collect::<Vec<_>>();
    allocate_by_scores(miner_reward_pool, &receipt_scores)
}

fn allocate_by_scores<K>(pool: u64, scores: &[(K, u64)]) -> BTreeMap<K, u64>
where
    K: Copy + Ord,
{
    let total_score = scores
        .iter()
        .fold(0_u64, |acc, (_, score)| acc.saturating_add(*score));
    if pool == 0 || total_score == 0 {
        return BTreeMap::new();
    }
    let mut allocations = BTreeMap::new();
    let mut remainders = Vec::new();
    let mut allocated = 0_u64;
    for (key, score) in scores {
        if *score == 0 {
            continue;
        }
        let numerator = (*score as u128) * (pool as u128);
        let base = (numerator / total_score as u128) as u64;
        let remainder = numerator % total_score as u128;
        allocations.insert(*key, base);
        allocated = allocated.saturating_add(base);
        remainders.push((*key, remainder));
    }
    remainders.sort_by(|(left_key, left_rem), (right_key, right_rem)| {
        right_rem
            .cmp(left_rem)
            .then_with(|| left_key.cmp(right_key))
    });
    for (key, _) in remainders
        .into_iter()
        .take(pool.saturating_sub(allocated) as usize)
    {
        if let Some(amount) = allocations.get_mut(&key) {
            *amount = amount.saturating_add(1);
        }
    }
    allocations
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

pub(super) fn void_pending_miner_tensor_work(state: &mut ChainState, receipt_id: &Hash) {
    let Some(receipt) = state.receipts.get(receipt_id) else {
        return;
    };
    let miner_address = receipt.miner();
    let work = receipt.tensor_work_units();
    let Some(miner) = state.miners.get_mut(&miner_address) else {
        return;
    };
    miner.pending_tensor_work = miner.pending_tensor_work.saturating_sub(work);
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
