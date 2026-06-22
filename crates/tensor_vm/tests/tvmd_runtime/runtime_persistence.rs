use super::*;

const VERIFIED_DRAND_PUBLIC_KEY_HEX: &str = "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11";
const VERIFIED_DRAND_SIGNATURE_HEX: &str = "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433";
const VERIFIED_DRAND_WRONG_SIGNATURE_HEX: &str = "86ecea71376e78abd19aaf0ad52f462a6483626563b1023bd04815a7b953da888c74f5bf6ee672a5688603ab310026230522898f33f23a7de363c66f90ffd49ec77ebf7f6c1478a9ecd6e714b4d532ab43d044da0a16fed13b4791d7fc999e2b";
const PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL: &str = "https://api.drand.sh/v2";
const PUBLIC_DRAND_DEFAULT_CHAIN_HASH: &str =
    "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce";
const PUBLIC_DRAND_DEFAULT_PUBLIC_KEY_HEX: &str = "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31";
const PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX: &str = "8d61d9100567de44682506aea1a7a6fa6e5491cd27a0a0ed349ef6910ac5ac20ff7bc3e09d7c046566c9f7f3c6f3b10104990e7cb424998203d8f7de586fb7fa5f60045417a432684f85093b06ca91c769f0e7ca19268375e659c2a2352b4655";
const PUBLIC_DRAND_DEFAULT_ROUND_1_PREVIOUS_SIGNATURE_HEX: &str =
    "176f93498eac9ca337150b46d21dd58673ea4e3581185f869672e59fa4cb390a";
const PUBLIC_DRAND_DEFAULT_ROUND_2_SIGNATURE_HEX: &str = "aa18facd2d51b616511d542de6f9af8a3b920121401dad1434ed1db4a565f10e04fad8d9b2b4e3e0094364374caafe9b10478bf75650124831509c638b5a36a7a232ec70289f8751a2adb47fc32eb70b57dc81c39d48cbcac9fec46cdfc31663";

fn hex_bytes(input: &str) -> Vec<u8> {
    assert_eq!(input.len() % 2, 0);
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("hex high nibble");
            let low = (pair[1] as char).to_digit(16).expect("hex low nibble");
            ((high << 4) | low) as u8
        })
        .collect()
}

fn verified_drand_vector() -> (u64, Vec<u8>, Vec<u8>, tensor_vm::types::Hash) {
    (
        223_344,
        hex_bytes(VERIFIED_DRAND_PUBLIC_KEY_HEX),
        hex_bytes(VERIFIED_DRAND_SIGNATURE_HEX),
        hex_bytes("f3d6adf1daa2c7877f90fb0f1a675ab0a42653a1e2a9b66fee0749d47a47bc57")
            .try_into()
            .unwrap(),
    )
}

fn public_drand_default_round_1_config() -> RandomnessBeaconRuntimeConfig {
    public_drand_default_round_config(
        1,
        PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX,
        PUBLIC_DRAND_DEFAULT_ROUND_1_PREVIOUS_SIGNATURE_HEX,
    )
}

fn public_drand_default_round_2_config() -> RandomnessBeaconRuntimeConfig {
    public_drand_default_round_config(
        2,
        PUBLIC_DRAND_DEFAULT_ROUND_2_SIGNATURE_HEX,
        PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX,
    )
}

fn public_drand_default_round_config(
    round: u64,
    signature_hex: &str,
    previous_signature_hex: &str,
) -> RandomnessBeaconRuntimeConfig {
    RandomnessBeaconRuntimeConfig::verified_chained_drand(
        round,
        hex_bytes(PUBLIC_DRAND_DEFAULT_PUBLIC_KEY_HEX),
        hex_bytes(signature_hex),
        hex_bytes(previous_signature_hex),
    )
    .unwrap()
}

fn with_public_drand_observation(
    mut config: RandomnessBeaconRuntimeConfig,
    expected_latest_round: u64,
    max_round_lag: u64,
) -> RandomnessBeaconRuntimeConfig {
    config.drand_genesis_time = 1_595_431_050;
    config.drand_period = 30;
    config.drand_expected_latest_round = expected_latest_round;
    config.drand_round_lag = expected_latest_round.saturating_sub(config.beacon_round);
    config.drand_max_round_lag = max_round_lag;
    config
}

struct ScriptedDrandClient {
    responses: std::sync::Mutex<
        std::collections::VecDeque<std::result::Result<RandomnessBeaconRuntimeConfig, String>>,
    >,
}

impl ScriptedDrandClient {
    fn new(
        responses: Vec<std::result::Result<RandomnessBeaconRuntimeConfig, String>>,
    ) -> ScriptedDrandClient {
        Self {
            responses: std::sync::Mutex::new(responses.into()),
        }
    }
}

impl DrandBeaconClient for ScriptedDrandClient {
    fn fetch_latest_chained(
        &self,
        _config: &RandomnessBeaconRuntimeConfig,
    ) -> std::result::Result<RandomnessBeaconRuntimeConfig, String> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("scripted drand response must exist")
    }
}

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
fn role_runtime_external_randomness_beacon_tick_persists_chain_and_status() {
    let data_dir = unique_temp_data_dir("role-runtime-randomness-beacon");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    config.randomness_beacon =
        RandomnessBeaconRuntimeConfig::local_deterministic("test-local-drand", 17);
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"runtime-randomness-beacon"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    runtime.tick_randomness_beacon_once().unwrap();
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), 17);
    assert!(
        persisted
            .state()
            .external_randomness_beacons()
            .contains_key(&17)
    );
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_field(&status, "role_randomness_beacon_mode"),
        "local_deterministic"
    );
    assert_eq!(
        report_field(&status, "role_randomness_latest_source_id"),
        "test-local-drand"
    );
    assert_eq!(report_u64(&status, "role_randomness_latest_round"), 17);
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 0);
    assert_eq!(report_u64(&status, "role_randomness_beacon_failures"), 0);

    runtime.tick_randomness_beacon_once().unwrap();
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 0);

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_verified_drand_beacon_tick_persists_chain_and_status() {
    let data_dir = unique_temp_data_dir("role-runtime-verified-drand");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    let (round, public_key, signature, expected_randomness) = verified_drand_vector();
    config.randomness_beacon =
        RandomnessBeaconRuntimeConfig::verified_drand(round, public_key, signature).unwrap();
    let expected_source_id = config.randomness_beacon.source_id.clone();
    let expected_proof_hash = config.randomness_beacon.proof_hash;
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"runtime-verified-drand"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    runtime.tick_randomness_beacon_once().unwrap();
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), round);
    assert_eq!(
        persisted.state().finalized_randomness(),
        expected_randomness
    );
    let record = persisted
        .state()
        .external_randomness_beacons()
        .get(&round)
        .expect("verified drand record must be persisted");
    assert_eq!(record.source_id, expected_source_id);
    assert_eq!(record.randomness, expected_randomness);
    assert_eq!(record.proof_hash, expected_proof_hash);
    assert!(matches!(
        record.proof,
        tensor_vm::chain::ExternalRandomnessBeaconProof::DrandPedersenBlsUnchainedV1 { .. }
    ));
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_field(&status, "role_randomness_beacon_mode"),
        "verified_drand"
    );
    assert_eq!(
        report_field(&status, "role_randomness_latest_source_id"),
        expected_source_id
    );
    assert_eq!(report_u64(&status, "role_randomness_latest_round"), round);
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 0);
    assert_eq!(report_u64(&status, "role_randomness_beacon_failures"), 0);

    runtime.tick_randomness_beacon_once().unwrap();
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 0);

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_public_drand_fetch_tick_persists_chain_and_status() {
    let data_dir = unique_temp_data_dir("role-runtime-public-drand");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    config.randomness_beacon = RandomnessBeaconRuntimeConfig::public_drand(
        PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL,
        PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
        1_000,
    )
    .unwrap();
    config.randomness_beacon.drand_poll_interval_ticks = 1;
    config.randomness_beacon.drand_failure_backoff_max_ticks = 4;
    let fetched_config = with_public_drand_observation(public_drand_default_round_1_config(), 2, 2);
    let expected_source_id = fetched_config.source_id.clone();
    let expected_randomness = fetched_config.randomness;
    let expected_proof_hash = fetched_config.proof_hash;
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"runtime-public-drand"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    runtime
        .tick_randomness_beacon_once_with_client(&ScriptedDrandClient::new(vec![Ok(
            fetched_config,
        )]))
        .unwrap();
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), 1);
    assert_eq!(
        persisted.state().finalized_randomness(),
        expected_randomness
    );
    let record = persisted
        .state()
        .external_randomness_beacons()
        .get(&1)
        .expect("public drand record must be persisted");
    assert_eq!(record.source_id, expected_source_id);
    assert_eq!(record.randomness, expected_randomness);
    assert_eq!(record.proof_hash, expected_proof_hash);
    assert!(matches!(
        record.proof,
        tensor_vm::chain::ExternalRandomnessBeaconProof::DrandPedersenBlsChainedV1 { .. }
    ));
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_field(&status, "role_randomness_beacon_mode"),
        "public_drand"
    );
    assert_eq!(
        report_field(&status, "role_randomness_latest_source_id"),
        expected_source_id
    );
    assert_eq!(report_u64(&status, "role_randomness_latest_round"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacon_failures"), 0);
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetch_attempts"),
        1
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetch_successes"),
        1
    );
    assert_eq!(
        report_u64(
            &status,
            "role_randomness_public_drand_expected_latest_round"
        ),
        2
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetched_round_lag"),
        1
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_max_round_lag"),
        2
    );
    assert_eq!(
        report_u64(
            &status,
            "role_randomness_public_drand_rounds_per_chain_epoch"
        ),
        20
    );
    assert_eq!(
        report_field(&status, "role_randomness_public_drand_fresh"),
        "true"
    );

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_public_drand_polling_skips_stale_rounds_and_backs_off_failures() {
    let data_dir = unique_temp_data_dir("role-runtime-public-drand-polling");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    config.randomness_beacon = RandomnessBeaconRuntimeConfig::public_drand(
        PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL,
        PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
        1_000,
    )
    .unwrap();
    config.randomness_beacon.drand_poll_interval_ticks = 1;
    config.randomness_beacon.drand_failure_backoff_max_ticks = 4;
    let round_1 = with_public_drand_observation(public_drand_default_round_1_config(), 2, 2);
    let round_2 = with_public_drand_observation(public_drand_default_round_2_config(), 2, 2);
    let expected_source_id = round_2.source_id.clone();
    let expected_randomness = round_2.randomness;
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"runtime-public-drand-polling"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    let client = ScriptedDrandClient::new(vec![
        Ok(round_1.clone()),
        Ok(round_1),
        Ok(round_2),
        Err("temporary drand outage".to_owned()),
    ]);
    runtime
        .tick_randomness_beacon_once_with_client(&client)
        .unwrap();
    runtime
        .tick_randomness_beacon_once_with_client(&client)
        .unwrap();
    runtime
        .tick_randomness_beacon_once_with_client(&client)
        .unwrap();
    runtime
        .tick_randomness_beacon_once_with_client(&client)
        .unwrap();

    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), 2);
    assert_eq!(
        persisted.state().finalized_randomness(),
        expected_randomness
    );
    assert!(
        persisted
            .state()
            .external_randomness_beacons()
            .get(&1)
            .is_some()
    );
    assert!(
        persisted
            .state()
            .external_randomness_beacons()
            .get(&2)
            .is_some()
    );
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_field(&status, "role_randomness_latest_source_id"),
        "public-drand:8990e7a9aaed2ffe"
    );
    assert_eq!(report_u64(&status, "role_randomness_latest_round"), 0);
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 3);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 2);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacon_failures"), 1);
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetch_attempts"),
        4
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetch_successes"),
        2
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetch_stale"),
        1
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_consecutive_failures"),
        1
    );
    assert_eq!(
        report_u64(
            &status,
            "role_randomness_public_drand_backoff_remaining_ticks"
        ),
        1
    );
    assert_eq!(
        report_u64(
            &status,
            "role_randomness_public_drand_expected_latest_round"
        ),
        2
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetched_round_lag"),
        0
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_max_round_lag"),
        2
    );
    assert_eq!(
        report_field(&status, "role_randomness_public_drand_fresh"),
        "true"
    );
    assert!(expected_source_id.starts_with("drand-pedersen-bls-chained-v1:"));

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_public_drand_skips_newer_round_outside_freshness_window() {
    let data_dir = unique_temp_data_dir("role-runtime-public-drand-stale-mapping");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    config.randomness_beacon = RandomnessBeaconRuntimeConfig::public_drand(
        PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL,
        PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
        1_000,
    )
    .unwrap();
    config.randomness_beacon.drand_poll_interval_ticks = 1;
    config.randomness_beacon.drand_failure_backoff_max_ticks = 4;
    config.randomness_beacon.drand_max_round_lag = 2;
    let stale_round = with_public_drand_observation(public_drand_default_round_2_config(), 9, 2);
    let chain = config.node.build_chain(hash_bytes(
        b"test",
        &[b"runtime-public-drand-stale-mapping"],
    ));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    runtime
        .tick_randomness_beacon_once_with_client(&ScriptedDrandClient::new(vec![Ok(stale_round)]))
        .unwrap();

    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), 0);
    assert!(persisted.state().external_randomness_beacons().is_empty());
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 0);
    assert_eq!(report_u64(&status, "role_randomness_beacons_skipped"), 1);
    assert_eq!(
        report_u64(
            &status,
            "role_randomness_public_drand_expected_latest_round"
        ),
        9
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_fetched_round_lag"),
        7
    );
    assert_eq!(
        report_u64(&status, "role_randomness_public_drand_max_round_lag"),
        2
    );
    assert_eq!(
        report_field(&status, "role_randomness_public_drand_fresh"),
        "false"
    );

    drop(runtime);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

#[test]
fn role_runtime_verified_drand_beacon_tick_records_invalid_signature_failure() {
    let data_dir = unique_temp_data_dir("role-runtime-verified-drand-invalid");
    let _ = std::fs::remove_dir_all(&data_dir);
    let mut config = test_service_runtime_config(&data_dir, "secret");
    let (round, public_key, signature, _) = verified_drand_vector();
    config.randomness_beacon =
        RandomnessBeaconRuntimeConfig::verified_drand(round, public_key, signature).unwrap();
    config.randomness_beacon.drand_signature = hex_bytes(VERIFIED_DRAND_WRONG_SIGNATURE_HEX);
    let chain = config
        .node
        .build_chain(hash_bytes(b"test", &[b"runtime-verified-drand-invalid"]));
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();

    let mut runtime = RoleRuntimeLoop::start(config).unwrap();
    runtime.tick_randomness_beacon_once().unwrap();
    let persisted = store.load_chain().unwrap();
    assert_eq!(persisted.state().finalized_beacon_round(), 0);
    assert!(
        persisted
            .state()
            .external_randomness_beacons()
            .get(&round)
            .is_none()
    );
    let status = std::fs::read_to_string(data_dir.join("role-runtime.status")).unwrap();
    assert_eq!(
        report_field(&status, "role_randomness_beacon_mode"),
        "verified_drand"
    );
    assert_eq!(report_u64(&status, "role_randomness_beacons_observed"), 1);
    assert_eq!(report_u64(&status, "role_randomness_beacons_applied"), 0);
    assert_eq!(report_u64(&status, "role_randomness_beacon_failures"), 1);
    assert!(
        report_field(&status, "role_randomness_last_error")
            .contains("drand signature verification failed")
    );

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
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
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
