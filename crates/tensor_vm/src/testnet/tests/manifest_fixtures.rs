use super::*;
use crate::hash::hex;
use crate::types::{address, hash_bytes};

pub(super) fn manifest_hash(domain: &[u8], label: &[u8]) -> String {
    hex(&hash_bytes(domain, &[label]))
}

pub(super) fn manifest_address(label: &[u8]) -> String {
    hex(&address(label))
}

pub(super) fn manifest_publication_signature() -> String {
    let publication = PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    );
    hex(&publication.manifest_signature)
}

pub(super) fn manifest_auditor_uri() -> String {
    format!(
        "https://auditors.tensorvm.net/{}/0",
        manifest_hash(b"test", b"public-evidence-bundle")
    )
}

pub(super) fn manifest_auditor_signature() -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let record = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        "https://tensorvm.net/tensorvm/public-evidence.json",
        address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        1_700_000_060,
    );
    hex(&record.auditor_signature)
}

pub(super) fn manifest_bundle() -> PublicTestnetEvidenceBundle {
    complete_public_evidence_bundle()
}

pub(super) fn manifest_node_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
        PublicNodeRole::Validator => {
            PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
        }
    };
    hex(&node.heartbeat_signature)
}

pub(super) fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
    format!("https://operators.tensorvm.net/{}", hex(operator_id))
}

pub(super) fn manifest_operator_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        node_address,
        operator_id,
        manifest_operator_identity_uri(&operator_id),
        1_700_000_000,
    );
    hex(&attestation.operator_signature)
}

pub(super) fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
    hex(&public_service(kind, label, 0, 9).health_check_signature)
}

pub(super) fn manifest_artifact_line(
    kind: PublicEvidenceRecordKind,
    root_label: &[u8],
    record_count: u64,
) -> String {
    manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
}

pub(super) fn manifest_artifact_line_for_root(
    kind: PublicEvidenceRecordKind,
    record_root: Hash,
    record_count: u64,
) -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let artifact_uri = public_evidence_supporting_artifact_uri(&bundle_id, kind);
    let signature = sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &bundle_id,
        kind,
        &artifact_uri,
        &record_root,
        record_count,
    );
    format!(
        "record_artifact={},{},{},{},{}",
        kind.manifest_tag(),
        artifact_uri,
        hex(&record_root),
        record_count,
        hex(&signature)
    )
}

pub(super) fn manifest_network_observation_lines() -> String {
    public_network_runtime_observations_for_run(&complete_public_run_evidence())
        .iter()
        .map(|observation| {
            format!(
                "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                hex(&observation.operator_id),
                observation.peer_id,
                observation.listen_address,
                observation.observed_at_unix_seconds,
                observation.gossip_topic_count,
                observation.request_response_protocol_count,
                observation.bootstrap_peer_count,
                observation.max_transmit_bytes,
                observation.request_timeout_seconds,
                observation.max_concurrent_streams,
                observation.idle_connection_timeout_seconds,
                hex(&observation.record_root),
                hex(&observation.observation_signature)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn resign_record_summary_and_artifact(
    bundle: &mut PublicTestnetEvidenceBundle,
    kind: PublicEvidenceRecordKind,
    record_root: Hash,
    record_count: u64,
) {
    let bundle_id = bundle.publication.bundle_id;
    let signer = bundle.publication.manifest_signer;
    let summary_signature =
        sign_public_evidence_record(&signer, &bundle_id, kind, &record_root, record_count);
    match kind {
        PublicEvidenceRecordKind::BlockHistory => {
            bundle.block_history_records = record_count;
            bundle.block_history_root = record_root;
            bundle.block_history_signature = summary_signature;
        }
        PublicEvidenceRecordKind::FinalityHistory => {
            bundle.finality_history_records = record_count;
            bundle.finality_history_root = record_root;
            bundle.finality_history_signature = summary_signature;
        }
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            bundle.network_runtime_observation_records = record_count;
            bundle.network_runtime_observation_root = record_root;
            bundle.network_runtime_observation_signature = summary_signature;
        }
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => {
            bundle.randomness_beacon_records = record_count;
            bundle.randomness_beacon_root = record_root;
            bundle.randomness_beacon_signature = summary_signature;
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            bundle.data_availability_measurement_records = record_count;
            bundle.data_availability_measurement_root = record_root;
            bundle.data_availability_measurement_signature = summary_signature;
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => {
            bundle.invalid_work_rejection_records = record_count;
            bundle.invalid_work_rejection_root = record_root;
            bundle.invalid_work_rejection_signature = summary_signature;
        }
        PublicEvidenceRecordKind::RewardSettlements => {
            bundle.reward_settlement_root = record_root;
            bundle.reward_settlement_signature = summary_signature;
        }
    }
    if let Some(artifact) = bundle
        .supporting_artifacts
        .iter_mut()
        .find(|artifact| artifact.kind == kind)
    {
        artifact.record_root = record_root;
        artifact.record_count = record_count;
        let artifact_uri = artifact.artifact_uri.clone();
        artifact.artifact_signature = sign_public_evidence_artifact(
            &signer,
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            record_count,
        );
    }
}

pub(super) fn complete_public_evidence_manifest_text() -> String {
    format!(
        "\
# TensorVM external public evidence manifest
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}

bundle_id=0x{}
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
        manifest_hash(b"test", b"public-evidence-bundle"),
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
            10,
        ),
        manifest_artifact_line_for_root(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            manifest_bundle().network_runtime_observation_root,
            3,
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            b"randomness-beacon-root",
            10,
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20,
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1,
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1,
        ),
        manifest_hash(b"test", b"block-history-root"),
        hex(&manifest_bundle().block_history_signature),
        manifest_hash(b"test", b"finality-history-root"),
        hex(&manifest_bundle().finality_history_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"test", b"miner-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"test", b"miner-b-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"test", b"validator-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
        manifest_operator_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator",
        ),
        manifest_network_observation_lines(),
        hex(&manifest_bundle().network_runtime_observation_root),
        hex(&manifest_bundle().network_runtime_observation_signature),
        manifest_hash(b"test", b"randomness-beacon-root"),
        hex(&manifest_bundle().randomness_beacon_signature),
        manifest_hash(b"test", b"data-availability-root"),
        hex(&manifest_bundle().data_availability_measurement_signature),
        hex(&manifest_bundle().run_window_signature),
        manifest_hash(b"test", b"invalid-work-root"),
        hex(&manifest_bundle().invalid_work_rejection_signature),
        manifest_hash(b"test", b"reward-settlement-root"),
        hex(&manifest_bundle().reward_settlement_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"test", b"miner-a-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"test", b"miner-b-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"test", b"validator-a-operator"),
        manifest_node_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator",
        ),
        manifest_hash(b"test", b"rpc-service"),
        manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_hash(b"test", b"explorer-service"),
        manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_hash(b"test", b"faucet-service"),
        manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_hash(b"test", b"telemetry-service"),
        manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
    )
}

pub(super) fn complete_public_preflight_manifest_text() -> String {
    format!(
        "\
# TensorVM public testnet launch preflight manifest
version={PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION}
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,{},https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,{},https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
",
        manifest_hash(b"test", b"rpc-service"),
        manifest_hash(b"test", b"explorer-service"),
        manifest_hash(b"test", b"faucet-service"),
        manifest_hash(b"test", b"telemetry-service"),
    )
}

pub(super) fn manifest_without_line(manifest: &str, prefix: &str) -> String {
    manifest
        .lines()
        .filter(|line| !line.starts_with(prefix))
        .collect::<Vec<_>>()
        .join("\n")
}
