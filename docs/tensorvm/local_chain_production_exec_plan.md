# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 65, structured delayed-reward maturity evidence, implemented, validated,
  committed, and pushed.
- Current status: delayed proposer, receipt, challenge, and credit rewards are already state-rooted pending
  claims, but status/explorer evidence mostly exposes counts. The local readiness checker still needs
  structured maturity evidence for proving pending-until-height and release-after-height behavior without
  indirect counter workarounds.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: select the next goal-aligned implementation slice.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | First and final `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; local checker expects positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial; Iteration 64 field `div` implemented | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core, exact unary/structural/comparison/reduction/generator/quantization ops, exact field `div`, dynamic-output `split`, and rank-2 matrix-contraction `einsum` | Continue remaining exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial; Iteration 64 `div` vector implemented | Registry-derived admitted-op guard, CPU profile evidence, exact vectors for current admitted ops including multi-output quantization, exact field `div`, `split`, and `einsum`; default CUDA non-admission | Add CUDA conformance evidence and continue exact Tier-B op vectors |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 58 invariant assessment implemented | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; executable `study::economic_invariant_study` | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, live parameter calibration, and broader invariant enforcement |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 65: Structured Delayed-Reward Maturity Evidence

Feature capability: expose state-rooted pending reward claim maturity details through status/explorer
surfaces so local checkers can prove delayed rewards directly instead of inferring them from aggregate
counts.

Readiness requirements covered:
- `mvp_spec.md` §20.3, §20.4, and §25.5: receipt and proposer rewards are pending until the
  reward-settlement delay plus challenge window closes.
- `mvp_spec.md` §35 and `local_chain_production_readiness.md` acceptance gate: local evidence must prove
  live pending reward claims and later mature release into spendable balances.
- `upow.md` §12.1: miner, validator, and challenger rewards derived from verifier-dependent evidence are
  pending claims before spendability.

Canonical owner: chain state remains the owner of pending reward ledgers and maturity heights.
Adapter callers: service status, explorer overview, checkers, and public evidence tools may observe but not
mutate delayed reward claims.
Old shortcut being removed: checker/evidence paths can no longer rely only on pending-reward counts to infer
that delay semantics are active.
Regression test that proves the shortcut is gone: focused status/RPC/explorer tests assert claimable-height
details are exported for pending proposer, receipt, challenge, and credit reward claims.
Behavior with local synthetic block production disabled: unchanged; this is read-only evidence over chain
state.
Behavior for producer and non-producer roles: unchanged; all roles persist and report the same rooted
pending reward state.
Structured evidence source: service status fields and explorer overview JSON.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no p2p, storage, block, or shared-codec changes; only local API/status JSON
and key-value status output change.

Narrow validation commands:
- `cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract`
- `cargo test -p tensor_vm rpc::tests::routes::explorer_routes_return_chain_state`
- `cargo test -p tensor_vm app::operator_checks::operator_status_reports_are_parseable`
Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.
Expected observable evidence: status and explorer outputs include bounded pending claim samples with
claim IDs, beneficiaries, amounts, claimable heights, and voided status where applicable.
Out of scope: changing consensus reward maturity, adding new reward ledgers, public deployment evidence, and
full Docker checker execution while `/health` remains blocked.
Split trigger: split smaller only if the evidence surface requires a new API route or storage format change.

Implementation summary:
- Added bounded service-status claim samples for pending proposer, receipt, challenge, and credit rewards.
- Added typed explorer overview pending reward samples with ledger, claim id, subject, beneficiary, amount,
  maturity height, and voided status.
- Added focused coverage for service-status claim maturity fields, explorer overview credit-claim samples,
  and explorer JSON serialization.
- Updated implementation status, coverage matrix, tarpaulin status, and this execution plan.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused explorer JSON:
  `cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract` passed.
- Focused RPC:
  `cargo test -p tensor_vm rpc::tests::routes::node_rpc_serves_explorer_telemetry_and_faucet_routes`
  passed.
- Focused service status:
  `cargo test -p tensor_vm app::status::tests::service_status_exports_pending_reward_claim_maturity_details`
  passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 399 library tests plus integration tests.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `aa627d8` (`Expose delayed reward maturity evidence`) pushed `d62bae3..aa627d8 main
  -> main` to `github.com:distributedstatemachine/tensor_vm.git`.

## Recent Iterations

### Iteration 64: Exact Field `div` Admission

Admitted exact field-only modular-inverse `div` with field dtype/scale validation, broadcast shape
inference, zero-divisor rejection, conformance vector/profile evidence, and graph verifier profile gating.
Validation passed focused IR/conformance/verifier tests, `cargo test -p tensor_vm`, formatting/whitespace,
clippy, workspace release, and first/final Gate 0. Tarpaulin remained blocked. Feature commit `1ef3552`
and evidence commit `d62bae3` are pushed.

### Iteration 63: Exact Tier-A Matrix-Contraction `einsum` Admission

Feature capability: admit the Tier-A `einsum` subset required by `upow.md` §4.7 and §4.9 for exact
two-input matrix contractions. The admitted subset is conservative: two rank-2 inputs, one shared
contracted label, two free output labels, no repeated labels in an operand, and output labels equal to the
two free labels. Unsupported equations remain vocabulary-only and fail consensus graph validation.

Readiness requirements covered:
- `upow.md` §4.7: `einsum` is Tier A only for contraction/permutation equations.
- `upow.md` §4.9: v0 admits Tier A `matmul` and contraction `einsum` when exact semantics are specified.
- `upow.md` §3.3 and §16: every admitted op spelling needs deterministic conformance vectors and CPU
  profile evidence before receipts are accepted.

Canonical owner: `ir::frozen_op_registry`, `TensorGraph::validate`, and `TensorGraph::execute_exact` own
admitted vocabulary, equation validation, shape inference, and exact replay.
Adapter callers: receipt verifiers, role runtimes, RPC/status surfaces, and checkers only observe graph
validation/execution outcomes.
Old shortcut being removed: `einsum` existed in Tier-A registry vocabulary but was not consensus-admitted
or executable, leaving a v0 Tier-A gap.
Regression test that proves the shortcut is gone: focused IR, conformance, and graph verifier tests for a
valid contraction equation plus a rejection test for unsupported equations.
Behavior with local synthetic block production disabled: unchanged; this is deterministic IR execution and
receipt-admission metadata only.
Behavior for producer and non-producer roles: unchanged; all roles consume the same conformance-gated graph
verification path.
Structured evidence source: IR tests, conformance vector/profile tests, graph verifier tests, status docs,
and coverage docs.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no p2p, storage, block, or shared-codec changes.

Expected observable evidence: a consensus-admitted graph with a supported matrix-contraction `einsum`
validates and exact-executes to the same field result as canonical matmul, unsupported equations reject,
conformance profile gating rejects receipts when `einsum` evidence is absent, and the registry-derived
admitted-op guard remains green.
Out of scope: arbitrary-rank Einstein notation, diagonal/repeated-label equations, Tier-B/C lowering for
non-contraction equations, CUDA conformance evidence, and role-runtime arbitrary graph production.
Split trigger: split smaller only if exact equation parsing requires changing canonical JSON, graph refs,
or shared-codec formats.

Implementation summary:
- Admitted `einsum` in the frozen registry for the conservative rank-2 matrix-contraction subset.
- Added equation parsing, shape inference, and exact replay through canonical matmul/transpose paths.
- Added deterministic conformance vector/profile coverage for the admitted `einsum` spelling.
- Added graph verifier profile-gating coverage for an `einsum` receipt.
- Updated implementation status, coverage matrix, tarpaulin status, and this execution plan.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR: `cargo test -p tensor_vm ir::tests::exact_interpreter_executes_einsum_matrix_contraction`
  passed.
- Focused IR rejection:
  `cargo test -p tensor_vm ir::tests::graph_validation_rejects_unsupported_einsum_equations` passed.
- Focused conformance guard:
  `cargo test -p tensor_vm conformance::tests::conformance_vectors_cover_every_consensus_admitted_op`
  passed.
- Focused CPU profile: `cargo test -p tensor_vm conformance::tests::cpu_reference_passes_all_vectors`
  passed.
- Focused verifier: `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_einsum_receipt` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 395 library tests plus integration tests.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `0efedcc` (`Admit exact einsum contractions`) pushed `1019527..0efedcc main -> main`
  to `github.com:distributedstatemachine/tensor_vm.git`.

### Iteration 62: Dynamic-Output Exact `split` Admission

Admitted exact Tier-B `split` with `outputs = len(sizes)`, shape inference, exact row-major replay,
multi-output conformance vectors, and graph verifier profile gating. Validation passed focused
IR/conformance/verifier tests, `cargo test -p tensor_vm`, formatting/whitespace, clippy, workspace release,
and first/final Gate 0. Tarpaulin remained blocked. Feature commit `903cf9b` and evidence commit
`1019527` are pushed.

### Iteration 61: Canonical Receipt Reward Maturity Delay

Receipt rewards now use the explicit reward maturity delay rather than the tensor-retention window proxy.
`chain::settlement::receipt_reward_claimable_height` computes initial receipt claim maturity with
`ChainParams::reward_maturity_delay_blocks()`, while block application keeps inclusion maturity as an
additional floor. Validation passed focused settlement/block/reward/audit tests, `cargo test -p tensor_vm`,
clippy, workspace release, and first/final Gate 0. Tarpaulin remained blocked by the missing subcommand.
Feature commit `8c297d9` (`Delay receipt rewards by maturity rule`) is pushed to `main`.

### Iteration 60: Exact Single-Output Structural Tier-B Admission

Admitted `squeeze`, `unsqueeze`, `slice`, `tril`, and `triu` into exact Tier-B replay with shape inference,
row-major execution, conformance vectors, and graph verifier profile gating. Dynamic-output `split` was
explicitly deferred to Iteration 62.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent. Stale readiness/exec text should be
  updated as part of feature work.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable acceptance
  command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in the shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping;
  this iteration stayed single-writer because IR/conformance/verifier edits were tightly coupled.

## Validation Evidence

Latest full validation is Iteration 64 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm ir::tests::exact_interpreter_executes_field_div
cargo test -p tensor_vm ir::tests::graph_validation_rejects_unsupported_div
cargo test -p tensor_vm conformance::tests::conformance_vectors_cover_every_consensus_admitted_op
cargo test -p tensor_vm conformance::tests::cpu_reference_passes_all_vectors
cargo test -p tensor_vm verify::tests::graph_verifier_accepts_field_div_receipt
cargo test -p tensor_vm
cargo fmt --check --all
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo test -p tensor_vm local_testnet --release
```

Current coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iteration 59: exact `clamp` Tier-B admission with conformance and graph verifier profile gating.
  Commit `85a2956` (`Add exact clamp IR conformance`) is pushed.
- Iteration 58: executable economic invariant helper for
  `slashable_bond * P(detection) > reward_from_fraud`. Commit `d659e14`
  (`Add economic invariant study helper`) is pushed.
- Iteration 57: registry-derived admitted-op conformance guard and CPU profile coverage. Commit `b6e0887`
  (`Guard admitted op conformance coverage`) is pushed.
- Iteration 56: explicit admitted `sum` conformance vector. Feature commit `d66f8c9` is pushed.
- Iteration 55: useful and fallback proposer rewards share the same full reward-settlement plus
  challenge-window delay. Feature commit `7094319` is pushed.
- Iteration 54: mixed-dtype comparison and `where` conformance/verifier evidence. Feature commit
  `f5dd68b` is pushed.
- Iteration 53: proposer reward delay cleanup. Feature commit `72e16b8` and evidence commit `fae9faf` are
  pushed.
- Iteration 52: canonical byte-packed int8 quantization layout. Feature commit `1b86f7f` and evidence
  commit `0387246` are pushed.
- Iteration 51: exact per-channel int8 quantize/dequantize admission. Commit `c04af93`
  (`Admit exact int8 quantize dequantize`) is pushed.
- Iteration 50: quantization dtype and gated registry foundation. Feature commit `b89bb18` and evidence
  commit `4c4d527` are pushed.
- Iteration 49: fixed-point scale metadata and round-half-even rescale foundation. Feature commit
  `a14ba9b` is pushed.
- Iteration 48: exact unary Tier-B IR replay and conformance. Feature commit `46050d2` is pushed.
- Iteration 47: graph-backed exact jobs and receipts. Feature commit `decdf91` is pushed.
- Iteration 46: canonical current-job IR trace roots. Feature commit `9aaf2c9` is pushed.
- Iteration 45: remaining exact Tier-B shape/reduction IR replay. Feature commit `7154f6a` is pushed.
- Iteration 44: wider exact Tensor IR replay coverage. Feature commit `ce3deea` is pushed.
- Iteration 43: explicit fallback reward maturity delay. Feature commit `b0fd68e` and evidence commit
  `699193e` are pushed.
- Iteration 42: state-rooted arbitrary Tensor IR graph-body admission. Feature commit `9a32039` and
  evidence commit `2ee2340` are pushed.
- Iteration 41: generic exact-IR interpreter foundation. Commits `e86258e` and `b5fd81d` are pushed.
- Iterations 30-34: delayed proposer, receipt, challenger, and credit reward-ledger foundations. Commit
  `5664acb` and related evidence commits are archived in git history.
