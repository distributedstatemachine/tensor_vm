use std::{collections::BTreeMap, path::Path};

use super::{KeyValueReport, KeyValueReportWriter};
use crate::{
    Chain, NodeStore,
    chain::{RewardClaimKey, RewardClaimLedger},
    hash::hex,
};

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
    let audit_economics = chain
        .state()
        .validator_audit_economic_calibration(chain.params());
    report.field(
        "validator_audit_economic_detection_numerator",
        audit_economics.detection_numerator,
    );
    report.field(
        "validator_audit_economic_detection_denominator",
        audit_economics.detection_denominator,
    );
    report.field(
        "validator_audit_economic_detection_probability_bps",
        audit_economics.detection_probability_bps,
    );
    report.field(
        "validator_audit_economic_slashable_bond",
        audit_economics.slashable_bond,
    );
    report.field(
        "validator_audit_economic_reward_from_fraud",
        audit_economics.reward_from_fraud,
    );
    report.field(
        "validator_audit_economic_at_risk_reward_claim_count",
        audit_economics.at_risk_validator_reward_claim_count,
    );
    report.field(
        "validator_audit_economic_required_slashable_bond",
        audit_economics.required_slashable_bond,
    );
    report.field(
        "validator_audit_economic_invariant_holds",
        audit_economics.invariant_holds,
    );
    let fraud_path_economics = chain
        .state()
        .fraud_path_economic_calibration(chain.params());
    report.field(
        "fraud_path_economic_path_count",
        fraud_path_economics.path_count,
    );
    report.field(
        "fraud_path_economic_all_invariants_hold",
        fraud_path_economics.all_invariants_hold,
    );
    report.field(
        "fraud_path_economic_max_required_slashable_bond",
        fraud_path_economics.max_required_slashable_bond,
    );
    report.field(
        "fraud_path_economic_worst_path",
        fraud_path_economics.worst_path,
    );
    for path in fraud_path_economics.paths {
        let prefix = format!("fraud_path_economic_{}", path.path);
        report.field(
            &format!("{prefix}_detection_numerator"),
            path.detection_numerator,
        );
        report.field(
            &format!("{prefix}_detection_denominator"),
            path.detection_denominator,
        );
        report.field(
            &format!("{prefix}_detection_probability_bps"),
            path.detection_probability_bps,
        );
        report.field(&format!("{prefix}_slashable_bond"), path.slashable_bond);
        report.field(
            &format!("{prefix}_reward_from_fraud"),
            path.reward_from_fraud,
        );
        report.field(
            &format!("{prefix}_at_risk_reward_claim_count"),
            path.at_risk_reward_claim_count,
        );
        report.field(
            &format!("{prefix}_required_slashable_bond"),
            path.required_slashable_bond,
        );
        report.field(&format!("{prefix}_invariant_holds"), path.invariant_holds);
    }
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
    report.field(
        "pending_proposer_reward_claims",
        pending_proposer_reward_claims(&chain, 16),
    );
    report.field(
        "pending_receipt_reward_claims",
        pending_receipt_reward_claims(&chain, 16),
    );
    report.field(
        "pending_challenge_reward_claims",
        pending_challenge_reward_claims(&chain, 16),
    );
    report.field(
        "pending_credit_reward_claims",
        pending_credit_reward_claims(&chain, 16),
    );
    report.field("model_count", chain.state().model_states().len());
    report.field("bootstrap_peer_count", bootstrap_peer_count);
    report.field("node_store_ready", true);
    report.field("status_source", "node_store");
    Ok(report.finish())
}

fn pending_proposer_reward_claims(chain: &Chain, limit: usize) -> String {
    let claims = chain
        .state()
        .pending_reward_claims()
        .into_iter()
        .filter(|claim| claim.ledger == RewardClaimLedger::Proposer)
        .take(limit)
        .map(|claim| {
            format!(
                "{}:{}:{}:{}:{}",
                claim_key_label(claim.claim_id),
                hex(&claim.beneficiary),
                claim.amount,
                claim.claimable_at_height,
                claim.voided_by_challenge
            )
        })
        .collect::<Vec<_>>();
    compact_claims(claims)
}

fn pending_receipt_reward_claims(chain: &Chain, limit: usize) -> String {
    let claims = chain
        .state()
        .pending_reward_claims()
        .into_iter()
        .filter(|claim| {
            matches!(
                claim.ledger,
                RewardClaimLedger::ReceiptMiner | RewardClaimLedger::ReceiptValidator
            )
        })
        .take(limit)
        .map(|claim| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                claim_key_label(claim.claim_id),
                claim_key_label(claim.subject_id),
                claim.ledger.receipt_kind_label().unwrap_or("unknown"),
                hex(&claim.beneficiary),
                claim.amount,
                claim.claimable_at_height,
                claim.voided_by_challenge
            )
        })
        .collect::<Vec<_>>();
    compact_claims(claims)
}

fn pending_challenge_reward_claims(chain: &Chain, limit: usize) -> String {
    let claims = chain
        .state()
        .pending_reward_claims()
        .into_iter()
        .filter(|claim| claim.ledger == RewardClaimLedger::Challenge)
        .take(limit)
        .map(|claim| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                claim_key_label(claim.claim_id),
                claim_key_label(claim.subject_id),
                claim
                    .related_id
                    .map(claim_key_label)
                    .unwrap_or_else(|| "none".to_owned()),
                hex(&claim.beneficiary),
                claim.amount,
                claim.claimable_at_height,
                claim.voided_by_challenge
            )
        })
        .collect::<Vec<_>>();
    compact_claims(claims)
}

fn pending_credit_reward_claims(chain: &Chain, limit: usize) -> String {
    let claims = chain
        .state()
        .pending_reward_claims()
        .into_iter()
        .filter(|claim| claim.ledger == RewardClaimLedger::Credit)
        .take(limit)
        .map(|claim| {
            format!(
                "{}:{}:{}:{}",
                claim_key_label(claim.claim_id),
                hex(&claim.beneficiary),
                claim.amount,
                claim.claimable_at_height
            )
        })
        .collect::<Vec<_>>();
    compact_claims(claims)
}

fn compact_claims(claims: Vec<String>) -> String {
    if claims.is_empty() {
        "none".to_owned()
    } else {
        claims.join(";")
    }
}

fn claim_key_label(key: RewardClaimKey) -> String {
    match key {
        RewardClaimKey::BlockHeight(height) => height.to_string(),
        RewardClaimKey::Hash(hash) => hex(&hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{
        ChainCommand, ChainEngine, ChainParams, PendingChallengeReward, PendingReceiptReward,
        ReceiptRewardKind,
    };
    use crate::types::{address, hash_bytes};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn service_status_exports_pending_reward_claim_maturity_details() {
        let beacon = hash_bytes(b"test", &[b"status-pending-rewards"]);
        let mut chain = Chain::with_params(
            ChainParams {
                reward_settlement_delay_epochs: 1,
                challenge_window_epochs: 1,
                epoch_length: 2,
                ..ChainParams::default()
            },
            beacon,
        );
        let proposer = address(b"status-proposer");
        let miner = address(b"status-miner");
        let validator = address(b"status-validator");
        let challenger = address(b"status-challenger");
        chain
            .register_validator(proposer, chain.params().validator_min_stake)
            .unwrap();
        chain
            .produce_block_with_rewards(proposer, 1_000, 40, 10)
            .unwrap();
        let receipt_claim = hash_bytes(b"test", &[b"status-receipt-claim"]);
        chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
            claim_id: receipt_claim,
            receipt_id: hash_bytes(b"test", &[b"status-receipt"]),
            beneficiary: miner,
            amount: 25,
            kind: ReceiptRewardKind::Miner,
            claimable_at_height: 8,
            voided_by_challenge: false,
        });
        let challenge_claim = hash_bytes(b"test", &[b"status-challenge-claim"]);
        chain.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
            claim_id: challenge_claim,
            challenge_id: hash_bytes(b"test", &[b"status-challenge"]),
            block_hash: chain.blocks().last().unwrap().hash(),
            receipt_id: hash_bytes(b"test", &[b"status-challenge-receipt"]),
            challenger,
            amount: 35,
            claimable_at_height: 9,
            voided_by_challenge: true,
        });
        chain
            .apply_command(ChainCommand::CreditReward {
                address: validator,
                amount: 45,
            })
            .unwrap();

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-status-reward-claims-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NodeStore::open(data_dir.clone());
        store.persist_chain(&chain).unwrap();

        let status = service_status(data_dir.to_str().unwrap()).unwrap();
        let fields = KeyValueReport::parse_strict(&status).unwrap();
        assert_ne!(fields.value("pending_proposer_reward_claims"), Some("none"));
        assert!(
            fields
                .value("pending_receipt_reward_claims")
                .unwrap()
                .contains(":miner:")
        );
        assert!(
            fields
                .value("pending_challenge_reward_claims")
                .unwrap()
                .contains(":true")
        );
        assert_ne!(fields.value("pending_credit_reward_claims"), Some("none"));

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn service_status_exports_validator_audit_economic_calibration() {
        let beacon = hash_bytes(b"test", &[b"status-audit-economics"]);
        let mut chain = Chain::with_params(
            ChainParams {
                validator_audit_sample_numerator: 1,
                validator_audit_sample_denominator: 2,
                validator_audit_slash_amount: 101,
                ..ChainParams::default()
            },
            beacon,
        );
        let proposer = address(b"status-fraud-path-proposer");
        chain
            .register_validator(proposer, chain.params().validator_min_stake)
            .unwrap();
        chain
            .produce_block_with_rewards(proposer, 1_000, 400, 100)
            .unwrap();
        chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
            claim_id: hash_bytes(b"test", &[b"status-audit-validator-claim"]),
            receipt_id: hash_bytes(b"test", &[b"status-audit-receipt"]),
            beneficiary: address(b"status-audit-validator"),
            amount: 50,
            kind: ReceiptRewardKind::Validator,
            claimable_at_height: 10,
            voided_by_challenge: false,
        });
        chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
            claim_id: hash_bytes(b"test", &[b"status-fraud-path-miner-claim"]),
            receipt_id: hash_bytes(b"test", &[b"status-fraud-path-miner-receipt"]),
            beneficiary: address(b"status-fraud-path-miner"),
            amount: 9,
            kind: ReceiptRewardKind::Miner,
            claimable_at_height: 10,
            voided_by_challenge: false,
        });

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-status-audit-economics-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NodeStore::open(data_dir.clone());
        store.persist_chain(&chain).unwrap();

        let status = service_status(data_dir.to_str().unwrap()).unwrap();
        let fields = KeyValueReport::parse_strict(&status).unwrap();
        assert_eq!(
            fields.value("validator_audit_economic_detection_numerator"),
            Some("1")
        );
        assert_eq!(
            fields.value("validator_audit_economic_detection_denominator"),
            Some("2")
        );
        assert_eq!(
            fields.value("validator_audit_economic_detection_probability_bps"),
            Some("5000")
        );
        assert_eq!(
            fields.value("validator_audit_economic_slashable_bond"),
            Some("101")
        );
        assert_eq!(
            fields.value("validator_audit_economic_reward_from_fraud"),
            Some("50")
        );
        assert_eq!(
            fields.value("validator_audit_economic_at_risk_reward_claim_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("validator_audit_economic_required_slashable_bond"),
            Some("101")
        );
        assert_eq!(
            fields.value("validator_audit_economic_invariant_holds"),
            Some("true")
        );
        assert_eq!(fields.value("fraud_path_economic_path_count"), Some("3"));
        assert_eq!(
            fields.value("fraud_path_economic_all_invariants_hold"),
            Some("false")
        );
        assert_eq!(
            fields.value("fraud_path_economic_max_required_slashable_bond"),
            Some("501")
        );
        assert_eq!(
            fields.value("fraud_path_economic_worst_path"),
            Some("block_check")
        );
        assert_eq!(
            fields.value("fraud_path_economic_validator_audit_required_slashable_bond"),
            Some("101")
        );
        assert_eq!(
            fields.value("fraud_path_economic_data_unavailability_required_slashable_bond"),
            Some("10")
        );
        assert_eq!(
            fields.value("fraud_path_economic_block_check_slashable_bond"),
            Some("500")
        );
        assert_eq!(
            fields.value("fraud_path_economic_block_check_invariant_holds"),
            Some("false")
        );

        let _ = std::fs::remove_dir_all(data_dir);
    }
}
