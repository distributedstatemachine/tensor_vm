use crate::error::{Result, TvmError};
use crate::record_fields::exact_comma_fields as exact_record_fields;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Hash, parse_hash_hex};

pub(super) fn exact_comma_fields<'a>(
    value: &'a str,
    expected_len: usize,
    error: &'static str,
) -> Result<Vec<&'a str>> {
    exact_record_fields(value, expected_len).ok_or(TvmError::InvalidReceipt(error))
}

pub(super) fn parse_u64_field(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

pub(super) fn public_service_kind_tag(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "rpc",
        PublicServiceKind::Explorer => "explorer",
        PublicServiceKind::Faucet => "faucet",
        PublicServiceKind::Telemetry => "telemetry",
    }
}

pub(super) fn parse_public_node_role(value: &str) -> Result<PublicNodeRole> {
    match value {
        "miner" => Ok(PublicNodeRole::Miner),
        "validator" => Ok(PublicNodeRole::Validator),
        _ => Err(TvmError::InvalidReceipt("invalid public node role")),
    }
}

pub(super) fn public_node_role_tag(role: PublicNodeRole) -> &'static str {
    match role {
        PublicNodeRole::Miner => "miner",
        PublicNodeRole::Validator => "validator",
    }
}

pub(super) fn public_evidence_record_kind_tag(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block-history",
        PublicEvidenceRecordKind::FinalityHistory => "finality-history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network-runtime",
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => "randomness-beacon",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data-availability",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid-work",
        PublicEvidenceRecordKind::RewardSettlements => "reward-settlement",
    }
}

pub(super) fn public_evidence_record_field_prefix(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block_history",
        PublicEvidenceRecordKind::FinalityHistory => "finality_history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network_runtime_observation",
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => "randomness_beacon",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data_availability_measurement",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid_work_rejection",
        PublicEvidenceRecordKind::RewardSettlements => "reward_settlement",
    }
}

pub(super) fn parse_hash_field(value: &str) -> Result<Hash> {
    parse_hash_hex(value).map_err(|_| TvmError::InvalidReceipt("invalid hash argument"))
}
