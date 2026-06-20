use super::{
    AccountState, BlockCheckChallengeRecord, BlockVote, ChainState, DataUnavailabilitySlashRecord,
    JobState, MinerState, ModelState, PendingChallengeReward, PendingCreditReward,
    PendingProposerReward, PendingReceiptReward, ReceiptState, RewardState,
    ValidatorAuditAssignment, ValidatorAuditResult, ValidatorAuditSlashRecord, ValidatorState,
};
use crate::codec::{dtype_tag, primitive_type_tag, verification_result_tag};
use crate::merkle::merkle_root;
use crate::types::{Address, Hash, hash_bytes};
use crate::verify::{ValidatorAttestation, VerificationResult};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn reward_root(rewards: &RewardState) -> Hash {
    let mut encoded = Vec::new();
    for (address, balance) in &rewards.balances {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&balance.to_le_bytes());
    }
    encoded.extend_from_slice(&rewards.treasury.to_le_bytes());
    hash_bytes(b"tensor-vm-reward-root-v1", &[&encoded])
}

pub(super) fn state_root(state: &ChainState) -> Hash {
    let mut parts = Vec::new();
    parts.extend_from_slice(&state.height.to_le_bytes());
    parts.extend_from_slice(&state.epoch.to_le_bytes());
    parts.extend_from_slice(&state.finalized_beacon_round.to_le_bytes());
    parts.extend_from_slice(&state.finalized_randomness);
    parts.extend_from_slice(&state.genesis_beacon_round.to_le_bytes());
    parts.extend_from_slice(&state.genesis_randomness);
    parts.extend_from_slice(&account_root(&state.accounts));
    parts.extend_from_slice(&miner_root(&state.miners));
    parts.extend_from_slice(&validator_root(&state.validators));
    parts.extend_from_slice(&job_root(&state.jobs));
    parts.extend_from_slice(&program_body_root(&state.program_bodies));
    parts.extend_from_slice(&receipt_root(&state.receipts));
    parts.extend_from_slice(&attestation_root(&state.attestations));
    parts.extend_from_slice(&block_finality_root(
        &state.block_votes,
        &state.finalized_blocks,
    ));
    parts.extend_from_slice(&hash_set_root(
        b"tensor-vm-data-unavailable-root-v1",
        &state.data_unavailable_receipts,
    ));
    parts.extend_from_slice(&data_unavailability_slash_root(
        &state.data_unavailability_slashes,
    ));
    parts.extend_from_slice(&validator_audit_assignment_root(
        &state.validator_audit_assignments,
    ));
    parts.extend_from_slice(&validator_audit_result_root(&state.validator_audit_results));
    parts.extend_from_slice(&validator_audit_slash_root(&state.validator_audit_slashes));
    parts.extend_from_slice(&settled_receipt_root(&state.settled_receipts));
    parts.extend_from_slice(&hash_set_root(
        b"tensor-vm-included-receipt-root-v1",
        &state.included_receipts,
    ));
    parts.extend_from_slice(&block_check_challenge_root(&state.block_check_challenges));
    parts.extend_from_slice(&hash_set_root(
        b"tensor-vm-challenged-receipt-root-v1",
        &state.challenged_receipts,
    ));
    parts.extend_from_slice(&proposer_penalty_root(&state.proposer_penalty_until));
    parts.extend_from_slice(&pending_proposer_reward_root(
        &state.pending_proposer_rewards,
    ));
    parts.extend_from_slice(&pending_receipt_reward_root(&state.pending_receipt_rewards));
    parts.extend_from_slice(&pending_challenge_reward_root(
        &state.pending_challenge_rewards,
    ));
    parts.extend_from_slice(&pending_credit_reward_root(&state.pending_credit_rewards));
    parts.extend_from_slice(&model_state_root(&state.model_states));
    parts.extend_from_slice(&reward_root(&state.rewards));
    hash_bytes(b"tensor-vm-state-root-v1", &[&parts])
}

pub(super) fn program_body_root(programs: &BTreeMap<Hash, Vec<u8>>) -> Hash {
    let mut encoded = Vec::new();
    for (graph_id, body) in programs {
        encoded.extend_from_slice(graph_id);
        encoded.extend_from_slice(&(body.len() as u64).to_le_bytes());
        encoded.extend_from_slice(body);
    }
    hash_bytes(b"tensor-vm-program-body-root-v1", &[&encoded])
}

pub(super) fn block_check_challenge_root(
    challenges: &BTreeMap<Hash, BlockCheckChallengeRecord>,
) -> Hash {
    let mut encoded = Vec::new();
    for (challenge_id, challenge) in challenges {
        encoded.extend_from_slice(challenge_id);
        encoded.extend_from_slice(&challenge.block_hash);
        encoded.extend_from_slice(&challenge.block_height.to_le_bytes());
        encoded.extend_from_slice(&challenge.receipt_id);
        encoded.extend_from_slice(&challenge.proposer);
        encoded.extend_from_slice(&challenge.challenger);
        encoded.extend_from_slice(&challenge.expected_check_leaf);
        encoded.extend_from_slice(&challenge.observed_check_leaf);
        encoded.extend_from_slice(&challenge.challenged_at_height.to_le_bytes());
        encoded.extend_from_slice(&challenge.proposer_reward_clawback.to_le_bytes());
        encoded.extend_from_slice(&challenge.challenger_reward.to_le_bytes());
        encoded.extend_from_slice(&challenge.penalty_until_height.to_le_bytes());
        encoded.extend_from_slice(&(challenge.reason.len() as u64).to_le_bytes());
        encoded.extend_from_slice(challenge.reason.as_bytes());
    }
    hash_bytes(b"tensor-vm-block-check-challenge-root-v1", &[&encoded])
}

pub(super) fn data_unavailability_slash_root(
    slashes: &BTreeMap<Hash, DataUnavailabilitySlashRecord>,
) -> Hash {
    let mut encoded = Vec::new();
    for (receipt_id, slash) in slashes {
        encoded.extend_from_slice(receipt_id);
        encoded.extend_from_slice(&slash.receipt_id);
        encoded.extend_from_slice(&slash.miner);
        encoded.extend_from_slice(&slash.evidence_validator);
        encoded.extend_from_slice(&slash.amount.to_le_bytes());
        encoded.extend_from_slice(&slash.slashed_at_height.to_le_bytes());
        encoded.extend_from_slice(&(slash.reason.len() as u64).to_le_bytes());
        encoded.extend_from_slice(slash.reason.as_bytes());
    }
    hash_bytes(b"tensor-vm-data-unavailability-slash-root-v1", &[&encoded])
}

pub(super) fn validator_audit_assignment_root(
    assignments: &BTreeMap<Hash, ValidatorAuditAssignment>,
) -> Hash {
    let mut encoded = Vec::new();
    for (audit_id, assignment) in assignments {
        encoded.extend_from_slice(audit_id);
        encoded.extend_from_slice(&assignment.audit_id);
        encoded.extend_from_slice(&assignment.receipt_id);
        encoded.extend_from_slice(&assignment.validator);
        encoded.extend_from_slice(&assignment.assigned_at_height.to_le_bytes());
        encoded.extend_from_slice(&assignment.deadline_height.to_le_bytes());
        encoded.extend_from_slice(&assignment.seed);
    }
    hash_bytes(b"tensor-vm-validator-audit-assignment-root-v1", &[&encoded])
}

pub(super) fn validator_audit_result_root(results: &BTreeMap<Hash, ValidatorAuditResult>) -> Hash {
    let mut encoded = Vec::new();
    for (audit_id, result) in results {
        encoded.extend_from_slice(audit_id);
        encoded.extend_from_slice(&result.audit_id);
        encoded.extend_from_slice(&result.receipt_id);
        encoded.extend_from_slice(&result.validator);
        encoded.extend_from_slice(&result.auditor);
        encoded.push(verification_result_tag(result.attested_result));
        encoded.push(verification_result_tag(result.canonical_result));
        encoded.push(u8::from(result.attested_data_availability_passed));
        encoded.push(u8::from(result.canonical_data_availability_passed));
        encoded.extend_from_slice(&result.checks_root);
        encoded.extend_from_slice(&result.submitted_at_height.to_le_bytes());
        encoded.push(u8::from(result.passed));
        encoded.extend_from_slice(&result.signature);
    }
    hash_bytes(b"tensor-vm-validator-audit-result-root-v1", &[&encoded])
}

pub(super) fn validator_audit_slash_root(
    slashes: &BTreeMap<Hash, ValidatorAuditSlashRecord>,
) -> Hash {
    let mut encoded = Vec::new();
    for (audit_id, slash) in slashes {
        encoded.extend_from_slice(audit_id);
        encoded.extend_from_slice(&slash.audit_id);
        encoded.extend_from_slice(&slash.receipt_id);
        encoded.extend_from_slice(&slash.validator);
        encoded.extend_from_slice(&slash.auditor);
        encoded.extend_from_slice(&slash.amount.to_le_bytes());
        encoded.extend_from_slice(&slash.slashed_at_height.to_le_bytes());
        encoded.extend_from_slice(&(slash.reason.len() as u64).to_le_bytes());
        encoded.extend_from_slice(slash.reason.as_bytes());
    }
    hash_bytes(b"tensor-vm-validator-audit-slash-root-v1", &[&encoded])
}

pub(super) fn proposer_penalty_root(penalties: &BTreeMap<Address, u64>) -> Hash {
    let mut encoded = Vec::new();
    for (proposer, penalty_until_height) in penalties {
        encoded.extend_from_slice(proposer);
        encoded.extend_from_slice(&penalty_until_height.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-proposer-penalty-root-v1", &[&encoded])
}

pub(super) fn pending_proposer_reward_root(rewards: &BTreeMap<u64, PendingProposerReward>) -> Hash {
    let mut encoded = Vec::new();
    for (height, reward) in rewards {
        encoded.extend_from_slice(&height.to_le_bytes());
        encoded.extend_from_slice(&reward.block_height.to_le_bytes());
        encoded.extend_from_slice(&reward.proposer);
        encoded.extend_from_slice(&reward.amount.to_le_bytes());
        encoded.extend_from_slice(&reward.claimable_at_height.to_le_bytes());
        encoded.push(u8::from(reward.voided_by_challenge));
    }
    hash_bytes(b"tensor-vm-pending-proposer-reward-root-v1", &[&encoded])
}

pub(super) fn pending_receipt_reward_root(rewards: &BTreeMap<Hash, PendingReceiptReward>) -> Hash {
    let mut encoded = Vec::new();
    for (claim_id, reward) in rewards {
        encoded.extend_from_slice(claim_id);
        encoded.extend_from_slice(&reward.claim_id);
        encoded.extend_from_slice(&reward.receipt_id);
        encoded.extend_from_slice(&reward.beneficiary);
        encoded.extend_from_slice(&reward.amount.to_le_bytes());
        encoded.push(reward.kind.tag());
        encoded.extend_from_slice(&reward.claimable_at_height.to_le_bytes());
        encoded.push(u8::from(reward.voided_by_challenge));
    }
    hash_bytes(b"tensor-vm-pending-receipt-reward-root-v1", &[&encoded])
}

pub(super) fn pending_challenge_reward_root(
    rewards: &BTreeMap<Hash, PendingChallengeReward>,
) -> Hash {
    let mut encoded = Vec::new();
    for (claim_id, reward) in rewards {
        encoded.extend_from_slice(claim_id);
        encoded.extend_from_slice(&reward.claim_id);
        encoded.extend_from_slice(&reward.challenge_id);
        encoded.extend_from_slice(&reward.block_hash);
        encoded.extend_from_slice(&reward.receipt_id);
        encoded.extend_from_slice(&reward.challenger);
        encoded.extend_from_slice(&reward.amount.to_le_bytes());
        encoded.extend_from_slice(&reward.claimable_at_height.to_le_bytes());
        encoded.push(u8::from(reward.voided_by_challenge));
    }
    hash_bytes(b"tensor-vm-pending-challenge-reward-root-v1", &[&encoded])
}

pub(super) fn pending_credit_reward_root(rewards: &BTreeMap<Hash, PendingCreditReward>) -> Hash {
    let mut encoded = Vec::new();
    for (claim_id, reward) in rewards {
        encoded.extend_from_slice(claim_id);
        encoded.extend_from_slice(&reward.claim_id);
        encoded.extend_from_slice(&reward.beneficiary);
        encoded.extend_from_slice(&reward.amount.to_le_bytes());
        encoded.extend_from_slice(&reward.claimable_at_height.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-pending-credit-reward-root-v1", &[&encoded])
}

pub(super) fn block_finality_root(
    votes: &BTreeMap<Hash, Vec<BlockVote>>,
    finalized: &BTreeSet<Hash>,
) -> Hash {
    let mut encoded = Vec::new();
    for (block_hash, votes) in votes {
        encoded.extend_from_slice(block_hash);
        encoded.extend_from_slice(&(votes.len() as u64).to_le_bytes());
        for vote in votes {
            encoded.extend_from_slice(&vote.validator);
            encoded.extend_from_slice(&vote.block_hash);
            encoded.extend_from_slice(&vote.block_height.to_le_bytes());
            encoded.extend_from_slice(&vote.stake.to_le_bytes());
            encoded.extend_from_slice(&vote.signature);
        }
    }
    encoded.extend_from_slice(&(finalized.len() as u64).to_le_bytes());
    for block_hash in finalized {
        encoded.extend_from_slice(block_hash);
    }
    hash_bytes(b"tensor-vm-block-finality-root-v1", &[&encoded])
}

pub(super) fn account_root(accounts: &BTreeMap<Address, AccountState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, account) in accounts {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&account.balance.to_le_bytes());
        encoded.extend_from_slice(&account.nonce.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-account-root-v1", &[&encoded])
}

pub(super) fn miner_root(miners: &BTreeMap<Address, MinerState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, miner) in miners {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&miner.address);
        encoded.extend_from_slice(&miner.operator_id);
        encoded.extend_from_slice(&miner.stake.to_le_bytes());
        encoded.extend_from_slice(&miner.reputation.to_le_bytes());
        encoded.extend_from_slice(&miner.settled_tensor_work.to_le_bytes());
        encoded.extend_from_slice(&miner.pending_tensor_work.to_le_bytes());
        encoded.push(miner.hardware_class.tag());
        encoded.extend_from_slice(&miner.gpu_utilization_bps.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-miner-root-v1", &[&encoded])
}

pub(super) fn validator_root(validators: &BTreeMap<Address, ValidatorState>) -> Hash {
    let mut encoded = Vec::new();
    for (address, validator) in validators {
        encoded.extend_from_slice(address);
        encoded.extend_from_slice(&validator.address);
        encoded.extend_from_slice(&validator.stake.to_le_bytes());
        encoded.extend_from_slice(&validator.reputation.to_le_bytes());
        encoded.extend_from_slice(&validator.valid_attestations.to_le_bytes());
        encoded.extend_from_slice(&validator.missed_assignments.to_le_bytes());
    }
    hash_bytes(b"tensor-vm-validator-root-v1", &[&encoded])
}

pub(super) fn job_root(jobs: &BTreeMap<Hash, JobState>) -> Hash {
    let mut encoded = Vec::new();
    for (job_id, job) in jobs {
        encoded.extend_from_slice(job_id);
        match job {
            JobState::TensorOp(job) => {
                encoded.push(1);
                encoded.extend_from_slice(&job.job_id);
                encoded.extend_from_slice(&job.epoch.to_le_bytes());
                encode_usize(&mut encoded, job.m);
                encode_usize(&mut encoded, job.k);
                encode_usize(&mut encoded, job.n);
                encoded.push(dtype_tag(job.dtype));
                encoded.extend_from_slice(&job.modulus.unwrap_or_default().to_le_bytes());
                encoded.extend_from_slice(&job.seed_a);
                encoded.extend_from_slice(&job.seed_b);
                encoded.extend_from_slice(&job.deadline_block.to_le_bytes());
                encoded.extend_from_slice(&job.reward_weight.to_le_bytes());
            }
            JobState::LinearTrainingStep(job) => {
                encoded.push(2);
                encoded.extend_from_slice(&job.job_id);
                encoded.extend_from_slice(&job.model_id);
                encoded.extend_from_slice(&job.step.to_le_bytes());
                encoded.extend_from_slice(&job.batch_seed);
                encoded.extend_from_slice(&job.weight_root_before);
                encode_usizes(&mut encoded, &job.input_shape);
                encode_usizes(&mut encoded, &job.weight_shape);
                encode_usizes(&mut encoded, &job.target_shape);
                encoded.extend_from_slice(&job.lr.to_le_bytes());
                encoded.push(dtype_tag(job.dtype));
                encoded.extend_from_slice(&job.deadline_block.to_le_bytes());
                encoded.extend_from_slice(&job.reward_weight.to_le_bytes());
            }
        }
    }
    hash_bytes(b"tensor-vm-job-root-v1", &[&encoded])
}

pub(super) fn receipt_root(receipts: &BTreeMap<Hash, ReceiptState>) -> Hash {
    let mut encoded = Vec::new();
    for (receipt_id, receipt) in receipts {
        encoded.extend_from_slice(receipt_id);
        match receipt {
            ReceiptState::TensorOp(receipt) => {
                encoded.push(1);
                encoded.extend_from_slice(&receipt.receipt_id);
                encoded.extend_from_slice(&receipt.job_id);
                encoded.extend_from_slice(&receipt.miner);
                encoded.extend_from_slice(&receipt.program_hash);
                encode_hashes(&mut encoded, &receipt.input_roots);
                encode_hashes(&mut encoded, &receipt.output_roots);
                encoded.extend_from_slice(&receipt.trace_root);
                encoded.extend_from_slice(&receipt.tensor_work_units.to_le_bytes());
                encoded.extend_from_slice(&receipt.execution_time_ms.to_le_bytes());
                encoded.extend_from_slice(&receipt.submitted_at_block.to_le_bytes());
                encoded.extend_from_slice(&receipt.signature);
            }
            ReceiptState::LinearTrainingStep(receipt) => {
                encoded.push(2);
                encoded.extend_from_slice(&receipt.receipt_id);
                encoded.extend_from_slice(&receipt.job_id);
                encoded.extend_from_slice(&receipt.miner);
                encoded.extend_from_slice(&receipt.model_id);
                encoded.extend_from_slice(&receipt.step.to_le_bytes());
                encoded.extend_from_slice(&receipt.weight_root_before);
                encoded.extend_from_slice(&receipt.batch_root);
                encoded.extend_from_slice(&receipt.y_root);
                encoded.extend_from_slice(&receipt.loss_commitment);
                encoded.extend_from_slice(&receipt.grad_w_root);
                encoded.extend_from_slice(&receipt.weight_root_after);
                encoded.extend_from_slice(&receipt.trace_root);
                encoded.extend_from_slice(&receipt.tensor_work_units.to_le_bytes());
                encoded.extend_from_slice(&receipt.execution_time_ms.to_le_bytes());
                encoded.extend_from_slice(&receipt.submitted_at_block.to_le_bytes());
                encoded.extend_from_slice(&receipt.signature);
            }
        }
    }
    hash_bytes(b"tensor-vm-receipt-root-v1", &[&encoded])
}

pub(super) fn attestation_root(attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>) -> Hash {
    let mut encoded = Vec::new();
    for (receipt_id, attestations) in attestations {
        encoded.extend_from_slice(receipt_id);
        encoded.extend_from_slice(&(attestations.len() as u64).to_le_bytes());
        for attestation in attestations {
            encoded.extend_from_slice(&attestation.validator);
            encoded.extend_from_slice(&attestation.receipt_id);
            encoded.extend_from_slice(&attestation.job_id);
            encoded.push(primitive_type_tag(attestation.primitive_type));
            encoded.push(verification_result_tag(attestation.result));
            encoded.push(attestation.data_availability_passed as u8);
            encoded.extend_from_slice(&attestation.checks_root);
            encoded.extend_from_slice(&attestation.stake.to_le_bytes());
            encoded.extend_from_slice(&attestation.signature);
        }
    }
    hash_bytes(b"tensor-vm-attestation-root-v1", &[&encoded])
}

pub(super) fn settled_receipt_root(receipts: &BTreeSet<Hash>) -> Hash {
    hash_set_root(b"tensor-vm-settled-receipt-root-v1", receipts)
}

pub(super) fn selected_receipt_root(receipts: &BTreeSet<Hash>) -> Hash {
    hash_set_root(b"tensor-vm-selected-receipt-root", receipts)
}

pub(super) fn selected_receipt_commitment_root(
    selected_receipts: &[Hash],
    receipts: &BTreeMap<Hash, ReceiptState>,
) -> Hash {
    if selected_receipts.is_empty() {
        return selected_receipt_root(&BTreeSet::new());
    }
    let leaves = selected_receipt_leaves(selected_receipts, receipts);
    merkle_root(&leaves)
}

pub(super) fn selected_receipt_leaves(
    selected_receipts: &[Hash],
    receipts: &BTreeMap<Hash, ReceiptState>,
) -> Vec<Hash> {
    selected_receipts
        .iter()
        .map(|receipt_id| {
            let receipt = receipts.get(receipt_id);
            selected_receipt_leaf(receipt_id, receipt)
        })
        .collect()
}

pub(super) fn selected_receipt_leaf(receipt_id: &Hash, receipt: Option<&ReceiptState>) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(receipt_id);
    match receipt {
        Some(receipt) => {
            encoded.push(1);
            encoded.push(primitive_type_tag(receipt.primitive_type()));
            encoded.extend_from_slice(&receipt.receipt_id());
            encoded.extend_from_slice(&receipt.job_id());
            encoded.extend_from_slice(&receipt.miner());
            encoded.extend_from_slice(&receipt.tensor_work_units().to_le_bytes());
            encoded.extend_from_slice(&receipt.estimated_block_bytes().to_le_bytes());
            encoded.extend_from_slice(&receipt.submitted_at_block().to_le_bytes());
        }
        None => encoded.push(0),
    }
    hash_bytes(b"tensor-vm-selected-receipt-leaf-v1", &[&encoded])
}

pub(super) fn block_checks_root(
    selected_receipts: &[Hash],
    receipts: &BTreeMap<Hash, ReceiptState>,
    attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>,
    beacon_round: u64,
    beacon: &Hash,
    parent_hash: &Hash,
) -> Hash {
    merkle_root(&block_check_leaves(
        selected_receipts,
        receipts,
        attestations,
        beacon_round,
        beacon,
        parent_hash,
    ))
}

pub(super) fn block_check_leaves(
    selected_receipts: &[Hash],
    receipts: &BTreeMap<Hash, ReceiptState>,
    attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>,
    beacon_round: u64,
    beacon: &Hash,
    parent_hash: &Hash,
) -> Vec<Hash> {
    selected_receipts
        .iter()
        .map(|receipt_id| {
            block_check_leaf(
                receipt_id,
                receipts.get(receipt_id),
                attestations,
                beacon_round,
                beacon,
                parent_hash,
            )
        })
        .collect()
}

pub(super) fn block_check_leaf(
    receipt_id: &Hash,
    receipt: Option<&ReceiptState>,
    attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>,
    beacon_round: u64,
    beacon: &Hash,
    parent_hash: &Hash,
) -> Hash {
    let receipt_checks_root =
        canonical_receipt_checks_root(receipt_id, attestations.get(receipt_id));
    let check_seed = block_check_seed(beacon_round, beacon, parent_hash, receipt_id);
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&beacon_round.to_le_bytes());
    encoded.extend_from_slice(beacon);
    encoded.extend_from_slice(parent_hash);
    encoded.extend_from_slice(&check_seed);
    encoded.extend_from_slice(receipt_id);
    encoded.extend_from_slice(&selected_receipt_leaf(receipt_id, receipt));
    encoded.extend_from_slice(&receipt_checks_root);
    if let Some(receipt) = receipt {
        encoded.push(primitive_type_tag(receipt.primitive_type()));
        encoded.extend_from_slice(&receipt.tensor_work_units().to_le_bytes());
        encoded.extend_from_slice(&receipt.estimated_block_bytes().to_le_bytes());
    }
    hash_bytes(b"tensor-vm-block-check-leaf-v1", &[&encoded])
}

pub(super) fn block_check_seed(
    beacon_round: u64,
    beacon: &Hash,
    parent_hash: &Hash,
    receipt_id: &Hash,
) -> Hash {
    hash_bytes(
        b"tensor-vm-block-check-seed-v1",
        &[
            &beacon_round.to_le_bytes(),
            beacon,
            parent_hash,
            receipt_id,
            b"checks",
        ],
    )
}

fn canonical_receipt_checks_root(
    receipt_id: &Hash,
    attestations: Option<&Vec<ValidatorAttestation>>,
) -> Hash {
    let mut roots = BTreeSet::new();
    for attestation in attestations.into_iter().flatten() {
        if attestation.result == VerificationResult::Valid
            && attestation.data_availability_passed
            && attestation.verify_signature()
            && attestation.receipt_id == *receipt_id
        {
            roots.insert(attestation.checks_root);
        }
    }
    let mut encoded = Vec::new();
    encoded.extend_from_slice(receipt_id);
    for checks_root in roots {
        encoded.extend_from_slice(&checks_root);
    }
    hash_bytes(b"tensor-vm-receipt-checks-root", &[&encoded])
}

pub(super) fn hash_set_root(domain: &[u8], items: &BTreeSet<Hash>) -> Hash {
    let mut encoded = Vec::new();
    for item in items {
        encoded.extend_from_slice(item);
    }
    hash_bytes(domain, &[&encoded])
}

pub(super) fn model_state_root(models: &BTreeMap<Hash, ModelState>) -> Hash {
    let mut encoded = Vec::new();
    for (model_id, model) in models {
        encoded.extend_from_slice(model_id);
        encoded.extend_from_slice(&model.model_id);
        encoded.extend_from_slice(&model.architecture_hash);
        encoded.extend_from_slice(&model.weight_root);
        match model.optimizer_state_root {
            Some(root) => {
                encoded.push(1);
                encoded.extend_from_slice(&root);
            }
            None => encoded.push(0),
        }
        encoded.extend_from_slice(&model.step.to_le_bytes());
        encoded.extend_from_slice(&model.config_hash);
    }
    hash_bytes(b"tensor-vm-model-state-root-v1", &[&encoded])
}

fn encode_hashes(out: &mut Vec<u8>, hashes: &[Hash]) {
    out.extend_from_slice(&(hashes.len() as u64).to_le_bytes());
    for hash in hashes {
        out.extend_from_slice(hash);
    }
}

fn encode_usizes(out: &mut Vec<u8>, values: &[usize]) {
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        encode_usize(out, *value);
    }
}

fn encode_usize(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u64).to_le_bytes());
}
