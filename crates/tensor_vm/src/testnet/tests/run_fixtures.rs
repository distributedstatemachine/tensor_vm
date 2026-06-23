use super::*;
use crate::hash::hex;
use crate::types::{address, hash_bytes};

pub(super) fn production_runtime_evidence() -> PublicNetworkRuntimeEvidence {
    PublicNetworkRuntimeEvidence {
        libp2p_runtime_used: true,
        peer_discovery_observed: true,
        gossip_propagation_observed: true,
        request_response_observed: true,
        dos_controls_enabled: true,
    }
}

pub(super) fn public_service(
    kind: PublicServiceKind,
    label: &[u8],
    first_seen_block: u64,
    last_seen_block: u64,
) -> PublicServiceEvidence {
    public_service_with_observations(kind, label, first_seen_block, last_seen_block, 10)
}

pub(super) fn public_service_with_observations(
    kind: PublicServiceKind,
    label: &[u8],
    first_seen_block: u64,
    last_seen_block: u64,
    observation_count: u64,
) -> PublicServiceEvidence {
    PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[label]),
            public_service_url(kind),
            "/health",
        ),
        first_seen_block,
        last_seen_block,
        observation_count,
        observation_count,
    )
}

pub(super) fn public_service_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
    }
}

pub(super) fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
    }
}

pub(super) fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "/chain/head",
        PublicServiceKind::Explorer => "/explorer",
        PublicServiceKind::Faucet => "/faucet/page",
        PublicServiceKind::Telemetry => "/telemetry/dashboard",
    }
}

pub(super) fn public_service_content(
    kind: PublicServiceKind,
    label: &[u8],
) -> PublicServiceContentEvidence {
    PublicServiceContentEvidence::new(
        kind,
        hash_bytes(b"test", &[label]),
        public_service_content_url(kind),
        public_service_content_path(kind),
        hash_bytes(b"test", &[label, b"content-root"]),
        1_700_000_000,
        64,
    )
}

pub(super) fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
    let content = public_service_content(kind, label);
    format!(
        "service_content={},{},{},{},{},{},{},{}",
        service_kind_tag(kind),
        hex(&content.endpoint_id),
        content.public_url,
        content.content_path,
        hex(&content.content_root),
        content.observed_at_unix_seconds,
        content.min_content_bytes,
        hex(&content.content_signature)
    )
}

pub(super) fn service_kind_tag(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "rpc",
        PublicServiceKind::Explorer => "explorer",
        PublicServiceKind::Faucet => "faucet",
        PublicServiceKind::Telemetry => "telemetry",
    }
}

pub(super) fn deployed_public_services(last_seen_block: u64) -> Vec<PublicServiceEvidence> {
    vec![
        public_service(PublicServiceKind::Rpc, b"rpc-service", 0, last_seen_block),
        public_service(
            PublicServiceKind::Explorer,
            b"explorer-service",
            0,
            last_seen_block,
        ),
        public_service(
            PublicServiceKind::Faucet,
            b"faucet-service",
            0,
            last_seen_block,
        ),
        public_service(
            PublicServiceKind::Telemetry,
            b"telemetry-service",
            0,
            last_seen_block,
        ),
    ]
}

pub(super) fn deployed_public_service_content() -> Vec<PublicServiceContentEvidence> {
    vec![
        public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
        public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
        public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
        public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
    ]
}

pub(super) fn complete_public_run_evidence() -> PublicTestnetRunEvidence {
    PublicTestnetRunEvidence {
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
        network_runtime: production_runtime_evidence(),
        services: deployed_public_services(9),
        service_content: deployed_public_service_content(),
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 10,
        checked_receipts: 20,
        available_receipts: 20,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
        cuda_verified_miner_count: 2,
        cuda_graph_execution_receipts: 1,
        validator_vrf_lifecycle_records: 40,
        detection_measurement_records: 1,
    }
}

pub(super) fn complete_public_evidence_bundle() -> PublicTestnetEvidenceBundle {
    let run = complete_public_run_evidence();
    let network_runtime_observation_root = network_runtime_root_for_run(&run);
    PublicTestnetEvidenceBundle::new(
        run,
        PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        ),
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
            detection_measurement_records: 1,
            detection_measurement_root: hash_bytes(b"test", &[b"detection-measurement-root"]),
            validator_vrf_lifecycle_records: 40,
            validator_vrf_lifecycle_root: hash_bytes(b"test", &[b"validator-vrf-lifecycle-root"]),
        },
    )
}

pub(super) fn compact_full_spec_block_time_seconds() -> u64 {
    required_duration_seconds_for_days(PublicTestnetCriteria::default().duration_days) / 20
}

pub(super) fn full_spec_public_evidence_bundle(
    block_time_seconds: u64,
) -> PublicTestnetEvidenceBundle {
    let criteria = PublicTestnetCriteria::default();
    let observed_blocks =
        required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
    let last_seen_block = observed_blocks.saturating_sub(1);
    let run_started_at_unix_seconds = 1_700_000_000;
    let run_ended_at_unix_seconds =
        run_started_at_unix_seconds + required_duration_seconds_for_days(criteria.duration_days);
    let mut nodes = Vec::new();
    for index in 0..criteria.min_miners {
        nodes.push(PublicNodeEvidence::miner(
            address(format!("full-spec-miner-{index}").as_bytes()),
            hash_bytes(
                b"test",
                &[format!("full-spec-miner-{index}-operator").as_bytes()],
            ),
            0,
            last_seen_block,
            observed_blocks,
        ));
    }
    for index in 0..criteria.min_validators {
        nodes.push(PublicNodeEvidence::validator(
            address(format!("full-spec-validator-{index}").as_bytes()),
            hash_bytes(
                b"test",
                &[format!("full-spec-validator-{index}-operator").as_bytes()],
            ),
            0,
            last_seen_block,
            observed_blocks,
        ));
    }
    let operator_records = nodes.len() as u64;
    let checked_receipts = observed_blocks;
    let run = PublicTestnetRunEvidence {
        nodes,
        network_runtime: production_runtime_evidence(),
        services: vec![
            public_service_with_observations(
                PublicServiceKind::Rpc,
                b"rpc-service",
                0,
                last_seen_block,
                observed_blocks,
            ),
            public_service_with_observations(
                PublicServiceKind::Explorer,
                b"explorer-service",
                0,
                last_seen_block,
                observed_blocks,
            ),
            public_service_with_observations(
                PublicServiceKind::Faucet,
                b"faucet-service",
                0,
                last_seen_block,
                observed_blocks,
            ),
            public_service_with_observations(
                PublicServiceKind::Telemetry,
                b"telemetry-service",
                0,
                last_seen_block,
                observed_blocks,
            ),
        ],
        service_content: deployed_public_service_content(),
        run_started_at_unix_seconds,
        run_ended_at_unix_seconds,
        observed_blocks,
        finalized_blocks: observed_blocks,
        checked_receipts,
        available_receipts: checked_receipts,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
        cuda_verified_miner_count: criteria.min_miners as u64,
        cuda_graph_execution_receipts: 1,
        validator_vrf_lifecycle_records: checked_receipts.saturating_mul(2),
        detection_measurement_records: 1,
    };
    let network_runtime_observation_root = network_runtime_root_for_run(&run);
    let block_history_raw_records = full_spec_block_history_records(observed_blocks);
    let block_history_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::BlockHistory,
        &block_history_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated block history roots should aggregate");
    let finality_history_raw_records =
        full_spec_finality_history_records(&block_history_raw_records);
    let finality_history_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::FinalityHistory,
        &finality_history_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated finality history roots should aggregate");
    let randomness_beacon_raw_records = full_spec_randomness_beacon_records(observed_blocks);
    let randomness_beacon_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::RandomnessBeaconEvidence,
        &randomness_beacon_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated randomness beacon roots should aggregate");
    let data_availability_raw_records = full_spec_data_availability_records(checked_receipts);
    let data_availability_measurement_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::DataAvailabilityMeasurements,
        &data_availability_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated data availability roots should aggregate");
    let invalid_work_raw_records = full_spec_invalid_work_records();
    let invalid_work_rejection_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::InvalidWorkRejections,
        &invalid_work_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated invalid work roots should aggregate");
    let reward_settlement_raw_records = full_spec_reward_settlement_records();
    let reward_settlement_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::RewardSettlements,
        &reward_settlement_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated reward settlement roots should aggregate");
    let detection_measurement_raw_records = full_spec_detection_measurement_records();
    let detection_measurement_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::DetectionMeasurements,
        &detection_measurement_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated detection measurement roots should aggregate");
    let validator_vrf_lifecycle_raw_records =
        full_spec_validator_vrf_lifecycle_records(checked_receipts);
    let validator_vrf_lifecycle_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::ValidatorVrfLifecycle,
        &validator_vrf_lifecycle_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("generated validator vrf lifecycle roots should aggregate");
    let mut bundle = PublicTestnetEvidenceBundle::new(
        run,
        PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"full-spec-public-evidence-bundle"]),
            String::from("https://tensorvm.net/tensorvm/full-spec-public-evidence.json"),
            address(b"full-spec-public-evidence-publisher"),
            1,
            1,
        ),
        PublicEvidenceRecordSummaries {
            block_history_records: observed_blocks,
            block_history_root,
            finality_history_records: observed_blocks,
            finality_history_root,
            operator_identity_attestation_records: operator_records,
            network_runtime_observation_records: operator_records,
            network_runtime_observation_root,
            randomness_beacon_records: observed_blocks,
            randomness_beacon_root,
            data_availability_measurement_records: checked_receipts,
            data_availability_measurement_root,
            invalid_work_rejection_records: 1,
            invalid_work_rejection_root,
            reward_settlement_root,
            detection_measurement_records: 1,
            detection_measurement_root,
            validator_vrf_lifecycle_records: checked_receipts.saturating_mul(2),
            validator_vrf_lifecycle_root,
        },
    );
    bundle.block_history_raw_records = block_history_raw_records;
    bundle.finality_history_raw_records = finality_history_raw_records;
    bundle.randomness_beacon_raw_records = randomness_beacon_raw_records;
    bundle.data_availability_raw_records = data_availability_raw_records;
    bundle.invalid_work_raw_records = invalid_work_raw_records;
    bundle.reward_settlement_raw_records = reward_settlement_raw_records;
    bundle.detection_measurement_raw_records = detection_measurement_raw_records;
    bundle.validator_vrf_lifecycle_raw_records = validator_vrf_lifecycle_raw_records;
    bundle
}

pub(super) fn full_spec_block_history_records(
    observed_blocks: u64,
) -> Vec<PublicBlockHistoryRecord> {
    (0..observed_blocks)
        .map(|index| PublicBlockHistoryRecord {
            block: index,
            block_root: hash_bytes(
                b"test",
                &[format!("full-spec-public-block-root-{index}").as_bytes()],
            ),
        })
        .collect()
}

pub(super) fn full_spec_finality_history_records(
    block_history: &[PublicBlockHistoryRecord],
) -> Vec<PublicFinalityHistoryRecord> {
    block_history
        .iter()
        .map(|record| PublicFinalityHistoryRecord {
            block: record.block,
            block_root: record.block_root,
            status: PublicFinalityHistoryStatus::Finalized,
        })
        .collect()
}

pub(super) fn full_spec_randomness_beacon_records(
    observed_blocks: u64,
) -> Vec<PublicRandomnessBeaconRecord> {
    (0..observed_blocks)
        .map(|index| {
            PublicRandomnessBeaconRecord::accepted_public(
                hash_bytes(b"test", &[b"full-spec-public-drand-source"]),
                index + 1,
                hash_bytes(
                    b"test",
                    &[format!("full-spec-public-drand-randomness-{index}").as_bytes()],
                ),
                hash_bytes(
                    b"test",
                    &[format!("full-spec-public-drand-proof-{index}").as_bytes()],
                ),
                PublicRandomnessBeaconProofKind::DrandV1,
                index,
            )
        })
        .collect()
}

pub(super) fn full_spec_data_availability_records(
    checked_receipts: u64,
) -> Vec<PublicDataAvailabilityMeasurementRecord> {
    (0..checked_receipts)
        .map(|index| PublicDataAvailabilityMeasurementRecord {
            receipt_root: full_spec_checked_receipt_root(index),
            status: PublicDataAvailabilityStatus::Available,
            observed_block: index,
        })
        .collect()
}

pub(super) fn full_spec_invalid_work_records() -> Vec<PublicInvalidWorkRejectionRecord> {
    vec![PublicInvalidWorkRejectionRecord {
        receipt_root: hash_bytes(b"test", &[b"full-spec-invalid-work-receipt"]),
        observed_block: 1,
    }]
}

pub(super) fn full_spec_reward_settlement_records() -> Vec<PublicRewardSettlementRecord> {
    vec![PublicRewardSettlementRecord {
        receipt_root: hash_bytes(b"test", &[b"full-spec-reward-settlement-receipt"]),
        miner_id: address(b"full-spec-reward-settlement-miner"),
        validator_id: address(b"full-spec-reward-settlement-validator"),
        observed_block: 1,
    }]
}

pub(super) fn full_spec_detection_measurement_records() -> Vec<PublicDetectionMeasurementRecord> {
    vec![PublicDetectionMeasurementRecord {
        mechanism: String::from("full-freivalds"),
        subject_root: hash_bytes(b"test", &[b"full-spec-detection-measurement"]),
        sample_count: 20,
        detected_count: 20,
        observed_block: 1,
    }]
}

pub(super) fn full_spec_validator_vrf_lifecycle_records(
    checked_receipts: u64,
) -> Vec<PublicValidatorVrfLifecycleRecord> {
    (0..checked_receipts)
        .flat_map(|index| {
            let receipt_root = full_spec_checked_receipt_root(index);
            let validator_id =
                address(format!("full-spec-vrf-lifecycle-validator-{index}").as_bytes());
            let beacon_round = index + 1;
            [
                PublicValidatorVrfLifecycleRecord {
                    receipt_root,
                    validator_id,
                    beacon_round,
                    phase: PublicValidatorVrfLifecyclePhase::Committed,
                    observed_block: index,
                },
                PublicValidatorVrfLifecycleRecord {
                    receipt_root,
                    validator_id,
                    beacon_round,
                    phase: PublicValidatorVrfLifecyclePhase::Revealed,
                    observed_block: index + 1,
                },
            ]
        })
        .collect()
}

fn full_spec_checked_receipt_root(index: u64) -> Hash {
    hash_bytes(
        b"test",
        &[format!("full-spec-checked-available-receipt-{index}").as_bytes()],
    )
}

pub(super) fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
    let record_roots = public_network_runtime_observations_for_run(run)
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_roots,
    )
    .expect("generated network observation roots should aggregate")
}

pub(super) fn public_network_runtime_observation(
    operator_id: Hash,
    node_index: usize,
    observed_at_unix_seconds: u64,
) -> PublicNetworkRuntimeObservation {
    PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
        operator_id,
        peer_id: deterministic_public_network_peer_id(&operator_id),
        listen_address: format!(
            "/dns/role-order-node-{node_index}.tensorvm.net/tcp/{}",
            4_101 + node_index
        ),
        observed_at_unix_seconds,
        gossip_topic_count: 5,
        request_response_protocol_count: 3,
        bootstrap_peer_count: 2,
        max_transmit_bytes: 1_048_576,
        request_timeout_seconds: 10,
        max_concurrent_streams: 128,
        idle_connection_timeout_seconds: 60,
    })
}
