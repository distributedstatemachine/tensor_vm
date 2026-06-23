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

fn assert_not_contains_any(document: &str, forbidden: &[&str], label: &str) {
    for phrase in forbidden {
        assert!(
            !document.contains(phrase),
            "{label} should not contain stale phrase {phrase:?}"
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
            "surface with distinct health URLs and distinct content URLs; missing, duplicate, reused-URL, or extra",
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
            "- exactly one signed artifact locator line for each required raw supporting-record kind, with distinct artifact URIs",
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
            "hostname to the local service. Public preflight and public evidence still have to use distinct",
            "service-health URLs and distinct service-content URLs for RPC, explorer, faucet, and telemetry, signed",
            "one signed `network_runtime_observation=...` record per counted public operator proving libp2p discovery,",
            "can be aggregated from the saved raw-record file with `evidence record summary-file` and",
            "`evidence record artifact-file`. Each signed block, finality, libp2p, randomness-beacon,",
            "artifact locator for each of those nine supporting record kinds, with distinct artifact URIs.",
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
            "public 7-day external deployment evidence",
            "CUDA miner evidence",
        ],
        "Codex workflow validation and blockers",
    );
}

#[test]
fn local_cpu_checker_requires_live_graph_execution_evidence() {
    let checker =
        include_str!("../../../../../deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh");

    for phrase in [
        "json_string_field_count primitive_type graph_execution",
        "graph_execution_receipt_count",
        "live receipt details did not include post-seed GraphExecution receipts",
        "service block view did not expose finalized live GraphExecution receipt evidence",
    ] {
        assert!(
            checker.contains(phrase),
            "local CPU checker should require GraphExecution evidence phrase {phrase:?}"
        );
    }

    assert_trimmed_lines(
        checker,
        &[
            "live_graph_execution_receipts=true",
            "live_graph_execution_receipt_count=${LIVE_GRAPH_EXECUTION_RECEIPT_COUNT}",
            "live_graph_execution_block_evidence=true",
            "live_graph_execution_block_height=${LIVE_GRAPH_EXECUTION_BLOCK_HEIGHT}",
            "live_graph_execution_block_receipts=${LIVE_GRAPH_EXECUTION_BLOCK_RECEIPTS}",
        ],
        "local CPU checker GraphExecution output",
    );
}

#[test]
fn local_status_docs_do_not_preserve_stale_health_blocker() {
    for (label, document) in [
        (
            "Codex local-chain workflow",
            include_str!("../../../../../docs/tensorvm/codex_5_5_local_chain_workflow.md"),
        ),
        (
            "local chain readiness",
            include_str!("../../../../../docs/tensorvm/local_chain_production_readiness.md"),
        ),
        (
            "implementation status",
            include_str!("../../../../../docs/tensorvm/implementation_status.md"),
        ),
        (
            "coverage matrix",
            include_str!("../../../../../docs/tensorvm/coverage_matrix.md"),
        ),
        (
            "completion audit",
            include_str!("../../../../../docs/tensorvm/completion_audit.md"),
        ),
    ] {
        assert_not_contains_any(
            document,
            &[
                "gateway /health timeout",
                "standing gateway `/health` timeout blocker",
                "current `/health` blocker",
                "fresh full Docker proof after",
                "full checker blocked",
            ],
            label,
        );
    }
}

#[test]
fn formal_status_docs_record_local_fallback_and_delayed_reward_evidence() {
    let formal_docs = [
        (
            "formal proof manifest",
            include_str!("../../../../../docs/formal/formal_proof_manifest_v0.md"),
        ),
        (
            "fallback liveness model",
            include_str!("../../../../../docs/formal/mvp_core_fallback_liveness_model.md"),
        ),
        (
            "reward finality model",
            include_str!("../../../../../docs/formal/mvp_core_reward_finality_challenge_model.md"),
        ),
        (
            "proof traceability matrix",
            include_str!("../../../../../docs/formal/mvp_core_proof_traceability_matrix.md"),
        ),
        (
            "state invariants",
            include_str!("../../../../../docs/formal/mvp_core_v2_state_invariants.md"),
        ),
        (
            "candidate block audit",
            include_str!("../../../../../docs/formal/mvp_core_candidate_v2_block_audit.md"),
        ),
        (
            "proof completion audit",
            include_str!("../../../../../docs/formal/mvp_core_proof_completion_audit.md"),
        ),
        (
            "v2 proof obligations",
            include_str!("../../../../../docs/formal/mvp_core_v2_consensus_proof_obligations.md"),
        ),
        (
            "adversary model",
            include_str!("../../../../../docs/formal/mvp_core_adversary_model.md"),
        ),
        (
            "theorem dependency graph",
            include_str!("../../../../../docs/formal/mvp_core_theorem_dependency_graph.md"),
        ),
        (
            "assumption discharge plan",
            include_str!("../../../../../docs/formal/mvp_core_assumption_discharge_plan.md"),
        ),
        (
            "bad assumptions ledger",
            include_str!("../../../../../docs/formal/bad_assumptions_ledger.md"),
        ),
    ];

    for (label, document) in formal_docs {
        assert_not_contains_any(
            document,
            &[
                "local v2-block path has no fallback path",
                "v2 PoW-skip fallback is not implemented or tested",
                "Current block type cannot support",
                "Current selector still uses settled TensorWork",
                "reward state, challenge openings, clawback, and settlement tests are missing",
                "no pending/challenged/invalidated/settled reward state or challenge resolution",
                "reward finality is paper-specified only",
                "reward-finality state and challenge resolution are not implemented",
                "RewardFinalityState` | pending claims, challenge windows, challenge resolutions, settled claims, clawbacks | Needed to prove `reward_root` and delayed verifier-dependent settlement. | Paper-specified",
            ],
            label,
        );
    }

    let fallback_model =
        include_str!("../../../../../docs/formal/mvp_core_fallback_liveness_model.md");
    assert!(fallback_model.contains("local reference now has a partial implementation"));
    assert!(fallback_model.contains("formal liveness proof remains implementation-blocked"));

    let reward_model =
        include_str!("../../../../../docs/formal/mvp_core_reward_finality_challenge_model.md");
    assert!(reward_model.contains("local reference now has partial delayed-reward"));
    assert!(reward_model.contains("full formal theorem remains implementation-blocked"));
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
