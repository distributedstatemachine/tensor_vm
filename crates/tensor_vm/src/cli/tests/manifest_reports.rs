use super::super::{validate_public_evidence_manifest, validate_public_testnet_preflight_manifest};
use super::{assert_report_fields, evidence_manifest, preflight_manifest};

#[test]
fn validate_public_evidence_manifest_reports_default_criteria_status() {
    let report = validate_public_evidence_manifest(&evidence_manifest()).unwrap();
    assert_report_fields(
        &report,
        &[
            ("public_evidence_full_spec", "false"),
            ("public_criterion", "false"),
            ("independently_checkable", "true"),
            ("published_evidence_bundle", "true"),
            ("independent_auditor_records", "true"),
            ("signed_run_window", "true"),
            ("block_history", "true"),
            ("finality_history", "true"),
            ("operator_identity_attestations", "true"),
            ("network_runtime_observations", "true"),
            ("randomness_beacon_evidence", "true"),
            ("data_availability_measurements", "true"),
            ("signed_invalid_work_rejection_records", "true"),
            ("signed_reward_settlement_records", "true"),
            ("supporting_record_artifacts", "true"),
            ("cuda_verified_miners", "true"),
            ("cuda_graph_execution_evidence", "true"),
            ("miners", "2"),
            ("validators", "1"),
            ("run_started_at_unix_seconds", "1700000000"),
            ("run_ended_at_unix_seconds", "1700000060"),
            ("observed_duration_seconds", "60"),
            ("required_duration_seconds", "604800"),
            ("observed_blocks", "10"),
            ("required_blocks", "100800"),
            ("finality_rate_bps", "10000"),
            ("data_availability_bps", "9500"),
            ("invalid_receipts_submitted", "1"),
            ("invalid_receipts_rejected", "1"),
            ("invalid_work_rejection_rate_bps", "10000"),
            ("reward_settlement_records", "1"),
            ("cuda_verified_miner_count", "2"),
            ("cuda_graph_execution_receipts", "1"),
            ("external_operator_evidence", "true"),
            ("required_miners", "false"),
            ("required_validators", "false"),
            ("required_run_duration", "false"),
            ("required_block_count", "false"),
            ("required_finality", "true"),
            ("required_data_availability", "true"),
            ("invalid_work_rejection_evidence", "true"),
            ("reward_settlement_evidence", "true"),
            ("cuda_miner_evidence", "true"),
            ("cuda_graph_execution_receipt_evidence", "true"),
            ("production_libp2p_runtime", "true"),
            ("deployed_rpc_service", "true"),
            ("deployed_explorer_service", "true"),
            ("deployed_faucet_service", "true"),
            ("deployed_telemetry_service", "true"),
            ("deployed_public_service_content", "true"),
            ("deployed_public_services", "true"),
        ],
    );

    let insufficient_operator_records = evidence_manifest().replace(
        "operator_identity_attestation_records=3",
        "operator_identity_attestation_records=2",
    );
    let insufficient_operator_report =
        validate_public_evidence_manifest(&insufficient_operator_records).unwrap();
    assert_report_fields(
        &insufficient_operator_report,
        &[
            ("operator_identity_attestations", "false"),
            ("external_operator_evidence", "false"),
            ("public_criterion", "false"),
        ],
    );

    let missing_auditor_records = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("auditor="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_auditor_report =
        validate_public_evidence_manifest(&missing_auditor_records).unwrap();
    assert_report_fields(
        &missing_auditor_report,
        &[
            ("published_evidence_bundle", "true"),
            ("independent_auditor_records", "false"),
            ("independently_checkable", "false"),
        ],
    );

    let missing_artifacts = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("record_artifact="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_artifacts_report = validate_public_evidence_manifest(&missing_artifacts).unwrap();
    assert_report_fields(
        &missing_artifacts_report,
        &[
            ("supporting_record_artifacts", "false"),
            ("independently_checkable", "false"),
        ],
    );

    let missing_service_content = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("service_content="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_service_content_report =
        validate_public_evidence_manifest(&missing_service_content).unwrap();
    assert_report_fields(
        &missing_service_content_report,
        &[
            ("deployed_public_service_content", "false"),
            ("deployed_public_services", "false"),
        ],
    );

    assert!(validate_public_evidence_manifest("bad-manifest").is_err());
}

#[test]
fn validate_public_testnet_preflight_manifest_reports_launch_readiness() {
    let report = validate_public_testnet_preflight_manifest(&preflight_manifest()).unwrap();
    assert_report_fields(
        &report,
        &[
            ("public_testnet_preflight_ready", "true"),
            ("local_shape_ready", "true"),
            ("deployment_plan_ready", "true"),
            ("miners", "10"),
            ("validators", "5"),
            ("required_blocks", "100800"),
            ("required_miners", "true"),
            ("required_validators", "true"),
            ("positive_stakes", "true"),
            ("funded_faucet", "true"),
            ("cuda_kernels_available", "true"),
            ("cuda_ready_miner_count", "10"),
            ("cuda_ready_miners", "true"),
            ("libp2p_ready_node_count", "15"),
            ("libp2p_ready_nodes", "true"),
            ("production_libp2p_runtime", "true"),
            ("rpc_service_plan", "true"),
            ("explorer_service_plan", "true"),
            ("faucet_service_plan", "true"),
            ("telemetry_service_plan", "true"),
            ("public_service_content_planned", "true"),
            ("public_services_planned", "true"),
        ],
    );

    assert!(validate_public_testnet_preflight_manifest("bad-manifest").is_err());
}
