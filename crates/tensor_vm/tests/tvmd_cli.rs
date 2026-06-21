use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use libp2p::PeerId;
use tensor_vm::hash::hex;
use tensor_vm::types::address;

#[path = "support/comma_records.rs"]
mod comma_records;
#[path = "support/report_fields.rs"]
mod report_fields;
use comma_records::{comma_record_fields, network_observation_root};
use report_fields::{
    report_u64 as stdout_u64, report_value as stdout_value, report_values as stdout_values,
};

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_tvmd(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd command must execute");

    assert!(
        output.status.success(),
        "tvmd failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("tvmd stdout must be utf8")
}

fn run_tvmd_failure(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd command must execute");

    assert!(
        !output.status.success(),
        "tvmd unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (
        output.status.code().unwrap_or_default(),
        String::from_utf8(output.stdout).expect("tvmd stdout must be utf8"),
        String::from_utf8(output.stderr).expect("tvmd stderr must be utf8"),
    )
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tensor-vm-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("test dir must be created");
    dir
}

fn free_local_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("local ephemeral port must bind")
        .local_addr()
        .expect("local addr must be available")
        .port()
}

fn service_request(
    port: u16,
    method: &str,
    path: &str,
    body: &str,
    auth_token: Option<&str>,
) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("read timeout must be set");
                let auth_header = auth_token
                    .map(|token| format!("x-tensorchain-auth: {token}\r\n"))
                    .unwrap_or_default();
                let request = format!(
                    "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth_header}content-length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(request.as_bytes())
                    .expect("service request must write");
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .expect("service response must read");
                return response;
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => panic!("service did not accept request: {error}"),
        }
    }
}

fn authenticated_request(port: u16, method: &str, path: &str, body: &str) -> String {
    service_request(port, method, path, body, Some("service-token"))
}

fn authenticated_get_request(port: u16, path: &str) -> String {
    authenticated_request(port, "GET", path, "")
}

fn unauthenticated_get_request(port: u16, path: &str) -> String {
    service_request(port, "GET", path, "", None)
}

fn response_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response must contain a body separator")
}

fn response_status_line(response: &str) -> &str {
    response
        .lines()
        .next()
        .expect("HTTP response must include status line")
}

fn response_json(response: &str) -> serde_json::Value {
    serde_json::from_str(response_body(response)).expect("HTTP response body must be JSON")
}

fn response_json_with_status(response: &str, status: &str) -> serde_json::Value {
    assert_eq!(response_status_line(response), status);
    response_json(response)
}

fn html_tag_text<'a>(html: &'a str, tag: &str) -> &'a str {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    html.split_once(&open)
        .and_then(|(_, tail)| tail.split_once(&close))
        .map(|(value, _)| value)
        .unwrap_or_else(|| panic!("HTML document must contain <{tag}> text"))
}

fn json_u64(json: &serde_json::Value, key: &str) -> u64 {
    json[key]
        .as_u64()
        .unwrap_or_else(|| panic!("JSON field {key} must be an unsigned integer"))
}

fn json_positive_field_count(json: &serde_json::Value, key: &str) -> usize {
    match json {
        serde_json::Value::Object(fields) => {
            let current = fields
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|value| value > 0) as usize;
            current
                + fields
                    .values()
                    .map(|value| json_positive_field_count(value, key))
                    .sum::<usize>()
        }
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| json_positive_field_count(value, key))
            .sum(),
        _ => 0,
    }
}

fn stdout_hex_hash<'a>(stdout: &'a str, key: &str) -> &'a str {
    let value = stdout_value(stdout, key);
    assert_eq!(value.len(), 64, "expected {key} to be a 32-byte hex hash");
    assert!(
        value.chars().all(|character| character.is_ascii_hexdigit()),
        "expected {key} to be hex"
    );
    value
}

fn trimmed_tvmd(args: &[&str]) -> String {
    run_tvmd(args).trim_end().to_owned()
}

fn assert_service_health_evidence_from_response(
    kind: &str,
    endpoint_id: &str,
    public_url: &str,
    response: &str,
) {
    let body = response_json_with_status(response, "HTTP/1.1 200 OK");
    assert_eq!(body["status"].as_str(), Some("ok"));
    assert_eq!(body["service"].as_str(), Some(kind));
    let health = run_tvmd(&[
        "public",
        "evidence",
        "service",
        "health",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--health-path",
        "/health",
        "--first-block",
        "0",
        "--last-block",
        "9",
        "--reachable-count",
        "10",
        "--signed-health-check-count",
        "10",
    ]);
    let fields = comma_record_fields(&health, "service=", 9);
    assert_eq!(
        fields[..8],
        [
            kind,
            endpoint_id,
            public_url,
            "/health",
            "0",
            "9",
            "10",
            "10"
        ]
    );
    assert_eq!(fields[8].len(), 64);
}

fn assert_service_content_evidence_from_response(
    data_dir: &Path,
    kind: &str,
    endpoint_id: &str,
    public_url: &str,
    content_path: &str,
    file_name: &str,
    response: &str,
) {
    let body = response_body(response);
    assert!(
        body.len() >= 64,
        "{content_path} body must satisfy service-content byte minimum"
    );
    let body_hex = hex(body.as_bytes());
    let content_from_bytes = run_tvmd(&[
        "public",
        "evidence",
        "service",
        "content-bytes",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--content-path",
        content_path,
        "--observed-at",
        "1700000000",
        "--content-hex",
        &body_hex,
    ]);
    let min_content_bytes = body.len().to_string();
    let fields = comma_record_fields(&content_from_bytes, "service_content=", 8);
    assert_eq!(fields[..4], [kind, endpoint_id, public_url, content_path]);
    assert_eq!(fields[4].len(), 64);
    assert_eq!(fields[5..7], ["1700000000", min_content_bytes.as_str()]);
    assert_eq!(fields[7].len(), 64);

    let content_file = data_dir.join(file_name);
    std::fs::write(&content_file, body.as_bytes()).expect("service body fixture must be written");
    let content_file_text = content_file.to_string_lossy().into_owned();
    let content_from_file = run_tvmd(&[
        "public",
        "evidence",
        "service",
        "content-file",
        "--kind",
        kind,
        "--endpoint-id",
        endpoint_id,
        "--public-url",
        public_url,
        "--content-path",
        content_path,
        "--observed-at",
        "1700000000",
        "--content-file",
        &content_file_text,
    ]);
    assert_eq!(content_from_file, content_from_bytes);
}

#[path = "tvmd_cli/public_evidence.rs"]
mod public_evidence;
#[path = "tvmd_cli/service_lifecycle.rs"]
mod service_lifecycle;

#[test]
fn local_testnet_service_gateway_does_not_produce_local_blocks() {
    let data_dir = unique_test_dir("local-testnet-seed");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let seed = run_tvmd(&["localnet", "seed", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&seed, "command"), "local_testnet_seed");
    assert_eq!(stdout_u64(&seed, "miners"), 10);
    assert_eq!(stdout_u64(&seed, "validators"), 5);
    assert_eq!(stdout_u64(&seed, "height"), 2);
    assert_eq!(stdout_u64(&seed, "blocks"), 2);
    assert_eq!(stdout_value(&seed, "matmul_settled"), "true");
    assert_eq!(stdout_value(&seed, "linear_training_settled"), "true");
    assert!(stdout_u64(&seed, "rewarded_miners") > 0);
    assert!(stdout_u64(&seed, "pending_receipt_rewards") > 0);
    assert_eq!(stdout_u64(&seed, "total_reward_balance"), 0);
    assert!(stdout_u64(&seed, "attestation_count") > 0);
    assert_eq!(stdout_u64(&seed, "data_availability_bps"), 10_000);
    assert_eq!(stdout_value(&seed, "node_store_ready"), "true");
    assert_eq!(stdout_u64(&seed, "persisted_block_count"), 2);
    assert_eq!(stdout_value(&seed, "public_evidence_full_spec"), "false");
    assert_eq!(stdout_value(&seed, "independently_checkable"), "false");

    let verify = run_tvmd(&["localnet", "verify", "--data-dir", &data_dir_text, "--json"]);
    let verify: serde_json::Value =
        serde_json::from_str(verify.trim()).expect("local CPU verify output must be JSON");
    assert_eq!(verify["command"], "local_cpu_verify");
    assert_eq!(verify["data_dir"], data_dir_text);
    assert_eq!(verify["structured_verifier_ready"], true);
    assert_eq!(verify["ready"], true);
    assert_eq!(verify["height"], 2);
    assert_eq!(verify["latest_block_height"], 1);
    assert_eq!(verify["block_count"], 2);
    assert_eq!(verify["finalized_block_count"], 2);
    assert_eq!(verify["node_store_ready"], true);

    let rpc_port = free_local_port();
    let listen = format!("127.0.0.1:{rpc_port}");
    let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "node",
            "serve",
            "--listen",
            &listen,
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
            "--auth-token",
            "service-token",
            "--max-requests",
            "4",
        ])
        .env("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25")
        .env("TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER", "true")
        .env("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER", "true")
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd node serve must spawn");

    let initial_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert_eq!(response_status_line(&initial_chain_head), "HTTP/1.1 200 OK");
    let initial_chain_head = response_json(&initial_chain_head);
    let initial_height = json_u64(&initial_chain_head, "height");
    let initial_block_count = json_u64(&initial_chain_head, "block_count");
    assert!(initial_height >= 2);
    assert!(initial_block_count >= 2);

    std::thread::sleep(Duration::from_millis(150));

    let overview = authenticated_get_request(rpc_port, "/explorer/overview");
    assert_eq!(response_status_line(&overview), "HTTP/1.1 200 OK");
    let overview = response_json(&overview);
    let summary = &overview["summary"];
    assert!(json_u64(summary, "job_count") >= 2);
    assert!(json_u64(summary, "receipt_count") >= 10);
    assert!(json_u64(summary, "settled_receipt_count") >= 10);

    let receipts = authenticated_get_request(rpc_port, "/explorer/receipts/latest/500");
    assert_eq!(response_status_line(&receipts), "HTTP/1.1 200 OK");
    let receipts = response_json(&receipts);
    let receipts_array = receipts["receipts"]
        .as_array()
        .expect("latest receipts response must be a JSON array");
    assert!(receipts_array.iter().all(|receipt| {
        receipt
            .get("validator_attestations")
            .is_some_and(serde_json::Value::is_array)
    }));
    assert!(json_positive_field_count(&receipts, "attestation_count") >= 10);

    let later_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert_eq!(response_status_line(&later_chain_head), "HTTP/1.1 200 OK");
    let later_chain_head = response_json(&later_chain_head);
    assert_eq!(json_u64(&later_chain_head, "height"), initial_height);
    assert_eq!(
        json_u64(&later_chain_head, "block_count"),
        initial_block_count
    );

    let output = child.wait_with_output().expect("service process must exit");
    assert!(
        output.status.success(),
        "service serve failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("service stdout must be utf8");
    assert_eq!(stdout_value(&stdout, "command"), "service_serve");
    assert_eq!(stdout_value(&stdout, "chain_profile"), "local_cpu");
    assert_eq!(stdout_value(&stdout, "role_can_produce_blocks"), "false");
    assert_eq!(stdout_value(&stdout, "local_producer"), "false");
    assert_eq!(stdout_value(&stdout, "local_block_proposer"), "false");
    assert_eq!(stdout_u64(&stdout, "local_block_proposer_delay_blocks"), 0);
    assert_eq!(
        stdout_value(&stdout, "local_block_proposer_delay_satisfied"),
        "true"
    );
    assert_eq!(stdout_u64(&stdout, "served_requests"), 4);
    assert_eq!(stdout_value(&stdout, "produced_blocks"), "0");

    let status = run_tvmd(&["node", "status", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&status, "command"), "service_status");
    assert_eq!(stdout_value(&status, "node_store_ready"), "true");
    assert_eq!(stdout_value(&status, "status_source"), "node_store");
    assert_eq!(stdout_value(&status, "operator_name"), "unknown");
    assert_eq!(stdout_value(&status, "role"), "unknown");
    assert_eq!(stdout_value(&status, "role_chain_profile"), "local_cpu");
    assert_eq!(stdout_value(&status, "role_can_produce_blocks"), "false");
    assert_eq!(stdout_value(&status, "role_local_producer"), "false");
    assert_eq!(stdout_value(&status, "role_local_block_proposer"), "false");
    assert_eq!(
        stdout_u64(&status, "role_local_block_proposer_delay_blocks"),
        0
    );
    assert_eq!(
        stdout_value(&status, "role_local_block_proposer_delay_satisfied"),
        "true"
    );
    assert_eq!(stdout_value(&status, "role_produced_blocks"), "0");
    assert_eq!(stdout_value(&status, "registered_miner_count"), "10");
    assert_eq!(stdout_value(&status, "registered_validator_count"), "5");
    assert!(
        stdout_value(&status, "job_count")
            .parse::<u64>()
            .expect("service status job count must parse")
            >= 2
    );
    assert!(
        stdout_value(&status, "receipt_count")
            .parse::<u64>()
            .expect("service status receipt count must parse")
            >= 10
    );
    assert!(
        stdout_value(&status, "attestation_count")
            .parse::<u64>()
            .expect("service status attestation count must parse")
            >= 10
    );
    assert_eq!(
        stdout_value(&status, "height")
            .parse::<u64>()
            .expect("service status height must parse"),
        initial_height
    );
    assert_eq!(
        stdout_value(&status, "block_count")
            .parse::<u64>()
            .expect("service status block count must parse"),
        initial_block_count
    );
    let latest_block_height = stdout_value(&status, "latest_block_height")
        .parse::<u64>()
        .expect("service status latest block height must parse");
    assert!(latest_block_height >= 1);
    let latest_block_height_text = latest_block_height.to_string();
    assert_ne!(stdout_value(&status, "block_log_root"), "0".repeat(64));
    assert!(
        stdout_value(&status, "finalized_block_count")
            .parse::<u64>()
            .expect("service status finalized block count must parse")
            >= 2
    );
    assert_eq!(stdout_value(&status, "first_live_block_height"), "0");
    let first_live_block_hash = stdout_value(&status, "first_live_block_hash");
    assert_eq!(first_live_block_hash, "0".repeat(64));

    let block = run_tvmd(&[
        "node",
        "block",
        "--data-dir",
        &data_dir_text,
        "--height",
        &latest_block_height_text,
    ]);
    assert_eq!(stdout_value(&block, "command"), "service_block");
    assert_eq!(stdout_value(&block, "height"), latest_block_height_text);
    assert_eq!(
        stdout_value(&block, "block_validation"),
        "useful_verification_pow"
    );
    assert_eq!(stdout_value(&block, "proposer_role"), "validator");
    assert_eq!(stdout_value(&block, "proposer_registered"), "true");
    assert_eq!(
        stdout_value(&block, "tensorwork_proposer_selection"),
        "false"
    );
    stdout_hex_hash(&block, "settled_receipt_set_root");
    stdout_hex_hash(&block, "checks_root");
    stdout_hex_hash(&block, "parent_snapshot_root");
    stdout_u64(&block, "parent_beacon_round");
    stdout_hex_hash(&block, "parent_beacon");
    stdout_hex_hash(&block, "parent_settled_receipt_pool_root");
    stdout_hex_hash(&block, "parent_included_receipt_root");
    stdout_hex_hash(&block, "parent_data_unavailable_receipt_root");
    stdout_hex_hash(&block, "child_state_root");
    stdout_hex_hash(&block, "child_reward_root");
    stdout_u64(&block, "child_height");
    stdout_u64(&block, "child_epoch");
    stdout_u64(&block, "child_beacon_round");
    stdout_hex_hash(&block, "child_beacon");
    stdout_u64(&block, "beacon_round");
    stdout_hex_hash(&block, "beacon");
    assert_eq!(
        stdout_value(&block, "block_uses_parent_finalized_beacon"),
        "true"
    );
    assert_eq!(stdout_value(&block, "checks_root_recomputed"), "true");
    assert_eq!(
        stdout_value(&block, "selected_receipt_root_recomputed"),
        "true"
    );
    assert_eq!(stdout_value(&block, "checks_root_openable"), "true");
    assert_eq!(stdout_value(&block, "child_state_root_recomputed"), "true");
    assert_eq!(stdout_value(&block, "child_reward_root_recomputed"), "true");
    stdout_hex_hash(&block, "difficulty_target");
    stdout_u64(&block, "nonce");
    stdout_hex_hash(&block, "pow_header_hash");
    stdout_hex_hash(&block, "pow_hash");
    if stdout_value(&block, "pow_required") == "true" {
        assert_eq!(stdout_value(&block, "pow_valid"), "true");
        assert_eq!(
            stdout_value(&block, "block_kind"),
            "useful_verification_pow"
        );
    } else {
        assert_eq!(stdout_value(&block, "block_kind"), "pow_skip_fallback");
        assert_eq!(stdout_value(&block, "pow_skip_fallback"), "true");
        assert_eq!(stdout_value(&block, "fallback_valid"), "true");
    }
    assert_ne!(stdout_hex_hash(&block, "state_root"), "0".repeat(64));
    assert_eq!(stdout_value(&block, "finalized"), "true");
    assert!(stdout_u64(&block, "block_vote_count") > 0);
    assert_ne!(stdout_value(&block, "block_vote_validators"), "none");
    assert_eq!(stdout_value(&block, "finality_validated_block"), "true");
    let receipt_count = stdout_u64(&block, "receipt_count");
    assert!(receipt_count > 0);
    assert_ne!(stdout_value(&block, "receipt_ids"), "none");
    let selected_receipt_count = stdout_u64(&block, "selected_receipt_count");
    assert_eq!(
        stdout_u64(&block, "selected_receipt_opening_count"),
        selected_receipt_count
    );
    assert_eq!(
        stdout_u64(&block, "checks_opening_count"),
        selected_receipt_count
    );
    assert_ne!(stdout_value(&block, "selected_receipt_leaf_ids"), "none");
    assert_ne!(stdout_value(&block, "selected_receipt_leaf_roots"), "none");
    assert_ne!(stdout_value(&block, "checks_leaf_roots"), "none");
    assert!(stdout_u64(&block, "settled_receipt_count") > 0);
    assert_eq!(
        stdout_u64(&block, "tensor_op_receipt_count")
            + stdout_u64(&block, "linear_training_receipt_count"),
        receipt_count
    );
    assert!(stdout_u64(&block, "latest_height") >= 1);

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_run_with_synthetic_job_producer_publishes_jobs_without_empty_fallback_blocks() {
    let data_dir = unique_test_dir("validator-local-producer");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let seed = run_tvmd(&["localnet", "seed", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&seed, "command"), "local_testnet_seed");

    let rpc_port = free_local_port();
    let listen = format!("127.0.0.1:{rpc_port}");
    let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "validator",
            "run",
            "--wallet",
            "testnet-validator-0",
            "--node",
            "/ip4/127.0.0.1/tcp/4002",
            "--listen",
            &listen,
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
            "--auth-token",
            "service-token",
            "--max-requests",
            "3",
        ])
        .env("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25")
        .env("TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER", "true")
        .env("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER", "true")
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd validator run must spawn");

    let initial_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert_eq!(response_status_line(&initial_chain_head), "HTTP/1.1 200 OK");
    let initial_chain_head = response_json(&initial_chain_head);
    let initial_height = json_u64(&initial_chain_head, "height");
    let initial_block_count = json_u64(&initial_chain_head, "block_count");
    assert!(initial_height >= 2);
    assert!(initial_block_count >= 2);

    std::thread::sleep(Duration::from_millis(150));

    let overview = authenticated_get_request(rpc_port, "/explorer/overview");
    assert_eq!(response_status_line(&overview), "HTTP/1.1 200 OK");
    let overview = response_json(&overview);
    assert!(json_u64(&overview["summary"], "job_count") > 2);
    let later_chain_head = authenticated_get_request(rpc_port, "/chain/head");
    assert_eq!(response_status_line(&later_chain_head), "HTTP/1.1 200 OK");
    let later_chain_head = response_json(&later_chain_head);
    assert_eq!(json_u64(&later_chain_head, "height"), initial_height);
    assert_eq!(
        json_u64(&later_chain_head, "block_count"),
        initial_block_count
    );

    let output = child
        .wait_with_output()
        .expect("validator process must exit");
    assert!(
        output.status.success(),
        "validator run failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("validator stdout must be utf8");
    assert_eq!(stdout_value(&stdout, "command"), "validator_run");
    assert_eq!(stdout_value(&stdout, "role"), "validator");
    assert_eq!(stdout_value(&stdout, "runtime_command"), "validator_run");
    assert_eq!(stdout_value(&stdout, "role_can_produce_blocks"), "true");
    assert_eq!(
        stdout_value(&stdout, "role_wallet_registration"),
        "validator"
    );
    assert_eq!(stdout_value(&stdout, "role_wallet_registered"), "true");
    assert_eq!(stdout_value(&stdout, "local_producer"), "true");
    assert_eq!(stdout_value(&stdout, "local_block_proposer"), "true");
    assert_eq!(stdout_u64(&stdout, "local_block_proposer_delay_blocks"), 0);
    assert_eq!(
        stdout_value(&stdout, "local_block_proposer_delay_satisfied"),
        "true"
    );
    assert_eq!(stdout_u64(&stdout, "produced_blocks"), 0);

    let status = run_tvmd(&["node", "status", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&status, "role_loop_role"), "validator");
    assert_eq!(stdout_value(&status, "role_can_produce_blocks"), "true");
    assert_eq!(
        stdout_value(&status, "role_wallet_registration"),
        "validator"
    );
    assert_eq!(stdout_value(&status, "role_local_producer"), "true");
    assert_eq!(stdout_value(&status, "role_local_block_proposer"), "true");
    assert_eq!(
        stdout_u64(&status, "role_local_block_proposer_delay_blocks"),
        0
    );
    assert_eq!(
        stdout_value(&status, "role_local_block_proposer_delay_satisfied"),
        "true"
    );
    assert_eq!(stdout_u64(&status, "role_produced_blocks"), 0);
    assert_eq!(stdout_u64(&status, "height"), initial_height);
    assert!(stdout_u64(&status, "job_count") > 2);

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_run_commands_serve_through_role_specific_surfaces() {
    for role in ["miner", "validator", "proposer"] {
        let data_dir = unique_test_dir(&format!("{role}-run"));
        let data_dir_text = data_dir.to_string_lossy().into_owned();
        let seed = run_tvmd(&["localnet", "seed", "--data-dir", &data_dir_text]);
        assert_eq!(stdout_value(&seed, "command"), "local_testnet_seed");

        let rpc_port = free_local_port();
        let listen = format!("127.0.0.1:{rpc_port}");
        let mut args = vec![role.to_owned(), "run".to_owned(), "--wallet".to_owned()];
        let (wallet, expected_registration) = match role {
            "miner" => ("testnet-miner-0", "miner"),
            "validator" => ("testnet-validator-0", "validator"),
            "proposer" => ("testnet-validator-0", "validator"),
            _ => unreachable!("covered role set"),
        };
        if role == "miner" {
            args.extend([
                wallet.to_owned(),
                "--device".to_owned(),
                "cpu".to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4001".to_owned(),
            ]);
        } else if role == "validator" {
            args.extend([
                wallet.to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4002".to_owned(),
            ]);
        } else {
            args.extend([
                wallet.to_owned(),
                "--node".to_owned(),
                "/ip4/127.0.0.1/tcp/4003".to_owned(),
            ]);
        }
        args.extend([
            "--listen".to_owned(),
            listen,
            "--p2p-listen".to_owned(),
            "/ip4/127.0.0.1/tcp/0".to_owned(),
            "--data-dir".to_owned(),
            data_dir_text.clone(),
            "--auth-token".to_owned(),
            "service-token".to_owned(),
            "--max-requests".to_owned(),
            "1".to_owned(),
        ]);

        let child = Command::new(env!("CARGO_BIN_EXE_tvmd"))
            .args(&args)
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("role-specific tvmd command must spawn");

        let health = authenticated_get_request(rpc_port, "/health");
        assert_eq!(response_status_line(&health), "HTTP/1.1 200 OK");

        let output = child.wait_with_output().expect("role process must exit");
        assert!(
            output.status.success(),
            "{role} run failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("role stdout must be utf8");
        assert_eq!(
            stdout_values(&stdout, "command"),
            [format!("{role}_run").as_str(), "service_serve"]
        );
        assert_eq!(stdout_value(&stdout, "role"), role);
        assert_eq!(stdout_value(&stdout, "role_runtime_ready"), "true");
        if role == "proposer" {
            assert_eq!(stdout_value(&stdout, "proposer_ready"), "true");
        }
        assert_eq!(stdout_value(&stdout, "role_loop_ready"), "true");
        assert_eq!(
            stdout_value(&stdout, "runtime_command"),
            format!("{role}_run")
        );
        assert_eq!(stdout_value(&stdout, "chain_profile"), "local_cpu");
        let role_can_produce_blocks = if role == "validator" { "true" } else { "false" };
        let wallet_address = hex(&address(wallet.as_bytes()));
        assert_eq!(
            stdout_value(&stdout, "role_can_produce_blocks"),
            role_can_produce_blocks
        );
        assert_eq!(stdout_value(&stdout, "role_wallet_address"), wallet_address);
        assert_eq!(
            stdout_value(&stdout, "role_wallet_registration"),
            expected_registration
        );
        assert_eq!(stdout_value(&stdout, "role_wallet_registered"), "true");
        assert!(matches!(
            stdout_value(&stdout, "miner_work_ready"),
            "true" | "false"
        ));
        assert!(stdout_u64(&stdout, "miner_assigned_jobs_seen") <= 10);
        assert!(stdout_u64(&stdout, "miner_unreceipted_jobs") <= 10);
        assert!(stdout_u64(&stdout, "miner_receipts_submitted") <= 10);
        assert!(stdout_u64(&stdout, "miner_tensors_inserted") <= 20);
        assert!(matches!(
            stdout_value(&stdout, "validator_work_ready"),
            "true" | "false"
        ));
        assert!(stdout_u64(&stdout, "validator_assigned_receipts_seen") <= 10);
        assert!(stdout_u64(&stdout, "validator_unattested_receipts") <= 10);
        assert!(stdout_u64(&stdout, "validator_artifact_ready_receipts") <= 10);
        assert!(stdout_u64(&stdout, "validator_artifact_missing_receipts") <= 10);
        assert!(stdout_u64(&stdout, "validator_remote_tensor_fetch_attempts") <= 10);
        assert!(stdout_u64(&stdout, "validator_remote_tensor_fetch_successes") <= 10);
        assert!(stdout_u64(&stdout, "validator_remote_tensor_fetch_failures") <= 10);
        assert!(stdout_u64(&stdout, "validator_remote_tensor_fetch_bytes") <= 1_000_000);
        assert!(stdout_u64(&stdout, "validator_remote_tensors_inserted") <= 20);
        assert!(stdout_u64(&stdout, "validator_attestations_submitted") <= 10);
        assert!(matches!(
            stdout_value(&stdout, "validator_proposer_work_ready"),
            "true" | "false"
        ));
        assert_eq!(
            stdout_u64(&stdout, "validator_proposer_settled_receipts_seen"),
            0
        );
        assert_eq!(stdout_u64(&stdout, "validator_blocks_proposed"), 0);
        assert_eq!(stdout_u64(&stdout, "validator_useful_blocks_proposed"), 0);
        assert_eq!(stdout_u64(&stdout, "validator_fallback_blocks_proposed"), 0);
        assert_eq!(stdout_u64(&stdout, "validator_receipts_proposed"), 0);
        assert_eq!(stdout_u64(&stdout, "validator_block_votes_submitted"), 0);
        assert_eq!(stdout_value(&stdout, "local_producer"), "false");
        assert_eq!(stdout_value(&stdout, "local_block_proposer"), "false");
        assert_eq!(stdout_u64(&stdout, "local_block_proposer_delay_blocks"), 0);
        assert_eq!(
            stdout_value(&stdout, "local_block_proposer_delay_satisfied"),
            "true"
        );
        assert_eq!(stdout_value(&stdout, "p2p_runtime"), "libp2p");
        assert_eq!(stdout_u64(&stdout, "p2p_connected_peers"), 0);
        assert_eq!(stdout_u64(&stdout, "p2p_observed_block_gossip_count"), 0);
        assert_eq!(
            stdout_u64(&stdout, "p2p_observed_block_payload_gossip_count"),
            0
        );
        assert_eq!(
            stdout_u64(&stdout, "p2p_observed_block_vote_gossip_count"),
            0
        );
        assert_eq!(stdout_u64(&stdout, "p2p_observed_job_gossip_count"), 0);
        assert_eq!(stdout_u64(&stdout, "p2p_observed_receipt_gossip_count"), 0);
        assert_eq!(
            stdout_u64(&stdout, "p2p_observed_attestation_gossip_count"),
            0
        );
        let zero_hash = hex(&[0u8; 32]);
        assert_eq!(stdout_u64(&stdout, "p2p_latest_observed_block_height"), 0);
        assert_eq!(
            stdout_value(&stdout, "p2p_latest_observed_block_hash"),
            zero_hash
        );
        assert_eq!(stdout_value(&stdout, "p2p_observed_block_hashes"), "none");
        assert_eq!(
            stdout_u64(&stdout, "p2p_latest_observed_block_payload_height"),
            0
        );
        assert_eq!(
            stdout_value(&stdout, "p2p_latest_observed_block_payload_hash"),
            zero_hash
        );
        assert_eq!(
            stdout_value(&stdout, "p2p_observed_block_payload_hashes"),
            "none"
        );
        assert_eq!(stdout_u64(&stdout, "served_requests"), 1);
        assert_eq!(stdout_u64(&stdout, "network_applied_blocks"), 0);
        assert_eq!(stdout_u64(&stdout, "network_events_ingested"), 0);
        assert_eq!(stdout_u64(&stdout, "network_block_payloads_ingested"), 0);
        assert_eq!(stdout_u64(&stdout, "network_block_payloads_applied"), 0);
        assert_eq!(stdout_u64(&stdout, "network_block_votes_ingested"), 0);
        assert_eq!(stdout_u64(&stdout, "network_block_votes_applied"), 0);
        assert_eq!(
            stdout_u64(&stdout, "network_external_randomness_beacons_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&stdout, "network_external_randomness_beacons_applied"),
            0
        );
        assert_eq!(
            stdout_u64(&stdout, "network_validator_vrf_reveals_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&stdout, "network_validator_vrf_reveals_applied"),
            0
        );
        assert_eq!(stdout_u64(&stdout, "network_invalid_events"), 0);

        let status = run_tvmd(&["node", "status", "--data-dir", &data_dir_text]);
        assert_eq!(
            stdout_value(&status, "role_runtime_command"),
            format!("{role}_run")
        );
        assert_eq!(stdout_value(&status, "role_loop_role"), role);
        assert_eq!(stdout_value(&status, "role_loop_ready"), "true");
        assert_eq!(stdout_value(&status, "role_chain_profile"), "local_cpu");
        assert_eq!(
            stdout_value(&status, "role_can_produce_blocks"),
            role_can_produce_blocks
        );
        assert_eq!(stdout_value(&status, "role_wallet_address"), wallet_address);
        assert_eq!(
            stdout_value(&status, "role_wallet_registration"),
            expected_registration
        );
        assert_eq!(stdout_value(&status, "role_wallet_registered"), "true");
        assert!(matches!(
            stdout_value(&status, "role_miner_work_ready"),
            "true" | "false"
        ));
        assert!(stdout_u64(&status, "role_miner_assigned_jobs_seen") <= 10);
        assert!(stdout_u64(&status, "role_miner_unreceipted_jobs") <= 10);
        assert!(stdout_u64(&status, "role_miner_receipts_submitted") <= 10);
        assert!(stdout_u64(&status, "role_miner_tensors_inserted") <= 20);
        assert!(matches!(
            stdout_value(&status, "role_validator_work_ready"),
            "true" | "false"
        ));
        assert!(stdout_u64(&status, "role_validator_assigned_receipts_seen") <= 10);
        assert!(stdout_u64(&status, "role_validator_unattested_receipts") <= 10);
        assert!(stdout_u64(&status, "role_validator_artifact_ready_receipts") <= 10);
        assert!(stdout_u64(&status, "role_validator_artifact_missing_receipts") <= 10);
        assert!(stdout_u64(&status, "role_validator_remote_tensor_fetch_attempts") <= 10);
        assert!(stdout_u64(&status, "role_validator_remote_tensor_fetch_successes") <= 10);
        assert!(stdout_u64(&status, "role_validator_remote_tensor_fetch_failures") <= 10);
        assert!(stdout_u64(&status, "role_validator_remote_tensor_fetch_bytes") <= 1_000_000);
        assert!(stdout_u64(&status, "role_validator_remote_tensors_inserted") <= 20);
        assert!(stdout_u64(&status, "role_validator_attestations_submitted") <= 10);
        assert!(matches!(
            stdout_value(&status, "role_validator_proposer_work_ready"),
            "true" | "false"
        ));
        assert_eq!(
            stdout_u64(&status, "role_validator_proposer_settled_receipts_seen"),
            0
        );
        assert_eq!(stdout_u64(&status, "role_validator_blocks_proposed"), 0);
        assert_eq!(
            stdout_u64(&status, "role_validator_useful_blocks_proposed"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_validator_fallback_blocks_proposed"),
            0
        );
        assert_eq!(stdout_u64(&status, "role_validator_receipts_proposed"), 0);
        assert_eq!(
            stdout_u64(&status, "role_validator_block_votes_submitted"),
            0
        );
        assert_eq!(stdout_value(&status, "role_local_producer"), "false");
        assert_eq!(stdout_value(&status, "role_local_block_proposer"), "false");
        assert_eq!(
            stdout_u64(&status, "role_local_block_proposer_delay_blocks"),
            0
        );
        assert_eq!(
            stdout_value(&status, "role_local_block_proposer_delay_satisfied"),
            "true"
        );
        assert_eq!(stdout_u64(&status, "role_proposer_cooldown_blocks"), 0);
        assert_eq!(stdout_value(&status, "role_proposer_cadence_ready"), "true");
        assert_eq!(
            stdout_u64(&status, "role_proposer_cadence_remaining_blocks"),
            0
        );
        assert_eq!(stdout_u64(&status, "role_served_requests"), 1);
        assert_eq!(stdout_u64(&status, "role_network_applied_blocks"), 0);
        assert_eq!(stdout_u64(&status, "role_network_events_ingested"), 0);
        assert_eq!(stdout_u64(&status, "role_network_block_events_ingested"), 0);
        assert_eq!(
            stdout_u64(&status, "role_network_block_headers_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_block_payloads_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_block_payloads_applied"),
            0
        );
        assert_eq!(stdout_u64(&status, "role_network_block_votes_ingested"), 0);
        assert_eq!(stdout_u64(&status, "role_network_block_votes_applied"), 0);
        assert_eq!(stdout_u64(&status, "role_network_job_events_ingested"), 0);
        assert_eq!(stdout_u64(&status, "role_network_job_payloads_ingested"), 0);
        assert_eq!(stdout_u64(&status, "role_network_job_payloads_applied"), 0);
        assert_eq!(
            stdout_u64(&status, "role_network_receipt_payloads_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_receipt_payloads_applied"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_attestation_payloads_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_attestation_payloads_applied"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_receipt_events_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_attestation_events_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_external_randomness_beacons_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_external_randomness_beacons_applied"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_validator_vrf_reveals_ingested"),
            0
        );
        assert_eq!(
            stdout_u64(&status, "role_network_validator_vrf_reveals_applied"),
            0
        );
        assert_eq!(stdout_u64(&status, "role_network_peer_events_ingested"), 0);
        assert_eq!(stdout_u64(&status, "role_network_invalid_events"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_connected_peers"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_blocks"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_block_payloads"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_block_votes"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_jobs"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_receipts"), 0);
        assert_eq!(stdout_u64(&status, "role_p2p_observed_attestations"), 0);
        assert_eq!(
            stdout_u64(&status, "role_p2p_latest_observed_block_height"),
            0
        );
        assert_eq!(
            stdout_value(&status, "role_p2p_latest_observed_block_hash"),
            zero_hash
        );
        assert_eq!(
            stdout_value(&status, "role_p2p_observed_block_hashes"),
            "none"
        );
        assert_eq!(
            stdout_u64(&status, "role_p2p_latest_observed_block_payload_height"),
            0
        );
        assert_eq!(
            stdout_value(&status, "role_p2p_latest_observed_block_payload_hash"),
            zero_hash
        );
        assert_eq!(
            stdout_value(&status, "role_p2p_observed_block_payload_hashes"),
            "none"
        );

        std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
    }
}
