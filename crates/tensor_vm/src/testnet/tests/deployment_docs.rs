use std::collections::BTreeSet;

fn trimmed_lines(document: &str) -> BTreeSet<&str> {
    document.lines().map(str::trim).collect()
}

fn assert_trimmed_lines(document: &str, expected_lines: &[&str], label: &str) {
    let lines = trimmed_lines(document);
    for expected in expected_lines {
        assert!(
            lines.contains(expected),
            "{label} should contain exact line {expected}"
        );
    }
}

fn assert_no_retired_tvmd_commands(document: &str, label: &str) {
    for command in [
        "role",
        "service",
        "testnet",
        "evidence",
        "public-evidence",
        "public-testnet",
        "local-testnet",
        "local-cpu",
    ] {
        let direct = format!("tvmd {command}");
        let cargo_run = format!("-- {command}");
        assert!(
            !document.contains(&direct),
            "{label} should not preserve retired CLI command {direct}"
        );
        assert!(
            !document.contains(&cargo_run),
            "{label} should not preserve retired cargo-run CLI command {cargo_run}"
        );
    }
}

#[test]
fn public_deployment_templates_require_libp2p_and_https_surfaces() {
    let env = include_str!("../../../../../deploy/tensorvm/env/public-testnet.env.example");
    assert_trimmed_lines(
        env,
        &[
            "TVMD_LISTEN=127.0.0.1:8545",
            "TVMD_P2P_LISTEN=/ip4/0.0.0.0/tcp/4001",
            "TVMD_DATA_DIR=/var/lib/tensorvm",
            "TVMD_AUTH_TOKEN=replace-with-high-entropy-token",
            "TVMD_MAX_REQUESTS=0",
            r#"# tvmd node peer add --data-dir "$TVMD_DATA_DIR" --peer-id "$BOOTSTRAP_PEER_ID" --address /dns/bootstrap.tensorvm.net/tcp/4001"#,
        ],
        "deployment env template",
    );

    let systemd = include_str!("../../../../../deploy/tensorvm/systemd/tensorvm.service");
    assert_trimmed_lines(
        systemd,
        &[
            "EnvironmentFile=/etc/tensorvm/public-testnet.env",
            "ExecStartPre=/usr/local/bin/tvmd node init --data-dir ${TVMD_DATA_DIR}",
            "ExecStart=/usr/local/bin/tvmd node serve --listen ${TVMD_LISTEN} --p2p-listen ${TVMD_P2P_LISTEN} --data-dir ${TVMD_DATA_DIR} --auth-token ${TVMD_AUTH_TOKEN} --max-requests ${TVMD_MAX_REQUESTS}",
            "ReadWritePaths=/var/lib/tensorvm",
            "NoNewPrivileges=true",
            "ProtectSystem=strict",
        ],
        "systemd service template",
    );

    let nginx = include_str!("../../../../../deploy/tensorvm/nginx/tensorvm.conf");
    assert_trimmed_lines(
        nginx,
        &[
            "upstream tensorvm_service {",
            "server 127.0.0.1:8545;",
            "listen 443 ssl http2;",
            "server_name rpc.example.test explorer.example.test faucet.example.test telemetry.example.test;",
            "proxy_set_header X-Forwarded-Proto https;",
            "client_max_body_size 2m;",
            "proxy_pass http://tensorvm_service;",
            "listen 80;",
            "return 301 https://$host$request_uri;",
        ],
        "nginx template",
    );
}

#[test]
fn public_deployment_runbook_records_required_evidence_flow() {
    let runbook = include_str!("../../../../../deploy/tensorvm/RUNBOOK.md");
    assert_trimmed_lines(
        runbook,
        &[
            "tvmd public preflight deploy/tensorvm/manifests/public-testnet.preflight.example",
            "public_testnet_preflight_ready=true",
            "deployment_plan_ready=true",
            "cuda_ready_miners=true",
            "libp2p_ready_nodes=true",
            "production_libp2p_runtime=true",
            "public_service_content_planned=true",
            "public_services_planned=true",
        ],
        "runbook preflight gate",
    );

    assert_trimmed_lines(
        runbook,
        &[
            "tvmd public evidence publish ...",
            "tvmd public evidence audit ...",
            "tvmd public evidence run window ...",
            "tvmd public evidence run window-file ...",
            "tvmd public evidence node heartbeat ...",
            "tvmd public evidence node heartbeat-file ...",
            "tvmd public evidence node operator-attestation ...",
            "tvmd public evidence service health ...",
            "tvmd public evidence service health-file ...",
            "tvmd public evidence service content ...",
            "tvmd public evidence service content-bytes ...",
            "tvmd public evidence service content-file ...",
            "tvmd public evidence network observation ...",
            "tvmd public evidence network from-service-log ...",
            "tvmd public evidence record summary ...",
            "tvmd public evidence record artifact ...",
            "tvmd public evidence record artifact-roots ...",
            "tvmd public evidence record artifact-file ...",
            "tvmd public evidence record summary-roots ...",
            "tvmd public evidence record summary-file ...",
        ],
        "runbook evidence command list",
    );

    assert_trimmed_lines(
        runbook,
        &[
            "The collected records must cover the full 7-day window, not only a final snapshot. The block observation",
            "- node heartbeats for every active miner and validator",
            "- exactly one service-health record for each public RPC, explorer, faucet, and telemetry service",
            "- exactly one service-content record for each public RPC, explorer, faucet, and telemetry service",
            "- libp2p network-observation records from independent observers, one per counted public operator",
            "Any outage or operator replacement must be reflected in the final evidence bundle. Do not backfill",
            "public_evidence_full_spec=true",
            "independently_checkable=true",
            "supporting_record_artifacts=true",
            "- exactly one signed artifact locator line for each required raw supporting-record kind",
            "After validation returns `public_evidence_full_spec=true`, link the published bundle from",
            "validators. It does not contain a real external 7-day public run or a published independently checkable",
        ],
        "runbook external evidence requirements",
    );
}

#[test]
fn public_deployment_readme_records_scaffold_boundary_and_operator_flow() {
    let readme = include_str!("../../../../../deploy/tensorvm/README.md");
    assert_trimmed_lines(
        readme,
        &[
            "the TensorVM MVP spec. These files are not public-testnet evidence by themselves; they are pre-run",
            "- `env/public-testnet.env.example` - environment file consumed by the systemd unit",
            "- `RUNBOOK.md` - operator runbook for launch, evidence collection, validation, and publication",
            "- `systemd/tensorvm.service` - `tvmd node serve` unit with mandatory libp2p listen configuration",
            "- `nginx/tensorvm.conf` - TLS reverse-proxy template for RPC, explorer, faucet, and telemetry hostnames",
            "- `manifests/public-testnet.preflight.example` - manifest shape accepted by the parser, but not launch-ready",
            "- `manifests/public-testnet.evidence.example` - structurally valid post-run evidence example accepted by",
        ],
        "deployment README scaffold artifact list",
    );

    assert_trimmed_lines(
        readme,
        &[
            "GET /health",
            "GET /rpc/health",
            "GET /explorer/health",
            "GET /faucet/health",
            "GET /telemetry/health",
            "GET /chain/head",
            "GET /explorer",
            "GET /faucet/page",
            "GET /telemetry/dashboard",
        ],
        "deployment README public route list",
    );

    assert_trimmed_lines(
        readme,
        &[
            "hostname to the local service. Public evidence still has to include signed service-health records for each",
            "external URL, signed service-content records for the deployed content paths using the same HTTPS authority",
            "one signed `network_runtime_observation=...` record per counted public operator proving libp2p discovery,",
            "can be aggregated from the saved raw-record file with `evidence record summary-file` and",
            "`evidence record artifact-file`. Each signed block, finality, libp2p,",
            "the raw records behind that root; publish exactly one artifact locator for each of those six supporting",
            "cargo build -p tensor_vm --release --features cuda-kernels",
            "target/release/tvmd miner check --wallet miner.key --device cuda:0 --node /dns/bootstrap.tensorvm.net/tcp/4001",
            "sudo -u tensorvm /usr/local/bin/tvmd node peer add --data-dir /var/lib/tensorvm --peer-id \"$BOOTSTRAP_PEER_ID\" --address /dns/bootstrap.tensorvm.net/tcp/4001",
            "sudo -u tensorvm /usr/local/bin/tvmd node check --p2p-listen /ip4/0.0.0.0/tcp/4001 --data-dir /var/lib/tensorvm",
            "it is not public GPU-miner evidence. Set `cuda_ready_miner_count` in the preflight manifest to the number",
            "`miner_count`. Set `libp2p_ready_node_count` to the number of planned miner and validator nodes where",
            "The checked example reports `independently_checkable=false` and `public_evidence_full_spec=false` because",
            "validator. The full-spec gate remains closed until a real 7-day public run publishes the evidence bundle documented in",
        ],
        "deployment README operator-flow requirements",
    );
}

#[test]
fn codex_local_chain_workflow_records_required_iteration_flow() {
    let workflow = include_str!("../../../../../docs/tensorvm/codex_5_5_local_chain_workflow.md");

    assert_trimmed_lines(
        workflow,
        &[
            "cargo test -p tensor_vm local_testnet --release",
            "goal.md",
            "docs/tensorvm/upow.md",
            "docs/tensorvm/mvp_spec.md",
            "docs/tensorvm/local_chain_production_readiness.md",
            "docs/tensorvm/local_chain_production_exec_plan.md",
            "docs/tensorvm/coverage_matrix.md",
            "docs/tensorvm/implementation_status.md",
            "docs/tensorvm/tarpaulin_report.md",
        ],
        "Codex workflow context and Gate 0",
    );

    assert_trimmed_lines(
        workflow,
        &[
            "docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet",
            "docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build",
            "docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait",
            "deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh",
            "deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh",
            "docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v",
        ],
        "Codex workflow Docker gate",
    );

    assert_trimmed_lines(
        workflow,
        &[
            "cargo fmt --check --all",
            "git diff --check",
            "cargo test -p tensor_vm --quiet",
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo test --workspace --release",
            "cargo tarpaulin --workspace --offline",
            "git status --short",
            "git commit -m \"<slice name>\"",
            "git push origin main",
            "error: no such command: tarpaulin",
            "gateway /health timeout during the recorded full Docker run",
        ],
        "Codex workflow validation and blockers",
    );
}

#[test]
fn operator_docs_do_not_preserve_retired_tvmd_commands() {
    for (label, document) in [
        (
            "public testnet preflight docs",
            include_str!("../../../../../docs/tensorvm/public_testnet_preflight.md"),
        ),
        (
            "public testnet evidence docs",
            include_str!("../../../../../docs/tensorvm/public_testnet_evidence.md"),
        ),
        (
            "deployment README",
            include_str!("../../../../../deploy/tensorvm/README.md"),
        ),
        (
            "deployment runbook",
            include_str!("../../../../../deploy/tensorvm/RUNBOOK.md"),
        ),
        (
            "public deployment env template",
            include_str!("../../../../../deploy/tensorvm/env/public-testnet.env.example"),
        ),
        (
            "public deployment systemd unit",
            include_str!("../../../../../deploy/tensorvm/systemd/tensorvm.service"),
        ),
        (
            "operator boundary goal",
            include_str!("../../../../../goal.md"),
        ),
    ] {
        assert_no_retired_tvmd_commands(document, label);
    }
}
