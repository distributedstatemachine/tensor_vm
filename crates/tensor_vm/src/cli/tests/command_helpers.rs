use super::*;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{Address, Hash};
use clap::Parser;

pub(super) fn parse_test_cli(
    args: &[&str],
) -> std::result::Result<super::TvmdCommand, clap::Error> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("tvmd");
    argv.extend_from_slice(args);
    TvmdCli::try_parse_from(argv).map(|cli| cli.command)
}

pub(super) fn execute_test_cli_command(
    cli_command: &super::TvmdCommand,
) -> std::result::Result<String, String> {
    crate::app::execute_tvmd_command(cli_command)
}

pub(super) fn execute_test_cli_args(args: &[&str]) -> std::result::Result<String, String> {
    let command = parse_test_cli(args).expect("test CLI args must parse");
    execute_test_cli_command(&command)
}

pub(super) fn block_height_window_args(first_block: u64, last_block: u64) -> BlockHeightWindowArgs {
    BlockHeightWindowArgs {
        first_block,
        last_block,
    }
}

pub(super) fn multiaddr_arg(value: String) -> libp2p::Multiaddr {
    value.parse().expect("test multiaddr must parse")
}

pub(super) fn hash_arg(value: Hash) -> HashArg {
    HashArg(value)
}

pub(super) fn evidence_bundle_id_args(bundle_id: Hash) -> EvidenceBundleIdArgs {
    EvidenceBundleIdArgs {
        bundle_id: HashArg(bundle_id),
    }
}

pub(super) fn operator_id_args(operator_id: Hash) -> OperatorIdArgs {
    OperatorIdArgs {
        operator_id: HashArg(operator_id),
    }
}

pub(super) fn address_arg(value: Address) -> AddressArg {
    AddressArg(value)
}

pub(super) fn manifest_signer_args(manifest_signer: Address) -> ManifestSignerArgs {
    ManifestSignerArgs {
        manifest_signer: AddressArg(manifest_signer),
    }
}

pub(super) fn observation_timestamp_args(observed_at: u64) -> ObservationTimestampArgs {
    ObservationTimestampArgs { observed_at }
}

pub(super) fn network_observation_target_args(
    operator_id: Hash,
    listen_address: &str,
    observed_at: u64,
) -> NetworkObservationTargetArgs {
    NetworkObservationTargetArgs {
        operator: operator_id_args(operator_id),
        listen_address: multiaddr_arg(listen_address.to_owned()),
        observation: observation_timestamp_args(observed_at),
    }
}

pub(super) fn network_observation_protocol_counts_args(
    gossip_topic_count: u64,
    request_response_protocol_count: u64,
    bootstrap_peer_count: u64,
) -> NetworkObservationProtocolCountsArgs {
    NetworkObservationProtocolCountsArgs {
        gossip_topic_count,
        request_response_protocol_count,
        bootstrap_peer_count,
    }
}

pub(super) fn network_observation_transport_limits_args(
    max_transmit_bytes: u64,
    request_timeout_seconds: u64,
    max_concurrent_streams: u64,
    idle_timeout_seconds: u64,
) -> NetworkObservationTransportLimitsArgs {
    NetworkObservationTransportLimitsArgs {
        max_transmit_bytes,
        request_timeout_seconds,
        max_concurrent_streams,
        idle_timeout_seconds,
    }
}

pub(super) fn publication_bundle_args(bundle_id: Hash, public_uri: &str) -> PublicationBundleArgs {
    PublicationBundleArgs {
        bundle: evidence_bundle_id_args(bundle_id),
        public_uri: public_uri.to_owned(),
    }
}

pub(super) fn run_window_context_args(
    bundle_id: Hash,
    manifest_signer: Address,
) -> RunWindowContextArgs {
    RunWindowContextArgs {
        bundle: evidence_bundle_id_args(bundle_id),
        signer: manifest_signer_args(manifest_signer),
    }
}

pub(super) fn record_artifact_locator_args(artifact_uri: &str) -> RecordArtifactLocatorArgs {
    RecordArtifactLocatorArgs {
        artifact_uri: artifact_uri.to_owned(),
    }
}

pub(super) fn record_file_args(record_file: std::path::PathBuf) -> RecordFileArgs {
    RecordFileArgs { record_file }
}

pub(super) fn record_root_args(record_root: Hash, record_count: u64) -> RecordRootArgs {
    RecordRootArgs {
        record_root: HashArg(record_root),
        record_count,
    }
}

pub(super) fn record_roots_args(record_roots: Vec<Hash>) -> RecordRootsArgs {
    RecordRootsArgs {
        record_roots: record_roots.into_iter().map(HashArg).collect(),
    }
}

pub(super) fn record_context_args(
    kind: PublicEvidenceRecordKind,
) -> PublicEvidenceRecordContextArgs {
    record_context_args_from(
        kind,
        crate::types::hash_bytes(b"test", &[b"public-evidence-bundle"]),
        crate::types::address(b"public-evidence-publisher"),
    )
}

pub(super) fn record_context_args_from(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
) -> PublicEvidenceRecordContextArgs {
    PublicEvidenceRecordContextArgs {
        kind: record_kind_arg(kind),
        bundle: evidence_bundle_id_args(bundle_id),
        signer: manifest_signer_args(manifest_signer),
    }
}

pub(super) fn service_health_path_args(health_path: &str) -> ServiceHealthPathArgs {
    ServiceHealthPathArgs {
        health_path: health_path.to_owned(),
    }
}

pub(super) fn service_kind_arg(kind: PublicServiceKind) -> PublicServiceKindArg {
    match kind {
        PublicServiceKind::Rpc => PublicServiceKindArg::Rpc,
        PublicServiceKind::Explorer => PublicServiceKindArg::Explorer,
        PublicServiceKind::Faucet => PublicServiceKindArg::Faucet,
        PublicServiceKind::Telemetry => PublicServiceKindArg::Telemetry,
    }
}

pub(super) fn node_role_arg(role: PublicNodeRole) -> PublicNodeRoleArg {
    match role {
        PublicNodeRole::Miner => PublicNodeRoleArg::Miner,
        PublicNodeRole::Validator => PublicNodeRoleArg::Validator,
    }
}

pub(super) fn record_kind_arg(kind: PublicEvidenceRecordKind) -> PublicEvidenceRecordKindArg {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => PublicEvidenceRecordKindArg::BlockHistory,
        PublicEvidenceRecordKind::FinalityHistory => PublicEvidenceRecordKindArg::FinalityHistory,
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            PublicEvidenceRecordKindArg::NetworkRuntime
        }
        PublicEvidenceRecordKind::RandomnessBeaconEvidence => {
            PublicEvidenceRecordKindArg::RandomnessBeacon
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            PublicEvidenceRecordKindArg::DataAvailability
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => PublicEvidenceRecordKindArg::InvalidWork,
        PublicEvidenceRecordKind::RewardSettlements => {
            PublicEvidenceRecordKindArg::RewardSettlement
        }
        PublicEvidenceRecordKind::DetectionMeasurements => {
            PublicEvidenceRecordKindArg::DetectionMeasurement
        }
    }
}
