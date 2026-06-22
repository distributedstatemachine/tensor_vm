# Goal Operating Contract

Read `docs/tensorvm/upow.md`, `docs/tensorvm/mvp_spec.md`, and `docs/tensorvm/local_chain_production_readiness.md` fully before editing. Maintain `docs/tensorvm/local_chain_production_exec_plan.md` as the source of truth for progress, decisions, validation commands, and blockers.

## Primary Objective

Implement `docs/tensorvm/upow.md` to a **full v0 MVP**: a standalone useful-work chain whose proof-of-work is the verification of useful tensor computation. `upow.md` is the canonical specification for the chain's design. Drive every feature iteration toward closing the gap between current code and the v0 scope below until the v0 Definition Of Done is met.

Spec precedence when documents conflict:

- `docs/tensorvm/upow.md` is canonical for chain design, the workload IR, the verification ladder, consensus, randomness, records, economics, and what is in v0 vs roadmap.
- `docs/tensorvm/mvp_spec.md` wins over older readiness/exec text where `upow.md` is silent.
- Update stale readiness/exec text as part of the feature instead of following it.

Keep every existing guardrail in this file in force while implementing `upow.md`. The Architecture overrides, shortcut ban, parallelization rules, slice-size rule, commit/push rule, and compaction rule all still apply.

## Useful-Work Chain v0 MVP Scope (upow.md)

v0 admits **only** ops and mechanisms whose canonical `F_p` semantics are fully specified and exactly (or committee-) verifiable (`upow.md` §4.9, §13). The MVP is "done" when every in-scope capability below is implemented end to end with tests and checker/docs evidence, and every roadmap item is explicitly carried but gated out of consensus.

### In scope for v0 (must be implemented)

| upow.md ref | Capability | Notes |
|---|---|---|
| §3, §3.1–3.3 | Determinism contract: `F_p` arithmetic, fixed-point scale discipline, round-half-to-even, fixed ascending reduction order, canonical dtype/layout | Bit-exact reproducibility is the load-bearing constraint; everything else depends on it |
| §3.3, §16 | Per-op `F_p` conformance test-vector suite any runtime (CPU reference, CUDA) must pass before receipts are accepted | Marked blocking-for-safety in the spec |
| §4 | Content-addressed Tensor IR: value model, refs, `Op`, `Graph`, canonical JSON encoding, `graph_id`, structural validity rules | Replaces fixed job-type plumbing with a real IR graph language |
| §4.7–4.9 | Frozen op registry with v0-admitted exact ops only (Tier A `matmul`/contraction `einsum`, exact Tier B elementwise/shaping/`sum`/`mean`/comparison/`relu`/exact quantization) plus the minimal set `LinearTrainingStep` needs | Tier-C ops carried in the registry as vocabulary but **gated out of consensus** |
| §4.9 | Canonical v0 jobs: `TensorOp` (single `matmul`) and `LinearTrainingStep` | Both must settle live after startup |
| §5 | Commitments and records: tensor Merkle commitments, `trace_root`, `Job`/`Receipt`/`Attestation` canonical bodies, `*_id = SHA256(canonical(body))`, asymmetric (sr25519/ed25519) signatures only | No shared-secret signing in the consensus path |
| §6 | Verification L1: full-output Freivalds for block-eligible Tier-A receipts; row-sampled only for telemetry/triage | Full-output is mandatory for block eligibility |
| §7 | Verification L2: sound random-linear checks for affine/elementwise Tier-B ops; enumerate which Tier-B ops are sound vs deferred | `gather`/`scatter`/`embedding` need index-consistency, not just linear checks |
| §8.1 | Verification L3a: redundancy + honest-majority committee agreement for Tier-C receipts, with delayed settlement on disagreement | v0 baseline; weaker than L1/L2 by design |
| §9 | Data availability for verification: miners serve tensor/trace chunks, validators do availability sampling, unserved chunks make a receipt non-finalizable and put bond at risk | Durable/erasure-coded DA is roadmap |
| §10 | Unbiasable randomness: VRF/beacon-seeded challenge vectors and audit/committee selection, commit→reveal binding `r` to `(receipt_id, beacon_round)`, block-hash-derived randomness banned | Pin exact beacon construction as part of the feature |
| §11 | Consensus: deterministic settled-receipt blockspace, UVPoW puzzle bound to a valid `checks_root`, difficulty retargeting, zero-receipt skip fallback, stake-weighted BFT finality separate from admission | Block proposal is validator-owned; TensorWork never selects proposers |
| §11.4 | `TensorBlock` structure: height/prev/beacon, settled receipts, `checks_root`, PoW nonce/target, reward allocations, proposer sig, finality votes | Canonical names only |
| §12 | Economics: miner/validator/challenger rewards, slashable miner bonds, lazy-validator mandatory-audit slashing, data-withholding timeout slashing, parameter table, the bond ≥ gain-from-fraud invariant | State and re-verify the economic invariant whenever a parameter changes |
| §14 | Honest soundness framing recorded in docs/status (Tier-A strong, Tier-C committee-trust until 3b) | Do not over-claim base-layer economic security |

### Roadmap, carried but gated out of v0 consensus

Do not chase these as v0 work; do not let them block v0 completion. Keep the hooks (e.g. the `trace_root` field) so they remain non-breaking additions.

- §8.2 interactive fraud proofs over `trace_root` (3b) — the priority post-v0 upgrade; `trace_root` ships in v0 to keep it non-breaking.
- §8.3 / L4 ZK proofs of op/segment execution.
- §9 durable erasure-coded DA and light-client guarantees.
- §4.8 transcendental and order-dependent Tier-C op **consensus admission** (needs published canonical fixed-point references + verifiers + soundness bounds).
- Externally-useful (non-synthetic) workloads and the §1.2 non-goals (arbitrary float consensus state, full Transformer training as one step, Turing-complete VM consensus, on-chain full tensors, ZKML for the whole workload, subjective usefulness scoring).

### Known v0 gaps to drive iterations (current code → upow.md v0)

These are the concrete deltas between the implemented reference core and full `upow.md` v0. Convert each into a feature-sized iteration using the checkpoint format below, and track status in the exec plan readiness matrix.

- Full content-addressed Tensor IR graph language, frozen op registry, canonical encoding, `graph_id`, and structural validity (today only fixed `TensorOp`/`LinearTrainingStep` job types exist).
- Per-op `F_p` conformance test-vector suite gating receipt acceptance (§3.3).
- Difficulty retargeting and zero-receipt skip fallback for UVPoW block production (§11.2).
- Exact parent-state snapshots, child-state apply semantics, selected-receipt lifecycle/opening metadata, and `checks_root` challenge openings (§11).
- VRF/beacon randomness binding with commit→reveal ordering and an enforced ban on block-hash-derived consensus randomness (§10).
- L2 random-linear soundness coverage enumerated per Tier-B op, with index-consistency handling for `gather`/`scatter`/`embedding` (§7).
- Slashable miner bonds, mandatory randomized validator audits with slashing, and data-withholding timeout slashing wired to the economic invariant (§12).
- Network-visible validator proposer/block-assembly tick that replaces the remaining service-owned synthetic round helper (carryover from the local readiness plan, required by §2 and §11).

## Canonical Architecture Override

The canonical TensorVM architecture is useful-verification PoW with deterministic settled-receipt blockspace. Treat this as the current MVP design, not as an optional v2 side path.

Hard rules:

- Replace v1 behavior outright. Do not preserve backward compatibility with TensorWork-weighted proposer selection, job-rooted blocks, or a separate miner proposer role.
- Do not add compatibility shims, legacy aliases, dual block formats, migration adapters, or runtime branches that keep v1 consensus alive.
- Do not name structs, enums, modules, fields, commands, or docs with `V2`, `v2`, `New`, `Legacy`, or `Compat` just because they implement the current design.
- Use canonical names for the current design, for example `TensorBlock`, `Blockspace`, `SettledReceipt`, `UsefulVerificationPow`, `ChecksRoot`, and `ValidatorProposer`.
- If existing code has v1 names or behavior, rename or replace it directly when touched. Update tests and docs to the new canonical model instead of layering translation code.
- If docs conflict, `docs/tensorvm/mvp_spec.md` wins over older readiness text. Update stale readiness text as part of the feature instead of following it.
- Miners produce work receipts and serve tensor data. Validators verify receipts, build `checks_root`, perform useful-verification PoW, propose blocks, and sign BFT finality.
- TensorWork is only for miner rewards, blockspace capacity, telemetry, and concentration analysis. It must not select proposers.

## Interprocess Node Boundary Override

`tvmd` is a process launcher, operator CLI, and node adapter. It is not allowed to be the hidden consensus orchestrator.

Hard rules:

- Counted miners and validators must be separate long-running node processes with separate durable state, libp2p identities, endpoints, and role loops.
- Jobs, receipts, attestations, tensor fetches, blocks, checks roots, PoW evidence, and finality votes must cross process boundaries through libp2p or node RPC before affecting another node.
- The shared chain engine may be called directly inside one node after that node validates a local or inbound event. It must not be used by one `tvmd` service loop to mutate multiple counted roles in memory.
- Replace `tvmd` paths that synthesize jobs, receipts, attestations, blocks, votes, or role counters for other operators. Do not preserve them behind compatibility flags.
- Do not count single-process helpers, deterministic replay, service-owned local producer loops, or in-memory propagation as local-chain readiness evidence.
- `tvmd miner run` owns only miner behavior. `tvmd validator run` owns validator verification, useful-verification PoW, block proposal, and finality voting. There is no separate miner proposer process.
- `tvmd node serve` may expose local node APIs and start one configured node role. It must not perform multi-role orchestration.
- Tests for pure chain state transitions may call the chain engine directly. Acceptance tests for local production readiness must prove interprocess libp2p/RPC behavior.

## Architecture Shortcut Ban

Hard rules:

- Do not gate inbound P2P ingest on `block_interval`, `local_producer`, profile synthetic jobs, local block production, or RPC serving mode.
- Do not let producer capability disable inbound sync. Producer policy controls outbound block creation only.
- Do not add settlement, model transitions, proposer selection, block validation, block-vote synthesis, reward allocation, or finality in `main.rs`, `node.rs`, p2p glue, RPC glue, checkers, deployment scripts, or other adapters.
- Do not synthesize validator votes except inside clearly named pure test helpers that cannot be reached by runtime code.
- Do not classify consensus outcomes by matching error strings. Add typed outcomes or typed error variants at the chain boundary.
- Do not add consensus transaction variants that do not mutate canonical chain state or explicitly queue into a block body.
- Do not add another codec for `TensorBlock`, jobs, receipts, attestations, block votes, tensors, or consensus payloads without a shared-codec plan and parity tests.
- Do not add unbounded length-prefixed wire reads. Bound before allocation.
- Do not add status/checker fields by copy-pasting format strings. Add or extend a typed status snapshot owner.
- Do not count shell assertions, deterministic replay, local single-process helper state, hardcoded booleans, or checker-only policy as readiness evidence.

Before implementing any feature that touches consensus, P2P, runtime, storage, status, or checker code, answer these in the implementation checkpoint:

```text
Canonical owner:
Adapter callers:
Old shortcut being removed:
Regression test that proves the shortcut is gone:
Behavior with local synthetic block production disabled:
Behavior for producer and non-producer roles:
Structured evidence source:
Finality source:
Wire-size and codec boundary:
```

Every verifier review for these areas must challenge whether the change moved logic into the canonical layer or merely added another adapter branch.

Default to **feature-sized iterations**, not tiny one-function slices. A feature-sized iteration should deliver one coherent readiness capability end to end: production code, tests, checker/docs evidence, and a commit. Only shrink to a smaller slice when the feature crosses unrelated ownership boundaries, the verifier flags high risk, or targeted validation is failing.

Good feature-sized iterations:

- Useful-verification PoW block validity: state types, command path, block checks, focused tests, docs status.
- Settled-receipt blockspace: pool state, deterministic selection, caps, duplicate/spent handling, tests.
- Receipt payload ingestion: p2p decode, runtime queue, `ChainCommand::SubmitReceipt`, counters, checker assertion.
- Validator attestation payload ingestion: p2p decode, runtime apply path, status counters, checker assertion.
- Role-owned miner loop: job subscription, execution, receipt submission, tensor serving, role tests.

Bad iterations:

- Rename one field and stop.
- Add a doc TODO without code.
- Touch every runtime module without a single acceptance capability.
- Run only formatting and commit.

For every feature iteration, write this checkpoint before edits:

```text
Iteration N: <short title>
Feature capability:
Readiness requirements covered:
Files/modules likely touched:
Parallel subagents to run:
Parallelizable implementation workstreams:
Tests/checkers/docs to add or update:
Narrow validation commands:
Broad validation commands before commit:
Expected observable evidence:
Out of scope:
Split trigger: what would force this feature to be split smaller?
```

## Parallelization Rule

Before implementation, launch subagents in parallel and use their results to divide the feature into workstreams:

- `readiness-mapper`: map the target capability to readiness requirements and current gaps.
- `tensorvm-codebase-explorer`: explore implementation paths, symbols, and coupling.
- `tensorvm-test-coverage-explorer`: find existing tests and missing behavior coverage.
- Optional second `tensorvm-codebase-explorer`: focus on checker scripts, Docker, or p2p if the feature spans those areas.

During implementation, parallelize as much as the tooling safely allows:

- Run read-only explorers while the parent plans the implementation boundary.
- Run test discovery in parallel with code-path discovery.
- Use separate implementation subagents or worktrees for independent development workstreams when they will not edit the same files.
- Use `tensorvm-test-runner` for noisy or long validation while the parent reviews the diff.
- Before commit, challenge the whole integrated diff with a manual verifier-style review of ownership
  boundaries, tests, and readiness evidence; do not require a nonexistent `tensorvm-verifier` binary.
- Use `tensorvm-goal-supervisor` before resuming after pauses, after several iterations, or whenever scope starts shrinking into busywork.

Do not parallelize two writers against the same files in the same worktree. If parallel implementation would collide, keep the parent as the single writer and use subagents for read-only exploration, test planning, and verification.

## Parallel Development Rule

Actual code development may be parallelized when the feature can be split into independent workstreams with clear file ownership.

Before launching writer subagents, write a workstream map:

```text
Feature:
Integrator/merge owner:

Workstream A:
Owner/subagent:
Files owned:
Allowed edits:
Forbidden files:
Validation:

Workstream B:
Owner/subagent:
Files owned:
Allowed edits:
Forbidden files:
Validation:

Merge order:
Final integrated validation:
```

Use parallel writer subagents for work like:

- chain/state/API boundary while another subagent updates checker-script evidence
- p2p message codec while another subagent writes tests for existing chain validation
- storage recovery tests while another subagent updates docs/status
- explorer/API read surfaces while another subagent works on local checker assertions

Do not use parallel writer subagents for work like:

- two agents editing `crates/tensor_vm/src/main.rs`
- two agents editing the same chain module
- one agent refactoring types while another depends on those unstable type names
- Docker Compose lifecycle work against the same project name
- commits from subagents

When parallel writers are used:

1. Give each writer a narrow ownership contract and forbidden-file list.
2. Prefer isolated worktrees or branches for writer subagents.
3. Require each writer to report changed files, tests run, and known risks.
4. The parent/integrator reviews each diff before merging.
5. The parent/integrator resolves conflicts and runs the final integrated validation.
6. Only the parent/integrator commits.

If workstreams collide, stop parallel writing and switch those subagents to read-only support.

## Slice Size Rule

Prefer the largest slice that can still be reviewed as one coherent feature. A slice is too small if it cannot produce observable behavior or update a meaningful acceptance gate. A slice is too large if it mixes unrelated readiness capabilities or cannot be validated before commit.

The iteration should usually include:

```text
1 production capability
1 focused test cluster
1 checker/docs evidence update when applicable
1 targeted validation run
1 commit
```

If a feature needs multiple commits, keep the commits feature-subdivided, not microscopic:

```text
commit 1: data/state/API boundary
commit 2: runtime/network integration
commit 3: checker/docs evidence
```

Do not stop after commit 1 if the goal feature needs commits 2 and 3 and the targeted validation is still incomplete. Continue until the feature capability is done or a real blocker is documented.

## Commit And Push After Every Iteration

Every successful feature iteration must end with a git commit and push. Do not start the next feature iteration until the current iteration has either:

- a commit hash and pushed branch/remote recorded in `docs/tensorvm/local_chain_production_exec_plan.md`, or
- an explicit blocker recorded with the exact failing command/output and reason commit or push was not completed.

Before committing:

```text
1. Run the targeted validation for the feature.
2. Review the integrated diff with a verifier-style pass over ownership boundaries, tests, and readiness
   evidence.
3. Review `git status --short` and `git diff`.
4. Update the exec plan with validation evidence.
5. Compact the exec plan if required by the compaction rule.
6. Commit only the files related to the iteration.
7. Push the commit to the configured upstream branch.
8. Record the commit hash, remote, branch, and push result in the exec plan.
```

Commit/push rule: commit and push after every successful iteration, but never commit a known-broken targeted gate. If a full Docker gate is environmentally blocked, document exact command/output in the exec plan before committing and pushing the narrower passing slice. Only the parent/integrator commits and pushes; subagents must not commit or push.

If push fails because no upstream exists, credentials are unavailable, the network is blocked, or policy forbids publishing the branch, record the exact blocker and do not start the next feature iteration until the user resolves it or explicitly approves continuing without push.

## Exec Plan Compaction Rule

Keep `docs/tensorvm/local_chain_production_exec_plan.md` useful as durable state, not as an ever-growing transcript. Compact it after every feature-sized iteration, after every 3 commits, or whenever it exceeds roughly 300 lines.

The exec plan should keep:

- Current goal and active feature capability.
- Current blocker list.
- Current readiness matrix with status, evidence path, and next action.
- Last 2 feature iterations in detail.
- Validation evidence for the latest successful commit and any current blocker.
- Decision log entries that still affect future implementation.
- Compact archive summaries for older iterations.

The exec plan should remove or compress:

- Full terminal transcripts after the pass/fail result and key error lines are recorded.
- Repeated command lists already captured in `goal.md` or the readiness doc.
- Stale file lists from completed iterations.
- Old subagent chatter once decisions and evidence have been summarized.
- Superseded plans that no longer affect future work.

Use this structure:

```text
# Local Chain Production Execution Plan

## Current State
- Active feature:
- Current status:
- Current blockers:
- Next action:

## Readiness Matrix
| Capability | Status | Evidence | Next action |

## Active Feature Iteration
<full current checkpoint, workstream map, validation target>

## Recent Iterations
<last 1-2 completed feature iterations with concise evidence>

## Decision Log
<durable decisions only>

## Validation Evidence
<latest command results and current blocker outputs only>

## Archive
<one-paragraph summaries of older completed iterations with commit hashes>
```

When compacting, preserve facts, commands, evidence, blockers, and commit hashes. Do not delete unresolved blockers or decisions that still constrain the implementation. If unsure whether to delete something, summarize it in the archive instead of keeping the full text.

## Definition Of Done

### upow.md v0 MVP

- Every in-scope `upow.md` v0 capability in the scope table above is implemented end to end with tests and checker/docs evidence, or explicitly reclassified with rationale recorded in the exec plan.
- The determinism contract (§3) holds: consensus-critical compute is bit-exact `F_p` fixed-point, and a per-op conformance test-vector suite gates receipt acceptance for both the CPU reference runtime and the CUDA path (§3.3).
- Workloads are expressed as the content-addressed Tensor IR (§4): graphs are addressed by `graph_id`, structurally validated before execution, and built only from v0-admitted ops; Tier-C ops are carried in the registry but gated out of consensus.
- The verification ladder is live for v0: full-output Freivalds for Tier-A block eligibility (§6), sound random-linear checks for the enumerated Tier-B ops (§7), and redundancy + committee agreement with delayed settlement for Tier-C (§8.1).
- Records (`Job`/`Receipt`/`Attestation`/`TensorBlock`) use canonical bodies, `SHA256(canonical(body))` ids, asymmetric signatures only, and reserve `trace_root` so §8.2 fraud proofs remain a non-breaking addition (§5, §13).
- Randomness for challenge vectors, audits, and committee selection is beacon/VRF-sourced with commit→reveal binding; block-hash-derived consensus randomness is banned and that ban is enforced, not just documented (§10).
- Consensus is deterministic settled-receipt blockspace + UVPoW bound to a valid `checks_root` with difficulty retargeting and zero-receipt fallback, admission separate from stake-weighted BFT finality (§11).
- Economics are wired: rewards, slashable bonds, mandatory validator audits, and data-withholding timeout slashing satisfy the bond ≥ gain-from-fraud invariant, which is re-verified on every parameter change (§12).
- Docs/status record the §14 honest soundness framing (Tier-A strong; Tier-C committee-trust until 3b) without over-claiming base-layer security.

### Local chain readiness (carryover guardrails)

- Every readiness gap in `docs/tensorvm/local_chain_production_readiness.md` is implemented or explicitly reclassified with rationale.
- v1 consensus assumptions are removed or rewritten wherever they affect active code, tests, checkers, or current docs.
- Role-owned miner and validator paths do not bypass the shared chain engine; block proposal is validator-owned useful-verification PoW, not a separate miner proposer path.
- `tvmd` no longer performs hidden multi-role consensus orchestration; counted operators interact through interprocess libp2p/RPC boundaries.
- Libp2p or the shared node event path drives jobs, receipts, attestations, and blocks.
- Blocks are built from deterministic settled-receipt blockspace and validated through `checks_root`, useful-verification PoW, and BFT finality.
- The local checker proves live post-startup receipt, attestation, reward, tensor fetch, telemetry, restart, and all-operator convergence evidence.
- Unit/integration tests cover changed chain/runtime/storage/network behavior.
- The full local acceptance gate from the readiness doc passes or any environmental blocker is documented with exact failing command/output.
