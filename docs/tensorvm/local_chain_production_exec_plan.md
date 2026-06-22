# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 190 complete: Proposer Reward Delay Tombstone.
- Current status: finalized proposer rewards are being moved away from a height-cutoff rematerialization
  workaround. The chain now records released proposer reward block heights in state, reward roots, state
  roots, and chain-state snapshots so late-finalized proposer rewards still materialize as delayed claims
  and already claimed/voided proposer rewards cannot reappear.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
  - Deployed full VRF construction, deployed commit-reveal lifecycle evidence, and public/CUDA graph
    execution evidence remain open.
- Next action: commit and push Iteration 190, then continue CUDA/public deployment evidence or remaining
  deployed-randomness/economic evidence.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Current iteration first command `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection messages, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, delayed pending reward holds, and state-rooted proposer reward release tombstones | Continue Tier-C committee policy and deployed public-operator evidence |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 190: Proposer Reward Delay Tombstone

Feature capability: replace the finalized proposer reward height-cutoff workaround with explicit,
state-rooted released proposer reward block tracking.

Readiness requirements covered: `goal.md`/`upow.md` delayed reward maturity, claim-owned spendability, and
avoiding adapter/workaround reward release paths.

Canonical owner: `chain::blocks` materializes finalized proposer rewards; `chain::commands` releases
claims through beneficiary `ClaimReward`; `ChainState` owns pending and released proposer reward state.

Adapter callers: runtime/status/RPC/explorer surfaces consume chain state and must not infer proposer
reward finality from block height cutoffs.

Old shortcut being removed: `materialize_finalized_proposer_rewards` skipped creating a pending proposer
reward when `state.height > claimable_at_height`, which prevented rematerialization after claim but also
made late-finalized rewards disappear instead of becoming delayed claims.

Regression tests that prove the shortcut is gone:
`late_finalized_proposer_reward_materializes_as_delayed_claim_once`, proposer-reward focused tests, and the
chain-state snapshot roundtrip.

Behavior with local synthetic block production disabled: unchanged; finalized rewarded blocks use the same
chain-owned pending/released proposer reward ledgers.

Behavior for producer and non-producer roles: producer and peers recompute the same reward/state roots
because released proposer reward block heights are included in state snapshots and roots.

Structured evidence source: `pending_proposer_rewards`, `released_proposer_reward_blocks`, `reward_root`,
`state_root`, and chain-state snapshot roundtrip.

Finality source: unchanged; proposer rewards materialize after block finality, then become spendable only
through beneficiary `ClaimReward`.

Wire-size and codec boundary: no p2p/RPC wire format changes; chain-state snapshot encoding changes.

Parallel subagents to run: none. The available subagent tool policy requires explicit user delegation; this
slice is confined to chain reward state, tests, and docs/status alignment.

Narrow validation commands:
`cargo test -p tensor_vm late_finalized_proposer_reward_materializes_as_delayed_claim_once --lib`,
`cargo test -p tensor_vm proposer_reward --lib`,
`cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib`,
`cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`,
and `cargo test -p tensor_vm block_transition_preserves_matured_rewards_until_claim --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, workspace
release tests, clippy, and tarpaulin because reward tests and reportable coverage change.

Expected observable evidence: a rewarded block finalized after its claimable height still creates a
pending proposer reward claim, `ClaimReward` releases it, and later materialization does not recreate that
block's claim.

Validation evidence:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Narrow evidence passed so far:
  `cargo test -p tensor_vm late_finalized_proposer_reward_materializes_as_delayed_claim_once --lib`,
  `cargo test -p tensor_vm proposer_reward --lib`,
  `cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib`,
  `cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`,
  and `cargo test -p tensor_vm block_transition_preserves_matured_rewards_until_claim --lib`.
- Broad gates passed:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  567 instrumented tests and 84.80% line coverage, 23142/27291 lines covered.

## Recent Completed Iterations

- Iteration 189: Public Evidence Raw Chain History Record Gate. Commit `8f84062`
  (`Require raw public chain history evidence`) pushed to `origin/main` on June 22, 2026; metadata commit
  `581f87a` recorded/pushed the evidence anchor.
- Iteration 188: Public Evidence Raw Operational Record Gate. Commit `e4c599f`
  (`Require raw public operational evidence`) pushed to `origin/main` on June 22, 2026.
- Iteration 187: Chain-Owned Verifier Bandwidth Evidence. Commit history before Iteration 188 contains the
  detailed anchor; local verifier-bandwidth evidence is implemented and documented.
- Iteration 186: Public Randomness Evidence Raw-Record Gate.
- Iteration 185: Mixed-Dtype Conformance Vector Coverage.
- Iteration 184: Trace-Bisection DoS Admission Bounds.
- Iteration 183: Isolated Trace-Bisection Timeout Policy.
- Iteration 182: Reward Sweep Boundary Naming.

## Decision Log

- `tensorvm-verifier` is not a repository binary. Validation uses tests, clippy, tarpaulin, and manual
  verifier-style review only.
- Do not spawn subagents unless the user explicitly asks for delegation.
- Public/CUDA/deployed evidence remains blocked until real external infrastructure and CUDA kernels exist.
- Reward claims remain delayed and chain-owned; valid matured claims become spendable only through
  beneficiary `ClaimReward`, while voided/prunable claims may be swept without credit.

## Validation Evidence

- Current Iteration 190 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 190 focused validation passed on June 22, 2026:
  `late_finalized_proposer_reward_materializes_as_delayed_claim_once`, `proposer_reward`,
  `chain_state_store_roundtrips_full_chain_and_detects_tampering`,
  `reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim`, and
  `block_transition_preserves_matured_rewards_until_claim`.
- Current Iteration 190 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 190 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  567 instrumented tests and 84.80% line coverage, 23142/27291 lines covered.
- Commit and push evidence will be recorded after push.

## Archive

Older detailed iteration notes were compacted on June 22, 2026 after the plan exceeded 300 lines. Durable
commit anchors and status are preserved above; detailed historical notes remain available in git history.
