# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none; Iteration 40 validation is complete and commit/push evidence is pending.
- Current status: Iteration 40 implemented reduced delayed fallback proposer rewards on June 20, 2026.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue with full VRF/drand commit-reveal lifecycle, generic arbitrary-IR
  execution/admission, multi-validator proposer competition/fork-choice policy, or the Docker `/health`
  blocker if the environment changes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 40: `cargo test -p tensor_vm local_testnet --release` passed first and after implementation on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | Iteration 37 split proposal gating from synthetic job production and added `validator_proposer_tick_runs_without_synthetic_producer_gate` | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job graph bodies in state/storage/P2P | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Add broader admitted-registry vectors, generic interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; Iteration 39 anchors admitted receipt assignment/validation seeds to persisted receipt-time beacon state | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial | Delayed proposer, reduced delayed fallback proposer, receipt, challenge, and credit rewards; full reward-root binding; block-transition mature release; data-unavailability and validator-audit slashing | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

None.

## Recent Iterations

### Iteration 40: Reduced Delayed Fallback Proposer Rewards

Feature capability: empty `PowSkipFallback` blocks can now carry a reduced proposer reward claim instead
of relying on a runtime workaround that skipped rewards entirely. The fallback claim is state-rooted and
delayed like other proposer rewards, but it is additionally blocked until a later useful block includes
settled receipt blockspace.

Architecture shortcut answers:
- Canonical owner: `chain` owns fallback reward delay and release.
- Adapter callers: validator role now submits a reduced rewarded fallback block command instead of
  bypassing reward creation for empty blockspace.
- Old shortcut removed: runtime no longer treats fallback production as automatically unrewarded.
- Regression test: `fallback_proposer_reward_waits_for_useful_successor`.
- Synthetic production disabled: unchanged; fallback reward release still requires later useful blockspace.
- Producer/non-producer roles: both recompute the pending fallback claim through block state roots and
  node-store snapshots.
- Structured evidence source: `PendingProposerReward::requires_useful_successor`, reward root, storage
  roundtrip, focused chain tests.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no new p2p payloads; node-store chain-state snapshot encoding extended.

Implemented locally:
- Added `requires_useful_successor` to pending proposer rewards and included it in reward roots and
  node-store snapshot encoding.
- Block application now unlocks blocked fallback proposer rewards only when selected useful receipts are
  included by a later block.
- Reward release skips blocked fallback claims while still pruning voided claims without credit.
- Validator role fallback proposals now use a reduced proposer reward instead of bypassing reward creation.
- Updated readiness/status docs to describe reduced delayed fallback reward behavior while keeping the full
  stake-weighted fallback rotation policy gap open.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused tests passed:
  - `cargo test -p tensor_vm --lib chain::tests::rewards -- --nocapture`
  - `cargo test -p tensor_vm --lib chain::tests::blocks -- --nocapture`
  - `cargo test -p tensor_vm --lib chain::tests -- --nocapture`
  - `cargo test -p tensor_vm --lib storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: pending.
- Push result: pending.

Out of scope: full stake-weighted fallback proposer rotation/timeout policy, multi-validator fork-choice,
full reward-claim object unification, and full Docker rerun while `/health` remains blocked.

### Iteration 39: Receipt-Bound Validation Randomness Anchors

Feature capability: admitted receipts record the finalized beacon round/randomness and derived assignment
seed used by future validator assignment and validation seed derivation, so a receipt's validation
randomness cannot drift when later blocks advance the finalized beacon.

Architecture shortcut answers:
- Canonical owner: `chain` owns receipt randomness anchors and seed derivation.
- Adapter callers: runtime/validator code continues to call chain assignment/seed helpers.
- Old shortcut removed: deriving admitted-receipt assignment/validation seeds from the mutable current
  finalized beacon at attestation time.
- Regression test: `admitted_receipt_validation_randomness_is_anchored_at_submission`.
- Synthetic production disabled: unchanged; accepted receipts still carry chain-owned randomness anchors.
- Producer/non-producer roles: both use the same persisted anchor through shared chain state/storage.
- Structured evidence source: `ChainState::receipt_randomness_anchors`, state root, storage roundtrip,
  focused chain tests.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no new p2p payloads; node-store chain-state snapshot encoding extended.

Implemented locally:
- Added `ReceiptRandomnessAnchor` to chain state, state root, genesis, and node-store snapshot encoding.
- Receipt admission stores the current finalized beacon round/randomness and derived assignment seed.
- Validator assignment and `Chain::validation_seed` now prefer the persisted receipt anchor, falling back
  to current finalized randomness only for synthetic/unknown receipt IDs used by low-level tests.
- Added focused chain and storage tests for stable admitted-receipt seeds and persistence.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused tests passed:
  - `cargo test -p tensor_vm --lib chain::tests::proposers -- --nocapture`
  - `cargo test -p tensor_vm --lib storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture`
  - `cargo test -p tensor_vm --lib chain::tests -- --nocapture`
  - `cargo test -p tensor_vm --lib storage::chain_state -- --nocapture`
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `a4c1378` (`Anchor receipt validation randomness`).
- Push result: `4984e6f..a4c1378  main -> main` on `origin/main`.

Out of scope: full VRF/drand implementation, external randomness service integration, public
commit-reveal networking, and generic arbitrary-IR execution.

### Iteration 38: Runtime Reward Delay Evidence

Feature capability: runtime role coverage now proves a delayed useful-proposer reward matures through
ordinary block production instead of an adapter-side release command.

Implemented locally:
- Tightened `producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks` so it advances one
  normal block past the pending proposer claim's `claimable_at_height`.
- Removed the manual `ReleaseMaturedProposerRewards` command from that runtime-role proof; the assertion
  now depends on the chain-owned block transition releasing the matured claim.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused test passed:
  - `cargo test -p tensor_vm --test tvmd_runtime producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks -- --nocapture`
- Lightweight gates passed: `cargo fmt --check --all` and `git diff --check`.

Out of scope: new reward ledger types, public reward-settlement evidence, and Docker rerun while `/health`
remains blocked.


## Decision Log

- `docs/tensorvm/upow.md` is canonical when it conflicts with older readiness text.
- Keep the missing workflow document visible as a standing blocker; do not treat the readiness doc as a
  substitute.
- Preserve one shared chain engine. Deployment profiles can vary, but transition logic must not fork.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through
  the shared P2P/event path.
- TensorWork affects rewards, blockspace, telemetry, and concentration analysis only; it never selects
  block proposers.
- `tvmd` is an adapter/process launcher, not a hidden consensus orchestrator.
- Current v0 admits exact Tier-A/B ops only. Tier-C vocabulary may exist in the registry but must be gated
  out of consensus until canonical references and verifiers exist.
- Current-job graph bodies are stored as canonical JSON bytes after graph validation; generic arbitrary-IR
  decoding/execution remains a separate future slice.
- Split configured validator block proposal from local synthetic job production: `local_block_proposer`
  controls configured validator proposal duty, while `local_synthetic_producer` controls profile-gated
  deterministic local job publication.

## Validation Evidence

Latest current-iteration evidence:
- Starting branch state: `## main...origin/main`.
- Iteration 37 required Gate 0 first and final Gate 0:
  `cargo test -p tensor_vm local_testnet --release` passed.
- Iteration 37 focused validation:
  - `cargo test -p tensor_vm --test tvmd_runtime runtime_roles -- --nocapture`: 8 tests passed.
  - `cargo test -p tensor_vm --test tvmd_runtime network_payloads -- --nocapture`: 4 tests passed.
  - `cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture`: 7 tests passed.
  - `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`: 1 test passed.
  - `cargo test -p tensor_vm --lib profile::tests -- --nocapture`: 4 tests passed.
- Iteration 37 broad validation:
  - `cargo fmt --check --all`: passed.
  - `git diff --check`: passed.
  - `cargo test -p tensor_vm`: passed with 350 library tests, 1 local CPU Compose integration test, 8
    `tvmd_cli` integration tests, 31 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo test --workspace --release`: passed with 14 `experiments`, 350 `tensor_vm`, 1 local CPU
    Compose, 8 `tvmd_cli`, 31 `tvmd_runtime`, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests,
    and doc-test targets.
- Iteration 37 feature commit: `9d9f716` (`Decouple validator proposals from synthetic production`).
- Iteration 37 push result: `5e9e182..9d9f716  main -> main` on `origin/main`.

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

- Iteration 35, `f53700c Bind reward root to pending claims`: block `reward_root` now commits spendable
  rewards plus pending proposer, receipt, challenge, and credit ledgers; old spendable-only roots are
  rejected. Evidence update was followed by Iteration 36.
- Iteration 34, delayed generic reward credits: converted `CreditReward`/faucet-style credits into
  state-rooted pending credit claims before spendability.
- Iteration 33, current-job conformance/IR status refresh: recorded current-job conformance and generic IR
  gaps after the conformance and graph-body slices.
- Iteration 32, `26e3e25 Move validator proposals into role tick`: moved useful proposal evidence into
  validator role ticks with settled/artifact-ready/attested counters, while still gated by synthetic
  producer policy before Iteration 37.
- Iteration 31, `9216461 Propagate block check challenges`: added bounded block-check challenge p2p
  payloads, pending retry, chain-command application, and delayed challenge reward evidence.
- Iteration 30, `5664acb Delay validator proposer rewards`: useful proposals create delayed proposer
  reward claims; later work changed fallback proposals from unrewarded to reduced delayed claims that
  require a useful successor before release.
- Iteration 29, `4e8b0c6 Propagate validator audit reports`: validator roles gossip/apply signed audit
  reports through bounded p2p/node payloads.
- Iteration 28, `99d819c Add validator audit reward slashing`: added audit assignments/results/slashes and
  delayed audited validator reward handling.
- Iteration 27, `cae45b5 Handle unavailable receipt rewards and slashing`: unavailable-data attestations
  void receipt rewards and slash miner bond once.
- Iteration 26, `25dbfe4 Delay challenger reward finality`: challenger bounties become pending challenge
  claims before spendability.
- Iteration 25, `0363bb6 Store Tensor IR graph bodies` with evidence `f734a69`: current-job graph bodies
  are state-rooted, persisted, and served through `RequestProgram`.
- Iteration 24, `f4d4491 Add Fp conformance vector gate`: current executable exact-op conformance vectors
  and CPU verifier gates.
- Iteration 23, `388c4d6 Delay receipt reward finality`: receipt settlement creates delayed miner and
  validator reward claims.
- Iterations 1-22: extracted reusable node runtime state, moved network payload application/event drivers
  into reusable runtime helpers, bound role runtimes to chain identities, added miner receipt submission,
  validator attestations, validator block votes, network-visible block payload admission, useful-verification
  PoW block validity, remote validator tensor fetch, validator-owned block proposal ticks, content-addressed
  Tensor IR foundation, finalized-beacon consensus randomness binding, block apply openings, retarget/fallback
  mode, delayed proposer rewards, and checker evidence for role-owned local work.
