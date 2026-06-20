use super::*;

#[test]
fn role_runtime_read_only_rpc_does_not_persist_chain() {
    let data_dir = unique_temp_data_dir("role-runtime-read-only-rpc");
    let _ = std::fs::remove_dir_all(&data_dir);
    let config = test_service_runtime_config(&data_dir, "secret");
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"read-only-rpc-no-persist"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let snapshot_modified = file_modified_at(store.snapshot_store().path());
    let chain_state_modified = file_modified_at(store.chain_state_store().path());
    thread::sleep(Duration::from_millis(1_100));

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    let addr = runtime.server().local_addr().unwrap();
    let client = thread::spawn(move || {
        send_http_request(
            addr,
            "GET /chain/head HTTP/1.1\r\nhost: localhost\r\nx-tensorchain-auth: secret\r\n\r\n",
        )
    });

    runtime.serve_rpc_once().unwrap();
    let response = client.join().unwrap();

    assert_eq!(http_status_line(&response), "HTTP/1.1 200 OK");
    assert_eq!(
        file_modified_at(store.snapshot_store().path()),
        snapshot_modified
    );
    assert_eq!(
        file_modified_at(store.chain_state_store().path()),
        chain_state_modified
    );
    assert_eq!(store.load_chain().unwrap(), chain);
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(report_u64(&status, "role_served_requests"), 1);

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_mutating_rpc_persists_chain() {
    let data_dir = unique_temp_data_dir("role-runtime-mutating-rpc");
    let _ = std::fs::remove_dir_all(&data_dir);
    let config = test_service_runtime_config(&data_dir, "secret");
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"mutating-rpc-persist"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let user = address(b"runtime-faucet-persist-user");

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    let addr = runtime.server().local_addr().unwrap();
    let request = format!(
        "POST /faucet/claim/{} HTTP/1.1\r\nhost: localhost\r\nx-tensorchain-auth: secret\r\ncontent-length: 0\r\n\r\n",
        hex(&user)
    );
    let client = thread::spawn(move || send_http_request(addr, &request));

    runtime.serve_rpc_once().unwrap();
    let response = client.join().unwrap();

    assert_eq!(http_status_line(&response), "HTTP/1.1 200 OK");
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().rewards().balance(&user), 0);
    assert!(
        persisted
            .state()
            .pending_credit_rewards()
            .values()
            .any(|reward| reward.beneficiary == user && reward.amount == 100)
    );
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(report_u64(&status, "role_served_requests"), 1);

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn validator_remote_tensor_fetch_status_does_not_persist_chain() {
    let data_dir = unique_temp_data_dir("validator-fetch-no-persist");
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let validator = address(b"validator-fetch-no-persist-validator");
    let mut chain = Chain::with_params(
        ChainParams {
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        },
        hash_bytes(b"test", &[b"validator-fetch-no-persist"]),
    );
    let miner = address(b"validator-fetch-no-persist-miner");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .unwrap();
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let snapshot_modified = file_modified_at(store.snapshot_store().path());
    let chain_state_modified = file_modified_at(store.chain_state_store().path());
    thread::sleep(Duration::from_millis(1_100));
    let config = ServiceRuntimeConfig {
        runtime_command: "validator_run",
        role: RuntimeRole::Validator,
        role_wallet_address: Some(validator),
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Validator,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[data_dir_text.as_bytes()])),
            "secret",
            0,
        )
        .unwrap(),
    };
    let mut runtime = RoleRuntimeLoop::start(config).unwrap();

    runtime.tick_validator_role_work_once().unwrap();

    assert_eq!(
        file_modified_at(store.snapshot_store().path()),
        snapshot_modified
    );
    assert_eq!(
        file_modified_at(store.chain_state_store().path()),
        chain_state_modified
    );
    assert_eq!(store.load_chain().unwrap(), chain);
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_u64(&status, "role_validator_remote_tensor_fetch_failures"),
        3
    );
    assert_eq!(
        report_u64(&status, "role_validator_attestations_submitted"),
        0
    );

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
