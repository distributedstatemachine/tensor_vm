# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 82 complete; commit/push pending.
- Current status: live diagnostic observed bad-block challenge emission is implemented in the validator
  proposer runtime and checker contract. Delayed proposer, receipt, challenge, and credit rewards are
  state-rooted pending claims
  and the checker gates on future-maturity claim evidence. Status and explorer consume the chain-owned
  pending reward-claim view, and observed block-check challenge payload application is tied to future
  challenger reward claims. Mandatory validator audits now include deterministic chain-owned auditor
  selection, report authorization, signed state-rooted appeal records, and pending validator-reward holds
  through the audit appeal deadline after a slash. Chain state, service status, and explorer overview now
  expose live validator-audit economic calibration from current params and pending validator reward
  exposure. Deterministic observed bad-block challenges now resolve through a noncanonical side cache and
  delayed pending challenger reward claims. Validator proposers publish the diagnostic over bounded p2p
  payloads after useful proposals, and the checker requires applied challenge counters plus future-maturity
  pending challenge reward claims. Empty fallback blocks now validate only for the deterministic
  stake-weighted proposer selected from parent state and beacon; useful UVPoW remains open to validator
  competition, and scheduled local job production no longer forces empty fallback blocks through the role
  wallet. Historical block apply evidence now stores exact parent `ChainState` snapshots keyed by block hash
  and persists them through the chain-state codec, keeping `BlockApplyOutcome` roots stable after later
  receipts/blocks and restart.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit and push Iteration 82 evidence, then continue with the next non-Docker consensus gap.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | First and final `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; local checker expects positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, block-check challenges, and observed malformed block-check challenge payloads | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, stake-weighted fallback proposer eligibility, fallback timeout enforcement, replay-stable parent snapshots/apply outcomes, delayed rewards, network-visible block-check challenges, deterministic diagnostic bad-block challenge generation, live diagnostic emission, and observed-malformed-block p2p/cache support | Remaining: full transcript disputes, fork-choice/withholding policy, fresh Docker proof |
| Tensor IR graph language | Partial; Iteration 64 field `div` implemented | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core, exact unary/structural/comparison/reduction/generator/quantization ops, exact field `div`, dynamic-output `split`, and rank-2 matrix-contraction `einsum` | Continue remaining exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial; Iteration 64 `div` vector implemented | Registry-derived admitted-op guard, CPU profile evidence, exact vectors for current admitted ops including multi-output quantization, exact field `div`, `split`, and `einsum`; default CUDA non-admission | Add CUDA conformance evidence and continue exact Tier-B op vectors |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 81 in progress | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; assigned auditor policy; appeal reward-void resolution through pending claims; chain-owned pending claim view; executable study helper; live validator-audit calibration status/explorer evidence | Enforce receipt reward release only after blockspace inclusion, then add broader bond calibration and governed stake-slash reversal |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 82: Chain-Owned Delayed Receipt Reward Release

Feature capability: receipt rewards stay in the state-rooted pending ledger until canonical block
transition release conditions are satisfied, and block application itself releases matured receipt claims on
producer and non-producer peers.

Readiness requirements covered: `upow.md` §12.1 and `mvp_spec.md` §20.3/§25 reward-finality delay.

Canonical owner: `chain::commands::release_all_matured_rewards` owns release, with `chain::blocks`
invoking it during canonical child-state construction after inclusion/slash updates.
Adapter callers: manual release commands remain test/operator entry points, while runtime, status,
explorer, p2p, RPC, and checker paths observe the chain-owned pending/spendable ledgers.
Old shortcut being removed: treating reward delay as adapter-side/manual post-processing after block
application rather than consensus-owned child-state transition behavior.
Regression test that proves the shortcut is gone: focused reward tests produce/apply blocks on a producer
and peer, show included receipt rewards remain pending before maturity, then release automatically through a
later block transition with matching state roots and balances.
Behavior with local synthetic block production disabled: no receipt reward becomes spendable until a valid
local or inbound canonical block transition reaches the claim maturity condition.
Behavior for producer and non-producer roles: producers and non-producers execute the same child-state
release sweep when producing or admitting a block.
Structured evidence source: `PendingReceiptReward`, `included_receipts`, `reward_root`,
`RewardState::balance`, and block child-state equality across producer/peer.
Finality source: unchanged; spendability is gated by claim maturity and inclusion, while BFT finality stays
separate from admission.
Wire-size and codec boundary: no wire change; existing block/state codecs carry the ledgers and roots.

Files/modules likely touched: `chain/tests/rewards.rs`, status/coverage/Tarpaulin docs, and this exec plan.
Parallel subagents to run: none; user prefers no subagents unless explicitly requested.
Parallelizable implementation workstreams: read-only discovery and validation only.
Tests/checkers/docs to add or update: focused reward transition regression and docs status wording.
Narrow validation commands: `cargo test -p tensor_vm reward --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt.
Expected observable evidence: delayed receipt rewards remain pending before maturity and become spendable
only through a canonical block transition shared by producer and peer.
Out of scope: Docker rerun, fork choice, transcript disputes, and reward parameter calibration.
Split trigger: if release semantics need storage schema or wire changes, split persistence/codec from the
release-rule proof.

Implementation summary:
- Added a focused producer/peer regression proving a pending included receipt reward is not credited before
  maturity and is later released by canonical block child-state application on both nodes without a manual
  release command.
- Updated status, coverage, and Tarpaulin docs to record chain-owned delayed receipt reward release
  evidence.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm reward --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 412 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

### Iteration 81: Inclusion-Gated Receipt Reward Release

Feature capability: settled receipt reward claims remain non-spendable until the receipt is included in
canonical blockspace and the full reward-settlement plus challenge-window maturity delay has elapsed from
that inclusion point.

Readiness requirements covered: `upow.md` §12.1 and `mvp_spec.md` §20.3/§25 receipt reward-finality delay.

Canonical owner: `chain::commands::release_matured_receipt_rewards` owns spendable reward release, and
`chain::blocks` owns inclusion-height claim delay updates during canonical block application.
Adapter callers: manual release commands, rewarded block transitions, local/runtime status, explorer, and
checkers observe the same state-rooted pending/spendable ledgers.
Old shortcut being removed: a settled receipt reward claim could mature and be swept to spendable balance
before its receipt was included in blockspace if blockspace inclusion lagged.
Regression test that proves the shortcut is gone: focused reward/settlement tests keep an originally mature
but unincluded receipt claim pending, include the receipt in a block, verify the claimable height is pushed
to inclusion plus maturity, and release only after that height.
Behavior with local synthetic block production disabled: settled but unincluded rewards remain pending and
non-spendable until an inbound or local valid block includes the receipt.
Behavior for producer and non-producer roles: producers and non-producers both execute the same block child
state and release sweeper; neither can credit an unincluded receipt reward.
Structured evidence source: `ChainState::included_receipts`, `PendingReceiptReward::receipt_id`,
`PendingReceiptReward::claimable_at_height`, reward balances, and `reward_root`.
Finality source: unchanged; reward finality remains separate from block admission/finality and is gated by
claim maturity after inclusion.
Wire-size and codec boundary: no wire change; existing state/block codecs already persist included receipts
and pending reward ledgers.

Files/modules likely touched: `chain/commands.rs`, focused reward/settlement tests, coverage/status docs,
and this exec plan.
Parallel subagents to run: none; user prefers no subagents unless explicitly requested.
Parallelizable implementation workstreams: read-only discovery and validation only.
Tests/checkers/docs to add or update: focused receipt reward release tests and docs status for reward
delay semantics.
Narrow validation commands: `cargo test -p tensor_vm receipt_rewards --quiet`, `cargo test -p tensor_vm
reward --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt.
Expected observable evidence: reward balances stay zero for settled-but-unincluded receipt claims; block
inclusion extends the claim maturity window; spendable credit happens only after the extended height.
Out of scope: Docker rerun, fork choice, fraud-proof transcripts, and broad bond calibration.
Split trigger: if inclusion-gating requires schema changes to persisted reward records, split storage
migration from the release-rule fix.

Implementation summary:
- Updated `release_matured_receipt_rewards` so a pending receipt claim must be height-mature and its
  receipt must be present in `included_receipts` before spendable credit can be released.
- Extended settlement/block tests so artificially mature settled-but-unincluded receipt claims remain
  pending, block inclusion pushes the claimable height to inclusion plus maturity, and release happens only
  after that height.
- Updated status, coverage, and Tarpaulin docs to record the stricter reward-finality rule.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm receipt_rewards --quiet`, `cargo test -p tensor_vm reward --quiet`,
  and `cargo test -p tensor_vm settlement --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `1647a47` (`Gate receipt rewards on inclusion`) is pushed to `origin/main`.

## Recent Iterations

### Iteration 80: PoW-Skip Fallback Timeout Enforcement

Non-genesis empty fallback blocks now require the configured timeout after the parent, while useful UVPoW
blocks remain undelayed. Validation passed focused fallback/retarget/runtime tests, full crate, clippy,
workspace release, and first/final Gate 0; tarpaulin remained blocked. Feature commit `f5a0aa2` is pushed.

### Iteration 74: Validator Audit Appeal Reward-Delay Resolution

Appeal resolution now mutates delayed pending validator reward claims directly. Upheld outcomes keep the
claim voided for normal pruning; reversed reward-void outcomes clear the void flag but do not credit
spendable balance until `release_matured_receipt_rewards`. Validation passed focused audit/storage tests,
full crate, clippy, workspace release, and first/final Gate 0; tarpaulin remained blocked. Feature commit
`c8a6f9e`, evidence commit `32fb557`, and push-evidence commit `7026c94` are pushed.

### Iteration 73: Live Validator-Audit Economic Calibration

Chain state, service status, and explorer overview expose live validator-audit economic calibration from
configured audit sampling, slash amount, and current non-voided pending validator reward exposure.
Validation passed focused chain/status/RPC tests, full crate, clippy, workspace release, and first/final
Gate 0; tarpaulin remained blocked. Feature commit `493191c` and evidence commit `8dbb654` are pushed.

### Iterations 75-77: Diagnostic Block-Check Challenge Path

Deterministic bad-block challenge fixtures, bounded observed malformed-block p2p/cache handling, and live
validator-proposer diagnostic emission are implemented and pushed in commits `8787912`, `40f14d5`, and
`06be27e`; validation passed focused challenge/wire/runtime tests, full crate, clippy, workspace release,
and first/final Gate 0, with tarpaulin still blocked.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent. Stale readiness/exec text should be
  updated as part of feature work.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable acceptance
  command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in the shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping;
  this iteration stayed single-writer because chain state, storage, roots, and tests were tightly coupled.

## Validation Evidence

Latest full validation is Iteration 82 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm reward --quiet
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
