use std::{collections::BTreeMap, path::Path};

use super::{KeyValueReport, KeyValueReportWriter};
use crate::{NodeStore, hash::hex};

pub fn hex_hash_list(hashes: &[[u8; 32]]) -> String {
    if hashes.is_empty() {
        return "none".to_owned();
    }
    hashes
        .iter()
        .map(|hash| hex(hash))
        .collect::<Vec<_>>()
        .join(",")
}

struct StatusFileFields {
    fields: BTreeMap<String, String>,
}

const UNKNOWN_STATUS_VALUE: &str = "unknown";

impl StatusFileFields {
    fn from_path(path: impl AsRef<Path>) -> Self {
        let fields = std::fs::read_to_string(path)
            .ok()
            .map(|contents| KeyValueReport::parse_lenient(&contents).into_owned())
            .unwrap_or_default();
        Self { fields }
    }

    fn value(&self, key: &str) -> &str {
        self.fields
            .get(key)
            .map(String::as_str)
            .unwrap_or(UNKNOWN_STATUS_VALUE)
    }

    fn write_fields(&self, report: &mut KeyValueReportWriter, keys: &[&str]) {
        for key in keys {
            report.field(key, self.value(key));
        }
    }
}

const READY_STATUS_IDENTITY_FIELDS: &[&str] =
    &["operator_name", "operator_id", "role", "runtime_command"];

const READY_STATUS_NETWORK_FIELDS: &[&str] = &["node_multiaddr", "p2p_peer_id"];

const ROLE_RUNTIME_STATUS_FIELDS: &[&str] = &[
    "role_runtime_command",
    "role_loop_ready",
    "role_loop_role",
    "role_chain_profile",
    "role_can_produce_blocks",
    "role_wallet_address",
    "role_wallet_registration",
    "role_wallet_registered",
    "role_miner_work_ready",
    "role_miner_assigned_jobs_seen",
    "role_miner_unreceipted_jobs",
    "role_miner_receipts_submitted",
    "role_miner_tensors_inserted",
    "role_validator_work_ready",
    "role_validator_assigned_receipts_seen",
    "role_validator_unattested_receipts",
    "role_validator_artifact_ready_receipts",
    "role_validator_artifact_missing_receipts",
    "role_validator_remote_tensor_fetch_attempts",
    "role_validator_remote_tensor_fetch_successes",
    "role_validator_remote_tensor_fetch_failures",
    "role_validator_remote_tensor_fetch_bytes",
    "role_validator_remote_tensors_inserted",
    "role_validator_attestations_submitted",
    "role_validator_audit_work_ready",
    "role_validator_assigned_audits_seen",
    "role_validator_unreported_audits",
    "role_validator_audit_artifact_ready",
    "role_validator_audit_artifact_missing",
    "role_validator_audit_reports_submitted",
    "role_validator_proposer_work_ready",
    "role_validator_proposer_settled_receipts_seen",
    "role_validator_proposer_artifact_ready_receipts_seen",
    "role_validator_proposer_attested_receipts_seen",
    "role_validator_blocks_proposed",
    "role_validator_useful_blocks_proposed",
    "role_validator_fallback_blocks_proposed",
    "role_validator_receipts_proposed",
    "role_validator_block_votes_submitted",
    "role_local_producer",
    "role_served_requests",
    "role_produced_blocks",
    "role_network_applied_blocks",
    "role_network_events_ingested",
    "role_network_block_events_ingested",
    "role_network_block_headers_ingested",
    "role_network_block_payloads_ingested",
    "role_network_block_payloads_applied",
    "role_network_block_votes_ingested",
    "role_network_block_votes_applied",
    "role_network_block_check_challenges_ingested",
    "role_network_block_check_challenges_applied",
    "role_network_job_events_ingested",
    "role_network_job_payloads_ingested",
    "role_network_job_payloads_applied",
    "role_network_receipt_events_ingested",
    "role_network_receipt_payloads_ingested",
    "role_network_receipt_payloads_applied",
    "role_network_attestation_events_ingested",
    "role_network_attestation_payloads_ingested",
    "role_network_attestation_payloads_applied",
    "role_network_validator_audit_reports_ingested",
    "role_network_validator_audit_reports_applied",
    "role_network_peer_events_ingested",
    "role_network_invalid_events",
    "role_latest_height",
    "role_p2p_connected_peers",
    "role_p2p_observed_blocks",
    "role_p2p_observed_block_payloads",
    "role_p2p_observed_block_votes",
    "role_p2p_observed_jobs",
    "role_p2p_observed_receipts",
    "role_p2p_observed_attestations",
    "role_p2p_latest_observed_block_height",
    "role_p2p_latest_observed_block_hash",
    "role_p2p_observed_block_hashes",
    "role_p2p_latest_observed_block_payload_height",
    "role_p2p_latest_observed_block_payload_hash",
    "role_p2p_observed_block_payload_hashes",
];

pub fn service_status(data_dir: &str) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let status = store
        .status()
        .map_err(|error| format!("failed to inspect node store {data_dir}: {error}"))?;
    let latest_block_height = chain
        .blocks()
        .last()
        .map(|block| block.height)
        .unwrap_or_default();
    let finalized_block_count = chain
        .blocks()
        .iter()
        .filter(|block| chain.is_block_finalized(&block.hash()))
        .count();
    let first_live_block = chain.blocks().iter().find(|block| block.height > 2);
    let first_live_block_height = first_live_block
        .map(|block| block.height)
        .unwrap_or_default();
    let first_live_block_hash = first_live_block
        .map(|block| block.hash())
        .unwrap_or([0; 32]);
    let bootstrap_peer_count = if store.peer_book_store().path().exists() {
        store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| format!("failed to inspect peer book {data_dir}: {error}"))?
            .len()
    } else {
        0
    };
    let attestation_count: usize = chain.state().attestations().values().map(Vec::len).sum();
    let reward_account_count = chain
        .state()
        .rewards()
        .balances()
        .values()
        .filter(|balance| **balance > 0)
        .count();
    let ready_status = StatusFileFields::from_path(Path::new(data_dir).join("local-cpu-ready"));
    let role_runtime_status =
        StatusFileFields::from_path(Path::new(data_dir).join("role-runtime.status"));
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_status");
    report.field("data_dir", status.data_dir.display());
    ready_status.write_fields(&mut report, READY_STATUS_IDENTITY_FIELDS);
    role_runtime_status.write_fields(&mut report, ROLE_RUNTIME_STATUS_FIELDS);
    ready_status.write_fields(&mut report, READY_STATUS_NETWORK_FIELDS);
    report.field("height", chain.state().height());
    report.field("epoch", chain.state().epoch());
    report.field("block_count", status.block_count);
    report.field("latest_block_height", latest_block_height);
    report.field("latest_block_hash", hex(&status.latest_block_hash));
    report.field("state_root", hex(&chain.state_root()));
    report.field("block_log_root", hex(&status.block_log_root));
    report.field("finalized_block_count", finalized_block_count);
    report.field("first_live_block_height", first_live_block_height);
    report.field("first_live_block_hash", hex(&first_live_block_hash));
    report.field("registered_miner_count", chain.state().miners().len());
    report.field(
        "registered_validator_count",
        chain.state().validators().len(),
    );
    report.field("job_count", chain.state().jobs().len());
    report.field("receipt_count", chain.state().receipts().len());
    report.field(
        "settled_receipt_count",
        chain.state().settled_receipts().len(),
    );
    report.field(
        "data_unavailable_receipt_count",
        chain.state().data_unavailable_receipts().len(),
    );
    report.field(
        "data_unavailability_slash_count",
        chain.state().data_unavailability_slashes().len(),
    );
    report.field(
        "data_unavailability_slashed_amount_total",
        chain
            .state()
            .data_unavailability_slashes()
            .values()
            .map(|slash| slash.amount)
            .sum::<u64>(),
    );
    report.field(
        "validator_audit_assignment_count",
        chain.state().validator_audit_assignments().len(),
    );
    report.field(
        "validator_audit_result_count",
        chain.state().validator_audit_results().len(),
    );
    report.field(
        "validator_audit_slash_count",
        chain.state().validator_audit_slashes().len(),
    );
    report.field(
        "validator_audit_slashed_amount_total",
        chain
            .state()
            .validator_audit_slashes()
            .values()
            .map(|slash| slash.amount)
            .sum::<u64>(),
    );
    report.field("attestation_count", attestation_count);
    report.field("reward_account_count", reward_account_count);
    report.field(
        "pending_receipt_reward_count",
        chain.state().pending_receipt_rewards().len(),
    );
    report.field(
        "pending_proposer_reward_count",
        chain.state().pending_proposer_rewards().len(),
    );
    report.field(
        "pending_challenge_reward_count",
        chain.state().pending_challenge_rewards().len(),
    );
    report.field(
        "pending_credit_reward_count",
        chain.state().pending_credit_rewards().len(),
    );
    report.field("model_count", chain.state().model_states().len());
    report.field("bootstrap_peer_count", bootstrap_peer_count);
    report.field("node_store_ready", true);
    report.field("status_source", "node_store");
    Ok(report.finish())
}
