use super::state::{BlockCheckChallengeRecord, PendingChallengeReward, TensorBlock};
use super::{Chain, settlement};
use crate::challenge::{BlockCheckChallenge, BlockCheckChallengeInput, ChallengeOutcome};
use crate::error::{Result, TvmError};
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
    let mut observed_block = block.clone();
    observed_block.checks_root = merkle_root(&[observed_check_leaf]);
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
        check_leaf_proof: build_proof(&[observed_check_leaf], 0)?,
        recomputed_checks_root: outcome.checks_root,
    });
    let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
    Ok(DeterministicBlockCheckChallenge {
        observed_block,
        challenge,
        challenge_id,
        selected_receipts: outcome.selected_receipt_ids,
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
    if opening.check_leaf != challenge.expected_check_leaf {
        return Err(TvmError::InvalidReceipt("challenge expected leaf mismatch"));
    }

    let pending_amount = chain
        .state
        .pending_proposer_rewards
        .get(&block.height)
        .filter(|reward| reward.proposer == block.proposer && !reward.voided_by_challenge)
        .map_or(0, |reward| reward.amount);
    let challenger_reward = pending_amount.saturating_mul(CHALLENGER_REWARD_BPS) / 10_000;
    let penalty_until_height = chain.state.height.saturating_add(challenge_window_blocks);
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
    for reward in chain.state.pending_receipt_rewards.values_mut() {
        if reward.receipt_id == record.receipt_id {
            reward.voided_by_challenge = true;
        }
    }
    chain
        .state
        .proposer_penalty_until
        .insert(record.proposer, record.penalty_until_height);
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

pub(super) fn challenge_reward_claim_id(challenge_id: &Hash, challenger: &Hash) -> Hash {
    hash_bytes(
        b"tensor-vm-challenge-reward-claim-id-v1",
        &[challenge_id, challenger],
    )
}

pub(super) fn block_check_challenge_id(block_hash: &Hash, receipt_id: &Hash) -> Hash {
    crate::challenge::block_check_challenge_id(block_hash, receipt_id)
}
