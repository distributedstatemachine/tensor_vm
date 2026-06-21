use super::{
    BlockVote, Chain, PendingChallengeReward, PendingReceiptReward, ReceiptRandomnessAnchor,
    ReceiptState, RewardState, TensorBlock, validation,
};
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

impl Chain {
    pub(crate) fn set_position_for_testing(&mut self, height: u64, epoch: u64) {
        self.state.height = height;
        self.state.epoch = epoch;
    }

    pub(crate) fn mark_receipt_settled_for_testing(&mut self, receipt_id: Hash) {
        self.state.settled_receipts.insert(receipt_id);
    }

    pub(crate) fn mark_receipt_data_unavailable_for_testing(&mut self, receipt_id: Hash) {
        self.state.data_unavailable_receipts.insert(receipt_id);
    }

    pub(crate) fn set_miner_settled_tensor_work_for_testing(
        &mut self,
        miner: Address,
        settled_tensor_work: u64,
    ) -> Result<()> {
        self.state
            .miners
            .get_mut(&miner)
            .ok_or(TvmError::UnknownMiner)?
            .settled_tensor_work = settled_tensor_work;
        Ok(())
    }

    pub(crate) fn set_miner_tensor_work_for_testing(
        &mut self,
        miner: Address,
        settled_tensor_work: u64,
        pending_tensor_work: u64,
    ) -> Result<()> {
        let miner = self
            .state
            .miners
            .get_mut(&miner)
            .ok_or(TvmError::UnknownMiner)?;
        miner.settled_tensor_work = settled_tensor_work;
        miner.pending_tensor_work = pending_tensor_work;
        Ok(())
    }

    pub(crate) fn set_validator_stake_for_testing(
        &mut self,
        validator: Address,
        stake: u64,
    ) -> Result<()> {
        self.state
            .validators
            .get_mut(&validator)
            .ok_or(TvmError::UnknownValidator)?
            .stake = stake;
        Ok(())
    }

    pub(crate) fn set_receipt_submission_window_for_testing(&mut self, window: u64) {
        self.params.receipt_submission_window = window;
    }

    pub(crate) fn set_validators_per_job_for_testing(&mut self, validators_per_job: usize) {
        self.params.freivalds.validators_per_job = validators_per_job;
    }

    pub(crate) fn set_replication_factor_for_testing(&mut self, replication_factor: usize) {
        self.params.replication_factor = replication_factor;
    }

    pub(crate) fn insert_receipt_for_testing(&mut self, receipt: ReceiptState) {
        self.state.receipts.insert(receipt.receipt_id(), receipt);
    }

    pub(crate) fn remove_receipt_randomness_anchor_for_testing(&mut self, receipt_id: &Hash) {
        self.state.receipt_randomness_anchors.remove(receipt_id);
    }

    pub(crate) fn anchor_receipt_randomness_for_testing(&mut self, receipt_id: Hash) {
        let beacon_round = self.state.finalized_beacon_round;
        let finalized_randomness = self.state.finalized_randomness;
        let assignment_seed =
            validation::assignment_seed(beacon_round, &finalized_randomness, &receipt_id);
        let validation_seed_commitment = validation::validation_seed_commitment(
            beacon_round,
            &finalized_randomness,
            &receipt_id,
        );
        self.state.receipt_randomness_anchors.insert(
            receipt_id,
            ReceiptRandomnessAnchor {
                receipt_id,
                beacon_round,
                finalized_randomness,
                assignment_seed,
                validation_seed_commitment,
            },
        );
    }

    pub(crate) fn insert_block_votes_for_testing(
        &mut self,
        block_hash: Hash,
        votes: Vec<BlockVote>,
    ) {
        self.state.block_votes.insert(block_hash, votes);
    }

    pub(crate) fn push_block_for_testing(&mut self, block: TensorBlock) {
        self.blocks.push(block);
    }

    pub(crate) fn pop_block_for_testing(&mut self) -> Option<TensorBlock> {
        self.blocks.pop()
    }

    pub(crate) fn insert_attestation_for_testing(&mut self, attestation: ValidatorAttestation) {
        self.state
            .attestations
            .entry(attestation.receipt_id)
            .or_default()
            .push(attestation);
    }

    pub(crate) fn set_model_optimizer_state_root_for_testing(
        &mut self,
        model_id: Hash,
        optimizer_state_root: Option<Hash>,
    ) -> Result<()> {
        self.state
            .model_states
            .get_mut(&model_id)
            .ok_or(TvmError::InvalidReceipt("unknown model"))?
            .optimizer_state_root = optimizer_state_root;
        Ok(())
    }

    pub(crate) fn remove_job_for_testing(&mut self, job_id: &Hash) {
        self.state.jobs.remove(job_id);
    }

    pub(crate) fn remove_receipt_for_testing(&mut self, receipt_id: &Hash) {
        self.state.receipts.remove(receipt_id);
    }

    pub(crate) fn remove_attestations_for_testing(&mut self, receipt_id: &Hash) {
        self.state.attestations.remove(receipt_id);
    }

    pub(crate) fn set_reward_treasury_for_testing(&mut self, treasury: u64) {
        self.state.rewards =
            RewardState::from_parts(self.state.rewards.balances().clone(), treasury);
    }

    pub(crate) fn credit_reward_for_testing(&mut self, address: Address, amount: u64) {
        self.state.rewards.credit(address, amount);
    }

    pub(crate) fn insert_pending_challenge_reward_for_testing(
        &mut self,
        reward: PendingChallengeReward,
    ) {
        self.state
            .pending_challenge_rewards
            .insert(reward.claim_id, reward);
    }

    pub(crate) fn insert_pending_receipt_reward_for_testing(
        &mut self,
        reward: PendingReceiptReward,
    ) {
        self.state
            .pending_receipt_rewards
            .insert(reward.claim_id, reward);
    }
}
