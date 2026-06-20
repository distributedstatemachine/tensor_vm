use crate::{
    Chain, ChainCommand, ChainEngine, ChainProfile, DeterministicBlockCheckChallenge, JobState,
    NetworkEventIngest, PendingNetworkPayloads, RpcHttpServer, TensorVmLibp2pService,
    api::P2pMessage,
    encode_attestation_payload, encode_block_payload, encode_block_vote_payload,
    encode_job_payload, encode_receipt_payload, encode_validator_audit_report_payload,
    localnet::produce_synthetic_cpu_work_with_profile,
    node::{
        NetworkBlockPayloadApply, NetworkEventContext, apply_network_block_payload,
        attestation_announcement_hash, ingest_network_messages,
    },
    p2p::encode_block_check_challenge_payload,
    scheduler::{JobSource, SyntheticLocalJobSource},
    types::{Address, Hash},
};
use std::collections::BTreeSet;

pub fn ingest_network_events(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let messages = p2p_service.drain_observed_messages();
    let mut context = RuntimeNetworkEventContext { server };
    ingest_network_messages(&mut context, messages, local_producer, pending_payloads)
}

struct RuntimeNetworkEventContext<'a> {
    server: &'a mut RpcHttpServer,
}

impl NetworkEventContext for RuntimeNetworkEventContext<'_> {
    fn chain(&mut self) -> &mut Chain {
        &mut self.server.gateway_mut().node.chain
    }

    fn apply_block_payload(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        apply_network_block_payload(
            &mut self.server.gateway_mut().node.chain,
            height,
            block_hash,
            payload,
        )
    }
}

pub fn produce_and_publish_synthetic_round(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
) -> std::result::Result<Option<Hash>, String> {
    let Some(_) = produce_and_publish_synthetic_work(server, p2p_service, profile)? else {
        return Ok(None);
    };
    let chain = &server.gateway().node.chain;
    let beacon = chain.state().finalized_randomness();
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap_or_default();
    let timestamp = chain
        .blocks()
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params().block_time_seconds)
        })
        .unwrap_or(0);
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer,
            timestamp,
        })
        .map_err(|error| format!("synthetic CPU round block production failed: {error}"))?;
    let Some(block) = server.gateway().node.chain.blocks().last() else {
        return Ok(None);
    };
    publish_block_announcements(p2p_service, block)?;
    Ok(Some(block.hash()))
}

pub fn produce_and_publish_synthetic_work(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
) -> std::result::Result<Option<()>, String> {
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    let Some(work) =
        produce_synthetic_cpu_work_with_profile(&mut server.gateway_mut().node.chain, profile)
            .map_err(|error| format!("synthetic CPU work failed: {error}"))?
    else {
        return Ok(None);
    };
    for tensor in work.tensors {
        p2p_service.register_tensor(tensor.clone());
        server.gateway_mut().node.insert_tensor(tensor);
    }
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    Ok(Some(()))
}

pub fn produce_and_publish_synthetic_job(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
) -> std::result::Result<Option<Hash>, String> {
    let Some(mut job_source) = profile.synthetic_job_source() else {
        return Ok(None);
    };
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    let Some(job) = job_source.next_job(&server.gateway().node.chain) else {
        return Ok(None);
    };
    let job_id = job.job_id();
    if let JobState::LinearTrainingStep(job) = &job {
        let chain = &mut server.gateway_mut().node.chain;
        if !chain.state().model_states().contains_key(&job.model_id) {
            chain
                .apply_command(ChainCommand::RegisterModel {
                    model_id: job.model_id,
                    architecture_hash: SyntheticLocalJobSource::linear_training_architecture_hash(),
                    weight_root: job.weight_root_before,
                    config_hash: SyntheticLocalJobSource::linear_training_config_hash(),
                })
                .map_err(|error| format!("synthetic linear model registration failed: {error}"))?;
        }
    }
    if let JobState::GraphExecution(job) = &job {
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        if graph.graph_id() != job.graph_id {
            return Err("synthetic graph job does not match configured graph body".to_owned());
        }
        let inputs = SyntheticLocalJobSource::graph_execution_inputs();
        {
            let node = &mut server.gateway_mut().node;
            node.chain
                .apply_command(ChainCommand::RegisterProgramBody {
                    graph_id: job.graph_id,
                    bytes: graph.canonical_json().into_bytes(),
                })
                .map_err(|error| format!("synthetic graph program registration failed: {error}"))?;
            for tensor in inputs.values() {
                node.insert_tensor(tensor.clone());
                p2p_service.register_tensor(tensor.clone());
            }
        }
    }
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitJob(job))
        .map_err(|error| format!("synthetic job submission failed: {error}"))?;
    publish_new_chain_announcements(
        p2p_service,
        &announcement_checkpoint,
        &server.gateway().node.chain,
    )?;
    Ok(Some(job_id))
}

pub fn publish_validator_block_proposal(
    p2p_service: &TensorVmLibp2pService,
    block: &crate::chain::TensorBlock,
) -> std::result::Result<(), String> {
    publish_block_announcements(p2p_service, block)?;
    Ok(())
}

pub fn publish_observed_block_check_challenge(
    p2p_service: &TensorVmLibp2pService,
    diagnostic: &DeterministicBlockCheckChallenge,
) -> std::result::Result<(), String> {
    for message in observed_block_check_challenge_messages(diagnostic) {
        p2p_service.publish_gossip(message).map_err(|error| {
            format!("failed to publish observed block-check challenge: {error}")
        })?;
    }
    Ok(())
}

pub fn observed_block_check_challenge_messages(
    diagnostic: &DeterministicBlockCheckChallenge,
) -> [P2pMessage; 2] {
    [
        P2pMessage::NewObservedBlockCheckChallengePayload {
            challenge_id: diagnostic.challenge_id,
            block_hash: diagnostic.challenge.block_hash,
            challenger: diagnostic.challenge.challenger,
            observed_block_payload: encode_block_payload(&diagnostic.observed_block),
            challenge_payload: encode_block_check_challenge_payload(&diagnostic.challenge),
        },
        P2pMessage::NewBlockCheckChallenge(diagnostic.challenge_id),
    ]
}

fn publish_block_announcements(
    p2p_service: &TensorVmLibp2pService,
    block: &crate::chain::TensorBlock,
) -> std::result::Result<(), String> {
    let block_hash = block.hash();
    p2p_service
        .publish_gossip(P2pMessage::NewBlockPayload {
            height: block.height,
            block_hash,
            payload: encode_block_payload(block),
        })
        .map_err(|error| format!("failed to publish block payload gossip: {error}"))?;
    p2p_service
        .publish_gossip(P2pMessage::NewBlockHeader {
            height: block.height,
            block_hash,
        })
        .map_err(|error| format!("failed to publish block header gossip: {error}"))?;
    p2p_service
        .publish_gossip(P2pMessage::NewBlock(block_hash))
        .map_err(|error| format!("failed to publish block hash gossip: {error}"))
}

pub struct ChainAnnouncementCheckpoint {
    jobs: BTreeSet<Hash>,
    receipts: BTreeSet<Hash>,
    attestations: BTreeSet<Hash>,
    validator_audit_reports: BTreeSet<(Hash, Address)>,
    block_votes: BTreeSet<(Hash, Address)>,
}

pub fn chain_announcement_checkpoint(chain: &Chain) -> ChainAnnouncementCheckpoint {
    ChainAnnouncementCheckpoint {
        jobs: chain.state().jobs().keys().copied().collect(),
        receipts: chain.state().receipts().keys().copied().collect(),
        attestations: attestation_announcement_hashes(chain).collect(),
        validator_audit_reports: chain
            .state()
            .validator_audit_results()
            .values()
            .map(|result| (result.audit_id, result.auditor))
            .collect(),
        block_votes: block_vote_announcement_keys(chain).collect(),
    }
}

pub fn publish_new_chain_announcements(
    p2p_service: &TensorVmLibp2pService,
    before: &ChainAnnouncementCheckpoint,
    chain: &Chain,
) -> std::result::Result<(), String> {
    for (job_id, job) in chain.state().jobs() {
        if !before.jobs.contains(job_id) {
            let program_hash = job.program_hash();
            if let Some(program_body) = chain.state().program_body(&program_hash) {
                p2p_service.register_program(program_hash, program_body.to_vec());
            }
            p2p_service
                .publish_gossip(P2pMessage::NewJobPayload {
                    job_id: *job_id,
                    payload: encode_job_payload(job),
                })
                .map_err(|error| format!("failed to publish job payload gossip: {error}"))?;
            p2p_service
                .publish_gossip(P2pMessage::NewJob(*job_id))
                .map_err(|error| format!("failed to publish job gossip: {error}"))?;
        }
    }
    for (receipt_id, receipt) in chain.state().receipts() {
        if !before.receipts.contains(receipt_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewReceiptPayload {
                    receipt_id: *receipt_id,
                    payload: encode_receipt_payload(receipt),
                })
                .map_err(|error| format!("failed to publish receipt payload gossip: {error}"))?;
            p2p_service
                .publish_gossip(P2pMessage::NewReceipt(*receipt_id))
                .map_err(|error| format!("failed to publish receipt gossip: {error}"))?;
        }
    }
    for attestation in chain
        .state()
        .attestations()
        .values()
        .flat_map(|attestations| attestations.iter())
    {
        let attestation_id = attestation_announcement_hash(attestation);
        if !before.attestations.contains(&attestation_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewAttestationPayload {
                    attestation_id,
                    payload: encode_attestation_payload(attestation),
                })
                .map_err(|error| {
                    format!("failed to publish attestation payload gossip: {error}")
                })?;
            p2p_service
                .publish_gossip(P2pMessage::NewAttestation(attestation_id))
                .map_err(|error| format!("failed to publish attestation gossip: {error}"))?;
        }
    }
    for result in chain.state().validator_audit_results().values() {
        let key = (result.audit_id, result.auditor);
        if !before.validator_audit_reports.contains(&key) {
            p2p_service
                .publish_gossip(P2pMessage::NewValidatorAuditReportPayload {
                    audit_id: result.audit_id,
                    auditor: result.auditor,
                    payload: encode_validator_audit_report_payload(
                        &crate::chain::ValidatorAuditReport {
                            audit_id: result.audit_id,
                            auditor: result.auditor,
                            canonical_result: result.canonical_result,
                            canonical_data_availability_passed: result
                                .canonical_data_availability_passed,
                            checks_root: result.checks_root,
                            signature: result.signature,
                        },
                    ),
                })
                .map_err(|error| {
                    format!("failed to publish validator audit report payload gossip: {error}")
                })?;
            p2p_service
                .publish_gossip(P2pMessage::NewValidatorAuditReport(result.audit_id))
                .map_err(|error| {
                    format!("failed to publish validator audit report gossip: {error}")
                })?;
        }
    }
    for (block_hash, votes) in chain.state().block_votes() {
        for vote in votes {
            let key = (*block_hash, vote.validator);
            if !before.block_votes.contains(&key) {
                p2p_service
                    .publish_gossip(P2pMessage::NewBlockVotePayload {
                        block_hash: *block_hash,
                        validator: vote.validator,
                        payload: encode_block_vote_payload(vote),
                    })
                    .map_err(|error| {
                        format!("failed to publish block vote payload gossip: {error}")
                    })?;
            }
        }
    }
    Ok(())
}

fn attestation_announcement_hashes(chain: &Chain) -> impl Iterator<Item = Hash> + '_ {
    chain
        .state()
        .attestations()
        .values()
        .flat_map(|attestations| attestations.iter().map(attestation_announcement_hash))
}

fn block_vote_announcement_keys(chain: &Chain) -> impl Iterator<Item = (Hash, Address)> + '_ {
    chain
        .state()
        .block_votes()
        .iter()
        .flat_map(|(block_hash, votes)| votes.iter().map(move |vote| (*block_hash, vote.validator)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decode_block_payload,
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::hash_bytes,
    };

    #[test]
    fn observed_block_check_challenge_messages_carry_delayed_reward_evidence_payload() {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"test", &[b"app-network-observed-challenge"]),
        );
        testnet.run_matmul_round(&JobScheduler::with_small_shape((8, 8, 8)));
        let block = testnet
            .chain
            .blocks()
            .last()
            .expect("local round should produce a useful block")
            .clone();
        let challenger = *testnet
            .chain
            .state()
            .validators()
            .keys()
            .find(|validator| **validator != block.proposer)
            .expect("testnet should include a challenger validator");
        let diagnostic = testnet
            .chain
            .deterministic_bad_block_check_challenge(&block, challenger)
            .expect("useful block should derive diagnostic challenge");

        let messages = observed_block_check_challenge_messages(&diagnostic);

        let P2pMessage::NewObservedBlockCheckChallengePayload {
            challenge_id,
            block_hash,
            challenger: message_challenger,
            observed_block_payload,
            challenge_payload,
        } = &messages[0]
        else {
            panic!("first message should carry observed challenge payload");
        };
        assert_eq!(*challenge_id, diagnostic.challenge_id);
        assert_eq!(*block_hash, diagnostic.challenge.block_hash);
        assert_eq!(*message_challenger, challenger);
        assert_eq!(
            decode_block_payload(observed_block_payload)
                .expect("observed block payload should decode")
                .hash(),
            diagnostic.challenge.block_hash
        );
        assert_eq!(
            crate::p2p::decode_block_check_challenge_payload(challenge_payload)
                .expect("challenge payload should decode"),
            diagnostic.challenge
        );
        assert_eq!(
            messages[1],
            P2pMessage::NewBlockCheckChallenge(diagnostic.challenge_id)
        );
    }
}
