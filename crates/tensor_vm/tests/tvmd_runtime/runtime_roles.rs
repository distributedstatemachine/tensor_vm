use super::*;
use std::time::Duration;
use tensor_vm::ChainEvent;
use tensor_vm::app::{
    LocalProductionContext, LocalProductionSchedule, RoleServiceConfig, RoleServiceRunner,
    RuntimeRole, chain_profile_from_label, produce_and_publish_synthetic_job,
    runtime_role_wallet_registered, runtime_role_wallet_registration, submit_miner_role_receipt,
    submit_validator_role_attestation, tick_validator_role_work_once,
};

#[test]
fn runtime_role_policy_allows_only_validator_local_production() {
    let profile = ChainProfile::local_cpu();
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Service.node_role(), "service")
            .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(
            profile.clone(),
            RuntimeRole::Proposer.node_role(),
            "proposer"
        )
        .can_produce_local_blocks()
    );
    assert!(
        !NodeConfig::new(profile.clone(), RuntimeRole::Miner.node_role(), "miner")
            .can_produce_local_blocks()
    );
    assert!(
        NodeConfig::new(profile, RuntimeRole::Validator.node_role(), "validator")
            .can_produce_local_blocks()
    );

    assert_eq!(RuntimeRole::Service.label(), "service");
    assert_eq!(RuntimeRole::Miner.label(), "miner");
    assert_eq!(RuntimeRole::Validator.label(), "validator");
    assert_eq!(RuntimeRole::Proposer.label(), "proposer");
}

#[test]
fn role_loop_configs_bind_expected_runtime_roles_and_wallets() {
    let cases = [
        (
            RoleServiceRunner::miner(),
            "miner_run",
            RuntimeRole::Miner,
            "miner",
        ),
        (
            RoleServiceRunner::validator(),
            "validator_run",
            RuntimeRole::Validator,
            "validator",
        ),
        (
            RoleServiceRunner::proposer(),
            "proposer_run",
            RuntimeRole::Proposer,
            "proposer",
        ),
    ];

    for (loop_config, runtime_command, role, wallet) in cases {
        let service_config = loop_config
            .service_runtime_config(RoleServiceConfig {
                wallet,
                device: Some("cpu"),
                node: "/ip4/127.0.0.1/tcp/4001",
                listen: "127.0.0.1:0",
                p2p_listen: "/ip4/127.0.0.1/tcp/0",
                data_dir: "role-loop-config-test",
                identity_seed: None,
                auth_token: "token",
                max_requests: 1,
            })
            .unwrap();

        assert_eq!(service_config.runtime_command, runtime_command);
        assert_eq!(service_config.role, role);
        assert_eq!(service_config.node.role, role.node_role());
        assert_eq!(
            service_config.node.can_produce_local_blocks(),
            matches!(role, RuntimeRole::Validator)
        );
        assert!(!service_config.node.local_synthetic_producer());
        assert!(!service_config.node.local_block_proposer());
        assert_eq!(
            service_config
                .node
                .local_validator_block_proposer_delay_blocks,
            0
        );
        assert_eq!(
            service_config.role_wallet_address,
            Some(address(wallet.as_bytes()))
        );
    }
}

#[test]
fn role_loop_reports_keep_role_specific_readiness_lines() {
    let config = RoleServiceConfig {
        wallet: "testnet-miner-0",
        device: Some("cpu"),
        node: "/ip4/127.0.0.1/tcp/4001",
        listen: "127.0.0.1:0",
        p2p_listen: "/ip4/127.0.0.1/tcp/0",
        data_dir: "role-loop-report-test",
        identity_seed: None,
        auth_token: "token",
        max_requests: 1,
    };

    let miner_report = RoleServiceRunner::miner().format_report(config, "service_report=true");
    assert_eq!(report_field(&miner_report, "command"), "miner_run");
    assert_eq!(report_field(&miner_report, "role"), "miner");
    assert_eq!(report_field(&miner_report, "device"), "cpu");
    assert_eq!(report_field(&miner_report, "role_runtime_ready"), "true");

    let validator_report =
        RoleServiceRunner::validator().format_report(config, "service_report=true");
    assert_eq!(report_field(&validator_report, "command"), "validator_run");
    assert_eq!(report_field(&validator_report, "role"), "validator");
    assert_eq!(
        report_field(&validator_report, "reference_verifier_ready"),
        "true"
    );
    assert_eq!(
        report_field(&validator_report, "role_runtime_ready"),
        "true"
    );

    let proposer_report =
        RoleServiceRunner::proposer().format_report(config, "service_report=true");
    assert_eq!(report_field(&proposer_report, "command"), "proposer_run");
    assert_eq!(report_field(&proposer_report, "role"), "proposer");
    assert_eq!(report_field(&proposer_report, "proposer_ready"), "true");
    assert_eq!(report_field(&proposer_report, "role_runtime_ready"), "true");
}

#[test]
fn role_wallet_registration_matches_loaded_chain_role() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"role-wallet-registration"]));
    let miner = address(b"runtime-wallet-miner");
    let validator = address(b"runtime-wallet-validator");
    let unknown = address(b"runtime-wallet-unknown");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);

    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(miner), &chain),
        "miner"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(validator), &chain),
        "validator"
    );
    assert!(runtime_role_wallet_registered(
        RuntimeRole::Proposer,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Miner, Some(validator), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Miner,
        Some(validator),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Validator, Some(miner), &chain),
        "unregistered"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Validator,
        Some(miner),
        &chain
    ));
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Proposer, Some(unknown), &chain),
        "unregistered"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, None, &chain),
        "none"
    );
    assert_eq!(
        runtime_role_wallet_registration(RuntimeRole::Service, Some(miner), &chain),
        "none"
    );
    assert!(!runtime_role_wallet_registered(
        RuntimeRole::Service,
        None,
        &chain
    ));
}

#[test]
fn chain_profile_labels_drive_runtime_synthetic_jobs() {
    let local = chain_profile_from_label("local_cpu").unwrap();
    let testnet = chain_profile_from_label("public_testnet").unwrap();
    let mainnet = chain_profile_from_label("mainnet").unwrap();

    assert_eq!(local.label(), "local_cpu");
    assert_eq!(testnet.label(), "public_testnet");
    assert_eq!(mainnet.label(), "mainnet");
    assert!(local.synthetic_job_source().is_some());
    assert!(testnet.synthetic_job_source().is_none());
    assert!(mainnet.synthetic_job_source().is_none());
    assert!(chain_profile_from_label("staging").is_err());
}

#[test]
fn scheduled_local_production_publishes_jobs_without_producer_receipts_or_attestations() {
    let data_dir = unique_temp_data_dir("scheduled-job-only-production");
    let _ = std::fs::remove_dir_all(&data_dir);
    let store = NodeStore::open(data_dir.clone());
    let validator = address(b"scheduled-job-only-validator");
    let mut chain = Chain::new(local_cpu_seed_beacon());
    register_miner(&mut chain, address(b"scheduled-job-only-miner"));
    register_validator(&mut chain, validator);
    store.persist_chain(&chain).unwrap();
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    let mut server = RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap();
    let p2p_service = spawn_libp2p_service(Libp2pControlPlaneConfig {
        identity_seed: Some([31; 32]),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    let mut runtime_state = NodeRuntimeState::default();
    let mut schedule = LocalProductionSchedule::new(Some(Duration::from_millis(0)));

    let changed = schedule
        .produce_if_due(LocalProductionContext {
            profile: &ChainProfile::local_cpu(),
            local_producer: true,
            validator: Some(validator),
            store: &store,
            server: &mut server,
            p2p_service: &p2p_service,
            runtime_state: &mut runtime_state,
        })
        .unwrap();

    assert!(changed);
    assert_eq!(server.gateway().node.chain.state().jobs().len(), 1);
    assert_eq!(server.gateway().node.chain.state().receipts().len(), 0);
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .attestations()
            .is_empty()
    );
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .settled_receipts()
            .is_empty()
    );
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .is_empty()
    );
    assert_eq!(server.gateway().node.chain.blocks().len(), 0);
    assert_eq!(runtime_state.produced_blocks(), 0);
    assert_eq!(runtime_state.validator_blocks_proposed(), 0);
    assert_eq!(runtime_state.validator_useful_blocks_proposed(), 0);
    assert_eq!(runtime_state.validator_fallback_blocks_proposed(), 0);
    assert_eq!(runtime_state.validator_receipts_proposed(), 0);
    assert_eq!(runtime_state.validator_proposer_settled_receipts_seen(), 0);
    assert!(!runtime_state.validator_proposer_work_ready());
    assert_eq!(runtime_state.miner_receipts_submitted(), 0);
    assert_eq!(runtime_state.validator_attestations_submitted(), 0);
    assert_eq!(p2p_service.observed_receipt_gossip_count(), 0);
    assert_eq!(p2p_service.observed_attestation_gossip_count(), 0);

    drop(p2p_service);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let miner = address(b"role-owned-pipeline-miner");
    let validator = address(b"role-owned-pipeline-validator");
    let mut chain = Chain::with_params(params, local_cpu_seed_beacon());
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    let mut server = RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap();
    let p2p_service = spawn_libp2p_service(Libp2pControlPlaneConfig {
        identity_seed: Some([32; 32]),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();

    let job_id =
        produce_and_publish_synthetic_job(&mut server, &p2p_service, &ChainProfile::local_cpu())
            .unwrap()
            .expect("local profile should publish a synthetic job");
    assert_eq!(server.gateway().node.chain.state().jobs().len(), 1);
    assert_eq!(server.gateway().node.chain.state().receipts().len(), 0);
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .attestations()
            .is_empty()
    );

    let miner_submission = submit_miner_role_receipt(&mut server.gateway_mut().node, miner, job_id)
        .unwrap()
        .expect("assigned miner should submit receipt for producer-published job");
    assert_eq!(miner_submission.receipts_submitted, 1);
    assert_eq!(server.gateway().node.chain.state().receipts().len(), 1);
    let receipt = server
        .gateway()
        .node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("role-owned receipt should be stored")
        .clone();
    assert_eq!(receipt.job_id(), job_id);
    assert_eq!(receipt.miner(), miner);

    let validator_submission = submit_validator_role_attestation(
        &mut server.gateway_mut().node,
        validator,
        receipt.receipt_id(),
    )
    .unwrap()
    .expect("assigned validator should attest role-owned receipt");
    assert_eq!(validator_submission.attestations_submitted, 1);
    let attestations = server
        .gateway()
        .node
        .chain
        .state()
        .attestations()
        .get(&receipt.receipt_id())
        .expect("role-owned attestation should be stored");
    assert_eq!(attestations.len(), 1);
    assert_eq!(attestations[0].validator, validator);

    assert!(
        !server
            .gateway()
            .node
            .chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id())
    );
    assert_eq!(server.gateway().node.chain.blocks().len(), 0);
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .is_empty()
    );
    let data_dir = unique_temp_data_dir("role-owned-proposal-tick");
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&server.gateway().node.chain).unwrap();
    let config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: NodeConfig::new(
            ChainProfile::local_cpu(),
            RuntimeRole::Validator.node_role(),
            data_dir.clone(),
        )
        .with_block_interval(Some(Duration::from_millis(1)))
        .with_local_synthetic_job_producer(true)
        .with_local_validator_block_proposer(true),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };
    let mut runtime_state = NodeRuntimeState::default();
    let changed = tick_validator_role_work_once(
        &config,
        &store,
        &mut server,
        &p2p_service,
        &mut runtime_state,
    )
    .unwrap();
    assert!(changed);
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id())
    );
    assert_eq!(runtime_state.produced_blocks(), 1);
    assert_eq!(runtime_state.validator_blocks_proposed(), 1);
    assert_eq!(runtime_state.validator_useful_blocks_proposed(), 1);
    assert_eq!(runtime_state.validator_fallback_blocks_proposed(), 0);
    assert_eq!(runtime_state.validator_receipts_proposed(), 1);
    assert_eq!(runtime_state.validator_proposer_settled_receipts_seen(), 1);
    assert_eq!(
        runtime_state.validator_proposer_artifact_ready_receipts_seen(),
        1
    );
    assert_eq!(runtime_state.validator_proposer_attested_receipts_seen(), 1);
    assert!(runtime_state.validator_proposer_work_ready());
    assert_eq!(server.gateway().node.chain.blocks().len(), 1);
    let block = server.gateway().node.chain.blocks().last().unwrap();
    let block_height = block.height;
    assert_eq!(block.proposer, validator);
    assert_eq!(block.proposer_reward, 500);
    let pending_proposer_reward = server
        .gateway()
        .node
        .chain
        .state()
        .pending_proposer_rewards()
        .get(&block_height)
        .expect("useful role proposal should delay proposer reward");
    assert_eq!(pending_proposer_reward.proposer, validator);
    assert_eq!(pending_proposer_reward.amount, 500);
    let claimable_at_height = pending_proposer_reward.claimable_at_height;
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .rewards()
            .balance(&validator),
        0
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .selected_receipts_for_block(block),
        vec![receipt.receipt_id()]
    );

    while server.gateway().node.chain.state().height() <= claimable_at_height {
        let timestamp = server
            .gateway()
            .node
            .chain
            .state()
            .height()
            .saturating_add(1)
            .saturating_mul(1_000);
        server
            .gateway_mut()
            .node
            .chain
            .apply_command(ChainCommand::ProduceBlock {
                proposer: validator,
                timestamp,
            })
            .unwrap();
    }
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .get(&block_height)
            .is_some()
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .rewards()
            .balance(&validator),
        0
    );
    let claim_events = server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::ClaimReward(validator))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: validator,
        amount: 1_000,
    }));
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .get(&block_height)
            .is_none()
    );
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .state()
            .accounts()
            .get(&validator)
            .unwrap()
            .balance,
        1_000
    );

    drop(p2p_service);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_proposer_tick_runs_without_synthetic_producer_gate() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let miner = address(b"ungated-proposer-miner");
    let validator = address(b"ungated-proposer-validator");
    let mut chain = Chain::with_params(params, local_cpu_seed_beacon());
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    let mut server = RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap();
    let p2p_service = spawn_libp2p_service(Libp2pControlPlaneConfig {
        identity_seed: Some([34; 32]),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();

    let job_id =
        produce_and_publish_synthetic_job(&mut server, &p2p_service, &ChainProfile::local_cpu())
            .unwrap()
            .expect("local profile should publish a synthetic job");
    let receipt_submission =
        submit_miner_role_receipt(&mut server.gateway_mut().node, miner, job_id)
            .unwrap()
            .expect("assigned miner should submit receipt");
    assert_eq!(receipt_submission.receipts_submitted, 1);
    let receipt_id = server
        .gateway()
        .node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("receipt should be stored")
        .receipt_id();
    let attestation_submission =
        submit_validator_role_attestation(&mut server.gateway_mut().node, validator, receipt_id)
            .unwrap()
            .expect("assigned validator should attest receipt");
    assert_eq!(attestation_submission.attestations_submitted, 1);
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .settled_receipts()
            .is_empty()
    );

    let data_dir = unique_temp_data_dir("ungated-validator-proposal-tick");
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&server.gateway().node.chain).unwrap();
    let config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: NodeConfig::new(
            ChainProfile::local_cpu(),
            RuntimeRole::Validator.node_role(),
            data_dir.clone(),
        )
        .with_local_validator_block_proposer(true),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };
    assert!(config.node.local_block_proposer());
    assert!(!config.node.local_synthetic_producer());
    let mut runtime_state = NodeRuntimeState::default();

    let changed = tick_validator_role_work_once(
        &config,
        &store,
        &mut server,
        &p2p_service,
        &mut runtime_state,
    )
    .unwrap();

    assert!(changed);
    assert_eq!(runtime_state.produced_blocks(), 1);
    assert_eq!(runtime_state.validator_blocks_proposed(), 1);
    assert_eq!(runtime_state.validator_useful_blocks_proposed(), 1);
    assert_eq!(runtime_state.validator_fallback_blocks_proposed(), 0);
    assert_eq!(runtime_state.validator_receipts_proposed(), 1);
    assert_eq!(runtime_state.validator_proposer_settled_receipts_seen(), 1);
    assert_eq!(
        runtime_state.validator_proposer_artifact_ready_receipts_seen(),
        1
    );
    assert_eq!(runtime_state.validator_proposer_attested_receipts_seen(), 1);
    assert!(runtime_state.validator_proposer_work_ready());
    let block = server
        .gateway()
        .node
        .chain
        .blocks()
        .last()
        .expect("validator tick should propose a block");
    assert_eq!(block.proposer, validator);
    assert_eq!(
        server
            .gateway()
            .node
            .chain
            .selected_receipts_for_block(block),
        vec![receipt_id]
    );
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );

    drop(p2p_service);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_proposer_delays_reward_without_waiting_for_validation_backlog() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let miner_a = address(b"delayed-proposer-miner-a");
    let miner_b = address(b"delayed-proposer-miner-b");
    let validator = address(b"delayed-proposer-validator");
    let mut chain = Chain::with_params(params, local_cpu_seed_beacon());
    register_miner(&mut chain, miner_a);
    register_miner(&mut chain, miner_b);
    register_validator(&mut chain, validator);
    let node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let gateway = RpcGateway::new(node, RpcPolicy::default());
    let mut server = RpcHttpServer::bind("127.0.0.1:0", gateway).unwrap();
    let p2p_service = spawn_libp2p_service(Libp2pControlPlaneConfig {
        identity_seed: Some([35; 32]),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();

    let beacon = server.gateway().node.chain.state().finalized_randomness();
    let job = tensor_vm::MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (first_receipt, a, b, c) =
        tensor_vm::TensorOpReceipt::from_job(&job, miner_a, 1, 5).unwrap();
    let (second_receipt, _a2, _b2, _c2) =
        tensor_vm::TensorOpReceipt::from_job(&job, miner_b, 2, 5).unwrap();
    let report = tensor_vm::verify_tensor_op(
        &job,
        &first_receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"delayed-proposer-backlog"]),
        &server.gateway().node.chain.params().freivalds,
    )
    .unwrap();
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(
            job.clone(),
        )))
        .unwrap();
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::TensorOp(
            first_receipt.clone(),
        )))
        .unwrap();
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::TensorOp(
            second_receipt.clone(),
        )))
        .unwrap();
    let validator_stake = server.gateway().node.chain.params().validator_min_stake;
    server
        .gateway_mut()
        .node
        .chain
        .apply_command(ChainCommand::SubmitAttestation(
            tensor_vm::ValidatorAttestation::new(
                validator,
                validator_stake,
                tensor_vm::AttestationStatement {
                    receipt_id: first_receipt.receipt_id,
                    job_id: first_receipt.job_id,
                    primitive_type: tensor_vm::PrimitiveType::TensorOp,
                    result: report.result,
                    checks_root: report.checks_root,
                    data_availability_passed: report.data_availability_passed,
                },
            ),
        ))
        .unwrap();

    let data_dir = unique_temp_data_dir("delayed-proposer-backlog");
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&server.gateway().node.chain).unwrap();
    let delayed_config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: NodeConfig::new(
            ChainProfile::local_cpu(),
            RuntimeRole::Validator.node_role(),
            data_dir.clone(),
        )
        .with_local_validator_block_proposer(true)
        .with_local_validator_block_proposer_delay_blocks(1),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };
    let mut runtime_state = NodeRuntimeState::default();

    let changed = tick_validator_role_work_once(
        &delayed_config,
        &store,
        &mut server,
        &p2p_service,
        &mut runtime_state,
    )
    .unwrap();

    assert!(changed);
    assert!(
        !delayed_config
            .node
            .local_block_proposer_delay_satisfied(server.gateway().node.chain.state().height())
    );
    assert_eq!(runtime_state.validator_blocks_proposed(), 0);
    assert_eq!(runtime_state.produced_blocks(), 0);
    assert_eq!(server.gateway().node.chain.blocks().len(), 0);

    let config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: NodeConfig::new(
            ChainProfile::local_cpu(),
            RuntimeRole::Validator.node_role(),
            data_dir.clone(),
        )
        .with_local_validator_block_proposer(true),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };

    let changed = tick_validator_role_work_once(
        &config,
        &store,
        &mut server,
        &p2p_service,
        &mut runtime_state,
    )
    .unwrap();

    assert!(changed);
    assert_eq!(runtime_state.validator_blocks_proposed(), 1);
    assert_eq!(runtime_state.produced_blocks(), 1);
    assert_eq!(server.gateway().node.chain.blocks().len(), 1);
    let block = server.gateway().node.chain.blocks().last().unwrap();
    assert!(
        server
            .gateway()
            .node
            .chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );
    assert_eq!(runtime_state.validator_proposer_settled_receipts_seen(), 1);
    assert_eq!(runtime_state.validator_assigned_receipts_seen(), 2);
    assert_eq!(runtime_state.validator_unattested_receipts(), 1);

    drop(p2p_service);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
