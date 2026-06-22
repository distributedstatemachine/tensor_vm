use super::state::{
    BlockCheckChallengeRecord, ChainState, JobState, PendingChallengeReward, TensorBlock,
    TraceBisectionRecord, TraceBisectionStatus,
};
use super::{Chain, blocks, settlement};
use crate::challenge::{
    BlockCheckChallenge, BlockCheckChallengeInput, ChallengeOutcome, TraceBisectionConfig,
    TraceBisectionExpectation, TraceBisectionOpen, TraceBisectionRound, TraceBisectionState,
    TraceBisectionStep, trace_bisection_challenge_id,
};
use crate::error::{Result, TvmError};
use crate::ir::{IrOpRefereeWitness, TensorGraph};
use crate::merkle::{build_proof, merkle_root, verify_proof};
use crate::types::{Address, Hash, hash_bytes, sign};

const CHALLENGER_REWARD_BPS: u64 = 5_000;
const MAX_OBSERVED_INVALID_BLOCKS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicBlockCheckChallenge {
    pub observed_block: TensorBlock,
    pub challenge: BlockCheckChallenge,
    pub challenge_id: Hash,
    pub selected_receipts: Vec<Hash>,
    pub parent_state: ChainState,
}

pub fn apply_outcome(chain: &mut Chain, outcome: ChallengeOutcome) -> Result<()> {
    match outcome {
        ChallengeOutcome::Rejected { .. } => Ok(()),
        ChallengeOutcome::ProvenInvalid {
            dishonest_party,
            slash_amount,
            ..
        } => {
            if let Some(miner) = chain.state.miners.get_mut(&dishonest_party) {
                miner.stake = miner.stake.saturating_sub(slash_amount);
                miner.reputation -= 10;
                chain.state.rewards.credit_treasury(slash_amount);
                return Ok(());
            }
            if let Some(validator) = chain.state.validators.get_mut(&dishonest_party) {
                validator.stake = validator.stake.saturating_sub(slash_amount);
                validator.reputation -= 10;
                chain.state.rewards.credit_treasury(slash_amount);
                return Ok(());
            }
            Err(TvmError::InvalidReceipt("unknown dishonest party"))
        }
        ChallengeOutcome::BlockCheckProvenInvalid {
            block_hash,
            receipt_id,
            proposer,
            challenger,
            proposer_reward_clawback,
            challenger_reward,
            penalty_until_height,
            reason,
        } => apply_block_check_resolution(
            chain,
            BlockCheckChallengeRecord {
                block_hash,
                block_height: chain
                    .blocks
                    .iter()
                    .find(|block| block.hash() == block_hash)
                    .map_or(chain.state.height, |block| block.height),
                receipt_id,
                proposer,
                challenger,
                expected_check_leaf: [0; 32],
                observed_check_leaf: [0; 32],
                challenged_at_height: chain.state.height,
                proposer_reward_clawback,
                challenger_reward,
                penalty_until_height,
                reason,
            },
        ),
    }
}

pub fn deterministic_bad_block_check_challenge(
    chain: &Chain,
    block: &TensorBlock,
    challenger: Address,
) -> Result<DeterministicBlockCheckChallenge> {
    if !chain.state.validators.contains_key(&challenger) {
        return Err(TvmError::UnknownValidator);
    }
    let parent_state = chain
        .block_parent_state_for_payload(&block.hash())
        .cloned()
        .ok_or(TvmError::InvalidReceipt("block parent state unavailable"))?;
    let outcome = chain.block_apply_outcome(block)?;
    let opening = outcome
        .selected_openings
        .first()
        .ok_or(TvmError::InvalidReceipt(
            "block has no selected receipt to challenge",
        ))?;
    let observed_check_leaf = hash_bytes(
        b"tensor-vm-diagnostic-bad-block-check-leaf-v1",
        &[
            &block.hash(),
            &opening.receipt_id,
            &opening.check_leaf_index.to_le_bytes(),
        ],
    );
    if observed_check_leaf == opening.check_leaf {
        return Err(TvmError::InvalidReceipt("diagnostic check leaf collision"));
    }
    let mut observed_check_leaves = outcome
        .selected_openings
        .iter()
        .map(|opening| opening.check_leaf)
        .collect::<Vec<_>>();
    let check_leaf_index = opening.check_leaf_index as usize;
    if check_leaf_index >= observed_check_leaves.len() {
        return Err(TvmError::InvalidReceipt("diagnostic check leaf missing"));
    }
    observed_check_leaves[check_leaf_index] = observed_check_leaf;
    let mut observed_block = block.clone();
    observed_block.checks_root = merkle_root(&observed_check_leaves);
    let observed_block_hash = observed_block.hash();
    observed_block.proposer_signature = sign(&observed_block.proposer, &observed_block_hash);
    observed_block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&observed_block_hash]);
    let challenge = BlockCheckChallenge::new(BlockCheckChallengeInput {
        challenger,
        block_hash: observed_block.hash(),
        receipt_id: opening.receipt_id,
        expected_check_leaf: opening.check_leaf,
        observed_check_leaf,
        check_leaf_index: opening.check_leaf_index,
        check_leaf_proof: build_proof(&observed_check_leaves, opening.check_leaf_index)?,
        recomputed_checks_root: outcome.checks_root,
    });
    let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
    Ok(DeterministicBlockCheckChallenge {
        observed_block,
        challenge,
        challenge_id,
        selected_receipts: outcome.selected_receipt_ids,
        parent_state,
    })
}

pub fn install_diagnostic_observed_block(
    chain: &mut Chain,
    diagnostic: &DeterministicBlockCheckChallenge,
) -> Result<()> {
    cache_observed_invalid_block(chain, diagnostic.observed_block.clone())
}

pub fn cache_observed_invalid_block(chain: &mut Chain, block: TensorBlock) -> Result<()> {
    let block_hash = block.hash();
    if block_hash == [0; 32] {
        return Err(TvmError::InvalidReceipt("observed block hash is zero"));
    }
    if chain
        .blocks
        .iter()
        .any(|canonical| canonical.hash() == block_hash)
    {
        return Err(TvmError::InvalidReceipt("observed block is canonical"));
    }
    let parent_known = chain.blocks.iter().any(|canonical| {
        canonical.height.saturating_add(1) == block.height && canonical.hash() == block.parent_hash
    }) || block.height == 0 && block.parent_hash == [0; 32];
    if !parent_known {
        return Err(TvmError::InvalidReceipt("observed block parent unknown"));
    }
    if chain.observed_invalid_blocks.len() >= MAX_OBSERVED_INVALID_BLOCKS
        && let Some(oldest) = chain.observed_invalid_blocks.keys().next().copied()
    {
        chain.observed_invalid_blocks.remove(&oldest);
    }
    chain.observed_invalid_blocks.insert(block_hash, block);
    Ok(())
}

pub fn submit_block_check(
    chain: &mut Chain,
    challenge: BlockCheckChallenge,
) -> Result<ChallengeOutcome> {
    if !chain.state.validators.contains_key(&challenge.challenger) {
        return Err(TvmError::UnknownValidator);
    }
    if !challenge.verify_signature() {
        return Err(TvmError::InvalidReceipt("bad challenge signature"));
    }
    if challenge.check_leaf_proof.leaf_index != challenge.check_leaf_index {
        return Err(TvmError::InvalidReceipt("challenge proof index mismatch"));
    }
    if challenge.expected_check_leaf == challenge.observed_check_leaf {
        return Err(TvmError::InvalidReceipt("challenge evidence agrees"));
    }
    let block = challenged_block(chain, &challenge.block_hash)
        .ok_or(TvmError::InvalidReceipt("unknown challenged block"))?;
    let challenge_window_blocks = chain
        .params
        .challenge_window_epochs
        .max(1)
        .saturating_mul(chain.params.epoch_length.max(1));
    if chain.state.height > block.height.saturating_add(challenge_window_blocks) {
        return Err(TvmError::InvalidReceipt("block challenge window expired"));
    }
    let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
    if chain
        .state
        .block_check_challenges
        .contains_key(&challenge_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate block check challenge"));
    }
    if !verify_proof(
        &block.checks_root,
        challenge.observed_check_leaf,
        &challenge.check_leaf_proof,
    ) {
        return Err(TvmError::InvalidReceipt("bad observed check opening"));
    }
    if challenge.recomputed_checks_root == block.checks_root {
        return Err(TvmError::InvalidReceipt(
            "challenge recomputes block checks root",
        ));
    }
    let outcome = chain.block_apply_outcome(&block)?;
    if challenge.recomputed_checks_root != outcome.checks_root {
        return Err(TvmError::InvalidReceipt(
            "noncanonical recomputed checks root",
        ));
    }
    let Some(opening) = outcome
        .selected_openings
        .iter()
        .find(|opening| opening.receipt_id == challenge.receipt_id)
    else {
        return Err(TvmError::InvalidReceipt("receipt not selected by block"));
    };
    if opening.check_leaf_index != challenge.check_leaf_index {
        return Err(TvmError::InvalidReceipt("challenge receipt index mismatch"));
    }
    if opening.check_transcript.leaf() != opening.check_leaf {
        return Err(TvmError::InvalidReceipt(
            "challenge transcript leaf mismatch",
        ));
    }
    if opening.check_leaf != challenge.expected_check_leaf {
        return Err(TvmError::InvalidReceipt("challenge expected leaf mismatch"));
    }

    let canonical_block = chain
        .blocks
        .iter()
        .any(|canonical| canonical.hash() == challenge.block_hash);
    if canonical_block {
        blocks::materialize_finalized_proposer_rewards(
            &mut chain.state,
            &chain.blocks,
            &chain.params,
        );
    }
    let pending_amount = if canonical_block {
        chain
            .state
            .pending_proposer_rewards
            .get(&block.height)
            .filter(|reward| reward.proposer == block.proposer && !reward.voided_by_challenge)
            .map_or(0, |reward| reward.amount)
    } else {
        0
    };
    let challenger_reward = pending_amount.saturating_mul(CHALLENGER_REWARD_BPS) / 10_000;
    let penalty_until_height = if canonical_block {
        chain.state.height.saturating_add(challenge_window_blocks)
    } else {
        chain.state.height
    };
    let record = BlockCheckChallengeRecord {
        block_hash: challenge.block_hash,
        block_height: block.height,
        receipt_id: challenge.receipt_id,
        proposer: block.proposer,
        challenger: challenge.challenger,
        expected_check_leaf: challenge.expected_check_leaf,
        observed_check_leaf: challenge.observed_check_leaf,
        challenged_at_height: chain.state.height,
        proposer_reward_clawback: pending_amount,
        challenger_reward,
        penalty_until_height,
        reason: "block checks root disproven".to_owned(),
    };
    apply_block_check_resolution(chain, record.clone())?;
    Ok(ChallengeOutcome::BlockCheckProvenInvalid {
        block_hash: record.block_hash,
        receipt_id: record.receipt_id,
        proposer: record.proposer,
        challenger: record.challenger,
        proposer_reward_clawback: record.proposer_reward_clawback,
        challenger_reward: record.challenger_reward,
        penalty_until_height: record.penalty_until_height,
        reason: record.reason,
    })
}

pub fn open_trace_bisection(
    chain: &mut Chain,
    config: TraceBisectionConfig,
) -> Result<TraceBisectionRecord> {
    let receipt = chain
        .state
        .receipts
        .get(&config.receipt_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection receipt"))?;
    if receipt.trace_root() != config.trace_root {
        return Err(TvmError::InvalidReceipt(
            "trace bisection receipt trace root mismatch",
        ));
    }
    if receipt.miner() != config.responder {
        return Err(TvmError::InvalidReceipt(
            "trace bisection responder is not receipt miner",
        ));
    }
    if config.challenger == config.responder {
        return Err(TvmError::InvalidReceipt(
            "trace bisection challenger is responder",
        ));
    }
    if config.response_deadline_height <= chain.state.height {
        return Err(TvmError::InvalidReceipt(
            "trace bisection deadline already expired",
        ));
    }
    let state = TraceBisectionState::new(config)?;
    let challenge_id = trace_bisection_challenge_id(
        &state.receipt_id,
        &state.trace_root,
        &state.challenger,
        &state.responder,
    );
    if chain
        .state
        .trace_bisection_challenges
        .contains_key(&challenge_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate trace bisection"));
    }
    let record = TraceBisectionRecord {
        challenge_id,
        state,
        opened_rounds: 0,
        pending_expected_output_roots: Vec::new(),
        pending_expectation_leaf: None,
        last_round_leaf: None,
        last_opening_input_roots: Vec::new(),
        last_opening_output_roots: Vec::new(),
        last_matched_midpoint: None,
        started_at_height: chain.state.height,
        updated_at_height: chain.state.height,
        status: TraceBisectionStatus::Active,
    };
    chain
        .state
        .trace_bisection_challenges
        .insert(challenge_id, record.clone());
    Ok(record)
}

pub fn open_signed_trace_bisection(
    chain: &mut Chain,
    open: TraceBisectionOpen,
) -> Result<TraceBisectionRecord> {
    if !open.verify_signature() {
        return Err(TvmError::InvalidReceipt(
            "trace bisection open signature mismatch",
        ));
    }
    open_trace_bisection(chain, open.config)
}

pub fn submit_trace_bisection_expectation(
    chain: &mut Chain,
    expectation: TraceBisectionExpectation,
) -> Result<TraceBisectionRecord> {
    let challenge_id = trace_bisection_challenge_id(
        &expectation.receipt_id,
        &expectation.trace_root,
        &expectation.challenger,
        &expectation.responder,
    );
    let record = chain
        .state
        .trace_bisection_challenges
        .get_mut(&challenge_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    if record.status != TraceBisectionStatus::Active {
        return Err(TvmError::InvalidReceipt("trace bisection is closed"));
    }
    if record.state.timed_out(chain.state.height) {
        return Err(TvmError::InvalidReceipt(
            "trace bisection expectation timed out",
        ));
    }
    expectation.verify_for_state(&record.state)?;
    record.pending_expected_output_roots = expectation.expected_output_roots.clone();
    record.pending_expectation_leaf = Some(expectation.expectation_leaf());
    record.updated_at_height = chain.state.height;
    Ok(record.clone())
}

pub fn submit_trace_bisection_round(
    chain: &mut Chain,
    round: TraceBisectionRound,
) -> Result<TraceBisectionRecord> {
    let challenge_id = trace_bisection_challenge_id(
        &round.receipt_id,
        &round.trace_root,
        &round.challenger,
        &round.responder,
    );
    let record = chain
        .state
        .trace_bisection_challenges
        .get_mut(&challenge_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    if record.status != TraceBisectionStatus::Active {
        return Err(TvmError::InvalidReceipt("trace bisection is closed"));
    }
    if record.state.timed_out(chain.state.height) {
        return Err(TvmError::InvalidReceipt("trace bisection round timed out"));
    }
    round.verify_for_state(&record.state)?;
    if record.pending_expectation_leaf.is_none() {
        return Err(TvmError::InvalidReceipt(
            "trace bisection expectation missing",
        ));
    }
    if record.pending_expected_output_roots != round.expected_output_roots {
        return Err(TvmError::InvalidReceipt(
            "trace bisection expectation mismatch",
        ));
    }
    let step = record.state.apply_round(&round)?;
    let round_leaf = round.transcript_leaf();
    match step {
        TraceBisectionStep::Narrowed {
            next_state,
            matched_midpoint,
        } => {
            record.state = next_state;
            record.opened_rounds = record.opened_rounds.saturating_add(1);
            record.last_round_leaf = Some(round_leaf);
            record.last_opening_input_roots = round.opening.op_trace.input_roots.clone();
            record.last_opening_output_roots = round.opening.op_trace.output_roots.clone();
            record.last_matched_midpoint = Some(matched_midpoint);
            record.pending_expected_output_roots.clear();
            record.pending_expectation_leaf = None;
            record.updated_at_height = chain.state.height;
        }
        TraceBisectionStep::Isolated { op_index } => {
            record.opened_rounds = record.opened_rounds.saturating_add(1);
            record.last_round_leaf = Some(round_leaf);
            record.last_opening_input_roots = round.opening.op_trace.input_roots.clone();
            record.last_opening_output_roots = round.opening.op_trace.output_roots.clone();
            record.last_matched_midpoint =
                Some(round.expected_output_roots == round.opening.op_trace.output_roots);
            record.pending_expected_output_roots.clear();
            record.pending_expectation_leaf = None;
            record.updated_at_height = chain.state.height;
            record.status = TraceBisectionStatus::Isolated { op_index };
        }
        TraceBisectionStep::TimedOut { .. } => unreachable!("round application cannot time out"),
    }
    Ok(record.clone())
}

pub fn referee_trace_bisection(
    chain: &mut Chain,
    challenge_id: Hash,
    witness: IrOpRefereeWitness,
) -> Result<TraceBisectionRecord> {
    let record = chain
        .state
        .trace_bisection_challenges
        .get(&challenge_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    let TraceBisectionStatus::Isolated { op_index } = record.status else {
        return Err(TvmError::InvalidReceipt("trace bisection is not isolated"));
    };
    if witness.op_index != op_index {
        return Err(TvmError::InvalidReceipt(
            "trace bisection referee op mismatch",
        ));
    }
    if record.last_opening_input_roots.is_empty() || record.last_opening_output_roots.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "trace bisection missing isolated opening roots",
        ));
    }
    let graph = trace_bisection_receipt_graph(chain, record.state.receipt_id)?;
    let verdict = graph.referee_op(&witness)?;
    if verdict.input_roots != record.last_opening_input_roots {
        return Err(TvmError::InvalidReceipt(
            "trace bisection referee input root mismatch",
        ));
    }
    let dishonest_party = if verdict.canonical_output_roots == record.last_opening_output_roots {
        record.state.challenger
    } else {
        record.state.responder
    };
    settle_trace_bisection_loss(chain, challenge_id, &record, dishonest_party)?;
    let updated = chain
        .state
        .trace_bisection_challenges
        .get_mut(&challenge_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    if !matches!(updated.status, TraceBisectionStatus::Isolated { .. }) {
        return Err(TvmError::InvalidReceipt("trace bisection is not isolated"));
    }
    updated.status = TraceBisectionStatus::Refereed {
        op_index,
        dishonest_party,
        canonical_output_roots: verdict.canonical_output_roots,
        disputed_output_roots: updated.last_opening_output_roots.clone(),
    };
    updated.updated_at_height = chain.state.height;
    Ok(updated.clone())
}

fn slash_trace_bisection_loser(
    chain: &mut Chain,
    dishonest_party: Address,
    slash_amount: u64,
) -> Result<()> {
    if let Some(miner) = chain.state.miners.get_mut(&dishonest_party) {
        miner.stake = miner.stake.saturating_sub(slash_amount);
        miner.reputation -= 10;
        return Ok(());
    }
    if let Some(validator) = chain.state.validators.get_mut(&dishonest_party) {
        validator.stake = validator.stake.saturating_sub(slash_amount);
        validator.reputation -= 10;
        return Ok(());
    }
    Err(TvmError::InvalidReceipt(
        "trace bisection loser is not slashable",
    ))
}

fn settle_trace_bisection_loss(
    chain: &mut Chain,
    challenge_id: Hash,
    record: &TraceBisectionRecord,
    dishonest_party: Address,
) -> Result<()> {
    let loser_bond = if dishonest_party == record.state.challenger {
        record.state.challenger_bond
    } else {
        record.state.responder_bond
    };
    let winner = if dishonest_party == record.state.challenger {
        record.state.responder
    } else {
        record.state.challenger
    };
    let winner_is_challenger = winner == record.state.challenger;
    let challenger_reward = if winner_is_challenger {
        loser_bond.saturating_mul(CHALLENGER_REWARD_BPS) / 10_000
    } else {
        0
    };
    if loser_bond == 0 {
        return Ok(());
    }
    slash_trace_bisection_loser(chain, dishonest_party, loser_bond)?;
    if winner_is_challenger {
        chain
            .state
            .challenged_receipts
            .insert(record.state.receipt_id);
        chain
            .state
            .settled_receipts
            .remove(&record.state.receipt_id);
        settlement::void_pending_miner_tensor_work(&mut chain.state, &record.state.receipt_id);
        let receipt_reward_hold_until_height = chain
            .state
            .height
            .saturating_add(chain.params.reward_maturity_delay_blocks());
        for reward in chain.state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == record.state.receipt_id {
                reward.delay_until(receipt_reward_hold_until_height);
                reward.voided_by_challenge = true;
            }
        }
    }
    if challenger_reward > 0 {
        let claimable_at_height = chain
            .state
            .height
            .saturating_add(chain.params.reward_maturity_delay_blocks());
        enqueue_pending_trace_bisection_challenge_reward(
            chain,
            challenge_id,
            record.state.receipt_id,
            winner,
            challenger_reward,
            claimable_at_height,
        );
    }
    let treasury_reward = loser_bond.saturating_sub(challenger_reward);
    if treasury_reward > 0 {
        chain.state.rewards.credit_treasury(treasury_reward);
    }
    Ok(())
}

fn trace_bisection_receipt_graph(chain: &Chain, receipt_id: Hash) -> Result<TensorGraph> {
    let receipt = chain
        .state
        .receipts
        .get(&receipt_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection receipt"))?;
    let job = chain
        .state
        .jobs
        .get(&receipt.job_id())
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection job"))?;
    match job {
        JobState::TensorOp(job) => Ok(job.tensor_ir_graph()),
        JobState::LinearTrainingStep(job) => Ok(job.tensor_ir_graph()),
        JobState::GraphExecution(job) => {
            let body = chain
                .state
                .program_body(&job.graph_id)
                .ok_or(TvmError::InvalidReceipt(
                    "missing trace bisection program body",
                ))?;
            TensorGraph::from_canonical_json_bytes(body)
        }
    }
}

pub fn record_trace_bisection_timeout(
    chain: &mut Chain,
    challenge_id: Hash,
) -> Result<TraceBisectionRecord> {
    let record = chain
        .state
        .trace_bisection_challenges
        .get(&challenge_id)
        .cloned()
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    let forfeiting_party = match record.status {
        TraceBisectionStatus::Active => {
            let Some(TraceBisectionStep::TimedOut {
                forfeiting_party, ..
            }) = record.state.timeout_step(chain.state.height)
            else {
                return Err(TvmError::InvalidReceipt("trace bisection deadline pending"));
            };
            forfeiting_party
        }
        TraceBisectionStatus::Isolated { .. } => {
            if !record.state.timed_out(chain.state.height) {
                return Err(TvmError::InvalidReceipt("trace bisection deadline pending"));
            }
            record.state.challenger
        }
        TraceBisectionStatus::Refereed { .. } | TraceBisectionStatus::TimedOut { .. } => {
            return Err(TvmError::InvalidReceipt("trace bisection is closed"));
        }
    };
    settle_trace_bisection_loss(chain, challenge_id, &record, forfeiting_party)?;
    let updated = chain
        .state
        .trace_bisection_challenges
        .get_mut(&challenge_id)
        .ok_or(TvmError::InvalidReceipt("unknown trace bisection"))?;
    if !matches!(
        updated.status,
        TraceBisectionStatus::Active | TraceBisectionStatus::Isolated { .. }
    ) {
        return Err(TvmError::InvalidReceipt("trace bisection is closed"));
    }
    updated.status = TraceBisectionStatus::TimedOut { forfeiting_party };
    updated.updated_at_height = chain.state.height;
    Ok(updated.clone())
}

fn challenged_block(chain: &Chain, block_hash: &Hash) -> Option<TensorBlock> {
    chain
        .blocks
        .iter()
        .find(|block| block.hash() == *block_hash)
        .cloned()
        .or_else(|| chain.observed_invalid_blocks.get(block_hash).cloned())
}

fn apply_block_check_resolution(
    chain: &mut Chain,
    record: BlockCheckChallengeRecord,
) -> Result<()> {
    let challenge_id = block_check_challenge_id(&record.block_hash, &record.receipt_id);
    if chain
        .state
        .block_check_challenges
        .contains_key(&challenge_id)
    {
        return Err(TvmError::InvalidReceipt("duplicate block check challenge"));
    }
    let canonical_block = chain
        .blocks
        .iter()
        .any(|block| block.hash() == record.block_hash);
    if canonical_block {
        if let Some(reward) = chain
            .state
            .pending_proposer_rewards
            .get_mut(&record.block_height)
            && reward.proposer == record.proposer
        {
            reward.voided_by_challenge = true;
            let treasury_reward = record
                .proposer_reward_clawback
                .saturating_sub(record.challenger_reward);
            if record.challenger_reward > 0 {
                let claimable_at_height = record
                    .challenged_at_height
                    .saturating_add(chain.params.reward_maturity_delay_blocks());
                enqueue_pending_challenge_reward(chain, challenge_id, &record, claimable_at_height);
            }
            if treasury_reward > 0 {
                chain.state.rewards.credit_treasury(treasury_reward);
            }
        }
        chain.state.challenged_receipts.insert(record.receipt_id);
        chain.state.settled_receipts.remove(&record.receipt_id);
        settlement::void_pending_miner_tensor_work(&mut chain.state, &record.receipt_id);
        let receipt_reward_hold_until_height = record
            .challenged_at_height
            .saturating_add(chain.params.reward_maturity_delay_blocks());
        for reward in chain.state.pending_receipt_rewards.values_mut() {
            if reward.receipt_id == record.receipt_id {
                reward.delay_until(receipt_reward_hold_until_height);
                reward.voided_by_challenge = true;
            }
        }
        chain
            .state
            .proposer_penalty_until
            .insert(record.proposer, record.penalty_until_height);
    }
    chain
        .state
        .block_check_challenges
        .insert(challenge_id, record);
    Ok(())
}

fn enqueue_pending_challenge_reward(
    chain: &mut Chain,
    challenge_id: Hash,
    record: &BlockCheckChallengeRecord,
    claimable_at_height: u64,
) {
    if record.challenger_reward == 0 {
        return;
    }
    let claim_id = challenge_reward_claim_id(&challenge_id, &record.challenger);
    chain
        .state
        .pending_challenge_rewards
        .entry(claim_id)
        .or_insert(PendingChallengeReward {
            claim_id,
            challenge_id,
            block_hash: record.block_hash,
            receipt_id: record.receipt_id,
            challenger: record.challenger,
            amount: record.challenger_reward,
            claimable_at_height,
            voided_by_challenge: false,
        });
}

fn enqueue_pending_trace_bisection_challenge_reward(
    chain: &mut Chain,
    challenge_id: Hash,
    receipt_id: Hash,
    challenger: Address,
    amount: u64,
    claimable_at_height: u64,
) {
    if amount == 0 {
        return;
    }
    let claim_id = challenge_reward_claim_id(&challenge_id, &challenger);
    chain
        .state
        .pending_challenge_rewards
        .entry(claim_id)
        .or_insert(PendingChallengeReward {
            claim_id,
            challenge_id,
            block_hash: [0; 32],
            receipt_id,
            challenger,
            amount,
            claimable_at_height,
            voided_by_challenge: false,
        });
}

pub(super) fn challenge_reward_claim_id(challenge_id: &Hash, challenger: &Hash) -> Hash {
    hash_bytes(
        b"tensor-vm-challenge-reward-claim-id-v1",
        &[challenge_id, challenger],
    )
}

pub(super) fn block_check_challenge_id(block_hash: &Hash, receipt_id: &Hash) -> Hash {
    crate::challenge::block_check_challenge_id(block_hash, receipt_id)
}
