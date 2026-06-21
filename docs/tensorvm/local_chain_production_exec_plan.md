# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 99 complete - TensorWork anti-monopoly reward curve.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage, automatic
  unfinalized side-branch deep reorg, graph-backed synthetic jobs, and delayed challenge rewards are
  implemented locally. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner reward
  settlement now applies a chain-owned square-root TensorWork curve per miner before delayed pending receipt
  claims are created.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue verifier-transcript disputes, external randomness, deployed-run economics evidence,
  or rerun Docker after the `/health` blocker clears.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; checker expects live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards; current-head useful competitor replacement, side-branch storage, and automatic unfinalized deep reorg | Rerun Docker and continue live proposer evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg | Remaining: full transcript disputes and fresh Docker proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface, role-owned local graph execution, and content-addressed `const_blob` artifact replay | Continue exact Tier-B verifier coverage, dispute-time blob availability, and CUDA graph evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; status/explorer expose seed-domain and block-hash-ban evidence | Add external drand/VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, mature release, late invalid-output reward voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, TensorWork reward concentration curve, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 99: TensorWork Anti-Monopoly Reward Curve

Feature capability: damp miner reward concentration by allocating the miner reward pool with a
chain-owned square-root TensorWork curve.

Canonical owner: `chain::settlement`.
Adapter callers: block transitions, status/explorer pending claim views, telemetry, and reward tests.
Old shortcut being removed: raw proportional miner receipt rewards paid `pool * receipt_twu / total_twu`
without a chain-owned diminishing-return curve.
Regression test that proves the shortcut is gone:
`chain::tests::miner_rewards_use_diminishing_tensorwork_curve_per_miner`.
Behavior with local synthetic block production disabled: any settled receipt path uses the same settlement
allocation, regardless of job source.
Behavior for producer and non-producer roles: all roles recompute pending receipt claims from canonical
chain state and settlement rules.
Structured evidence source: pending receipt reward claims, raw miner settled TensorWork, and the focused
settlement regression.
Finality source: unchanged; rewards remain delayed pending claims until normal block inclusion and maturity.
Wire-size and codec boundary: no new wire payload; no chain-state codec field changes.

Implementation summary:
- Settlement aggregates newly settled raw TWU per miner, scores each miner with integer square root, and
  allocates the miner pool by adjusted scores with deterministic remainder handling.
- Each miner's allocation is split back across that miner's receipts by raw receipt TWU, preserving
  receipt-level reward claims while damping miner-level concentration.
- `upow.md`, `mvp_spec.md`, coverage, implementation status, and Tarpaulin notes now describe the local
  curve and keep deployed concentration measurements open.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm miner_rewards_use_diminishing_tensorwork_curve_per_miner --quiet`
  passed.
- Focused settlement suite: `cargo test -p tensor_vm settlement --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 434 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `0020c61` (`Dampen TensorWork reward concentration`) is pushed to `origin/main`.
- Evidence commit: `334610d` (`Record TensorWork concentration evidence`) is pushed to `origin/main`.

## Recent Iterations

### Iteration 97: Role-Owned Graph Execution Production

Local synthetic production now cycles TensorOp, LinearTrainingStep, and deterministic exact Tier-B
GraphExecution jobs. Local CPU helper rounds register graph bodies, execute graph receipts, attest them,
and settle them, while long-running miner/validator role helpers reconstruct graph jobs from registered
program bodies plus node-local tensor artifacts. Validation passed focused scheduler/localnet/graph/role
tests, full crate, clippy, workspace release, and first/final Gate 0. Commit `5af3fcf` is pushed to
`origin/main`.

### Iteration 96: Automatic Side-Branch Deep Reorg

Strictly longer valid unfinalized side branches now promote into canonical state through chain-owned
fork-choice while finalized canonical blocks remain protected. Validation passed focused side-branch,
reorg, block, node-payload, full crate, clippy, workspace release, and first/final Gate 0. Commit
`4d585f8` is pushed to `origin/main`.

### Iteration 95: Invalid-Output Miner Stake Slashing

Assigned invalid-output evidence now slashes the receipt miner once, credits treasury, records a
state-rooted slash, marks the receipt challenged, voids delayed receipt rewards, persists through storage,
and appears in fraud-path calibration/status/explorer output. Validation passed focused
attestation/storage/economics/status tests, full crate, clippy, workspace release, and first/final Gate 0.
Commit `695c66e` is pushed to `origin/main`.

### Iteration 94: Side-Branch Fork Tree Storage

Valid known-parent non-canonical blocks are retained in chain-owned side-branch storage with parent/child
state snapshots, persisted through chain-state snapshots, and applied through normal node payload admission
without mutating canonical head state. Validation passed focused fork-tree/storage/payload tests, full
crate, clippy, workspace release, and first/final Gate 0. Commit `c33ef38` is pushed to `origin/main`.

### Iteration 93: Invalid-Output Delayed Reward Voiding

Late assigned `Invalid` attestations now mark the receipt challenged, remove it from settled receipts, and
void matching pending miner and validator receipt rewards before spendability. Validation passed focused
settlement tests, full crate, clippy, workspace release, and first/final Gate 0. Commit `bf0d5fa` is pushed
to `origin/main`.

### Iteration 91: Explicit Fraud-Window Reward Delay Evidence

`ChainParams::fraud_reward_hold_blocks` now makes receipt reward maturity explicitly cover the challenge
window plus active audit window before spendability. Reward economics regression coverage uses delayed
pending miner rewards rather than a mature-claim workaround. Validation passed focused reward/params,
full crate, clippy, workspace release, and first/final Gate 0. Commit `31bcc49` is pushed to `origin/main`.

### Iteration 92: Live Detection Probability Evidence

`ChainState::detection_probability_evidence`, service status, and explorer overview now expose live
per-mechanism probability evidence for implemented verifier/fraud paths. Validation passed focused
detection/status/explorer tests, full crate, clippy, workspace release, and first/final Gate 0. Commit
`5697593` is pushed to `origin/main`.

### Iteration 90: Chain-Owned Randomness Binding Evidence

Status/explorer now expose finalized-beacon randomness source, seed domains, anchor consistency counts,
commit-reveal ordering, and the current-block-hash randomness ban. Validation passed focused randomness,
status, explorer, full crate, clippy, workspace release, and first/final Gate 0. Commit `c6baaf5` is
pushed to `origin/main`.

### Iteration 89: Delayed Receipt Reward Fraud Exposure

Validator-audit and data-unavailability calibration now count `reward_from_fraud` only from non-voided
receipt claims whose `claimable_at_height` is at or below canonical chain height. Immature pending miner
and validator receipt rewards remain at-risk escrow but no longer inflate immediate fraud proceeds.
Validation passed focused reward/status/explorer tests, full crate, clippy, workspace release, and
first/final Gate 0. Commit `ece08ff` is pushed to `origin/main`.

### Iteration 88: Competing-Head Fork-Choice And Withholding Policy

Current-head useful UVPoW competitors can replace only an unfinalized same-parent useful head when the new
PoW evidence is strictly preferred; finalized heads, historical heights, different-parent conflicts, and
fallback heads remain stable. Validation passed focused block/payload tests, full crate, clippy, workspace
release, and first/final Gate 0. Commits `3a75b33`, `d2e758d`, and `1484592` are pushed to `origin/main`.

### Iteration 87: Delayed Block-Check Proposer Reward Protection

`ChainParams::proposer_reward_hold_blocks` delays proposer rewards through the block-check fraud window.
Block-check economics now treats delayed proposer rewards as slashable escrow and only mature claims as
fraud proceeds. Validation passed focused reward/storage/status/explorer/params/challenge tests, full
crate, clippy, workspace release, and first/final Gate 0. Commits `1923692`, `b638369`, and `d4d11b8` are
pushed to `origin/main`.

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

Latest full validation is Iteration 98 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm const_blob --quiet
cargo test -p tensor_vm graph_jobs --quiet
cargo test -p tensor_vm role --quiet
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo test -p tensor_vm local_testnet --release
cargo tarpaulin --workspace --offline
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
