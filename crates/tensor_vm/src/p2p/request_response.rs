use crate::api::P2pMessage;
use crate::error::Result as TvmResult;
use crate::tensor::Tensor;
use crate::types::Hash;
use libp2p::{PeerId, Swarm};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use super::behaviour::TensorVmNetworkBehaviour;
use super::service_events::ServiceEventMetrics;
use super::wire::{
    encode_tensor_payload, is_request_response_request, request_response_protocol_for_message,
    request_response_stream_protocol,
};
use super::{Libp2pControlPlaneConfig, RequestResponseProtocol};

pub type P2pRequestResponseBehaviour =
    libp2p::request_response::json::Behaviour<P2pMessage, P2pMessage>;

pub(super) type P2pRequestResponseEvent = libp2p::request_response::Event<P2pMessage, P2pMessage>;

pub(super) struct RequestResponseCommand {
    pub(super) peer_id: PeerId,
    pub(super) protocol: RequestResponseProtocol,
    pub(super) request: P2pMessage,
    pub(super) response_tx: mpsc::SyncSender<std::result::Result<P2pMessage, &'static str>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct PendingRequestKey {
    pub(super) protocol: RequestResponseProtocol,
    pub(super) request_id: libp2p::request_response::OutboundRequestId,
}

pub(super) fn handle_request_response_event(
    protocol: RequestResponseProtocol,
    event: P2pRequestResponseEvent,
    metrics: &ServiceEventMetrics<'_>,
    pending_requests: &mut HashMap<
        PendingRequestKey,
        mpsc::SyncSender<std::result::Result<P2pMessage, &'static str>>,
    >,
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
) {
    match event {
        libp2p::request_response::Event::Message { message, .. } => match message {
            libp2p::request_response::Message::Request {
                request, channel, ..
            } => {
                if !is_request_response_request(&request)
                    || request_response_protocol_for_message(&request) != Some(protocol)
                {
                    return;
                }
                let response =
                    response_for_request(&request, metrics.tensor_store, metrics.program_store);
                let _ = send_response_for_protocol(swarm, protocol, channel, response);
            }
            libp2p::request_response::Message::Response {
                request_id,
                response,
            } => {
                let key = PendingRequestKey {
                    protocol,
                    request_id,
                };
                if let Some(response_tx) = pending_requests.remove(&key) {
                    if request_response_protocol_for_message(&response) == Some(protocol) {
                        let _ = response_tx.send(Ok(response));
                    } else {
                        let _ = response_tx.send(Err("libp2p request-response protocol mismatch"));
                    }
                }
            }
        },
        libp2p::request_response::Event::OutboundFailure { request_id, .. } => {
            let key = PendingRequestKey {
                protocol,
                request_id,
            };
            if let Some(response_tx) = pending_requests.remove(&key) {
                let _ = response_tx.send(Err("libp2p request-response failed"));
            }
        }
        _ => {}
    }
}

fn request_response_behaviour_mut(
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
    protocol: RequestResponseProtocol,
) -> &mut P2pRequestResponseBehaviour {
    match protocol {
        RequestResponseProtocol::TensorChunk => {
            &mut swarm.behaviour_mut().tensor_chunk_request_response
        }
        RequestResponseProtocol::TensorRow => {
            &mut swarm.behaviour_mut().tensor_row_request_response
        }
        RequestResponseProtocol::TensorByRoot => {
            &mut swarm.behaviour_mut().tensor_by_root_request_response
        }
        RequestResponseProtocol::Program => &mut swarm.behaviour_mut().program_request_response,
    }
}

pub(super) fn send_request_for_protocol(
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
    protocol: RequestResponseProtocol,
    peer_id: &PeerId,
    request: P2pMessage,
) -> libp2p::request_response::OutboundRequestId {
    request_response_behaviour_mut(swarm, protocol).send_request(peer_id, request)
}

pub(super) fn send_response_for_protocol(
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
    protocol: RequestResponseProtocol,
    channel: libp2p::request_response::ResponseChannel<P2pMessage>,
    response: P2pMessage,
) -> Result<(), P2pMessage> {
    request_response_behaviour_mut(swarm, protocol).send_response(channel, response)
}

fn response_for_request(
    request: &P2pMessage,
    tensor_store: &Mutex<BTreeMap<Hash, Tensor>>,
    program_store: &Mutex<BTreeMap<Hash, Vec<u8>>>,
) -> P2pMessage {
    match request {
        P2pMessage::RequestTensorByCommitmentRoot { commitment_root } => {
            let payload = tensor_store
                .lock()
                .ok()
                .and_then(|tensors| tensor_by_commitment_root(&tensors, commitment_root).cloned())
                .map(|tensor| encode_tensor_payload(&tensor));
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: *commitment_root,
                payload,
            }
        }
        P2pMessage::RequestTensorRow {
            tensor_id,
            row_index,
        } => {
            let values = tensor_store
                .lock()
                .ok()
                .and_then(|tensors| tensors.get(tensor_id).cloned())
                .and_then(|tensor| tensor.row(*row_index as usize).ok().map(|row| row.to_vec()))
                .unwrap_or_default();
            P2pMessage::TensorRowResponse {
                tensor_id: *tensor_id,
                row_index: *row_index,
                values,
            }
        }
        P2pMessage::RequestTensorChunk {
            tensor_id,
            chunk_index,
        } => {
            let bytes = tensor_store
                .lock()
                .ok()
                .and_then(|tensors| tensors.get(tensor_id).cloned())
                .and_then(|tensor| {
                    tensor
                        .opening(*chunk_index, crate::tensor::DEFAULT_CHUNK_SIZE)
                        .ok()
                        .map(|opening| opening.chunk_bytes)
                })
                .unwrap_or_default();
            P2pMessage::TensorChunkResponse {
                tensor_id: *tensor_id,
                chunk_index: *chunk_index,
                bytes,
            }
        }
        P2pMessage::RequestProgram(program_hash) => P2pMessage::ProgramResponse {
            program_hash: *program_hash,
            bytes: program_store
                .lock()
                .ok()
                .and_then(|programs| programs.get(program_hash).cloned())
                .unwrap_or_default(),
        },
        _ => P2pMessage::ProgramResponse {
            program_hash: [0; 32],
            bytes: Vec::new(),
        },
    }
}

fn tensor_by_commitment_root<'a>(
    tensors: &'a BTreeMap<Hash, Tensor>,
    commitment_root: &Hash,
) -> Option<&'a Tensor> {
    tensors
        .values()
        .find(|tensor| tensor.commitment_root() == *commitment_root)
}

pub(super) fn build_request_response_behaviour(
    config: &Libp2pControlPlaneConfig,
    protocol: RequestResponseProtocol,
) -> TvmResult<P2pRequestResponseBehaviour> {
    let request_protocols = if config.request_response_protocols.contains(&protocol) {
        vec![(
            request_response_stream_protocol(protocol)?,
            libp2p::request_response::ProtocolSupport::Full,
        )]
    } else {
        Vec::new()
    };
    Ok(libp2p::request_response::json::Behaviour::new(
        request_protocols,
        libp2p::request_response::Config::default()
            .with_request_timeout(Duration::from_secs(config.request_timeout_seconds))
            .with_max_concurrent_streams(config.max_concurrent_request_streams),
    ))
}
