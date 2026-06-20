# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 66, local checker delayed-reward claim gate, implemented, validated, and
  committed; push/evidence commit pending.
- Current status: delayed proposer, receipt, challenge, and credit rewards are state-rooted pending claims
  and are visible through status/explorer claim samples. The local readiness checker still relies on
  aggregate pending-reward counters for live acceptance and must gate on future-maturity claim evidence
  directly.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit this evidence update and push Iteration 66.

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
| Economics and slashing invariant | Partial; Iteration 58 invariant assessment implemented | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; structured pending-claim status/explorer evidence; executable `study::economic_invariant_study` | Gate local checker on structured delayed-claim maturity; add auditor-selection policy, appeal paths, unified formal reward-claim objects, live parameter calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 66: Local Checker Delayed-Reward Claim Gate

Feature capability: make the local CPU readiness checker prove delayed rewards directly from pending reward
claim samples instead of accepting aggregate pending-reward counter increases as a proxy.

Readiness requirements covered:
- `mvp_spec.md` §20.3, §20.4, and §25.5: receipt and proposer rewards are pending until the
  reward-settlement delay plus challenge window closes.
- `mvp_spec.md` §35 and `local_chain_production_readiness.md` acceptance gate: local evidence must prove
  live pending reward claims before spendability.
- `upow.md` §12.1: miner, validator, and challenger rewards derived from verifier-dependent evidence are
  pending claims before spendability.

Canonical owner: chain state remains the owner of pending reward ledgers and maturity heights.
Adapter callers: the local checker observes explorer overview/status fields and does not mutate delayed
reward claims.
Old shortcut being removed: the local checker can no longer rely only on pending-reward counts to infer that
delay semantics are active.
Regression test that proves the shortcut is gone: compose-bundle artifact tests assert the checker requires
future-maturity pending reward claim samples for live receipt and proposer rewards.
Behavior with local synthetic block production disabled: unchanged; this is read-only evidence over chain
state.
Behavior for producer and non-producer roles: unchanged; all roles persist and report the same rooted
pending reward state.
Structured evidence source: explorer overview `pending_rewards` objects and service status claim samples.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no p2p, storage, block, shared-codec, or consensus changes; only deployment
checker acceptance and docs change.

Narrow validation commands:
- `bash -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.
Expected observable evidence: the checker requires at least one live pending receipt claim and one live
pending proposer claim whose `claimable_at_height` is greater than the observed live height, and records
those counts in its readiness output.
Out of scope: changing consensus reward maturity, adding new reward ledgers, public deployment evidence, and
full Docker checker execution while `/health` remains blocked.
Split trigger: split smaller only if the checker needs a new API route or storage format change.

Implementation summary:
- Added `json_future_pending_reward_count` to the local CPU checker and required future-maturity,
  non-voided receipt/proposer pending reward claims in the live readiness loop.
- Added readiness output fields for delayed receipt and proposer claim counts.
- Updated the compose artifact test and status/coverage/tarpaulin docs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Shell syntax: `bash -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` passed.
- Focused compose artifact:
  `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
  passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 399 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `2232724` (`Gate delayed reward claims in local checker`) created on `main`; push
  pending.

## Recent Iterations

### Iteration 64: Exact Field `div` Admission

Admitted exact field-only modular-inverse `div` with field dtype/scale validation, broadcast shape
inference, zero-divisor rejection, conformance vector/profile evidence, and graph verifier profile gating.
Validation passed focused IR/conformance/verifier tests, `cargo test -p tensor_vm`, formatting/whitespace,
clippy, workspace release, and first/final Gate 0. Tarpaulin remained blocked. Feature commit `1ef3552`
and evidence commit `d62bae3` are pushed.

### Iteration 63: Exact Tier-A Matrix-Contraction `einsum` Admission

Admitted the conservative exact rank-2 matrix-contraction `einsum` subset with registry admission,
equation validation, shape inference, exact replay, conformance vectors, and graph verifier gating.
Validation passed focused IR/conformance/verifier tests, `cargo test -p tensor_vm`,
formatting/whitespace, clippy, workspace release, and first/final Gate 0. Tarpaulin remained blocked.
Feature commit `0efedcc` and evidence commit `1019527` are pushed.

### Iteration 65: Structured Delayed-Reward Maturity Evidence

Exposed state-rooted pending reward claim maturity details through service status and explorer overview so
local/public evidence can prove delayed rewards directly. Added bounded status samples for proposer,
receipt, challenge, and credit ledgers, typed explorer pending reward samples, focused status/RPC/explorer
tests, and docs. Validation passed focused tests, `cargo test -p tensor_vm`, formatting/whitespace, clippy,
workspace release, and first/final Gate 0. Tarpaulin remained blocked. Feature commit `aa627d8` and
evidence commit `3d9cd9b` are pushed.

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

Latest full validation is Iteration 65 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract
cargo test -p tensor_vm rpc::tests::routes::node_rpc_serves_explorer_telemetry_and_faucet_routes
cargo test -p tensor_vm app::status::tests::service_status_exports_pending_reward_claim_maturity_details
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
