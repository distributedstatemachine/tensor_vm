use super::PendingNetworkPayloads;
use crate::types::Hash;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkEventIngest {
    pub events: usize,
    pub block_announcements: usize,
    pub block_headers: usize,
    pub block_payloads: usize,
    pub block_payloads_applied: usize,
    pub block_votes: usize,
    pub block_votes_applied: usize,
    pub block_check_challenges: usize,
    pub block_check_challenges_applied: usize,
    pub trace_bisection_opens: usize,
    pub trace_bisection_opens_applied: usize,
    pub trace_bisection_expectations: usize,
    pub trace_bisection_expectations_applied: usize,
    pub trace_bisection_rounds: usize,
    pub trace_bisection_rounds_applied: usize,
    pub trace_bisection_referees: usize,
    pub trace_bisection_referees_applied: usize,
    pub jobs: usize,
    pub job_payloads: usize,
    pub job_payloads_applied: usize,
    pub receipts: usize,
    pub receipt_payloads: usize,
    pub receipt_payloads_applied: usize,
    pub attestations: usize,
    pub attestation_payloads: usize,
    pub attestation_payloads_applied: usize,
    pub validator_audit_reports: usize,
    pub validator_audit_reports_applied: usize,
    pub external_randomness_beacons: usize,
    pub external_randomness_beacons_applied: usize,
    pub validator_vrf_reveals: usize,
    pub validator_vrf_reveals_applied: usize,
    pub peers: usize,
    pub invalid_events: usize,
    pub applied_blocks: usize,
}

impl NetworkEventIngest {
    pub fn has_activity(self) -> bool {
        self.events > 0
            || self.job_payloads_applied > 0
            || self.receipt_payloads_applied > 0
            || self.attestation_payloads_applied > 0
            || self.validator_audit_reports_applied > 0
            || self.external_randomness_beacons_applied > 0
            || self.validator_vrf_reveals_applied > 0
            || self.block_payloads_applied > 0
            || self.block_votes_applied > 0
            || self.block_check_challenges_applied > 0
            || self.trace_bisection_opens_applied > 0
            || self.trace_bisection_expectations_applied > 0
            || self.trace_bisection_rounds_applied > 0
            || self.trace_bisection_referees_applied > 0
            || self.invalid_events > 0
            || self.applied_blocks > 0
    }

    pub fn accumulate(&mut self, other: Self) {
        self.events = self.events.saturating_add(other.events);
        self.block_announcements = self
            .block_announcements
            .saturating_add(other.block_announcements);
        self.block_headers = self.block_headers.saturating_add(other.block_headers);
        self.block_payloads = self.block_payloads.saturating_add(other.block_payloads);
        self.block_payloads_applied = self
            .block_payloads_applied
            .saturating_add(other.block_payloads_applied);
        self.block_votes = self.block_votes.saturating_add(other.block_votes);
        self.block_votes_applied = self
            .block_votes_applied
            .saturating_add(other.block_votes_applied);
        self.block_check_challenges = self
            .block_check_challenges
            .saturating_add(other.block_check_challenges);
        self.block_check_challenges_applied = self
            .block_check_challenges_applied
            .saturating_add(other.block_check_challenges_applied);
        self.trace_bisection_opens = self
            .trace_bisection_opens
            .saturating_add(other.trace_bisection_opens);
        self.trace_bisection_opens_applied = self
            .trace_bisection_opens_applied
            .saturating_add(other.trace_bisection_opens_applied);
        self.trace_bisection_expectations = self
            .trace_bisection_expectations
            .saturating_add(other.trace_bisection_expectations);
        self.trace_bisection_expectations_applied = self
            .trace_bisection_expectations_applied
            .saturating_add(other.trace_bisection_expectations_applied);
        self.trace_bisection_rounds = self
            .trace_bisection_rounds
            .saturating_add(other.trace_bisection_rounds);
        self.trace_bisection_rounds_applied = self
            .trace_bisection_rounds_applied
            .saturating_add(other.trace_bisection_rounds_applied);
        self.trace_bisection_referees = self
            .trace_bisection_referees
            .saturating_add(other.trace_bisection_referees);
        self.trace_bisection_referees_applied = self
            .trace_bisection_referees_applied
            .saturating_add(other.trace_bisection_referees_applied);
        self.jobs = self.jobs.saturating_add(other.jobs);
        self.job_payloads = self.job_payloads.saturating_add(other.job_payloads);
        self.job_payloads_applied = self
            .job_payloads_applied
            .saturating_add(other.job_payloads_applied);
        self.receipts = self.receipts.saturating_add(other.receipts);
        self.receipt_payloads = self.receipt_payloads.saturating_add(other.receipt_payloads);
        self.receipt_payloads_applied = self
            .receipt_payloads_applied
            .saturating_add(other.receipt_payloads_applied);
        self.attestations = self.attestations.saturating_add(other.attestations);
        self.attestation_payloads = self
            .attestation_payloads
            .saturating_add(other.attestation_payloads);
        self.attestation_payloads_applied = self
            .attestation_payloads_applied
            .saturating_add(other.attestation_payloads_applied);
        self.validator_audit_reports = self
            .validator_audit_reports
            .saturating_add(other.validator_audit_reports);
        self.validator_audit_reports_applied = self
            .validator_audit_reports_applied
            .saturating_add(other.validator_audit_reports_applied);
        self.external_randomness_beacons = self
            .external_randomness_beacons
            .saturating_add(other.external_randomness_beacons);
        self.external_randomness_beacons_applied = self
            .external_randomness_beacons_applied
            .saturating_add(other.external_randomness_beacons_applied);
        self.validator_vrf_reveals = self
            .validator_vrf_reveals
            .saturating_add(other.validator_vrf_reveals);
        self.validator_vrf_reveals_applied = self
            .validator_vrf_reveals_applied
            .saturating_add(other.validator_vrf_reveals_applied);
        self.peers = self.peers.saturating_add(other.peers);
        self.invalid_events = self.invalid_events.saturating_add(other.invalid_events);
        self.applied_blocks = self.applied_blocks.saturating_add(other.applied_blocks);
    }
}

#[derive(Debug, Default)]
pub struct NodeRuntimeState {
    served_requests: usize,
    produced_blocks: usize,
    local_synthetic_jobs_published: usize,
    network_applied_blocks: usize,
    network_events: NetworkEventIngest,
    pending_network_payloads: PendingNetworkPayloads,
    miner_assigned_jobs_seen: BTreeSet<Hash>,
    miner_unreceipted_jobs: BTreeSet<Hash>,
    miner_receipts_submitted: usize,
    miner_tensors_inserted: usize,
    miner_trace_bisection_rounds_submitted: usize,
    validator_assigned_receipts_seen: BTreeSet<Hash>,
    validator_unattested_receipts: BTreeSet<Hash>,
    validator_artifact_ready_receipts: BTreeSet<Hash>,
    validator_artifact_missing_receipts: BTreeSet<Hash>,
    validator_assigned_audits_seen: BTreeSet<Hash>,
    validator_unreported_audits: BTreeSet<Hash>,
    validator_audit_artifact_ready: BTreeSet<Hash>,
    validator_audit_artifact_missing: BTreeSet<Hash>,
    validator_proposer_settled_receipts_seen: BTreeSet<Hash>,
    validator_proposer_artifact_ready_receipts_seen: BTreeSet<Hash>,
    validator_proposer_attested_receipts_seen: BTreeSet<Hash>,
    validator_attestations_submitted: usize,
    validator_audit_reports_submitted: usize,
    validator_vrf_key_public_key: Hash,
    validator_vrf_key_registration_count: usize,
    validator_blocks_proposed: usize,
    validator_useful_blocks_proposed: usize,
    validator_fallback_blocks_proposed: usize,
    validator_receipts_proposed: usize,
    validator_block_votes_submitted: usize,
    validator_trace_bisection_opens_submitted: usize,
    validator_trace_bisection_expectations_submitted: usize,
    validator_trace_bisection_referees_submitted: usize,
    validator_remote_tensor_fetch_attempts: usize,
    validator_remote_tensor_fetch_successes: usize,
    validator_remote_tensor_fetch_failures: usize,
    validator_remote_tensor_fetch_bytes: usize,
    validator_remote_tensors_inserted: usize,
    randomness_beacons_observed: usize,
    randomness_beacons_applied: usize,
    randomness_beacons_skipped: usize,
    randomness_beacon_failures: usize,
    randomness_latest_source_id: String,
    randomness_latest_round: u64,
    randomness_last_error: String,
    randomness_published_source_id: String,
    randomness_published_round: u64,
    randomness_public_drand_fetch_attempts: usize,
    randomness_public_drand_fetch_successes: usize,
    randomness_public_drand_fetch_stale: usize,
    randomness_public_drand_consecutive_failures: usize,
    randomness_public_drand_backoff_remaining_ticks: u64,
    randomness_public_drand_expected_latest_round: u64,
    randomness_public_drand_fetched_round_lag: u64,
    randomness_public_drand_max_round_lag: u64,
    randomness_public_drand_rounds_per_chain_epoch: u64,
    randomness_public_drand_chain_epoch: u64,
    randomness_public_drand_fresh: bool,
}

impl NodeRuntimeState {
    pub fn served_requests(&self) -> usize {
        self.served_requests
    }

    pub fn produced_blocks(&self) -> usize {
        self.produced_blocks
    }

    pub fn local_synthetic_jobs_published(&self) -> usize {
        self.local_synthetic_jobs_published
    }

    pub fn network_applied_blocks(&self) -> usize {
        self.network_applied_blocks
    }

    pub fn network_events(&self) -> NetworkEventIngest {
        self.network_events
    }

    pub fn pending_payloads(&self) -> &PendingNetworkPayloads {
        &self.pending_network_payloads
    }

    pub fn pending_payloads_mut(&mut self) -> &mut PendingNetworkPayloads {
        &mut self.pending_network_payloads
    }

    pub fn miner_assigned_jobs_seen(&self) -> usize {
        self.miner_assigned_jobs_seen.len()
    }

    pub fn miner_unreceipted_jobs(&self) -> usize {
        self.miner_unreceipted_jobs.len()
    }

    pub fn miner_work_ready(&self) -> bool {
        !self.miner_unreceipted_jobs.is_empty()
    }

    pub fn miner_receipts_submitted(&self) -> usize {
        self.miner_receipts_submitted
    }

    pub fn miner_tensors_inserted(&self) -> usize {
        self.miner_tensors_inserted
    }

    pub fn miner_trace_bisection_rounds_submitted(&self) -> usize {
        self.miner_trace_bisection_rounds_submitted
    }

    pub fn validator_assigned_receipts_seen(&self) -> usize {
        self.validator_assigned_receipts_seen.len()
    }

    pub fn validator_unattested_receipts(&self) -> usize {
        self.validator_unattested_receipts.len()
    }

    pub fn validator_artifact_ready_receipts(&self) -> usize {
        self.validator_artifact_ready_receipts.len()
    }

    pub fn validator_artifact_missing_receipts(&self) -> usize {
        self.validator_artifact_missing_receipts.len()
    }

    pub fn validator_work_ready(&self) -> bool {
        !self.validator_artifact_ready_receipts.is_empty()
    }

    pub fn validator_assigned_audits_seen(&self) -> usize {
        self.validator_assigned_audits_seen.len()
    }

    pub fn validator_unreported_audits(&self) -> usize {
        self.validator_unreported_audits.len()
    }

    pub fn validator_audit_artifact_ready(&self) -> usize {
        self.validator_audit_artifact_ready.len()
    }

    pub fn validator_audit_artifact_missing(&self) -> usize {
        self.validator_audit_artifact_missing.len()
    }

    pub fn validator_audit_work_ready(&self) -> bool {
        !self.validator_audit_artifact_ready.is_empty()
    }

    pub fn validator_proposer_settled_receipts_seen(&self) -> usize {
        self.validator_proposer_settled_receipts_seen.len()
    }

    pub fn validator_proposer_artifact_ready_receipts_seen(&self) -> usize {
        self.validator_proposer_artifact_ready_receipts_seen.len()
    }

    pub fn validator_proposer_attested_receipts_seen(&self) -> usize {
        self.validator_proposer_attested_receipts_seen.len()
    }

    pub fn validator_proposer_work_ready(&self) -> bool {
        !self.validator_proposer_settled_receipts_seen.is_empty()
    }

    pub fn validator_attestations_submitted(&self) -> usize {
        self.validator_attestations_submitted
    }

    pub fn validator_audit_reports_submitted(&self) -> usize {
        self.validator_audit_reports_submitted
    }

    pub fn validator_vrf_key_public_key(&self) -> Hash {
        self.validator_vrf_key_public_key
    }

    pub fn validator_vrf_key_registration_count(&self) -> usize {
        self.validator_vrf_key_registration_count
    }

    pub fn validator_vrf_key_registered(&self) -> bool {
        self.validator_vrf_key_public_key != [0; 32]
    }

    pub fn validator_blocks_proposed(&self) -> usize {
        self.validator_blocks_proposed
    }

    pub fn validator_useful_blocks_proposed(&self) -> usize {
        self.validator_useful_blocks_proposed
    }

    pub fn validator_fallback_blocks_proposed(&self) -> usize {
        self.validator_fallback_blocks_proposed
    }

    pub fn validator_receipts_proposed(&self) -> usize {
        self.validator_receipts_proposed
    }

    pub fn validator_block_votes_submitted(&self) -> usize {
        self.validator_block_votes_submitted
    }

    pub fn validator_trace_bisection_opens_submitted(&self) -> usize {
        self.validator_trace_bisection_opens_submitted
    }

    pub fn validator_trace_bisection_expectations_submitted(&self) -> usize {
        self.validator_trace_bisection_expectations_submitted
    }

    pub fn validator_trace_bisection_referees_submitted(&self) -> usize {
        self.validator_trace_bisection_referees_submitted
    }

    pub fn validator_remote_tensor_fetch_attempts(&self) -> usize {
        self.validator_remote_tensor_fetch_attempts
    }

    pub fn validator_remote_tensor_fetch_successes(&self) -> usize {
        self.validator_remote_tensor_fetch_successes
    }

    pub fn validator_remote_tensor_fetch_failures(&self) -> usize {
        self.validator_remote_tensor_fetch_failures
    }

    pub fn validator_remote_tensor_fetch_bytes(&self) -> usize {
        self.validator_remote_tensor_fetch_bytes
    }

    pub fn validator_remote_tensors_inserted(&self) -> usize {
        self.validator_remote_tensors_inserted
    }

    pub fn randomness_beacons_observed(&self) -> usize {
        self.randomness_beacons_observed
    }

    pub fn randomness_beacons_applied(&self) -> usize {
        self.randomness_beacons_applied
    }

    pub fn randomness_beacons_skipped(&self) -> usize {
        self.randomness_beacons_skipped
    }

    pub fn randomness_beacon_failures(&self) -> usize {
        self.randomness_beacon_failures
    }

    pub fn randomness_latest_source_id(&self) -> &str {
        &self.randomness_latest_source_id
    }

    pub fn randomness_latest_round(&self) -> u64 {
        self.randomness_latest_round
    }

    pub fn randomness_last_error(&self) -> &str {
        &self.randomness_last_error
    }

    pub fn randomness_public_drand_fetch_attempts(&self) -> usize {
        self.randomness_public_drand_fetch_attempts
    }

    pub fn randomness_public_drand_fetch_successes(&self) -> usize {
        self.randomness_public_drand_fetch_successes
    }

    pub fn randomness_public_drand_fetch_stale(&self) -> usize {
        self.randomness_public_drand_fetch_stale
    }

    pub fn randomness_public_drand_consecutive_failures(&self) -> usize {
        self.randomness_public_drand_consecutive_failures
    }

    pub fn randomness_public_drand_backoff_remaining_ticks(&self) -> u64 {
        self.randomness_public_drand_backoff_remaining_ticks
    }

    pub fn randomness_public_drand_expected_latest_round(&self) -> u64 {
        self.randomness_public_drand_expected_latest_round
    }

    pub fn randomness_public_drand_fetched_round_lag(&self) -> u64 {
        self.randomness_public_drand_fetched_round_lag
    }

    pub fn randomness_public_drand_max_round_lag(&self) -> u64 {
        self.randomness_public_drand_max_round_lag
    }

    pub fn randomness_public_drand_rounds_per_chain_epoch(&self) -> u64 {
        self.randomness_public_drand_rounds_per_chain_epoch
    }

    pub fn randomness_public_drand_chain_epoch(&self) -> u64 {
        self.randomness_public_drand_chain_epoch
    }

    pub fn randomness_public_drand_fresh(&self) -> bool {
        self.randomness_public_drand_fresh
    }

    pub fn randomness_beacon_published(&self, source_id: &str, beacon_round: u64) -> bool {
        self.randomness_published_source_id == source_id
            && self.randomness_published_round == beacon_round
    }

    pub fn randomness_public_drand_poll_due(&mut self) -> bool {
        if self.randomness_public_drand_backoff_remaining_ticks == 0 {
            return true;
        }
        self.randomness_public_drand_backoff_remaining_ticks = self
            .randomness_public_drand_backoff_remaining_ticks
            .saturating_sub(1);
        self.randomness_public_drand_backoff_remaining_ticks == 0
    }

    pub fn record_served_request(&mut self) {
        self.served_requests = self.served_requests.saturating_add(1);
    }

    pub fn record_produced_block(&mut self) {
        self.produced_blocks = self.produced_blocks.saturating_add(1);
    }

    pub fn record_local_synthetic_job_published(&mut self) {
        self.local_synthetic_jobs_published = self.local_synthetic_jobs_published.saturating_add(1);
    }

    pub fn record_network_ingest(&mut self, ingested: NetworkEventIngest) {
        let applied_block_evidence = if ingested.applied_blocks > 0 {
            ingested.applied_blocks
        } else if ingested.block_payloads_applied > 0 {
            1
        } else {
            0
        };
        self.network_applied_blocks = self
            .network_applied_blocks
            .saturating_add(applied_block_evidence);
        self.network_events.accumulate(ingested);
    }

    pub fn record_miner_work_observation(
        &mut self,
        assigned_jobs: BTreeSet<Hash>,
        unreceipted_jobs: BTreeSet<Hash>,
    ) -> bool {
        let changed = self.miner_assigned_jobs_seen != assigned_jobs
            || self.miner_unreceipted_jobs != unreceipted_jobs;
        self.miner_assigned_jobs_seen = assigned_jobs;
        self.miner_unreceipted_jobs = unreceipted_jobs;
        changed
    }

    pub fn record_miner_receipt_submission(
        &mut self,
        receipts_submitted: usize,
        tensors_inserted: usize,
    ) {
        self.miner_receipts_submitted = self
            .miner_receipts_submitted
            .saturating_add(receipts_submitted);
        self.miner_tensors_inserted = self.miner_tensors_inserted.saturating_add(tensors_inserted);
    }

    pub fn record_miner_trace_bisection_round_submission(&mut self, rounds_submitted: usize) {
        self.miner_trace_bisection_rounds_submitted = self
            .miner_trace_bisection_rounds_submitted
            .saturating_add(rounds_submitted);
    }

    pub fn record_validator_trace_bisection_expectation_submission(
        &mut self,
        expectations_submitted: usize,
    ) {
        self.validator_trace_bisection_expectations_submitted = self
            .validator_trace_bisection_expectations_submitted
            .saturating_add(expectations_submitted);
    }

    pub fn record_validator_trace_bisection_referee_submission(
        &mut self,
        referees_submitted: usize,
    ) {
        self.validator_trace_bisection_referees_submitted = self
            .validator_trace_bisection_referees_submitted
            .saturating_add(referees_submitted);
    }

    pub fn record_validator_work_observation(
        &mut self,
        assigned_receipts: BTreeSet<Hash>,
        unattested_receipts: BTreeSet<Hash>,
        artifact_ready_receipts: BTreeSet<Hash>,
        artifact_missing_receipts: BTreeSet<Hash>,
    ) -> bool {
        let changed = self.validator_assigned_receipts_seen != assigned_receipts
            || self.validator_unattested_receipts != unattested_receipts
            || self.validator_artifact_ready_receipts != artifact_ready_receipts
            || self.validator_artifact_missing_receipts != artifact_missing_receipts;
        self.validator_assigned_receipts_seen = assigned_receipts;
        self.validator_unattested_receipts = unattested_receipts;
        self.validator_artifact_ready_receipts = artifact_ready_receipts;
        self.validator_artifact_missing_receipts = artifact_missing_receipts;
        changed
    }

    pub fn record_validator_audit_observation(
        &mut self,
        assigned_audits: BTreeSet<Hash>,
        unreported_audits: BTreeSet<Hash>,
        artifact_ready_audits: BTreeSet<Hash>,
        artifact_missing_audits: BTreeSet<Hash>,
    ) -> bool {
        let changed = self.validator_assigned_audits_seen != assigned_audits
            || self.validator_unreported_audits != unreported_audits
            || self.validator_audit_artifact_ready != artifact_ready_audits
            || self.validator_audit_artifact_missing != artifact_missing_audits;
        self.validator_assigned_audits_seen = assigned_audits;
        self.validator_unreported_audits = unreported_audits;
        self.validator_audit_artifact_ready = artifact_ready_audits;
        self.validator_audit_artifact_missing = artifact_missing_audits;
        changed
    }

    pub fn record_validator_block_proposal_observation(
        &mut self,
        settled_receipts: BTreeSet<Hash>,
        artifact_ready_receipts: BTreeSet<Hash>,
        attested_receipts: BTreeSet<Hash>,
    ) -> bool {
        let changed = self.validator_proposer_settled_receipts_seen != settled_receipts
            || self.validator_proposer_artifact_ready_receipts_seen != artifact_ready_receipts
            || self.validator_proposer_attested_receipts_seen != attested_receipts;
        self.validator_proposer_settled_receipts_seen = settled_receipts;
        self.validator_proposer_artifact_ready_receipts_seen = artifact_ready_receipts;
        self.validator_proposer_attested_receipts_seen = attested_receipts;
        changed
    }

    pub fn record_validator_attestation_submission(&mut self, attestations_submitted: usize) {
        self.validator_attestations_submitted = self
            .validator_attestations_submitted
            .saturating_add(attestations_submitted);
    }

    pub fn record_validator_audit_report_submission(&mut self, audit_reports_submitted: usize) {
        self.validator_audit_reports_submitted = self
            .validator_audit_reports_submitted
            .saturating_add(audit_reports_submitted);
    }

    pub fn record_validator_vrf_key_observation(
        &mut self,
        vrf_public_key: Hash,
        registered_new_key: bool,
    ) {
        self.validator_vrf_key_public_key = vrf_public_key;
        if registered_new_key {
            self.validator_vrf_key_registration_count =
                self.validator_vrf_key_registration_count.saturating_add(1);
        }
    }

    pub fn record_validator_block_proposal_submission(
        &mut self,
        blocks_proposed: usize,
        useful_blocks_proposed: usize,
        fallback_blocks_proposed: usize,
        receipts_proposed: usize,
    ) {
        self.validator_blocks_proposed = self
            .validator_blocks_proposed
            .saturating_add(blocks_proposed);
        self.validator_useful_blocks_proposed = self
            .validator_useful_blocks_proposed
            .saturating_add(useful_blocks_proposed);
        self.validator_fallback_blocks_proposed = self
            .validator_fallback_blocks_proposed
            .saturating_add(fallback_blocks_proposed);
        self.validator_receipts_proposed = self
            .validator_receipts_proposed
            .saturating_add(receipts_proposed);
    }

    pub fn record_validator_block_vote_submission(&mut self, block_votes_submitted: usize) {
        self.validator_block_votes_submitted = self
            .validator_block_votes_submitted
            .saturating_add(block_votes_submitted);
    }

    pub fn record_validator_trace_bisection_open_submission(&mut self, opens_submitted: usize) {
        self.validator_trace_bisection_opens_submitted = self
            .validator_trace_bisection_opens_submitted
            .saturating_add(opens_submitted);
    }

    pub fn record_validator_remote_tensor_fetch(
        &mut self,
        attempts: usize,
        successes: usize,
        failures: usize,
        bytes: usize,
        tensors_inserted: usize,
    ) {
        self.validator_remote_tensor_fetch_attempts = self
            .validator_remote_tensor_fetch_attempts
            .saturating_add(attempts);
        self.validator_remote_tensor_fetch_successes = self
            .validator_remote_tensor_fetch_successes
            .saturating_add(successes);
        self.validator_remote_tensor_fetch_failures = self
            .validator_remote_tensor_fetch_failures
            .saturating_add(failures);
        self.validator_remote_tensor_fetch_bytes = self
            .validator_remote_tensor_fetch_bytes
            .saturating_add(bytes);
        self.validator_remote_tensors_inserted = self
            .validator_remote_tensors_inserted
            .saturating_add(tensors_inserted);
    }

    pub fn record_randomness_beacon_observed(&mut self, source_id: &str, beacon_round: u64) {
        self.randomness_beacons_observed = self.randomness_beacons_observed.saturating_add(1);
        self.randomness_latest_source_id = source_id.to_owned();
        self.randomness_latest_round = beacon_round;
        self.randomness_last_error.clear();
    }

    pub fn record_randomness_beacon_applied(&mut self, source_id: &str, beacon_round: u64) {
        self.randomness_beacons_applied = self.randomness_beacons_applied.saturating_add(1);
        self.randomness_latest_source_id = source_id.to_owned();
        self.randomness_latest_round = beacon_round;
        self.randomness_last_error.clear();
    }

    pub fn record_randomness_beacon_published(&mut self, source_id: &str, beacon_round: u64) {
        self.randomness_published_source_id = source_id.to_owned();
        self.randomness_published_round = beacon_round;
    }

    pub fn record_randomness_beacon_skipped(&mut self, source_id: &str, beacon_round: u64) {
        self.randomness_beacons_skipped = self.randomness_beacons_skipped.saturating_add(1);
        self.randomness_latest_source_id = source_id.to_owned();
        self.randomness_latest_round = beacon_round;
        self.randomness_last_error.clear();
    }

    pub fn record_randomness_beacon_failure(
        &mut self,
        source_id: &str,
        beacon_round: u64,
        error: &str,
    ) {
        self.randomness_beacon_failures = self.randomness_beacon_failures.saturating_add(1);
        self.randomness_latest_source_id = source_id.to_owned();
        self.randomness_latest_round = beacon_round;
        self.randomness_last_error = error.to_owned();
    }

    pub fn record_randomness_public_drand_fetch_attempt(&mut self) {
        self.randomness_public_drand_fetch_attempts = self
            .randomness_public_drand_fetch_attempts
            .saturating_add(1);
    }

    pub fn record_randomness_public_drand_fetch_success(&mut self, poll_interval_ticks: u64) {
        self.randomness_public_drand_fetch_successes = self
            .randomness_public_drand_fetch_successes
            .saturating_add(1);
        self.randomness_public_drand_consecutive_failures = 0;
        self.randomness_public_drand_backoff_remaining_ticks = poll_interval_ticks;
    }

    pub fn record_randomness_public_drand_fetch_stale(&mut self, poll_interval_ticks: u64) {
        self.randomness_public_drand_fetch_stale =
            self.randomness_public_drand_fetch_stale.saturating_add(1);
        self.randomness_public_drand_consecutive_failures = 0;
        self.randomness_public_drand_backoff_remaining_ticks = poll_interval_ticks;
    }

    pub fn record_randomness_public_drand_fetch_failure(
        &mut self,
        poll_interval_ticks: u64,
        max_backoff_ticks: u64,
    ) {
        self.randomness_public_drand_consecutive_failures = self
            .randomness_public_drand_consecutive_failures
            .saturating_add(1);
        let exponent = self
            .randomness_public_drand_consecutive_failures
            .saturating_sub(1)
            .min(16) as u32;
        let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
        let backoff = poll_interval_ticks
            .saturating_mul(multiplier)
            .min(max_backoff_ticks);
        self.randomness_public_drand_backoff_remaining_ticks = backoff.max(1);
    }

    pub fn record_randomness_public_drand_mapping_observation(
        &mut self,
        expected_latest_round: u64,
        fetched_round_lag: u64,
        max_round_lag: u64,
        rounds_per_chain_epoch: u64,
        chain_epoch: u64,
    ) {
        self.randomness_public_drand_expected_latest_round = expected_latest_round;
        self.randomness_public_drand_fetched_round_lag = fetched_round_lag;
        self.randomness_public_drand_max_round_lag = max_round_lag;
        self.randomness_public_drand_rounds_per_chain_epoch = rounds_per_chain_epoch;
        self.randomness_public_drand_chain_epoch = chain_epoch;
        self.randomness_public_drand_fresh = fetched_round_lag <= max_round_lag;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn runtime_state_tracks_loop_counters() {
        let mut state = NodeRuntimeState::default();
        state.pending_payloads_mut().queue_receipt([9; 32], vec![9]);
        let mut assigned_jobs = BTreeSet::new();
        assigned_jobs.insert([7; 32]);
        let mut unreceipted_jobs = BTreeSet::new();
        unreceipted_jobs.insert([7; 32]);
        assert!(state.record_miner_work_observation(assigned_jobs, unreceipted_jobs));
        state.record_served_request();
        state.record_produced_block();
        state.record_network_ingest(NetworkEventIngest {
            events: 1,
            applied_blocks: 2,
            ..NetworkEventIngest::default()
        });

        assert_eq!(state.served_requests(), 1);
        assert_eq!(state.produced_blocks(), 1);
        state.record_local_synthetic_job_published();
        assert_eq!(state.local_synthetic_jobs_published(), 1);
        assert_eq!(state.network_applied_blocks(), 2);
        assert_eq!(state.network_events().events, 1);
        assert_eq!(state.pending_payloads().pending_receipt_count(), 1);
        assert_eq!(state.pending_payloads().pending_attestation_count(), 0);
        assert_eq!(state.miner_assigned_jobs_seen(), 1);
        assert_eq!(state.miner_unreceipted_jobs(), 1);
        assert!(state.miner_work_ready());
        assert!(state.record_miner_work_observation(BTreeSet::from([[7; 32]]), BTreeSet::new()));
        assert_eq!(state.miner_assigned_jobs_seen(), 1);
        assert_eq!(state.miner_unreceipted_jobs(), 0);
        assert!(!state.miner_work_ready());
        state.record_miner_receipt_submission(1, 3);
        assert_eq!(state.miner_receipts_submitted(), 1);
        assert_eq!(state.miner_tensors_inserted(), 3);
        state.record_miner_trace_bisection_round_submission(1);
        assert_eq!(state.miner_trace_bisection_rounds_submitted(), 1);
        assert!(state.record_validator_work_observation(
            BTreeSet::from([[8; 32]]),
            BTreeSet::from([[8; 32]]),
            BTreeSet::from([[8; 32]]),
            BTreeSet::new(),
        ));
        assert_eq!(state.validator_assigned_receipts_seen(), 1);
        assert_eq!(state.validator_unattested_receipts(), 1);
        assert_eq!(state.validator_artifact_ready_receipts(), 1);
        assert_eq!(state.validator_artifact_missing_receipts(), 0);
        assert!(state.validator_work_ready());
        assert!(state.record_validator_work_observation(
            BTreeSet::from([[8; 32]]),
            BTreeSet::from([[8; 32]]),
            BTreeSet::new(),
            BTreeSet::from([[8; 32]]),
        ));
        assert_eq!(state.validator_artifact_ready_receipts(), 0);
        assert_eq!(state.validator_artifact_missing_receipts(), 1);
        assert!(!state.validator_work_ready());
        state.record_validator_attestation_submission(1);
        assert_eq!(state.validator_attestations_submitted(), 1);
        assert!(state.record_validator_block_proposal_observation(
            BTreeSet::from([[5; 32]]),
            BTreeSet::from([[5; 32]]),
            BTreeSet::from([[5; 32]]),
        ));
        assert_eq!(state.validator_proposer_settled_receipts_seen(), 1);
        assert_eq!(state.validator_proposer_artifact_ready_receipts_seen(), 1);
        assert_eq!(state.validator_proposer_attested_receipts_seen(), 1);
        assert!(state.validator_proposer_work_ready());
        state.record_validator_block_proposal_submission(2, 1, 1, 3);
        assert_eq!(state.validator_blocks_proposed(), 2);
        assert_eq!(state.validator_useful_blocks_proposed(), 1);
        assert_eq!(state.validator_fallback_blocks_proposed(), 1);
        assert_eq!(state.validator_receipts_proposed(), 3);
        state.record_validator_block_vote_submission(1);
        assert_eq!(state.validator_block_votes_submitted(), 1);
        state.record_validator_trace_bisection_open_submission(1);
        assert_eq!(state.validator_trace_bisection_opens_submitted(), 1);
        state.record_validator_trace_bisection_referee_submission(1);
        assert_eq!(state.validator_trace_bisection_referees_submitted(), 1);
        state.record_validator_remote_tensor_fetch(3, 2, 1, 128, 2);
        assert_eq!(state.validator_remote_tensor_fetch_attempts(), 3);
        assert_eq!(state.validator_remote_tensor_fetch_successes(), 2);
        assert_eq!(state.validator_remote_tensor_fetch_failures(), 1);
        assert_eq!(state.validator_remote_tensor_fetch_bytes(), 128);
        assert_eq!(state.validator_remote_tensors_inserted(), 2);
        state.record_randomness_beacon_observed("fixture", 7);
        state.record_randomness_beacon_applied("fixture", 7);
        state.record_randomness_beacon_skipped("fixture", 7);
        state.record_randomness_beacon_failure("fixture", 8, "bad proof");
        state.record_randomness_public_drand_fetch_attempt();
        state.record_randomness_public_drand_fetch_success(3);
        state.record_randomness_public_drand_mapping_observation(10, 1, 2, 20, 4);
        assert!(!state.randomness_public_drand_poll_due());
        assert!(!state.randomness_public_drand_poll_due());
        assert!(state.randomness_public_drand_poll_due());
        state.record_randomness_public_drand_fetch_failure(3, 12);
        assert_eq!(state.randomness_beacons_observed(), 1);
        assert_eq!(state.randomness_beacons_applied(), 1);
        assert_eq!(state.randomness_beacons_skipped(), 1);
        assert_eq!(state.randomness_beacon_failures(), 1);
        assert_eq!(state.randomness_latest_source_id(), "fixture");
        assert_eq!(state.randomness_latest_round(), 8);
        assert_eq!(state.randomness_last_error(), "bad proof");
        assert_eq!(state.randomness_public_drand_fetch_attempts(), 1);
        assert_eq!(state.randomness_public_drand_fetch_successes(), 1);
        assert_eq!(state.randomness_public_drand_consecutive_failures(), 1);
        assert_eq!(state.randomness_public_drand_backoff_remaining_ticks(), 3);
        assert_eq!(state.randomness_public_drand_expected_latest_round(), 10);
        assert_eq!(state.randomness_public_drand_fetched_round_lag(), 1);
        assert_eq!(state.randomness_public_drand_max_round_lag(), 2);
        assert_eq!(state.randomness_public_drand_rounds_per_chain_epoch(), 20);
        assert_eq!(state.randomness_public_drand_chain_epoch(), 4);
        assert!(state.randomness_public_drand_fresh());
    }

    #[test]
    fn network_event_ingest_activity_checks_each_progress_signal() {
        assert!(!NetworkEventIngest::default().has_activity());
        assert!(
            NetworkEventIngest {
                events: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                job_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                receipt_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                attestation_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                block_payloads_applied: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                invalid_events: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
        assert!(
            NetworkEventIngest {
                applied_blocks: 1,
                ..NetworkEventIngest::default()
            }
            .has_activity()
        );
    }
}
