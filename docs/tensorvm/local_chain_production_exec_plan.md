# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 84 in progress: Validator Audit Stake-Slash Reversal.
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
- Next action: implement governed stake refund on reversed validator-audit appeals, then continue with the
  next non-Docker consensus gap or rerun the full Docker scenario after the `/health` blocker clears.

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
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness, assignment seed, and validation seed commitment; attestations require the anchor | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 84 in progress | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; assigned auditor policy; appeal reward-void resolution through pending claims; chain-owned pending claim view; executable study helper; live validator-audit calibration status/explorer evidence | Add governed validator-audit stake-slash reversal, then broader bond calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 84: Validator Audit Stake-Slash Reversal

Feature capability: reversed validator-audit appeals refund the recorded stake slash from treasury back to
the slashed validator exactly once, while upheld appeals remain unchanged and reward reinstatement still
uses the delayed pending-claim path.

Readiness requirements covered: `upow.md` §12 economics/slashing and the local readiness economics gap for
governed stake-slash reversal.

Canonical owner: `chain::validation::resolve_validator_audit_appeal` mutates validator stake, treasury, and
the state-rooted appeal record.
Adapter callers: CLI/RPC/runtime callers use `ChainCommand::ResolveValidatorAuditAppeal`; storage/status
observe the same chain state.
Old shortcut being removed: reversed appeals previously cleared only the delayed reward void flag and left
the stake slash permanently applied.
Regression test that proves the shortcut is gone: focused audit appeal tests assert reversed appeals refund
validator stake, debit treasury, persist the refunded amount, and reject duplicate resolution; upheld
appeals do not refund.
Behavior with local synthetic block production disabled: audit appeal resolution is a chain command over
existing audit/slash records and does not depend on local job or block production.
Behavior for producer and non-producer roles: producers and non-producers replay the same chain command and
state-rooted appeal record; no adapter branch owns the refund.
Structured evidence source: `ValidatorAuditAppealRecord::stake_refunded_amount`, validator stake,
treasury balance, state root, command event, and storage roundtrip.
Finality source: the appeal resolution command records resolved height in canonical state; block
admission/finality remains separate.
Wire-size and codec boundary: chain-state storage/root encoding changes for audit appeal records; no p2p
payload tag or bounded wire format change.

Files/modules likely touched: `chain/state.rs`, `chain/validation.rs`, `chain/commands.rs`,
`chain/roots.rs`, `storage/chain_state.rs`, focused audit/storage tests, status docs, and this plan.
Parallel subagents to run: none; user prefers no subagents unless explicitly requested.
Parallelizable implementation workstreams: read-only discovery and validation only.
Tests/checkers/docs to add or update: focused audit appeal/storage tests plus docs status for §12 progress.
Narrow validation commands: `cargo test -p tensor_vm validator_audit --quiet`, `cargo test -p tensor_vm
storage::chain_state --quiet`, `cargo test -p tensor_vm reward --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt.
Expected observable evidence: reversed audit appeals restore the slashed validator stake and reduce treasury
by the refunded amount without immediately crediting delayed validator rewards.
Out of scope: broader fraud-path bond calibration, external governance process, and fork-choice policy.
Split trigger: if appeal records need a broader governance identity/signature format, split that from the
canonical refund state transition.

Implementation summary:
- Added `stake_refunded_amount` to `ValidatorAuditAppealRecord`, state roots, and chain-state storage.
- Reversed validator-audit appeals now debit treasury and restore the recorded slash to validator stake
  through `resolve_validator_audit_appeal`; upheld appeals keep the slash applied.
- `ValidatorAuditAppealResolved` events now expose the stake refund amount, while delayed validator reward
  release still uses the pending receipt reward sweeper.
- Updated §12/status/coverage docs for governed stake-slash reversal progress.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm validator_audit --quiet`, `cargo test -p tensor_vm
  storage::chain_state --quiet`, and `cargo test -p tensor_vm reward --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 413 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

## Recent Iterations

### Iteration 83: Receipt-Bound Validation Challenge Seed

Admitted receipts now persist receipt-time finalized beacon randomness, assignment seed, and validation
seed commitment. `Chain::validation_seed` derives admitted-receipt challenge vectors from that commitment,
and attestation admission rejects stored receipts missing their anchor. Focused randomness/proposer/storage
tests, full crate, clippy, workspace release, first/final Gate 0 passed; tarpaulin remained blocked.
Feature commit `e08f7c9` and evidence commit `22641f7` are pushed.

### Iteration 82: Chain-Owned Delayed Receipt Reward Release

Receipt rewards now have focused producer/peer evidence that canonical block child-state application, not a
manual adapter release command, credits included matured receipt claims. Validation passed focused reward
tests, full crate, clippy, workspace release, first/final Gate 0, and tarpaulin remained blocked. Feature
commit `8ce051f` and evidence commit `bbe06c6` are pushed.

### Iteration 81: Inclusion-Gated Receipt Reward Release

Receipt rewards now require blockspace inclusion before height-mature pending claims can become spendable,
with inclusion pushing claim maturity to inclusion plus the full delay. Validation passed focused
reward/settlement tests, full crate, clippy, workspace release, first/final Gate 0, and tarpaulin remained
blocked. Feature commit `1647a47` and evidence commit `8f35712` are pushed.

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

Latest full validation is Iteration 84 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm validator_audit --quiet
cargo test -p tensor_vm storage::chain_state --quiet
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
