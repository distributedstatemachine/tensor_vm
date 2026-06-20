# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none. The next feature-sized slice is delayed challenge reward finality or the next
  highest-priority v0 gap from the readiness matrix.
- Current status: Iteration 25, graph-body propagation/storage, is implemented, locally validated, and
  pushed as `0363bb6` (`Store Tensor IR graph bodies`).
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: start delayed challenge reward finality, generic IR execution, or the next highest-priority
  v0 gap.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 25: `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 and again after implementation | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`, Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, and block votes | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partially complete | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and local check challenges | Remaining: full transcript disputes, network/RPC challenge propagation, exact replayable snapshots, live validator proposer networking |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job `program_hash` binding, current-job graph body state-root/storage, and P2P `RequestProgram` serving | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Remaining: broader executable admitted registry vectors, generic graph interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add after IR/conformance and remaining block validity gaps |
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, local challenge penalties, and challenge voiding for pending receipt claims exist; hard miner/validator bond invariant not complete | Add slashable bond/audit/data-withholding invariant slice |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 25: Graph-Body Propagation and Storage

Feature capability:
Store canonical Tensor IR graph bodies in chain state keyed by `graph_id` whenever current TensorOp or
LinearTrainingStep jobs are admitted, commit those bodies in the state root and node-store snapshot, and
serve canonical graph bytes through the existing `RequestProgram`/`ProgramResponse` request-response
protocol. Current fixed jobs remain the admitted execution surface; generic arbitrary-IR execution remains
out of scope.

Readiness requirements covered:
- `upow.md` §4.5: jobs reference immutable content-addressed programs by `graph_id`.
- `upow.md` §4.6 and §4.9: admitted graph bodies are structurally validated before storage/use.
- `mvp_spec.md` §8.4 and §29: canonical program hashing plus required `RequestProgram`/`ProgramResponse`
  message family.
- `goal.md` known gap: graph-body propagation/storage after the content-addressed Tensor IR foundation.

Subagents run:
- `readiness-mapper`: mapped graph-body storage/fetch to `upow.md` §4, `mvp_spec.md` §4.6/§29, docs, and
  shortcut risks.
- `tensorvm-codebase-explorer`: identified `TensorGraph::canonical_json()` as the canonical body bytes and
  recommended chain-state registration plus existing program request-response serving.
- `tensorvm-test-coverage-explorer`: mapped focused chain/storage/p2p/job tests and validation commands.

Architecture shortcut answers:
- Canonical owner: `chain::state` and job admission own the durable graph-body registry; `ir` owns graph
  canonicalization and validation.
- Adapter callers: P2P/runtime/RPC can register or serve canonical bytes already accepted by chain state;
  adapters do not decide graph validity.
- Old shortcut being removed: current nodes could reconstruct fixed-job graph bodies locally, but graph
  bodies were not stored or served as content-addressed programs.
- Regression test that proves the shortcut is gone: submitted jobs create durable program-body entries and
  P2P `RequestProgram` returns registered bytes instead of an empty placeholder response.
- Behavior with local synthetic block production disabled: unchanged; inbound network job admission still
  registers graph bodies through `ChainCommand::SubmitJob`.
- Behavior for producer and non-producer roles: both store graph bodies after accepting the same job
  payload through chain admission; producers register newly announced bodies with their P2P service.
- Structured evidence source: chain, storage, and P2P tests plus docs/status; no shell-only evidence.
- Finality source: unchanged, signed validator block votes through `SubmitBlockVote`.
- Wire-size and codec boundary: uses the existing bounded P2P `ProgramResponse` bytes field and existing
  job payload codec; no parallel block/job/receipt codec was added.

Implementation summary:
- Added `JobState::program_hash`, `JobState::tensor_ir_graph`, and `JobState::canonical_program_body`
  helpers.
- Added `ChainState.program_bodies`, state accessors, genesis/from-parts wiring, state-root commitment, and
  chain-state snapshot encoding/decoding.
- `chain::receipts::submit_job` now validates the current job graph and stores canonical graph bytes keyed
  by graph ID before accepting the job.
- `TensorVmLibp2pService` now has a program store and `register_program`; program request-response returns
  registered graph bytes for `RequestProgram`.
- New-chain announcement publishing registers graph bytes with the local P2P service before gossiping job
  payloads.
- Updated `upow.md`, coverage/status/audit/Tarpaulin docs with current-job graph body scope and remaining
  arbitrary-IR gaps.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and `local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo test -p tensor_vm --lib chain::tests -- --nocapture` passed with 48 focused chain tests.
- `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture` passed with 2 storage tests.
- `cargo test -p tensor_vm --lib p2p -- --nocapture` passed with 30 focused P2P tests.
- `cargo test -p tensor_vm --lib jobs -- --nocapture` passed with 11 filtered tests.
- `cargo fmt --check --all` passed after applying `cargo fmt --all`.
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed with 333 library tests, 1 local CPU Compose integration test, 8
  `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed with 14 `experiments`, 333 `tensor_vm`, 1 local CPU Compose,
  8 `tvmd_cli`, 28 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and
  doc-test targets.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because this environment does not have
  the `cargo-tarpaulin` subcommand installed.

Push evidence:
- Feature commit: `0363bb6` (`Store Tensor IR graph bodies`).
- Remote/branch: `origin/main`.
- Push result: `b0fe92c..0363bb6  main -> main`.

Expected observable evidence:
- Chain state exposes nonempty canonical program bytes for submitted TensorOp and LinearTrainingStep jobs.
- State roots and node-store snapshots commit and roundtrip the program-body registry.
- Existing P2P `RequestProgram` returns registered canonical graph body bytes.

Out of scope:
- Generic arbitrary-IR graph execution.
- New consensus transaction type for user-submitted arbitrary graph bodies.
- A second P2P or storage codec for graph payloads.
- CUDA kernel implementation or GPU-readiness claims.

## Recent Iterations

### Iteration 24: Per-Op `F_p` Conformance Vector Gate

Implemented and pushed as `f4d4491` (`Add Fp conformance vector gate`).

Summary:
- Added deterministic `F_p` conformance vectors for current executable admitted ops used by TensorOp and
  LinearTrainingStep: `add`, `sub`, `mul`, `scalar_mul`, `transpose`, `reduce_sum`, `matmul`, and
  `mse_loss`.
- Added a stable conformance suite hash and `ConformanceProfile`.
- CPU reference reports a passing profile; default non-CUDA builds reject GPU conformance with
  `cuda kernels not compiled`.
- TensorOp and LinearTrainingStep verification now reject otherwise-valid receipts when the required
  conformance profile is unavailable.

Validation:
- Required Gate 0 first and final Gate 0 passed.
- Focused conformance/runtime/verifier/jobs tests passed.
- `cargo fmt --check --all`, `cargo test -p tensor_vm`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --release`, and `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `94c8007..f4d4491  main -> main` on `origin/main`.

### Iteration 23: Delayed Receipt Reward Finality

Implemented and pushed as `388c4d6` (`Delay receipt reward finality`).

Summary:
- Replaced immediate miner/validator receipt reward crediting with state-rooted pending receipt reward
  claims.
- Receipt settlement creates pending claims; block inclusion extends maturity to
  `reward_settlement_delay + challenge_window`; release credits only mature non-void claims.
- Successful block-check challenges void affected pending receipt rewards before spendability.
- Storage, RPC/explorer summary, local checker evidence, tests, and docs distinguish pending claims from
  spendable balances.

Validation:
- Required Gate 0 first and final Gate 0 passed.
- Focused delayed reward, challenge, storage, local compose, and CLI tests passed.
- `cargo fmt --check --all`, `cargo test -p tensor_vm`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --release`, and `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is missing.
- Push result: `c08f340..388c4d6  main -> main` on `origin/main`.

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
- Iteration 25 required Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 25 validation before feature commit:
  - `cargo test -p tensor_vm --lib chain::tests -- --nocapture`: 48 tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib p2p -- --nocapture`: 30 tests passed.
  - `cargo test -p tensor_vm --lib jobs -- --nocapture`: 11 tests passed.
  - `cargo fmt --check --all`: passed.
  - Final `cargo test -p tensor_vm local_testnet --release`: passed.
  - `git diff --check`: passed.
  - `cargo test -p tensor_vm`: passed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 25 feature commit: `0363bb6` (`Store Tensor IR graph bodies`).
- Iteration 25 push result: `b0fe92c..0363bb6  main -> main` on `origin/main`.

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
