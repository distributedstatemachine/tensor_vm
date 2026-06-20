use crate::chain::{
    AccountState, BlockCheckChallengeRecord, BlockVote, Chain, ChainParams, ChainParts, ChainState,
    ChainStateParts, DataUnavailabilitySlashRecord, HardwareClass, JobState, MinerState,
    ModelState, PendingChallengeReward, PendingCreditReward, PendingProposerReward,
    PendingReceiptReward, ReceiptRandomnessAnchor, ReceiptRewardKind, ReceiptState, RewardState,
    ValidatorAuditAssignment, ValidatorAuditResult, ValidatorAuditSlashRecord, ValidatorState,
};
use crate::codec::{self as payload_codec, verification_result_from_tag, verification_result_tag};
use crate::error::{Result, TvmError};
use crate::types::{Hash, hash_bytes};
use crate::verify::{FreivaldsParams, ValidatorAttestation};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::block_log::{BLOCK_PAYLOAD_LEN, decode_block_payload, encode_block_payload};
use super::codec::{
    HASH_LEN, StateReader, write_bytes, write_hash, write_i64, write_len, write_option_hash,
    write_u64,
};

const CHAIN_STATE_MAGIC: &[u8] = b"TENSORVM_STATE\n";
const CHAIN_STATE_DIGEST_LEN: usize = HASH_LEN;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainStateStore {
    path: PathBuf,
}

impl ChainStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_chain(&self, chain: &Chain) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|_| TvmError::Storage("failed to create chain state directory"))?;
        }

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, encode_chain_state_file(chain))
            .map_err(|_| TvmError::Storage("failed to write chain state"))?;
        fs::rename(&temp_path, &self.path)
            .map_err(|_| TvmError::Storage("failed to commit chain state"))?;
        Ok(())
    }

    pub fn load_chain(&self) -> Result<Chain> {
        let bytes =
            fs::read(&self.path).map_err(|_| TvmError::Storage("failed to read chain state"))?;
        decode_chain_state_file(&bytes)
    }
}

fn encode_chain_state_file(chain: &Chain) -> Vec<u8> {
    let payload = encode_chain_state_payload(chain);
    let digest = hash_bytes(b"tensor-vm-state-file-v1", &[&payload]);
    let mut encoded =
        Vec::with_capacity(CHAIN_STATE_MAGIC.len() + payload.len() + CHAIN_STATE_DIGEST_LEN);
    encoded.extend_from_slice(CHAIN_STATE_MAGIC);
    encoded.extend_from_slice(&payload);
    encoded.extend_from_slice(&digest);
    encoded
}

fn decode_chain_state_file(bytes: &[u8]) -> Result<Chain> {
    if !bytes.starts_with(CHAIN_STATE_MAGIC) {
        return Err(TvmError::Storage("invalid chain state magic"));
    }
    if bytes.len() < CHAIN_STATE_MAGIC.len() + CHAIN_STATE_DIGEST_LEN {
        return Err(TvmError::Storage("invalid chain state length"));
    }
    let payload_end = bytes.len() - CHAIN_STATE_DIGEST_LEN;
    let payload = &bytes[CHAIN_STATE_MAGIC.len()..payload_end];
    let expected_digest = hash_bytes(b"tensor-vm-state-file-v1", &[payload]);
    if bytes[payload_end..] != expected_digest {
        return Err(TvmError::Storage("chain state checksum mismatch"));
    }
    decode_chain_state_payload(payload)
}

fn encode_chain_state_payload(chain: &Chain) -> Vec<u8> {
    let mut out = Vec::new();
    encode_chain_params(&mut out, chain.params());
    encode_chain_state(&mut out, chain.state());
    write_len(&mut out, chain.blocks().len());
    for block in chain.blocks() {
        out.extend_from_slice(&encode_block_payload(block));
    }
    out
}

fn decode_chain_state_payload(bytes: &[u8]) -> Result<Chain> {
    let mut reader = StateReader::new(bytes);
    let params = decode_chain_params(&mut reader)?;
    let state = decode_chain_state(&mut reader)?;
    let block_count = reader.read_len()?;
    let mut blocks = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        blocks.push(decode_block_payload(reader.read_exact(BLOCK_PAYLOAD_LEN)?)?);
    }
    reader.finish()?;
    Ok(Chain::from_parts(ChainParts {
        params,
        state,
        blocks,
    }))
}

fn encode_chain_params(out: &mut Vec<u8>, params: &ChainParams) {
    write_u64(out, params.block_time_seconds);
    write_u64(out, params.epoch_length);
    write_u64(out, params.receipt_submission_window);
    write_u64(out, params.verification_window);
    write_u64(out, params.reward_settlement_delay_epochs);
    write_u64(out, params.challenge_window_epochs);
    write_len(out, params.replication_factor);
    write_len(out, params.agreement_quorum);
    write_u64(out, params.finality_stake_numerator);
    write_u64(out, params.finality_stake_denominator);
    write_u64(out, params.miner_reward_bps);
    write_u64(out, params.validator_reward_bps);
    write_u64(out, params.proposer_reward_bps);
    write_u64(out, params.treasury_reward_bps);
    write_u64(out, params.miner_min_stake);
    write_u64(out, params.validator_min_stake);
    write_u64(out, params.data_unavailability_miner_slash_amount);
    write_u64(out, params.validator_audit_sample_numerator);
    write_u64(out, params.validator_audit_sample_denominator);
    write_u64(out, params.validator_audit_window_blocks);
    write_u64(out, params.validator_audit_slash_amount);
    write_hash(out, &params.difficulty_initial_target);
    write_hash(out, &params.difficulty_floor_target);
    write_hash(out, &params.difficulty_ceiling_target);
    write_u64(out, params.difficulty_target_block_time_seconds);
    write_u64(out, params.difficulty_retarget_epoch_length);
    write_u64(out, params.difficulty_retarget_max_ratio);
    write_u64(out, params.pow_timeout_blocks);
    encode_freivalds_params(out, &params.freivalds);
}

fn decode_chain_params(reader: &mut StateReader<'_>) -> Result<ChainParams> {
    Ok(ChainParams {
        block_time_seconds: reader.read_u64()?,
        epoch_length: reader.read_u64()?,
        receipt_submission_window: reader.read_u64()?,
        verification_window: reader.read_u64()?,
        reward_settlement_delay_epochs: reader.read_u64()?,
        challenge_window_epochs: reader.read_u64()?,
        replication_factor: reader.read_len()?,
        agreement_quorum: reader.read_len()?,
        finality_stake_numerator: reader.read_u64()?,
        finality_stake_denominator: reader.read_u64()?,
        miner_reward_bps: reader.read_u64()?,
        validator_reward_bps: reader.read_u64()?,
        proposer_reward_bps: reader.read_u64()?,
        treasury_reward_bps: reader.read_u64()?,
        miner_min_stake: reader.read_u64()?,
        validator_min_stake: reader.read_u64()?,
        data_unavailability_miner_slash_amount: reader.read_u64()?,
        validator_audit_sample_numerator: reader.read_u64()?,
        validator_audit_sample_denominator: reader.read_u64()?,
        validator_audit_window_blocks: reader.read_u64()?,
        validator_audit_slash_amount: reader.read_u64()?,
        difficulty_initial_target: reader.read_hash()?,
        difficulty_floor_target: reader.read_hash()?,
        difficulty_ceiling_target: reader.read_hash()?,
        difficulty_target_block_time_seconds: reader.read_u64()?,
        difficulty_retarget_epoch_length: reader.read_u64()?,
        difficulty_retarget_max_ratio: reader.read_u64()?,
        pow_timeout_blocks: reader.read_u64()?,
        freivalds: decode_freivalds_params(reader)?,
    })
}

fn encode_freivalds_params(out: &mut Vec<u8>, params: &FreivaldsParams) {
    write_len(out, params.full_rounds);
    write_len(out, params.audit_rows);
    write_len(out, params.validators_per_job);
    write_len(out, params.minimum_validators);
    write_u64(out, params.minimum_stake_numerator);
    write_u64(out, params.minimum_stake_denominator);
}

fn decode_freivalds_params(reader: &mut StateReader<'_>) -> Result<FreivaldsParams> {
    Ok(FreivaldsParams {
        full_rounds: reader.read_len()?,
        audit_rows: reader.read_len()?,
        validators_per_job: reader.read_len()?,
        minimum_validators: reader.read_len()?,
        minimum_stake_numerator: reader.read_u64()?,
        minimum_stake_denominator: reader.read_u64()?,
    })
}

fn encode_chain_state(out: &mut Vec<u8>, state: &ChainState) {
    write_u64(out, state.height());
    write_u64(out, state.epoch());
    write_u64(out, state.finalized_beacon_round());
    write_hash(out, &state.finalized_randomness());
    write_u64(out, state.genesis_beacon_round());
    write_hash(out, &state.genesis_randomness());
    encode_accounts(out, state.accounts());
    encode_miners(out, state.miners());
    encode_validators(out, state.validators());
    encode_jobs(out, state.jobs());
    encode_program_bodies(out, state.program_bodies());
    encode_receipts(out, state.receipts());
    encode_receipt_randomness_anchors(out, state.receipt_randomness_anchors());
    encode_attestations(out, state.attestations());
    encode_block_votes(out, state.block_votes());
    encode_hash_set(out, state.finalized_blocks());
    encode_hash_set(out, state.data_unavailable_receipts());
    encode_data_unavailability_slashes(out, state.data_unavailability_slashes());
    encode_validator_audit_assignments(out, state.validator_audit_assignments());
    encode_validator_audit_results(out, state.validator_audit_results());
    encode_validator_audit_slashes(out, state.validator_audit_slashes());
    encode_hash_set(out, state.settled_receipts());
    encode_hash_set(out, state.included_receipts());
    encode_hash_vec_map(out, state.block_selected_receipts());
    encode_block_check_challenges(out, state.block_check_challenges());
    encode_hash_set(out, state.challenged_receipts());
    encode_u64_by_hash_map(out, state.proposer_penalty_until());
    encode_pending_proposer_rewards(out, state.pending_proposer_rewards());
    encode_pending_receipt_rewards(out, state.pending_receipt_rewards());
    encode_pending_challenge_rewards(out, state.pending_challenge_rewards());
    encode_pending_credit_rewards(out, state.pending_credit_rewards());
    encode_model_states(out, state.model_states());
    encode_rewards(out, state.rewards());
}

fn decode_chain_state(reader: &mut StateReader<'_>) -> Result<ChainState> {
    Ok(ChainState::from_parts(ChainStateParts {
        height: reader.read_u64()?,
        epoch: reader.read_u64()?,
        finalized_beacon_round: reader.read_u64()?,
        finalized_randomness: reader.read_hash()?,
        genesis_beacon_round: reader.read_u64()?,
        genesis_randomness: reader.read_hash()?,
        accounts: decode_accounts(reader)?,
        miners: decode_miners(reader)?,
        validators: decode_validators(reader)?,
        jobs: decode_jobs(reader)?,
        program_bodies: decode_program_bodies(reader)?,
        receipts: decode_receipts(reader)?,
        receipt_randomness_anchors: decode_receipt_randomness_anchors(reader)?,
        attestations: decode_attestations(reader)?,
        block_votes: decode_block_votes(reader)?,
        finalized_blocks: decode_hash_set(reader)?,
        data_unavailable_receipts: decode_hash_set(reader)?,
        data_unavailability_slashes: decode_data_unavailability_slashes(reader)?,
        validator_audit_assignments: decode_validator_audit_assignments(reader)?,
        validator_audit_results: decode_validator_audit_results(reader)?,
        validator_audit_slashes: decode_validator_audit_slashes(reader)?,
        settled_receipts: decode_hash_set(reader)?,
        included_receipts: decode_hash_set(reader)?,
        block_selected_receipts: decode_hash_vec_map(reader)?,
        block_check_challenges: decode_block_check_challenges(reader)?,
        challenged_receipts: decode_hash_set(reader)?,
        proposer_penalty_until: decode_u64_by_hash_map(reader)?,
        pending_proposer_rewards: decode_pending_proposer_rewards(reader)?,
        pending_receipt_rewards: decode_pending_receipt_rewards(reader)?,
        pending_challenge_rewards: decode_pending_challenge_rewards(reader)?,
        pending_credit_rewards: decode_pending_credit_rewards(reader)?,
        model_states: decode_model_states(reader)?,
        rewards: decode_rewards(reader)?,
    }))
}

fn encode_accounts(out: &mut Vec<u8>, accounts: &BTreeMap<Hash, AccountState>) {
    write_len(out, accounts.len());
    for (address, account) in accounts {
        write_hash(out, address);
        write_hash(out, &account.address);
        write_u64(out, account.balance);
        write_u64(out, account.nonce);
    }
}

fn decode_accounts(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, AccountState>> {
    let mut accounts = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        accounts.insert(
            key,
            AccountState {
                address,
                balance: reader.read_u64()?,
                nonce: reader.read_u64()?,
            },
        );
    }
    Ok(accounts)
}

fn encode_miners(out: &mut Vec<u8>, miners: &BTreeMap<Hash, MinerState>) {
    write_len(out, miners.len());
    for (address, miner) in miners {
        write_hash(out, address);
        write_hash(out, &miner.address);
        write_hash(out, &miner.operator_id);
        write_u64(out, miner.stake);
        write_i64(out, miner.reputation);
        write_u64(out, miner.settled_tensor_work);
        write_u64(out, miner.pending_tensor_work);
        out.push(hardware_class_code(miner.hardware_class));
        write_u64(out, miner.gpu_utilization_bps);
    }
}

fn decode_miners(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, MinerState>> {
    let mut miners = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        miners.insert(
            key,
            MinerState {
                address,
                operator_id: reader.read_hash()?,
                stake: reader.read_u64()?,
                reputation: reader.read_i64()?,
                settled_tensor_work: reader.read_u64()?,
                pending_tensor_work: reader.read_u64()?,
                hardware_class: decode_hardware_class(reader.read_u8()?)?,
                gpu_utilization_bps: reader.read_u64()?,
            },
        );
    }
    Ok(miners)
}

fn encode_validators(out: &mut Vec<u8>, validators: &BTreeMap<Hash, ValidatorState>) {
    write_len(out, validators.len());
    for (address, validator) in validators {
        write_hash(out, address);
        write_hash(out, &validator.address);
        write_u64(out, validator.stake);
        write_i64(out, validator.reputation);
        write_u64(out, validator.valid_attestations);
        write_u64(out, validator.missed_assignments);
    }
}

fn decode_validators(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ValidatorState>> {
    let mut validators = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let address = reader.read_hash()?;
        validators.insert(
            key,
            ValidatorState {
                address,
                stake: reader.read_u64()?,
                reputation: reader.read_i64()?,
                valid_attestations: reader.read_u64()?,
                missed_assignments: reader.read_u64()?,
            },
        );
    }
    Ok(validators)
}

fn encode_jobs(out: &mut Vec<u8>, jobs: &BTreeMap<Hash, JobState>) {
    write_len(out, jobs.len());
    for (job_id, job) in jobs {
        write_hash(out, job_id);
        out.extend_from_slice(&payload_codec::encode_job_payload(job));
    }
}

fn decode_jobs(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, JobState>> {
    let mut jobs = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let job = payload_codec::decode_job_payload_from(reader.input, &mut reader.offset, None)
            .map_err(storage_codec_error)?;
        jobs.insert(key, job);
    }
    Ok(jobs)
}

fn encode_program_bodies(out: &mut Vec<u8>, programs: &BTreeMap<Hash, Vec<u8>>) {
    write_len(out, programs.len());
    for (graph_id, body) in programs {
        write_hash(out, graph_id);
        write_bytes(out, body);
    }
}

fn decode_program_bodies(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, Vec<u8>>> {
    let mut programs = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let graph_id = reader.read_hash()?;
        let body = reader.read_bytes()?;
        programs.insert(graph_id, body);
    }
    Ok(programs)
}

fn storage_codec_error(error: payload_codec::CodecError) -> TvmError {
    match error {
        payload_codec::CodecError::Truncated => TvmError::Storage("truncated chain state"),
        payload_codec::CodecError::TrailingBytes => TvmError::Storage("trailing chain state bytes"),
        payload_codec::CodecError::UnknownJobTag => TvmError::Storage("unknown job tag"),
        payload_codec::CodecError::UnknownReceiptTag => TvmError::Storage("unknown receipt tag"),
        payload_codec::CodecError::UnknownDType => TvmError::Storage("unknown dtype"),
        payload_codec::CodecError::UnknownPrimitiveType => {
            TvmError::Storage("unknown primitive type")
        }
        payload_codec::CodecError::UnknownVerificationResult => {
            TvmError::Storage("unknown verification result")
        }
        payload_codec::CodecError::InvalidOptionalU64 => TvmError::Storage("invalid optional u64"),
        payload_codec::CodecError::InvalidBool => TvmError::Storage("invalid boolean"),
        payload_codec::CodecError::InvalidString => TvmError::Storage("invalid string"),
        payload_codec::CodecError::UsizeOverflow => {
            TvmError::Storage("chain state length overflow")
        }
        payload_codec::CodecError::ShapeVectorTooLarge => {
            TvmError::Storage("shape vector too large")
        }
        payload_codec::CodecError::HashVectorTooLarge => TvmError::Storage("hash vector too large"),
        payload_codec::CodecError::StringTooLarge => TvmError::Storage("string too large"),
    }
}

fn encode_receipts(out: &mut Vec<u8>, receipts: &BTreeMap<Hash, ReceiptState>) {
    write_len(out, receipts.len());
    for (receipt_id, receipt) in receipts {
        write_hash(out, receipt_id);
        out.extend_from_slice(&payload_codec::encode_receipt_payload(receipt));
    }
}

fn decode_receipts(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ReceiptState>> {
    let mut receipts = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let receipt =
            payload_codec::decode_receipt_payload_from(reader.input, &mut reader.offset, None)
                .map_err(storage_codec_error)?;
        receipts.insert(key, receipt);
    }
    Ok(receipts)
}

fn encode_receipt_randomness_anchors(
    out: &mut Vec<u8>,
    anchors: &BTreeMap<Hash, ReceiptRandomnessAnchor>,
) {
    write_len(out, anchors.len());
    for (receipt_id, anchor) in anchors {
        write_hash(out, receipt_id);
        write_hash(out, &anchor.receipt_id);
        write_u64(out, anchor.beacon_round);
        write_hash(out, &anchor.finalized_randomness);
        write_hash(out, &anchor.assignment_seed);
    }
}

fn decode_receipt_randomness_anchors(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, ReceiptRandomnessAnchor>> {
    let mut anchors = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        anchors.insert(
            key,
            ReceiptRandomnessAnchor {
                receipt_id: reader.read_hash()?,
                beacon_round: reader.read_u64()?,
                finalized_randomness: reader.read_hash()?,
                assignment_seed: reader.read_hash()?,
            },
        );
    }
    Ok(anchors)
}

fn encode_attestations(
    out: &mut Vec<u8>,
    attestations: &BTreeMap<Hash, Vec<ValidatorAttestation>>,
) {
    write_len(out, attestations.len());
    for (receipt_id, items) in attestations {
        write_hash(out, receipt_id);
        write_len(out, items.len());
        for attestation in items {
            out.extend_from_slice(&payload_codec::encode_attestation_payload(attestation));
        }
    }
}

fn decode_attestations(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, Vec<ValidatorAttestation>>> {
    let mut attestations = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let receipt_id = reader.read_hash()?;
        let item_count = reader.read_len()?;
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            let attestation =
                payload_codec::decode_attestation_payload_from(reader.input, &mut reader.offset)
                    .map_err(storage_codec_error)?;
            items.push(attestation);
        }
        attestations.insert(receipt_id, items);
    }
    Ok(attestations)
}

fn encode_block_votes(out: &mut Vec<u8>, votes: &BTreeMap<Hash, Vec<BlockVote>>) {
    write_len(out, votes.len());
    for (block_hash, votes) in votes {
        write_hash(out, block_hash);
        write_len(out, votes.len());
        for vote in votes {
            out.extend_from_slice(&payload_codec::encode_block_vote_payload(vote));
        }
    }
}

fn decode_block_votes(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, Vec<BlockVote>>> {
    let mut block_votes = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let block_hash = reader.read_hash()?;
        let vote_count = reader.read_len()?;
        let mut votes = Vec::with_capacity(vote_count);
        for _ in 0..vote_count {
            let vote = payload_codec::decode_block_vote_payload(
                reader.read_exact(payload_codec::BLOCK_VOTE_PAYLOAD_LEN)?,
            )
            .ok_or(TvmError::Storage("invalid block vote payload length"))?;
            votes.push(vote);
        }
        block_votes.insert(block_hash, votes);
    }
    Ok(block_votes)
}

fn encode_model_states(out: &mut Vec<u8>, models: &BTreeMap<Hash, ModelState>) {
    write_len(out, models.len());
    for (model_id, model) in models {
        write_hash(out, model_id);
        write_hash(out, &model.model_id);
        write_hash(out, &model.architecture_hash);
        write_hash(out, &model.weight_root);
        write_option_hash(out, &model.optimizer_state_root);
        write_u64(out, model.step);
        write_hash(out, &model.config_hash);
    }
}

fn decode_model_states(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, ModelState>> {
    let mut models = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let model_id = reader.read_hash()?;
        models.insert(
            key,
            ModelState {
                model_id,
                architecture_hash: reader.read_hash()?,
                weight_root: reader.read_hash()?,
                optimizer_state_root: reader.read_option_hash()?,
                step: reader.read_u64()?,
                config_hash: reader.read_hash()?,
            },
        );
    }
    Ok(models)
}

fn encode_rewards(out: &mut Vec<u8>, rewards: &RewardState) {
    write_len(out, rewards.balances().len());
    for (address, balance) in rewards.balances() {
        write_hash(out, address);
        write_u64(out, *balance);
    }
    write_u64(out, rewards.treasury());
}

fn decode_rewards(reader: &mut StateReader<'_>) -> Result<RewardState> {
    let mut balances = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        balances.insert(reader.read_hash()?, reader.read_u64()?);
    }
    Ok(RewardState::from_parts(balances, reader.read_u64()?))
}

fn encode_hash_set(out: &mut Vec<u8>, items: &BTreeSet<Hash>) {
    write_len(out, items.len());
    for item in items {
        write_hash(out, item);
    }
}

fn decode_hash_set(reader: &mut StateReader<'_>) -> Result<BTreeSet<Hash>> {
    let mut items = BTreeSet::new();
    for _ in 0..reader.read_len()? {
        items.insert(reader.read_hash()?);
    }
    Ok(items)
}

fn encode_data_unavailability_slashes(
    out: &mut Vec<u8>,
    slashes: &BTreeMap<Hash, DataUnavailabilitySlashRecord>,
) {
    write_len(out, slashes.len());
    for (receipt_id, slash) in slashes {
        write_hash(out, receipt_id);
        write_hash(out, &slash.receipt_id);
        write_hash(out, &slash.miner);
        write_hash(out, &slash.evidence_validator);
        write_u64(out, slash.amount);
        write_u64(out, slash.slashed_at_height);
        write_len(out, slash.reason.len());
        out.extend_from_slice(slash.reason.as_bytes());
    }
}

fn decode_data_unavailability_slashes(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, DataUnavailabilitySlashRecord>> {
    let mut slashes = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let receipt_id = reader.read_hash()?;
        let miner = reader.read_hash()?;
        let evidence_validator = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let slashed_at_height = reader.read_u64()?;
        let reason_len = reader.read_len()?;
        let reason = std::str::from_utf8(reader.read_exact(reason_len)?)
            .map_err(|_| TvmError::Storage("invalid data-unavailability slash reason"))?
            .to_owned();
        slashes.insert(
            key,
            DataUnavailabilitySlashRecord {
                receipt_id,
                miner,
                evidence_validator,
                amount,
                slashed_at_height,
                reason,
            },
        );
    }
    Ok(slashes)
}

fn encode_validator_audit_assignments(
    out: &mut Vec<u8>,
    assignments: &BTreeMap<Hash, ValidatorAuditAssignment>,
) {
    write_len(out, assignments.len());
    for (audit_id, assignment) in assignments {
        write_hash(out, audit_id);
        write_hash(out, &assignment.audit_id);
        write_hash(out, &assignment.receipt_id);
        write_hash(out, &assignment.validator);
        write_u64(out, assignment.assigned_at_height);
        write_u64(out, assignment.deadline_height);
        write_hash(out, &assignment.seed);
    }
}

fn decode_validator_audit_assignments(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, ValidatorAuditAssignment>> {
    let mut assignments = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let audit_id = reader.read_hash()?;
        assignments.insert(
            audit_id,
            ValidatorAuditAssignment {
                audit_id: reader.read_hash()?,
                receipt_id: reader.read_hash()?,
                validator: reader.read_hash()?,
                assigned_at_height: reader.read_u64()?,
                deadline_height: reader.read_u64()?,
                seed: reader.read_hash()?,
            },
        );
    }
    Ok(assignments)
}

fn encode_validator_audit_results(
    out: &mut Vec<u8>,
    results: &BTreeMap<Hash, ValidatorAuditResult>,
) {
    write_len(out, results.len());
    for (audit_id, result) in results {
        write_hash(out, audit_id);
        write_hash(out, &result.audit_id);
        write_hash(out, &result.receipt_id);
        write_hash(out, &result.validator);
        write_hash(out, &result.auditor);
        out.push(verification_result_tag(result.attested_result));
        out.push(verification_result_tag(result.canonical_result));
        out.push(u8::from(result.attested_data_availability_passed));
        out.push(u8::from(result.canonical_data_availability_passed));
        write_hash(out, &result.checks_root);
        write_u64(out, result.submitted_at_height);
        out.push(u8::from(result.passed));
        out.extend_from_slice(&result.signature);
    }
}

fn decode_validator_audit_results(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, ValidatorAuditResult>> {
    let mut results = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let audit_id = reader.read_hash()?;
        let result_audit_id = reader.read_hash()?;
        let receipt_id = reader.read_hash()?;
        let validator = reader.read_hash()?;
        let auditor = reader.read_hash()?;
        let attested_result = verification_result_from_tag(reader.read_u8()?)
            .ok_or(TvmError::Storage("invalid validator audit attested result"))?;
        let canonical_result = verification_result_from_tag(reader.read_u8()?).ok_or(
            TvmError::Storage("invalid validator audit canonical result"),
        )?;
        let attested_data_availability_passed = read_bool(reader, "invalid audit attested DA")?;
        let canonical_data_availability_passed = read_bool(reader, "invalid audit canonical DA")?;
        let checks_root = reader.read_hash()?;
        let submitted_at_height = reader.read_u64()?;
        let passed = read_bool(reader, "invalid validator audit pass flag")?;
        let signature = reader.read_hash()?;
        results.insert(
            audit_id,
            ValidatorAuditResult {
                audit_id: result_audit_id,
                receipt_id,
                validator,
                auditor,
                attested_result,
                canonical_result,
                attested_data_availability_passed,
                canonical_data_availability_passed,
                checks_root,
                submitted_at_height,
                passed,
                signature,
            },
        );
    }
    Ok(results)
}

fn encode_validator_audit_slashes(
    out: &mut Vec<u8>,
    slashes: &BTreeMap<Hash, ValidatorAuditSlashRecord>,
) {
    write_len(out, slashes.len());
    for (audit_id, slash) in slashes {
        write_hash(out, audit_id);
        write_hash(out, &slash.audit_id);
        write_hash(out, &slash.receipt_id);
        write_hash(out, &slash.validator);
        write_hash(out, &slash.auditor);
        write_u64(out, slash.amount);
        write_u64(out, slash.slashed_at_height);
        write_len(out, slash.reason.len());
        out.extend_from_slice(slash.reason.as_bytes());
    }
}

fn decode_validator_audit_slashes(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, ValidatorAuditSlashRecord>> {
    let mut slashes = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let audit_id = reader.read_hash()?;
        let record_audit_id = reader.read_hash()?;
        let receipt_id = reader.read_hash()?;
        let validator = reader.read_hash()?;
        let auditor = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let slashed_at_height = reader.read_u64()?;
        let reason_len = reader.read_len()?;
        let reason = std::str::from_utf8(reader.read_exact(reason_len)?)
            .map_err(|_| TvmError::Storage("invalid validator audit slash reason"))?
            .to_owned();
        slashes.insert(
            audit_id,
            ValidatorAuditSlashRecord {
                audit_id: record_audit_id,
                receipt_id,
                validator,
                auditor,
                amount,
                slashed_at_height,
                reason,
            },
        );
    }
    Ok(slashes)
}

fn read_bool(reader: &mut StateReader<'_>, error: &'static str) -> Result<bool> {
    match reader.read_u8()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(TvmError::Storage(error)),
    }
}

fn encode_hash_vec_map(out: &mut Vec<u8>, items: &BTreeMap<Hash, Vec<Hash>>) {
    write_len(out, items.len());
    for (key, values) in items {
        write_hash(out, key);
        write_len(out, values.len());
        for value in values {
            write_hash(out, value);
        }
    }
}

fn decode_hash_vec_map(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, Vec<Hash>>> {
    let mut items = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let mut values = Vec::new();
        for _ in 0..reader.read_len()? {
            values.push(reader.read_hash()?);
        }
        items.insert(key, values);
    }
    Ok(items)
}

fn encode_block_check_challenges(
    out: &mut Vec<u8>,
    challenges: &BTreeMap<Hash, BlockCheckChallengeRecord>,
) {
    write_len(out, challenges.len());
    for (challenge_id, challenge) in challenges {
        write_hash(out, challenge_id);
        write_hash(out, &challenge.block_hash);
        write_u64(out, challenge.block_height);
        write_hash(out, &challenge.receipt_id);
        write_hash(out, &challenge.proposer);
        write_hash(out, &challenge.challenger);
        write_hash(out, &challenge.expected_check_leaf);
        write_hash(out, &challenge.observed_check_leaf);
        write_u64(out, challenge.challenged_at_height);
        write_u64(out, challenge.proposer_reward_clawback);
        write_u64(out, challenge.challenger_reward);
        write_u64(out, challenge.penalty_until_height);
        write_len(out, challenge.reason.len());
        out.extend_from_slice(challenge.reason.as_bytes());
    }
}

fn decode_block_check_challenges(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, BlockCheckChallengeRecord>> {
    let mut challenges = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let challenge_id = reader.read_hash()?;
        let block_hash = reader.read_hash()?;
        let block_height = reader.read_u64()?;
        let receipt_id = reader.read_hash()?;
        let proposer = reader.read_hash()?;
        let challenger = reader.read_hash()?;
        let expected_check_leaf = reader.read_hash()?;
        let observed_check_leaf = reader.read_hash()?;
        let challenged_at_height = reader.read_u64()?;
        let proposer_reward_clawback = reader.read_u64()?;
        let challenger_reward = reader.read_u64()?;
        let penalty_until_height = reader.read_u64()?;
        let reason_len = reader.read_len()?;
        let reason = std::str::from_utf8(reader.read_exact(reason_len)?)
            .map_err(|_| TvmError::Storage("invalid challenge reason"))?
            .to_owned();
        challenges.insert(
            challenge_id,
            BlockCheckChallengeRecord {
                block_hash,
                block_height,
                receipt_id,
                proposer,
                challenger,
                expected_check_leaf,
                observed_check_leaf,
                challenged_at_height,
                proposer_reward_clawback,
                challenger_reward,
                penalty_until_height,
                reason,
            },
        );
    }
    Ok(challenges)
}

fn encode_u64_by_hash_map(out: &mut Vec<u8>, items: &BTreeMap<Hash, u64>) {
    write_len(out, items.len());
    for (key, value) in items {
        write_hash(out, key);
        write_u64(out, *value);
    }
}

fn decode_u64_by_hash_map(reader: &mut StateReader<'_>) -> Result<BTreeMap<Hash, u64>> {
    let mut items = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        items.insert(reader.read_hash()?, reader.read_u64()?);
    }
    Ok(items)
}

fn encode_pending_proposer_rewards(
    out: &mut Vec<u8>,
    rewards: &BTreeMap<u64, PendingProposerReward>,
) {
    write_len(out, rewards.len());
    for (height, reward) in rewards {
        write_u64(out, *height);
        write_u64(out, reward.block_height);
        write_hash(out, &reward.proposer);
        write_u64(out, reward.amount);
        write_u64(out, reward.claimable_at_height);
        out.push(u8::from(reward.voided_by_challenge));
    }
}

fn decode_pending_proposer_rewards(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<u64, PendingProposerReward>> {
    let mut rewards = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let height = reader.read_u64()?;
        let block_height = reader.read_u64()?;
        let proposer = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let claimable_at_height = reader.read_u64()?;
        let voided_by_challenge = match reader.read_u8()? {
            0 => false,
            1 => true,
            _ => return Err(TvmError::Storage("invalid pending reward boolean")),
        };
        rewards.insert(
            height,
            PendingProposerReward {
                block_height,
                proposer,
                amount,
                claimable_at_height,
                voided_by_challenge,
            },
        );
    }
    Ok(rewards)
}

fn encode_pending_receipt_rewards(
    out: &mut Vec<u8>,
    rewards: &BTreeMap<Hash, PendingReceiptReward>,
) {
    write_len(out, rewards.len());
    for (claim_id, reward) in rewards {
        write_hash(out, claim_id);
        write_hash(out, &reward.claim_id);
        write_hash(out, &reward.receipt_id);
        write_hash(out, &reward.beneficiary);
        write_u64(out, reward.amount);
        out.push(reward.kind.tag());
        write_u64(out, reward.claimable_at_height);
        out.push(u8::from(reward.voided_by_challenge));
    }
}

fn decode_pending_receipt_rewards(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, PendingReceiptReward>> {
    let mut rewards = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let claim_id = reader.read_hash()?;
        let receipt_id = reader.read_hash()?;
        let beneficiary = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let kind = match reader.read_u8()? {
            1 => ReceiptRewardKind::Miner,
            2 => ReceiptRewardKind::Validator,
            _ => return Err(TvmError::Storage("invalid pending receipt reward kind")),
        };
        let claimable_at_height = reader.read_u64()?;
        let voided_by_challenge = match reader.read_u8()? {
            0 => false,
            1 => true,
            _ => return Err(TvmError::Storage("invalid pending receipt reward boolean")),
        };
        rewards.insert(
            key,
            PendingReceiptReward {
                claim_id,
                receipt_id,
                beneficiary,
                amount,
                kind,
                claimable_at_height,
                voided_by_challenge,
            },
        );
    }
    Ok(rewards)
}

fn encode_pending_challenge_rewards(
    out: &mut Vec<u8>,
    rewards: &BTreeMap<Hash, PendingChallengeReward>,
) {
    write_len(out, rewards.len());
    for (claim_id, reward) in rewards {
        write_hash(out, claim_id);
        write_hash(out, &reward.claim_id);
        write_hash(out, &reward.challenge_id);
        write_hash(out, &reward.block_hash);
        write_hash(out, &reward.receipt_id);
        write_hash(out, &reward.challenger);
        write_u64(out, reward.amount);
        write_u64(out, reward.claimable_at_height);
        out.push(u8::from(reward.voided_by_challenge));
    }
}

fn decode_pending_challenge_rewards(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, PendingChallengeReward>> {
    let mut rewards = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let claim_id = reader.read_hash()?;
        let challenge_id = reader.read_hash()?;
        let block_hash = reader.read_hash()?;
        let receipt_id = reader.read_hash()?;
        let challenger = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let claimable_at_height = reader.read_u64()?;
        let voided_by_challenge = match reader.read_u8()? {
            0 => false,
            1 => true,
            _ => {
                return Err(TvmError::Storage(
                    "invalid pending challenge reward boolean",
                ));
            }
        };
        rewards.insert(
            key,
            PendingChallengeReward {
                claim_id,
                challenge_id,
                block_hash,
                receipt_id,
                challenger,
                amount,
                claimable_at_height,
                voided_by_challenge,
            },
        );
    }
    Ok(rewards)
}

fn encode_pending_credit_rewards(out: &mut Vec<u8>, rewards: &BTreeMap<Hash, PendingCreditReward>) {
    write_len(out, rewards.len());
    for (claim_id, reward) in rewards {
        write_hash(out, claim_id);
        write_hash(out, &reward.claim_id);
        write_hash(out, &reward.beneficiary);
        write_u64(out, reward.amount);
        write_u64(out, reward.claimable_at_height);
    }
}

fn decode_pending_credit_rewards(
    reader: &mut StateReader<'_>,
) -> Result<BTreeMap<Hash, PendingCreditReward>> {
    let mut rewards = BTreeMap::new();
    for _ in 0..reader.read_len()? {
        let key = reader.read_hash()?;
        let claim_id = reader.read_hash()?;
        let beneficiary = reader.read_hash()?;
        let amount = reader.read_u64()?;
        let claimable_at_height = reader.read_u64()?;
        rewards.insert(
            key,
            PendingCreditReward {
                claim_id,
                beneficiary,
                amount,
                claimable_at_height,
            },
        );
    }
    Ok(rewards)
}

fn hardware_class_code(hardware_class: HardwareClass) -> u8 {
    match hardware_class {
        HardwareClass::Cpu => 1,
        HardwareClass::ConsumerGpu => 2,
        HardwareClass::DatacenterGpu => 3,
        HardwareClass::Other => 4,
    }
}

fn decode_hardware_class(tag: u8) -> Result<HardwareClass> {
    match tag {
        1 => Ok(HardwareClass::Cpu),
        2 => Ok(HardwareClass::ConsumerGpu),
        3 => Ok(HardwareClass::DatacenterGpu),
        4 => Ok(HardwareClass::Other),
        _ => Err(TvmError::Storage("unknown hardware class")),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        credit_reward, produce_block, register_model, register_validator, submit_attestation,
        submit_block_vote, submit_job, submit_receipt, transfer,
    };
    use super::*;
    use crate::chain::{ChainCommand, ChainEngine};
    use crate::ir::canonical_matmul_graph;
    use crate::jobs::{
        GraphJob, GraphReceipt, LinearTrainingStepJob, LinearTrainingStepReceipt,
        LinearTrainingStepSpec, MatmulJob, PrimitiveType, TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::address;
    use crate::verify::{AttestationStatement, VerificationResult};
    fn durable_chain_fixture(label: &[u8]) -> Chain {
        let beacon = hash_bytes(b"test", &[label]);
        let params = ChainParams {
            replication_factor: 2,
            agreement_quorum: 1,
            validator_audit_sample_numerator: 1,
            validator_audit_sample_denominator: 1,
            validator_audit_window_blocks: 1,
            validator_audit_slash_amount: 17,
            freivalds: FreivaldsParams {
                full_rounds: 2,
                audit_rows: 3,
                validators_per_job: 2,
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 2,
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"durable-miner");
        let validator = address(b"durable-validator");
        let arbitrary_graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: arbitrary_graph.graph_id(),
                bytes: arbitrary_graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .register_miner_with_profile_and_operator(
                miner,
                chain.params().miner_min_stake,
                hash_bytes(b"test", &[b"durable-operator"]),
                HardwareClass::DatacenterGpu,
                8_500,
            )
            .unwrap();
        register_validator(&mut chain, validator);
        chain.credit_account(miner, 1_000);
        transfer(&mut chain, miner, validator, 125);

        let matmul = MatmulJob::synthetic(0, 7, 4, 3, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&matmul, miner, 1, 5).unwrap();
        submit_job(&mut chain, JobState::TensorOp(matmul.clone()));
        let mut no_modulus = MatmulJob::synthetic(0, 8, 2, 2, 2, &beacon, 11);
        no_modulus.modulus = None;
        submit_job(&mut chain, JobState::TensorOp(no_modulus));
        submit_receipt(&mut chain, ReceiptState::TensorOp(receipt.clone()));
        let attestation = ValidatorAttestation::new(
            validator,
            chain.params().validator_min_stake,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );
        submit_attestation(&mut chain, attestation);

        let model_id = hash_bytes(b"test", &[b"durable-model"]);
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        register_model(
            &mut chain,
            model_id,
            hash_bytes(b"test", &[b"architecture"]),
            weights.commitment_root(),
            hash_bytes(b"test", &[b"config"]),
        );
        register_model(
            &mut chain,
            hash_bytes(b"test", &[b"durable-model-with-optimizer"]),
            hash_bytes(b"test", &[b"architecture-2"]),
            weights.commitment_root(),
            hash_bytes(b"test", &[b"config-2"]),
        );
        chain
            .set_model_optimizer_state_root_for_testing(
                model_id,
                Some(hash_bytes(b"test", &[b"optimizer"])),
            )
            .unwrap();
        let linear = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 12,
        });
        let (linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&linear, miner, &weights, 2, 7).unwrap();
        submit_job(&mut chain, JobState::LinearTrainingStep(linear));
        submit_receipt(
            &mut chain,
            ReceiptState::LinearTrainingStep(linear_receipt.clone()),
        );

        let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
        let graph_id = graph.validate_for_consensus().unwrap();
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        let graph_a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let graph_b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
        let graph_inputs = BTreeMap::from([
            ("a".to_owned(), graph_a.clone()),
            ("b".to_owned(), graph_b.clone()),
        ]);
        let graph_input_roots = graph_inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let graph_job = GraphJob::new(0, graph_id, graph_input_roots, BTreeMap::new(), 13, 1, 8);
        let (graph_receipt, _outputs) =
            GraphReceipt::from_execution(&graph_job, &graph, miner, &graph_inputs, 3, 8).unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(graph_job)))
            .unwrap();
        submit_receipt(
            &mut chain,
            ReceiptState::GraphExecution(graph_receipt.clone()),
        );
        chain.settle_epoch(1_000, 500);
        assert!(
            chain
                .state()
                .pending_receipt_rewards()
                .values()
                .any(|reward| {
                    reward.receipt_id == receipt.receipt_id
                        && reward.beneficiary == miner
                        && reward.amount == 1_000
                        && reward.kind == ReceiptRewardKind::Miner
                        && !reward.voided_by_challenge
                })
        );
        chain.mark_receipt_data_unavailable_for_testing(linear_receipt.receipt_id);
        credit_reward(&mut chain, miner, 77);
        chain.set_reward_treasury_for_testing(11);
        let challenge_id = hash_bytes(b"test", &[b"durable-block-check-challenge"]);
        let claim_id = hash_bytes(b"test", &[b"durable-pending-challenge-reward"]);
        chain.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
            claim_id,
            challenge_id,
            block_hash: hash_bytes(b"test", &[b"durable-challenged-block"]),
            receipt_id: receipt.receipt_id,
            challenger: validator,
            amount: 33,
            claimable_at_height: 42,
            voided_by_challenge: false,
        });

        let block = produce_block(&mut chain, validator, 1_000);
        let validator_stake = chain.params().validator_min_stake;
        submit_block_vote(
            &mut chain,
            BlockVote::new(validator, validator_stake, &block),
        );
        produce_block(&mut chain, validator, 1_006);
        chain
    }

    #[test]
    fn chain_state_store_roundtrips_full_chain_and_detects_tampering() {
        let chain = durable_chain_fixture(b"chain-state-store");
        let path = std::env::temp_dir().join(format!(
            "tensor-vm-state-{}-{}.bin",
            std::process::id(),
            chain.state().height()
        ));
        let store = ChainStateStore::new(path.clone());
        assert_eq!(store.path(), path.as_path());
        assert_eq!(
            store.load_chain(),
            Err(TvmError::Storage("failed to read chain state"))
        );

        store.save_chain(&chain).unwrap();
        let loaded = store.load_chain().unwrap();
        assert_eq!(loaded, chain);
        assert_eq!(loaded.state_root(), chain.state_root());
        assert_eq!(loaded.state().validator_audit_assignments().len(), 1);
        assert_eq!(loaded.state().validator_audit_slashes().len(), 1);
        assert_eq!(
            loaded
                .state()
                .validator_audit_slashes()
                .values()
                .next()
                .unwrap()
                .amount,
            17
        );
        assert_eq!(
            loaded.state().program_bodies(),
            chain.state().program_bodies()
        );
        assert_eq!(
            loaded.state().receipt_randomness_anchors(),
            chain.state().receipt_randomness_anchors()
        );
        assert!(loaded.state().receipts().keys().all(|receipt_id| {
            loaded
                .state()
                .receipt_randomness_anchors()
                .contains_key(receipt_id)
        }));
        assert!(
            loaded
                .state()
                .jobs()
                .values()
                .all(|job| loaded.state().program_body(&job.program_hash()).is_some())
        );
        assert_eq!(
            loaded.state().pending_receipt_rewards(),
            chain.state().pending_receipt_rewards()
        );
        assert_eq!(
            loaded.state().pending_challenge_rewards(),
            chain.state().pending_challenge_rewards()
        );
        assert_eq!(
            loaded.state().pending_credit_rewards(),
            chain.state().pending_credit_rewards()
        );
        assert_eq!(
            loaded.state().data_unavailability_slashes(),
            chain.state().data_unavailability_slashes()
        );
        assert!(
            loaded
                .state()
                .data_unavailability_slashes()
                .values()
                .any(|slash| slash.miner == address(b"durable-miner")
                    && slash.amount == chain.params().data_unavailability_miner_slash_amount
                    && slash.slashed_at_height == 0
                    && slash.reason == "data unavailable for receipt verification")
        );
        assert_eq!(
            loaded
                .state()
                .miners()
                .get(&address(b"durable-miner"))
                .unwrap()
                .stake,
            chain
                .params()
                .miner_min_stake
                .saturating_sub(chain.params().data_unavailability_miner_slash_amount)
        );
        assert_eq!(
            loaded.state().rewards().treasury(),
            11 + chain.params().data_unavailability_miner_slash_amount
                + chain.params().validator_audit_slash_amount
        );
        assert!(
            loaded
                .state()
                .pending_receipt_rewards()
                .values()
                .any(|reward| reward.amount == 1_000
                    && reward.kind == ReceiptRewardKind::Miner
                    && reward.claim_id != [0; 32]
                    && reward.claimable_at_height > chain.state().height()
                    && !reward.voided_by_challenge)
        );
        assert!(
            loaded
                .state()
                .pending_challenge_rewards()
                .values()
                .any(|reward| reward.amount == 33
                    && reward.claimable_at_height == 42
                    && reward.challenger == address(b"durable-validator")
                    && reward.claim_id != [0; 32]
                    && !reward.voided_by_challenge)
        );
        assert!(
            loaded
                .state()
                .pending_credit_rewards()
                .values()
                .any(|reward| reward.amount == 77
                    && reward.beneficiary == address(b"durable-miner")
                    && reward.claim_id != [0; 32]
                    && reward.claimable_at_height > chain.state().height())
        );
        assert_eq!(
            decode_chain_state_file(&encode_chain_state_file(&chain)).unwrap(),
            chain
        );

        let mut tampered = encode_chain_state_file(&chain);
        tampered[CHAIN_STATE_MAGIC.len()] ^= 1;
        assert_eq!(
            decode_chain_state_file(&tampered),
            Err(TvmError::Storage("chain state checksum mismatch"))
        );
        assert_eq!(
            decode_chain_state_file(b"bad"),
            Err(TvmError::Storage("invalid chain state magic"))
        );
        assert_eq!(
            decode_chain_state_file(CHAIN_STATE_MAGIC),
            Err(TvmError::Storage("invalid chain state length"))
        );

        let mut trailing = encode_chain_state_payload(&chain);
        trailing.push(0);
        assert_eq!(
            decode_chain_state_payload(&trailing),
            Err(TvmError::Storage("trailing chain state bytes"))
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn chain_state_decoder_rejects_invalid_tags_and_values() {
        const TENSOR_DTYPE_OFFSET: usize = 1 + 32 + 8 + 8 + 8 + 8;
        const TENSOR_OPTIONAL_MODULUS_OFFSET: usize = TENSOR_DTYPE_OFFSET + 1;

        assert_eq!(hardware_class_code(HardwareClass::Cpu), 1);
        assert_eq!(hardware_class_code(HardwareClass::ConsumerGpu), 2);
        assert_eq!(hardware_class_code(HardwareClass::DatacenterGpu), 3);
        assert_eq!(hardware_class_code(HardwareClass::Other), 4);
        assert_eq!(decode_hardware_class(1).unwrap(), HardwareClass::Cpu);
        assert_eq!(
            decode_hardware_class(2).unwrap(),
            HardwareClass::ConsumerGpu
        );
        assert_eq!(
            decode_hardware_class(3).unwrap(),
            HardwareClass::DatacenterGpu
        );
        assert_eq!(decode_hardware_class(4).unwrap(), HardwareClass::Other);
        assert_eq!(
            decode_hardware_class(9),
            Err(TvmError::Storage("unknown hardware class"))
        );

        assert_eq!(
            StateReader::new(&[]).read_u8(),
            Err(TvmError::Storage("truncated chain state"))
        );
        assert_eq!(
            StateReader::new(&[2]).read_option_hash(),
            Err(TvmError::Storage("invalid optional hash"))
        );
        let mut none = Vec::new();
        write_option_hash(&mut none, &None);
        assert_eq!(StateReader::new(&none).read_option_hash().unwrap(), None);

        let key = hash_bytes(b"test", &[b"bad-key"]);
        let mut bad_job = Vec::new();
        write_len(&mut bad_job, 1);
        write_hash(&mut bad_job, &key);
        bad_job.push(9);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job)),
            Err(TvmError::Storage("unknown job tag"))
        );

        let bad_job_template = JobState::TensorOp(MatmulJob::synthetic(
            0,
            2,
            2,
            2,
            2,
            &hash_bytes(b"test", &[b"bad-job-beacon"]),
            10,
        ));

        let mut bad_job_dtype = Vec::new();
        write_len(&mut bad_job_dtype, 1);
        write_hash(&mut bad_job_dtype, &key);
        let mut bad_job_dtype_payload = payload_codec::encode_job_payload(&bad_job_template);
        bad_job_dtype_payload[TENSOR_DTYPE_OFFSET] = 9;
        bad_job_dtype.extend_from_slice(&bad_job_dtype_payload);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job_dtype)),
            Err(TvmError::Storage("unknown dtype"))
        );

        let mut bad_job_optional = Vec::new();
        write_len(&mut bad_job_optional, 1);
        write_hash(&mut bad_job_optional, &key);
        let mut bad_job_optional_payload = payload_codec::encode_job_payload(&bad_job_template);
        bad_job_optional_payload[TENSOR_OPTIONAL_MODULUS_OFFSET] = 9;
        bad_job_optional.extend_from_slice(&bad_job_optional_payload);
        assert_eq!(
            decode_jobs(&mut StateReader::new(&bad_job_optional)),
            Err(TvmError::Storage("invalid optional u64"))
        );

        let mut bad_receipt = Vec::new();
        write_len(&mut bad_receipt, 1);
        write_hash(&mut bad_receipt, &key);
        bad_receipt.push(9);
        assert_eq!(
            decode_receipts(&mut StateReader::new(&bad_receipt)),
            Err(TvmError::Storage("unknown receipt tag"))
        );
    }
}
