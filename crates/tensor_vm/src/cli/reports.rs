use crate::app::KeyValueReportWriter;
use crate::chain::ChainParams;
use crate::error::Result;
use crate::testnet::{
    PublicTestnetCriteria, parse_public_testnet_evidence_manifest,
    parse_public_testnet_preflight_manifest,
};

pub fn validate_public_evidence_manifest(input: &str) -> Result<String> {
    let bundle = parse_public_testnet_evidence_manifest(input)?;
    let report = bundle.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );
    let run = &report.run_evidence;
    let mut output = KeyValueReportWriter::new();
    output.field("public_evidence_full_spec", report.full_spec_evidence_met);
    output.field("public_criterion", run.public_criterion_met);
    output.field("independently_checkable", report.independently_checkable);
    output.field(
        "published_evidence_bundle",
        report.has_published_evidence_bundle,
    );
    output.field(
        "independent_auditor_records",
        report.has_independent_auditor_records,
    );
    output.field("signed_run_window", report.has_signed_run_window);
    output.field("block_history", report.has_block_history);
    output.field("finality_history", report.has_finality_history);
    output.field(
        "operator_identity_attestations",
        report.has_operator_identity_attestations,
    );
    output.field(
        "network_runtime_observations",
        report.has_network_runtime_observations,
    );
    output.field(
        "deployed_public_service_evidence",
        report.has_deployed_public_service_evidence,
    );
    output.field(
        "randomness_beacon_evidence",
        report.has_randomness_beacon_evidence,
    );
    output.field(
        "data_availability_measurements",
        report.has_data_availability_measurements,
    );
    output.field(
        "signed_invalid_work_rejection_records",
        report.has_invalid_work_rejection_records,
    );
    output.field(
        "signed_reward_settlement_records",
        report.has_reward_settlement_record_summary,
    );
    output.field(
        "signed_detection_measurement_records",
        report.has_deployed_detection_measurement_records,
    );
    output.field(
        "signed_validator_vrf_lifecycle_records",
        report.has_validator_vrf_lifecycle_record_summary,
    );
    output.field(
        "supporting_record_artifacts",
        report.has_public_supporting_record_artifacts,
    );
    output.field("cuda_verified_miners", report.has_cuda_verified_miners);
    output.field(
        "cuda_graph_execution_evidence",
        report.has_cuda_graph_execution_evidence,
    );
    output.field(
        "validator_vrf_lifecycle_evidence",
        run.has_validator_vrf_lifecycle_evidence,
    );
    output.field("miners", run.miner_count);
    output.field("validators", run.validator_count);
    output.field(
        "run_started_at_unix_seconds",
        run.run_started_at_unix_seconds,
    );
    output.field("run_ended_at_unix_seconds", run.run_ended_at_unix_seconds);
    output.field("observed_duration_seconds", run.observed_duration_seconds);
    output.field("required_duration_seconds", run.required_duration_seconds);
    output.field("observed_blocks", run.observed_blocks);
    output.field("required_blocks", run.required_blocks);
    output.field("finality_rate_bps", run.finality_rate_bps);
    output.field("data_availability_bps", run.data_availability_bps);
    output.field("invalid_receipts_submitted", run.invalid_receipts_submitted);
    output.field("invalid_receipts_rejected", run.invalid_receipts_rejected);
    output.field(
        "invalid_work_rejection_rate_bps",
        run.invalid_work_rejection_rate_bps,
    );
    output.field("reward_settlement_records", run.reward_settlement_records);
    output.field("cuda_verified_miner_count", run.cuda_verified_miner_count);
    output.field(
        "cuda_graph_execution_receipts",
        run.cuda_graph_execution_receipts,
    );
    output.field(
        "validator_vrf_lifecycle_records",
        run.validator_vrf_lifecycle_records,
    );
    output.field(
        "detection_measurement_records",
        run.detection_measurement_records,
    );
    output.field("external_operator_evidence", run.external_operator_evidence);
    output.field("required_miners", run.has_required_miners);
    output.field("required_validators", run.has_required_validators);
    output.field("required_run_duration", run.has_required_run_duration);
    output.field("required_block_count", run.has_required_block_count);
    output.field("required_finality", run.has_required_finality);
    output.field(
        "required_data_availability",
        run.has_required_data_availability,
    );
    output.field(
        "invalid_work_rejection_evidence",
        run.has_invalid_work_rejection_evidence,
    );
    output.field(
        "reward_settlement_evidence",
        run.has_reward_settlement_records,
    );
    output.field("cuda_miner_evidence", run.has_cuda_verified_miners);
    output.field(
        "cuda_graph_execution_receipt_evidence",
        run.has_cuda_graph_execution_evidence,
    );
    output.field(
        "validator_vrf_lifecycle_record_evidence",
        run.has_validator_vrf_lifecycle_evidence,
    );
    output.field(
        "deployed_detection_measurement_evidence",
        run.has_deployed_detection_measurements,
    );
    output.field(
        "production_libp2p_runtime",
        run.has_production_libp2p_runtime,
    );
    output.field("deployed_rpc_service", run.has_deployed_rpc_service);
    output.field(
        "deployed_explorer_service",
        run.has_deployed_explorer_service,
    );
    output.field("deployed_faucet_service", run.has_deployed_faucet_service);
    output.field(
        "deployed_telemetry_service",
        run.has_deployed_telemetry_service,
    );
    output.field(
        "deployed_public_service_content",
        run.has_deployed_public_service_content,
    );
    output.field("deployed_public_services", run.has_deployed_public_services);
    Ok(output.finish())
}

pub fn validate_public_testnet_preflight_manifest(input: &str) -> Result<String> {
    let plan = parse_public_testnet_preflight_manifest(input)?;
    let report = plan.evaluate(ChainParams::default().block_time_seconds);
    let mut output = KeyValueReportWriter::new();
    output.field(
        "public_testnet_preflight_ready",
        report.can_start_public_run,
    );
    output.field("local_shape_ready", report.local_shape_ready);
    output.field("deployment_plan_ready", report.deployment_plan_ready);
    output.field("miners", report.miner_count);
    output.field("validators", report.validator_count);
    output.field("required_blocks", report.required_blocks);
    output.field("required_miners", report.has_required_miners);
    output.field("required_validators", report.has_required_validators);
    output.field("positive_stakes", report.has_positive_stakes);
    output.field("funded_faucet", report.has_funded_faucet);
    output.field("cuda_kernels_available", report.has_cuda_kernels_available);
    output.field("cuda_ready_miner_count", report.cuda_ready_miner_count);
    output.field("cuda_ready_miners", report.has_cuda_ready_miners);
    output.field("libp2p_ready_node_count", report.libp2p_ready_node_count);
    output.field("libp2p_ready_nodes", report.has_libp2p_ready_nodes);
    output.field(
        "production_libp2p_runtime",
        report.has_production_libp2p_runtime,
    );
    output.field("rpc_service_plan", report.has_rpc_service_plan);
    output.field("explorer_service_plan", report.has_explorer_service_plan);
    output.field("faucet_service_plan", report.has_faucet_service_plan);
    output.field("telemetry_service_plan", report.has_telemetry_service_plan);
    output.field(
        "public_service_content_planned",
        report.has_public_service_content_plan,
    );
    output.field("public_services_planned", report.has_public_service_plan);
    Ok(output.finish())
}
