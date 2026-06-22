# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 175 complete: Admitted-Op Conformance Identity Guard.
- Current status: deterministic `F_p` conformance vectors now have unique identity checks, every
  consensus-admitted frozen registry op must have vector and CPU profile evidence, and any non-registry
  conformance vector/profile entry must be explicitly marked as an auxiliary verifier vector. Delayed
  proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Valid matured claims remain non-spendable until the beneficiary calls `ClaimReward`;
  direct maturity-sweep commands can prune only voided/unavailable matured claims and cannot credit live
  beneficiary balances. Trace-bisection has signed sessions/rounds, bounded p2p open/expectation/round/
  referee payloads, pending-queue application, runtime session-open, challenger expectation, responder
  round generation, runtime challenger referee-witness generation from local graph replay, one-op referee
  verdicts, timeout settlement, slashing, and delayed challenger rewards.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: continue CUDA/public deployment evidence, generic arbitrary-IR job admission and role
  execution, or remaining trace-dispute hardening without treating roadmap trace-bisection work as a v0
  blocker.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 173 first command and pre-commit rerun of `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection expectations/rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof or remaining fraud-proof DoS policy |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, packed int8 APIs, const blobs, role-owned graph execution, and automatic runtime trace-bisection referee witnesses when isolated-opening roots match local replay | Continue CUDA graph evidence, multi-round trace-bisection DoS policy, and incomplete-transcript handling |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 175: Admitted-Op Conformance Identity Guard

Feature capability: tighten the v0 deterministic `F_p` conformance evidence by making vector IDs unique,
asserting that every conformance vector op is either registry-admitted or explicitly auxiliary, and proving
that the CPU reference profile exactly matches the vector op set with the same auxiliary boundary.

Readiness requirements covered: `upow.md` §3.3 per-op `F_p` conformance vectors and §4.7-§4.9 frozen
registry admission gates for exact v0 ops.

Canonical owner: `crates/tensor_vm/src/conformance.rs` owns the vector suite, stable hash, and CPU profile;
receipt/graph verifiers continue to consume `ConformanceProfile` rather than reimplementing suite policy.

Adapter callers: `verify.rs` and `runtime::backend_conformance_profile` consume the profile. No network,
storage, or consensus command boundary changes are expected.

Old shortcut being removed: vector/profile evidence could be weakened by duplicate vector IDs or by
non-registry vector/profile entries that were not explicitly identified as auxiliary verifier-only
coverage.

Regression test that proves the shortcut is gone: conformance tests will reject duplicate vector IDs and
will reject non-admitted vector/profile entries unless they are explicitly listed as auxiliary.

Behavior with local synthetic block production disabled: unchanged; this is a verifier/conformance gate.

Behavior for producer and non-producer roles: unchanged; both roles consume the same verifier profile when
validating receipts/graph executions.

Structured evidence source: conformance test names, coverage matrix, tarpaulin report, and this exec plan.

Finality source: unchanged; no block/finality mutation in this slice.

Wire-size and codec boundary: unchanged; no p2p/RPC/storage codec changes.

Tests/checkers/docs to add or update: focused conformance tests, coverage matrix, tarpaulin report, and
exec plan.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm conformance --lib -- --nocapture` passed: 10 focused conformance/profile tests.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 540 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered `tvmd_cli`
  local-testnet gateway test.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 555
  instrumented tests, 84.48% workspace line coverage, 22570/26717 lines covered.
- Manual verifier-style review: conformance remains owned by `conformance.rs`; verifiers consume the same
  profile interface; no adapter, network, storage, or consensus command path was changed.
- Feature commit `e45c876` prepared for push to `main` on June 22, 2026.

Out of scope: CUDA conformance execution, adding new admitted ops, generic arbitrary-IR job admission,
public deployment evidence, and trace-bisection DoS policy.

Split trigger: if exact admitted-op profile matching exposes missing execution support for an op, split the
missing op implementation/vector work into a separate feature-sized iteration.

### Iteration 174: Runtime Trace-Bisection Referee Witness Generation

Feature capability: a validator/challenger node with local graph evidence automatically derives an
isolated one-op referee witness from canonical graph replay when the isolated session's stored opening
input roots match the generated witness, submits `ChainCommand::RefereeTraceBisection`, and gossips the
existing bounded referee payload. This removes the manual referee step for referrable isolated transcripts
without adding or depending on any standalone verifier binary.

Readiness requirements covered: `upow.md` §16 trace-bisection fraud-proof lifecycle and `mvp_spec.md`
§38 v1 fraud-proof path for single invalid op proof after interactive bisection.

Canonical owner: chain admission remains `ChainCommand::RefereeTraceBisection`; runtime only derives the
witness from local graph inputs and submits/publishes it. P2P/node boundaries continue using the existing
`NewTraceBisectionRefereePayload` codec and pending-queue application.

Verifier reality check: no `tensorvm-verifier` binary exists in this repository. Validation for this slice
will use cargo/tarpaulin evidence plus manual verifier-style review of the chain/runtime boundaries.

Runtime behavior: validator ticks now run the referee candidate after session-open and expected-root
submission. Successful submissions are persisted, published over the existing bounded referee payload, and
reported through `validator_trace_bisection_referees_submitted` and
`role_validator_trace_bisection_referees_submitted` status fields.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm trace_bisection_referee_generation_requires_challenger_and_local_witness --lib -- --nocapture` passed.
- `cargo test -p tensor_vm exact_interpreter_executes_hand_built_graph_and_commits_trace --lib -- --nocapture` passed.
- `cargo test -p tensor_vm trace_bisection --lib -- --nocapture` passed: 23 tests.
- `cargo test -p tensor_vm runtime_state_tracks_loop_counters --lib -- --nocapture` passed.
- `cargo fmt --all -- --check` passed.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed after mechanical cleanup of current-toolchain clippy warnings.
- `cargo test -p tensor_vm --lib` passed: 538 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.48% line coverage.
- Manual verifier-style review: there is no standalone verifier binary. Runtime only materializes witness
  inputs from local graph replay and submits the existing chain-owned `RefereeTraceBisection` command;
  chain admission still validates op index, input roots, canonical op output roots, slashing, and delayed
  challenger reward settlement.
- Feature commit `b5bf0d9` pushed to `main` on June 22, 2026:
  `git push` returned `2d353a8..b5bf0d9  main -> main`.

Out of scope: adding a standalone verifier binary, changing trace-bisection economics, multi-round DoS
policy, incomplete-transcript final-opening automation when isolation advances past the last opened op, and
public/CUDA deployment evidence.

## Recent Iterations

### Iteration 173: Reward Claim Boundary Regression Hardening

Feature capability: matured reward sweeps cannot be used as a workaround for verifier-dependent reward
finality. Validation covered the reward-boundary regression, reward module tests, fmt/check/diff, lib
tests, release local-testnet, tarpaulin, and manual review. Feature commit `919f77f` pushed to `main` on
June 22, 2026.

### Iteration 172: Trace-Bisection Expected-Root Gossip

Feature capability: challenger-signed trace-bisection expected-root claims cross the bounded p2p/node/
runtime path. A validator/challenger with local graph evidence submits and gossips expectations for the
active midpoint; non-producers apply or queue through `ChainCommand::SubmitTraceBisectionExpectation`;
responder rounds remain pending until that canonical expectation arrives.

Validation completed on June 22, 2026: first executable Gate 0, focused expectation/pending/
trace-bisection/runtime/network tests, fmt/check/diff, `cargo test -p tensor_vm --lib` (536 tests),
release local-testnet, tarpaulin 84.52%, and manual ownership review all passed.

Feature commit `bbb3d28` pushed to `main` on June 22, 2026:
`git push` returned `8ff69c1..bbb3d28  main -> main`.

### Iteration 171: Trace-Bisection Challenger Expected Roots

Feature capability: chain-owned challenger expected-root claims are required before responder rounds can
narrow or isolate trace-bisection disputes; responder-carried branch roots cannot self-select the branch.
Validation covered focused chain/storage/runtime/p2p tests, fmt/check/diff, lib tests, release
local-testnet, tarpaulin, and manual review. Feature commit `6901655` pushed to `main` on June 22, 2026.

## Decision Log

- Gate 0 remains `cargo test -p tensor_vm local_testnet --release` and must be the first executable command
  on every resume before edits.
- Chain validation is the canonical owner for accepted randomness proof verification, typed proof metadata,
  state-rooted records, finalized beacon advancement, seed derivation, rewards, slashing, and challenge
  settlement. Runtime/checkers only observe or submit commands.
- Runtime may observe wall-clock public endpoint freshness only for locally fetched public drand. Chain
  validation/state own the accepted public drand anchor and deterministic chain-epoch round window.
- Reward delays, reveal holds, slashing, challenge settlement, and spendability are chain-owned pending
  claim/state transitions. Valid matured claims become spendable only through beneficiary `ClaimReward`.
- Bounded p2p/node payloads remain the only network wire surface for randomness, reveal records, and
  trace-bisection expectation/round/referee evidence.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, incomplete-transcript
  final-opening automation, and multi-round DoS policy remain deployment or future-feature gates, not
  local-completion claims.

## Validation Evidence

- Iteration 174 validation passed on June 22, 2026: first executable Gate 0; focused referee/IR/runtime
  state tests; trace-bisection filtered tests; fmt/check/diff; clippy; `cargo test -p tensor_vm --lib`
  (538 passed); release local-testnet; tarpaulin 84.48%; and manual chain/runtime boundary review.
  Feature commit `b5bf0d9` pushed to `main` (`2d353a8..b5bf0d9`).
- Iteration 173 validation passed on June 22, 2026: first executable Gate 0; focused reward-boundary and
  reward module tests; broad fmt/check/diff/lib (537 passed), release local-testnet, tarpaulin 84.52%, and
  manual review. Feature commit `919f77f` pushed to `main` (`8b94508..919f77f`).
- Iteration 172 feature commit `bbb3d28` and metadata commit `8b94508` pushed to `main` on June 22, 2026.

## Archive

- Iteration 170 (`d88a14d` plus metadata `04a85d4`, pushed `main` -> `main`): runtime responder round
  generation builds signed midpoint rounds from local graph evidence, applies them through
  `ChainCommand::SubmitTraceBisectionRound`, persists state, publishes bounded round payloads, and reports
  role counters.
- Iteration 169 (`091142d` plus metadata `16e9b17`, pushed `main` -> `main`): runtime roles detect local
  disputable graph evidence and open signed trace-bisection sessions through shared chain commands plus
  bounded gossip.
- Iterations 165 through 168 (`e3af101`, `bfcefa7`, `a93676b`, `54912ce`, all pushed `main` -> `main`):
  established bounded signed session-open gossip, explicit challenge reward claim boundaries,
  trace-bisection timeout slashing, and one-op referee slashing with delayed challenger rewards.
- Iteration 164 (`e42ad44`, pushed `main` -> `main`): one-op referee witness crosses node boundaries as a
  bounded gossip payload and is admitted only through shared node pending queue plus
  `ChainCommand::RefereeTraceBisection`.
- Iterations 158 through 163 (`6f6344a`, `713c6a4`, `2662d5a`, `f1372a4`, `02e288f`, `0487f77`, all
  pushed `main` -> `main`): established signed trace-bisection core, delayed block-check challenger
  rewards, bounded round wire payloads, chain session/round admission, node pending-queue round
  application, input-rooted trace openings, and chain-owned one-op referee verdicts.
- Iterations 143 through 157 established graph exact-op coverage, verified drand/network randomness,
  validator reveal proofs, finality-delayed proposer rewards, finalized side-branch convergence, durable
  restart-rehydrated tensor artifacts, deployment preflight/evidence surfaces, rolling restart evidence,
  richer IR/Tier-B execution, delayed reward maturity, claim-owned spendability, audit and challenge
  reward holds, exact trace openings, and local CPU Docker proof evidence.
