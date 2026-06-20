use crate::ir::TensorGraph;
use crate::jobs::{
    GraphJob, GraphReceipt, LinearTrainingStepJob, LinearTrainingStepReceipt, MatmulJob,
    PrimitiveType, TensorOpReceipt,
};
use crate::merkle::MerkleProof;
use crate::types::{Address, Hash, Signature, hash_bytes, sign, verify_signature};
use crate::verify::{FreivaldsParams, ValidatorAttestation, VerificationResult};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainParams {
    pub block_time_seconds: u64,
    pub epoch_length: u64,
    pub receipt_submission_window: u64,
    pub verification_window: u64,
    pub reward_settlement_delay_epochs: u64,
    pub challenge_window_epochs: u64,
    pub replication_factor: usize,
    pub agreement_quorum: usize,
    pub finality_stake_numerator: u64,
    pub finality_stake_denominator: u64,
    pub miner_reward_bps: u64,
    pub validator_reward_bps: u64,
    pub proposer_reward_bps: u64,
    pub treasury_reward_bps: u64,
    pub miner_min_stake: u64,
    pub validator_min_stake: u64,
    pub data_unavailability_miner_slash_amount: u64,
    pub validator_audit_sample_numerator: u64,
    pub validator_audit_sample_denominator: u64,
    pub validator_audit_window_blocks: u64,
    pub validator_audit_slash_amount: u64,
    pub difficulty_initial_target: Hash,
    pub difficulty_floor_target: Hash,
    pub difficulty_ceiling_target: Hash,
    pub difficulty_target_block_time_seconds: u64,
    pub difficulty_retarget_epoch_length: u64,
    pub difficulty_retarget_max_ratio: u64,
    pub pow_timeout_blocks: u64,
    pub freivalds: FreivaldsParams,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            block_time_seconds: 6,
            epoch_length: 100,
            receipt_submission_window: 20,
            verification_window: 40,
            reward_settlement_delay_epochs: 1,
            challenge_window_epochs: 1,
            replication_factor: 5,
            agreement_quorum: 3,
            finality_stake_numerator: 2,
            finality_stake_denominator: 3,
            miner_reward_bps: 7_000,
            validator_reward_bps: 2_000,
            proposer_reward_bps: 500,
            treasury_reward_bps: 500,
            miner_min_stake: 100,
            validator_min_stake: 10_000,
            data_unavailability_miner_slash_amount: 10,
            validator_audit_sample_numerator: 0,
            validator_audit_sample_denominator: 1,
            validator_audit_window_blocks: 10,
            validator_audit_slash_amount: 100,
            difficulty_initial_target: default_useful_pow_target(),
            difficulty_floor_target: default_difficulty_floor_target(),
            difficulty_ceiling_target: [0xff; 32],
            difficulty_target_block_time_seconds: 6,
            difficulty_retarget_epoch_length: 100,
            difficulty_retarget_max_ratio: 4,
            pow_timeout_blocks: 2,
            freivalds: FreivaldsParams::default(),
        }
    }
}

fn default_useful_pow_target() -> Hash {
    let mut target = [0xff; 32];
    target[0] = 0x7f;
    target
}

fn default_difficulty_floor_target() -> Hash {
    let mut target = [0xff; 32];
    target[0] = 0x03;
    target
}

impl ChainParams {
    pub fn reward_maturity_delay_blocks(&self) -> u64 {
        self.reward_settlement_delay_epochs
            .saturating_add(self.challenge_window_epochs)
            .max(1)
            .saturating_mul(self.epoch_length.max(1))
    }

    pub fn challenge_window_blocks(&self) -> u64 {
        self.challenge_window_epochs
            .max(1)
            .saturating_mul(self.epoch_length.max(1))
    }

    pub fn tensor_retention_window_blocks(&self) -> u64 {
        self.reward_settlement_delay_epochs
            .saturating_add(self.challenge_window_epochs)
            .saturating_mul(self.epoch_length.max(1))
    }

    pub fn tensor_retention_deadline(&self, submitted_at_block: u64) -> u64 {
        submitted_at_block.saturating_add(self.tensor_retention_window_blocks())
    }

    pub fn reward_allocation(&self, total_emission: u64) -> RewardAllocation {
        let miner_reward_pool = reward_share(total_emission, self.miner_reward_bps);
        let validator_reward_pool = reward_share(total_emission, self.validator_reward_bps);
        let proposer_reward = reward_share(total_emission, self.proposer_reward_bps);
        let explicit_treasury = reward_share(total_emission, self.treasury_reward_bps);
        let allocated = miner_reward_pool
            .saturating_add(validator_reward_pool)
            .saturating_add(proposer_reward)
            .saturating_add(explicit_treasury);
        RewardAllocation {
            miner_reward_pool,
            validator_reward_pool,
            proposer_reward,
            treasury_reward: explicit_treasury
                .saturating_add(total_emission.saturating_sub(allocated)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockspaceCaps {
    pub max_receipts: usize,
    pub max_tensor_work_units: u64,
    pub max_bytes: u64,
}

impl Default for BlockspaceCaps {
    fn default() -> Self {
        Self {
            max_receipts: 64,
            max_tensor_work_units: 1_000_000,
            max_bytes: 1_048_576,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockspaceSelection {
    pub receipt_ids: Vec<Hash>,
    pub total_tensor_work_units: u64,
    pub total_bytes: u64,
    pub caps: BlockspaceCaps,
}

impl BlockspaceSelection {
    pub fn receipt_set(&self) -> BTreeSet<Hash> {
        self.receipt_ids.iter().copied().collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockParentSnapshot {
    pub parent_hash: Hash,
    pub height: u64,
    pub epoch: u64,
    pub state_root: Hash,
    pub beacon_round: u64,
    pub beacon: Hash,
    pub attestation_root: Hash,
    pub reward_root: Hash,
    pub settled_receipt_pool_root: Hash,
    pub included_receipt_root: Hash,
    pub data_unavailable_receipt_root: Hash,
    pub data_unavailability_slash_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedReceiptOpening {
    pub receipt_id: Hash,
    pub receipt_leaf: Hash,
    pub receipt_leaf_index: u64,
    pub receipt_leaf_proof: Option<MerkleProof>,
    pub check_leaf: Hash,
    pub check_leaf_index: u64,
    pub check_leaf_proof: Option<MerkleProof>,
    pub primitive_type: Option<PrimitiveType>,
    pub tensor_work_units: u64,
    pub estimated_block_bytes: u64,
    pub submitted_at_block: u64,
    pub settled: bool,
    pub included_before_parent: bool,
    pub data_available: bool,
    pub expires_at_block: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockApplyOutcome {
    pub parent_snapshot: BlockParentSnapshot,
    pub selected_receipt_ids: Vec<Hash>,
    pub selected_receipt_root: Hash,
    pub checks_root: Hash,
    pub selected_openings: Vec<SelectedReceiptOpening>,
    pub child_state_root: Hash,
    pub child_reward_root: Hash,
    pub child_height: u64,
    pub child_epoch: u64,
    pub child_beacon_round: u64,
    pub child_beacon: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockCheckChallengeRecord {
    pub block_hash: Hash,
    pub block_height: u64,
    pub receipt_id: Hash,
    pub proposer: Address,
    pub challenger: Address,
    pub expected_check_leaf: Hash,
    pub observed_check_leaf: Hash,
    pub challenged_at_height: u64,
    pub proposer_reward_clawback: u64,
    pub challenger_reward: u64,
    pub penalty_until_height: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingProposerReward {
    pub block_height: u64,
    pub proposer: Address,
    pub amount: u64,
    pub claimable_at_height: u64,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReceiptRewardKind {
    Miner,
    Validator,
}

impl ReceiptRewardKind {
    pub fn tag(self) -> u8 {
        match self {
            Self::Miner => 1,
            Self::Validator => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingReceiptReward {
    pub claim_id: Hash,
    pub receipt_id: Hash,
    pub beneficiary: Address,
    pub amount: u64,
    pub kind: ReceiptRewardKind,
    pub claimable_at_height: u64,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingChallengeReward {
    pub claim_id: Hash,
    pub challenge_id: Hash,
    pub block_hash: Hash,
    pub receipt_id: Hash,
    pub challenger: Address,
    pub amount: u64,
    pub claimable_at_height: u64,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCreditReward {
    pub claim_id: Hash,
    pub beneficiary: Address,
    pub amount: u64,
    pub claimable_at_height: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RewardClaimLedger {
    Proposer,
    ReceiptMiner,
    ReceiptValidator,
    Challenge,
    Credit,
}

impl RewardClaimLedger {
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposer => "proposer",
            Self::ReceiptMiner => "receipt_miner",
            Self::ReceiptValidator => "receipt_validator",
            Self::Challenge => "challenge",
            Self::Credit => "credit",
        }
    }

    pub fn receipt_kind_label(self) -> Option<&'static str> {
        match self {
            Self::ReceiptMiner => Some("miner"),
            Self::ReceiptValidator => Some("validator"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RewardClaimKey {
    BlockHeight(u64),
    Hash(Hash),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardClaimView {
    pub ledger: RewardClaimLedger,
    pub claim_id: RewardClaimKey,
    pub subject_id: RewardClaimKey,
    pub related_id: Option<RewardClaimKey>,
    pub beneficiary: Address,
    pub amount: u64,
    pub claimable_at_height: u64,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptRandomnessAnchor {
    pub receipt_id: Hash,
    pub beacon_round: u64,
    pub finalized_randomness: Hash,
    pub assignment_seed: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataUnavailabilitySlashRecord {
    pub receipt_id: Hash,
    pub miner: Address,
    pub evidence_validator: Address,
    pub amount: u64,
    pub slashed_at_height: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditAssignment {
    pub audit_id: Hash,
    pub receipt_id: Hash,
    pub validator: Address,
    pub assigned_at_height: u64,
    pub deadline_height: u64,
    pub seed: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditReport {
    pub audit_id: Hash,
    pub auditor: Address,
    pub canonical_result: VerificationResult,
    pub canonical_data_availability_passed: bool,
    pub checks_root: Hash,
    pub signature: Signature,
}

impl ValidatorAuditReport {
    pub fn new(
        audit_id: Hash,
        auditor: Address,
        canonical_result: VerificationResult,
        canonical_data_availability_passed: bool,
        checks_root: Hash,
    ) -> Self {
        let message = Self::message_hash(
            &audit_id,
            &auditor,
            canonical_result,
            canonical_data_availability_passed,
            &checks_root,
        );
        Self {
            audit_id,
            auditor,
            canonical_result,
            canonical_data_availability_passed,
            checks_root,
            signature: sign(&auditor, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        verify_signature(
            &self.auditor,
            &Self::message_hash(
                &self.audit_id,
                &self.auditor,
                self.canonical_result,
                self.canonical_data_availability_passed,
                &self.checks_root,
            ),
            &self.signature,
        )
    }

    fn message_hash(
        audit_id: &Hash,
        auditor: &Address,
        canonical_result: VerificationResult,
        canonical_data_availability_passed: bool,
        checks_root: &Hash,
    ) -> Hash {
        hash_bytes(
            b"tensor-vm-validator-audit-report-v1",
            &[
                audit_id,
                auditor,
                &[verification_result_tag(canonical_result)],
                &[u8::from(canonical_data_availability_passed)],
                checks_root,
            ],
        )
    }
}

fn verification_result_tag(result: VerificationResult) -> u8 {
    match result {
        VerificationResult::Valid => 1,
        VerificationResult::Invalid => 2,
        VerificationResult::Unavailable => 3,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditResult {
    pub audit_id: Hash,
    pub receipt_id: Hash,
    pub validator: Address,
    pub auditor: Address,
    pub attested_result: VerificationResult,
    pub canonical_result: VerificationResult,
    pub attested_data_availability_passed: bool,
    pub canonical_data_availability_passed: bool,
    pub checks_root: Hash,
    pub submitted_at_height: u64,
    pub passed: bool,
    pub signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditSlashRecord {
    pub audit_id: Hash,
    pub receipt_id: Hash,
    pub validator: Address,
    pub auditor: Address,
    pub amount: u64,
    pub slashed_at_height: u64,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RewardAllocation {
    pub miner_reward_pool: u64,
    pub validator_reward_pool: u64,
    pub proposer_reward: u64,
    pub treasury_reward: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum HardwareClass {
    Cpu,
    ConsumerGpu,
    DatacenterGpu,
    Other,
}

impl HardwareClass {
    pub fn is_gpu(self) -> bool {
        matches!(self, Self::ConsumerGpu | Self::DatacenterGpu)
    }

    pub fn tag(self) -> u8 {
        match self {
            Self::Cpu => 1,
            Self::ConsumerGpu => 2,
            Self::DatacenterGpu => 3,
            Self::Other => 4,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerState {
    pub address: Address,
    pub operator_id: Hash,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
    pub hardware_class: HardwareClass,
    pub gpu_utilization_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorState {
    pub address: Address,
    pub stake: u64,
    pub reputation: i64,
    pub valid_attestations: u64,
    pub missed_assignments: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct AccountState {
    pub address: Address,
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct RewardState {
    pub(in crate::chain) balances: BTreeMap<Address, u64>,
    pub(in crate::chain) treasury: u64,
}

impl RewardState {
    pub(crate) fn from_parts(balances: BTreeMap<Address, u64>, treasury: u64) -> Self {
        Self { balances, treasury }
    }

    pub(in crate::chain) fn credit(&mut self, address: Address, amount: u64) {
        *self.balances.entry(address).or_default() += amount;
    }

    pub(in crate::chain) fn clear_balance(&mut self, address: Address) {
        self.balances.insert(address, 0);
    }

    pub(in crate::chain) fn credit_treasury(&mut self, amount: u64) {
        self.treasury = self.treasury.saturating_add(amount);
    }

    pub fn balance(&self, address: &Address) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn balances(&self) -> &BTreeMap<Address, u64> {
        &self.balances
    }

    pub fn total_balance(&self) -> u64 {
        self.balances.values().sum()
    }

    pub fn treasury(&self) -> u64 {
        self.treasury
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobState {
    TensorOp(MatmulJob),
    LinearTrainingStep(LinearTrainingStepJob),
    GraphExecution(GraphJob),
}

impl JobState {
    pub fn job_id(&self) -> Hash {
        match self {
            Self::TensorOp(job) => job.job_id,
            Self::LinearTrainingStep(job) => job.job_id,
            Self::GraphExecution(job) => job.job_id,
        }
    }

    pub fn deadline_block(&self) -> u64 {
        match self {
            Self::TensorOp(job) => job.deadline_block,
            Self::LinearTrainingStep(job) => job.deadline_block,
            Self::GraphExecution(job) => job.deadline_block,
        }
    }

    pub fn program_hash(&self) -> Hash {
        match self {
            Self::TensorOp(job) => job.program_hash(),
            Self::LinearTrainingStep(job) => job.program_hash(),
            Self::GraphExecution(job) => job.program_hash(),
        }
    }

    pub fn tensor_ir_graph(&self) -> Option<TensorGraph> {
        match self {
            Self::TensorOp(job) => Some(job.tensor_ir_graph()),
            Self::LinearTrainingStep(job) => Some(job.tensor_ir_graph()),
            Self::GraphExecution(_) => None,
        }
    }

    pub fn canonical_program_body(&self) -> Option<Vec<u8>> {
        self.tensor_ir_graph()
            .map(|graph| graph.canonical_json().into_bytes())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptState {
    TensorOp(TensorOpReceipt),
    LinearTrainingStep(LinearTrainingStepReceipt),
    GraphExecution(GraphReceipt),
}

impl ReceiptState {
    pub fn receipt_id(&self) -> Hash {
        match self {
            Self::TensorOp(receipt) => receipt.receipt_id,
            Self::LinearTrainingStep(receipt) => receipt.receipt_id,
            Self::GraphExecution(receipt) => receipt.receipt_id,
        }
    }

    pub fn job_id(&self) -> Hash {
        match self {
            Self::TensorOp(receipt) => receipt.job_id,
            Self::LinearTrainingStep(receipt) => receipt.job_id,
            Self::GraphExecution(receipt) => receipt.job_id,
        }
    }

    pub fn miner(&self) -> Address {
        match self {
            Self::TensorOp(receipt) => receipt.miner,
            Self::LinearTrainingStep(receipt) => receipt.miner,
            Self::GraphExecution(receipt) => receipt.miner,
        }
    }

    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::TensorOp(_) => PrimitiveType::TensorOp,
            Self::LinearTrainingStep(_) => PrimitiveType::LinearTrainingStep,
            Self::GraphExecution(_) => PrimitiveType::GraphExecution,
        }
    }

    pub fn submitted_at_block(&self) -> u64 {
        match self {
            Self::TensorOp(receipt) => receipt.submitted_at_block,
            Self::LinearTrainingStep(receipt) => receipt.submitted_at_block,
            Self::GraphExecution(receipt) => receipt.submitted_at_block,
        }
    }

    pub fn tensor_work_units(&self) -> u64 {
        match self {
            Self::TensorOp(receipt) => receipt.tensor_work_units,
            Self::LinearTrainingStep(receipt) => receipt.tensor_work_units,
            Self::GraphExecution(receipt) => receipt.tensor_work_units,
        }
    }

    pub fn estimated_block_bytes(&self) -> u64 {
        match self {
            Self::TensorOp(receipt) => {
                let roots = receipt
                    .input_roots
                    .len()
                    .saturating_add(receipt.output_roots.len()) as u64;
                32 * (7 + roots) + 8 * 3
            }
            Self::LinearTrainingStep(_) => 32 * 10 + 8 * 4,
            Self::GraphExecution(receipt) => {
                let roots = receipt
                    .input_roots
                    .len()
                    .saturating_add(receipt.output_roots.len()) as u64;
                32 * (6 + roots) + 8 * 3
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelState {
    pub model_id: Hash,
    pub architecture_hash: Hash,
    pub weight_root: Hash,
    pub optimizer_state_root: Option<Hash>,
    pub step: u64,
    pub config_hash: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Transaction {
    RegisterMiner(Address),
    RegisterValidator(Address),
    SubmitTensorOpReceipt(Hash),
    SubmitLinearTrainingStepReceipt(Hash),
    SubmitAttestation(Hash),
    Transfer { to: Address, amount: u64 },
    ClaimReward(Address),
}

impl Transaction {
    pub fn is_reference_submission(&self) -> bool {
        matches!(
            self,
            Self::SubmitTensorOpReceipt(_)
                | Self::SubmitLinearTrainingStepReceipt(_)
                | Self::SubmitAttestation(_)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockProductionKind {
    UsefulVerificationPow,
    PowSkipFallback,
}

impl BlockProductionKind {
    pub fn tag(self) -> u8 {
        match self {
            Self::UsefulVerificationPow => 1,
            Self::PowSkipFallback => 2,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::UsefulVerificationPow),
            2 => Some(Self::PowSkipFallback),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UsefulVerificationPow => "useful_verification_pow",
            Self::PowSkipFallback => "pow_skip_fallback",
        }
    }

    pub fn requires_pow(self) -> bool {
        matches!(self, Self::UsefulVerificationPow)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorBlock {
    pub height: u64,
    pub parent_hash: Hash,
    pub epoch: u64,
    pub proposer: Address,
    pub settled_receipt_set_root: Hash,
    pub checks_root: Hash,
    pub attestation_root: Hash,
    pub state_root: Hash,
    pub reward_root: Hash,
    pub beacon_round: u64,
    pub beacon: Hash,
    pub production_kind: BlockProductionKind,
    pub proposer_reward: u64,
    pub difficulty_target: Hash,
    pub nonce: u64,
    pub timestamp: u64,
    pub proposer_signature: Signature,
    pub validator_signature_aggregate: Signature,
}

impl TensorBlock {
    pub fn hash(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-block",
            &[
                &self.height.to_le_bytes(),
                &self.parent_hash,
                &self.epoch.to_le_bytes(),
                &self.proposer,
                &self.settled_receipt_set_root,
                &self.checks_root,
                &self.attestation_root,
                &self.state_root,
                &self.reward_root,
                &self.beacon_round.to_le_bytes(),
                &self.beacon,
                &[self.production_kind.tag()],
                &self.proposer_reward.to_le_bytes(),
                &self.difficulty_target,
                &self.nonce.to_le_bytes(),
                &self.timestamp.to_le_bytes(),
            ],
        )
    }

    pub fn pow_header_hash(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-useful-pow-header",
            &[
                &self.height.to_le_bytes(),
                &self.parent_hash,
                &self.epoch.to_le_bytes(),
                &self.proposer,
                &self.settled_receipt_set_root,
                &self.checks_root,
                &self.attestation_root,
                &self.state_root,
                &self.reward_root,
                &self.beacon_round.to_le_bytes(),
                &self.beacon,
                &[self.production_kind.tag()],
                &self.proposer_reward.to_le_bytes(),
                &self.difficulty_target,
                &self.timestamp.to_le_bytes(),
            ],
        )
    }

    pub fn pow_hash(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-useful-pow",
            &[&self.pow_header_hash(), &self.nonce.to_le_bytes()],
        )
    }

    pub fn pow_valid(&self) -> bool {
        hash_below_target(&self.pow_hash(), &self.difficulty_target)
    }
}

pub fn hash_below_target(hash: &Hash, target: &Hash) -> bool {
    hash < target
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockVote {
    pub validator: Address,
    pub block_hash: Hash,
    pub block_height: u64,
    pub stake: u64,
    pub signature: Signature,
}

impl BlockVote {
    pub fn new(validator: Address, stake: u64, block: &TensorBlock) -> Self {
        let block_hash = block.hash();
        let message = Self::message_hash(&block_hash, block.height, stake);
        Self {
            validator,
            block_hash,
            block_height: block.height,
            stake,
            signature: sign(&validator, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        verify_signature(
            &self.validator,
            &Self::message_hash(&self.block_hash, self.block_height, self.stake),
            &self.signature,
        )
    }

    fn message_hash(block_hash: &Hash, block_height: u64, stake: u64) -> Hash {
        hash_bytes(
            b"tensor-vm-block-vote-v1",
            &[
                block_hash,
                &block_height.to_le_bytes(),
                &stake.to_le_bytes(),
            ],
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainState {
    pub(in crate::chain) height: u64,
    pub(in crate::chain) epoch: u64,
    pub(in crate::chain) finalized_beacon_round: u64,
    pub(in crate::chain) finalized_randomness: Hash,
    pub(in crate::chain) genesis_beacon_round: u64,
    pub(in crate::chain) genesis_randomness: Hash,
    pub(in crate::chain) accounts: BTreeMap<Address, AccountState>,
    pub(in crate::chain) miners: BTreeMap<Address, MinerState>,
    pub(in crate::chain) validators: BTreeMap<Address, ValidatorState>,
    pub(in crate::chain) jobs: BTreeMap<Hash, JobState>,
    pub(in crate::chain) program_bodies: BTreeMap<Hash, Vec<u8>>,
    pub(in crate::chain) receipts: BTreeMap<Hash, ReceiptState>,
    pub(in crate::chain) receipt_randomness_anchors: BTreeMap<Hash, ReceiptRandomnessAnchor>,
    pub(in crate::chain) attestations: BTreeMap<Hash, Vec<ValidatorAttestation>>,
    pub(in crate::chain) block_votes: BTreeMap<Hash, Vec<BlockVote>>,
    pub(in crate::chain) finalized_blocks: BTreeSet<Hash>,
    pub(in crate::chain) data_unavailable_receipts: BTreeSet<Hash>,
    pub(in crate::chain) data_unavailability_slashes: BTreeMap<Hash, DataUnavailabilitySlashRecord>,
    pub(in crate::chain) validator_audit_assignments: BTreeMap<Hash, ValidatorAuditAssignment>,
    pub(in crate::chain) validator_audit_results: BTreeMap<Hash, ValidatorAuditResult>,
    pub(in crate::chain) validator_audit_slashes: BTreeMap<Hash, ValidatorAuditSlashRecord>,
    pub(in crate::chain) settled_receipts: BTreeSet<Hash>,
    pub(in crate::chain) included_receipts: BTreeSet<Hash>,
    pub(in crate::chain) block_selected_receipts: BTreeMap<Hash, Vec<Hash>>,
    pub(in crate::chain) block_check_challenges: BTreeMap<Hash, BlockCheckChallengeRecord>,
    pub(in crate::chain) challenged_receipts: BTreeSet<Hash>,
    pub(in crate::chain) proposer_penalty_until: BTreeMap<Address, u64>,
    pub(in crate::chain) pending_proposer_rewards: BTreeMap<u64, PendingProposerReward>,
    pub(in crate::chain) pending_receipt_rewards: BTreeMap<Hash, PendingReceiptReward>,
    pub(in crate::chain) pending_challenge_rewards: BTreeMap<Hash, PendingChallengeReward>,
    pub(in crate::chain) pending_credit_rewards: BTreeMap<Hash, PendingCreditReward>,
    pub(in crate::chain) model_states: BTreeMap<Hash, ModelState>,
    pub(in crate::chain) rewards: RewardState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChainStateParts {
    pub height: u64,
    pub epoch: u64,
    pub finalized_beacon_round: u64,
    pub finalized_randomness: Hash,
    pub genesis_beacon_round: u64,
    pub genesis_randomness: Hash,
    pub accounts: BTreeMap<Address, AccountState>,
    pub miners: BTreeMap<Address, MinerState>,
    pub validators: BTreeMap<Address, ValidatorState>,
    pub jobs: BTreeMap<Hash, JobState>,
    pub program_bodies: BTreeMap<Hash, Vec<u8>>,
    pub receipts: BTreeMap<Hash, ReceiptState>,
    pub receipt_randomness_anchors: BTreeMap<Hash, ReceiptRandomnessAnchor>,
    pub attestations: BTreeMap<Hash, Vec<ValidatorAttestation>>,
    pub block_votes: BTreeMap<Hash, Vec<BlockVote>>,
    pub finalized_blocks: BTreeSet<Hash>,
    pub data_unavailable_receipts: BTreeSet<Hash>,
    pub data_unavailability_slashes: BTreeMap<Hash, DataUnavailabilitySlashRecord>,
    pub validator_audit_assignments: BTreeMap<Hash, ValidatorAuditAssignment>,
    pub validator_audit_results: BTreeMap<Hash, ValidatorAuditResult>,
    pub validator_audit_slashes: BTreeMap<Hash, ValidatorAuditSlashRecord>,
    pub settled_receipts: BTreeSet<Hash>,
    pub included_receipts: BTreeSet<Hash>,
    pub block_selected_receipts: BTreeMap<Hash, Vec<Hash>>,
    pub block_check_challenges: BTreeMap<Hash, BlockCheckChallengeRecord>,
    pub challenged_receipts: BTreeSet<Hash>,
    pub proposer_penalty_until: BTreeMap<Address, u64>,
    pub pending_proposer_rewards: BTreeMap<u64, PendingProposerReward>,
    pub pending_receipt_rewards: BTreeMap<Hash, PendingReceiptReward>,
    pub pending_challenge_rewards: BTreeMap<Hash, PendingChallengeReward>,
    pub pending_credit_rewards: BTreeMap<Hash, PendingCreditReward>,
    pub model_states: BTreeMap<Hash, ModelState>,
    pub rewards: RewardState,
}

impl ChainState {
    pub(crate) fn from_parts(parts: ChainStateParts) -> Self {
        Self {
            height: parts.height,
            epoch: parts.epoch,
            finalized_beacon_round: parts.finalized_beacon_round,
            finalized_randomness: parts.finalized_randomness,
            genesis_beacon_round: parts.genesis_beacon_round,
            genesis_randomness: parts.genesis_randomness,
            accounts: parts.accounts,
            miners: parts.miners,
            validators: parts.validators,
            jobs: parts.jobs,
            program_bodies: parts.program_bodies,
            receipts: parts.receipts,
            receipt_randomness_anchors: parts.receipt_randomness_anchors,
            attestations: parts.attestations,
            block_votes: parts.block_votes,
            finalized_blocks: parts.finalized_blocks,
            data_unavailable_receipts: parts.data_unavailable_receipts,
            data_unavailability_slashes: parts.data_unavailability_slashes,
            validator_audit_assignments: parts.validator_audit_assignments,
            validator_audit_results: parts.validator_audit_results,
            validator_audit_slashes: parts.validator_audit_slashes,
            settled_receipts: parts.settled_receipts,
            included_receipts: parts.included_receipts,
            block_selected_receipts: parts.block_selected_receipts,
            block_check_challenges: parts.block_check_challenges,
            challenged_receipts: parts.challenged_receipts,
            proposer_penalty_until: parts.proposer_penalty_until,
            pending_proposer_rewards: parts.pending_proposer_rewards,
            pending_receipt_rewards: parts.pending_receipt_rewards,
            pending_challenge_rewards: parts.pending_challenge_rewards,
            pending_credit_rewards: parts.pending_credit_rewards,
            model_states: parts.model_states,
            rewards: parts.rewards,
        }
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn finalized_beacon_round(&self) -> u64 {
        self.finalized_beacon_round
    }

    pub fn finalized_randomness(&self) -> Hash {
        self.finalized_randomness
    }

    pub fn genesis_beacon_round(&self) -> u64 {
        self.genesis_beacon_round
    }

    pub fn genesis_randomness(&self) -> Hash {
        self.genesis_randomness
    }

    pub fn accounts(&self) -> &BTreeMap<Address, AccountState> {
        &self.accounts
    }

    pub fn miners(&self) -> &BTreeMap<Address, MinerState> {
        &self.miners
    }

    pub fn validators(&self) -> &BTreeMap<Address, ValidatorState> {
        &self.validators
    }

    pub fn jobs(&self) -> &BTreeMap<Hash, JobState> {
        &self.jobs
    }

    pub fn program_bodies(&self) -> &BTreeMap<Hash, Vec<u8>> {
        &self.program_bodies
    }

    pub fn program_body(&self, graph_id: &Hash) -> Option<&[u8]> {
        self.program_bodies.get(graph_id).map(Vec::as_slice)
    }

    pub fn receipts(&self) -> &BTreeMap<Hash, ReceiptState> {
        &self.receipts
    }

    pub fn receipt_randomness_anchors(&self) -> &BTreeMap<Hash, ReceiptRandomnessAnchor> {
        &self.receipt_randomness_anchors
    }

    pub fn attestations(&self) -> &BTreeMap<Hash, Vec<ValidatorAttestation>> {
        &self.attestations
    }

    pub fn block_votes(&self) -> &BTreeMap<Hash, Vec<BlockVote>> {
        &self.block_votes
    }

    pub fn finalized_blocks(&self) -> &BTreeSet<Hash> {
        &self.finalized_blocks
    }

    pub fn data_unavailable_receipts(&self) -> &BTreeSet<Hash> {
        &self.data_unavailable_receipts
    }

    pub fn data_unavailability_slashes(&self) -> &BTreeMap<Hash, DataUnavailabilitySlashRecord> {
        &self.data_unavailability_slashes
    }

    pub fn validator_audit_assignments(&self) -> &BTreeMap<Hash, ValidatorAuditAssignment> {
        &self.validator_audit_assignments
    }

    pub fn validator_audit_results(&self) -> &BTreeMap<Hash, ValidatorAuditResult> {
        &self.validator_audit_results
    }

    pub fn validator_audit_slashes(&self) -> &BTreeMap<Hash, ValidatorAuditSlashRecord> {
        &self.validator_audit_slashes
    }

    pub fn settled_receipts(&self) -> &BTreeSet<Hash> {
        &self.settled_receipts
    }

    pub fn included_receipts(&self) -> &BTreeSet<Hash> {
        &self.included_receipts
    }

    pub fn block_selected_receipts(&self) -> &BTreeMap<Hash, Vec<Hash>> {
        &self.block_selected_receipts
    }

    pub fn block_check_challenges(&self) -> &BTreeMap<Hash, BlockCheckChallengeRecord> {
        &self.block_check_challenges
    }

    pub fn challenged_receipts(&self) -> &BTreeSet<Hash> {
        &self.challenged_receipts
    }

    pub fn proposer_penalty_until(&self) -> &BTreeMap<Address, u64> {
        &self.proposer_penalty_until
    }

    pub fn pending_proposer_rewards(&self) -> &BTreeMap<u64, PendingProposerReward> {
        &self.pending_proposer_rewards
    }

    pub fn pending_receipt_rewards(&self) -> &BTreeMap<Hash, PendingReceiptReward> {
        &self.pending_receipt_rewards
    }

    pub fn pending_challenge_rewards(&self) -> &BTreeMap<Hash, PendingChallengeReward> {
        &self.pending_challenge_rewards
    }

    pub fn pending_credit_rewards(&self) -> &BTreeMap<Hash, PendingCreditReward> {
        &self.pending_credit_rewards
    }

    pub fn pending_reward_claims(&self) -> Vec<RewardClaimView> {
        let mut claims = Vec::new();
        for (block_height, reward) in &self.pending_proposer_rewards {
            claims.push(RewardClaimView {
                ledger: RewardClaimLedger::Proposer,
                claim_id: RewardClaimKey::BlockHeight(*block_height),
                subject_id: RewardClaimKey::BlockHeight(*block_height),
                related_id: None,
                beneficiary: reward.proposer,
                amount: reward.amount,
                claimable_at_height: reward.claimable_at_height,
                voided_by_challenge: reward.voided_by_challenge,
            });
        }
        for (claim_id, reward) in &self.pending_receipt_rewards {
            claims.push(RewardClaimView {
                ledger: match reward.kind {
                    ReceiptRewardKind::Miner => RewardClaimLedger::ReceiptMiner,
                    ReceiptRewardKind::Validator => RewardClaimLedger::ReceiptValidator,
                },
                claim_id: RewardClaimKey::Hash(*claim_id),
                subject_id: RewardClaimKey::Hash(reward.receipt_id),
                related_id: None,
                beneficiary: reward.beneficiary,
                amount: reward.amount,
                claimable_at_height: reward.claimable_at_height,
                voided_by_challenge: reward.voided_by_challenge,
            });
        }
        for (claim_id, reward) in &self.pending_challenge_rewards {
            claims.push(RewardClaimView {
                ledger: RewardClaimLedger::Challenge,
                claim_id: RewardClaimKey::Hash(*claim_id),
                subject_id: RewardClaimKey::Hash(reward.challenge_id),
                related_id: Some(RewardClaimKey::Hash(reward.receipt_id)),
                beneficiary: reward.challenger,
                amount: reward.amount,
                claimable_at_height: reward.claimable_at_height,
                voided_by_challenge: reward.voided_by_challenge,
            });
        }
        for (claim_id, reward) in &self.pending_credit_rewards {
            claims.push(RewardClaimView {
                ledger: RewardClaimLedger::Credit,
                claim_id: RewardClaimKey::Hash(*claim_id),
                subject_id: RewardClaimKey::Hash(*claim_id),
                related_id: None,
                beneficiary: reward.beneficiary,
                amount: reward.amount,
                claimable_at_height: reward.claimable_at_height,
                voided_by_challenge: false,
            });
        }
        claims.sort_by(|left, right| {
            left.claimable_at_height
                .cmp(&right.claimable_at_height)
                .then_with(|| left.ledger.cmp(&right.ledger))
                .then_with(|| left.claim_id.cmp(&right.claim_id))
        });
        claims
    }

    pub fn model_states(&self) -> &BTreeMap<Hash, ModelState> {
        &self.model_states
    }

    pub fn rewards(&self) -> &RewardState {
        &self.rewards
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chain {
    pub(crate) params: ChainParams,
    pub(crate) state: ChainState,
    pub(crate) blocks: Vec<TensorBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChainParts {
    pub params: ChainParams,
    pub state: ChainState,
    pub blocks: Vec<TensorBlock>,
}

impl Chain {
    pub(crate) fn from_parts(parts: ChainParts) -> Self {
        Self {
            params: parts.params,
            state: parts.state,
            blocks: parts.blocks,
        }
    }
}

fn reward_share(total_emission: u64, basis_points: u64) -> u64 {
    total_emission.saturating_mul(basis_points) / 10_000
}
