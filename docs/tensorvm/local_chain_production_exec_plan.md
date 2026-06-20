# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 43 explicit fallback reward maturity delay.
- Current status: Iteration 43 implemented and validated on June 20, 2026; commit/push evidence pending.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action after Iteration 43: continue with arbitrary graph-backed jobs/receipts, wider admitted-registry
  executor/verifier coverage, full VRF/drand commit-reveal lifecycle, multi-validator proposer
  competition/fork-choice policy, or the Docker `/health` blocker if the environment changes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 42: `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | Iteration 37 split proposal gating from synthetic job production and added `validator_proposer_tick_runs_without_synthetic_producer_gate` | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial, graph-body admission/fetch foundation implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job and arbitrary canonical graph bodies in state/storage/runtime program serving, `TensorGraph::execute_exact` for currently implemented exact tensor ops | Add arbitrary graph-backed jobs/receipts and wider admitted-registry executor coverage |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Add broader admitted-registry vectors, generic interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; Iteration 39 anchors admitted receipt assignment/validation seeds to persisted receipt-time beacon state | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial | Delayed proposer, reduced delayed fallback proposer, receipt, challenge, and credit rewards; full reward-root binding; block-transition mature release; data-unavailability and validator-audit slashing | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 43: Explicit Fallback Reward Maturity Delay

Feature capability: empty `PowSkipFallback` proposer claims now mature by height using the full
reward-settlement plus challenge-window delay, instead of remaining blocked behind a later-useful-block
unlock latch.

Architecture shortcut answers:
- Canonical owner: `chain` owns reward maturity policy and pending-claim release.
- Adapter callers: role runtime, RPC, p2p ingest, storage, and checkers observe or apply chain commands
  only; they do not release fallback rewards directly.
- Old shortcut being removed: fallback proposer rewards no longer require `requires_useful_successor` to be
  cleared by unrelated useful blockspace before they can release.
- Regression tests: `fallback_proposer_reward_uses_explicit_maturity_delay`,
  `reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury`,
  `release_matured_proposer_rewards_sweeps_voided_claims_without_credit`, and
  `block_transition_releases_matured_rewards_without_manual_command`.
- Synthetic production disabled: unchanged; fallback reward maturity is a chain height rule.
- Producer/non-producer roles: both validate/apply the same pending reward claim and reward root.
- Structured evidence source: pending proposer reward `claimable_at_height`, reward roots, release events,
  and spendable reward balances.
- Finality source: unchanged; reward maturity remains separate from block finality.
- Wire-size and codec boundary: no network/storage payload shape change; the old snapshot flag remains
  decodable but new fallback claims do not depend on it.

Validation target:
- Focused: `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`,
  `cargo test -p tensor_vm local_testnet --release`, and the expected tarpaulin blocked check.

Validation evidence:
- Required first gate for this iteration: `cargo test -p tensor_vm local_testnet --release` passed before
  edits on June 20, 2026.
- `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture` passed: 7 reward tests.
- `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture` passed: 4 command tests.
- `cargo test -p tensor_vm --lib chain::tests::attestations -- --nocapture` passed: 8 attestation/audit
  tests after separating retention-window math from reward maturity policy.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed: 358 library tests, 8 `tvmd_cli` tests, 31 `tvmd_runtime` tests, and
  the `local_cpu_compose` integration test.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed: experiments, tensor_vm, tensor_vm_explorer, integration, and
  doc-test targets.
- Final `cargo test -p tensor_vm local_testnet --release` passed: 5 library local-testnet tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` failed as expected because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.

Commit/push evidence: pending.

### Iteration 42: State-Rooted Arbitrary Tensor IR Graph-Body Admission

Feature capability: arbitrary user-submitted canonical Tensor IR graph bodies can enter canonical chain
state independently of the current fixed job constructors, survive node-store persistence, and be hydrated
into the runtime program server for the existing bounded `RequestProgram`/`ProgramResponse` fetch path.

Architecture shortcut answers:
- Canonical owner: `chain` owns graph-body admission into the state-rooted program registry; `ir` owns
  parsing, canonical encoding, graph ID, and consensus validation.
- Adapter callers: job sources, RPC/CLI/P2P adapters, and tests can submit through
  `ChainCommand::RegisterProgramBody`; runtime startup hydrates the P2P program server from chain state.
- Old shortcut being removed: graph bodies no longer require a fixed `TensorOp` or `LinearTrainingStep`
  job submission before they can be state-rooted and fetchable.
- Regression tests: `chain_engine_registers_valid_canonical_program_body_without_job`,
  `chain_engine_rejects_invalid_or_conflicting_program_bodies`,
  `startup_program_hydration_registers_state_rooted_program_bodies`, and the chain-state roundtrip test.
- Synthetic production disabled: unchanged; direct graph-body admission does not depend on synthetic jobs or
  local block production.
- Producer/non-producer roles: both load the same persisted graph registry; runtime program serving is
  hydrated from canonical chain state at startup.
- Structured evidence source: `ChainEvent::ProgramBodyRegistered`, state-rooted `program_bodies`, graph ID,
  canonical graph bytes, node-store snapshot, and P2P program response bytes.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no new wire format; existing bounded `RequestProgram`/`ProgramResponse`
  request-response codec remains the fetch boundary.

Implementation target:
- Parse Tensor IR graph JSON bytes back into `TensorGraph`.
- Add `ChainCommand::RegisterProgramBody` and `ChainEvent::ProgramBodyRegistered`.
- Require graph ID match, consensus validation, and byte-for-byte canonical graph encoding before state
  admission.
- Keep matching duplicate submissions idempotent and reject malformed, noncanonical, or mismatched bodies.
- Hydrate runtime P2P program serving from persisted chain `program_bodies` at startup.
- Update coverage/status/tarpaulin docs and this execution plan.

Validation target:
- Focused: `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture`,
  `cargo test -p tensor_vm --lib ir::tests -- --nocapture`,
  `cargo test -p tensor_vm --lib app::runtime_services::tests -- --nocapture`, and
  `cargo test -p tensor_vm --lib storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`, and final
  `cargo test -p tensor_vm local_testnet --release`.
- Coverage attempt: `cargo tarpaulin --workspace --offline`.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused tests passed:
  - `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture`
  - `cargo test -p tensor_vm --lib ir::tests -- --nocapture`
  - `cargo test -p tensor_vm --lib app::runtime_services::tests -- --nocapture`
  - `cargo test -p tensor_vm --lib storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`
  (358 library tests plus integration tests), `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `9a32039` (`Admit arbitrary Tensor IR graph bodies`).
- Push result: `b5fd81d..9a32039  main -> main` on `origin/main`.

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, generic graph P2P gossip, full admitted-registry exact
replay/verifier coverage, and CUDA generic graph execution.

## Recent Iterations

### Iteration 41: Generic Exact-IR Interpreter Foundation

Feature capability: validated Tensor IR graphs can execute through a generic deterministic interpreter over
the exact tensor ops already implemented by the reference runtime. Execution returns named output tensors,
per-op output commitment roots, and a Merkle `trace_root`.

Architecture shortcut answers:
- Canonical owner: `ir` owns graph structural validation, op admission, ref resolution, deterministic
  graph execution, and trace-output commitments.
- Adapter callers: current job structs remain unchanged for this slice; tests call the interpreter
  directly.
- Old shortcut being removed: fixed TensorOp/LinearTrainingStep constructors are no longer the only
  executable path for validated graph bodies over implemented exact ops.
- Regression test: `exact_interpreter_executes_hand_built_graph_and_commits_trace`.
- Synthetic production disabled: unchanged; this is a deterministic library capability without runtime
  scheduling behavior.
- Producer/non-producer roles: unchanged; no network behavior changes in this slice.
- Structured evidence source: `IrExecution` named outputs, `IrOpTrace` output roots, graph ID, and
  `trace_root`.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no new p2p or storage payloads in this slice.

Implementation target:
- Add `TensorGraph::execute_exact` and execution input/result types.
- Execute `matmul`, `add`, `sub`, `mul`, `scalar_mul`, `transpose`, explicit-dim `sum`/`reduce_sum`,
  `identity`, and `neg` through existing `Tensor` primitives.
- Validate bound tensor shapes/dtypes and field-scalar params.
- Fail closed for Tier-C/deferred ops, `const_blob`, and admitted registry ops not yet backed by exact
  replay implementation.
- Update `coverage_matrix.md`, `implementation_status.md`, `tarpaulin_report.md`, and this execution plan.

Validation target:
- Focused: `cargo test -p tensor_vm --lib ir::tests -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`, and final
  `cargo test -p tensor_vm local_testnet --release`.
- Coverage attempt: `cargo tarpaulin --workspace --offline`.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused test passed: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` with 10 IR tests.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`
  (355 library tests plus integration tests), `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `6b7260c` (`Add exact Tensor IR interpreter foundation`).
- Push result: `72816da..6b7260c  main -> main` on `origin/main`.

Out of scope: arbitrary graph-backed jobs/receipts, role/runtime execution of arbitrary graph jobs,
const-blob fetching, full admitted-registry exact replay, Tier-C consensus admission, and CUDA generic
graph execution.

## Decision Log

- `docs/tensorvm/upow.md` is canonical when it conflicts with older readiness text.
- Keep the missing workflow document visible as a standing blocker; do not treat the readiness doc as a
  substitute.
- Preserve one shared chain engine. Deployment profiles can vary, but transition logic must not fork.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through
  the shared P2P/event path.
- TensorWork affects rewards, blockspace, telemetry, and concentration analysis only; it never selects
  block proposers.
- `tvmd` is an adapter/process launcher, not a hidden consensus orchestrator.
- Current v0 admits exact Tier-A/B ops only. Tier-C vocabulary may exist in the registry but must be gated
  out of consensus until canonical references and verifiers exist.
- Current-job and arbitrary canonical graph bodies are stored as canonical JSON bytes after graph
  validation; arbitrary graph-backed jobs/receipts and role execution remain a separate future slice.
- Split configured validator block proposal from local synthetic job production: `local_block_proposer`
  controls configured validator proposal duty, while `local_synthetic_producer` controls profile-gated
  deterministic local job publication.

## Validation Evidence

Latest current-iteration evidence:
- Starting branch state: `## main...origin/main`.
- Iteration 42 required Gate 0 first and final Gate 0:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 42 focused validation:
  - `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture`: 4 command tests passed.
  - `cargo test -p tensor_vm --lib ir::tests -- --nocapture`: 10 IR tests passed.
  - `cargo test -p tensor_vm --lib app::runtime_services::tests -- --nocapture`: 1 runtime-services test passed.
  - `cargo test -p tensor_vm --lib storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`: 1 storage test passed.
- Iteration 42 broad validation:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - `cargo test -p tensor_vm`: passed with 358 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 31 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 358 `tensor_vm`, 1 local CPU
    Compose, 8 `tvmd_cli`, 31 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests,
    and doc-test targets.
- Iteration 42 feature commit: `9a32039` (`Admit arbitrary Tensor IR graph bodies`).
- Iteration 42 push result: `b5fd81d..9a32039  main -> main` on `origin/main`.

Latest unresolved full-gate blocker:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /health
```

Latest coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iteration 40, `6cea749 Delay fallback proposer rewards`: empty `PowSkipFallback` blocks created reduced
  delayed proposer claims that required a later useful block before release; Iteration 43 replaced that
  release latch with explicit full reward-maturity delay while preserving snapshot decoding of the
  `requires_useful_successor` flag.
- Iteration 35, `f53700c Bind reward root to pending claims`: block `reward_root` now commits spendable
  rewards plus pending proposer, receipt, challenge, and credit ledgers; old spendable-only roots are
  rejected. Evidence update was followed by Iteration 36.
- Iteration 39, `a4c1378 Anchor receipt validation randomness`: persisted receipt-time finalized-beacon
  randomness anchors for validator assignment and validation seeds; evidence commit `41edc0e`.
- Iteration 38, `4984e6f Record runtime reward delay evidence`: runtime role proof now matures delayed
  useful-proposer rewards through ordinary block production instead of adapter-side release.
- Iteration 34, delayed generic reward credits: converted `CreditReward`/faucet-style credits into
  state-rooted pending credit claims before spendability.
- Iteration 33, current-job conformance/IR status refresh: recorded current-job conformance and generic IR
  gaps after the conformance and graph-body slices.
- Iteration 32, `26e3e25 Move validator proposals into role tick`: moved useful proposal evidence into
  validator role ticks with settled/artifact-ready/attested counters, while still gated by synthetic
  producer policy before Iteration 37.
- Iteration 31, `9216461 Propagate block check challenges`: added bounded block-check challenge p2p
  payloads, pending retry, chain-command application, and delayed challenge reward evidence.
- Iteration 30, `5664acb Delay validator proposer rewards`: useful proposals create delayed proposer
  reward claims; later work changed fallback proposals from unrewarded to reduced delayed claims, and
  Iteration 43 made fallback release depend on explicit reward maturity height.
- Iteration 29, `4e8b0c6 Propagate validator audit reports`: validator roles gossip/apply signed audit
  reports through bounded p2p/node payloads.
- Iteration 28, `99d819c Add validator audit reward slashing`: added audit assignments/results/slashes and
  delayed audited validator reward handling.
- Iteration 27, `cae45b5 Handle unavailable receipt rewards and slashing`: unavailable-data attestations
  void receipt rewards and slash miner bond once.
- Iteration 26, `25dbfe4 Delay challenger reward finality`: challenger bounties become pending challenge
  claims before spendability.
- Iteration 25, `0363bb6 Store Tensor IR graph bodies` with evidence `f734a69`: current-job graph bodies
  are state-rooted, persisted, and served through `RequestProgram`.
- Iteration 24, `f4d4491 Add Fp conformance vector gate`: current executable exact-op conformance vectors
  and CPU verifier gates.
- Iteration 23, `388c4d6 Delay receipt reward finality`: receipt settlement creates delayed miner and
  validator reward claims.
- Iterations 1-22: extracted reusable node runtime state, moved network payload application/event drivers
  into reusable runtime helpers, bound role runtimes to chain identities, added miner receipt submission,
  validator attestations, validator block votes, network-visible block payload admission, useful-verification
  PoW block validity, remote validator tensor fetch, validator-owned block proposal ticks, content-addressed
  Tensor IR foundation, finalized-beacon consensus randomness binding, block apply openings, retarget/fallback
  mode, delayed proposer rewards, and checker evidence for role-owned local work.
