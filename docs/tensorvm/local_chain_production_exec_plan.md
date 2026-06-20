# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: none; Iteration 38 is complete.
- Current status: Iteration 38 clarified delayed proposer reward runtime evidence on June 20, 2026.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: choose the next readiness slice from the remaining v0 gaps. Strong candidates are
  randomness commit/reveal binding, generic arbitrary-IR execution/admission, multi-validator proposer
  competition/fork-choice policy, or the Docker `/health` blocker if the environment changes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 38: `cargo test -p tensor_vm local_testnet --release` passed first and after implementation on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | Iteration 37 split proposal gating from synthetic job production and added `validator_proposer_tick_runs_without_synthetic_producer_gate` | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial, current-job graph body storage implemented locally | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, current-job graph bodies in state/storage/P2P | Add generic arbitrary-IR execution and user-submitted graph body admission/fetch |
| Per-op `F_p` conformance vectors | Partial current-job gate implemented locally | Deterministic vectors for current executable ops, stable suite hash, CPU pass profile, default CUDA non-admission, verifier gates | Add broader admitted-registry vectors, generic interpreter coverage, CUDA pass evidence when compiled |
| Randomness commit/reveal or VRF beacon | Partial | Finalized-beacon binding exists; no full commit-reveal/VRF lifecycle | Add commit/reveal ordering and enforce receipt-bound unbiasable seed lifecycle |
| Economics and slashing invariant | Partial | Delayed proposer, receipt, challenge, and credit rewards; full reward-root binding; block-transition mature release; data-unavailability and validator-audit slashing | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

None.

## Recent Iterations

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

### Iteration 37: Validator Proposer Tick Without Synthetic-Producer Gating

Feature capability: a configured validator proposer can produce a useful block from already accepted,
settled, artifact-ready, attested state even when `local_synthetic_producer=false`. The synthetic producer
path now controls deterministic local job publication only; validator proposal uses `local_block_proposer`.

Readiness requirements covered: `upow.md` §2 and §11 validator-owned block proposal over deterministic
settled-receipt blockspace; `mvp_spec.md` §4.6 adapter shortcut ban; local readiness requirement to replace
the remaining synthetic-round block-assembly coupling with role-owned proposer evidence.

Architecture shortcut answers:
- Canonical owner: `chain` remains the owner of parent-state preparation, settlement, block production,
  UVPoW, selected receipts, delayed rewards, and finality commands.
- Adapter callers: validator role tick observes local chain/node state, calls existing chain commands, and
  publishes the resulting block payload; runtime/status/checker surfaces only record structured evidence.
- Old shortcut removed: validator proposal observation and block proposal were gated by
  `config.node.local_synthetic_producer()`, coupling block assembly to the timed synthetic job producer.
- Regression test: `validator_proposer_tick_runs_without_synthetic_producer_gate` proves a configured
  validator proposer with no synthetic block interval has `local_synthetic_producer=false`, yet proposes a
  useful block from settled accepted state and records proposer counters.
- Behavior with synthetic production disabled: deterministic local job publication remains disabled, while
  accepted ready work can still be proposed by the configured validator proposer.
- Behavior for producer and non-producer roles: configured validator proposers may propose from ready
  state; miners, gateways, and legacy proposer roles still cannot produce local blocks through the role
  tick.
- Structured evidence source: `NodeRuntimeState` proposer counters, `ChainState` selected receipts and
  pending proposer rewards, and Rust runtime tests.
- Finality source: unchanged stake-weighted `SubmitBlockVote`; this feature does not synthesize votes.
- Wire-size and codec boundary: no new payloads or codecs; existing bounded `TensorBlock` publication is
  reused.

Implemented locally:
- Added `NodeConfig::local_block_proposer()` to split configured validator proposal duty from
  profile-gated synthetic job scheduling.
- Changed the validator role tick to use `local_block_proposer()` for proposal observation/submission while
  `local_synthetic_producer()` remains the gate for scheduled deterministic local job publication.
- Added focused runtime coverage proving a validator proposer can submit a useful block with
  `local_synthetic_producer=false`.
- Updated coverage/status/readiness docs while keeping the Docker `/health` and public-readiness blockers
  explicit.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused tests passed:
  - `cargo test -p tensor_vm --test tvmd_runtime runtime_roles -- --nocapture`
  - `cargo test -p tensor_vm --test tvmd_runtime network_payloads -- --nocapture`
  - `cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture`
  - `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
  - `cargo test -p tensor_vm --lib profile::tests -- --nocapture`
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `9d9f716` (`Decouple validator proposals from synthetic production`).
- Push result: `5e9e182..9d9f716  main -> main` on `origin/main`.

Out of scope: full Docker rerun while `/health` remains blocked, multi-validator proposer competition,
deterministic live bad-block generation, randomness commit/reveal, generic arbitrary-IR execution, and
public-run evidence.

### Iteration 36: Block Transitions Release Matured Rewards

Feature capability: normal block production/admission releases matured reward claims through the
chain-owned child-state transition instead of requiring adapters/checkers to call explicit release commands.

Implemented locally:
- Factored proposer, receipt, challenge, and credit reward release into shared chain helpers.
- `apply_block_to_parent_state` now applies current-block receipt-inclusion delays and slash/audit voiding,
  then sweeps still-matured pending reward claims before height/beacon update and the new proposer claim.
- Matured voided proposer reward claims are pruned without credit.
- Added focused tests for producer/non-producer automatic mature release and voided proposer-claim sweep.

Validation completed locally:
- Required Gate 0 first and final: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused chain reward/block/settlement/challenge/root/audit tests passed.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` blocked because `cargo-tarpaulin` is not installed.
- Feature commit: `58da0e6` (`Release matured rewards during blocks`).
- Evidence commit: `5e9e182` (`Record matured reward release evidence`).
- Push result: `58da0e6..5e9e182  main -> main` on `origin/main`.

Out of scope: unified formal reward-claim object/status, replacing test-only spendable-balance helpers,
public reward-settlement evidence, full auditor-selection policy, appeal paths, and bond calibration.

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
  reward claims; fallback proposals remain unrewarded.
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
