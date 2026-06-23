use super::*;

#[test]
fn public_testnet_preflight_manifest_reports_launch_readiness() {
    let manifest = complete_public_preflight_manifest_text();
    let plan = parse_public_testnet_preflight_manifest(&manifest).unwrap();
    let report = plan.evaluate(ChainParams::default().block_time_seconds);

    assert_eq!(report.miner_count, 10);
    assert_eq!(report.validator_count, 5);
    assert_eq!(report.required_blocks, 100_800);
    assert!(report.has_required_miners);
    assert!(report.has_required_validators);
    assert!(report.has_positive_stakes);
    assert!(report.has_funded_faucet);
    assert!(report.has_cuda_kernels_available);
    assert_eq!(report.cuda_ready_miner_count, 10);
    assert!(report.has_cuda_ready_miners);
    assert_eq!(report.libp2p_ready_node_count, 15);
    assert!(report.has_libp2p_ready_nodes);
    assert!(report.has_production_libp2p_runtime);
    assert!(report.has_rpc_service_plan);
    assert!(report.has_explorer_service_plan);
    assert!(report.has_faucet_service_plan);
    assert!(report.has_telemetry_service_plan);
    assert!(report.has_public_service_content_plan);
    assert!(report.has_public_service_plan);
    assert!(report.local_shape_ready);
    assert!(report.deployment_plan_ready);
    assert!(report.can_start_public_run);

    let duplicate_service_endpoint = manifest.replace(
        &manifest_hash(b"test", b"explorer-service"),
        &manifest_hash(b"test", b"rpc-service"),
    );
    let duplicate_service_endpoint_report =
        parse_public_testnet_preflight_manifest(&duplicate_service_endpoint)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(duplicate_service_endpoint_report.has_rpc_service_plan);
    assert!(duplicate_service_endpoint_report.has_explorer_service_plan);
    assert!(duplicate_service_endpoint_report.has_public_service_content_plan);
    assert!(!duplicate_service_endpoint_report.has_public_service_plan);
    assert!(!duplicate_service_endpoint_report.deployment_plan_ready);
    assert!(!duplicate_service_endpoint_report.can_start_public_run);

    let reused_rpc_service_urls = manifest
        .replace(
            "https://explorer.tensorvm.net/health,/health",
            "https://rpc.tensorvm.net/health,/health",
        )
        .replace(
            "https://explorer.tensorvm.net/explorer,/explorer",
            "https://rpc.tensorvm.net/explorer,/explorer",
        );
    let reused_rpc_service_url_report =
        parse_public_testnet_preflight_manifest(&reused_rpc_service_urls)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(reused_rpc_service_url_report.has_rpc_service_plan);
    assert!(reused_rpc_service_url_report.has_explorer_service_plan);
    assert!(reused_rpc_service_url_report.has_public_service_content_plan);
    assert!(!reused_rpc_service_url_report.has_public_service_plan);
    assert!(!reused_rpc_service_url_report.deployment_plan_ready);
    assert!(!reused_rpc_service_url_report.can_start_public_run);

    let duplicate_rpc_service_plan = format!(
        "{manifest}service=rpc,{},https://rpc-backup.tensorvm.net/health,/health,https://rpc-backup.tensorvm.net/chain/head,/chain/head,true,true\n",
        manifest_hash(b"test", b"rpc-backup-service")
    );
    let duplicate_rpc_service_plan_report =
        parse_public_testnet_preflight_manifest(&duplicate_rpc_service_plan)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(duplicate_rpc_service_plan_report.has_rpc_service_plan);
    assert!(duplicate_rpc_service_plan_report.has_public_service_content_plan);
    assert!(!duplicate_rpc_service_plan_report.has_public_service_plan);
    assert!(!duplicate_rpc_service_plan_report.deployment_plan_ready);
    assert!(!duplicate_rpc_service_plan_report.can_start_public_run);

    let mut missing_service_plan = plan.clone();
    missing_service_plan
        .services
        .retain(|service| service.kind != PublicServiceKind::Explorer);
    assert!(!missing_service_plan.has_distinct_ready_service_endpoint_ids());

    let rpc = plan
        .services
        .iter()
        .find(|service| service.kind == PublicServiceKind::Rpc)
        .unwrap();
    assert_eq!(public_https_host("https:///missing-host"), None);
    assert_eq!(
        public_https_host("https://rpc.tensorvm.net@localhost/health"),
        None
    );
    assert_eq!(
        public_https_host("https://rpc.tensorvm.net:bad/health"),
        None
    );
    assert_eq!(public_https_host("https://node/health"), None);
    assert_eq!(
        public_https_host("https://bad_host.tensorvm.net/health"),
        None
    );
    assert_eq!(public_https_host("https://-bad.tensorvm.net/health"), None);
    assert_eq!(
        public_https_host("https://rpc.tensorvm.net\\evil/health"),
        None
    );
    assert_eq!(public_https_host(" https://rpc.tensorvm.net/health"), None);
    assert_eq!(public_https_host("https://rpc.tensorvm.net/health "), None);
    assert_eq!(public_https_host("https://rpc.tensorvm.net/health\n"), None);
    assert_eq!(public_https_host("https://rpc.tensorvm.net/bad path"), None);
    assert_eq!(public_https_host("https://rpc.tensorvm.net /health"), None);
    assert_eq!(public_https_host("https://rpc[bad]/health"), None);
    assert_eq!(public_https_host("https://[not-ip]/health"), None);
    assert_eq!(
        public_https_host("https://2001:4860:4860::8888/health"),
        None
    );
    assert_eq!(
        public_https_host("https://rpc.tensorvm.net:443/health"),
        Some("rpc.tensorvm.net")
    );
    assert_eq!(
        public_https_path("https://rpc.tensorvm.net/health?probe=1"),
        None
    );
    assert_eq!(
        public_https_path("https://rpc.tensorvm.net/health#probe"),
        None
    );
    assert!(public_https_authorities_match(
        "https://rpc.tensorvm.net:443/health",
        "https://rpc.tensorvm.net/chain/head"
    ));
    assert!(!public_https_authorities_match(
        "https://rpc.tensorvm.net:444/health",
        "https://rpc.tensorvm.net/chain/head"
    ));
    assert!(!public_https_authorities_match(
        "https://rpc.tensorvm.net/health",
        "http://rpc.tensorvm.net/chain/head"
    ));
    assert!(!public_https_authorities_match(
        "https://rpc.tensorvm.net/health",
        "https://rpc-content.tensorvm.net/chain/head"
    ));
    assert!(public_https_authorities_match(
        "https://[2001:4860:4860::8888]/health",
        "https://[2001:4860:4860:0:0:0:0:8888]/chain/head"
    ));
    assert_eq!(public_https_host("https://[::1]:443/health"), Some("::1"));
    assert_eq!(
        public_https_host("https://[2001:4860:4860::8888]:443/health"),
        Some("2001:4860:4860::8888")
    );
    assert!(rpc.is_public_https_endpoint());
    assert!(rpc.has_public_content_surface());
    assert!(rpc.is_ready_for_public_run());
    let mut http_rpc = rpc.clone();
    http_rpc.public_url = String::from("http://rpc.tensorvm.net/health");
    assert!(!http_rpc.is_public_https_endpoint());

    let mut mismatched_health_path_rpc = rpc.clone();
    mismatched_health_path_rpc.public_url = String::from("https://rpc.tensorvm.net/wrong");
    assert!(!mismatched_health_path_rpc.is_ready_for_public_run());

    let mut root_health_path_rpc = rpc.clone();
    root_health_path_rpc.public_url = String::from("https://rpc.tensorvm.net/");
    assert!(!root_health_path_rpc.is_ready_for_public_run());

    let mut wrong_content_path_rpc = rpc.clone();
    wrong_content_path_rpc.content_url = String::from("https://rpc.tensorvm.net/wrong");
    assert!(!wrong_content_path_rpc.has_public_content_surface());
    assert!(!wrong_content_path_rpc.is_ready_for_public_run());

    let mut root_content_path_rpc = rpc.clone();
    root_content_path_rpc.content_url = String::from("https://rpc.tensorvm.net/");
    assert!(!root_content_path_rpc.has_public_content_surface());
    assert!(!root_content_path_rpc.is_ready_for_public_run());

    let mut http_content_rpc = rpc.clone();
    http_content_rpc.content_url = String::from("http://rpc.tensorvm.net/chain/head");
    assert!(!http_content_rpc.has_public_content_surface());
    assert!(!http_content_rpc.is_ready_for_public_run());

    let mut ipv6_loopback_rpc = rpc.clone();
    ipv6_loopback_rpc.public_url = String::from("https://[::1]:443/health");
    assert!(!ipv6_loopback_rpc.is_public_https_endpoint());

    let mut private_ip_rpc = rpc.clone();
    private_ip_rpc.public_url = String::from("https://10.0.0.5/health");
    assert!(!private_ip_rpc.is_public_https_endpoint());
    for host in [
        "100.64.0.1",
        "192.0.0.1",
        "192.0.2.10",
        "198.18.0.1",
        "198.51.100.10",
        "203.0.113.10",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "2001:db8::1",
        "ff02::1",
    ] {
        assert!(!public_host_is_external(host));
    }
    assert!(public_host_is_external("8.8.8.8"));
    assert!(public_host_is_external("2001:4860:4860::8888"));
    assert!(!public_host_is_external(""));
    assert!(!public_host_is_external("node"));
    assert!(!public_host_is_external("bad..tensorvm.net"));
    assert!(!public_host_is_external("123.456"));
    for host in [
        "example.com",
        "www.example.net",
        "rpc.example.test",
        "rpc.tensorvm.example",
        "operator.invalid",
    ] {
        assert!(!public_host_is_external(host));
    }

    let local_rpc = manifest.replace(
        "https://rpc.tensorvm.net/health",
        "https://localhost:8545/health",
    );
    let local_rpc_report = parse_public_testnet_preflight_manifest(&local_rpc)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!local_rpc_report.has_rpc_service_plan);
    assert!(!local_rpc_report.has_public_service_plan);
    assert!(!local_rpc_report.can_start_public_run);

    let obfuscated_local_rpc = manifest.replace(
        "https://rpc.tensorvm.net/health",
        "https://rpc.tensorvm.net@localhost/health",
    );
    let obfuscated_local_rpc_report =
        parse_public_testnet_preflight_manifest(&obfuscated_local_rpc)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(!obfuscated_local_rpc_report.has_rpc_service_plan);
    assert!(!obfuscated_local_rpc_report.has_public_service_plan);
    assert!(!obfuscated_local_rpc_report.can_start_public_run);

    let root_health_path = manifest.replace(
        "https://rpc.tensorvm.net/health,/health",
        "https://rpc.tensorvm.net/,/health",
    );
    let root_health_path_report = parse_public_testnet_preflight_manifest(&root_health_path)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!root_health_path_report.has_rpc_service_plan);
    assert!(!root_health_path_report.has_public_service_plan);
    assert!(!root_health_path_report.can_start_public_run);

    let bad_content_path = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        "https://rpc.tensorvm.net/wrong,/chain/head",
    );
    let bad_content_path_report = parse_public_testnet_preflight_manifest(&bad_content_path)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!bad_content_path_report.has_rpc_service_plan);
    assert!(!bad_content_path_report.has_public_service_content_plan);
    assert!(!bad_content_path_report.has_public_service_plan);
    assert!(!bad_content_path_report.can_start_public_run);

    let root_content_path = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        "https://rpc.tensorvm.net/,/chain/head",
    );
    let root_content_path_report = parse_public_testnet_preflight_manifest(&root_content_path)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!root_content_path_report.has_rpc_service_plan);
    assert!(!root_content_path_report.has_public_service_content_plan);
    assert!(!root_content_path_report.has_public_service_plan);
    assert!(!root_content_path_report.can_start_public_run);

    let health_url_with_space = manifest.replace(
        "https://rpc.tensorvm.net/health,/health",
        "https://rpc.tensorvm.net/health ,/health",
    );
    assert!(parse_public_testnet_preflight_manifest(&health_url_with_space).is_err());

    let content_url_with_space = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        " https://rpc.tensorvm.net/chain/head,/chain/head",
    );
    assert!(parse_public_testnet_preflight_manifest(&content_url_with_space).is_err());

    let health_query = manifest.replace(
        "https://rpc.tensorvm.net/health,/health",
        "https://rpc.tensorvm.net/health?probe=1,/health",
    );
    let health_query_report = parse_public_testnet_preflight_manifest(&health_query)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!health_query_report.has_rpc_service_plan);
    assert!(!health_query_report.has_public_service_plan);
    assert!(!health_query_report.can_start_public_run);

    let content_fragment = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        "https://rpc.tensorvm.net/chain/head#head,/chain/head",
    );
    let content_fragment_report = parse_public_testnet_preflight_manifest(&content_fragment)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!content_fragment_report.has_rpc_service_plan);
    assert!(!content_fragment_report.has_public_service_content_plan);
    assert!(!content_fragment_report.has_public_service_plan);
    assert!(!content_fragment_report.can_start_public_run);

    let mismatched_content_authority = manifest.replace(
        "https://rpc.tensorvm.net/chain/head,/chain/head",
        "https://rpc-content.tensorvm.net/chain/head,/chain/head",
    );
    let mismatched_content_authority_report =
        parse_public_testnet_preflight_manifest(&mismatched_content_authority)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(!mismatched_content_authority_report.has_rpc_service_plan);
    assert!(!mismatched_content_authority_report.has_public_service_content_plan);
    assert!(!mismatched_content_authority_report.has_public_service_plan);
    assert!(!mismatched_content_authority_report.can_start_public_run);

    let no_cuda = manifest.replace(
        "cuda_kernels_available=true",
        "cuda_kernels_available=false",
    );
    let no_cuda_report = parse_public_testnet_preflight_manifest(&no_cuda)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(no_cuda_report.local_shape_ready);
    assert!(!no_cuda_report.has_cuda_kernels_available);
    assert!(!no_cuda_report.has_cuda_ready_miners);
    assert!(!no_cuda_report.deployment_plan_ready);
    assert!(!no_cuda_report.can_start_public_run);

    let undercounted_cuda_miners =
        manifest.replace("cuda_ready_miner_count=10", "cuda_ready_miner_count=9");
    let undercounted_cuda_miner_report =
        parse_public_testnet_preflight_manifest(&undercounted_cuda_miners)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(undercounted_cuda_miner_report.has_cuda_kernels_available);
    assert_eq!(undercounted_cuda_miner_report.cuda_ready_miner_count, 9);
    assert!(!undercounted_cuda_miner_report.has_cuda_ready_miners);
    assert!(!undercounted_cuda_miner_report.deployment_plan_ready);
    assert!(!undercounted_cuda_miner_report.can_start_public_run);

    let undercounted_libp2p_nodes =
        manifest.replace("libp2p_ready_node_count=15", "libp2p_ready_node_count=14");
    let undercounted_libp2p_node_report =
        parse_public_testnet_preflight_manifest(&undercounted_libp2p_nodes)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
    assert!(undercounted_libp2p_node_report.has_production_libp2p_runtime);
    assert_eq!(undercounted_libp2p_node_report.libp2p_ready_node_count, 14);
    assert!(!undercounted_libp2p_node_report.has_libp2p_ready_nodes);
    assert!(!undercounted_libp2p_node_report.deployment_plan_ready);
    assert!(!undercounted_libp2p_node_report.can_start_public_run);

    let no_auth = manifest.replace(
        "https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true",
        "https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,false,true",
    );
    let no_auth_report = parse_public_testnet_preflight_manifest(&no_auth)
        .unwrap()
        .evaluate(ChainParams::default().block_time_seconds);
    assert!(!no_auth_report.has_telemetry_service_plan);
    assert!(!no_auth_report.can_start_public_run);
}

#[test]
fn deployed_public_testnet_preflight_example_rejects_placeholder_domains() {
    let manifest =
        include_str!("../../../../../deploy/tensorvm/manifests/public-testnet.preflight.example");
    assert_public_testnet_preflight_manifest_is_pending(manifest);
}

#[test]
fn docs_public_testnet_preflight_manifest_rejects_placeholder_domains() {
    let manifest = include_str!("../../../../../docs/tensorvm/public-testnet.preflight");
    assert_public_testnet_preflight_manifest_is_pending(manifest);
}

fn assert_public_testnet_preflight_manifest_is_pending(manifest: &str) {
    let plan = parse_public_testnet_preflight_manifest(manifest).unwrap();
    let report = plan.evaluate(ChainParams::default().block_time_seconds);

    assert!(report.local_shape_ready);
    assert!(!report.deployment_plan_ready);
    assert!(!report.can_start_public_run);
}

#[test]
fn public_testnet_preflight_manifest_rejects_malformed_input() {
    let manifest = complete_public_preflight_manifest_text();
    let cases = [
        manifest_without_line(&manifest, "version="),
        manifest.replace(
            PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
            "tensor-vm-public-testnet-preflight-v0",
        ),
        manifest_without_line(&manifest, "miner_count="),
        manifest.replace("miner_count=10", "miner_count=abc"),
        manifest.replace(
            "cuda_kernels_available=true",
            "cuda_kernels_available=maybe",
        ),
        manifest_without_line(&manifest, "cuda_ready_miner_count="),
        manifest.replace("cuda_ready_miner_count=10", "cuda_ready_miner_count=abc"),
        manifest_without_line(&manifest, "libp2p_ready_node_count="),
        manifest.replace("libp2p_ready_node_count=15", "libp2p_ready_node_count=abc"),
        format!("{manifest}\nminer_count=10"),
        manifest.replace("miner_count=", " miner_count="),
        manifest.replace("miner_count=", "miner_count ="),
        manifest.replace("miner_count=10", "miner_count=10 "),
        manifest.replace(
            "cuda_kernels_available=true",
            "cuda_kernels_available= true",
        ),
        manifest.replace("service=rpc", "service=archive"),
        manifest.replace("service=rpc,", "service=rpc ,"),
        manifest.replace(",true,true", ", true,true"),
        manifest.replace(
            "service=rpc,",
            "service=rpc,too,few,fields\n# removed original service=",
        ),
        manifest.replace("service=rpc,", "service=rpc,zz"),
        format!("{manifest}\nunknown_field=1\n"),
        manifest.replace("service=rpc", "malformed-line"),
    ];

    for case in cases {
        assert!(parse_public_testnet_preflight_manifest(&case).is_err());
    }
}
