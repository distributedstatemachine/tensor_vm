# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 127 complete - Codex 5.5 local-chain workflow doc.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage, automatic
  unfinalized side-branch deep reorg, graph-backed synthetic jobs, and delayed challenge rewards are
  implemented locally. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner
  TensorWork activation now follows delayed miner receipt reward maturity instead of immediate settlement,
  and settled receipt rewards carry explicit awaiting-inclusion or claimable-height maturity state before release.
  Newly emitted receipt-reward pending events now carry that maturity state directly instead of flattening
  awaiting-inclusion rewards into a synthetic claim height.
  Block-check, invalid-output, and data-unavailability evidence now delays voided receipt claims to a
  state-rooted challenge hold height before they can be swept without credit.
  Selected-receipt block openings now expose typed block-check transcript commitments and
  submission-anchored retention deadlines. Redundancy-delayed receipts now have chain-owned state-rooted
  records when quorum-backed work cannot settle because distinct-operator agreement is missing or
  conflicting, and later pending receipt reward claims inherit those redundant reward holds. Redundant
  delay records persist both agreeing miner-address and agreeing operator counts. External randomness
  beacon records can now advance future receipt randomness through a rooted chain command. `Fixed32`
  multiplication now rescales the signed raw product back to the lhs/output scale with round-half-to-even
  semantics in tensor, exact IR replay, and conformance vectors. Mixed-scale `Fixed32` `add`/`sub` now
  rescale the RHS to the lhs/output scale with the same half-even policy. `Fixed32` reciprocal division now
  returns to the lhs/output scale with the same half-even policy and rejects zero divisors. `Fixed32`
  matmul now accumulates signed raw products in fixed order and rescales once into the lhs/output scale.
  Packed int8 quantization now has a tensor-owned `TVQ8` payload API for bounded length calculation and
  shared encode/decode validation used by IR replay and conformance. External graph job payloads with
  missing graph bodies now stay pending through the shared node payload path, runtime ingest fetches
  missing graph bodies by request-response before retry, and miner/validator role loops fetch missing graph
  tensor artifacts, including `const_blob` tensors, before execution or attestation. Exact IR execution
  now exposes verified per-op trace openings, and libp2p can sample them by `trace_root` and op index.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue Tier-C committee policy, deployed-run economics evidence, CUDA graph evidence, or
  rerun Docker after the `/health` blocker clears.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 21, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; checker expects live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards; current-head useful competitor replacement, side-branch storage, and automatic unfinalized deep reorg | Rerun Docker and continue live proposer evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, submission-anchored opening retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg | Remaining: full interactive transcript disputes and fresh Docker proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface, packed int8 artifact APIs, role-owned graph execution, content-addressed `const_blob` replay/fetch, and p2p-sampled verified trace openings | Continue exact Tier-B verifier coverage, full interactive trace disputes, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records with miner/operator counts, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; status/explorer expose seed-domain, local finalized-beacon round mapping, local validator VRF-seed derivation, external beacon record evidence, and block-hash-ban evidence | Add live drand/VRF client wiring and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit receipt reward maturity state, inclusion-started receipt reward maturity, mature release, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 127: Codex 5.5 Local Chain Workflow Doc

Feature capability: provide the referenced Codex local-chain workflow artifact for future goal-loop runs.
Readiness requirements covered: `goal.md` commit/push workflow, `mvp_spec.md` §32.1 autonomous agent
completion contract, and the local readiness acceptance gate.
Files/modules likely touched: `docs/tensorvm/codex_5_5_local_chain_workflow.md`,
`testnet/tests/deployment_docs.rs`, exec/status/coverage/tarpaulin docs.
Parallel subagents to run: not used; available subagent tool requires explicit user delegation.
Tests/checkers/docs to add or update: focused deployment-doc test requiring Gate 0, context refresh,
Docker gate, validation, evidence, and commit/push workflow lines.
Narrow validation commands: `cargo test -p tensor_vm codex_local_chain_workflow --quiet`.
Broad validation commands before commit: fmt, diff check, tensor_vm tests, clippy, workspace release tests,
tarpaulin attempt, final Gate 0.
Expected observable evidence: the referenced workflow file exists and names required commands/evidence flow.
Out of scope: consensus mutation, Docker rerun, public evidence generation, CUDA evidence.
Split trigger: if the doc guard reveals stale command names outside this workflow artifact.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused test passed: `cargo test -p tensor_vm codex_local_chain_workflow --quiet`.
- Broader checks passed: `cargo fmt --check --all`, `git diff --check`,
  `cargo test -p tensor_vm --quiet`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace --release`.
- Coverage regeneration remains blocked because `cargo tarpaulin --workspace --offline` reports
  `error: no such command: tarpaulin`.
- Final Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.

### Iteration 126: Receipt Reward Pending Event Delay State

Feature capability: make `ReceiptRewardPending` events carry explicit delayed maturity state.
Readiness requirements covered: reward settlement events must reflect chain-owned reward delay and must not
make downstream callers infer awaiting-inclusion from `u64::MAX`.
Canonical owner: `ReceiptRewardMaturity` remains the chain state source of truth; settlement event emission
only renders that state.
Old shortcut being removed: pending settlement events no longer report awaiting-inclusion receipt rewards as
a synthetic far-future claim height.
Regression test that proves the shortcut is gone: command settlement events assert `claimable_at_height:
None` and `awaiting_inclusion: true` for newly pending miner and validator receipt rewards.
Structured evidence source: `ChainEvent::ReceiptRewardPending` plus focused command tests.
Files/modules touched: `chain::engine`, `chain::settlement`, command tests, and status docs.
Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused test passed: `cargo test -p tensor_vm chain::tests::commands --quiet`.
- Broader checks passed: `cargo fmt --check --all`, `git diff --check`,
  `cargo test -p tensor_vm --quiet`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace --release`.
- Coverage regeneration remains blocked because `cargo tarpaulin --workspace --offline` reports
  `error: no such command: tarpaulin`.
- Final Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.
- Commit/push evidence: implementation committed as `2c5cb68`; evidence follow-up committed as `c6613cb`;
  both pushed to `origin/main`.

### Iteration 125: Explicit Pending Reward Maturity Views

Feature capability: expose awaiting-inclusion receipt rewards as first-class delayed claims instead of
reporting a synthetic far-future claim height.
Readiness requirements covered: `mvp_spec.md` §20.3/§25 delayed receipt reward finality, structured local
checker evidence for pending rewards, and reward-rooted state visibility.
Canonical owner: chain state owns pending reward maturity; RPC/status/explorer views only render that state.
Adapter callers: service status and explorer overview pending-reward samples.
Old shortcut being removed: presentation code no longer treats `AwaitingInclusion` as a claimable
`u64::MAX` height.
Regression test that proves the shortcut is gone: pending receipt claims before inclusion report
`awaiting_inclusion=true` and no concrete claim height, then included claims report a real delayed height.
Behavior with local synthetic block production disabled: unchanged; only reward evidence views change.
Behavior for producer and non-producer roles: unchanged; canonical block inclusion still assigns maturity.
Structured evidence source: chain pending-claim view, service status, and explorer overview.
Finality source: unchanged delayed reward release path.
Wire-size and codec boundary: explorer JSON gains explicit maturity-state evidence; chain storage is
unchanged.
Files/modules likely touched: chain state view, status/explorer rendering, focused tests, docs.
Parallel subagents to run: not used; available subagent tool forbids spawning unless explicitly requested.
Parallelizable implementation workstreams: reward view/API rendering and docs evidence.
Tests/checkers/docs to add or update: focused reward/status/explorer tests and docs evidence.
Narrow validation commands: `cargo test -p tensor_vm pending_reward_claim --quiet`.
Broad validation commands before commit: Gate 0, fmt, diff check, tensor_vm tests, clippy, workspace
release tests, tarpaulin attempt, and final Gate 0.
Expected observable evidence: awaiting-inclusion rewards are visibly delayed without sentinel-height
workarounds, and actual spendable credit remains blocked until inclusion-derived maturity.
Out of scope: reward-root storage format, public Docker rerun, CUDA evidence, and full fraud proofs.
Split trigger: explorer JSON compatibility fallout outside pending reward samples.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused tests passed: `cargo test -p tensor_vm pending_reward_claim --quiet`,
  `cargo test -p tensor_vm service_status_exports_pending_reward_claim_maturity_details --quiet`,
  `cargo test -p tensor_vm explorer_overview_exports_validator_audit_economic_calibration --quiet`, and
  `cargo test -p tensor_vm_explorer --quiet`.
- Final checks passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --quiet`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`, and
  final `cargo test -p tensor_vm local_testnet --release`.
- Coverage regeneration remains blocked because `cargo tarpaulin --workspace --offline` reports
  `error: no such command: tarpaulin`.

## Recent Iterations

- Iteration 124 made collusion-risk study evidence operator-aware in commit `bdac46b`.
- Iteration 123 made redundant settlement quorum operator-distinct in commit `1c86e13`.
- Iteration 122 delayed voided receipt rewards through challenge holds in commit `bde7e51`.
- Iterations 120-121: trace openings and p2p trace-opening sampling landed in `b3fe556` and `f631084`.
- Iterations 116-119: packed int8 artifacts, external graph artifact fetch, and explicit receipt reward
  maturity landed in prior pushed commits.
- Iterations 110-115: fixed-point rescale semantics, packed int8 payloads, and delayed reward cleanup
  landed in pushed commits including `ce665a5`, `4de9463`, `506b020`, `4fceaeb`, and `1c65b80`.

Earlier detailed iterations are summarized in the archive to keep this plan compact.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest full validation is Iteration 127 on June 21, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm codex_local_chain_workflow --quiet
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo tarpaulin --workspace --offline
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
- Iterations 84-86: validator-audit stake-slash reversal, audit-window reward escrow, and fraud-path
  economic calibration landed in commits `1feeb1d`, `ea230b3`, `5df4870`, `1116beb`, and `abf78d1`.
- Iterations 87-93: delayed block-check proposer rewards, competing-head fork choice, receipt fraud
  exposure, chain-owned randomness binding, explicit fraud-window delay, live detection probability, and
  invalid-output reward voiding landed across commits including `1923692`, `1484592`, `ece08ff`,
  `c6baaf5`, `31bcc49`, `5697593`, and `bf0d5fa`.
- Iterations 94-103: side-branch storage/reorg, invalid-output miner stake slashing, role-owned graph
  production, typed block-check openings, submission-anchored retention, and inclusion-started receipt
  reward delay landed across commits including `c33ef38`, `695c66e`, `4d585f8`, `5af3fcf`, `8aef9bb`,
  `aa2e9f3`, and `456ab81`.
- Iterations 73-74: live validator-audit economic calibration and appeal reward-delay resolution landed in
  commits `493191c`, `8dbb654`, `c8a6f9e`, `32fb557`, and `7026c94`.
- Iterations 59-64: exact `clamp`, field `div`, split/einsum, registry/conformance guard, and graph
  verifier coverage landed across commits including `85a2956`, `d659e14`, and `b6e0887`.
