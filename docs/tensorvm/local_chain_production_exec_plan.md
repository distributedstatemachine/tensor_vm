# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none. The next feature-sized slice is the next highest-priority v0 gap from the
  readiness matrix.
- Current status: Iteration 27, data-unavailability reward cancellation and miner bond slashing, is
  implemented, locally validated, and pushed as `cae45b5` (`Handle unavailable receipt rewards and
  slashing`).
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue with validator mandatory-audit slashing, broader bond calibration, or generic
  arbitrary-IR execution.

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
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, delayed challenger reward claims, local challenge penalties, challenge/unavailable-data voiding for pending receipt claims, and data-unavailability miner bond slashing exist; hard validator audit slashing and full bond calibration are not complete | Add validator mandatory-audit slashing and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Recent Iterations

### Iteration 27: Data-Unavailability Reward Cancellation and Miner Bond Slashing

Feature capability:
When an assigned validator submits canonical evidence that a receipt's required tensor data is unavailable
(`VerificationResult::Unavailable` or failed data availability), the chain marks the receipt
non-finalizable, voids any delayed pending receipt rewards for that receipt before spendability, records a
deterministic slash against the receipt miner's bond, reduces the miner's stake once, credits the slashed
amount to treasury, and exposes state-rooted evidence. This turns the prior reputation-only path into an
actual delayed-reward cancellation plus miner-bond invariant slice.

Readiness requirements covered:
- `upow.md` §9: unserved chunks make a receipt non-finalizable and put the miner bond at risk.
- `upow.md` §12.2: withholding data needed to settle/dispute causes a timeout-loss slash.
- `upow.md` §12.3: the bond/gain-from-fraud invariant must be stated and re-verified when parameters
  change.
- `mvp_spec.md` §16 and §26: unavailable data means invalid/no reward/reputation penalty now, and hard
  data-withholding stake slash is the next economics step.
- `goal.md` economics gap: add slashable miner bonds and data-withholding slashing wired to observable
  consensus state.

Files/modules likely touched:
- `crates/tensor_vm/src/chain/state.rs`
- `crates/tensor_vm/src/chain/validation.rs`
- `crates/tensor_vm/src/chain/roots.rs`
- `crates/tensor_vm/src/chain/engine.rs`
- `crates/tensor_vm/src/storage/chain_state.rs`
- `crates/tensor_vm/src/rpc` and/or typed status snapshot owners
- focused chain/storage/RPC/explorer tests
- `docs/tensorvm/upow.md`, `docs/tensorvm/local_chain_production_exec_plan.md`

Parallel subagents to run:
- `readiness-mapper`: map data-unavailability slashing to v0 economics/readiness and identify exact proof
  obligations.
- `tensorvm-codebase-explorer`: inspect chain/state/root/storage/status implementation paths for slash
  records and stake mutation.
- `tensorvm-test-coverage-explorer`: identify focused tests for unavailable attestations, duplicate slash
  prevention, storage roundtrip, state-root changes, and status evidence.

Parallelizable implementation workstreams:
- Parent/integrator owns code edits because chain state, roots, storage, and tests share types and would
  collide if split across writers.
- Read-only subagents run in parallel to challenge the implementation boundary.

Tests/checkers/docs to add or update:
- Chain attestation test proving unavailable data slashes the miner once, marks receipt unavailable, prevents
  settlement/reward, and credits treasury.
- Chain settlement test proving unavailable-data evidence voids already pending delayed receipt rewards
  before release.
- Storage snapshot roundtrip test for slash records.
- Root/status/RPC or explorer test proving slash evidence is observable and state-rooted.
- Docs/exec-plan update naming remaining economics gaps: validator audit slashing and broader invariant
  calibration.

Narrow validation commands:
- `cargo test -p tensor_vm --lib chain::tests::attestations -- --nocapture`
- `cargo test -p tensor_vm --lib chain::tests::settlement -- --nocapture`
- `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`
- `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture`
- `cargo test -p tensor_vm_explorer --lib`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo tarpaulin --workspace --offline` (expected blocked here unless `cargo-tarpaulin` is installed)

Expected observable evidence:
- An unavailable-data attestation reduces the receipt miner's stake and increases treasury by the slash
  amount exactly once per receipt.
- If a receipt reward was already pending, unavailable-data evidence marks those delayed claims void, and
  release does not credit miner or validator balances.
- The slash record is committed in state roots and persists through node-store snapshot roundtrip.
- Status/explorer surfaces report nonzero slashing evidence without requiring shell-only assertions.

Architecture shortcut answers:
- Canonical owner: `chain` owns unavailable-data slash detection, miner stake mutation, slash records,
  roots, and persistence.
- Adapter callers: validator/runtime/RPC may submit or observe attestations and slash evidence; they do not
  decide stake loss.
- Old shortcut being removed: unavailable data only marked the receipt unavailable and subtracted miner
  reputation; delayed pending rewards could still release if evidence arrived after settlement.
- Regression tests that prove the shortcut is gone: one test submits unavailable-data evidence for an
  already pending delayed receipt reward and verifies release credits nothing; another records a slash,
  reduces miner stake, credits treasury, prevents settlement, and proves duplicate unavailable evidence does
  not slash again.
- Behavior with local synthetic block production disabled: unchanged; inbound `SubmitAttestation` commands
  mutate the same canonical chain state.
- Behavior for producer and non-producer roles: both apply the same attestation command and persist the same
  slashing state after network/RPC ingest.
- Structured evidence source: chain tests, storage roundtrip, state-root/status/RPC evidence; no checker-only
  hardcoded boolean.
- Finality source: unchanged, signed block votes finalize blocks; slashing is triggered by canonical
  attestation admission and is state-rooted.
- Wire-size and codec boundary: no new P2P family; storage snapshot codec extends the existing bounded chain
  state encoding.

Out of scope:
- Validator mandatory-audit slashing.
- Interactive fraud-proof timeout games.
- Full economic calibration against measured fraud gain.
- Network/RPC challenge gossip.

Split trigger:
Split only if the slash ledger requires a broad status snapshot refactor or if changing chain-state storage
requires a migration-compatible codec redesign.

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
- Iteration 27 required Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 27 focused validation:
  - `cargo test -p tensor_vm --lib chain::tests::attestations -- --nocapture`: 6 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::settlement -- --nocapture`: 5 tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm_explorer --lib`: 1 test passed.
  - `cargo test -p tensor_vm --lib rpc::tests::routes -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::root_hashes -- --nocapture`: 2 tests passed.
  - `cargo test -p tensor_vm --lib testnet::tests::local_harness -- --nocapture`: 4 tests passed.
- Iteration 27 broad validation before feature commit:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - Final `cargo test -p tensor_vm local_testnet --release`: passed.
  - `cargo test -p tensor_vm`: passed with 335 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 335 `tensor_vm`, 1 local CPU Compose,
    8 `tvmd_cli`, 28 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and
    doc-test targets.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 27 feature commit: `cae45b5` (`Handle unavailable receipt rewards and slashing`).
- Iteration 27 push result: `9977f2c..cae45b5  main -> main` on `origin/main`.

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
