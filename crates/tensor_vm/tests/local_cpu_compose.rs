use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[path = "support/report_fields.rs"]
mod report_fields;

fn repo_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

fn trimmed_yaml_scalar(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

fn has_trimmed_line(text: &str, expected: &str) -> bool {
    text.lines().any(|line| line.trim() == expected)
}

fn assert_has_trimmed_lines(text: &str, label: &str, expected_lines: &[&str]) {
    for expected in expected_lines {
        assert!(
            has_trimmed_line(text, expected),
            "{label} should contain exact trimmed line {expected}"
        );
    }
}

fn assert_lacks_trimmed_lines(text: &str, label: &str, forbidden_lines: &[&str]) {
    for forbidden in forbidden_lines {
        assert!(
            !has_trimmed_line(text, forbidden),
            "{label} should not contain exact trimmed line {forbidden}"
        );
    }
}

fn trimmed_lines(text: &str) -> BTreeSet<&str> {
    text.lines().map(str::trim).collect()
}

fn prefixed_trimmed_values<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .collect()
}

fn shell_logical_lines(script: &str) -> Vec<String> {
    let mut logical_lines = Vec::new();
    let mut current = String::new();

    for line in script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let (segment, continues) = match line.strip_suffix('\\') {
            Some(segment) => (segment.trim_end(), true),
            None => (line, false),
        };
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(segment);
        if !continues {
            logical_lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        logical_lines.push(current);
    }

    logical_lines
}

fn assert_shell_logical_lines(script: &str, expected_lines: &[&str]) {
    let actual_lines = shell_logical_lines(script);
    for expected in expected_lines {
        assert!(
            actual_lines.iter().any(|line| line == expected),
            "script should contain logical line {expected}"
        );
    }
}

fn assert_lacks_shell_logical_lines(script: &str, forbidden_lines: &[&str]) {
    let actual_lines = shell_logical_lines(script);
    for forbidden in forbidden_lines {
        assert!(
            actual_lines.iter().all(|line| line != forbidden),
            "script should not contain logical line {forbidden}"
        );
    }
}

fn assert_status_value_reads(script: &str, document: &str, expected_reads: &[(&str, &str)]) {
    let actual_lines = shell_logical_lines(script);
    for (variable, key) in expected_reads {
        let expected = format!("{variable}=$(status_value {key} \"${document}\")");
        assert!(
            actual_lines.iter().any(|line| line == &expected),
            "script should read status field {key} into {variable}"
        );
    }
}

fn compose_service_section<'a>(compose: &'a str, service: &str) -> &'a str {
    let marker = format!("  {service}:\n");
    let start = compose
        .find(&marker)
        .unwrap_or_else(|| panic!("compose service {service} must exist"));
    let body_start = start + marker.len();
    let rest = &compose[body_start..];
    let next_service = rest.match_indices("\n  ").find_map(|(idx, _)| {
        rest[idx + 3..]
            .chars()
            .next()
            .is_some_and(|character| !character.is_whitespace())
            .then_some(idx)
    });
    let networks = rest.find("\nnetworks:");
    let end = match (next_service, networks) {
        (Some(next_service), Some(networks)) => next_service.min(networks),
        (Some(next_service), None) => next_service,
        (None, Some(networks)) => networks,
        (None, None) => rest.len(),
    };
    &rest[..end]
}

fn compose_env_value<'a>(service_section: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}: ");
    service_section
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(trimmed_yaml_scalar))
        .unwrap_or_else(|| panic!("compose service missing environment field {key}"))
}

fn env_file_value<'a>(env_file: &'a str, key: &str) -> &'a str {
    report_fields::report_value(env_file, key)
}

fn env_file_u64(env_file: &str, key: &str) -> u64 {
    report_fields::report_u64(env_file, key)
}

#[test]
fn local_cpu_compose_bundle_matches_spec_artifact_shape() {
    for path in [
        "deploy/tensorvm/local-cpu/docker-compose.yml",
        "deploy/tensorvm/local-cpu/Dockerfile",
        "deploy/tensorvm/local-cpu/README.md",
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
        "deploy/tensorvm/local-cpu/scripts/entrypoint.sh",
        "deploy/tensorvm/local-cpu/scripts/local-cpu-topology.sh",
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
        ".dockerignore",
    ] {
        assert!(Path::new(&repo_path(path)).exists(), "missing {path}");
    }

    let compose = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/docker-compose.yml"))
        .expect("compose file should be readable");
    let dockerfile = fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/Dockerfile"))
        .expect("Dockerfile should be readable");
    let entrypoint =
        fs::read_to_string(repo_path("deploy/tensorvm/local-cpu/scripts/entrypoint.sh"))
            .expect("entrypoint should be readable");
    let topology_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/local-cpu-topology.sh",
    ))
    .expect("local CPU topology script should be readable");
    let env_file = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/env/local-cpu.env.example",
    ))
    .expect("local CPU env file should be readable");
    let check_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
    ))
    .expect("check script should be readable");
    let restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-restart-continuity.sh",
    ))
    .expect("restart continuity script should be readable");
    let rolling_restart_script = fs::read_to_string(repo_path(
        "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
    ))
    .expect("rolling restart continuity script should be readable");
    let spec = fs::read_to_string(repo_path("docs/tensorvm/local_cpu_testnet_spec.md"))
        .expect("local CPU spec should be readable");
    let dockerignore =
        fs::read_to_string(repo_path(".dockerignore")).expect(".dockerignore should be readable");

    let miners = [
        "miner-00", "miner-01", "miner-02", "miner-03", "miner-04", "miner-05", "miner-06",
        "miner-07", "miner-08", "miner-09",
    ];
    let validators = [
        "validator-00",
        "validator-01",
        "validator-02",
        "validator-03",
        "validator-04",
    ];
    for service in miners.into_iter().chain(validators) {
        compose_service_section(&compose, service);
        assert!(
            has_trimmed_line(&spec, service),
            "spec topology should name exact service {service}"
        );
    }

    assert!(has_trimmed_line(&compose, "name: tensorvm-local-cpu"));
    assert!(has_trimmed_line(&compose, "tensorvm-local:"));
    assert!(has_trimmed_line(&compose, "driver: bridge"));
    assert!(has_trimmed_line(
        &compose,
        r#"'test -f /var/lib/tensorvm/local-cpu-ready && while IFS= read -r line; do [ "$${line}" = "libp2p_ready=true" ] && exit 0; done < /var/lib/tensorvm/local-cpu-ready; exit 1',"#
    ));
    assert!(!has_trimmed_line(
        &compose,
        "TENSORVM_ROLE_RUNTIME_COMMAND: proposer_run"
    ));

    let miner_sections = miners
        .iter()
        .map(|service| (*service, compose_service_section(&compose, service)))
        .collect::<Vec<_>>();
    let validator_sections = validators
        .iter()
        .map(|service| (*service, compose_service_section(&compose, service)))
        .collect::<Vec<_>>();
    assert_eq!(
        miner_sections
            .iter()
            .filter(|(_, section)| compose_env_value(section, "TENSORVM_ROLE") == "miner")
            .count(),
        10
    );
    assert_eq!(
        validator_sections
            .iter()
            .filter(|(_, section)| compose_env_value(section, "TENSORVM_ROLE") == "validator")
            .count(),
        5
    );
    for (idx, (service, section)) in miner_sections.iter().enumerate() {
        assert_eq!(
            compose_env_value(section, "TENSORVM_OPERATOR_NAME"),
            *service
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_WALLET"),
            format!("testnet-miner-{idx}")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_NODE_MULTIADDR"),
            format!("/dns4/{service}/tcp/4001")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_P2P_LISTEN"),
            "/ip4/0.0.0.0/tcp/4001"
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_RPC_LISTEN"),
            "0.0.0.0:8545"
        );
        assert!(
            has_trimmed_line(section, &format!("- {service}-data:/var/lib/tensorvm")),
            "compose service {service} must mount its data volume"
        );
    }
    for (idx, (service, section)) in validator_sections.iter().enumerate() {
        assert_eq!(
            compose_env_value(section, "TENSORVM_OPERATOR_NAME"),
            *service
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_WALLET"),
            format!("testnet-validator-{idx}")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_NODE_MULTIADDR"),
            format!("/dns4/{service}/tcp/4001")
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_P2P_LISTEN"),
            "/ip4/0.0.0.0/tcp/4001"
        );
        assert_eq!(
            compose_env_value(section, "TENSORVM_RPC_LISTEN"),
            "0.0.0.0:8545"
        );
        assert!(
            has_trimmed_line(section, &format!("- {service}-data:/var/lib/tensorvm")),
            "compose service {service} must mount its data volume"
        );
    }
    let bootstrap_miner = compose_service_section(&compose, "miner-00");
    assert_eq!(
        compose_env_value(bootstrap_miner, "TENSORVM_IS_BOOTSTRAP"),
        "true"
    );
    assert_eq!(
        compose_env_value(bootstrap_miner, "TENSORVM_SEED_LOCAL_TESTNET"),
        "true"
    );
    assert!(has_trimmed_line(
        bootstrap_miner,
        r#"- "127.0.0.1:8545:8545""#
    ));
    assert!(has_trimmed_line(
        bootstrap_miner,
        r#"- "127.0.0.1:4001:4001""#
    ));
    let producer_validator = compose_service_section(&compose, "validator-00");
    assert_eq!(
        compose_env_value(producer_validator, "TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS"),
        "1000"
    );
    assert_eq!(
        compose_env_value(producer_validator, "TENSORVM_LOCAL_CPU_ROLE_PRODUCER"),
        "true"
    );
    for service in validators.iter().skip(1) {
        let section = compose_service_section(&compose, service);
        assert!(
            section
                .lines()
                .all(|line| !line.trim().starts_with("TENSORVM_LOCAL_CPU_ROLE_PRODUCER:")),
            "only validator-00 should enable local production"
        );
    }

    let explorer = compose_service_section(&compose, "explorer");
    assert!(has_trimmed_line(
        explorer,
        r#"entrypoint: ["/usr/local/bin/tensorvm-explorer", "serve"]"#
    ));
    assert_eq!(
        compose_env_value(explorer, "TENSORVM_EXPLORER_LISTEN"),
        "0.0.0.0:8080"
    );
    assert_eq!(
        compose_env_value(explorer, "TENSORVM_EXPLORER_WS_URL"),
        "ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token"
    );
    assert!(has_trimmed_line(
        explorer,
        r#"- "127.0.0.1:${TENSORVM_LOCAL_CPU_EXPLORER_PORT:-8080}:8080""#
    ));
    assert!(has_trimmed_line(explorer, r#""health-check","#));
    assert!(has_trimmed_line(explorer, "condition: service_healthy"));

    assert_eq!(
        env_file_value(&env_file, "TENSORVM_SEED_LOCAL_TESTNET"),
        "true"
    );
    assert_eq!(
        env_file_u64(&env_file, "TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS"),
        1000
    );
    assert_eq!(
        env_file_value(&env_file, "TENSORVM_LOCAL_CPU_ROLE_PRODUCER"),
        "false"
    );
    assert_eq!(
        env_file_value(&env_file, "TENSORVM_BOOTSTRAP_PEER_ID"),
        "12D3KooWS2oXcVvmNNWTiUzwDWJavRHQmewe1NDfJB7SxP43jA7s"
    );
    assert_has_trimmed_lines(
        &compose,
        "local CPU compose build config",
        &["dockerfile: deploy/tensorvm/local-cpu/Dockerfile"],
    );

    let operator_ids = miner_sections
        .iter()
        .chain(validator_sections.iter())
        .map(|(_, section)| compose_env_value(section, "TENSORVM_OPERATOR_ID"))
        .collect::<BTreeSet<_>>();
    assert_eq!(operator_ids.len(), 15);

    assert_eq!(
        prefixed_trimmed_values(&dockerfile, "RUN "),
        [
            "cargo build -p tensor_vm --release",
            "cargo build -p tensor_vm_explorer --release",
            r#"useradd --system --home-dir /var/lib/tensorvm --shell /usr/sbin/nologin tensorvm \"#,
            "chmod 0755 /usr/local/bin/tvmd /usr/local/bin/tensorvm-explorer /usr/local/bin/tensorvm-local-entrypoint",
        ]
    );
    assert_eq!(
        prefixed_trimmed_values(&dockerfile, "COPY --from=builder "),
        [
            "/workspace/target/release/tvmd /usr/local/bin/tvmd",
            "/workspace/target/release/tensorvm-explorer /usr/local/bin/tensorvm-explorer",
        ]
    );
    assert!(has_trimmed_line(
        &dockerfile,
        "COPY deploy/tensorvm/local-cpu/scripts/entrypoint.sh /usr/local/bin/tensorvm-local-entrypoint"
    ));
    assert_has_trimmed_lines(&dockerignore, ".dockerignore", &["target", ".git"]);
    assert!(
        prefixed_trimmed_values(&dockerfile, "RUN ")
            .iter()
            .all(|command| !command
                .split_whitespace()
                .any(|token| token == "--features"))
    );
    let compose_lines = trimmed_lines(&compose);
    assert!(!compose_lines.iter().any(|line| line.starts_with("NVIDIA_")));
    assert_lacks_trimmed_lines(
        &compose,
        "local CPU compose",
        &[
            "runtime: nvidia",
            "devices:",
            r#""test -f /var/lib/tensorvm/local-cpu-ready && grep -q 'libp2p_ready=true' /var/lib/tensorvm/local-cpu-ready","#,
        ],
    );

    assert_shell_logical_lines(
        &entrypoint,
        &[
            r#"RUNTIME_COMMAND="${TENSORVM_ROLE_RUNTIME_COMMAND:-${ROLE}_run}""#,
            r#"LOCAL_CPU_ROLE_PRODUCER="${TENSORVM_LOCAL_CPU_ROLE_PRODUCER:-false}""#,
            r#"tvmd node init --data-dir "$DATA_DIR" > "$INIT_OUT""#,
            r#"tvmd node peer add --data-dir "$DATA_DIR" --peer-id "$BOOTSTRAP_PEER_ID" --address "$BOOTSTRAP_ADDRESS" > "$DATA_DIR/service-peer-add.out""#,
            r#"tvmd miner register --stake "$MINER_STAKE" > "$DATA_DIR/role-register.out""#,
            r#"tvmd miner check --wallet "$WALLET" --device cpu --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out""#,
            r#"tvmd validator register --stake "$VALIDATOR_STAKE" > "$DATA_DIR/role-register.out""#,
            r#"tvmd validator check --wallet "$WALLET" --node "$NODE_MULTIADDR" > "$DATA_DIR/role-start.out""#,
            r#"tvmd localnet seed --data-dir "$DATA_DIR" > "$DATA_DIR/local-testnet-seed.out""#,
            r#"tvmd node check --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" > "$DATA_DIR/service-readiness.out""#,
            r#"echo "runtime_command=$RUNTIME_COMMAND""#,
            r#"echo "local_cpu_role_producer=$LOCAL_CPU_ROLE_PRODUCER""#,
            r#"echo "public_evidence_full_spec=false""#,
            r#"echo "independently_checkable=false""#,
            r#"exec tvmd proposer run --wallet "$WALLET" --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
            r#"exec tvmd miner run --wallet "$WALLET" --device cpu --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
            r#"exec tvmd validator run --wallet "$WALLET" --node "$NODE_MULTIADDR" --listen "$RPC_LISTEN" --p2p-listen "$P2P_LISTEN" --data-dir "$DATA_DIR" --identity-seed "$IDENTITY_SEED" --auth-token "$AUTH_TOKEN" --max-requests 0"#,
        ],
    );

    assert_shell_logical_lines(
        &topology_script,
        &[
            r#"LOCAL_CPU_MINERS="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09""#,
            r#"LOCAL_CPU_VALIDATORS="validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"LOCAL_CPU_EXPECTED_SERVICES="$LOCAL_CPU_MINERS $LOCAL_CPU_VALIDATORS""#,
            r#"local_cpu_count_words() {"#,
            r#"for item in "$@"; do"#,
            r#"LOCAL_CPU_MINER_COUNT=$(local_cpu_count_words $LOCAL_CPU_MINERS)"#,
            r#"LOCAL_CPU_VALIDATOR_COUNT=$(local_cpu_count_words $LOCAL_CPU_VALIDATORS)"#,
            r#"LOCAL_CPU_EXPECTED_SERVICE_COUNT=$(local_cpu_count_words $LOCAL_CPU_EXPECTED_SERVICES)"#,
            r#"LOCAL_CPU_EXPECTED_SETTLED_RECEIPTS="$LOCAL_CPU_MINER_COUNT""#,
            r#"LOCAL_CPU_CUDA_REQUIRED_MINER_COUNT=0"#,
            r#"LOCAL_CPU_BOOTSTRAP_SERVICE=miner-00"#,
            r#"LOCAL_CPU_NETWORK_OBSERVER_SERVICE=miner-01"#,
            r#"LOCAL_CPU_DEFAULT_RESTART_SERVICES="miner-03 validator-02""#,
            r#"LOCAL_CPU_SEED_HEIGHT=2"#,
            r#"LOCAL_CPU_SEED_BLOCKS=2"#,
            r#"LOCAL_CPU_FULL_RATE_BPS=10000"#,
            r#"LOCAL_CPU_LIVE_PRIMITIVE_RECEIPT_FLOOR=5"#,
            r#"LOCAL_CPU_LIVE_RECEIPT_QUERY_LIMIT=500"#,
            r#"LOCAL_CPU_BLOCK_SCAN_DEPTH=40"#,
            r#"LOCAL_CPU_CHECKER_RETRY_LIMIT=30"#,
            r#"LOCAL_CPU_OPERATOR_CONVERGENCE_RETRY_LIMIT=60"#,
            r#"LOCAL_CPU_RESTART_CHECKER_RETRY_LIMIT=90"#,
            r#"LOCAL_CPU_DOCKER_EXEC_TIMEOUT_SECONDS=15"#,
            r#"LOCAL_CPU_HTTP_TIMEOUT_SECONDS=15"#,
            r#"LOCAL_CPU_RESTART_COMMAND_TIMEOUT_SECONDS=60"#,
            r#"LOCAL_CPU_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS=600"#,
            r#"LOCAL_CPU_CHECKER_RETRY_SLEEP_SECONDS=1"#,
        ],
    );

    assert_shell_logical_lines(
        &check_script,
        &[
            r#"TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh""#,
            r#"[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable""#,
            r#". "$TOPOLOGY_FILE""#,
            r#"EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES""#,
            r#"MINERS="$LOCAL_CPU_MINERS""#,
            r#"VALIDATORS="$LOCAL_CPU_VALIDATORS""#,
            r#"EXPECTED_SERVICE_COUNT="$LOCAL_CPU_EXPECTED_SERVICE_COUNT""#,
            r#"EXPECTED_MINER_COUNT="$LOCAL_CPU_MINER_COUNT""#,
            r#"EXPECTED_VALIDATOR_COUNT="$LOCAL_CPU_VALIDATOR_COUNT""#,
            r#"EXPECTED_SETTLED_RECEIPTS="$LOCAL_CPU_EXPECTED_SETTLED_RECEIPTS""#,
            r#"EXPECTED_CUDA_REQUIRED_MINER_COUNT="$LOCAL_CPU_CUDA_REQUIRED_MINER_COUNT""#,
            r#"EXPECTED_BOOTSTRAP_SERVICE="$LOCAL_CPU_BOOTSTRAP_SERVICE""#,
            r#"EXPECTED_NETWORK_OBSERVER_SERVICE="$LOCAL_CPU_NETWORK_OBSERVER_SERVICE""#,
            r#"EXPECTED_SEED_HEIGHT="$LOCAL_CPU_SEED_HEIGHT""#,
            r#"EXPECTED_SEED_BLOCKS="$LOCAL_CPU_SEED_BLOCKS""#,
            r#"EXPECTED_FULL_RATE_BPS="$LOCAL_CPU_FULL_RATE_BPS""#,
            r#"EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR="$LOCAL_CPU_LIVE_PRIMITIVE_RECEIPT_FLOOR""#,
            r#"EXPECTED_LIVE_RECEIPT_QUERY_LIMIT="$LOCAL_CPU_LIVE_RECEIPT_QUERY_LIMIT""#,
            r#"EXPECTED_BLOCK_SCAN_DEPTH="$LOCAL_CPU_BLOCK_SCAN_DEPTH""#,
            r#"EXPECTED_CHECKER_RETRY_LIMIT="$LOCAL_CPU_CHECKER_RETRY_LIMIT""#,
            r#"EXPECTED_OPERATOR_CONVERGENCE_RETRY_LIMIT="$LOCAL_CPU_OPERATOR_CONVERGENCE_RETRY_LIMIT""#,
            r#"EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS="$LOCAL_CPU_DOCKER_EXEC_TIMEOUT_SECONDS""#,
            r#"EXPECTED_HTTP_TIMEOUT_SECONDS="$LOCAL_CPU_HTTP_TIMEOUT_SECONDS""#,
            r#"EXPECTED_CHECKER_RETRY_SLEEP_SECONDS="$LOCAL_CPU_CHECKER_RETRY_SLEEP_SECONDS""#,
            r#"docker compose -f "$COMPOSE_FILE" "$@" < /dev/null"#,
            r#"require_command docker"#,
            r#"require_command sort"#,
            r#"require_command wc"#,
            r#"require_command curl"#,
            r#"require_command python3"#,
            r#"require_command timeout"#,
            r#"output=$(compose exec -T "$service" cat "$path") || return 1"#,
            r#"compose config --quiet"#,
            r#"CONFIG_SERVICES=$(compose config --services)"#,
            r#"[ "$(unique_count "$TMP_DIR/operator_ids")" = "$EXPECTED_SERVICE_COUNT" ] || fail "operator IDs are not distinct""#,
            r#"[ "$(unique_count "$TMP_DIR/p2p_peer_ids")" = "$EXPECTED_SERVICE_COUNT" ] || fail "libp2p peer IDs are not distinct""#,
            r#"[ "$(unique_count "$TMP_DIR/node_multiaddrs")" = "$EXPECTED_SERVICE_COUNT" ] || fail "node multiaddrs are not distinct""#,
            r#"READY_REPORT=$(read_ready_report "$service") || fail "$service has not written /var/lib/tensorvm/local-cpu-ready""#,
            r#"[ "$(status_value operator_name "$READY_REPORT")" = "$service" ] || fail "$service readiness file does not name its operator""#,
            r#"[ "$(status_value p2p_runtime "$READY_REPORT")" = "libp2p" ] || fail "$service is missing libp2p runtime readiness""#,
            r#"[ "$(status_value node_store_ready "$READY_REPORT")" = "true" ] || fail "$service is missing node store readiness""#,
            r#"[ "$(status_value libp2p_ready "$READY_REPORT")" = "true" ] || fail "$service is missing libp2p readiness""#,
            r#"[ "$(status_value p2p_identity_seeded "$READY_REPORT")" = "true" ] || fail "$service is missing stable libp2p identity readiness""#,
            r#"[ "$(status_value p2p_identity_seed "$READY_REPORT")" = "$operator_id" ] || fail "$service libp2p identity seed does not match its operator ID""#,
            r#"READY_LOCAL_CPU_ROLE_PRODUCER=$(status_value local_cpu_role_producer "$READY_REPORT")"#,
            r#"[ -n "$READY_LOCAL_CPU_ROLE_PRODUCER" ] || fail "$service readiness file does not report local CPU producer mode""#,
            r#"[ "$(status_value chain_profile "$READY_REPORT")" = "local_cpu" ] || fail "$service readiness file does not report the local CPU chain profile""#,
            r#"READY_P2P_PEER_ID=$(status_value p2p_peer_id "$READY_REPORT")"#,
            r#"[ -n "$READY_P2P_PEER_ID" ] || fail "$service readiness file does not report a libp2p peer ID""#,
            r#"READY_ROLE=$(status_value role "$READY_REPORT")"#,
            r#"READY_RUNTIME_COMMAND=$(status_value runtime_command "$READY_REPORT")"#,
            r#"[ "$READY_ROLE" = "miner" ] || fail "$service is not marked as a miner""#,
            r#"[ "$READY_RUNTIME_COMMAND" = "miner_run" ] || fail "$service is not running the miner role command""#,
            r#"[ "$(status_value device "$READY_REPORT")" = "cpu" ] || fail "$service is not using the CPU backend""#,
            r#"[ "$READY_ROLE" = "validator" ] || fail "$service is not marked as a validator""#,
            r#"[ "$READY_RUNTIME_COMMAND" = "validator_run" ] || fail "$service is not running the validator role command""#,
            r#"[ "$(status_value reference_verifier_ready "$READY_REPORT")" = "true" ] || fail "$service validator readiness is missing""#,
            r#"printf '%s\n' "$READY_P2P_PEER_ID" >> "$TMP_DIR/p2p_peer_ids""#,
            r#"printf '%s\n' "$operator_id" >> "$TMP_DIR/operator_ids""#,
            r#"SEED_REPORT=$(read_seed_report "$service") || fail "$service did not seed local testnet chain state""#,
            r#"[ "$(status_value command "$SEED_REPORT")" = "local_testnet_seed" ] || fail "$service did not seed local testnet chain state""#,
            r#"[ "$(status_value height "$SEED_REPORT")" = "$EXPECTED_SEED_HEIGHT" ] || fail "$service seeded local testnet did not start at height $EXPECTED_SEED_HEIGHT""#,
            r#"[ "$(status_value blocks "$SEED_REPORT")" = "$EXPECTED_SEED_BLOCKS" ] || fail "$service seeded local testnet did not start with $EXPECTED_SEED_BLOCKS blocks""#,
            r#"LOCAL_CPU_VERIFY=$(compose exec -T "$service" tvmd localnet verify --data-dir /var/lib/tensorvm --json | tr -d '\r')"#,
            r#"json_bool_true structured_verifier_ready "$LOCAL_CPU_VERIFY" || fail "$service local CPU structured verifier is not ready""#,
            r#"json_bool_true ready "$LOCAL_CPU_VERIFY" || fail "$service local CPU structured verifier did not accept node store""#,
            r#"MINER_SEED_REPORT=$(read_seed_report "$EXPECTED_BOOTSTRAP_SERVICE") || fail "$EXPECTED_BOOTSTRAP_SERVICE did not seed local testnet chain state""#,
            r#"[ "$(status_value command "$MINER_SEED_REPORT")" = "local_testnet_seed" ] || fail "$EXPECTED_BOOTSTRAP_SERVICE did not seed local testnet chain state""#,
            r#"SEED_SETTLED_RECEIPTS=$(status_value settled_receipts "$MINER_SEED_REPORT")"#,
            r#"[ "$SEED_SETTLED_RECEIPTS" = "$EXPECTED_SETTLED_RECEIPTS" ] || fail "seeded local testnet did not report settled receipts""#,
            r#"SEED_MATMUL_SETTLED=$(status_value matmul_settled "$MINER_SEED_REPORT")"#,
            r#"[ "$SEED_MATMUL_SETTLED" = "true" ] || fail "seeded local testnet did not settle matmul work""#,
            r#"SEED_LINEAR_TRAINING_SETTLED=$(status_value linear_training_settled "$MINER_SEED_REPORT")"#,
            r#"[ "$SEED_LINEAR_TRAINING_SETTLED" = "true" ] || fail "seeded local testnet did not settle linear training work""#,
            r#"SEED_FINALITY_RATE_BPS=$(status_value finality_rate_bps "$MINER_SEED_REPORT")"#,
            r#"[ "$SEED_FINALITY_RATE_BPS" = "$EXPECTED_FULL_RATE_BPS" ] || fail "seeded local testnet did not report full finality""#,
            r#"SEED_DATA_AVAILABILITY_BPS=$(status_value data_availability_bps "$MINER_SEED_REPORT")"#,
            r#"[ "$SEED_DATA_AVAILABILITY_BPS" = "$EXPECTED_FULL_RATE_BPS" ] || fail "seeded local testnet did not report full data availability""#,
            r#"SEED_REWARDED_MINERS=$(status_value rewarded_miners "$MINER_SEED_REPORT")"#,
            r#"[ "${SEED_REWARDED_MINERS:-0}" -gt 0 ] || fail "seeded local testnet did not report miner rewards""#,
            r#"SEED_PENDING_RECEIPT_REWARDS=$(status_value pending_receipt_rewards "$MINER_SEED_REPORT")"#,
            r#"[ "${SEED_PENDING_RECEIPT_REWARDS:-0}" -gt 0 ] || fail "seeded local testnet did not report pending receipt rewards""#,
            r#"SEED_TOTAL_REWARD_BALANCE=$(status_value total_reward_balance "$MINER_SEED_REPORT")"#,
            r#"[ -n "$SEED_TOTAL_REWARD_BALANCE" ] || fail "seeded local testnet did not report total reward balance""#,
            r#"SEED_ATTESTATION_COUNT=$(status_value attestation_count "$MINER_SEED_REPORT")"#,
            r#"[ -n "$SEED_ATTESTATION_COUNT" ] || fail "seeded local testnet did not report attestation count""#,
            r#"ready_miners=${EXPECTED_MINER_COUNT}"#,
            r#"ready_validators=${EXPECTED_VALIDATOR_COUNT}"#,
            r#"distinct_operator_ids=${EXPECTED_SERVICE_COUNT}"#,
            r#"distinct_libp2p_peer_ids=${EXPECTED_SERVICE_COUNT}"#,
            r#"distinct_node_multiaddrs=${EXPECTED_SERVICE_COUNT}"#,
            r#"libp2p_ready_node_count=${EXPECTED_SERVICE_COUNT}"#,
            r#"cpu_ready_miner_count=${EXPECTED_MINER_COUNT}"#,
            r#"cuda_required_miner_count=${EXPECTED_CUDA_REQUIRED_MINER_COUNT}"#,
            r#"settled_receipts=${EXPECTED_SETTLED_RECEIPTS}"#,
            r#"matmul_settled=true"#,
            r#"linear_training_settled=true"#,
            r#"rewarded_miners=${SEED_REWARDED_MINERS}"#,
            r#"pending_receipt_rewards=${SEED_PENDING_RECEIPT_REWARDS}"#,
            r#"finality_rate_bps=${EXPECTED_FULL_RATE_BPS}"#,
            r#"data_availability_bps=${EXPECTED_FULL_RATE_BPS}"#,
        ],
    );

    assert_lacks_shell_logical_lines(
        &check_script,
        &[
            r#"[ "$(unique_count "$TMP_DIR/operator_ids")" = "15" ] || fail "operator IDs are not distinct""#,
            r#"[ "$(unique_count "$TMP_DIR/p2p_peer_ids")" = "15" ] || fail "libp2p peer IDs are not distinct""#,
            r#"[ "$(unique_count "$TMP_DIR/node_multiaddrs")" = "15" ] || fail "node multiaddrs are not distinct""#,
            r#"[ "$SEED_SETTLED_RECEIPTS" = "10" ] || fail "seeded local testnet did not report settled receipts""#,
            r#"ready_miners=10"#,
            r#"ready_validators=5"#,
            r#"distinct_operator_ids=15"#,
            r#"distinct_libp2p_peer_ids=15"#,
            r#"distinct_node_multiaddrs=15"#,
            r#"libp2p_ready_node_count=15"#,
            r#"cpu_ready_miner_count=10"#,
            r#"cuda_required_miner_count=0"#,
            r#"settled_receipts=10"#,
            r#"all_operator_status_count=15"#,
            r#"[ "$(status_value height "$SEED_REPORT")" = "2" ] || fail "$service seeded local testnet did not start at height 2""#,
            r#"[ "$(status_value blocks "$SEED_REPORT")" = "2" ] || fail "$service seeded local testnet did not start with 2 blocks""#,
            r#"MINER_SEED_REPORT=$(read_seed_report miner-00) || fail "miner-00 did not seed local testnet chain state""#,
            r#"[ "$(status_value command "$MINER_SEED_REPORT")" = "local_testnet_seed" ] || fail "miner-00 did not seed local testnet chain state""#,
            r#"[ "$SEED_FINALITY_RATE_BPS" = "10000" ] || fail "seeded local testnet did not report full finality""#,
            r#"[ "$SEED_DATA_AVAILABILITY_BPS" = "10000" ] || fail "seeded local testnet did not report full data availability""#,
            r#"finality_rate_bps=10000"#,
            r#"data_availability_bps=10000"#,
            r#"TARGET_STATUS_RAW=$(read_service_status miner-01) || fail "could not read miner-01 network-observed service status""#,
            r#"if BLOCK_RAW=$(read_service_block miner-00 "$BLOCK_SCAN_HEIGHT"); then"#,
            r#"BLOCK_SCAN_START=$((LIVE_HEIGHT - 40))"#,
            r#"while [ "$attempt" -lt 30 ]; do"#,
            r#"while [ "$attempt" -lt 60 ]; do"#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then"#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then"#,
            r#"sleep 1"#,
            r#"if NETWORK_BLOCK_RAW=$(read_service_block miner-01 "$CANDIDATE_NETWORK_HEAD_HEIGHT"); then"#,
            r#"[ "${LIVE_HEIGHT:-0}" -gt 2 ] || fail "gateway chain head did not advance past seeded height 2""#,
            r#"[ "${LIVE_BLOCK_COUNT:-0}" -gt 2 ] || fail "gateway chain block count did not advance past seeded 2 blocks""#,
            r#"[ "${LIVE_JOB_COUNT:-0}" -gt 2 ] || fail "protocol did not generate synthetic jobs after seed""#,
            r#"[ "${LIVE_RECEIPT_COUNT:-0}" -gt 10 ] || fail "synthetic jobs did not produce additional receipts""#,
            r#"[ "${LIVE_SETTLED_RECEIPT_COUNT:-0}" -gt 10 ] || fail "synthetic jobs did not settle additional receipts""#,
            r#"[ "${LIVE_ATTESTED_RECEIPT_COUNT:-0}" -gt 10 ] || fail "live receipt details did not include validator attestations""#,
            r#"[ "${LIVE_TENSOR_OP_RECEIPT_COUNT:-0}" -gt 5 ] || fail "live receipt details did not include post-seed TensorOp receipts""#,
            r#"[ "${LIVE_LINEAR_TRAINING_RECEIPT_COUNT:-0}" -gt 5 ] || fail "live receipt details did not include post-seed LinearTrainingStep receipts""#,
            r#"LIVE_RECEIPTS=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/receipts/latest/500")"#,
            r#"curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}${path}" >/dev/null || fail "gateway route is not reachable: $path""#,
            r#"EXPLORER_HEALTH=$(curl -fsS --max-time 15 "http://127.0.0.1:${EXPLORER_PORT}/health")"#,
            r#"EXPLORER_PAGE=$(curl -fsS --max-time 15 "http://127.0.0.1:${EXPLORER_PORT}/")"#,
            r#"LIVE_CHAIN_HEAD=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/chain/head")"#,
            r#"LIVE_OVERVIEW=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/overview")"#,
            r#"LIVE_RECEIPTS=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/receipts/latest/${EXPECTED_LIVE_RECEIPT_QUERY_LIMIT}")"#,
            r#"LIVE_TENSOR=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/latest")"#,
            r#"LIVE_TENSOR_DESCRIPTOR=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/descriptor")"#,
            r#"LIVE_TENSOR_ROW=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/row/0")"#,
            r#"LIVE_TENSOR_CHUNK=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/chunk/0")"#,
            r#"LIVE_TENSOR_OPENING=$(curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/opening/0")"#,
            concat!(
                r#"if [ "$SERVICE_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_BLOCK_COUNT" -le 2 ] "#,
                r#"|| [ "$SERVICE_LATEST_BLOCK_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_LATEST_BLOCK_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_STATE_ROOT" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_BLOCK_LOG_ROOT" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_FINALIZED_BLOCK_COUNT" -le 2 ] "#,
                r#"|| [ "$SERVICE_FIRST_LIVE_BLOCK_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_REGISTERED_MINER_COUNT" -ne 10 ] "#,
                r#"|| [ "$SERVICE_REGISTERED_VALIDATOR_COUNT" -ne 5 ] "#,
                r#"|| [ "$SERVICE_JOB_COUNT" -le 2 ] "#,
                r#"|| [ "$SERVICE_RECEIPT_COUNT" -le 10 ] "#,
                r#"|| [ "$SERVICE_ATTESTATION_COUNT" -le "$SEED_ATTESTATION_COUNT" ] "#,
                r#"|| [ "$SERVICE_ROLE_LATEST_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_CONNECTED_PEERS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCKS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_JOBS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_RECEIPTS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT" -le 2 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_FIRST_LIVE_BLOCK_HASH" = "$ZERO_HASH" ]; then"#
            ),
            r#"EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"MINERS="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09""#,
            r#"VALIDATORS="validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"printf '%s\n' "$LOCAL_CPU_VERIFY" | grep -q '"structured_verifier_ready":true' || fail "$service local CPU structured verifier is not ready""#,
            r#"printf '%s\n' "$LOCAL_CPU_VERIFY" | grep -q '"ready":true' || fail "$service local CPU structured verifier did not accept node store""#,
            r#"compose exec -T "$service" test -f /var/lib/tensorvm/local-cpu-ready || fail "$service has not written /var/lib/tensorvm/local-cpu-ready""#,
            r#"compose exec -T "$service" grep -q "operator_name=$service" /var/lib/tensorvm/local-cpu-ready || fail "$service readiness file does not name its operator""#,
            r#"compose exec -T "$service" grep -q "p2p_runtime=libp2p" /var/lib/tensorvm/local-cpu-ready || fail "$service is missing libp2p runtime readiness""#,
            r#"compose exec -T "$service" grep -q "node_store_ready=true" /var/lib/tensorvm/local-cpu-ready || fail "$service is missing node store readiness""#,
            r#"compose exec -T "$service" grep -q "libp2p_ready=true" /var/lib/tensorvm/local-cpu-ready || fail "$service is missing libp2p readiness""#,
            r#"compose exec -T "$service" grep -q "p2p_identity_seeded=true" /var/lib/tensorvm/local-cpu-ready || fail "$service is missing stable libp2p identity readiness""#,
            r#"compose exec -T "$service" grep -q "p2p_identity_seed=$operator_id" /var/lib/tensorvm/local-cpu-ready || fail "$service libp2p identity seed does not match its operator ID""#,
            r#"compose exec -T "$service" grep -q "^local_cpu_role_producer=" /var/lib/tensorvm/local-cpu-ready || fail "$service readiness file does not report local CPU producer mode""#,
            r#"compose exec -T "$service" grep -q "^chain_profile=local_cpu" /var/lib/tensorvm/local-cpu-ready || fail "$service readiness file does not report the local CPU chain profile""#,
            r#"compose exec -T "$service" grep "^p2p_peer_id=" /var/lib/tensorvm/local-cpu-ready >> "$TMP_DIR/p2p_peer_ids""#,
            r#"compose exec -T "$service" grep -q "role=miner" /var/lib/tensorvm/local-cpu-ready || fail "$service is not marked as a miner""#,
            r#"compose exec -T "$service" grep -q "runtime_command=miner_run" /var/lib/tensorvm/local-cpu-ready || fail "$service is not running the miner role command""#,
            r#"compose exec -T "$service" grep -q "device=cpu" /var/lib/tensorvm/local-cpu-ready || fail "$service is not using the CPU backend""#,
            r#"compose exec -T "$service" grep -q "role=validator" /var/lib/tensorvm/local-cpu-ready || fail "$service is not marked as a validator""#,
            r#"compose exec -T "$service" grep -q "runtime_command=validator_run" /var/lib/tensorvm/local-cpu-ready || fail "$service is not running the validator role command""#,
            r#"compose exec -T "$service" grep -q "reference_verifier_ready=true" /var/lib/tensorvm/local-cpu-ready || fail "$service validator readiness is missing""#,
            r#"compose exec -T "$service" grep -q "command=local_testnet_seed" /var/lib/tensorvm/local-testnet-seed.out || fail "$service did not seed local testnet chain state""#,
            r#"compose exec -T "$service" grep -q "height=2" /var/lib/tensorvm/local-testnet-seed.out || fail "$service seeded local testnet did not start at height 2""#,
            r#"compose exec -T "$service" grep -q "blocks=2" /var/lib/tensorvm/local-testnet-seed.out || fail "$service seeded local testnet did not start with 2 blocks""#,
            r#"compose exec -T miner-00 grep -q "command=local_testnet_seed" /var/lib/tensorvm/local-testnet-seed.out || fail "miner-00 did not seed local testnet chain state""#,
            r#"compose exec -T miner-00 grep -q "settled_receipts=10" /var/lib/tensorvm/local-testnet-seed.out || fail "seeded local testnet did not report settled receipts""#,
            r#"compose exec -T miner-00 grep -q "matmul_settled=true" /var/lib/tensorvm/local-testnet-seed.out || fail "seeded local testnet did not settle matmul work""#,
            r#"compose exec -T miner-00 grep -q "linear_training_settled=true" /var/lib/tensorvm/local-testnet-seed.out || fail "seeded local testnet did not settle linear training work""#,
            r#"compose exec -T miner-00 grep -q "finality_rate_bps=10000" /var/lib/tensorvm/local-testnet-seed.out || fail "seeded local testnet did not report full finality""#,
            r#"compose exec -T miner-00 grep -q "data_availability_bps=10000" /var/lib/tensorvm/local-testnet-seed.out || fail "seeded local testnet did not report full data availability""#,
            r#"SEED_REWARDED_MINERS=$(seed_report_value rewarded_miners)"#,
            r#"SEED_TOTAL_REWARD_BALANCE=$(seed_report_value total_reward_balance)"#,
            r#"SEED_ATTESTATION_COUNT=$(seed_report_value attestation_count)"#,
            r#"printf '%s\n' "$document" | tr ',' '\n' | sed -n "s/.*\"$key\":\([0-9][0-9]*\).*/\1/p" | sed -n '1p'"#,
            r#"printf '%s\n' "$document" | tr ',' '\n' | sed -n "s/.*\"$key\":\"\([^\"]*\)\".*/\1/p" | sed -n '1p'"#,
            r#"printf '%s\n' "$document" | grep -o "\"$key\":[1-9][0-9]*" | wc -l | tr -d ' '"#,
            r#"printf '%s\n' "$document" | grep -o "\"$key\":\"$value\"" | wc -l | tr -d ' '"#,
            r#"printf '%s\n' "$EXPLORER_HEALTH" | grep -q '"tensorvm_explorer_ready":true' || fail "standalone explorer health is not ready""#,
            r#"printf '%s\n' "$EXPLORER_HEALTH" | grep -q '/explorer/ws?token=' || fail "standalone explorer does not publish the TensorVM websocket URL""#,
            r#"curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/descriptor" | grep -q '"root":"' || fail "live tensor descriptor was not fetchable""#,
            r#"curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/row/0" | grep -q '"row":' || fail "live tensor row was not fetchable""#,
            r#"curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/chunk/0" | grep -q '"bytes":"' || fail "live tensor chunk was not fetchable""#,
            r#"curl -fsS --max-time 15 -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/opening/0" | grep -q '"proof_len":' || fail "live tensor opening was not fetchable""#,
            r#"printf '%s\n' "$EXPLORER_PAGE" | grep -q 'TensorVM Explorer' || fail "standalone explorer page is not reachable""#,
            r#"printf '%s\n' "$EXPLORER_PAGE" | grep -q 'data-ui="ratzilla-tui"' || fail "standalone explorer page is not the default Ratzilla-style TUI""#,
            r#"printf '%s\n' "$EXPLORER_PAGE" | grep -q 'new WebSocket' || fail "standalone explorer page does not poll TensorVM over websocket""#,
            r#"require_command grep"#,
            r#"require_command sed"#,
            r#"output=$(compose exec -T "$service" sed -n 'p' "$path") || return 1"#,
            r#"printf '%s\n' "$document" | sed -n "s/^${key}=//p" | sed -n '1p'"#,
        ],
    );

    assert_shell_logical_lines(
        &check_script,
        &[
            r#"curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}${path}" >/dev/null || fail "gateway route is not reachable: $path""#,
            r#"EXPLORER_HEALTH=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" "http://127.0.0.1:${EXPLORER_PORT}/health")"#,
            r#"json_bool_true tensorvm_explorer_ready "$EXPLORER_HEALTH" || fail "standalone explorer health is not ready""#,
            r#"EXPLORER_WS_URL=$(json_string websocket_url "$EXPLORER_HEALTH") || fail "standalone explorer does not publish the TensorVM websocket URL""#,
            r#"text_contains "$EXPLORER_WS_URL" "/explorer/ws?token=" || fail "standalone explorer does not publish the TensorVM websocket URL""#,
            r#"EXPLORER_PAGE=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" "http://127.0.0.1:${EXPLORER_PORT}/")"#,
            r#"text_contains "$EXPLORER_PAGE" "TensorVM Explorer" || fail "standalone explorer page is not reachable""#,
            r#"text_contains "$EXPLORER_PAGE" 'data-ui="ratzilla-tui"' || fail "standalone explorer page is not the default Ratzilla-style TUI""#,
            r#"text_contains "$EXPLORER_PAGE" "new WebSocket" || fail "standalone explorer page does not poll TensorVM over websocket""#,
            r#"LIVE_SETTLED_RECEIPT_COUNT=$(json_number settled_receipt_count "$LIVE_OVERVIEW")"#,
            r#"LIVE_PENDING_RECEIPT_REWARD_COUNT=$(json_number pending_receipt_reward_count "$LIVE_OVERVIEW")"#,
            r#"LIVE_RECEIPTS=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/explorer/receipts/latest/${EXPECTED_LIVE_RECEIPT_QUERY_LIMIT}")"#,
            r#"[ "${LIVE_HEIGHT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "gateway chain head did not advance past seeded height $EXPECTED_SEED_HEIGHT""#,
            r#"[ "${LIVE_BLOCK_COUNT:-0}" -gt "$EXPECTED_SEED_BLOCKS" ] || fail "gateway chain block count did not advance past seeded $EXPECTED_SEED_BLOCKS blocks""#,
            r#"[ "${LIVE_JOB_COUNT:-0}" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "protocol did not generate synthetic jobs after seed""#,
            r#"[ "${LIVE_MODEL_COUNT:-0}" -gt 1 ] || fail "protocol did not settle a live LinearTrainingStep after seed""#,
            r#"[ "${LIVE_ATTESTATION_COUNT:-0}" -gt "$SEED_ATTESTATION_COUNT" ] || fail "live synthetic jobs did not add validator attestations""#,
            r#"[ "${LIVE_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "synthetic jobs did not produce additional receipts""#,
            r#"[ "${LIVE_SETTLED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "synthetic jobs did not settle additional receipts""#,
            r#"[ "${LIVE_ATTESTED_RECEIPT_COUNT:-0}" -gt "$EXPECTED_SETTLED_RECEIPTS" ] || fail "live receipt details did not include validator attestations""#,
            r#"[ "${LIVE_TENSOR_OP_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] || fail "live receipt details did not include post-seed TensorOp receipts""#,
            r#"[ "${LIVE_LINEAR_TRAINING_RECEIPT_COUNT:-0}" -gt "$EXPECTED_LIVE_PRIMITIVE_RECEIPT_FLOOR" ] || fail "live receipt details did not include post-seed LinearTrainingStep receipts""#,
            r#"[ "${LIVE_PENDING_RECEIPT_REWARD_COUNT:-0}" -gt "$SEED_PENDING_RECEIPT_REWARDS" ] || fail "live synthetic jobs did not add pending receipt rewards""#,
            r#"LIVE_TENSOR=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/latest")"#,
            r#"LIVE_TENSOR_ID=$(json_string tensor_id "$LIVE_TENSOR")"#,
            r#"[ -n "$LIVE_TENSOR_ID" ] || fail "live tensor route did not report a tensor id""#,
            r#"LIVE_TENSOR_ROOT=$(json_string root "$LIVE_TENSOR")"#,
            r#"[ -n "$LIVE_TENSOR_ROOT" ] || fail "live tensor route did not report a tensor root""#,
            r#"[ "$(json_number tensor_count "$LIVE_TENSOR")" -gt 0 ] || fail "live tensor route did not report retained tensors""#,
            r#"LIVE_TENSOR_DESCRIPTOR=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/descriptor")"#,
            r#"LIVE_TENSOR_DESCRIPTOR_ROOT=$(json_string root "$LIVE_TENSOR_DESCRIPTOR") || fail "live tensor descriptor was not fetchable""#,
            r#"[ "$LIVE_TENSOR_DESCRIPTOR_ROOT" = "$LIVE_TENSOR_ROOT" ] || fail "live tensor descriptor root did not match latest tensor root""#,
            r#"LIVE_TENSOR_ROW=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/row/0")"#,
            r#"[ "$(json_array_length row "$LIVE_TENSOR_ROW")" -gt 0 ] || fail "live tensor row was not fetchable""#,
            r#"LIVE_TENSOR_CHUNK=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/chunk/0")"#,
            r#"LIVE_TENSOR_CHUNK_BYTES=$(json_string bytes "$LIVE_TENSOR_CHUNK") || fail "live tensor chunk was not fetchable""#,
            r#"[ -n "$LIVE_TENSOR_CHUNK_BYTES" ] || fail "live tensor chunk was empty""#,
            r#"[ "$(json_number chunk_index "$LIVE_TENSOR_CHUNK")" = "0" ] || fail "live tensor chunk index did not match request""#,
            r#"LIVE_TENSOR_OPENING=$(curl -fsS --max-time "$EXPECTED_HTTP_TIMEOUT_SECONDS" -H "Authorization: Bearer ${AUTH_TOKEN}" "http://127.0.0.1:${RPC_PORT}/tensor/${LIVE_TENSOR_ID}/opening/0")"#,
            r#"LIVE_TENSOR_OPENING_PROOF_LEN=$(json_number proof_len "$LIVE_TENSOR_OPENING") || fail "live tensor opening was not fetchable""#,
            r#"[ -n "$LIVE_TENSOR_OPENING_PROOF_LEN" ] || fail "live tensor opening did not report a proof length""#,
            r#"[ "$(json_number chunk_index "$LIVE_TENSOR_OPENING")" = "0" ] || fail "live tensor opening index did not match request""#,
            r#"standalone_explorer_ready=true"#,
            r#"standalone_explorer_websocket_polling=true"#,
            r#"live_block_production=true"#,
            r#"live_synthetic_jobs=true"#,
            r#"live_linear_training_jobs=true"#,
            r#"live_attestations=true"#,
            r#"live_receipt_attestations=true"#,
            r#"live_tensor_op_receipts=true"#,
            r#"live_linear_training_receipts=true"#,
            r#"live_tensor_op_block_evidence=true"#,
            r#"live_tensor_op_block_height=${LIVE_TENSOR_OP_BLOCK_HEIGHT}"#,
            r#"live_linear_training_block_evidence=true"#,
            r#"live_linear_training_block_height=${LIVE_LINEAR_TRAINING_BLOCK_HEIGHT}"#,
            r#"live_tensor_fetch=true"#,
            r#"live_rewards=true"#,
        ],
    );
    assert!(
        !shell_logical_lines(&check_script)
            .iter()
            .any(|line| line.starts_with("cargo test")),
        "deployment checker should not run unit tests"
    );
    assert!(
        !shell_logical_lines(&check_script)
            .iter()
            .any(|line| line == "require_command cargo"),
        "deployment checker should not require a Rust toolchain"
    );

    assert_status_value_reads(
        &check_script,
        "STATUS",
        &[
            ("SERVICE_LATEST_BLOCK_HEIGHT", "latest_block_height"),
            ("SERVICE_BLOCK_LOG_ROOT", "block_log_root"),
            ("SERVICE_REGISTERED_MINER_COUNT", "registered_miner_count"),
            (
                "SERVICE_REGISTERED_VALIDATOR_COUNT",
                "registered_validator_count",
            ),
            ("SERVICE_JOB_COUNT", "job_count"),
            ("SERVICE_RECEIPT_COUNT", "receipt_count"),
            ("SERVICE_ATTESTATION_COUNT", "attestation_count"),
            ("SERVICE_ROLE_RUNTIME_COMMAND", "role_runtime_command"),
            ("SERVICE_ROLE_LOOP_READY", "role_loop_ready"),
            ("SERVICE_ROLE_LOOP_ROLE", "role_loop_role"),
            ("SERVICE_ROLE_CHAIN_PROFILE", "role_chain_profile"),
            ("SERVICE_ROLE_CAN_PRODUCE_BLOCKS", "role_can_produce_blocks"),
            ("SERVICE_ROLE_WALLET_ADDRESS", "role_wallet_address"),
            (
                "SERVICE_ROLE_WALLET_REGISTRATION",
                "role_wallet_registration",
            ),
            ("SERVICE_ROLE_WALLET_REGISTERED", "role_wallet_registered"),
            ("SERVICE_ROLE_MINER_WORK_READY", "role_miner_work_ready"),
            (
                "SERVICE_ROLE_MINER_ASSIGNED_JOBS_SEEN",
                "role_miner_assigned_jobs_seen",
            ),
            (
                "SERVICE_ROLE_MINER_UNRECEIPTED_JOBS",
                "role_miner_unreceipted_jobs",
            ),
            (
                "SERVICE_ROLE_MINER_RECEIPTS_SUBMITTED",
                "role_miner_receipts_submitted",
            ),
            (
                "SERVICE_ROLE_MINER_TENSORS_INSERTED",
                "role_miner_tensors_inserted",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_WORK_READY",
                "role_validator_work_ready",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_ASSIGNED_RECEIPTS_SEEN",
                "role_validator_assigned_receipts_seen",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_UNATTESTED_RECEIPTS",
                "role_validator_unattested_receipts",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_ARTIFACT_READY_RECEIPTS",
                "role_validator_artifact_ready_receipts",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_ARTIFACT_MISSING_RECEIPTS",
                "role_validator_artifact_missing_receipts",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_ATTEMPTS",
                "role_validator_remote_tensor_fetch_attempts",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_SUCCESSES",
                "role_validator_remote_tensor_fetch_successes",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_FAILURES",
                "role_validator_remote_tensor_fetch_failures",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_REMOTE_FETCH_BYTES",
                "role_validator_remote_tensor_fetch_bytes",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_REMOTE_TENSORS_INSERTED",
                "role_validator_remote_tensors_inserted",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED",
                "role_validator_attestations_submitted",
            ),
            (
                "SERVICE_ROLE_VALIDATOR_BLOCK_VOTES_SUBMITTED",
                "role_validator_block_votes_submitted",
            ),
            ("SERVICE_ROLE_LOCAL_PRODUCER", "role_local_producer"),
            (
                "SERVICE_ROLE_NETWORK_APPLIED_BLOCKS",
                "role_network_applied_blocks",
            ),
            (
                "SERVICE_ROLE_NETWORK_EVENTS",
                "role_network_events_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_BLOCK_HEADERS",
                "role_network_block_headers_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS",
                "role_network_block_payloads_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_BLOCK_PAYLOADS_APPLIED",
                "role_network_block_payloads_applied",
            ),
            (
                "SERVICE_ROLE_NETWORK_BLOCK_VOTES",
                "role_network_block_votes_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_BLOCK_VOTES_APPLIED",
                "role_network_block_votes_applied",
            ),
            (
                "SERVICE_ROLE_NETWORK_JOB_EVENTS",
                "role_network_job_events_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_JOB_PAYLOADS",
                "role_network_job_payloads_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_JOB_PAYLOADS_APPLIED",
                "role_network_job_payloads_applied",
            ),
            (
                "SERVICE_ROLE_NETWORK_RECEIPT_EVENTS",
                "role_network_receipt_events_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS",
                "role_network_receipt_payloads_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_RECEIPT_PAYLOADS_APPLIED",
                "role_network_receipt_payloads_applied",
            ),
            (
                "SERVICE_ROLE_NETWORK_ATTESTATION_EVENTS",
                "role_network_attestation_events_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS",
                "role_network_attestation_payloads_ingested",
            ),
            (
                "SERVICE_ROLE_NETWORK_ATTESTATION_PAYLOADS_APPLIED",
                "role_network_attestation_payloads_applied",
            ),
            (
                "SERVICE_ROLE_NETWORK_INVALID_EVENTS",
                "role_network_invalid_events",
            ),
            (
                "SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS",
                "role_p2p_observed_block_payloads",
            ),
            (
                "SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES",
                "role_p2p_observed_block_votes",
            ),
            (
                "SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT",
                "role_p2p_latest_observed_block_height",
            ),
            (
                "SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT",
                "role_p2p_latest_observed_block_payload_height",
            ),
            (
                "SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH",
                "role_p2p_latest_observed_block_payload_hash",
            ),
            (
                "SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOAD_HASHES",
                "role_p2p_observed_block_payload_hashes",
            ),
        ],
    );
    assert_shell_logical_lines(
        &check_script,
        &[
            r#"TARGET_STATUS_RAW=$(read_service_status "$EXPECTED_NETWORK_OBSERVER_SERVICE") || fail "could not read $EXPECTED_NETWORK_OBSERVER_SERVICE network-observed service status""#,
            r#"CANDIDATE_NETWORK_HEAD_HEIGHT=$(status_value role_p2p_latest_observed_block_payload_height "$TARGET_STATUS")"#,
            r#"if STATUS_RAW=$(read_service_status "$service"); then"#,
            concat!(
                r#"if [ "$SERVICE_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_BLOCK_COUNT" -le "$EXPECTED_SEED_BLOCKS" ] "#,
                r#"|| [ "$SERVICE_LATEST_BLOCK_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_LATEST_BLOCK_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_STATE_ROOT" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_BLOCK_LOG_ROOT" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_FINALIZED_BLOCK_COUNT" -le "$EXPECTED_SEED_BLOCKS" ] "#,
                r#"|| [ "$SERVICE_FIRST_LIVE_BLOCK_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_REGISTERED_MINER_COUNT" -ne "$EXPECTED_MINER_COUNT" ] "#,
                r#"|| [ "$SERVICE_REGISTERED_VALIDATOR_COUNT" -ne "$EXPECTED_VALIDATOR_COUNT" ] "#,
                r#"|| [ "$SERVICE_JOB_COUNT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_RECEIPT_COUNT" -le "$EXPECTED_SETTLED_RECEIPTS" ] "#,
                r#"|| [ "$SERVICE_ATTESTATION_COUNT" -le "$SEED_ATTESTATION_COUNT" ] "#,
                r#"|| [ "$SERVICE_ROLE_LATEST_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_CONNECTED_PEERS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCKS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_PAYLOADS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_BLOCK_VOTES" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_JOBS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_RECEIPTS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_OBSERVED_ATTESTATIONS" -le 0 ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HEIGHT" -le "$EXPECTED_SEED_HEIGHT" ] "#,
                r#"|| [ "$SERVICE_ROLE_P2P_LATEST_OBSERVED_BLOCK_PAYLOAD_HASH" = "$ZERO_HASH" ] "#,
                r#"|| [ "$SERVICE_FIRST_LIVE_BLOCK_HASH" = "$ZERO_HASH" ]; then"#
            ),
            r#"all_operator_status_count=${EXPECTED_SERVICE_COUNT}"#,
            r#"all_operator_min_height=${ALL_OPERATOR_MIN_HEIGHT}"#,
            r#"all_operator_first_live_block_hash=${ALL_OPERATOR_FIRST_LIVE_BLOCK_HASH}"#,
            r#"all_operator_live_block_convergence=true"#,
            r#"all_operator_role_status=true"#,
            r#"all_operator_role_runtime_commands=true"#,
            r#"all_operator_role_wallets_registered=true"#,
            r#"all_operator_miner_work_status=true"#,
            r#"all_operator_miner_receipt_status=true"#,
            r#"all_operator_validator_attestation_status=true"#,
            r#"all_operator_validator_remote_tensor_fetch_status=true"#,
            r#"all_operator_chain_profiles=true"#,
            r#"all_operator_role_production_policy=true"#,
            r#"all_operator_role_runtime_counters=true"#,
            r#"live_role_miner_receipt_operators=${LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT}"#,
            r#"live_role_miner_tensor_operators=${LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT}"#,
            r#"live_role_miner_receipts_submitted=${LIVE_ROLE_MINER_RECEIPTS_SUBMITTED}"#,
            r#"live_role_miner_tensors_inserted=${LIVE_ROLE_MINER_TENSORS_INSERTED}"#,
            r#"live_role_validator_attestation_operators=${LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT}"#,
            r#"live_role_validator_attestations_submitted=${LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED}"#,
            r#"live_role_owned_miner_receipts=true"#,
            r#"live_role_owned_validator_attestations=true"#,
            r#"single_local_producer=true"#,
            r#"local_proposer_runtime=false"#,
            r#"local_validator_producer=true"#,
        ],
    );

    assert_status_value_reads(
        &check_script,
        "BLOCK_STATUS",
        &[
            ("BLOCK_VALIDATION", "block_validation"),
            ("BLOCK_POW_VALID", "pow_valid"),
            (
                "BLOCK_CANONICAL_BLOCKSPACE_VALID",
                "canonical_blockspace_valid",
            ),
            ("BLOCK_SETTLED_RECEIPT_SET_ROOT", "settled_receipt_set_root"),
            ("BLOCK_CHECKS_ROOT_RECOMPUTED", "checks_root_recomputed"),
            ("BLOCK_FINALITY_VALIDATED", "finality_validated_block"),
            ("BLOCK_VOTE_COUNT", "block_vote_count"),
            ("BLOCK_VOTE_VALIDATORS", "block_vote_validators"),
            ("BLOCK_VOTE_STAKE", "block_vote_stake"),
            ("BLOCK_FINALITY_THRESHOLD_STAKE", "finality_threshold_stake"),
            ("BLOCK_SELECTED_RECEIPT_COUNT", "selected_receipt_count"),
            ("BLOCK_TENSOR_OP_RECEIPTS", "tensor_op_receipt_count"),
            (
                "BLOCK_LINEAR_TRAINING_RECEIPTS",
                "linear_training_receipt_count",
            ),
        ],
    );
    assert_status_value_reads(
        &check_script,
        "NETWORK_BLOCK_STATUS",
        &[
            ("NETWORK_BLOCK_HASH", "block_hash"),
            ("NETWORK_BLOCK_STATE_ROOT", "state_root"),
            ("NETWORK_BLOCK_FINALIZED", "finalized"),
            ("NETWORK_BLOCK_VOTE_COUNT", "block_vote_count"),
        ],
    );
    assert_shell_logical_lines(
        &check_script,
        &[
            r#"if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then"#,
            r#"if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then"#,
            r#"BLOCK_SCAN_START=$((LIVE_HEIGHT - EXPECTED_BLOCK_SCAN_DEPTH))"#,
            r#"while [ "$attempt" -lt "$EXPECTED_CHECKER_RETRY_LIMIT" ]; do"#,
            r#"while [ "$attempt" -lt "$EXPECTED_OPERATOR_CONVERGENCE_RETRY_LIMIT" ]; do"#,
            r#"sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS""#,
            r#"if BLOCK_RAW=$(read_service_block "$EXPECTED_BOOTSTRAP_SERVICE" "$BLOCK_SCAN_HEIGHT"); then"#,
            r#"if [ "$BLOCK_FINALIZED" = "true" ] && [ "$BLOCK_VALIDATION" = "useful_verification_pow" ] && [ "$BLOCK_POW_VALID" = "true" ] && [ -n "$BLOCK_NONCE" ] && [ -n "$BLOCK_DIFFICULTY_TARGET" ] && [ -n "$BLOCK_POW_HASH" ]; then"#,
            r#"USEFUL_POW_BLOCK_EVIDENCE=true"#,
            r#"CANONICAL_BLOCKSPACE_EVIDENCE=true"#,
            r#"BLOCK_CHECKS_ROOT_EVIDENCE=true"#,
            r#"VALIDATOR_PROPOSER_EVIDENCE=true"#,
            r#"FINALITY_REQUIRES_USEFUL_POW=true"#,
            r#"BLOCK_FINALITY_VOTE_EVIDENCE=true"#,
            r#"[ "$LIVE_TENSOR_OP_BLOCK_HEIGHT" -gt 0 ] || fail "service block view did not expose finalized live TensorOp receipt evidence""#,
            r#"[ "$LIVE_LINEAR_TRAINING_BLOCK_HEIGHT" -gt 0 ] || fail "service block view did not expose finalized live LinearTrainingStep receipt evidence""#,
            r#"[ "$USEFUL_POW_BLOCK_EVIDENCE" = "true" ] || fail "service block view did not expose finalized useful-verification PoW evidence""#,
            r#"[ "$CANONICAL_BLOCKSPACE_EVIDENCE" = "true" ] || fail "service block view did not expose finalized canonical blockspace evidence""#,
            r#"[ "$BLOCK_CHECKS_ROOT_EVIDENCE" = "true" ] || fail "service block view did not expose finalized block checks-root evidence""#,
            r#"[ "$VALIDATOR_PROPOSER_EVIDENCE" = "true" ] || fail "service block view did not expose validator proposer evidence""#,
            r#"[ "$FINALITY_REQUIRES_USEFUL_POW" = "true" ] || fail "service block view did not expose useful-PoW finality validation evidence""#,
            r#"[ "$LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT" -gt 0 ] || fail "no miner role reported positive live receipt submissions""#,
            r#"[ "$LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT" -gt 0 ] || fail "no miner role reported positive live tensor inserts""#,
            r#"[ "$LIVE_ROLE_MINER_RECEIPTS_SUBMITTED" -gt 0 ] || fail "miner role receipt submission total did not advance""#,
            r#"[ "$LIVE_ROLE_MINER_TENSORS_INSERTED" -gt 0 ] || fail "miner role tensor insert total did not advance""#,
            r#"[ "$LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT" -gt 0 ] || fail "no validator role reported positive live attestation submissions""#,
            r#"[ "$LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED" -gt 0 ] || fail "validator role attestation submission total did not advance""#,
            r#"if NETWORK_BLOCK_RAW=$(read_service_block "$EXPECTED_NETWORK_OBSERVER_SERVICE" "$CANDIDATE_NETWORK_HEAD_HEIGHT"); then"#,
            r#"ALL_OPERATOR_NETWORK_HEAD_HEIGHT="$NETWORK_BLOCK_HEIGHT""#,
            r#"ALL_OPERATOR_NETWORK_HEAD_HASH="$NETWORK_BLOCK_HASH""#,
            r#"ALL_OPERATOR_NETWORK_STATE_ROOT="$NETWORK_BLOCK_STATE_ROOT""#,
            r#"ALL_OPERATOR_TARGET_HEAD_HEIGHT="$ALL_OPERATOR_NETWORK_HEAD_HEIGHT""#,
            r#"ALL_OPERATOR_TARGET_HEAD_HASH="$ALL_OPERATOR_NETWORK_HEAD_HASH""#,
            r#"ALL_OPERATOR_TARGET_STATE_ROOT="$ALL_OPERATOR_NETWORK_STATE_ROOT""#,
            r#"if BLOCK_RAW=$(read_service_block "$service" "$ALL_OPERATOR_COMMON_HEAD_HEIGHT"); then"#,
            r#"if BLOCK_RAW=$(read_service_block "$service" "$ALL_OPERATOR_NETWORK_HEAD_HEIGHT"); then"#,
            r#"all_operator_common_head_height=${ALL_OPERATOR_COMMON_HEAD_HEIGHT}"#,
            r#"all_operator_common_head_hash=${ALL_OPERATOR_COMMON_HEAD_HASH}"#,
            r#"all_operator_common_head_convergence=true"#,
            r#"all_operator_target_head_height=${ALL_OPERATOR_TARGET_HEAD_HEIGHT}"#,
            r#"all_operator_target_head_hash=${ALL_OPERATOR_TARGET_HEAD_HASH}"#,
            r#"all_operator_target_state_root=${ALL_OPERATOR_TARGET_STATE_ROOT}"#,
            r#"all_operator_target_head_convergence=true"#,
            r#"all_operator_network_head_height=${ALL_OPERATOR_NETWORK_HEAD_HEIGHT}"#,
            r#"all_operator_network_head_hash=${ALL_OPERATOR_NETWORK_HEAD_HASH}"#,
            r#"all_operator_network_state_root=${ALL_OPERATOR_NETWORK_STATE_ROOT}"#,
            r#"all_operator_network_head_convergence=true"#,
            r#"live_role_miner_receipt_operators=${LIVE_ROLE_MINER_RECEIPT_OPERATOR_COUNT}"#,
            r#"live_role_miner_tensor_operators=${LIVE_ROLE_MINER_TENSOR_OPERATOR_COUNT}"#,
            r#"live_role_miner_receipts_submitted=${LIVE_ROLE_MINER_RECEIPTS_SUBMITTED}"#,
            r#"live_role_miner_tensors_inserted=${LIVE_ROLE_MINER_TENSORS_INSERTED}"#,
            r#"live_role_validator_attestation_operators=${LIVE_ROLE_VALIDATOR_ATTESTATION_OPERATOR_COUNT}"#,
            r#"live_role_validator_attestations_submitted=${LIVE_ROLE_VALIDATOR_ATTESTATIONS_SUBMITTED}"#,
            r#"live_role_owned_miner_receipts=true"#,
            r#"live_role_owned_validator_attestations=true"#,
            r#"useful_pow_block_evidence=${USEFUL_POW_BLOCK_EVIDENCE}"#,
            r#"canonical_blockspace_evidence=${CANONICAL_BLOCKSPACE_EVIDENCE}"#,
            r#"block_checks_root_evidence=${BLOCK_CHECKS_ROOT_EVIDENCE}"#,
            r#"validator_proposer_evidence=${VALIDATOR_PROPOSER_EVIDENCE}"#,
            r#"tensorwork_proposer_selection_removed=true"#,
            r#"finality_requires_useful_pow=${FINALITY_REQUIRES_USEFUL_POW}"#,
            r#"live_validator_proposer_networking=false"#,
            r#"live_validator_block_vote_networking=true"#,
            r#"all_non_producer_network_applied_blocks=true"#,
            r#"all_non_producer_network_block_payload_ingestion=true"#,
            r#"all_non_producer_network_block_payload_application=true"#,
            r#"all_non_producer_network_block_vote_ingestion=true"#,
            r#"all_non_producer_network_block_vote_application=true"#,
            r#"all_non_producer_network_event_ingestion=true"#,
            r#"all_non_producer_network_payload_announcements=true"#,
            r#"all_non_producer_network_job_payload_application=true"#,
            r#"all_non_producer_network_receipt_payload_application=true"#,
            r#"all_non_producer_network_attestation_payload_application=true"#,
            r#"all_operator_p2p_connected_peers=true"#,
            r#"all_operator_p2p_block_gossip=true"#,
            r#"all_operator_p2p_block_payload_gossip=true"#,
            r#"all_operator_p2p_block_vote_gossip=true"#,
            r#"all_operator_p2p_block_payload_head_observed=true"#,
            r#"all_operator_p2p_job_gossip=true"#,
            r#"all_operator_p2p_receipt_gossip=true"#,
            r#"all_operator_p2p_attestation_gossip=true"#,
            r#"all_operator_p2p_target_head_observed=true"#,
            r#"all_operator_p2p_latest_head_observed=true"#,
            r#"all_operator_chain_counters=true"#,
            r#"all_operator_block_log_roots_observed=true"#,
            r#"public_evidence_full_spec=false"#,
            r#"independently_checkable=false"#,
        ],
    );

    assert_shell_logical_lines(
        &restart_script,
        &[
            r#"CHECK_SCRIPT="$SCRIPT_DIR/check-local-testnet.sh""#,
            r#"TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh""#,
            r#"[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable""#,
            r#". "$TOPOLOGY_FILE""#,
            r#"EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES""#,
            r#"DEFAULT_RESTART_SERVICES="$LOCAL_CPU_DEFAULT_RESTART_SERVICES""#,
            r#"EXPECTED_SEED_HEIGHT="$LOCAL_CPU_SEED_HEIGHT""#,
            r#"EXPECTED_RESTART_CHECKER_RETRY_LIMIT="$LOCAL_CPU_RESTART_CHECKER_RETRY_LIMIT""#,
            r#"EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS="$LOCAL_CPU_DOCKER_EXEC_TIMEOUT_SECONDS""#,
            r#"EXPECTED_CHECKER_RETRY_SLEEP_SECONDS="$LOCAL_CPU_CHECKER_RETRY_SLEEP_SECONDS""#,
            r#"EXPECTED_RESTART_COMMAND_TIMEOUT_SECONDS="$LOCAL_CPU_RESTART_COMMAND_TIMEOUT_SECONDS""#,
            r#"EXPECTED_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS="$LOCAL_CPU_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS""#,
            r#"RESTART_SERVICES="${*:-$DEFAULT_RESTART_SERVICES}""#,
            r#"while IFS= read -r line || [ -n "$line" ]; do"#,
            r#"printf '%s\n' "${line#"$prefix"}""#,
            r#"key_value_from_stdin "$key" <<EOF"#,
            r#"key_value_from_stdin "$key" < "$file""#,
            r#"while [ "$attempt" -lt "$EXPECTED_RESTART_CHECKER_RETRY_LIMIT" ]; do"#,
            r#"if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then"#,
            r#"if output=$(timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then"#,
            r#"if timeout "${EXPECTED_DOCKER_EXEC_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" exec -T "$service" test -f /var/lib/tensorvm/local-cpu-ready 2>/dev/null < /dev/null; then"#,
            r#"sleep "$EXPECTED_CHECKER_RETRY_SLEEP_SECONDS""#,
            r#"[ "$latest_block_height" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "$phase status for $service latest block did not advance past seeded height $EXPECTED_SEED_HEIGHT""#,
            r#"[ "$min_height" -gt "$EXPECTED_SEED_HEIGHT" ] || fail "$phase common head height did not advance past seeded height $EXPECTED_SEED_HEIGHT""#,
            r#"[ -x "$CHECK_SCRIPT" ] || fail "check-local-testnet.sh is not executable""#,
            r#"timeout "${EXPECTED_RESTART_COMMAND_TIMEOUT_SECONDS}s" docker compose -f "$COMPOSE_FILE" restart $RESTART_SERVICES"#,
            r#"timeout "${EXPECTED_RESTART_CHECK_SCRIPT_TIMEOUT_SECONDS}s" "$CHECK_SCRIPT""#,
            r#"local_cpu_restart_continuity_ready=true"#,
            r#"restart_services=${RESTART_SERVICE_LIST}"#,
            r#"before_common_head_height=${BEFORE_COMMON_HEIGHT}"#,
            r#"before_common_head_hash=${BEFORE_COMMON_HASH}"#,
            r#"before_common_state_root=${BEFORE_COMMON_STATE_ROOT}"#,
            r#"after_common_head_height=${AFTER_COMMON_HEIGHT}"#,
            r#"after_common_head_hash=${AFTER_COMMON_HASH}"#,
            r#"after_common_state_root=${AFTER_COMMON_STATE_ROOT}"#,
            r#"restart_peer_ids_stable=true"#,
            r#"restart_heights_non_decreasing=true"#,
            r#"restart_heights_advance=true"#,
            r#"restart_block_counts_non_decreasing=true"#,
            r#"restart_block_counts_advance=true"#,
            r#"restart_state_roots_observed=true"#,
            r#"restart_state_roots_advance=true"#,
            r#"restart_block_log_roots_observed=true"#,
            r#"restart_block_log_roots_advance=true"#,
            r#"restart_previous_common_head_preserved=true"#,
            r#"restart_previous_common_state_root_preserved=true"#,
            r#"restart_blocks_continue=true"#,
            r#"restart_common_head_convergence=true"#,
        ],
    );
    assert_lacks_shell_logical_lines(
        &restart_script,
        &[
            r#"EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04""#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node status --data-dir /var/lib/tensorvm 2>/dev/null < /dev/null); then"#,
            r#"if output=$(timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" tvmd node block --data-dir /var/lib/tensorvm --height "$height" 2>/dev/null < /dev/null); then"#,
            r#"if timeout 15s docker compose -f "$COMPOSE_FILE" exec -T "$service" test -f /var/lib/tensorvm/local-cpu-ready 2>/dev/null < /dev/null; then"#,
            r#"while [ "$attempt" -lt 90 ]; do"#,
            r#"sleep 1"#,
            r#"[ "$latest_block_height" -gt 2 ] || fail "$phase status for $service latest block did not advance past seed""#,
            r#"[ "$min_height" -gt 2 ] || fail "$phase common head height did not advance past the seed""#,
            r#"timeout 60s docker compose -f "$COMPOSE_FILE" restart $RESTART_SERVICES"#,
            r#"timeout 600s "$CHECK_SCRIPT""#,
            r#"RESTART_SERVICES="${*:-miner-03 validator-02}""#,
            r#"printf '%s\n' "$document" | sed -n "s/^${key}=//p" | sed -n '1p'"#,
            r#"sed -n "s/^${key}=//p" "$file" | sed -n '1p'"#,
        ],
    );

    assert_shell_logical_lines(
        &rolling_restart_script,
        &[
            r#"RESTART_SCRIPT="$SCRIPT_DIR/check-restart-continuity.sh""#,
            r#"TOPOLOGY_FILE="$SCRIPT_DIR/local-cpu-topology.sh""#,
            r#"[ -r "$TOPOLOGY_FILE" ] || fail "local CPU topology file is not readable""#,
            r#". "$TOPOLOGY_FILE""#,
            r#"EXPECTED_SERVICES="$LOCAL_CPU_EXPECTED_SERVICES""#,
            r#"ROLLING_SERVICES="${*:-$EXPECTED_SERVICES}""#,
            r#"[ -x "$RESTART_SCRIPT" ] || fail "check-restart-continuity.sh is not executable""#,
            r#"if "$RESTART_SCRIPT" "$service"; then"#,
            r#"printf 'rolling_restart_service=%s,ready\n' "$service""#,
            r#"local_cpu_rolling_restart_continuity_ready=true"#,
            r#"rolling_restart_services=${ROLLING_SERVICE_LIST}"#,
            r#"rolling_restart_service_count=${ROLLING_COUNT}"#,
            r#"rolling_restart_peer_ids_stable=true"#,
            r#"rolling_restart_heights_advance=true"#,
            r#"rolling_restart_block_counts_advance=true"#,
            r#"rolling_restart_state_roots_advance=true"#,
            r#"rolling_restart_block_log_roots_advance=true"#,
            r#"rolling_restart_previous_common_head_preserved=true"#,
            r#"rolling_restart_previous_common_state_root_preserved=true"#,
            r#"rolling_restart_blocks_continue=true"#,
            r#"rolling_restart_common_head_convergence=true"#,
        ],
    );
    assert_lacks_shell_logical_lines(
        &rolling_restart_script,
        &[
            r#"EXPECTED_SERVICES="miner-00 miner-01 miner-02 miner-03 miner-04 miner-05 miner-06 miner-07 miner-08 miner-09 validator-00 validator-01 validator-02 validator-03 validator-04""#,
        ],
    );
}
