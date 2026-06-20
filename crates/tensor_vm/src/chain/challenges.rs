use super::Chain;
use super::state::BlockCheckChallengeRecord;
use crate::challenge::{BlockCheckChallenge, ChallengeOutcome};
use crate::error::{Result, TvmError};
use crate::merkle::verify_proof;
use crate::types::{Hash, hash_bytes};

const CHALLENGER_REWARD_BPS: u64 = 5_000;

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
    let block = chain
        .blocks
        .iter()
        .find(|block| block.hash() == challenge.block_hash)
        .cloned()
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
            chain
                .state
                .rewards
                .credit(record.challenger, record.challenger_reward);
        }
        if treasury_reward > 0 {
            chain.state.rewards.credit_treasury(treasury_reward);
        }
    }
    chain.state.challenged_receipts.insert(record.receipt_id);
    chain.state.settled_receipts.remove(&record.receipt_id);
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

pub(super) fn block_check_challenge_id(block_hash: &Hash, receipt_id: &Hash) -> Hash {
    hash_bytes(
        b"tensor-vm-block-check-challenge-id-v1",
        &[block_hash, receipt_id],
    )
}
