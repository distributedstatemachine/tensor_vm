use crate::{
    ChainCommand, ChainEngine, JobState, NodeStore, ReceiptState, RpcNode, Tensor, TensorGraph,
    TensorVmLibp2pService,
    api::P2pMessage,
    decode_tensor_payload,
    hash::hex,
    types::{Hash, parse_hash_hex},
};
use std::time::Duration;

use super::persist_runtime_tensor;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRemoteTensorFetchReport {
    pub program_attempts: usize,
    pub program_successes: usize,
    pub program_failures: usize,
    pub program_bytes: usize,
    pub programs_registered: usize,
    pub attempts: usize,
    pub successes: usize,
    pub failures: usize,
    pub bytes: usize,
    pub tensors_inserted: usize,
}

impl ValidatorRemoteTensorFetchReport {
    pub fn has_activity(&self) -> bool {
        self.program_attempts > 0
            || self.program_successes > 0
            || self.program_failures > 0
            || self.programs_registered > 0
            || self.attempts > 0
            || self.successes > 0
            || self.failures > 0
            || self.tensors_inserted > 0
    }
}

pub fn fetch_graph_program_body_if_missing(
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    graph_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let mut report = ValidatorRemoteTensorFetchReport::default();
    fetch_graph_program_if_missing(node, p2p_service, graph_id, &mut report)?;
    Ok(report)
}

pub fn fetch_miner_role_missing_graph_artifacts(
    store: &NodeStore,
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    job_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let Some(JobState::GraphExecution(job)) = node.chain.state().jobs().get(&job_id).cloned()
    else {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    };
    let mut report = ValidatorRemoteTensorFetchReport::default();
    fetch_graph_program_if_missing(node, p2p_service, job.graph_id, &mut report)?;
    let mut roots = Vec::new();
    roots.extend(job.input_roots.values().copied());
    if let Some(graph) = graph_from_program_body(node, &job.graph_id)
        && let Ok(const_blobs) = graph.const_blob_specs()
    {
        roots.extend(
            const_blobs
                .keys()
                .filter_map(|uri| parse_hash_hex(uri).ok()),
        );
    }
    fetch_missing_tensor_roots(store, node, p2p_service, roots, &mut report)?;
    Ok(report)
}

pub fn fetch_validator_role_missing_tensors(
    store: &NodeStore,
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    receipt_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    };
    let mut report = ValidatorRemoteTensorFetchReport::default();
    if let ReceiptState::GraphExecution(receipt) = &receipt {
        fetch_graph_program_if_missing(node, p2p_service, receipt.graph_id, &mut report)?;
    }
    let missing_roots = validator_receipt_required_remote_roots(node, &receipt);
    if missing_roots.is_empty() {
        return Ok(report);
    }
    fetch_missing_tensor_roots(store, node, p2p_service, missing_roots, &mut report)?;
    Ok(report)
}

fn fetch_graph_program_if_missing(
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    graph_id: Hash,
    report: &mut ValidatorRemoteTensorFetchReport,
) -> std::result::Result<(), String> {
    if node.chain.state().program_body(&graph_id).is_some() {
        return Ok(());
    }
    let peers = p2p_service.connected_peer_ids();
    if peers.is_empty() {
        report.program_failures = report.program_failures.saturating_add(1);
        return Ok(());
    }
    let mut failed_response_recorded = false;
    for peer in &peers {
        report.program_attempts = report.program_attempts.saturating_add(1);
        let response = p2p_service.request_response(
            *peer,
            P2pMessage::RequestProgram(graph_id),
            Duration::from_secs(2),
        );
        let Ok(response) = response else {
            continue;
        };
        match validator_remote_program_response(graph_id, response) {
            ValidatorRemoteProgramResponse::Found { bytes } => {
                let byte_len = bytes.len();
                node.chain
                    .apply_command(ChainCommand::RegisterProgramBody {
                        graph_id,
                        bytes: bytes.clone(),
                    })
                    .map_err(|error| {
                        format!(
                            "role failed to register fetched graph program {}: {error}",
                            hex(&graph_id)
                        )
                    })?;
                p2p_service.register_program(graph_id, bytes);
                report.program_bytes = report.program_bytes.saturating_add(byte_len);
                report.program_successes = report.program_successes.saturating_add(1);
                report.programs_registered = report.programs_registered.saturating_add(1);
                return Ok(());
            }
            ValidatorRemoteProgramResponse::Missing => {}
            ValidatorRemoteProgramResponse::Invalid => {
                if !failed_response_recorded {
                    report.program_failures = report.program_failures.saturating_add(1);
                    failed_response_recorded = true;
                }
            }
        }
    }
    if !failed_response_recorded {
        report.program_failures = report.program_failures.saturating_add(1);
    }
    Ok(())
}

fn fetch_missing_tensor_roots(
    store: &NodeStore,
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    roots: Vec<Hash>,
    report: &mut ValidatorRemoteTensorFetchReport,
) -> std::result::Result<(), String> {
    let mut missing_roots = roots;
    missing_roots.sort();
    missing_roots.dedup();
    missing_roots.retain(|root| !node.contains_tensor_commitment_root(root));
    if missing_roots.is_empty() {
        return Ok(());
    }
    let peers = p2p_service.connected_peer_ids();
    if peers.is_empty() {
        report.failures = missing_roots.len();
        return Ok(());
    }
    for root in missing_roots {
        let mut fetched = false;
        let mut failed_response_recorded = false;
        for peer in &peers {
            report.attempts = report.attempts.saturating_add(1);
            let response = p2p_service.request_response(
                *peer,
                P2pMessage::RequestTensorByCommitmentRoot {
                    commitment_root: root,
                },
                Duration::from_secs(2),
            );
            let Ok(response) = response else {
                continue;
            };
            match validator_remote_tensor_response(root, response) {
                ValidatorRemoteTensorResponse::Found { tensor, bytes } => {
                    persist_runtime_tensor(store, &node.chain, &tensor)?;
                    node.insert_tensor(tensor.clone());
                    p2p_service.register_tensor(tensor);
                    report.bytes = report.bytes.saturating_add(bytes);
                    report.successes = report.successes.saturating_add(1);
                    report.tensors_inserted = report.tensors_inserted.saturating_add(1);
                    fetched = true;
                    break;
                }
                ValidatorRemoteTensorResponse::Missing => {}
                ValidatorRemoteTensorResponse::Invalid => {
                    record_validator_remote_fetch_failure(report, &mut failed_response_recorded);
                }
            }
        }
        if !fetched && !failed_response_recorded {
            report.failures = report.failures.saturating_add(1);
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidatorRemoteTensorResponse {
    Found { tensor: Tensor, bytes: usize },
    Missing,
    Invalid,
}

pub fn validator_remote_tensor_response(
    requested_root: Hash,
    response: P2pMessage,
) -> ValidatorRemoteTensorResponse {
    let P2pMessage::TensorByCommitmentRootResponse {
        commitment_root,
        payload,
    } = response
    else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    if commitment_root != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    let Some(payload) = payload else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    let bytes = payload.len();
    let Ok(tensor) = decode_tensor_payload(&payload) else {
        return ValidatorRemoteTensorResponse::Invalid;
    };
    if tensor.commitment_root() != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    ValidatorRemoteTensorResponse::Found { tensor, bytes }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidatorRemoteProgramResponse {
    Found { bytes: Vec<u8> },
    Missing,
    Invalid,
}

pub fn validator_remote_program_response(
    requested_graph_id: Hash,
    response: P2pMessage,
) -> ValidatorRemoteProgramResponse {
    let P2pMessage::ProgramResponse {
        program_hash,
        bytes,
    } = response
    else {
        return ValidatorRemoteProgramResponse::Missing;
    };
    if program_hash != requested_graph_id {
        return ValidatorRemoteProgramResponse::Invalid;
    }
    if bytes.is_empty() {
        return ValidatorRemoteProgramResponse::Missing;
    }
    let Ok(graph) = TensorGraph::from_canonical_json_bytes(&bytes) else {
        return ValidatorRemoteProgramResponse::Invalid;
    };
    // Admit consensus (Tier-A/B) and §8.1 committee (Tier-C reference) graph
    // bodies; otherwise the committee graph body can never propagate to peers via
    // fetch and only the producer ever holds it, starving committee verification.
    let Ok(graph_id) = graph.validate_for_committee() else {
        return ValidatorRemoteProgramResponse::Invalid;
    };
    if graph_id != requested_graph_id || graph.canonical_json().as_bytes() != bytes.as_slice() {
        return ValidatorRemoteProgramResponse::Invalid;
    }
    ValidatorRemoteProgramResponse::Found { bytes }
}

fn record_validator_remote_fetch_failure(
    report: &mut ValidatorRemoteTensorFetchReport,
    recorded_for_root: &mut bool,
) {
    if !*recorded_for_root {
        report.failures = report.failures.saturating_add(1);
        *recorded_for_root = true;
    }
}

fn validator_receipt_required_remote_roots(node: &RpcNode, receipt: &ReceiptState) -> Vec<Hash> {
    let mut roots = Vec::new();
    match receipt {
        ReceiptState::TensorOp(receipt) => {
            roots.extend(receipt.input_roots.iter().copied());
            roots.extend(receipt.output_roots.iter().copied());
        }
        ReceiptState::LinearTrainingStep(receipt) => {
            roots.push(receipt.y_root);
            roots.push(receipt.grad_w_root);
            roots.push(receipt.weight_root_after);
        }
        ReceiptState::GraphExecution(receipt) => {
            // Verification re-executes the graph from its inputs (+ const blobs)
            // and compares the recomputed roots against the receipt's claimed
            // output roots, which are already in the receipt body — the output
            // *tensors* are never needed to verify. Committee (Tier-C) receipts'
            // outputs are only served by the few miners that computed them, while
            // the input is reliably served by the producer, so fetching outputs
            // for committee receipts blocks settlement for no verification gain.
            // Tier-A/B graph receipts keep fetching outputs (unchanged baseline).
            roots.extend(receipt.input_roots.values().copied());
            let graph = graph_from_program_body(node, &receipt.graph_id);
            let committee = graph
                .as_ref()
                .map(|graph| graph.requires_committee_verification())
                .unwrap_or(false);
            if !committee {
                roots.extend(receipt.output_roots.values().copied());
            }
            if let Some(graph) = graph.as_ref()
                && let Ok(const_blobs) = graph.const_blob_specs()
            {
                roots.extend(
                    const_blobs
                        .keys()
                        .filter_map(|uri| parse_hash_hex(uri).ok()),
                );
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .filter(|root| !node.contains_tensor_commitment_root(root))
        .collect()
}

fn graph_from_program_body(node: &RpcNode, graph_id: &Hash) -> Option<TensorGraph> {
    let bytes = node.chain.state().program_body(graph_id)?;
    let graph = TensorGraph::from_canonical_json_bytes(bytes).ok()?;
    // Admit Tier-C committee graphs so committee receipts' required roots and
    // const blobs are resolved on the live fetch path.
    if graph.validate_for_committee().ok()? != *graph_id {
        return None;
    }
    Some(graph)
}
