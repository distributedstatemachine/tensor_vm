use super::{KeyValueReportWriter, status::hex_hash_list};
use std::collections::BTreeSet;

use crate::{Chain, NodeStore, PrimitiveType, hash::hex};

pub fn service_block_status(data_dir: &str, height: u64) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let Some(block) = chain.blocks().iter().find(|block| block.height == height) else {
        return Err(format!(
            "block height {height} is not in node store {data_dir}"
        ));
    };
    let block_hash = block.hash();
    let selected_receipt_ids = chain.selected_receipts_for_block(block);
    let blockspace_caps = chain.blockspace_caps();
    let selected_receipt_twu = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.tensor_work_units())
        .sum::<u64>();
    let selected_receipt_bytes = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.estimated_block_bytes())
        .sum::<u64>();
    let block_apply_outcome = chain.block_apply_outcome(block).ok();
    let block_valid = chain.validate_block(block).is_ok();
    let expected_difficulty_target = chain.expected_difficulty_target(block.height);
    let fallback_valid = !block.production_kind.requires_pow()
        && selected_receipt_ids.is_empty()
        && block.nonce == 0
        && block.difficulty_target == expected_difficulty_target;
    let proposer_registered = chain.state().validators().contains_key(&block.proposer);
    let pow_hash = block.pow_hash();
    let pow_header_hash = block.pow_header_hash();
    let block_votes = chain
        .state()
        .block_votes()
        .get(&block_hash)
        .cloned()
        .unwrap_or_default();
    let total_validator_stake = chain
        .state()
        .validators()
        .values()
        .map(|validator| validator.stake)
        .sum::<u64>();
    let finality_threshold_stake = finality_threshold_stake(&chain, total_validator_stake);
    let mut seen_vote_validators = BTreeSet::new();
    let mut valid_vote_validators = Vec::new();
    let mut valid_vote_stake = 0_u64;
    for vote in &block_votes {
        let Some(validator) = chain.state().validators().get(&vote.validator) else {
            continue;
        };
        if validator.stake != vote.stake || !vote.verify_signature() {
            continue;
        }
        if seen_vote_validators.insert(vote.validator) {
            valid_vote_validators.push(vote.validator);
            valid_vote_stake = valid_vote_stake.saturating_add(vote.stake);
        }
    }
    let mut receipt_ids = Vec::new();
    let mut tensor_op_receipt_ids = Vec::new();
    let mut linear_training_receipt_ids = Vec::new();
    let mut settled_receipt_ids = Vec::new();
    for receipt in chain
        .state()
        .receipts()
        .values()
        .filter(|receipt| receipt.submitted_at_block() == height)
    {
        let receipt_id = receipt.receipt_id();
        receipt_ids.push(receipt_id);
        if chain.state().settled_receipts().contains(&receipt_id) {
            settled_receipt_ids.push(receipt_id);
        }
        match receipt.primitive_type() {
            PrimitiveType::TensorOp => tensor_op_receipt_ids.push(receipt_id),
            PrimitiveType::LinearTrainingStep => linear_training_receipt_ids.push(receipt_id),
        }
    }
    let finalized = chain.is_block_finalized(&block_hash);
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_block");
    report.field("data_dir", data_dir);
    report.field("height", height);
    report.field("block_hash", hex(&block_hash));
    report.field("block_validation", block.production_kind.label());
    report.field("block_kind", block.production_kind.label());
    report.field("pow_skip_fallback", !block.production_kind.requires_pow());
    report.field("fallback_valid", fallback_valid);
    report.field("parent_hash", hex(&block.parent_hash));
    if let Some(outcome) = &block_apply_outcome {
        report.field(
            "parent_snapshot_root",
            hex(&outcome.parent_snapshot.state_root),
        );
        report.field("parent_beacon_round", outcome.parent_snapshot.beacon_round);
        report.field("parent_beacon", hex(&outcome.parent_snapshot.beacon));
        report.field(
            "parent_settled_receipt_pool_root",
            hex(&outcome.parent_snapshot.settled_receipt_pool_root),
        );
        report.field(
            "parent_included_receipt_root",
            hex(&outcome.parent_snapshot.included_receipt_root),
        );
        report.field(
            "parent_data_unavailable_receipt_root",
            hex(&outcome.parent_snapshot.data_unavailable_receipt_root),
        );
        report.field(
            "parent_data_unavailability_slash_root",
            hex(&outcome.parent_snapshot.data_unavailability_slash_root),
        );
        report.field("child_state_root", hex(&outcome.child_state_root));
        report.field("child_reward_root", hex(&outcome.child_reward_root));
        report.field("child_height", outcome.child_height);
        report.field("child_epoch", outcome.child_epoch);
        report.field("child_beacon_round", outcome.child_beacon_round);
        report.field("child_beacon", hex(&outcome.child_beacon));
        report.field(
            "block_uses_parent_finalized_beacon",
            block.beacon_round == outcome.parent_snapshot.beacon_round
                && block.beacon == outcome.parent_snapshot.beacon,
        );
    }
    report.field("proposer", hex(&block.proposer));
    report.field("proposer_role", "validator");
    report.field("proposer_registered", proposer_registered);
    report.field("tensorwork_proposer_selection", false);
    report.field("state_root", hex(&block.state_root));
    report.field("epoch", block.epoch);
    report.field("beacon_round", block.beacon_round);
    report.field("beacon", hex(&block.beacon));
    report.field("latest_height", chain.state().height());
    report.field("finalized", finalized);
    report.field(
        "settled_receipt_set_root",
        hex(&block.settled_receipt_set_root),
    );
    report.field("selected_receipt_ids", hex_hash_list(&selected_receipt_ids));
    report.field("selected_receipt_count", selected_receipt_ids.len());
    if let Some(outcome) = &block_apply_outcome {
        report.field(
            "selected_receipt_root_recomputed",
            block.settled_receipt_set_root == outcome.selected_receipt_root,
        );
        report.field(
            "selected_receipt_opening_count",
            outcome.selected_openings.len(),
        );
        report.field(
            "selected_receipt_leaf_ids",
            hex_hash_list(
                &outcome
                    .selected_openings
                    .iter()
                    .map(|opening| opening.receipt_id)
                    .collect::<Vec<_>>(),
            ),
        );
        report.field(
            "selected_receipt_leaf_roots",
            hex_hash_list(
                &outcome
                    .selected_openings
                    .iter()
                    .map(|opening| opening.receipt_leaf)
                    .collect::<Vec<_>>(),
            ),
        );
        report.field(
            "checks_opening_count",
            outcome
                .selected_openings
                .iter()
                .filter(|opening| opening.check_leaf_proof.is_some())
                .count(),
        );
        report.field(
            "checks_leaf_roots",
            hex_hash_list(
                &outcome
                    .selected_openings
                    .iter()
                    .map(|opening| opening.check_leaf)
                    .collect::<Vec<_>>(),
            ),
        );
        report.field(
            "child_state_root_recomputed",
            block.state_root == outcome.child_state_root,
        );
        report.field(
            "child_reward_root_recomputed",
            block.reward_root == outcome.child_reward_root,
        );
    }
    report.field("selected_receipt_twu", selected_receipt_twu);
    report.field("selected_receipt_bytes", selected_receipt_bytes);
    report.field("block_twu_cap", blockspace_caps.max_tensor_work_units);
    report.field("block_byte_cap", blockspace_caps.max_bytes);
    report.field("block_receipt_cap", blockspace_caps.max_receipts);
    report.field("checks_root", hex(&block.checks_root));
    report.field("check_leaf_count", selected_receipt_ids.len());
    report.field("checks_root_recomputed", block_valid);
    if let Some(outcome) = &block_apply_outcome {
        report.field(
            "checks_root_openable",
            block.checks_root == outcome.checks_root,
        );
    }
    report.field("difficulty_target", hex(&block.difficulty_target));
    report.field(
        "expected_difficulty_target",
        hex(&expected_difficulty_target),
    );
    report.field(
        "difficulty_retarget_epoch_length",
        chain.params().difficulty_retarget_epoch_length,
    );
    report.field(
        "difficulty_target_block_time_seconds",
        chain.params().difficulty_target_block_time_seconds,
    );
    report.field(
        "difficulty_retarget_max_ratio",
        chain.params().difficulty_retarget_max_ratio,
    );
    report.field("pow_timeout_blocks", chain.params().pow_timeout_blocks);
    report.field("nonce", block.nonce);
    report.field("pow_header_hash", hex(&pow_header_hash));
    report.field("pow_hash", hex(&pow_hash));
    report.field("pow_valid", block.pow_valid());
    report.field("pow_required", block.production_kind.requires_pow());
    report.field("canonical_blockspace_valid", block_valid);
    report.field("block_vote_count", valid_vote_validators.len());
    report.field(
        "block_vote_validators",
        hex_hash_list(&valid_vote_validators),
    );
    report.field("block_vote_stake", valid_vote_stake);
    report.field("finality_threshold_stake", finality_threshold_stake);
    report.field("finality_validated_block", finalized && block_valid);
    report.field("receipt_count", receipt_ids.len());
    report.field("receipt_ids", hex_hash_list(&receipt_ids));
    report.field("tensor_op_receipt_count", tensor_op_receipt_ids.len());
    report.field(
        "tensor_op_receipt_ids",
        hex_hash_list(&tensor_op_receipt_ids),
    );
    report.field(
        "linear_training_receipt_count",
        linear_training_receipt_ids.len(),
    );
    report.field(
        "linear_training_receipt_ids",
        hex_hash_list(&linear_training_receipt_ids),
    );
    report.field("settled_receipt_count", settled_receipt_ids.len());
    report.field("settled_receipt_ids", hex_hash_list(&settled_receipt_ids));
    report.field("status_source", "node_store");
    Ok(report.finish())
}

fn finality_threshold_stake(chain: &Chain, total_validator_stake: u64) -> u64 {
    let numerator = chain.params().finality_stake_numerator;
    let denominator = chain.params().finality_stake_denominator.max(1);
    total_validator_stake
        .saturating_mul(numerator)
        .saturating_add(denominator.saturating_sub(1))
        / denominator
}
