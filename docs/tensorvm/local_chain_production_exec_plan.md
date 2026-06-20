# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none; Iteration 31 is validated and awaiting commit/push evidence.
- Current status: Iteration 31 implemented and validated on June 20, 2026; commit/push evidence update pending.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit and push Iteration 31, then record commit/push evidence.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 26: `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 and again after implementation | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`, Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenge payloads | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partially complete | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots, live validator proposer networking |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job `program_hash` binding, current-job graph body state-root/storage, and P2P `RequestProgram` serving | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Remaining: broader executable admitted registry vectors, generic graph interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add after IR/conformance and remaining block validity gaps |
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, delayed challenger reward claims, local challenge penalties, challenge/unavailable-data voiding for pending receipt claims, data-unavailability miner bond slashing, configured validator mandatory-audit reward delay/slashing, and network-visible validator audit reports exist; full bond calibration and appeal-safe security are not complete | Add auditor-selection policy, appeal paths, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Recent Iterations

### Iteration 31: Network-Visible Block-Check Challenge Propagation

Implemented and awaiting commit/push evidence.

Summary:
- Added bounded block-check challenge p2p payloads with explicit challenge-id, block-hash, challenger,
  proof-size, and trailing-byte validation.
- Node ingestion now queues challenges while the challenged block is missing, retries through the shared
  pending-payload processor, and applies only through `ChainCommand::SubmitBlockCheckChallenge`.
- Network-ingested challenges preserve the canonical delayed challenger reward claim: acceptance creates a
  pending challenge reward and spendable balance is credited only by `ReleaseMaturedChallengeRewards`.
- Runtime/status/checker surfaces report block-check challenge ingestion and application counters without
  claiming live deterministic bad-block generation.

Validation:
- Required Gate 0 first and final Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.
- Focused p2p wire, node payload application, pending payload, message ingest, runtime state, CLI status,
  and local CPU Compose tests passed.
- Added focused delayed-reward network challenge coverage in
  `node::payload_application::tests::block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges`.
- `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.

Architecture shortcut answers:
- Canonical owner: `chain::challenges` via `ChainCommand::SubmitBlockCheckChallenge`.
- Adapter callers: p2p decode, node payload application/retry, runtime status, and local checker.
- Old shortcut being removed: block-check challenge evidence is local chain-only and not network-visible.
- Regression test: non-producer node ingest applies a serialized challenge payload through the shared node
  event path, including pending retry when the challenged block arrives later.
- Synthetic production disabled: inbound challenge payloads still decode, queue, retry, and apply when
  dependencies exist.
- Producer/non-producer behavior: any node may ingest valid challenges; only chain admission mutates state.
- Structured evidence source: `NetworkEventIngest`, role runtime status fields, and checker status reads.
- Finality source: unchanged block-vote finality; challenges affect rewards, receipt settlement, and
  proposer throttle state.
- Wire boundary: reuse shared bounded codec/wire patterns with a hard Merkle-proof sibling limit before
  allocation.

Out of scope: interactive `trace_root` fraud proofs, deterministic live bad-block generation in Docker, and
changing reward maturity rules.

### Iteration 30: Validator Proposer Delayed-Reward Evidence

Implemented and pushed as `5664acb` (`Delay validator proposer rewards`).

Summary:
- Validator runtime status separates useful settled-receipt block proposals from empty fallback proposals.
- Useful validator-owned proposals use `ChainCommand::ProduceRewardedBlock` to create pending proposer
  reward claims instead of spendable balances; fallback proposals remain unrewarded.
- The local checker requires useful proposal, proposed-receipt, and pending proposer reward evidence.
- `chain` remains the canonical owner of block production, fallback classification, delayed rewards, and
  finality; runtime/status/checker code only records structured evidence.

Validation:
- Required Gate 0 first and final Gate 0 passed.
- Focused runtime-state, runtime-role, CLI status, local CPU Compose, and explorer-schema tests passed.
- `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `6dfd688..5664acb  main -> main`.

### Iteration 29: Network-Visible Validator Audit Reports

Implemented and pushed as `4e8b0c6` (`Propagate validator audit reports`).

Summary:
- Registered validator roles observe state-rooted audit assignments, skip self-audits and expired or
  already-settled audits, verify local receipt artifacts, and submit signed audit reports through
  `ChainCommand::SubmitValidatorAuditReport`.
- Added bounded validator-audit-report p2p payloads on the existing attestation gossip topic, node
  application/retry handling, duplicate/conflict rejection, and runtime status counters for submitted,
  ingested, and applied audit reports.
- Remaining economics boundaries are full auditor-selection policy, transcript disputes, appeal-safe
  slashing, challenge gossip, and bond/gain calibration.

Validation:
- Required Gate 0 first and final Gate 0 passed.
- Focused p2p codec, node pending/application/ingest, validator role, network payload, runtime state,
  runtime roles, and runtime persistence tests passed.
- `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `fcbf2e8..4e8b0c6  main -> main`.

### Iteration 28: Validator Audit Reward Slashing

Implemented and pushed as `99d819c` (`Add validator audit reward slashing`).

Summary:
- Added state-rooted mandatory validator audit assignments/results/slashes under configured sampling.
- Audit assignment delays the audited validator's pending receipt reward through the audit deadline; missed
  or contradictory audits slash once, credit treasury, and void the delayed validator reward.
- Full runtime audit-report propagation was left to Iteration 29; appeal paths and bond calibration remain
  open.
- Required Gate 0, focused attestation/root/storage/RPC/explorer tests, fmt, diff check, full tensor_vm
  tests, clippy, and release workspace tests passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `8236dfa..99d819c  main -> main`.

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
- Iteration 30 required Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 30 focused validation:
  - `cargo test -p tensor_vm --lib node::runtime_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --test tvmd_runtime runtime_roles -- --nocapture`: 7 tests passed.
  - `cargo test -p tensor_vm --test tvmd_runtime runtime_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --test local_cpu_compose -- --nocapture`: 1 test passed.
  - `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces -- --nocapture`: 1 test passed.
  - `cargo test -p tensor_vm_explorer --lib -- --nocapture`: 1 test passed.
- Iteration 30 broad validation before feature commit:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - Final `cargo test -p tensor_vm local_testnet --release`: passed.
  - `cargo test -p tensor_vm`: passed with 341 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 29 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 341 `tensor_vm`, 1 local CPU
    Compose, 8 `tvmd_cli`, 29 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests,
    and doc-test targets.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 30 feature commit: `5664acb` (`Delay validator proposer rewards`).
- Iteration 30 push result: `6dfd688..5664acb  main -> main` on `origin/main`.

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
