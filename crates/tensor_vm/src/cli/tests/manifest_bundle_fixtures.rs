use crate::testnet::{
    PublicEvidenceRecordSummaries, PublicNetworkRuntimeEvidence, PublicNodeEvidence,
    PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind, PublicTestnetEvidenceBundle,
    PublicTestnetRunEvidence,
};
use crate::types::{address, hash_bytes};

use super::manifest_network_fixtures::network_runtime_root_for_run;
use super::manifest_publication_fixtures::manifest_publication;
use super::manifest_service_fixtures::{public_service_content, public_service_url};

pub(super) fn manifest_bundle() -> PublicTestnetEvidenceBundle {
    let run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::miner(
                address(b"miner-b"),
                hash_bytes(b"test", &[b"miner-b-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::validator(
                address(b"validator-a"),
                hash_bytes(b"test", &[b"validator-a-operator"]),
                0,
                9,
                10,
            ),
        ],
        network_runtime: PublicNetworkRuntimeEvidence {
            libp2p_runtime_used: true,
            peer_discovery_observed: true,
            gossip_propagation_observed: true,
            request_response_observed: true,
            dos_controls_enabled: true,
        },
        services: vec![
            PublicServiceEvidence::new(
                PublicServiceKind::Rpc,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"rpc-service"]),
                    public_service_url(PublicServiceKind::Rpc),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Explorer,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"explorer-service"]),
                    public_service_url(PublicServiceKind::Explorer),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Faucet,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"faucet-service"]),
                    public_service_url(PublicServiceKind::Faucet),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Telemetry,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"telemetry-service"]),
                    public_service_url(PublicServiceKind::Telemetry),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
        ],
        service_content: vec![
            public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
            public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
            public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
            public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
        ],
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 10,
        checked_receipts: 20,
        available_receipts: 19,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
        cuda_verified_miner_count: 2,
        cuda_graph_execution_receipts: 1,
    };
    let network_runtime_observation_root = network_runtime_root_for_run(&run);
    PublicTestnetEvidenceBundle::new(
        run,
        manifest_publication(),
        PublicEvidenceRecordSummaries {
            block_history_records: 10,
            block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
            finality_history_records: 10,
            finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
            operator_identity_attestation_records: 3,
            network_runtime_observation_records: 3,
            network_runtime_observation_root,
            randomness_beacon_records: 10,
            randomness_beacon_root: hash_bytes(b"test", &[b"randomness-beacon-root"]),
            data_availability_measurement_records: 20,
            data_availability_measurement_root: hash_bytes(b"test", &[b"data-availability-root"]),
            invalid_work_rejection_records: 1,
            invalid_work_rejection_root: hash_bytes(b"test", &[b"invalid-work-root"]),
            reward_settlement_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
        },
    )
}
