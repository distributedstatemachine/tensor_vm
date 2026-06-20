use super::roots::{
    attestation_root, block_check_leaves, block_checks_root, data_unavailability_slash_root,
    hash_set_root, reward_root, selected_receipt_commitment_root, selected_receipt_leaves,
    selected_receipt_root, state_root,
};
use super::{
    BlockAdmission, BlockApplyOutcome, BlockInvalidReason, BlockParentSnapshot, BlockspaceCaps,
    BlockspaceSelection, Chain, ChainCommand, ChainEngine, ChainState,
    DataUnavailabilitySlashRecord, PendingProposerReward, ReceiptRewardKind, ReceiptState,
    SelectedReceiptOpening, TensorBlock, ValidatorAuditAssignment, ValidatorAuditSlashRecord,
};
use crate::error::{Result, TvmError};
use crate::merkle::{build_proof, merkle_root, verify_proof};
use crate::types::{Address, Hash, hash_bytes, sign, verify_signature};
use num_bigint::BigUint;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BlockRewardContext {
    proposer: Address,
    proposer_reward: u64,
    reward_settlement_delay_epochs: u64,
    challenge_window_epochs: u64,
    proposer_reward_hold_epochs: u64,
    data_unavailability_miner_slash_amount: u64,
    validator_audit_sample_numerator: u64,
    validator_audit_sample_denominator: u64,
    validator_audit_window_blocks: u64,
    validator_audit_slash_amount: u64,
}

impl BlockRewardContext {
    fn reward_maturity_delay_blocks(self, epoch_length: u64) -> u64 {
        self.reward_settlement_delay_epochs
            .saturating_add(self.challenge_window_epochs)
            .max(1)
            .saturating_mul(epoch_length.max(1))
    }

    fn proposer_claimable_at_height(self, block_height: u64, epoch_length: u64) -> u64 {
        block_height.saturating_add(
            self.reward_maturity_delay_blocks(epoch_length)
                .saturating_add(
                    self.proposer_reward_hold_epochs
                        .saturating_mul(epoch_length.max(1)),
                ),
        )
    }
}

pub(super) fn produce(chain: &mut Chain, proposer: Address, timestamp: u64) -> Result<TensorBlock> {
    produce_inner(chain, proposer, timestamp, 0)
}

fn produce_inner(
    chain: &mut Chain,
    proposer: Address,
    timestamp: u64,
    pending_proposer_reward: u64,
) -> Result<TensorBlock> {
    if !chain.state.validators.contains_key(&proposer) {
        return Err(TvmError::UnknownValidator);
    }
    if chain
        .state
        .proposer_penalty_until
        .get(&proposer)
        .is_some_and(|penalty_until| chain.state.height < *penalty_until)
    {
        return Err(TvmError::InvalidReceipt("proposer is challenge-throttled"));
    }

    let parent_hash = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let parent_state = chain.state.clone();
    let beacon_round = chain.state.finalized_beacon_round;
    let beacon = chain.state.finalized_randomness;
    let selection = canonical_blockspace(
        &chain.state,
        &parent_hash,
        beacon_round,
        &beacon,
        blockspace_caps(),
    );
    let settled_receipt_set_root =
        selected_receipt_commitment_root(&selection.receipt_ids, &chain.state.receipts);
    let checks_root = block_checks_root(
        &selection.receipt_ids,
        &chain.state.receipts,
        &chain.state.attestations,
        beacon_round,
        &beacon,
        &parent_hash,
    );
    let attestation_root = attestation_root(&chain.state.attestations);
    let production_kind = if selection.receipt_ids.is_empty() {
        super::BlockProductionKind::PowSkipFallback
    } else {
        super::BlockProductionKind::UsefulVerificationPow
    };
    let difficulty_target = expected_difficulty_target(chain, chain.state.height);
    let child_state = apply_block_to_parent_state(
        &chain.state,
        chain.params.epoch_length,
        beacon_round,
        &beacon,
        chain.state.height,
        &selection.receipt_ids,
        BlockRewardContext {
            proposer,
            proposer_reward: pending_proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    );
    let chain_state_root = state_root(&child_state);
    let reward_root = reward_root(&child_state);
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
        beacon_round,
        beacon,
        production_kind,
        proposer_reward: pending_proposer_reward,
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
    chain.state = child_state;
    chain
        .state
        .block_selected_receipts
        .insert(block_hash, selection.receipt_ids.clone());
    chain.block_parent_states.insert(block_hash, parent_state);
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
    let proposer_reward = fixed_block_reward.saturating_add(fee_share);
    produce_inner(chain, proposer, timestamp, proposer_reward)
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
    if chain.side_branch_blocks.contains_key(&block_hash) {
        return Ok(BlockAdmission::Duplicate {
            height,
            hash: block_hash,
        });
    }
    if let Some((existing_index, existing)) = chain
        .blocks
        .iter()
        .enumerate()
        .find(|(_, candidate)| candidate.height == block.height)
    {
        if existing.hash() == block_hash {
            return Ok(BlockAdmission::Duplicate {
                height,
                hash: block_hash,
            });
        }
        if existing_index + 1 != chain.blocks.len() || block.parent_hash != existing.parent_hash {
            return admit_side_branch(chain, block, block_hash);
        }
        return admit_competing_head(chain, existing_index, block, block_hash);
    }
    if block.height != chain.state.height {
        return admit_side_branch(chain, block, block_hash);
    }
    let expected_parent = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    if block.parent_hash != expected_parent {
        return admit_side_branch(chain, block, block_hash);
    }

    validate(chain, &block, true)?;
    let parent_state = chain.state.clone();
    let outcome = apply_outcome(chain, &block)?;
    chain.blocks.push(block.clone());
    chain
        .state
        .block_selected_receipts
        .insert(block_hash, outcome.selected_receipt_ids.clone());
    chain.block_parent_states.insert(block_hash, parent_state);
    chain.state = apply_block_to_parent_state(
        &chain.state,
        chain.params.epoch_length,
        block.beacon_round,
        &block.beacon,
        height,
        &outcome.selected_receipt_ids,
        BlockRewardContext {
            proposer: block.proposer,
            proposer_reward: block.proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    );
    Ok(BlockAdmission::Applied {
        height,
        hash: block_hash,
    })
}

fn admit_side_branch(
    chain: &mut Chain,
    block: TensorBlock,
    block_hash: Hash,
) -> Result<BlockAdmission> {
    let height = block.height;
    if chain
        .blocks
        .iter()
        .any(|canonical| canonical.height == height && chain.is_block_finalized(&canonical.hash()))
    {
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::FinalizedConflict,
        });
    }
    let Some(parent_state) = known_sibling_parent_state(chain, &block)
        .or_else(|| known_parent_child_state(chain, &block.parent_hash))
    else {
        return Ok(BlockAdmission::PendingParent {
            height,
            parent_hash: block.parent_hash,
        });
    };
    let mut validation_chain = chain.clone();
    validation_chain
        .block_parent_states
        .insert(block_hash, parent_state.clone());
    validate(&validation_chain, &block, true)?;
    let outcome = apply_outcome(&validation_chain, &block)?;
    let mut child_state = apply_block_to_parent_state(
        &parent_state,
        chain.params.epoch_length,
        block.beacon_round,
        &block.beacon,
        height,
        &outcome.selected_receipt_ids,
        BlockRewardContext {
            proposer: block.proposer,
            proposer_reward: block.proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    );
    child_state
        .block_selected_receipts
        .insert(block_hash, outcome.selected_receipt_ids);
    chain.block_parent_states.insert(block_hash, parent_state);
    let parent_hash = block.parent_hash;
    chain
        .side_branch_child_states
        .insert(block_hash, child_state);
    chain.side_branch_blocks.insert(block_hash, block);
    if let Some(admission) = try_promote_side_branch(chain, block_hash) {
        return Ok(admission);
    }
    Ok(BlockAdmission::SideBranchStored {
        height,
        parent_hash,
        hash: block_hash,
    })
}

fn try_promote_side_branch(chain: &mut Chain, tip_hash: Hash) -> Option<BlockAdmission> {
    let tip = chain.side_branch_blocks.get(&tip_hash)?.clone();
    if tip.height.saturating_add(1) <= chain.state.height {
        return None;
    }

    let mut path = Vec::new();
    let mut cursor_hash = tip_hash;
    let ancestor_index = loop {
        let branch_block = chain.side_branch_blocks.get(&cursor_hash)?.clone();
        path.push(branch_block.clone());
        if branch_block.parent_hash == [0; 32] {
            break None;
        }
        if let Some(index) = chain
            .blocks
            .iter()
            .position(|canonical| canonical.hash() == branch_block.parent_hash)
        {
            break Some(index);
        }
        cursor_hash = branch_block.parent_hash;
    };

    let replaced_start = ancestor_index.map_or(0, |index| index + 1);
    if chain.blocks[replaced_start..]
        .iter()
        .any(|canonical| chain.is_block_finalized(&canonical.hash()))
    {
        return None;
    }

    path.reverse();
    let old_head = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let old_child_states = chain.blocks[replaced_start..]
        .iter()
        .filter_map(|block| {
            Some((
                block.hash(),
                known_parent_child_state(chain, &block.hash())?,
            ))
        })
        .collect::<Vec<_>>();
    let old_suffix = chain.blocks.split_off(replaced_start);
    let old_suffix_hashes = old_suffix
        .iter()
        .map(TensorBlock::hash)
        .collect::<BTreeSet<_>>();
    for old_hash in &old_suffix_hashes {
        chain.state.block_selected_receipts.remove(old_hash);
        chain.state.block_votes.remove(old_hash);
        chain.state.finalized_blocks.remove(old_hash);
    }

    for block in &path {
        chain.blocks.push(block.clone());
        chain.side_branch_blocks.remove(&block.hash());
    }
    let new_head_hash = tip_hash;
    chain.state = chain
        .side_branch_child_states
        .remove(&new_head_hash)
        .unwrap_or_else(|| known_parent_child_state(chain, &new_head_hash).unwrap());
    for block in &path {
        chain.side_branch_child_states.remove(&block.hash());
    }

    for old_block in old_suffix {
        let old_hash = old_block.hash();
        chain
            .side_branch_blocks
            .entry(old_hash)
            .or_insert(old_block);
        if !chain.side_branch_child_states.contains_key(&old_hash)
            && let Some((_, child_state)) = old_child_states
                .iter()
                .find(|(candidate_hash, _)| *candidate_hash == old_hash)
        {
            chain
                .side_branch_child_states
                .insert(old_hash, child_state.clone());
        } else if !chain.side_branch_child_states.contains_key(&old_hash)
            && let Some(child_state) = known_parent_child_state(chain, &old_hash)
        {
            chain.side_branch_child_states.insert(old_hash, child_state);
        }
    }

    Some(BlockAdmission::Reorganized {
        height: tip.height,
        old_head,
        hash: new_head_hash,
    })
}

fn admit_competing_head(
    chain: &mut Chain,
    existing_index: usize,
    block: TensorBlock,
    block_hash: Hash,
) -> Result<BlockAdmission> {
    let height = block.height;
    if existing_index + 1 != chain.blocks.len() || chain.state.height != height.saturating_add(1) {
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::ConflictingHeight,
        });
    }

    let existing = chain.blocks[existing_index].clone();
    let existing_hash = existing.hash();
    if chain.is_block_finalized(&existing_hash) {
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::FinalizedConflict,
        });
    }
    if block.parent_hash != existing.parent_hash {
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::ConflictingHeight,
        });
    }
    let parent_state = chain
        .block_parent_states
        .get(&existing_hash)
        .cloned()
        .unwrap_or_else(|| parent_state_for_validation(chain, &existing));
    let mut validation_chain = chain.clone();
    validation_chain
        .block_parent_states
        .insert(block_hash, parent_state.clone());
    validate(&validation_chain, &block, true)?;
    if !competing_head_preferred(&block, &existing) {
        return Ok(BlockAdmission::Invalid {
            height,
            hash: block_hash,
            reason: BlockInvalidReason::NonPreferredCompetingHead,
        });
    }

    let mut parent_state = parent_state;
    parent_state.block_selected_receipts.remove(&existing_hash);
    parent_state.block_selected_receipts.remove(&block_hash);
    let outcome = apply_outcome(&validation_chain, &block)?;
    chain.blocks[existing_index] = block.clone();
    chain.block_parent_states.remove(&existing_hash);
    chain
        .block_parent_states
        .insert(block_hash, parent_state.clone());
    chain.state = apply_block_to_parent_state(
        &parent_state,
        chain.params.epoch_length,
        block.beacon_round,
        &block.beacon,
        height,
        &outcome.selected_receipt_ids,
        BlockRewardContext {
            proposer: block.proposer,
            proposer_reward: block.proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    );
    chain
        .state
        .block_selected_receipts
        .insert(block_hash, outcome.selected_receipt_ids);
    Ok(BlockAdmission::Replaced {
        height,
        old_hash: existing_hash,
        hash: block_hash,
    })
}

fn competing_head_preferred(candidate: &TensorBlock, existing: &TensorBlock) -> bool {
    if candidate.production_kind != super::BlockProductionKind::UsefulVerificationPow
        || existing.production_kind != super::BlockProductionKind::UsefulVerificationPow
    {
        return false;
    }
    candidate
        .pow_hash()
        .cmp(&existing.pow_hash())
        .then_with(|| candidate.hash().cmp(&existing.hash()))
        .is_lt()
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
    beacon_round: u64,
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
            &[&beacon_round.to_le_bytes(), beacon, parent_hash, receipt_id],
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

    let outcome = apply_outcome(chain, block)?;
    let parent_state = parent_state_for_validation(chain, block);
    if block.beacon_round != outcome.parent_snapshot.beacon_round {
        return Err(TvmError::InvalidReceipt("block beacon round mismatch"));
    }
    if block.beacon != outcome.parent_snapshot.beacon {
        return Err(TvmError::InvalidReceipt("block beacon mismatch"));
    }
    let selected_receipts = outcome.selected_receipt_ids.clone();
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
            validate_fallback_timeout(chain, block)?;
            validate_fallback_proposer(&parent_state, block)?;
        }
    }
    if block.settled_receipt_set_root != outcome.selected_receipt_root {
        return Err(TvmError::InvalidReceipt("noncanonical settled receipt set"));
    }
    if block.checks_root != outcome.checks_root {
        return Err(TvmError::InvalidReceipt("block checks root mismatch"));
    }
    if block.attestation_root != attestation_root(&parent_state.attestations) {
        return Err(TvmError::InvalidReceipt("block attestation root mismatch"));
    }
    if block.reward_root != outcome.child_reward_root {
        return Err(TvmError::InvalidReceipt("block reward root mismatch"));
    }
    if strict_state_root && block.state_root != outcome.child_state_root {
        return Err(TvmError::InvalidReceipt("block state root mismatch"));
    }
    Ok(())
}

fn validate_fallback_timeout(chain: &Chain, block: &TensorBlock) -> Result<()> {
    if block.height == 0 {
        return Ok(());
    }
    let Some(parent) = known_block(chain, &block.parent_hash) else {
        return Err(TvmError::InvalidReceipt("block parent mismatch"));
    };
    if parent.height + 1 != block.height {
        return Err(TvmError::InvalidReceipt("block parent mismatch"));
    }
    let timeout_seconds = chain
        .params
        .pow_timeout_blocks
        .max(1)
        .saturating_mul(chain.params.block_time_seconds.max(1));
    let earliest_fallback_at = parent.timestamp.saturating_add(timeout_seconds);
    if block.timestamp < earliest_fallback_at {
        return Err(TvmError::InvalidReceipt("fallback before pow timeout"));
    }
    Ok(())
}

fn validate_fallback_proposer(parent_state: &ChainState, block: &TensorBlock) -> Result<()> {
    let Some(expected_proposer) = super::proposer::for_next_epoch(parent_state, &block.beacon)
    else {
        return Err(TvmError::UnknownValidator);
    };
    if block.proposer != expected_proposer {
        return Err(TvmError::InvalidReceipt(
            "fallback proposer is not selected",
        ));
    }
    Ok(())
}

pub(super) fn apply_outcome(chain: &Chain, block: &TensorBlock) -> Result<BlockApplyOutcome> {
    let parent_state = parent_state_for_validation(chain, block);
    let parent_snapshot = parent_snapshot(block, &parent_state);
    let selection = canonical_blockspace(
        &parent_state,
        &block.parent_hash,
        block.beacon_round,
        &block.beacon,
        blockspace_caps(),
    );
    let block_hash = block.hash();
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
    let selected_receipt_root =
        selected_receipt_commitment_root(&selected_receipts, &parent_state.receipts);
    let checks_root = block_checks_root(
        &selected_receipts,
        &parent_state.receipts,
        &parent_state.attestations,
        block.beacon_round,
        &block.beacon,
        &block.parent_hash,
    );
    let child_state = apply_block_to_parent_state(
        &parent_state,
        chain.params.epoch_length,
        block.beacon_round,
        &block.beacon,
        block.height,
        &selected_receipts,
        BlockRewardContext {
            proposer: block.proposer,
            proposer_reward: block.proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    );
    let selected_openings = selected_receipt_openings(
        &parent_state,
        chain.params.tensor_retention_window_blocks(),
        block.beacon_round,
        &block.beacon,
        &block.parent_hash,
        &selected_receipts,
    );
    Ok(BlockApplyOutcome {
        parent_snapshot,
        selected_receipt_ids: selected_receipts,
        selected_receipt_root,
        checks_root,
        selected_openings,
        child_state_root: state_root(&child_state),
        child_reward_root: reward_root(&child_state),
        child_height: child_state.height,
        child_epoch: child_state.epoch,
        child_beacon_round: child_state.finalized_beacon_round,
        child_beacon: child_state.finalized_randomness,
    })
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
                block.beacon_round,
                &block.beacon,
                blockspace_caps(),
            )
            .receipt_ids
        })
}

fn parent_state_for_validation(chain: &Chain, block: &TensorBlock) -> ChainState {
    if let Some(parent_state) = chain.block_parent_states.get(&block.hash()) {
        return parent_state.clone();
    }
    if block.parent_hash != [0; 32]
        && let Some(parent_state) = known_parent_child_state(chain, &block.parent_hash)
    {
        return parent_state;
    }

    let mut parent_state = chain.state.clone();
    let block_hash = block.hash();
    parent_state.height = block.height;
    parent_state.epoch = block.epoch;
    let parent_beacon = expected_parent_beacon(chain, block);
    parent_state.finalized_beacon_round = parent_beacon.0;
    parent_state.finalized_randomness = parent_beacon.1;
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
        parent_state.block_selected_receipts.remove(&candidate_hash);
        parent_state.block_votes.remove(&candidate_hash);
        parent_state.finalized_blocks.remove(&candidate_hash);
    }
    parent_state.block_selected_receipts.remove(&block_hash);
    parent_state.block_votes.remove(&block_hash);
    parent_state.finalized_blocks.remove(&block_hash);
    parent_state
}

fn known_parent_child_state(chain: &Chain, parent_hash: &Hash) -> Option<ChainState> {
    if *parent_hash == [0; 32] {
        let mut genesis_parent = chain.state.clone();
        genesis_parent.height = 0;
        genesis_parent.epoch = 0;
        genesis_parent.finalized_beacon_round = genesis_parent.genesis_beacon_round;
        genesis_parent.finalized_randomness = genesis_parent.genesis_randomness;
        return Some(genesis_parent);
    }
    if chain
        .blocks
        .last()
        .is_some_and(|block| block.hash() == *parent_hash)
    {
        return Some(chain.state.clone());
    }
    if let Some(child_state) = chain.side_branch_child_states.get(parent_hash) {
        return Some(child_state.clone());
    }
    let parent = chain
        .blocks
        .iter()
        .find(|candidate| candidate.hash() == *parent_hash)?;
    let parent_parent_state = chain.block_parent_states.get(parent_hash)?;
    Some(apply_block_to_parent_state(
        parent_parent_state,
        chain.params.epoch_length,
        parent.beacon_round,
        &parent.beacon,
        parent.height,
        &selected_receipts(chain, parent),
        BlockRewardContext {
            proposer: parent.proposer,
            proposer_reward: parent.proposer_reward,
            reward_settlement_delay_epochs: chain.params.reward_settlement_delay_epochs,
            challenge_window_epochs: chain.params.challenge_window_epochs,
            proposer_reward_hold_epochs: chain.params.proposer_reward_hold_epochs,
            data_unavailability_miner_slash_amount: chain
                .params
                .data_unavailability_miner_slash_amount,
            validator_audit_sample_numerator: chain.params.validator_audit_sample_numerator,
            validator_audit_sample_denominator: chain.params.validator_audit_sample_denominator,
            validator_audit_window_blocks: chain.params.validator_audit_window_blocks,
            validator_audit_slash_amount: chain.params.validator_audit_slash_amount,
        },
    ))
}

fn known_block<'a>(chain: &'a Chain, block_hash: &Hash) -> Option<&'a TensorBlock> {
    chain
        .blocks
        .iter()
        .find(|candidate| candidate.hash() == *block_hash)
        .or_else(|| chain.side_branch_blocks.get(block_hash))
}

fn known_sibling_parent_state(chain: &Chain, block: &TensorBlock) -> Option<ChainState> {
    chain
        .blocks
        .iter()
        .chain(chain.side_branch_blocks.values())
        .find(|sibling| sibling.height == block.height && sibling.parent_hash == block.parent_hash)
        .and_then(|sibling| chain.block_parent_states.get(&sibling.hash()))
        .cloned()
}

fn parent_snapshot(block: &TensorBlock, parent_state: &ChainState) -> BlockParentSnapshot {
    BlockParentSnapshot {
        parent_hash: block.parent_hash,
        height: parent_state.height,
        epoch: parent_state.epoch,
        state_root: state_root(parent_state),
        beacon_round: parent_state.finalized_beacon_round,
        beacon: parent_state.finalized_randomness,
        attestation_root: attestation_root(&parent_state.attestations),
        reward_root: reward_root(parent_state),
        settled_receipt_pool_root: selected_receipt_commitment_root(
            &parent_state
                .settled_receipts
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            &parent_state.receipts,
        ),
        included_receipt_root: hash_set_root(
            b"tensor-vm-included-receipt-root-v1",
            &parent_state.included_receipts,
        ),
        data_unavailable_receipt_root: hash_set_root(
            b"tensor-vm-data-unavailable-root-v1",
            &parent_state.data_unavailable_receipts,
        ),
        data_unavailability_slash_root: data_unavailability_slash_root(
            &parent_state.data_unavailability_slashes,
        ),
    }
}

fn apply_block_to_parent_state(
    parent_state: &ChainState,
    epoch_length: u64,
    beacon_round: u64,
    beacon: &Hash,
    block_height: u64,
    selected_receipts: &[Hash],
    reward_context: BlockRewardContext,
) -> ChainState {
    let mut child_state = parent_state.clone();
    let receipt_reward_claimable_at_height =
        block_height.saturating_add(reward_context.reward_maturity_delay_blocks(epoch_length));
    for receipt_id in selected_receipts {
        child_state.included_receipts.insert(*receipt_id);
        for reward in child_state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == *receipt_id {
                reward.claimable_at_height = reward
                    .claimable_at_height
                    .max(receipt_reward_claimable_at_height);
            }
        }
    }
    apply_data_unavailability_slashes(
        &mut child_state,
        block_height,
        reward_context.data_unavailability_miner_slash_amount,
    );
    apply_missed_validator_audit_slashes(
        &mut child_state,
        block_height,
        reward_context.validator_audit_slash_amount,
        reward_context.validator_audit_window_blocks,
    );
    assign_validator_audits(
        &mut child_state,
        block_height,
        beacon_round,
        beacon,
        reward_context.validator_audit_sample_numerator,
        reward_context.validator_audit_sample_denominator,
        reward_context.validator_audit_window_blocks,
    );
    super::commands::release_all_matured_rewards(&mut child_state);
    child_state.height = block_height.saturating_add(1);
    child_state.epoch = child_state.height / epoch_length.max(1);
    let (next_round, next_beacon) =
        next_finalized_beacon(beacon_round, beacon, child_state.height, child_state.epoch);
    child_state.finalized_beacon_round = next_round;
    child_state.finalized_randomness = next_beacon;
    if reward_context.proposer_reward > 0 {
        child_state.pending_proposer_rewards.insert(
            block_height,
            PendingProposerReward {
                block_height,
                proposer: reward_context.proposer,
                amount: reward_context.proposer_reward,
                claimable_at_height: reward_context
                    .proposer_claimable_at_height(block_height, epoch_length),
                voided_by_challenge: false,
            },
        );
    }
    child_state
}

fn apply_missed_validator_audit_slashes(
    child_state: &mut ChainState,
    block_height: u64,
    slash_amount: u64,
    audit_window_blocks: u64,
) {
    let assignments = child_state
        .validator_audit_assignments
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for assignment in assignments {
        if block_height < assignment.deadline_height {
            continue;
        }
        if child_state
            .validator_audit_results
            .contains_key(&assignment.audit_id)
            || child_state
                .validator_audit_slashes
                .contains_key(&assignment.audit_id)
        {
            continue;
        }
        let Some(validator) = child_state.validators.get_mut(&assignment.validator) else {
            continue;
        };
        let actual_slash = validator.stake.min(slash_amount);
        validator.stake = validator.stake.saturating_sub(actual_slash);
        validator.reputation -= 10;
        validator.missed_assignments = validator.missed_assignments.saturating_add(1);
        let reward_hold_until_height = block_height.saturating_add(audit_window_blocks.max(1));
        void_validator_audit_reward(
            child_state,
            &assignment.receipt_id,
            &assignment.validator,
            reward_hold_until_height,
        );
        child_state.rewards.credit_treasury(actual_slash);
        child_state.validator_audit_slashes.insert(
            assignment.audit_id,
            ValidatorAuditSlashRecord {
                audit_id: assignment.audit_id,
                receipt_id: assignment.receipt_id,
                validator: assignment.validator,
                auditor: assignment.auditor,
                amount: actual_slash,
                slashed_at_height: block_height,
                reason: "validator missed mandatory audit".to_owned(),
            },
        );
    }
}

fn assign_validator_audits(
    child_state: &mut ChainState,
    block_height: u64,
    beacon_round: u64,
    beacon: &Hash,
    sample_numerator: u64,
    sample_denominator: u64,
    audit_window_blocks: u64,
) {
    if sample_numerator == 0 {
        return;
    }
    let denominator = sample_denominator.max(1);
    let numerator = sample_numerator.min(denominator);
    let deadline_height = block_height.saturating_add(audit_window_blocks.max(1));
    let attestations = child_state
        .attestations
        .iter()
        .flat_map(|(receipt_id, items)| {
            items
                .iter()
                .map(|attestation| (*receipt_id, attestation.validator))
        })
        .collect::<Vec<_>>();
    for (receipt_id, validator) in attestations {
        let audit_id = super::validation::validator_audit_id(&receipt_id, &validator);
        if child_state
            .validator_audit_assignments
            .contains_key(&audit_id)
        {
            continue;
        }
        let seed =
            super::validation::validator_audit_seed(beacon_round, beacon, &receipt_id, &validator);
        let draw = u64::from_le_bytes(seed[..8].try_into().expect("slice has length 8"));
        if draw % denominator >= numerator {
            continue;
        }
        let Some(auditor) = select_validator_auditor(child_state, &validator, &seed) else {
            continue;
        };
        delay_validator_audit_reward(child_state, &receipt_id, &validator, deadline_height);
        child_state.validator_audit_assignments.insert(
            audit_id,
            ValidatorAuditAssignment {
                audit_id,
                receipt_id,
                validator,
                auditor,
                assigned_at_height: block_height,
                deadline_height,
                seed,
            },
        );
    }
}

fn select_validator_auditor(
    child_state: &ChainState,
    audited_validator: &Address,
    seed: &Hash,
) -> Option<Address> {
    let candidates = child_state
        .validators
        .keys()
        .copied()
        .filter(|validator| validator != audited_validator)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    let draw = u64::from_le_bytes(seed[8..16].try_into().expect("slice has length 8"));
    Some(candidates[(draw as usize) % candidates.len()])
}

fn delay_validator_audit_reward(
    child_state: &mut ChainState,
    receipt_id: &Hash,
    validator: &Address,
    claimable_at_height: u64,
) {
    for reward in child_state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == *receipt_id
            && reward.beneficiary == *validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            reward.claimable_at_height = reward.claimable_at_height.max(claimable_at_height);
        }
    }
}

fn void_validator_audit_reward(
    child_state: &mut ChainState,
    receipt_id: &Hash,
    validator: &Address,
    claimable_at_height: u64,
) {
    for reward in child_state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == *receipt_id
            && reward.beneficiary == *validator
            && reward.kind == ReceiptRewardKind::Validator
        {
            reward.claimable_at_height = reward.claimable_at_height.max(claimable_at_height);
            reward.voided_by_challenge = true;
        }
    }
}

fn apply_data_unavailability_slashes(
    child_state: &mut ChainState,
    block_height: u64,
    slash_amount: u64,
) {
    let unavailable_receipts = child_state
        .data_unavailable_receipts
        .iter()
        .copied()
        .collect::<Vec<_>>();
    for receipt_id in unavailable_receipts {
        if child_state
            .data_unavailability_slashes
            .contains_key(&receipt_id)
        {
            continue;
        }
        let Some(receipt) = child_state.receipts.get(&receipt_id) else {
            continue;
        };
        let miner_address = receipt.miner();
        let evidence_validator = child_state
            .attestations
            .get(&receipt_id)
            .and_then(|attestations| {
                attestations
                    .iter()
                    .find(|attestation| {
                        !attestation.data_availability_passed
                            || matches!(
                                attestation.result,
                                crate::verify::VerificationResult::Unavailable
                            )
                    })
                    .map(|attestation| attestation.validator)
            })
            .unwrap_or([0; 32]);
        let Some(miner) = child_state.miners.get_mut(&miner_address) else {
            continue;
        };
        let actual_slash = miner.stake.min(slash_amount);
        miner.stake = miner.stake.saturating_sub(actual_slash);
        child_state.rewards.credit_treasury(actual_slash);
        child_state.data_unavailability_slashes.insert(
            receipt_id,
            DataUnavailabilitySlashRecord {
                receipt_id,
                miner: miner_address,
                evidence_validator,
                amount: actual_slash,
                slashed_at_height: block_height,
                reason: "data unavailable for receipt verification".to_owned(),
            },
        );
    }
}

fn selected_receipt_openings(
    parent_state: &ChainState,
    retention_window_blocks: u64,
    beacon_round: u64,
    beacon: &Hash,
    parent_hash: &Hash,
    selected_receipts: &[Hash],
) -> Vec<SelectedReceiptOpening> {
    let receipt_leaves = selected_receipt_leaves(selected_receipts, &parent_state.receipts);
    let receipt_root = if selected_receipts.is_empty() {
        selected_receipt_root(&BTreeSet::new())
    } else {
        merkle_root(&receipt_leaves)
    };
    let check_leaves = block_check_leaves(
        selected_receipts,
        &parent_state.receipts,
        &parent_state.attestations,
        beacon_round,
        beacon,
        parent_hash,
    );
    let checks_root = merkle_root(&check_leaves);
    selected_receipts
        .iter()
        .enumerate()
        .map(|(index, receipt_id)| {
            let receipt = parent_state.receipts.get(receipt_id);
            let receipt_leaf = receipt_leaves[index];
            let receipt_leaf_proof = build_proof(&receipt_leaves, index as u64).ok();
            let check_leaf = check_leaves[index];
            let check_leaf_proof = build_proof(&check_leaves, index as u64).ok();
            let receipt_opening_valid = receipt_leaf_proof
                .as_ref()
                .is_some_and(|proof| verify_proof(&receipt_root, receipt_leaf, proof));
            let check_opening_valid = check_leaf_proof
                .as_ref()
                .is_some_and(|proof| verify_proof(&checks_root, check_leaf, proof));
            debug_assert!(receipt_opening_valid);
            debug_assert!(check_opening_valid);
            SelectedReceiptOpening {
                receipt_id: *receipt_id,
                receipt_leaf,
                receipt_leaf_index: index as u64,
                receipt_leaf_proof,
                check_leaf,
                check_leaf_index: index as u64,
                check_leaf_proof,
                primitive_type: receipt.map(ReceiptState::primitive_type),
                tensor_work_units: receipt.map_or(0, ReceiptState::tensor_work_units),
                estimated_block_bytes: receipt.map_or(0, ReceiptState::estimated_block_bytes),
                submitted_at_block: receipt.map_or(0, ReceiptState::submitted_at_block),
                settled: parent_state.settled_receipts.contains(receipt_id),
                included_before_parent: parent_state.included_receipts.contains(receipt_id),
                data_available: !parent_state.data_unavailable_receipts.contains(receipt_id),
                expires_at_block: receipt
                    .map(|receipt| {
                        parent_state
                            .height
                            .max(receipt.submitted_at_block())
                            .saturating_add(retention_window_blocks)
                    })
                    .unwrap_or(parent_state.height),
            }
        })
        .collect()
}

fn expected_parent_beacon(chain: &Chain, block: &TensorBlock) -> (u64, Hash) {
    if block.height == 0 {
        return (
            chain.state.genesis_beacon_round,
            chain.state.genesis_randomness,
        );
    }
    chain
        .blocks
        .iter()
        .chain(chain.side_branch_blocks.values())
        .find(|candidate| {
            candidate.height + 1 == block.height && candidate.hash() == block.parent_hash
        })
        .map(|parent| {
            next_finalized_beacon(
                parent.beacon_round,
                &parent.beacon,
                block.height,
                block.epoch,
            )
        })
        .unwrap_or((
            chain.state.finalized_beacon_round,
            chain.state.finalized_randomness,
        ))
}

fn parent_matches(chain: &Chain, block: &TensorBlock) -> bool {
    if block.height == 0 {
        return block.parent_hash == [0; 32];
    }
    chain.blocks.iter().any(|candidate| {
        candidate.height + 1 == block.height && candidate.hash() == block.parent_hash
    }) || chain.side_branch_blocks.values().any(|candidate| {
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

fn next_finalized_beacon(
    parent_beacon_round: u64,
    parent_beacon: &Hash,
    next_height: u64,
    next_epoch: u64,
) -> (u64, Hash) {
    let next_round = parent_beacon_round.saturating_add(1);
    let next_beacon = hash_bytes(
        b"tensor-vm-finalized-beacon-v2",
        &[
            &parent_beacon_round.to_le_bytes(),
            parent_beacon,
            &next_round.to_le_bytes(),
            &next_height.to_le_bytes(),
            &next_epoch.to_le_bytes(),
        ],
    );
    (next_round, next_beacon)
}
