# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 167 implementation and validation complete; commit and push pending.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Maturity release commands cannot move active matured rewards into spendable balances;
  explicit beneficiary `ClaimReward` remains the canonical spendability boundary. The trace-bisection path
  has signed sessions/rounds, bounded p2p round/referee payloads, node pending-queue application,
  input-rooted openings, chain-owned one-op referee verdicts, responder timeout settlement, and
  chain-owned slashing plus delayed challenger reward claims.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Required `tensorvm-verifier` pre-commit step is blocked because the package has no such binary target:
    `error: no bin target named 'tensorvm-verifier' in 'tensor_vm' package`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: commit and push Iteration 167, then choose the next feature-sized slice. Current high-value
  options are trace-bisection session-open/runtime challenge generation, multi-round DoS policy, deployed
  full VRF lifecycle evidence, public/CUDA deployment evidence, or remaining exact Tier-B/CUDA conformance.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 167 first and final `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, trace-bisection rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof, and trace-bisection p2p/chain/referee/timeout admission with slashing/delayed challenger settlement | Add runtime challenge generation, session-open gossip, or deployed public/CUDA proof |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, role-owned graph execution, `const_blob`, input-rooted p2p trace openings, signed trace-bisection core, bounded round/referee p2p payloads, state-rooted sessions/rounds, chain-owned referee/timeout verdicts and economic settlement | Continue session-open/runtime generation and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments; local runtimes ingest deterministic fixtures, verified drand, public chained drand, chain-owned epoch windows, registered validator reveal keys, and keyed Ed25519 reveal proofs before reward release | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, block-check/trace-bisection challenger delayed bounties, pending claim view, study helper, calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 167: Explicit Challenge Reward Claim Boundary

Feature capability: successful block-check and trace-bisection challenger bounties remain state-rooted
pending claims until their maturity height, then become account-spendable only through beneficiary
`ClaimReward`. Mature reward sweep/block-transition helpers may prune voided challenge claims, but they
must not credit active challenger rewards as a workaround.

Readiness requirements covered: `upow.md` §12.1 pending challenger rewards before spendability,
`mvp_spec.md` §4.6 canonical transition ownership for reward allocation, and the user-requested direction
to add delayed rewards directly instead of working around immediate credit paths.

Canonical owner: `chain::commands` owns claim-time release into the reward ledger and account claim;
`chain::challenges` owns creation of pending challenge reward claims; block transitions only call canonical
pruning. Node/runtime/p2p/checkers must not materialize challenge rewards.

Regression evidence: `block_transition_preserves_matured_challenge_rewards_until_claim` creates a mature
non-voided pending challenge reward, proves `ReleaseMaturedChallengeRewards` emits no credit events and
does not remove the active claim, submits/finalizes a block while the claim stays pending and the
challenger reward balance stays zero, then proves `ChainCommand::ClaimReward` releases the claim and moves
the amount to the challenger account.

Behavior boundaries: unchanged with local synthetic production disabled; unchanged for producer and
non-producer roles; no wire-size or codec changes; finality remains normal block append/vote/finality.

Out of scope: session-open gossip, runtime automatic challenge generation, multi-round DoS policy,
public/CUDA evidence, and new p2p payloads.

Validation passed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release`.
- Focused: `cargo test -p tensor_vm challenge_reward --lib -- --nocapture`.
- Focused: `cargo test -p tensor_vm reward --lib -- --nocapture`.
- Broad: `cargo fmt --check --all`.
- Broad: `cargo check -p tensor_vm --tests`.
- Broad: `git diff --check`.
- Broad: `cargo test -p tensor_vm --lib` (527 passed).
- Final gate: `cargo test -p tensor_vm local_testnet --release`.

Verifier command blocked on June 22, 2026:
`cargo run -p tensor_vm --bin tensorvm-verifier -- --help` returned
`error: no bin target named 'tensorvm-verifier' in 'tensor_vm' package`.

Coverage command remained environmentally blocked on June 22, 2026:
`cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.

Commit: pending.

## Recent Iterations

### Iteration 166: Trace-Bisection Timeout Slashing And Delayed Challenger Rewards

Commit: `bfcefa7` (pushed `main` -> `main`).

Feature capability: a chain-owned trace-bisection timeout settles the economic side without adapter
workarounds. When the responder/miner forfeits by timeout, the chain slashes the responder bond, credits
treasury with the net slash, creates a delayed `PendingChallengeReward` claim for the challenger, keeps
spendability behind beneficiary `ClaimReward`, voids affected receipt rewards, and removes affected
TensorWork from pending/settled blockspace as appropriate.

Validation passed on June 22, 2026: first-command Gate 0; focused `records_timeout`, `trace_bisection`,
`reward`, and `chain::tests::challenges`; broad `cargo fmt --check --all`, `cargo check -p tensor_vm
--tests`, `git diff --check`, `cargo test -p tensor_vm --lib` (526 passed); final Gate 0. Verifier and
tarpaulin remained blocked by missing targets/tools.

### Iteration 165: Trace-Bisection Referee Slashing And Delayed Challenger Rewards

Commit: `e3af101` (pushed `main` -> `main`).

Feature capability: a chain-owned one-op trace-bisection referee verdict settles slashing and delayed
challenger bounty claims without adapter workarounds. Losing registered miner/validator stake is slashed,
treasury receives the net slash, a winning challenger receives a delayed `PendingChallengeReward` when the
responder/miner loses, and spendability remains behind `ClaimReward`.

Validation passed on June 22, 2026: first-command Gate 0; focused `trace_bisection_referee`, `reward`,
`chain::tests::challenges`, `pending_payloads`; broad fmt/check/diff/lib (526 passed); final Gate 0.
Tarpaulin remained blocked because `cargo-tarpaulin` is not installed.

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
  transcript disputes, session-open gossip, and multi-round DoS policy remain deployment or future-feature
  gates, not local-completion claims.

## Validation Evidence

- Iteration 167 first executable command passed before edits on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 167 focused validation passed on June 22, 2026:
  `cargo test -p tensor_vm challenge_reward --lib -- --nocapture`;
  `cargo test -p tensor_vm reward --lib -- --nocapture`.
- Iteration 167 broad validation passed on June 22, 2026:
  `cargo fmt --check --all`; `cargo check -p tensor_vm --tests`; `git diff --check`;
  `cargo test -p tensor_vm --lib` (527 passed); final
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 167 verifier command blocked on June 22, 2026:
  `cargo run -p tensor_vm --bin tensorvm-verifier -- --help` returned
  `error: no bin target named 'tensorvm-verifier' in 'tensor_vm' package`.
- Iteration 167 coverage command remained environmentally blocked on June 22, 2026:
  `cargo tarpaulin --workspace --offline` returned `error: no such command: tarpaulin`.
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
