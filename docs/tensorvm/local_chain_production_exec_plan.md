# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 71, chain-owned validator audit appeals.
- Current status: delayed proposer, receipt, challenge, and credit rewards are state-rooted pending claims
  and the checker gates on future-maturity claim evidence. Status and explorer consume the chain-owned
  pending reward-claim view, and observed block-check challenge payload application is tied to future
  challenger reward claims. Mandatory validator audits now include deterministic chain-owned auditor
  selection, report authorization, and signed state-rooted appeal records for existing audit slashes.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue with appeal adjudication/refund mechanics, live parameter calibration,
  deterministic live bad-block generation, or Docker `/health`.

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
| Economics and slashing invariant | Partial; Iteration 70 auditor selection implemented locally | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; assigned auditor policy; chain-owned pending claim view; executable `study::economic_invariant_study` | Add audit appeal records and live parameter calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 71: Chain-Owned Validator Audit Appeals

Feature capability: validators slashed by a mandatory audit can submit a signed, bounded appeal that is
state-rooted, persisted, and tied to the existing audit slash record.

Readiness requirements covered:
- `upow.md` §12.2 and `mvp_spec.md` §26: hard slashing needs battle-tested appeal/challenge flow evidence.
- `coverage_matrix.md` criterion 5: appeals are the remaining local economics gap after assigned auditors.

Canonical owner: chain audit/slash state and command validation.
Adapter callers: none in this slice; future p2p/runtime appeal gossip must call the same command.
Old shortcut being removed: audit slashes currently have no canonical dispute marker.
Regression test that proves the shortcut is gone: focused chain test accepts exactly one signed validator
appeal for an existing slash and rejects unknown, wrong-signer, duplicate, and expired appeals.
Behavior with local synthetic block production disabled: appeal submission is independent of producers.
Behavior for producer and non-producer roles: no role-specific path yet; future network ingestion must
apply the same command.
Structured evidence source: `ChainState::validator_audit_appeals()` and the state root/storage snapshot.
Finality source: appeal records do not alter block finality in this slice.
Wire-size and codec boundary: no wire format in this slice; reason text is bounded before state insertion.

Files/modules likely touched: chain state/validation/commands/roots/tests, storage snapshot tests,
status/coverage/readiness docs, exec plan.
Narrow validation commands: focused audit/root/storage tests.
Broad validation commands before commit: fmt, diff check, full crate, clippy, workspace release, final Gate 0,
tarpaulin attempt.
Out of scope: adjudication/refund mechanics, p2p appeal gossip, data-unavailability appeals, live parameter
calibration, Docker `/health`.
Split trigger: if state persistence or root integration forces network codec changes, split network gossip
into a later iteration.

Implementation summary:
- Added signed `ValidatorAuditAppeal` submissions and persisted `ValidatorAuditAppealRecord` state.
- Accepted appeals only for existing audit slashes, from the slashed validator, before a bounded deadline.
- Committed appeal records in the state root and persisted them through chain-state snapshots.
- Updated audit, root, storage, and economics/readiness docs while leaving adjudication/refund mechanics
  out of scope.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused tests passed for audit appeal admission/rejection, audit state roots, and chain-state persistence.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 401 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `09b2a49` (`Record validator audit appeals`) is pushed to `main`.

## Recent Iterations

### Iteration 70: Chain-Owned Validator Auditor Selection

Mandatory audit assignments now persist a deterministic auditor distinct from the audited validator,
reject reports from non-assigned auditors, and limit validator role observation to local assignments.
Validation passed focused audit/root/storage/node/role tests, full crate, clippy, workspace release, and
first/final Gate 0; tarpaulin remained blocked. Feature commit `79b4d12` and evidence commit `94767bd`
are pushed.

### Iteration 69: Full Challenge-Reward Maturity Delay

Feature capability: successful block-check challenger rewards must use the chain's full
`reward_maturity_delay_blocks()` rule instead of the shorter proposer-throttle/challenge-window height.

Readiness requirements covered:
- `mvp_spec.md` §20.7: challengers receive pending claims that release only after maturity.
- `upow.md` §12.1: verifier-dependent rewards are pending consensus claims before spendability.

Canonical owner: chain challenge transition and pending challenge reward ledger.
Old shortcut being removed: challenger bounty maturity cannot be derived from proposer throttle height.
Validation plan: focused chain/node challenge tests, formatting/whitespace, final Gate 0, tarpaulin attempt.

Implementation summary:
- Changed block-check challenge resolution so challenger bounty claims mature at
  `challenged_at_height + ChainParams::reward_maturity_delay_blocks()`.
- Kept proposer throttling on the challenge-window penalty height, making penalty duration distinct from
  reward spendability.
- Updated chain and network payload tests plus readiness/status/coverage docs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused challenge tests passed through `cargo test -p tensor_vm --quiet` after locking the new maturity
  assertion.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 400 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `7595b0e` (`Delay challenge rewards by maturity rule`) is pushed to `main`.

### Iteration 68: Local Checker Challenge-Reward Evidence

Feature capability: when the local run observes block-check challenge payload ingestion/application, require
future-maturity, non-voided pending challenge reward claim evidence from the chain-owned reward view.

Readiness requirements covered:
- `mvp_spec.md` §20.7 and §25.5: successful checks-root challengers receive delayed pending claims.
- `local_chain_production_readiness.md` checker gap: pending challenger rewards must be observable instead
  of inferred from challenge counters.

Canonical owner: chain state and `ChainState::pending_reward_claims()`.
Adapter caller: local CPU checker reads explorer overview `pending_rewards`.
Old shortcut being removed: challenge-payload counters no longer stand alone as reward evidence when present.
Out of scope: deterministic live bad-block generation, appeal policy, and Docker `/health` resolution.
Validation plan: shell syntax, compose artifact test, focused status/RPC tests, formatting/whitespace,
`cargo test -p tensor_vm --quiet`, clippy, workspace release, final Gate 0, tarpaulin attempt.

Implementation summary:
- Added checker parsing for pending challenge reward count and future-maturity challenge claims.
- Added a conditional failure when pending challenge rewards or applied live block-check challenges exist
  without future-maturity pending challenge reward evidence.
- Added output fields for challenge reward claims and block-check challenge counters.
- Updated local compose artifact coverage and status/coverage/readiness docs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Shell syntax: `bash -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` passed.
- Focused compose artifact:
  `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
  passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 400 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `53eaa9e` (`Gate challenge reward evidence in local checker`) is pushed to `main`.

### Iteration 67: Chain-Owned Reward Claim View

Unified chain-state pending reward claim view for proposer, receipt, challenge, and credit ledgers.
Status/explorer/checkers now consume formal chain data instead of rebuilding ledger-specific adapter
projections. Validation passed focused chain/status/RPC tests, full crate, clippy, workspace release, and
first/final Gate 0; tarpaulin remained blocked. Feature commit `cf886d4` is pushed.

### Iteration 66: Local Checker Delayed-Reward Claim Gate

The local CPU checker now requires future-maturity, non-voided receipt/proposer pending reward claims
instead of aggregate pending-reward counts. Validation passed shell syntax, compose artifact, full crate,
clippy, workspace release, and first/final Gate 0; tarpaulin remained blocked. Feature commit `2232724`
and evidence commits `9547dde`/`7c9c209` are pushed.

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

Latest full validation is Iteration 71 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm validator_audit --quiet
cargo test -p tensor_vm state_root_commits_to_validator_audit_records --quiet
cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --quiet
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
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
