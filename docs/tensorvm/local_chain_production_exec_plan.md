# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 111 complete - mixed-scale `Fixed32` multiplication semantics.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage, automatic
  unfinalized side-branch deep reorg, graph-backed synthetic jobs, and delayed challenge rewards are
  implemented locally. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner
  TensorWork activation now follows delayed miner receipt reward maturity instead of immediate settlement,
  and settled receipt rewards explicitly await canonical blockspace inclusion before their maturity clock starts.
  Selected-receipt block openings now expose typed block-check transcript commitments and
  submission-anchored retention deadlines. Redundancy-delayed receipts now have chain-owned state-rooted
  records when quorum-backed work cannot settle because agreement is missing or conflicting, and later
  pending receipt reward claims inherit those redundant reward holds. External randomness beacon records
  can now advance future receipt randomness through a rooted chain command. `Fixed32`
  multiplication now rescales the signed raw product back to the lhs/output scale with round-half-to-even
  semantics in tensor, exact IR replay, and conformance vectors. Mixed-scale `Fixed32` `add`/`sub` now
  rescale the RHS to the lhs/output scale with the same half-even policy.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue fixed-point reciprocal division or matmul accumulation/range policy, Tier-C
  committee policy, deployed-run economics evidence, or rerun Docker after the `/health` blocker clears.

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
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface including mixed-scale fixed-point `add`/`sub`/`mul`, role-owned local graph execution, and content-addressed `const_blob` artifact replay | Continue fixed-point reciprocal division, matmul accumulation/range policy, exact Tier-B verifier coverage, dispute-time blob availability, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and public/operator independence evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; status/explorer expose seed-domain, local finalized-beacon round mapping, local validator VRF-seed derivation, external beacon record evidence, and block-hash-ban evidence | Add live drand/VRF client wiring and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, inclusion-started receipt reward maturity, mature release, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 111: Mixed-Scale Fixed32 Multiplication Semantics

Feature capability: allow `Fixed32` `mul` with different input scales by treating the signed product as
scale `lhs_scale + rhs_scale`, then rescaling it into the lhs/output scale with canonical round-half-to-even
semantics.
Readiness requirements covered: `upow.md` §3.1/§4.8 fixed-point scale discipline and §3.3 conformance
evidence for exact Tier-B arithmetic.
Files/modules likely touched: `tensor`, `ir`, `conformance`, fixed-point docs/status.
Parallel subagents to run: not used; available subagent tool requires explicit user delegation and this is a
single-owner deterministic semantics change.
Parallelizable implementation workstreams: none in this single-writer deterministic semantics change.
Canonical owner: `tensor` owns fixed-point signed-product rescale/arithmetic; `ir` permits the same policy
for validated graph `mul`; conformance records vectors.
Adapter callers: runtime/verifier/roles consume the same tensor and IR APIs.
Old shortcut being removed: mixed-scale `Fixed32` `mul` failed dtype/scale checks and required an explicit
pre-cast workaround.
Regression test that proves the shortcut is gone: tensor/IR/conformance mixed-scale mul tests.
Behavior with local synthetic block production disabled: unchanged; this is pure deterministic execution.
Behavior for producer and non-producer roles: unchanged; receipts replay to the same canonical roots.
Structured evidence source: conformance vectors plus exact interpreter output roots.
Finality source: unchanged block finality; affected receipts still pass through canonical verification.
Wire-size and codec boundary: no wire/storage shape change; only deterministic value semantics.
Narrow validation commands: mixed-scale mul tensor/IR/conformance tests.
Broad validation commands before commit: format, diff check, full `tensor_vm`, clippy, release workspace,
final Gate 0, tarpaulin attempt.
Expected observable evidence: `Fixed32` mul outputs keep lhs scale and match product half-even rescale from
combined input scale.
Out of scope: fixed-point reciprocal division, matmul accumulation/range, CUDA graph execution,
public/Docker run.
Split trigger: unexpected IR validation or conformance profile changes outside `mul`.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused: `cargo test -p tensor_vm fixed32_multiply_rescales_mixed_scales_to_lhs_scale_half_even --quiet` passed.
- Focused: `cargo test -p tensor_vm exact_interpreter_executes_fixed32_mul_with_mixed_scales --quiet` passed.
- Focused: `cargo test -p tensor_vm conformance_vectors_are_stable_and_cover_current_ops --quiet` passed.
- Focused: `cargo test -p tensor_vm cpu_reference_passes_all_vectors --quiet` passed.
- Focused: `cargo test -p tensor_vm conformance_vectors_cover_every_consensus_admitted_op --quiet` passed.
- Focused: `cargo test -p tensor_vm cpu_reference_passes_all_admitted_ops --quiet` passed.
- Formatting/whitespace: `cargo fmt --all`, `cargo fmt --check --all`, and `git diff --check` passed.
- Verifier tool: no `tensorvm-verifier` binary/script found in workspace or PATH.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 443 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

## Recent Iterations

### Iteration 110: Mixed-Scale Fixed32 Add/Sub Semantics

Feature capability: allow `Fixed32` `add` and `sub` with different input scales by rescaling the RHS into
the lhs/declaration scale with canonical round-half-to-even semantics before field add/sub.

Validation evidence:
- First/final Gate 0 passed.
- Focused tensor, IR, and conformance tests passed.
- `cargo fmt --all`, `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --quiet`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` remains blocked by missing `cargo-tarpaulin`.
- Feature commit: `ce665a5` (`Implement mixed-scale fixed32 add sub`) pushed to `origin/main`.

### Iteration 109: Fixed-Point Multiply Scale Semantics

Feature capability: make same-scale `Fixed32` multiplication rescale its signed raw product back to the
declared output scale with round-half-to-even semantics instead of preserving scale metadata over raw field
multiplication.
Readiness requirements covered: `upow.md` §3.1/§4.8 fixed-point scale discipline and MVP AC #1/#10
deterministic output roots.
Files/modules likely touched: `tensor`, `ir`, `conformance`, verifier/docs tests.
Parallel subagents to run: not used; available subagent tool requires explicit user delegation.
Parallelizable implementation workstreams: none in this single-writer semantic change.
Canonical owner: `tensor` owns fixed-point element arithmetic; `ir` calls that helper for broadcasted
`mul`; conformance records the vector.
Adapter callers: runtime/verifier/roles consume the same tensor and IR APIs.
Old shortcut being removed: `Fixed32` `mul` kept the input scale while multiplying raw field elements
without canonical rescale.
Regression test that proves the shortcut is gone: fixed-point tensor/IR/conformance multiply tests.
Behavior with local synthetic block production disabled: unchanged; this is pure deterministic execution.
Behavior for producer and non-producer roles: unchanged; receipts replay to the same canonical roots.
Structured evidence source: conformance vector plus exact interpreter output roots.
Finality source: unchanged block finality; affected receipts still pass through canonical verification.
Wire-size and codec boundary: no wire/storage shape change; only deterministic value semantics.
Narrow validation commands: fixed-point tensor/IR/conformance tests.
Broad validation commands before commit: format, diff check, full `tensor_vm`, clippy, release workspace,
final Gate 0, tarpaulin attempt.
Out of scope: fixed-point division/reciprocal, mixed-scale binary ops, CUDA graph execution, public run.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Resumed Gate 0: `cargo test -p tensor_vm local_testnet --release` passed on June 21, 2026.
- Focused: `cargo test -p tensor_vm fixed32_multiply_rescales_to_input_scale_half_even --quiet` passed.
- Focused: `cargo test -p tensor_vm exact_interpreter_executes_fixed32_mul_with_scale_rescale --quiet` passed.
- Focused: `cargo test -p tensor_vm cpu_reference_passes_all_vectors --quiet` passed.
- Focused: `cargo test -p tensor_vm conformance_vectors_are_stable_and_cover_current_ops --quiet` passed.
- Focused: `cargo test -p tensor_vm conformance_vectors_cover_every_consensus_admitted_op --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all`, `cargo fmt --all`, and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 439 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `9bcf389` (`Implement fixed-point multiply rescale`) pushed to `origin/main`.
- Evidence commit: `f708c4b` (`Record fixed-point multiply evidence`) pushed to `origin/main`.

### Iteration 108: Redundant Delayed Reward Claims

Feature capability: carry redundant settlement delay heights into the normal pending receipt reward ledger
once delayed work finally settles.
Canonical owner: `chain::settlement` creates receipt reward claims; `ChainState::pending_receipt_rewards`
owns rooted claim maturity.
Adapter callers: unchanged; status/RPC/storage read the same rooted pending reward map.
Old shortcut being removed: redundant reward delay existed only as a delay record or omitted claim before
settlement.
Regression test that proves the shortcut is gone: `chain::tests::redundant_agreement_quorum_is_required_before_settlement`.
Behavior with local synthetic block production disabled: unchanged; claims are created by shared settlement.
Behavior for producer and non-producer roles: unchanged; inclusion later applies the same max maturity rule.
Structured evidence source: `RedundantSettlementDelayRecord.reward_delay_until_height` and
`PendingReceiptReward.claimable_at_height`.
Finality source: canonical block inclusion still starts the ordinary receipt maturity clock.
Wire-size and codec boundary: no wire or storage codec shape change; existing claim field carries the hold.
Validation target: focused settlement test, format/diff, crate/workspace tests, clippy, final Gate 0,
tarpaulin attempt.
Out of scope: public Tier-C committee evidence, live drand/VRF, Docker rerun, interactive fraud proofs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused: `cargo test -p tensor_vm redundant_agreement_quorum_is_required_before_settlement --quiet` passed.
- Focused: `cargo test -p tensor_vm conflicting_linear_training_roots_do_not_settle --quiet` passed.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 437 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.

Earlier detailed iterations are summarized in the archive to keep this plan compact.

Older recent iterations are summarized in the archive to keep this plan compact.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest full validation is Iteration 111 on June 21, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm fixed32_multiply_rescales_mixed_scales_to_lhs_scale_half_even --quiet
cargo test -p tensor_vm exact_interpreter_executes_fixed32_mul_with_mixed_scales --quiet
cargo test -p tensor_vm conformance_vectors_are_stable_and_cover_current_ops --quiet
cargo test -p tensor_vm cpu_reference_passes_all_vectors --quiet
cargo test -p tensor_vm conformance_vectors_cover_every_consensus_admitted_op --quiet
cargo test -p tensor_vm cpu_reference_passes_all_admitted_ops --quiet
cargo fmt --all
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
cargo test -p tensor_vm local_testnet --release
cargo tarpaulin --workspace --offline
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
- Iterations 41-58: Tensor IR, graph-backed jobs/receipts, exact replay, quantization, Tier-B coverage,
  delayed proposer reward cleanup, and economic helper foundations landed in git history.
- Iterations 30-34: delayed proposer, receipt, challenger, and credit reward-ledger foundations landed in
  commit `5664acb` and related evidence commits.
