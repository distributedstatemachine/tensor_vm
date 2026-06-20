use crate::challenge::ChallengeOutcome;
use crate::error::Result;
#[cfg(test)]
use crate::error::TvmError;
#[cfg(test)]
use crate::jobs::PrimitiveType;
use crate::jobs::{LinearTrainingStepReceipt, TensorOpReceipt};
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

mod accounts;
mod blocks;
mod challenges;
mod commands;
mod engine;
mod genesis;
mod models;
mod operators;
mod proposer;
mod receipts;
mod roots;
mod settlement;
mod state;
#[cfg(test)]
mod test_helpers;
mod transactions;
mod validation;

pub use engine::{BlockAdmission, BlockInvalidReason, ChainCommand, ChainEngine, ChainEvent};
#[cfg(test)]
use settlement::{has_conflicting_linear_receipt, receipts_agree};
pub use state::{
    AccountState, BlockApplyOutcome, BlockParentSnapshot, BlockProductionKind, BlockVote,
    BlockspaceCaps, BlockspaceSelection, Chain, ChainParams, ChainState, HardwareClass, JobState,
    MinerState, ModelState, ReceiptState, RewardAllocation, RewardState, SelectedReceiptOpening,
    TensorBlock, Transaction, ValidatorState,
};
pub(crate) use state::{ChainParts, ChainStateParts};

impl Chain {
    pub fn new(finalized_randomness: Hash) -> Self {
        Self::with_params(ChainParams::default(), finalized_randomness)
    }

    pub fn with_params(params: ChainParams, finalized_randomness: Hash) -> Self {
        genesis::with_params(params, finalized_randomness)
    }

    pub fn params(&self) -> &ChainParams {
        &self.params
    }

    pub fn state(&self) -> &ChainState {
        &self.state
    }

    pub fn blocks(&self) -> &[TensorBlock] {
        &self.blocks
    }

    pub fn register_miner(&mut self, address: Address, stake: u64) -> Result<()> {
        operators::register_miner(self, address, stake)
    }

    pub fn register_miner_with_operator(
        &mut self,
        address: Address,
        stake: u64,
        operator_id: Hash,
    ) -> Result<()> {
        operators::register_miner_with_operator(self, address, stake, operator_id)
    }

    pub fn register_miner_with_profile(
        &mut self,
        address: Address,
        stake: u64,
        hardware_class: HardwareClass,
        gpu_utilization_bps: u64,
    ) -> Result<()> {
        operators::register_miner_with_profile(
            self,
            address,
            stake,
            hardware_class,
            gpu_utilization_bps,
        )
    }

    pub fn register_miner_with_profile_and_operator(
        &mut self,
        address: Address,
        stake: u64,
        operator_id: Hash,
        hardware_class: HardwareClass,
        gpu_utilization_bps: u64,
    ) -> Result<()> {
        operators::register_miner_with_profile_and_operator(
            self,
            address,
            stake,
            operator_id,
            hardware_class,
            gpu_utilization_bps,
        )
    }

    pub fn register_validator(&mut self, address: Address, stake: u64) -> Result<()> {
        operators::register_validator(self, address, stake)
    }

    pub fn credit_account(&mut self, address: Address, amount: u64) {
        accounts::credit(self, address, amount);
    }

    pub fn transfer(&mut self, from: Address, to: Address, amount: u64) -> Result<()> {
        accounts::transfer(self, from, to, amount)
    }

    pub fn submit_job(&mut self, job: JobState) {
        receipts::submit_job(self, job);
    }

    pub fn job(&self, job_id: &Hash) -> Option<&JobState> {
        receipts::job(self, job_id)
    }

    pub fn submit_tensor_op_receipt(&mut self, receipt: TensorOpReceipt) -> Result<()> {
        receipts::submit_tensor_op(self, receipt)
    }

    pub fn submit_linear_receipt(&mut self, receipt: LinearTrainingStepReceipt) -> Result<()> {
        receipts::submit_linear_training_step(self, receipt)
    }

    pub fn apply_transaction(
        &mut self,
        from: Option<Address>,
        tx: Transaction,
    ) -> Result<Vec<ChainEvent>> {
        transactions::apply(self, from, tx)
    }

    pub fn submit_attestation(&mut self, attestation: ValidatorAttestation) -> Result<()> {
        validation::submit_attestation(self, attestation)
    }

    pub fn has_attestation_quorum(&self, receipt_id: &Hash) -> bool {
        validation::has_attestation_quorum(self, receipt_id)
    }

    pub fn redundant_agreement_count(&self, receipt_id: &Hash) -> usize {
        settlement::redundant_agreement_count(self, receipt_id)
    }

    pub fn has_redundant_agreement(&self, receipt_id: &Hash) -> bool {
        settlement::has_redundant_agreement(self, receipt_id)
    }

    pub fn submit_block_vote(&mut self, vote: BlockVote) -> Result<()> {
        validation::submit_block_vote(self, vote)
    }

    pub fn has_block_finality(&self, block_hash: &Hash) -> bool {
        validation::has_block_finality(self, block_hash)
    }

    pub fn is_block_finalized(&self, block_hash: &Hash) -> bool {
        self.state.finalized_blocks.contains(block_hash)
    }

    pub fn register_model(
        &mut self,
        model_id: Hash,
        architecture_hash: Hash,
        weight_root: Hash,
        config_hash: Hash,
    ) -> Result<()> {
        models::register(self, model_id, architecture_hash, weight_root, config_hash)
    }

    pub fn apply_model_transition(
        &mut self,
        model_id: &Hash,
        step: u64,
        weight_root_before: &Hash,
        weight_root_after: Hash,
    ) -> Result<()> {
        models::apply_transition(self, model_id, step, weight_root_before, weight_root_after)
    }

    pub fn apply_challenge_outcome(&mut self, outcome: ChallengeOutcome) -> Result<()> {
        self.apply_command(ChainCommand::ApplyChallengeOutcome(outcome))
            .map(|_| ())
    }

    pub fn validator_assignment_seed(&self, receipt_id: &Hash) -> Hash {
        validation::assignment_seed(
            self.state.finalized_beacon_round,
            &self.state.finalized_randomness,
            receipt_id,
        )
    }

    pub fn miner_assignment_seed(&self, job_id: &Hash) -> Hash {
        validation::miner_assignment_seed(
            self.state.finalized_beacon_round,
            &self.state.finalized_randomness,
            job_id,
        )
    }

    pub fn validation_seed(&self, receipt_id: &Hash, validator: &Address) -> Hash {
        let job_id = self
            .state
            .receipts
            .get(receipt_id)
            .map(ReceiptState::job_id)
            .unwrap_or([0; 32]);
        validation::seed(
            self.state.finalized_beacon_round,
            &self.state.finalized_randomness,
            receipt_id,
            &job_id,
            validator,
            0,
        )
    }

    pub fn settle_epoch(&mut self, miner_reward_pool: u64, validator_reward_pool: u64) {
        settlement::settle_epoch(self, miner_reward_pool, validator_reward_pool);
    }

    pub fn settle_epoch_rewards(&mut self, allocation: RewardAllocation, proposer: Address) {
        self.settle_epoch(
            allocation.miner_reward_pool,
            allocation.validator_reward_pool,
        );
        if allocation.proposer_reward > 0 {
            self.state
                .rewards
                .credit(proposer, allocation.proposer_reward);
        }
        self.state
            .rewards
            .credit_treasury(allocation.treasury_reward);
    }

    pub fn proposer_for_next_epoch(&self, beacon: &Hash) -> Option<Address> {
        proposer::for_next_epoch(&self.state, beacon)
    }

    pub fn produce_block(&mut self, proposer: Address, timestamp: u64) -> Result<TensorBlock> {
        blocks::produce(self, proposer, timestamp)
    }

    pub fn produce_block_with_rewards(
        &mut self,
        proposer: Address,
        timestamp: u64,
        fixed_block_reward: u64,
        fee_share: u64,
    ) -> Result<TensorBlock> {
        blocks::produce_with_rewards(self, proposer, timestamp, fixed_block_reward, fee_share)
    }

    pub fn prepare_block_parent_state(&mut self) -> Result<()> {
        blocks::prepare_parent_state(self)
    }

    pub fn admit_block(&mut self, block: TensorBlock) -> Result<BlockAdmission> {
        blocks::admit(self, block)
    }

    pub fn blockspace_caps(&self) -> BlockspaceCaps {
        blocks::blockspace_caps()
    }

    pub fn canonical_blockspace(
        &self,
        parent_hash: &Hash,
        beacon_round: u64,
        beacon: &Hash,
    ) -> BlockspaceSelection {
        blocks::canonical_blockspace(
            &self.state,
            parent_hash,
            beacon_round,
            beacon,
            self.blockspace_caps(),
        )
    }

    pub fn selected_receipts_for_block(&self, block: &TensorBlock) -> Vec<Hash> {
        blocks::selected_receipts(self, block)
    }

    pub fn block_apply_outcome(&self, block: &TensorBlock) -> Result<BlockApplyOutcome> {
        blocks::apply_outcome(self, block)
    }

    pub fn validate_block(&self, block: &TensorBlock) -> Result<()> {
        blocks::validate(self, block, true)
    }

    pub fn expected_difficulty_target(&self, height: u64) -> Hash {
        blocks::expected_difficulty_target(self, height)
    }

    pub fn state_root(&self) -> Hash {
        roots::state_root(&self.state)
    }
}

#[cfg(test)]
mod tests;
