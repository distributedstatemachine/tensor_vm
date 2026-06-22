use super::*;

#[test]
fn execute_public_evidence_record_reports_outputs() {
    let supporting_record_cases = [
        (
            PublicEvidenceRecordKind::BlockHistory,
            "block_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "block_history",
        ),
        (
            PublicEvidenceRecordKind::FinalityHistory,
            "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,finalized",
            "finality_history",
        ),
        (
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,available,0",
            "data_availability_measurement",
        ),
        (
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            concat!(
                "randomness_beacon_record=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,7,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,",
                "drand-v1,9,accepted"
            ),
            "randomness_beacon",
        ),
        (
            PublicEvidenceRecordKind::InvalidWorkRejections,
            "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,rejected,0",
            "invalid_work_rejection",
        ),
        (
            PublicEvidenceRecordKind::RewardSettlements,
            concat!(
                "reward_settlement=",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc,0"
            ),
            "reward_settlement",
        ),
    ];
    for (kind, raw_line, field_prefix) in supporting_record_cases {
        let raw_root = supporting_record_root_from_line(
            kind,
            raw_line,
            supporting_record_line_prefix(kind).unwrap(),
        )
        .unwrap();
        assert_eq!(
            public_evidence_record_root_from_line(kind, raw_line).unwrap(),
            raw_root
        );
        let extra_root = hash_bytes(b"test", &[public_evidence_record_kind_tag(kind).as_bytes()]);
        let roots = vec![raw_root, extra_root];
        let aggregate_root = aggregate_public_evidence_record_roots(kind, &roots).unwrap();
        let raw_record_file = std::env::temp_dir().join(format!(
            "tensor-vm-{}-records-{}-{}.records",
            public_evidence_record_kind_tag(kind),
            std::process::id(),
            aggregate_root[0]
        ));
        std::fs::write(
            &raw_record_file,
            format!(
                "# raw supporting records\n{raw_line}\nrecord_root={}\n",
                hex(&extra_root)
            ),
        )
        .unwrap();
        let raw_record_file_path = raw_record_file.to_string_lossy().into_owned();
        assert_eq!(
            public_evidence_record_roots_from_file(kind, &raw_record_file_path).unwrap(),
            roots
        );
        let summary = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
                context: record_context_args(kind),
                file: record_file_args(raw_record_file.clone()),
            }),
        ))
        .unwrap();
        let signature = sign_public_evidence_record(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            summary,
            format!(
                "{field_prefix}_records=2\n{field_prefix}_root={}\n{field_prefix}_signature={}",
                hex(&aggregate_root),
                hex(&signature)
            )
        );
        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}.json",
            public_evidence_record_kind_tag(kind)
        );
        let artifact = execute_public_evidence_command(&EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
                context: record_context_args(kind),
                artifact: record_artifact_locator_args(&artifact_uri),
                file: record_file_args(raw_record_file.clone()),
            }),
        ))
        .unwrap();
        let artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            kind,
            &artifact_uri,
            &aggregate_root,
            roots.len() as u64,
        );
        assert_eq!(
            artifact,
            format!(
                "record_artifact={},{},{},2,{}",
                public_evidence_record_kind_tag(kind),
                artifact_uri,
                hex(&aggregate_root),
                hex(&artifact_signature)
            )
        );
        std::fs::remove_file(&raw_record_file).unwrap();
    }
}
