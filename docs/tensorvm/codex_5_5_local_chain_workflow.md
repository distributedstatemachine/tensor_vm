# Codex 5.5 Local Chain Workflow

This document is the operational workflow for Codex 5.5 implementation iterations on the TensorVM local
chain. It is not a protocol specification; `upow.md`, `mvp_spec.md`, and the readiness document remain
authoritative. This file exists so future iterations have one checked artifact for the goal loop, required
gates, evidence updates, and commit/push discipline.

## Required First Gate

Every new or resumed MVP implementation iteration must run this command first from the repository root:

```bash
cargo test -p tensor_vm local_testnet --release
```

No later local, CUDA, Docker, public-evidence, or deployment-gated acceptance result counts until this
Gate 0 command passes against the current worktree.

## Context Refresh

Before editing, refresh these files:

```text
goal.md
docs/tensorvm/upow.md
docs/tensorvm/mvp_spec.md
docs/tensorvm/local_chain_production_readiness.md
docs/tensorvm/local_chain_production_exec_plan.md
docs/tensorvm/coverage_matrix.md
docs/tensorvm/implementation_status.md
docs/tensorvm/tarpaulin_report.md
```

Use `docs/tensorvm/local_chain_production_exec_plan.md` as the durable source of truth for active
feature scope, validation evidence, blockers, commit hashes, and pushed branches. Keep it under 300 lines
by compacting completed iteration detail into recent/archive summaries.

## Slice Selection

Choose one coherent missing local-reference slice that moves the real `upow.md` v0 objective forward.
Before editing, write an iteration checkpoint in `docs/tensorvm/local_chain_production_exec_plan.md` with
the feature capability, readiness requirements, likely files, tests, validation commands, expected
observable evidence, out-of-scope items, and split trigger. Do not count public-run, CUDA, Docker, or
tarpaulin results as passing when they are blocked by missing infrastructure or tools.

Subagents are optional only when the active tooling and user request authorize delegation. If delegation is
not authorized, record that the parent agent is the single writer.

## Local Docker Gate

The local production-readiness gate is:

```bash
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

If the Docker gate is blocked, record the exact command, exit status, and key error output in
`docs/tensorvm/local_chain_production_exec_plan.md`.

## Validation Sequence

Run the focused test or experiment for the selected slice first. Before commit, run the relevant broad
checks from the workspace root:

```bash
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo tarpaulin --workspace --offline
cargo test -p tensor_vm local_testnet --release
```

If `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed, record
the exact `error: no such command: tarpaulin` blocker in the execution plan and
`docs/tensorvm/tarpaulin_report.md`. Do not claim regenerated coverage.

## Evidence Updates

Update the docs that changed evidence for the slice:

```text
docs/tensorvm/local_chain_production_exec_plan.md
docs/tensorvm/coverage_matrix.md
docs/tensorvm/implementation_status.md
docs/tensorvm/tarpaulin_report.md
```

Only move a coverage-matrix or status item from incomplete to complete when the current evidence proves
that exact requirement. Keep deployment-gated and public-testnet evidence explicitly marked incomplete
until real external evidence exists.

## Commit And Push

After targeted validation and required docs are updated:

```bash
git status --short
git diff --check
git commit -m "<slice name>"
git push origin main
```

Record the final commit hash, remote, branch, and push result in
`docs/tensorvm/local_chain_production_exec_plan.md`. If push fails, record the exact blocker and do not
start another iteration until the push requirement is resolved or explicitly waived by the user.

## Standing Blockers

Current environment blockers must stay visible until resolved:

```text
cargo tarpaulin --workspace --offline
error: no such command: tarpaulin

public 7-day external deployment evidence
CUDA miner evidence
```

These blockers do not justify marking the full goal complete. They are evidence that the full objective
remains active.
