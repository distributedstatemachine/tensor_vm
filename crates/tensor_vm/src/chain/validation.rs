use super::{
    BlockVote, Chain, ExternalRandomnessBeaconProof, ExternalRandomnessBeaconRecord,
    InvalidOutputSlashRecord, ReceiptRewardKind, ValidatorAuditAppeal, ValidatorAuditAppealRecord,
    ValidatorAuditAppealResolution, ValidatorAuditAssignment, ValidatorAuditReport,
    ValidatorAuditResult, ValidatorAuditSlashRecord, ValidatorVrfRevealRecord, blocks, settlement,
};
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::scheduler::JobScheduler;
use crate::types::{Address, Hash, hash_bytes, sign, verify_signature};
use crate::verify::{ValidatorAttestation, VerificationResult};
use drand_verify::{G1Pubkey, Pubkey, derive_randomness};
use std::collections::BTreeSet;

const VALIDATOR_AUDIT_APPEAL_REASON_MAX_BYTES: usize = 256;
const EXTERNAL_RANDOMNESS_SOURCE_ID_MAX_BYTES: usize = 96;
const DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES: usize = 48;
const DRAND_PEDERSEN_BLS_SIGNATURE_BYTES: usize = 96;
pub const RANDOMNESS_BEACON_SOURCE: &str = "local_finalized_chain_beacon_v1";
pub const RANDOMNESS_DRAND_ROUND_MAPPING: &str =
    "local_finalized_height_to_beacon_round_v1:round=receipt_submission_finalized_beacon_round";
pub const RANDOMNESS_VRF_CONSTRUCTION: &str =
    "local_validator_vrf_seed_v1:H(commitment,receipt_id,job_id,validator,round)";
pub const ASSIGNMENT_SEED_DOMAIN: &str = "tensor-vm-validator-assignment-seed-v1";
pub const VALIDATION_SEED_COMMITMENT_DOMAIN: &str = "tensor-vm-validation-seed-commitment-v1";
pub const VALIDATION_SEED_REVEAL_DOMAIN: &str = "tensor-vm-committed-validation-seed-v1";
pub const VALIDATOR_VRF_REVEAL_ID_DOMAIN: &str = "tensor-vm-validator-vrf-reveal-id-v1";
pub const VALIDATOR_VRF_PROOF_DOMAIN: &str = "tensor-vm-validator-vrf-proof-v1";
pub const VALIDATOR_VRF_SIGNATURE_DOMAIN: &str = "tensor-vm-validator-vrf-signature-v1";

pub fn submit_external_randomness_beacon(
    chain: &mut Chain,
    source_id: String,
    beacon_round: u64,
    randomness: Hash,
    proof_hash: Hash,
) -> Result<ExternalRandomnessBeaconRecord> {
    record_external_randomness_beacon(
        chain,
        source_id,
        beacon_round,
        randomness,
        proof_hash,
        ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1,
    )
}

pub fn submit_verified_drand_beacon(
    chain: &mut Chain,
    source_id: String,
    beacon_round: u64,
    public_key: Vec<u8>,
    signature: Vec<u8>,
) -> Result<ExternalRandomnessBeaconRecord> {
    let record = verified_drand_beacon_record(
        source_id,
        beacon_round,
        &public_key,
        &signature,
        chain.state.height,
    )?;
    record_external_randomness_beacon(
        chain,
        record.source_id,
        record.beacon_round,
        record.randomness,
        record.proof_hash,
        record.proof,
    )
}

pub fn verified_drand_beacon_record(
    source_id: String,
    beacon_round: u64,
    public_key: &[u8],
    signature: &[u8],
    observed_at_height: u64,
) -> Result<ExternalRandomnessBeaconRecord> {
    if public_key.len() != DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES {
        return Err(TvmError::InvalidReceipt("drand public key length mismatch"));
    }
    if signature.len() != DRAND_PEDERSEN_BLS_SIGNATURE_BYTES {
        return Err(TvmError::InvalidReceipt("drand signature length mismatch"));
    }
    let mut public_key_bytes = [0; DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES];
    public_key_bytes.copy_from_slice(&public_key);
    let public_key = G1Pubkey::from_fixed(public_key_bytes)
        .map_err(|_| TvmError::InvalidReceipt("invalid drand public key"))?;
    let verified = public_key
        .verify(beacon_round, b"", &signature)
        .map_err(|_| TvmError::InvalidReceipt("invalid drand signature point"))?;
    if !verified {
        return Err(TvmError::InvalidReceipt(
            "drand signature verification failed",
        ));
    }
    let randomness = derive_randomness(&signature);
    let public_key_hash = hash_bytes(
        b"tensor-vm-drand-pedersen-bls-unchained-public-key-v1",
        &[&public_key_bytes],
    );
    let signature_hash = hash_bytes(
        b"tensor-vm-drand-pedersen-bls-unchained-signature-v1",
        &[&signature],
    );
    let proof_hash = hash_bytes(
        b"tensor-vm-drand-pedersen-bls-unchained-proof-v1",
        &[
            source_id.as_bytes(),
            &beacon_round.to_le_bytes(),
            &public_key_hash,
            &signature_hash,
            &randomness,
        ],
    );
    Ok(ExternalRandomnessBeaconRecord {
        source_id,
        beacon_round,
        randomness,
        proof_hash,
        proof: ExternalRandomnessBeaconProof::DrandPedersenBlsUnchainedV1 {
            public_key_hash,
            signature_hash,
            public_key_len: DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES as u64,
            signature_len: DRAND_PEDERSEN_BLS_SIGNATURE_BYTES as u64,
        },
        observed_at_height,
    })
}

pub fn verified_drand_source_id(public_key: &[u8]) -> String {
    let public_key_hash = hash_bytes(
        b"tensor-vm-drand-pedersen-bls-unchained-public-key-v1",
        &[public_key],
    );
    format!("drand-pedersen-bls-unchained-v1:{}", hex(&public_key_hash))
}

fn record_external_randomness_beacon(
    chain: &mut Chain,
    source_id: String,
    beacon_round: u64,
    randomness: Hash,
    proof_hash: Hash,
    proof: ExternalRandomnessBeaconProof,
) -> Result<ExternalRandomnessBeaconRecord> {
    if source_id.is_empty() || source_id.len() > EXTERNAL_RANDOMNESS_SOURCE_ID_MAX_BYTES {
        return Err(TvmError::InvalidReceipt(
            "external randomness source id out of bounds",
        ));
    }
    if beacon_round <= chain.state.finalized_beacon_round {
        return Err(TvmError::InvalidReceipt(
            "external randomness beacon round is not newer",
        ));
    }
    if randomness == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "external randomness beacon value is empty",
        ));
    }
    if chain
        .state
        .external_randomness_beacons
        .contains_key(&beacon_round)
    {
        return Err(TvmError::InvalidReceipt(
            "external randomness beacon round already recorded",
        ));
    }
    let record = ExternalRandomnessBeaconRecord {
        source_id,
        beacon_round,
        randomness,
        proof_hash,
        proof,
        observed_at_height: chain.state.height,
    };
    chain.state.finalized_beacon_round = beacon_round;
    chain.state.finalized_randomness = randomness;
    chain
        .state
        .external_randomness_beacons
        .insert(beacon_round, record.clone());
    Ok(record)
}

pub fn submit_validator_vrf_reveal(
    chain: &mut Chain,
    mut reveal: ValidatorVrfRevealRecord,
) -> Result<ValidatorVrfRevealRecord> {
    if !chain.state.validators.contains_key(&reveal.validator) {
        return Err(TvmError::UnknownValidator);
    }
    let receipt = chain
        .state
        .receipts
        .get(&reveal.receipt_id)
        .ok_or(TvmError::UnknownReceipt)?;
    if receipt.job_id() != reveal.job_id {
        return Err(TvmError::InvalidReceipt(
            "validator vrf reveal job mismatch",
        ));
    }
    let anchor = chain
        .state
        .receipt_randomness_anchors
        .get(&reveal.receipt_id)
        .ok_or(TvmError::InvalidReceipt(
            "receipt randomness anchor missing for validator vrf reveal",
        ))?;
    if anchor.beacon_round != reveal.beacon_round {
        return Err(TvmError::InvalidReceipt(
            "validator vrf reveal beacon round mismatch",
        ));
    }
    let expected_output = committed_seed(
        &anchor.validation_seed_commitment,
        &reveal.receipt_id,
        &reveal.job_id,
        &reveal.validator,
        reveal.validation_round,
    );
    if reveal.vrf_output != expected_output {
        return Err(TvmError::InvalidReceipt(
            "validator vrf reveal output mismatch",
        ));
    }
    let expected_proof = validator_vrf_proof_hash(
        &anchor.validation_seed_commitment,
        &reveal.receipt_id,
        &reveal.job_id,
        &reveal.validator,
        reveal.validation_round,
        &reveal.vrf_output,
    );
    if reveal.proof_hash != expected_proof {
        return Err(TvmError::InvalidReceipt(
            "validator vrf reveal proof mismatch",
        ));
    }
    let expected_reveal_id = validator_vrf_reveal_id(
        &reveal.receipt_id,
        &reveal.job_id,
        &reveal.validator,
        reveal.beacon_round,
        reveal.validation_round,
        &reveal.vrf_output,
        &reveal.proof_hash,
    );
    if reveal.reveal_id != expected_reveal_id {
        return Err(TvmError::InvalidReceipt("validator vrf reveal id mismatch"));
    }
    let message = validator_vrf_reveal_signature_message(&reveal);
    if !verify_signature(&reveal.validator, &message, &reveal.signature) {
        return Err(TvmError::InvalidReceipt(
            "bad validator vrf reveal signature",
        ));
    }
    if chain
        .state
        .validator_vrf_reveals
        .contains_key(&reveal.reveal_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate validator vrf reveal"));
    }
    reveal.observed_at_height = chain.state.height;
    chain
        .state
        .validator_vrf_reveals
        .insert(reveal.reveal_id, reveal.clone());
    for reward in chain.state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == reveal.receipt_id
            && reward.beneficiary == reveal.validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            reward.mark_validator_vrf_revealed();
        }
    }
    Ok(reveal)
}

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
    let (receipt_job_id, receipt_primitive_type, receipt_miner) = {
        let receipt = chain
            .state
            .receipts
            .get(&attestation.receipt_id)
            .ok_or(TvmError::UnknownReceipt)?;
        (receipt.job_id(), receipt.primitive_type(), receipt.miner())
    };
    if !chain
        .state
        .receipt_randomness_anchors
        .contains_key(&attestation.receipt_id)
    {
        return Err(TvmError::InvalidReceipt(
            "receipt randomness anchor missing",
        ));
    }
    if !is_assigned_validator(chain, attestation.validator, attestation.receipt_id) {
        return Err(TvmError::InvalidReceipt(
            "validator not assigned to receipt",
        ));
    }
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
        settlement::void_pending_miner_tensor_work(&mut chain.state, &attestation.receipt_id);
        let receipt_reward_hold_until_height = chain
            .state
            .height
            .saturating_add(chain.params.reward_maturity_delay_blocks());
        for reward in chain.state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == attestation.receipt_id {
                reward.delay_until(receipt_reward_hold_until_height);
                reward.voided_by_challenge = true;
            }
        }
    }
    if attestation.result == VerificationResult::Invalid
        && chain
            .state
            .challenged_receipts
            .insert(attestation.receipt_id)
    {
        chain.state.settled_receipts.remove(&attestation.receipt_id);
        if let Some(miner) = chain.state.miners.get_mut(&receipt_miner) {
            miner.reputation -= 1;
        }
        settlement::void_pending_miner_tensor_work(&mut chain.state, &attestation.receipt_id);
        apply_invalid_output_slash(
            chain,
            attestation.receipt_id,
            receipt_miner,
            attestation.validator,
        );
        let receipt_reward_hold_until_height = chain
            .state
            .height
            .saturating_add(chain.params.reward_maturity_delay_blocks());
        for reward in chain.state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == attestation.receipt_id {
                reward.delay_until(receipt_reward_hold_until_height);
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

fn apply_invalid_output_slash(
    chain: &mut Chain,
    receipt_id: Hash,
    miner_address: Address,
    evidence_validator: Address,
) {
    if chain.state.invalid_output_slashes.contains_key(&receipt_id) {
        return;
    }
    let Some(miner) = chain.state.miners.get_mut(&miner_address) else {
        return;
    };
    let actual_slash = miner
        .stake
        .min(chain.params.invalid_output_miner_slash_amount);
    miner.stake = miner.stake.saturating_sub(actual_slash);
    chain.state.rewards.credit_treasury(actual_slash);
    chain.state.invalid_output_slashes.insert(
        receipt_id,
        InvalidOutputSlashRecord {
            receipt_id,
            miner: miner_address,
            evidence_validator,
            amount: actual_slash,
            slashed_at_height: chain.state.height,
            reason: "invalid output for receipt verification".to_owned(),
        },
    );
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
    if report.auditor != assignment.auditor {
        return Err(TvmError::InvalidReceipt("validator audit auditor mismatch"));
    }
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

pub fn submit_validator_audit_appeal(
    chain: &mut Chain,
    appeal: ValidatorAuditAppeal,
) -> Result<ValidatorAuditAppealRecord> {
    if !chain.state.validators.contains_key(&appeal.validator) {
        return Err(TvmError::UnknownValidator);
    }
    if appeal.reason.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "validator audit appeal reason empty",
        ));
    }
    if appeal.reason.len() > VALIDATOR_AUDIT_APPEAL_REASON_MAX_BYTES {
        return Err(TvmError::InvalidReceipt(
            "validator audit appeal reason too long",
        ));
    }
    if !appeal.verify_signature() {
        return Err(TvmError::InvalidReceipt(
            "bad validator audit appeal signature",
        ));
    }
    if chain
        .state
        .validator_audit_appeals
        .contains_key(&appeal.audit_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate validator audit appeal"));
    }
    let slash = chain
        .state
        .validator_audit_slashes
        .get(&appeal.audit_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown validator audit slash"))?;
    if appeal.validator != slash.validator {
        return Err(TvmError::InvalidReceipt(
            "validator audit appeal signer mismatch",
        ));
    }
    let deadline_height = slash
        .slashed_at_height
        .saturating_add(chain.params.validator_audit_window_blocks.max(1));
    if chain.state.height > deadline_height {
        return Err(TvmError::InvalidReceipt("validator audit appeal expired"));
    }
    let record = ValidatorAuditAppealRecord {
        audit_id: slash.audit_id,
        receipt_id: slash.receipt_id,
        validator: slash.validator,
        auditor: slash.auditor,
        slash_amount: slash.amount,
        appealed_at_height: chain.state.height,
        deadline_height,
        reason: appeal.reason,
        signature: appeal.signature,
        resolved_at_height: None,
        resolution: None,
        stake_refunded_amount: 0,
    };
    chain
        .state
        .validator_audit_appeals
        .insert(record.audit_id, record.clone());
    Ok(record)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ValidatorAuditAppealResolutionOutcome {
    pub validator: crate::types::Address,
    pub receipt_reward_reinstated: bool,
    pub stake_refunded_amount: u64,
}

pub fn resolve_validator_audit_appeal(
    chain: &mut Chain,
    audit_id: crate::types::Hash,
    resolution: ValidatorAuditAppealResolution,
) -> Result<ValidatorAuditAppealResolutionOutcome> {
    let appeal = chain
        .state
        .validator_audit_appeals
        .get(&audit_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown validator audit appeal"))?;
    if appeal.resolution.is_some() {
        return Err(TvmError::InvalidReceipt(
            "validator audit appeal already resolved",
        ));
    }
    let slash = chain
        .state
        .validator_audit_slashes
        .get(&audit_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown validator audit slash"))?;
    if slash.validator != appeal.validator || slash.receipt_id != appeal.receipt_id {
        return Err(TvmError::InvalidReceipt("validator audit appeal mismatch"));
    }

    let mut receipt_reward_reinstated = false;
    let mut matched_reward = false;
    for reward in chain.state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == appeal.receipt_id
            && reward.beneficiary == appeal.validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            matched_reward = true;
            reward.delay_until(appeal.deadline_height);
            if resolution == ValidatorAuditAppealResolution::ReverseRewardVoid {
                reward.voided_by_challenge = false;
                receipt_reward_reinstated = true;
            }
        }
    }
    if !matched_reward {
        return Err(TvmError::InvalidReceipt(
            "validator audit appeal reward claim missing",
        ));
    }
    let stake_refunded_amount = if resolution == ValidatorAuditAppealResolution::ReverseRewardVoid {
        let refunded = chain.state.rewards.debit_treasury(slash.amount);
        if let Some(validator) = chain.state.validators.get_mut(&appeal.validator) {
            validator.stake = validator.stake.saturating_add(refunded);
        }
        refunded
    } else {
        0
    };
    if let Some(record) = chain.state.validator_audit_appeals.get_mut(&audit_id) {
        record.resolved_at_height = Some(chain.state.height);
        record.resolution = Some(resolution);
        record.stake_refunded_amount = stake_refunded_amount;
    }

    Ok(ValidatorAuditAppealResolutionOutcome {
        validator: appeal.validator,
        receipt_reward_reinstated,
        stake_refunded_amount,
    })
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
    let reward_hold_until_height = chain
        .state
        .height
        .saturating_add(chain.params.validator_audit_window_blocks.max(1));
    void_validator_audit_reward(
        chain,
        &assignment.receipt_id,
        &assignment.validator,
        reward_hold_until_height,
    );
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

fn void_validator_audit_reward(
    chain: &mut Chain,
    receipt_id: &Hash,
    validator: &Address,
    claimable_at_height: u64,
) {
    for reward in chain.state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == *receipt_id
            && reward.beneficiary == *validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            reward.delay_until(claimable_at_height);
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
    let side_branch_vote = chain.side_branch_blocks.contains_key(&vote.block_hash);
    let Some(block) = chain
        .blocks
        .iter()
        .find(|block| block.height == vote.block_height && block.hash() == vote.block_hash)
        .or_else(|| {
            chain
                .side_branch_blocks
                .get(&vote.block_hash)
                .filter(|block| block.height == vote.block_height)
        })
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
        if side_branch_vote {
            blocks::try_promote_side_branch_or_descendant(chain, block_hash);
        }
        if chain
            .blocks
            .last()
            .is_some_and(|block| block.hash() == block_hash)
        {
            blocks::materialize_finalized_proposer_rewards(
                &mut chain.state,
                &chain.blocks,
                &chain.params,
            );
        }
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
        ASSIGNMENT_SEED_DOMAIN.as_bytes(),
        &[
            &beacon_round.to_le_bytes(),
            finalized_randomness,
            receipt_id,
        ],
    )
}

pub fn validation_seed_commitment(
    beacon_round: u64,
    finalized_randomness: &Hash,
    receipt_id: &Hash,
) -> Hash {
    hash_bytes(
        VALIDATION_SEED_COMMITMENT_DOMAIN.as_bytes(),
        &[
            &beacon_round.to_le_bytes(),
            finalized_randomness,
            receipt_id,
        ],
    )
}

pub fn committed_seed(
    validation_seed_commitment: &Hash,
    receipt_id: &Hash,
    job_id: &Hash,
    validator: &Address,
    validation_round: u64,
) -> Hash {
    hash_bytes(
        VALIDATION_SEED_REVEAL_DOMAIN.as_bytes(),
        &[
            validation_seed_commitment,
            receipt_id,
            job_id,
            validator,
            &validation_round.to_le_bytes(),
        ],
    )
}

pub fn validator_vrf_proof_hash(
    validation_seed_commitment: &Hash,
    receipt_id: &Hash,
    job_id: &Hash,
    validator: &Address,
    validation_round: u64,
    vrf_output: &Hash,
) -> Hash {
    hash_bytes(
        VALIDATOR_VRF_PROOF_DOMAIN.as_bytes(),
        &[
            validation_seed_commitment,
            receipt_id,
            job_id,
            validator,
            &validation_round.to_le_bytes(),
            vrf_output,
        ],
    )
}

pub fn validator_vrf_reveal_id(
    receipt_id: &Hash,
    job_id: &Hash,
    validator: &Address,
    beacon_round: u64,
    validation_round: u64,
    vrf_output: &Hash,
    proof_hash: &Hash,
) -> Hash {
    hash_bytes(
        VALIDATOR_VRF_REVEAL_ID_DOMAIN.as_bytes(),
        &[
            receipt_id,
            job_id,
            validator,
            &beacon_round.to_le_bytes(),
            &validation_round.to_le_bytes(),
            vrf_output,
            proof_hash,
        ],
    )
}

pub fn validator_vrf_reveal_signature_message(reveal: &ValidatorVrfRevealRecord) -> Hash {
    hash_bytes(
        VALIDATOR_VRF_SIGNATURE_DOMAIN.as_bytes(),
        &[
            &reveal.reveal_id,
            &reveal.receipt_id,
            &reveal.job_id,
            &reveal.validator,
            &reveal.beacon_round.to_le_bytes(),
            &reveal.validation_round.to_le_bytes(),
            &reveal.vrf_output,
            &reveal.proof_hash,
        ],
    )
}

pub fn validator_vrf_reveal_record(
    chain: &Chain,
    receipt_id: Hash,
    validator: Address,
    validation_round: u64,
) -> Result<ValidatorVrfRevealRecord> {
    let receipt = chain
        .state()
        .receipts()
        .get(&receipt_id)
        .ok_or(TvmError::UnknownReceipt)?;
    let anchor = chain
        .state()
        .receipt_randomness_anchors()
        .get(&receipt_id)
        .ok_or(TvmError::InvalidReceipt(
            "receipt randomness anchor missing for validator vrf reveal",
        ))?;
    let job_id = receipt.job_id();
    let vrf_output = committed_seed(
        &anchor.validation_seed_commitment,
        &receipt_id,
        &job_id,
        &validator,
        validation_round,
    );
    let proof_hash = validator_vrf_proof_hash(
        &anchor.validation_seed_commitment,
        &receipt_id,
        &job_id,
        &validator,
        validation_round,
        &vrf_output,
    );
    let reveal_id = validator_vrf_reveal_id(
        &receipt_id,
        &job_id,
        &validator,
        anchor.beacon_round,
        validation_round,
        &vrf_output,
        &proof_hash,
    );
    let mut reveal = ValidatorVrfRevealRecord {
        reveal_id,
        receipt_id,
        job_id,
        validator,
        beacon_round: anchor.beacon_round,
        validation_round,
        vrf_output,
        proof_hash,
        signature: [0; 32],
        observed_at_height: chain.state().height(),
    };
    let message = validator_vrf_reveal_signature_message(&reveal);
    reveal.signature = sign(&validator, &message);
    Ok(reveal)
}

pub fn missing_anchor_seed(receipt_id: &Hash, validator: &Address) -> Hash {
    hash_bytes(
        b"tensor-vm-missing-receipt-randomness-anchor-v1",
        &[receipt_id, validator],
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
