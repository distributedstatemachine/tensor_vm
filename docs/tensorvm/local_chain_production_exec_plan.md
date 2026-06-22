# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 173 complete: Reward Claim Boundary Regression Hardening.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Valid matured claims remain non-spendable until the beneficiary calls `ClaimReward`;
  direct maturity-sweep commands can prune only voided/unavailable matured claims and cannot credit live
  beneficiary balances. Trace-bisection has signed sessions/rounds, bounded p2p open/expectation/round/
  referee payloads, pending-queue application, runtime session-open, challenger expectation, responder
  round generation, one-op referee verdicts, timeout settlement, slashing, and delayed challenger rewards.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: continue trace-bisection DoS policy, automatic referee witness generation, or public/CUDA
  deployment evidence.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 173 first command and pre-commit rerun of `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection expectations/rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof or remaining fraud-proof DoS policy |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, packed int8 APIs, const blobs, role-owned graph execution, and trace-bisection state | Continue CUDA graph evidence and fraud-proof DoS/referee automation |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 173: Reward Claim Boundary Regression Hardening

Feature capability: matured reward sweeps cannot be used as a workaround for verifier-dependent reward
finality. Proposer, receipt, challenge, and credit rewards remain pending through maturity and become
spendable only through the beneficiary-owned `ClaimReward` command; direct release/sweep commands prune
only voided or unavailable matured claims and never credit live beneficiary balances.

Readiness requirements covered: `upow.md` §12.1 delayed verifier-dependent rewards, `mvp_spec.md` §20.3
receipt reward finality, §20.7/§25.5 challenge/proposer reward finality, and the readiness requirement
that live pending reward claims mature into spendable balances only through `ClaimReward`.

Canonical owner: `chain::commands` remains the only owner of pending reward claim release/pruning and
beneficiary claim spendability. Runtime, p2p, status, and checkers only observe or request chain commands.

Old shortcut being removed: any maturity-sweep command that converts a valid pending claim into a
spendable balance without the beneficiary explicitly claiming it.

Regression test that proves the shortcut is gone: `reward_release_commands_preserve_live_matured_claims_
until_beneficiary_claim` seeds mature valid proposer, receipt, challenge, and credit claims, invokes every
direct `ReleaseMatured*` command, proves no spendable reward/account balance changes and live claims remain
pending, then proves `ClaimReward` is the only command that releases them.

Behavior with local synthetic block production disabled: reward claimability depends only on chain state,
claim maturity, and beneficiary command submission, not on local production mode.

Behavior for producer and non-producer roles: producers and non-producers apply the same chain command
boundary; neither role can gain spendable rewards from a sweep-only command.

Structured evidence source: focused `chain::tests::rewards` assertions over pending reward ledgers,
`RewardState`, account balances, and emitted `ChainEvent`s.

Finality source: unchanged block append/vote/finality; reward finality is the pending-claim maturity plus
explicit beneficiary claim boundary.

Wire-size and codec boundary: no network wire change.

Parallel subagents to run: skipped; available subagent tooling requires explicit user authorization.
Read-only discovery was parallelized with local shell tools.

Tests/checkers/docs updated: `crates/tensor_vm/src/chain/tests/rewards.rs` and this execution plan.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib -- --nocapture` passed.
- `cargo test -p tensor_vm rewards --lib -- --nocapture` passed: 26 tests.
- `cargo fmt --all -- --check` passed after applying `cargo fmt --all`.
- `cargo check --all-targets` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 537 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 84.52% line coverage.
- Manual verifier-style review: production reward commands already used pending claims; the new test locks
  the critical boundary that direct `ReleaseMatured*` commands cannot credit live matured claims, while
  `ClaimReward` remains the only spendability path.
- Feature commit `919f77f` pushed to `main` on June 22, 2026:
  `git push` returned `8b94508..919f77f  main -> main`.

Out of scope: changing reward allocation formulas, adding new reward ledgers, trace-bisection DoS policy,
automatic referee witness generation, and public/CUDA deployment evidence.

## Recent Iterations

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
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, automatic referee witness
  generation, and multi-round DoS policy remain deployment or future-feature gates, not local-completion
  claims.

## Validation Evidence

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
