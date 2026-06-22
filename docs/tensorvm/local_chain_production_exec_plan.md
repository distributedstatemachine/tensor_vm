# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: none; next implementation slice not yet checkpointed.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  chain-owned pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage,
  automatic unfinalized side-branch deep reorg, graph-backed synthetic jobs, and claim-owned reward
  spendability are implemented locally. Receipt reward maturity is explicit for awaiting-inclusion,
  claimable-height, and validator-VRF-reveal-held states. Maturity release commands cannot move matured
  rewards into spendable balances; explicit `ClaimReward` remains the canonical spendability boundary.
  The graph verifier exact replay path has focused admission evidence for admitted exact generator,
  shaping, and comparison op clusters. A chain-neutral signed trace-bisection message/state core over
  verified IR trace openings now exists; p2p/chain integration, one-op referee execution, slashing, and
  challenger settlement remain open.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: choose the next feature-sized slice. Current high-value options are p2p/chain integration for
  trace-bisection rounds, deployed full VRF lifecycle evidence, public/CUDA deployment evidence, or the
  remaining exact Tier-B/CUDA conformance surface.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | `cargo test -p tensor_vm local_testnet --release` passed as first command and final gate for Iteration 158 on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof | Add trace-bisection p2p/chain integration or deployed public/CUDA proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, focused admitted exact-op graph-verifier evidence, role-owned graph execution, `const_blob`, p2p trace openings, signed trace-bisection core | Continue interactive trace dispute integration and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments. Local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit reward maturity, VRF reveal holds, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

No active feature is checkpointed. Start the next slice with the required Gate 0 command and a fresh
checkpoint before edits.

## Recent Iterations

### Iteration 158: Trace Bisection Dispute Protocol Core

Commit: `6f6344a` (pushed `main` -> `main`).

Feature capability: added a signed, deterministic trace-bisection dispute round/state core over verified
`IrTraceOpening` values. This creates the protocol message/state boundary for the `upow.md` §8.2
interactive fraud-proof game before wiring it into p2p codecs, chain commands, or slashing.

Readiness requirements covered: `upow.md` §8.2 interactive fraud proofs over `trace_root`, §9 trace
opening availability, and §16 fraud-proof game TODOs for message shape, timeouts, and griefing-bond
envelope fields.

Files/modules touched: `crates/tensor_vm/src/challenge.rs`, `crates/tensor_vm/src/lib.rs`,
`docs/tensorvm/upow.md`, and this execution plan.

Parallel subagents run: skipped because the available subagent tool explicitly requires user authorization
for delegation before spawning agents.

Expected observable evidence: a challenge session over a multi-op graph accepts a signed midpoint opening,
verifies the opening against `trace_root`, narrows left or right depending on expected versus opened output
roots, reaches a single disputed op in logarithmic rounds, rejects tampered signatures/openings, and reports
timeout after the response deadline with challenger/responder bond fields.

Canonical owner: `challenge` owns fraud-dispute protocol state and validation; IR owns trace openings and
Merkle proof verification. Chain integration will later consume this state instead of inventing adapter
logic.

Out of scope: p2p wire messages for bisection rounds, chain command admission, final one-op referee
re-execution, stake mutation, challenger reward payment, deployed evidence, and CUDA evidence.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm challenge --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (516 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

### Iteration 157: Graph Verifier Admitted Exact-Op Coverage

Commit: `fc14b63` (pushed `main` -> `main`; metadata commit `7d4e172` recorded the iteration).

Feature capability: focused graph-verifier evidence that admitted exact Tier-B op clusters are not only
executable by the IR interpreter and conformance suite, but accepted through `verify_graph_execution` only
when their conformance profile entries are present.

Validation passed on June 22, 2026: first-command Gate 0, focused
`cargo test -p tensor_vm graph_verifier_accepts --lib -- --nocapture`,
`cargo test -p tensor_vm conformance --lib -- --nocapture`, `cargo test -p tensor_vm --lib` (512 passed),
`cargo fmt --check --all`, `cargo check -p tensor_vm --tests`, `git diff --check`, and final
`cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

## Decision Log

- Gate 0 remains `cargo test -p tensor_vm local_testnet --release` and must be the first executable command
  on every resume before edits.
- Chain validation is the canonical owner for accepted randomness proof verification, typed proof metadata,
  state-rooted records, finalized beacon advancement, and seed derivation.
- Runtime may observe wall-clock public endpoint freshness only for locally fetched public drand. Chain
  validation/state own the accepted public drand anchor and deterministic chain-epoch round window.
- Reward delays, reveal holds, and spendability are chain-owned pending-claim state. Checkers and runtime
  surfaces only observe these states.
- Bounded p2p/node payloads remain the only network wire surface for randomness and reveal records.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, and fully chain-admitted
  interactive transcript disputes remain deployment or future-feature gates, not local-completion claims.

## Validation Evidence

- Iteration 158 first executable command passed before code edits on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 158 feature commit `6f6344a` pushed to `main` on June 22, 2026:
  `git push` returned `7d4e172..6f6344a  main -> main`.
- Iteration 158 focused validation passed on June 22, 2026:
  `cargo test -p tensor_vm trace_bisection --lib -- --nocapture` and
  `cargo test -p tensor_vm challenge --lib -- --nocapture`.
- Iteration 158 broad validation passed on June 22, 2026:
  `cargo fmt --check --all`; `cargo check -p tensor_vm --tests`; `git diff --check`;
  `cargo test -p tensor_vm --lib` (516 passed); and final
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 158 coverage command remained environmentally blocked on June 22, 2026:
  `cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

## Archive

- Iteration 156 (`846f369`, pushed `main` -> `main`): explicit reward claim spendability. Matured
  proposer, receipt, challenge, and credit rewards remain pending claims until beneficiary `ClaimReward`;
  maintenance release commands only prune voided/prunable claims. Validation included focused reward,
  settlement, challenge, attestation, transaction, telemetry, node payload tests, full lib tests, check,
  fmt, diff check, and final Gate 0.
- Iterations 143 through 155 established verified drand/network randomness, production validator reveal
  proofs, finality-delayed proposer rewards, finalized side-branch convergence, durable restart-rehydrated
  tensor artifacts, deployment preflight/evidence surfaces, rolling restart evidence, richer IR/Tier-B
  execution, delayed reward maturity, claim-owned spendability, audit and challenge reward holds, exact
  trace openings, and related local CPU Docker proof evidence. Detailed historical command transcripts and
  commit hashes are preserved in git history.
