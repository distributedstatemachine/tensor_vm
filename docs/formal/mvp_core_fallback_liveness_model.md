# TensorVM MVP Core Fallback Liveness Model

Status: documentation-only fallback proof model for the reviewed v2 MVP core.

Purpose: specify the zero-receipt and no-PoW path without letting fallback blocks masquerade as
useful-verification PoW. This document narrows `TVM-FALLBACK-001` / `V2-FALLBACK-001`; it does not
discharge implementation, networking, or mechanized-proof obligations.

Related documents:

- [`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md) defines the normal
  useful-PoW path and says empty or below-floor selected sets must use explicit fallback.
- [`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md) defines the
  parent-state validation and child-root transition that fallback blocks must also import.
- [`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md) defines
  the selected-receipt pool whose empty or below-floor state can trigger fallback.
- [`mvp_core_v2_consensus_proof_obligations.md`](mvp_core_v2_consensus_proof_obligations.md) tracks the
  blocked v2 theorem spine.
- [`bad_assumptions_ledger.md`](bad_assumptions_ledger.md) records the wording guardrails this model adds.

## Current Verdict

Fallback is specified as a paper model, and the local reference now has a partial implementation, but the
formal liveness proof remains implementation-blocked.

The reviewed MVP needs a v2 fallback object with timeout evidence, deterministic validator rotation,
reduced rewards, no miner TensorWork rewards, and parent-state validation. The current worktree has local
`PowSkipFallback` timeout/rotation/reduced-reward tests and local CPU evidence for the single configured
producer, but not the full multi-validator/public liveness evidence or mechanized proof.

The safe claim today is:

```text
Zero-receipt and no-PoW liveness have a documented v2 proof obligation.
They are partially implemented in the local reference but not fully proved for the current consensus core.
```

The unsafe claim is:

```text
Empty blocks, v1 TensorWork fallback, or timeout blocks are useful-verification PoW.
```

## Block Kinds

The v2 theorem must distinguish two block kinds.

| Block Kind | Trigger | Required Evidence | Reward Shape | May Claim Useful-PoW |
| --- | --- | --- | --- | --- |
| `UsefulPowBlock` | Eligible selected receipts meet the nonfallback work floor and a valid nonce is found. | Parent-state selected root, checks root, beacon, target, nonce, proposer eligibility, state/reward roots. | Normal v2 proposer and receipt/reward rules. | Yes, subject to useful-work economic assumptions. |
| `FallbackBlock` | No valid useful-PoW block appears before `pow_timeout_blocks`, or parent-state selected receipts are empty/below floor. | Timeout/no-work evidence, deterministic fallback proposer rotation, parent-state transition roots, reduced reward certificate. | Reduced proposer reward only; no miner TWU rewards for empty blockspace. | No. |

This split is mandatory. A fallback block is a liveness mechanism, not a proof that verification work was
performed.

## State And Evidence Objects

The future v2 state needs these objects before the fallback theorem can be discharged.

| Object | Required Fields | Proof Role |
| --- | --- | --- |
| `FallbackParams` | `pow_timeout_blocks`, reduced reward, rotation seed domain, synchrony parameter version. | Makes timeout and reward rules consensus-visible. |
| `FallbackClock` | parent height, parent time/slot, first eligible useful-PoW deadline, observed timeout window. | Prevents arbitrary immediate fallback. |
| `NoUsefulPowEvidence` | parent hash, selected-root status, work-floor status, timeout observations, best known useful-PoW candidate hash if any. | Separates empty/below-floor liveness from censorship of available useful-PoW. |
| `FallbackProposerSet` | validator registry snapshot, stake weights, active/jailed status, rotation seed. | Defines who may produce the fallback block. |
| `FallbackBlockHeader` | parent hash, proposer, fallback flag, timeout evidence root, selected-root status, state root, reward root, params version. | Binds finality to the fallback path and prevents normal PoW claims. |
| `FallbackCertificate` | validation result for the exact parent state, proposer eligibility proof, timeout/no-work proof, finality votes. | Lets finality import fallback validity without importing useful-PoW. |

The timeout evidence can start as a deterministic local rule under synchrony assumptions, but the theorem
must name that assumption. A future public network claim needs signed observations from enough peers; local
timeouts alone do not prove global absence of useful-PoW.

## Validity Predicate

A fallback block is valid for parent state `S` only if every predicate below holds.

```text
valid_fallback_block(S, B) :=
  B.kind == FallbackBlock
  && parent_hash_valid(S, B.parent_hash)
  && fallback_params_valid(S.params, B.params_version)
  && fallback_trigger_valid(S, B.timeout_evidence)
  && fallback_proposer_valid(S.validator_set, B.proposer, B.timeout_evidence)
  && B.selected_receipt_set_root == fallback_selected_root(S, B.timeout_evidence)
  && no_useful_pow_claim(B)
  && fallback_state_transition_valid(S, B)
  && fallback_reward_transition_valid(S, B)
  && fallback_roots_match(apply_fallback_block(S, B), B.state_root, B.reward_root)
```

`fallback_trigger_valid` has two allowed cases:

1. `empty_or_below_floor`: the canonical selected set for `S` is empty or does not meet the nonfallback
   work floor.
2. `timeout`: a valid useful-PoW block for the same parent has not been observed before the consensus
   timeout under the declared synchrony model.

The second case is assumption-heavy. It must not be used to prove censorship resistance unless timeout
evidence is public, independently observed, and tied to a network synchrony model.

## Theorem Split

### FB-001: Fallback Trigger Soundness

Statement:

```text
If valid_fallback_block(S, B), then B was admitted only after either canonical blockspace was empty/below
the nonfallback work floor or the useful-PoW timeout predicate was satisfied for parent S.
```

Proof dependencies:

- canonical selected-receipt blockspace
- nonfallback work floor
- timeout/synchrony assumption
- timeout evidence root binding

Current status: local implementation partial; formal proof still `implementation-blocked`.

### FB-002: Deterministic Fallback Proposer Rotation

Statement:

```text
For a fixed parent state, fallback params, and validator registry snapshot, every honest node derives the
same eligible fallback proposer order and accepts only the next eligible proposer for the timeout slot.
```

Proof dependencies:

- validator registry snapshot
- stake weighting rule
- active/jailed validator state
- rotation seed domain separation
- tie-breaking and zero-stake exclusion

Current status: local implementation partial; formal proof still `implementation-blocked`.

### FB-003: Fallback Imports Parent-State Validation

Statement:

```text
If finality imports a fallback certificate for B, then B passed valid_fallback_block(parent_state(B), B)
before any fallback votes counted.
```

Proof dependencies:

- parent-state lookup
- atomic fallback apply transition
- finality certificate model
- vote admission gate for fallback certificates

Current status: local implementation partial; formal proof still `implementation-blocked`.

### FB-004: Fallback Rewards Are Reduced And Miner-Free

Statement:

```text
Applying a valid fallback block awards at most the configured reduced proposer reward and awards no miner
TensorWork or receipt-inclusion rewards for empty/below-floor blockspace.
```

Proof dependencies:

- reward state transition
- fallback parameter version
- selected-receipt status
- no miner reward mutation outside selected receipt settlement

Current status: local implementation partial; formal proof still `implementation-blocked`.

### FB-005: Fallback Does Not Claim Useful Work

Statement:

```text
No valid fallback block can satisfy the normal UsefulPowBlock predicate, and no public wording may count it
as useful-verification PoW.
```

Proof dependencies:

- disjoint block-kind tag
- normal useful-PoW predicate
- no-useful-PoW flag/root binding
- claim wording rules in the bad-assumptions ledger

Current status: local implementation partial; formal proof still `implementation-blocked` for public
enforcement.

### FB-006: Fallback Preserves Safety

Statement:

```text
If a fallback block is valid for parent state S, applying it produces the unique child state allowed by
apply_fallback_block(S, B), and finality for B conflicts with any other child only under the same quorum
safety assumptions used by normal v2 finality.
```

Proof dependencies:

- deterministic parent-state transition
- finality quorum theorem
- equivocation handling
- same-parent conflict rule

Current status: local implementation partial; formal proof still `implementation-blocked`.

### FB-007: Timeout Liveness Under Partial Synchrony

Statement:

```text
Under the declared partial-synchrony bound and with enough honest online validator stake, if no valid
UsefulPowBlock becomes available for parent S, some valid FallbackBlock can be proposed and finalized.
```

Proof dependencies:

- synchrony/timeout bound
- honest online validator-stake lower bound
- deterministic proposer rotation
- message delivery and finality liveness assumptions
- reduced reward rule that does not incentivize permanent fallback over useful-PoW

Current status: local implementation partial; formal proof remains `assumption-bound` and
`implementation-blocked`.

## Anti-Abuse Rules

The fallback path creates new attack surfaces. The proof must reject these cases explicitly.

| Abuse Case | Required Rejection |
| --- | --- |
| A validator skips useful-PoW and immediately emits fallback. | Reject unless `fallback_trigger_valid` holds for the exact parent state. |
| A proposer censors an available useful-PoW block and claims timeout. | Require timeout evidence and define the synchrony/public-observation assumption. |
| A fallback block receives normal useful-PoW rewards. | Reject through `fallback_reward_transition_valid`. |
| Empty selected receipts are mined as normal useful-PoW. | Reject through `nonfallback_work_floor`; use fallback instead. |
| V1 TensorWork proposer fallback is reused as v2 fallback. | Reject because v2 fallback proposer rotation is over validator state, not TensorWork. |
| Finality votes count before fallback validation. | Reject unless vote admission imports `valid_fallback_block(parent_state, B)`. |
| Fallback resets challenge windows or deletes carry-over receipts. | Reject through parent-state apply rules and settled-receipt lifecycle invariants. |

## Required Tests Before Upgrade

The proof status cannot move beyond paper-specified until tests cover at least:

1. Empty selected receipts produce only a fallback-eligible block, not a normal useful-PoW block.
2. Below-floor selected receipts are rejected on the normal path and accepted only through fallback rules.
3. Fallback before timeout is rejected.
4. Fallback with the wrong validator rotation proposer is rejected.
5. Fallback with normal proposer reward or miner TWU rewards is rejected.
6. Fallback vote admission rejects blocks not validated against their exact parent state.
7. Useful-PoW arriving before timeout prevents fallback for the same parent.
8. Equivocating fallback proposals for the same parent cannot both finalize without violating the stated
   quorum assumption.
9. Timeout evidence root changes alter the block hash and validation result.
10. Build-clean v2 block code is not counted as fallback evidence unless it implements the fallback path.

## Bad Assumptions Added

This model adds or sharpens these bad assumptions:

- "A zero-receipt block is still useful-verification PoW."
- "The v1 TensorWork fallback is close enough to the v2 fallback."
- "Timeout evidence is unnecessary; a proposer can just say no useful-PoW arrived."
- "Fallback can pay the same rewards as useful-PoW without distorting incentives."
- "Fallback liveness proves public availability or censorship resistance."
- "Fallback finality can skip parent-state validation because it is only a liveness path."

Correct framing:

```text
Fallback is an explicitly reduced, parent-state-validated liveness transition.
It preserves chain progress when useful-PoW is unavailable, but it does not prove useful work.
```

## Discharge Gate

Do not classify `TVM-FALLBACK-001` / `V2-FALLBACK-001` as locally proof-ready until all of these are true:

1. The block type has a disjoint fallback kind or flag.
2. Parent-state validation exposes `valid_fallback_block(parent_state, block)`.
3. Timeout/no-work evidence is consensus-visible and hash-bound.
4. Fallback proposer rotation is deterministic from parent validator state.
5. Reward transition enforces reduced proposer reward and no miner TWU rewards for empty/below-floor
   blockspace.
6. Finality votes for fallback blocks are counted only after fallback validation.
7. Normal useful-PoW blocks reject empty or below-floor selected sets.
8. Tests cover early fallback, wrong proposer, wrong reward, wrong parent state, and pre-timeout useful-PoW.
9. The theorem states synchrony, honest-online-stake, and public-observation assumptions explicitly.
10. Public docs say fallback is liveness-only and never useful-verification PoW.
