# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 170 validation complete: Runtime Trace-Bisection Round Progression.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Maturity release commands cannot move active matured rewards into spendable balances;
  explicit beneficiary `ClaimReward` remains the canonical spendability boundary. The trace-bisection path
  has signed sessions/rounds, bounded p2p round/referee payloads, node pending-queue application,
  input-rooted openings, runtime-generated signed session opens from local graph evidence, chain-owned
  one-op referee verdicts, responder timeout settlement, and chain-owned slashing plus delayed challenger
  reward claims.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: commit and push Iteration 170, then continue public/CUDA deployment evidence or
  trace-bisection DoS/expected-root negotiation work.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 170 `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, trace-bisection rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof, and trace-bisection p2p/chain/referee/timeout admission with runtime session-open and responder-round generation plus slashing/delayed challenger settlement | Add deployed public/CUDA proof or remaining fraud-proof DoS/negotiation policy |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, role-owned graph execution, `const_blob`, input-rooted p2p trace openings, signed trace-bisection core, bounded open/round/referee p2p payloads, runtime-generated session opens and responder rounds, state-rooted sessions/rounds, chain-owned referee/timeout verdicts and economic settlement | Continue CUDA graph evidence and remaining fraud-proof policy |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments; local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, block-check/trace-bisection challenger delayed bounties, pending claim view, study helper, calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 170: Runtime Trace-Bisection Round Progression

Feature capability: a miner/responder role with a locally reproducible graph execution for an active
trace-bisection session can build the next responder-signed midpoint `TraceBisectionRound`, apply it
through `ChainCommand::SubmitTraceBisectionRound`, persist the chain state, publish the bounded
`NewTraceBisectionRoundPayload`, and expose a role counter. Runtime round generation must not let a
challenger or unrelated node fabricate responder openings, and it must skip sessions where local exact
replay cannot prove the committed `trace_root`.

Readiness requirements covered: `upow.md` §8.2 midpoint opening / binary-search round progression over
`trace_root`, `upow.md` §9 trace-opening availability, `mvp_spec.md` §4.6 canonical runtime/transition
boundary, and the exec-plan gap for runtime-generated interactive transcript disputes.

Files/modules likely touched: app network helpers, miner role tick, node runtime state, runtime status
snapshot/output, local checker/status tests, focused runtime/network tests, and this execution plan.

Canonical owner: `chain::challenges` remains the owner of round admission, transcript-root advancement,
status changes, and duplicate/closed/timed-out rejection. Runtime code only derives a local opening,
submits the existing chain command, and gossips the existing bounded payload.

Adapter callers: miner role runtime calls the shared helper for the configured role wallet; p2p publish
uses `NewTraceBisectionRoundPayload`; status/checkers observe counters only.

Old shortcut being removed: trace-bisection rounds can be submitted by tests/direct commands or inbound
network payloads, but no role/runtime path currently generates the next responder round from local trace
evidence.

Regression test that proves the shortcut is gone: a runtime helper test opens an active graph
trace-bisection session for a miner responder, keeps the graph inputs/const blobs locally available, calls
the round-generation helper, observes a signed `NewTraceBisectionRoundPayload` plus state-rooted round
progress, and proves non-responder or missing-evidence nodes do not mutate state.

Behavior with local synthetic block production disabled: round generation depends only on accepted
receipt/session state and local tensor/program artifacts, not timed local production or block intervals.

Behavior for producer and non-producer roles: only the responder wallet can submit the responder-signed
round from local evidence; producer capability does not affect inbound or local round admission.

Structured evidence source: `trace_bisection_challenges`, `opened_rounds`, transcript/last-opening roots,
role miner trace-bisection round counters, p2p published round payload counters, and existing network
round ingest/application counters.

Finality source: unchanged block append/vote/finality; a round is an ordinary chain state transition and
does not finalize the fraud outcome by itself.

Wire-size and codec boundary: reuse `NewTraceBisectionRoundPayload` and
`encode_trace_bisection_round_payload`; no new wire format.

Parallel subagents to run: skipped; available subagent tooling requires explicit user authorization.
Read-only discovery is parallelized with local shell tools.

Parallelizable implementation workstreams: helper construction, miner tick integration, and status/checker
surfaces are separable, but the parent remains the single writer because they share runtime status types.

Tests/checkers/docs to add or update: focused runtime trace-bisection round-generation tests, miner runtime
state/status tests if touched, local checker role field assertions, and this execution plan.

Narrow validation commands:
- `cargo test -p tensor_vm trace_bisection_round_generation --lib -- --nocapture`
- `cargo test -p tensor_vm trace_bisection_round --lib -- --nocapture`
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --all -- --check`
- `cargo check --all-targets`
- `git diff --check`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`

Expected observable evidence: a responder miner with local exact graph evidence emits and applies exactly
the next midpoint round for an active session; the p2p message envelope matches the round context; the
challenge record advances or isolates through the canonical chain command; non-responder wallets, closed
sessions, trace-root mismatches, and absent local artifacts produce no mutation.

Out of scope: challenger-side expected-root negotiation beyond the current round model, automatic one-op
referee witness generation, multi-round DoS throttling, public/CUDA deployment evidence, and new codecs.

Split trigger: if supporting non-graph TensorOp/LinearTrainingStep openings requires broad artifact
plumbing or changes the signed round format, keep this iteration graph-only and split the additional
receipt families into a later feature.

Validation started on June 22, 2026:
- Gate 0 ordering note: read-only inspection ran before this iteration's local-testnet command during the
  resumed turn; the first acceptance gate still passed before edits:
  `cargo test -p tensor_vm local_testnet --release`.
- `cargo test -p tensor_vm trace_bisection_round_generation --lib -- --nocapture` passed.
- `cargo test -p tensor_vm trace_bisection_round --lib -- --nocapture` passed: 6 tests.
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture` passed: 2 tests.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces -- --nocapture` passed.
- `cargo fmt --all -- --check` passed.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 532 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.52%
  line coverage.
- Manual verifier-style diff review completed after validation; chain admission remains in
  `chain::challenges`, runtime round generation is responder-wallet scoped, requires local replay to match
  the committed `trace_root`, publishes only the existing bounded round payload, and status/checkers only
  observe typed counters.

### Iteration 169: Runtime Trace-Bisection Challenge Generation

Feature capability: a node role can detect a locally disputable graph receipt, create a signed
trace-bisection session-open payload from canonical chain/receipt state, apply it through the shared chain
command path, and publish the bounded p2p payload. Trace-bisection sessions should no longer be only direct
test/helper commands after a node has the receipt and trace evidence needed to challenge.

Readiness requirements covered: `upow.md` §8.2 interactive fraud-proof setup over `trace_root`,
`upow.md` §9 verification-time trace availability, `mvp_spec.md` §4.6 canonical runtime/transition
boundary, and the readiness gap for runtime-generated interactive transcript disputes.

Files/modules likely touched: role/runtime challenger or watcher code, app/network announcement helpers,
node/runtime status counters, chain/p2p payload helpers if needed, focused role/node tests, and this
execution plan.

Canonical owner: `chain::challenges` remains the only owner of challenge admission and state mutation.
Runtime code only chooses a candidate receipt from local evidence, builds a signed `TraceBisectionOpen`,
and submits/publishes the same bounded payload already accepted by node ingest.

Adapter callers: role/runtime watcher or validator loop calls shared chain command/application helpers and
shared p2p publish helpers. RPC/checkers/status only observe counters and challenge records.

Old shortcut being removed: trace-bisection sessions can currently be created by tests/direct commands, or
by manually submitting a signed network payload, but no role/runtime path generates the session when a
node has local disputable graph evidence.

Regression test that proves the shortcut is gone: a role/runtime test starts with a graph receipt and local
trace evidence, runs the challenge-generation tick, observes a signed `NewTraceBisectionOpenPayload`
publish attempt and a state-rooted challenge record, and proves a second tick is idempotent. A no-evidence
or TensorOp-only state must not publish or mutate.

Behavior with local synthetic block production disabled: challenge generation depends only on existing
chain receipts and local trace evidence, not local job production or block-production timers.

Behavior for producer and non-producer roles: any configured challenger-capable node with the evidence can
open a session through the same signed chain command; producer capability does not change inbound or local
challenge admission.

Structured evidence source: `trace_bisection_challenges`, role/runtime challenge-open counters, p2p
published `NewTraceBisectionOpenPayload` counters, and node status/checker fields if surfaced.

Finality source: unchanged block append/vote/finality; opening a challenge is an ordinary chain state
transition.

Wire-size and codec boundary: reuse `NewTraceBisectionOpenPayload` and
`encode_trace_bisection_open_payload`; no new wire format unless a missing status-only counter requires
typed local evidence.

Parallel subagents to run: skipped; available subagent tooling requires explicit user authorization.
Read-only discovery is parallelized with local shell tools.

Parallelizable implementation workstreams: runtime candidate selection, p2p publish plumbing, and status
tests are separable, but the parent remains the single writer because role/runtime files are shared.

Tests/checkers/docs to add or update: focused runtime/role test for generated session-open payloads,
existing trace-bisection focused tests if touched, p2p roundtrip only if the payload helper changes, and
this execution plan.

Narrow validation commands:
- `cargo test -p tensor_vm trace_bisection_challenge_generation --lib -- --nocapture`
- `cargo test -p tensor_vm trace_bisection_open --lib -- --nocapture`
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --all -- --check`
- `cargo check -p tensor_vm --tests`
- `git diff --check`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline`

Expected observable evidence: running the runtime challenge tick with a graph receipt whose committed
trace/output roots disagree with local exact replay produces one signed open payload whose envelope matches
the receipt/trace/challenger/responder. The chain records the session through `OpenSignedTraceBisection`,
valid replaying receipts and absent evidence produce no mutation, and duplicate ticks do not create
duplicate records or payloads.

Out of scope: automatic round selection after session open, multi-round DoS limits, public/CUDA evidence,
durable erasure-coded DA, and changing the signed-open wire format.

Split trigger: if runtime challenge generation requires broad role-loop scheduling or checker/Docker
changes, split the pure challenge-generation library path from Docker/status enforcement.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm trace_bisection_challenge_generation --lib -- --nocapture` passed.
- `cargo test -p tensor_vm trace_bisection_open --lib -- --nocapture` passed.
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture` passed.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture` passed.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces -- --nocapture` passed.
- `cargo fmt --all -- --check` passed.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 531 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.56%
  line coverage.
- Manual verifier-style diff review completed after validation; no unowned state mutation path or unsigned
  trace-bisection open path was introduced, and runtime session opens require local exact replay to
  disagree with the receipt before mutating state.

## Recent Iterations

### Iteration 168: Signed Trace-Bisection Session-Open Gossip

Commits: `54912ce` plus completion metadata `16e9b17` (pushed `main` -> `main`). Validation passed with
Gate 0, focused trace-bisection/open/p2p/pending tests, full lib, release local-testnet, tarpaulin 84.65%,
and manual review. Capability: bounded signed session-open gossip enters state only through
`ChainCommand::OpenSignedTraceBisection`, with tamper rejection, unknown-receipt queuing, retry
application, and duplicate replay tests.

### Iteration 167: Explicit Challenge Reward Claim Boundary

Commit: `a93676b` (pushed `main` -> `main`). Capability: successful block-check and trace-bisection
challenger bounties remain state-rooted pending claims until beneficiary `ClaimReward`; matured challenge
reward sweeps do not credit active claims. Validation passed Gate 0, focused `challenge_reward`/`reward`,
fmt/check/diff/lib/final Gate 0, tarpaulin 84.58%, and manual review.

### Iteration 166: Trace-Bisection Timeout Slashing And Delayed Challenger Rewards

Commit: `bfcefa7` (pushed `main` -> `main`).

Feature capability: a chain-owned trace-bisection timeout settles the economic side without adapter
workarounds. When the responder/miner forfeits by timeout, the chain slashes the responder bond, credits
treasury with the net slash, creates a delayed `PendingChallengeReward` claim for the challenger, keeps
spendability behind beneficiary `ClaimReward`, voids affected receipt rewards, and removes affected
TensorWork from pending/settled blockspace as appropriate.

Validation passed on June 22, 2026: first-command Gate 0; focused `records_timeout`, `trace_bisection`,
`reward`, and `chain::tests::challenges`; broad `cargo fmt --check --all`, `cargo check -p tensor_vm
--tests`, `git diff --check`, `cargo test -p tensor_vm --lib` (526 passed); final Gate 0. Tarpaulin was not
yet installed during this iteration, and the old verifier-binary workflow requirement was later corrected
as stale.

### Iteration 165: Trace-Bisection Referee Slashing And Delayed Challenger Rewards

Commit: `e3af101` (pushed `main` -> `main`).

Feature capability: a chain-owned one-op trace-bisection referee verdict settles slashing and delayed
challenger bounty claims without adapter workarounds. Losing registered miner/validator stake is slashed,
treasury receives the net slash, a winning challenger receives a delayed `PendingChallengeReward` when the
responder/miner loses, and spendability remains behind `ClaimReward`.

Validation passed on June 22, 2026: first-command Gate 0; focused `trace_bisection_referee`, `reward`,
`chain::tests::challenges`, `pending_payloads`; broad fmt/check/diff/lib (526 passed); final Gate 0.
Tarpaulin was not yet installed during this iteration.

## Decision Log

- Gate 0 remains `cargo test -p tensor_vm local_testnet --release` and must be the first executable command
  on every resume before edits.
- Chain validation is the canonical owner for accepted randomness proof verification, typed proof metadata,
  state-rooted records, finalized beacon advancement, and seed derivation.
- Runtime may observe wall-clock public endpoint freshness only for locally fetched public drand. Chain
  validation/state own the accepted public drand anchor and deterministic chain-epoch round window.
- Reward delays, reveal holds, slashing, challenge settlement, and spendability are chain-owned pending
  claim/state transitions. Checkers and runtime surfaces only observe these states.
- Bounded p2p/node payloads remain the only network wire surface for randomness, reveal records, and
  trace-bisection round/referee evidence.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, full expected-root negotiation,
  automatic referee witness generation, and multi-round DoS policy remain deployment or future-feature
  gates, not local-completion claims.

## Validation Evidence

- Iteration 170 validation passed on June 22, 2026: Gate 0 after resumed-turn read-only inspection,
  focused `trace_bisection_round_generation`, `trace_bisection_round`, `tvmd_runtime::runtime_state`,
  and `tvmd_cli::role_run_commands_serve_through_role_specific_surfaces`; broad fmt/check/diff/lib
  (532 passed), release local-testnet, and tarpaulin 84.52%.
- Iteration 166 feature commit `bfcefa7` pushed to `main` on June 22, 2026:
  `git push` returned `7ab5580..bfcefa7  main -> main`.
- Iteration 165 feature commit `e3af101` pushed to `main` on June 22, 2026:
  `git push` returned `f7a5fe6..e3af101  main -> main`.

## Archive

- Iteration 164 (`e42ad44`, pushed `main` -> `main`): one-op referee witness crosses node boundaries as a
  bounded gossip payload and is admitted only through shared node pending queue plus
  `ChainCommand::RefereeTraceBisection`.
- Iterations 158 through 163 (`6f6344a`, `713c6a4`, `2662d5a`, `f1372a4`, `02e288f`, `0487f77`, all
  pushed `main` -> `main`): established signed trace-bisection core, delayed block-check challenger
  rewards, bounded round wire payloads, chain session/round admission, node pending-queue round
  application, input-rooted trace openings, and chain-owned one-op referee verdicts.
- Iteration 157 (`fc14b63`, pushed `main` -> `main`; metadata `7d4e172`): graph-verifier exact-op coverage
  for admitted Tier-B op clusters.
- Iterations 143 through 156 established verified drand/network randomness, production validator reveal
  proofs, finality-delayed proposer rewards, finalized side-branch convergence, durable
  restart-rehydrated tensor artifacts, deployment preflight/evidence surfaces, rolling restart evidence,
  richer IR/Tier-B execution, delayed reward maturity, claim-owned spendability, audit and challenge
  reward holds, exact trace openings, and related local CPU Docker proof evidence. Detailed historical
  command transcripts and commit hashes are preserved in git history.
