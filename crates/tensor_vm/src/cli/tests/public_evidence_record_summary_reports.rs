use super::*;

#[test]
fn execute_public_evidence_record_summary_and_artifact_reports_outputs() {
    let record_cases: [(PublicEvidenceRecordKind, &[u8], u64, &str, String); 8] = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10,
            "block_history",
            hex(&manifest_bundle().block_history_signature),
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10,
            "finality_history",
            hex(&manifest_bundle().finality_history_signature),
        ),
        (
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            b"network-runtime-root",
            3,
            "network_runtime_observation",
            hex(&manifest_bundle().network_runtime_observation_signature),
        ),
        (
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            b"randomness-beacon-root",
            10,
            "randomness_beacon",
            hex(&manifest_bundle().randomness_beacon_signature),
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20,
            "data_availability_measurement",
            hex(&manifest_bundle().data_availability_measurement_signature),
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1,
            "invalid_work_rejection",
            hex(&manifest_bundle().invalid_work_rejection_signature),
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1,
            "reward_settlement",
            hex(&manifest_bundle().reward_settlement_signature),
        ),
        (
            PublicEvidenceRecordKind::DetectionMeasurements,
            b"detection-measurement-root",
            1,
            "detection_measurement",
            hex(&manifest_bundle().detection_measurement_signature),
        ),
    ];

    for (kind, root_label, count, field_prefix, expected_signature) in record_cases {
        let record_root = if matches!(kind, PublicEvidenceRecordKind::NetworkRuntimeObservations) {
            manifest_bundle().network_runtime_observation_root
        } else {
            hash_bytes(b"test", &[root_label])
        };
        let root = hex(&record_root);
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let manifest_signer = address(b"public-evidence-publisher");
        let line = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::Summary(RecordSummaryArgs {
                context: record_context_args_from(kind, bundle_id, manifest_signer),
                root: record_root_args(record_root, count),
            }),
        ))
        .unwrap();
        assert_eq!(
            line,
            format!(
                "{field_prefix}_records={count}\n{field_prefix}_root={root}\n{field_prefix}_signature={expected_signature}"
            )
        );

        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}/{}.json",
            manifest_hash(b"public-evidence-bundle"),
            public_evidence_record_kind_tag(kind)
        );
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &manifest_signer,
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            count,
        );
        let artifact_line = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::Artifact(RecordArtifactArgs {
                context: record_context_args_from(kind, bundle_id, manifest_signer),
                artifact: record_artifact_locator_args(&artifact_uri),
                root: record_root_args(record_root, count),
            }),
        ))
        .unwrap();
        assert_eq!(
            artifact_line,
            format!(
                "record_artifact={},{artifact_uri},{root},{count},{}",
                public_evidence_record_kind_tag(kind),
                hex(&artifact_signature)
            )
        );
    }
}
