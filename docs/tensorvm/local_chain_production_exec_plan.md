# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none. The next feature-sized slice is the next highest-priority v0 gap from the
  readiness matrix.
- Current status: Iteration 26, delayed challenger reward finality, is implemented, locally validated, and
  pushed as `25dbfe4` (`Delay challenger reward finality`).
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue with the next highest-priority v0 gap, likely slashable bond/audit/data-withholding
  invariants or generic arbitrary-IR execution.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 26: `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 and again after implementation | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`, Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, and block votes | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partially complete | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and local check challenges | Remaining: full transcript disputes, network/RPC challenge propagation, exact replayable snapshots, live validator proposer networking |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job `program_hash` binding, current-job graph body state-root/storage, and P2P `RequestProgram` serving | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Remaining: broader executable admitted registry vectors, generic graph interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add after IR/conformance and remaining block validity gaps |
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, delayed challenger reward claims, local challenge penalties, and challenge voiding for pending receipt claims exist; hard miner/validator bond invariant not complete | Add slashable bond/audit/data-withholding invariant slice |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 26: Delayed Challenger Reward Finality

Feature capability:
Replace immediate spendable crediting of successful block-check challenger bounties with a state-rooted
pending challenger reward claim. A successful block-check challenge still voids the proposer reward, sends
the clawback remainder to treasury, quarantines affected receipt rewards, and throttles the proposer, but
the challenger bounty becomes spendable only after maturity through an explicit release command.

Readiness requirements covered:
- `upow.md` §12.1: challenger rewards are part of the fraud-detection incentive path and must remain
  challenge-window aware.
- `mvp_spec.md` §19 and §20.4: rewards are calculated/finalized after verification challenge windows,
  while block finality and reward finality are distinct.
- `mvp_spec.md` §20.7 and §25.5: successful checks-root challenges claw back proposer rewards and reward
  challengers without bypassing consensus reward finality.
- `goal.md` economics gap: move reward finality from immediate spendable balances into explicit
  consensus state instead of adapter workarounds.

Subagents run:
- `readiness-mapper`: maps delayed challenger rewards to reward-finality and challenge-window
  requirements.
- `tensorvm-codebase-explorer`: maps chain/state/root/storage/RPC-status implementation path.
- `tensorvm-test-coverage-explorer`: maps focused reward/challenge/storage tests and validation commands.

Architecture shortcut answers:
- Canonical owner: `chain` owns challenge resolution, pending challenger reward claims, release, roots, and
  storage; adapters may only observe events or request commands.
- Adapter callers: RPC/runtime/checkers can surface pending and released rewards, but they do not decide
  challenger spendability.
- Old shortcut being removed: successful block-check challenges credited the challenger directly into
  spendable `RewardState` during challenge resolution.
- Regression test that proves the shortcut is gone: successful block-check challenge records a pending
  challenger claim, leaves the challenger spendable reward balance at zero until maturity, and release later
  credits exactly the pending amount once.
- Behavior with local synthetic block production disabled: unchanged; inbound challenge commands mutate the
  same chain state regardless of local producer policy.
- Behavior for producer and non-producer roles: both validate and persist the same delayed reward state
  after applying the challenge command.
- Structured evidence source: chain challenge/reward tests, chain-state storage roundtrip test, status/docs
  updates; no shell-only assertion.
- Finality source: unchanged, signed validator block votes finalize blocks; reward finality is separate and
  state-rooted.
- Wire-size and codec boundary: no new P2P or storage wire family; storage extends the existing bounded
  chain-state snapshot codec.

Implementation plan:
Implementation summary:
- Added `PendingChallengeReward` state keyed by deterministic claim id.
- Included pending challenge rewards in `ChainState`, state roots, genesis/from-parts, and node-store
  snapshot encode/decode.
- Changed block-check challenge resolution to enqueue the challenger bounty instead of crediting it
  immediately; the clawback remainder still credits treasury.
- Added `ReleaseMaturedChallengeRewards` command/event handling matching existing proposer/receipt release
  commands.
- Surfaced `pending_challenge_reward_count` in node status and explorer summary output.
- Updated chain/storage/docs tests to prove delayed spendability, single release, and persistence.

Narrow validation commands:
- `cargo test -p tensor_vm --lib chain::tests::challenges -- --nocapture`
- `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`
- `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `git diff --check`
- `cargo tarpaulin --workspace --offline` (expected blocked here unless `cargo-tarpaulin` is installed)

Expected observable evidence:
- A proven block-check challenge no longer immediately increases the challenger reward balance.
- Pending challenger rewards are committed in the state root and survive storage roundtrip.
- Matured pending challenger rewards release once into spendable reward balances.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused validation passed:
  - `cargo test -p tensor_vm --lib chain::tests::challenges -- --nocapture`: 3 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm_explorer --lib`: 1 test passed.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm` passed with 333 library tests, 1 local CPU Compose integration test, 8
  `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed with 14 `experiments`, 333 `tensor_vm`, 1 local CPU Compose,
  8 `tvmd_cli`, 28 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and
  doc-test targets.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because this environment does not have
  the `cargo-tarpaulin` subcommand installed.

Push evidence:
- Feature commit: `25dbfe4` (`Delay challenger reward finality`).
- Remote/branch: `origin/main`.
- Push result: `f734a69..25dbfe4  main -> main`.

Out of scope:
- Full network/RPC challenge gossip.
- Hard stake slashing for invalid attestations or data withholding.
- Interactive trace fraud proofs.

Split trigger:
Split only if status/RPC exposure needs a broader typed snapshot refactor beyond the chain/storage/docs
surface.

## Recent Iterations

### Iteration 25: Graph-Body Propagation and Storage

Implemented and pushed as `0363bb6` (`Store Tensor IR graph bodies`), with evidence update `f734a69`.

Summary:
- Current TensorOp and LinearTrainingStep job admission stores validated canonical graph bodies keyed by
  `graph_id`.
- State roots and node-store snapshots commit and roundtrip the graph-body registry.
- The existing libp2p `RequestProgram`/`ProgramResponse` path serves registered canonical graph bytes.
- Generic arbitrary-IR admission/execution remains out of scope.

Validation:
- Required Gate 0 first and final Gate 0 passed.
- Focused chain/storage/p2p/jobs tests passed.
- `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `b0fe92c..0363bb6  main -> main`; evidence push `0363bb6..f734a69  main -> main`.

### Iteration 24: Per-Op `F_p` Conformance Vector Gate

Implemented and pushed as `f4d4491` (`Add Fp conformance vector gate`).

Summary:
- Added deterministic current-job `F_p` conformance vectors, a stable suite hash, CPU pass reporting,
  default-build CUDA non-admission, and TensorOp/LinearTrainingStep verifier gates.
- Required Gate 0, focused conformance/runtime/verifier/jobs tests, fmt, full tensor_vm tests, clippy,
  release workspace tests, and diff check passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `94c8007..f4d4491  main -> main`.

### Iteration 23: Delayed Receipt Reward Finality

Implemented and pushed as `388c4d6` (`Delay receipt reward finality`): receipt settlement creates
state-rooted miner/validator pending reward claims, block inclusion extends maturity through the
settlement/challenge window, release credits only mature non-void claims, and successful block-check
challenges void affected pending receipt rewards before spendability.

## Decision Log

- `docs/tensorvm/upow.md` is canonical when it conflicts with older readiness text.
- Keep the missing workflow document visible as a standing blocker; do not treat the readiness doc as a
  substitute.
- Preserve one shared chain engine. Deployment profiles can vary, but transition logic must not fork.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through the
  shared P2P/event path.
- TensorWork affects rewards, blockspace, telemetry, and concentration analysis only; it never selects
  block proposers.
- `tvmd` is an adapter/process launcher, not a hidden consensus orchestrator.
- Current v0 admits exact Tier-A/B ops only. Tier-C vocabulary may exist in the registry but must be gated
  out of consensus until canonical references and verifiers exist.
- Current-job graph bodies are stored as canonical JSON bytes after graph validation; generic arbitrary-IR
  decoding/execution remains a separate future slice.

## Validation Evidence

Latest current-iteration evidence:
- Starting branch state: `## main...origin/main`.
- Iteration 26 required Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 26 validation before feature commit:
  - `cargo test -p tensor_vm --lib chain::tests::challenges -- --nocapture`: 3 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm_explorer --lib`: 1 test passed.
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - Final `cargo test -p tensor_vm local_testnet --release`: passed.
  - `cargo test -p tensor_vm`: passed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 26 feature commit: `25dbfe4` (`Delay challenger reward finality`).
- Iteration 26 push result: `f734a69..25dbfe4  main -> main` on `origin/main`.

Latest unresolved full-gate blocker:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /health
```

Latest coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iteration 22, `8e17789 Add content addressed tensor IR`: added `TensorGraph`, canonical JSON graph IDs,
  frozen registry metadata, structural validation, Tier-C consensus gating, canonical current-job graph
  constructors, and current receipt `program_hash` binding to IR `graph_id`.
- Iteration 21, `62e5600 Add delayed proposer reward challenges`: added delayed proposer rewards and local
  block-check challenges.
- Iteration 20, `1f2b74d Bind consensus randomness to finalized beacon` plus evidence `a3784ad`: bound
  blocks, assignments, validation seeds, and check leaves to finalized beacon state.
- Iteration 19, `232256d Add canonical block apply openings`: blocks commit to child state roots,
  Merkle-openable selected receipt/check roots, and parent/child block status evidence.
- Iteration 18, `af33fe1 Add UVPoW retarget fallback mode`: added bounded difficulty retargeting and
  explicit zero-receipt PoW-skip fallback blocks.
- Iterations 1-17: extracted reusable node runtime state, moved network payload application and event
  drivers into reusable runtime helpers, bound role runtimes to chain identities, added role loop
  boundaries, miner receipt submission, validator attestations, validator block votes, network-visible block
  payload admission, useful-verification PoW block validity, remote validator tensor fetch, validator-owned
  block proposal ticks, and checker evidence for role-owned local work.
