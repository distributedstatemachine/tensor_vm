use super::*;

#[test]
fn network_event_ingest_accumulates_runtime_counters() {
    let mut cumulative = NetworkEventIngest {
        events: 2,
        block_announcements: 1,
        block_headers: 1,
        block_payloads: 1,
        block_payloads_applied: 1,
        block_votes: 1,
        block_votes_applied: 1,
        block_check_challenges: 1,
        block_check_challenges_applied: 1,
        jobs: 1,
        job_payloads: 1,
        job_payloads_applied: 1,
        receipts: 0,
        receipt_payloads: 0,
        receipt_payloads_applied: 0,
        attestations: 0,
        attestation_payloads: 0,
        attestation_payloads_applied: 0,
        validator_audit_reports: 0,
        validator_audit_reports_applied: 0,
        peers: 0,
        invalid_events: 0,
        applied_blocks: 1,
    };
    cumulative.accumulate(NetworkEventIngest {
        events: 4,
        block_announcements: 1,
        block_headers: 0,
        block_payloads: 2,
        block_payloads_applied: 2,
        block_votes: 2,
        block_votes_applied: 2,
        block_check_challenges: 2,
        block_check_challenges_applied: 2,
        jobs: 0,
        job_payloads: 2,
        job_payloads_applied: 2,
        receipts: 1,
        receipt_payloads: 1,
        receipt_payloads_applied: 1,
        attestations: 1,
        attestation_payloads: 1,
        attestation_payloads_applied: 1,
        validator_audit_reports: 1,
        validator_audit_reports_applied: 1,
        peers: 1,
        invalid_events: 1,
        applied_blocks: 2,
    });

    assert!(cumulative.has_activity());
    assert_eq!(cumulative.events, 6);
    assert_eq!(cumulative.block_announcements, 2);
    assert_eq!(cumulative.block_headers, 1);
    assert_eq!(cumulative.block_payloads, 3);
    assert_eq!(cumulative.block_payloads_applied, 3);
    assert_eq!(cumulative.block_votes, 3);
    assert_eq!(cumulative.block_votes_applied, 3);
    assert_eq!(cumulative.block_check_challenges, 3);
    assert_eq!(cumulative.block_check_challenges_applied, 3);
    assert_eq!(cumulative.jobs, 1);
    assert_eq!(cumulative.job_payloads, 3);
    assert_eq!(cumulative.job_payloads_applied, 3);
    assert_eq!(cumulative.receipts, 1);
    assert_eq!(cumulative.receipt_payloads, 1);
    assert_eq!(cumulative.receipt_payloads_applied, 1);
    assert_eq!(cumulative.attestations, 1);
    assert_eq!(cumulative.attestation_payloads, 1);
    assert_eq!(cumulative.attestation_payloads_applied, 1);
    assert_eq!(cumulative.validator_audit_reports, 1);
    assert_eq!(cumulative.validator_audit_reports_applied, 1);
    assert_eq!(cumulative.peers, 1);
    assert_eq!(cumulative.invalid_events, 1);
    assert_eq!(cumulative.applied_blocks, 3);
}

#[test]
fn service_runtime_state_owns_loop_counters_and_pending_payloads() {
    let mut state = NodeRuntimeState::default();
    state.record_served_request();
    state.record_produced_block();
    state.record_network_ingest(NetworkEventIngest {
        events: 1,
        receipt_payloads: 1,
        receipt_payloads_applied: 1,
        applied_blocks: 2,
        ..NetworkEventIngest::default()
    });

    assert_eq!(state.served_requests(), 1);
    assert_eq!(state.produced_blocks(), 1);
    assert_eq!(state.network_applied_blocks(), 2);
    assert_eq!(state.network_events().events, 1);
    assert_eq!(state.network_events().receipt_payloads, 1);
    assert_eq!(state.network_events().receipt_payloads_applied, 1);
    assert!(state.pending_payloads().is_empty());
    state.record_validator_block_vote_submission(1);
    assert_eq!(state.validator_block_votes_submitted(), 1);
}
