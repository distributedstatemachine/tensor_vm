use super::evidence_fields::{
    exact_comma_fields, parse_hash_field, parse_u64_field, public_evidence_record_kind_tag,
};
use crate::error::{Result, TvmError};
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{Hash, hash_bytes};

pub(super) fn supporting_record_line_prefix(
    kind: PublicEvidenceRecordKind,
) -> Option<&'static str> {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => Some("block_history_record="),
        PublicEvidenceRecordKind::FinalityHistory => Some("finality_history_record="),
        PublicEvidenceRecordKind::NetworkRuntimeObservations => None,
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => Some("randomness_beacon_record="),
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            Some("data_availability_measurement=")
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => Some("invalid_work_rejection="),
        PublicEvidenceRecordKind::RewardSettlements => Some("reward_settlement="),
        PublicEvidenceRecordKind::DetectionMeasurements => Some("detection_measurement="),
        PublicEvidenceRecordKind::ValidatorVrfLifecycle => Some("validator_vrf_lifecycle="),
    }
}

pub(super) fn supporting_record_root_from_line(
    kind: PublicEvidenceRecordKind,
    line: &str,
    prefix: &str,
) -> Result<Hash> {
    let payload = line.strip_prefix(prefix).ok_or(TvmError::InvalidReceipt(
        "unsupported public evidence record line",
    ))?;
    if payload.is_empty() || payload.trim() != payload {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    validate_supporting_record_payload(kind, payload)?;
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-supporting-record-root-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            line.as_bytes(),
        ],
    ))
}

pub(super) fn validate_supporting_record_payload(
    kind: PublicEvidenceRecordKind,
    payload: &str,
) -> Result<()> {
    const INVALID_SUPPORTING_RECORD: &str = "invalid public evidence supporting record line";
    match kind {
        PublicEvidenceRecordKind::BlockHistory => {
            let fields = exact_comma_fields(payload, 2, INVALID_SUPPORTING_RECORD)?;
            parse_u64_field(fields[0])?;
            parse_hash_field(fields[1])?;
        }
        PublicEvidenceRecordKind::FinalityHistory => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_u64_field(fields[0])?;
            parse_hash_field(fields[1])?;
            require_supporting_record_status(fields[2], &["finalized", "unfinalized"])?;
        }
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            return Err(TvmError::InvalidReceipt(INVALID_SUPPORTING_RECORD));
        }
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => {
            let fields = exact_comma_fields(payload, 7, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            if parse_u64_field(fields[1])? == 0 {
                return Err(TvmError::InvalidReceipt(INVALID_SUPPORTING_RECORD));
            }
            parse_hash_field(fields[2])?;
            parse_hash_field(fields[3])?;
            require_supporting_record_status(
                fields[4],
                &[
                    "drand-v1",
                    "validator-vrf-v1",
                    "local-deterministic-fixture-v1",
                ],
            )?;
            parse_u64_field(fields[5])?;
            require_supporting_record_status(fields[6], &["accepted", "rejected"])?;
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            require_supporting_record_status(fields[1], &["available", "unavailable"])?;
            parse_u64_field(fields[2])?;
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => {
            let fields = exact_comma_fields(payload, 3, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            require_supporting_record_status(fields[1], &["rejected"])?;
            parse_u64_field(fields[2])?;
        }
        PublicEvidenceRecordKind::RewardSettlements => {
            let fields = exact_comma_fields(payload, 4, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            parse_hash_field(fields[1])?;
            parse_hash_field(fields[2])?;
            parse_u64_field(fields[3])?;
        }
        PublicEvidenceRecordKind::DetectionMeasurements => {
            let fields = exact_comma_fields(payload, 5, INVALID_SUPPORTING_RECORD)?;
            require_detection_measurement_mechanism(fields[0])?;
            parse_hash_field(fields[1])?;
            let sample_count = parse_u64_field(fields[2])?;
            let detected_count = parse_u64_field(fields[3])?;
            if sample_count == 0 || detected_count > sample_count {
                return Err(TvmError::InvalidReceipt(INVALID_SUPPORTING_RECORD));
            }
            parse_u64_field(fields[4])?;
        }
        PublicEvidenceRecordKind::ValidatorVrfLifecycle => {
            let fields = exact_comma_fields(payload, 5, INVALID_SUPPORTING_RECORD)?;
            parse_hash_field(fields[0])?;
            parse_hash_field(fields[1])?;
            if parse_u64_field(fields[2])? == 0 {
                return Err(TvmError::InvalidReceipt(INVALID_SUPPORTING_RECORD));
            }
            require_supporting_record_status(fields[3], &["committed", "revealed"])?;
            parse_u64_field(fields[4])?;
        }
    }
    Ok(())
}

fn require_supporting_record_status(status: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&status) {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(())
}

fn require_detection_measurement_mechanism(mechanism: &str) -> Result<()> {
    if mechanism.is_empty()
        || mechanism.trim() != mechanism
        || !mechanism
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(())
}
