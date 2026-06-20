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
            || self.block_payloads_applied > 0
            || self.block_votes_applied > 0
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
        self.peers = self.peers.saturating_add(other.peers);
        self.invalid_events = self.invalid_events.saturating_add(other.invalid_events);
        self.applied_blocks = self.applied_blocks.saturating_add(other.applied_blocks);
    }
}

#[derive(Debug, Default)]
pub struct NodeRuntimeState {
    served_requests: usize,
    produced_blocks: usize,
    network_applied_blocks: usize,
    network_events: NetworkEventIngest,
    pending_network_payloads: PendingNetworkPayloads,
    miner_assigned_jobs_seen: BTreeSet<Hash>,
    miner_unreceipted_jobs: BTreeSet<Hash>,
    miner_receipts_submitted: usize,
    miner_tensors_inserted: usize,
    validator_assigned_receipts_seen: BTreeSet<Hash>,
    validator_unattested_receipts: BTreeSet<Hash>,
    validator_artifact_ready_receipts: BTreeSet<Hash>,
    validator_artifact_missing_receipts: BTreeSet<Hash>,
    validator_assigned_audits_seen: BTreeSet<Hash>,
    validator_unreported_audits: BTreeSet<Hash>,
    validator_audit_artifact_ready: BTreeSet<Hash>,
    validator_audit_artifact_missing: BTreeSet<Hash>,
    validator_proposer_settled_receipts_seen: BTreeSet<Hash>,
    validator_attestations_submitted: usize,
    validator_audit_reports_submitted: usize,
    validator_blocks_proposed: usize,
    validator_useful_blocks_proposed: usize,
    validator_fallback_blocks_proposed: usize,
    validator_receipts_proposed: usize,
    validator_block_votes_submitted: usize,
    validator_remote_tensor_fetch_attempts: usize,
    validator_remote_tensor_fetch_successes: usize,
    validator_remote_tensor_fetch_failures: usize,
    validator_remote_tensor_fetch_bytes: usize,
    validator_remote_tensors_inserted: usize,
}

impl NodeRuntimeState {
    pub fn served_requests(&self) -> usize {
        self.served_requests
    }

    pub fn produced_blocks(&self) -> usize {
        self.produced_blocks
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

    pub fn validator_proposer_work_ready(&self) -> bool {
        !self.validator_proposer_settled_receipts_seen.is_empty()
    }

    pub fn validator_attestations_submitted(&self) -> usize {
        self.validator_attestations_submitted
    }

    pub fn validator_audit_reports_submitted(&self) -> usize {
        self.validator_audit_reports_submitted
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

    pub fn record_served_request(&mut self) {
        self.served_requests = self.served_requests.saturating_add(1);
    }

    pub fn record_produced_block(&mut self) {
        self.produced_blocks = self.produced_blocks.saturating_add(1);
    }

    pub fn record_network_ingest(&mut self, ingested: NetworkEventIngest) {
        self.network_applied_blocks = self
            .network_applied_blocks
            .saturating_add(ingested.applied_blocks);
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
    ) -> bool {
        let changed = self.validator_proposer_settled_receipts_seen != settled_receipts;
        self.validator_proposer_settled_receipts_seen = settled_receipts;
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
        assert!(state.record_validator_block_proposal_observation(BTreeSet::from([[5; 32]])));
        assert_eq!(state.validator_proposer_settled_receipts_seen(), 1);
        assert!(state.validator_proposer_work_ready());
        state.record_validator_block_proposal_submission(2, 1, 1, 3);
        assert_eq!(state.validator_blocks_proposed(), 2);
        assert_eq!(state.validator_useful_blocks_proposed(), 1);
        assert_eq!(state.validator_fallback_blocks_proposed(), 1);
        assert_eq!(state.validator_receipts_proposed(), 3);
        state.record_validator_block_vote_submission(1);
        assert_eq!(state.validator_block_votes_submitted(), 1);
        state.record_validator_remote_tensor_fetch(3, 2, 1, 128, 2);
        assert_eq!(state.validator_remote_tensor_fetch_attempts(), 3);
        assert_eq!(state.validator_remote_tensor_fetch_successes(), 2);
        assert_eq!(state.validator_remote_tensor_fetch_failures(), 1);
        assert_eq!(state.validator_remote_tensor_fetch_bytes(), 128);
        assert_eq!(state.validator_remote_tensors_inserted(), 2);
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
