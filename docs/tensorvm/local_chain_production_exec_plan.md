# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 87 complete - delayed block-check proposer reward protection.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, and delayed challenge rewards are
  implemented locally. Iteration 85 added audit-window-aware reward escrow and tensor retention.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue fork-choice/withholding policy, measured economics, or rerun the full Docker
  scenario after the `/health` blocker clears.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; checker expects live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Add multi-validator proposer competition/fork-choice policy; rerun Docker |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges | Remaining: full transcript disputes, fork-choice/withholding policy, fresh Docker proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface | Continue exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor | Pin full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, mature release, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit and fraud-path calibration | Add measured detection probabilities, remaining fraud paths, and broader invalid-output slashing |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 87: Delayed Block-Check Proposer Reward Protection

Feature capability: delay proposer rewards through the block-check fraud window so the block-check path is
protected by canonical reward escrow instead of a calibration workaround that treats the pending reward as
both fraud proceeds and slashable bond.

Readiness requirements covered: `upow.md` §12 economics/slashing invariant and the local readiness gap for
block-check/proposer reward timing.

Canonical owner: `ChainParams` and shared chain reward/block transitions compute and enforce delayed
proposer reward claim heights; `ChainState` derives block-check economic exposure from live pending claims.
Adapter callers: status and explorer/RPC render chain-owned evidence; runtime, p2p, and checkers remain
consumers of the same state.
Old shortcut being removed: block-check economics reports the pending proposer reward as immediate
fraud profit, causing `bond * P(detection) > reward_from_fraud` to fail by construction even while the
reward is still escrowed.
Regression test that proves the shortcut is gone: focused reward/status/RPC tests show proposer reward
claims mature after the block-check hold and the block-check path reports zero exposed reward while held.
Behavior with local synthetic block production disabled: calibration is a read-only chain-state view over
pending claims and params, independent of local synthetic job/block production.
Behavior for producer and non-producer roles: producers and non-producers replay the same chain state and
derive identical claim heights and calibration; no role-local branch owns economics.
Structured evidence source: `ChainParams::proposer_reward_hold_blocks`, pending proposer reward claims,
`ChainState::fraud_path_economic_calibration`, service status fields, and explorer overview JSON.
Finality source: finalized/replayed chain state; reward release remains tied to canonical height and pending
claim state, not adapter clocks.
Wire-size and codec boundary: one persisted chain-param field is added to the storage codec; no p2p payload
or block codec change.

Files/modules likely touched: `chain/state.rs`, `chain/blocks.rs`, `chain.rs`, storage codec tests,
focused reward/status/RPC tests, status docs, and this plan.
Parallel subagents to run: none; user prefers no subagents unless explicitly requested.
Parallelizable implementation workstreams: read-only discovery and validation only.
Tests/checkers/docs to add or update: focused chain reward/economics test, storage codec test, status test,
explorer overview test, coverage/status/upow/readiness text.
Narrow validation commands: `cargo test -p tensor_vm reward --quiet`, `cargo test -p tensor_vm storage --quiet`,
`cargo test -p tensor_vm status --quiet`, `cargo test -p tensor_vm explorer_overview_exports --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt if feasible.
Expected observable evidence: proposer pending claims include a block-check hold and block-check economic
calibration no longer fails solely because held rewards were counted as immediate fraud proceeds.
Out of scope: changing slash amounts, adding full fraud-proof transcript disputes, fork-choice policy,
CUDA evidence, or Docker rerun.
Split trigger: if delayed rewards require changing block, p2p, or receipt wire payloads, split that from
this parameterized chain-state transition.

Implementation summary:
- Added `ChainParams::proposer_reward_hold_blocks` and proposer-specific maturity for pending proposer
  claims created by block production and epoch reward settlement.
- Persisted the new chain parameter in the chain-state codec.
- Updated block-check fraud-path economics so delayed proposer claims are slashable escrow and only
  claimable rewards count as immediate fraud proceeds.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm reward --quiet`, `cargo test -p tensor_vm storage --quiet`,
  `cargo test -p tensor_vm status --quiet`, `cargo test -p tensor_vm explorer_overview_exports --quiet`,
  `cargo test -p tensor_vm params --quiet`, and `cargo test -p tensor_vm challenge --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 415 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

## Recent Iterations

### Iteration 86: Fraud-Path Economic Calibration

Implementation summary:
- Added `ChainState::fraud_path_economic_calibration` for validator-audit, miner data-unavailability, and
  block-check/proposer clawback paths.
- Service status and explorer overview now render aggregate and per-path required-bond/pass-fail evidence.
- Updated `upow.md`, readiness, coverage, implementation status, and tarpaulin docs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm economic --quiet`, `cargo test -p tensor_vm status --quiet`, and
  `cargo test -p tensor_vm explorer_overview_exports --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 415 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `1116beb` (`Expose fraud path economic calibration`) pushed to `origin/main`.
- Evidence commit: `abf78d1` (`Record fraud path calibration evidence`) pushed to `origin/main`.

### Iteration 85: Audit-Window Reward Escrow

`ChainParams::reward_maturity_delay_blocks` now includes the validator-audit hold when audit sampling is
enabled, and tensor retention mirrors the same audit-window bound. No-audit profiles keep the old
zero-retention behavior. Validation passed focused parameter/audit/reward tests, full crate, clippy,
workspace release, and first/final Gate 0. Commit `5df4870` (`Delay rewards through audit window`) is
pushed to `origin/main`.

### Iteration 84: Validator Audit Stake-Slash Reversal

Reversed validator-audit appeals now refund the recorded stake slash from treasury back to validator stake
exactly once while reward reinstatement still uses delayed pending claims. Validation passed focused
audit/storage/reward tests, full crate, clippy, workspace release, and first/final Gate 0. Commits
`1feeb1d` and `ea230b3` are pushed to `origin/main`.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest full validation is Iteration 86 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm economic --quiet
cargo test -p tensor_vm status --quiet
cargo test -p tensor_vm explorer_overview_exports --quiet
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo test -p tensor_vm local_testnet --release
cargo tarpaulin --workspace --offline
```

Current iteration Gate 0 passed:

```text
cargo test -p tensor_vm local_testnet --release
```

Current coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iterations 75-83: diagnostic block-check challenge path, fallback timeout, inclusion-gated and
  chain-owned reward release, and receipt-bound validation seed landed in commits `8787912`, `40f14d5`,
  `06be27e`, `f5a0aa2`, `1647a47`, `8ce051f`, `e08f7c9`, and related evidence commits.
- Iterations 73-74: live validator-audit economic calibration and appeal reward-delay resolution landed in
  commits `493191c`, `8dbb654`, `c8a6f9e`, `32fb557`, and `7026c94`.
- Iterations 59-64: exact `clamp`, field `div`, split/einsum, registry/conformance guard, and graph
  verifier coverage landed across commits including `85a2956`, `d659e14`, and `b6e0887`.
- Iterations 41-58: Tensor IR, graph-backed jobs/receipts, exact replay, quantization, Tier-B coverage,
  delayed proposer reward cleanup, and economic helper foundations landed in git history.
- Iterations 30-34: delayed proposer, receipt, challenger, and credit reward-ledger foundations landed in
  commit `5664acb` and related evidence commits.
