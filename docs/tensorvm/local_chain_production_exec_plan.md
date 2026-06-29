# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 247 COMPLETE — live interprocess Tier-C §8.2 interactive fraud proof proven
  (opt-in malicious-miner caught + slashed via live trace-bisection). Iteration 246 COMPLETE — live
  interprocess Tier-C §8.1 committee settlement is proven and is now a standing local-cpu gate. With committee jobs on by default, the checker passes with
  committee settlement required: all committee receipts settle (e.g. `live_settled_committee_receipt_count`
  equal to `live_committee_receipt_count`, 0 escalations), all operators converge. Iteration 245 (Tier-C
  verification-ladder thread + producer plateau fix) is complete and pushed.
- Current status: v0 work follows the 2026-06-23 owner scope decision (live verified drand + local A100
  CUDA in scope; 7-day external public-run roadmap). Two threads are live: (1) the CUDA graph-op coverage
  thread (through `split`, Iteration 244); (2) the Tier-C verification-ladder thread (Iteration 245) which
  landed canonical fixed-point transcendental references (`exp`/`log`/`sqrt`/`sigmoid`/`tanh`/`silu`/
  `gelu`/`softmax`/`layer_norm`/`rmsnorm`, CPU + CUDA bit-exact), the §8.1 redundancy+committee verifier,
  §8.1 committee settlement wiring, §8.2 Tier-C interactive fraud proofs, §8.1→§8.2 escalation, committee
  agreement-root audit hardening, the Fixed32 `mean` round-half-even fix, and the synthetic-producer
  monotonic-nonce plateau fix that unblocks continuous live production.
- Scope note (avoid over-claiming): §4.8 transcendental ops are admitted via the §8.1 **committee** path
  (committee-trust, not exact-verified). This is v0-legitimate (§8.1 is v0) but must be documented per §14
  as committee-trust, not Tier-A/B exact security. Exact verifiers/soundness bounds for transcendentals
  remain roadmap (§4.8).
- Current blockers (none gating v0): both live Tier-C milestones are proven — §8.1 committee settlement
  (standing gate) and §8.2 interactive fraud proof (opt-in malicious-miner scenario). Former blockers
  "7-day external run" and "deployed full VRF construction" remain roadmap.
- Next action: continue CUDA multi-output graph-op coverage (parallel thread). Optionally fold the §8.2
  malicious-miner dispute scenario into CI as a separate chaos run. The transcendental exact-verification
  (vs committee-trust) for §4.8 remains roadmap.

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
| §8.1 Tier-C redundancy + committee verification | Implemented (Iter 245) | Committee verifier core + seed-independent agreement root, `Committee` op-admission policy + committee execution path, settlement gate with delayed settlement/escalation on disagreement, audit verifies committee agreement root, honest-majority calibration, miner role produces Tier-C committee receipts (commits `94b9fff`,`ba395dd`,`f2b524b`,`57135ec`,`19da56b`) | Prove live across real processes (Iter 246) |
| §8.2 Tier-C interactive fraud proofs | Implemented (Iter 245) | Trace-bisection referee re-executes Tier-C ops via `validate_for_committee`, runtime challenger auto-opens Tier-C disputes, §8.1→§8.2 escalation routing, full dispute test (open→isolate→referee→slash) (commits `000481f`,`326c4c3`) | Prove live dispute across real processes (Iter 246) |
| Fixed-point transcendental references | Implemented (Iter 245), committee-admitted | Canonical Q32 `exp`/`log`/`sqrt`/`sigmoid`/`tanh`/`silu`/`gelu`/`softmax` + composed `layer_norm`/`rmsnorm`, CPU + bit-exact CUDA, conformance vectors, exhaustive s=16 accuracy proof, Fixed32 `mean` round-half-even fix (commits `b3b8984`,`3a48598`,`5ff941f`) | Exact verifiers/soundness bounds remain roadmap (§4.8); keep §14 committee-trust framing |
| Live interprocess Tier-C §8.1 committee settlement | DONE (Iter 246), standing gate | Committee jobs on by default; checker requires committee settlement and passes — all committee receipts settle live across 15 processes, all operators converge, 0 escalations. Fixes: committee body fetch/serve (`d173f1d`), committee output over-fetch (`427650e`), gossip-announce of job input tensors + tensor-codec scale preservation (`9197ead`), validator attests committee receipts without the output tensor (`01050ba`) | Keep as standing gate |
| Live interprocess Tier-C §8.2 interactive fraud proof | DONE (Iter 247), opt-in scenario | Opt-in malicious-miner mode (`379ee9a`) submits a non-canonical Tier-C committee receipt; honest challengers open a live trace-bisection dispute and the referee re-executes the isolated op → invalid-output slash. Verified live with the dispute gate: 80 trace-bisection challenges opened, 7 invalid-output slashes, while §8.1 honest committee receipts kept settling; checker EXIT=0 with both gates required | Optionally add to CI as a chaos run |
| Randomness commit/reveal (drand beacon) | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Make verified drand binding the live consensus randomness source; bespoke per-validator VRF is roadmap |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| CUDA miner/runtime + conformance | Partial local A100 evidence | CUDA matmul/add/sub/mul/div/clamp/sum/mean/reshape/squeeze/unsqueeze/slice/tril/triu/concat/stack/broadcast/relu/identity/neg/abs/sign/eq/gt/lt/ge/le/where/scalar_mul/transpose kernels exist in `crates/tensor_vm/kernels/cuda/field_matmul.cu`; native CUDA-feature runtime and miner-role tests pass for current TensorOp, LinearTrainingStep, local synthetic GraphExecution, and supported multi-op field GraphExecution | Continue kernels/conformance for remaining admitted exact ops without CPU fallback |
| Public deployment evidence (7-day run) | Roadmap, not v0 | Public evidence validators/templates exist; reclassified out of v0 scope on 2026-06-23 | Carry as production-launch milestone; do not treat as a v0 blocker |

## Active Feature Iteration

### Iteration 246 (planned): Live Interprocess Tier-C Evidence

Feature capability: prove, across the 15 real local-cpu node processes, that (1) a Tier-C committee
GraphExecution receipt settles only on honest-majority agreement on the deterministic agreement root, and
(2) a Tier-C committee disagreement drives a live §8.2 interactive fraud-proof dispute (open → trace
bisection → on-chain referee re-execution → slash + challenger reward). This is the goal.md readiness gate
for the Tier-C verification ladder; the chain machinery + unit/integration tests already exist (Iter 245),
this iteration adds the live cross-process emission, fault injection, and checker assertions.

Ownership boundary (to fill before edits):

- Canonical owner: synthetic job source emits an opt-in Tier-C committee graph; chain settlement +
  trace-bisection own the committee gate and dispute (already canonical from Iter 245).
- Adapter callers: live producer (`produce_and_publish_synthetic_job_with_store`), role runtime loop,
  status snapshot, local-cpu checker.
- Old shortcut being removed: no live Tier-C job source exists yet, so committee settlement / Tier-C
  disputes are only proven by in-process tests, not cross-process.
- Fault injection: a clearly-named opt-in malicious-miner mode (env/profile flag) that emits a
  non-canonical Tier-C root to trigger committee disagreement → escalation, reachable only when explicitly
  enabled; never on by default.
- Structured evidence source: typed status snapshot fields for committee settlement + Tier-C dispute
  outcome; checker reads them (no hardcoded booleans, no error-string matching).

Planned files: `crates/tensor_vm/src/scheduler.rs` (opt-in Tier-C committee job), `app/network.rs` /
profile flag, status snapshot owner, `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`, focused
role/runtime tests, `docs/tensorvm/upow.md` §8.1/§8.2 live-evidence status.

Validation: lib + clippy + fmt + Gate 0, then Docker build + 15-node compose + checker with the
fault-injection scenario.

Out of scope: exact transcendental verifiers/soundness bounds (roadmap §4.8), reward workarounds,
consensus/finality changes, 7-day external public-run evidence.

## Recent Iterations

### Iteration 247 (COMPLETE): Live Interprocess Tier-C §8.2 Interactive Fraud Proof

A malicious miner is now caught and slashed live by the interactive fraud proof. Opt-in fault injection
(`TENSORVM_LOCAL_CPU_MALICIOUS_COMMITTEE_MINER`, off by default) makes a miner submit a non-canonical
Tier-C committee receipt — honest inputs and claimed output roots, but a tampered op trace
(`try_malicious_committee_bundle`). Honest miners gossip their served output tensors so committee
validators / challengers can re-execute and detect the disagreement; the runtime challenger opens a live
trace-bisection dispute (open → bisect → referee re-execution of the isolated op → invalid-output slash).
Exposed `trace_bisection_challenge_count` + `invalid_output_slash_count` in the overview and added an
opt-in checker gate `TENSORVM_LOCAL_CPU_REQUIRE_COMMITTEE_DISPUTE`.

Live evidence (one malicious miner among honest ones, both gates required): checker EXIT=0 with
`live_trace_bisection_challenge_count=80`, `live_invalid_output_slash_count=7`, while §8.1 honest committee
receipts kept settling (`live_settled_committee_receipt_count` advancing) and all operators converged
(height 64). So §8.1 honest-majority settlement and §8.2 1-of-N fraud-proof slashing run simultaneously on
real processes. Commit `379ee9a`; regression test `malicious_committee_miner_submits_a_disputable_tier_c_receipt`;
601 lib + workspace tests pass, clippy + fmt clean.

### Iteration 246 (COMPLETE): Live Interprocess Tier-C §8.1 Committee Settlement

Live Tier-C committee settlement now works across the 15 real local-cpu processes and is a standing gate
(committee jobs on by default; checker requires committee settlement). Evidence: checker EXIT=0 with
`live_tier_c_committee_settlement_required=true`, `live_settled_committee_receipt_count` == 17/17 (all
committee receipts settled), all operators converged (min height 24+), 0 escalations. Validation: 599 lib
tests + workspace tests pass, clippy + fmt clean.

The five fixes that unblocked it (each a real correctness bug on the live libp2p path):

- `d173f1d` libp2p program-body fetch/serve used `validate_for_consensus`, rejecting Tier-C bodies — the
  committee graph body could never propagate.
- `427650e` committee receipts over-fetched output tensors they never need (verification re-executes from
  inputs).
- `9197ead` (two interlocking fixes): (1) job input tensors were pull-only over direct-peer
  request/response, but the producer that holds them is not a direct peer of the miners that must execute
  the job — so committee jobs never ran. Added `NewJobInputTensorPayload` gossip on the Jobs topic; the
  producer announces each input when it publishes a graph job, gossipsub relays it multi-hop, receivers
  verify it against its commitment root before storing/serving. (2) `encode/decode_tensor_payload` dropped
  the fixed-point scale, corrupting the commitment root of every Fixed32 tensor on the wire so it failed
  content-address verification on BOTH gossip and pull — fixed to preserve scale.
- `01050ba` the validator attestation-bundle builder required the receipt's output tensors (early-returns
  None), so committee receipts (whose outputs are not fetched) could never build a bundle → validators
  never attested them (attestation_count=0) → no committee agreement. Committee bundles now build without
  output tensors.

Diagnosis was via temporary gated tracing (`TENSORVM_TRACE_COMMITTEE`, since removed) which showed
required roots returning `Missing` from all peers and then, after the DA fixes, committee receipts with
full attestations that disagreed on root — pinpointing the bundle-builder output requirement. Design note:
the interim is gossip-relay of (small synthetic) input tensors; the general §9 design is content-routed DA
(Kademlia provider records for tensor roots), which remains the roadmap item.

Superseded record (kept for history): earlier in Iter 246 the machinery landed with committee OFF

Landed the opt-in live Tier-C committee evidence machinery and kept the baseline gate green; the live
committee settlement itself is blocked on an interprocess bug.

- `a704ece` Opt-in committee synthetic job source: `next_job_with_nonce_committee` (4-slot rotation keeps
  the exact graph_execution slot; slot 3 emits a Tier-C `gelu` committee graph),
  `committee_graph_execution_graph/inputs`, `synthetic_graph_and_inputs_for(graph_id)`,
  `ChainProfile.committee_synthetic_jobs`, env `TENSORVM_LOCAL_CPU_COMMITTEE_SYNTHETIC_JOBS`, producer
  wiring.
- `dfa7b7c` Committee evidence counters: `Chain::committee_receipt_count /
  settled_committee_receipt_count / escalated_committee_dispute_count`, exposed in the explorer overview,
  service status, and harness summary; checker reads them. Fixed a pre-existing needless_range_loop lint.
- `86ca385` Fixed `ExplorerSummary::to_json` (a hand-written serializer) to actually emit the committee
  counters — without it the overview omitted the fields and the `set -e` checker died silently on the
  failed command substitution.
- Gate restore (this commit): committee jobs are OFF by default in compose and the checker's committee
  settlement assertion is gated behind `TENSORVM_LOCAL_CPU_REQUIRE_COMMITTEE_SETTLEMENT` (default false),
  so the baseline local-cpu gate is green (verified: height 15, 55 graph receipts, checker exit 0).

Live-path debugging — two fetch/serve bugs fixed, root cause fully diagnosed:

- `d173f1d`: the libp2p program-body fetch/serve helpers (`validator_remote_program_response`,
  `graph_from_program_body` in validator_fetch + validator_role) validated graph bodies with
  `validate_for_consensus`, rejecting Tier-C bodies so the committee gelu body could never propagate via
  fetch. (This also explains the earlier "height 4 / job_count 15" — a cascading rejection, not a real
  producer stall; with the fix the producer runs fine and the chain advances with committee on.)
- `427650e`: graph verification re-executes from inputs and compares recomputed roots against the
  receipt's claimed output roots (already in the body); the output tensor is never needed. Committee
  receipts now fetch only inputs (Tier-A/B graph receipts keep fetching outputs unchanged).

Remaining blocker (P2P tensor DA / topology), found via gated tracing (since removed): synthetic graph
input tensors originate ONLY at the producer (validator-00) at runtime; `localnet seed` runs only matmul +
linear rounds (no graph round), so no graph tensors are seeded at genesis. The peer mesh is partitioned —
validators peer with validators + the bootstrap miner-00; miners peer with miner-00 + miners, not
validator-00. Trace showed required roots returning `Missing` from all 5 reachable peers (0 transport
errors), and validators only ever process the exact graph `961ec8…`, never the committee graph `9551ab…`.
So miners cannot fetch the committee input (held only by validator-00) → no committee receipt is ever
produced → nothing to settle. In-process committee settlement
(`tier_c_receipt_settles_only_on_committee_agreement`) and Tier-C fraud-proof
(`trace_bisection_referee_resolves_tier_c_committee_dispute`) tests pass; the gap is purely the live
libp2p tensor-availability path. Next: seed/broadcast the fixed committee input tensor to all nodes (or
fix the peer mesh) so assigned miners can execute committee jobs.

Validation: `cargo test -p tensor_vm --lib` 598 pass, `cargo test -p tensor_vm_explorer --lib` pass,
`cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean, compose test
pass; baseline Docker checker exit 0 with committee off.

### Iteration 245: Tier-C Verification Ladder + Producer Plateau Fix

Landed the Tier-C verification ladder end to end plus the live-production unblock. Commits (pushed to
`origin/main`):

- `b3b8984` Canonical fixed-point Q32 transcendental references (`exp`/`log`/`sqrt`/`sigmoid`/`tanh`/
  `silu`/`gelu`/`softmax`) on CPU with bit-exact CUDA kernels, conformance vectors (suite v2), and an
  exhaustive s=16 accuracy proof (f64 oracle after dropping `inari`).
- `3a48598` Compose `layer_norm`/`rmsnorm` from canonical primitives (proves transformer-block
  composition).
- `5ff941f` Fix Fixed32 `mean` to round-half-even integer averaging (i128 accumulation).
- `94b9fff` §8.1 committee verifier core: `Committee` op-admission, committee execution path,
  seed-independent agreement root, honest-majority agreement count.
- `ba395dd` Wire §8.1 committee settlement into `settle_epoch` (delayed settlement on disagreement) so
  Tier-C work enters consensus.
- `f2b524b` Harden §8.1: mandatory-audit deterrent + honest economic calibration (strict vs committee
  graph split, `redundancy_k` param).
- `000481f` Extend §8.2 interactive fraud proofs to Tier-C committee disputes (referee re-executes via
  `validate_for_committee`; 1-of-N honest resolution).
- `326c4c3` Route §8.1 committee disagreement to §8.2 escalation (detect conflict, signal fraud proof).
- `57135ec` Audit verifies committee agreement root (a validator attesting Valid with a non-canonical
  root now fails audit).
- `19da56b` Let the miner role produce Tier-C committee receipts (closes the production gap).
- `5050b61` Fix synthetic-producer plateau: monotonic job nonce (`next_job_with_nonce` keyed on chain job
  count) decouples production from chain height. Verified live: head advanced past the old height-7
  plateau; graph_execution receipts 50 vs the >5 checker floor; checker exits 0.

Scope note: transcendentals are committee-admitted (§8.1), i.e. committee-trust per §14 — not Tier-A/B
exact security. Exact verifiers/soundness bounds remain roadmap (§4.8). Validation: `cargo test -p
tensor_vm --lib` 597 pass, clippy clean, fmt clean across the thread.

### Iteration 244: CUDA Field Split Graph Kernel/Conformance

Added CUDA field multi-output `split(dim,sizes)` graph execution for scale-0 field tensors (per-segment
CUDA slicing through the multi-output runtime path), with direct/graph parity, miner-role CUDA graph
fixture (`stack`->`split`->`concat`), and unsupported `cast` rejection. Full validation (Gate 0, default +
CUDA suites, workspace release/clippy, Tarpaulin 84.95%, CUDA clippy, `git diff --check`) passed June 24,
2026. Commit `97e5128` (`Add CUDA field split graph support`) pushed to `origin/main`.

### Iteration 243: CUDA Field Concat/Stack Graph Kernel/Conformance

Added CUDA field `concat(dim)`/`stack(dim)` graph execution for variadic scale-0 field tensors via
device-side row-major structural-copy kernels, with direct/graph parity, miner-role fixture
(`tril`/`triu`->`concat`->`stack`), and unsupported `split` rejection. Full validation passed June 23,
2026. Commit `a5d248e` (`Add CUDA field concat stack graph support`) pushed to `origin/main`.

### Iteration 242: CUDA Field Triangular Graph Kernel/Conformance

Added CUDA field `tril(diagonal)`/`triu(diagonal)` graph execution for scale-0 rank-2 field tensors via a
device-side triangular-mask copy kernel, with direct/graph parity, miner-role fixture
(`slice`->`unsqueeze`->`triu`->`tril`), and unsupported `concat` rejection. Full validation passed June 23,
2026. Commit `85cb8c3` (`Add CUDA field triangular graph support`) pushed to `origin/main`.

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
- Tier-C transcendental ops (§4.8) are admitted through the §8.1 committee path (committee-trust per §14),
  not exact-verified. This is v0-legitimate but must not be framed as Tier-A/B exact security; exact
  verifiers + soundness bounds for transcendentals stay roadmap.
- Synthetic-producer job uniqueness must derive from a monotonic nonce (chain job count), never chain
  height. Height-keyed jobs deadlock the producer the instant a job fails to advance the head. The
  height-keyed `next_job`/`next_*_job` stay unchanged for deterministic-replay callers; the live producer
  uses the additive `next_job_with_nonce`.

## Validation Evidence

Iteration 245 (Tier-C ladder + plateau fix): `cargo test -p tensor_vm --lib` 597 pass, clippy clean, fmt
clean; live Docker local-cpu run after the plateau fix advanced past the old height-7 plateau with 50
graph_execution receipts (>5 floor) and the checker exiting 0. Per-commit detail is in the Iteration 245
record above. Gate 0 (`cargo test -p tensor_vm local_testnet --release`) remains the first executable
acceptance command on resume.

## Archive

- Iterations 238 and earlier progressively broadened CUDA graph coverage through field `mean`,
  `broadcast`, `sum`, `clamp`, `div`, comparison masks, `where`, unary field ops, linear-step CUDA paths,
  and the CUDA graph conformance boundary. Their commit anchors remain in git history before
  `012ff56`.
- Earlier local-chain readiness work established the shared chain engine, role-owned miner and validator
  loops, libp2p/node payload ingestion, delayed reward claims, proposer reward holds, trace-bisection
  economics, validator audit/slashing paths, public evidence scaffolding, and local CPU Gate 0.
