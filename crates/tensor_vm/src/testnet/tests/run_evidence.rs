use super::*;

#[test]
fn public_testnet_run_evidence_requires_independent_external_operators() {
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 2,
        min_reward_settlement_records: 3,
    };
    let shared_operator = hash_bytes(b"test", &[b"shared-operator"]);
    let validator_operator = hash_bytes(b"test", &[b"validator-operator"]);
    let mut run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(address(b"miner-a"), shared_operator, 0, 9, 10),
            PublicNodeEvidence::miner(address(b"miner-b"), shared_operator, 0, 9, 10),
            PublicNodeEvidence::validator(address(b"validator-a"), validator_operator, 0, 9, 10),
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
        invalid_receipts_submitted: 2,
        invalid_receipts_rejected: 2,
        reward_settlement_records: 3,
        cuda_verified_miner_count: 2,
        cuda_graph_execution_receipts: 1,
        validator_vrf_lifecycle_records: 20,
        detection_measurement_records: 1,
    };

    let insufficient = run.evaluate(&criteria, 6, true);
    assert_eq!(insufficient.miner_count, 1);
    assert_eq!(insufficient.validator_count, 1);
    assert_eq!(insufficient.required_blocks, 0);
    assert_eq!(insufficient.finality_rate_bps, 10_000);
    assert_eq!(insufficient.data_availability_bps, 9_500);
    assert_eq!(insufficient.invalid_work_rejection_rate_bps, 10_000);
    assert!(insufficient.external_operator_evidence);
    assert!(insufficient.has_production_libp2p_runtime);
    assert!(insufficient.has_deployed_public_services);
    assert!(!insufficient.has_required_miners);
    assert!(insufficient.has_invalid_work_rejection_evidence);
    assert!(insufficient.has_reward_settlement_records);
    assert!(!insufficient.public_criterion_met);

    let mut role_order_conflict = run.clone();
    let shared_node_address = address(b"role-order-shared-address");
    role_order_conflict.nodes = vec![
        PublicNodeEvidence::miner(
            shared_node_address,
            hash_bytes(b"test", &[b"role-order-miner-shared-operator"]),
            0,
            9,
            10,
        ),
        PublicNodeEvidence::miner(
            address(b"role-order-independent-miner-address"),
            hash_bytes(b"test", &[b"role-order-independent-miner-operator"]),
            0,
            9,
            10,
        ),
        PublicNodeEvidence::validator(
            shared_node_address,
            hash_bytes(b"test", &[b"role-order-validator-operator"]),
            0,
            9,
            10,
        ),
    ];
    let role_order_criteria = PublicTestnetCriteria {
        min_miners: 1,
        min_validators: 1,
        ..criteria.clone()
    };
    let role_order_match = role_order_conflict.evaluate(&role_order_criteria, 6, true);
    assert_eq!(role_order_match.miner_count, 1);
    assert_eq!(role_order_match.validator_count, 1);
    assert!(role_order_match.has_required_miners);
    assert!(role_order_match.has_required_validators);
    assert!(role_order_match.public_criterion_met);

    let mut greedy_address_conflict = run.clone();
    greedy_address_conflict.nodes = vec![
        PublicNodeEvidence::miner([1; 32], [1; 32], 0, 9, 10),
        PublicNodeEvidence::miner([2; 32], [2; 32], 0, 9, 10),
        PublicNodeEvidence::miner([3; 32], [2; 32], 0, 9, 10),
        PublicNodeEvidence::validator([1; 32], [10; 32], 0, 9, 10),
        PublicNodeEvidence::validator([2; 32], [10; 32], 0, 9, 10),
    ];
    let greedy_address_match = greedy_address_conflict.evaluate(&criteria, 6, true);
    assert_eq!(greedy_address_match.miner_count, 2);
    assert_eq!(greedy_address_match.validator_count, 1);
    assert!(greedy_address_match.has_required_miners);
    assert!(greedy_address_match.has_required_validators);
    assert!(greedy_address_match.public_criterion_met);

    let zero_quota_criteria = PublicTestnetCriteria {
        min_miners: 0,
        min_validators: 0,
        ..criteria.clone()
    };
    let (zero_quota_miners, zero_quota_validators) = greedy_address_conflict
        .find_public_operator_quota_matching(&zero_quota_criteria)
        .expect("zero public-operator quotas should always match");
    assert!(zero_quota_miners.operator_ids.is_empty());
    assert!(zero_quota_validators.operator_ids.is_empty());

    run.nodes[1] = PublicNodeEvidence::miner(
        address(b"miner-a"),
        hash_bytes(b"test", &[b"miner-b-operator"]),
        0,
        9,
        10,
    );
    let shared_node_address = run.evaluate(&criteria, 6, true);
    assert_eq!(shared_node_address.miner_count, 1);
    assert!(!shared_node_address.has_required_miners);
    assert!(!shared_node_address.public_criterion_met);

    let mut unmatched_independence_graph = run.clone();
    let operator_a = hash_bytes(b"test", &[b"matching-miner-a-operator"]);
    let operator_b = hash_bytes(b"test", &[b"matching-miner-b-operator"]);
    let operator_c = hash_bytes(b"test", &[b"matching-miner-c-operator"]);
    let address_a = address(b"matching-miner-a-address");
    let address_b = address(b"matching-miner-b-address");
    let address_c = address(b"matching-miner-c-address");
    unmatched_independence_graph.nodes = vec![
        PublicNodeEvidence::miner(address_a, operator_a, 0, 9, 10),
        PublicNodeEvidence::miner(address_b, operator_a, 0, 9, 10),
        PublicNodeEvidence::miner(address_c, operator_a, 0, 9, 10),
        PublicNodeEvidence::miner(address_a, operator_b, 0, 9, 10),
        PublicNodeEvidence::miner(address_a, operator_c, 0, 9, 10),
        PublicNodeEvidence::validator(address(b"validator-a"), validator_operator, 0, 9, 10),
    ];
    let matching_criteria = PublicTestnetCriteria {
        min_miners: 3,
        min_validators: 1,
        ..criteria
    };
    let unmatched_independence_graph =
        unmatched_independence_graph.evaluate(&matching_criteria, 6, true);
    assert_eq!(unmatched_independence_graph.miner_count, 2);
    assert_eq!(unmatched_independence_graph.validator_count, 1);
    assert!(!unmatched_independence_graph.has_required_miners);
    assert!(!unmatched_independence_graph.public_criterion_met);

    run.nodes[1] = PublicNodeEvidence::miner(
        address(b"miner-b"),
        hash_bytes(b"test", &[b"miner-b-operator"]),
        0,
        9,
        10,
    );
    let no_external_flag = run.evaluate(&criteria, 6, false);
    assert!(!no_external_flag.external_operator_evidence);
    assert!(!no_external_flag.public_criterion_met);

    let sufficient = run.evaluate(&criteria, 6, true);
    assert_eq!(sufficient.miner_count, 2);
    assert!(sufficient.has_required_miners);
    assert!(sufficient.has_required_validators);
    assert!(sufficient.has_required_run_duration);
    assert!(sufficient.has_required_block_count);
    assert!(sufficient.has_required_finality);
    assert!(sufficient.has_required_data_availability);
    assert!(sufficient.has_invalid_work_rejection_evidence);
    assert!(sufficient.has_reward_settlement_records);
    assert!(sufficient.has_production_libp2p_runtime);
    assert!(sufficient.has_deployed_public_services);
    assert!(!sufficient.has_validator_vrf_lifecycle_evidence);
    assert!(sufficient.public_criterion_met);

    let mut fully_available_lifecycle = run.clone();
    fully_available_lifecycle.available_receipts = fully_available_lifecycle.checked_receipts;
    let fully_available_lifecycle = fully_available_lifecycle.evaluate(&criteria, 6, true);
    assert!(fully_available_lifecycle.has_required_data_availability);
    assert!(fully_available_lifecycle.has_validator_vrf_lifecycle_evidence);
    assert!(fully_available_lifecycle.public_criterion_met);

    let mut graph_without_cuda_miners = run.clone();
    graph_without_cuda_miners.cuda_verified_miner_count = 0;
    let graph_without_cuda_miners = graph_without_cuda_miners.evaluate(&criteria, 6, true);
    assert!(!graph_without_cuda_miners.has_cuda_verified_miners);
    assert!(!graph_without_cuda_miners.has_cuda_graph_execution_evidence);
    assert!(graph_without_cuda_miners.public_criterion_met);

    let mut over_finalized = run.clone();
    over_finalized.finalized_blocks = over_finalized.observed_blocks + 1;
    let over_finalized = over_finalized.evaluate(&criteria, 6, true);
    assert_eq!(over_finalized.finality_rate_bps, 10_000);
    assert!(!over_finalized.has_required_finality);
    assert!(!over_finalized.public_criterion_met);

    let mut over_available = run.clone();
    over_available.available_receipts = over_available.checked_receipts + 1;
    let over_available = over_available.evaluate(&criteria, 6, true);
    assert_eq!(over_available.data_availability_bps, 10_000);
    assert!(!over_available.has_required_data_availability);
    assert!(!over_available.public_criterion_met);

    let mut shared_cross_role_operator = run.clone();
    shared_cross_role_operator.nodes[2] =
        PublicNodeEvidence::validator(address(b"validator-a"), shared_operator, 0, 9, 10);
    let shared_cross_role_operator = shared_cross_role_operator.evaluate(&criteria, 6, true);
    assert_eq!(shared_cross_role_operator.miner_count, 2);
    assert_eq!(shared_cross_role_operator.validator_count, 0);
    assert!(!shared_cross_role_operator.has_required_validators);
    assert!(!shared_cross_role_operator.public_criterion_met);

    let mut sparse_heartbeat = run.clone();
    sparse_heartbeat.nodes[0] = PublicNodeEvidence::miner(
        address(b"miner-a"),
        hash_bytes(b"test", &[b"miner-a-operator"]),
        0,
        9,
        9,
    );
    let sparse_heartbeat = sparse_heartbeat.evaluate(&criteria, 6, true);
    assert_eq!(sparse_heartbeat.miner_count, 1);
    assert!(!sparse_heartbeat.has_required_miners);
    assert!(!sparse_heartbeat.public_criterion_met);

    let mut zero_address = run.clone();
    zero_address.nodes[0] = PublicNodeEvidence::miner(
        [0; 32],
        hash_bytes(b"test", &[b"miner-a-operator"]),
        0,
        9,
        10,
    );
    let zero_address = zero_address.evaluate(&criteria, 6, true);
    assert_eq!(zero_address.miner_count, 1);
    assert!(!zero_address.public_criterion_met);

    let one_day_criteria = PublicTestnetCriteria {
        duration_days: 1,
        ..criteria.clone()
    };
    let short_window = run.evaluate(&one_day_criteria, 8_640, true);
    assert!(short_window.has_required_block_count);
    assert!(!short_window.has_required_run_duration);
    assert!(!short_window.public_criterion_met);

    let mut full_window = run.clone();
    full_window.run_ended_at_unix_seconds = full_window.run_started_at_unix_seconds + 86_400;
    let full_window = full_window.evaluate(&one_day_criteria, 8_640, true);
    assert!(full_window.has_required_run_duration);
    assert!(full_window.public_criterion_met);

    let mut tampered_heartbeat = run.clone();
    tampered_heartbeat.nodes[0].heartbeat_signature = [7; 32];
    let tampered_heartbeat = tampered_heartbeat.evaluate(&criteria, 6, true);
    assert_eq!(tampered_heartbeat.miner_count, 1);
    assert!(!tampered_heartbeat.has_required_miners);
    assert!(!tampered_heartbeat.public_criterion_met);

    run.invalid_receipts_rejected = 1;
    let accepted_invalid_work = run.evaluate(&criteria, 6, true);
    assert_eq!(accepted_invalid_work.invalid_work_rejection_rate_bps, 5_000);
    assert!(!accepted_invalid_work.has_invalid_work_rejection_evidence);
    assert!(!accepted_invalid_work.public_criterion_met);
}

#[test]
fn public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes() {
    let criteria = PublicTestnetCriteria {
        min_miners: 1,
        min_validators: 1,
        duration_days: 1,
        min_finality_rate_bps: 1,
        min_data_availability_bps: 1,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(
                address(b"unsigned-miner"),
                hash_bytes(b"test", &[b"unsigned-miner-operator"]),
                0,
                9,
                0,
            ),
            PublicNodeEvidence::miner(
                address(b"late-miner"),
                hash_bytes(b"test", &[b"late-miner-operator"]),
                1,
                9,
                8,
            ),
            PublicNodeEvidence::validator(address(b"zero-operator-validator"), [0; 32], 0, 9, 10),
        ],
        network_runtime: PublicNetworkRuntimeEvidence::default(),
        services: Vec::new(),
        service_content: Vec::new(),
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 11,
        checked_receipts: 0,
        available_receipts: 0,
        invalid_receipts_submitted: 0,
        invalid_receipts_rejected: 0,
        reward_settlement_records: 0,
        cuda_verified_miner_count: 0,
        cuda_graph_execution_receipts: 0,
        validator_vrf_lifecycle_records: 0,
        detection_measurement_records: 0,
    };

    assert!(run.nodes[0].covers_run(0));
    assert!(!run.nodes[0].has_external_operator_proof());
    assert!(!run.nodes[1].covers_run(run.observed_blocks));
    assert!(!run.nodes[2].has_external_operator_proof());

    let report = run.evaluate(&criteria, 6, true);
    assert_eq!(report.miner_count, 0);
    assert_eq!(report.validator_count, 0);
    assert_eq!(report.observed_duration_seconds, 60);
    assert_eq!(report.required_duration_seconds, 86_400);
    assert_eq!(report.required_blocks, 14_400);
    assert_eq!(report.finality_rate_bps, 10_000);
    assert_eq!(report.data_availability_bps, 0);
    assert_eq!(report.invalid_work_rejection_rate_bps, 0);
    assert!(!report.external_operator_evidence);
    assert!(!report.has_required_finality);
    assert!(!report.has_required_run_duration);
    assert!(!report.has_production_libp2p_runtime);
    assert!(!report.has_deployed_rpc_service);
    assert!(!report.has_deployed_explorer_service);
    assert!(!report.has_deployed_faucet_service);
    assert!(!report.has_deployed_telemetry_service);
    assert!(!report.has_deployed_public_services);
    assert!(!report.has_invalid_work_rejection_evidence);
    assert!(!report.has_reward_settlement_records);
    assert!(!report.public_criterion_met);
}
