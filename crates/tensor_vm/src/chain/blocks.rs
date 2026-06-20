use super::roots::{
    attestation_root, block_checks_root, reward_root, selected_receipt_root, state_root,
};
use super::{
    BlockAdmission, BlockInvalidReason, BlockspaceCaps, BlockspaceSelection, Chain, ChainCommand,
    ChainEngine, ChainState, ReceiptState, TensorBlock,
};
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash, hash_bytes, sign, verify_signature};
use num_bigint::BigUint;
use std::collections::BTreeSet;

pub(super) fn produce(chain: &mut Chain, proposer: Address, timestamp: u64) -> Result<TensorBlock> {
    if !chain.state.validators.contains_key(&proposer) {
        return Err(TvmError::UnknownValidator);
    }

    let parent_hash = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let beacon = chain.state.finalized_randomness;
    let selection = canonical_blockspace(&chain.state, &parent_hash, &beacon, blockspace_caps());
    let selected_set = selection.receipt_set();
    let settled_receipt_set_root = selected_receipt_root(&selected_set);
    let checks_root = block_checks_root(&selection.receipt_ids, &chain.state.attestations);
    let attestation_root = attestation_root(&chain.state.attestations);
    let chain_state_root = state_root(&chain.state);
    let reward_root = reward_root(&chain.state.rewards);
    let production_kind = if selection.receipt_ids.is_empty() {
        super::BlockProductionKind::PowSkipFallback
    } else {
        super::BlockProductionKind::UsefulVerificationPow
    };
    let difficulty_target = expected_difficulty_target(chain, chain.state.height);
    let mut block = TensorBlock {
        height: chain.state.height,
        parent_hash,
        epoch: chain.state.epoch,
        proposer,
        settled_receipt_set_root,
        checks_root,
        attestation_root,
        state_root: chain_state_root,
        reward_root,
        beacon,
        production_kind,
        difficulty_target,
        nonce: 0,
        timestamp,
        proposer_signature: [0; 32],
        validator_signature_aggregate: [0; 32],
    };
    if block.production_kind.requires_pow() {
        block.nonce = find_nonce(&block);
    }
    let block_hash = block.hash();
    block.proposer_signature = sign(&proposer, &block_hash);
    block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
    validate(chain, &block, true)?;

    chain.blocks.push(block.clone());
    chain
        .state
        .block_selected_receipts
        .insert(block_hash, selection.receipt_ids.clone());
    for receipt_id in &selection.receipt_ids {
        chain.state.included_receipts.insert(*receipt_id);
    }
    chain.state.height += 1;
    chain.state.epoch = chain.state.height / chain.params.epoch_length.max(1);
    chain.state.finalized_randomness =
        next_finalized_randomness(&beacon, &block_hash, chain.state.height);
    if block.production_kind.requires_pow() && !block.pow_valid() {
        return Err(TvmError::InvalidReceipt(
            "invalid useful-verification proof",
        ));
    }
    Ok(block)
}

pub(super) fn produce_with_rewards(
    chain: &mut Chain,
    proposer: Address,
    timestamp: u64,
    fixed_block_reward: u64,
    fee_share: u64,
) -> Result<TensorBlock> {
    if !chain.state.validators.contains_key(&proposer) {
        return Err(TvmError::UnknownValidator);
    }
    let rewards_before = chain.state.rewards.clone();
    let proposer_reward = fixed_block_reward.saturating_add(fee_share);
    if proposer_reward > 0 {
        chain.state.rewards.credit(proposer, proposer_reward);
    }
    match produce(chain, proposer, timestamp) {
        Ok(block) => Ok(block),
        Err(error) => {
            chain.state.rewards = rewards_before;
            Err(error)
        }
    }
}

pub(super) fn prepare_parent_state(chain: &mut Chain) -> Result<()> {
    let settled_before = chain.state.settled_receipts.clone();
    chain.apply_command(ChainCommand::SettleEpoch {
        miner_reward_pool: 1_000,
        validator_reward_pool: 500,
    })?;
    let newly_settled = chain
        .state
        .settled_receipts
        .difference(&settled_before)
        .copied()
        .collect::<Vec<_>>();
    for receipt_id in newly_settled {
        let Some(ReceiptState::LinearTrainingStep(receipt)) =
            chain.state.receipts.get(&receipt_id).cloned()
        else {
            continue;
        };
        chain.apply_command(ChainCommand::ApplyModelTransition {
            model_id: receipt.model_id,
            step: receipt.step,
            weight_root_before: receipt.weight_root_before,
            weight_root_after: receipt.weight_root_after,
        })?;
    }
    Ok(())
}

pub(super) fn admit(chain: &mut Chain, block: TensorBlock) -> Result<BlockAdmission> {
    let block_hash = block.hash();
    let height = block.height;
    if let Some(existing) = chain
        .blocks
        .iter()
        .find(|candidate| candidate.height == block.height)
    {
        if existing.hash() == block_hash {
            return Ok(BlockAdmission::Duplicate {
                height,
                hash: block_hash,
            });
        }
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::ConflictingHeight,
        });
    }
    if block.height != chain.state.height {
        return Ok(BlockAdmission::PendingParent {
            height,
            parent_hash: block.parent_hash,
        });
    }
    let expected_parent = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    if block.parent_hash != expected_parent {
        return Ok(BlockAdmission::PendingParent {
            height,
            parent_hash: block.parent_hash,
        });
    }

    validate(chain, &block, true)?;
    let beacon = block.beacon;
    let selection =
        canonical_blockspace(&chain.state, &block.parent_hash, &beacon, blockspace_caps());
    chain.blocks.push(block);
    chain
        .state
        .block_selected_receipts
        .insert(block_hash, selection.receipt_ids.clone());
    for receipt_id in &selection.receipt_ids {
        chain.state.included_receipts.insert(*receipt_id);
    }
    chain.state.height += 1;
    chain.state.epoch = chain.state.height / chain.params.epoch_length.max(1);
    chain.state.finalized_randomness =
        next_finalized_randomness(&beacon, &block_hash, chain.state.height);
    Ok(BlockAdmission::Applied {
        height,
        hash: block_hash,
    })
}

pub(super) fn blockspace_caps() -> BlockspaceCaps {
    BlockspaceCaps::default()
}

pub(super) fn expected_difficulty_target(chain: &Chain, height: u64) -> Hash {
    let parent_target = if height == 0 {
        chain.params.difficulty_initial_target
    } else {
        chain
            .blocks
            .iter()
            .find(|block| block.height + 1 == height)
            .or_else(|| chain.blocks.last())
            .map(|block| block.difficulty_target)
            .unwrap_or(chain.params.difficulty_initial_target)
    };
    let interval = chain.params.difficulty_retarget_epoch_length.max(1);
    let window_start_height = height.saturating_sub(interval);
    let window = chain
        .blocks
        .iter()
        .filter(|block| block.height >= window_start_height && block.height < height)
        .collect::<Vec<_>>();
    if height == 0 || !height.is_multiple_of(interval) || window.len() < interval as usize {
        return clamp_target(&chain.params, parent_target);
    }

    let Some(first) = window.first() else {
        return clamp_target(&chain.params, parent_target);
    };
    let Some(last) = window.last() else {
        return clamp_target(&chain.params, parent_target);
    };
    let parent_target = chain
        .blocks
        .iter()
        .find(|block| block.height + 1 == height)
        .map(|block| block.difficulty_target)
        .unwrap_or(chain.params.difficulty_initial_target);

    let observed_time = last.timestamp.saturating_sub(first.timestamp).max(1);
    let target_epoch_time = chain
        .params
        .difficulty_target_block_time_seconds
        .max(1)
        .saturating_mul(interval)
        .max(1);
    let max_ratio = chain.params.difficulty_retarget_max_ratio.max(1);
    let (numerator, denominator) = bounded_ratio(observed_time, target_epoch_time, max_ratio);
    clamp_target(
        &chain.params,
        scale_target(parent_target, numerator, denominator),
    )
}

fn bounded_ratio(observed_time: u64, target_epoch_time: u64, max_ratio: u64) -> (u64, u64) {
    let observed = observed_time as u128;
    let target = target_epoch_time.max(1) as u128;
    let max = max_ratio.max(1) as u128;
    if observed.saturating_mul(max) < target {
        (1, max_ratio.max(1))
    } else if observed > target.saturating_mul(max) {
        (max_ratio.max(1), 1)
    } else {
        (observed_time, target_epoch_time.max(1))
    }
}

fn scale_target(target: Hash, numerator: u64, denominator: u64) -> Hash {
    let value = BigUint::from_bytes_be(&target);
    let scaled = value * BigUint::from(numerator) / BigUint::from(denominator.max(1));
    biguint_to_hash(scaled)
}

fn biguint_to_hash(value: BigUint) -> Hash {
    let bytes = value.to_bytes_be();
    if bytes.len() > 32 {
        return [0xff; 32];
    }
    let mut out = [0; 32];
    out[32 - bytes.len()..].copy_from_slice(&bytes);
    out
}

fn clamp_target(params: &super::ChainParams, target: Hash) -> Hash {
    target
        .max(params.difficulty_floor_target)
        .min(params.difficulty_ceiling_target)
}

pub(super) fn canonical_blockspace(
    state: &ChainState,
    parent_hash: &Hash,
    beacon: &Hash,
    caps: BlockspaceCaps,
) -> BlockspaceSelection {
    let mut candidates = Vec::new();
    for receipt_id in &state.settled_receipts {
        if state.included_receipts.contains(receipt_id) {
            continue;
        }
        if state.data_unavailable_receipts.contains(receipt_id) {
            continue;
        }
        let Some(receipt) = state.receipts.get(receipt_id) else {
            continue;
        };
        let draw = hash_bytes(
            b"tensor-vm-settled-receipt-draw",
            &[beacon, parent_hash, receipt_id],
        );
        candidates.push((
            draw,
            *receipt_id,
            receipt.tensor_work_units(),
            receipt.estimated_block_bytes(),
        ));
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

    let mut receipt_ids = Vec::new();
    let mut total_tensor_work_units = 0_u64;
    let mut total_bytes = 0_u64;
    for (_, receipt_id, tensor_work_units, bytes) in candidates {
        if receipt_ids.len() >= caps.max_receipts {
            break;
        }
        let next_twu = total_tensor_work_units.saturating_add(tensor_work_units);
        let next_bytes = total_bytes.saturating_add(bytes);
        if next_twu > caps.max_tensor_work_units || next_bytes > caps.max_bytes {
            continue;
        }
        receipt_ids.push(receipt_id);
        total_tensor_work_units = next_twu;
        total_bytes = next_bytes;
    }

    BlockspaceSelection {
        receipt_ids,
        total_tensor_work_units,
        total_bytes,
        caps,
    }
}

pub(super) fn validate(chain: &Chain, block: &TensorBlock, strict_state_root: bool) -> Result<()> {
    if !chain.state.validators.contains_key(&block.proposer) {
        return Err(TvmError::UnknownValidator);
    }
    if !parent_matches(chain, block) {
        return Err(TvmError::InvalidReceipt("block parent mismatch"));
    }
    if block.difficulty_target != expected_difficulty_target(chain, block.height) {
        return Err(TvmError::InvalidReceipt("block difficulty target mismatch"));
    }
    if block.production_kind.requires_pow() && !block.pow_valid() {
        return Err(TvmError::InvalidReceipt(
            "invalid useful-verification proof",
        ));
    }
    if matches!(
        block.production_kind,
        super::BlockProductionKind::PowSkipFallback
    ) && block.nonce != 0
    {
        return Err(TvmError::InvalidReceipt("fallback nonce must be zero"));
    }
    let block_hash = block.hash();
    if !verify_signature(&block.proposer, &block_hash, &block.proposer_signature) {
        return Err(TvmError::InvalidReceipt("bad block proposer signature"));
    }
    if block.validator_signature_aggregate
        != hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash])
    {
        return Err(TvmError::InvalidReceipt(
            "bad block validator signature aggregate",
        ));
    }

    let parent_state = parent_state_for_validation(chain, block);
    if block.beacon != parent_state.finalized_randomness {
        return Err(TvmError::InvalidReceipt("block beacon mismatch"));
    }
    let selection = canonical_blockspace(
        &parent_state,
        &block.parent_hash,
        &block.beacon,
        blockspace_caps(),
    );
    let selected_receipts = match chain.state.block_selected_receipts.get(&block_hash) {
        Some(receipts) => {
            if *receipts != selection.receipt_ids {
                return Err(TvmError::InvalidReceipt(
                    "noncanonical block receipt selection",
                ));
            }
            receipts.clone()
        }
        None => selection.receipt_ids,
    };
    match block.production_kind {
        super::BlockProductionKind::UsefulVerificationPow => {
            if selected_receipts.is_empty() {
                return Err(TvmError::InvalidReceipt(
                    "useful pow requires selected receipts",
                ));
            }
        }
        super::BlockProductionKind::PowSkipFallback => {
            if !selected_receipts.is_empty() {
                return Err(TvmError::InvalidReceipt(
                    "fallback requires zero selected receipts",
                ));
            }
        }
    }
    let selected_set: BTreeSet<Hash> = selected_receipts.iter().copied().collect();
    if block.settled_receipt_set_root != selected_receipt_root(&selected_set) {
        return Err(TvmError::InvalidReceipt("noncanonical settled receipt set"));
    }
    if block.checks_root != block_checks_root(&selected_receipts, &parent_state.attestations) {
        return Err(TvmError::InvalidReceipt("block checks root mismatch"));
    }
    if block.attestation_root != attestation_root(&parent_state.attestations) {
        return Err(TvmError::InvalidReceipt("block attestation root mismatch"));
    }
    if block.reward_root != reward_root(&parent_state.rewards) {
        return Err(TvmError::InvalidReceipt("block reward root mismatch"));
    }
    if strict_state_root && block.state_root != state_root(&parent_state) {
        return Err(TvmError::InvalidReceipt("block state root mismatch"));
    }
    Ok(())
}

pub(super) fn selected_receipts(chain: &Chain, block: &TensorBlock) -> Vec<Hash> {
    let block_hash = block.hash();
    chain
        .state
        .block_selected_receipts
        .get(&block_hash)
        .cloned()
        .unwrap_or_else(|| {
            canonical_blockspace(
                &parent_state_for_validation(chain, block),
                &block.parent_hash,
                &block.beacon,
                blockspace_caps(),
            )
            .receipt_ids
        })
}

fn parent_state_for_validation(chain: &Chain, block: &TensorBlock) -> ChainState {
    let mut parent_state = chain.state.clone();
    let block_hash = block.hash();
    parent_state.height = block.height;
    parent_state.epoch = block.epoch;
    parent_state.finalized_randomness = expected_parent_beacon(chain, block);
    for candidate in chain
        .blocks
        .iter()
        .filter(|candidate| candidate.height >= block.height)
    {
        let candidate_hash = candidate.hash();
        if let Some(receipts) = parent_state
            .block_selected_receipts
            .get(&candidate_hash)
            .cloned()
        {
            for receipt_id in receipts {
                parent_state.included_receipts.remove(&receipt_id);
            }
        }
        parent_state.block_votes.remove(&candidate_hash);
        parent_state.finalized_blocks.remove(&candidate_hash);
    }
    parent_state.block_votes.remove(&block_hash);
    parent_state.finalized_blocks.remove(&block_hash);
    parent_state
}

fn expected_parent_beacon(chain: &Chain, block: &TensorBlock) -> Hash {
    if block.height == 0 {
        return chain.state.genesis_randomness;
    }
    chain
        .blocks
        .iter()
        .find(|candidate| candidate.height + 1 == block.height)
        .map(|parent| next_finalized_randomness(&parent.beacon, &parent.hash(), block.height))
        .unwrap_or(chain.state.finalized_randomness)
}

fn parent_matches(chain: &Chain, block: &TensorBlock) -> bool {
    if block.height == 0 {
        return block.parent_hash == [0; 32];
    }
    chain.blocks.iter().any(|candidate| {
        candidate.height + 1 == block.height && candidate.hash() == block.parent_hash
    }) || chain.blocks.last().is_some_and(|candidate| {
        candidate.height + 1 == block.height && candidate.hash() == block.parent_hash
    })
}

fn find_nonce(block: &TensorBlock) -> u64 {
    let mut candidate = block.clone();
    for nonce in 0..=u64::MAX {
        candidate.nonce = nonce;
        if candidate.pow_valid() {
            return nonce;
        }
    }
    unreachable!("nonzero proof target must have a solution")
}

fn next_finalized_randomness(beacon: &Hash, block_hash: &Hash, next_height: u64) -> Hash {
    hash_bytes(
        b"tensor-vm-finalized-beacon",
        &[beacon, block_hash, &next_height.to_le_bytes()],
    )
}
