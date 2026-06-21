use crate::api::P2pMessage;
use crate::error::{Result as TvmResult, TvmError};
use crate::tensor::Tensor;
use crate::types::Hash;
use futures::StreamExt;
use libp2p::PeerId;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::peer_book::parse_multiaddr;
use super::request_response::{
    PendingRequestKey, RequestResponseCommand, send_request_for_protocol,
};
use super::service_events::{ServiceEventMetrics, handle_swarm_event};
use super::wire::{
    encode_gossipsub_message, is_request_response_request, request_response_protocol_for_message,
};
use super::{Libp2pControlPlaneConfig, build_libp2p_node};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorVmLibp2pServiceInfo {
    pub peer_id: PeerId,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

pub struct TensorVmLibp2pService {
    info: TensorVmLibp2pServiceInfo,
    connected_peer_count: Arc<AtomicUsize>,
    observed_block_gossip_count: Arc<AtomicUsize>,
    observed_block_payload_gossip_count: Arc<AtomicUsize>,
    observed_block_vote_gossip_count: Arc<AtomicUsize>,
    observed_job_gossip_count: Arc<AtomicUsize>,
    observed_receipt_gossip_count: Arc<AtomicUsize>,
    observed_attestation_gossip_count: Arc<AtomicUsize>,
    latest_observed_block_height: Arc<AtomicU64>,
    latest_observed_block_hash: Arc<Mutex<Hash>>,
    observed_block_hashes: Arc<Mutex<VecDeque<Hash>>>,
    latest_observed_block_payload_height: Arc<AtomicU64>,
    latest_observed_block_payload_hash: Arc<Mutex<Hash>>,
    observed_block_payload_hashes: Arc<Mutex<VecDeque<Hash>>>,
    connected_peer_ids: Arc<Mutex<Vec<PeerId>>>,
    tensor_store: Arc<Mutex<BTreeMap<Hash, Tensor>>>,
    program_store: Arc<Mutex<BTreeMap<Hash, Vec<u8>>>>,
    observed_message_rx: Mutex<mpsc::Receiver<P2pMessage>>,
    publish_tx: mpsc::Sender<P2pMessage>,
    request_tx: mpsc::Sender<RequestResponseCommand>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl TensorVmLibp2pService {
    pub fn info(&self) -> &TensorVmLibp2pServiceInfo {
        &self.info
    }

    pub fn peer_id(&self) -> PeerId {
        self.info.peer_id
    }

    pub fn connected_peer_count(&self) -> usize {
        self.connected_peer_count.load(Ordering::Relaxed)
    }

    pub fn observed_block_gossip_count(&self) -> usize {
        self.observed_block_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_block_payload_gossip_count(&self) -> usize {
        self.observed_block_payload_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn observed_block_vote_gossip_count(&self) -> usize {
        self.observed_block_vote_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn observed_job_gossip_count(&self) -> usize {
        self.observed_job_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_receipt_gossip_count(&self) -> usize {
        self.observed_receipt_gossip_count.load(Ordering::Relaxed)
    }

    pub fn observed_attestation_gossip_count(&self) -> usize {
        self.observed_attestation_gossip_count
            .load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_height(&self) -> u64 {
        self.latest_observed_block_height.load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_hash(&self) -> Hash {
        self.latest_observed_block_hash
            .lock()
            .map(|hash| *hash)
            .unwrap_or([0; 32])
    }

    pub fn observed_block_hashes(&self) -> Vec<Hash> {
        self.observed_block_hashes
            .lock()
            .map(|hashes| hashes.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn latest_observed_block_payload_height(&self) -> u64 {
        self.latest_observed_block_payload_height
            .load(Ordering::Relaxed)
    }

    pub fn latest_observed_block_payload_hash(&self) -> Hash {
        self.latest_observed_block_payload_hash
            .lock()
            .map(|hash| *hash)
            .unwrap_or([0; 32])
    }

    pub fn observed_block_payload_hashes(&self) -> Vec<Hash> {
        self.observed_block_payload_hashes
            .lock()
            .map(|hashes| hashes.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn connected_peer_ids(&self) -> Vec<PeerId> {
        self.connected_peer_ids
            .lock()
            .map(|peer_ids| peer_ids.clone())
            .unwrap_or_default()
    }

    pub fn drain_observed_messages(&self) -> Vec<P2pMessage> {
        let receiver = self
            .observed_message_rx
            .lock()
            .expect("observed message receiver mutex poisoned");
        let mut messages = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    pub fn publish_gossip(&self, message: P2pMessage) -> TvmResult<()> {
        encode_gossipsub_message(&message)?;
        self.publish_tx
            .send(message)
            .map_err(|_| TvmError::InvalidReceipt("libp2p publish worker stopped"))
    }

    pub fn register_tensor(&self, tensor: Tensor) {
        if let Ok(mut tensors) = self.tensor_store.lock() {
            tensors.insert(tensor.tensor_id(), tensor);
        }
    }

    pub fn register_program(&self, program_hash: Hash, bytes: Vec<u8>) {
        if let Ok(mut programs) = self.program_store.lock() {
            programs.insert(program_hash, bytes);
        }
    }

    pub fn request_response(
        &self,
        peer_id: PeerId,
        request: P2pMessage,
        timeout: Duration,
    ) -> TvmResult<P2pMessage> {
        if !is_request_response_request(&request) {
            return Err(TvmError::InvalidReceipt(
                "message is not a request-response request",
            ));
        }
        let protocol = request_response_protocol_for_message(&request).ok_or(
            TvmError::InvalidReceipt("request-response protocol missing"),
        )?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        self.request_tx
            .send(RequestResponseCommand {
                peer_id,
                protocol,
                request,
                response_tx,
            })
            .map_err(|_| TvmError::InvalidReceipt("libp2p request worker stopped"))?;
        response_rx
            .recv_timeout(timeout)
            .map_err(|_| TvmError::InvalidReceipt("libp2p request-response timeout"))?
            .map_err(TvmError::InvalidReceipt)
    }
}

impl Drop for TensorVmLibp2pService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn spawn_libp2p_service(config: Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pService> {
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let connected_peer_count = Arc::new(AtomicUsize::new(0));
    let worker_connected_peer_count = Arc::clone(&connected_peer_count);
    let observed_block_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_gossip_count = Arc::clone(&observed_block_gossip_count);
    let observed_block_payload_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_payload_gossip_count =
        Arc::clone(&observed_block_payload_gossip_count);
    let observed_block_vote_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_block_vote_gossip_count = Arc::clone(&observed_block_vote_gossip_count);
    let observed_job_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_job_gossip_count = Arc::clone(&observed_job_gossip_count);
    let observed_receipt_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_receipt_gossip_count = Arc::clone(&observed_receipt_gossip_count);
    let observed_attestation_gossip_count = Arc::new(AtomicUsize::new(0));
    let worker_observed_attestation_gossip_count = Arc::clone(&observed_attestation_gossip_count);
    let latest_observed_block_height = Arc::new(AtomicU64::new(0));
    let worker_latest_observed_block_height = Arc::clone(&latest_observed_block_height);
    let latest_observed_block_hash = Arc::new(Mutex::new([0; 32]));
    let worker_latest_observed_block_hash = Arc::clone(&latest_observed_block_hash);
    let observed_block_hashes = Arc::new(Mutex::new(VecDeque::new()));
    let worker_observed_block_hashes = Arc::clone(&observed_block_hashes);
    let latest_observed_block_payload_height = Arc::new(AtomicU64::new(0));
    let worker_latest_observed_block_payload_height =
        Arc::clone(&latest_observed_block_payload_height);
    let latest_observed_block_payload_hash = Arc::new(Mutex::new([0; 32]));
    let worker_latest_observed_block_payload_hash = Arc::clone(&latest_observed_block_payload_hash);
    let observed_block_payload_hashes = Arc::new(Mutex::new(VecDeque::new()));
    let worker_observed_block_payload_hashes = Arc::clone(&observed_block_payload_hashes);
    let connected_peer_ids = Arc::new(Mutex::new(Vec::new()));
    let worker_connected_peer_ids = Arc::clone(&connected_peer_ids);
    let tensor_store = Arc::new(Mutex::new(BTreeMap::new()));
    let worker_tensor_store = Arc::clone(&tensor_store);
    let program_store = Arc::new(Mutex::new(BTreeMap::new()));
    let worker_program_store = Arc::clone(&program_store);
    let (publish_tx, publish_rx) = mpsc::channel();
    let (request_tx, request_rx) = mpsc::channel::<RequestResponseCommand>();
    let (observed_message_tx, observed_message_rx) = mpsc::channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| TvmError::InvalidReceipt("libp2p runtime build failed"))?;
    let worker = thread::spawn(move || {
        runtime.block_on(async move {
            let mut node = match build_libp2p_node(&config) {
                Ok(node) => node,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            let info = TensorVmLibp2pServiceInfo {
                peer_id: node.peer_id,
                identify_protocol: node.identify_protocol.clone(),
                subscribed_topics: node.subscribed_topics.clone(),
                request_response_protocols: node.request_response_protocols.clone(),
            };
            let _ = ready_tx.send(Ok(info));
            let bootstrap_multiaddrs = config
                .bootstrap_addresses
                .iter()
                .filter_map(|address| parse_multiaddr(address).ok())
                .collect::<Vec<_>>();
            let mut next_bootstrap_dial = Instant::now() + Duration::from_millis(250);
            let mut peer_connections = HashMap::new();
            let event_metrics = ServiceEventMetrics {
                connected_peer_count: worker_connected_peer_count.as_ref(),
                observed_block_gossip_count: worker_observed_block_gossip_count.as_ref(),
                observed_block_payload_gossip_count: worker_observed_block_payload_gossip_count
                    .as_ref(),
                observed_block_vote_gossip_count: worker_observed_block_vote_gossip_count.as_ref(),
                observed_job_gossip_count: worker_observed_job_gossip_count.as_ref(),
                observed_receipt_gossip_count: worker_observed_receipt_gossip_count.as_ref(),
                observed_attestation_gossip_count: worker_observed_attestation_gossip_count
                    .as_ref(),
                latest_observed_block_height: worker_latest_observed_block_height.as_ref(),
                latest_observed_block_hash: worker_latest_observed_block_hash.as_ref(),
                observed_block_hashes: worker_observed_block_hashes.as_ref(),
                latest_observed_block_payload_height: worker_latest_observed_block_payload_height
                    .as_ref(),
                latest_observed_block_payload_hash: worker_latest_observed_block_payload_hash
                    .as_ref(),
                observed_block_payload_hashes: worker_observed_block_payload_hashes.as_ref(),
                connected_peer_ids: worker_connected_peer_ids.as_ref(),
                tensor_store: worker_tensor_store.as_ref(),
                program_store: worker_program_store.as_ref(),
                observed_message_tx: &observed_message_tx,
            };
            let mut pending_requests = HashMap::new();

            while !worker_stop.load(Ordering::Relaxed) {
                while let Ok(message) = publish_rx.try_recv() {
                    if let Ok((topic, payload)) = encode_gossipsub_message(&message) {
                        let _ = node.swarm.behaviour_mut().gossipsub.publish(topic, payload);
                    }
                }
                while let Ok(command) = request_rx.try_recv() {
                    if request_response_protocol_for_message(&command.request)
                        != Some(command.protocol)
                        || !node
                            .request_response_protocols
                            .iter()
                            .any(|protocol| protocol == command.protocol.as_str())
                    {
                        let _ = command
                            .response_tx
                            .send(Err("message is not a request-response request"));
                        continue;
                    }
                    let request_id = send_request_for_protocol(
                        &mut node.swarm,
                        command.protocol,
                        &command.peer_id,
                        command.request,
                    );
                    pending_requests.insert(
                        PendingRequestKey {
                            protocol: command.protocol,
                            request_id,
                        },
                        command.response_tx,
                    );
                }
                if let Ok(event) =
                    tokio::time::timeout(Duration::from_millis(100), node.swarm.select_next_some())
                        .await
                {
                    handle_swarm_event(
                        event,
                        &mut peer_connections,
                        &event_metrics,
                        &mut pending_requests,
                        &mut node.swarm,
                    );
                }
                if !bootstrap_multiaddrs.is_empty()
                    && peer_connections.is_empty()
                    && Instant::now() >= next_bootstrap_dial
                {
                    for address in &bootstrap_multiaddrs {
                        let _ = node.swarm.dial(address.clone());
                    }
                    next_bootstrap_dial = Instant::now() + Duration::from_secs(1);
                }
            }
        });
    });

    match ready_rx
        .recv()
        .map_err(|_| TvmError::InvalidReceipt("libp2p service failed to start"))?
    {
        Ok(info) => Ok(TensorVmLibp2pService {
            info,
            connected_peer_count,
            observed_block_gossip_count,
            observed_block_payload_gossip_count,
            observed_block_vote_gossip_count,
            observed_job_gossip_count,
            observed_receipt_gossip_count,
            observed_attestation_gossip_count,
            latest_observed_block_height,
            latest_observed_block_hash,
            observed_block_hashes,
            latest_observed_block_payload_height,
            latest_observed_block_payload_hash,
            observed_block_payload_hashes,
            connected_peer_ids,
            tensor_store,
            program_store,
            observed_message_rx: Mutex::new(observed_message_rx),
            publish_tx,
            request_tx,
            stop,
            worker: Some(worker),
        }),
        Err(error) => {
            let _ = worker.join();
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Libp2pControlPlaneConfig;
    use super::super::wire::{
        decode_tensor_payload, encode_block_payload, encode_block_vote_payload, encode_job_payload,
    };
    use super::{TensorVmLibp2pService, spawn_libp2p_service};
    use crate::api::P2pMessage;
    use crate::chain::{BlockVote, Chain, ChainCommand, ChainEngine, JobState, TensorBlock};
    use crate::error::TvmError;
    use crate::node::{NetworkPayloadApply, apply_network_job_payload};
    use crate::scheduler::SyntheticLocalJobSource;
    use crate::tensor::{DType, Tensor};
    use crate::types::{Hash, address, hash_bytes};
    use std::time::{Duration, Instant};

    #[test]
    fn libp2p_service_spawns_background_runtime() {
        let service = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        assert!(!service.peer_id().to_string().is_empty());
        assert_eq!(service.info().identify_protocol, "/tensorchain/1/identify");
        assert_eq!(service.info().subscribed_topics.len(), 5);
        assert_eq!(service.info().request_response_protocols.len(), 4);
        std::thread::sleep(Duration::from_millis(150));
    }

    #[test]
    fn libp2p_service_reports_connected_peer_count() {
        let port = free_tcp_port();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-connected-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-connected-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
    }

    #[test]
    fn libp2p_service_fetches_tensor_by_commitment_root() {
        let port = free_tcp_port();
        let tensor =
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![11, 13, 17, 19]).unwrap();
        let commitment_root = tensor.commitment_root();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-fetch-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        service_a.register_tensor(tensor.clone());
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-fetch-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
        assert!(
            service_b
                .connected_peer_ids()
                .contains(&service_a.peer_id())
        );
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestTensorByCommitmentRoot { commitment_root },
                Duration::from_secs(5),
            )
            .unwrap();
        let P2pMessage::TensorByCommitmentRootResponse {
            commitment_root: response_root,
            payload: Some(payload),
        } = response
        else {
            panic!("expected tensor-by-root response");
        };
        assert_eq!(response_root, commitment_root);
        assert_eq!(decode_tensor_payload(&payload).unwrap(), tensor);

        let missing_root = hash_bytes(b"test", &[b"missing-tensor-root"]);
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestTensorByCommitmentRoot {
                    commitment_root: missing_root,
                },
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(
            response,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: missing_root,
                payload: None,
            }
        );
    }

    #[test]
    fn libp2p_service_fetches_registered_program_body() {
        let port = free_tcp_port();
        let program_hash = hash_bytes(b"test", &[b"libp2p-service-program"]);
        let program_body = b"{\"ir_version\":1,\"ops\":[]}".to_vec();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-program-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        service_a.register_program(program_hash, program_body.clone());
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-program-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestProgram(program_hash),
                Duration::from_secs(3),
            )
            .unwrap();
        assert_eq!(
            response,
            P2pMessage::ProgramResponse {
                program_hash,
                bytes: program_body,
            }
        );

        let missing = hash_bytes(b"test", &[b"missing-program"]);
        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestProgram(missing),
                Duration::from_secs(3),
            )
            .unwrap();
        assert_eq!(
            response,
            P2pMessage::ProgramResponse {
                program_hash: missing,
                bytes: Vec::new(),
            }
        );
    }

    #[test]
    fn libp2p_service_propagates_external_graph_job_artifacts() {
        let port = free_tcp_port();
        let seed = hash_bytes(b"test", &[b"libp2p-external-graph-artifacts"]);
        let source_chain = Chain::new(seed);
        let mut job_source = SyntheticLocalJobSource::default();
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let program_hash = graph.graph_id();
        let program_body = graph.canonical_json().into_bytes();
        let inputs = SyntheticLocalJobSource::graph_execution_inputs();
        let job = JobState::GraphExecution(job_source.next_graph_job(&source_chain));
        let job_id = job.job_id();
        let job_payload = encode_job_payload(&job);

        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-external-graph-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        service_a.register_program(program_hash, program_body.clone());
        for tensor in inputs.values() {
            service_a.register_tensor(tensor.clone());
        }
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-external-graph-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        wait_for_connected_services(&service_a, &service_b);

        let mut receiver = crate::RpcNode::new(Chain::new(seed));
        assert_eq!(
            apply_network_job_payload(&mut receiver.chain, job_id, &job_payload),
            NetworkPayloadApply::Pending
        );
        let observed_job = wait_for_observed_job_payload(
            &service_a,
            &service_b,
            P2pMessage::NewJobPayload {
                job_id,
                payload: job_payload.clone(),
            },
        );
        assert_eq!(observed_job, job_payload);

        let response = service_b
            .request_response(
                service_a.peer_id(),
                P2pMessage::RequestProgram(program_hash),
                Duration::from_secs(3),
            )
            .unwrap();
        assert_eq!(
            response,
            P2pMessage::ProgramResponse {
                program_hash,
                bytes: program_body.clone(),
            }
        );
        receiver
            .chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: program_hash,
                bytes: program_body,
            })
            .unwrap();

        for expected in inputs.values() {
            let commitment_root = expected.commitment_root();
            let response = service_b
                .request_response(
                    service_a.peer_id(),
                    P2pMessage::RequestTensorByCommitmentRoot { commitment_root },
                    Duration::from_secs(5),
                )
                .unwrap();
            let P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: response_root,
                payload: Some(payload),
            } = response
            else {
                panic!("expected tensor artifact response");
            };
            assert_eq!(response_root, commitment_root);
            let tensor = decode_tensor_payload(&payload).unwrap();
            assert_eq!(tensor, *expected);
            receiver.insert_tensor(tensor);
        }

        assert_eq!(
            apply_network_job_payload(&mut receiver.chain, job_id, &job_payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(receiver.chain.state().jobs().get(&job_id), Some(&job));
        for tensor in inputs.values() {
            assert!(receiver.contains_tensor_commitment_root(&tensor.commitment_root()));
        }
    }

    #[test]
    fn libp2p_service_redials_bootstrap_peer_after_restart() {
        let port = free_tcp_port();
        let seed_a = hash_bytes(b"test", &[b"libp2p-service-redial-a"]);
        let mut service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(seed_a),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-redial-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        wait_for_connected_services(&service_a, &service_b);

        drop(service_a);
        wait_for_peer_count(&service_b, 0);
        service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(seed_a),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();

        wait_for_connected_services(&service_a, &service_b);
    }

    #[test]
    fn libp2p_service_publishes_and_observes_block_gossip() {
        let port = free_tcp_port();
        let service_a = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-gossip-a"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/{port}/p2p/{}", service_a.peer_id());
        let service_b = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-gossip-b"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        wait_for_connected_services(&service_a, &service_b);

        let block_hash = hash_bytes(b"test", &[b"libp2p-service-observed-block"]);
        wait_for_observed_block(&service_a, &service_b, block_hash);
        let block_header_hash = hash_bytes(b"test", &[b"libp2p-service-observed-block-header"]);
        wait_for_observed_block_header(&service_a, &service_b, 7, block_header_hash);
        wait_for_stale_block_announcements_to_preserve_latest_header(
            &service_a,
            &service_b,
            7,
            block_header_hash,
        );
        let block_payload = wire_test_block(b"libp2p-service-observed-block-payload", 8);
        wait_for_observed_block_payload(&service_a, &service_b, &block_payload);
        let block_vote = BlockVote::new(
            address(b"libp2p-observed-vote-validator"),
            10_000,
            &block_payload,
        );
        wait_for_observed_block_vote(&service_a, &service_b, &block_vote);
        wait_for_observed_consensus_gossip(&service_a, &service_b);
        let observed_messages = service_b.drain_observed_messages();
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlock(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockHeader { height: 7, .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockPayload { height: 8, .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewBlockVotePayload { .. }))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewJob(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewReceipt(_)))
        );
        assert!(
            observed_messages
                .iter()
                .any(|message| matches!(message, P2pMessage::NewAttestation(_)))
        );
        assert!(service_b.drain_observed_messages().is_empty());
    }

    #[test]
    fn libp2p_service_rejects_request_response_gossip_publish() {
        let service = spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            identity_seed: Some(hash_bytes(b"test", &[b"libp2p-service-bad-publish"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap();
        let hash = hash_bytes(b"test", &[b"request-response-publish"]);

        assert_eq!(
            service.publish_gossip(P2pMessage::RequestProgram(hash)),
            Err(TvmError::InvalidReceipt(
                "message is not a gossipsub announcement"
            ))
        );
    }

    fn free_tcp_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn wire_test_block(label: &[u8], height: u64) -> TensorBlock {
        TensorBlock {
            height,
            parent_hash: hash_bytes(b"test-block", &[label, b"parent"]),
            epoch: height / 4,
            proposer: hash_bytes(b"test-block", &[label, b"proposer"]),
            settled_receipt_set_root: hash_bytes(b"test-block", &[label, b"settled"]),
            checks_root: hash_bytes(b"test-block", &[label, b"checks"]),
            attestation_root: hash_bytes(b"test-block", &[label, b"attestations"]),
            state_root: hash_bytes(b"test-block", &[label, b"state"]),
            reward_root: hash_bytes(b"test-block", &[label, b"rewards"]),
            beacon_round: height,
            beacon: hash_bytes(b"test-block", &[label, b"beacon"]),
            production_kind: crate::chain::BlockProductionKind::UsefulVerificationPow,
            proposer_reward: 0,
            difficulty_target: [0xff; 32],
            nonce: height.saturating_add(1),
            timestamp: height.saturating_mul(6),
            proposer_signature: hash_bytes(b"test-block", &[label, b"proposer-signature"]),
            validator_signature_aggregate: hash_bytes(
                b"test-block",
                &[label, b"validator-signature"],
            ),
        }
    }

    fn wait_for_connected_services(
        service_a: &TensorVmLibp2pService,
        service_b: &TensorVmLibp2pService,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (service_a.connected_peer_count() == 0 || service_b.connected_peer_count() == 0)
        {
            std::thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(service_a.connected_peer_count(), 1);
        assert_eq!(service_b.connected_peer_count(), 1);
    }

    fn wait_for_peer_count(service: &TensorVmLibp2pService, expected_count: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && service.connected_peer_count() != expected_count {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(service.connected_peer_count(), expected_count);
    }

    fn wait_for_observed_block(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && observer.latest_observed_block_hash() != block_hash {
            publisher
                .publish_gossip(P2pMessage::NewBlock(block_hash))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_gossip_count() > 0);
        assert_eq!(observer.latest_observed_block_hash(), block_hash);
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_block_header(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        height: u64,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.latest_observed_block_height() != height
                || observer.latest_observed_block_hash() != block_hash)
        {
            publisher
                .publish_gossip(P2pMessage::NewBlockHeader { height, block_hash })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_gossip_count() > 1);
        assert_eq!(observer.latest_observed_block_height(), height);
        assert_eq!(observer.latest_observed_block_hash(), block_hash);
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_block_payload(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        block: &TensorBlock,
    ) {
        let block_hash = block.hash();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.latest_observed_block_payload_height() != block.height
                || observer.latest_observed_block_payload_hash() != block_hash)
        {
            publisher
                .publish_gossip(P2pMessage::NewBlockPayload {
                    height: block.height,
                    block_hash,
                    payload: encode_block_payload(block),
                })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_payload_gossip_count() > 0);
        assert_eq!(
            observer.latest_observed_block_payload_height(),
            block.height
        );
        assert_eq!(observer.latest_observed_block_payload_hash(), block_hash);
        assert!(
            observer
                .observed_block_payload_hashes()
                .contains(&block_hash)
        );
    }

    fn wait_for_observed_block_vote(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        vote: &BlockVote,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && observer.observed_block_vote_gossip_count() == 0 {
            publisher
                .publish_gossip(P2pMessage::NewBlockVotePayload {
                    block_hash: vote.block_hash,
                    validator: vote.validator,
                    payload: encode_block_vote_payload(vote),
                })
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_block_vote_gossip_count() > 0);
    }

    fn wait_for_stale_block_announcements_to_preserve_latest_header(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        latest_height: u64,
        latest_hash: Hash,
    ) {
        let stale_header_hash = hash_bytes(b"test", &[b"stale-block-header"]);
        wait_for_observed_hash(
            publisher,
            observer,
            P2pMessage::NewBlockHeader {
                height: latest_height - 1,
                block_hash: stale_header_hash,
            },
            stale_header_hash,
        );
        assert_eq!(observer.latest_observed_block_height(), latest_height);
        assert_eq!(observer.latest_observed_block_hash(), latest_hash);

        let legacy_block_hash = hash_bytes(b"test", &[b"legacy-block-without-height"]);
        wait_for_observed_hash(
            publisher,
            observer,
            P2pMessage::NewBlock(legacy_block_hash),
            legacy_block_hash,
        );
        assert_eq!(observer.latest_observed_block_height(), latest_height);
        assert_eq!(observer.latest_observed_block_hash(), latest_hash);
    }

    fn wait_for_observed_hash(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        message: P2pMessage,
        block_hash: Hash,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !observer.observed_block_hashes().contains(&block_hash) {
            publisher.publish_gossip(message.clone()).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(observer.observed_block_hashes().contains(&block_hash));
    }

    fn wait_for_observed_consensus_gossip(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
    ) {
        let job_hash = hash_bytes(b"test", &[b"libp2p-service-observed-job"]);
        let receipt_hash = hash_bytes(b"test", &[b"libp2p-service-observed-receipt"]);
        let attestation_hash = hash_bytes(b"test", &[b"libp2p-service-observed-attestation"]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
            && (observer.observed_job_gossip_count() == 0
                || observer.observed_receipt_gossip_count() == 0
                || observer.observed_attestation_gossip_count() == 0)
        {
            publisher
                .publish_gossip(P2pMessage::NewJob(job_hash))
                .unwrap();
            publisher
                .publish_gossip(P2pMessage::NewReceipt(receipt_hash))
                .unwrap();
            publisher
                .publish_gossip(P2pMessage::NewAttestation(attestation_hash))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }

        assert!(observer.observed_job_gossip_count() > 0);
        assert!(observer.observed_receipt_gossip_count() > 0);
        assert!(observer.observed_attestation_gossip_count() > 0);
    }

    fn wait_for_observed_job_payload(
        publisher: &TensorVmLibp2pService,
        observer: &TensorVmLibp2pService,
        message: P2pMessage,
    ) -> Vec<u8> {
        let expected_job_id = match &message {
            P2pMessage::NewJobPayload { job_id, .. } => *job_id,
            _ => panic!("expected job payload message"),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            publisher.publish_gossip(message.clone()).unwrap();
            for observed in observer.drain_observed_messages() {
                if let P2pMessage::NewJobPayload { job_id, payload } = observed
                    && job_id == expected_job_id
                {
                    return payload;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("timed out waiting for graph job payload gossip");
    }

    #[test]
    fn libp2p_service_rejects_invalid_runtime_config() {
        let error = match spawn_libp2p_service(Libp2pControlPlaneConfig {
            listen_addresses: vec!["not-a-multiaddr".to_owned()],
            ..Libp2pControlPlaneConfig::default()
        }) {
            Err(error) => error,
            Ok(_) => panic!("invalid libp2p config started"),
        };
        assert_eq!(error, TvmError::InvalidReceipt("invalid libp2p multiaddr"));
    }
}
