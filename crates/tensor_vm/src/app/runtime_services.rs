use super::{RuntimeP2pReport, ServiceRuntimeConfig, p2p_identity_report};
use crate::{
    Faucet, Libp2pControlPlaneConfig, NodeStore, RpcGateway, RpcHttpServer, RpcNode, RpcPolicy,
    Tensor, TensorVmLibp2pService, spawn_libp2p_service, storage::TensorArtifact, types::Hash,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct RuntimeServices {
    pub store: NodeStore,
    pub server: RpcHttpServer,
    pub p2p_service: TensorVmLibp2pService,
    pub p2p_metadata: RuntimeP2pMetadata,
}

pub struct RuntimeP2pMetadata {
    peer_id: String,
    topics: usize,
    request_response_protocols: usize,
    bootstrap_peer_count: usize,
    identity: String,
    max_transmit_bytes: usize,
    request_timeout_seconds: u64,
    max_concurrent_streams: usize,
    idle_timeout_seconds: u64,
}

impl RuntimeP2pMetadata {
    pub fn report(&self) -> RuntimeP2pReport<'_> {
        RuntimeP2pReport {
            peer_id: &self.peer_id,
            topics: self.topics,
            request_response_protocols: self.request_response_protocols,
            bootstrap_peer_count: self.bootstrap_peer_count,
            identity: &self.identity,
            max_transmit_bytes: self.max_transmit_bytes,
            request_timeout_seconds: self.request_timeout_seconds,
            max_concurrent_streams: self.max_concurrent_streams,
            idle_timeout_seconds: self.idle_timeout_seconds,
        }
    }
}

pub fn start_runtime_services(
    config: &ServiceRuntimeConfig,
) -> std::result::Result<RuntimeServices, String> {
    let network = &config.node.network;
    let store = NodeStore::open(config.node.data_dir());
    let chain = store.load_chain().map_err(|error| {
        format!(
            "failed to load node store {}: {error}",
            config.node.data_dir().display()
        )
    })?;
    let bootstrap_addresses =
        merged_bootstrap_addresses(&store, &config.node.network.bootstrap_addresses)?;
    let bootstrap_peer_count = bootstrap_addresses.len();
    let p2p_config = Libp2pControlPlaneConfig {
        listen_addresses: vec![network.p2p_listen.clone()],
        bootstrap_addresses,
        identity_seed: network.identity_seed,
        ..Libp2pControlPlaneConfig::default()
    };
    let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
    let request_timeout_seconds = p2p_config.request_timeout_seconds;
    let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
    let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
    let p2p_service = spawn_libp2p_service(p2p_config)
        .map_err(|error| format!("failed to start mandatory libp2p service: {error}"))?;
    hydrate_program_store(chain.state().program_bodies(), |graph_id, body| {
        p2p_service.register_program(graph_id, body)
    });
    let tensor_artifacts = store.load_tensors().map_err(|error| {
        format!(
            "failed to load tensor artifacts {}: {error}",
            config.node.data_dir().display()
        )
    })?;
    let p2p_info = p2p_service.info();
    let p2p_metadata = RuntimeP2pMetadata {
        peer_id: p2p_service.peer_id().to_string(),
        topics: p2p_info.subscribed_topics.len(),
        request_response_protocols: p2p_info.request_response_protocols.len(),
        bootstrap_peer_count,
        identity: p2p_identity_report(network.identity_seed),
        max_transmit_bytes,
        request_timeout_seconds,
        max_concurrent_streams,
        idle_timeout_seconds,
    };
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    hydrate_tensor_artifacts(&mut node, &tensor_artifacts, |tensor| {
        p2p_service.register_tensor(tensor)
    });
    let gateway = RpcGateway::new(
        node,
        RpcPolicy {
            auth_token: Some(network.auth_token.clone()),
            ..RpcPolicy::default()
        },
    );
    let server = RpcHttpServer::bind(&network.rpc_listen, gateway).map_err(|error| {
        format!(
            "failed to bind service listener {}: {error}",
            network.rpc_listen
        )
    })?;
    Ok(RuntimeServices {
        store,
        server,
        p2p_service,
        p2p_metadata,
    })
}

pub(super) fn merged_bootstrap_addresses(
    store: &NodeStore,
    configured_addresses: &[String],
) -> std::result::Result<Vec<String>, String> {
    let mut seen = BTreeSet::new();
    let mut addresses = Vec::new();
    for address in configured_addresses {
        if seen.insert(address.clone()) {
            addresses.push(address.clone());
        }
    }
    if store.peer_book_store().path().exists() {
        for address in store
            .peer_book_store()
            .load_bootstrap_addresses()
            .map_err(|error| {
                format!(
                    "failed to load libp2p peer book {}: {error}",
                    store.data_dir().display()
                )
            })?
        {
            if seen.insert(address.clone()) {
                addresses.push(address);
            }
        }
    }
    Ok(addresses)
}

fn hydrate_program_store(
    program_bodies: &BTreeMap<Hash, Vec<u8>>,
    mut register: impl FnMut(Hash, Vec<u8>),
) {
    for (graph_id, body) in program_bodies {
        register(*graph_id, body.clone());
    }
}

fn hydrate_tensor_artifacts(
    node: &mut RpcNode,
    artifacts: &[TensorArtifact],
    mut register: impl FnMut(Tensor),
) {
    for artifact in artifacts {
        node.insert_tensor(artifact.tensor.clone());
        register(artifact.tensor.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chain, DType, Faucet, Tensor, canonical_matmul_graph, types::hash_bytes};

    #[test]
    fn startup_program_hydration_registers_state_rooted_program_bodies() {
        let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
        let graph_id = graph.graph_id();
        let graph_body = graph.canonical_json().into_bytes();
        let mut program_bodies = BTreeMap::new();
        program_bodies.insert(graph_id, graph_body.clone());
        let mut registered = BTreeMap::new();

        hydrate_program_store(&program_bodies, |graph_id, body| {
            registered.insert(graph_id, body);
        });

        assert_eq!(registered.get(&graph_id), Some(&graph_body));
    }

    #[test]
    fn startup_tensor_hydration_registers_rpc_and_p2p_artifacts() {
        let tensor = Tensor::from_vec_with_scale(vec![2], DType::Fixed32, 3, vec![11, 12]).unwrap();
        let root = tensor.commitment_root();
        let artifact = TensorArtifact {
            tensor: tensor.clone(),
            retain_until_block: 99,
        };
        let mut node = RpcNode::with_faucet(
            Chain::new(hash_bytes(b"test", &[b"startup-tensor-hydration"])),
            Faucet::new(1_000_000, 100),
        );
        let mut registered = Vec::new();

        hydrate_tensor_artifacts(&mut node, &[artifact], |tensor| registered.push(tensor));

        assert_eq!(
            node.tensor_by_commitment_root(&root).map(Tensor::scale),
            Some(3)
        );
        assert_eq!(registered, vec![tensor]);
    }
}
