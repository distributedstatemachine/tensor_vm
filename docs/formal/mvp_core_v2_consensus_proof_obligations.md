# TensorVM MVP v2 Consensus Proof Obligations

Status: documentation-only theorem spine for the blocked v2 consensus core.

Purpose: define the exact objects and proof obligations that must exist before TensorVM can honestly claim
the reviewed v2 MVP consensus theorem. This document does not implement code and does not mark the theorem
proved. It is the target shape that future implementation and mechanization must satisfy.

The state invariants that these proof obligations must preserve are listed in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).
The canonical encoding and commitment model for selected roots and check roots is defined in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The useful-PoW work model that separates structural header validity from economic work dominance is defined
in [`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
The difficulty retarget model for parent-state target validity is defined in
[`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md).
The verifier evidence model for recomputable or challengeable check leaves is defined in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
The reward-finality model for delayed settlement, challenges, and clawback is defined in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md).
The local v2-block reference path is audited in
[`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md); it does not yet discharge
these obligations.
The parent-state transition model required by V2-STATE and V2-FIN is defined in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).
The settled-receipt blockspace model required by V2-BLK is defined in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).

## Current Verdict

The v2 consensus theorem is still **implementation-blocked**.

Current Rust evidence still shows the active block object is the v1/reference block:

```text
height
parent_hash
epoch
proposer
job_root
receipt_root
attestation_root
state_root
reward_root
randomness
timestamp
proposer_signature
validator_signature_aggregate
```

The reviewed v2 spec requires a block object that can express:

```text
settled_receipt_set_root
checks_root
beacon
difficulty_target
nonce
registered-validator useful-PoW proposer
```

Until those objects exist and finality depends on them, any theorem saying "finality implies
useful-verification PoW validity" is false over the current implementation.

## Target Top-Level Theorem

Target theorem name:

```text
finality_implies_v2_block_valid
```

Target statement:

```text
If block B is finalized as a non-fallback v2 block at parent state S, then:
  1. B.parent_hash is the hash of the accepted parent.
  2. B.proposer is a registered validator eligible at S.
  3. B.settled_receipt_set_root is the root of canonical_selected_receipts(S, B.beacon, caps).
  4. B.checks_root recomputes from verification transcripts for every selected receipt.
  5. H(pow_header(B) || B.nonce) < B.difficulty_target.
  6. B.state_root and B.reward_root are the deterministic result of applying the valid block transition.
  7. Enough unique registered validator stake signed B.hash after B passed v2 validation.
```

This theorem must not be stated over the current v1 block type.

## Required Consensus Objects

| Object | Required Fields | Why It Is Needed | Current Status |
| --- | --- | --- | --- |
| `V2TensorBlock` or equivalent | parent, proposer, selected receipt root, checks root, beacon, target, nonce, state root, reward root, signatures | The proof needs a block object that carries the useful-verification witness. | Missing from current chain state. |
| `SettledReceipt` metadata | receipt id, receipt hash, primitive type, TWU, byte size, miner, settled height, expiry, DA status, spent/included marker | Canonical blockspace cannot be defined from a bare receipt id set. | Missing. |
| `SettledReceiptPool` | deterministic map of eligible settled receipts and carry-over state | Needed to prove inclusion, omission, and spent/carry-over rules. | Missing. |
| `BlockspaceCaps` | TWU cap, byte cap, receipt-count cap | Needed to make deterministic truncation provable. | Missing as consensus object. |
| `CheckLeaf` | receipt id, primitive type, Freivalds transcript root, random-linear root, DA root | Needed to recompute `checks_root`. | Missing as block-level object. |
| `PowHeader` | parent hash, selected receipt root, checks root, beacon, proposer | Needed to bind nonce search to useful verification. | Missing. |
| `DifficultyState` | target, retarget window, hardest/easiest bounds, hash-to-target version, work-floor params | Needed to prove target validity and liveness bounds. | Paper-specified in `mvp_core_difficulty_retarget_model.md`; implementation not started. |
| `BlockVoteV2` or validation rule | vote over valid v2 block hash after block validation | Needed to prove finality counts valid blocks only. | Missing. |
| `RewardFinalityState` | pending claims, challenge windows, challenge resolutions, settled claims, clawbacks | Needed to prove `reward_root` and delayed verifier-dependent settlement. | Local pending receipt, proposer, challenge, audit-related, and credit reward ledgers exist with maturity, voiding, and nonpayment/prune paths for implemented disputes. Full proof remains blocked on verifier-transcript challenges, DA-through-window evidence, public dispute propagation, and theorem discharge. |
| `FallbackBlock` rule | timeout, stake rotation, reduced reward, no miner TWU rewards for empty blockspace | Needed for zero-receipt/no-PoW liveness theorem. | Paper-specified in `mvp_core_fallback_liveness_model.md`; implementation not started. |

## Theorem Spine

### V2-BLK-001: Canonical Settled-Receipt Selection

Statement:

```text
canonical_selected_receipts(S, beacon, caps) is deterministic, contains only settled eligible unspent
receipts, respects TWU/byte/count caps, and leaves nonincluded eligible receipts in carry-over state.
```

Dependencies:

- `SettledReceiptPool`
- eligibility predicate
- expiry predicate
- data-available-through-challenge-window predicate
- deterministic ordering `H(beacon || parent_hash || receipt_id)`
- caps accounting
- spent/carry-over lifecycle
- omission theorem for nonselected eligible receipts

Counterexamples killed:

- Receipt map root as blockspace.
- Validator-selected receipt subset grinding.
- Discretionary censorship by omission from the canonical set.
- Double inclusion of a previously selected settled receipt.

Current status: implementation-blocked.

### V2-BLK-002: Selected Receipt Root Binding

Statement:

```text
B.settled_receipt_set_root = MerkleRoot(receipt_id || receipt_hash || tensor_work_units for every receipt
in canonical_selected_receipts(parent_state, B.beacon, caps)).
```

Dependencies:

- V2-BLK-001
- canonical receipt leaf encoding
- hash collision-resistance assumption
- selected receipt lifecycle fields needed for cap and reward proofs

Counterexamples killed:

- Block says it included one set while validators recompute another.
- Global receipt root substituted for selected settled-receipt root.
- Receipt id root omits eligibility, cap accounting, or reward-relevant selected leaf fields.

Current status: implementation-blocked.

### V2-CHK-001: Check Leaf Recomputability

Statement:

```text
For each selected receipt r, every validating node can recompute check_leaf(r, B.parent_hash, B.beacon)
from the receipt, required artifacts, verifier transcript, and DA evidence.
```

Dependencies:

- TensorOp and LinearTrainingStep verifier kernel.
- receipt artifact availability during validation.
- transcript formats for Freivalds, random-linear checks, and DA checks.
- challenge/opening format for hidden details.
- verifier evidence model linking signed statements to recomputable or challengeable leaves.

Counterexamples killed:

- Per-receipt arbitrary `checks_root`.
- Quorum saying "Valid" without recomputable verifier evidence.
- Aggregate roots over signed `checks_root` claims without transcript truth.

Current status: implementation-blocked.

### V2-CHK-002: Block-Level Checks Root Binding

Statement:

```text
B.checks_root = MerkleRoot(check_leaf_0, check_leaf_1, ...), where check_leaf_i is the recomputed leaf for
the i-th canonical selected receipt.
```

Dependencies:

- V2-BLK-001
- V2-CHK-001
- canonical check leaf order
- hash collision-resistance assumption
- verifier evidence model

Counterexamples killed:

- Proposer mines over a bogus checks root.
- Validators cannot reproduce the verification commitment.
- Block root aggregates signed but false check claims.

Current status: implementation-blocked.

### V2-DIFF-001: Parent-State Difficulty Target Validity

Statement:

```text
For nonfallback v2 block B at parent state S, B.difficulty_target is valid only if it equals
expected_target(S.difficulty_state, B.height) under bounded retarget rules and canonical hash-to-target
semantics.
```

Dependencies:

- difficulty retarget model
- parent-state `DifficultyState`
- versioned `DifficultyParams`
- bounded retarget window
- hardest/easiest target bounds
- hash-to-uint and target encoding vectors
- nonfallback work floor

Counterexamples killed:

- Candidate block chooses an easy target and mines a cheap nonce.
- Static local target is treated as production retarget evidence.
- Nodes disagree on byte ordering or target boundary comparison.

Current status: paper-specified, implementation-blocked.

### V2-POW-001: Useful-Verification PoW Validity

Statement:

```text
valid_useful_pow(B, S) ->
  H(H(B.parent_hash || B.settled_receipt_set_root || B.checks_root || B.beacon || B.proposer) || B.nonce)
    < B.difficulty_target
```

and the header fields in the hash are exactly those validated by V2-BLK-002 and V2-CHK-002.

Dependencies:

- V2-BLK-002
- V2-CHK-002
- registered-validator proposer eligibility
- parent-state difficulty target validity
- nonfallback verification-work floor and useful-work cost model
- hash model

Counterexamples killed:

- Adding a nonce to a v1 block and calling it useful PoW.
- Nonce search independent of verification work.
- Miner TensorWork proposer selection.
- Claiming useful-work dominance from a valid hash target alone.

Current status: implementation-blocked.

### V2-PROP-001: Proposer Eligibility

Statement:

```text
If B is a valid non-fallback v2 block, then B.proposer is a registered validator eligible for the epoch and
won the useful-verification PoW race for the validated header.
```

Dependencies:

- validator registry
- eligibility predicate
- V2-POW-001
- exclusion of `settled_tensor_work` from proposer eligibility

Counterexamples killed:

- `produce_block(address)` with arbitrary address.
- TensorWork-selected miner as block proposer.

Current status: contradicted by current v1/reference block path.

### V2-STATE-001: Valid Block Transition

Statement:

```text
Applying valid v2 block B to parent state S produces deterministic child state S' with:
  B.state_root = state_root(S')
  B.reward_root = reward_root(S')
```

Dependencies:

- selected receipt application rules
- spent/carry-over mutation
- reward allocation after challenge-window semantics
- no double inclusion
- deterministic state root encoding
- exact parent-state validation and child-state apply theorem

Counterexamples killed:

- Finalizing a block whose roots do not match the deterministic transition.
- Paying miner/proposer rewards before required challenge semantics.
- Validating a block against mutable current state instead of its parent state.

Current status: implementation-blocked for v2.

### V2-REWARD-001: Delayed Reward Finality And Challenge Settlement

Statement:

```text
Verifier-dependent rewards created by a valid v2 block are pending until direct recomputation or
challenge-window finality; valid challenges deterministically invalidate dependent claims before they become
spendable.
```

Dependencies:

- reward finality model
- `RewardClaim` status encoding
- challenge opening validation against `checks_root`
- DA-through-challenge-window assumption
- challenger availability or direct recomputation assumption
- deterministic reward root encoding
- clawback/nonpayment transition

Counterexamples killed:

- Paying spendable proposer, miner, or validator rewards before challenge finality.
- Treating signed reward-settlement records as proof that verifier evidence was sound.
- Mutating unrelated balances while resolving a targeted challenge.

Current status: paper-specified, implementation-blocked.

### V2-FIN-001: Vote Admission Requires V2 Validity

Statement:

```text
SubmitBlockVoteV2(vote, B) succeeds -> validate_block_v2(parent_state, B) = true
```

Dependencies:

- `validate_block_v2`
- parent-state lookup for `B.parent_hash`
- vote signature relation
- validator stake registry
- duplicate-vote rejection

Counterexamples killed:

- Stake finality over a known but invalid v1 block hash.
- Finality bypassing blockspace, checks-root, or PoW validation.

Current status: implementation-blocked.

### V2-FIN-002: Finality Implies Valid Block

Statement:

```text
is_finalized_v2(B.hash) -> validate_block_v2(parent_state, B) = true
```

Dependencies:

- V2-FIN-001
- unique validator counting
- stake-threshold theorem
- no direct mutation of finalized set except through validated votes
- finality certificate that records parent-state validation

Counterexamples killed:

- Current `finalized_blocks` containing hashes whose v2 validity was never checked.
- Finalized hashes whose validation was against the wrong state.

Current status: implementation-blocked.

### V2-FALLBACK-001: PoW-Skip Fallback Validity

Statement:

```text
If no valid useful-PoW block appears before pow_timeout_blocks, a fallback block is valid only if it follows
stake-weighted validator rotation, carries reduced proposer reward, and pays no miner TWU rewards for empty
blockspace.
```

Dependencies:

- fallback liveness model
- timeout/no-work evidence rule
- deterministic fallback proposer rotation
- reduced reward rule
- parent-state fallback validation
- telemetry for fallback events

Counterexamples killed:

- Reusing v1 TensorWork fallback as if it were v2 liveness.
- Counting fallback or empty blocks as useful-verification PoW.

Current status: paper-specified, implementation-blocked.

## Validation Predicate Shape

The future validator should expose a single predicate with this shape:

```text
validate_block_v2(parent_state, block) =
  parent_valid(parent_state, block)
  && proposer_eligible(parent_state, block.proposer)
  && selected_receipt_root_valid(parent_state, block)
  && checks_root_valid(parent_state, block)
  && useful_pow_valid(parent_state, block)
  && state_transition_valid(parent_state, block)
  && reward_transition_valid(parent_state, block)
  && reward_finality_state_valid(parent_state, block)
```

Finality must depend on this predicate:

```text
submit_block_vote(vote, block) accepts only if validate_block_v2(parent_state, block)
```

Any path that mutates `finalized_blocks` without this predicate is outside the v2 proof.

## Current Bad Assumptions This Document Rejects

| Bad assumption | Why rejected |
| --- | --- |
| Adding `nonce` to the current v1 block is enough | The nonce must bind to canonical blockspace and `checks_root`. |
| `receipt_root` is close enough to `settled_receipt_set_root` | A global receipt map root does not define selected eligible settled receipts. |
| Per-attestation `checks_root` is a block checks root | Blocks need an aggregate root over canonical selected receipt transcripts. |
| `difficulty_target` validates itself | A target must be derived from parent difficulty state and bounded retarget rules. |
| Finality validates whatever block exists | Finality must count only votes for blocks that pass `validate_block_v2`. |
| TensorWork is still acceptable for proposer choice | v2 makes validator useful-PoW the proposer primitive. |
| Empty blocks can use old fallback | v2 fallback has different reward and eligibility semantics. |
| Fallback is useful-PoW | v2 fallback is a reduced-reward liveness path and must not claim useful work. |
| Finalized block means settled rewards | Reward finality is delayed until direct recomputation or challenge-window settlement. |

## Release Gate For Moving Consensus Out Of Blocked Status

Do not move any consensus theorem from `implementation-blocked` to `local-proof-ready` until all of these
are true:

1. The current block type or successor carries the required v2 fields.
2. A canonical settled-receipt selector exists and is deterministic under tests and theorem statement.
3. A block-level `checks_root` is recomputable from selected receipt transcripts.
4. Useful-PoW validation binds nonce search to the selected receipt root and checks root.
5. Proposer eligibility excludes miner TensorWork.
6. Vote admission requires block validation.
7. Finalized set mutation is only possible through validated votes or a separately validated fallback.
8. Fallback validity is specified separately from the superseded v1 proposer fallback.
9. Negative proof counterexamples CEX-001 through CEX-004 no longer construct accepted/finalized states.

## Current Judgment

The v2 consensus proof target is now sharply specified, but it remains a target. The current implementation
still supports the narrow sound kernel and v1/reference finality only. The reviewed v2 MVP core is not sound
until the proof obligations in this document are backed by code, tests, and formal artifacts.
