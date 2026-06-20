use crate::error::{Result as TvmResult, TvmError};
use libp2p::{PeerId, Swarm};
use std::time::Duration;

use super::behaviour::{TensorVmNetworkBehaviour, build_libp2p_behaviour};
use super::peer_book::{bootstrap_peer_address, parse_multiaddr};
use super::{LIBP2P_PROTOCOL_PREFIX, Libp2pControlPlaneConfig};

pub struct TensorVmLibp2pNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<TensorVmNetworkBehaviour>,
    pub identify_protocol: String,
    pub subscribed_topics: Vec<String>,
    pub request_response_protocols: Vec<String>,
}

pub fn build_libp2p_node(config: &Libp2pControlPlaneConfig) -> TvmResult<TensorVmLibp2pNode> {
    let keypair = match config.identity_seed {
        Some(seed) => libp2p::identity::Keypair::ed25519_from_bytes(seed)
            .map_err(|_| TvmError::InvalidReceipt("libp2p identity seed rejected"))?,
        None => libp2p::identity::Keypair::generate_ed25519(),
    };
    let peer_id = PeerId::from(keypair.public());
    let behaviour = build_libp2p_behaviour(config, &keypair)?;
    let identify_protocol = format!("{LIBP2P_PROTOCOL_PREFIX}/identify");
    let subscribed_topics = config
        .gossipsub_topics
        .iter()
        .map(|topic| topic.as_str().to_owned())
        .collect();
    let request_response_protocols = config
        .request_response_protocols
        .iter()
        .map(|protocol| protocol.as_str().to_owned())
        .collect();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|_| TvmError::InvalidReceipt("libp2p transport build failed"))?
        .with_dns()
        .map_err(|_| TvmError::InvalidReceipt("libp2p dns transport build failed"))?
        .with_behaviour(|_| behaviour)
        .map_err(|_| TvmError::InvalidReceipt("libp2p behaviour build failed"))?
        .with_swarm_config(|swarm_config| {
            swarm_config.with_idle_connection_timeout(Duration::from_secs(
                config.idle_connection_timeout_seconds,
            ))
        })
        .build();

    for address in &config.listen_addresses {
        swarm
            .listen_on(parse_multiaddr(address)?)
            .map_err(|_| TvmError::InvalidReceipt("libp2p listen address rejected"))?;
    }
    for address in &config.bootstrap_addresses {
        let multiaddr = parse_multiaddr(address)?;
        if let Some((peer_id, peer_address)) = bootstrap_peer_address(&multiaddr) {
            swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, peer_address);
        }
        swarm
            .dial(multiaddr)
            .map_err(|_| TvmError::InvalidReceipt("libp2p bootstrap address rejected"))?;
    }

    Ok(TensorVmLibp2pNode {
        peer_id,
        swarm,
        identify_protocol,
        subscribed_topics,
        request_response_protocols,
    })
}

#[cfg(test)]
mod tests {
    use super::super::behaviour::TensorVmNetworkBehaviourEvent;
    use super::super::request_response::{
        P2pRequestResponseEvent, send_request_for_protocol, send_response_for_protocol,
    };
    use super::super::wire::{
        decode_message, encode_block_payload, encode_gossipsub_message, encode_tensor_payload,
        request_response_protocol_for_message,
    };
    use super::super::{GossipTopic, RequestResponseProtocol};
    use super::*;
    use crate::api::P2pMessage;
    use crate::chain::TensorBlock;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use futures::{FutureExt, StreamExt};
    use libp2p::multiaddr::Protocol;
    use libp2p::swarm::SwarmEvent;
    use libp2p::{Multiaddr, PeerId};

    #[test]
    fn libp2p_node_builds_real_swarm_and_protocol_behaviour() {
        let config = Libp2pControlPlaneConfig::default();
        let node = build_libp2p_node(&config).unwrap();
        assert!(!node.peer_id.to_string().is_empty());
        assert_eq!(node.subscribed_topics.len(), 5);
        assert!(
            node.subscribed_topics
                .contains(&"/tensorchain/1/blocks".to_owned())
        );
        assert_eq!(node.request_response_protocols.len(), 4);
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/chunk".to_owned())
        );
        assert!(
            node.request_response_protocols
                .contains(&"/tensorchain/1/tensor/by-root".to_owned())
        );
        assert_eq!(node.identify_protocol, "/tensorchain/1/identify");
    }

    #[test]
    fn libp2p_node_uses_configured_identity_seed() {
        let seed = hash_bytes(b"test", &[b"libp2p-identity-seed"]);
        let peer_a = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_b = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(seed),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;
        let peer_c = build_libp2p_node(&Libp2pControlPlaneConfig {
            identity_seed: Some(hash_bytes(b"test", &[b"other-libp2p-identity-seed"])),
            ..Libp2pControlPlaneConfig::default()
        })
        .unwrap()
        .peer_id;

        assert_eq!(peer_a, peer_b);
        assert_ne!(peer_a, peer_c);
    }

    #[test]
    fn libp2p_node_accepts_listen_and_bootstrap_multiaddrs() {
        let bootstrap_peer = PeerId::random();
        let bootstrap_address = format!("/ip4/127.0.0.1/tcp/4001/p2p/{bootstrap_peer}");
        let (discovered_peer, discovered_address) =
            bootstrap_peer_address(&bootstrap_address.parse().unwrap()).unwrap();
        assert_eq!(discovered_peer, bootstrap_peer);
        assert_eq!(discovered_address.to_string(), "/ip4/127.0.0.1/tcp/4001");
        let plain_address: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        assert_eq!(bootstrap_peer_address(&plain_address), None);
        let config = Libp2pControlPlaneConfig {
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
            bootstrap_addresses: vec![bootstrap_address],
            ..Libp2pControlPlaneConfig::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        runtime.block_on(async {
            let node = build_libp2p_node(&config).unwrap();
            assert!(!node.peer_id.to_string().is_empty());
        });
    }

    #[test]
    fn local_testnet_libp2p_swarms_exchange_gossip_and_request_response() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut producer = build_libp2p_node(&Libp2pControlPlaneConfig {
                listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
                ..Libp2pControlPlaneConfig::default()
            })
            .unwrap();
            let mut consumer = build_libp2p_node(&Libp2pControlPlaneConfig::default()).unwrap();
            let listen_addr = wait_for_listen_addr(&mut producer).await;
            let mut dial_addr = listen_addr;
            dial_addr.push(Protocol::P2p(producer.peer_id));
            consumer.swarm.dial(dial_addr).unwrap();

            wait_for_connection(&mut producer, &mut consumer).await;
            producer
                .swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&consumer.peer_id);
            consumer
                .swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&producer.peer_id);
            wait_for_gossip_subscriptions(
                &mut producer,
                consumer.peer_id,
                &[
                    GossipTopic::Blocks,
                    GossipTopic::Jobs,
                    GossipTopic::Receipts,
                    GossipTopic::Attestations,
                    GossipTopic::Peers,
                ],
            )
            .await;

            let gossip_messages = [
                P2pMessage::NewBlock(hash_bytes(b"test", &[b"gate-0-libp2p-block"])),
                P2pMessage::NewBlockHeader {
                    height: 3,
                    block_hash: hash_bytes(b"test", &[b"gate-0-libp2p-block-header"]),
                },
                {
                    let block = node_test_block(b"gate-0-libp2p-block-payload", 4);
                    P2pMessage::NewBlockPayload {
                        height: block.height,
                        block_hash: block.hash(),
                        payload: encode_block_payload(&block),
                    }
                },
                P2pMessage::NewJob(hash_bytes(b"test", &[b"gate-0-libp2p-job"])),
                P2pMessage::NewReceipt(hash_bytes(b"test", &[b"gate-0-libp2p-receipt"])),
                P2pMessage::NewAttestation(hash_bytes(b"test", &[b"gate-0-libp2p-attestation"])),
                P2pMessage::PeerInfo {
                    address: address(b"gate-0-libp2p-peer"),
                },
            ];
            for message in gossip_messages {
                let (topic, payload) = encode_gossipsub_message(&message).unwrap();
                producer
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, payload)
                    .unwrap();
                wait_for_gossip_message(&mut producer, &mut consumer, message).await;
            }

            let tensor_id = hash_bytes(b"test", &[b"gate-0-libp2p-tensor"]);
            let tensor = Tensor::from_vec(vec![1, 3], DType::FieldElement, vec![3, 5, 8]).unwrap();
            let commitment_root = tensor.commitment_root();
            let program_hash = hash_bytes(b"test", &[b"gate-0-libp2p-program"]);
            let request_response_messages = [
                (
                    P2pMessage::RequestTensorChunk {
                        tensor_id,
                        chunk_index: 1,
                    },
                    P2pMessage::TensorChunkResponse {
                        tensor_id,
                        chunk_index: 1,
                        bytes: vec![1, 1, 2, 3, 5, 8],
                    },
                ),
                (
                    P2pMessage::RequestTensorRow {
                        tensor_id,
                        row_index: 2,
                    },
                    P2pMessage::TensorRowResponse {
                        tensor_id,
                        row_index: 2,
                        values: vec![3, 5, 8],
                    },
                ),
                (
                    P2pMessage::RequestTensorByCommitmentRoot { commitment_root },
                    P2pMessage::TensorByCommitmentRootResponse {
                        commitment_root,
                        payload: Some(encode_tensor_payload(&tensor)),
                    },
                ),
                (
                    P2pMessage::RequestProgram(program_hash),
                    P2pMessage::ProgramResponse {
                        program_hash,
                        bytes: b"tensor-vm-gate-0-program".to_vec(),
                    },
                ),
            ];
            for (request, response) in request_response_messages {
                let protocol = request_response_protocol_for_message(&request).unwrap();
                let request_id = send_request_for_protocol(
                    &mut consumer.swarm,
                    protocol,
                    &producer.peer_id,
                    request.clone(),
                );
                wait_for_request_response(
                    &mut producer,
                    &mut consumer,
                    protocol,
                    &request,
                    &response,
                    request_id,
                )
                .await;
            }
        });
    }

    fn node_test_block(label: &[u8], height: u64) -> TensorBlock {
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
            beacon: hash_bytes(b"test-block", &[label, b"beacon"]),
            production_kind: crate::chain::BlockProductionKind::UsefulVerificationPow,
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

    async fn wait_for_listen_addr(node: &mut TensorVmLibp2pNode) -> Multiaddr {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let SwarmEvent::NewListenAddr { address, .. } =
                    node.swarm.select_next_some().await
                {
                    break address;
                }
            }
        })
        .await
        .expect("libp2p node must begin listening")
    }

    async fn wait_for_connection(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut producer_connected = false;
            let mut consumer_connected = false;
            while !(producer_connected && consumer_connected) {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    event = producer_event => {
                        if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                            producer_connected |= peer_id == consumer.peer_id;
                        }
                    }
                    event = consumer_event => {
                        if let SwarmEvent::ConnectionEstablished { peer_id, .. } = event {
                            consumer_connected |= peer_id == producer.peer_id;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p swarms must connect");
    }

    async fn wait_for_gossip_subscriptions(
        node: &mut TensorVmLibp2pNode,
        expected_peer: PeerId,
        expected_topics: &[GossipTopic],
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut seen_topics = Vec::new();
            loop {
                if let SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic },
                )) = node.swarm.select_next_some().await
                    && peer_id == expected_peer
                    && expected_topics
                        .iter()
                        .any(|expected| topic.to_string() == expected.as_str())
                    && !seen_topics.contains(&topic.to_string())
                {
                    seen_topics.push(topic.to_string());
                    if seen_topics.len() == expected_topics.len() {
                        break;
                    }
                }
            }
        })
        .await
        .expect("libp2p peer must advertise all TensorVM gossip subscriptions");
    }

    async fn wait_for_gossip_message(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
        expected: P2pMessage,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    _ = producer_event => {}
                    event = consumer_event => {
                        if let SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::Gossipsub(
                            libp2p::gossipsub::Event::Message {
                                propagation_source,
                                message,
                                ..
                            },
                        )) = event
                        {
                            assert_eq!(propagation_source, producer.peer_id);
                            assert_eq!(decode_message(&message.data).unwrap(), expected);
                            break;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p gossipsub message must be delivered");
    }

    async fn wait_for_request_response(
        producer: &mut TensorVmLibp2pNode,
        consumer: &mut TensorVmLibp2pNode,
        protocol: RequestResponseProtocol,
        expected_request: &P2pMessage,
        response: &P2pMessage,
        expected_request_id: libp2p::request_response::OutboundRequestId,
    ) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let producer_event = producer.swarm.select_next_some().fuse();
                let consumer_event = consumer.swarm.select_next_some().fuse();
                futures::pin_mut!(producer_event, consumer_event);
                futures::select! {
                    event = producer_event => {
                        if let Some(libp2p::request_response::Event::Message {
                                peer,
                                message:
                                    libp2p::request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    },
                            }) = request_response_event_for_protocol(event, protocol)
                        {
                            assert_eq!(peer, consumer.peer_id);
                            assert_eq!(&request, expected_request);
                            send_response_for_protocol(
                                &mut producer.swarm,
                                protocol,
                                channel,
                                response.clone(),
                            )
                                .unwrap();
                        }
                    }
                    event = consumer_event => {
                        if let Some(libp2p::request_response::Event::Message {
                                peer,
                                message:
                                    libp2p::request_response::Message::Response {
                                        request_id,
                                        response: actual_response,
                                    },
                            }) = request_response_event_for_protocol(event, protocol)
                        {
                            assert_eq!(peer, producer.peer_id);
                            assert_eq!(request_id, expected_request_id);
                            assert_eq!(&actual_response, response);
                            break;
                        }
                    }
                }
            }
        })
        .await
        .expect("libp2p request-response exchange must complete");
    }

    fn request_response_event_for_protocol(
        event: SwarmEvent<TensorVmNetworkBehaviourEvent>,
        protocol: RequestResponseProtocol,
    ) -> Option<P2pRequestResponseEvent> {
        match (protocol, event) {
            (
                RequestResponseProtocol::TensorChunk,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorChunkRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::TensorRow,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorRowRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::TensorByRoot,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::TensorByRootRequestResponse(
                    event,
                )),
            )
            | (
                RequestResponseProtocol::Program,
                SwarmEvent::Behaviour(TensorVmNetworkBehaviourEvent::ProgramRequestResponse(event)),
            ) => Some(event),
            _ => None,
        }
    }
}
