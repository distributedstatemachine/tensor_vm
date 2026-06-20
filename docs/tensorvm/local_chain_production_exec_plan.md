# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 29, network-visible validator audit reports.
- Current status: Iteration 29 implementation and validation completed after the required Gate 0 release
  local-testnet command passed first on June 20, 2026. Validator audit reports now have a shared validator
  role, p2p payload, node-ingest, retry, publication, and runtime-status path.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit and push the audit-report payload/runtime slice, then record commit/push evidence.

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
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, delayed challenger reward claims, local challenge penalties, challenge/unavailable-data voiding for pending receipt claims, data-unavailability miner bond slashing, configured validator mandatory-audit reward delay/slashing, and network-visible validator audit reports exist; full bond calibration and appeal-safe security are not complete | Add auditor-selection policy, appeal paths, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 29: Network-Visible Validator Audit Reports

Feature capability:
Registered validator roles observe state-rooted validator audit assignments, submit signed audit reports
through `ChainCommand::SubmitValidatorAuditReport`, publish those reports over the shared p2p gossip payload
path, and non-producers apply or retry the payload through the same node-ingest boundary as jobs, receipts,
attestations, blocks, and block votes. Runtime status exposes submitted and network-applied audit-report
counters.

Readiness requirements covered:
- `upow.md` §12.2 and §14: lazy-validator mandatory audits should be live protocol evidence, not only
  direct chain tests.
- `goal.md` interprocess boundary: audit report mutations must cross libp2p/node paths before affecting
  another node.
- Local readiness: role-owned validators should perform their own audit work and shared node ingest should
  carry the resulting consensus payloads.

Files/modules likely touched:
- `crates/tensor_vm/src/api.rs`, `crates/tensor_vm/src/codec.rs`, `crates/tensor_vm/src/p2p.rs`,
  `crates/tensor_vm/src/p2p/wire.rs`
- `crates/tensor_vm/src/node/{payload_application,payload_processor,pending_payloads,message_ingest,runtime_state}.rs`
- `crates/tensor_vm/src/app/{network,validator_role,runtime_validator,runtime_status_snapshot,runtime_status,status}.rs`
- focused p2p/node/runtime tests and docs/exec plan

Parallel subagents run:
- `readiness-mapper`: confirmed runtime/network audit workers as the next local-readiness slice and marked
  full auditor-selection, transcript disputes, appeal-safe slashing, and bond/gain calibration as overclaim
  boundaries.
- `tensorvm-codebase-explorer`: mapped p2p codec, node apply/retry, publication, runtime status, and
  validator-role submission paths.
- `tensorvm-test-coverage-explorer`: mapped codec, ingest, pending retry, validator role, and status tests.

Parallelizable implementation workstreams:
- Parent/integrator owns implementation because p2p payload types, runtime counters, and validator role
  tests share cross-cutting types and would collide if split across writers.
- Subagents remain read-only support.

Tests/checkers/docs to add or update:
- P2P wire roundtrip/malformed/mismatch/topic tests for validator audit report payloads.
- Node payload application and pending retry tests proving unknown assignments pend, later apply, and
  conflicting duplicates reject.
- Validator role/runtime tests proving a registered validator submits a signed audit report from an
  assignment and increments a new status counter.
- Runtime/node status tests for audit report ingest/apply/submission counters.
- Docs/exec plan status updates naming remaining gaps: auditor-selection policy, full appeal paths, and
  bond calibration.

Narrow validation commands:
- `cargo test -p tensor_vm --lib p2p::wire -- --nocapture`
- `cargo test -p tensor_vm --lib node::pending_payloads -- --nocapture`
- `cargo test -p tensor_vm --lib node::payload_application -- --nocapture`
- `cargo test -p tensor_vm --lib node::message_ingest -- --nocapture`
- `cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture`
- `cargo test -p tensor_vm --test tvmd_runtime network_payloads -- --nocapture`

Broad validation commands before commit:
- Passed: `cargo fmt --check --all`
- Passed: `git diff --check`
- Passed first and again after implementation: `cargo test -p tensor_vm local_testnet --release`
- Passed: `cargo test -p tensor_vm` (341 lib tests, local CPU Compose integration, 8 `tvmd_cli` tests,
  29 `tvmd_runtime` tests)
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `cargo test --workspace --release` (14 `experiments` tests, 341 `tensor_vm` lib tests, local
  CPU Compose integration, 8 `tvmd_cli` tests, 29 `tvmd_runtime` tests, 1 `tensor_vm_explorer` lib test,
  2 explorer binary tests)
- Blocked as expected: `cargo tarpaulin --workspace --offline` failed with `error: no such command:
  tarpaulin`.

Expected observable evidence:
- A locally submitted validator audit report is published as a bounded p2p payload.
- A non-producer can ingest the report, apply it through the chain command, and persist the same
  audit-result/slash state.
- Out-of-order audit reports are queued until their assignment exists, then retried and applied.
- Runtime status reports validator audit reports submitted and network audit reports ingested/applied.

Architecture shortcut answers:
- Canonical owner: `chain` remains the owner of audit assignment, report validation, reward voiding, and
  slashing. `node` owns payload admission/retry. Runtime only observes and submits.
- Adapter callers: validator role loop, p2p gossip, RPC/status, and checkers.
- Old shortcut being removed: audit reports could only be applied by direct local chain calls in tests or
  adapters, with no shared network payload or role-owned worker path.
- Regression test that proves the shortcut is gone: a validator-role test submits a report from an
  assignment, and node-ingest tests show non-producers apply/retry audit report payloads with local producer
  disabled.
- Behavior with local synthetic block production disabled: inbound audit report payloads still apply
  through `ChainCommand::SubmitValidatorAuditReport`; missed-audit slashes still occur only on canonical
  block application.
- Behavior for producer and non-producer roles: both ingest and apply identical signed report payloads;
  only registered validators with local audit evidence submit outbound reports.
- Structured evidence source: p2p codec tests, node-ingest/retry tests, validator-role/runtime status tests,
  state-rooted audit result/slash state.
- Finality source: unchanged; signed block votes finalize blocks. Audit report admission is pre-block
  consensus state carried by the shared payload path.
- Wire-size and codec boundary: extend the existing bounded p2p/storage codec family with one audit-report
  payload; do not introduce a second block/job/receipt/attestation codec.

Out of scope:
- Full auditor-selection protocol; assignments still identify the audited validator, and any registered
  validator may submit one local report in this reference slice.
- Full transcript disputes, appeal-safe slashing, challenge gossip, and bond/gain-from-fraud calibration.
- Docker readiness while gateway `/health` remains blocked.

Split trigger:
Split if adding a separate audit gossip topic cascades into public evidence topic-count updates; otherwise
reuse the attestation gossip topic to keep this as one feature-sized runtime/payload slice.

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
- Iteration 28 feature commit: `99d819c` (`Add validator audit reward slashing`).
- Iteration 28 push result: `8236dfa..99d819c  main -> main` on `origin/main`.

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
