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
  verified IR trace openings now exists; bounded p2p wire payloads can carry signed bisection rounds; and
  chain admission now records sessions, signed rounds, transcript-root progress, isolated-op outcomes, and
  responder timeouts in state-rooted dispute records. Node-side application of bounded trace-bisection
  round payloads now uses the shared pending queue and canonical chain command path with status counters.
  One-op referee execution, slashing, challenger settlement, session-open gossip, and runtime generation
  remain open.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: choose the next feature-sized slice. Current high-value options are one-op referee execution
  for isolated trace-bisection disputes, session-open/runtime challenge generation, deployed full VRF
  lifecycle evidence, public/CUDA deployment evidence, or the remaining exact Tier-B/CUDA conformance
  surface.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | `cargo test -p tensor_vm local_testnet --release` passed as first command and final gate for Iteration 162 on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, trace-bisection rounds, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof, and trace-bisection p2p/chain admission | Add referee execution or deployed public/CUDA proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, focused admitted exact-op graph-verifier evidence, role-owned graph execution, `const_blob`, p2p trace openings, signed trace-bisection core, bounded trace-bisection round p2p payloads, state-rooted chain admission for trace-bisection sessions/rounds, and node pending-queue application | Continue one-op referee execution, session-open/runtime generation, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments. Local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit reward maturity, VRF reveal holds, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

No active feature is checkpointed. Start the next slice with the required Gate 0 command and a fresh
checkpoint before edits.

## Recent Iterations

### Iteration 162: Trace-Bisection Node Payload Application

Commit: `02e288f` (pushed `main` -> `main`).

Feature capability: bounded trace-bisection round p2p payloads should be applied through the shared node
payload processor into `ChainCommand::SubmitTraceBisectionRound` when the matching chain session exists, and
queued for retry when the session is not known yet.

Readiness requirements covered: `upow.md` §8.2 interactive fraud proofs over `trace_root`, §9 trace opening
availability, `mvp_spec.md` §4.6 canonical transition boundary, and the Iteration 160/161 out-of-scope item
for p2p pending-queue application counters.

Canonical owner: `chain::challenges` owns transcript verification, duplicate/state rejection, narrowed or
isolated status transitions, and all economic effects. `node::payload_application` owns bounded decode,
message wrapper consistency, idempotency classification, and returning pending when the chain session is
absent. `PendingNetworkPayloads` owns bounded retry identity and counters.

Adapter callers: `node::message_ingest`, `ChainNetworkPayloadProcessor`, and pending payload retry.

Shortcut being avoided: `NewTraceBisectionRoundPayload` must not remain a known gossip event that only
increments block-announcement counters. It also must not mutate dispute/economic state outside the chain
command path.

Expected observable evidence: ingesting a valid round for an existing trace-bisection session records the
round in `trace_bisection_challenges`, increments trace-bisection ingested/applied counters, and preserves
local producer neutrality. Ingesting a round before its session exists queues it, retrying applies it after
the session is opened. Mismatched wrapper fields, zero identities, malformed payloads, and conflicting
duplicates are invalid. One-op referee re-execution, slashing, challenger bounty settlement, session-open
gossip, runtime challenge generation, public/CUDA evidence, and multi-round DoS policy remain out of scope.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed on June 22,
  2026.
- Focused: `cargo test -p tensor_vm trace_bisection_round_payload --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm pending_payloads --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm network_event_driver_applies_and_retries_trace_bisection_round_payloads --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (522 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

### Iteration 161: Trace-Bisection Chain Admission

Commit: `f1372a4` (pushed `main` -> `main`).

Feature capability: signed trace-bisection sessions and rounds should be admitted through the canonical
chain command path, recorded in state-rooted dispute records, and exposed as chain events before any node,
p2p, or runtime adapter treats a round as protocol progress.

Readiness requirements covered: `upow.md` §8.2 interactive fraud proofs over `trace_root`, §9 trace opening
availability, `mvp_spec.md` §4.6 canonical transition boundary, and the Iteration 160 out-of-scope item for
chain command admission.

Canonical owner: `chain::challenges` owns trace-bisection session creation, round verification against the
current state, transcript-root advancement, duplicate rejection, timeout recording, and final isolated-op
state. P2P/node surfaces remain bounded codec and retry adapters.

Shortcut being avoided: the new trace-bisection p2p payload must not become a network-only acknowledgement
path. A round only counts after `ChainCommand` applies it to the stored session state.

Parallel subagents to run: skipped; the available subagent tool requires explicit user authorization before
spawning agents. Read-only code/test discovery is being parallelized with local shell tools.

Expected observable evidence: chain tests create a graph receipt dispute session, apply a signed midpoint
round through `ChainCommand`, observe a state-rooted narrowed transcript, reject duplicate/tampered/mismatched
rounds, record an isolated disputed op after the final round, and record responder timeout without mutating
stake or rewards. One-op referee re-execution, slashing, challenger bounty settlement, runtime challenge
generation, public/CUDA evidence, and p2p pending-queue application counters remain out of scope.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm challenge --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm chain_state --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm chain::tests::challenges --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (520 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

### Iteration 160: Trace-Bisection P2P Wire Payloads

Commit: `2662d5a` (pushed `main` -> `main`).

Feature capability: signed `TraceBisectionRound` values can cross node boundaries as bounded shared-codec
gossip payloads with message-level identity checks for receipt, trace root, parties, and transcript leaf.

Readiness requirements covered: `upow.md` §8.2 interactive fraud proofs over `trace_root`, §9 trace opening
availability, `goal.md` shared-codec and bounded-wire rules, and the Iteration 158 out-of-scope item for
p2p wire messages.

Files/modules touched: `crates/tensor_vm/src/api.rs`, `crates/tensor_vm/src/p2p/wire.rs`,
`crates/tensor_vm/src/p2p.rs`, `crates/tensor_vm/src/node/message_ingest.rs`, `docs/tensorvm/upow.md`,
and this execution plan.

Parallel subagents to run: skipped; the available subagent tool requires explicit user authorization before
spawning agents. Read-only code/test discovery is being parallelized with local shell tools.

Expected observable evidence: `P2pMessage::NewTraceBisectionRoundPayload` gossips on the blocks topic,
decodes only when the bounded payload's decoded round matches the announced receipt, trace root, parties,
and transcript leaf, rejects oversize/trailing/tampered opening or signature bytes, and reuses the existing
trace-opening payload codec instead of adding an unbounded reader.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm trace_bisection_round_payload --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm p2p_messages_roundtrip --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm libp2p_mapping_separates_gossip_and_request_response --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm network_ingest_order_applies_payload_dependencies_before_blocks --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (518 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

Out of scope: chain command admission, pending queues/application counters, one-op referee re-execution,
stake mutation, challenger settlement, runtime challenge generation, deployed evidence, and CUDA evidence.

### Iteration 159: Chain-Owned Delayed Challenge Rewards

Commit: `713c6a4` (pushed `main` -> `main`).

Feature capability: `SubmitBlockCheckChallenge` should create or consume the canonical delayed proposer
reward claim before resolving a canonical block-check challenge, so challenger bounty accounting is a
state-rooted delayed claim instead of a node/network adapter workaround.

Requirements covered: `mvp_spec.md` reward settlement and block-check challenge sections require challenge
rewards to release only after maturity; `local_chain_production_readiness.md` requires chain-owned full
reward-maturity delay for challenge bounties; `goal.md` forbids working around incomplete protocol behavior
in adapter code.

Canonical owner: chain challenge admission owns delayed reward materialization and voiding. Node payload
application remains a bounded decode/idempotency adapter and must not force reward materialization as a
precondition.

Shortcut being removed: `node::payload_application::prepare_block_check_challenge_reward` currently calls
`materialize_finalized_proposer_rewards()` to make network challenge application succeed. This iteration
moves that behavior into the chain command path.

Files/modules touched: `crates/tensor_vm/src/chain/challenges.rs`,
`crates/tensor_vm/src/node/payload_application.rs`, `crates/tensor_vm/src/chain.rs`,
`crates/tensor_vm/src/chain/tests/challenges.rs`, `docs/tensorvm/upow.md`, and this execution plan.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm canonical_block_check_challenge_materializes_and_delays_reward_in_chain --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm block_check_challenge --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm reward --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm payload_application::tests::block_check --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (517 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

Out of scope: trace-bisection p2p wire payloads, interactive dispute chain admission, stake slashing beyond
the already implemented block-check path, public/CUDA evidence, and any immediate spendable challenge
credit.

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
- Iteration 159 first executable command passed before code edits on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 159 feature commit `713c6a4` pushed to `main` on June 22, 2026:
  `git push` returned `44951a9..713c6a4  main -> main`.
- Iteration 159 focused validation passed on June 22, 2026:
  `cargo test -p tensor_vm canonical_block_check_challenge_materializes_and_delays_reward_in_chain --lib -- --nocapture`;
  `cargo test -p tensor_vm block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges --lib -- --nocapture`;
  `cargo test -p tensor_vm block_check_challenge --lib -- --nocapture`;
  `cargo test -p tensor_vm reward --lib -- --nocapture`; and
  `cargo test -p tensor_vm payload_application::tests::block_check --lib -- --nocapture`.
- Iteration 159 broad validation passed on June 22, 2026:
  `cargo fmt --check --all`; `cargo check -p tensor_vm --tests`; `git diff --check`;
  `cargo test -p tensor_vm --lib` (517 passed); and final
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 159 coverage command remained environmentally blocked on June 22, 2026:
  `cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.
- Iteration 160 first executable command passed before code edits on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 160 feature commit `2662d5a` pushed to `main` on June 22, 2026:
  `git push` returned `0c98e71..2662d5a  main -> main`.
- Iteration 160 focused validation passed on June 22, 2026:
  `cargo test -p tensor_vm trace_bisection_round_payload --lib -- --nocapture`;
  `cargo test -p tensor_vm trace_bisection --lib -- --nocapture`;
  `cargo test -p tensor_vm p2p_messages_roundtrip --lib -- --nocapture`;
  `cargo test -p tensor_vm libp2p_mapping_separates_gossip_and_request_response --lib -- --nocapture`;
  and `cargo test -p tensor_vm network_ingest_order_applies_payload_dependencies_before_blocks --lib -- --nocapture`.
- Iteration 160 broad validation passed on June 22, 2026:
  `cargo fmt --check --all`; `cargo check -p tensor_vm --tests`; `git diff --check`;
  `cargo test -p tensor_vm --lib` (518 passed); and final
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 160 coverage command remained environmentally blocked on June 22, 2026:
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
