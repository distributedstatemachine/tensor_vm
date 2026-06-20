use super::*;

#[test]
fn local_testnet_can_bootstrap_from_shared_profile() {
    let mut profile = ChainProfile::local_cpu();
    profile.chain_params.epoch_length = 17;
    profile.miner_count = 3;
    profile.validator_count = 2;
    profile.miner_stake = 150;
    profile.validator_stake = 12_000;

    let testnet = LocalTestnet::from_profile(&profile, hash_bytes(b"test", &[b"profile"]));

    assert_eq!(TestnetConfig::from_profile(&profile).miner_count, 3);
    assert_eq!(testnet.chain.params().epoch_length, 17);
    assert_eq!(testnet.miners.len(), 3);
    assert_eq!(testnet.validators.len(), 2);
    assert_eq!(
        testnet.chain.state().miners()[&testnet.miners[0]].stake,
        profile.miner_stake
    );
    assert_eq!(
        testnet.chain.state().validators()[&testnet.validators[0]].stake,
        profile.validator_stake
    );
    assert!(testnet.has_mandatory_libp2p_participant_paths());
}

#[test]
fn local_testnet_bootstraps_required_public_shape() {
    let mut testnet =
        LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
    assert_eq!(testnet.miners.len(), 10);
    assert_eq!(testnet.validators.len(), 5);
    assert_eq!(testnet.participant_endpoints.len(), 15);
    assert!(testnet.has_mandatory_libp2p_participant_paths());
    assert!(
        testnet
            .participant_endpoints
            .iter()
            .all(LocalParticipantEndpoint::has_mandatory_libp2p_node_path)
    );
    assert!(!local_libp2p_multiaddr_has_tcp_node_path("not-a-multiaddr"));
    assert!(!local_libp2p_multiaddr_has_tcp_node_path(
        "/ip4/127.0.0.1/tcp/0"
    ));
    let gate0_peer = libp2p::PeerId::random();
    assert!(local_libp2p_multiaddr_has_tcp_node_path(&format!(
        "/ip4/127.0.0.1/tcp/4001/p2p/{gate0_peer}"
    )));
    let mut missing_endpoint = testnet.clone();
    missing_endpoint.participant_endpoints.pop();
    assert!(!missing_endpoint.has_mandatory_libp2p_participant_paths());
    let mut duplicate_endpoint = testnet.clone();
    duplicate_endpoint.participant_endpoints[0].node_endpoint = duplicate_endpoint
        .participant_endpoints[1]
        .node_endpoint
        .clone();
    assert!(!duplicate_endpoint.has_mandatory_libp2p_participant_paths());
    let libp2p_service = crate::p2p::spawn_libp2p_service(crate::p2p::Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        ..crate::p2p::Libp2pControlPlaneConfig::default()
    })
    .expect("Gate 0 must construct the mandatory libp2p control-plane runtime");
    assert_eq!(libp2p_service.info().subscribed_topics.len(), 5);
    assert_eq!(libp2p_service.info().request_response_protocols.len(), 4);
    assert!(
        libp2p_service
            .info()
            .identify_protocol
            .starts_with(crate::p2p::LIBP2P_PROTOCOL_PREFIX)
    );
    testnet.run_blocks(12);
    let summary = testnet.explorer_summary();
    assert_eq!(summary.block_count, 12);
    assert_eq!(testnet.expected_blocks_for_days(7), 100_800);
    assert_eq!(testnet.telemetry().block_finality_rate, 1.0);
    let public_evidence = testnet.public_testnet_evidence(&PublicTestnetCriteria::default(), false);
    assert_eq!(public_evidence.required_blocks, 100_800);
    assert!(public_evidence.has_required_miners);
    assert!(public_evidence.has_required_validators);
    assert!(!public_evidence.has_required_block_count);
    assert!(!public_evidence.external_operator_evidence);
    assert!(!public_evidence.has_production_libp2p_runtime);
    assert!(!public_evidence.has_deployed_public_service_content);
    assert!(!public_evidence.has_deployed_public_services);
    assert!(!public_evidence.public_criterion_met);
}

#[test]
fn local_testnet_runs_full_matmul_receipt_attestation_settlement_round() {
    let mut testnet =
        LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_matmul_round(&scheduler);

    assert_eq!(
        testnet.chain.state().receipts().len(),
        testnet.chain.params().replication_factor
    );
    assert_eq!(
        testnet.chain.state().settled_receipts().len(),
        testnet.chain.params().replication_factor
    );
    assert_eq!(testnet.chain.blocks().len(), 1);
    assert!(testnet.telemetry().total_tensor_work > 0);
    let rewarded_miners = testnet
        .miners
        .iter()
        .filter(|miner| {
            testnet
                .chain
                .state()
                .pending_receipt_rewards()
                .values()
                .any(|reward| reward.beneficiary == **miner && reward.amount > 0)
        })
        .count();
    assert!(rewarded_miners >= testnet.chain.params().agreement_quorum);

    let evidence = testnet.public_testnet_evidence(
        &PublicTestnetCriteria {
            duration_days: 0,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            ..PublicTestnetCriteria::default()
        },
        true,
    );
    assert_eq!(evidence.observed_blocks, 1);
    assert_eq!(evidence.required_blocks, 0);
    assert_eq!(evidence.finality_rate_bps, 10_000);
    assert_eq!(evidence.data_availability_bps, 10_000);
    assert!(evidence.has_reward_settlement_records);
    assert!(!evidence.has_invalid_work_rejection_evidence);
    assert!(!evidence.public_criterion_met);

    let invalid_receipt_id = hash_bytes(b"test", &[b"public-invalid-receipt"]);
    let invalid_statement = crate::verify::AttestationStatement {
        receipt_id: invalid_receipt_id,
        job_id: hash_bytes(b"test", &[b"public-invalid-job"]),
        primitive_type: crate::jobs::PrimitiveType::TensorOp,
        result: crate::verify::VerificationResult::Invalid,
        checks_root: hash_bytes(b"test", &[b"public-invalid-checks"]),
        data_availability_passed: true,
    };
    let invalid_validator = testnet.validators[0];
    let invalid_stake = testnet
        .chain
        .state()
        .validators()
        .get(&invalid_validator)
        .unwrap()
        .stake;
    testnet
        .chain
        .insert_attestation_for_testing(crate::verify::ValidatorAttestation::new(
            invalid_validator,
            invalid_stake,
            invalid_statement,
        ));

    let complete_local_evidence = testnet.public_testnet_evidence(
        &PublicTestnetCriteria {
            duration_days: 0,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            ..PublicTestnetCriteria::default()
        },
        true,
    );
    assert_eq!(complete_local_evidence.invalid_receipts_submitted, 1);
    assert_eq!(complete_local_evidence.invalid_receipts_rejected, 1);
    assert_eq!(
        complete_local_evidence.invalid_work_rejection_rate_bps,
        10_000
    );
    assert!(complete_local_evidence.has_invalid_work_rejection_evidence);
    assert!(complete_local_evidence.has_reward_settlement_records);
    assert!(!complete_local_evidence.has_production_libp2p_runtime);
    assert!(!complete_local_evidence.has_deployed_rpc_service);
    assert!(!complete_local_evidence.has_deployed_explorer_service);
    assert!(!complete_local_evidence.has_deployed_faucet_service);
    assert!(!complete_local_evidence.has_deployed_telemetry_service);
    assert!(!complete_local_evidence.has_deployed_public_services);
    assert!(!complete_local_evidence.public_criterion_met);
}

#[test]
fn local_testnet_runs_linear_training_receipt_state_transition_round() {
    let mut testnet =
        LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    testnet.run_linear_training_round(&scheduler);

    assert_eq!(
        testnet.chain.state().receipts().len(),
        testnet.chain.params().replication_factor
    );
    assert_eq!(
        testnet.chain.state().settled_receipts().len(),
        testnet.chain.params().replication_factor
    );
    assert_eq!(testnet.chain.blocks().len(), 1);
    assert_eq!(testnet.chain.state().model_states().len(), 1);
    assert_eq!(
        testnet
            .chain
            .state()
            .model_states()
            .values()
            .next()
            .unwrap()
            .step,
        1
    );
    let rewarded_miners = testnet
        .miners
        .iter()
        .filter(|miner| {
            testnet
                .chain
                .state()
                .pending_receipt_rewards()
                .values()
                .any(|reward| reward.beneficiary == **miner && reward.amount > 0)
        })
        .count();
    assert!(rewarded_miners >= testnet.chain.params().agreement_quorum);
}
