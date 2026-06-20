use super::{
    BlockVote, Chain, ReceiptRewardKind, ValidatorAuditAssignment, ValidatorAuditReport,
    ValidatorAuditResult, ValidatorAuditSlashRecord, blocks,
};
use crate::error::{Result, TvmError};
use crate::scheduler::JobScheduler;
use crate::types::{Address, Hash, hash_bytes};
use crate::verify::{ValidatorAttestation, VerificationResult};
use std::collections::BTreeSet;

pub fn submit_attestation(chain: &mut Chain, attestation: ValidatorAttestation) -> Result<()> {
    let validator_stake = chain
        .state
        .validators
        .get(&attestation.validator)
        .ok_or(TvmError::UnknownValidator)?
        .stake;
    if attestation.stake != validator_stake {
        return Err(TvmError::InvalidReceipt("attestation stake mismatch"));
    }
    if !attestation.verify_signature() {
        return Err(TvmError::InvalidReceipt("bad attestation signature"));
    }
    if !is_assigned_validator(chain, attestation.validator, attestation.receipt_id) {
        return Err(TvmError::InvalidReceipt(
            "validator not assigned to receipt",
        ));
    }
    let (receipt_job_id, receipt_primitive_type, receipt_miner) = {
        let receipt = chain
            .state
            .receipts
            .get(&attestation.receipt_id)
            .ok_or(TvmError::UnknownReceipt)?;
        (receipt.job_id(), receipt.primitive_type(), receipt.miner())
    };
    if attestation.job_id != receipt_job_id || attestation.primitive_type != receipt_primitive_type
    {
        if let Some(validator) = chain.state.validators.get_mut(&attestation.validator) {
            validator.reputation -= 1;
        }
        return Err(TvmError::InvalidReceipt("attestation receipt mismatch"));
    }
    if chain
        .state
        .attestations
        .get(&attestation.receipt_id)
        .is_some_and(|items| {
            items
                .iter()
                .any(|existing| existing.validator == attestation.validator)
        })
    {
        return Err(TvmError::InvalidReceipt("duplicate validator attestation"));
    }
    if attestation.result == VerificationResult::Valid
        && let Some(validator) = chain.state.validators.get_mut(&attestation.validator)
    {
        validator.valid_attestations += 1;
    }
    if (attestation.result == VerificationResult::Unavailable
        || !attestation.data_availability_passed)
        && chain
            .state
            .data_unavailable_receipts
            .insert(attestation.receipt_id)
    {
        if let Some(miner) = chain.state.miners.get_mut(&receipt_miner) {
            miner.reputation -= 1;
        }
        for reward in chain.state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == attestation.receipt_id {
                reward.voided_by_challenge = true;
            }
        }
    }
    chain
        .state
        .attestations
        .entry(attestation.receipt_id)
        .or_default()
        .push(attestation);
    Ok(())
}

pub fn has_attestation_quorum(chain: &Chain, receipt_id: &Hash) -> bool {
    let attestations = match chain.state.attestations.get(receipt_id) {
        Some(attestations) => attestations,
        None => return false,
    };
    let receipt = match chain.state.receipts.get(receipt_id) {
        Some(receipt) => receipt,
        None => return false,
    };
    let mut valid_count = 0_usize;
    let mut valid_stake = 0_u64;
    let mut seen_validators = BTreeSet::new();
    let assigned_validators = assigned_validators(chain, *receipt_id);
    let assigned_stake: u64 = assigned_validators
        .iter()
        .filter_map(|validator| chain.state.validators.get(validator))
        .map(|validator| validator.stake)
        .sum();
    for attestation in attestations {
        if !assigned_validators.contains(&attestation.validator) {
            continue;
        }
        if !seen_validators.insert(attestation.validator) {
            continue;
        }
        if attestation.result == VerificationResult::Valid
            && attestation.data_availability_passed
            && attestation.verify_signature()
            && attestation.job_id == receipt.job_id()
            && attestation.primitive_type == receipt.primitive_type()
        {
            valid_count += 1;
            valid_stake = valid_stake.saturating_add(attestation.stake);
        }
    }
    let stake_num = chain.params.freivalds.minimum_stake_numerator;
    let stake_den = chain.params.freivalds.minimum_stake_denominator.max(1);
    valid_count >= chain.params.freivalds.minimum_validators
        && valid_stake.saturating_mul(stake_den) >= assigned_stake.saturating_mul(stake_num)
}

pub fn submit_validator_audit_report(
    chain: &mut Chain,
    report: ValidatorAuditReport,
) -> Result<(ValidatorAuditResult, Option<ValidatorAuditSlashRecord>)> {
    if !chain.state.validators.contains_key(&report.auditor) {
        return Err(TvmError::UnknownValidator);
    }
    if !report.verify_signature() {
        return Err(TvmError::InvalidReceipt("bad validator audit signature"));
    }
    let assignment = chain
        .state
        .validator_audit_assignments
        .get(&report.audit_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown validator audit"))?;
    if chain.state.height > assignment.deadline_height {
        return Err(TvmError::InvalidReceipt("validator audit deadline expired"));
    }
    if chain
        .state
        .validator_audit_results
        .contains_key(&report.audit_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate validator audit result"));
    }
    if chain
        .state
        .validator_audit_slashes
        .contains_key(&report.audit_id)
    {
        return Err(TvmError::InvalidReceipt("validator audit already slashed"));
    }
    let audited_attestation = chain
        .state
        .attestations
        .get(&assignment.receipt_id)
        .and_then(|items| {
            items
                .iter()
                .find(|attestation| attestation.validator == assignment.validator)
        })
        .ok_or(TvmError::InvalidReceipt(
            "audited validator attestation missing",
        ))?;
    let passed = audited_attestation.result == report.canonical_result
        && audited_attestation.data_availability_passed
            == report.canonical_data_availability_passed;
    let result = ValidatorAuditResult {
        audit_id: report.audit_id,
        receipt_id: assignment.receipt_id,
        validator: assignment.validator,
        auditor: report.auditor,
        attested_result: audited_attestation.result,
        canonical_result: report.canonical_result,
        attested_data_availability_passed: audited_attestation.data_availability_passed,
        canonical_data_availability_passed: report.canonical_data_availability_passed,
        checks_root: report.checks_root,
        submitted_at_height: chain.state.height,
        passed,
        signature: report.signature,
    };
    chain
        .state
        .validator_audit_results
        .insert(result.audit_id, result.clone());
    let slash = if passed {
        None
    } else {
        Some(apply_validator_audit_slash(
            chain,
            &assignment,
            report.auditor,
            "validator audit contradicted attestation",
        ))
    };
    Ok((result, slash))
}

pub(super) fn apply_validator_audit_slash(
    chain: &mut Chain,
    assignment: &ValidatorAuditAssignment,
    auditor: Address,
    reason: &str,
) -> ValidatorAuditSlashRecord {
    let Some(validator) = chain.state.validators.get_mut(&assignment.validator) else {
        return ValidatorAuditSlashRecord {
            audit_id: assignment.audit_id,
            receipt_id: assignment.receipt_id,
            validator: assignment.validator,
            auditor,
            amount: 0,
            slashed_at_height: chain.state.height,
            reason: reason.to_owned(),
        };
    };
    let amount = validator
        .stake
        .min(chain.params.validator_audit_slash_amount);
    validator.stake = validator.stake.saturating_sub(amount);
    validator.reputation -= 10;
    if reason == "validator missed mandatory audit" {
        validator.missed_assignments = validator.missed_assignments.saturating_add(1);
    }
    void_validator_audit_reward(chain, &assignment.receipt_id, &assignment.validator);
    chain.state.rewards.credit_treasury(amount);
    let record = ValidatorAuditSlashRecord {
        audit_id: assignment.audit_id,
        receipt_id: assignment.receipt_id,
        validator: assignment.validator,
        auditor,
        amount,
        slashed_at_height: chain.state.height,
        reason: reason.to_owned(),
    };
    chain
        .state
        .validator_audit_slashes
        .insert(assignment.audit_id, record.clone());
    record
}

fn void_validator_audit_reward(chain: &mut Chain, receipt_id: &Hash, validator: &Address) {
    for reward in chain.state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == *receipt_id
            && reward.beneficiary == *validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            reward.voided_by_challenge = true;
        }
    }
}

pub(super) fn validator_audit_id(receipt_id: &Hash, validator: &Address) -> Hash {
    hash_bytes(b"tensor-vm-validator-audit-id-v1", &[receipt_id, validator])
}

pub(super) fn validator_audit_seed(
    beacon_round: u64,
    beacon: &Hash,
    receipt_id: &Hash,
    validator: &Address,
) -> Hash {
    hash_bytes(
        b"tensor-vm-validator-audit-seed-v1",
        &[&beacon_round.to_le_bytes(), beacon, receipt_id, validator],
    )
}

fn is_assigned_validator(chain: &Chain, validator: Address, receipt_id: Hash) -> bool {
    assigned_validators(chain, receipt_id).contains(&validator)
}

fn assigned_validators(chain: &Chain, receipt_id: Hash) -> BTreeSet<Address> {
    let assignment_seed = receipt_assignment_seed(chain, &receipt_id);
    JobScheduler::default()
        .assign_validators(chain, receipt_id, &assignment_seed)
        .validators
        .into_iter()
        .collect()
}

pub(super) fn receipt_assignment_seed(chain: &Chain, receipt_id: &Hash) -> Hash {
    chain
        .state
        .receipt_randomness_anchors
        .get(receipt_id)
        .map_or_else(
            || {
                assignment_seed(
                    chain.state.finalized_beacon_round,
                    &chain.state.finalized_randomness,
                    receipt_id,
                )
            },
            |anchor| anchor.assignment_seed,
        )
}

pub fn submit_block_vote(chain: &mut Chain, vote: BlockVote) -> Result<()> {
    let validator = chain
        .state
        .validators
        .get(&vote.validator)
        .ok_or(TvmError::UnknownValidator)?;
    if validator.stake != vote.stake {
        return Err(TvmError::InvalidReceipt("block vote stake mismatch"));
    }
    if !vote.verify_signature() {
        return Err(TvmError::InvalidReceipt("bad block vote signature"));
    }
    let Some(block) = chain
        .blocks
        .iter()
        .find(|block| block.height == vote.block_height && block.hash() == vote.block_hash)
        .cloned()
    else {
        return Err(TvmError::InvalidReceipt("unknown block"));
    };
    blocks::validate(chain, &block, true)?;
    if chain
        .state
        .block_votes
        .get(&vote.block_hash)
        .is_some_and(|votes| {
            votes
                .iter()
                .any(|existing| existing.validator == vote.validator)
        })
    {
        return Err(TvmError::InvalidReceipt("duplicate block vote"));
    }

    let block_hash = vote.block_hash;
    chain
        .state
        .block_votes
        .entry(block_hash)
        .or_default()
        .push(vote);
    if has_block_finality(chain, &block_hash) {
        chain.state.finalized_blocks.insert(block_hash);
    }
    Ok(())
}

pub fn has_block_finality(chain: &Chain, block_hash: &Hash) -> bool {
    let total_stake: u64 = chain
        .state
        .validators
        .values()
        .map(|validator| validator.stake)
        .sum();
    if total_stake == 0 {
        return false;
    }
    let mut seen_validators = BTreeSet::new();
    let mut signed_stake = 0_u64;
    for vote in chain
        .state
        .block_votes
        .get(block_hash)
        .into_iter()
        .flatten()
    {
        let Some(validator) = chain.state.validators.get(&vote.validator) else {
            continue;
        };
        if validator.stake != vote.stake {
            continue;
        }
        if !seen_validators.insert(vote.validator) {
            continue;
        }
        if vote.verify_signature() {
            signed_stake = signed_stake.saturating_add(vote.stake);
        }
    }
    let numerator = chain.params.finality_stake_numerator;
    let denominator = chain.params.finality_stake_denominator.max(1);
    signed_stake.saturating_mul(denominator) >= total_stake.saturating_mul(numerator)
}

pub fn assignment_seed(beacon_round: u64, finalized_randomness: &Hash, receipt_id: &Hash) -> Hash {
    hash_bytes(
        b"tensor-vm-validator-assignment-seed-v1",
        &[
            &beacon_round.to_le_bytes(),
            finalized_randomness,
            receipt_id,
        ],
    )
}

pub fn miner_assignment_seed(
    beacon_round: u64,
    finalized_randomness: &Hash,
    job_id: &Hash,
) -> Hash {
    hash_bytes(
        b"tensor-vm-miner-assignment-seed-v1",
        &[&beacon_round.to_le_bytes(), finalized_randomness, job_id],
    )
}

pub fn seed(
    beacon_round: u64,
    finalized_randomness: &Hash,
    receipt_id: &Hash,
    job_id: &Hash,
    validator: &Address,
    validation_round: u64,
) -> Hash {
    hash_bytes(
        b"tensor-vm-validation-seed-v1",
        &[
            &beacon_round.to_le_bytes(),
            finalized_randomness,
            receipt_id,
            job_id,
            validator,
            &validation_round.to_le_bytes(),
        ],
    )
}
