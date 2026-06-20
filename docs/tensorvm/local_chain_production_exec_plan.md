# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 79 complete and pushed.
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
- Next action: select the next non-Docker consensus gap or rerun the full Docker scenario after the
  `/health` blocker clears.

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
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, stake-weighted fallback proposer eligibility, replay-stable parent snapshots/apply outcomes, delayed rewards, network-visible block-check challenges, deterministic diagnostic bad-block challenge generation, live diagnostic emission, and observed-malformed-block p2p/cache support | Remaining: full transcript disputes, timeout/fork-choice policy, fresh Docker proof |
| Tensor IR graph language | Partial; Iteration 64 field `div` implemented | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core, exact unary/structural/comparison/reduction/generator/quantization ops, exact field `div`, dynamic-output `split`, and rank-2 matrix-contraction `einsum` | Continue remaining exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial; Iteration 64 `div` vector implemented | Registry-derived admitted-op guard, CPU profile evidence, exact vectors for current admitted ops including multi-output quantization, exact field `div`, `split`, and `einsum`; default CUDA non-admission | Add CUDA conformance evidence and continue exact Tier-B op vectors |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 74 appeal reward resolution implemented | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; assigned auditor policy; appeal reward-void resolution through pending claims; chain-owned pending claim view; executable study helper; live validator-audit calibration status/explorer evidence | Add broader bond calibration and governed stake-slash reversal |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 79: Replay-Stable Historical Block Apply Evidence

Feature capability: historical `BlockApplyOutcome` and block-status evidence use durable exact block-parent
state snapshots before recomputing selected receipts, checks roots, parent snapshots, and child roots.

Readiness requirements covered: `upow.md`/`mvp_spec.md` §11 block validity evidence, selected-receipt
lifecycle/opening metadata, and the local readiness gap for exact replayable parent-state/apply semantics.

Canonical owner: `chain::blocks::parent_state_for_validation` remains the single chain-owned parent
reconstruction boundary for block validation, apply outcomes, selected receipts, status, and challenges.
Adapter callers: block status, challenge helpers, `Chain::validate_block`, block production/admission, and
runtime/network adapters consume the chain-owned outcome rather than reconstructing state themselves.
Old shortcut being removed: historical parent reconstruction inferred an old parent from the current head
state, letting later jobs, receipts, attestations, rewards, and audit metadata change old block evidence.
Regression test that proves the shortcut is gone: produce an old useful block, add a later settled receipt
and block, then require the old block's apply outcome/checks root/child state root to remain stable and not
select the future receipt.
Behavior with local synthetic block production disabled: unchanged; inbound/status evidence is derived from
canonical chain state only and does not depend on scheduled local production.
Behavior for producer and non-producer roles: producers persist selected receipts for emitted blocks;
non-producers recompute the same historical parent boundary while admitting or reporting received blocks.
Structured evidence source: `BlockApplyOutcome`, `BlockParentSnapshot`, selected receipt openings, and block
status fields backed by chain roots.
Finality source: unchanged; parent/apply evidence is admission/status evidence, while finalized blocks still
come from explicit block votes.
Wire-size and codec boundary: no wire change; parent snapshots persist in the existing chain-state file
codec and existing bounded block/receipt/attestation p2p codecs are unchanged.

Files/modules likely touched: `chain/blocks.rs`, chain block tests, coverage/status/readiness docs, and this
exec plan.
Parallel subagents to run: none; user asked not to use subagents unless explicitly requested.
Parallelizable implementation workstreams: read-only discovery and validation only; code edits are
single-writer because the parent-state boundary and tests are tightly coupled.
Tests/checkers/docs to add or update: focused chain test for historical apply evidence after future receipts,
storage roundtrip coverage for parent snapshots, and docs status for replayable parent reconstruction.
Narrow validation commands: `cargo test -p tensor_vm historical --quiet`, `cargo test -p tensor_vm block_apply_outcome --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt.
Expected observable evidence: an old useful block's selected receipts, checks root, parent snapshot, and
child root remain valid after newer receipts/blocks are added and after chain-state save/load.
Out of scope: fork-choice side branches, timeout scheduling, full transcript fraud proofs, and Docker rerun
until `/health`.
Split trigger: if exact replay requires multi-branch fork choice, split that work from this canonical
linear-chain snapshot boundary.

Implementation summary:
- Added exact block-parent `ChainState` snapshots to `Chain`, populated by local production and inbound
  block admission.
- Persisted the parent snapshot map in the existing chain-state file codec and restored it through
  `ChainParts`.
- Changed historical parent reconstruction to prefer the stored snapshot, preserving selected receipts,
  checks roots, child roots, and block status/challenge evidence after later receipts/blocks.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm historical --quiet`, `cargo test -p tensor_vm block_apply_outcome --quiet`,
  and `cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 410 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `617ffa9` (`Persist block parent snapshots`) is pushed to `origin/main`.

## Recent Iterations

### Iteration 77: Live Diagnostic Observed Bad-Block Challenge Emission

Validator proposers now gossip one deterministic observed malformed-block challenge after useful proposal
gossip; receivers apply it through the normal delayed pending challenger reward path. The local checker now
requires applied diagnostic challenge counters plus future-maturity challenge reward claims. Validation
passed focused challenge/message/compose tests, full crate, clippy, workspace release, and first/final Gate
0; tarpaulin remained blocked. Feature commit `06be27e` is pushed.

### Iteration 76: Network-Visible Observed Bad-Block Challenges

Observed malformed blocks now propagate through bounded p2p tag 24, cache outside canonical chain state,
and resolve through `ChainCommand::SubmitBlockCheckChallenge`, preserving delayed challenger reward claims.
Validation passed focused challenge/wire tests, full crate, clippy, workspace release, and first/final Gate
0; tarpaulin remained blocked. Feature commit `40f14d5` is pushed.

### Iteration 75: Deterministic Bad-Block Challenge Generation

Deterministic bad-block challenge fixtures now derive from useful blocks without admitting malformed blocks
through consensus validation. Validation passed focused challenge tests, full crate, clippy, workspace
release, and first/final Gate 0; tarpaulin remained blocked. Feature commit `8787912` is pushed.

## Recent Iterations

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

Latest full validation is Iteration 79 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm historical --quiet
cargo test -p tensor_vm block_apply_outcome --quiet
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
