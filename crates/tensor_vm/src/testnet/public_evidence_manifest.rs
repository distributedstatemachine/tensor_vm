use super::public_evidence_crypto::{
    PublicNetworkRuntimeObservationDetails, parse_public_evidence_record_kind_tag,
};
use super::public_manifest_fields::{
    exact_manifest_record_fields, exact_manifest_scalar, parse_hash_hex, parse_manifest_bool,
    parse_manifest_entries, parse_manifest_u64, parse_service_kind, required_bool, required_hash,
    required_string, required_u64,
};
use super::{
    PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PublicBlockHistoryRecord,
    PublicDataAvailabilityMeasurementRecord, PublicDataAvailabilityStatus,
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceSupportingArtifact,
    PublicFinalityHistoryRecord, PublicFinalityHistoryStatus, PublicInvalidWorkRejectionRecord,
    PublicNetworkRuntimeEvidence, PublicNetworkRuntimeObservation, PublicNodeEvidence,
    PublicNodeRole, PublicOperatorIdentityAttestation, PublicRandomnessBeaconProofKind,
    PublicRandomnessBeaconRecord, PublicRandomnessBeaconRecordStatus, PublicRewardSettlementRecord,
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence,
    PublicTestnetEvidenceBundle, PublicTestnetRunEvidence,
};
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash, Signature};

pub fn parse_public_testnet_evidence_manifest(input: &str) -> Result<PublicTestnetEvidenceBundle> {
    let mut builder = PublicEvidenceManifestBuilder::default();
    for entry in parse_manifest_entries(
        input,
        public_evidence_manifest_field_allows_repeated,
        "malformed evidence manifest line",
        "duplicate evidence manifest field",
    )? {
        builder.set(entry.key, entry.value)?;
    }
    builder.finish()
}

fn public_evidence_manifest_field_allows_repeated(key: &str) -> bool {
    matches!(
        key,
        "auditor"
            | "record_artifact"
            | "operator"
            | "network_runtime_observation"
            | "block_history_record"
            | "finality_history_record"
            | "randomness_beacon_record"
            | "data_availability_measurement"
            | "invalid_work_rejection"
            | "reward_settlement"
            | "node"
            | "service"
            | "service_content"
    )
}

#[derive(Default)]
struct PublicEvidenceManifestBuilder {
    version_seen: bool,
    bundle_id: Option<Hash>,
    public_uri: Option<String>,
    manifest_signer: Option<Address>,
    manifest_signature: Option<Signature>,
    manifest_signature_count: Option<u64>,
    independent_auditor_count: Option<u64>,
    auditor_records: Vec<PublicEvidenceAuditorRecord>,
    supporting_artifacts: Vec<PublicEvidenceSupportingArtifact>,
    block_history_records: Option<u64>,
    block_history_root: Option<Hash>,
    block_history_signature: Option<Signature>,
    block_history_raw_records: Vec<PublicBlockHistoryRecord>,
    finality_history_records: Option<u64>,
    finality_history_root: Option<Hash>,
    finality_history_signature: Option<Signature>,
    finality_history_raw_records: Vec<PublicFinalityHistoryRecord>,
    operator_identity_attestation_records: Option<u64>,
    operator_identity_attestations: Vec<PublicOperatorIdentityAttestation>,
    network_runtime_observations: Vec<PublicNetworkRuntimeObservation>,
    network_runtime_observation_records: Option<u64>,
    network_runtime_observation_root: Option<Hash>,
    network_runtime_observation_signature: Option<Signature>,
    randomness_beacon_records: Option<u64>,
    randomness_beacon_root: Option<Hash>,
    randomness_beacon_signature: Option<Signature>,
    randomness_beacon_raw_records: Vec<PublicRandomnessBeaconRecord>,
    data_availability_measurement_records: Option<u64>,
    data_availability_measurement_root: Option<Hash>,
    data_availability_measurement_signature: Option<Signature>,
    data_availability_raw_records: Vec<PublicDataAvailabilityMeasurementRecord>,
    invalid_work_rejection_records: Option<u64>,
    invalid_work_rejection_root: Option<Hash>,
    invalid_work_rejection_signature: Option<Signature>,
    invalid_work_raw_records: Vec<PublicInvalidWorkRejectionRecord>,
    reward_settlement_root: Option<Hash>,
    reward_settlement_signature: Option<Signature>,
    reward_settlement_raw_records: Vec<PublicRewardSettlementRecord>,
    run_started_at_unix_seconds: Option<u64>,
    run_ended_at_unix_seconds: Option<u64>,
    run_window_signature: Option<Signature>,
    libp2p_runtime_used: Option<bool>,
    peer_discovery_observed: Option<bool>,
    gossip_propagation_observed: Option<bool>,
    request_response_observed: Option<bool>,
    dos_controls_enabled: Option<bool>,
    nodes: Vec<PublicNodeEvidence>,
    services: Vec<PublicServiceEvidence>,
    service_content: Vec<PublicServiceContentEvidence>,
    observed_blocks: Option<u64>,
    finalized_blocks: Option<u64>,
    checked_receipts: Option<u64>,
    available_receipts: Option<u64>,
    invalid_receipts_submitted: Option<u64>,
    invalid_receipts_rejected: Option<u64>,
    reward_settlement_records: Option<u64>,
    cuda_verified_miner_count: Option<u64>,
    cuda_graph_execution_receipts: Option<u64>,
    validator_vrf_lifecycle_records: Option<u64>,
}

impl PublicEvidenceManifestBuilder {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let scalar = exact_manifest_scalar(value)?;
        match key {
            "version" => {
                if scalar != PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION {
                    return Err(TvmError::InvalidReceipt(
                        "unsupported evidence manifest version",
                    ));
                }
                self.version_seen = true;
            }
            "bundle_id" => self.bundle_id = Some(parse_hash_hex(scalar)?),
            "public_uri" => self.public_uri = Some(scalar.to_owned()),
            "manifest_signer" => self.manifest_signer = Some(parse_hash_hex(scalar)?),
            "manifest_signature" => self.manifest_signature = Some(parse_hash_hex(scalar)?),
            "manifest_signature_count" => {
                self.manifest_signature_count = Some(parse_manifest_u64(scalar)?);
            }
            "independent_auditor_count" => {
                self.independent_auditor_count = Some(parse_manifest_u64(scalar)?);
            }
            "auditor" => self
                .auditor_records
                .push(parse_manifest_auditor_record(value)?),
            "record_artifact" => self
                .supporting_artifacts
                .push(parse_manifest_supporting_artifact(value)?),
            "block_history_records" => {
                self.block_history_records = Some(parse_manifest_u64(scalar)?);
            }
            "block_history_root" => self.block_history_root = Some(parse_hash_hex(scalar)?),
            "block_history_signature" => {
                self.block_history_signature = Some(parse_hash_hex(scalar)?);
            }
            "block_history_record" => self
                .block_history_raw_records
                .push(parse_manifest_block_history_record(value)?),
            "finality_history_records" => {
                self.finality_history_records = Some(parse_manifest_u64(scalar)?);
            }
            "finality_history_root" => self.finality_history_root = Some(parse_hash_hex(scalar)?),
            "finality_history_signature" => {
                self.finality_history_signature = Some(parse_hash_hex(scalar)?);
            }
            "finality_history_record" => self
                .finality_history_raw_records
                .push(parse_manifest_finality_history_record(value)?),
            "operator_identity_attestation_records" => {
                self.operator_identity_attestation_records = Some(parse_manifest_u64(scalar)?);
            }
            "operator" => self
                .operator_identity_attestations
                .push(parse_manifest_operator_identity_attestation(value)?),
            "network_runtime_observation" => self
                .network_runtime_observations
                .push(parse_manifest_network_runtime_observation(value)?),
            "network_runtime_observation_records" => {
                self.network_runtime_observation_records = Some(parse_manifest_u64(scalar)?);
            }
            "network_runtime_observation_root" => {
                self.network_runtime_observation_root = Some(parse_hash_hex(scalar)?);
            }
            "network_runtime_observation_signature" => {
                self.network_runtime_observation_signature = Some(parse_hash_hex(scalar)?);
            }
            "randomness_beacon_records" => {
                self.randomness_beacon_records = Some(parse_manifest_u64(scalar)?);
            }
            "randomness_beacon_root" => {
                self.randomness_beacon_root = Some(parse_hash_hex(scalar)?);
            }
            "randomness_beacon_signature" => {
                self.randomness_beacon_signature = Some(parse_hash_hex(scalar)?);
            }
            "randomness_beacon_record" => self
                .randomness_beacon_raw_records
                .push(parse_manifest_randomness_beacon_record(value)?),
            "data_availability_measurement_records" => {
                self.data_availability_measurement_records = Some(parse_manifest_u64(scalar)?);
            }
            "data_availability_measurement_root" => {
                self.data_availability_measurement_root = Some(parse_hash_hex(scalar)?);
            }
            "data_availability_measurement_signature" => {
                self.data_availability_measurement_signature = Some(parse_hash_hex(scalar)?);
            }
            "data_availability_measurement" => self
                .data_availability_raw_records
                .push(parse_manifest_data_availability_measurement(value)?),
            "invalid_work_rejection_records" => {
                self.invalid_work_rejection_records = Some(parse_manifest_u64(scalar)?);
            }
            "invalid_work_rejection_root" => {
                self.invalid_work_rejection_root = Some(parse_hash_hex(scalar)?);
            }
            "invalid_work_rejection_signature" => {
                self.invalid_work_rejection_signature = Some(parse_hash_hex(scalar)?);
            }
            "invalid_work_rejection" => self
                .invalid_work_raw_records
                .push(parse_manifest_invalid_work_rejection(value)?),
            "reward_settlement_root" => self.reward_settlement_root = Some(parse_hash_hex(scalar)?),
            "reward_settlement_signature" => {
                self.reward_settlement_signature = Some(parse_hash_hex(scalar)?);
            }
            "reward_settlement" => self
                .reward_settlement_raw_records
                .push(parse_manifest_reward_settlement(value)?),
            "run_started_at_unix_seconds" => {
                self.run_started_at_unix_seconds = Some(parse_manifest_u64(scalar)?);
            }
            "run_ended_at_unix_seconds" => {
                self.run_ended_at_unix_seconds = Some(parse_manifest_u64(scalar)?);
            }
            "run_window_signature" => self.run_window_signature = Some(parse_hash_hex(scalar)?),
            "libp2p_runtime_used" => self.libp2p_runtime_used = Some(parse_manifest_bool(scalar)?),
            "peer_discovery_observed" => {
                self.peer_discovery_observed = Some(parse_manifest_bool(scalar)?);
            }
            "gossip_propagation_observed" => {
                self.gossip_propagation_observed = Some(parse_manifest_bool(scalar)?);
            }
            "request_response_observed" => {
                self.request_response_observed = Some(parse_manifest_bool(scalar)?);
            }
            "dos_controls_enabled" => {
                self.dos_controls_enabled = Some(parse_manifest_bool(scalar)?)
            }
            "node" => self.nodes.push(parse_manifest_node(value)?),
            "service" => self.services.push(parse_manifest_service(value)?),
            "service_content" => self
                .service_content
                .push(parse_manifest_service_content(value)?),
            "observed_blocks" => self.observed_blocks = Some(parse_manifest_u64(scalar)?),
            "finalized_blocks" => self.finalized_blocks = Some(parse_manifest_u64(scalar)?),
            "checked_receipts" => self.checked_receipts = Some(parse_manifest_u64(scalar)?),
            "available_receipts" => self.available_receipts = Some(parse_manifest_u64(scalar)?),
            "invalid_receipts_submitted" => {
                self.invalid_receipts_submitted = Some(parse_manifest_u64(scalar)?);
            }
            "invalid_receipts_rejected" => {
                self.invalid_receipts_rejected = Some(parse_manifest_u64(scalar)?);
            }
            "reward_settlement_records" => {
                self.reward_settlement_records = Some(parse_manifest_u64(scalar)?);
            }
            "cuda_verified_miner_count" => {
                self.cuda_verified_miner_count = Some(parse_manifest_u64(scalar)?);
            }
            "cuda_graph_execution_receipts" => {
                self.cuda_graph_execution_receipts = Some(parse_manifest_u64(scalar)?);
            }
            "validator_vrf_lifecycle_records" => {
                self.validator_vrf_lifecycle_records = Some(parse_manifest_u64(scalar)?);
            }
            _ => return Err(TvmError::InvalidReceipt("unknown evidence manifest field")),
        }
        Ok(())
    }

    fn finish(self) -> Result<PublicTestnetEvidenceBundle> {
        if !self.version_seen {
            return Err(TvmError::InvalidReceipt(
                "missing evidence manifest version",
            ));
        }
        Ok(PublicTestnetEvidenceBundle {
            run: PublicTestnetRunEvidence {
                nodes: self.nodes,
                network_runtime: PublicNetworkRuntimeEvidence {
                    libp2p_runtime_used: required_bool(self.libp2p_runtime_used)?,
                    peer_discovery_observed: required_bool(self.peer_discovery_observed)?,
                    gossip_propagation_observed: required_bool(self.gossip_propagation_observed)?,
                    request_response_observed: required_bool(self.request_response_observed)?,
                    dos_controls_enabled: required_bool(self.dos_controls_enabled)?,
                },
                services: self.services,
                service_content: self.service_content,
                run_started_at_unix_seconds: required_u64(self.run_started_at_unix_seconds)?,
                run_ended_at_unix_seconds: required_u64(self.run_ended_at_unix_seconds)?,
                observed_blocks: required_u64(self.observed_blocks)?,
                finalized_blocks: required_u64(self.finalized_blocks)?,
                checked_receipts: required_u64(self.checked_receipts)?,
                available_receipts: required_u64(self.available_receipts)?,
                invalid_receipts_submitted: required_u64(self.invalid_receipts_submitted)?,
                invalid_receipts_rejected: required_u64(self.invalid_receipts_rejected)?,
                reward_settlement_records: required_u64(self.reward_settlement_records)?,
                cuda_verified_miner_count: required_u64(self.cuda_verified_miner_count)?,
                cuda_graph_execution_receipts: required_u64(self.cuda_graph_execution_receipts)?,
                validator_vrf_lifecycle_records: required_u64(
                    self.validator_vrf_lifecycle_records,
                )?,
            },
            publication: {
                let mut publication = PublicEvidencePublication::new(
                    required_hash(self.bundle_id)?,
                    required_string(self.public_uri)?,
                    required_hash(self.manifest_signer)?,
                    required_u64(self.manifest_signature_count)?,
                    required_u64(self.independent_auditor_count)?,
                );
                publication.manifest_signature = required_hash(self.manifest_signature)?;
                publication
            },
            auditor_records: self.auditor_records,
            supporting_artifacts: self.supporting_artifacts,
            run_window_signature: required_hash(self.run_window_signature)?,
            block_history_records: required_u64(self.block_history_records)?,
            block_history_root: required_hash(self.block_history_root)?,
            block_history_signature: required_hash(self.block_history_signature)?,
            block_history_raw_records: self.block_history_raw_records,
            finality_history_records: required_u64(self.finality_history_records)?,
            finality_history_root: required_hash(self.finality_history_root)?,
            finality_history_signature: required_hash(self.finality_history_signature)?,
            finality_history_raw_records: self.finality_history_raw_records,
            operator_identity_attestation_records: required_u64(
                self.operator_identity_attestation_records,
            )?,
            operator_identity_attestations: self.operator_identity_attestations,
            network_runtime_observations: self.network_runtime_observations,
            network_runtime_observation_records: required_u64(
                self.network_runtime_observation_records,
            )?,
            network_runtime_observation_root: required_hash(self.network_runtime_observation_root)?,
            network_runtime_observation_signature: required_hash(
                self.network_runtime_observation_signature,
            )?,
            randomness_beacon_records: required_u64(self.randomness_beacon_records)?,
            randomness_beacon_root: required_hash(self.randomness_beacon_root)?,
            randomness_beacon_signature: required_hash(self.randomness_beacon_signature)?,
            randomness_beacon_raw_records: self.randomness_beacon_raw_records,
            data_availability_measurement_records: required_u64(
                self.data_availability_measurement_records,
            )?,
            data_availability_measurement_root: required_hash(
                self.data_availability_measurement_root,
            )?,
            data_availability_measurement_signature: required_hash(
                self.data_availability_measurement_signature,
            )?,
            data_availability_raw_records: self.data_availability_raw_records,
            invalid_work_rejection_records: required_u64(self.invalid_work_rejection_records)?,
            invalid_work_rejection_root: required_hash(self.invalid_work_rejection_root)?,
            invalid_work_rejection_signature: required_hash(self.invalid_work_rejection_signature)?,
            invalid_work_raw_records: self.invalid_work_raw_records,
            reward_settlement_root: required_hash(self.reward_settlement_root)?,
            reward_settlement_signature: required_hash(self.reward_settlement_signature)?,
            reward_settlement_raw_records: self.reward_settlement_raw_records,
        })
    }
}

fn parse_manifest_block_history_record(value: &str) -> Result<PublicBlockHistoryRecord> {
    let fields = exact_manifest_record_fields(value, 2, "malformed block history record")?;
    Ok(PublicBlockHistoryRecord {
        block: parse_manifest_u64(fields[0])?,
        block_root: parse_hash_hex(fields[1])?,
    })
}

fn parse_manifest_finality_history_record(value: &str) -> Result<PublicFinalityHistoryRecord> {
    let fields = exact_manifest_record_fields(value, 3, "malformed finality history record")?;
    let status = match fields[2] {
        "finalized" => PublicFinalityHistoryStatus::Finalized,
        "unfinalized" => PublicFinalityHistoryStatus::Unfinalized,
        _ => return Err(TvmError::InvalidReceipt("unknown finality history status")),
    };
    Ok(PublicFinalityHistoryRecord {
        block: parse_manifest_u64(fields[0])?,
        block_root: parse_hash_hex(fields[1])?,
        status,
    })
}

fn parse_manifest_supporting_artifact(value: &str) -> Result<PublicEvidenceSupportingArtifact> {
    let fields = exact_manifest_record_fields(value, 5, "malformed supporting evidence artifact")?;
    Ok(PublicEvidenceSupportingArtifact {
        kind: parse_public_evidence_record_kind_tag(fields[0])?,
        artifact_uri: fields[1].to_owned(),
        record_root: parse_hash_hex(fields[2])?,
        record_count: parse_manifest_u64(fields[3])?,
        artifact_signature: parse_hash_hex(fields[4])?,
    })
}

fn parse_manifest_randomness_beacon_record(value: &str) -> Result<PublicRandomnessBeaconRecord> {
    let fields = exact_manifest_record_fields(value, 7, "malformed randomness beacon record")?;
    let proof_kind = match fields[4] {
        "drand-v1" => PublicRandomnessBeaconProofKind::DrandV1,
        "validator-vrf-v1" => PublicRandomnessBeaconProofKind::ValidatorVrfV1,
        "local-deterministic-fixture-v1" => {
            PublicRandomnessBeaconProofKind::LocalDeterministicFixtureV1
        }
        _ => {
            return Err(TvmError::InvalidReceipt(
                "unknown randomness beacon proof kind",
            ));
        }
    };
    let status = match fields[6] {
        "accepted" => PublicRandomnessBeaconRecordStatus::Accepted,
        "rejected" => PublicRandomnessBeaconRecordStatus::Rejected,
        _ => return Err(TvmError::InvalidReceipt("unknown randomness beacon status")),
    };
    Ok(PublicRandomnessBeaconRecord {
        source_id: parse_hash_hex(fields[0])?,
        beacon_round: parse_manifest_u64(fields[1])?,
        randomness_root: parse_hash_hex(fields[2])?,
        proof_root: parse_hash_hex(fields[3])?,
        proof_kind,
        observed_block: parse_manifest_u64(fields[5])?,
        status,
    })
}

fn parse_manifest_data_availability_measurement(
    value: &str,
) -> Result<PublicDataAvailabilityMeasurementRecord> {
    let fields = exact_manifest_record_fields(value, 3, "malformed data availability measurement")?;
    let status = match fields[1] {
        "available" => PublicDataAvailabilityStatus::Available,
        "unavailable" => PublicDataAvailabilityStatus::Unavailable,
        _ => {
            return Err(TvmError::InvalidReceipt(
                "unknown data availability measurement status",
            ));
        }
    };
    Ok(PublicDataAvailabilityMeasurementRecord {
        receipt_root: parse_hash_hex(fields[0])?,
        status,
        observed_block: parse_manifest_u64(fields[2])?,
    })
}

fn parse_manifest_invalid_work_rejection(value: &str) -> Result<PublicInvalidWorkRejectionRecord> {
    let fields = exact_manifest_record_fields(value, 3, "malformed invalid work rejection")?;
    if fields[1] != "rejected" {
        return Err(TvmError::InvalidReceipt(
            "unknown invalid work rejection status",
        ));
    }
    Ok(PublicInvalidWorkRejectionRecord {
        receipt_root: parse_hash_hex(fields[0])?,
        observed_block: parse_manifest_u64(fields[2])?,
    })
}

fn parse_manifest_reward_settlement(value: &str) -> Result<PublicRewardSettlementRecord> {
    let fields = exact_manifest_record_fields(value, 4, "malformed reward settlement")?;
    Ok(PublicRewardSettlementRecord {
        receipt_root: parse_hash_hex(fields[0])?,
        miner_id: parse_hash_hex(fields[1])?,
        validator_id: parse_hash_hex(fields[2])?,
        observed_block: parse_manifest_u64(fields[3])?,
    })
}

fn parse_manifest_node(value: &str) -> Result<PublicNodeEvidence> {
    let fields = exact_manifest_record_fields(value, 7, "malformed node evidence")?;
    let address = parse_hash_hex(fields[1])?;
    let operator_id = parse_hash_hex(fields[2])?;
    let first_seen_block = parse_manifest_u64(fields[3])?;
    let last_seen_block = parse_manifest_u64(fields[4])?;
    let signed_heartbeat_count = parse_manifest_u64(fields[5])?;
    let heartbeat_signature = parse_hash_hex(fields[6])?;
    let mut evidence = match fields[0] {
        "miner" => PublicNodeEvidence::miner(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        "validator" => PublicNodeEvidence::validator(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        _ => return Err(TvmError::InvalidReceipt("unknown node evidence role")),
    };
    evidence.heartbeat_signature = heartbeat_signature;
    Ok(evidence)
}

fn parse_manifest_operator_identity_attestation(
    value: &str,
) -> Result<PublicOperatorIdentityAttestation> {
    let fields = exact_manifest_record_fields(value, 6, "malformed operator identity attestation")?;
    let role = match fields[0] {
        "miner" => PublicNodeRole::Miner,
        "validator" => PublicNodeRole::Validator,
        _ => {
            return Err(TvmError::InvalidReceipt(
                "unknown operator attestation role",
            ));
        }
    };
    let mut attestation = PublicOperatorIdentityAttestation::new(
        role,
        parse_hash_hex(fields[1])?,
        parse_hash_hex(fields[2])?,
        fields[3].to_owned(),
        parse_manifest_u64(fields[4])?,
    );
    attestation.operator_signature = parse_hash_hex(fields[5])?;
    Ok(attestation)
}

fn parse_manifest_network_runtime_observation(
    value: &str,
) -> Result<PublicNetworkRuntimeObservation> {
    let fields = exact_manifest_record_fields(value, 13, "malformed network runtime observation")?;
    let mut observation =
        PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
            operator_id: parse_hash_hex(fields[0])?,
            peer_id: fields[1].to_owned(),
            listen_address: fields[2].to_owned(),
            observed_at_unix_seconds: parse_manifest_u64(fields[3])?,
            gossip_topic_count: parse_manifest_u64(fields[4])?,
            request_response_protocol_count: parse_manifest_u64(fields[5])?,
            bootstrap_peer_count: parse_manifest_u64(fields[6])?,
            max_transmit_bytes: parse_manifest_u64(fields[7])?,
            request_timeout_seconds: parse_manifest_u64(fields[8])?,
            max_concurrent_streams: parse_manifest_u64(fields[9])?,
            idle_connection_timeout_seconds: parse_manifest_u64(fields[10])?,
        });
    observation.record_root = parse_hash_hex(fields[11])?;
    observation.observation_signature = parse_hash_hex(fields[12])?;
    Ok(observation)
}

fn parse_manifest_auditor_record(value: &str) -> Result<PublicEvidenceAuditorRecord> {
    let fields = exact_manifest_record_fields(value, 4, "malformed auditor record")?;
    Ok(PublicEvidenceAuditorRecord {
        auditor_id: parse_hash_hex(fields[0])?,
        audit_uri: fields[1].to_owned(),
        observed_at_unix_seconds: parse_manifest_u64(fields[2])?,
        auditor_signature: parse_hash_hex(fields[3])?,
    })
}

fn parse_manifest_service(value: &str) -> Result<PublicServiceEvidence> {
    let fields = exact_manifest_record_fields(value, 9, "malformed service evidence")?;
    let kind = parse_service_kind(fields[0])?;
    let endpoint_id = parse_hash_hex(fields[1])?;
    let public_url = fields[2].to_owned();
    let health_path = fields[3].to_owned();
    let first_seen_block = parse_manifest_u64(fields[4])?;
    let last_seen_block = parse_manifest_u64(fields[5])?;
    let reachable_observation_count = parse_manifest_u64(fields[6])?;
    let signed_health_check_count = parse_manifest_u64(fields[7])?;
    let mut evidence = PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(endpoint_id, public_url, health_path),
        first_seen_block,
        last_seen_block,
        reachable_observation_count,
        signed_health_check_count,
    );
    evidence.health_check_signature = parse_hash_hex(fields[8])?;
    Ok(evidence)
}

fn parse_manifest_service_content(value: &str) -> Result<PublicServiceContentEvidence> {
    let fields = exact_manifest_record_fields(value, 8, "malformed service content evidence")?;
    let mut evidence = PublicServiceContentEvidence::new(
        parse_service_kind(fields[0])?,
        parse_hash_hex(fields[1])?,
        fields[2].to_owned(),
        fields[3].to_owned(),
        parse_hash_hex(fields[4])?,
        parse_manifest_u64(fields[5])?,
        parse_manifest_u64(fields[6])?,
    );
    evidence.content_signature = parse_hash_hex(fields[7])?;
    Ok(evidence)
}
