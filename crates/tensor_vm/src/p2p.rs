mod behaviour;
mod node;
mod peer_book;
mod request_response;
mod service;
mod service_events;
mod wire;

pub use behaviour::TensorVmNetworkBehaviour;
pub use node::{TensorVmLibp2pNode, build_libp2p_node};
pub use peer_book::{PeerBookStore, PeerRecord, normalize_bootstrap_multiaddr};
pub use request_response::P2pRequestResponseBehaviour;
pub use service::{TensorVmLibp2pService, TensorVmLibp2pServiceInfo, spawn_libp2p_service};
pub use wire::{
    decode_attestation_payload, decode_block_check_challenge_payload, decode_block_payload,
    decode_block_payload_with_selected_receipts, decode_block_vote_payload,
    decode_external_randomness_beacon_payload, decode_job_payload, decode_message,
    decode_receipt_payload, decode_tensor_payload, decode_trace_bisection_expectation_payload,
    decode_trace_bisection_open_payload, decode_trace_bisection_referee_payload,
    decode_trace_bisection_round_payload, decode_trace_opening_payload,
    decode_validator_audit_report_payload, decode_validator_vrf_reveal_payload,
    decode_verified_chained_drand_beacon_payload, decode_verified_drand_beacon_payload,
    encode_attestation_payload, encode_block_check_challenge_payload, encode_block_payload,
    encode_block_payload_with_selected_receipts, encode_block_vote_payload,
    encode_external_randomness_beacon_payload, encode_gossipsub_message, encode_job_payload,
    encode_message, encode_receipt_payload, encode_tensor_payload,
    encode_trace_bisection_expectation_payload, encode_trace_bisection_open_payload,
    encode_trace_bisection_referee_payload, encode_trace_bisection_round_payload,
    encode_trace_opening_payload, encode_validator_audit_report_payload,
    encode_validator_vrf_reveal_payload, encode_verified_chained_drand_beacon_payload,
    encode_verified_drand_beacon_payload, gossip_topic_for_message, gossipsub_ident_topic,
    request_response_protocol_for_message, request_response_stream_protocol,
};

pub const LIBP2P_PROTOCOL_PREFIX: &str = "/tensorchain/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkStackRecommendation {
    pub libp2p_required: bool,
    pub consensus_transport: &'static str,
    pub tensor_fetch_transport: &'static str,
    pub rationale: Vec<&'static str>,
}

pub fn recommended_network_stack() -> NetworkStackRecommendation {
    NetworkStackRecommendation {
        libp2p_required: true,
        consensus_transport: "rust-libp2p gossipsub/identify/kademlia",
        tensor_fetch_transport: "rust-libp2p request-response",
        rationale: vec![
            "rust-libp2p is the mandatory TensorVM P2P runtime dependency",
            "gossipsub carries block, job, receipt, attestation, and peer announcements",
            "identify advertises TensorVM protocol support to connected peers",
            "request-response streams carry tensor roots, rows, chunks, trace openings, and program fetches",
            "the TensorVM MVP uses libp2p for both consensus propagation and bounded tensor/program/trace fetches",
        ],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GossipTopic {
    Blocks,
    Jobs,
    Receipts,
    Attestations,
    Peers,
}

impl GossipTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "/tensorchain/1/blocks",
            Self::Jobs => "/tensorchain/1/jobs",
            Self::Receipts => "/tensorchain/1/receipts",
            Self::Attestations => "/tensorchain/1/attestations",
            Self::Peers => "/tensorchain/1/peers",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RequestResponseProtocol {
    TensorChunk,
    TensorRow,
    TensorByRoot,
    Program,
    TraceOpening,
}

impl RequestResponseProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TensorChunk => "/tensorchain/1/tensor/chunk",
            Self::TensorRow => "/tensorchain/1/tensor/row",
            Self::TensorByRoot => "/tensorchain/1/tensor/by-root",
            Self::Program => "/tensorchain/1/program",
            Self::TraceOpening => "/tensorchain/1/trace/opening",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Libp2pControlPlaneConfig {
    pub gossipsub_topics: Vec<GossipTopic>,
    pub request_response_protocols: Vec<RequestResponseProtocol>,
    pub listen_addresses: Vec<String>,
    pub bootstrap_addresses: Vec<String>,
    pub identity_seed: Option<[u8; 32]>,
    pub max_gossipsub_transmit_bytes: usize,
    pub request_timeout_seconds: u64,
    pub max_concurrent_request_streams: usize,
    pub idle_connection_timeout_seconds: u64,
}

impl Default for Libp2pControlPlaneConfig {
    fn default() -> Self {
        Self {
            gossipsub_topics: vec![
                GossipTopic::Blocks,
                GossipTopic::Jobs,
                GossipTopic::Receipts,
                GossipTopic::Attestations,
                GossipTopic::Peers,
            ],
            request_response_protocols: vec![
                RequestResponseProtocol::TensorChunk,
                RequestResponseProtocol::TensorRow,
                RequestResponseProtocol::TensorByRoot,
                RequestResponseProtocol::Program,
                RequestResponseProtocol::TraceOpening,
            ],
            listen_addresses: Vec::new(),
            bootstrap_addresses: Vec::new(),
            identity_seed: None,
            max_gossipsub_transmit_bytes: 1024 * 1024,
            request_timeout_seconds: 10,
            max_concurrent_request_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    }
}
