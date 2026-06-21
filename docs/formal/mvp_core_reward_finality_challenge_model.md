# TensorVM MVP Core Reward Finality And Challenge Model

Status: documentation-only reward-finality proof model for the reviewed v2 MVP core.

Purpose: separate block finality from reward finality. A finalized block can be safe as an ordering object
while its proposer, miner, or validator rewards remain pending until verifier evidence is directly
recomputed or the challenge window closes without a valid dispute.

This document narrows the missing reward and challenge-window side of `V2-STATE-001` and the verifier
evidence bridge. It does not discharge implementation, public data-availability, slashing, or mechanized
proof obligations.

Related documents:

- [`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md) defines the recomputable or
  challengeable evidence required before signed statements become semantic verifier evidence.
- [`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md) defines the
  parent-state transition and child `reward_root` theorem this model refines.
- [`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md) defines
  selected receipt lifecycle and carry-over state.
- [`mvp_core_data_availability_boundary.md`](mvp_core_data_availability_boundary.md) separates
  verification-time retrieval from availability through a challenge window.
- [`mvp_core_fallback_liveness_model.md`](mvp_core_fallback_liveness_model.md) defines the reduced-reward
  fallback path.

## Current Verdict

Reward finality is paper-specified here, and the local reference now has partial delayed-reward
implementation, but the full formal theorem remains implementation-blocked.

The safe claim today is:

```text
The docs define a target delayed reward-finality and challenge model.
Current consensus proof status shows local pending-ledger behavior, but not that finalized blocks imply
settled, irreversible rewards under the full verifier-transcript challenge model.
```

The unsafe claim is:

```text
Once a block is finalized, all proposer, miner, and validator rewards are final.
```

That claim is false for the reviewed v2 design. Block finality and reward finality must be separate states.

## State Machine

Rewards tied to selected receipts or block-level `checks_root` evidence must move through explicit states.

| State | Meaning | Spendable | Exit Conditions |
| --- | --- | --- | --- |
| `NoRewardClaim` | No valid block has created a reward claim. | No | Valid v2 block or fallback block application creates pending claims. |
| `PendingChallenge` | Reward claim exists, but evidence is still challengeable. | No | Challenge window closes, direct recomputation finalizes, or valid challenge opens. |
| `Challenged` | A timely challenge has been accepted for resolution. | No | Challenge resolution accepts or rejects the challenge. |
| `Invalidated` | The claim was disproved or made unpayable by missing evidence. | No | Clawback/nonpayment is recorded; optional reputation or slashing transition applies. |
| `Settled` | Challenge window closed or direct recomputation succeeded with no unresolved challenge. | Yes | Terminal unless an explicit appeal/fraud-proof extension is later specified. |
| `Expired` | Required artifacts or deadlines failed before settlement. | No | Terminal nonpayment unless a separate recovery rule is specified. |

`PendingChallenge` balances must not be usable as spendable balances. They can be represented in a reward
root, but the root must encode their pending status and maturity deadline.

Current local implementation status: receipt, proposer, challenger, validator-audit, and credit rewards use
state-rooted pending claims with explicit maturity or awaiting-inclusion state, mature release, voiding, and
prune-without-credit paths for implemented challenges. This partially discharges delayed reward finality
locally. It does not yet prove full verifier-transcript challenges, deployed public dispute propagation, or
complete full-v0 economic security.

## Required Objects

The future v2 state needs these objects before reward finality can be proved.

| Object | Required Fields | Proof Role |
| --- | --- | --- |
| `RewardParams` | challenge window, settlement delay, proposer/miner/validator split, fallback reward, challenger reward, parameter version. | Makes reward maturity and amounts consensus-visible. |
| `RewardClaim` | claim id, block hash, parent hash, selected receipt id or fallback marker, participant id, role, amount, evidence root, maturity height/time. | Binds a pending payout to the exact evidence and block transition that created it. |
| `ChallengeWindowState` | open/close height or time, direct-recompute status, unresolved challenge set, DA retention requirement. | Separates finalized ordering from reward maturity. |
| `CheckChallenge` | challenged block, selected receipt index, claimed check leaf, opening path, recomputed transcript, challenger signature, submitted height/time. | Lets any node validate whether `checks_root` evidence is false. |
| `ChallengeResolution` | accepted/rejected result, reason code, affected claims, challenger reward, clawback target. | Makes challenge outcomes deterministic state transitions. |
| `RewardSettlementRecord` | settled claim id, participant, amount, evidence root, settlement height/time, final status. | Provides auditable evidence for public reward-settlement records. |
| `ClawbackRecord` | invalidated claim id, amount, source pending balance, destination or burn rule, reason. | Prevents invalid evidence from being paid after a successful challenge. |

Hard slashing can be added later, but v0 proof obligations can be satisfied by deterministic nonpayment,
clawback from pending rewards, challenger reward, and reputation/throttle state. If hard slashing is
claimed, the slashing object and appeal rules become part of the theorem.

## Transition Predicates

### Pending Reward Creation

Valid block application can create pending reward claims:

```text
create_pending_rewards(S, B) = S_pending
```

Allowed only if:

```text
valid_v2_block(parent_state, B)
&& selected_receipts_or_fallback_valid(parent_state, B)
&& checks_evidence_root_bound(parent_state, B)
&& reward_amounts_from_params(parent_state.reward_params, B)
&& pending_claims_not_spendable(S_pending)
```

Normal useful-PoW blocks may create pending proposer, miner, and validator claims. Fallback blocks may
create only the reduced fallback proposer claim defined by the fallback model.

### Challenge Admission

A challenge is admitted only if:

```text
valid_challenge(S, C) :=
  challenge_in_window(S.challenge_window(C.block_hash), C)
  && challenge_targets_existing_pending_claim(S, C)
  && opening_matches_checks_root(C)
  && transcript_recomputes_from_committed_artifacts(C)
  && challenger_signature_valid(C)
```

Late challenges, challenges against settled claims, challenges without artifact openings, and challenges
whose recomputation does not contradict the claimed leaf must be rejected.

### Challenge Resolution

Challenge resolution must be deterministic:

```text
resolve_challenge(S, C) = S'
```

If the challenge is accepted:

- affected pending rewards move to `Invalidated`;
- the affected receipt is removed from reward settlement until reverified or requeued by rule;
- challenger reward is created or settled according to `RewardParams`;
- optional reputation/throttle/slashing state is updated by a named transition;
- `reward_root(S')` reflects the invalidation and clawback.

If the challenge is rejected:

- the challenged pending reward remains pending until the window closes;
- any challenger bond or spam penalty must follow a named rule if one exists;
- no unrelated reward state changes.

### Reward Settlement

Rewards settle only when:

```text
settle_reward_claim(S, claim) = S'
```

and:

```text
claim.status == PendingChallenge
&& challenge_window_closed_or_direct_recomputed(S, claim)
&& no_unresolved_challenge(S, claim)
&& evidence_still_available_through_required_window(S, claim)
&& claim_amount_matches_reward_params(S, claim)
&& no_double_settlement(S, claim)
```

Settlement changes pending balances into spendable balances and records a `RewardSettlementRecord`.

## Theorem Split

### RW-001: Block Finality Does Not Imply Reward Finality

Statement:

```text
If finalized_v2_block(B), then B may create pending reward claims, but those claims are not spendable until
the reward-finality predicate settles them.
```

Dependencies:

- parent-state block validity
- reward claim status encoding
- spendable vs pending balance separation

Current status: local implementation partial; formal proof still `implementation-blocked`.

### RW-002: Pending Rewards Are Root-Bound But Not Spendable

Statement:

```text
If a pending reward claim appears in reward_root(S), then its amount, participant, role, evidence root,
maturity deadline, and status are encoded; no spendable balance includes it before settlement.
```

Dependencies:

- reward claim canonical encoding
- reward root binding
- balance accounting invariant

Current status: local implementation partial; formal proof still `implementation-blocked`.

### RW-003: Valid Challenge Invalidates Only Targeted Claims

Statement:

```text
Resolving a valid accepted challenge invalidates exactly the reward claims dependent on the disproved
checks_root evidence and preserves unrelated claims.
```

Dependencies:

- challenge opening validation
- dependency map from check leaves to reward claims
- frame theorem for unrelated rewards

Current status: local implementation partial; formal proof still `implementation-blocked`.

### RW-004: Challenge Absence Is Conditional Evidence

Statement:

```text
If a reward claim settles only because no challenge was accepted, the soundness claim depends on DA through
the challenge window, challenger availability, timeout rules, and verifier false-accept bounds.
```

Dependencies:

- DA retention model
- honest challenger or direct recomputation assumption
- verifier evidence model
- probabilistic soundness budget

Current status: `assumption-bound` and implementation-blocked.

### RW-005: Reward Settlement Is Deterministic And Single-Use

Statement:

```text
For a fixed parent reward state and valid settlement event, every honest node derives the same child reward
state, and no claim can settle twice.
```

Dependencies:

- reward claim id uniqueness
- deterministic settlement ordering
- no double-settlement invariant
- reward root canonical encoding

Current status: local implementation partial; formal proof still `implementation-blocked`.

### RW-006: Clawback Precedes Spendability

Statement:

```text
If verifier evidence for a pending claim is disproved during the challenge window, the invalidated amount is
clawed back or never made spendable before any participant can spend it.
```

Dependencies:

- pending/spendable separation
- atomic challenge resolution
- no partial settlement mutation

Current status: local implementation partial; formal proof still `implementation-blocked`.

## Required Tests Before Upgrade

The proof status cannot move beyond paper-specified until tests cover at least:

1. Finalized block creates pending reward claims, not spendable balances.
2. Pending claims encode block hash, selected receipt id, participant, role, amount, evidence root, and
   maturity deadline.
3. Settlement before the challenge window closes is rejected unless direct recomputation is recorded.
4. A valid challenge against a selected check leaf invalidates dependent pending rewards.
5. A valid challenge does not mutate unrelated reward claims.
6. A late challenge against a settled claim is rejected by the base v0 rules.
7. A malformed opening or noncontradictory recomputation is rejected.
8. Missing challenge-window DA prevents unconditional reward-finality claims.
9. Fallback blocks create only reduced fallback proposer rewards and no miner TWU rewards.
10. Double settlement of the same claim is rejected.
11. Failed challenge admission and failed settlement leave reward state unchanged.
12. `reward_root` changes for pending, challenged, invalidated, and settled states are deterministic.

## Bad Assumptions Added

This model adds or sharpens these bad assumptions:

- "Block finality means reward finality."
- "A `reward_root` is safe if it includes already-spendable rewards for unchallengeable verifier evidence."
- "Challenge absence proves correctness without DA and honest challenger assumptions."
- "A challenge helper is equivalent to a consensus challenge window."
- "Clawback can be added later without changing the reward theorem."
- "Fallback proposer rewards can reuse normal useful-PoW reward settlement."
- "Signed reward-settlement records prove the underlying verifier evidence was sound."

Correct framing:

```text
Reward finality is a separate delayed state transition.
Until direct recomputation or challenge-window finality, verifier-dependent rewards are pending claims.
```

## Discharge Gate

Do not classify reward finality as locally proof-ready until all of these are true:

1. Reward state distinguishes pending, challenged, invalidated, settled, expired, and spendable balances.
2. `reward_root` encodes claim status, participant, amount, evidence dependency, and maturity deadline.
3. Block application creates pending claims instead of immediate spendable verifier-dependent rewards.
4. Challenge openings are consensus-valid events tied to block `checks_root` and selected receipts.
5. Challenge resolution deterministically invalidates targeted claims and preserves unrelated claims.
6. Settlement requires closed challenge window or direct recomputation, no unresolved challenge, and no double
   settlement.
7. DA assumptions cover every artifact needed to challenge through the full window.
8. Fallback reward settlement follows the reduced-reward fallback model.
9. Public reward-settlement evidence is tied to settled claims, not merely signed summaries.
10. The theorem states whether soundness relies on honest challengers, direct recomputation, or proof-carrying
    evidence.
