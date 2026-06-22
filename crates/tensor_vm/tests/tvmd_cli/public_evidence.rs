use super::*;

#[test]
fn documented_public_testnet_preflight_command_reports_pending_status() {
    let stdout = run_tvmd(&[
        "public",
        "preflight",
        "docs/tensorvm/public-testnet.preflight",
    ]);

    assert_eq!(
        stdout_value(&stdout, "public_testnet_preflight_ready"),
        "false"
    );
    assert_eq!(stdout_value(&stdout, "local_shape_ready"), "true");
    assert_eq!(stdout_value(&stdout, "deployment_plan_ready"), "false");
    assert_eq!(stdout_value(&stdout, "production_libp2p_runtime"), "true");
    assert_eq!(stdout_value(&stdout, "public_services_planned"), "false");
}

#[test]
fn generated_public_testnet_preflight_manifest_reports_ready() {
    let data_dir = unique_test_dir("generated-public-preflight");
    let manifest_path = data_dir.join("generated-public-testnet.preflight");
    let manifest_path_text = manifest_path.to_string_lossy().into_owned();
    let manifest = "\
version=tensor-vm-public-testnet-preflight-v1
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,1111111111111111111111111111111111111111111111111111111111111111,https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,2222222222222222222222222222222222222222222222222222222222222222,https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,3333333333333333333333333333333333333333333333333333333333333333,https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,4444444444444444444444444444444444444444444444444444444444444444,https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
";
    std::fs::write(&manifest_path, manifest).expect("generated preflight manifest must be written");

    let stdout = run_tvmd(&["public", "preflight", &manifest_path_text]);
    assert_eq!(
        stdout_value(&stdout, "public_testnet_preflight_ready"),
        "true"
    );
    assert_eq!(stdout_value(&stdout, "local_shape_ready"), "true");
    assert_eq!(stdout_value(&stdout, "deployment_plan_ready"), "true");
    assert_eq!(stdout_u64(&stdout, "miners"), 10);
    assert_eq!(stdout_u64(&stdout, "validators"), 5);
    assert_eq!(stdout_u64(&stdout, "required_blocks"), 100_800);
    assert_eq!(stdout_u64(&stdout, "cuda_ready_miner_count"), 10);
    assert_eq!(stdout_value(&stdout, "cuda_ready_miners"), "true");
    assert_eq!(stdout_u64(&stdout, "libp2p_ready_node_count"), 15);
    assert_eq!(stdout_value(&stdout, "libp2p_ready_nodes"), "true");
    assert_eq!(stdout_value(&stdout, "production_libp2p_runtime"), "true");
    assert_eq!(
        stdout_value(&stdout, "public_service_content_planned"),
        "true"
    );
    assert_eq!(stdout_value(&stdout, "public_services_planned"), "true");

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn documented_public_testnet_evidence_command_reports_non_full_spec_status() {
    let stdout = run_tvmd(&[
        "public",
        "evidence",
        "validate",
        "docs/tensorvm/public-testnet.evidence",
    ]);

    assert_eq!(stdout_value(&stdout, "public_evidence_full_spec"), "false");
    assert_eq!(stdout_value(&stdout, "public_criterion"), "false");
    assert_eq!(stdout_value(&stdout, "independently_checkable"), "false");
    assert_eq!(stdout_value(&stdout, "published_evidence_bundle"), "false");
    assert_eq!(stdout_value(&stdout, "signed_run_window"), "true");
    assert_eq!(
        stdout_value(&stdout, "supporting_record_artifacts"),
        "false"
    );
    assert_eq!(stdout_value(&stdout, "required_run_duration"), "false");
    assert_eq!(stdout_value(&stdout, "required_block_count"), "false");
}

#[test]
fn generated_public_evidence_manifest_round_trips_through_tvmd_validator() {
    let data_dir = unique_test_dir("generated-public-evidence");
    let manifest_path = data_dir.join("generated-public-testnet.evidence");
    let manifest_path_text = manifest_path.to_string_lossy().into_owned();

    let bundle_id = "11".repeat(32);
    let manifest_signer = "22".repeat(32);
    let public_uri = "https://tensorvm.net/tensorvm/public-evidence.json";
    let publication = trimmed_tvmd(&[
        "public",
        "evidence",
        "publish",
        "--bundle-id",
        &bundle_id,
        "--public-uri",
        public_uri,
        "--manifest-signer",
        &manifest_signer,
        "--manifest-signature-count",
        "1",
        "--independent-auditor-count",
        "1",
    ]);
    let auditor = trimmed_tvmd(&[
        "public",
        "evidence",
        "audit",
        "--bundle-id",
        &bundle_id,
        "--public-uri",
        public_uri,
        "--auditor-id",
        &"33".repeat(32),
        "--audit-uri",
        "https://auditors.tensorvm.net/tensorvm/generated-audit.json",
        "--observed-at",
        "1700000060",
    ]);

    let mut artifact_lines = Vec::new();
    let mut summary_lines = Vec::new();
    for (kind, root, count) in [
        ("block-history", "44".repeat(32), "10"),
        ("finality-history", "55".repeat(32), "10"),
        ("randomness-beacon", "a0".repeat(32), "10"),
        ("data-availability", "77".repeat(32), "20"),
        ("invalid-work", "88".repeat(32), "1"),
        ("reward-settlement", "99".repeat(32), "1"),
        ("detection-measurement", "cc".repeat(32), "1"),
    ] {
        summary_lines.push(trimmed_tvmd(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            kind,
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &root,
            "--record-count",
            count,
        ]));
        artifact_lines.push(trimmed_tvmd(&[
            "public",
            "evidence",
            "record",
            "artifact",
            "--kind",
            kind,
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--artifact-uri",
            &format!("https://evidence.tensorvm.net/tensorvm/{kind}.json"),
            "--record-root",
            &root,
            "--record-count",
            count,
        ]));
    }

    let miner_a = "aa".repeat(32);
    let miner_a_operator = "dd".repeat(32);
    let miner_b = "bb".repeat(32);
    let miner_b_operator = "ee".repeat(32);
    let validator_a = "cc".repeat(32);
    let validator_a_operator = "ff".repeat(32);
    let participants = [
        (
            "miner",
            miner_a.as_str(),
            miner_a_operator.as_str(),
            "node-a.tensorvm.net",
            "4001",
        ),
        (
            "miner",
            miner_b.as_str(),
            miner_b_operator.as_str(),
            "node-b.tensorvm.net",
            "4002",
        ),
        (
            "validator",
            validator_a.as_str(),
            validator_a_operator.as_str(),
            "node-c.tensorvm.net",
            "4003",
        ),
    ];
    let mut operator_lines = Vec::new();
    let mut node_lines = Vec::new();
    let mut network_lines = Vec::new();
    let mut network_roots = Vec::new();
    for (role, address, operator_id, host, port) in participants {
        let identity_uri = format!("https://operators.tensorvm.net/{operator_id}.json");
        operator_lines.push(trimmed_tvmd(&[
            "public",
            "evidence",
            "node",
            "operator-attestation",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--identity-uri",
            &identity_uri,
            "--observed-at",
            "1700000000",
        ]));
        let node_heartbeat = trimmed_tvmd(&[
            "public",
            "evidence",
            "node",
            "heartbeat",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--first-block",
            "0",
            "--last-block",
            "9",
            "--heartbeat-count",
            "10",
        ]);
        let heartbeat_file = data_dir.join(format!("{role}-{port}-heartbeats.records"));
        let heartbeat_records = (0..10)
            .map(|block| {
                format!("node_heartbeat_observation={role},{address},{operator_id},{block}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&heartbeat_file, heartbeat_records)
            .expect("node heartbeat file must be written");
        let heartbeat_file_text = heartbeat_file.to_string_lossy().into_owned();
        let node_heartbeat_from_file = trimmed_tvmd(&[
            "public",
            "evidence",
            "node",
            "heartbeat-file",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--heartbeat-file",
            &heartbeat_file_text,
        ]);
        assert_eq!(node_heartbeat_from_file, node_heartbeat);
        node_lines.push(node_heartbeat);
        let peer_id = PeerId::random().to_string();
        let listen_address = format!("/dns/{host}/tcp/{port}");
        let observation = trimmed_tvmd(&[
            "public",
            "evidence",
            "network",
            "observation",
            "--operator-id",
            operator_id,
            "--peer-id",
            &peer_id,
            "--listen-address",
            &listen_address,
            "--observed-at",
            "1700000000",
            "--gossip-topics",
            "5",
            "--request-response-protocols",
            "3",
            "--bootstrap-peers",
            "2",
            "--max-transmit-bytes",
            "1048576",
            "--request-timeout-seconds",
            "10",
            "--max-concurrent-streams",
            "128",
            "--idle-timeout-seconds",
            "60",
        ]);
        network_roots.push(network_observation_root(&observation).to_owned());
        network_lines.push(observation);
    }
    let network_root_csv = network_roots.join(",");
    let network_summary = trimmed_tvmd(&[
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
        &network_root_csv,
    ]);
    artifact_lines.push(trimmed_tvmd(&[
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
        "https://evidence.tensorvm.net/tensorvm/network-runtime.json",
        "--record-roots",
        &network_root_csv,
    ]));
    let network_record_file = data_dir.join("network-runtime.records");
    std::fs::write(&network_record_file, network_lines.join("\n"))
        .expect("network runtime record file must be written");
    let network_record_file_text = network_record_file.to_string_lossy().into_owned();
    let network_summary_from_file = trimmed_tvmd(&[
        "public",
        "evidence",
        "record",
        "summary-file",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--record-file",
        &network_record_file_text,
    ]);
    assert_eq!(network_summary_from_file, network_summary);
    let network_artifact_from_file = trimmed_tvmd(&[
        "public",
        "evidence",
        "record",
        "artifact-file",
        "--kind",
        "network-runtime",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--artifact-uri",
        "https://evidence.tensorvm.net/tensorvm/network-runtime.json",
        "--record-file",
        &network_record_file_text,
    ]);
    assert_eq!(
        network_artifact_from_file,
        artifact_lines
            .last()
            .expect("network runtime artifact line must exist")
            .as_str()
    );

    let run_window = trimmed_tvmd(&[
        "public",
        "evidence",
        "run",
        "window",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--started-at",
        "1700000000",
        "--ended-at",
        "1700000060",
        "--observed-blocks",
        "10",
    ]);
    let run_window_record_file = data_dir.join("run-window.records");
    let run_window_records = (0..10)
        .map(|block| {
            let timestamp = if block == 9 {
                1_700_000_060
            } else {
                1_700_000_000 + block * 6
            };
            format!("run_window_observation={block},{timestamp}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&run_window_record_file, run_window_records)
        .expect("run window record file must be written");
    let run_window_record_file_text = run_window_record_file.to_string_lossy().into_owned();
    let run_window_from_file = trimmed_tvmd(&[
        "public",
        "evidence",
        "run",
        "window-file",
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--block-observation-file",
        &run_window_record_file_text,
    ]);
    assert_eq!(run_window_from_file, run_window);

    let mut service_lines = Vec::new();
    let mut service_content_lines = Vec::new();
    for (kind, endpoint_id, health_url, content_url, content_path, content_root) in [
        (
            "rpc",
            "12".repeat(32),
            "https://rpc.tensorvm.net/health",
            "https://rpc.tensorvm.net/chain/head",
            "/chain/head",
            "a1".repeat(32),
        ),
        (
            "explorer",
            "13".repeat(32),
            "https://explorer.tensorvm.net/health",
            "https://explorer.tensorvm.net/explorer",
            "/explorer",
            "a2".repeat(32),
        ),
        (
            "faucet",
            "14".repeat(32),
            "https://faucet.tensorvm.net/health",
            "https://faucet.tensorvm.net/faucet/page",
            "/faucet/page",
            "a3".repeat(32),
        ),
        (
            "telemetry",
            "15".repeat(32),
            "https://telemetry.tensorvm.net/health",
            "https://telemetry.tensorvm.net/telemetry/dashboard",
            "/telemetry/dashboard",
            "a4".repeat(32),
        ),
    ] {
        let service_health = trimmed_tvmd(&[
            "public",
            "evidence",
            "service",
            "health",
            "--kind",
            kind,
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            health_url,
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
        let health_record_file = data_dir.join(format!("{kind}-health.records"));
        let health_records = (0..10)
            .map(|block| format!("service_health_observation={block},reachable"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&health_record_file, health_records)
            .expect("service health record file must be written");
        let health_record_file_text = health_record_file.to_string_lossy().into_owned();
        let service_health_from_file = trimmed_tvmd(&[
            "public",
            "evidence",
            "service",
            "health-file",
            "--kind",
            kind,
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            health_url,
            "--health-path",
            "/health",
            "--observation-file",
            &health_record_file_text,
        ]);
        assert_eq!(service_health_from_file, service_health);
        service_lines.push(service_health);
        service_content_lines.push(trimmed_tvmd(&[
            "public",
            "evidence",
            "service",
            "content",
            "--kind",
            kind,
            "--endpoint-id",
            &endpoint_id,
            "--public-url",
            content_url,
            "--content-path",
            content_path,
            "--content-root",
            &content_root,
            "--observed-at",
            "1700000000",
            "--min-content-bytes",
            "64",
        ]));
    }

    let manifest = format!(
        "\
version=tensor-vm-public-testnet-evidence-v1
{publication}
{auditor}
{}
{}
operator_identity_attestation_records=3
{}
{}
{network_summary}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
{run_window}
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
cuda_verified_miner_count=2
cuda_graph_execution_receipts=1
validator_vrf_lifecycle_records=20
{}
{}
{}
",
        artifact_lines.join("\n"),
        summary_lines.join("\n"),
        operator_lines.join("\n"),
        network_lines.join("\n"),
        node_lines.join("\n"),
        service_lines.join("\n"),
        service_content_lines.join("\n"),
    );
    std::fs::write(&manifest_path, manifest).expect("generated evidence manifest must be written");

    let report = run_tvmd(&["public", "evidence", "validate", &manifest_path_text]);
    assert_eq!(stdout_value(&report, "public_evidence_full_spec"), "false");
    assert_eq!(stdout_value(&report, "public_criterion"), "false");
    assert_eq!(stdout_value(&report, "independently_checkable"), "true");
    assert_eq!(stdout_value(&report, "published_evidence_bundle"), "true");
    assert_eq!(stdout_value(&report, "supporting_record_artifacts"), "true");
    assert_eq!(
        stdout_value(&report, "network_runtime_observations"),
        "true"
    );
    assert_eq!(stdout_value(&report, "randomness_beacon_evidence"), "true");
    assert_eq!(stdout_value(&report, "deployed_public_services"), "true");
    assert_eq!(
        stdout_value(&report, "deployed_public_service_content"),
        "true"
    );
    assert_eq!(stdout_value(&report, "production_libp2p_runtime"), "true");
    assert_eq!(stdout_value(&report, "cuda_verified_miner_count"), "2");
    assert_eq!(stdout_value(&report, "cuda_miner_evidence"), "true");
    assert_eq!(
        stdout_value(&report, "validator_vrf_lifecycle_records"),
        "20"
    );
    assert_eq!(
        stdout_value(&report, "validator_vrf_lifecycle_record_evidence"),
        "true"
    );
    assert_eq!(stdout_value(&report, "detection_measurement_records"), "1");
    assert_eq!(
        stdout_value(&report, "deployed_detection_measurement_evidence"),
        "true"
    );
    assert_eq!(stdout_value(&report, "required_run_duration"), "false");
    assert_eq!(stdout_value(&report, "required_block_count"), "false");
    assert_eq!(stdout_value(&report, "required_miners"), "false");
    assert_eq!(stdout_value(&report, "required_validators"), "false");

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
