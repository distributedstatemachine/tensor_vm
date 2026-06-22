use crate::{
    Chain, ChainCommand, ChainEngine, ChainProfile, DeterministicBlockCheckChallenge, JobState,
    NetworkEventIngest, NodeStore, PendingNetworkPayloads, ReceiptState, RpcHttpServer, RpcNode,
    Tensor, TensorGraph, TensorVmLibp2pService, TraceBisectionConfig,
    api::P2pMessage,
    chain::{ExternalRandomnessBeaconProof, TraceBisectionStatus},
    challenge::{TraceBisectionOpen, TraceBisectionRound},
    decode_job_payload, encode_attestation_payload, encode_block_payload_with_selected_receipts,
    encode_block_vote_payload, encode_external_randomness_beacon_payload, encode_job_payload,
    encode_receipt_payload, encode_validator_audit_report_payload,
    encode_validator_vrf_reveal_payload,
    ir::IrExecution,
    jobs::{GraphJob, GraphReceipt},
    localnet::produce_synthetic_cpu_work_with_profile,
    node::{
        NetworkBlockPayloadApply, NetworkEventContext, apply_network_block_payload,
        attestation_announcement_hash, ingest_network_messages,
    },
    p2p::{
        encode_block_check_challenge_payload, encode_trace_bisection_open_payload,
        encode_trace_bisection_round_payload,
    },
    scheduler::{JobSource, SyntheticLocalJobSource},
    types::{Address, Hash, parse_hash_hex},
};
use std::collections::{BTreeMap, BTreeSet};

use super::validator_fetch::fetch_graph_program_body_if_missing;

const RECENT_BLOCK_PAYLOAD_REBROADCAST_LIMIT: usize = 16;

pub fn ingest_network_events(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    local_producer: bool,
    pending_payloads: &mut PendingNetworkPayloads,
) -> std::result::Result<NetworkEventIngest, String> {
    let messages = p2p_service.drain_observed_messages();
    let mut context = RuntimeNetworkEventContext { server };
    let mut ingested =
        ingest_network_messages(&mut context, messages, local_producer, pending_payloads)?;
    if fetch_pending_graph_job_programs(&mut context, p2p_service, pending_payloads)? {
        let retry =
            ingest_network_messages(&mut context, Vec::new(), local_producer, pending_payloads)?;
        ingested.accumulate(retry);
    }
    Ok(ingested)
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

fn fetch_pending_graph_job_programs(
    context: &mut RuntimeNetworkEventContext<'_>,
    p2p_service: &TensorVmLibp2pService,
    pending_payloads: &PendingNetworkPayloads,
) -> std::result::Result<bool, String> {
    let mut fetched = false;
    for (job_id, payload) in pending_payloads.pending_job_payloads() {
        let Ok(JobState::GraphExecution(job)) = decode_job_payload(&payload) else {
            continue;
        };
        if job.job_id != job_id
            || context
                .server
                .gateway()
                .node
                .chain
                .state()
                .program_body(&job.graph_id)
                .is_some()
        {
            continue;
        }
        let report = fetch_graph_program_body_if_missing(
            &mut context.server.gateway_mut().node,
            p2p_service,
            job.graph_id,
        )?;
        if report.programs_registered > 0 {
            fetched = true;
        }
    }
    Ok(fetched)
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
    let block_hash = block.hash();
    let selected_receipts = server
        .gateway()
        .node
        .chain
        .selected_receipts_for_block(block);
    let parent_state = server
        .gateway()
        .node
        .chain
        .block_parent_state_for_payload(&block_hash)
        .ok_or_else(|| "synthetic block missing parent-state payload".to_owned())?;
    publish_block_announcements(p2p_service, block, &selected_receipts, parent_state)?;
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
    produce_and_publish_synthetic_job_with_store(server, p2p_service, profile, None)
}

pub fn produce_and_publish_synthetic_job_with_store(
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    profile: &ChainProfile,
    store: Option<&NodeStore>,
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
                if let Some(store) = store {
                    persist_runtime_tensor(store, &node.chain, tensor)?;
                }
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

pub fn persist_runtime_tensor(
    store: &NodeStore,
    chain: &Chain,
    tensor: &Tensor,
) -> std::result::Result<Hash, String> {
    let retain_until_block = chain
        .params()
        .tensor_retention_deadline(chain.state().height());
    store
        .persist_tensor(tensor, retain_until_block)
        .map_err(|error| format!("failed to persist tensor artifact: {error}"))
}

pub fn publish_validator_block_proposal(
    p2p_service: &TensorVmLibp2pService,
    block: &crate::chain::TensorBlock,
    selected_receipts: &[Hash],
    parent_state: &crate::chain::ChainState,
) -> std::result::Result<(), String> {
    publish_block_announcements(p2p_service, block, selected_receipts, parent_state)?;
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
            observed_block_payload: encode_block_payload_with_selected_receipts(
                &diagnostic.observed_block,
                &diagnostic.selected_receipts,
                &diagnostic.parent_state,
            ),
            challenge_payload: encode_block_check_challenge_payload(&diagnostic.challenge),
        },
        P2pMessage::NewBlockCheckChallenge(diagnostic.challenge_id),
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTraceBisectionOpen {
    pub challenge_id: Hash,
    pub open: TraceBisectionOpen,
    pub message: P2pMessage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTraceBisectionRound {
    pub challenge_id: Hash,
    pub round: TraceBisectionRound,
    pub message: P2pMessage,
}

pub fn submit_runtime_trace_bisection_open(
    node: &mut RpcNode,
    challenger: Address,
) -> std::result::Result<Option<RuntimeTraceBisectionOpen>, String> {
    let Some(open) = trace_bisection_open_candidate(node, challenger)? else {
        return Ok(None);
    };
    let challenge_id = open.challenge_id();
    node.chain
        .apply_command(ChainCommand::OpenSignedTraceBisection(open.clone()))
        .map_err(|error| format!("failed to open runtime trace-bisection challenge: {error}"))?;
    Ok(Some(RuntimeTraceBisectionOpen {
        challenge_id,
        message: trace_bisection_open_message(&open),
        open,
    }))
}

pub fn publish_runtime_trace_bisection_open(
    p2p_service: &TensorVmLibp2pService,
    open: &RuntimeTraceBisectionOpen,
) -> std::result::Result<(), String> {
    p2p_service
        .publish_gossip(open.message.clone())
        .map_err(|error| format!("failed to publish trace-bisection open payload: {error}"))
}

pub fn submit_runtime_trace_bisection_round(
    node: &mut RpcNode,
    responder: Address,
) -> std::result::Result<Option<RuntimeTraceBisectionRound>, String> {
    let Some((challenge_id, round)) = trace_bisection_round_candidate(node, responder)? else {
        return Ok(None);
    };
    node.chain
        .apply_command(ChainCommand::SubmitTraceBisectionRound(round.clone()))
        .map_err(|error| format!("failed to submit runtime trace-bisection round: {error}"))?;
    Ok(Some(RuntimeTraceBisectionRound {
        challenge_id,
        message: trace_bisection_round_message(&round),
        round,
    }))
}

pub fn publish_runtime_trace_bisection_round(
    p2p_service: &TensorVmLibp2pService,
    round: &RuntimeTraceBisectionRound,
) -> std::result::Result<(), String> {
    p2p_service
        .publish_gossip(round.message.clone())
        .map_err(|error| format!("failed to publish trace-bisection round payload: {error}"))
}

fn trace_bisection_open_candidate(
    node: &RpcNode,
    challenger: Address,
) -> std::result::Result<Option<TraceBisectionOpen>, String> {
    for (receipt_id, receipt) in node.chain.state().receipts() {
        let ReceiptState::GraphExecution(receipt) = receipt else {
            continue;
        };
        if receipt.miner == challenger {
            continue;
        }
        let Some(JobState::GraphExecution(job)) = node.chain.state().jobs().get(&receipt.job_id)
        else {
            continue;
        };
        let Some(graph) = local_graph_receipt_evidence_graph(node, job.graph_id)? else {
            continue;
        };
        if graph.ops.is_empty() || !local_graph_receipt_disagrees(node, job, receipt, &graph)? {
            continue;
        }
        let config = TraceBisectionConfig {
            receipt_id: *receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: graph.ops.len() as u64,
            response_deadline_height: node
                .chain
                .state()
                .height()
                .saturating_add(node.chain.params().challenge_window_blocks().max(1)),
            challenger_bond: node
                .chain
                .params()
                .data_unavailability_miner_slash_amount
                .max(1),
            responder_bond: node.chain.params().invalid_output_miner_slash_amount.max(1),
        };
        let open = TraceBisectionOpen::new(config);
        if node
            .chain
            .state()
            .trace_bisection_challenges()
            .contains_key(&open.challenge_id())
        {
            continue;
        }
        return Ok(Some(open));
    }
    Ok(None)
}

fn trace_bisection_round_candidate(
    node: &RpcNode,
    responder: Address,
) -> std::result::Result<Option<(Hash, TraceBisectionRound)>, String> {
    for (challenge_id, record) in node.chain.state().trace_bisection_challenges() {
        if record.status != TraceBisectionStatus::Active
            || record.state.responder != responder
            || record.state.is_isolated()
            || record.pending_expectation_leaf.is_none()
        {
            continue;
        }
        let Some(ReceiptState::GraphExecution(receipt)) =
            node.chain.state().receipts().get(&record.state.receipt_id)
        else {
            continue;
        };
        if receipt.trace_root != record.state.trace_root || receipt.miner != responder {
            continue;
        }
        let Some(JobState::GraphExecution(job)) = node.chain.state().jobs().get(&receipt.job_id)
        else {
            continue;
        };
        let Some(graph) = local_graph_receipt_evidence_graph(node, job.graph_id)? else {
            continue;
        };
        if graph.ops.len() as u64 <= record.state.midpoint() {
            continue;
        }
        let Some(execution) = local_graph_receipt_execution(node, job, &graph)? else {
            continue;
        };
        if execution.trace_root != record.state.trace_root {
            continue;
        }
        let opening = execution
            .trace_opening(record.state.midpoint())
            .map_err(|error| format!("failed to open runtime trace-bisection midpoint: {error}"))?;
        let round = TraceBisectionRound::new(
            &record.state,
            record.pending_expected_output_roots.clone(),
            opening,
        )
        .map_err(|error| format!("failed to build runtime trace-bisection round: {error}"))?;
        return Ok(Some((*challenge_id, round)));
    }
    Ok(None)
}

fn local_graph_receipt_evidence_graph(
    node: &RpcNode,
    graph_id: Hash,
) -> std::result::Result<Option<TensorGraph>, String> {
    let Some(bytes) = node.chain.state().program_body(&graph_id) else {
        return Ok(None);
    };
    let graph = TensorGraph::from_canonical_json_bytes(bytes)
        .map_err(|error| format!("failed to decode trace-bisection graph body: {error}"))?;
    let validated = graph
        .validate_for_consensus()
        .map_err(|error| format!("failed to validate trace-bisection graph body: {error}"))?;
    if validated != graph_id {
        return Ok(None);
    }
    Ok(Some(graph))
}

fn local_graph_receipt_disagrees(
    node: &RpcNode,
    job: &GraphJob,
    receipt: &GraphReceipt,
    graph: &TensorGraph,
) -> std::result::Result<bool, String> {
    let Some(execution) = local_graph_receipt_execution(node, job, graph)? else {
        return Ok(false);
    };
    for root in receipt.output_roots.values() {
        if !node.contains_tensor_commitment_root(root) {
            return Ok(false);
        }
    }
    if execution.trace_root != receipt.trace_root {
        return Ok(true);
    }
    if execution.outputs.len() != receipt.output_roots.len() {
        return Ok(true);
    }
    for (name, root) in &receipt.output_roots {
        let Some(output) = execution.outputs.get(name) else {
            return Ok(true);
        };
        if output.commitment_root() != *root {
            return Ok(true);
        }
    }
    Ok(false)
}

fn local_graph_receipt_execution(
    node: &RpcNode,
    job: &GraphJob,
    graph: &TensorGraph,
) -> std::result::Result<Option<IrExecution>, String> {
    let mut inputs = BTreeMap::new();
    for (name, root) in &job.input_roots {
        let Some(tensor) = node.tensor_by_commitment_root(root) else {
            return Ok(None);
        };
        inputs.insert(name.clone(), tensor.clone());
    }
    let mut const_blobs = BTreeMap::new();
    for (uri, _) in graph
        .const_blob_specs()
        .map_err(|error| format!("failed to inspect trace-bisection const blobs: {error}"))?
    {
        let root = parse_hash_hex(&uri).map_err(|error| {
            format!("failed to parse trace-bisection const blob root: {error:?}")
        })?;
        let Some(tensor) = node.tensor_by_commitment_root(&root) else {
            return Ok(None);
        };
        const_blobs.insert(uri, tensor.clone());
    }
    let execution = job
        .exact_ir_execution_with_const_blobs(graph, &inputs, &const_blobs)
        .map_err(|error| format!("failed to replay trace-bisection graph receipt: {error}"))?;
    Ok(Some(execution))
}

fn trace_bisection_open_message(open: &TraceBisectionOpen) -> P2pMessage {
    P2pMessage::NewTraceBisectionOpenPayload {
        challenge_id: open.challenge_id(),
        receipt_id: open.config.receipt_id,
        trace_root: open.config.trace_root,
        challenger: open.config.challenger,
        responder: open.config.responder,
        payload: encode_trace_bisection_open_payload(open),
    }
}

fn trace_bisection_round_message(round: &TraceBisectionRound) -> P2pMessage {
    P2pMessage::NewTraceBisectionRoundPayload {
        receipt_id: round.receipt_id,
        trace_root: round.trace_root,
        challenger: round.challenger,
        responder: round.responder,
        transcript_leaf: round.transcript_leaf(),
        payload: encode_trace_bisection_round_payload(round),
    }
}

fn publish_block_announcements(
    p2p_service: &TensorVmLibp2pService,
    block: &crate::chain::TensorBlock,
    selected_receipts: &[Hash],
    parent_state: &crate::chain::ChainState,
) -> std::result::Result<(), String> {
    let block_hash = block.hash();
    p2p_service
        .publish_gossip(P2pMessage::NewBlockPayload {
            height: block.height,
            block_hash,
            payload: encode_block_payload_with_selected_receipts(
                block,
                selected_receipts,
                parent_state,
            ),
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
    validator_vrf_reveals: BTreeSet<Hash>,
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
        validator_vrf_reveals: chain
            .state()
            .validator_vrf_reveals()
            .keys()
            .copied()
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
            if let Some(messages) = receipt_dependency_job_messages(chain, receipt) {
                for message in messages {
                    p2p_service.publish_gossip(message).map_err(|error| {
                        format!("failed to publish receipt dependency job gossip: {error}")
                    })?;
                }
            }
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
    for reveal in chain.state().validator_vrf_reveals().values() {
        if !before.validator_vrf_reveals.contains(&reveal.reveal_id) {
            p2p_service
                .publish_gossip(P2pMessage::NewValidatorVrfRevealPayload {
                    reveal_id: reveal.reveal_id,
                    receipt_id: reveal.receipt_id,
                    validator: reveal.validator,
                    payload: encode_validator_vrf_reveal_payload(reveal),
                })
                .map_err(|error| {
                    format!("failed to publish validator vrf reveal payload gossip: {error}")
                })?;
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
    for (beacon_round, record) in chain.state().external_randomness_beacons() {
        if !matches!(
            record.proof,
            ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1
        ) {
            continue;
        }
        p2p_service
            .publish_gossip(P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: record.source_id.clone(),
                beacon_round: *beacon_round,
                payload: encode_external_randomness_beacon_payload(
                    &record.source_id,
                    record.beacon_round,
                    &record.randomness,
                    &record.proof_hash,
                ),
            })
            .map_err(|error| {
                format!("failed to publish external randomness beacon gossip: {error}")
            })?;
    }
    Ok(())
}

pub fn publish_block_vote_announcements(
    p2p_service: &TensorVmLibp2pService,
    chain: &Chain,
) -> std::result::Result<usize, String> {
    let mut published = 0_usize;
    for (block_hash, votes) in chain.state().block_votes() {
        for vote in votes {
            p2p_service
                .publish_gossip(P2pMessage::NewBlockVotePayload {
                    block_hash: *block_hash,
                    validator: vote.validator,
                    payload: encode_block_vote_payload(vote),
                })
                .map_err(|error| format!("failed to publish block vote payload gossip: {error}"))?;
            published = published.saturating_add(1);
        }
    }
    Ok(published)
}

pub fn publish_block_payload_announcements(
    p2p_service: &TensorVmLibp2pService,
    chain: &Chain,
) -> std::result::Result<usize, String> {
    let mut published = 0_usize;
    for block in chain
        .blocks()
        .iter()
        .rev()
        .take(RECENT_BLOCK_PAYLOAD_REBROADCAST_LIMIT)
        .rev()
    {
        let block_hash = block.hash();
        let Some(parent_state) = chain.block_parent_state_for_payload(&block_hash) else {
            continue;
        };
        let selected_receipts = chain.selected_receipts_for_block(block);
        publish_block_announcements(p2p_service, block, &selected_receipts, parent_state)?;
        published = published.saturating_add(1);
    }
    Ok(published)
}

pub fn publish_chain_payload_announcements(
    p2p_service: &TensorVmLibp2pService,
    chain: &Chain,
) -> std::result::Result<usize, String> {
    let mut published = 0_usize;
    if let Some((job_id, job)) = chain.state().jobs().iter().next() {
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
        published = published.saturating_add(1);
    }
    if let Some((receipt_id, receipt)) = chain.state().receipts().iter().next() {
        if let Some(messages) = receipt_dependency_job_messages(chain, receipt) {
            for message in messages {
                p2p_service.publish_gossip(message).map_err(|error| {
                    format!("failed to publish receipt dependency job gossip: {error}")
                })?;
            }
        }
        p2p_service
            .publish_gossip(P2pMessage::NewReceiptPayload {
                receipt_id: *receipt_id,
                payload: encode_receipt_payload(receipt),
            })
            .map_err(|error| format!("failed to publish receipt payload gossip: {error}"))?;
        p2p_service
            .publish_gossip(P2pMessage::NewReceipt(*receipt_id))
            .map_err(|error| format!("failed to publish receipt gossip: {error}"))?;
        published = published.saturating_add(1);
    }
    if let Some(attestation) = chain
        .state()
        .attestations()
        .values()
        .flat_map(|attestations| attestations.iter())
        .next()
    {
        let attestation_id = attestation_announcement_hash(attestation);
        p2p_service
            .publish_gossip(P2pMessage::NewAttestationPayload {
                attestation_id,
                payload: encode_attestation_payload(attestation),
            })
            .map_err(|error| format!("failed to publish attestation payload gossip: {error}"))?;
        p2p_service
            .publish_gossip(P2pMessage::NewAttestation(attestation_id))
            .map_err(|error| format!("failed to publish attestation gossip: {error}"))?;
        published = published.saturating_add(1);
    }
    Ok(published)
}

fn receipt_dependency_job_messages(
    chain: &Chain,
    receipt: &crate::chain::ReceiptState,
) -> Option<[P2pMessage; 2]> {
    let job_id = receipt.job_id();
    let job = chain.state().jobs().get(&job_id)?;
    Some([
        P2pMessage::NewJobPayload {
            job_id,
            payload: encode_job_payload(job),
        },
        P2pMessage::NewJob(job_id),
    ])
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
        ChainEvent,
        challenge::TraceBisectionExpectation,
        decode_block_payload_with_selected_receipts,
        p2p::{decode_trace_bisection_open_payload, decode_trace_bisection_round_payload},
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::{address, hash_bytes},
    };

    fn submit_trace_bisection_expectation(
        node: &mut RpcNode,
        challenge_id: Hash,
        expected_output_roots: Vec<Hash>,
    ) {
        let state = node
            .chain
            .state()
            .trace_bisection_challenges()
            .get(&challenge_id)
            .unwrap()
            .state
            .clone();
        let expectation = TraceBisectionExpectation::new(&state, expected_output_roots).unwrap();
        node.chain
            .apply_command(ChainCommand::SubmitTraceBisectionExpectation(expectation))
            .unwrap();
    }

    #[test]
    fn receipt_dependency_job_messages_replay_referenced_job_payload() {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"test", &[b"app-network-receipt-job-dependency"]),
        );
        testnet.run_matmul_round(&JobScheduler::with_small_shape((8, 8, 8)));
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round should produce a receipt");
        let job_id = receipt.job_id();

        let messages = receipt_dependency_job_messages(&testnet.chain, receipt)
            .expect("known receipt should replay its job dependency");

        assert!(matches!(
            &messages[0],
            P2pMessage::NewJobPayload {
                job_id: replayed,
                payload
            } if *replayed == job_id
                && decode_job_payload(payload)
                    .expect("replayed job payload should decode")
                    .job_id()
                    == job_id
        ));
        assert_eq!(messages[1], P2pMessage::NewJob(job_id));
    }

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
            decode_block_payload_with_selected_receipts(observed_block_payload)
                .expect("observed block payload should decode")
                .0
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

    fn graph_receipt_node(
        seed: Hash,
        challenger: Address,
        miner: Address,
        graph: &TensorGraph,
        job: GraphJob,
        receipt: GraphReceipt,
    ) -> RpcNode {
        let mut chain = Chain::new(seed);
        chain
            .apply_command(ChainCommand::RegisterValidator {
                address: challenger,
                stake: chain.params().validator_min_stake,
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake: chain.params().miner_min_stake,
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: job.graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(job)))
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
                receipt,
            )))
            .unwrap();
        RpcNode::new(chain)
    }

    #[test]
    fn trace_bisection_challenge_generation_requires_local_graph_evidence_and_disagreement() {
        let seed = hash_bytes(b"test", &[b"app-network-trace-bisection-open"]);
        let challenger = address(b"trace-open-validator");
        let miner = address(b"trace-open-miner");

        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let inputs = SyntheticLocalJobSource::graph_execution_inputs();
        let job_chain = Chain::new(seed);
        let job = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((8, 8, 8)))
            .next_graph_job(&job_chain);
        let (valid_receipt, outputs) =
            GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
        let output_roots = outputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let bad_receipt = GraphReceipt::from_roots(
            &job,
            miner,
            output_roots,
            hash_bytes(b"test", &[b"runtime-trace-bisection-bad-trace-root"]),
            1,
            3,
        );
        let mut valid_node = graph_receipt_node(
            seed,
            challenger,
            miner,
            &graph,
            job.clone(),
            valid_receipt.clone(),
        );
        for tensor in inputs
            .clone()
            .into_values()
            .chain(outputs.clone().into_values())
        {
            valid_node.insert_tensor(tensor);
        }
        assert_eq!(
            submit_runtime_trace_bisection_open(&mut valid_node, challenger).unwrap(),
            None
        );

        let mut node =
            graph_receipt_node(seed, challenger, miner, &graph, job, bad_receipt.clone());
        let receipt_id = bad_receipt.receipt_id;
        assert_eq!(
            submit_runtime_trace_bisection_open(&mut node, challenger).unwrap(),
            None
        );

        for tensor in inputs.into_values().chain(outputs.into_values()) {
            node.insert_tensor(tensor);
        }
        let opened = submit_runtime_trace_bisection_open(&mut node, challenger)
            .unwrap()
            .expect("local graph receipt evidence should open a trace-bisection session");

        assert_eq!(opened.open.config.receipt_id, receipt_id);
        assert_eq!(opened.open.config.trace_root, bad_receipt.trace_root);
        assert_eq!(opened.open.config.challenger, challenger);
        assert_eq!(opened.open.config.responder, miner);
        assert_eq!(opened.open.config.op_count, graph.ops.len() as u64);
        assert!(opened.open.verify_signature());
        assert!(
            node.chain
                .state()
                .trace_bisection_challenges()
                .contains_key(&opened.challenge_id)
        );
        let P2pMessage::NewTraceBisectionOpenPayload {
            challenge_id,
            receipt_id: message_receipt_id,
            trace_root,
            challenger: message_challenger,
            responder,
            payload,
        } = &opened.message
        else {
            panic!("runtime open should carry signed trace-bisection payload");
        };
        assert_eq!(*challenge_id, opened.challenge_id);
        assert_eq!(*message_receipt_id, receipt_id);
        assert_eq!(*trace_root, bad_receipt.trace_root);
        assert_eq!(*message_challenger, challenger);
        assert_eq!(*responder, miner);
        assert_eq!(
            decode_trace_bisection_open_payload(payload).unwrap(),
            opened.open
        );
        assert_eq!(
            submit_runtime_trace_bisection_open(&mut node, challenger).unwrap(),
            None
        );
    }

    #[test]
    fn trace_bisection_round_generation_requires_responder_and_committed_local_trace() {
        let seed = hash_bytes(b"test", &[b"app-network-trace-bisection-round"]);
        let challenger = address(b"trace-round-validator");
        let miner = address(b"trace-round-miner");
        let unrelated = address(b"trace-round-unrelated");

        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let inputs = SyntheticLocalJobSource::graph_execution_inputs();
        let job_chain = Chain::new(seed);
        let job = SyntheticLocalJobSource::new(JobScheduler::with_small_shape((8, 8, 8)))
            .next_graph_job(&job_chain);
        let (receipt, _outputs) =
            GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
        let execution = job.exact_ir_execution(&graph, &inputs).unwrap();
        let expected_output_roots = execution.trace_opening(0).unwrap().op_trace.output_roots;
        let receipt_id = receipt.receipt_id;

        let mut missing_node = graph_receipt_node(
            seed,
            challenger,
            miner,
            &graph,
            job.clone(),
            receipt.clone(),
        );
        let missing_open_events = missing_node
            .chain
            .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
                receipt_id,
                trace_root: receipt.trace_root,
                challenger,
                responder: miner,
                op_count: graph.ops.len() as u64,
                response_deadline_height: 10,
                challenger_bond: 1,
                responder_bond: 1,
            }))
            .unwrap();
        let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] =
            missing_open_events.as_slice()
        else {
            panic!("expected trace bisection open event");
        };
        submit_trace_bisection_expectation(
            &mut missing_node,
            *challenge_id,
            expected_output_roots.clone(),
        );
        assert_eq!(
            submit_runtime_trace_bisection_round(&mut missing_node, miner).unwrap(),
            None
        );

        let mut wrong_wallet_node = graph_receipt_node(
            seed,
            challenger,
            miner,
            &graph,
            job.clone(),
            receipt.clone(),
        );
        let wrong_wallet_open_events = wrong_wallet_node
            .chain
            .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
                receipt_id,
                trace_root: receipt.trace_root,
                challenger,
                responder: miner,
                op_count: graph.ops.len() as u64,
                response_deadline_height: 10,
                challenger_bond: 1,
                responder_bond: 1,
            }))
            .unwrap();
        let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] =
            wrong_wallet_open_events.as_slice()
        else {
            panic!("expected trace bisection open event");
        };
        submit_trace_bisection_expectation(
            &mut wrong_wallet_node,
            *challenge_id,
            expected_output_roots.clone(),
        );
        for tensor in inputs.clone().into_values() {
            wrong_wallet_node.insert_tensor(tensor);
        }
        assert_eq!(
            submit_runtime_trace_bisection_round(&mut wrong_wallet_node, unrelated).unwrap(),
            None
        );

        let mut node = graph_receipt_node(seed, challenger, miner, &graph, job, receipt.clone());
        let open_events = node
            .chain
            .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
                receipt_id,
                trace_root: receipt.trace_root,
                challenger,
                responder: miner,
                op_count: graph.ops.len() as u64,
                response_deadline_height: 10,
                challenger_bond: 1,
                responder_bond: 1,
            }))
            .unwrap();
        let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
            panic!("expected trace bisection open event");
        };
        submit_trace_bisection_expectation(&mut node, *challenge_id, expected_output_roots);
        for tensor in inputs.into_values() {
            node.insert_tensor(tensor);
        }

        let generated = submit_runtime_trace_bisection_round(&mut node, miner)
            .unwrap()
            .expect("responder with local committed trace should submit a round");

        assert_eq!(generated.round.receipt_id, receipt_id);
        assert_eq!(generated.round.trace_root, receipt.trace_root);
        assert_eq!(generated.round.responder, miner);
        assert_eq!(generated.round.midpoint_op, 0);
        let record = node
            .chain
            .state()
            .trace_bisection_challenges()
            .get(&generated.challenge_id)
            .expect("round should update challenge record");
        assert_eq!(record.opened_rounds, 1);
        assert_eq!(
            record.status,
            TraceBisectionStatus::Isolated { op_index: 1 }
        );
        assert_eq!(record.last_matched_midpoint, Some(true));
        let P2pMessage::NewTraceBisectionRoundPayload {
            receipt_id: message_receipt_id,
            trace_root,
            challenger: message_challenger,
            responder,
            transcript_leaf,
            payload,
        } = &generated.message
        else {
            panic!("runtime round should carry signed trace-bisection round payload");
        };
        assert_eq!(*message_receipt_id, receipt_id);
        assert_eq!(*trace_root, receipt.trace_root);
        assert_eq!(*message_challenger, challenger);
        assert_eq!(*responder, miner);
        assert_eq!(*transcript_leaf, generated.round.transcript_leaf());
        assert_eq!(
            decode_trace_bisection_round_payload(payload).unwrap(),
            generated.round
        );
        assert_eq!(
            submit_runtime_trace_bisection_round(&mut node, miner).unwrap(),
            None
        );
    }
}
