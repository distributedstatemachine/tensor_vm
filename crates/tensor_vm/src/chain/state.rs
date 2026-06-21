use crate::codec::primitive_type_tag;
use crate::field;
use crate::ir::TensorGraph;
use crate::jobs::{
    GraphJob, GraphReceipt, LinearTrainingStepJob, LinearTrainingStepReceipt, MatmulJob,
    PrimitiveType, TensorOpReceipt,
};
use crate::merkle::MerkleProof;
use crate::types::{Address, Hash, Signature, hash_bytes, sign, verify_signature};
use crate::verify::{
    FreivaldsParams, ValidatorAttestation, VerificationResult, row_sample_detection_probability,
};
use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_REWARD_AWAITING_INCLUSION_SORT_HEIGHT: u64 = u64::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainParams {
    pub block_time_seconds: u64,
    pub epoch_length: u64,
    pub receipt_submission_window: u64,
    pub verification_window: u64,
    pub reward_settlement_delay_epochs: u64,
    pub challenge_window_epochs: u64,
    pub proposer_reward_hold_epochs: u64,
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
    pub invalid_output_miner_slash_amount: u64,
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
    pub proposer_cooldown_blocks: u64,
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
            proposer_reward_hold_epochs: 1,
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
            invalid_output_miner_slash_amount: 25,
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
            proposer_cooldown_blocks: 0,
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
        self.base_reward_maturity_delay_blocks()
            .max(self.fraud_reward_hold_blocks())
    }

    pub fn proposer_reward_maturity_delay_blocks(&self) -> u64 {
        self.reward_maturity_delay_blocks()
            .saturating_add(self.proposer_reward_hold_blocks())
    }

    pub fn proposer_reward_hold_blocks(&self) -> u64 {
        self.proposer_reward_hold_epochs
            .saturating_mul(self.epoch_length.max(1))
    }

    fn base_reward_maturity_delay_blocks(&self) -> u64 {
        self.reward_settlement_delay_epochs
            .saturating_add(self.challenge_window_epochs)
            .max(1)
            .saturating_mul(self.epoch_length.max(1))
    }

    pub fn validator_audit_reward_hold_blocks(&self) -> u64 {
        if self.validator_audit_sample_numerator == 0 {
            0
        } else {
            self.validator_audit_window_blocks.max(1)
        }
    }

    pub fn fraud_reward_hold_blocks(&self) -> u64 {
        self.challenge_window_blocks()
            .max(self.validator_audit_reward_hold_blocks())
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
            .max(self.validator_audit_reward_hold_blocks())
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
pub struct BlockCheckTranscript {
    pub receipt_id: Hash,
    pub beacon_round: u64,
    pub beacon: Hash,
    pub parent_hash: Hash,
    pub check_seed: Hash,
    pub selected_receipt_leaf: Hash,
    pub receipt_checks_root: Hash,
    pub primitive_type: Option<PrimitiveType>,
    pub tensor_work_units: u64,
    pub estimated_block_bytes: u64,
}

impl BlockCheckTranscript {
    pub fn leaf(&self) -> Hash {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&self.beacon_round.to_le_bytes());
        encoded.extend_from_slice(&self.beacon);
        encoded.extend_from_slice(&self.parent_hash);
        encoded.extend_from_slice(&self.check_seed);
        encoded.extend_from_slice(&self.receipt_id);
        encoded.extend_from_slice(&self.selected_receipt_leaf);
        encoded.extend_from_slice(&self.receipt_checks_root);
        if let Some(primitive_type) = self.primitive_type {
            encoded.push(primitive_type_tag(primitive_type));
            encoded.extend_from_slice(&self.tensor_work_units.to_le_bytes());
            encoded.extend_from_slice(&self.estimated_block_bytes.to_le_bytes());
        }
        hash_bytes(b"tensor-vm-block-check-leaf-v1", &[&encoded])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedReceiptOpening {
    pub receipt_id: Hash,
    pub receipt_leaf: Hash,
    pub receipt_leaf_index: u64,
    pub receipt_leaf_proof: Option<MerkleProof>,
    pub check_transcript: BlockCheckTranscript,
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
pub struct RedundantSettlementDelayRecord {
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub primitive_type: PrimitiveType,
    pub observed_agreeing_miners: usize,
    pub observed_agreeing_operators: usize,
    pub required_agreement_quorum: usize,
    pub conflicting_quorum_receipts: usize,
    pub recorded_at_height: u64,
    pub reward_delay_until_height: u64,
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
    pub maturity: ReceiptRewardMaturity,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptRewardMaturity {
    AwaitingInclusion,
    AwaitingInclusionUntil(u64),
    AwaitingValidatorVrfReveal(u64),
    ClaimableAt(u64),
}

impl ReceiptRewardMaturity {
    pub fn claimable_at_height(self) -> Option<u64> {
        match self {
            Self::AwaitingInclusion => None,
            Self::AwaitingInclusionUntil(_) => None,
            Self::AwaitingValidatorVrfReveal(_) => None,
            Self::ClaimableAt(height) => Some(height),
        }
    }

    pub fn is_mature_at(self, height: u64) -> bool {
        matches!(self, Self::ClaimableAt(claimable_at_height) if claimable_at_height <= height)
    }

    pub fn hold_mature_at(self, height: u64) -> bool {
        match self {
            Self::AwaitingInclusion => false,
            Self::AwaitingInclusionUntil(hold_height)
            | Self::AwaitingValidatorVrfReveal(hold_height)
            | Self::ClaimableAt(hold_height) => hold_height <= height,
        }
    }

    pub fn delayed_until(self, height: u64) -> Self {
        match self {
            Self::AwaitingInclusion => Self::AwaitingInclusionUntil(height),
            Self::AwaitingInclusionUntil(current) => {
                Self::AwaitingInclusionUntil(current.max(height))
            }
            Self::AwaitingValidatorVrfReveal(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::ClaimableAt(current) => Self::ClaimableAt(current.max(height)),
        }
    }

    pub fn delayed_until_validator_vrf_reveal(self, height: u64) -> Self {
        match self {
            Self::AwaitingInclusion => Self::AwaitingValidatorVrfReveal(height),
            Self::AwaitingInclusionUntil(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::AwaitingValidatorVrfReveal(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::ClaimableAt(current) => Self::AwaitingValidatorVrfReveal(current.max(height)),
        }
    }

    pub fn reveal_available(self) -> Self {
        match self {
            Self::AwaitingValidatorVrfReveal(height) => Self::ClaimableAt(height),
            other => other,
        }
    }

    pub fn included_with_delay(self, height: u64) -> Self {
        match self {
            Self::AwaitingInclusion => Self::ClaimableAt(height),
            Self::AwaitingInclusionUntil(current) => Self::ClaimableAt(current.max(height)),
            Self::AwaitingValidatorVrfReveal(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::ClaimableAt(current) => Self::ClaimableAt(current.max(height)),
        }
    }

    pub fn included_with_validator_vrf_reveal_delay(self, height: u64) -> Self {
        match self {
            Self::AwaitingInclusion => Self::AwaitingValidatorVrfReveal(height),
            Self::AwaitingInclusionUntil(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::AwaitingValidatorVrfReveal(current) => {
                Self::AwaitingValidatorVrfReveal(current.max(height))
            }
            Self::ClaimableAt(current) => Self::AwaitingValidatorVrfReveal(current.max(height)),
        }
    }
}

impl PendingReceiptReward {
    pub fn awaiting_inclusion(&self) -> bool {
        matches!(
            self.maturity,
            ReceiptRewardMaturity::AwaitingInclusion
                | ReceiptRewardMaturity::AwaitingInclusionUntil(_)
        )
    }

    pub fn claimable_at_height(&self) -> Option<u64> {
        self.maturity.claimable_at_height()
    }

    pub fn is_mature_at(&self, height: u64) -> bool {
        self.maturity.is_mature_at(height)
    }

    pub fn hold_mature_at(&self, height: u64) -> bool {
        self.maturity.hold_mature_at(height)
    }

    pub fn delay_until(&mut self, height: u64) {
        self.maturity = self.maturity.delayed_until(height);
    }

    pub fn delay_until_validator_vrf_reveal(&mut self, height: u64) {
        self.maturity = self.maturity.delayed_until_validator_vrf_reveal(height);
    }

    pub fn include_with_delay(&mut self, height: u64) {
        self.maturity = self.maturity.included_with_delay(height);
    }

    pub fn include_with_validator_vrf_reveal_delay(&mut self, height: u64) {
        self.maturity = self
            .maturity
            .included_with_validator_vrf_reveal_delay(height);
    }

    pub fn mark_validator_vrf_revealed(&mut self) {
        self.maturity = self.maturity.reveal_available();
    }
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
    pub claimable_at_height: Option<u64>,
    pub awaiting_inclusion: bool,
    pub awaiting_validator_vrf_reveal: bool,
    pub voided_by_challenge: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditEconomicCalibration {
    pub detection_numerator: u64,
    pub detection_denominator: u64,
    pub detection_probability_bps: u64,
    pub slashable_bond: u64,
    pub reward_from_fraud: u64,
    pub at_risk_validator_reward_claim_count: usize,
    pub required_slashable_bond: u64,
    pub invariant_holds: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FraudPathEconomicCalibration {
    pub path: &'static str,
    pub detection_numerator: u64,
    pub detection_denominator: u64,
    pub detection_probability_bps: u64,
    pub slashable_bond: u64,
    pub reward_from_fraud: u64,
    pub at_risk_reward_claim_count: usize,
    pub required_slashable_bond: u64,
    pub invariant_holds: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FraudPathEconomicCalibrationSummary {
    pub paths: Vec<FraudPathEconomicCalibration>,
    pub path_count: usize,
    pub all_invariants_hold: bool,
    pub max_required_slashable_bond: u64,
    pub worst_path: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectionProbabilityEvidence {
    pub mechanism: &'static str,
    pub source: &'static str,
    pub sample_numerator: u64,
    pub sample_denominator: u64,
    pub detection_probability_bps: u64,
    pub false_accept_probability_bps: u64,
    pub live_subject_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectionProbabilityEvidenceSummary {
    pub mechanisms: Vec<DetectionProbabilityEvidence>,
    pub mechanism_count: usize,
    pub minimum_detection_probability_bps: u64,
    pub maximum_false_accept_probability_bps: u64,
    pub live_subject_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomnessBindingEvidence {
    pub beacon_source: &'static str,
    pub drand_round_mapping: &'static str,
    pub vrf_construction: &'static str,
    pub assignment_seed_domain: &'static str,
    pub validation_seed_commitment_domain: &'static str,
    pub validation_seed_reveal_domain: &'static str,
    pub commit_reveal_ordering: &'static str,
    pub current_block_hash_randomness_allowed: bool,
    pub receipt_anchor_count: usize,
    pub finalized_beacon_anchor_count: usize,
    pub finalized_beacon_round_mapping_count: usize,
    pub validator_vrf_seed_count: usize,
    pub receipt_bound_anchor_count: usize,
    pub consistent_anchor_count: usize,
    pub current_block_hash_anchor_count: usize,
    pub external_beacon_record_count: usize,
    pub latest_external_beacon_round: u64,
    pub validator_vrf_reveal_count: usize,
    pub all_receipt_anchors_consistent: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalRandomnessBeaconRecord {
    pub source_id: String,
    pub beacon_round: u64,
    pub randomness: Hash,
    pub proof_hash: Hash,
    pub observed_at_height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptRandomnessAnchor {
    pub receipt_id: Hash,
    pub beacon_round: u64,
    pub finalized_randomness: Hash,
    pub assignment_seed: Hash,
    pub validation_seed_commitment: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorVrfRevealRecord {
    pub reveal_id: Hash,
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub validator: Address,
    pub beacon_round: u64,
    pub validation_round: u64,
    pub vrf_output: Hash,
    pub proof_hash: Hash,
    pub signature: Signature,
    pub observed_at_height: u64,
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
pub struct InvalidOutputSlashRecord {
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
    pub auditor: Address,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditAppeal {
    pub audit_id: Hash,
    pub validator: Address,
    pub reason: String,
    pub signature: Signature,
}

impl ValidatorAuditAppeal {
    pub fn new(audit_id: Hash, validator: Address, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let message = Self::message_hash(&audit_id, &validator, &reason);
        Self {
            audit_id,
            validator,
            reason,
            signature: sign(&validator, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        verify_signature(
            &self.validator,
            &Self::message_hash(&self.audit_id, &self.validator, &self.reason),
            &self.signature,
        )
    }

    fn message_hash(audit_id: &Hash, validator: &Address, reason: &str) -> Hash {
        hash_bytes(
            b"tensor-vm-validator-audit-appeal-v1",
            &[audit_id, validator, reason.as_bytes()],
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorAuditAppealRecord {
    pub audit_id: Hash,
    pub receipt_id: Hash,
    pub validator: Address,
    pub auditor: Address,
    pub slash_amount: u64,
    pub appealed_at_height: u64,
    pub deadline_height: u64,
    pub reason: String,
    pub signature: Signature,
    pub resolved_at_height: Option<u64>,
    pub resolution: Option<ValidatorAuditAppealResolution>,
    pub stake_refunded_amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatorAuditAppealResolution {
    UpholdSlash,
    ReverseRewardVoid,
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

    pub(in crate::chain) fn debit_treasury(&mut self, amount: u64) -> u64 {
        let debited = self.treasury.min(amount);
        self.treasury = self.treasury.saturating_sub(debited);
        debited
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
    pub(in crate::chain) external_randomness_beacons: BTreeMap<u64, ExternalRandomnessBeaconRecord>,
    pub(in crate::chain) genesis_beacon_round: u64,
    pub(in crate::chain) genesis_randomness: Hash,
    pub(in crate::chain) accounts: BTreeMap<Address, AccountState>,
    pub(in crate::chain) miners: BTreeMap<Address, MinerState>,
    pub(in crate::chain) validators: BTreeMap<Address, ValidatorState>,
    pub(in crate::chain) jobs: BTreeMap<Hash, JobState>,
    pub(in crate::chain) program_bodies: BTreeMap<Hash, Vec<u8>>,
    pub(in crate::chain) receipts: BTreeMap<Hash, ReceiptState>,
    pub(in crate::chain) receipt_randomness_anchors: BTreeMap<Hash, ReceiptRandomnessAnchor>,
    pub(in crate::chain) validator_vrf_reveals: BTreeMap<Hash, ValidatorVrfRevealRecord>,
    pub(in crate::chain) attestations: BTreeMap<Hash, Vec<ValidatorAttestation>>,
    pub(in crate::chain) block_votes: BTreeMap<Hash, Vec<BlockVote>>,
    pub(in crate::chain) finalized_blocks: BTreeSet<Hash>,
    pub(in crate::chain) data_unavailable_receipts: BTreeSet<Hash>,
    pub(in crate::chain) data_unavailability_slashes: BTreeMap<Hash, DataUnavailabilitySlashRecord>,
    pub(in crate::chain) invalid_output_slashes: BTreeMap<Hash, InvalidOutputSlashRecord>,
    pub(in crate::chain) validator_audit_assignments: BTreeMap<Hash, ValidatorAuditAssignment>,
    pub(in crate::chain) validator_audit_results: BTreeMap<Hash, ValidatorAuditResult>,
    pub(in crate::chain) validator_audit_slashes: BTreeMap<Hash, ValidatorAuditSlashRecord>,
    pub(in crate::chain) validator_audit_appeals: BTreeMap<Hash, ValidatorAuditAppealRecord>,
    pub(in crate::chain) settled_receipts: BTreeSet<Hash>,
    pub(in crate::chain) redundant_settlement_delays:
        BTreeMap<Hash, RedundantSettlementDelayRecord>,
    pub(in crate::chain) included_receipts: BTreeSet<Hash>,
    pub(in crate::chain) block_selected_receipts: BTreeMap<Hash, Vec<Hash>>,
    pub(in crate::chain) block_check_challenges: BTreeMap<Hash, BlockCheckChallengeRecord>,
    pub(in crate::chain) challenged_receipts: BTreeSet<Hash>,
    pub(in crate::chain) proposer_penalty_until: BTreeMap<Address, u64>,
    pub(in crate::chain) proposer_cadence_last_proposed: BTreeMap<Address, u64>,
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
    pub external_randomness_beacons: BTreeMap<u64, ExternalRandomnessBeaconRecord>,
    pub genesis_beacon_round: u64,
    pub genesis_randomness: Hash,
    pub accounts: BTreeMap<Address, AccountState>,
    pub miners: BTreeMap<Address, MinerState>,
    pub validators: BTreeMap<Address, ValidatorState>,
    pub jobs: BTreeMap<Hash, JobState>,
    pub program_bodies: BTreeMap<Hash, Vec<u8>>,
    pub receipts: BTreeMap<Hash, ReceiptState>,
    pub receipt_randomness_anchors: BTreeMap<Hash, ReceiptRandomnessAnchor>,
    pub validator_vrf_reveals: BTreeMap<Hash, ValidatorVrfRevealRecord>,
    pub attestations: BTreeMap<Hash, Vec<ValidatorAttestation>>,
    pub block_votes: BTreeMap<Hash, Vec<BlockVote>>,
    pub finalized_blocks: BTreeSet<Hash>,
    pub data_unavailable_receipts: BTreeSet<Hash>,
    pub data_unavailability_slashes: BTreeMap<Hash, DataUnavailabilitySlashRecord>,
    pub invalid_output_slashes: BTreeMap<Hash, InvalidOutputSlashRecord>,
    pub validator_audit_assignments: BTreeMap<Hash, ValidatorAuditAssignment>,
    pub validator_audit_results: BTreeMap<Hash, ValidatorAuditResult>,
    pub validator_audit_slashes: BTreeMap<Hash, ValidatorAuditSlashRecord>,
    pub validator_audit_appeals: BTreeMap<Hash, ValidatorAuditAppealRecord>,
    pub settled_receipts: BTreeSet<Hash>,
    pub redundant_settlement_delays: BTreeMap<Hash, RedundantSettlementDelayRecord>,
    pub included_receipts: BTreeSet<Hash>,
    pub block_selected_receipts: BTreeMap<Hash, Vec<Hash>>,
    pub block_check_challenges: BTreeMap<Hash, BlockCheckChallengeRecord>,
    pub challenged_receipts: BTreeSet<Hash>,
    pub proposer_penalty_until: BTreeMap<Address, u64>,
    pub proposer_cadence_last_proposed: BTreeMap<Address, u64>,
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
            external_randomness_beacons: parts.external_randomness_beacons,
            genesis_beacon_round: parts.genesis_beacon_round,
            genesis_randomness: parts.genesis_randomness,
            accounts: parts.accounts,
            miners: parts.miners,
            validators: parts.validators,
            jobs: parts.jobs,
            program_bodies: parts.program_bodies,
            receipts: parts.receipts,
            receipt_randomness_anchors: parts.receipt_randomness_anchors,
            validator_vrf_reveals: parts.validator_vrf_reveals,
            attestations: parts.attestations,
            block_votes: parts.block_votes,
            finalized_blocks: parts.finalized_blocks,
            data_unavailable_receipts: parts.data_unavailable_receipts,
            data_unavailability_slashes: parts.data_unavailability_slashes,
            invalid_output_slashes: parts.invalid_output_slashes,
            validator_audit_assignments: parts.validator_audit_assignments,
            validator_audit_results: parts.validator_audit_results,
            validator_audit_slashes: parts.validator_audit_slashes,
            validator_audit_appeals: parts.validator_audit_appeals,
            settled_receipts: parts.settled_receipts,
            redundant_settlement_delays: parts.redundant_settlement_delays,
            included_receipts: parts.included_receipts,
            block_selected_receipts: parts.block_selected_receipts,
            block_check_challenges: parts.block_check_challenges,
            challenged_receipts: parts.challenged_receipts,
            proposer_penalty_until: parts.proposer_penalty_until,
            proposer_cadence_last_proposed: parts.proposer_cadence_last_proposed,
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

    pub fn external_randomness_beacons(&self) -> &BTreeMap<u64, ExternalRandomnessBeaconRecord> {
        &self.external_randomness_beacons
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

    pub fn validator_vrf_reveals(&self) -> &BTreeMap<Hash, ValidatorVrfRevealRecord> {
        &self.validator_vrf_reveals
    }

    pub fn randomness_binding_evidence(&self) -> RandomnessBindingEvidence {
        let mut finalized_beacon_anchor_count = 0_usize;
        let mut finalized_beacon_round_mapping_count = 0_usize;
        let mut validator_vrf_seed_count = 0_usize;
        let mut receipt_bound_anchor_count = 0_usize;
        let mut consistent_anchor_count = 0_usize;
        for (receipt_id, anchor) in &self.receipt_randomness_anchors {
            if anchor.finalized_randomness != [0; 32] {
                finalized_beacon_anchor_count += 1;
            }
            if anchor.finalized_randomness != [0; 32]
                && anchor.beacon_round <= self.finalized_beacon_round
            {
                finalized_beacon_round_mapping_count += 1;
            }
            if anchor.receipt_id == *receipt_id {
                receipt_bound_anchor_count += 1;
            }
            if anchor.receipt_id == *receipt_id
                && anchor.assignment_seed
                    == super::validation::assignment_seed(
                        anchor.beacon_round,
                        &anchor.finalized_randomness,
                        receipt_id,
                    )
                && anchor.validation_seed_commitment
                    == super::validation::validation_seed_commitment(
                        anchor.beacon_round,
                        &anchor.finalized_randomness,
                        receipt_id,
                    )
            {
                consistent_anchor_count += 1;
                if self.receipts.contains_key(receipt_id) {
                    validator_vrf_seed_count =
                        validator_vrf_seed_count.saturating_add(self.validators.len());
                }
            }
        }
        let receipt_anchor_count = self.receipt_randomness_anchors.len();
        let latest_external_beacon_round = self
            .external_randomness_beacons
            .keys()
            .next_back()
            .copied()
            .unwrap_or_default();
        RandomnessBindingEvidence {
            beacon_source: super::validation::RANDOMNESS_BEACON_SOURCE,
            drand_round_mapping: super::validation::RANDOMNESS_DRAND_ROUND_MAPPING,
            vrf_construction: super::validation::RANDOMNESS_VRF_CONSTRUCTION,
            assignment_seed_domain: super::validation::ASSIGNMENT_SEED_DOMAIN,
            validation_seed_commitment_domain: super::validation::VALIDATION_SEED_COMMITMENT_DOMAIN,
            validation_seed_reveal_domain: super::validation::VALIDATION_SEED_REVEAL_DOMAIN,
            commit_reveal_ordering: "commit=receipt_id+finalized_beacon_round;reveal=validator+job+round",
            current_block_hash_randomness_allowed: false,
            receipt_anchor_count,
            finalized_beacon_anchor_count,
            finalized_beacon_round_mapping_count,
            validator_vrf_seed_count,
            receipt_bound_anchor_count,
            consistent_anchor_count,
            current_block_hash_anchor_count: 0,
            external_beacon_record_count: self.external_randomness_beacons.len(),
            latest_external_beacon_round,
            validator_vrf_reveal_count: self.validator_vrf_reveals.len(),
            all_receipt_anchors_consistent: receipt_anchor_count == consistent_anchor_count,
        }
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

    pub fn invalid_output_slashes(&self) -> &BTreeMap<Hash, InvalidOutputSlashRecord> {
        &self.invalid_output_slashes
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

    pub fn validator_audit_appeals(&self) -> &BTreeMap<Hash, ValidatorAuditAppealRecord> {
        &self.validator_audit_appeals
    }

    pub fn settled_receipts(&self) -> &BTreeSet<Hash> {
        &self.settled_receipts
    }

    pub fn redundant_settlement_delays(&self) -> &BTreeMap<Hash, RedundantSettlementDelayRecord> {
        &self.redundant_settlement_delays
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

    pub fn proposer_cadence_last_proposed(&self) -> &BTreeMap<Address, u64> {
        &self.proposer_cadence_last_proposed
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
                claimable_at_height: Some(reward.claimable_at_height),
                awaiting_inclusion: false,
                awaiting_validator_vrf_reveal: false,
                voided_by_challenge: reward.voided_by_challenge,
            });
        }
        for (claim_id, reward) in &self.pending_receipt_rewards {
            let (claimable_at_height, awaiting_inclusion, awaiting_validator_vrf_reveal) =
                match reward.maturity {
                    ReceiptRewardMaturity::AwaitingInclusion => (None, true, false),
                    ReceiptRewardMaturity::AwaitingInclusionUntil(height) => {
                        (Some(height), true, false)
                    }
                    ReceiptRewardMaturity::AwaitingValidatorVrfReveal(_) => (None, false, true),
                    ReceiptRewardMaturity::ClaimableAt(height) => (Some(height), false, false),
                };
            let claimable_at_height = match reward.maturity {
                ReceiptRewardMaturity::AwaitingValidatorVrfReveal(height) => Some(height),
                _ => claimable_at_height,
            };
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
                claimable_at_height,
                awaiting_inclusion,
                awaiting_validator_vrf_reveal,
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
                claimable_at_height: Some(reward.claimable_at_height),
                awaiting_inclusion: false,
                awaiting_validator_vrf_reveal: false,
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
                claimable_at_height: Some(reward.claimable_at_height),
                awaiting_inclusion: false,
                awaiting_validator_vrf_reveal: false,
                voided_by_challenge: false,
            });
        }
        claims.sort_by(|left, right| {
            left.claimable_at_height
                .unwrap_or(RECEIPT_REWARD_AWAITING_INCLUSION_SORT_HEIGHT)
                .cmp(
                    &right
                        .claimable_at_height
                        .unwrap_or(RECEIPT_REWARD_AWAITING_INCLUSION_SORT_HEIGHT),
                )
                .then_with(|| left.ledger.cmp(&right.ledger))
                .then_with(|| left.claim_id.cmp(&right.claim_id))
        });
        claims
    }

    pub fn validator_audit_economic_calibration(
        &self,
        params: &ChainParams,
    ) -> ValidatorAuditEconomicCalibration {
        let detection_denominator = params.validator_audit_sample_denominator.max(1);
        let detection_numerator = params
            .validator_audit_sample_numerator
            .min(detection_denominator);
        let at_risk_validator_rewards = self
            .pending_receipt_rewards
            .values()
            .filter(|reward| {
                reward.kind == ReceiptRewardKind::Validator && !reward.voided_by_challenge
            })
            .collect::<Vec<_>>();
        let reward_from_fraud = at_risk_validator_rewards
            .iter()
            .filter(|reward| reward.is_mature_at(self.height))
            .map(|reward| reward.amount)
            .max()
            .unwrap_or_default();
        let required_slashable_bond = required_slashable_bond(
            reward_from_fraud,
            detection_numerator,
            detection_denominator,
        );
        let invariant_holds = reward_from_fraud == 0
            || (detection_numerator > 0
                && (params.validator_audit_slash_amount as u128)
                    .saturating_mul(detection_numerator as u128)
                    > (reward_from_fraud as u128).saturating_mul(detection_denominator as u128));
        ValidatorAuditEconomicCalibration {
            detection_numerator,
            detection_denominator,
            detection_probability_bps: ((detection_numerator as u128) * 10_000
                / detection_denominator as u128)
                .min(u64::MAX as u128) as u64,
            slashable_bond: params.validator_audit_slash_amount,
            reward_from_fraud,
            at_risk_validator_reward_claim_count: at_risk_validator_rewards.len(),
            required_slashable_bond,
            invariant_holds,
        }
    }

    pub fn fraud_path_economic_calibration(
        &self,
        params: &ChainParams,
    ) -> FraudPathEconomicCalibrationSummary {
        let validator_audit = self.validator_audit_economic_calibration(params);
        let mut paths = vec![FraudPathEconomicCalibration {
            path: "validator_audit",
            detection_numerator: validator_audit.detection_numerator,
            detection_denominator: validator_audit.detection_denominator,
            detection_probability_bps: validator_audit.detection_probability_bps,
            slashable_bond: validator_audit.slashable_bond,
            reward_from_fraud: validator_audit.reward_from_fraud,
            at_risk_reward_claim_count: validator_audit.at_risk_validator_reward_claim_count,
            required_slashable_bond: validator_audit.required_slashable_bond,
            invariant_holds: validator_audit.invariant_holds,
        }];

        let at_risk_miner_rewards = self
            .pending_receipt_rewards
            .values()
            .filter(|reward| reward.kind == ReceiptRewardKind::Miner && !reward.voided_by_challenge)
            .collect::<Vec<_>>();
        paths.push(fraud_path_calibration(
            "data_unavailability",
            1,
            1,
            params.data_unavailability_miner_slash_amount,
            at_risk_miner_rewards
                .iter()
                .filter(|reward| reward.is_mature_at(self.height))
                .map(|reward| reward.amount)
                .max()
                .unwrap_or_default(),
            at_risk_miner_rewards.len(),
        ));
        paths.push(fraud_path_calibration(
            "invalid_output",
            1,
            1,
            params.invalid_output_miner_slash_amount,
            at_risk_miner_rewards
                .iter()
                .filter(|reward| reward.is_mature_at(self.height))
                .map(|reward| reward.amount)
                .max()
                .unwrap_or_default(),
            at_risk_miner_rewards.len(),
        ));

        let at_risk_proposer_rewards = self
            .pending_proposer_rewards
            .values()
            .filter(|reward| !reward.voided_by_challenge)
            .collect::<Vec<_>>();
        let slashable_bond = at_risk_proposer_rewards
            .iter()
            .map(|reward| reward.amount)
            .max()
            .unwrap_or_default();
        let reward_from_fraud = at_risk_proposer_rewards
            .iter()
            .filter(|reward| reward.claimable_at_height <= self.height)
            .map(|reward| reward.amount)
            .max()
            .unwrap_or_default();
        paths.push(fraud_path_calibration(
            "block_check",
            1,
            1,
            slashable_bond,
            reward_from_fraud,
            at_risk_proposer_rewards.len(),
        ));

        let path_count = paths.len();
        let all_invariants_hold = paths.iter().all(|path| path.invariant_holds);
        let worst = paths
            .iter()
            .max_by_key(|path| path.required_slashable_bond)
            .expect("fraud path calibration must include at least one path");
        FraudPathEconomicCalibrationSummary {
            path_count,
            all_invariants_hold,
            max_required_slashable_bond: worst.required_slashable_bond,
            worst_path: worst.path,
            paths,
        }
    }

    pub fn detection_probability_evidence(
        &self,
        params: &ChainParams,
    ) -> DetectionProbabilityEvidenceSummary {
        let tensor_job_rows = self
            .jobs
            .values()
            .filter_map(|job| match job {
                JobState::TensorOp(job) => Some(job.m),
                _ => None,
            })
            .collect::<Vec<_>>();
        let tensor_job_count = tensor_job_rows.len();
        let max_tensor_rows = tensor_job_rows.iter().copied().max().unwrap_or_default();
        let row_sample_bps = probability_bps(row_sample_detection_probability(
            max_tensor_rows,
            usize::from(max_tensor_rows > 0),
            params.freivalds.audit_rows.min(max_tensor_rows),
        ));
        let freivalds_false_accept_bps = freivalds_false_accept_bps(params.freivalds.full_rounds);
        let freivalds_detection_bps = 10_000_u64.saturating_sub(freivalds_false_accept_bps);
        let linear_job_count = self
            .jobs
            .values()
            .filter(|job| matches!(job, JobState::LinearTrainingStep(_)))
            .count();
        let graph_job_count = self
            .jobs
            .values()
            .filter(|job| matches!(job, JobState::GraphExecution(_)))
            .count();
        let audit_denominator = params.validator_audit_sample_denominator.max(1);
        let audit_numerator = params
            .validator_audit_sample_numerator
            .min(audit_denominator);
        let data_availability_probability_bps =
            replication_availability_bps(params.replication_factor, 9_500);
        let fraud_calibration = self.fraud_path_economic_calibration(params);
        let block_check_subjects = self
            .pending_proposer_rewards
            .values()
            .filter(|reward| !reward.voided_by_challenge)
            .count()
            .saturating_add(self.block_check_challenges.len());
        let mut mechanisms = vec![
            DetectionProbabilityEvidence {
                mechanism: "full_freivalds",
                source: "params.freivalds.full_rounds+field_modulus",
                sample_numerator: params.freivalds.full_rounds.max(1) as u64,
                sample_denominator: field::MODULUS,
                detection_probability_bps: freivalds_detection_bps,
                false_accept_probability_bps: freivalds_false_accept_bps,
                live_subject_count: tensor_job_count,
            },
            DetectionProbabilityEvidence {
                mechanism: "row_sampling_sparse_audit",
                source: "live_tensorop_job_rows+params.freivalds.audit_rows",
                sample_numerator: params.freivalds.audit_rows.min(max_tensor_rows) as u64,
                sample_denominator: max_tensor_rows as u64,
                detection_probability_bps: row_sample_bps,
                false_accept_probability_bps: 10_000_u64.saturating_sub(row_sample_bps),
                live_subject_count: tensor_job_count,
            },
            DetectionProbabilityEvidence {
                mechanism: "linear_random_linear",
                source: "field_modulus+linear_training_jobs",
                sample_numerator: 1,
                sample_denominator: field::MODULUS,
                detection_probability_bps: freivalds_detection_bps,
                false_accept_probability_bps: freivalds_false_accept_bps,
                live_subject_count: linear_job_count,
            },
            DetectionProbabilityEvidence {
                mechanism: "graph_exact_replay",
                source: "registered_graph_jobs",
                sample_numerator: 1,
                sample_denominator: 1,
                detection_probability_bps: 10_000,
                false_accept_probability_bps: 0,
                live_subject_count: graph_job_count,
            },
            DetectionProbabilityEvidence {
                mechanism: "data_availability_replication",
                source: "params.replication_factor+95pct_per_replica_target",
                sample_numerator: params.replication_factor as u64,
                sample_denominator: 10_000,
                detection_probability_bps: data_availability_probability_bps,
                false_accept_probability_bps: 10_000_u64
                    .saturating_sub(data_availability_probability_bps),
                live_subject_count: self.receipts.len(),
            },
            DetectionProbabilityEvidence {
                mechanism: "validator_audit",
                source: "params.validator_audit_sample_rate",
                sample_numerator: audit_numerator,
                sample_denominator: audit_denominator,
                detection_probability_bps: bps_from_ratio(audit_numerator, audit_denominator),
                false_accept_probability_bps: 10_000_u64
                    .saturating_sub(bps_from_ratio(audit_numerator, audit_denominator)),
                live_subject_count: self.validator_audit_assignments.len(),
            },
            DetectionProbabilityEvidence {
                mechanism: "block_check",
                source: "implemented_block_check_challenge_path",
                sample_numerator: 1,
                sample_denominator: 1,
                detection_probability_bps: 10_000,
                false_accept_probability_bps: 0,
                live_subject_count: block_check_subjects,
            },
        ];
        for path in fraud_calibration.paths {
            if path.path == "validator_audit" || path.path == "block_check" {
                continue;
            }
            mechanisms.push(DetectionProbabilityEvidence {
                mechanism: path.path,
                source: "fraud_path_economic_calibration",
                sample_numerator: path.detection_numerator,
                sample_denominator: path.detection_denominator,
                detection_probability_bps: path.detection_probability_bps,
                false_accept_probability_bps: 10_000_u64
                    .saturating_sub(path.detection_probability_bps),
                live_subject_count: path.at_risk_reward_claim_count,
            });
        }
        let mechanism_count = mechanisms.len();
        let minimum_detection_probability_bps = mechanisms
            .iter()
            .map(|evidence| evidence.detection_probability_bps)
            .min()
            .unwrap_or_default();
        let maximum_false_accept_probability_bps = mechanisms
            .iter()
            .map(|evidence| evidence.false_accept_probability_bps)
            .max()
            .unwrap_or_default();
        let live_subject_count = mechanisms
            .iter()
            .map(|evidence| evidence.live_subject_count)
            .sum();
        DetectionProbabilityEvidenceSummary {
            mechanisms,
            mechanism_count,
            minimum_detection_probability_bps,
            maximum_false_accept_probability_bps,
            live_subject_count,
        }
    }

    pub fn model_states(&self) -> &BTreeMap<Hash, ModelState> {
        &self.model_states
    }

    pub fn rewards(&self) -> &RewardState {
        &self.rewards
    }
}

fn bps_from_ratio(numerator: u64, denominator: u64) -> u64 {
    let denominator = denominator.max(1);
    ((numerator.min(denominator) as u128) * 10_000 / denominator as u128).min(u64::MAX as u128)
        as u64
}

fn probability_bps(probability: f64) -> u64 {
    (probability.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn freivalds_false_accept_bps(rounds: usize) -> u64 {
    let mut denominator = 1_u128;
    for _ in 0..rounds.max(1) {
        denominator = denominator.saturating_mul(field::MODULUS as u128);
    }
    (10_000_u128 / denominator).min(u64::MAX as u128) as u64
}

fn replication_availability_bps(replicas: usize, per_replica_availability_bps: u64) -> u64 {
    if replicas == 0 {
        return 0;
    }
    let unavailable_bps =
        10_000_u128.saturating_sub(per_replica_availability_bps.min(10_000) as u128);
    let mut all_unavailable_scaled = 10_000_u128;
    for _ in 0..replicas {
        all_unavailable_scaled = all_unavailable_scaled.saturating_mul(unavailable_bps) / 10_000;
    }
    10_000_u64.saturating_sub(all_unavailable_scaled.min(10_000) as u64)
}

fn required_slashable_bond(
    reward_from_fraud: u64,
    detection_numerator: u64,
    detection_denominator: u64,
) -> u64 {
    if reward_from_fraud == 0 {
        return 0;
    }
    if detection_numerator == 0 {
        return u64::MAX;
    }
    let quotient = (reward_from_fraud as u128).saturating_mul(detection_denominator as u128)
        / detection_numerator as u128;
    quotient.saturating_add(1).min(u64::MAX as u128) as u64
}

fn fraud_path_calibration(
    path: &'static str,
    detection_numerator: u64,
    detection_denominator: u64,
    slashable_bond: u64,
    reward_from_fraud: u64,
    at_risk_reward_claim_count: usize,
) -> FraudPathEconomicCalibration {
    let detection_denominator = detection_denominator.max(1);
    let detection_numerator = detection_numerator.min(detection_denominator);
    let required_slashable_bond = required_slashable_bond(
        reward_from_fraud,
        detection_numerator,
        detection_denominator,
    );
    let invariant_holds = reward_from_fraud == 0
        || (detection_numerator > 0
            && (slashable_bond as u128).saturating_mul(detection_numerator as u128)
                > (reward_from_fraud as u128).saturating_mul(detection_denominator as u128));
    FraudPathEconomicCalibration {
        path,
        detection_numerator,
        detection_denominator,
        detection_probability_bps: ((detection_numerator as u128) * 10_000
            / detection_denominator as u128)
            .min(u64::MAX as u128) as u64,
        slashable_bond,
        reward_from_fraud,
        at_risk_reward_claim_count,
        required_slashable_bond,
        invariant_holds,
    }
}

#[derive(Clone, Debug)]
pub struct Chain {
    pub(crate) params: ChainParams,
    pub(crate) state: ChainState,
    pub(crate) blocks: Vec<TensorBlock>,
    pub(crate) block_parent_states: BTreeMap<Hash, ChainState>,
    pub(crate) side_branch_blocks: BTreeMap<Hash, TensorBlock>,
    pub(crate) side_branch_child_states: BTreeMap<Hash, ChainState>,
    pub(crate) observed_invalid_blocks: BTreeMap<Hash, TensorBlock>,
}

impl PartialEq for Chain {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params
            && self.state == other.state
            && self.blocks == other.blocks
            && self.block_parent_states == other.block_parent_states
            && self.side_branch_blocks == other.side_branch_blocks
            && self.side_branch_child_states == other.side_branch_child_states
    }
}

impl Eq for Chain {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChainParts {
    pub params: ChainParams,
    pub state: ChainState,
    pub blocks: Vec<TensorBlock>,
    pub block_parent_states: BTreeMap<Hash, ChainState>,
    pub side_branch_blocks: BTreeMap<Hash, TensorBlock>,
    pub side_branch_child_states: BTreeMap<Hash, ChainState>,
}

impl Chain {
    pub(crate) fn from_parts(parts: ChainParts) -> Self {
        Self {
            params: parts.params,
            state: parts.state,
            blocks: parts.blocks,
            block_parent_states: parts.block_parent_states,
            side_branch_blocks: parts.side_branch_blocks,
            side_branch_child_states: parts.side_branch_child_states,
            observed_invalid_blocks: BTreeMap::new(),
        }
    }

    pub(crate) fn set_block_selected_receipts_for_admission(
        &mut self,
        block_hash: Hash,
        selected_receipts: Vec<Hash>,
    ) {
        self.state
            .block_selected_receipts
            .insert(block_hash, selected_receipts);
    }

    pub(crate) fn set_block_parent_state_for_admission(
        &mut self,
        block_hash: Hash,
        parent_state: ChainState,
    ) {
        self.block_parent_states.insert(block_hash, parent_state);
    }

    pub(crate) fn block_parent_state_for_payload(&self, block_hash: &Hash) -> Option<&ChainState> {
        self.block_parent_states.get(block_hash)
    }
}

fn reward_share(total_emission: u64, basis_points: u64) -> u64 {
    total_emission.saturating_mul(basis_points) / 10_000
}
