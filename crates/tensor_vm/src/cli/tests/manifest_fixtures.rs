use crate::hash::hex;
use crate::testnet::{
    PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PublicEvidenceRecordKind, PublicNodeRole,
    PublicServiceKind,
};
use crate::types::{address, hash_bytes};

use super::manifest_bundle_fixtures::manifest_bundle;
use super::manifest_network_fixtures::manifest_network_observation_lines_for_run;
use super::manifest_node_fixtures::{
    manifest_node_signature, manifest_operator_identity_uri, manifest_operator_signature,
};
use super::manifest_publication_fixtures::{
    manifest_artifact_line, manifest_artifact_line_for_root, manifest_auditor_signature,
    manifest_auditor_uri, manifest_publication_signature,
};
use super::manifest_service_fixtures::{manifest_service_content_line, manifest_service_signature};

pub(super) fn manifest_hash(label: &[u8]) -> String {
    hex(&hash_bytes(b"test", &[label]))
}

pub(super) fn manifest_address(label: &[u8]) -> String {
    hex(&address(label))
}

pub(super) fn evidence_manifest() -> String {
    format!(
        "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
auditor={},{},1700000060,{}
{}
{}
{}
{}
{}
{}
{}
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
operator=miner,{},{},{},1700000000,{}
operator=miner,{},{},{},1700000000,{}
operator=validator,{},{},{},1700000000,{}
{}
network_runtime_observation_records=3
network_runtime_observation_root={}
network_runtime_observation_signature={}
randomness_beacon_records=10
randomness_beacon_root={}
randomness_beacon_signature={}
data_availability_measurement_records=20
data_availability_measurement_root={}
data_availability_measurement_signature={}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
run_window_signature={}
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root={}
invalid_work_rejection_signature={}
reward_settlement_records=1
reward_settlement_root={}
reward_settlement_signature={}
cuda_verified_miner_count=2
cuda_graph_execution_receipts=1
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.net/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.net/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.net/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,0,9,10,10,{}
{}
{}
{}
{}
",
        manifest_hash(b"public-evidence-bundle"),
        manifest_address(b"public-evidence-publisher"),
        manifest_publication_signature(),
        manifest_address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        manifest_auditor_signature(),
        manifest_artifact_line(
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10
        ),
        manifest_artifact_line_for_root(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            manifest_bundle().network_runtime_observation_root,
            3
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            b"randomness-beacon-root",
            10
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1
        ),
        manifest_hash(b"block-history-root"),
        hex(&manifest_bundle().block_history_signature),
        manifest_hash(b"finality-history-root"),
        hex(&manifest_bundle().finality_history_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
        manifest_operator_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_network_observation_lines_for_run(&manifest_bundle().run),
        hex(&manifest_bundle().network_runtime_observation_root),
        hex(&manifest_bundle().network_runtime_observation_signature),
        manifest_hash(b"randomness-beacon-root"),
        hex(&manifest_bundle().randomness_beacon_signature),
        manifest_hash(b"data-availability-root"),
        hex(&manifest_bundle().data_availability_measurement_signature),
        hex(&manifest_bundle().run_window_signature),
        manifest_hash(b"invalid-work-root"),
        hex(&manifest_bundle().invalid_work_rejection_signature),
        manifest_hash(b"reward-settlement-root"),
        hex(&manifest_bundle().reward_settlement_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_node_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_hash(b"rpc-service"),
        manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_hash(b"explorer-service"),
        manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_hash(b"faucet-service"),
        manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_hash(b"telemetry-service"),
        manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
    )
}
