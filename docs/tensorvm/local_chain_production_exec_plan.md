# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 243 in progress: CUDA Field Concat/Stack Graph Kernel/Conformance.
- Current status: v0 work follows the 2026-06-23 owner scope decision: live verified drand consensus
  randomness and local A100 CUDA evidence are in v0 scope; 7-day external public-run evidence is a
  production-launch roadmap milestone. The latest CUDA graph subset now covers scale-0 field
  `matmul`/`add`/`sub`/`mul`/`div`/`clamp`/`sum`/`mean`/`reshape`/`squeeze`/`unsqueeze`/`slice`/
  `tril`/`triu`/`concat`/`stack`/`broadcast`/`transpose`/`scalar_mul`/`relu`/`identity`/`neg`/`abs`/`sign`/`eq`/`gt`/`lt`/`ge`/
  `le`/`where` without claiming fixed-point CUDA graph ops, multi-output split, quantization, or
  full frozen-registry CUDA coverage.
- Current blockers: none gating v0. Former blockers "7-day external run" and "deployed full VRF
  construction" are reclassified to roadmap per the 2026-06-23 scope decision.
- Next action: continue broadening CUDA kernels/conformance for remaining admitted exact ops without CPU
  fallback or overclaiming unsupported frozen-registry coverage.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 241 first `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, runtime profile env-scope tests, Gate 0 | Preserve one transition engine while adding runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection messages, drand, validator reveals, and runtime/peer-book bootstrap config | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, delayed pending reward holds, and state-rooted proposer reward release tombstones | Continue Tier-C committee policy and deployed public-operator evidence |
| Randomness commit/reveal (drand beacon) | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Make verified drand binding the live consensus randomness source; bespoke per-validator VRF is roadmap |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| CUDA miner/runtime + conformance | Partial local A100 evidence | CUDA matmul/add/sub/mul/div/clamp/sum/mean/reshape/squeeze/unsqueeze/slice/tril/triu/concat/stack/broadcast/relu/identity/neg/abs/sign/eq/gt/lt/ge/le/where/scalar_mul/transpose kernels exist in `crates/tensor_vm/kernels/cuda/field_matmul.cu`; native CUDA-feature runtime and miner-role tests pass for current TensorOp, LinearTrainingStep, local synthetic GraphExecution, and supported multi-op field GraphExecution | Continue kernels/conformance for remaining admitted exact ops without CPU fallback |
| Public deployment evidence (7-day run) | Roadmap, not v0 | Public evidence validators/templates exist; reclassified out of v0 scope on 2026-06-23 | Carry as production-launch milestone; do not treat as a v0 blocker |

## Active Feature Iteration

### Iteration 243: CUDA Field Concat/Stack Graph Kernel/Conformance

Feature capability: add CUDA field `concat(dim)` and `stack(dim)` graph execution for variadic scale-0
field tensors using device-side row-major structural-copy kernels, route both ops through
`GpuMinerBackend`, and expand supported CUDA graph/conformance/miner-role fixtures so the remaining
single-output structural join ops are exercised on the local A100 path without CPU fallback.

Readiness requirements covered: `goal.md` v0 CUDA scope decision plus `upow.md` sections 3.1-3.3, 4.7,
4.8, 7, and 16 require bit-exact CUDA evidence for admitted exact ops. `concat` and `stack` are Tier-B
structural ops with deterministic row-major coordinate semantics; this iteration covers only scale-0 field
tensors in the CUDA graph subset.

Ownership boundary:

- Canonical owner: CUDA runtime owns accelerated scale-0 field `concat` and `stack` for canonical
  structural join semantics.
- Adapter callers: CUDA miner readiness, `tvmd miner run --device cuda:N`, role service runtime loop, and
  focused runtime/miner-role tests.
- Old shortcut removed: exact graph `concat`/`stack` stopped at the CUDA graph boundary.
- Regression proof: CUDA-feature runtime tests assert direct CUDA field `concat`/`stack` parity and
  mismatch rejection; supported CUDA graph parity includes kwargs-backed structural joins; miner-role CUDA
  graph receipt tests submit the expanded graph through `BackendKind::GpuMiner`; unsupported CUDA-op
  coverage remains on multi-output `split`.
- Behavior with local synthetic block production disabled: unchanged.
- Behavior for producer and non-producer roles: unchanged.
- Structured evidence source: `ConformanceProfile.passed_ops`, CPU/GPU GraphExecution trace roots,
  miner-role `backend_kind`, direct CUDA structural parity assertions, and explicit unsupported-op errors.
- Finality source: unchanged; no block admission, settlement, voting, rewards, reward maturity, delayed
  claims, TensorWork activation, or finality changes.
- Wire-size and codec boundary: no wire or codec changes; the CUDA C ABI grows only behind
  `--features cuda-kernels`.

Parallel subagents: none. Available subagent tooling currently says not to spawn agents unless the user
explicitly asks for delegation, so the parent remains the single writer.

Out of scope: reward workarounds, immediate reward release, fixed-point structural CUDA ops, split,
CUDA quantization, consensus changes, and public deployment evidence.

Validation evidence:

- Gate 0 first executable acceptance command: `cargo test -p tensor_vm local_testnet --release` passed on
  June 23, 2026 before other acceptance commands in this resumed iteration.
- `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib` passed on June 23, 2026 with
  10 CUDA-feature runtime tests, including direct field `concat`/`stack` parity, supported CUDA graph
  parity, and unsupported `split` rejection.
- `cargo fmt --check` passed on June 23, 2026.
- `cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
  miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend` passed on
  June 23, 2026 with the supported miner-role CUDA graph fixture extended through `tril`/`triu` ->
  `concat` -> `stack`.
- `cargo test -p tensor_vm --lib` passed on June 23, 2026 with 573 tests.
- Post-change Gate 0 `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026 with 5
  release local_testnet library tests and 1 service-gateway CLI test.
- `cargo test --workspace --release` passed on June 23, 2026 with 14 experiments tests, 573 tensor_vm
  library tests, 9 tvmd CLI tests, 50 tvmd runtime tests, 1 local CPU compose test, 1 explorer library
  test, and 2 explorer CLI tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed on June 23, 2026.
- `cargo test -p tensor_vm --features cuda-kernels --release` passed on June 23, 2026 with 580
  CUDA-feature tensor_vm library tests, 9 tvmd CLI tests, 54 tvmd runtime tests, and doc-tests.
- `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings` passed on
  June 23, 2026.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed on
  June 23, 2026 with 588 instrumented tests and 84.95% workspace line coverage
  (23831/28052 lines).

### Iteration 242: CUDA Field Triangular Graph Kernel/Conformance

Feature capability: add CUDA field `tril(diagonal)` and `triu(diagonal)` graph execution for scale-0
rank-2 field tensors using a device-side triangular-mask copy kernel, route both ops through
`GpuMinerBackend`, and expand supported CUDA graph/conformance/miner-role fixtures so the exact
single-output triangular structural ops are exercised on the local A100 path without CPU fallback.

Readiness requirements covered: `goal.md` v0 CUDA scope decision plus `upow.md` sections 3.1-3.3, 4.7,
4.8, 7, and 16 require bit-exact CUDA evidence for admitted exact ops. `tril`/`triu` are Tier-B
structural ops with deterministic rank-2 row/column mask semantics; this iteration covers only scale-0
field tensors in the CUDA graph subset.

Ownership boundary:

- Canonical owner: CUDA runtime owns accelerated scale-0 rank-2 field `tril`/`triu` for canonical
  triangular structural semantics.
- Adapter callers: CUDA miner readiness, `tvmd miner run --device cuda:N`, role service runtime loop, and
  focused runtime/miner-role tests.
- Old shortcut removed: exact graph `tril`/`triu` stopped at the CUDA graph boundary.
- Regression proof: CUDA-feature runtime tests assert direct CUDA field `tril`/`triu` parity and rank
  rejection; supported CUDA graph parity includes kwargs-backed triangular ops; miner-role CUDA graph
  receipt tests submit the expanded graph through `BackendKind::GpuMiner`; unsupported CUDA-op coverage
  moved to still-unsupported `concat`.
- Behavior with local synthetic block production disabled: unchanged.
- Behavior for producer and non-producer roles: unchanged.
- Structured evidence source: `ConformanceProfile.passed_ops`, CPU/GPU GraphExecution trace roots,
  miner-role `backend_kind`, direct CUDA triangular parity assertions, and explicit unsupported-op errors.
- Finality source: unchanged; no block admission, settlement, voting, rewards, reward maturity, delayed
  claims, TensorWork activation, or finality changes.
- Wire-size and codec boundary: no wire or codec changes; the CUDA C ABI grows only behind
  `--features cuda-kernels`.

Parallel subagents: none. Available subagent tooling currently says not to spawn agents unless the user
explicitly asks for delegation, so the parent remains the single writer.

Out of scope: reward workarounds, immediate reward release, fixed-point structural CUDA ops,
split/concat/stack, CUDA quantization, consensus changes, and public deployment evidence.

Validation evidence:

- Gate 0 first executable acceptance command: `cargo test -p tensor_vm local_testnet --release` passed on
  June 23, 2026 before other acceptance commands in this resumed iteration.
- `cargo test -p tensor_vm --features cuda-kernels runtime::tests --lib` passed on June 23, 2026 with
  10 CUDA-feature runtime tests, including direct field `tril`/`triu` parity, supported CUDA graph parity,
  and unsupported `concat` rejection.
- `cargo test -p tensor_vm --features cuda-kernels --test tvmd_runtime
  miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend` passed on
  June 23, 2026 with the supported miner-role CUDA graph fixture extended through `slice` ->
  `unsqueeze` -> `triu` -> `tril`.
- `cargo fmt --check` passed on June 23, 2026.
- `cargo test -p tensor_vm --lib` passed on June 23, 2026 with 573 tests.
- Post-change Gate 0 `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026 with 5
  release local_testnet library tests and 1 service-gateway CLI test.
- `cargo test --workspace --release` passed on June 23, 2026 with 14 experiments tests, 573 tensor_vm
  library tests, 9 tvmd CLI tests, 50 tvmd runtime tests, 1 local CPU compose test, 1 explorer library
  test, and 2 explorer CLI tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed on June 23, 2026.
- `cargo test -p tensor_vm --features cuda-kernels --release` passed on June 23, 2026 with 580
  CUDA-feature tensor_vm library tests, 9 tvmd CLI tests, 54 tvmd runtime tests, and doc-tests.
- `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings` passed on
  June 23, 2026.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed on
  June 23, 2026 with 588 instrumented tests and 84.96% workspace line coverage
  (23831/28050 lines).
- `git diff --check` passed on June 23, 2026.
- Commit: `85cb8c3` (`Add CUDA field triangular graph support`).
- Push: `85cb8c3` pushed to `origin/main` on June 23, 2026.

## Recent Iterations

### Iteration 241: CUDA Field Slice Graph Kernel/Conformance

Added CUDA field `slice(dim,start,end)` graph execution for scale-0 field tensors using a device-side
row-major slice-copy kernel, routed graph `slice` through `GpuMinerBackend`, and expanded supported CUDA
graph/conformance/miner-role fixtures. Validation included Gate 0, focused CUDA runtime and miner-role
tests, default/CUDA test suites, workspace release/clippy, Tarpaulin, CUDA clippy, and `git diff --check`.
Commit `99cfe2b` (`Add CUDA field slice graph support`) and metadata commit `da06019` pushed to
`origin/main` on June 23, 2026.

### Iteration 240: CUDA Field Squeeze/Unsqueeze Graph Kernel/Conformance

Added CUDA field `squeeze(dim=...)` and `unsqueeze(dim=...)` graph execution for scale-0 field tensors
using canonical structural shape validation plus the existing device-side row-major identity copy. CUDA
runtime direct parity and mismatch tests, supported CUDA graph parity, and miner-role CUDA graph receipt
tests passed. Validation included Gate 0, `cargo fmt --check`, default library tests, workspace release,
workspace clippy, Tarpaulin, full CUDA release, CUDA clippy, and `git diff --check`. Commit `c720931`
(`Add CUDA field squeeze graph support`) and metadata commit `7f95a4f` pushed to `origin/main` on
June 23, 2026.

### Iteration 239: CUDA Field Reshape Graph Kernel/Conformance

Added CUDA field `reshape(shape=...)` graph execution for scale-0 field tensors using a device-side
row-major identity copy after canonical shape-product validation. CUDA runtime direct parity and
shape-mismatch tests, supported CUDA graph parity, and miner-role CUDA graph receipt tests passed.
Validation included Gate 0, default/CUDA test suites, workspace release/clippy, Tarpaulin, CUDA clippy,
and `git diff --check`. Commit `012ff56` (`Add CUDA field reshape graph support`) and metadata commit
`9518bd7` pushed to `origin/main` on June 23, 2026.

## Decision Log

- 2026-06-23 owner scope decision: v0 uses verified drand as the canonical randomness source; bespoke
  per-validator VRF is roadmap.
- 2026-06-23 owner scope decision: CUDA is in v0 scope and locally provable on the A100 box.
- 2026-06-23 owner scope decision: the 7-day external public run is a production-launch roadmap milestone,
  not a v0 blocker.
- Rewards must remain delayed claims with maturity/challenge holds. Do not add immediate reward-release
  workarounds.
- There is no standalone verifier binary. Verifier evidence comes from existing runtime, graph,
  conformance, verify, and role tests.
- Parallel subagents are not used unless the user explicitly asks for delegation; keep the parent as the
  single writer.

## Validation Evidence

Latest full validation set for Iteration 243 is recorded in the Active Feature Iteration section. Gate 0
was the first executable acceptance command after the required doc/context reads.

## Archive

- Iterations 238 and earlier progressively broadened CUDA graph coverage through field `mean`,
  `broadcast`, `sum`, `clamp`, `div`, comparison masks, `where`, unary field ops, linear-step CUDA paths,
  and the CUDA graph conformance boundary. Their commit anchors remain in git history before
  `012ff56`.
- Earlier local-chain readiness work established the shared chain engine, role-owned miner and validator
  loops, libp2p/node payload ingestion, delayed reward claims, proposer reward holds, trace-bisection
  economics, validator audit/slashing paths, public evidence scaffolding, and local CPU Gate 0.
