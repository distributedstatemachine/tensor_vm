use crate::error::{Result as TvmResult, TvmError};
use libp2p::PeerId;

use super::request_response::{P2pRequestResponseBehaviour, build_request_response_behaviour};
use super::wire::gossipsub_ident_topic;
use super::{LIBP2P_PROTOCOL_PREFIX, Libp2pControlPlaneConfig, RequestResponseProtocol};

#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct TensorVmNetworkBehaviour {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub identify: libp2p::identify::Behaviour,
    pub kademlia: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub tensor_chunk_request_response: P2pRequestResponseBehaviour,
    pub tensor_row_request_response: P2pRequestResponseBehaviour,
    pub tensor_by_root_request_response: P2pRequestResponseBehaviour,
    pub program_request_response: P2pRequestResponseBehaviour,
    pub trace_opening_request_response: P2pRequestResponseBehaviour,
}

pub(super) fn build_libp2p_behaviour(
    config: &Libp2pControlPlaneConfig,
    keypair: &libp2p::identity::Keypair,
) -> TvmResult<TensorVmNetworkBehaviour> {
    let mut gossipsub_config = libp2p::gossipsub::ConfigBuilder::default();
    gossipsub_config
        .max_transmit_size(config.max_gossipsub_transmit_bytes)
        .validation_mode(libp2p::gossipsub::ValidationMode::Strict);
    let mut gossipsub = libp2p::gossipsub::Behaviour::new(
        libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config
            .build()
            .map_err(|_| TvmError::InvalidReceipt("invalid gossipsub configuration"))?,
    )
    .map_err(|_| TvmError::InvalidReceipt("gossipsub build failed"))?;
    for topic in &config.gossipsub_topics {
        let ident_topic = gossipsub_ident_topic(*topic);
        gossipsub
            .subscribe(&ident_topic)
            .map_err(|_| TvmError::InvalidReceipt("gossipsub subscription failed"))?;
    }

    let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
        format!("{LIBP2P_PROTOCOL_PREFIX}/identify"),
        keypair.public(),
    ));
    let local_peer_id = PeerId::from(keypair.public());
    let kademlia_store = libp2p::kad::store::MemoryStore::new(local_peer_id);
    let kademlia = libp2p::kad::Behaviour::new(local_peer_id, kademlia_store);
    Ok(TensorVmNetworkBehaviour {
        gossipsub,
        identify,
        kademlia,
        tensor_chunk_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorChunk,
        )?,
        tensor_row_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorRow,
        )?,
        tensor_by_root_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TensorByRoot,
        )?,
        program_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::Program,
        )?,
        trace_opening_request_response: build_request_response_behaviour(
            config,
            RequestResponseProtocol::TraceOpening,
        )?,
    })
}
