# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 232 complete pending commit/push: CUDA Field Comparison Graph Kernels/Conformance.
- Current status: v0 work is redirected by the 2026-06-23 owner scope decision toward live verified drand
  consensus randomness and local A100 CUDA evidence. Iteration 232 targets same-shape CUDA field
  comparison graph kernels for `eq`, `gt`, `lt`, `ge`, and `le`, extending the supported
  CUDA graph/conformance subset without claiming broadcasting, fixed-point comparisons, reductions,
  quantization, structural ops, or full frozen-registry CUDA coverage.
- Current blockers: none gating v0. Former blockers "7-day external run" and "deployed full VRF
  construction" are reclassified to roadmap per the 2026-06-23 scope decision.
- Next action: implement, validate, commit, and push Iteration 232.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 231 first and post-change `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, runtime profile env-scope tests, Gate 0 | Preserve one transition engine while adding runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection messages, drand, validator reveals, and runtime/peer-book bootstrap config | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, delayed pending reward holds, and state-rooted proposer reward release tombstones | Continue Tier-C committee policy and deployed public-operator evidence |
| Randomness commit/reveal (drand beacon) | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Make verified drand binding the live consensus randomness source; bespoke per-validator VRF is roadmap |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| CUDA miner/runtime + conformance | Partial local A100 evidence | CUDA matmul/add/sub/mul/relu/identity/neg/abs/sign/eq/gt/lt/ge/le/scalar_mul/transpose kernels exist in `kernels/cuda/field_matmul.cu`; native CUDA-feature runtime and miner-role tests pass for current TensorOp, LinearTrainingStep, local synthetic GraphExecution, and supported multi-op field GraphExecution | Add kernels/conformance for remaining admitted exact ops without CPU fallback |
| Public deployment evidence (7-day run) | Roadmap, not v0 | Public evidence validators/templates exist; reclassified out of v0 scope on 2026-06-23 | Carry as production-launch milestone; do not treat as a v0 blocker |

## Active Feature Iteration

### Iteration 232: CUDA Field Comparison Graph Kernels/Conformance

Feature capability: add exported CUDA same-shape field comparison kernels for `eq`, `gt`, `lt`, `ge`, and
`le`, route those exact graph ops through `GpuMinerBackend`, and expand the supported CUDA graph
conformance/miner-role fixture so comparison masks can be produced by the CUDA graph path.

Readiness requirements covered: `goal.md` v0 CUDA scope decision plus `upow.md` §3.2/§3.3, §4.7, §4.8,
and §16 require bit-exact CUDA evidence for admitted exact comparison ops while keeping unsupported CUDA
coverage explicitly gated.

Canonical owner: CUDA runtime owns accelerated same-shape field comparison execution and conformance
reporting; `TensorGraph` continues to own canonical comparison semantics and dtype expectations.

Adapter callers: CUDA miner readiness, `tvmd miner run --device cuda:N`, role service runtime loop, and
focused runtime/miner-role tests.

Old shortcut removed: exact comparison graph ops currently stop at the CUDA graph boundary, so CUDA
conformance cannot include comparison masks even though the CPU canonical interpreter supports them.

Regression test that proves the shortcut is gone: CUDA-feature runtime tests will assert direct CUDA
`eq`/`gt`/`lt`/`ge`/`le` parity against canonical CPU field comparison masks, supported CUDA graph parity
will include those ops, and miner-role CUDA graph receipt tests will submit the expanded graph through
`BackendKind::GpuMiner`.

Behavior with local synthetic block production disabled: unchanged; graph execution uses existing chain
jobs and backend-selected receipt execution.

Behavior for producer and non-producer roles: unchanged; validators/proposers consume the same graph
receipts and finality logic, and miners do not produce blocks.

Structured evidence source: `ConformanceProfile.passed_ops`, CPU/GPU GraphExecution trace roots,
miner-role `backend_kind`, direct CUDA comparison kernel parity assertions, and explicit unsupported-op
CUDA graph errors for still-unsupported ops.

Finality source: unchanged; this iteration does not alter block admission, settlement, voting, rewards, or
finality.

Wire-size and codec boundary: no wire or codec changes; existing graph/job/receipt payload codecs remain
unchanged.

Parallel subagents: none. The decision log says not to spawn subagents unless the user explicitly asks for
delegation; parent will do single-writer implementation and direct code review.

Tests/checkers/docs to add or update: CUDA runtime direct kernel parity, CUDA graph conformance profile,
miner-role supported CUDA graph fixture, `upow.md`, coverage matrix, implementation status, tarpaulin
report, and this execution plan.

Narrow validation commands: `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib`;
`cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend`; default unsupported
CUDA-feature boundary test.

Broad validation commands before commit: `cargo fmt --check`; `cargo test -p tensor_vm --lib`;
`cargo test --workspace --release`; `cargo clippy --workspace --all-targets -- -D warnings`;
post-change Gate 0; `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`;
full CUDA release and CUDA-feature clippy.

Expected observable evidence: CUDA comparison masks match CPU canonical `0/1` `Int32` tensors for
same-shape field inputs, CUDA graph CPU/GPU receipt roots match with comparison ops included, and GPU
conformance only reports those comparison ops after the parity case passes.

Out of scope: CUDA broadcasting, fixed-point comparison rescale, bool/broadcast masks, reductions,
quantization, structural ops, reward logic, consensus changes, and public deployment evidence.

Split trigger: split smaller if CUDA comparison mask dtype handling collides with graph typing, if direct
kernel parity fails on A100, or if expanding the miner-role fixture requires unrelated graph receipt
changes.

Validation evidence:
- Gate 0 first executable acceptance command: `cargo test -p tensor_vm local_testnet --release` passed on
  June 23, 2026 before other acceptance commands in this resumed iteration.
- CUDA runtime module: `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib` passed, 10
  tests, covering direct same-shape field comparison parity, supported multi-op graph parity with the
  comparison ops, unsupported-op rejection, and GPU conformance subset assertions.
- CUDA miner-role multi-op graph: `cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
  miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend` passed.
- Default unsupported-build boundary: `cargo test -p tensor_vm --test tvmd_runtime
  miner_role_supported_multi_op_graph_cuda_device_selection_reaches_gpu_backend_without_cuda_feature`
  passed.
- Broad default library suite: `cargo test -p tensor_vm --lib` passed, 573 tests.
- Workspace release suite: `cargo test --workspace --release` passed.
- Lints and hygiene: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings` passed.
- Post-change Gate 0: `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026.
- Full CUDA-feature release sweep: `cargo test -p tensor_vm --features cuda-kernels --release` passed with
  580 TensorVM library tests and 54 `tvmd_runtime` tests, including CUDA miner-role TensorOp,
  LinearTrainingStep, local graph, and supported multi-op graph execution through `GpuMinerBackend`.
- Coverage: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed
  with 588 instrumented tests and 85.03% workspace line coverage, 23831/28027 lines covered. CUDA-feature
  native paths are validated by the focused and release `--features cuda-kernels` commands above, not by
  the portable default tarpaulin run.
- Commit/push: pending.

### Iteration 231: CUDA Field Unary Graph Kernels/Conformance

Feature capability: add exported CUDA same-shape field unary kernels for `identity`, `neg`, `abs`, and
`sign`, route those exact graph ops through `GpuMinerBackend`, and expand the supported CUDA graph
conformance/miner-role fixture so the CUDA subset covers the exact field unary ops already implemented by
the CPU IR interpreter.

Readiness requirements covered: `goal.md` v0 CUDA scope decision plus `upow.md` §3.2/§3.3, §4.7, and
§16 require local A100 CUDA evidence for admitted exact `F_p` ops while keeping unsupported CUDA coverage
explicitly gated.

Canonical owner: CUDA runtime owns accelerated field unary execution and conformance reporting;
`TensorGraph` continues to own canonical graph semantics; miner role only selects the requested backend
and submits the resulting canonical receipt.

Adapter callers: CUDA miner readiness, `tvmd miner run --device cuda:N`, role service runtime loop, and
focused runtime/miner-role tests.

Old shortcut removed: admitted exact field unary graph ops previously failed at the CUDA graph boundary,
forcing CUDA conformance to omit them even though the CPU canonical interpreter supported them.

Regression evidence: CUDA-feature runtime tests assert direct CUDA `identity`/`neg`/`abs`/`sign` parity
with CPU signed-field semantics, the supported CUDA graph parity case includes those ops, and the GPU
profile passes those op names; miner-role CUDA graph receipt tests submit the expanded multi-op graph
through `BackendKind::GpuMiner`.

Behavior with local synthetic block production disabled: unchanged; graph execution uses existing chain
jobs and backend-selected receipt execution.

Behavior for producer and non-producer roles: unchanged; validators/proposers consume the same graph
receipts and finality logic, and miners do not produce blocks.

Structured evidence source: `ConformanceProfile.passed_ops`, CPU/GPU GraphExecution trace roots,
miner-role `backend_kind`, direct CUDA kernel parity assertions, and explicit unsupported-op CUDA graph
errors for still-unsupported ops.

Finality source: unchanged; this iteration does not alter block admission, settlement, voting, rewards, or
finality.

Wire-size and codec boundary: no wire or codec changes; existing graph/job/receipt payload codecs remain
unchanged.

Parallel subagents: none. The available subagent tool forbids spawning unless the user asks for
delegation; parent did single-writer implementation and direct code review.

Implementation summary: CUDA now exports same-shape field `identity`, `neg`, `abs`, and `sign` kernels,
Rust bindings `cuda::field_identity`, `cuda::field_neg`, `cuda::field_abs`, and `cuda::field_sign`, and
`GraphExecution` dispatch branches for those field tensor ops in `GpuMinerBackend`. The supported CUDA
graph conformance case and miner-role supported graph fixture now include
`matmul -> add -> sub -> mul -> transpose -> scalar_mul -> relu -> neg -> abs -> sign -> identity`.
GPU conformance reporting now marks those unary ops only after the CPU/GPU graph parity case passes, while
fixed-point unary semantics, `round`, `div`, reductions, comparisons, quantization, and structural ops
remain unclaimed at the CUDA boundary.

Validation evidence:
- Gate 0 first executable acceptance command: `cargo test -p tensor_vm local_testnet --release` passed on
  June 23, 2026 before other acceptance commands in this resumed iteration.
- CUDA runtime module: `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib` passed, 10
  tests, covering direct field unary parity, supported multi-op graph parity with the unary ops,
  unsupported-op rejection, and GPU conformance subset assertions.
- CUDA miner-role multi-op graph: `cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
  miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend` passed.
- Default unsupported-build boundary: `cargo test -p tensor_vm --test tvmd_runtime
  miner_role_supported_multi_op_graph_cuda_device_selection_reaches_gpu_backend_without_cuda_feature`
  passed.
- Broad default library suite: `cargo test -p tensor_vm --lib` passed, 573 tests.
- Workspace release suite: `cargo test --workspace --release` passed.
- Lints and hygiene: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings` passed. The CUDA
  clippy run first exposed a `type_complexity` lint in the touched helper; it was fixed with a local type
  alias and rerun successfully.
- Post-change Gate 0: `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026.
- Full CUDA-feature release sweep: `cargo test -p tensor_vm --features cuda-kernels --release` passed with
  580 TensorVM library tests and 54 `tvmd_runtime` tests, including CUDA miner-role TensorOp,
  LinearTrainingStep, local graph, and supported multi-op graph execution through `GpuMinerBackend`.
- Coverage: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed
  with 588 instrumented tests and 85.03% workspace line coverage, 23831/28026 lines covered. CUDA-feature
  native paths are validated by the focused and release `--features cuda-kernels` commands above, not by
  the portable default tarpaulin run.
- Commit `0e759f4` (`Add CUDA field unary graph support`) pushed to `origin/main` on June 23, 2026.

## Recent Iterations

### Iteration 230: CUDA Field Mul Graph Kernel/Conformance

Feature capability: added an exported CUDA elementwise field `mul` kernel and routed field `mul`
GraphExecution ops through `GpuMinerBackend`, broadening CUDA graph conformance without falling back to
CPU or claiming fixed-point/mixed-scale `mul`.

Evidence: CUDA runtime tests covered direct `field_mul` parity, supported multi-op graph parity with
`mul`, unsupported-op rejection, and GPU conformance subset assertions. CUDA miner-role multi-op graph and
default unsupported-build boundary tests passed. Broad default library, workspace release, clippy,
tarpaulin, and post-change Gate 0 passed.

Commit `78540b2` (`Add CUDA field mul graph support`) pushed to `origin/main` on June 23, 2026.
Metadata commit `e9e75c7` (`Record CUDA field mul graph push`) pushed to `origin/main` on June 23, 2026.

### Iteration 229: CUDA Supported Graph Op Conformance Boundary

Feature capability: broadened CUDA GraphExecution evidence from the current synthetic `add -> relu` graph
to a supported multi-op field graph covering the then-implemented CUDA graph op set
`add`/`sub`/`matmul`/`transpose`/`relu`/`scalar_mul`, while making `GpuMinerBackend` conformance reporting
name only CUDA ops actually exercised.

Commit `eb6ac34` (`Tighten CUDA graph conformance boundary`) and metadata commit `131bd10` pushed to
`origin/main` on June 23, 2026.

## Decision Log

- `tensorvm-verifier` is not a repository binary. Validation uses the `tvmd` CLI surfaces, tests, clippy,
  tarpaulin, focused CUDA runs, and direct code review.
- Do not spawn subagents unless the user explicitly asks for delegation.
- 2026-06-23 owner override: drand is the canonical v0 randomness beacon. Bespoke per-validator VRF is
  roadmap; v0 §10 = verified drand round → chain-epoch binding + validator commit→reveal.
- 2026-06-23 owner override: real CUDA miner/runtime + per-op CUDA conformance is in v0 scope and
  provable on the local A100×8 box, not deployment-gated.
- 2026-06-23 owner override: the 7-day external public run + full public-deployment evidence is roadmap,
  not a v0 done gate.
- Reward claims remain delayed and chain-owned; valid matured claims become spendable only through
  beneficiary `ClaimReward`, while voided/prunable claims may be swept without credit.
- Index-consistency ops `gather`/`scatter`/`embedding` remain registry vocabulary only and cannot be
  consensus program bodies until their index-consistency proofs exist.

## Validation Evidence

- Latest Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026.
- Latest broad default validation: `cargo test -p tensor_vm --lib`, `cargo test --workspace --release`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check`
  passed on June 23, 2026.
- Latest CUDA validation: `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib`,
  `cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
  miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend`,
  `cargo test -p tensor_vm --features cuda-kernels --release`, and
  `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings` passed on
  June 23, 2026.
- Latest coverage: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`
  passed on June 23, 2026 with 588 instrumented tests and 85.03% line coverage, 23831/28027 lines
  covered.
- Latest commit/push: pending Iteration 232; previous pushed implementation commit is `0e759f4`
  (`Add CUDA field unary graph support`) on `origin/main`.

## Archive

- Iterations 224-228: runtime randomness now defaults to verified public drand; accepted verified drand
  remains finalized consensus randomness across block application; local CUDA build/architecture detection
  was repaired for A100-compatible builds; `tvmd miner run --device cuda:N` reaches `GpuMinerBackend` for
  TensorOp, LinearTrainingStep, and the local synthetic GraphExecution path.
- Iterations 214-223: public evidence gates were tightened for deployed service health/content,
  block/finality/raw operational evidence, randomness, validator-VRF lifecycle, and chain-accepted drand
  exports. These gates remain roadmap production-launch evidence, not v0 blockers.
- Iterations 203-213: public randomness/raw evidence gates, bootstrap/runtime profile scoping, and
  index-consistency admission boundaries were tightened. Notable commits include `81e673c`,
  `f80c181`, `3d4789f`, `2e52ef5`, and `e9e75c7` in `origin/main` history.
- Iterations 187-202: chain-owned verifier bandwidth evidence, public randomness evidence raw-record
  gates, mixed-dtype conformance vectors, trace-bisection DoS admission bounds, isolated trace-bisection
  timeout policy, and reward sweep boundary naming were implemented and documented in prior commits.
