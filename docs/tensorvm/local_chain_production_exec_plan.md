# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 48, exact unary Tier-B IR replay and conformance.
- Current status: Iteration 48 implementation and validation pushed to `main` on June 20, 2026
  (`1ac2197..e12ab32`).
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: move to full VRF/drand commit-reveal lifecycle, multi-validator proposer
  competition/fork-choice policy, exact quantization replay, fixed-point
  rescale/round-half-even semantics beyond current field/integer identity rounding, or the Docker `/health`
  blocker if the environment changes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 48 first command and final gate: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial; Iteration 48 exact unary replay complete locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, state/storage/runtime program serving, exact interpreter for current core plus Iteration 44 shaping/generator/comparison coverage, Iteration 45 `mean`/`cast`/`concat`/`stack` replay, Iteration 46 current TensorOp/LinearTrainingStep receipt trace roots from canonical graph execution, Iteration 47 graph-backed jobs/receipts, and Iteration 48 exact unary Tier-B replay for `identity`, `neg`, `abs`, `sign`, `round`, and `relu` | Add exact quantization, fixed-point rescale/round-half-even beyond current field/integer identity rounding, mixed-dtype schema, and remaining admitted-registry executor coverage |
| Per-op `F_p` conformance vectors | Partial; Iteration 48 unary profile complete locally | Deterministic vectors for current executable ops plus Iteration 44 field-only shaping/generator vectors, Iteration 45 `mean`/`concat`/`stack` vectors, and Iteration 48 unary vectors for `identity`, `neg`, `abs`, `sign`, `round`, and `relu`; CPU pass profile; default CUDA non-admission | Add mixed-dtype vector schema, exact quantization/fixed-point vectors, remaining admitted-registry vectors, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial | Delayed proposer, reduced delayed fallback proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; data-unavailability and validator-audit slashing | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 48: Exact Unary Tier-B IR Replay And Conformance

Feature capability: consensus-admitted exact unary Tier-B ops `identity`, `neg`, `abs`, `sign`, `round`,
and `relu` execute
through `TensorGraph::execute_exact`, receive deterministic `F_p` conformance vectors, and can be used by
graph-backed receipts without failing the conformance gate.

Readiness requirements covered:
- `upow.md` §3.3 and §16: the CPU reference conformance suite must cover admitted deterministic `F_p`
  semantics before receipts are accepted.
- `upow.md` §4.7-§4.9: exact Tier-B unary ops carried as consensus-admitted registry entries must have
  executable deterministic semantics rather than validate-but-fail interpreter behavior.
- `mvp_spec.md` §8 and §35: deterministic TensorVM operation semantics and cross-runtime conformance
  evidence for block-eligible receipts.

Canonical owner: `ir::TensorGraph::execute_exact` owns exact unary op execution; `conformance` owns the
runtime pass profile and suite hash used by graph receipt verification.
Adapter callers: `verify_graph_execution`, role graph verification, and graph receipt tests consume the
conformance profile; no adapter gets new consensus logic.
Old shortcut being removed: registry admission can no longer list these unary Tier-B ops as admitted while
the exact interpreter rejects them or the conformance profile lacks pass evidence.
Regression test that proves the shortcut is gone: IR tests execute a graph containing unary ops and graph
verification tests accept a graph receipt using one of the newly covered ops only when the CPU conformance
profile includes it.
Behavior with local synthetic block production disabled: unchanged; this is pure IR/conformance capability.
Behavior for producer and non-producer roles: unchanged; validators that see graph receipts use the same
conformance profile and exact replay path.
Structured evidence source: `IrExecution` outputs/traces, conformance vector suite hash/pass set, graph
verification report, and focused tests.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: unchanged; no payload format changes.

Parallel subagents:
- Meitner mapped exact unary Tier-B coverage to `upow.md`/MVP requirements and risks.
- Linnaeus inspected `ir`, `conformance`, and verifier touch points.
- Dewey identified focused tests and stale docs.

Parallelizable implementation workstreams:
- Parent/integrator owns edits because the slice is small and centered on shared IR/conformance modules.
- Subagents remain read-only support; no parallel writers.

Tests/checkers/docs to add or update:
- Focused IR execution test for unary ops.
- Conformance vector coverage/pass tests for `identity`, `neg`, `abs`, `sign`, `round`, and `relu`.
- Graph verification test proving a graph receipt using a newly covered op is accepted.
- Update status/exec/tarpaulin docs after validation.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR module: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 14 tests.
- Focused unary IR: `cargo test -p tensor_vm ir::tests::exact_interpreter_executes_unary_tier_b_ops -- --nocapture`
  passed.
- Focused conformance: `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3 tests.
- Focused graph verifier: `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_unary_tier_b_graph_receipt -- --nocapture`
  passed.
- After clippy's `manual_div_ceil` finding was fixed, `cargo test -p tensor_vm ir::tests::exact_interpreter_executes_unary_tier_b_ops -- --nocapture`
  and `cargo test -p tensor_vm conformance::tests::cpu_reference_passes_all_vectors -- --nocapture` passed.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed 367 library tests plus integration tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- Final `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.

Expected observable evidence: a consensus-valid graph containing exact unary Tier-B ops can exact-execute,
its receipt can be verified through the graph verifier using the CPU conformance profile, and missing
conformance still rejects it.

Out of scope: exact quantization, fixed-point rescale/round-half-even semantics beyond identity rounding for
current field/integer tensors, Tier-C/transcendental admission, runtime role changes, codec changes,
VRF/drand, fork-choice, and Docker `/health`.

Commit evidence:
- Feature commit: `46050d2` (`Add exact unary IR conformance`).
- Evidence commit: `e12ab32` (`Record exact unary validation evidence`), pushed to `main` on June 20,
  2026 (`1ac2197..e12ab32`).

## Recent Iterations

### Iteration 47: Graph-Backed Exact Job And Receipt Admission

Feature capability: registered canonical Tensor IR graph bodies can be referenced by first-class
`GraphExecution` jobs and receipts, encoded over the shared payload codec, rooted/persisted in chain state,
exact-replay verified through `TensorGraph::execute_exact`, and settled/rewarded through the existing receipt
machinery when validators attest.

Readiness requirements covered:
- `upow.md` §4.4-§4.6: jobs reference content-addressed graphs by `graph_id` and require registered,
  consensus-valid canonical bodies.
- `upow.md` §5: graph receipts commit to named input roots, output roots, `trace_root`, miner identity,
  deterministic receipt id, and signature.
- `upow.md` §11.1: graph receipts enter the same canonical receipt/attestation/settlement/blockspace path
  as fixed current jobs.

Canonical owner: `ir::TensorGraph::execute_exact` remains the owner of exact graph execution and trace-root
construction; `chain::receipts` owns job/receipt admission checks.
Adapter callers: shared payload codec, storage, p2p wire wrappers, node payload application, and role tests
consume the new `JobState`/`ReceiptState` variants without adding consensus decisions.
Old shortcut being removed: arbitrary registered graph bodies can no longer only exist as inert program
bodies; they can be bound to executable jobs and receipt records.
Regression test that proves the shortcut is gone: graph job/receipt chain tests, codec/storage/root tests,
role exact replay tests, and settlement tests for graph receipts.
Behavior with local synthetic block production disabled: unchanged; graph jobs can be admitted through
shared commands/payloads, but the local synthetic job source keeps emitting only current canonical jobs.
Behavior for producer and non-producer roles: producer policy is unchanged; validators verify graph receipts
through the same role verifier once graph tensors/artifacts are available.
Structured evidence source: `GraphJob`, `GraphReceipt`, state roots, shared payload roundtrips, receipt
settlement events, and focused tests.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: add bounded map/string encodings to the existing shared job/receipt payload
codec; p2p wire continues delegating to that shared codec.

Parallel subagents:
- Copernicus mapped the readiness slice and scope boundaries.
- Dalton mapped the data model, admission checks, codec/storage/p2p/role paths.
- Lorentz mapped existing coverage and missing tests.

Parallelizable implementation workstreams:
- Parent/integrator owns code changes because `JobState`/`ReceiptState` variants touch shared files.
- Read-only explorers remain background support only; no parallel writers.

Tests/checkers/docs to add or update:
- Added focused `jobs`, `codec`, `chain`, `storage`, `roles`, and settlement tests.
- Updated status/tarpaulin evidence after validation.

Narrow validation commands:
- `cargo test -p tensor_vm graph -- --nocapture` passed 10 focused graph tests.
- `cargo test -p tensor_vm codec::tests` passed 6 codec/storage-codec tests.
- `cargo test -p tensor_vm storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`
  passed.

Broad validation commands before commit:
- First required gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- `cargo fmt --all` applied formatting; `git diff --check` passed.
- `cargo test -p tensor_vm` passed 365 library tests plus integration tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- Final `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.
- Feature commit: `decdf91` (`Add graph execution jobs and receipts`).
- Evidence commit: `1ac2197` (`Record graph execution validation evidence`), pushed to `main` on June 20,
  2026 (`c3706cc..1ac2197`).

Observable evidence: a registered non-fixed graph can be submitted as a graph job, produce an
exact trace-root receipt, survive codec/storage/root paths, receive a valid graph attestation, and settle
through the same delayed reward path.

Out of scope: Tier-C consensus admission, redundancy committee redesign, fraud games, `const_blob` fetching,
CUDA generic graph execution, mixed-dtype conformance vectors, exact quantization/signed fixed-point unary
completion, VRF/drand lifecycle, multi-validator fork-choice, and Docker `/health`.

Split trigger: split smaller if the app role runtime or p2p payload admission changes require unrelated
status/checker rewrites beyond compiling the new variants and proving shared-codec roundtrips.

### Iteration 46: Canonical Current-Job IR Trace Roots

Feature capability: current canonical TensorOp and LinearTrainingStep receipts derive and verify
`trace_root` from their canonical `TensorGraph::execute_exact` op traces instead of parallel handcrafted
trace-hash shortcuts.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph::execute_exact` remains the owner of exact graph execution and trace
  root construction.
- Adapter callers: `jobs` and `verify` consume canonical graph execution results for current fixed job
  records only; role/runtime/p2p adapters stay unchanged.
- Old shortcut removed: current receipt constructors and verifiers no longer build separate
  receipt-specific trace roots that can diverge from the canonical IR DAG trace.
- Regression tests: current job receipt tests assert trace roots equal exact graph execution roots; verifier
  mismatch tests continue rejecting altered trace commitments.
- Synthetic production disabled: unchanged; current canonical job execution semantics only.
- Producer/non-producer roles: unchanged; arbitrary graph-backed role execution remains later work.
- Structured evidence source: receipt `trace_root`, `IrExecution.trace_root`, focused jobs/verify tests,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused jobs tests: `cargo test -p tensor_vm --lib jobs::tests -- --nocapture` passed 2 tests.
- Focused verifier tests: `cargo test -p tensor_vm --lib verify::tests -- --nocapture` passed 13 tests.
- Focused challenge/settlement/watcher/reward regressions passed:
  `challenge::tests::fraud_challenge_proves_invalid_tensorop_and_resolves_slash`,
  `chain::tests::settlement::conflicting_linear_training_roots_do_not_settle`, `watcher::tests`, and
  `chain::tests::rewards::reward_root_commits_to_all_pending_reward_ledgers`.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 361 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final local-testnet gate: `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet
  library tests plus the filtered service-gateway integration test.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `9aaf2c9` (`Use canonical IR trace roots for receipts`).
- Push evidence: pushed to `main` on June 20, 2026 (`37d1446..9aaf2c9`).

Out of scope: arbitrary graph-backed job/receipt record types, chain/runtime receipt production for
arbitrary registered graphs, p2p/codec changes, generic graph verifier economics, const-blob fetching,
signed/fixed-point unary semantics, exact quantization, and Docker `/health`.

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
- Feature commit: `7154f6a` (`Complete exact Tier-B IR replay`).
- Push evidence: pushed to `main` on June 20, 2026 (`da008c3..7154f6a`).

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, signed/fixed-point unary op semantics, exact
quantization, mixed-dtype conformance-vector schema, and CUDA generic graph execution.

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
- Latest focused tests: `cargo test -p tensor_vm graph -- --nocapture` passed 10 focused graph tests;
  `cargo test -p tensor_vm codec::tests` passed 6 codec/storage-codec tests;
  `cargo test -p tensor_vm storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`
  passed.
- Latest broad gates: `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`
  passed after `cargo fmt --all`.
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
