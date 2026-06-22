# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 172 ready to commit: Trace-Bisection Expected-Root Gossip.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Maturity release commands cannot move active matured rewards into spendable balances;
  explicit beneficiary `ClaimReward` remains the canonical spendability boundary. The trace-bisection path
  has signed sessions/rounds, bounded p2p expectation/round/referee payloads, node pending-queue
  application, input-rooted openings, runtime-generated signed session opens and challenger
  expected-root claims from local graph evidence, chain-owned challenger expected-root claims before
  responder rounds, one-op referee verdicts, responder timeout settlement, and chain-owned slashing plus
  delayed challenger reward claims.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: commit/push Iteration 172 and record the hash.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 171 `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, trace-bisection expectations/rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof, and trace-bisection p2p/chain/expectation/referee/timeout admission with runtime session-open, challenger-expectation, and responder-round generation plus slashing/delayed challenger settlement | Add deployed public/CUDA proof or remaining fraud-proof DoS policy |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, role-owned graph execution, `const_blob`, input-rooted p2p trace openings, signed trace-bisection core, bounded open/expectation/round/referee p2p payloads, runtime-generated session opens, challenger expectations, and responder rounds, state-rooted sessions/expectations/rounds, chain-owned referee/timeout verdicts and economic settlement | Continue CUDA graph evidence and remaining fraud-proof DoS/referee automation policy |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments; local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, block-check/trace-bisection challenger delayed bounties, pending claim view, study helper, calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 172: Trace-Bisection Expected-Root Gossip

Feature capability: challenger-signed trace-bisection expected-root claims cross the same bounded
p2p/node/runtime path as trace-bisection opens, rounds, and referee witnesses. A validator/challenger with
local graph evidence can submit and gossip the expectation for the active midpoint; non-producers apply or
queue the payload through `ChainCommand::SubmitTraceBisectionExpectation`; responder rounds remain pending
until that canonical expectation arrives.

Readiness requirements covered: `upow.md` §8.2 challenger/responder expected-root negotiation,
`mvp_spec.md` §4.6 bounded p2p/transition boundary, and the exec-plan gap for p2p/runtime expected-root
claim gossip.

Files/modules likely touched: `p2p/wire.rs`, `node/runtime_state.rs`, `node/pending_payloads.rs`,
`node/payload_application.rs`, `node/message_ingest.rs`, `node/payload_processor.rs`, app network/validator
role/status surfaces, focused p2p/node/runtime tests, `upow.md`, and this plan.

Canonical owner: `chain::challenges` remains the only owner of expectation admission, pending expected
roots, and round progression. P2P/node/runtime only decode, queue, submit the chain command, publish, and
report typed counters.

Adapter callers: validator/challenger role runtime generates the expectation from local exact replay;
non-producers ingest the bounded payload; miner/responder runtime consumes the already-admitted pending
expectation when generating the responder round.

Old shortcut being removed: after Iteration 171, expectations are chain-required but only local direct
commands/tests can provide them; cross-node responder progression can stall without a network-visible
expectation message.

Regression test that proves the shortcut is gone: network event tests queue an expectation before the
session exists, apply it after the open payload is admitted, then retry a previously pending round. Runtime
tests prove a challenger submits/publishes the expectation from local replay and duplicate ticks are
idempotent.

Behavior with local synthetic block production disabled: expectation generation depends only on accepted
graph receipts, active challenge state, and local tensor/program artifacts; it does not require timed local
job or block production.

Behavior for producer and non-producer roles: producer capability is irrelevant. Any node with the
challenger wallet and evidence can publish the signed expectation; all roles ingest and apply it through
the same pending payload path.

Structured evidence source: typed runtime/network counters for expected-root claims, pending payload
counts, `trace_bisection_challenges.pending_expectation_leaf`, and focused node/runtime tests.

Finality source: unchanged block append/vote/finality; expectation admission is an ordinary chain state
transition.

Wire-size and codec boundary: add one bounded `NewTraceBisectionExpectationPayload` using the existing
trace-bisection expected-root limit and shared p2p codec tests.

Parallel subagents to run: skipped; available subagent tooling requires explicit user authorization.
Read-only discovery is parallelized with local shell tools.

Parallelizable implementation workstreams: wire/pending/status/runtime pieces are coupled by shared
counters and message variants, so the parent remains the single writer.

Tests/checkers/docs to add or update: p2p malformed/roundtrip tests, node payload application and retry
tests, runtime generation/status tests, `upow.md` §16 status text, and this execution plan.

Narrow validation commands:
- `cargo test -p tensor_vm trace_bisection_expectation --lib -- --nocapture`
- `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --all -- --check`
- `cargo check --all-targets`
- `git diff --check`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`

Expected observable evidence: a signed expectation payload can be gossiped, decoded, queued until its
challenge exists, applied once through the chain command, exposed through typed counters, and allows a
queued responder round to progress only after the expectation is admitted.

Out of scope: automatic one-op referee witness generation, multi-round DoS throttling, public/CUDA
deployment evidence, and changing the round/referee wire formats.

Split trigger: if adding validator runtime expectation generation requires broad role scheduling changes,
split the p2p/node expectation transport from the runtime generation helper.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo check -p tensor_vm --tests` passed during integration.
- `cargo test -p tensor_vm trace_bisection_expectation --lib -- --nocapture` passed: 4 tests.
- `cargo test -p tensor_vm pending_payloads --lib -- --nocapture` passed: 5 tests.
- `cargo test -p tensor_vm trace_bisection --lib -- --nocapture` passed: 22 tests.
- `cargo test -p tensor_vm runtime_state --test tvmd_runtime -- --nocapture` passed: 2 tests.
- `cargo test -p tensor_vm network_event_driver_applies_and_retries_trace_bisection --lib -- --nocapture`
  passed: 4 tests.
- `cargo fmt --all -- --check` passed.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 536 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.52%
  line coverage.
- Manual verifier-style diff review completed after validation; expectation payloads are bounded and
  signature-checked at the wire boundary, node/runtime paths only queue, submit
  `ChainCommand::SubmitTraceBisectionExpectation`, publish, and count typed events, while
  `chain::challenges` remains the only owner of pending expected roots and round unblocking.

### Iteration 171: Trace-Bisection Challenger Expected Roots

Feature capability: before a responder-signed trace-bisection round can narrow or isolate a dispute, the
chain must first admit a challenger-signed expected-root claim for the current midpoint. Round admission
must reject responder-selected `expected_output_roots` unless they exactly match that pending challenger
claim, then clear the claim after the round is consumed.

Readiness requirements covered: `upow.md` §8.2 challenger/responder trace-root negotiation, `upow.md` §5
signed canonical dispute records, `mvp_spec.md` §4.6 canonical transition ownership, and the exec-plan gap
for expected-root negotiation beyond the current responder-carried round model.

Files/modules likely touched: `challenge.rs`, `chain/state.rs`, `chain/challenges.rs`,
`chain/engine.rs`, `chain/commands.rs`, `chain/roots.rs`, `storage/chain_state.rs`, focused chain/storage
tests, and this execution plan.

Canonical owner: `chain::challenges` owns expected-root claim admission, pending-claim state, round
matching, claim clearing, and transcript advancement.

Adapter callers: tests and future runtime/p2p adapters use a new `ChainCommand` for the signed
expectation. Existing responder round payloads remain bounded network inputs, but they no longer advance
state unless the canonical pending expectation exists.

Old shortcut being removed: the responder-signed round currently carries `expected_output_roots` and can
therefore choose the branch condition it asks the chain to apply.

Regression test that proves the shortcut is gone: a focused chain test opens a session, proves a round is
rejected without a challenger expectation, admits a signed expectation for the active midpoint, rejects a
round with mismatched responder-carried expected roots, accepts the matching round, clears the expectation,
and state-roots/persists the pending claim.

Behavior with local synthetic block production disabled: expectation and round admission depend only on
existing chain challenge state and signatures, not job production or block timers.

Behavior for producer and non-producer roles: producer capability is irrelevant; all nodes must apply the
same chain commands and reject responder-only branch selection.

Structured evidence source: `trace_bisection_challenges` with pending expectation leaf/roots, focused
chain events/tests, state root changes, and chain-state snapshot roundtrip.

Finality source: unchanged block append/vote/finality; expected-root claims and rounds are ordinary chain
state transitions.

Wire-size and codec boundary: no new network wire format in this slice. The new signed expectation is a
chain record/command boundary; p2p/runtime gossip for it remains a later integration slice.

Parallel subagents to run: skipped; available subagent tooling requires explicit user authorization.
Read-only discovery is parallelized with local shell tools.

Parallelizable implementation workstreams: challenge type/state admission, state-root/storage encoding,
and tests are related but touch the same core files, so the parent remains the single writer.

Tests/checkers/docs to add or update: focused trace-bisection expected-root chain tests, storage snapshot
roundtrip coverage if existing tests do not exercise the new fields, `upow.md` §16 status text, and this
execution plan.

Narrow validation commands:
- `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`
- `cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --all -- --check`
- `cargo check --all-targets`
- `git diff --check`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`

Expected observable evidence: challenger expectations are signature-checked and keyed to the active
challenge midpoint; responder rounds cannot self-select branch roots; consumed expectations are cleared;
the state root and snapshot codec commit the pending expectation.

Out of scope: p2p/runtime gossip for expectation claims, automatic challenger expectation generation,
automatic referee witness generation, multi-round DoS throttling, and public/CUDA deployment evidence.

Split trigger: if enforcing expectations breaks existing p2p/runtime round ingestion broadly, keep this
iteration to the chain/state boundary and document the follow-up runtime/gossip requirement before commit.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo check -p tensor_vm --tests` passed.
- `cargo test -p tensor_vm trace_bisection --lib -- --nocapture` passed: 18 tests.
- `cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib -- --nocapture` passed.
- `cargo test -p tensor_vm trace_bisection_round_generation --lib -- --nocapture` passed.
- `cargo test -p tensor_vm trace_bisection_round_payload_application --lib -- --nocapture` passed.
- `cargo test -p tensor_vm network_event_driver_applies_and_retries_trace_bisection --lib -- --nocapture` passed: 3 tests.
- `cargo fmt --all -- --check` passed after applying `cargo fmt --all`.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 532 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.55%
  line coverage.
- Manual verifier-style diff review completed after validation; challenger expectations are admitted only
  through `chain::challenges`, pending expected roots are state-rooted and snapshot-persisted, responder
  rounds are still bounded existing p2p payloads but now remain pending or rejected unless the canonical
  pending expectation exists, and runtime responder generation skips sessions without an expectation.
- Feature commit `6901655` pushed to `main` on June 22, 2026:
  `git push` returned `04a85d4..6901655  main -> main`.

## Recent Iterations

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
  trace-bisection expectation/round/referee evidence.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, automatic referee witness
  generation, and multi-round DoS policy remain deployment or future-feature gates, not local-completion
  claims.

## Validation Evidence

- Iteration 172 validation passed on June 22, 2026: first executable Gate 0; focused
  `trace_bisection_expectation`, `pending_payloads`, `trace_bisection`, runtime-state, and network retry
  tests; broad fmt/check/diff/lib (536 passed), release local-testnet, tarpaulin 84.52%, and manual review.
- Iteration 171 feature commit `6901655` pushed to `main` on June 22, 2026:
  `git push` returned `04a85d4..6901655  main -> main`.

## Archive

- Iteration 170 (`d88a14d` plus metadata `04a85d4`, pushed `main` -> `main`): runtime responder round
  generation builds signed midpoint rounds from local graph evidence, applies them through
  `ChainCommand::SubmitTraceBisectionRound`, persists state, publishes bounded round payloads, and reports
  role counters.
- Iteration 169 (`091142d` plus metadata `16e9b17`, pushed `main` -> `main`): runtime roles detect local
  disputable graph evidence and open signed trace-bisection sessions through shared chain commands plus
  bounded gossip.
- Iterations 165 through 168 (`e3af101`, `bfcefa7`, `a93676b`, `54912ce`, all pushed `main` -> `main`):
  established bounded signed session-open gossip, explicit challenge reward claim boundaries, trace-bisection
  timeout slashing, and one-op referee slashing with delayed challenger rewards.
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
