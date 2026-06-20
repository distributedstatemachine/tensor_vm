use super::{BlockVote, Chain, blocks};
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
        && let Some(miner) = chain.state.miners.get_mut(&receipt_miner)
    {
        miner.reputation -= 1;
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

fn is_assigned_validator(chain: &Chain, validator: Address, receipt_id: Hash) -> bool {
    assigned_validators(chain, receipt_id).contains(&validator)
}

fn assigned_validators(chain: &Chain, receipt_id: Hash) -> BTreeSet<Address> {
    let assignment_seed = assignment_seed(
        chain.state.finalized_beacon_round,
        &chain.state.finalized_randomness,
        &receipt_id,
    );
    JobScheduler::default()
        .assign_validators(chain, receipt_id, &assignment_seed)
        .validators
        .into_iter()
        .collect()
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
