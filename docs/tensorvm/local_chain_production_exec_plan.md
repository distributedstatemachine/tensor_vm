# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: none; Iteration 165 implementation and validation are complete, commit/push metadata is pending.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Maturity release commands cannot move matured rewards into spendable balances; explicit
  `ClaimReward` remains the canonical spendability boundary. The trace-bisection path now has signed
  sessions/rounds, bounded p2p round and referee payloads, node pending-queue application, input-rooted
  trace openings, chain-owned one-op referee verdicts, and referee economic settlement: the losing
  registered miner or validator stake is slashed from the session bond envelope, treasury receives the net
  slash, and a winning challenger receives only a delayed `PendingChallengeReward` claim.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: choose the next feature-sized slice. Current high-value options are trace-bisection
  session-open/runtime challenge generation, timeout economic settlement and multi-round DoS policy,
  deployed full VRF lifecycle evidence, public/CUDA deployment evidence, or remaining exact Tier-B/CUDA
  conformance surface.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | `cargo test -p tensor_vm local_testnet --release` passed as first command and final gate for Iteration 165 on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, trace-bisection rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof, and trace-bisection p2p/chain/referee admission with slashing/delayed challenger settlement | Add runtime challenge generation, timeout settlement, or deployed public/CUDA proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, role-owned graph execution, `const_blob`, input-rooted p2p trace openings, signed trace-bisection core, bounded round/referee p2p payloads, state-rooted sessions/rounds, chain-owned referee verdicts and economic settlement | Continue session-open/runtime generation and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments. Local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, block-check challenger delayed bounties, trace-bisection referee slashing/delayed challenger rewards, pending claim view, study helper, calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

No active feature is checkpointed. Start the next slice with the required Gate 0 command and a fresh
checkpoint before edits.

## Recent Iterations

### Iteration 165: Trace-Bisection Referee Slashing And Delayed Challenger Rewards

Commit: pending.

Feature capability: a chain-owned one-op trace-bisection referee verdict settles the economic side without
adapter workarounds. The losing registered miner or validator is slashed through consensus state, treasury
receives the net slash, a winning challenger receives a delayed `PendingChallengeReward` claim when the
responder/miner loses, and spendability remains behind beneficiary `ClaimReward`.

Readiness requirements covered: `upow.md` §8.2 loser slashing and challenger bounty, §12.1 pending
challenger rewards before spendability, §12.2 miner slashing for failed fraud-proof verification, and
`mvp_spec.md` §4.6 chain transition ownership for challenge outcomes.

Canonical owner: `chain::challenges` owns referee verdict settlement, stake mutation, treasury accounting,
pending challenge reward creation, and duplicate/idempotency rejection. `node::payload_application`,
pending queues, p2p codecs, runtime loops, and checkers only carry or observe payloads and must not
materialize rewards or slash balances.

Adapter callers: direct `ChainCommand::RefereeTraceBisection`, p2p referee payload application, and pending
payload retry. All callers use the same chain command.

Old shortcut removed: trace-bisection referee verdicts no longer record dishonest-party evidence while
leaving slashing and challenger settlement to later/manual work. The economic boundary is chain-owned.

Regression evidence: applying a referee verdict against a receipt miner reduces miner stake, credits
treasury for slash net of bounty, creates a delayed challenge reward claim for the challenger, does not
credit spendable balance before maturity, and releases only after `ClaimReward`. A verdict against a
dishonest challenger slashes the challenger/validator side and creates no challenger bounty. Network
fixtures register the challenger as a validator so retry/application tests exercise real slashable state.

Behavior with local synthetic block production disabled: unchanged; referee settlement is an explicit
chain command independent of synthetic jobs, local producer mode, block proposal, or finality helpers.

Behavior for producer and non-producer roles: both roles ingest the same referee payload and settle only
through `ChainCommand::RefereeTraceBisection`; no role-specific reward preparation is allowed.

Structured evidence source: `trace_bisection_challenges`, `pending_challenge_rewards`, miner/validator
stake maps, treasury balance, reward-claim views, chain events, state roots, and snapshot codec.

Finality source: unchanged block append/vote/finality. Referee settlement is a normal chain state
transition and reward spendability remains delayed until claim maturity plus `ClaimReward`.

Wire-size and codec boundary: no new wire type; this reuses the bounded referee payload added in Iteration
164 and changes only chain/state economic effects.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm trace_bisection_referee --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm reward --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm chain::tests::challenges --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm pending_payloads --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (526 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

Out of scope: session-open gossip, runtime automatic challenge generation, multi-round DoS policy, timeout
economic settlement, public/CUDA evidence, and changing the bounded referee wire payload.

### Iteration 164: Trace-Bisection Referee Payload Gossip And Node Application

Commit: `e42ad44` (pushed `main` -> `main`).

Feature capability: a one-op referee witness for an isolated trace-bisection dispute crosses node
boundaries as a bounded gossip payload and is admitted only through the shared node pending queue plus
canonical `ChainCommand::RefereeTraceBisection` path.

Readiness requirements covered: `upow.md` §8.2 interactive fraud proofs over `trace_root`, §9 trace
opening availability, `mvp_spec.md` §4.6 canonical transition boundary, and the Iteration 163 out-of-scope
item for p2p referee payloads.

Canonical owner: `chain::challenges` remains the only owner of referee admission, verdict state, duplicate
rejection, and economic effects. P2P/node only decode, bound, queue, retry, and call the chain command.

Validation passed on June 22, 2026: first-command Gate 0, focused
`trace_bisection_referee`, `trace_bisection_round_payload`, `pending_payloads`,
`network_event_driver_applies_and_retries_trace_bisection_referee_payloads`, `p2p_messages_roundtrip`,
`cargo fmt --check --all`, `cargo check -p tensor_vm --tests`, `git diff --check`,
`cargo test -p tensor_vm --lib` (525 passed), and final Gate 0. Tarpaulin remained blocked because
`cargo-tarpaulin` is not installed.

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
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, runtime-generated interactive
  transcript disputes, and timeout economic settlement remain deployment or future-feature gates, not
  local-completion claims.

## Validation Evidence

- Iteration 165 first executable command passed before edits on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 165 focused validation passed on June 22, 2026:
  `cargo test -p tensor_vm trace_bisection_referee --lib -- --nocapture`;
  `cargo test -p tensor_vm reward --lib -- --nocapture`;
  `cargo test -p tensor_vm chain::tests::challenges --lib -- --nocapture`; and
  `cargo test -p tensor_vm pending_payloads --lib -- --nocapture`.
- Iteration 165 broad validation passed on June 22, 2026:
  `cargo fmt --check --all`; `cargo check -p tensor_vm --tests`; `git diff --check`;
  `cargo test -p tensor_vm --lib` (526 passed); and final
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 165 coverage command remained environmentally blocked on June 22, 2026:
  `cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.
- Iteration 165 commit/push metadata: pending.

## Archive

- Iteration 163 (`0487f77`, pushed `main` -> `main`): input-rooted trace openings and chain-owned
  one-op referee verdicts from explicit witness values. Validation included first-command Gate 0, focused
  trace-opening/bisection/referee tests, full lib tests, and final Gate 0.
- Iteration 162 (`02e288f`, pushed `main` -> `main`): node payload application for bounded
  trace-bisection rounds through shared pending queues into `SubmitTraceBisectionRound`, with retry and
  invalid-edge tests.
- Iteration 161 (`f1372a4`, pushed `main` -> `main`): chain admission for signed trace-bisection sessions
  and rounds with state-rooted dispute records, transcript-root advancement, isolation, and timeout records.
- Iteration 160 (`2662d5a`, pushed `main` -> `main`): bounded p2p wire payloads for signed
  trace-bisection rounds with identity/wrapper checks and malformed-edge coverage.
- Iteration 159 (`713c6a4`, pushed `main` -> `main`): chain-owned delayed block-check challenger rewards;
  adapters no longer materialize proposer rewards as a workaround before challenge admission.
- Iteration 158 (`6f6344a`, pushed `main` -> `main`): signed deterministic trace-bisection dispute core
  over verified `IrTraceOpening` values.
- Iteration 157 (`fc14b63`, pushed `main` -> `main`; metadata `7d4e172`): graph-verifier exact-op
  coverage for admitted Tier-B op clusters.
- Iterations 143 through 156 established verified drand/network randomness, production validator reveal
  proofs, finality-delayed proposer rewards, finalized side-branch convergence, durable restart-rehydrated
  tensor artifacts, deployment preflight/evidence surfaces, rolling restart evidence, richer IR/Tier-B
  execution, delayed reward maturity, claim-owned spendability, audit and challenge reward holds, exact
  trace openings, and related local CPU Docker proof evidence. Detailed historical command transcripts and
  commit hashes are preserved in git history.
