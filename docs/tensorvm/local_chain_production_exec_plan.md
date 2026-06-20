# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 45, remaining exact Tier-B shape/reduction IR replay.
- Current status: Iteration 45 implemented and validated on June 20, 2026; commit/push evidence pending.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit and push Iteration 45 evidence, then move to arbitrary graph-backed jobs/receipts,
  full VRF/drand commit-reveal lifecycle, multi-validator proposer competition/fork-choice policy, or the
  Docker `/health` blocker if the environment changes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 45 first command and final gate: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial; Iteration 45 replay complete | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, state/storage/runtime program serving, exact interpreter for current core plus Iteration 44 shaping/generator/comparison coverage and Iteration 45 `mean`/`cast`/`concat`/`stack` replay | Add arbitrary graph-backed jobs/receipts and remaining admitted-registry executor coverage |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops plus Iteration 44 field-only shaping/generator vectors and Iteration 45 `mean`/`concat`/`stack` vectors; CPU pass profile; default CUDA non-admission | Add mixed-dtype vector schema, remaining admitted-registry vectors, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial | Delayed proposer, reduced delayed fallback proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; data-unavailability and validator-audit slashing | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 45: Remaining Exact Tier-B Shape/Reduction IR Replay

Feature capability: validated Tensor IR graphs can exact-execute `mean`, `cast`, `concat`, and `stack`;
concat/stack validation uses real axis shape rules instead of the previous same-shape placeholder.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph` validation/exact execution and conformance suite metadata.
- Adapter callers: current runtime/job verification paths consume accepted graph IDs/profiles; no adapter
  gains consensus mutation.
- Old shortcut being removed: admitted Tier-B graph ops no longer fail closed or validate with placeholder
  same-shape typing solely because deterministic exact replay was missing.
- Regression tests: new IR execution tests for mean/cast/concat/stack and conformance vector tests where
  the current same-dtype vector schema fits.
- Synthetic production disabled: unchanged; this is pure IR execution capability.
- Producer/non-producer roles: unchanged until graph-backed job admission is wired later.
- Structured evidence source: `IrExecution` named outputs, `IrOpTrace` roots, conformance profile pass set,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Validation target:
- Focused: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` and
  `cargo test -p tensor_vm --lib conformance::tests -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`,
  final `cargo test -p tensor_vm local_testnet --release`, and the expected tarpaulin blocked check.

Current validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR tests: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` passed 13 tests.
- Focused conformance tests: `cargo test -p tensor_vm --lib conformance::tests -- --nocapture` passed 3
  tests.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 361 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final local-testnet gate: `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet
  library tests plus the filtered service-gateway integration test.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: pending.
- Push evidence: pending.

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, signed/fixed-point unary op semantics, exact
quantization, mixed-dtype conformance-vector schema, and CUDA generic graph execution.

## Recent Iterations

### Iteration 44: Wider Exact Tier-B IR Interpreter Coverage

Feature capability: validated Tensor IR graphs can exact-execute a broader admitted Tier-B surface:
broadcast-aware `add`/`sub`/`mul`, `reshape`, `broadcast`, comparisons `gt`/`lt`/`ge`/`le`/`eq`, `where`,
`full`, and `arange`. Field-only conformance vectors now cover `reshape`, `broadcast`, `full`, and
`arange`.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph` validation/exact execution and conformance suite metadata.
- Adapter callers: current runtime/job verification paths consume accepted graph IDs/profiles; no adapter
  gains consensus mutation.
- Old shortcut being removed: admitted Tier-B graph ops no longer fail closed solely because deterministic
  exact replay was missing.
- Regression tests: `exact_interpreter_executes_shaping_comparison_generators_and_where`,
  `graph_validation_rejects_inconsistent_exact_tier_b_shapes`, and conformance vector tests.
- Synthetic production disabled: unchanged; this is pure IR execution capability.
- Producer/non-producer roles: unchanged until graph-backed job admission is wired later.
- Structured evidence source: `IrExecution` named outputs, `IrOpTrace` roots, conformance profile pass set,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Implementation target:
- Add deterministic exact replay helpers inside `ir.rs` rather than broadening the public tensor API.
- Tighten validation for reshape element counts and arange declared output length.
- Add graph-level execution tests for shaping, comparison, generator, and selection ops.
- Add field-only conformance vectors where the current single-dtype vector schema fits.
- Update coverage/status/readiness/tarpaulin docs and keep this plan compact.

Validation target:
- Focused: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` and
  `cargo test -p tensor_vm --lib conformance::tests -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`,
  final `cargo test -p tensor_vm local_testnet --release`, and the expected tarpaulin blocked check.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR tests passed: 12 `ir::tests`.
- Focused conformance tests passed: 3 `conformance::tests`.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed: 360 library tests, 1 local CPU Compose integration test, 8
  `tvmd_cli` tests, 31 `tvmd_runtime` tests, and doc-tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed: 14 `experiments` tests, 360 `tensor_vm` library tests,
  integration tests, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and doc-tests.
- Final `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` failed as expected because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `ce3deea` (`Widen exact Tensor IR replay coverage`).
- Feature push: `git push` to `github.com:distributedstatemachine/tensor_vm.git` updated `main -> main`
  from `699193e` to `ce3deea`.

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, mixed-dtype conformance-vector schema, exact replay for
`mean`, `cast`, exact unary fixed-point ops, concat/stack, exact quantization, and CUDA generic graph
execution.

### Iteration 43: Explicit Fallback Reward Maturity Delay

Empty `PowSkipFallback` proposer claims now mature by height using the full reward-settlement plus
challenge-window delay, instead of remaining blocked behind a later-useful-block unlock latch. Evidence:
first and final `cargo test -p tensor_vm local_testnet --release` passed; focused reward, command, and
attestation/audit tests passed; `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
`cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed;
`cargo tarpaulin --workspace --offline` remained blocked by missing `cargo-tarpaulin`. Feature commit
`b0fd68e` pushed `2ee2340..b0fd68e main -> main`; evidence commit `699193e` pushed
`b0fd68e..699193e main -> main`.

### Iteration 42: State-Rooted Arbitrary Tensor IR Graph-Body Admission

Arbitrary canonical Tensor IR graph bodies can enter canonical chain state independently of fixed job
constructors, survive node-store persistence, and hydrate the runtime program server for the existing
bounded `RequestProgram`/`ProgramResponse` path. Evidence: first/final Gate 0 passed; focused chain, IR,
runtime hydration, and chain-state tests passed; broad format, unit, clippy, workspace release gates
passed; tarpaulin remained blocked by missing `cargo-tarpaulin`. Feature commit `9a32039` pushed
`b5fd81d..9a32039 main -> main`; evidence commit `2ee2340` pushed `9a32039..2ee2340 main -> main`.

## Decision Log

- `upow.md` is canonical where docs conflict; update stale readiness text when touched.
- Validators own useful-verification block proposal; TensorWork affects rewards, blockspace, telemetry, and
  concentration analysis only.
- `tvmd` remains an adapter/process launcher; counted roles must communicate through libp2p/RPC before
  affecting another node.
- Reward spendability is delayed through state-rooted pending claims. Fallback proposer rewards now use an
  explicit reward-maturity height and no longer require a useful-block unlock.
- Full Docker local CPU readiness remains blocked by gateway `/health`; do not claim full local production
  readiness until that gate passes.
- `cargo tarpaulin --workspace --offline` cannot regenerate coverage in this environment until
  `cargo-tarpaulin` is installed.

## Validation Evidence

- Latest current-iteration Gate 0: `cargo test -p tensor_vm local_testnet --release` passed first and
  final on June 20, 2026 with 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Latest focused tests: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` passed 12 tests;
  `cargo test -p tensor_vm --lib conformance::tests -- --nocapture` passed 3 tests.
- Latest broad gates: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`
  passed.
- Current tarpaulin blocker:

```text
error: no such command: `tarpaulin`
```

- Current Docker blocker:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
```

## Archive

- Iteration 41, `e86258e` / `b5fd81d`: added the generic exact-IR interpreter foundation with named
  outputs, per-op output roots, and `trace_root` over the exact ops already available at the time.
- Iterations 39-40, `3001501`, `7652f13`, `6cea749`, `b3637fc`: anchored receipt validation randomness
  to receipt-time finalized beacon state and added reduced delayed fallback proposer claims, later refined
  by Iteration 43.
- Iterations 35-38, `f53700c`, `587b111`, `584e5d4`, `4984e6f`: bound reward roots to all pending reward
  ledgers and moved mature reward release through normal block transitions.
- Iterations 30-34, `5664acb` and related evidence commits: delayed proposer, receipt, challenger, and
  generic credit rewards as state-rooted pending claims.
- Iterations 26-29: added challenge rewards, unavailable-receipt slashing/reward voiding, validator audit
  slashing, and network-visible audit/challenge ingestion.
- Earlier iterations established role-owned local miner/validator work, network event ingestion,
  TensorBlock/UVPoW foundations, Tensor IR graph IDs, frozen registry metadata, conformance gates for
  current jobs, public-evidence validators/templates, finalized-beacon randomness binding, block apply
  openings, retarget/fallback mode, and checker evidence for role-owned local work.
