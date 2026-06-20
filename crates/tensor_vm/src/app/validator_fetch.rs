use crate::{
    ReceiptState, RpcNode, Tensor, TensorVmLibp2pService, api::P2pMessage, decode_tensor_payload,
    types::Hash,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRemoteTensorFetchReport {
    pub attempts: usize,
    pub successes: usize,
    pub failures: usize,
    pub bytes: usize,
    pub tensors_inserted: usize,
}

pub fn fetch_validator_role_missing_tensors(
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    receipt_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    };
    let missing_roots = validator_receipt_required_remote_roots(node, &receipt);
    if missing_roots.is_empty() {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    }
    let peers = p2p_service.connected_peer_ids();
    let mut report = ValidatorRemoteTensorFetchReport::default();
    if peers.is_empty() {
        report.failures = missing_roots.len();
        return Ok(report);
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
                    record_validator_remote_fetch_failure(
                        &mut report,
                        &mut failed_response_recorded,
                    );
                }
            }
        }
        if !fetched && !failed_response_recorded {
            report.failures = report.failures.saturating_add(1);
        }
    }
    Ok(report)
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
            roots.extend(receipt.input_roots.values().copied());
            roots.extend(receipt.output_roots.values().copied());
        }
    }
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .filter(|root| !node.contains_tensor_commitment_root(root))
        .collect()
}
