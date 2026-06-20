use super::state::{
    BlockVote, ChainParams, ChainState, JobState, ReceiptState, TensorBlock, ValidatorAuditAppeal,
    ValidatorAuditAppealResolution, ValidatorAuditReport,
};
use crate::challenge::{BlockCheckChallenge, ChallengeOutcome};
use crate::error::Result;
use crate::ir::GraphId;
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockInvalidReason {
    ConflictingHeight,
    InvalidPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockAdmission {
    Applied {
        height: u64,
        hash: Hash,
    },
    Duplicate {
        height: u64,
        hash: Hash,
    },
    PendingParent {
        height: u64,
        parent_hash: Hash,
    },
    Invalid {
        height: u64,
        hash: Hash,
        reason: BlockInvalidReason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainCommand {
    RegisterMiner {
        address: Address,
        stake: u64,
    },
    RegisterValidator {
        address: Address,
        stake: u64,
    },
    Transfer {
        from: Address,
        to: Address,
        amount: u64,
    },
    CreditReward {
        address: Address,
        amount: u64,
    },
    ClaimReward(Address),
    RegisterProgramBody {
        graph_id: GraphId,
        bytes: Vec<u8>,
    },
    SubmitJob(JobState),
    SubmitReceipt(ReceiptState),
    SubmitAttestation(ValidatorAttestation),
    SubmitValidatorAuditReport(ValidatorAuditReport),
    SubmitValidatorAuditAppeal(ValidatorAuditAppeal),
    ResolveValidatorAuditAppeal {
        audit_id: Hash,
        resolution: ValidatorAuditAppealResolution,
    },
    SubmitBlock(TensorBlock),
    SubmitBlockVote(BlockVote),
    SettleEpoch {
        miner_reward_pool: u64,
        validator_reward_pool: u64,
    },
    ProduceBlock {
        proposer: Address,
        timestamp: u64,
    },
    ProduceRewardedBlock {
        proposer: Address,
        timestamp: u64,
        fixed_block_reward: u64,
        fee_share: u64,
    },
    ReleaseMaturedProposerRewards,
    ReleaseMaturedReceiptRewards,
    ReleaseMaturedChallengeRewards,
    ReleaseMaturedCreditRewards,
    RegisterModel {
        model_id: Hash,
        architecture_hash: Hash,
        weight_root: Hash,
        config_hash: Hash,
    },
    ApplyModelTransition {
        model_id: Hash,
        step: u64,
        weight_root_before: Hash,
        weight_root_after: Hash,
    },
    ApplyChallengeOutcome(ChallengeOutcome),
    SubmitBlockCheckChallenge(BlockCheckChallenge),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainEvent {
    MinerRegistered(Address),
    ValidatorRegistered(Address),
    AccountTransferred {
        from: Address,
        to: Address,
        amount: u64,
    },
    RewardClaimed {
        address: Address,
        amount: u64,
    },
    JobAccepted(Hash),
    ReceiptAccepted(Hash),
    AttestationAccepted {
        receipt_id: Hash,
        validator: Address,
    },
    ValidatorAuditAccepted {
        audit_id: Hash,
        auditor: Address,
        validator: Address,
        passed: bool,
    },
    ValidatorAuditSlashApplied {
        audit_id: Hash,
        validator: Address,
        amount: u64,
        reason: String,
    },
    ValidatorAuditAppealAccepted {
        audit_id: Hash,
        validator: Address,
        deadline_height: u64,
    },
    ValidatorAuditAppealResolved {
        audit_id: Hash,
        validator: Address,
        resolution: ValidatorAuditAppealResolution,
        receipt_reward_reinstated: bool,
    },
    BlockVoteAccepted {
        block_hash: Hash,
        validator: Address,
    },
    ReceiptSettled(Hash),
    RewardCredited {
        address: Address,
        amount: u64,
    },
    CreditRewardPending {
        claim_id: Hash,
        beneficiary: Address,
        amount: u64,
        claimable_at_height: u64,
    },
    CreditRewardReleased {
        claim_id: Hash,
        beneficiary: Address,
        amount: u64,
    },
    ProgramBodyRegistered {
        graph_id: GraphId,
    },
    BlockProduced {
        height: u64,
        hash: Hash,
    },
    ProposerRewardPending {
        block_height: u64,
        proposer: Address,
        amount: u64,
        claimable_at_height: u64,
    },
    ProposerRewardReleased {
        block_height: u64,
        proposer: Address,
        amount: u64,
    },
    ReceiptRewardPending {
        claim_id: Hash,
        receipt_id: Hash,
        beneficiary: Address,
        amount: u64,
        claimable_at_height: u64,
    },
    ReceiptRewardReleased {
        claim_id: Hash,
        receipt_id: Hash,
        beneficiary: Address,
        amount: u64,
    },
    ChallengeRewardPending {
        claim_id: Hash,
        challenge_id: Hash,
        block_hash: Hash,
        receipt_id: Hash,
        challenger: Address,
        amount: u64,
        claimable_at_height: u64,
    },
    ChallengeRewardReleased {
        claim_id: Hash,
        challenge_id: Hash,
        challenger: Address,
        amount: u64,
    },
    BlockAccepted {
        height: u64,
        hash: Hash,
    },
    BlockFinalized(Hash),
    ModelRegistered(Hash),
    ModelTransitionApplied {
        model_id: Hash,
        step: u64,
        weight_root_after: Hash,
    },
    ChallengeRejected {
        reason: String,
    },
    ChallengeProvenInvalid {
        dishonest_party: Address,
        slash_amount: u64,
        reason: String,
    },
    BlockCheckChallengeProven {
        block_hash: Hash,
        receipt_id: Hash,
        proposer: Address,
        challenger: Address,
        proposer_reward_clawback: u64,
        challenger_reward: u64,
        penalty_until_height: u64,
        reason: String,
    },
}

pub trait ChainEngine {
    fn apply_command(&mut self, command: ChainCommand) -> Result<Vec<ChainEvent>>;
    fn view(&self) -> &ChainState;
    fn params(&self) -> &ChainParams;
    fn blocks(&self) -> &[TensorBlock];
}
