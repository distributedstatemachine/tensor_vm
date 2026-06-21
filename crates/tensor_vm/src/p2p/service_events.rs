use crate::api::P2pMessage;
use crate::ir::IrTraceOpening;
use crate::tensor::Tensor;
use crate::types::Hash;
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, Swarm};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, mpsc};

use super::RequestResponseProtocol;
use super::behaviour::{TensorVmNetworkBehaviour, TensorVmNetworkBehaviourEvent};
use super::request_response::{PendingRequestKey, handle_request_response_event};
use super::wire::decode_message;

const OBSERVED_BLOCK_HASH_LIMIT: usize = 256;

pub(super) struct ServiceEventMetrics<'a> {
    pub(super) connected_peer_count: &'a AtomicUsize,
    pub(super) observed_block_gossip_count: &'a AtomicUsize,
    pub(super) observed_block_payload_gossip_count: &'a AtomicUsize,
    pub(super) observed_block_vote_gossip_count: &'a AtomicUsize,
    pub(super) observed_job_gossip_count: &'a AtomicUsize,
    pub(super) observed_receipt_gossip_count: &'a AtomicUsize,
    pub(super) observed_attestation_gossip_count: &'a AtomicUsize,
    pub(super) latest_observed_block_height: &'a AtomicU64,
    pub(super) latest_observed_block_hash: &'a Mutex<Hash>,
    pub(super) observed_block_hashes: &'a Mutex<VecDeque<Hash>>,
    pub(super) latest_observed_block_payload_height: &'a AtomicU64,
    pub(super) latest_observed_block_payload_hash: &'a Mutex<Hash>,
    pub(super) observed_block_payload_hashes: &'a Mutex<VecDeque<Hash>>,
    pub(super) connected_peer_ids: &'a Mutex<Vec<PeerId>>,
    pub(super) tensor_store: &'a Mutex<BTreeMap<Hash, Tensor>>,
    pub(super) program_store: &'a Mutex<BTreeMap<Hash, Vec<u8>>>,
    pub(super) trace_opening_store: &'a Mutex<BTreeMap<(Hash, u64), IrTraceOpening>>,
    pub(super) observed_message_tx: &'a mpsc::Sender<P2pMessage>,
}

pub(super) fn handle_swarm_event(
    event: SwarmEvent<TensorVmNetworkBehaviourEvent>,
    peer_connections: &mut HashMap<PeerId, usize>,
    metrics: &ServiceEventMetrics<'_>,
    pending_requests: &mut HashMap<
        PendingRequestKey,
        mpsc::SyncSender<std::result::Result<P2pMessage, &'static str>>,
    >,
    swarm: &mut Swarm<TensorVmNetworkBehaviour>,
) {
    match event {
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            *peer_connections.entry(peer_id).or_default() += 1;
            metrics
                .connected_peer_count
                .store(peer_connections.len(), Ordering::Relaxed);
            update_connected_peer_ids(metrics.connected_peer_ids, peer_connections);
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            if let Some(connection_count) = peer_connections.get_mut(&peer_id) {
                *connection_count = connection_count.saturating_sub(1);
                if *connection_count == 0 {
                    peer_connections.remove(&peer_id);
                }
            }
            metrics
                .connected_peer_count
                .store(peer_connections.len(), Ordering::Relaxed);
            update_connected_peer_ids(metrics.connected_peer_ids, peer_connections);
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
            libp2p::gossipsub::Event::Message { message, .. },
        )) => {
            if let Ok(message) = decode_message(&message.data) {
                let _ = metrics.observed_message_tx.send(message.clone());
                if let Some((height, block_hash)) = block_announcement(&message) {
                    metrics
                        .observed_block_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                    let update_latest_block_hash = if height > 0 {
                        let current_height =
                            metrics.latest_observed_block_height.load(Ordering::Relaxed);
                        if height >= current_height {
                            metrics
                                .latest_observed_block_height
                                .store(height, Ordering::Relaxed);
                            true
                        } else {
                            false
                        }
                    } else {
                        metrics.latest_observed_block_height.load(Ordering::Relaxed) == 0
                    };
                    if update_latest_block_hash
                        && let Ok(mut latest_block_hash) = metrics.latest_observed_block_hash.lock()
                    {
                        *latest_block_hash = block_hash;
                    }
                    if let Ok(mut block_hashes) = metrics.observed_block_hashes.lock() {
                        remember_observed_block_hash(&mut block_hashes, block_hash);
                    }
                }
                if let P2pMessage::NewBlockPayload {
                    height, block_hash, ..
                } = &message
                {
                    metrics
                        .observed_block_payload_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                    let current_height = metrics
                        .latest_observed_block_payload_height
                        .load(Ordering::Relaxed);
                    if *height >= current_height {
                        metrics
                            .latest_observed_block_payload_height
                            .store(*height, Ordering::Relaxed);
                        if let Ok(mut latest_block_hash) =
                            metrics.latest_observed_block_payload_hash.lock()
                        {
                            *latest_block_hash = *block_hash;
                        }
                    }
                    if let Ok(mut block_hashes) = metrics.observed_block_payload_hashes.lock() {
                        remember_observed_block_hash(&mut block_hashes, *block_hash);
                    }
                }
                if matches!(&message, P2pMessage::NewBlockVotePayload { .. }) {
                    metrics
                        .observed_block_vote_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewJob(_) | P2pMessage::NewJobPayload { .. }
                ) {
                    metrics
                        .observed_job_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewReceipt(_) | P2pMessage::NewReceiptPayload { .. }
                ) {
                    metrics
                        .observed_receipt_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                if matches!(
                    &message,
                    P2pMessage::NewAttestation(_)
                        | P2pMessage::NewAttestationPayload { .. }
                        | P2pMessage::NewValidatorVrfRevealPayload { .. }
                ) {
                    metrics
                        .observed_attestation_gossip_count
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorChunkRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorChunk,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorRowRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorRow,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorByRootRequestResponse(
            event,
        )) => {
            handle_request_response_event(
                RequestResponseProtocol::TensorByRoot,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::ProgramRequestResponse(event)) => {
            handle_request_response_event(
                RequestResponseProtocol::Program,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TraceOpeningRequestResponse(
            event,
        )) => {
            handle_request_response_event(
                RequestResponseProtocol::TraceOpening,
                event,
                metrics,
                pending_requests,
                swarm,
            );
        }
        _ => {}
    }
}

fn update_connected_peer_ids(
    connected_peer_ids: &Mutex<Vec<PeerId>>,
    peer_connections: &HashMap<PeerId, usize>,
) {
    if let Ok(mut peer_ids) = connected_peer_ids.lock() {
        *peer_ids = peer_connections.keys().copied().collect();
        peer_ids.sort_by_key(|peer_id| peer_id.to_string());
    }
}

fn remember_observed_block_hash(block_hashes: &mut VecDeque<Hash>, block_hash: Hash) {
    if block_hashes.contains(&block_hash) {
        return;
    }
    block_hashes.push_back(block_hash);
    while block_hashes.len() > OBSERVED_BLOCK_HASH_LIMIT {
        block_hashes.pop_front();
    }
}

fn block_announcement(message: &P2pMessage) -> Option<(u64, Hash)> {
    match message {
        P2pMessage::NewBlock(block_hash) => Some((0, *block_hash)),
        P2pMessage::NewBlockHeader { height, block_hash } => Some((*height, *block_hash)),
        P2pMessage::NewBlockPayload {
            height, block_hash, ..
        } => Some((*height, *block_hash)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hash_bytes;

    #[test]
    fn observed_block_hashes_are_bounded_and_deduplicated() {
        let mut block_hashes = VecDeque::new();
        let first_hash = hash_bytes(b"test", &[b"first-observed-block"]);
        remember_observed_block_hash(&mut block_hashes, first_hash);
        remember_observed_block_hash(&mut block_hashes, first_hash);

        for height in 0..(OBSERVED_BLOCK_HASH_LIMIT + 3) {
            let block_hash = hash_bytes(b"test", &[&height.to_le_bytes()]);
            remember_observed_block_hash(&mut block_hashes, block_hash);
        }

        assert_eq!(block_hashes.len(), OBSERVED_BLOCK_HASH_LIMIT);
        assert!(!block_hashes.contains(&first_hash));
        let last_hash = hash_bytes(b"test", &[&(OBSERVED_BLOCK_HASH_LIMIT + 2).to_le_bytes()]);
        assert_eq!(block_hashes.back(), Some(&last_hash));
    }
}
