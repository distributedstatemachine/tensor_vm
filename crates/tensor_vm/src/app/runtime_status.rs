use super::{
    KeyValueReportWriter, RuntimeP2pReport, RuntimeStatusSnapshot, ServiceRuntimeConfig,
    hex_hash_list, randomness_beacon_hash_label, randomness_beacon_source_label,
    runtime_role_wallet_address_text,
};
use crate::hash::hex;

pub fn format_role_runtime_report(
    config: &ServiceRuntimeConfig,
    snapshot: &RuntimeStatusSnapshot,
    p2p: &RuntimeP2pReport<'_>,
) -> String {
    let network = &config.node.network;
    let network_events = snapshot.network_events;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_serve");
    report.field("runtime_command", config.runtime_command);
    report.field("role", config.role.label());
    report.field("chain_profile", config.node.profile.label());
    report.field("role_loop_ready", true);
    report.field(
        "role_can_produce_blocks",
        config.node.can_produce_local_blocks(),
    );
    report.field(
        "role_wallet_address",
        runtime_role_wallet_address_text(snapshot.role_wallet_address),
    );
    report.field(
        "role_wallet_registration",
        snapshot.role_wallet_registration,
    );
    report.field("role_wallet_registered", snapshot.role_wallet_registered);
    report.field("miner_work_ready", snapshot.miner_work_ready);
    report.field(
        "miner_assigned_jobs_seen",
        snapshot.miner_assigned_jobs_seen,
    );
    report.field("miner_unreceipted_jobs", snapshot.miner_unreceipted_jobs);
    report.field(
        "miner_receipts_submitted",
        snapshot.miner_receipts_submitted,
    );
    report.field("miner_tensors_inserted", snapshot.miner_tensors_inserted);
    report.field("validator_work_ready", snapshot.validator_work_ready);
    report.field(
        "validator_assigned_receipts_seen",
        snapshot.validator_assigned_receipts_seen,
    );
    report.field(
        "validator_unattested_receipts",
        snapshot.validator_unattested_receipts,
    );
    report.field(
        "validator_artifact_ready_receipts",
        snapshot.validator_artifact_ready_receipts,
    );
    report.field(
        "validator_artifact_missing_receipts",
        snapshot.validator_artifact_missing_receipts,
    );
    report.field(
        "validator_remote_tensor_fetch_attempts",
        snapshot.validator_remote_tensor_fetch_attempts,
    );
    report.field(
        "validator_remote_tensor_fetch_successes",
        snapshot.validator_remote_tensor_fetch_successes,
    );
    report.field(
        "validator_remote_tensor_fetch_failures",
        snapshot.validator_remote_tensor_fetch_failures,
    );
    report.field(
        "validator_remote_tensor_fetch_bytes",
        snapshot.validator_remote_tensor_fetch_bytes,
    );
    report.field(
        "validator_remote_tensors_inserted",
        snapshot.validator_remote_tensors_inserted,
    );
    report.field(
        "randomness_beacon_mode",
        config.randomness_beacon.mode.label(),
    );
    report.field(
        "randomness_beacon_configured",
        config.randomness_beacon.enabled(),
    );
    report.field(
        "randomness_beacon_configured_source",
        randomness_beacon_source_label(&config.randomness_beacon),
    );
    report.field(
        "randomness_beacon_configured_round",
        config.randomness_beacon.beacon_round,
    );
    report.field(
        "randomness_beacon_configured_randomness",
        randomness_beacon_hash_label(&config.randomness_beacon.randomness),
    );
    report.field(
        "randomness_beacon_configured_proof_hash",
        randomness_beacon_hash_label(&config.randomness_beacon.proof_hash),
    );
    report.field(
        "randomness_beacons_observed",
        snapshot.randomness_beacons_observed,
    );
    report.field(
        "randomness_beacons_applied",
        snapshot.randomness_beacons_applied,
    );
    report.field(
        "randomness_beacons_skipped",
        snapshot.randomness_beacons_skipped,
    );
    report.field(
        "randomness_beacon_failures",
        snapshot.randomness_beacon_failures,
    );
    report.field(
        "randomness_latest_source_id",
        nonempty_status_label(&snapshot.randomness_latest_source_id),
    );
    report.field("randomness_latest_round", snapshot.randomness_latest_round);
    report.field(
        "randomness_last_error",
        nonempty_status_label(&snapshot.randomness_last_error),
    );
    report.field(
        "validator_attestations_submitted",
        snapshot.validator_attestations_submitted,
    );
    report.field(
        "validator_audit_work_ready",
        snapshot.validator_audit_work_ready,
    );
    report.field(
        "validator_assigned_audits_seen",
        snapshot.validator_assigned_audits_seen,
    );
    report.field(
        "validator_unreported_audits",
        snapshot.validator_unreported_audits,
    );
    report.field(
        "validator_audit_artifact_ready",
        snapshot.validator_audit_artifact_ready,
    );
    report.field(
        "validator_audit_artifact_missing",
        snapshot.validator_audit_artifact_missing,
    );
    report.field(
        "validator_audit_reports_submitted",
        snapshot.validator_audit_reports_submitted,
    );
    report.field(
        "validator_proposer_work_ready",
        snapshot.validator_proposer_work_ready,
    );
    report.field(
        "validator_proposer_settled_receipts_seen",
        snapshot.validator_proposer_settled_receipts_seen,
    );
    report.field(
        "validator_proposer_artifact_ready_receipts_seen",
        snapshot.validator_proposer_artifact_ready_receipts_seen,
    );
    report.field(
        "validator_proposer_attested_receipts_seen",
        snapshot.validator_proposer_attested_receipts_seen,
    );
    report.field(
        "validator_blocks_proposed",
        snapshot.validator_blocks_proposed,
    );
    report.field(
        "validator_useful_blocks_proposed",
        snapshot.validator_useful_blocks_proposed,
    );
    report.field(
        "validator_fallback_blocks_proposed",
        snapshot.validator_fallback_blocks_proposed,
    );
    report.field(
        "validator_receipts_proposed",
        snapshot.validator_receipts_proposed,
    );
    report.field(
        "validator_block_votes_submitted",
        snapshot.validator_block_votes_submitted,
    );
    report.field("local_producer", snapshot.local_producer);
    report.field("listen", &network.rpc_listen);
    report.field("p2p_listen", &network.p2p_listen);
    report.field("p2p_runtime", "libp2p");
    report.field("p2p_peer_id", p2p.peer_id);
    report.field("p2p_connected_peers", snapshot.p2p_connected_peers);
    report.field(
        "p2p_observed_block_gossip_count",
        snapshot.p2p_observed_blocks,
    );
    report.field(
        "p2p_observed_block_payload_gossip_count",
        snapshot.p2p_observed_block_payloads,
    );
    report.field(
        "p2p_observed_block_vote_gossip_count",
        snapshot.p2p_observed_block_votes,
    );
    report.field("p2p_observed_job_gossip_count", snapshot.p2p_observed_jobs);
    report.field(
        "p2p_observed_receipt_gossip_count",
        snapshot.p2p_observed_receipts,
    );
    report.field(
        "p2p_observed_attestation_gossip_count",
        snapshot.p2p_observed_attestations,
    );
    report.field(
        "p2p_latest_observed_block_height",
        snapshot.p2p_latest_observed_block_height,
    );
    report.field(
        "p2p_latest_observed_block_hash",
        hex(&snapshot.p2p_latest_observed_block_hash),
    );
    report.field(
        "p2p_observed_block_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_hashes),
    );
    report.field(
        "p2p_latest_observed_block_payload_height",
        snapshot.p2p_latest_observed_block_payload_height,
    );
    report.field(
        "p2p_latest_observed_block_payload_hash",
        hex(&snapshot.p2p_latest_observed_block_payload_hash),
    );
    report.field(
        "p2p_observed_block_payload_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_payload_hashes),
    );
    report.field("p2p_gossipsub_topics", p2p.topics);
    report.field(
        "p2p_request_response_protocols",
        p2p.request_response_protocols,
    );
    report.field("p2p_bootstrap_peers", p2p.bootstrap_peer_count);
    report.append_report(p2p.identity);
    report.field("p2p_max_transmit_bytes", p2p.max_transmit_bytes);
    report.field("p2p_request_timeout_seconds", p2p.request_timeout_seconds);
    report.field("p2p_max_concurrent_streams", p2p.max_concurrent_streams);
    report.field("p2p_idle_timeout_seconds", p2p.idle_timeout_seconds);
    report.field("data_dir", config.node.data_dir().display());
    report.field("served_requests", snapshot.served_requests);
    report.field("produced_blocks", snapshot.produced_blocks);
    report.field("network_applied_blocks", snapshot.network_applied_blocks);
    report.field("network_events_ingested", network_events.events);
    report.field(
        "network_block_events_ingested",
        network_events.block_announcements,
    );
    report.field(
        "network_block_headers_ingested",
        network_events.block_headers,
    );
    report.field(
        "network_block_payloads_ingested",
        network_events.block_payloads,
    );
    report.field(
        "network_block_payloads_applied",
        network_events.block_payloads_applied,
    );
    report.field("network_block_votes_ingested", network_events.block_votes);
    report.field(
        "network_block_votes_applied",
        network_events.block_votes_applied,
    );
    report.field(
        "network_block_check_challenges_ingested",
        network_events.block_check_challenges,
    );
    report.field(
        "network_block_check_challenges_applied",
        network_events.block_check_challenges_applied,
    );
    report.field("network_job_events_ingested", network_events.jobs);
    report.field("network_job_payloads_ingested", network_events.job_payloads);
    report.field(
        "network_job_payloads_applied",
        network_events.job_payloads_applied,
    );
    report.field("network_receipt_events_ingested", network_events.receipts);
    report.field(
        "network_receipt_payloads_ingested",
        network_events.receipt_payloads,
    );
    report.field(
        "network_receipt_payloads_applied",
        network_events.receipt_payloads_applied,
    );
    report.field(
        "network_attestation_events_ingested",
        network_events.attestations,
    );
    report.field(
        "network_attestation_payloads_ingested",
        network_events.attestation_payloads,
    );
    report.field(
        "network_attestation_payloads_applied",
        network_events.attestation_payloads_applied,
    );
    report.field(
        "network_validator_audit_reports_ingested",
        network_events.validator_audit_reports,
    );
    report.field(
        "network_validator_audit_reports_applied",
        network_events.validator_audit_reports_applied,
    );
    report.field("network_peer_events_ingested", network_events.peers);
    report.field("network_invalid_events", network_events.invalid_events);
    report.finish()
}

pub fn write_role_runtime_status(
    config: &ServiceRuntimeConfig,
    snapshot: &RuntimeStatusSnapshot,
) -> std::result::Result<(), String> {
    let path = config.node.data_dir().join("role-runtime.status");
    let network_events = snapshot.network_events;
    let mut report = KeyValueReportWriter::new();
    report.field("role_runtime_command", config.runtime_command);
    report.field("role_loop_role", config.role.label());
    report.field("role_loop_ready", true);
    report.field("role_chain_profile", config.node.profile.label());
    report.field(
        "role_can_produce_blocks",
        config.node.can_produce_local_blocks(),
    );
    report.field(
        "role_wallet_address",
        runtime_role_wallet_address_text(snapshot.role_wallet_address),
    );
    report.field(
        "role_wallet_registration",
        snapshot.role_wallet_registration,
    );
    report.field("role_wallet_registered", snapshot.role_wallet_registered);
    report.field("role_miner_work_ready", snapshot.miner_work_ready);
    report.field(
        "role_miner_assigned_jobs_seen",
        snapshot.miner_assigned_jobs_seen,
    );
    report.field(
        "role_miner_unreceipted_jobs",
        snapshot.miner_unreceipted_jobs,
    );
    report.field(
        "role_miner_receipts_submitted",
        snapshot.miner_receipts_submitted,
    );
    report.field(
        "role_miner_tensors_inserted",
        snapshot.miner_tensors_inserted,
    );
    report.field("role_validator_work_ready", snapshot.validator_work_ready);
    report.field(
        "role_validator_assigned_receipts_seen",
        snapshot.validator_assigned_receipts_seen,
    );
    report.field(
        "role_validator_unattested_receipts",
        snapshot.validator_unattested_receipts,
    );
    report.field(
        "role_validator_artifact_ready_receipts",
        snapshot.validator_artifact_ready_receipts,
    );
    report.field(
        "role_validator_artifact_missing_receipts",
        snapshot.validator_artifact_missing_receipts,
    );
    report.field(
        "role_validator_remote_tensor_fetch_attempts",
        snapshot.validator_remote_tensor_fetch_attempts,
    );
    report.field(
        "role_validator_remote_tensor_fetch_successes",
        snapshot.validator_remote_tensor_fetch_successes,
    );
    report.field(
        "role_validator_remote_tensor_fetch_failures",
        snapshot.validator_remote_tensor_fetch_failures,
    );
    report.field(
        "role_validator_remote_tensor_fetch_bytes",
        snapshot.validator_remote_tensor_fetch_bytes,
    );
    report.field(
        "role_validator_remote_tensors_inserted",
        snapshot.validator_remote_tensors_inserted,
    );
    report.field(
        "role_randomness_beacon_mode",
        config.randomness_beacon.mode.label(),
    );
    report.field(
        "role_randomness_beacon_configured",
        config.randomness_beacon.enabled(),
    );
    report.field(
        "role_randomness_beacon_configured_source",
        randomness_beacon_source_label(&config.randomness_beacon),
    );
    report.field(
        "role_randomness_beacon_configured_round",
        config.randomness_beacon.beacon_round,
    );
    report.field(
        "role_randomness_beacon_configured_randomness",
        randomness_beacon_hash_label(&config.randomness_beacon.randomness),
    );
    report.field(
        "role_randomness_beacon_configured_proof_hash",
        randomness_beacon_hash_label(&config.randomness_beacon.proof_hash),
    );
    report.field(
        "role_randomness_beacons_observed",
        snapshot.randomness_beacons_observed,
    );
    report.field(
        "role_randomness_beacons_applied",
        snapshot.randomness_beacons_applied,
    );
    report.field(
        "role_randomness_beacons_skipped",
        snapshot.randomness_beacons_skipped,
    );
    report.field(
        "role_randomness_beacon_failures",
        snapshot.randomness_beacon_failures,
    );
    report.field(
        "role_randomness_latest_source_id",
        nonempty_status_label(&snapshot.randomness_latest_source_id),
    );
    report.field(
        "role_randomness_latest_round",
        snapshot.randomness_latest_round,
    );
    report.field(
        "role_randomness_last_error",
        nonempty_status_label(&snapshot.randomness_last_error),
    );
    report.field(
        "role_validator_attestations_submitted",
        snapshot.validator_attestations_submitted,
    );
    report.field(
        "role_validator_audit_work_ready",
        snapshot.validator_audit_work_ready,
    );
    report.field(
        "role_validator_assigned_audits_seen",
        snapshot.validator_assigned_audits_seen,
    );
    report.field(
        "role_validator_unreported_audits",
        snapshot.validator_unreported_audits,
    );
    report.field(
        "role_validator_audit_artifact_ready",
        snapshot.validator_audit_artifact_ready,
    );
    report.field(
        "role_validator_audit_artifact_missing",
        snapshot.validator_audit_artifact_missing,
    );
    report.field(
        "role_validator_audit_reports_submitted",
        snapshot.validator_audit_reports_submitted,
    );
    report.field(
        "role_validator_proposer_work_ready",
        snapshot.validator_proposer_work_ready,
    );
    report.field(
        "role_validator_proposer_settled_receipts_seen",
        snapshot.validator_proposer_settled_receipts_seen,
    );
    report.field(
        "role_validator_proposer_artifact_ready_receipts_seen",
        snapshot.validator_proposer_artifact_ready_receipts_seen,
    );
    report.field(
        "role_validator_proposer_attested_receipts_seen",
        snapshot.validator_proposer_attested_receipts_seen,
    );
    report.field(
        "role_validator_blocks_proposed",
        snapshot.validator_blocks_proposed,
    );
    report.field(
        "role_validator_useful_blocks_proposed",
        snapshot.validator_useful_blocks_proposed,
    );
    report.field(
        "role_validator_fallback_blocks_proposed",
        snapshot.validator_fallback_blocks_proposed,
    );
    report.field(
        "role_validator_receipts_proposed",
        snapshot.validator_receipts_proposed,
    );
    report.field(
        "role_validator_block_votes_submitted",
        snapshot.validator_block_votes_submitted,
    );
    report.field("role_local_producer", snapshot.local_producer);
    report.field("role_served_requests", snapshot.served_requests);
    report.field("role_produced_blocks", snapshot.produced_blocks);
    report.field(
        "role_network_applied_blocks",
        snapshot.network_applied_blocks,
    );
    report.field("role_network_events_ingested", network_events.events);
    report.field(
        "role_network_block_events_ingested",
        network_events.block_announcements,
    );
    report.field(
        "role_network_block_headers_ingested",
        network_events.block_headers,
    );
    report.field(
        "role_network_block_payloads_ingested",
        network_events.block_payloads,
    );
    report.field(
        "role_network_block_payloads_applied",
        network_events.block_payloads_applied,
    );
    report.field(
        "role_network_block_votes_ingested",
        network_events.block_votes,
    );
    report.field(
        "role_network_block_votes_applied",
        network_events.block_votes_applied,
    );
    report.field(
        "role_network_block_check_challenges_ingested",
        network_events.block_check_challenges,
    );
    report.field(
        "role_network_block_check_challenges_applied",
        network_events.block_check_challenges_applied,
    );
    report.field("role_network_job_events_ingested", network_events.jobs);
    report.field(
        "role_network_job_payloads_ingested",
        network_events.job_payloads,
    );
    report.field(
        "role_network_job_payloads_applied",
        network_events.job_payloads_applied,
    );
    report.field(
        "role_network_receipt_events_ingested",
        network_events.receipts,
    );
    report.field(
        "role_network_receipt_payloads_ingested",
        network_events.receipt_payloads,
    );
    report.field(
        "role_network_receipt_payloads_applied",
        network_events.receipt_payloads_applied,
    );
    report.field(
        "role_network_attestation_events_ingested",
        network_events.attestations,
    );
    report.field(
        "role_network_attestation_payloads_ingested",
        network_events.attestation_payloads,
    );
    report.field(
        "role_network_attestation_payloads_applied",
        network_events.attestation_payloads_applied,
    );
    report.field(
        "role_network_validator_audit_reports_ingested",
        network_events.validator_audit_reports,
    );
    report.field(
        "role_network_validator_audit_reports_applied",
        network_events.validator_audit_reports_applied,
    );
    report.field("role_network_peer_events_ingested", network_events.peers);
    report.field("role_network_invalid_events", network_events.invalid_events);
    report.field("role_latest_height", snapshot.latest_height);
    report.field("role_p2p_connected_peers", snapshot.p2p_connected_peers);
    report.field("role_p2p_observed_blocks", snapshot.p2p_observed_blocks);
    report.field(
        "role_p2p_observed_block_payloads",
        snapshot.p2p_observed_block_payloads,
    );
    report.field(
        "role_p2p_observed_block_votes",
        snapshot.p2p_observed_block_votes,
    );
    report.field("role_p2p_observed_jobs", snapshot.p2p_observed_jobs);
    report.field("role_p2p_observed_receipts", snapshot.p2p_observed_receipts);
    report.field(
        "role_p2p_observed_attestations",
        snapshot.p2p_observed_attestations,
    );
    report.field(
        "role_p2p_latest_observed_block_height",
        snapshot.p2p_latest_observed_block_height,
    );
    report.field(
        "role_p2p_latest_observed_block_hash",
        hex(&snapshot.p2p_latest_observed_block_hash),
    );
    report.field(
        "role_p2p_observed_block_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_hashes),
    );
    report.field(
        "role_p2p_latest_observed_block_payload_height",
        snapshot.p2p_latest_observed_block_payload_height,
    );
    report.field(
        "role_p2p_latest_observed_block_payload_hash",
        hex(&snapshot.p2p_latest_observed_block_payload_hash),
    );
    report.field(
        "role_p2p_observed_block_payload_hashes",
        hex_hash_list(&snapshot.p2p_observed_block_payload_hashes),
    );
    let contents = report.finish();
    std::fs::write(&path, contents).map_err(|error| {
        format!(
            "failed to write role runtime status {}: {error}",
            path.display()
        )
    })
}

fn nonempty_status_label(value: &str) -> &str {
    if value.is_empty() { "none" } else { value }
}
