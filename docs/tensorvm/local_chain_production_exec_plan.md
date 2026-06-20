# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 28, validator mandatory-audit slashing.
- Current status: Iteration 28 implementation is in validation on June 20, 2026 after the required Gate 0
  release local-testnet command passed first. The narrow local-reference validator audit path now has
  deterministic audit assignments, signed audit results, delayed audited-validator receipt rewards,
  once-only validator slashing for missed or contradicted audits, and state-rooted observable evidence.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: finish broad validation, commit, push, and record evidence.

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
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, delayed challenger reward claims, local challenge penalties, challenge/unavailable-data voiding for pending receipt claims, data-unavailability miner bond slashing, and configured validator mandatory-audit reward delay/slashing exist; full bond calibration and appeal-safe security are not complete | Add runtime audit workers, appeal paths, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 28: Validator Mandatory-Audit Slashing

Implemented locally, validation still in progress:
- Chain state now stores validator audit assignments, signed reports/results, and slash records; state root
  and chain-state storage commit/persist all three collections.
- Canonical block application samples submitted attestations from finalized-beacon-bound randomness when
  audit sampling is configured. Assignment IDs are deterministic from receipt id and audited validator.
- Assignment explicitly delays the audited validator's pending receipt reward through the audit deadline.
  A missed audit or contradictory signed audit result slashes the audited validator once, credits treasury,
  voids that delayed validator reward before spendability, and exposes status/RPC/explorer counts.
- Architecture checkpoint: `chain` owns assignment, result admission, stake mutation, reward voiding, roots,
  and persistence; runtime/RPC/checkers are adapters. Producer and non-producer roles recompute the same
  deterministic transition during block validation/application. No new P2P family is included in this slice.
- Parallel subagents mapped readiness obligations, chain/state/root/storage/status ownership, and adjacent
  tests. They warned not to overclaim full economics.
- Out of scope remains runtime validator audit workers, P2P/RPC propagation of audit payloads, transcript
  disputes, appeals, challenge gossip, full bond/gain-from-fraud calibration, Tier-C/general IR audit
  security, and Docker readiness while gateway `/health` remains blocked.

Validation evidence so far:
- Required Gate 0 first executable: `cargo test -p tensor_vm local_testnet --release` passed on
  June 20, 2026 before implementation.
- `cargo test -p tensor_vm --lib chain::tests::attestations -- --nocapture` passed.
- `cargo test -p tensor_vm --lib chain::tests::root_hashes -- --nocapture` passed.
- `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture` passed.
- `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture` passed.
- `cargo test -p tensor_vm_explorer --lib` passed.
- `cargo test -p tensor_vm` passed after adding explicit audit reward delay/voiding.

Final validation before commit:
- `cargo fmt --check --all`: passed.
- `git diff --check`: passed.
- Final `cargo test -p tensor_vm local_testnet --release`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace --release`: passed.
- `cargo tarpaulin --workspace --offline`: blocked because `cargo-tarpaulin` is not installed.

## Recent Iterations

### Iteration 27: Data-Unavailability Reward Cancellation and Miner Bond Slashing

Implemented and pushed as `cae45b5` (`Handle unavailable receipt rewards and slashing`).

Summary:
- Unavailable-data attestations mark receipts non-finalizable, void pending receipt rewards, slash the
  receipt miner once, credit treasury, and persist/expose state-rooted slash records.
- Required Gate 0, focused attestation/settlement/storage/RPC/explorer tests, fmt, diff check, full
  tensor_vm tests, clippy, and release workspace tests passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `336463f..cae45b5  main -> main`.

### Iteration 26: Delayed Challenger Reward Finality

Implemented and pushed as `25dbfe4` (`Delay challenger reward finality`).

Summary:
- Added state-rooted pending challenge reward claims, storage/root/status/explorer support, and explicit
  maturity release instead of immediate challenger spendability.
- Required Gate 0, focused challenge/reward/storage/RPC/explorer tests, fmt, diff check, full tensor_vm
  tests, clippy, and release workspace tests passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `f734a69..25dbfe4  main -> main`.

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
- Iteration 28 required Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 28 focused validation:
  - `cargo test -p tensor_vm --lib chain::tests::attestations -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::root_hashes -- --nocapture`: 3 tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm_explorer --lib`: 1 test passed.
- Iteration 28 broad validation before feature commit:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - Final `cargo test -p tensor_vm local_testnet --release`: passed.
  - `cargo test -p tensor_vm`: passed with 338 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 338 `tensor_vm`, 1 local CPU
    Compose, 8 `tvmd_cli`, 28 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests,
    and doc-test targets.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 28 feature commit: pending.
- Iteration 28 push result: pending.

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
