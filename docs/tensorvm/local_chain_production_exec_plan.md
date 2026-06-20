# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 24, per-op `F_p` conformance vector gate.
- Current status: Gate 0 for the Iteration 24 resumed work passed first on June 20, 2026. Iteration 24 is
  implemented and validated locally except for the standing missing `cargo-tarpaulin` tool blocker; commit
  and push are in progress.
- Latest completed feature: Iteration 23, delayed receipt reward finality, is implemented and pushed as
  `388c4d6` (`Delay receipt reward finality`). Miner and validator receipt rewards now settle into
  state-rooted pending claims, block inclusion delays claim maturity through the reward-settlement delay
  plus challenge window, challenge success voids pending receipt rewards, and release only credits mature
  non-void claims.
- Previous completed feature: Iteration 22, content-addressed Tensor IR foundation, is implemented and pushed
  as `8e17789` (`Add content addressed tensor IR`). The crate now has a chain-owned Tensor IR foundation
  with canonical JSON graph IDs, frozen op-registry metadata, structural validation, Tier-C consensus
  gating, canonical TensorOp and LinearTrainingStep graph constructors, and current receipt `program_hash`
  binding to IR `graph_id`.
- Earlier completed feature: Iteration 21, delayed proposer rewards and local block-check challenges, is
  implemented and pushed as `62e5600` (`Add delayed proposer reward challenges`). `TensorBlock` now carries
  proposer reward amounts, rewarded block production creates `PendingProposerReward` records, matured
  proposer rewards are released only after the challenge window, and successful local block-check
  challenges void pending proposer rewards, pay challengers, quarantine affected receipts, and throttle
  proposers.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit and push Iteration 24, then start graph-body propagation/storage or the next
  highest-priority v0 gap from the readiness matrix.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR validation |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`, Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, and block votes | Extend only through shared codecs/events when IR records become networked |
| Canonical useful-verification block validity | Partially complete | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and local check challenges | Remaining: full transcript disputes, network/RPC challenge propagation, exact replayable snapshots, live validator proposer networking |
| Tensor IR graph language | Foundation implemented/pushed | `8e17789`; `ir::TensorGraph`, canonical JSON, `graph_id`, frozen op registry, structural validation, Tier-C consensus gating, current TensorOp/LinearTrainingStep `program_hash` binding to IR graph IDs, and Iteration 24 current-job conformance vectors | Next add graph-body propagation/storage and generic IR execution |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Iteration 24 added deterministic vectors for current executable admitted ops, a stable suite hash, CPU backend pass reporting, default CUDA non-admission, and TensorOp/LinearTrainingStep verifier gates | Remaining: broader executable admitted registry vectors, generic graph interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add after IR/conformance and remaining block validity gaps |
| Economics and slashing invariant | Partial | Delayed proposer rewards, delayed receipt reward claims, local challenge penalties, and challenge voiding for pending receipt claims exist; hard miner/validator bond invariant not complete | Add slashable bond/audit/data-withholding invariant slice |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 24: Per-Op `F_p` Conformance Vector Gate

Feature capability:
Add a deterministic conformance vector suite for the admitted reference `F_p` ops used by current v0
canonical jobs, expose a stable suite hash, require the CPU reference backend to pass those vectors, and
gate receipt verification on the relevant job program's conformance profile. CUDA/GPU admission remains
gated unless compiled kernels can pass the same suite.

Readiness requirements covered:
- `upow.md` §3.3 and §16 determinism conformance suite for per-op input-to-output vectors.
- `upow.md` §4.8 canonical `F_p` semantics for exact admitted ops and Tier-C consensus gating.
- `goal.md` known gap: per-op `F_p` conformance test-vector suite gating receipt acceptance.
- `mvp_spec.md` §4.1, §8.3, §32, and Acceptance Criteria 1, 2, 8, 10 deterministic reference runtime
  evidence.

Files/modules likely touched:
- `crates/tensor_vm/src/conformance.rs` (new) and `crates/tensor_vm/src/lib.rs`
- `crates/tensor_vm/src/runtime.rs`, `crates/tensor_vm/src/verify.rs`, and focused tests
- `docs/tensorvm/coverage_matrix.md`, `docs/tensorvm/implementation_status.md`,
  `docs/tensorvm/tarpaulin_report.md`, and this plan

Parallel subagents to run:
- `readiness-mapper`: map conformance vectors to v0 readiness and docs/status boundaries.
- `tensorvm-codebase-explorer`: identify the safest production API and admission points.
- `tensorvm-test-coverage-explorer`: identify existing tests and missing conformance/gating coverage.

Parallelizable implementation workstreams:
- Parent/integrator owns all file edits because the change crosses runtime, verification, exports, and docs.
- Subagents remain read-only reviewers.

Tests/checkers/docs to add or update:
- Unit tests for vector determinism, suite-hash stability, vector coverage of admitted current-job ops,
  CPU backend pass, default-build CUDA non-admission, and verifier rejection when required conformance is
  missing.
- Runtime tests should prove CUDA admission is not claimed in default builds.
- Status, coverage, Tarpaulin, and spec progress docs updated without claiming full arbitrary-graph
  execution or all Tier-B/Tier-C vector coverage.

Narrow validation commands:
- `cargo test -p tensor_vm --lib conformance -- --nocapture`
- `cargo test -p tensor_vm --lib runtime -- --nocapture`
- `cargo test -p tensor_vm --lib verify::tests -- --nocapture`
- `cargo test -p tensor_vm --lib jobs -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `git diff --check`
- `cargo tarpaulin --workspace --offline` (expected blocked here unless `cargo-tarpaulin` is installed)

Implementation summary:
- Added `crates/tensor_vm/src/conformance.rs` with deterministic `F_p` vectors for current executable
  admitted ops used by TensorOp and LinearTrainingStep: `add`, `sub`, `mul`, `scalar_mul`, `transpose`,
  `reduce_sum`, `matmul`, and `mse_loss`.
- Added a stable conformance suite hash and `ConformanceProfile` pass set.
- Added `runtime::backend_conformance_profile`; CPU reference reports the conformance profile, while the
  default non-CUDA build rejects GPU conformance with `cuda kernels not compiled`.
- Added TensorOp and LinearTrainingStep verifier gates. The default verifier path derives the CPU reference
  profile, and explicit profile helpers reject otherwise-valid receipts when the suite is unavailable or a
  required op is missing.
- Updated status, coverage, completion-audit, Tarpaulin, and `upow.md` docs with the current-job scope and
  remaining broader conformance gaps.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and `local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo test -p tensor_vm --lib conformance -- --nocapture` passed with 6 filtered tests including
  conformance, runtime, and verifier conformance-profile tests.
- `cargo test -p tensor_vm --lib runtime -- --nocapture` passed with 14 filtered runtime/profile/network
  tests.
- `cargo test -p tensor_vm --lib verify::tests -- --nocapture` passed with 13 verifier tests.
- `cargo test -p tensor_vm --lib jobs -- --nocapture` passed with 11 filtered job/localnet/role tests.
- `cargo fmt --check --all` passed.
- `cargo test -p tensor_vm --lib` passed with 332 library tests.
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm` passed with 332 library tests, 1 local CPU Compose integration test,
  8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed with 14 `experiments`, 332 `tensor_vm`, 1 local CPU Compose,
  8 `tvmd_cli`, 28 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and
  doc-test targets.
- `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because this environment does not have
  the `cargo-tarpaulin` subcommand installed.

Expected observable evidence:
- A stable conformance suite hash commits the exact vector set and expected outputs.
- The CPU reference backend passes the suite before its receipts can verify as valid.
- TensorOp and LinearTrainingStep verification fail if their required conformance profile is unavailable.
- Tier-C/deferred ops remain present as vocabulary but cannot satisfy consensus conformance.

Out of scope:
- Generic arbitrary-IR graph execution.
- Exhaustive vectors for every registry vocabulary op not yet implemented by the reference runtime.
- CUDA kernel implementation or GPU-readiness claims in default builds.
- Transcendental/fixed-point approximation references for Tier-C ops.

Split trigger:
Split if generic graph execution is required before the current canonical TensorOp and LinearTrainingStep
jobs can be conformance-gated.

Architecture shortcut answers:
- Canonical owner: `conformance`, `runtime`, and `verify` own conformance vectors, backend checks, and
  receipt-verification gating.
- Adapter callers: miner/validator/runtime paths can ask whether a backend/profile passes; adapters do not
  decide vector contents or bypass failed conformance.
- Old shortcut being removed: accepting current receipts based only on ad hoc job execution and program
  hash equality, without an explicit per-op deterministic vector gate.
- Regression test that proves the shortcut is gone: verifier tests reject otherwise-valid receipts when
  the required conformance profile is unavailable or altered.
- Behavior with local synthetic block production disabled: unchanged; conformance checks are pure runtime
  validation and receipt-verification prerequisites.
- Behavior for producer and non-producer roles: both use the same reference vector suite and suite hash.
- Structured evidence source: unit tests and docs/status; no shell-only conformance assertion.
- Finality source: unchanged, signed validator block votes through `SubmitBlockVote`.
- Wire-size and codec boundary: no P2P/storage codec change in this slice; conformance is local
  admission/verification metadata.

### Iteration 23: Delayed Receipt Reward Finality

Feature capability:
Replace immediate miner/validator receipt reward crediting with consensus-visible pending receipt reward
claims. Receipt settlement creates claims, block inclusion extends their maturity to
`reward_settlement_delay + challenge_window`, release credits only matured non-void claims, and successful
block-check challenges void affected pending receipt rewards before spendability.

Readiness requirements covered:
- `upow.md` §12 reward finality and challenge safety for verifier-dependent rewards.
- `mvp_spec.md` §19, §20.4, §20.7, §25, and §28 delayed reward settlement after challenge windows.
- `local_chain_production_readiness.md` reward evidence boundary: pending live reward claims are distinct
  from mature released balances.

Files/modules touched:
- `crates/tensor_vm/src/chain/state.rs`, `settlement.rs`, `commands.rs`, `blocks.rs`, `challenges.rs`,
  `roots.rs`, `genesis.rs`, and `test_helpers.rs`
- `crates/tensor_vm/src/storage/chain_state.rs`
- `crates/tensor_vm/src/rpc/explorer.rs`, `crates/tensor_vm_explorer/src/lib.rs`
- `crates/tensor_vm/src/app/local_testnet_seed.rs`, `crates/tensor_vm/src/testnet/local_harness.rs`
- local-testnet checker/tests and docs

Parallel subagents run:
- `readiness-mapper`: mapped reward-finality readiness/docs risks and flagged pending-vs-released evidence.
- `tensorvm-codebase-explorer`: checked delayed reward code paths and flagged inclusion-height maturity,
  missing summary field, missing production release caller, and voided-claim pruning.
- `tensorvm-test-coverage-explorer`: mapped focused tests and flagged missing non-empty storage roundtrip
  and voided-release coverage.

Parallelizable implementation workstreams:
- Parent/integrator owns all file edits because the change spans shared chain state, storage, RPC summary
  shape, checker script evidence, and docs.
- Subagents remain read-only reviewers.

Tests/checkers/docs added or updated:
- Settlement tests assert pending claims and mature release.
- Block tests assert receipt reward maturity is anchored to inclusion height.
- Challenge tests assert pending receipt rewards are voided and matured voided claims are pruned without
  crediting balances.
- Storage tests roundtrip non-empty pending receipt reward claims.
- Local seed/CLI/checker tests distinguish pending claims from spendable balances.
- Status, coverage, readiness, Tarpaulin, and spec docs updated.

Narrow validation commands:
- `cargo test -p tensor_vm --lib produced_blocks_delay_receipt_rewards_from_inclusion_height -- --nocapture`
- `cargo test -p tensor_vm --lib block_check_challenge_voids_pending_reward_and_throttles_proposer -- --nocapture`
- `cargo test -p tensor_vm --lib chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture`
- `cargo test -p tensor_vm --test tvmd_cli local_testnet_service_gateway_does_not_produce_local_blocks -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `git diff --check`
- `cargo tarpaulin --workspace --offline` (expected blocked here unless `cargo-tarpaulin` is installed)

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and `local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo test -p tensor_vm --lib produced_blocks_delay_receipt_rewards_from_inclusion_height -- --nocapture`
  passed.
- `cargo test -p tensor_vm --lib block_check_challenge_voids_pending_reward_and_throttles_proposer -- --nocapture`
  passed.
- `cargo test -p tensor_vm --lib chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`
  passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture`
  passed.
- `cargo test -p tensor_vm --test tvmd_cli local_testnet_service_gateway_does_not_produce_local_blocks -- --nocapture`
  passed.
- `cargo fmt --check --all` passed.
- `cargo test -p tensor_vm --lib chain::tests -- --nocapture` passed with 48 focused chain tests.
- `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture` passed with 2 storage tests.
- `cargo test -p tensor_vm --lib rpc::tests -- --nocapture` passed with 21 RPC tests.
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm` passed with 326 library tests, 1 local CPU Compose integration test,
  8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed with 14 `experiments`, 326 `tensor_vm`, 1 local CPU Compose,
  8 `tvmd_cli`, 28 `tvmd_runtime`, 3 `tensor_vm_explorer`, and doc-test targets.
- `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because this environment does not have
  the `cargo-tarpaulin` subcommand installed.

Push evidence:
- Feature commit: `388c4d6` (`Delay receipt reward finality`).
- Remote/branch: `origin/main`.
- Push result: `c08f340..388c4d6  main -> main`.

Expected observable evidence:
- `SettleEpoch` emits `ReceiptRewardPending` events and leaves miner/validator balances unchanged.
- `ReleaseMaturedReceiptRewards` credits only non-void mature claims.
- Selected receipt inclusion pushes claim maturity to the inclusion block plus reward-settlement delay and
  challenge window.
- Pending receipt rewards are committed in state roots and persisted across node-store roundtrips.
- Local checker evidence uses pending receipt reward growth rather than immediate spendable balance growth.

Out of scope:
- Hard stake slashing for invalid receipts/attestations.
- Network/RPC challenge propagation.
- Automatic runtime release cadence for matured claims after long challenge windows.
- Full clawback of already released rewards; inclusion-height maturity prevents release before the local
  block-check challenge window for included receipts.

Split trigger:
Split if reward finality requires broad transaction/RPC surfaces or a full challenge protocol migration.

Architecture shortcut answers:
- Canonical owner: `chain::settlement`, `chain::blocks`, `chain::commands`, `chain::challenges`, and
  `chain::state` own receipt reward claim creation, inclusion maturity, release, challenge voiding, and
  state-rooted persistence.
- Adapter callers: local seed, explorer/RPC, and checker paths only observe pending/released reward state;
  they do not decide reward finality.
- Old shortcut being removed: immediate spendable miner/validator reward crediting at receipt settlement.
- Regression test that proves the shortcut is gone: settlement and command tests assert pending claims and
  zero balances before maturity; challenge tests assert voided claims do not release.
- Behavior with local synthetic block production disabled: unchanged; delayed reward state transitions are
  pure chain commands.
- Behavior for producer and non-producer roles: both observe the same state-rooted pending receipt reward
  claims and release semantics after syncing accepted chain state.
- Structured evidence source: chain/storage/RPC/checker tests and docs; no shell-only reward workaround.
- Finality source: unchanged, signed validator block votes through `SubmitBlockVote`.
- Wire-size and codec boundary: no consensus P2P payload format change; state storage encoding gains the
  pending receipt reward map and explorer JSON adds a typed summary count.

### Iteration 22: Content-Addressed Tensor IR Foundation

Feature capability:
Implement the v0 Tensor IR foundation from `upow.md` §4: typed tensor specs, refs, ops, graph outputs,
frozen registry metadata, canonical JSON encoding, `graph_id = SHA256(canonical_json(graph))`, and
structural validation for admitted v0 ops. Add light constructors for the canonical TensorOp matmul and
LinearTrainingStep graphs so current fixed jobs can be tied to the IR without replacing every receipt path
in one risky step.

Readiness requirements covered:
- `upow.md` §4 value model, refs, `Op`, `Graph`, canonical encoding, structural validity, frozen op
  registry, and canonical v0 jobs.
- `goal.md` known gap: "Full content-addressed Tensor IR graph language, frozen op registry, canonical
  encoding, `graph_id`, and structural validity".
- `mvp_spec.md` §8.4 canonical program hashing and §32 local reference runtime requirements.

Files/modules likely touched:
- `crates/tensor_vm/src/ir.rs` (new)
- `crates/tensor_vm/src/lib.rs`
- `crates/tensor_vm/src/jobs.rs` and/or `crates/tensor_vm/src/vm.rs` for light graph constructors or
  program-hash binding
- `docs/tensorvm/coverage_matrix.md`
- `docs/tensorvm/implementation_status.md`
- `docs/tensorvm/tarpaulin_report.md`
- `docs/tensorvm/local_chain_production_exec_plan.md`

Parallel subagents to run:
- `readiness-mapper`: map the IR foundation to v0 readiness, docs, and shortcut risks.
- `tensorvm-codebase-explorer`: identify current program-hash/job coupling and the safest API shape.
- `tensorvm-test-coverage-explorer`: identify focused tests and validation commands.

Parallelizable implementation workstreams:
- Parent/integrator owns all file edits in this worktree.
- Read-only explorers run in parallel. No writer subagents are used because the likely changes converge on
  shared job/runtime/docs files.

Tests/checkers/docs to add or update:
- IR unit tests for canonical JSON stability, graph-id determinism, invalid op refs, duplicate names,
  unknown ops, Tier-C consensus rejection, shape/type mismatch rejection, admitted matmul validation, and
  canonical LinearTrainingStep graph construction.
- Existing job/program-hash tests updated only if the IR graph id replaces an old ad hoc program hash for
  current primitives.
- Update implementation status, coverage matrix, Tarpaulin report, and this plan.

Narrow validation commands:
- `cargo fmt --check --all`
- `cargo test -p tensor_vm --lib ir -- --nocapture`
- `cargo test -p tensor_vm --lib jobs -- --nocapture`
- `cargo test -p tensor_vm --lib vm -- --nocapture`

Broad validation commands before commit:
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo test --workspace --release`
- `git diff --check`
- `cargo tarpaulin --workspace --offline` (expected blocked here unless `cargo-tarpaulin` is installed)

Expected observable evidence:
- `TensorGraph::graph_id()` is content-addressed over canonical JSON.
- The frozen registry exposes v0-admitted ops and carries Tier-C vocabulary while rejecting Tier-C consensus
  admission.
- Structural validation rejects non-topological refs, bad output indexes, duplicate graph names, invalid
  kwargs, arity errors, and declared shape/type mismatches.
- Canonical TensorOp and LinearTrainingStep graph constructors validate and produce stable graph ids.

Implementation summary:
- Added `crates/tensor_vm/src/ir.rs` with `TensorGraph`, typed tensor/param/ref/op/output structs,
  canonical JSON, graph IDs, frozen op registry metadata, structural validation, and v0 consensus admission
  gating.
- Added canonical graph constructors for TensorOp matmul and LinearTrainingStep.
- Updated `MatmulJob::program_hash()` and `LinearTrainingStepJob::program_hash()` to return validated IR
  graph IDs, so current receipts bind to content-addressed graph identity without changing wire payload
  shapes.
- Updated public crate exports and status/coverage/Tarpaulin docs.

Out of scope:
- Full runtime execution from arbitrary IR graphs.
- Per-op conformance vector suite and CUDA admission gating.
- Networked program fetch/storage for arbitrary graph bodies.
- Interactive fraud-proof transcript games.
- Tier-C consensus admission.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and `local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo fmt --check --all` passed.
- `cargo test -p tensor_vm --lib ir -- --nocapture` passed with 40 filtered tests including 5 IR tests.
- `cargo test -p tensor_vm --lib jobs -- --nocapture` passed with 10 filtered tests.
- `cargo test -p tensor_vm --lib verify::tests -- --nocapture` passed with 11 filtered verifier tests.
- `cargo test -p tensor_vm --lib` passed with 325 library tests.
- `cargo test -p tensor_vm` passed with 325 library tests, 1 local CPU Compose integration test,
  8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test --workspace --release` passed with 14 `experiments`, 325 `tensor_vm`, 1 local CPU Compose,
  8 `tvmd_cli`, 28 `tvmd_runtime`, 3 `tensor_vm_explorer`, and doc-test targets.
- `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because this environment does not have
  the `cargo-tarpaulin` subcommand installed.

Push evidence:
- Feature commit: `8e17789` (`Add content addressed tensor IR`).
- Remote/branch: `origin/main`.
- Push result: `0f2d65c..8e17789  main -> main`.

Split trigger:
Split smaller if replacing existing receipt `program_hash` semantics forces broad codec/storage migrations
or breaks local-testnet gates. In that case, land the standalone IR foundation first and defer receipt
format migration to a follow-up.

Architecture shortcut answers:
- Canonical owner: `ir` owns graph structure, registry metadata, canonical encoding, graph ids, and
  structural validation. `jobs` may expose current primitive graph constructors.
- Adapter callers: miner/validator/runtime/job paths may ask for graph ids or validation, but adapters do
  not decide whether an op is consensus-admitted.
- Old shortcut being removed: fixed job program hashes without a self-describing content-addressed IR graph
  as the program identity.
- Regression test that proves the shortcut is gone: canonical TensorOp and LinearTrainingStep constructors
  produce stable validated `graph_id`s, and malformed graphs cannot be accepted as consensus-admitted.
- Behavior with local synthetic block production disabled: unchanged; graph validation is pure and does not
  synthesize jobs, receipts, attestations, blocks, or votes.
- Behavior for producer and non-producer roles: unchanged; both roles derive identical graph ids from the
  same canonical graph body.
- Structured evidence source: unit tests plus implementation/coverage docs; no shell-only evidence.
- Finality source: unchanged, signed validator block votes through `SubmitBlockVote`.
- Wire-size and codec boundary: no P2P/storage payload change in this slice unless current job program
  hashes are updated through existing fixed-size hashes; arbitrary graph body networking is deferred.

## Recent Iterations

### Iteration 21: Delayed Proposer Rewards and Block-Check Challenges

Implemented and pushed as `62e5600` (`Add delayed proposer reward challenges`).

Summary:
- Added `TensorBlock.proposer_reward` and persisted pending proposer rewards.
- Block production and aggregate reward settlement queue proposer rewards instead of immediately crediting
  spendable balances.
- `ReleaseMaturedProposerRewards` releases unchallenged rewards only after the challenge window.
- Added local `BlockCheckChallenge` admission with signature/opening/recomputed-root validation.
- Successful challenges void pending proposer rewards, pay challengers from the pending amount, route the
  remainder to treasury, quarantine affected receipts, and throttle proposer eligibility.
- Updated storage, codec/P2P fixtures, chain roots, tests, and docs.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo check -p tensor_vm --all-targets` passed.
- Focused chain challenge/reward/proposer/storage/command/block tests passed.
- `cargo test -p tensor_vm --lib` passed with 320 tests.
- `cargo fmt --check --all` passed.
- `cargo test -p tensor_vm` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- `git diff --check` passed.
- `cargo tarpaulin --workspace --offline` was attempted and blocked because `cargo-tarpaulin` is missing.

Out of scope:
- Full verifier-transcript fraud proofs.
- Network/RPC challenge propagation.
- Hard stake slashing or full appeal flows.

### Iteration 20: Finalized-Beacon Consensus Randomness Binding

Implemented and pushed as `1f2b74d` (`Bind consensus randomness to finalized beacon`), with evidence commit
`a3784ad`. Blocks carry persisted finalized beacon rounds, state roots commit to finalized/genesis beacon
rounds, assignment and validation seeds use chain-owned finalized-beacon helpers, and block check leaves
bind the finalized beacon round/value and parent commitment. Out of scope remains full VRF/drand or
commit-reveal lifecycle.

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

## Validation Evidence

Latest current-iteration evidence:
- June 20, 2026 Gate 0 first:
  `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests and
  `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Required workflow doc check:
  `docs/tensorvm/codex_5_5_local_chain_workflow.md` is missing.
- Starting branch state:
  `## main...origin/main`.
- Starting `HEAD`: `62e5600 Add delayed proposer reward challenges`.
- Iteration 22 validation before commit:
  - `cargo fmt --check --all`: passed.
  - `cargo test -p tensor_vm --lib ir -- --nocapture`: passed.
  - `cargo test -p tensor_vm --lib jobs -- --nocapture`: passed.
  - `cargo test -p tensor_vm --lib verify::tests -- --nocapture`: passed.
  - `cargo test -p tensor_vm --lib`: 325 tests passed.
  - `cargo test -p tensor_vm`: passed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test -p tensor_vm local_testnet --release`: passed.
  - `cargo test --workspace --release`: passed.
  - `git diff --check`: passed.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 22 feature commit/push:
  - Feature commit: `8e17789` (`Add content addressed tensor IR`).
  - Push result: `0f2d65c..8e17789  main -> main` on `origin/main`.
- Iteration 23 validation before commit:
  - `cargo test -p tensor_vm local_testnet --release`: passed first and again after changes.
  - Focused delayed receipt reward, challenge, storage, local compose, and CLI tests: passed.
  - `cargo fmt --check --all`: passed.
  - `cargo test -p tensor_vm --lib chain::tests -- --nocapture`: 48 focused chain tests passed.
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`: passed.
  - `cargo test -p tensor_vm --lib rpc::tests -- --nocapture`: passed.
  - `cargo test -p tensor_vm`: passed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed.
  - `git diff --check`: passed.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 23 feature commit/push:
  - Feature commit: `388c4d6` (`Delay receipt reward finality`).
  - Push result: `c08f340..388c4d6  main -> main` on `origin/main`.

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

- Iteration 19, `232256d Add canonical block apply openings`: blocks commit to child state roots,
  Merkle-openable selected receipt/check roots, and parent/child block status evidence.
- Iteration 18, `af33fe1 Add UVPoW retarget fallback mode`: added bounded difficulty retargeting and
  explicit zero-receipt PoW-skip fallback blocks.
- Iteration 17, `d4a6182 Harden role-owned local checker evidence`: Docker checker requires positive live
  miner receipt/tensor and validator attestation counters.
- Iteration 16, `e18d5b3 Publish local jobs before role-owned work`: scheduled production publishes jobs
  only and leaves receipts/attestations to role loops before proposal.
- Iteration 15, `0d7debcd Add validator-owned block proposal tick`: validator runtime owns local proposal
  tick and publishes block payload/header/hash while finality remains explicit votes.
- Iteration 14, `1d556efa Move local producer to validator runtime`: single local timed producer moved to
  `validator-00`; miners cannot be local block producers.
- Iteration 13, `fb0feb0 Add role-owned block vote finality`: removed runtime-synthesized finality votes and
  added validator-owned block-vote submission/gossip/evidence.
- Iteration 12, `f6f9507 Add network-visible block payload admission` plus evidence `133fbcb`: replaced
  header-triggered deterministic replay with full `TensorBlock` payload gossip and strict chain admission.
- Iteration 11, `e6129d1` plus evidence `800b031`: added canonical useful-verification PoW block validity
  over deterministic settled-receipt blockspace.
- Iteration 10, `2d6609e Add remote validator tensor fetch` plus evidence `1687f86`: validators fetch
  missing receipt tensor artifacts over libp2p request-response before attesting.
- Iterations 1-9: extracted reusable node runtime state, moved network payload application and event
  drivers into reusable runtime helpers, bound role runtimes to chain identities, added role loop
  boundaries, miner receipt submission, validator attestations, and formalized the MVP core soundness
  boundary.
