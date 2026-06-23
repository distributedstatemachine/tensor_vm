use super::*;

#[test]
fn public_testnet_run_evidence_requires_production_runtime_and_reachable_services() {
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let mut run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::miner(
                address(b"miner-b"),
                hash_bytes(b"test", &[b"miner-b-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::validator(
                address(b"validator-a"),
                hash_bytes(b"test", &[b"validator-a-operator"]),
                0,
                9,
                10,
            ),
        ],
        network_runtime: production_runtime_evidence(),
        services: deployed_public_services(9),
        service_content: deployed_public_service_content(),
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 10,
        checked_receipts: 20,
        available_receipts: 19,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
        cuda_verified_miner_count: 2,
        cuda_graph_execution_receipts: 1,
        validator_vrf_lifecycle_records: 40,
        detection_measurement_records: 1,
    };

    assert!(run.services[0].covers_run(0));
    let complete = run.evaluate(&criteria, 6, true);
    assert!(complete.has_production_libp2p_runtime);
    assert!(complete.has_deployed_rpc_service);
    assert!(complete.has_deployed_explorer_service);
    assert!(complete.has_deployed_faucet_service);
    assert!(complete.has_deployed_telemetry_service);
    assert!(complete.has_deployed_public_service_content);
    assert!(complete.has_deployed_public_services);
    assert!(complete.public_criterion_met);

    run.services.push(public_service(
        PublicServiceKind::Rpc,
        b"extra-rpc-service",
        0,
        9,
    ));
    let extra_rpc_service = run.evaluate(&criteria, 6, true);
    assert!(!extra_rpc_service.has_deployed_rpc_service);
    assert!(!extra_rpc_service.has_deployed_public_service_content);
    assert!(!extra_rpc_service.has_deployed_public_services);
    assert!(!extra_rpc_service.public_criterion_met);
    run.services = deployed_public_services(9);

    run.service_content.push(public_service_content(
        PublicServiceKind::Rpc,
        b"extra-rpc-service",
    ));
    let extra_rpc_content = run.evaluate(&criteria, 6, true);
    assert!(!extra_rpc_content.has_deployed_rpc_service);
    assert!(!extra_rpc_content.has_deployed_public_service_content);
    assert!(!extra_rpc_content.has_deployed_public_services);
    assert!(!extra_rpc_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.services[1] = PublicServiceEvidence::new(
        PublicServiceKind::Explorer,
        PublicServiceEndpoint::new(
            run.services[0].endpoint_id,
            public_service_url(PublicServiceKind::Explorer),
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    run.service_content[1] = PublicServiceContentEvidence::new(
        PublicServiceKind::Explorer,
        run.services[0].endpoint_id,
        public_service_content_url(PublicServiceKind::Explorer),
        public_service_content_path(PublicServiceKind::Explorer),
        hash_bytes(b"test", &[b"explorer-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let duplicate_service_endpoint = run.evaluate(&criteria, 6, true);
    assert!(duplicate_service_endpoint.has_deployed_explorer_service);
    assert!(duplicate_service_endpoint.has_deployed_public_service_content);
    assert!(!duplicate_service_endpoint.has_deployed_public_services);
    assert!(!duplicate_service_endpoint.public_criterion_met);
    run.services = deployed_public_services(9);
    run.service_content = deployed_public_service_content();

    run.services[1] = PublicServiceEvidence::new(
        PublicServiceKind::Explorer,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"explorer-service"]),
            public_service_url(PublicServiceKind::Rpc),
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    run.service_content[1] = PublicServiceContentEvidence::new(
        PublicServiceKind::Explorer,
        hash_bytes(b"test", &[b"explorer-service"]),
        "https://rpc.tensorvm.net/explorer",
        public_service_content_path(PublicServiceKind::Explorer),
        hash_bytes(b"test", &[b"explorer-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let duplicate_service_url = run.evaluate(&criteria, 6, true);
    assert!(duplicate_service_url.has_deployed_explorer_service);
    assert!(duplicate_service_url.has_deployed_public_service_content);
    assert!(!duplicate_service_url.has_deployed_public_services);
    assert!(!duplicate_service_url.public_criterion_met);
    run.services = deployed_public_services(9);
    run.service_content = deployed_public_service_content();

    run.service_content[1] = PublicServiceContentEvidence::new(
        PublicServiceKind::Explorer,
        hash_bytes(b"test", &[b"explorer-service"]),
        public_service_content_url(PublicServiceKind::Explorer),
        public_service_content_path(PublicServiceKind::Explorer),
        run.service_content[0].content_root,
        1_700_000_000,
        64,
    );
    let duplicate_service_content_root = run.evaluate(&criteria, 6, true);
    assert!(duplicate_service_content_root.has_deployed_explorer_service);
    assert!(!duplicate_service_content_root.has_deployed_public_service_content);
    assert!(!duplicate_service_content_root.has_deployed_public_services);
    assert!(!duplicate_service_content_root.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0].content_signature = [8; 32];
    let tampered_rpc_content = run.evaluate(&criteria, 6, true);
    assert!(!tampered_rpc_content.has_deployed_rpc_service);
    assert!(!tampered_rpc_content.has_deployed_public_service_content);
    assert!(!tampered_rpc_content.has_deployed_public_services);
    assert!(!tampered_rpc_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0].public_url = String::from("https://localhost/chain/head");
    let local_rpc_content = run.evaluate(&criteria, 6, true);
    assert!(!local_rpc_content.has_deployed_rpc_service);
    assert!(!local_rpc_content.has_deployed_public_service_content);
    assert!(!local_rpc_content.has_deployed_public_services);
    assert!(!local_rpc_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        "https://rpc.tensorvm.net@localhost/chain/head",
        public_service_content_path(PublicServiceKind::Rpc),
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let obfuscated_local_rpc_content = run.evaluate(&criteria, 6, true);
    assert!(!obfuscated_local_rpc_content.has_deployed_rpc_service);
    assert!(!obfuscated_local_rpc_content.has_deployed_public_service_content);
    assert!(!obfuscated_local_rpc_content.has_deployed_public_services);
    assert!(!obfuscated_local_rpc_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] =
        public_service_content(PublicServiceKind::Rpc, b"independent-rpc-content");
    let mismatched_rpc_content_endpoint = run.evaluate(&criteria, 6, true);
    assert!(!mismatched_rpc_content_endpoint.has_deployed_rpc_service);
    assert!(!mismatched_rpc_content_endpoint.has_deployed_public_service_content);
    assert!(!mismatched_rpc_content_endpoint.has_deployed_public_services);
    assert!(!mismatched_rpc_content_endpoint.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        "https://rpc-content.tensorvm.net/chain/head",
        public_service_content_path(PublicServiceKind::Rpc),
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let mismatched_rpc_content_authority = run.evaluate(&criteria, 6, true);
    assert!(!mismatched_rpc_content_authority.has_deployed_rpc_service);
    assert!(!mismatched_rpc_content_authority.has_deployed_public_service_content);
    assert!(!mismatched_rpc_content_authority.has_deployed_public_services);
    assert!(!mismatched_rpc_content_authority.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        "https://rpc.tensorvm.net/wrong",
        "/wrong",
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let wrong_rpc_content_path = run.evaluate(&criteria, 6, true);
    assert!(!wrong_rpc_content_path.has_deployed_rpc_service);
    assert!(!wrong_rpc_content_path.has_deployed_public_service_content);
    assert!(!wrong_rpc_content_path.has_deployed_public_services);
    assert!(!wrong_rpc_content_path.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        "https://rpc.tensorvm.net/chain/head?variant=raw",
        public_service_content_path(PublicServiceKind::Rpc),
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_000,
        64,
    );
    let rpc_content_query = run.evaluate(&criteria, 6, true);
    assert!(!rpc_content_query.has_deployed_rpc_service);
    assert!(!rpc_content_query.has_deployed_public_service_content);
    assert!(!rpc_content_query.has_deployed_public_services);
    assert!(!rpc_content_query.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        public_service_content_url(PublicServiceKind::Rpc),
        public_service_content_path(PublicServiceKind::Rpc),
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_061,
        64,
    );
    let content_after_run = run.evaluate(&criteria, 6, true);
    assert!(!content_after_run.has_deployed_rpc_service);
    assert!(!content_after_run.has_deployed_public_service_content);
    assert!(!content_after_run.has_deployed_public_services);
    assert!(!content_after_run.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content[0] = PublicServiceContentEvidence::new(
        PublicServiceKind::Rpc,
        hash_bytes(b"test", &[b"rpc-service"]),
        public_service_content_url(PublicServiceKind::Rpc),
        public_service_content_path(PublicServiceKind::Rpc),
        hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
        1_700_000_000,
        PUBLIC_SERVICE_MIN_CONTENT_BYTES - 1,
    );
    let undersized_rpc_content = run.evaluate(&criteria, 6, true);
    assert!(!undersized_rpc_content.has_deployed_rpc_service);
    assert!(!undersized_rpc_content.has_deployed_public_service_content);
    assert!(!undersized_rpc_content.has_deployed_public_services);
    assert!(!undersized_rpc_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.service_content
        .retain(|content| content.kind != PublicServiceKind::Faucet);
    let missing_faucet_content = run.evaluate(&criteria, 6, true);
    assert!(!missing_faucet_content.has_deployed_faucet_service);
    assert!(!missing_faucet_content.has_deployed_public_service_content);
    assert!(!missing_faucet_content.has_deployed_public_services);
    assert!(!missing_faucet_content.public_criterion_met);
    run.service_content = deployed_public_service_content();

    run.services[0].health_check_signature = [8; 32];
    let tampered_rpc_health = run.evaluate(&criteria, 6, true);
    assert!(!tampered_rpc_health.has_deployed_rpc_service);
    assert!(!tampered_rpc_health.has_deployed_public_services);
    assert!(!tampered_rpc_health.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"local-rpc-service"]),
            "https://localhost/health",
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    let local_rpc_url = run.evaluate(&criteria, 6, true);
    assert!(!local_rpc_url.has_deployed_rpc_service);
    assert!(!local_rpc_url.has_deployed_public_services);
    assert!(!local_rpc_url.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"obfuscated-local-rpc-service"]),
            "https://rpc.tensorvm.net@localhost/health",
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    let obfuscated_local_rpc_url = run.evaluate(&criteria, 6, true);
    assert!(!obfuscated_local_rpc_url.has_deployed_rpc_service);
    assert!(!obfuscated_local_rpc_url.has_deployed_public_services);
    assert!(!obfuscated_local_rpc_url.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"bad-health-path-rpc-service"]),
            public_service_url(PublicServiceKind::Rpc),
            "health",
        ),
        0,
        9,
        10,
        10,
    );
    let bad_health_path = run.evaluate(&criteria, 6, true);
    assert!(!bad_health_path.has_deployed_rpc_service);
    assert!(!bad_health_path.has_deployed_public_services);
    assert!(!bad_health_path.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"rpc-service"]),
            "https://rpc.tensorvm.net/health?probe=1",
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    let rpc_health_query = run.evaluate(&criteria, 6, true);
    assert!(!rpc_health_query.has_deployed_rpc_service);
    assert!(!rpc_health_query.has_deployed_public_services);
    assert!(!rpc_health_query.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"rpc-service"]),
            public_service_url(PublicServiceKind::Rpc),
            "/health",
        ),
        0,
        9,
        9,
        10,
    );
    let sparse_rpc_reachability = run.evaluate(&criteria, 6, true);
    assert!(!sparse_rpc_reachability.has_deployed_rpc_service);
    assert!(!sparse_rpc_reachability.has_deployed_public_services);
    assert!(!sparse_rpc_reachability.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"rpc-service"]),
            public_service_url(PublicServiceKind::Rpc),
            "/health",
        ),
        0,
        9,
        10,
        9,
    );
    let sparse_rpc_health_signatures = run.evaluate(&criteria, 6, true);
    assert!(!sparse_rpc_health_signatures.has_deployed_rpc_service);
    assert!(!sparse_rpc_health_signatures.has_deployed_public_services);
    assert!(!sparse_rpc_health_signatures.public_criterion_met);
    run.services = deployed_public_services(9);

    run.services[0] = PublicServiceEvidence::new(
        PublicServiceKind::Rpc,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[b"rpc-service"]),
            public_service_url(PublicServiceKind::Rpc),
            "/health",
        ),
        0,
        9,
        11,
        10,
    );
    assert!(!run.services[0].has_reachable_endpoint_proof());
    let overreported_rpc_reachability = run.evaluate(&criteria, 6, true);
    assert!(!overreported_rpc_reachability.has_deployed_rpc_service);
    assert!(!overreported_rpc_reachability.has_deployed_public_services);
    assert!(!overreported_rpc_reachability.public_criterion_met);
    run.services = deployed_public_services(9);

    run.network_runtime.request_response_observed = false;
    let no_request_response = run.evaluate(&criteria, 6, true);
    assert!(!no_request_response.has_production_libp2p_runtime);
    assert!(no_request_response.has_deployed_public_services);
    assert!(!no_request_response.public_criterion_met);
    run.network_runtime = production_runtime_evidence();

    run.services
        .retain(|service| service.kind != PublicServiceKind::Telemetry);
    let missing_telemetry = run.evaluate(&criteria, 6, true);
    assert!(missing_telemetry.has_production_libp2p_runtime);
    assert!(!missing_telemetry.has_deployed_telemetry_service);
    assert!(!missing_telemetry.has_deployed_public_services);
    assert!(!missing_telemetry.public_criterion_met);

    run.services.push(public_service(
        PublicServiceKind::Telemetry,
        b"late-telemetry-service",
        1,
        9,
    ));
    let late_telemetry = run.evaluate(&criteria, 6, true);
    assert!(!late_telemetry.has_deployed_telemetry_service);
    assert!(!late_telemetry.public_criterion_met);

    run.services.pop();
    let mut unsigned_telemetry = public_service(
        PublicServiceKind::Telemetry,
        b"unsigned-telemetry-service",
        0,
        9,
    );
    unsigned_telemetry.signed_health_check_count = 0;
    assert!(!unsigned_telemetry.has_reachable_endpoint_proof());
    run.services.push(unsigned_telemetry);
    let unsigned_telemetry = run.evaluate(&criteria, 6, true);
    assert!(!unsigned_telemetry.has_deployed_telemetry_service);
    assert!(!unsigned_telemetry.public_criterion_met);
}
