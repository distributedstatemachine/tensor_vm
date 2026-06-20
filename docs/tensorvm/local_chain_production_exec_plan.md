# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 96 complete - automatic side-branch deep reorg.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage, automatic
  unfinalized side-branch deep reorg, and delayed challenge rewards are implemented locally. Late
  invalid-output attestations now contest settled receipts by voiding delayed pending receipt rewards and
  slashing miner stake before reward maturity.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue verifier-transcript disputes, remaining fraud-path/slashing work, graph-runtime
  production, or rerun the full Docker scenario after the `/health` blocker clears.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; checker expects live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards; current-head useful competitor replacement, side-branch storage, and automatic unfinalized deep reorg | Rerun Docker and continue live proposer evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg | Remaining: full transcript disputes and fresh Docker proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface | Continue exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; status/explorer expose seed-domain and block-hash-ban evidence | Add external drand/VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, mature release, late invalid-output reward voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 96: Automatic Side-Branch Deep Reorg

Feature capability: promote a strictly longer valid side branch into canonical state automatically while
preserving finalized canonical blocks.

Canonical owner: `chain::blocks` fork-choice/admission and `Chain` side-branch state.
Adapter callers: `ChainCommand::SubmitBlock`, p2p/node payload application, storage/node-store snapshots,
and tests call the same chain admission boundary.
Old shortcut being removed: valid multi-block side branches were retained but never promoted automatically
even when they overtook canonical unfinalized height.
Regression test that proves the shortcut is gone:
`chain::tests::longer_side_branch_reorganizes_unfinalized_canonical_suffix`.
Behavior with local synthetic block production disabled: inbound block payloads can trigger the same
chain-owned reorg without synthetic block production.
Behavior for producer and non-producer roles: both converge through `SubmitBlock`/payload admission;
canonical head changes only through shared chain fork-choice.
Structured evidence source: canonical `blocks`, `block_parent_states`, `side_branch_blocks`,
`side_branch_child_states`, state root/height/head, and node payload application.
Finality source: `ChainState::finalized_blocks` and `is_block_finalized` prevent reorg across finalized
canonical blocks.
Wire-size and codec boundary: no p2p wire change; reuse persisted side-branch maps already in the
chain-state codec.

Files/modules likely touched: `chain/blocks.rs`, `chain/engine.rs`, `chain/commands.rs`, node payload
application, focused tests, docs, and this plan.
Parallel subagents to run: none; user prefers no subagents unless explicitly requested.
Tests/checkers/docs to add or update: focused fork-choice/reorg and node payload tests plus consensus docs.
Narrow validation commands: `cargo test -p tensor_vm side_branch --quiet`,
`cargo test -p tensor_vm reorg --quiet`, `cargo test -p tensor_vm blocks --quiet`, and
`cargo test -p tensor_vm block_payload_application --quiet`.
Broad validation commands before commit: final Gate 0, fmt, diff check, full tensor_vm crate, clippy,
workspace release, tarpaulin attempt if feasible.
Expected observable evidence: a longer side branch replaces only the unfinalized canonical suffix; old
canonical suffix moves into side-branch storage; finalized canonical blocks reject conflicting branches.
Out of scope: public Docker rerun, verifier transcript fraud proofs, external drand/VRF, or p2p wire
changes.
Split trigger: if cumulative-work scoring beyond strict branch length is required, split that policy work
from this deterministic longer-branch promotion.

Implementation summary:
- Added `BlockAdmission::Reorganized` / `ChainEvent::ChainReorganized`.
- Side-branch admission now records selected-receipt metadata in branch child states and promotes strictly
  longer branches when no replaced canonical block is finalized.
- Node block payload application now lets known side-branch descendants reach chain admission.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm side_branch --quiet`,
  `cargo test -p tensor_vm reorg --quiet`, `cargo test -p tensor_vm blocks --quiet`, and
  `cargo test -p tensor_vm block_payload_application --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 428 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

## Recent Iterations

### Iteration 95: Invalid-Output Miner Stake Slashing

Assigned invalid-output evidence now slashes the receipt miner once, credits treasury, records a
state-rooted slash, marks the receipt challenged, voids delayed receipt rewards, persists through storage,
and appears in fraud-path calibration/status/explorer output. Validation passed focused
attestation/storage/economics/status tests, full crate, clippy, workspace release, and first/final Gate 0.
Commit `695c66e` is pushed to `origin/main`.

### Iteration 94: Side-Branch Fork Tree Storage

Valid known-parent non-canonical blocks are retained in chain-owned side-branch storage with parent/child
state snapshots, persisted through chain-state snapshots, and applied through normal node payload admission
without mutating canonical head state. Validation passed focused fork-tree/storage/payload tests, full
crate, clippy, workspace release, and first/final Gate 0. Commit `c33ef38` is pushed to `origin/main`.

### Iteration 93: Invalid-Output Delayed Reward Voiding

Late assigned `Invalid` attestations now mark the receipt challenged, remove it from settled receipts, and
void matching pending miner and validator receipt rewards before spendability. Validation passed focused
settlement tests, full crate, clippy, workspace release, and first/final Gate 0. Commit `bf0d5fa` is pushed
to `origin/main`.

### Iteration 91: Explicit Fraud-Window Reward Delay Evidence

`ChainParams::fraud_reward_hold_blocks` now makes receipt reward maturity explicitly cover the challenge
window plus active audit window before spendability. Reward economics regression coverage uses delayed
pending miner rewards rather than a mature-claim workaround. Validation passed focused reward/params,
full crate, clippy, workspace release, and first/final Gate 0. Commit `31bcc49` is pushed to `origin/main`.

### Iteration 92: Live Detection Probability Evidence

`ChainState::detection_probability_evidence`, service status, and explorer overview now expose live
per-mechanism probability evidence for implemented verifier/fraud paths. Validation passed focused
detection/status/explorer tests, full crate, clippy, workspace release, and first/final Gate 0. Commit
`5697593` is pushed to `origin/main`.

### Iteration 90: Chain-Owned Randomness Binding Evidence

Status/explorer now expose finalized-beacon randomness source, seed domains, anchor consistency counts,
commit-reveal ordering, and the current-block-hash randomness ban. Validation passed focused randomness,
status, explorer, full crate, clippy, workspace release, and first/final Gate 0. Commit `c6baaf5` is
pushed to `origin/main`.

### Iteration 89: Delayed Receipt Reward Fraud Exposure

Validator-audit and data-unavailability calibration now count `reward_from_fraud` only from non-voided
receipt claims whose `claimable_at_height` is at or below canonical chain height. Immature pending miner
and validator receipt rewards remain at-risk escrow but no longer inflate immediate fraud proceeds.
Validation passed focused reward/status/explorer tests, full crate, clippy, workspace release, and
first/final Gate 0. Commit `ece08ff` is pushed to `origin/main`.

### Iteration 88: Competing-Head Fork-Choice And Withholding Policy

Current-head useful UVPoW competitors can replace only an unfinalized same-parent useful head when the new
PoW evidence is strictly preferred; finalized heads, historical heights, different-parent conflicts, and
fallback heads remain stable. Validation passed focused block/payload tests, full crate, clippy, workspace
release, and first/final Gate 0. Commits `3a75b33`, `d2e758d`, and `1484592` are pushed to `origin/main`.

### Iteration 87: Delayed Block-Check Proposer Reward Protection

`ChainParams::proposer_reward_hold_blocks` delays proposer rewards through the block-check fraud window.
Block-check economics now treats delayed proposer rewards as slashable escrow and only mature claims as
fraud proceeds. Validation passed focused reward/storage/status/explorer/params/challenge tests, full
crate, clippy, workspace release, and first/final Gate 0. Commits `1923692`, `b638369`, and `d4d11b8` are
pushed to `origin/main`.

### Iteration 86: Fraud-Path Economic Calibration

Implementation summary:
- Added `ChainState::fraud_path_economic_calibration` for validator-audit, miner data-unavailability, and
  block-check/proposer clawback paths.
- Service status and explorer overview now render aggregate and per-path required-bond/pass-fail evidence.
- Updated `upow.md`, readiness, coverage, implementation status, and tarpaulin docs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm economic --quiet`, `cargo test -p tensor_vm status --quiet`, and
  `cargo test -p tensor_vm explorer_overview_exports --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 415 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `1116beb` (`Expose fraud path economic calibration`) pushed to `origin/main`.
- Evidence commit: `abf78d1` (`Record fraud path calibration evidence`) pushed to `origin/main`.

### Iteration 85: Audit-Window Reward Escrow

`ChainParams::reward_maturity_delay_blocks` now includes the validator-audit hold when audit sampling is
enabled, and tensor retention mirrors the same audit-window bound. No-audit profiles keep the old
zero-retention behavior. Validation passed focused parameter/audit/reward tests, full crate, clippy,
workspace release, and first/final Gate 0. Commit `5df4870` (`Delay rewards through audit window`) is
pushed to `origin/main`.

### Iteration 84: Validator Audit Stake-Slash Reversal

Reversed validator-audit appeals now refund the recorded stake slash from treasury back to validator stake
exactly once while reward reinstatement still uses delayed pending claims. Validation passed focused
audit/storage/reward tests, full crate, clippy, workspace release, and first/final Gate 0. Commits
`1feeb1d` and `ea230b3` are pushed to `origin/main`.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest full validation is Iteration 96 on June 20, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm side_branch --quiet
cargo test -p tensor_vm reorg --quiet
cargo test -p tensor_vm blocks --quiet
cargo test -p tensor_vm block_payload_application --quiet
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo test -p tensor_vm local_testnet --release
cargo tarpaulin --workspace --offline
```

Current iteration Gate 0 passed:

```text
cargo test -p tensor_vm local_testnet --release
```

Current coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iterations 75-83: diagnostic block-check challenge path, fallback timeout, inclusion-gated and
  chain-owned reward release, and receipt-bound validation seed landed in commits `8787912`, `40f14d5`,
  `06be27e`, `f5a0aa2`, `1647a47`, `8ce051f`, `e08f7c9`, and related evidence commits.
- Iterations 73-74: live validator-audit economic calibration and appeal reward-delay resolution landed in
  commits `493191c`, `8dbb654`, `c8a6f9e`, `32fb557`, and `7026c94`.
- Iterations 59-64: exact `clamp`, field `div`, split/einsum, registry/conformance guard, and graph
  verifier coverage landed across commits including `85a2956`, `d659e14`, and `b6e0887`.
- Iterations 41-58: Tensor IR, graph-backed jobs/receipts, exact replay, quantization, Tier-B coverage,
  delayed proposer reward cleanup, and economic helper foundations landed in git history.
- Iterations 30-34: delayed proposer, receipt, challenger, and credit reward-ledger foundations landed in
  commit `5664acb` and related evidence commits.
