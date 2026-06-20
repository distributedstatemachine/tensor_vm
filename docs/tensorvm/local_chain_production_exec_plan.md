# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none; Iteration 36 is complete.
- Current status: Iteration 36 implemented, validated, committed, and pushed on June 20, 2026.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked in this environment because `cargo-tarpaulin` is not
    installed: `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: choose the next readiness slice. Standing blockers remain the missing workflow document,
  missing `cargo-tarpaulin`, and the full Docker `/health` timeout.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 36: `cargo test -p tensor_vm local_testnet --release` passed first on June 20, 2026 and again after implementation | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`, Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenge payloads | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partially complete | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots, live validator proposer networking |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job `program_hash` binding, current-job graph body state-root/storage, and P2P `RequestProgram` serving | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Remaining: broader executable admitted registry vectors, generic graph interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add after IR/conformance and remaining block validity gaps |
| Economics and slashing invariant | Partial | Delayed proposer, receipt, challenge, and credit rewards exist; block `reward_root` now binds spendable plus pending reward ledgers; block transitions now release still-mature pending claims after applying current-block delay/void effects; local challenge penalties, challenge/unavailable-data voiding for pending receipt claims, data-unavailability miner bond slashing, configured validator mandatory-audit reward delay/slashing, and network-visible validator audit reports exist; full bond calibration and appeal-safe security are not complete | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators and templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 36: Block Transitions Release Matured Rewards

Feature capability: make normal block production/admission release matured reward claims through the
chain-owned child-state transition, instead of requiring adapters or checkers to work around delayed rewards
with out-of-band release commands.

Readiness requirements covered: `upow.md` §12 and `mvp_spec.md` §20.3/§20.4/§25.5 require reward finality
to be delayed from block finality, while the local readiness gate requires mature pending claims to release
into spendable balances. Iteration 36 keeps delayed spendability but moves mature release into block
progression.

Files/modules touched: `crates/tensor_vm/src/chain/commands.rs`,
`crates/tensor_vm/src/chain/blocks.rs`, `crates/tensor_vm/src/chain/tests/attestations.rs`,
`crates/tensor_vm/src/chain/tests/rewards.rs`, `docs/tensorvm/coverage_matrix.md`,
`docs/tensorvm/implementation_status.md`, `docs/tensorvm/tarpaulin_report.md`, and this exec plan.

Parallel subagents run:
- Reward path explorer: confirmed no remaining production immediate-credit path, but identified test-only
  direct spendable-credit helpers and a matured voided proposer-claim sweep bug.
- Reward lifecycle mapper: confirmed proposer, receipt, challenge, and credit releases were explicit
  commands only; block child-state construction did not release matured claims.

Architecture shortcut answers:
- Canonical owner: `chain::commands` owns the shared matured-release semantics; `chain::blocks` invokes it
  inside canonical child-state construction for both production and block admission.
- Adapter callers: RPC/runtime/checker surfaces only observe the resulting state; they do not release
  mature claims as a workaround.
- Old shortcut being removed: delayed rewards required explicit `ReleaseMatured*Rewards` commands outside
  block progression for spendable settlement evidence.
- Regression test: a producer and peer applying the same next block both sweep a matured proposer reward
  into spendable balance without a manual release command, and a matured voided proposer claim is pruned
  without credit.
- Behavior with local synthetic block production disabled: any accepted block still runs the same
  chain-owned mature-release transition.
- Behavior for producer and non-producer roles: producers build blocks with post-release child roots;
  non-producers recompute the same reward release through `SubmitBlock`.
- Structured evidence source: `ChainState` reward ledgers and Rust chain tests, not checker-only fields.
- Finality source: unchanged stake-weighted block votes; this changes reward spendability timing once
  maturity is reached.
- Wire-size and codec boundary: no new p2p payloads or storage codecs.

Implemented locally:
- Factored proposer, receipt, challenge, and credit reward release into shared chain helpers used by the
  explicit release commands.
- `apply_block_to_parent_state` now applies the current block's receipt-inclusion delay and slashing/audit
  effects, then sweeps still-matured pending reward claims before height advancement, beacon update, and the
  new proposer reward claim.
- Matured voided proposer reward claims are now pruned without credit instead of remaining pending forever.
- Added focused tests for producer/non-producer automatic mature release and voided proposer-claim sweep.

Validation completed locally:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused tests passed: `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`,
  `cargo test -p tensor_vm --lib chain::tests::blocks -- --nocapture`,
  `cargo test -p tensor_vm --lib chain::tests::settlement -- --nocapture`,
  `cargo test -p tensor_vm --lib chain::tests::challenges -- --nocapture`,
  `cargo test -p tensor_vm --lib chain::tests::root_hashes -- --nocapture`, and
  `cargo test -p tensor_vm --lib chain::tests::attestations::mandatory_validator_audit_assignment_missed_slashes_once_on_block_apply -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, final
  `cargo test -p tensor_vm local_testnet --release`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `58da0e6` (`Release matured rewards during blocks`).
- Push result: `9e28b71..58da0e6  main -> main` on `origin/main`.

Out of scope: unified formal reward-claim object/status, replacing test-only spendable-balance helpers,
public reward-settlement evidence, full auditor-selection policy, appeal paths, and bond calibration.

### Iteration 35: Reward Root Binds Pending Reward Ledgers

Feature capability: redefine block `reward_root` as the canonical reward-finality ledger commitment for
the child state, covering spendable reward balances/treasury plus pending proposer, receipt, challenge, and
generic credit reward claims.

Readiness requirements covered: `upow.md` §12.1, `mvp_spec.md` §20.3/§20.4/§21/§25.5, and
`docs/formal/mvp_core_reward_finality_challenge_model.md` RW-002/RW-005 require delayed reward claims to
be root-bound and non-spendable until maturity. Iteration 34 made generic credits pending; this iteration
removes the block-level spendable-only `reward_root` shortcut.

Files/modules likely touched: `crates/tensor_vm/src/chain/roots.rs`,
`crates/tensor_vm/src/chain/blocks.rs`, `crates/tensor_vm/src/chain/tests/rewards.rs`,
`crates/tensor_vm/src/chain/tests/blocks.rs`, `docs/tensorvm/coverage_matrix.md`,
`docs/tensorvm/implementation_status.md`, `docs/tensorvm/tarpaulin_report.md`, and this exec plan.

Parallel subagents run:
- Readiness mapper: confirmed that `TensorBlock.reward_root` currently excludes pending reward-finality
  ledgers even though the specs/formal model assign reward root to pending claim state.
- Codebase explorer: confirmed `reward_root(&RewardState)` hashes only spendable balances/treasury and
  block production/validation use that spendable-only root; recommended keeping the old boundary, but that
  conflicts with the current reward-finality target.
- Test-coverage explorer: identified existing tests that prove the old behavior and missing tests for
  pending proposer/receipt/challenge/credit reward root binding.

Parallelizable implementation workstreams: parent/integrator owns all edits because the root function and
block validation call sites are coupled; subagents remain read-only.

Tests/checkers/docs to add or update: update the existing proposer reward test to expect pending claims in
`reward_root`; add block validation rejection for a spendable-only root; add root mutation tests proving
each pending reward ledger changes the canonical reward root; update coverage/status/tarpaulin docs.

Narrow validation commands: `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`,
`cargo test -p tensor_vm --lib chain::tests::blocks -- --nocapture`, and `cargo test -p tensor_vm --lib
chain::tests::root_hashes -- --nocapture`.

Broad validation commands before commit: `cargo fmt --check --all`, `git diff --check`, final
`cargo test -p tensor_vm local_testnet --release`, `cargo test -p tensor_vm`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace --release`, and `cargo tarpaulin --workspace
--offline` if available.

Expected observable evidence: a produced useful block with a pending proposer reward has
`block.reward_root != spendable_reward_root(rewards)` and equals the full reward ledger root; block
validation rejects a spendable-only reward root; changing any pending reward ledger changes the full reward
root without crediting spendable balances.

Architecture shortcut answers:
- Canonical owner: `chain::roots` owns canonical reward-root encoding; `chain::blocks` owns child-state
  block production and validation against that root.
- Adapter callers: CLI/RPC/status/block views only display the chain-owned block/apply outcome roots.
- Old shortcut being removed: block `reward_root` commits only spendable `RewardState`, while pending
  reward-finality claims are visible only through `state_root`.
- Regression test: a block whose `reward_root` is the old spendable-only root is rejected with
  `block reward root mismatch`; pending proposer/receipt/challenge/credit mutations change reward root.
- Behavior with local synthetic block production disabled: inbound or locally produced blocks validate
  against the same child-state reward ledger root; no synthetic path is needed.
- Behavior for producer and non-producer roles: producers emit full reward ledger roots; non-producers
  recompute and reject old spendable-only roots through the shared chain validation path.
- Structured evidence source: typed root functions and Rust chain tests, not checker-only fields.
- Finality source: unchanged stake-weighted block votes; this feature changes reward commitment semantics,
  not finality voting.
- Wire-size and codec boundary: no new p2p payloads or storage codecs; existing `TensorBlock.reward_root`
  field carries the strengthened canonical root.

Out of scope: introducing a unified formal `RewardClaim` object with block hash/evidence-root fields,
automatic reward release on every block, new challenge resolution semantics, public reward-settlement
evidence, and full bond calibration.

Split trigger: if changing the root signature forces broad public API/storage migration, split API cleanup
from the consensus root semantic change.

Implemented locally:
- Renamed the old spendable-only reward commitment helper to `spendable_reward_root`.
- Redefined canonical block `reward_root` as a full child-state reward ledger root over spendable rewards,
  pending proposer rewards, pending receipt rewards, pending challenge rewards, and pending credit rewards.
- Updated block production, block validation/apply outcome, and parent snapshots to use the full reward
  ledger root.
- Added tests proving a block with the old spendable-only root is rejected and mutations to each pending
  reward ledger change the full reward root while balances remain non-spendable.
- Updated coverage/status docs to record the stronger reward-root boundary while keeping unified formal
  reward claims and full economics calibration out of scope.

Validation completed locally:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused tests passed: `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`,
  `cargo test -p tensor_vm --lib chain::tests::blocks -- --nocapture`, and `cargo test -p tensor_vm --lib
  chain::tests::root_hashes -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, final
  `cargo test -p tensor_vm local_testnet --release`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `f53700c` (`Bind reward root to pending claims`).
- Push result: `36c7fbd..f53700c  main -> main` on `origin/main`.

### Iteration 34: Delayed Generic Reward Credits

Feature capability: remove the remaining immediate spendable `CreditReward` bypass by making generic/faucet
reward credits enter a state-rooted pending reward-credit ledger before any spendable `RewardState` balance
is credited.

Readiness requirements covered: `upow.md` §12 and `mvp_spec.md` §20.3/§20.4/§25.5 require reward
finality to remain delayed after block/order finality; the local readiness checker also requires pending
claims to mature into spendable balances rather than immediate bounty/faucet-style credits being confused
with protocol settlement.

Files/modules likely touched: `chain/state.rs`, `chain/engine.rs`, `chain/commands.rs`, `chain/roots.rs`,
`chain/genesis.rs`, `chain.rs`, `storage/chain_state.rs`, `rpc/mutations.rs`, `rpc/explorer.rs`,
`app/status.rs`, `testnet/local_harness.rs`, `tensor_vm_explorer/src/lib.rs`, reward/RPC/storage tests,
coverage/status/tarpaulin docs, and this exec plan.

Parallel subagents run:
- Readiness mapper: identified `CreditReward` and faucet/RPC as the remaining immediate-credit workaround;
  canonical owner must be `chain`, not RPC/faucet glue.
- Codebase explorer: mapped all call sites, storage/root implications, and recommended a distinct pending
  reward map because proposer rewards are keyed by height and cannot safely hold generic credits.
- Test-coverage explorer: identified existing delayed proposer/receipt/challenge coverage and missing tests
  proving `CreditReward` no longer bypasses maturity.

Parallelizable implementation workstreams: parent/integrator owns all edits because the feature changes the
same chain state, storage codec, and tests; subagents remain read-only.

Tests/checkers/docs to add or update: focused command tests for pending credit creation and mature release,
RPC faucet tests for pending/non-spendable credit, storage roundtrip/root tests for the new ledger,
explorer/status count tests, and coverage/status docs clarifying that generic reward credits are delayed.

Narrow validation commands: `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture`,
`cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`, `cargo test -p tensor_vm --lib
rpc::tests -- --nocapture`, `cargo test -p tensor_vm --test tvmd_runtime runtime_persistence --
--nocapture`, and `cargo test -p tensor_vm_explorer --lib -- --nocapture`.

Broad validation commands before commit: `cargo fmt --check --all`, `git diff --check`, final
`cargo test -p tensor_vm local_testnet --release`, `cargo test -p tensor_vm`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace --release`, and `cargo tarpaulin --workspace
--offline` if available.

Expected observable evidence: `CreditReward` emits a pending event and leaves spendable reward balance zero;
`ReleaseMaturedCreditRewards` credits spendable balances only after `claimable_at_height`; faucet RPC
returns a pending claim; state roots and node-store snapshots include the new pending ledger.

Architecture shortcut answers:
- Canonical owner: `chain` state/commands/roots own pending credit rewards and maturity release.
- Adapter callers: RPC faucet, tests, explorer/status, and storage helpers call or observe the chain-owned
  command path only.
- Old shortcut being removed: `ChainCommand::CreditReward` immediately mutates spendable `RewardState`.
- Regression test: direct command and RPC tests prove a credited reward remains pending/non-spendable until
  the maturity release command runs.
- Behavior with local synthetic block production disabled: generic reward credits still enter pending state
  through the chain command; no synthetic production path is required for release tests.
- Behavior for producer and non-producer roles: unchanged; any node applying the command observes the same
  deterministic pending ledger and release rule.
- Structured evidence source: typed `PendingCreditReward`, `ChainEvent::CreditRewardPending`, state-root and
  storage roundtrip tests, explorer/status counts.
- Finality source: unchanged stake-weighted block votes; this feature changes reward spendability only.
- Wire-size and codec boundary: no new p2p wire payloads; storage codec is extended through the shared
  chain-state encoder/decoder.

Out of scope: unifying all reward classes into one formal claim object, changing block `reward_root`
naming, automatic release during every block transition, public-run reward evidence, and full bond
calibration.

Split trigger: if storage migration/backward compatibility requires a schema-version overhaul, split that
into a follow-up and keep this iteration to current-state delayed semantics plus tests.

Implemented locally:
- Added `PendingCreditReward` to chain state with a state-root commitment, storage snapshot roundtrip, and
  explorer/status summary counts.
- Changed `ChainCommand::CreditReward` to enqueue a pending credit reward and emit
  `CreditRewardPending` instead of immediately mutating spendable `RewardState`.
- Added `ChainCommand::ReleaseMaturedCreditRewards`/`Chain::release_matured_credit_rewards`, which credits
  spendable balances only after the claim's maturity height and emits `CreditRewardReleased` plus
  `RewardCredited`.
- Updated faucet RPC to return pending claim metadata and persist pending credits rather than spendable
  balances.
- Updated coverage/status docs to record that generic/faucet credits no longer bypass reward maturity,
  while leaving full formal reward-claim unification and automatic release policy out of scope.

Validation completed locally:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused tests passed: `cargo test -p tensor_vm --lib chain::tests::commands -- --nocapture`,
  `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`, `cargo test -p tensor_vm --lib
  rpc::tests -- --nocapture`, `cargo test -p tensor_vm --test tvmd_runtime runtime_persistence --
  --nocapture`, and `cargo test -p tensor_vm_explorer --lib -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  final `cargo test -p tensor_vm local_testnet --release`, `cargo clippy --workspace --all-targets --
  -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `63052e0` (`Delay generic reward credits`).
- Push result: `369609e..63052e0  main -> main` on `origin/main`.

## Recent Iterations

### Iteration 33: Tier-B Verifier Coverage Contract And Index-Op Gating

Implemented and pushed as `84b1098` (`Classify Tensor IR verifier coverage`).

Feature capability: add a machine-readable verifier/soundness classification to the frozen Tensor IR
registry so the local reference explicitly distinguishes Freivalds-covered Tier-A ops, random-linear
Tier-B relations, exact deterministic Tier-B replay checks, deferred canonical-reference ops, and
index-consistency-gated ops. This narrows the `upow.md` §7 TODO without claiming generic arbitrary-IR
execution or admitting `gather`/`scatter`/`embedding`.

Readiness requirements covered: `upow.md` §4.7-§4.9 and §7 require explicit admitted-op and verifier
coverage boundaries; `mvp_spec.md` §14 and §35 criteria 8-9 require LinearTrainingStep random-linear
error/update checks and sparse-corruption rejection; `goal.md` names L2 random-linear coverage enumeration
and index-consistency handling as a known v0 gap.

Files/modules likely touched: `crates/tensor_vm/src/ir.rs`, possibly `crates/tensor_vm/src/lib.rs`,
`crates/tensor_vm/src/verify.rs` tests, `docs/tensorvm/upow.md`, `docs/tensorvm/coverage_matrix.md`,
`docs/tensorvm/implementation_status.md`, `docs/tensorvm/tarpaulin_report.md`, and this exec plan.

Parallel subagents: readiness mapper, codebase explorer, and test-coverage explorer launched in parallel
before implementation.

Tests/checkers/docs to add or update: IR registry classification tests for every frozen op,
index-consistency gating tests for `gather`/`scatter`/`embedding`, focused LinearTrainingStep
random-linear report coverage if needed, coverage/status docs, and the `upow.md` TODO language.

Validation: focused `ir`, `verify`, and `conformance` library tests; `cargo fmt --check --all`,
`git diff --check`, final Gate 0, `cargo test -p tensor_vm`, clippy, release workspace tests, and
tarpaulin if available.

Architecture shortcut answers:
- Canonical owner: Tensor IR registry metadata in `ir.rs` and existing verifier functions in `verify.rs`;
  consensus admission still flows through `TensorGraph::validate_for_consensus`.
- Adapter callers: current job/conformance gates and docs/status evidence; no shell/checker source of
  truth.
- Old shortcut being removed: Tier-B and index-op verifier status is implicit in prose and
  `consensus_admitted` booleans instead of an executable coverage contract.
- Regression test: every frozen op must have an explicit verifier class; admitted ops cannot use deferred
  or index-consistency-required verifier classes; `gather`, `scatter`, and `embedding` remain
  non-admitted.
- Synthetic production disabled: verifier classification is independent of local synthetic jobs and still
  guards graph validation/conformance.
- Producer/non-producer behavior: unchanged; this slice affects receipt/graph validity metadata, not role
  production policy.
- Structured evidence source: typed `OpSpec`/registry fields and focused Rust tests.
- Finality source: unchanged stake-weighted block votes; no finality or block admission behavior changes.
- Wire-size and codec boundary: unchanged; no new wire payloads or unbounded reads.

Out of scope: generic arbitrary-IR job admission/execution, admitting `gather`/`scatter`/`embedding`,
designing index-consistency proofs, fraud-proof bisection, CUDA coverage for the broader registry,
public deployment evidence, or Docker full-gate rerun.

Implemented locally:
- Added `IrVerificationClass` to each frozen `OpSpec` so op verifier coverage is executable metadata
  instead of prose.
- Added non-admitted `scatter` vocabulary alongside `gather` and `embedding`; all three are explicitly
  classified as `IndexConsistencyRequired`.
- Added registry tests proving every frozen op has a verifier class, admitted ops do not use deferred
  verifier classes, Tier-A admitted ops use full Freivalds, selected Tier-B relations are classified as
  random-linear, and index ops remain non-admitted.
- Updated `upow.md`, coverage matrix, implementation status, and tarpaulin report while preserving the
  boundary that generic arbitrary-IR execution and full Tier-B verifier coverage remain incomplete.

Validation completed locally:
- Required Gate 0 first and final Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.
- Focused tests passed: `cargo test -p tensor_vm --lib ir::tests -- --nocapture`, `cargo test -p
  tensor_vm --lib conformance::tests -- --nocapture`, and `cargo test -p tensor_vm --lib
  linear_training_verifier -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.

### Iteration 32: Validator Proposer Settled-Work Readiness Evidence

Implemented and pushed as `26e3e25` (`Move validator proposals into role tick`).

Feature capability: validator proposer runtime/status/checker evidence distinguishes raw settled-receipt
visibility from settled receipts that have local tensor artifacts and validator attestations available before
useful block proposal. This tightens the remaining proposer-networking gap without moving consensus logic
out of `chain` or claiming the full Docker proposer gate is complete.

Readiness requirements covered: `mvp_spec.md` §4.6/§20.5 and readiness gap 1 require validators to build
blocks from accepted state and canonical blockspace, while the checker must avoid overclaiming hidden
service-owned orchestration. Required Gate 0 passed first with
`cargo test -p tensor_vm local_testnet --release`.

Files/modules likely touched: `app/runtime_production.rs`, `app/runtime_validator.rs`,
`app/validator_role.rs`, `node/runtime_state.rs`, `app/runtime_status_snapshot.rs`,
`app/runtime_status.rs`, `app/status.rs`,
`deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`, `tests/tvmd_runtime/*`,
`tests/local_cpu_compose.rs`, and readiness/status docs.

Parallel subagents: code-path explorer, test-coverage explorer, and checker/readiness explorer launched in
parallel before implementation.

Tests/checkers/docs to add or update: validator role proposal observation tests, runtime-state/status
field tests, local CPU Compose checker-field test, checker assertions for producer-only useful proposal
artifact/attestation evidence, and docs that keep `live_validator_proposer_networking=false` until real
network-derived block assembly is implemented.

Validation: focused validator-role/runtime-status/local-CPU-Compose tests, `cargo fmt --check --all`,
`git diff --check`, final Gate 0, `cargo test -p tensor_vm`, clippy, release workspace tests, and
tarpaulin if available.

Architecture shortcut answers:
- Canonical owner: `chain` remains the only owner of settlement, canonical blockspace, block production,
  reward delay, and finality through `ChainCommand::Produce*Block` and block validation.
- Adapter callers: validator role observation/submission, runtime status snapshots, and the local checker.
- Old shortcut being removed: proposer evidence only says "some settled receipts existed" before proposal.
- Regression test: role proposal observation reports artifact-ready and attested settled receipts before a
  useful proposal, and the checker requires those counts for the sole local validator producer.
- Synthetic production disabled: no local proposal counters advance; status fields remain zero/false.
- Producer/non-producer behavior: only the configured validator producer may report useful proposal and
  proposer settled-work evidence; miners and non-producer validators must report zero proposal counters.
- Structured evidence source: typed `NodeRuntimeState` and `RuntimeStatusSnapshot` fields, not shell-only
  derived booleans.
- Finality source: unchanged stake-weighted block votes; this slice does not synthesize or alter votes.
- Wire-size and codec boundary: unchanged; no new wire payloads or unbounded reads.

Out of scope: replacing the remaining timed synthetic job trigger, proving live proposer networking end to
end, full verifier transcript fraud proofs, or deterministic live bad-block generation.

Implemented locally:
- Scheduled local production now publishes deterministic jobs only; validator-role tick observation and
  submission own useful block proposals for the local validator producer.
- Proposer observation/status/checker evidence now distinguishes settled receipts, local-artifact-ready
  receipts, and attested receipts before useful proposal.
- Useful role-owned proposals still use `ChainCommand::ProduceRewardedBlock`, creating delayed pending
  proposer reward claims; the runtime regression test releases the claim only after the recorded maturity
  height through `ChainCommand::ReleaseMaturedProposerRewards`.
- The local checker requires positive artifact-ready and attested proposer evidence for `validator-00` and
  zero proposer evidence from miners and non-producer validators.

Validation completed locally:
- `cargo test -p tensor_vm local_testnet --release` passed first and again after implementation.
- Focused runtime/status/checker coverage passed: `cargo test -p tensor_vm --test tvmd_runtime
  runtime_roles -- --nocapture`, `cargo test -p tensor_vm --test tvmd_runtime validator_role --
  --nocapture`, `cargo test -p tensor_vm --test tvmd_runtime network_payloads -- --nocapture`, `cargo test
  -p tensor_vm --test tvmd_runtime runtime_state -- --nocapture`, `cargo test -p tensor_vm --lib
  node::runtime_state -- --nocapture`, `cargo test -p tensor_vm --test local_cpu_compose -- --nocapture`,
  and `cargo test -p tensor_vm --test tvmd_cli
  role_run_commands_serve_through_role_specific_surfaces -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.

### Iteration 31: Network-Visible Block-Check Challenge Propagation

Implemented and pushed as `9216461` (`Propagate block check challenges`).

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
- Push result: `6556db2..9216461  main -> main`.

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
- Iteration 36 required Gate 0 first and final Gate 0:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 36 focused validation:
  - `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`: 6 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::blocks -- --nocapture`: 16 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::settlement -- --nocapture`: 5 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::challenges -- --nocapture`: 3 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::root_hashes -- --nocapture`: 3 tests passed.
  - `cargo test -p tensor_vm --lib chain::tests::attestations::mandatory_validator_audit_assignment_missed_slashes_once_on_block_apply -- --nocapture`: 1 test passed.
- Iteration 36 broad validation before feature commit:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - `cargo test -p tensor_vm`: passed with 350 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 30 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 350 `tensor_vm`, 1 local CPU
    Compose, 8 `tvmd_cli`, 30 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests,
    and doc-test targets.
  - `cargo tarpaulin --workspace --offline`: blocked, missing `cargo-tarpaulin`.
- Iteration 36 feature commit: `58da0e6` (`Release matured rewards during blocks`).
- Iteration 36 push result: `9e28b71..58da0e6  main -> main` on `origin/main`.

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
