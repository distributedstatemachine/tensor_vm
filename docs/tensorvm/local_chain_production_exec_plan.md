# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 77, live diagnostic observed bad-block challenge emission.
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
  pending challenge reward claims.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: rerun the full Docker scenario after the `/health` blocker clears.

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
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, network-visible block-check challenges, deterministic diagnostic bad-block challenge generation, live diagnostic emission, and observed-malformed-block p2p/cache support | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, fresh Docker proof |
| Tensor IR graph language | Partial; Iteration 64 field `div` implemented | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core, exact unary/structural/comparison/reduction/generator/quantization ops, exact field `div`, dynamic-output `split`, and rank-2 matrix-contraction `einsum` | Continue remaining exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial; Iteration 64 `div` vector implemented | Registry-derived admitted-op guard, CPU profile evidence, exact vectors for current admitted ops including multi-output quantization, exact field `div`, `split`, and `einsum`; default CUDA non-admission | Add CUDA conformance evidence and continue exact Tier-B op vectors |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 74 appeal reward resolution implemented | Delayed proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; audit/data-unavailability slashing; assigned auditor policy; appeal reward-void resolution through pending claims; chain-owned pending claim view; executable study helper; live validator-audit calibration status/explorer evidence | Add broader bond calibration and governed stake-slash reversal |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 77: Live Diagnostic Observed Bad-Block Challenge Emission

Feature capability: when a validator role produces a useful block, derive one deterministic observed
malformed-block challenge, gossip it over the shared p2p/node path, and require checker evidence from the
normal delayed pending challenger reward claim path.

Readiness requirements covered:
- `mvp_spec.md` §20.7 and acceptance criterion 4: network-visible validators can dispute a bad
  `checks_root` observation from live block proposal gossip.
- `local_chain_production_readiness.md` gap 7: close the checker-triggered live diagnostic emission gap on
  top of the observed malformed block cache/wire support.

Canonical owner: `chain::challenges` continues to own observed invalid block lookup, proposer clawback, and
pending challenger reward enqueue.
Adapter callers: validator runtime publication and p2p/node ingest call shared codecs and
`ChainCommand::SubmitBlockCheckChallenge`; checker reads status/explorer evidence only.
Old shortcut being removed: checker accepted missing live challenge evidence unless a challenge happened by
chance.
Regression test that proves the shortcut is gone: focused runtime/message/checker tests must prove the
diagnostic payload is emitted/applied and checker output gates on future-maturity challenge reward claims.
Behavior with local synthetic block production disabled: local block production remains disabled for
non-producers; they only ingest/apply the observed challenge payload and delayed reward state.
Behavior for producer and non-producer roles: a producer gossips the observed malformed block challenge for
its useful proposal; non-producers cache/apply it and surface pending challenge reward claims.
Structured evidence source: `NetworkEventIngest.block_check_challenges_applied`,
`ChainState::block_check_challenges`, explorer/status pending challenge reward counts, and future-maturity
pending challenge reward claims.
Finality source: unchanged; challenge affects reward finality, not block-vote finality.
Wire-size and codec boundary: reuse bounded tag 24 `TensorBlock` plus `BlockCheckChallenge` payloads; no
new block codec and no unbounded checker-only blob.

Implementation summary:
- Added `observed_block_check_challenge_messages` and `publish_observed_block_check_challenge` so runtime
  publication reuses bounded `TensorBlock` and `BlockCheckChallenge` codecs.
- The validator proposer tick now derives and gossips one deterministic observed malformed-block challenge
  after useful block proposal gossip, choosing a registered non-proposer challenger when available.
- Tightened the local CPU checker to require positive pending challenge rewards, future-maturity challenge
  reward claims, applied live block-check challenges, and matching reward-evidence output.
- Added focused message-builder, node ingest, and compose contract tests proving the delayed reward path.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm block_check_challenge --quiet` passed.
- Focused: `cargo test -p tensor_vm observed_block_check_challenge --quiet` passed.
- Focused compose: `cargo test -p tensor_vm --test local_cpu_compose --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 407 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
Out of scope: full verifier-transcript fraud proofs, hard proposer stake slashing, and Docker rerun until
the `/health` blocker clears.

## Recent Iterations

### Iteration 76: Network-Visible Observed Bad-Block Challenges

Observed malformed blocks now propagate through bounded p2p tag 24, cache outside canonical chain state,
and resolve through `ChainCommand::SubmitBlockCheckChallenge`, preserving delayed challenger reward claims.
Validation passed focused challenge/wire tests, full crate, clippy, workspace release, and first/final Gate
0; tarpaulin remained blocked. Feature commit `40f14d5` is pushed.

### Iteration 75: Deterministic Bad-Block Challenge Generation

Feature capability: deterministically derive a signed block-check challenge from a produced useful block by
corrupting its observed `checks_root` evidence in a diagnostic path, without admitting malformed blocks
through normal consensus validation.

Implementation summary:
- Added `DeterministicBlockCheckChallenge`, `Chain::deterministic_bad_block_check_challenge`, and
  `Chain::install_diagnostic_observed_block`.
- Replaced the hand-built bad-block fixtures in chain and node payload tests with the shared helper.
- Preserved the existing shared `ChainCommand::SubmitBlockCheckChallenge` resolution path: proposer reward
  clawback, receipt quarantine, proposer throttle, and delayed challenger reward.

Boundary decision: network-wide live propagation of malformed blocks still requires either an
observed-invalid-block side cache or a wire extension carrying the observed invalid block. This iteration
does not weaken `admit_block` to accept malformed blocks.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm block_check_challenge --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 404 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `8787912` (`Add deterministic bad-block challenge helper`) is pushed to `origin/main`.

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

Latest full validation is Iteration 77 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm block_check_challenge --quiet
cargo test -p tensor_vm observed_block_check_challenge --quiet
cargo test -p tensor_vm --test local_cpu_compose --quiet
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
