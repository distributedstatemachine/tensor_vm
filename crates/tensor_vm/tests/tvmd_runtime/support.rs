use super::*;

pub(super) fn assert_tensor_count(node: &RpcNode, expected: usize) {
    let response = node.handle(&tensor_vm::RpcRequest {
        method: "GET".to_owned(),
        path: "/tensor/latest".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(response.status, 200);
    let body: serde_json::Value =
        serde_json::from_str(&response.body).expect("tensor latest response body must be JSON");
    assert_eq!(body["tensor_count"].as_u64(), Some(expected as u64));
}

pub(super) fn insert_bundle_tensors(node: &mut RpcNode, bundle: &RoleReceiptBundle) {
    for tensor in bundle.served_tensors() {
        node.insert_tensor(tensor);
    }
}

pub(super) fn report_field<'a>(report: &'a str, key: &str) -> &'a str {
    super::report_fields::report_value(report, key)
}

pub(super) fn report_u64(report: &str, key: &str) -> u64 {
    super::report_fields::report_u64(report, key)
}

pub(super) fn http_status_line(response: &str) -> &str {
    response
        .lines()
        .next()
        .expect("HTTP response must include status line")
}

pub(super) fn register_miner(chain: &mut Chain, miner: tensor_vm::Address) {
    let stake = chain.params().miner_min_stake;
    chain
        .apply_command(ChainCommand::RegisterMiner {
            address: miner,
            stake,
        })
        .unwrap();
}

pub(super) fn register_validator(chain: &mut Chain, validator: tensor_vm::Address) {
    let stake = chain.params().validator_min_stake;
    chain
        .apply_command(ChainCommand::RegisterValidator {
            address: validator,
            stake,
        })
        .unwrap();
}

pub(super) fn produce_block(
    chain: &mut Chain,
    proposer: tensor_vm::Address,
    timestamp: u64,
) -> tensor_vm::chain::TensorBlock {
    let block_count = chain.blocks().len();
    let proposer = chain
        .proposer_for_next_epoch(&chain.state().finalized_randomness())
        .unwrap_or(proposer);
    let timestamp = chain
        .blocks()
        .last()
        .map(|parent| {
            timestamp.max(
                parent.timestamp.saturating_add(
                    chain
                        .params()
                        .pow_timeout_blocks
                        .max(1)
                        .saturating_mul(chain.params().block_time_seconds.max(1)),
                ),
            )
        })
        .unwrap_or(timestamp);
    chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer,
            timestamp,
        })
        .unwrap();
    assert_eq!(chain.blocks().len(), block_count + 1);
    chain.blocks().last().unwrap().clone()
}

pub(super) fn unique_temp_data_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("tensor-vm-{name}-{}-{now}", std::process::id()))
}

pub(super) fn test_service_runtime_config(
    data_dir: &Path,
    auth_token: &str,
) -> ServiceRuntimeConfig {
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    ServiceRuntimeConfig {
        runtime_command: "service_serve",
        role: RuntimeRole::Service,
        role_wallet_address: None,
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Service,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[data_dir_text.as_bytes()])),
            auth_token,
            0,
        )
        .unwrap(),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    }
}

pub(super) fn file_modified_at(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

pub(super) fn send_http_request(addr: std::net::SocketAddr, request: &str) -> String {
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    response
}

pub(super) fn free_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub(super) fn wait_for_connected_role_services(
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
