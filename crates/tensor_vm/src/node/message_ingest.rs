use super::{
    NetworkBlockPayloadApply, NetworkEventContext, NetworkEventIngest, NetworkPayloadApply,
    PendingNetworkPayloads,
    payload_application::{
        apply_network_attestation_payload, apply_network_block_check_challenge_payload,
        apply_network_block_vote_payload, apply_network_job_payload,
        apply_network_observed_block_check_challenge_payload, apply_network_receipt_payload,
        apply_network_validator_audit_report_payload,
    },
    payload_processor,
};
use crate::api::P2pMessage;

pub fn ingest_network_messages<C: NetworkEventContext + ?Sized>(
    context: &mut C,
    messages: Vec<P2pMessage>,
    _local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let mut ingested = NetworkEventIngest::default();
    for message in network_ingest_order(messages) {
        ingested.events = ingested.events.saturating_add(1);
        match message {
            P2pMessage::NewBlock(block_hash) => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                if block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewBlockHeader { height, block_hash } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_headers = ingested.block_headers.saturating_add(1);
                if height == 0 || block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
            }
            P2pMessage::NewBlockPayload {
                height,
                block_hash,
                payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_payloads = ingested.block_payloads.saturating_add(1);
                if height == 0 || block_hash == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match context.apply_block_payload(height, block_hash, &payload) {
                    NetworkBlockPayloadApply::Applied { appended } => {
                        ingested.block_payloads_applied =
                            ingested.block_payloads_applied.saturating_add(1);
                        ingested.applied_blocks = ingested.applied_blocks.saturating_add(appended);
                    }
                    NetworkBlockPayloadApply::Pending => {
                        pending_payloads.queue_block(height, block_hash, payload);
                    }
                    NetworkBlockPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewBlockVotePayload {
                block_hash,
                validator,
                payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_votes = ingested.block_votes.saturating_add(1);
                if block_hash == [0; 32] || validator == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_block_vote_payload(
                    context.chain(),
                    block_hash,
                    validator,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.block_votes_applied =
                            ingested.block_votes_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_block_vote(block_hash, validator, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewBlockCheckChallenge(challenge_id) => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_check_challenges = ingested.block_check_challenges.saturating_add(1);
                if challenge_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewBlockCheckChallengePayload {
                challenge_id,
                block_hash,
                challenger,
                payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_check_challenges = ingested.block_check_challenges.saturating_add(1);
                if challenge_id == [0; 32] || block_hash == [0; 32] || challenger == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_block_check_challenge_payload(
                    context.chain(),
                    challenge_id,
                    block_hash,
                    challenger,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.block_check_challenges_applied =
                            ingested.block_check_challenges_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_block_check_challenge(
                            challenge_id,
                            block_hash,
                            challenger,
                            payload,
                        );
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id,
                block_hash,
                challenger,
                observed_block_payload,
                challenge_payload,
            } => {
                ingested.block_announcements = ingested.block_announcements.saturating_add(1);
                ingested.block_check_challenges = ingested.block_check_challenges.saturating_add(1);
                if challenge_id == [0; 32] || block_hash == [0; 32] || challenger == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_observed_block_check_challenge_payload(
                    context.chain(),
                    challenge_id,
                    block_hash,
                    challenger,
                    &observed_block_payload,
                    &challenge_payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.block_check_challenges_applied =
                            ingested.block_check_challenges_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_observed_block_check_challenge(
                            challenge_id,
                            block_hash,
                            challenger,
                            observed_block_payload,
                            challenge_payload,
                        );
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewJob(job_id) => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                if job_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewJobPayload { job_id, payload } => {
                ingested.jobs = ingested.jobs.saturating_add(1);
                ingested.job_payloads = ingested.job_payloads.saturating_add(1);
                match apply_network_job_payload(context.chain(), job_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.job_payloads_applied =
                            ingested.job_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_job(job_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewReceipt(receipt_id) => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                if receipt_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewReceiptPayload {
                receipt_id,
                payload,
            } => {
                ingested.receipts = ingested.receipts.saturating_add(1);
                ingested.receipt_payloads = ingested.receipt_payloads.saturating_add(1);
                match apply_network_receipt_payload(context.chain(), receipt_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.receipt_payloads_applied =
                            ingested.receipt_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_receipt(receipt_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewAttestation(attestation_id) => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                if attestation_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewAttestationPayload {
                attestation_id,
                payload,
            } => {
                ingested.attestations = ingested.attestations.saturating_add(1);
                ingested.attestation_payloads = ingested.attestation_payloads.saturating_add(1);
                match apply_network_attestation_payload(context.chain(), attestation_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        ingested.attestation_payloads_applied =
                            ingested.attestation_payloads_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_attestation(attestation_id, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewValidatorAuditReport(audit_id) => {
                ingested.validator_audit_reports =
                    ingested.validator_audit_reports.saturating_add(1);
                if audit_id == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::NewValidatorAuditReportPayload {
                audit_id,
                auditor,
                payload,
            } => {
                ingested.validator_audit_reports =
                    ingested.validator_audit_reports.saturating_add(1);
                if audit_id == [0; 32] || auditor == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_validator_audit_report_payload(
                    context.chain(),
                    audit_id,
                    auditor,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.validator_audit_reports_applied =
                            ingested.validator_audit_reports_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads.queue_validator_audit_report(audit_id, auditor, payload);
                    }
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::PeerInfo { address } => {
                ingested.peers = ingested.peers.saturating_add(1);
                if address == [0; 32] {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                }
            }
            P2pMessage::RequestTensorChunk { .. }
            | P2pMessage::TensorChunkResponse { .. }
            | P2pMessage::RequestTensorRow { .. }
            | P2pMessage::TensorRowResponse { .. }
            | P2pMessage::RequestTensorByCommitmentRoot { .. }
            | P2pMessage::TensorByCommitmentRootResponse { .. }
            | P2pMessage::RequestProgram(_)
            | P2pMessage::ProgramResponse { .. }
            | P2pMessage::RequestTraceOpening { .. }
            | P2pMessage::TraceOpeningResponse { .. } => {
                ingested.invalid_events = ingested.invalid_events.saturating_add(1);
            }
        }
    }
    let mut processor = payload_processor::ContextNetworkPayloadProcessor { context };
    ingested.accumulate(pending_payloads.retry_with(&mut processor));
    Ok(ingested)
}

pub fn network_ingest_order(messages: Vec<P2pMessage>) -> Vec<P2pMessage> {
    let mut other_messages = Vec::new();
    let mut block_payloads = Vec::new();
    let mut block_announcements = Vec::new();
    for message in messages {
        if is_block_payload(&message) {
            block_payloads.push(message);
        } else if is_block_announcement(&message) {
            block_announcements.push(message);
        } else {
            other_messages.push(message);
        }
    }
    other_messages.append(&mut block_payloads);
    other_messages.append(&mut block_announcements);
    other_messages
}

fn is_block_announcement(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::NewBlock(_) | P2pMessage::NewBlockHeader { .. }
    )
}

fn is_block_payload(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::NewBlockPayload { .. }
            | P2pMessage::NewBlockCheckChallengePayload { .. }
            | P2pMessage::NewObservedBlockCheckChallengePayload { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::super::{
        ChainNetworkPayloadProcessor, NetworkBlockPayloadApply, NetworkEventContext,
        PendingNetworkPayloads, attestation_announcement_hash,
    };
    use super::*;
    use crate::{
        chain::{Chain, ChainCommand, ChainEngine, ChainParams, JobState, ValidatorAuditReport},
        jobs::{MatmulJob, PrimitiveType, TensorOpReceipt},
        p2p::{
            encode_attestation_payload, encode_block_check_challenge_payload, encode_block_payload,
            encode_job_payload, encode_receipt_payload, encode_validator_audit_report_payload,
        },
        scheduler::{JobScheduler, SyntheticLocalJobSource},
        testnet::{LocalTestnet, TestnetConfig},
        types::{Hash, address, hash_bytes},
        verify::{
            AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
            verify_tensor_op,
        },
    };

    fn local_matmul_round(seed_label: &[u8]) -> LocalTestnet {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"tensor-vm-node-payload-test", &[seed_label]),
        );
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet
    }

    fn audit_report_chain() -> (Chain, Hash, Hash) {
        let beacon = hash_bytes(b"test", &[b"ingest-audit-report-beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            validator_audit_sample_numerator: 1,
            validator_audit_sample_denominator: 1,
            validator_audit_window_blocks: 3,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"ingest-audit-miner");
        let candidate_auditor = address(b"ingest-audit-auditor");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(candidate_auditor, 10_000).unwrap();
        let validators: Vec<_> = (0..4)
            .map(|i| address(format!("ingest-audit-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let assigned = JobScheduler::default()
            .assign_validators(
                &chain,
                receipt.receipt_id,
                &chain.validator_assignment_seed(&receipt.receipt_id),
            )
            .validators[0];
        chain
            .submit_attestation(ValidatorAttestation::new(
                assigned,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"ingest-audit-attestation"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
        let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
        chain.produce_block(proposer, 1_000).unwrap();
        let audit_id = *chain
            .state()
            .validator_audit_assignments()
            .keys()
            .next()
            .expect("audit assignment should exist");
        let auditor = chain.state().validator_audit_assignments()[&audit_id].auditor;
        (chain, audit_id, auditor)
    }

    fn rewarded_block_check_challenge_chain() -> (
        Chain,
        crate::chain::TensorBlock,
        crate::DeterministicBlockCheckChallenge,
        Hash,
    ) {
        let beacon = hash_bytes(b"test", &[b"ingest-block-check-challenge-beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            challenge_window_epochs: 1,
            epoch_length: 4,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                validators_per_job: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"ingest-block-check-miner");
        let proposer = address(b"ingest-block-check-proposer");
        let challenger = address(b"ingest-block-check-challenger");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(proposer, 10_000).unwrap();
        chain.register_validator(challenger, 10_000).unwrap();

        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &hash_bytes(b"test", &[b"ingest-block-check-validation"]),
            &chain.params().freivalds,
        )
        .unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
        let assigned_validator = JobScheduler::default()
            .assign_validators(&chain, receipt.receipt_id, &assignment_seed)
            .validators
            .into_iter()
            .next()
            .unwrap();
        chain.insert_attestation_for_testing(ValidatorAttestation::new(
            assigned_validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ));
        chain.settle_epoch(1_000, 500);
        let block = chain
            .produce_block_with_rewards(proposer, 1_000, 900, 100)
            .unwrap();
        let diagnostic = chain
            .deterministic_bad_block_check_challenge(&block, challenger)
            .unwrap();
        (chain, block, diagnostic, challenger)
    }

    struct TestNetworkEventContext {
        chain: Chain,
        applied_payloads: Vec<(u64, Hash)>,
        applied_blocks: usize,
    }

    impl TestNetworkEventContext {
        fn new(seed_label: &[u8]) -> Self {
            Self {
                chain: Chain::new(hash_bytes(
                    b"tensor-vm-node-event-context-test",
                    &[seed_label],
                )),
                applied_payloads: Vec::new(),
                applied_blocks: 2,
            }
        }
    }

    impl NetworkEventContext for TestNetworkEventContext {
        fn chain(&mut self) -> &mut Chain {
            &mut self.chain
        }

        fn apply_block_payload(
            &mut self,
            height: u64,
            block_hash: Hash,
            _payload: &[u8],
        ) -> NetworkBlockPayloadApply {
            self.applied_payloads.push((height, block_hash));
            NetworkBlockPayloadApply::Applied {
                appended: self.applied_blocks,
            }
        }
    }

    #[test]
    fn network_ingest_order_applies_payload_dependencies_before_blocks() {
        let block_hash = hash_bytes(b"test", &[b"announced-block"]);
        let job_id = hash_bytes(b"test", &[b"announced-job"]);
        let receipt_id = hash_bytes(b"test", &[b"announced-receipt"]);
        let messages = network_ingest_order(vec![
            P2pMessage::NewJobPayload {
                job_id,
                payload: vec![1, 2, 3],
            },
            P2pMessage::NewReceipt(receipt_id),
            P2pMessage::NewBlockHeader {
                height: 3,
                block_hash,
            },
            P2pMessage::NewBlockPayload {
                height: 3,
                block_hash,
                payload: vec![4, 5, 6],
            },
            P2pMessage::NewJob(job_id),
            P2pMessage::NewBlock(block_hash),
        ]);

        assert!(matches!(messages[0], P2pMessage::NewJobPayload { .. }));
        assert!(matches!(messages[1], P2pMessage::NewReceipt(_)));
        assert!(matches!(messages[2], P2pMessage::NewJob(_)));
        assert!(matches!(messages[3], P2pMessage::NewBlockPayload { .. }));
        assert!(matches!(messages[4], P2pMessage::NewBlockHeader { .. }));
        assert!(matches!(messages[5], P2pMessage::NewBlock(_)));
    }

    #[test]
    fn network_event_driver_treats_block_headers_as_announcements_only() {
        let block_hash = hash_bytes(b"test", &[b"network-head"]);
        let messages = vec![P2pMessage::NewBlockHeader {
            height: 4,
            block_hash,
        }];
        let mut producer_context = TestNetworkEventContext::new(b"producer");
        let mut pending = PendingNetworkPayloads::default();

        let producer_ingested =
            ingest_network_messages(&mut producer_context, messages.clone(), true, &mut pending)
                .unwrap();

        assert_eq!(producer_ingested.block_headers, 1);
        assert_eq!(producer_ingested.applied_blocks, 0);

        let mut non_producer_context = TestNetworkEventContext::new(b"non-producer");
        let non_producer_ingested = ingest_network_messages(
            &mut non_producer_context,
            messages,
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(non_producer_ingested.block_headers, 1);
        assert_eq!(non_producer_ingested.applied_blocks, 0);
    }

    #[test]
    fn network_event_driver_dispatches_block_payloads_for_all_roles() {
        let block_hash = hash_bytes(b"test", &[b"network-payload-head"]);
        let messages = vec![P2pMessage::NewBlockPayload {
            height: 4,
            block_hash,
            payload: vec![7, 8, 9],
        }];
        let mut producer_context = TestNetworkEventContext::new(b"producer-payload");
        let producer_ingested = ingest_network_messages(
            &mut producer_context,
            messages.clone(),
            true,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(producer_ingested.block_payloads, 1);
        assert_eq!(producer_ingested.block_payloads_applied, 1);
        assert_eq!(producer_ingested.applied_blocks, 2);
        assert_eq!(producer_context.applied_payloads, vec![(4, block_hash)]);

        let mut non_producer_context = TestNetworkEventContext::new(b"non-producer-payload");
        let non_producer_ingested = ingest_network_messages(
            &mut non_producer_context,
            messages,
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();

        assert_eq!(non_producer_ingested.block_payloads, 1);
        assert_eq!(non_producer_ingested.block_payloads_applied, 1);
        assert_eq!(non_producer_ingested.applied_blocks, 2);
        assert_eq!(non_producer_context.applied_payloads, vec![(4, block_hash)]);
    }

    #[test]
    fn network_event_driver_counts_invalid_runtime_messages() {
        let mut context = TestNetworkEventContext::new(b"invalid-events");
        let mut pending = PendingNetworkPayloads::default();
        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewBlock([0; 32]),
                P2pMessage::NewBlockHeader {
                    height: 0,
                    block_hash: hash_bytes(b"test", &[b"bad-height"]),
                },
                P2pMessage::NewJob([0; 32]),
                P2pMessage::NewReceipt([0; 32]),
                P2pMessage::NewAttestation([0; 32]),
                P2pMessage::PeerInfo { address: [0; 32] },
                P2pMessage::RequestProgram(hash_bytes(b"test", &[b"program"])),
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 7);
        assert_eq!(ingested.block_announcements, 2);
        assert_eq!(ingested.block_headers, 1);
        assert_eq!(ingested.jobs, 1);
        assert_eq!(ingested.receipts, 1);
        assert_eq!(ingested.attestations, 1);
        assert_eq!(ingested.peers, 1);
        assert_eq!(ingested.invalid_events, 7);
    }

    #[test]
    fn network_event_driver_applies_payloads_and_retries_pending_payloads() {
        let testnet = local_matmul_round(b"driver-payloads");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);
        let mut context = TestNetworkEventContext {
            chain: testnet.chain.clone(),
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        context.chain.remove_job_for_testing(&job_id);
        context.chain.remove_receipt_for_testing(&receipt_id);
        context.chain.remove_attestations_for_testing(&receipt_id);
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: encode_receipt_payload(&receipt),
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(&attestation),
                },
                P2pMessage::NewJobPayload {
                    job_id,
                    payload: encode_job_payload(&job),
                },
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 3);
        assert_eq!(ingested.job_payloads_applied, 1);
        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(context.chain.state().jobs().get(&job_id), Some(&job));
        assert_eq!(
            context.chain.state().receipts().get(&receipt_id),
            Some(&receipt)
        );
        assert_eq!(
            context
                .chain
                .state()
                .attestations()
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }

    #[test]
    fn network_event_driver_queues_graph_job_until_program_body_arrives() {
        let seed = hash_bytes(b"test", &[b"ingest-graph-job-pending-program"]);
        let mut source = SyntheticLocalJobSource::default();
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let job = JobState::GraphExecution(source.next_graph_job(&Chain::new(seed)));
        let job_id = job.job_id();
        let mut context = TestNetworkEventContext {
            chain: Chain::new(seed),
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewJobPayload {
                job_id,
                payload: encode_job_payload(&job),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.job_payloads, 1);
        assert_eq!(ingested.job_payloads_applied, 0);
        assert_eq!(ingested.invalid_events, 0);
        assert_eq!(pending.pending_job_count(), 1);
        assert!(!context.chain.state().jobs().contains_key(&job_id));

        context
            .chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: graph.graph_id(),
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        let mut processor = ChainNetworkPayloadProcessor::new(&mut context.chain);
        let retried = pending.retry_with(&mut processor);

        assert_eq!(retried.job_payloads_applied, 1);
        assert_eq!(retried.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(context.chain.state().jobs().get(&job_id), Some(&job));
    }

    #[test]
    fn network_event_driver_reports_direct_applied_and_invalid_payload_edges() {
        let testnet = local_matmul_round(b"driver-direct-payloads");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);
        let mut context = TestNetworkEventContext {
            chain: testnet.chain.clone(),
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: encode_receipt_payload(&receipt),
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(&attestation),
                },
                P2pMessage::NewJobPayload {
                    job_id: job.job_id(),
                    payload: vec![0xff],
                },
                P2pMessage::NewReceiptPayload {
                    receipt_id,
                    payload: vec![0xff],
                },
                P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: vec![0xff],
                },
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 5);
        assert_eq!(ingested.job_payloads, 1);
        assert_eq!(ingested.receipt_payloads, 2);
        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads, 2);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 3);
        assert!(pending.is_empty());
    }

    #[test]
    fn network_event_driver_applies_validator_audit_report_payloads_for_non_producers() {
        let (chain, audit_id, auditor) = audit_report_chain();
        let report = ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Valid,
            true,
            hash_bytes(b"test", &[b"ingest-audit-canonical"]),
        );
        let mut context = TestNetworkEventContext {
            chain,
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewValidatorAuditReportPayload {
                audit_id,
                auditor,
                payload: encode_validator_audit_report_payload(&report),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 1);
        assert_eq!(ingested.validator_audit_reports, 1);
        assert_eq!(ingested.validator_audit_reports_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert!(context.chain.state().validator_audit_results()[&audit_id].passed);
    }

    #[test]
    fn network_event_driver_applies_observed_block_check_challenge_with_delayed_reward() {
        let (chain, block, diagnostic, challenger) = rewarded_block_check_challenge_chain();
        let canonical_hash = block.hash();
        let mut context = TestNetworkEventContext {
            chain,
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: diagnostic.challenge_id,
                block_hash: diagnostic.challenge.block_hash,
                challenger,
                observed_block_payload: encode_block_payload(&diagnostic.observed_block),
                challenge_payload: encode_block_check_challenge_payload(&diagnostic.challenge),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.block_check_challenges, 1);
        assert_eq!(ingested.block_check_challenges_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(
            context
                .chain
                .blocks()
                .last()
                .expect("canonical block should remain present")
                .hash(),
            canonical_hash
        );
        assert!(
            context
                .chain
                .state()
                .block_check_challenges()
                .contains_key(&diagnostic.challenge_id)
        );
        assert!(
            context
                .chain
                .state()
                .pending_challenge_rewards()
                .values()
                .any(|reward| {
                    reward.challenge_id == diagnostic.challenge_id
                        && reward.challenger == challenger
                        && reward.amount > 0
                        && !reward.voided_by_challenge
                })
        );
        assert_eq!(context.chain.state().rewards().balance(&challenger), 0);
    }
}
