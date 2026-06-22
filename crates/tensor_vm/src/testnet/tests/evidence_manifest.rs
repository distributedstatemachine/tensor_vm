use super::*;

#[test]
fn public_testnet_evidence_manifest_parses_into_bundle() {
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let manifest = complete_public_evidence_manifest_text();
    let parsed = parse_public_testnet_evidence_manifest(&manifest).unwrap();

    assert_eq!(parsed, complete_public_evidence_bundle());
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .has_independent_auditor_records
    );
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .has_invalid_work_rejection_records
    );
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .has_reward_settlement_record_summary
    );
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .has_public_supporting_record_artifacts
    );
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .run_evidence
            .has_deployed_public_service_content
    );
    assert!(
        parsed
            .evaluate(&criteria, 6)
            .run_evidence
            .public_criterion_met
    );
    assert!(parsed.evaluate(&criteria, 6).independently_checkable);
    assert!(!parsed.evaluate(&criteria, 6).full_spec_evidence_met);

    let false_runtime = manifest.replace("libp2p_runtime_used=true", "libp2p_runtime_used=false");
    let parsed_false_runtime = parse_public_testnet_evidence_manifest(&false_runtime).unwrap();
    assert!(!parsed_false_runtime.run.network_runtime.libp2p_runtime_used);
    assert!(
        !parsed_false_runtime
            .evaluate(&criteria, 6)
            .full_spec_evidence_met
    );

    let local_rpc_service = manifest.replace(
        "https://rpc.tensorvm.net/health",
        "https://localhost/health",
    );
    let parsed_local_rpc_service =
        parse_public_testnet_evidence_manifest(&local_rpc_service).unwrap();
    let local_rpc_report = parsed_local_rpc_service.evaluate(&criteria, 6);
    assert!(!local_rpc_report.run_evidence.has_deployed_rpc_service);
    assert!(!local_rpc_report.run_evidence.has_deployed_public_services);
    assert!(!local_rpc_report.full_spec_evidence_met);

    let trailing_public_uri = manifest.replace(
        "public_uri=https://tensorvm.net/tensorvm/public-evidence.json",
        "public_uri=https://tensorvm.net/tensorvm/public-evidence.json ",
    );
    assert!(parse_public_testnet_evidence_manifest(&trailing_public_uri).is_err());

    let auditor_uri = manifest_auditor_uri();
    let auditor_uri_with_space = manifest.replace(
        &format!("{auditor_uri},1700000060"),
        &format!("{auditor_uri} ,1700000060"),
    );
    assert!(parse_public_testnet_evidence_manifest(&auditor_uri_with_space).is_err());

    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let block_artifact_uri =
        public_evidence_supporting_artifact_uri(&bundle_id, PublicEvidenceRecordKind::BlockHistory);
    let artifact_uri_with_space = manifest.replace(
        &format!("record_artifact=block-history,{block_artifact_uri},"),
        &format!("record_artifact=block-history,{block_artifact_uri} ,"),
    );
    assert!(parse_public_testnet_evidence_manifest(&artifact_uri_with_space).is_err());

    let miner_operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
    let operator_uri = manifest_operator_identity_uri(&miner_operator_id);
    let operator_uri_with_space = manifest.replace(
        &format!("{operator_uri},1700000000"),
        &format!(" {operator_uri},1700000000"),
    );
    assert!(parse_public_testnet_evidence_manifest(&operator_uri_with_space).is_err());

    let service_url_with_space = manifest.replace(
        "https://rpc.tensorvm.net/health,/health",
        "https://rpc.tensorvm.net/health ,/health",
    );
    assert!(parse_public_testnet_evidence_manifest(&service_url_with_space).is_err());

    let service_content_url_with_space = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        "https://rpc.tensorvm.net/chain/head ,/chain/head",
    );
    assert!(parse_public_testnet_evidence_manifest(&service_content_url_with_space).is_err());

    let missing_operator_lines = manifest_without_line(&manifest, "operator=");
    let parsed_missing_operator_lines =
        parse_public_testnet_evidence_manifest(&missing_operator_lines).unwrap();
    let missing_operator_report = parsed_missing_operator_lines.evaluate(&criteria, 6);
    assert!(!missing_operator_report.has_operator_identity_attestations);
    assert!(!missing_operator_report.independently_checkable);
    assert!(!missing_operator_report.full_spec_evidence_met);

    let missing_auditor_lines = manifest_without_line(&manifest, "auditor=");
    let parsed_missing_auditor_lines =
        parse_public_testnet_evidence_manifest(&missing_auditor_lines).unwrap();
    let missing_auditor_report = parsed_missing_auditor_lines.evaluate(&criteria, 6);
    assert!(!missing_auditor_report.has_independent_auditor_records);
    assert!(!missing_auditor_report.independently_checkable);
    assert!(!missing_auditor_report.full_spec_evidence_met);

    let missing_artifact_lines = manifest_without_line(&manifest, "record_artifact=");
    let parsed_missing_artifact_lines =
        parse_public_testnet_evidence_manifest(&missing_artifact_lines).unwrap();
    let missing_artifact_report = parsed_missing_artifact_lines.evaluate(&criteria, 6);
    assert!(!missing_artifact_report.has_public_supporting_record_artifacts);
    assert!(!missing_artifact_report.independently_checkable);
    assert!(!missing_artifact_report.full_spec_evidence_met);

    let missing_service_content_lines = manifest_without_line(&manifest, "service_content=");
    let parsed_missing_service_content =
        parse_public_testnet_evidence_manifest(&missing_service_content_lines).unwrap();
    let missing_service_content_report = parsed_missing_service_content.evaluate(&criteria, 6);
    assert!(
        !missing_service_content_report
            .run_evidence
            .has_deployed_public_service_content
    );
    assert!(!missing_service_content_report.full_spec_evidence_met);

    let uppercase_hash = manifest_hash(b"test", b"public-evidence-bundle").to_uppercase();
    assert_eq!(
        parse_hash_hex(&uppercase_hash).unwrap(),
        hash_bytes(b"test", &[b"public-evidence-bundle"])
    );
    assert!(parse_hash_hex(&format!("z{}", "0".repeat(63))).is_err());
}

#[test]
fn deployed_public_testnet_evidence_example_is_parseable_but_not_full_spec() {
    let manifest =
        include_str!("../../../../../deploy/tensorvm/manifests/public-testnet.evidence.example");
    assert_public_testnet_evidence_manifest_is_pending(manifest);
}

#[test]
fn docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec() {
    let manifest = include_str!("../../../../../docs/tensorvm/public-testnet.evidence");
    assert_public_testnet_evidence_manifest_is_pending(manifest);
}

fn assert_public_testnet_evidence_manifest_is_pending(manifest: &str) {
    let parsed = parse_public_testnet_evidence_manifest(manifest).unwrap();
    let report = parsed.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );

    assert!(!report.has_published_evidence_bundle);
    assert!(!report.has_independent_auditor_records);
    assert!(report.has_signed_run_window);
    assert!(report.has_block_history);
    assert!(report.has_finality_history);
    assert!(!report.has_operator_identity_attestations);
    assert!(!report.has_network_runtime_observations);
    assert!(!report.has_randomness_beacon_evidence);
    assert!(report.has_data_availability_measurements);
    assert!(report.has_invalid_work_rejection_records);
    assert!(report.has_reward_settlement_record_summary);
    assert!(!report.has_public_supporting_record_artifacts);
    assert!(!report.run_evidence.has_deployed_public_service_content);
    assert!(!report.independently_checkable);
    assert!(!report.run_evidence.public_criterion_met);
    assert!(!report.run_evidence.has_required_miners);
    assert!(!report.run_evidence.has_required_validators);
    assert!(!report.run_evidence.has_required_run_duration);
    assert!(!report.run_evidence.has_required_block_count);
    assert!(!report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_manifest_rejects_malformed_input() {
    let manifest = complete_public_evidence_manifest_text();
    let cases = [
        manifest_without_line(&manifest, "version="),
        manifest.replace(
            PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION,
            "tensor-vm-public-testnet-evidence-v0",
        ),
        manifest_without_line(&manifest, "bundle_id="),
        manifest_without_line(&manifest, "public_uri="),
        manifest_without_line(&manifest, "manifest_signer="),
        manifest_without_line(&manifest, "manifest_signature="),
        manifest_without_line(&manifest, "block_history_root="),
        manifest_without_line(&manifest, "block_history_signature="),
        manifest_without_line(&manifest, "finality_history_root="),
        manifest_without_line(&manifest, "finality_history_signature="),
        manifest_without_line(&manifest, "network_runtime_observation_records="),
        manifest_without_line(&manifest, "network_runtime_observation_root="),
        manifest_without_line(&manifest, "network_runtime_observation_signature="),
        manifest_without_line(&manifest, "data_availability_measurement_root="),
        manifest_without_line(&manifest, "data_availability_measurement_signature="),
        manifest_without_line(&manifest, "invalid_work_rejection_records="),
        manifest_without_line(&manifest, "invalid_work_rejection_root="),
        manifest_without_line(&manifest, "invalid_work_rejection_signature="),
        manifest_without_line(&manifest, "reward_settlement_root="),
        manifest_without_line(&manifest, "reward_settlement_signature="),
        manifest_without_line(&manifest, "run_started_at_unix_seconds="),
        manifest_without_line(&manifest, "run_ended_at_unix_seconds="),
        manifest_without_line(&manifest, "run_window_signature="),
        manifest_without_line(&manifest, "observed_blocks="),
        manifest_without_line(&manifest, "dos_controls_enabled="),
        manifest.replace("bundle_id=0x", "bundle_id=0x12"),
        manifest.replace("bundle_id=0x", "bundle_id=0xz"),
        format!("{manifest}\nobserved_blocks=10"),
        manifest.replace("bundle_id=", " bundle_id="),
        manifest.replace("bundle_id=", "bundle_id ="),
        manifest.replace("observed_blocks=10", "observed_blocks=10 "),
        manifest.replace("libp2p_runtime_used=true", "libp2p_runtime_used= true"),
        manifest.replace("manifest_signature_count=1", "manifest_signature_count=abc"),
        manifest.replace("dos_controls_enabled=true", "dos_controls_enabled=maybe"),
        manifest.replace("node=miner", "node=archive"),
        manifest.replace(
            "node=miner,",
            "node=miner,too,few,fields\n# removed original node=",
        ),
        manifest.replace("operator=miner", "operator=archive"),
        manifest.replace(
            "operator=miner,",
            "operator=miner,too,few,fields\n# removed original operator=",
        ),
        manifest.replace(
            "network_runtime_observation=",
            "network_runtime_observation=too,few,fields\n# removed original network_runtime_observation=",
        ),
        manifest.replace(
            "auditor=",
            "auditor=too,few,fields\n# removed original auditor=",
        ),
        manifest.replace("record_artifact=block-history", "record_artifact=archive"),
        manifest.replace(
            "record_artifact=block-history,",
            "record_artifact=block-history,too,few,fields\n# removed original record_artifact=",
        ),
        manifest.replace("service=rpc", "service=archive"),
        manifest.replace(
            "service=rpc,",
            "service=rpc,too,few,fields\n# removed original service=",
        ),
        manifest.replace("service_content=rpc", "service_content=archive"),
        manifest.replace(
            "service_content=rpc,",
            "service_content=rpc,too,few,fields\n# removed original service_content=",
        ),
        format!(
            "{}randomness_beacon_record={},1,{},{},unknown-kind,1,accepted\n",
            manifest,
            manifest_hash(b"test", b"randomness-source"),
            manifest_hash(b"test", b"randomness-root"),
            manifest_hash(b"test", b"randomness-proof")
        ),
        format!(
            "{}randomness_beacon_record={},1,{},{},drand-v1,1,pending\n",
            manifest,
            manifest_hash(b"test", b"randomness-source"),
            manifest_hash(b"test", b"randomness-root"),
            manifest_hash(b"test", b"randomness-proof")
        ),
        manifest.replace("reward_settlement_records=1", "unknown_field=1"),
        manifest.replace("reward_settlement_records=1", "malformed-line"),
    ];

    for case in cases {
        assert!(parse_public_testnet_evidence_manifest(&case).is_err());
    }
}
