# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 120 complete - trace opening availability for receipt disputes.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage, automatic
  unfinalized side-branch deep reorg, graph-backed synthetic jobs, and delayed challenge rewards are
  implemented locally. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner
  TensorWork activation now follows delayed miner receipt reward maturity instead of immediate settlement,
  and settled receipt rewards carry explicit awaiting-inclusion or claimable-height maturity state before release.
  Selected-receipt block openings now expose typed block-check transcript commitments and
  submission-anchored retention deadlines. Redundancy-delayed receipts now have chain-owned state-rooted
  records when quorum-backed work cannot settle because agreement is missing or conflicting, and later
  pending receipt reward claims inherit those redundant reward holds. External randomness beacon records
  can now advance future receipt randomness through a rooted chain command. `Fixed32`
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
  now exposes verified per-op trace openings for receipt dispute evidence anchored by `trace_root`.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: continue p2p trace-opening sampling, Tier-C committee policy, deployed-run economics
  evidence, CUDA graph evidence, or rerun Docker after the `/health` blocker clears.

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
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface including mixed-scale fixed-point `add`/`sub`/`mul`, `Fixed32` `div`, `Fixed32` `matmul`, tensor-owned packed int8 payload APIs, role-owned local graph execution, content-addressed `const_blob` artifact replay, pending external graph job payloads, automatic runtime program fetch, miner/validator graph tensor fetch evidence, and verified per-op trace openings | Continue exact Tier-B verifier coverage, p2p trace-opening/blob dispute sampling, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and public/operator independence evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; status/explorer expose seed-domain, local finalized-beacon round mapping, local validator VRF-seed derivation, external beacon record evidence, and block-hash-ban evidence | Add live drand/VRF client wiring and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit receipt reward maturity state, inclusion-started receipt reward maturity, mature release, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 120: Trace Opening Availability

Feature capability: expose Merkle openings for exact IR per-op trace commitments so graph and tensor
receipts can serve dispute-ready trace evidence anchored by `trace_root`.
Readiness requirements covered: `mvp_spec.md` trace-root availability, `upow.md` future interactive
fraud-proof availability, and the coverage matrix gap for trace-chunk/blob dispute availability.
Canonical owner: exact IR execution trace commitments and receipt replay helpers.
Adapter callers: graph and tensor receipt paths use the same IR execution commitment layout.
Parallel subagents: not used; available subagent tool forbids spawning unless explicitly requested.
Out of scope: full interactive fraud game, durable erasure-coded DA, CUDA graph execution, and Docker
`/health` rerun.

Validation evidence: first/final `cargo test -p tensor_vm local_testnet --release`, focused
`ir::tests::exact_interpreter_executes_hand_built_graph_and_commits_trace` and `jobs::tests`,
`cargo fmt --all`, `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --quiet`,
`cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
Coverage attempt remains blocked: `cargo tarpaulin --workspace --offline` reports no `tarpaulin` command.

### Iteration 119: Explicit Receipt Reward Maturity State

Feature capability: model receipt reward delay directly as `ReceiptRewardMaturity` rather than a magic
height value. Pending receipt rewards start as `AwaitingInclusion`, canonical block inclusion converts
them to `ClaimableAt(height)`, and reward roots/storage commit the maturity tag.
Readiness requirements covered: `mvp_spec.md` reward settlement delay, `upow.md` economics/TensorWork
activation delay, and production evidence for delayed rewards from chain state.
Canonical owner: chain state, reward roots, settlement, block application, and reward release.
Adapter callers: status/RPC continue using the chain-owned pending reward claim view.
Old shortcut being removed: the receipt reward ledger no longer uses `u64::MAX` as an
awaiting-inclusion sentinel.
Regression tests: reward, settlement, block, attestation, and storage tests cover awaiting inclusion,
inclusion-derived claimable heights, mature release, audit delay/voiding, and persistence.
Parallel subagents: not used; available subagent tool forbids spawning unless explicitly requested.
Out of scope: Docker `/health` rerun, deployed-run measurements, and trace-chunk disputes.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused reward maturity tests passed: `chain::tests::rewards`, `chain::tests::settlement`,
  `chain::tests::attestations`, and `storage::chain_state`.
- Formatting/whitespace: `cargo fmt --all`, `cargo fmt --check --all`, and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 453 library tests plus integrations.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by missing `cargo-tarpaulin`.

### Iteration 118: Automatic External Graph Artifact Fetch

Feature capability: resolve missing external graph artifacts automatically at runtime boundaries. Pending
graph job payloads fetch canonical program bodies over bounded `RequestProgram` before retry; miner roles
fetch missing graph input and `const_blob` tensors before execution; validator roles fetch graph input,
output, and `const_blob` tensors before attestation.
Readiness requirements covered: `upow.md` §2 process boundaries, §4 content-addressed programs, §5 tensor
commitments, and §9 verification data availability.
Canonical owner: chain admission still validates graph bodies and receipts; app/node role boundaries only
fetch missing network dependencies and retry canonical commands.
Adapter callers: runtime network ingest, miner role tick, and validator role tick.
Old shortcut being removed: graph program/tensor dependencies had to be preloaded manually outside the
role-loop/network retry path.
Regression tests: pending graph job program fetch/retry, miner graph input/blob fetch before execution,
and validator graph input/output/blob fetch before attestation.
Behavior with local synthetic block production disabled: externally supplied graph jobs can still resolve
their program and tensor artifacts through libp2p request-response.
Behavior for producer and non-producer roles: both use the same bounded artifact fetch and canonical chain
command retry paths.
Structured evidence source: pending payload retry counters plus role remote-fetch reports.
Finality source: unchanged block-vote finality after graph receipts settle normally.
Wire-size and codec boundary: reuses bounded job, program, and tensor codecs; no new wire format.
Out of scope: Docker `/health` rerun, CUDA graph execution, trace-chunk dispute availability, and
interactive fraud proofs.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused graph-artifact tests passed: network pending program fetch, miner input/blob fetch, validator
  input/output/blob fetch, existing graph payload pending/retry, and libp2p propagation.
- Formatting/whitespace: `cargo fmt --all`, `cargo fmt --check --all`, and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm --quiet` passed 453 library tests plus integrations.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by missing `cargo-tarpaulin`.

### Iteration 116: Packed Int8 Tensor Artifact API

Feature capability: expose byte-packed `TVQ8` int8 quantization payloads as first-class `Uint8` tensor
artifacts with tensor-owned constructor/decode methods and descriptor/chunk/opening evidence.
Readiness requirements covered: `upow.md` §3.1/§4.8 canonical dtype/layout, §5 tensor commitments and
openings, §9 verification data availability for tensor chunks, and the documented packed tensor
public-artifact API gap.
Files/modules likely touched: `tensor`, `ir`, `conformance`, Tensor IR/conformance docs/status.
Parallel subagents to run: not used; available subagent tool forbids spawning unless the user explicitly
requests delegation.
Parallelizable implementation workstreams: none in this single-owner tensor/IR/conformance change.
Canonical owner: `tensor` owns packed artifact construction, decode validation, descriptor, chunk, and
opening behavior.
Adapter callers: IR exact replay and conformance vector execution construct/decode packed payload tensors
through `Tensor` methods.
Old shortcut being removed: callers manually wrapped encoded bytes as a flat `Uint8` tensor and manually
checked dtype/scale/rank before free-function decode.
Regression test that proves the shortcut is gone: a packed tensor artifact test verifies descriptor chunks
and openings, round-trips decode, and rejects non-artifact tensors.
Behavior with local synthetic block production disabled: unchanged; this is pure tensor artifact/exact
replay behavior.
Behavior for producer and non-producer roles: unchanged; every role sees the same tensor root/openings.
Structured evidence source: tensor descriptor/opening tests plus existing IR and conformance packed tests.
Finality source: unchanged block finality; affected receipts still pass through canonical verification.
Wire-size and codec boundary: preserves the existing `TVQ8` bytes and normal tensor chunking semantics.
Narrow validation commands: packed tensor artifact test, packed IR replay test, conformance vector/profile
tests.
Broad validation commands before commit: format, diff check, full `tensor_vm`, clippy, release workspace,
final Gate 0, tarpaulin attempt.
Expected observable evidence: packed payloads are constructed as `Uint8` tensors, descriptor/opening proofs
verify over chunked tensor bytes, and malformed/non-artifact tensors reject before decode.
Out of scope: CUDA graph execution, Docker/public run, Tier-C committee/fraud games.
Split trigger: any wire-format or storage codec migration requirement.

Validation evidence:
- First Gate 0 passed before edits; focused packed tensor artifact, packed IR replay, verifier, and
  conformance/profile tests passed; format, diff, full `tensor_vm`, clippy, release workspace, and final
  Gate 0 passed. Tarpaulin remains blocked by missing `cargo-tarpaulin`.
  Feature commit: `6f615f6` (`Add packed int8 tensor artifact APIs`) pushed to `origin/main`.

## Recent Iterations

### Iteration 114: Tensor-Owned Packed Int8 Payload API

Centralized the canonical `TVQ8` packed int8 byte layout in tensor-owned bounded encode/decode APIs and
updated IR/conformance callers. Validation passed; tarpaulin remained blocked. Feature commit: `4fceaeb`.

### Iteration 115: Delayed Reward Path Cleanup

Removed the remaining direct spendable reward-credit test helper. Command, transaction, generic credit,
and telemetry tests now create pending credit rewards, prove pre-maturity claims are blocked, release
through `ReleaseMaturedCreditRewards`, and only then claim/use spendable rewards. Focused/broad validation
passed; tarpaulin remained blocked. Feature commit: `1c65b80` (`Use delayed credit rewards in tests`)
pushed to `origin/main`; evidence commit `59b1cf2` pushed.

### Iteration 117: External Graph Artifact Propagation Evidence

External graph job payloads now stay pending when the program body is missing, and focused libp2p evidence
fetches externally supplied graph bodies plus input tensors before applying the same graph job payload.
Validation passed; tarpaulin remained blocked. Feature commit: `529cb16`; evidence commit: `3120ea5`.

### Iteration 113: Fixed-Point Matmul Accumulation/Range Policy

Feature capability: make `Fixed32` `matmul` accumulate signed raw products in fixed ascending order with a
widened integer accumulator, then rescale once from product scale into the lhs/output scale with canonical
round-half-to-even semantics. Validation passed; tarpaulin remained blocked. Feature commit: `506b020`.

### Iteration 111: Mixed-Scale Fixed32 Multiplication Semantics

Feature capability: allow `Fixed32` `mul` with different input scales by treating the signed product as
scale `lhs_scale + rhs_scale`, then rescaling it into the lhs/output scale with canonical round-half-to-even
semantics.

Validation evidence:
- First/final Gate 0 passed.
- Focused tensor, IR, and conformance tests passed.
- `cargo fmt --all`, `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --quiet`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed.
- `cargo tarpaulin --workspace --offline` remains blocked by missing `cargo-tarpaulin`.
- Feature commit: `4de9463` (`Implement mixed-scale fixed32 multiply`) pushed to `origin/main`.

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

Latest full validation is Iteration 118 on June 21, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm network_ingest_fetches_pending_graph_job_program_before_retry --quiet
cargo test -p tensor_vm miner_role_fetches_remote_graph_inputs_and_const_blobs_before_execution --quiet
cargo test -p tensor_vm validator_role_fetches_remote_graph_const_blobs_before_attesting --quiet
cargo test -p tensor_vm network_event_driver_queues_graph_job_until_program_body_arrives --quiet
cargo test -p tensor_vm libp2p_service_propagates_external_graph_job_artifacts --quiet
cargo test -p tensor_vm pending_payloads_retry_keeps_pending_payloads --quiet
cargo test -p tensor_vm validator_remote_tensor_response_rejects_corrupt_or_mismatched_payloads --quiet
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
