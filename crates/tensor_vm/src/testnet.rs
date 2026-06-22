use crate::hash::hex;
use crate::profile::ChainProfile;
#[cfg(test)]
use crate::types::address;
use crate::types::{Hash, Signature, hash_bytes};
#[cfg(test)]
use crate::{ChainParams, JobScheduler};
#[cfg(test)]
use libp2p::Multiaddr;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

mod local_harness;
mod public_evidence_bundle;
mod public_evidence_crypto;
mod public_evidence_manifest;
mod public_evidence_publication;
mod public_manifest_fields;
mod public_network_runtime;
mod public_node_evidence;
mod public_operators;
mod public_preflight_manifest;
mod public_preflight_plan;
mod public_run_evidence;
mod public_services;
mod public_urls;

#[cfg(test)]
use local_harness::local_libp2p_multiaddr_has_tcp_node_path;
pub use local_harness::{LocalParticipantEndpoint, LocalTestnet};
#[cfg(test)]
use public_evidence_crypto::PublicNetworkRuntimeObservationDetails;
#[cfg(test)]
use public_evidence_crypto::deterministic_public_network_peer_id;
#[cfg(test)]
use public_evidence_crypto::public_evidence_supporting_artifact_uri;
pub use public_evidence_crypto::{
    PublicEvidenceRecordKind, sign_public_evidence_artifact, sign_public_evidence_record,
    sign_public_run_window,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use public_evidence_crypto::{
    aggregate_public_evidence_record_roots, public_network_runtime_observations_for_run,
};
pub use public_evidence_manifest::parse_public_testnet_evidence_manifest;
pub use public_evidence_publication::{
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceSupportingArtifact,
};
#[cfg(test)]
use public_manifest_fields::parse_hash_hex;
pub use public_network_runtime::{PublicNetworkRuntimeEvidence, PublicNetworkRuntimeObservation};
pub use public_node_evidence::{
    PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation,
};
#[cfg(test)]
use public_operators::match_public_operator_address;
pub use public_preflight_manifest::parse_public_testnet_preflight_manifest;
use public_services::public_service_kinds;
pub use public_services::{
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind,
};
pub(crate) use public_urls::public_network_runtime_multiaddr_is_external;
#[cfg(test)]
use public_urls::{
    public_evidence_uri_is_external, public_host_is_external, public_https_authorities_match,
    public_https_host, public_https_path,
};

pub const PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-evidence-v1";
pub const PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-preflight-v1";
pub const PUBLIC_SERVICE_MIN_CONTENT_BYTES: u64 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestnetConfig {
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
}

impl Default for TestnetConfig {
    fn default() -> Self {
        Self {
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
        }
    }
}

impl TestnetConfig {
    pub fn from_profile(profile: &ChainProfile) -> Self {
        Self {
            miner_count: profile.miner_count,
            validator_count: profile.validator_count,
            miner_stake: profile.miner_stake,
            validator_stake: profile.validator_stake,
            faucet_balance: profile.faucet_balance,
            faucet_drip: profile.faucet_drip,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetCriteria {
    pub min_miners: usize,
    pub min_validators: usize,
    pub duration_days: u64,
    pub min_finality_rate_bps: u64,
    pub min_data_availability_bps: u64,
    pub min_invalid_work_rejections: u64,
    pub min_reward_settlement_records: u64,
}

impl Default for PublicTestnetCriteria {
    fn default() -> Self {
        Self {
            min_miners: 10,
            min_validators: 5,
            duration_days: 7,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicDeploymentServicePlan {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
    pub content_url: String,
    pub content_path: String,
    pub auth_enabled: bool,
    pub rate_limit_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetPreflightPlan {
    pub config: TestnetConfig,
    pub criteria: PublicTestnetCriteria,
    pub cuda_kernels_available: bool,
    pub cuda_ready_miner_count: usize,
    pub libp2p_ready_node_count: usize,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicDeploymentServicePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetPreflightReport {
    pub miner_count: usize,
    pub validator_count: usize,
    pub required_blocks: u64,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_positive_stakes: bool,
    pub has_funded_faucet: bool,
    pub has_cuda_kernels_available: bool,
    pub cuda_ready_miner_count: usize,
    pub has_cuda_ready_miners: bool,
    pub libp2p_ready_node_count: usize,
    pub has_libp2p_ready_nodes: bool,
    pub has_production_libp2p_runtime: bool,
    pub has_rpc_service_plan: bool,
    pub has_explorer_service_plan: bool,
    pub has_faucet_service_plan: bool,
    pub has_telemetry_service_plan: bool,
    pub has_public_service_content_plan: bool,
    pub has_public_service_plan: bool,
    pub local_shape_ready: bool,
    pub deployment_plan_ready: bool,
    pub can_start_public_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidence {
    pub miner_count: usize,
    pub validator_count: usize,
    pub run_started_at_unix_seconds: u64,
    pub run_ended_at_unix_seconds: u64,
    pub observed_duration_seconds: u64,
    pub required_duration_seconds: u64,
    pub observed_blocks: u64,
    pub required_blocks: u64,
    pub finality_rate_bps: u64,
    pub data_availability_bps: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub invalid_work_rejection_rate_bps: u64,
    pub reward_settlement_records: u64,
    pub external_operator_evidence: bool,
    pub has_production_libp2p_runtime: bool,
    pub has_deployed_rpc_service: bool,
    pub has_deployed_explorer_service: bool,
    pub has_deployed_faucet_service: bool,
    pub has_deployed_telemetry_service: bool,
    pub has_deployed_public_service_content: bool,
    pub has_deployed_public_services: bool,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_required_run_duration: bool,
    pub has_required_block_count: bool,
    pub has_required_finality: bool,
    pub has_required_data_availability: bool,
    pub has_invalid_work_rejection_evidence: bool,
    pub has_reward_settlement_records: bool,
    pub cuda_verified_miner_count: u64,
    pub has_cuda_verified_miners: bool,
    pub cuda_graph_execution_receipts: u64,
    pub has_cuda_graph_execution_evidence: bool,
    pub validator_vrf_lifecycle_records: u64,
    pub has_validator_vrf_lifecycle_evidence: bool,
    pub public_criterion_met: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetRunEvidence {
    pub nodes: Vec<PublicNodeEvidence>,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicServiceEvidence>,
    pub service_content: Vec<PublicServiceContentEvidence>,
    pub run_started_at_unix_seconds: u64,
    pub run_ended_at_unix_seconds: u64,
    pub observed_blocks: u64,
    pub finalized_blocks: u64,
    pub checked_receipts: u64,
    pub available_receipts: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub reward_settlement_records: u64,
    pub cuda_verified_miner_count: u64,
    pub cuda_graph_execution_receipts: u64,
    pub validator_vrf_lifecycle_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceRecordSummaries {
    pub block_history_records: u64,
    pub block_history_root: Hash,
    pub finality_history_records: u64,
    pub finality_history_root: Hash,
    pub operator_identity_attestation_records: u64,
    pub network_runtime_observation_records: u64,
    pub network_runtime_observation_root: Hash,
    pub randomness_beacon_records: u64,
    pub randomness_beacon_root: Hash,
    pub data_availability_measurement_records: u64,
    pub data_availability_measurement_root: Hash,
    pub invalid_work_rejection_records: u64,
    pub invalid_work_rejection_root: Hash,
    pub reward_settlement_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundle {
    pub run: PublicTestnetRunEvidence,
    pub publication: PublicEvidencePublication,
    pub auditor_records: Vec<PublicEvidenceAuditorRecord>,
    pub supporting_artifacts: Vec<PublicEvidenceSupportingArtifact>,
    pub run_window_signature: Signature,
    pub block_history_records: u64,
    pub block_history_root: Hash,
    pub block_history_signature: Signature,
    pub block_history_raw_records: Vec<PublicBlockHistoryRecord>,
    pub finality_history_records: u64,
    pub finality_history_root: Hash,
    pub finality_history_signature: Signature,
    pub finality_history_raw_records: Vec<PublicFinalityHistoryRecord>,
    pub operator_identity_attestation_records: u64,
    pub operator_identity_attestations: Vec<PublicOperatorIdentityAttestation>,
    pub network_runtime_observations: Vec<PublicNetworkRuntimeObservation>,
    pub network_runtime_observation_records: u64,
    pub network_runtime_observation_root: Hash,
    pub network_runtime_observation_signature: Signature,
    pub randomness_beacon_records: u64,
    pub randomness_beacon_root: Hash,
    pub randomness_beacon_signature: Signature,
    pub randomness_beacon_raw_records: Vec<PublicRandomnessBeaconRecord>,
    pub data_availability_measurement_records: u64,
    pub data_availability_measurement_root: Hash,
    pub data_availability_measurement_signature: Signature,
    pub data_availability_raw_records: Vec<PublicDataAvailabilityMeasurementRecord>,
    pub invalid_work_rejection_records: u64,
    pub invalid_work_rejection_root: Hash,
    pub invalid_work_rejection_signature: Signature,
    pub invalid_work_raw_records: Vec<PublicInvalidWorkRejectionRecord>,
    pub reward_settlement_root: Hash,
    pub reward_settlement_signature: Signature,
    pub reward_settlement_raw_records: Vec<PublicRewardSettlementRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundleReport {
    pub run_evidence: PublicTestnetEvidence,
    pub has_published_evidence_bundle: bool,
    pub has_independent_auditor_records: bool,
    pub has_signed_run_window: bool,
    pub has_block_history: bool,
    pub has_finality_history: bool,
    pub has_operator_identity_attestations: bool,
    pub has_network_runtime_observations: bool,
    pub has_randomness_beacon_evidence: bool,
    pub has_data_availability_measurements: bool,
    pub has_invalid_work_rejection_records: bool,
    pub has_reward_settlement_record_summary: bool,
    pub has_public_supporting_record_artifacts: bool,
    pub has_cuda_verified_miners: bool,
    pub has_cuda_graph_execution_evidence: bool,
    pub independently_checkable: bool,
    pub full_spec_evidence_met: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicDataAvailabilityStatus {
    Available,
    Unavailable,
}

impl PublicDataAvailabilityStatus {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicBlockHistoryRecord {
    pub block: u64,
    pub block_root: Hash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicFinalityHistoryStatus {
    Finalized,
    Unfinalized,
}

impl PublicFinalityHistoryStatus {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Finalized => "finalized",
            Self::Unfinalized => "unfinalized",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicFinalityHistoryRecord {
    pub block: u64,
    pub block_root: Hash,
    pub status: PublicFinalityHistoryStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicDataAvailabilityMeasurementRecord {
    pub receipt_root: Hash,
    pub status: PublicDataAvailabilityStatus,
    pub observed_block: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicInvalidWorkRejectionRecord {
    pub receipt_root: Hash,
    pub observed_block: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicRewardSettlementRecord {
    pub receipt_root: Hash,
    pub miner_id: Hash,
    pub validator_id: Hash,
    pub observed_block: u64,
}

impl PublicBlockHistoryRecord {
    pub fn record_line(&self) -> String {
        format!(
            "block_history_record={},{}",
            self.block,
            hex(&self.block_root)
        )
    }

    pub fn record_root(&self) -> Hash {
        supporting_record_root(PublicEvidenceRecordKind::BlockHistory, &self.record_line())
    }
}

impl PublicFinalityHistoryRecord {
    pub fn record_line(&self) -> String {
        format!(
            "finality_history_record={},{},{}",
            self.block,
            hex(&self.block_root),
            self.status.tag()
        )
    }

    pub fn record_root(&self) -> Hash {
        supporting_record_root(
            PublicEvidenceRecordKind::FinalityHistory,
            &self.record_line(),
        )
    }
}

impl PublicDataAvailabilityMeasurementRecord {
    pub fn record_line(&self) -> String {
        format!(
            "data_availability_measurement={},{},{}",
            hex(&self.receipt_root),
            self.status.tag(),
            self.observed_block
        )
    }

    pub fn record_root(&self) -> Hash {
        supporting_record_root(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            &self.record_line(),
        )
    }
}

impl PublicInvalidWorkRejectionRecord {
    pub fn record_line(&self) -> String {
        format!(
            "invalid_work_rejection={},rejected,{}",
            hex(&self.receipt_root),
            self.observed_block
        )
    }

    pub fn record_root(&self) -> Hash {
        supporting_record_root(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            &self.record_line(),
        )
    }
}

impl PublicRewardSettlementRecord {
    pub fn record_line(&self) -> String {
        format!(
            "reward_settlement={},{},{},{}",
            hex(&self.receipt_root),
            hex(&self.miner_id),
            hex(&self.validator_id),
            self.observed_block
        )
    }

    pub fn record_root(&self) -> Hash {
        supporting_record_root(
            PublicEvidenceRecordKind::RewardSettlements,
            &self.record_line(),
        )
    }
}

fn supporting_record_root(kind: PublicEvidenceRecordKind, line: &str) -> Hash {
    hash_bytes(
        b"tensor-vm-public-evidence-supporting-record-root-v1",
        &[kind.manifest_tag().as_bytes(), line.as_bytes()],
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicRandomnessBeaconProofKind {
    DrandV1,
    ValidatorVrfV1,
    LocalDeterministicFixtureV1,
}

impl PublicRandomnessBeaconProofKind {
    pub fn tag(self) -> &'static str {
        match self {
            Self::DrandV1 => "drand-v1",
            Self::ValidatorVrfV1 => "validator-vrf-v1",
            Self::LocalDeterministicFixtureV1 => "local-deterministic-fixture-v1",
        }
    }

    pub fn is_public_unbiasable(self) -> bool {
        matches!(self, Self::DrandV1 | Self::ValidatorVrfV1)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicRandomnessBeaconRecordStatus {
    Accepted,
    Rejected,
}

impl PublicRandomnessBeaconRecordStatus {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicRandomnessBeaconRecord {
    pub source_id: Hash,
    pub beacon_round: u64,
    pub randomness_root: Hash,
    pub proof_root: Hash,
    pub proof_kind: PublicRandomnessBeaconProofKind,
    pub observed_block: u64,
    pub status: PublicRandomnessBeaconRecordStatus,
}

impl PublicRandomnessBeaconRecord {
    pub fn accepted_public(
        source_id: Hash,
        beacon_round: u64,
        randomness_root: Hash,
        proof_root: Hash,
        proof_kind: PublicRandomnessBeaconProofKind,
        observed_block: u64,
    ) -> Self {
        Self {
            source_id,
            beacon_round,
            randomness_root,
            proof_root,
            proof_kind,
            observed_block,
            status: PublicRandomnessBeaconRecordStatus::Accepted,
        }
    }

    pub fn local_fixture(
        source_id: Hash,
        beacon_round: u64,
        randomness_root: Hash,
        proof_root: Hash,
        observed_block: u64,
    ) -> Self {
        Self {
            source_id,
            beacon_round,
            randomness_root,
            proof_root,
            proof_kind: PublicRandomnessBeaconProofKind::LocalDeterministicFixtureV1,
            observed_block,
            status: PublicRandomnessBeaconRecordStatus::Accepted,
        }
    }

    pub fn record_line(&self) -> String {
        format!(
            "randomness_beacon_record={},{},{},{},{},{},{}",
            hex(&self.source_id),
            self.beacon_round,
            hex(&self.randomness_root),
            hex(&self.proof_root),
            self.proof_kind.tag(),
            self.observed_block,
            self.status.tag()
        )
    }

    pub fn record_root(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-public-evidence-supporting-record-root-v1",
            &[
                PublicEvidenceRecordKind::RandomnessBeaconEvidence
                    .manifest_tag()
                    .as_bytes(),
                self.record_line().as_bytes(),
            ],
        )
    }

    pub fn is_accepted_public_unbiasable(&self) -> bool {
        self.status == PublicRandomnessBeaconRecordStatus::Accepted
            && self.proof_kind.is_public_unbiasable()
            && self.source_id != [0; 32]
            && self.randomness_root != [0; 32]
            && self.proof_root != [0; 32]
    }
}

fn ratio_to_bps(value: f64) -> u64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn ratio_parts_to_bps(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    let numerator = u128::from(numerator.min(denominator));
    let denominator = u128::from(denominator);
    (((numerator * 10_000) + (denominator / 2)) / denominator) as u64
}

fn required_blocks_for_days(days: u64, block_time_seconds: u64) -> u64 {
    required_duration_seconds_for_days(days) / block_time_seconds.max(1)
}

fn required_duration_seconds_for_days(days: u64) -> u64 {
    days.saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
}

fn public_testnet_criteria_are_full_spec(criteria: &PublicTestnetCriteria) -> bool {
    let full_spec = PublicTestnetCriteria::default();
    criteria.min_miners >= full_spec.min_miners
        && criteria.min_validators >= full_spec.min_validators
        && criteria.duration_days >= full_spec.duration_days
        && criteria.min_finality_rate_bps >= full_spec.min_finality_rate_bps
        && criteria.min_data_availability_bps >= full_spec.min_data_availability_bps
        && criteria.min_invalid_work_rejections >= full_spec.min_invalid_work_rejections
        && criteria.min_reward_settlement_records >= full_spec.min_reward_settlement_records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::types::hash_bytes;

    mod deployment_docs;
    mod evidence_bundle;
    mod evidence_manifest;
    mod local_harness;
    mod manifest_fixtures;
    mod network_runtime;
    mod preflight_manifest;
    mod run_evidence;
    mod run_fixtures;
    mod run_services;
    use manifest_fixtures::*;
    use run_fixtures::*;
}
