use super::*;

#[test]
fn service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces() {
    let data_dir = unique_test_dir("service-cli-lifecycle");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let init = run_tvmd(&["node", "init", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&init, "command"), "service_init");
    assert_eq!(stdout_value(&init, "data_dir"), data_dir_text);
    assert_eq!(stdout_value(&init, "existing_store"), "false");
    assert_eq!(stdout_value(&init, "recovered_store"), "false");
    assert_eq!(stdout_u64(&init, "block_count"), 0);
    assert_eq!(stdout_value(&init, "latest_block_hash").len(), 64);

    let peer_id = PeerId::random().to_string();
    let peer_add = run_tvmd(&[
        "node",
        "peer",
        "add",
        "--data-dir",
        &data_dir_text,
        "--peer-id",
        &peer_id,
        "--address",
        "/ip4/127.0.0.1/tcp/4001",
    ]);
    assert_eq!(stdout_value(&peer_add, "command"), "service_peer_add");
    assert_eq!(stdout_value(&peer_add, "data_dir"), data_dir_text);
    assert_eq!(stdout_value(&peer_add, "peer_id"), peer_id);
    assert_eq!(
        stdout_value(&peer_add, "address"),
        "/ip4/127.0.0.1/tcp/4001"
    );
    assert_eq!(
        stdout_value(&peer_add, "bootstrap_address"),
        format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}")
    );
    assert_eq!(stdout_u64(&peer_add, "bootstrap_peers"), 1);

    let readiness = run_tvmd(&[
        "node",
        "check",
        "--p2p-listen",
        "/ip4/127.0.0.1/tcp/0",
        "--data-dir",
        &data_dir_text,
    ]);
    assert_eq!(stdout_value(&readiness, "command"), "service_readiness");
    assert_eq!(stdout_value(&readiness, "p2p_runtime"), "libp2p");
    assert!(
        stdout_value(&readiness, "p2p_peer_id")
            .parse::<PeerId>()
            .is_ok()
    );
    assert!(stdout_u64(&readiness, "p2p_gossipsub_topics") > 0);
    assert!(stdout_u64(&readiness, "p2p_request_response_protocols") > 0);
    assert_eq!(stdout_u64(&readiness, "p2p_bootstrap_peers"), 1);
    assert_eq!(stdout_u64(&readiness, "p2p_max_transmit_bytes"), 1_048_576);
    assert_eq!(stdout_u64(&readiness, "p2p_request_timeout_seconds"), 10);
    assert_eq!(stdout_u64(&readiness, "p2p_max_concurrent_streams"), 128);
    assert_eq!(stdout_u64(&readiness, "p2p_idle_timeout_seconds"), 60);
    assert_eq!(stdout_value(&readiness, "node_store_ready"), "true");
    assert_eq!(stdout_value(&readiness, "libp2p_ready"), "true");

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
            "19",
        ])
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tvmd node serve must spawn");

    let unauthenticated_health = unauthenticated_get_request(rpc_port, "/health");
    let unauthenticated_health =
        response_json_with_status(&unauthenticated_health, "HTTP/1.1 401 Unauthorized");
    assert_eq!(
        unauthenticated_health["error"].as_str(),
        Some("unauthorized")
    );

    let health = authenticated_get_request(rpc_port, "/health");
    let health = response_json_with_status(&health, "HTTP/1.1 200 OK");
    assert_eq!(health["status"].as_str(), Some("ok"));
    assert_eq!(health["service"].as_str(), Some("all"));

    for (path, service, endpoint_id, public_url) in [
        (
            "/rpc/health",
            "rpc",
            "55",
            "https://rpc.tensorvm.net/health",
        ),
        (
            "/explorer/health",
            "explorer",
            "66",
            "https://explorer.tensorvm.net/health",
        ),
        (
            "/faucet/health",
            "faucet",
            "77",
            "https://faucet.tensorvm.net/health",
        ),
        (
            "/telemetry/health",
            "telemetry",
            "88",
            "https://telemetry.tensorvm.net/health",
        ),
    ] {
        let response = authenticated_get_request(rpc_port, path);
        let body = response_json_with_status(&response, "HTTP/1.1 200 OK");
        assert_eq!(body["status"].as_str(), Some("ok"));
        assert_eq!(body["service"].as_str(), Some(service));
        assert_service_health_evidence_from_response(
            service,
            &endpoint_id.repeat(32),
            public_url,
            &response,
        );
    }

    let chain_head = authenticated_get_request(rpc_port, "/chain/head");
    let chain_head_body = response_json_with_status(&chain_head, "HTTP/1.1 200 OK");
    json_u64(&chain_head_body, "height");
    json_u64(&chain_head_body, "block_count");
    assert_eq!(
        chain_head_body["state_root"]
            .as_str()
            .expect("chain head state root must be a string")
            .len(),
        64
    );
    assert_service_content_evidence_from_response(
        &data_dir,
        "rpc",
        &"55".repeat(32),
        "https://rpc.tensorvm.net/chain/head",
        "/chain/head",
        "rpc-chain-head.body",
        &chain_head,
    );

    let current_epoch = authenticated_get_request(rpc_port, "/epoch/current");
    let current_epoch = response_json_with_status(&current_epoch, "HTTP/1.1 200 OK");
    json_u64(&current_epoch, "epoch");

    let current_jobs = authenticated_get_request(rpc_port, "/jobs/current");
    let current_jobs = response_json_with_status(&current_jobs, "HTTP/1.1 200 OK");
    assert!(
        current_jobs["jobs"]
            .as_array()
            .expect("current jobs must be an array")
            .is_empty()
    );

    let genesis_block = authenticated_get_request(rpc_port, "/chain/block/0");
    let genesis_block = response_json_with_status(&genesis_block, "HTTP/1.1 404 Not Found");
    assert_eq!(genesis_block["error"].as_str(), Some("block not found"));

    let miner_address = "11".repeat(32);
    let tx = authenticated_request(
        rpc_port,
        "POST",
        "/tx",
        &format!("register_miner {miner_address}"),
    );
    let tx = response_json_with_status(&tx, "HTTP/1.1 202 Accepted");
    assert_eq!(tx["accepted"].as_bool(), Some(true));

    let validator_address = "44".repeat(32);
    let validator_tx = authenticated_request(
        rpc_port,
        "POST",
        "/tx",
        &format!("register_validator {validator_address}"),
    );
    let validator_tx = response_json_with_status(&validator_tx, "HTTP/1.1 202 Accepted");
    assert_eq!(validator_tx["accepted"].as_bool(), Some(true));

    let miner_state = authenticated_get_request(rpc_port, &format!("/miners/{miner_address}"));
    let miner_state = response_json_with_status(&miner_state, "HTTP/1.1 200 OK");
    assert_eq!(
        miner_state["address"].as_str(),
        Some(miner_address.as_str())
    );
    assert_eq!(miner_state["stake"].as_u64(), Some(100));

    let validator_state =
        authenticated_get_request(rpc_port, &format!("/validators/{validator_address}"));
    let validator_state = response_json_with_status(&validator_state, "HTTP/1.1 200 OK");
    assert_eq!(
        validator_state["address"].as_str(),
        Some(validator_address.as_str())
    );
    assert_eq!(validator_state["stake"].as_u64(), Some(10_000));

    let receipt = authenticated_request(rpc_port, "POST", "/receipt", &"22".repeat(32));
    let receipt = response_json_with_status(&receipt, "HTTP/1.1 202 Accepted");
    assert_eq!(receipt["accepted"].as_bool(), Some(true));

    let attestation = authenticated_request(rpc_port, "POST", "/attestation", &"33".repeat(32));
    let attestation = response_json_with_status(&attestation, "HTTP/1.1 202 Accepted");
    assert_eq!(attestation["accepted"].as_bool(), Some(true));

    let explorer = authenticated_get_request(rpc_port, "/explorer");
    assert_eq!(response_status_line(&explorer), "HTTP/1.1 200 OK");
    assert_eq!(
        html_tag_text(response_body(&explorer), "title"),
        "TensorVM Explorer"
    );
    assert_service_content_evidence_from_response(
        &data_dir,
        "explorer",
        &"66".repeat(32),
        "https://explorer.tensorvm.net/explorer",
        "/explorer",
        "explorer.body",
        &explorer,
    );

    let faucet = authenticated_get_request(rpc_port, "/faucet/page");
    assert_eq!(response_status_line(&faucet), "HTTP/1.1 200 OK");
    assert_eq!(
        html_tag_text(response_body(&faucet), "title"),
        "TensorVM Faucet"
    );
    assert_service_content_evidence_from_response(
        &data_dir,
        "faucet",
        &"77".repeat(32),
        "https://faucet.tensorvm.net/faucet/page",
        "/faucet/page",
        "faucet-page.body",
        &faucet,
    );

    let telemetry = authenticated_get_request(rpc_port, "/telemetry/dashboard");
    assert_eq!(response_status_line(&telemetry), "HTTP/1.1 200 OK");
    assert_eq!(
        html_tag_text(response_body(&telemetry), "title"),
        "TensorVM Telemetry"
    );
    assert_service_content_evidence_from_response(
        &data_dir,
        "telemetry",
        &"88".repeat(32),
        "https://telemetry.tensorvm.net/telemetry/dashboard",
        "/telemetry/dashboard",
        "telemetry-dashboard.body",
        &telemetry,
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
    assert_eq!(stdout_value(&stdout, "p2p_runtime"), "libp2p");
    assert!(
        stdout_value(&stdout, "p2p_peer_id")
            .parse::<PeerId>()
            .is_ok()
    );
    assert!(stdout_u64(&stdout, "p2p_gossipsub_topics") > 0);
    assert!(stdout_u64(&stdout, "p2p_request_response_protocols") > 0);
    assert_eq!(stdout_u64(&stdout, "p2p_bootstrap_peers"), 1);
    assert_eq!(stdout_u64(&stdout, "p2p_max_transmit_bytes"), 1_048_576);
    assert_eq!(stdout_u64(&stdout, "p2p_request_timeout_seconds"), 10);
    assert_eq!(stdout_u64(&stdout, "p2p_max_concurrent_streams"), 128);
    assert_eq!(stdout_u64(&stdout, "p2p_idle_timeout_seconds"), 60);
    assert_eq!(stdout_u64(&stdout, "served_requests"), 19);
    let p2p_peer_id = stdout_value(&stdout, "p2p_peer_id");
    let p2p_gossipsub_topics = stdout_value(&stdout, "p2p_gossipsub_topics");
    let p2p_request_response_protocols = stdout_value(&stdout, "p2p_request_response_protocols");
    let p2p_bootstrap_peers = stdout_value(&stdout, "p2p_bootstrap_peers");
    let service_log = data_dir.join("service.log");
    std::fs::write(&service_log, stdout.as_bytes()).expect("service log fixture must be written");
    let service_log_text = service_log.to_string_lossy().into_owned();
    let public_observation = run_tvmd(&[
        "public",
        "evidence",
        "network",
        "observation",
        "--operator-id",
        &"99".repeat(32),
        "--peer-id",
        p2p_peer_id,
        "--listen-address",
        "/dns/node-a.tensorvm.net/tcp/4001",
        "--observed-at",
        "1700000000",
        "--gossip-topics",
        p2p_gossipsub_topics,
        "--request-response-protocols",
        p2p_request_response_protocols,
        "--bootstrap-peers",
        p2p_bootstrap_peers,
        "--max-transmit-bytes",
        "1048576",
        "--request-timeout-seconds",
        "10",
        "--max-concurrent-streams",
        "128",
        "--idle-timeout-seconds",
        "60",
    ]);
    let public_observation_fields =
        comma_record_fields(&public_observation, "network_runtime_observation=", 13);
    assert_eq!(public_observation_fields[0], "99".repeat(32));
    assert_eq!(public_observation_fields[1], p2p_peer_id);
    assert_eq!(
        public_observation_fields[2],
        "/dns/node-a.tensorvm.net/tcp/4001"
    );
    assert_eq!(public_observation_fields[3], "1700000000");
    assert_eq!(public_observation_fields[4], p2p_gossipsub_topics);
    assert_eq!(public_observation_fields[5], p2p_request_response_protocols);
    assert_eq!(public_observation_fields[6], p2p_bootstrap_peers);
    assert_eq!(public_observation_fields[7], "1048576");
    assert_eq!(public_observation_fields[8], "10");
    assert_eq!(public_observation_fields[9], "128");
    assert_eq!(public_observation_fields[10], "60");
    assert_eq!(public_observation_fields[11].len(), 64);
    assert_eq!(public_observation_fields[12].len(), 64);
    let public_observation_from_service_log = run_tvmd(&[
        "public",
        "evidence",
        "network",
        "from-service-log",
        "--operator-id",
        &"99".repeat(32),
        "--listen-address",
        "/dns/node-a.tensorvm.net/tcp/4001",
        "--observed-at",
        "1700000000",
        "--service-log",
        &service_log_text,
    ]);
    assert_eq!(public_observation_from_service_log, public_observation);
    let observation_root = network_observation_root(&public_observation);
    let bundle_id = "aa".repeat(32);
    let manifest_signer = "bb".repeat(32);
    let summary_from_root = run_tvmd(&[
        "public",
        "evidence",
        "record",
        "summary-roots",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--record-roots",
        observation_root,
    ]);
    assert_eq!(
        stdout_u64(&summary_from_root, "network_runtime_observation_records"),
        1
    );
    let summary_record_root = stdout_value(&summary_from_root, "network_runtime_observation_root");
    let summary_signature =
        stdout_value(&summary_from_root, "network_runtime_observation_signature");
    assert_eq!(summary_record_root.len(), 64);
    assert_ne!(summary_record_root, "0".repeat(64));
    assert_eq!(summary_signature.len(), 64);
    assert_ne!(summary_signature, "0".repeat(64));
    let artifact_from_root = run_tvmd(&[
        "public",
        "evidence",
        "record",
        "artifact-roots",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--artifact-uri",
        "https://evidence.tensorvm.net/network-runtime.json",
        "--record-roots",
        observation_root,
    ]);
    let artifact_fields = comma_record_fields(&artifact_from_root, "record_artifact=", 5);
    assert_eq!(artifact_fields[0], "network-runtime");
    assert_eq!(
        artifact_fields[1],
        "https://evidence.tensorvm.net/network-runtime.json"
    );
    assert_eq!(artifact_fields[2], summary_record_root);
    assert_eq!(artifact_fields[3], "1");
    assert_eq!(artifact_fields[4].len(), 64);
    assert_ne!(artifact_fields[4], "0".repeat(64));
    let (status, public_observation_stdout, public_observation_stderr) = run_tvmd_failure(&[
        "public",
        "evidence",
        "network",
        "observation",
        "--operator-id",
        &"99".repeat(32),
        "--peer-id",
        p2p_peer_id,
        "--listen-address",
        "/ip4/127.0.0.1/tcp/4001",
        "--observed-at",
        "1700000000",
        "--gossip-topics",
        p2p_gossipsub_topics,
        "--request-response-protocols",
        p2p_request_response_protocols,
        "--bootstrap-peers",
        p2p_bootstrap_peers,
        "--max-transmit-bytes",
        "1048576",
        "--request-timeout-seconds",
        "10",
        "--max-concurrent-streams",
        "128",
        "--idle-timeout-seconds",
        "60",
    ]);
    assert_eq!(status, 1);
    assert!(public_observation_stdout.is_empty());
    assert_eq!(
        public_observation_stderr.trim_end(),
        "invalid receipt: network observation address is not public"
    );
    let (status, log_observation_stdout, log_observation_stderr) = run_tvmd_failure(&[
        "public",
        "evidence",
        "network",
        "from-service-log",
        "--operator-id",
        &"99".repeat(32),
        "--listen-address",
        "/ip4/127.0.0.1/tcp/4001",
        "--observed-at",
        "1700000000",
        "--service-log",
        &service_log_text,
    ]);
    assert_eq!(status, 1);
    assert!(log_observation_stdout.is_empty());
    assert_eq!(
        log_observation_stderr.trim_end(),
        "invalid receipt: network observation address is not public"
    );

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn service_readiness_loads_runtime_bootstrap_peers_from_env_without_peer_book() {
    let data_dir = unique_test_dir("service-env-bootstrap");
    let data_dir_text = data_dir.to_string_lossy().into_owned();

    let init = run_tvmd(&["node", "init", "--data-dir", &data_dir_text]);
    assert_eq!(stdout_value(&init, "command"), "service_init");
    assert!(!data_dir.join("peers.book").exists());

    let peer_id = PeerId::random();
    let bootstrap_address = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}");
    let readiness = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "node",
            "check",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
        ])
        .env("TENSORVM_BOOTSTRAP_PEERS", &bootstrap_address)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd node check must execute");
    assert!(
        readiness.status.success(),
        "tvmd node check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&readiness.stdout),
        String::from_utf8_lossy(&readiness.stderr)
    );
    let readiness = String::from_utf8(readiness.stdout).expect("readiness stdout must be utf8");
    assert_eq!(stdout_value(&readiness, "command"), "service_readiness");
    assert_eq!(stdout_u64(&readiness, "p2p_bootstrap_peers"), 1);

    let failure = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args([
            "node",
            "check",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            &data_dir_text,
        ])
        .env("TENSORVM_BOOTSTRAP_PEERS", "/ip4/127.0.0.1/tcp/4001")
        .current_dir(workspace_root())
        .output()
        .expect("tvmd node check must execute");
    assert!(
        !failure.status.success(),
        "tvmd node check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&failure.stdout),
        String::from_utf8_lossy(&failure.stderr)
    );
    let stderr = String::from_utf8(failure.stderr).expect("failure stderr must be utf8");
    assert!(stderr.contains("invalid TENSORVM_BOOTSTRAP_PEERS address"));
    assert!(stderr.contains("bootstrap address missing peer id"));
}
