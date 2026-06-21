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
    "role_randomness_beacon_mode",
    "role_randomness_beacon_configured",
    "role_randomness_beacon_configured_source",
    "role_randomness_beacon_configured_round",
    "role_randomness_beacon_configured_randomness",
    "role_randomness_beacon_configured_proof_hash",
    "role_randomness_beacons_observed",
    "role_randomness_beacons_applied",
    "role_randomness_beacons_skipped",
    "role_randomness_beacon_failures",
    "role_randomness_latest_source_id",
    "role_randomness_latest_round",
    "role_randomness_last_error",
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
    "role_local_block_proposer",
    "role_local_block_proposer_delay_blocks",
    "role_local_block_proposer_delay_satisfied",
    "role_proposer_cooldown_blocks",
    "role_proposer_cadence_ready",
    "role_proposer_cadence_remaining_blocks",
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
    "role_network_external_randomness_beacons_ingested",
    "role_network_external_randomness_beacons_applied",
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
    let detection_evidence = chain.state().detection_probability_evidence(chain.params());
    report.field(
        "detection_probability_mechanism_count",
        detection_evidence.mechanism_count,
    );
    report.field(
        "detection_probability_minimum_detection_bps",
        detection_evidence.minimum_detection_probability_bps,
    );
    report.field(
        "detection_probability_maximum_false_accept_bps",
        detection_evidence.maximum_false_accept_probability_bps,
    );
    report.field(
        "detection_probability_live_subject_count",
        detection_evidence.live_subject_count,
    );
    for mechanism in detection_evidence.mechanisms {
        let prefix = format!("detection_probability_{}", mechanism.mechanism);
        report.field(&format!("{prefix}_source"), mechanism.source);
        report.field(
            &format!("{prefix}_sample_numerator"),
            mechanism.sample_numerator,
        );
        report.field(
            &format!("{prefix}_sample_denominator"),
            mechanism.sample_denominator,
        );
        report.field(
            &format!("{prefix}_detection_probability_bps"),
            mechanism.detection_probability_bps,
        );
        report.field(
            &format!("{prefix}_false_accept_probability_bps"),
            mechanism.false_accept_probability_bps,
        );
        report.field(
            &format!("{prefix}_live_subject_count"),
            mechanism.live_subject_count,
        );
    }
    let randomness = chain.state().randomness_binding_evidence();
    report.field("randomness_beacon_source", randomness.beacon_source);
    report.field(
        "randomness_drand_round_mapping",
        randomness.drand_round_mapping,
    );
    report.field("randomness_vrf_construction", randomness.vrf_construction);
    report.field(
        "randomness_assignment_seed_domain",
        randomness.assignment_seed_domain,
    );
    report.field(
        "randomness_validation_seed_commitment_domain",
        randomness.validation_seed_commitment_domain,
    );
    report.field(
        "randomness_validation_seed_reveal_domain",
        randomness.validation_seed_reveal_domain,
    );
    report.field(
        "randomness_commit_reveal_ordering",
        randomness.commit_reveal_ordering,
    );
    report.field(
        "randomness_current_block_hash_allowed",
        randomness.current_block_hash_randomness_allowed,
    );
    report.field(
        "randomness_receipt_anchor_count",
        randomness.receipt_anchor_count,
    );
    report.field(
        "randomness_finalized_beacon_anchor_count",
        randomness.finalized_beacon_anchor_count,
    );
    report.field(
        "randomness_finalized_beacon_round_mapping_count",
        randomness.finalized_beacon_round_mapping_count,
    );
    report.field(
        "randomness_validator_vrf_seed_count",
        randomness.validator_vrf_seed_count,
    );
    report.field(
        "randomness_receipt_bound_anchor_count",
        randomness.receipt_bound_anchor_count,
    );
    report.field(
        "randomness_consistent_anchor_count",
        randomness.consistent_anchor_count,
    );
    report.field(
        "randomness_current_block_hash_anchor_count",
        randomness.current_block_hash_anchor_count,
    );
    report.field(
        "randomness_external_beacon_record_count",
        randomness.external_beacon_record_count,
    );
    report.field(
        "randomness_latest_external_beacon_round",
        randomness.latest_external_beacon_round,
    );
    report.field(
        "randomness_all_receipt_anchors_consistent",
        randomness.all_receipt_anchors_consistent,
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
                claimable_height_label(claim.claimable_at_height),
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
                "{}:{}:{}:{}:{}:{}:{}:{}",
                claim_key_label(claim.claim_id),
                claim_key_label(claim.subject_id),
                claim.ledger.receipt_kind_label().unwrap_or("unknown"),
                hex(&claim.beneficiary),
                claim.amount,
                claimable_height_label(claim.claimable_at_height),
                claim.awaiting_inclusion,
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
                claimable_height_label(claim.claimable_at_height),
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
                claimable_height_label(claim.claimable_at_height)
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

fn claimable_height_label(claimable_at_height: Option<u64>) -> String {
    claimable_at_height
        .map(|height| height.to_string())
        .unwrap_or_else(|| "awaiting_inclusion".to_owned())
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
        ASSIGNMENT_SEED_DOMAIN, ChainCommand, ChainEngine, ChainParams, JobState,
        PendingChallengeReward, PendingReceiptReward, RANDOMNESS_BEACON_SOURCE,
        RANDOMNESS_DRAND_ROUND_MAPPING, RANDOMNESS_VRF_CONSTRUCTION, ReceiptRewardKind,
        ReceiptRewardMaturity, VALIDATION_SEED_COMMITMENT_DOMAIN, VALIDATION_SEED_REVEAL_DOMAIN,
    };
    use crate::jobs::MatmulJob;
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
            maturity: ReceiptRewardMaturity::AwaitingInclusion,
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
                .value("pending_receipt_reward_claims")
                .unwrap()
                .contains(":awaiting_inclusion:true:false")
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
        chain.submit_job(JobState::TensorOp(MatmulJob::synthetic(
            0, 0, 32, 8, 16, &beacon, 20,
        )));
        chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
            claim_id: hash_bytes(b"test", &[b"status-audit-validator-claim"]),
            receipt_id: hash_bytes(b"test", &[b"status-audit-receipt"]),
            beneficiary: address(b"status-audit-validator"),
            amount: 50,
            kind: ReceiptRewardKind::Validator,
            maturity: ReceiptRewardMaturity::ClaimableAt(10),
            voided_by_challenge: false,
        });
        chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
            claim_id: hash_bytes(b"test", &[b"status-fraud-path-miner-claim"]),
            receipt_id: hash_bytes(b"test", &[b"status-fraud-path-miner-receipt"]),
            beneficiary: address(b"status-fraud-path-miner"),
            amount: 9,
            kind: ReceiptRewardKind::Miner,
            maturity: ReceiptRewardMaturity::ClaimableAt(10),
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
            Some("0")
        );
        assert_eq!(
            fields.value("validator_audit_economic_at_risk_reward_claim_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("validator_audit_economic_required_slashable_bond"),
            Some("0")
        );
        assert_eq!(
            fields.value("validator_audit_economic_invariant_holds"),
            Some("true")
        );
        assert_eq!(fields.value("fraud_path_economic_path_count"), Some("4"));
        assert_eq!(
            fields.value("fraud_path_economic_all_invariants_hold"),
            Some("true")
        );
        assert_eq!(
            fields.value("fraud_path_economic_max_required_slashable_bond"),
            Some("0")
        );
        assert_eq!(
            fields.value("fraud_path_economic_worst_path"),
            Some("block_check")
        );
        assert_eq!(
            fields.value("fraud_path_economic_validator_audit_required_slashable_bond"),
            Some("0")
        );
        assert_eq!(
            fields.value("fraud_path_economic_data_unavailability_required_slashable_bond"),
            Some("0")
        );
        assert_eq!(
            fields.value("fraud_path_economic_invalid_output_slashable_bond"),
            Some("25")
        );
        assert_eq!(
            fields.value("fraud_path_economic_invalid_output_required_slashable_bond"),
            Some("0")
        );
        assert_eq!(
            fields.value("fraud_path_economic_block_check_slashable_bond"),
            Some("500")
        );
        assert_eq!(
            fields.value("fraud_path_economic_block_check_invariant_holds"),
            Some("true")
        );
        assert_eq!(
            fields.value("detection_probability_mechanism_count"),
            Some("9")
        );
        assert_eq!(
            fields.value("detection_probability_full_freivalds_detection_probability_bps"),
            Some("10000")
        );
        assert_eq!(
            fields
                .value("detection_probability_row_sampling_sparse_audit_detection_probability_bps"),
            Some("5000")
        );
        assert_eq!(
            fields.value("detection_probability_validator_audit_detection_probability_bps"),
            Some("5000")
        );
        assert_eq!(
            fields.value("detection_probability_data_unavailability_detection_probability_bps"),
            Some("10000")
        );

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn service_status_exports_randomness_binding_evidence() {
        let beacon = hash_bytes(b"test", &[b"status-randomness-binding"]);
        let mut chain = Chain::new(beacon);
        chain
            .apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
                source_id: "drand-mainnet-round-v1".to_owned(),
                beacon_round: 9,
                randomness: hash_bytes(b"test", &[b"status-external-randomness"]),
                proof_hash: hash_bytes(b"test", &[b"status-external-randomness-proof"]),
            })
            .unwrap();
        chain.anchor_receipt_randomness_for_testing(hash_bytes(
            b"test",
            &[b"status-randomness-receipt"],
        ));

        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-status-randomness-{}-{}",
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
            fields.value("randomness_beacon_source"),
            Some(RANDOMNESS_BEACON_SOURCE)
        );
        assert_eq!(
            fields.value("randomness_drand_round_mapping"),
            Some(RANDOMNESS_DRAND_ROUND_MAPPING)
        );
        assert_ne!(
            fields.value("randomness_drand_round_mapping"),
            Some("not_configured_local_finalized_beacon")
        );
        assert_eq!(
            fields.value("randomness_vrf_construction"),
            Some(RANDOMNESS_VRF_CONSTRUCTION)
        );
        assert_ne!(
            fields.value("randomness_vrf_construction"),
            Some("not_configured_local_finalized_beacon")
        );
        assert_eq!(
            fields.value("randomness_assignment_seed_domain"),
            Some(ASSIGNMENT_SEED_DOMAIN)
        );
        assert_eq!(
            fields.value("randomness_validation_seed_commitment_domain"),
            Some(VALIDATION_SEED_COMMITMENT_DOMAIN)
        );
        assert_eq!(
            fields.value("randomness_validation_seed_reveal_domain"),
            Some(VALIDATION_SEED_REVEAL_DOMAIN)
        );
        assert_eq!(
            fields.value("randomness_current_block_hash_allowed"),
            Some("false")
        );
        assert_eq!(fields.value("randomness_receipt_anchor_count"), Some("1"));
        assert_eq!(
            fields.value("randomness_finalized_beacon_anchor_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("randomness_finalized_beacon_round_mapping_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("randomness_validator_vrf_seed_count"),
            Some("0")
        );
        assert_eq!(
            fields.value("randomness_receipt_bound_anchor_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("randomness_consistent_anchor_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("randomness_current_block_hash_anchor_count"),
            Some("0")
        );
        assert_eq!(
            fields.value("randomness_external_beacon_record_count"),
            Some("1")
        );
        assert_eq!(
            fields.value("randomness_latest_external_beacon_round"),
            Some("9")
        );
        assert_eq!(
            fields.value("randomness_all_receipt_anchors_consistent"),
            Some("true")
        );

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn service_status_forwards_role_randomness_beacon_evidence() {
        let beacon = hash_bytes(b"test", &[b"status-role-randomness"]);
        let chain = Chain::new(beacon);
        let data_dir = std::env::temp_dir().join(format!(
            "tensor-vm-status-role-randomness-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NodeStore::open(data_dir.clone());
        store.persist_chain(&chain).unwrap();
        std::fs::write(
            data_dir.join("role-runtime.status"),
            "\
role_randomness_beacon_mode=local_deterministic
role_randomness_beacon_configured=true
role_randomness_beacon_configured_source=local_deterministic:local_drand_fixture_v1
role_randomness_beacon_configured_round=1000
role_randomness_beacon_configured_randomness=0000000000000000000000000000000000000000000000000000000000000001
role_randomness_beacon_configured_proof_hash=0000000000000000000000000000000000000000000000000000000000000002
role_randomness_beacons_observed=1
role_randomness_beacons_applied=1
role_randomness_beacons_skipped=0
role_randomness_beacon_failures=0
role_randomness_latest_source_id=local_drand_fixture_v1
role_randomness_latest_round=1000
role_randomness_last_error=none
",
        )
        .unwrap();

        let status = service_status(data_dir.to_str().unwrap()).unwrap();
        let fields = KeyValueReport::parse_strict(&status).unwrap();
        assert_eq!(
            fields.value("role_randomness_beacon_mode"),
            Some("local_deterministic")
        );
        assert_eq!(
            fields.value("role_randomness_beacon_configured"),
            Some("true")
        );
        assert_eq!(
            fields.value("role_randomness_beacon_configured_source"),
            Some("local_deterministic:local_drand_fixture_v1")
        );
        assert_eq!(
            fields.value("role_randomness_beacon_configured_round"),
            Some("1000")
        );
        assert_eq!(fields.value("role_randomness_beacons_observed"), Some("1"));
        assert_eq!(fields.value("role_randomness_beacons_applied"), Some("1"));
        assert_eq!(fields.value("role_randomness_beacons_skipped"), Some("0"));
        assert_eq!(fields.value("role_randomness_beacon_failures"), Some("0"));
        assert_eq!(
            fields.value("role_randomness_latest_source_id"),
            Some("local_drand_fixture_v1")
        );
        assert_eq!(fields.value("role_randomness_latest_round"), Some("1000"));
        assert_eq!(fields.value("role_randomness_last_error"), Some("none"));

        let _ = std::fs::remove_dir_all(data_dir);
    }
}
