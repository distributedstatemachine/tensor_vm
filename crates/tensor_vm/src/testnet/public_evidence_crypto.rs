use super::{
    PublicNetworkRuntimeObservation, PublicNodeRole, PublicServiceEndpoint, PublicServiceKind,
    PublicTestnetRunEvidence,
};
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::types::{Address, Hash, Signature, hash_bytes, sign};
use libp2p::PeerId;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PublicNetworkRuntimeObservationDetails {
    pub(super) operator_id: Hash,
    pub(super) peer_id: String,
    pub(super) listen_address: String,
    pub(super) observed_at_unix_seconds: u64,
    pub(super) gossip_topic_count: u64,
    pub(super) request_response_protocol_count: u64,
    pub(super) bootstrap_peer_count: u64,
    pub(super) max_transmit_bytes: u64,
    pub(super) request_timeout_seconds: u64,
    pub(super) max_concurrent_streams: u64,
    pub(super) idle_connection_timeout_seconds: u64,
}

pub(super) fn public_evidence_manifest_message(
    bundle_id: &Hash,
    public_uri: &str,
    manifest_signature_count: u64,
    independent_auditor_count: u64,
) -> Hash {
    let signature_count = manifest_signature_count.to_le_bytes();
    let auditor_count = independent_auditor_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-manifest-v1",
        &[
            bundle_id,
            public_uri.as_bytes(),
            &signature_count,
            &auditor_count,
        ],
    )
}

pub(super) fn public_evidence_auditor_message(
    bundle_id: &Hash,
    public_uri: &str,
    auditor_id: &Address,
    audit_uri: &str,
    observed_at_unix_seconds: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-auditor-v1",
        &[
            bundle_id,
            public_uri.as_bytes(),
            auditor_id,
            audit_uri.as_bytes(),
            &observed_at,
        ],
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicEvidenceRecordKind {
    BlockHistory,
    FinalityHistory,
    NetworkRuntimeObservations,
    RandomnessBeaconEvidence,
    DataAvailabilityMeasurements,
    InvalidWorkRejections,
    RewardSettlements,
    DetectionMeasurements,
}

impl PublicEvidenceRecordKind {
    fn tag(self) -> &'static [u8] {
        match self {
            Self::BlockHistory => b"block-history",
            Self::FinalityHistory => b"finality-history",
            Self::NetworkRuntimeObservations => b"network-runtime-observations",
            Self::RandomnessBeaconEvidence => b"randomness-beacon-evidence",
            Self::DataAvailabilityMeasurements => b"data-availability-measurements",
            Self::InvalidWorkRejections => b"invalid-work-rejections",
            Self::RewardSettlements => b"reward-settlements",
            Self::DetectionMeasurements => b"detection-measurements",
        }
    }

    pub fn manifest_tag(self) -> &'static str {
        match self {
            Self::BlockHistory => "block-history",
            Self::FinalityHistory => "finality-history",
            Self::NetworkRuntimeObservations => "network-runtime",
            Self::RandomnessBeaconEvidence => "randomness-beacon",
            Self::DataAvailabilityMeasurements => "data-availability",
            Self::InvalidWorkRejections => "invalid-work",
            Self::RewardSettlements => "reward-settlement",
            Self::DetectionMeasurements => "detection-measurement",
        }
    }
}

pub(super) fn parse_public_evidence_record_kind_tag(
    value: &str,
) -> Result<PublicEvidenceRecordKind> {
    match value {
        "block-history" => Ok(PublicEvidenceRecordKind::BlockHistory),
        "finality-history" => Ok(PublicEvidenceRecordKind::FinalityHistory),
        "network-runtime" => Ok(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        "randomness-beacon" => Ok(PublicEvidenceRecordKind::RandomnessBeaconEvidence),
        "data-availability" => Ok(PublicEvidenceRecordKind::DataAvailabilityMeasurements),
        "invalid-work" => Ok(PublicEvidenceRecordKind::InvalidWorkRejections),
        "reward-settlement" => Ok(PublicEvidenceRecordKind::RewardSettlements),
        "detection-measurement" => Ok(PublicEvidenceRecordKind::DetectionMeasurements),
        _ => Err(TvmError::InvalidReceipt(
            "invalid public evidence record kind",
        )),
    }
}

pub(super) fn public_evidence_record_message(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    record_root: &Hash,
    record_count: u64,
) -> Hash {
    let count = record_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-record-v1",
        &[bundle_id, kind.tag(), record_root, &count],
    )
}

pub(super) fn public_evidence_artifact_message(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    artifact_uri: &str,
    record_root: &Hash,
    record_count: u64,
) -> Hash {
    let count = record_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-artifact-v1",
        &[
            bundle_id,
            kind.tag(),
            artifact_uri.as_bytes(),
            record_root,
            &count,
        ],
    )
}

pub fn sign_public_evidence_record(
    signer: &Address,
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    record_root: &Hash,
    record_count: u64,
) -> Signature {
    sign(
        signer,
        &public_evidence_record_message(bundle_id, kind, record_root, record_count),
    )
}

pub fn sign_public_evidence_artifact(
    signer: &Address,
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    artifact_uri: &str,
    record_root: &Hash,
    record_count: u64,
) -> Signature {
    sign(
        signer,
        &public_evidence_artifact_message(bundle_id, kind, artifact_uri, record_root, record_count),
    )
}

pub(super) fn public_evidence_supporting_artifact_uri(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
) -> String {
    format!(
        "https://evidence.tensorvm.net/{}/{}.json",
        hex(bundle_id),
        kind.manifest_tag()
    )
}

pub(super) fn public_run_window_message(
    bundle_id: &Hash,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Hash {
    let started = run_started_at_unix_seconds.to_le_bytes();
    let ended = run_ended_at_unix_seconds.to_le_bytes();
    let blocks = observed_blocks.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-run-window-v1",
        &[bundle_id, &started, &ended, &blocks],
    )
}

pub fn sign_public_run_window(
    signer: &Address,
    bundle_id: &Hash,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Signature {
    sign(
        signer,
        &public_run_window_message(
            bundle_id,
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
        ),
    )
}

pub(super) fn public_node_role_tag(role: PublicNodeRole) -> &'static [u8] {
    match role {
        PublicNodeRole::Miner => b"miner",
        PublicNodeRole::Validator => b"validator",
    }
}

pub(super) fn public_node_heartbeat_message(
    address: &Address,
    operator_id: &Hash,
    role: PublicNodeRole,
    first_seen_block: u64,
    last_seen_block: u64,
    signed_heartbeat_count: u64,
) -> Hash {
    let first_seen = first_seen_block.to_le_bytes();
    let last_seen = last_seen_block.to_le_bytes();
    let heartbeat_count = signed_heartbeat_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-node-heartbeat-v1",
        &[
            address,
            operator_id,
            public_node_role_tag(role),
            &first_seen,
            &last_seen,
            &heartbeat_count,
        ],
    )
}

pub(super) fn public_operator_identity_message(
    role: PublicNodeRole,
    address: &Address,
    operator_id: &Hash,
    identity_uri: &str,
    observed_at_unix_seconds: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-operator-identity-v1",
        &[
            public_node_role_tag(role),
            address,
            operator_id,
            identity_uri.as_bytes(),
            &observed_at,
        ],
    )
}

pub(super) fn public_service_health_message(
    kind: PublicServiceKind,
    endpoint: &PublicServiceEndpoint,
    first_seen_block: u64,
    last_seen_block: u64,
    reachable_observation_count: u64,
    signed_health_check_count: u64,
) -> Hash {
    let first_seen = first_seen_block.to_le_bytes();
    let last_seen = last_seen_block.to_le_bytes();
    let reachable_count = reachable_observation_count.to_le_bytes();
    let signed_count = signed_health_check_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-service-health-v1",
        &[
            kind.evidence_tag(),
            &endpoint.endpoint_id,
            endpoint.public_url.as_bytes(),
            endpoint.health_path.as_bytes(),
            &first_seen,
            &last_seen,
            &reachable_count,
            &signed_count,
        ],
    )
}

pub(super) fn public_service_content_message(
    kind: PublicServiceKind,
    endpoint_id: &Hash,
    public_url: &str,
    content_path: &str,
    content_root: &Hash,
    observed_at_unix_seconds: u64,
    min_content_bytes: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    let min_bytes = min_content_bytes.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-service-content-v1",
        &[
            kind.evidence_tag(),
            endpoint_id,
            public_url.as_bytes(),
            content_path.as_bytes(),
            content_root,
            &observed_at,
            &min_bytes,
        ],
    )
}

pub(super) fn public_network_runtime_observation_root(
    details: &PublicNetworkRuntimeObservationDetails,
) -> Hash {
    let observed_at = details.observed_at_unix_seconds.to_le_bytes();
    let gossip_topics = details.gossip_topic_count.to_le_bytes();
    let request_response_protocols = details.request_response_protocol_count.to_le_bytes();
    let bootstrap_peers = details.bootstrap_peer_count.to_le_bytes();
    let max_transmit = details.max_transmit_bytes.to_le_bytes();
    let request_timeout = details.request_timeout_seconds.to_le_bytes();
    let max_streams = details.max_concurrent_streams.to_le_bytes();
    let idle_timeout = details.idle_connection_timeout_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-network-runtime-observation-v1",
        &[
            &details.operator_id,
            details.peer_id.as_bytes(),
            details.listen_address.as_bytes(),
            &observed_at,
            &gossip_topics,
            &request_response_protocols,
            &bootstrap_peers,
            &max_transmit,
            &request_timeout,
            &max_streams,
            &idle_timeout,
        ],
    )
}

pub(super) fn public_network_runtime_observation_signature(
    operator_id: &Hash,
    record_root: &Hash,
) -> Signature {
    hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[operator_id, record_root],
    )
}

pub(crate) fn aggregate_public_evidence_record_roots(
    kind: PublicEvidenceRecordKind,
    record_roots: &[Hash],
) -> Result<Hash> {
    if record_roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record roots argument is empty"));
    }
    if record_roots.contains(&[0; 32]) {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    let mut unique_roots = BTreeSet::new();
    if record_roots.iter().any(|root| !unique_roots.insert(*root)) {
        return Err(TvmError::InvalidReceipt("duplicate record root argument"));
    }
    let record_count = (record_roots.len() as u64).to_le_bytes();
    let mut encoded_roots = Vec::with_capacity(record_roots.len() * 32);
    for root in record_roots {
        encoded_roots.extend_from_slice(root);
    }
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-record-root-aggregation-v1",
        &[
            kind.manifest_tag().as_bytes(),
            &record_count,
            &encoded_roots,
        ],
    ))
}

#[cfg(test)]
pub(super) fn deterministic_public_network_peer_id(operator_id: &Hash) -> String {
    public_network_peer_id(operator_id)
}

#[cfg(not(test))]
fn deterministic_public_network_peer_id(operator_id: &Hash) -> String {
    public_network_peer_id(operator_id)
}

fn public_network_peer_id(operator_id: &Hash) -> String {
    let seed = hash_bytes(
        b"tensor-vm-public-network-observation-peer-id-v1",
        &[operator_id],
    );
    let keypair = libp2p::identity::Keypair::ed25519_from_bytes(seed)
        .expect("hashed operator id should form an ed25519 secret key");
    PeerId::from(keypair.public()).to_string()
}

pub(crate) fn public_network_runtime_observations_for_run(
    run: &PublicTestnetRunEvidence,
) -> Vec<PublicNetworkRuntimeObservation> {
    run.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
                operator_id: node.operator_id,
                peer_id: deterministic_public_network_peer_id(&node.operator_id),
                listen_address: format!("/dns/node-{index}.tensorvm.net/tcp/{}", 4_001 + index),
                observed_at_unix_seconds: run.run_started_at_unix_seconds,
                gossip_topic_count: 5,
                request_response_protocol_count: 3,
                bootstrap_peer_count: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            })
        })
        .collect()
}
