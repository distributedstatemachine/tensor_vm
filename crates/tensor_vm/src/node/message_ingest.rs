use super::{
    NetworkBlockPayloadApply, NetworkEventContext, NetworkEventIngest, NetworkPayloadApply,
    PendingNetworkPayloads,
    payload_application::{
        apply_network_attestation_payload, apply_network_block_check_challenge_payload,
        apply_network_block_vote_payload, apply_network_external_randomness_beacon_payload,
        apply_network_job_payload, apply_network_observed_block_check_challenge_payload,
        apply_network_receipt_payload, apply_network_validator_audit_report_payload,
        apply_network_validator_vrf_reveal_payload, apply_network_verified_drand_beacon_payload,
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
            P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id,
                beacon_round,
                payload,
            } => {
                ingested.external_randomness_beacons =
                    ingested.external_randomness_beacons.saturating_add(1);
                if source_id.is_empty() || beacon_round == 0 {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_external_randomness_beacon_payload(
                    context.chain(),
                    &source_id,
                    beacon_round,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.external_randomness_beacons_applied = ingested
                            .external_randomness_beacons_applied
                            .saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id,
                beacon_round,
                payload,
            } => {
                ingested.external_randomness_beacons =
                    ingested.external_randomness_beacons.saturating_add(1);
                if source_id.is_empty() || beacon_round == 0 {
                    ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    continue;
                }
                match apply_network_verified_drand_beacon_payload(
                    context.chain(),
                    &source_id,
                    beacon_round,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.external_randomness_beacons_applied = ingested
                            .external_randomness_beacons_applied
                            .saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                    }
                }
            }
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id,
                receipt_id,
                validator,
                payload,
            } => {
                ingested.validator_vrf_reveals = ingested.validator_vrf_reveals.saturating_add(1);
                match apply_network_validator_vrf_reveal_payload(
                    context.chain(),
                    &reveal_id,
                    &receipt_id,
                    &validator,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        ingested.validator_vrf_reveals_applied =
                            ingested.validator_vrf_reveals_applied.saturating_add(1);
                    }
                    NetworkPayloadApply::Pending => {
                        pending_payloads
                            .queue_validator_vrf_reveal(reveal_id, receipt_id, validator, payload);
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
        chain::{
            BlockVote, Chain, ChainCommand, ChainEngine, ChainParams,
            ExternalRandomnessBeaconProof, JobState, ValidatorAuditReport,
            verified_drand_source_id,
        },
        jobs::{MatmulJob, PrimitiveType, TensorOpReceipt},
        p2p::{
            encode_attestation_payload, encode_block_check_challenge_payload,
            encode_block_payload_with_selected_receipts, encode_external_randomness_beacon_payload,
            encode_job_payload, encode_receipt_payload, encode_validator_audit_report_payload,
            encode_validator_vrf_reveal_payload, encode_verified_drand_beacon_payload,
        },
        scheduler::{JobScheduler, SyntheticLocalJobSource},
        testnet::{LocalTestnet, TestnetConfig},
        types::{Hash, address, hash_bytes},
        verify::{
            AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
            verify_tensor_op,
        },
    };

    fn hex_bytes(input: &str) -> Vec<u8> {
        assert_eq!(input.len() % 2, 0);
        input
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let high = (pair[0] as char).to_digit(16).expect("hex high nibble");
                let low = (pair[1] as char).to_digit(16).expect("hex low nibble");
                ((high << 4) | low) as u8
            })
            .collect()
    }

    fn verified_drand_vector() -> (String, u64, Vec<u8>, Vec<u8>, Hash) {
        let public_key = hex_bytes(
            "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11",
        );
        let signature = hex_bytes(
            "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433",
        );
        let expected_randomness: Hash =
            hex_bytes("f3d6adf1daa2c7877f90fb0f1a675ab0a42653a1e2a9b66fee0749d47a47bc57")
                .try_into()
                .unwrap();
        (
            verified_drand_source_id(&public_key),
            223_344,
            public_key,
            signature,
            expected_randomness,
        )
    }

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
        let validators = chain
            .state()
            .validators()
            .iter()
            .map(|(address, validator)| (*address, validator.stake))
            .collect::<Vec<_>>();
        for (validator, stake) in validators {
            if chain.is_block_finalized(&block.hash()) {
                break;
            }
            chain
                .submit_block_vote(BlockVote::new(validator, stake, &block))
                .unwrap();
        }
        assert!(chain.is_block_finalized(&block.hash()));
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
                P2pMessage::NewExternalRandomnessBeaconPayload {
                    source_id: String::new(),
                    beacon_round: 0,
                    payload: Vec::new(),
                },
                P2pMessage::PeerInfo { address: [0; 32] },
                P2pMessage::RequestProgram(hash_bytes(b"test", &[b"program"])),
            ],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 8);
        assert_eq!(ingested.block_announcements, 2);
        assert_eq!(ingested.block_headers, 1);
        assert_eq!(ingested.jobs, 1);
        assert_eq!(ingested.receipts, 1);
        assert_eq!(ingested.attestations, 1);
        assert_eq!(ingested.external_randomness_beacons, 1);
        assert_eq!(ingested.peers, 1);
        assert_eq!(ingested.invalid_events, 8);
    }

    #[test]
    fn network_event_driver_applies_external_randomness_beacon_payloads() {
        let mut context = TestNetworkEventContext::new(b"external-beacon-payload");
        let source_id = "local_drand_fixture_v1";
        let beacon_round = 17;
        let randomness = hash_bytes(b"test", &[b"network-beacon-randomness"]);
        let proof_hash = hash_bytes(b"test", &[b"network-beacon-proof"]);
        let payload = encode_external_randomness_beacon_payload(
            source_id,
            beacon_round,
            &randomness,
            &proof_hash,
        );
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: source_id.to_owned(),
                beacon_round,
                payload: payload.clone(),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 1);
        assert_eq!(ingested.external_randomness_beacons, 1);
        assert_eq!(ingested.external_randomness_beacons_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(context.chain.state().finalized_beacon_round(), beacon_round);
        assert_eq!(context.chain.state().finalized_randomness(), randomness);

        let duplicate = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: source_id.to_owned(),
                beacon_round,
                payload,
            }],
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();
        assert_eq!(duplicate.external_randomness_beacons_applied, 1);
        assert_eq!(duplicate.invalid_events, 0);

        let (verified_source_id, verified_round, _public_key, _signature, _expected_randomness) =
            verified_drand_vector();
        let downgraded_payload = encode_external_randomness_beacon_payload(
            &verified_source_id,
            verified_round,
            &hash_bytes(b"test", &[b"downgraded-randomness"]),
            &hash_bytes(b"test", &[b"downgraded-proof"]),
        );
        let downgraded = ingest_network_messages(
            &mut TestNetworkEventContext::new(b"downgraded-verified-drand"),
            vec![P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: verified_source_id,
                beacon_round: verified_round,
                payload: downgraded_payload,
            }],
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();
        assert_eq!(downgraded.external_randomness_beacons, 1);
        assert_eq!(downgraded.external_randomness_beacons_applied, 0);
        assert_eq!(downgraded.invalid_events, 1);
    }

    #[test]
    fn network_event_driver_applies_verified_drand_beacon_payloads() {
        let mut context = TestNetworkEventContext::new(b"verified-drand-beacon-payload");
        let (source_id, beacon_round, public_key, signature, expected_randomness) =
            verified_drand_vector();
        let payload =
            encode_verified_drand_beacon_payload(&source_id, beacon_round, &public_key, &signature);
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id: source_id.clone(),
                beacon_round,
                payload: payload.clone(),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 1);
        assert_eq!(ingested.external_randomness_beacons, 1);
        assert_eq!(ingested.external_randomness_beacons_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert_eq!(context.chain.state().finalized_beacon_round(), beacon_round);
        assert_eq!(
            context.chain.state().finalized_randomness(),
            expected_randomness
        );
        let record = context
            .chain
            .state()
            .external_randomness_beacons()
            .get(&beacon_round)
            .expect("verified drand beacon should be stored");
        assert_eq!(record.randomness, expected_randomness);
        assert!(matches!(
            record.proof,
            ExternalRandomnessBeaconProof::DrandPedersenBlsUnchainedV1 { .. }
        ));

        let duplicate = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id: source_id.clone(),
                beacon_round,
                payload: payload.clone(),
            }],
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();
        assert_eq!(duplicate.external_randomness_beacons_applied, 1);
        assert_eq!(duplicate.invalid_events, 0);

        let mut invalid_context = TestNetworkEventContext::new(b"verified-drand-invalid");
        let invalid = ingest_network_messages(
            &mut invalid_context,
            vec![P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id,
                beacon_round: beacon_round - 1,
                payload,
            }],
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();
        assert_eq!(invalid.external_randomness_beacons, 1);
        assert_eq!(invalid.external_randomness_beacons_applied, 0);
        assert_eq!(invalid.invalid_events, 1);
        assert_eq!(invalid_context.chain.state().finalized_beacon_round(), 0);
    }

    #[test]
    fn network_event_driver_applies_validator_vrf_reveal_payloads() {
        let testnet = local_matmul_round(b"driver-vrf-reveal");
        let receipt_id = *testnet
            .chain
            .state()
            .receipts()
            .keys()
            .next()
            .expect("local round must produce a receipt");
        let validator = *testnet
            .chain
            .state()
            .validators()
            .keys()
            .next()
            .expect("local round must register validators");
        let reveal = testnet
            .chain
            .validator_vrf_reveal_record(receipt_id, validator, 0)
            .unwrap();
        let payload = encode_validator_vrf_reveal_payload(&reveal);
        let mut context = TestNetworkEventContext {
            chain: testnet.chain,
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: reveal.reveal_id,
                receipt_id,
                validator,
                payload: payload.clone(),
            }],
            false,
            &mut pending,
        )
        .unwrap();

        assert_eq!(ingested.events, 1);
        assert_eq!(ingested.validator_vrf_reveals, 1);
        assert_eq!(ingested.validator_vrf_reveals_applied, 1);
        assert_eq!(ingested.invalid_events, 0);
        assert!(pending.is_empty());
        assert!(
            context
                .chain
                .state()
                .validator_vrf_reveals()
                .contains_key(&reveal.reveal_id)
        );

        let duplicate = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: reveal.reveal_id,
                receipt_id,
                validator,
                payload,
            }],
            false,
            &mut PendingNetworkPayloads::default(),
        )
        .unwrap();
        assert_eq!(duplicate.validator_vrf_reveals_applied, 1);
        assert_eq!(duplicate.invalid_events, 0);
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
    fn network_event_driver_applies_observed_block_check_challenge_without_punishing_canonical_reward()
     {
        let (chain, block, diagnostic, challenger) = rewarded_block_check_challenge_chain();
        let canonical_hash = block.hash();
        let mut context = TestNetworkEventContext {
            chain,
            applied_payloads: Vec::new(),
            applied_blocks: 0,
        };
        let mut pending = PendingNetworkPayloads::default();

        let observed_block_payload = encode_block_payload_with_selected_receipts(
            &diagnostic.observed_block,
            &diagnostic.selected_receipts,
            &diagnostic.parent_state,
        );
        let ingested = ingest_network_messages(
            &mut context,
            vec![P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: diagnostic.challenge_id,
                block_hash: diagnostic.challenge.block_hash,
                challenger,
                observed_block_payload,
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
        assert!(context.chain.state().pending_challenge_rewards().is_empty());
        assert!(
            context
                .chain
                .state()
                .pending_proposer_rewards()
                .values()
                .all(|reward| !reward.voided_by_challenge)
        );
        assert!(context.chain.state().proposer_penalty_until().is_empty());
        assert_eq!(context.chain.state().rewards().balance(&challenger), 0);
    }
}
