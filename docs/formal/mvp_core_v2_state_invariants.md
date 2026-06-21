# TensorVM MVP Core v2 State Invariants

Status: documentation-only invariant map for the blocked v2 consensus proof.

Purpose: state the invariants that must be preserved before TensorVM can honestly prove that finality
implies valid useful-verification PoW over canonical blockspace. The existing proof docs list target
objects and counterexamples; this document turns them into state-transition invariants.

The receipt-lifecycle seed invariant used by `INV-002` is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The settled-receipt blockspace model used by `INV-003` through `INV-006` is specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
The canonical encoding and commitment model used by `INV-006` and `INV-008` is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The useful-PoW work model used by `INV-009` is specified in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
The verifier evidence model used by `INV-007` and `INV-008` is specified in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
The parent-state transition model used by `INV-011`, `INV-012`, and `INV-013` is specified in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).

This document does not implement v2 and does not mark v2 sound. It is a proof target for future code and
mechanization.

## Current Verdict

The current chain state does not satisfy the v2 invariant set. Some invariants are not representable, and
some are contradicted by the current reference block path.

The core missing proof shape is:

```text
Init(S0)
forall valid_v2_block(B, S):
  Inv(S) -> apply_v2_block(S, B) = S' -> Inv(S')
finalized(B, S') -> valid_v2_block(B, parent(S'))
```

Current Rust can support narrower syntactic invariants over receipt admission, attestation admission, and
v1/reference settlement. It cannot support the v2 finality invariant because current blocks do not carry the
required witness fields and vote admission does not require v2 validation.

## Invariant Classes

| Class | Meaning | Current Status |
| --- | --- | --- |
| Receipt lifecycle | Receipt commitments, seeds, eligibility, and challenge windows are immutable where needed. | Partially represented, seed lifecycle is weak. |
| Canonical blockspace | Eligible settled receipts have deterministic inclusion, omission, spent, and carry-over semantics. | Missing. |
| Verification transcript | Every selected receipt has recomputable check leaves and block-level aggregate binding. | Missing. |
| Useful-PoW block validity | Header, nonce, target, selected root, checks root, beacon, and proposer are validated together. | Local implementation partial; full theorem blocked. |
| Difficulty validity | Targets are derived from parent difficulty state and bounded retarget rules. | Paper-specified, implementation-blocked. |
| Proposer eligibility | Block proposer is a registered validator that won useful-verification PoW. | Local implementation partial; multi-validator/public proof blocked. |
| Reward finality | Verifier-dependent rewards remain pending until challenge-window finality or direct recomputation. | Local implementation partial; full theorem blocked. |
| Finality safety | Votes and finalized-set mutation are allowed only after v2 block validation. | Missing. |
| Fallback safety | Empty/timeout fallback has explicit validity and reward restrictions. | Local implementation partial; full liveness proof blocked. |
| Public evidence | DA and operator-independence claims are backed by external observations. | Evidence-bound. |

## Target Invariants

| ID | Invariant | Required State | Preservation Obligation | Current Failure |
| --- | --- | --- | --- | --- |
| INV-001 | Receipt identity is stable after admission. | receipt id, receipt hash, job id, primitive, roots, miner, TWU, submitted height. | No later transition mutates fields included in the receipt id or verifier transcript. | Partially present, but v2 selected receipt metadata is incomplete. |
| INV-002 | Receipt validation seed is lifecycle-stable. | receipt-lifecycle seed or immutable assignment anchor. | Delayed attestations and v2 check leaves use the same seed fixed at receipt admission or settlement. | Assignment can depend on current finalized randomness. |
| INV-003 | Settled receipt eligibility is deterministic. | settled height, expiry, DA status, spent/included marker, challenge-window status. | Every node derives the same eligible set from the same parent state. | Current `settled_receipts` is a bare id set without full lifecycle metadata. |
| INV-004 | Canonical selected receipt set is deterministic and cap-respecting. | settled receipt pool, TWU cap, byte cap, count cap, deterministic order, cap policy version. | Applying a v2 block either selects exactly `canonical_selected_receipts(S, beacon, caps, policy)` or rejects, and every omission is explained. | Cap policy and omission/carry-over theorem are not discharged. |
| INV-005 | Nonselected eligible receipts carry over unless expired or invalidated. | carry-over state and expiry rules. | Block application marks selected receipts spent and preserves unselected eligible receipts according to the rule. | No v2 carry-over transition exists. |
| INV-006 | Selected receipt root binds the canonical selected set. | `settled_receipt_set_root` and canonical selected receipt leaf encoding. | Recompute root from selected leaves and reject mismatches before votes count. | Receipt id roots do not prove eligibility, cap accounting, or selected leaf semantics. |
| INV-007 | Check leaves are recomputable or challengeable. | check leaf schema, verifier transcript roots, DA proof root, challenge openings. | Every selected receipt has a leaf that can be recomputed under parent state and block beacon, or disproven by a consensus-valid opening. | Per-attestation `checks_root` is arbitrary statement evidence unless bound to transcript evidence. |
| INV-008 | Block `checks_root` binds every selected check leaf. | aggregate check root, selected order, evidence schema version. | Recompute aggregate root from all selected check leaves or validate openings against it; reject mismatches before semantic claims. | Aggregate roots over signed claims do not prove transcript truth. |
| INV-009 | Useful-PoW header is bound to validated content. | parent, selected root, checks root, beacon, proposer, target, nonce, parameter version. | Nonce target is checked over exactly the fields validated by block validity; economic useful-work claims separately satisfy the work model. | Current block has no target/nonce predicate, and useful-work dominance is unmodeled. |
| INV-009A | Difficulty target is parent-state-derived and bounded. | `DifficultyState`, `DifficultyParams`, retarget window, hardest/easiest target bounds, hash-to-target encoding, work-floor params. | Block validation rejects targets that do not equal the parent-derived expected target or violate retarget bounds. | No v2 difficulty state or retarget theorem exists. |
| INV-010 | Proposer is v2 eligible. | validator registry, eligibility rules, useful-PoW result. | Block admission rejects arbitrary proposers and excludes TensorWork proposer selection. | Local validator proposer checks exist, but complete useful-PoW winner semantics and public multi-validator evidence are not discharged. |
| INV-011 | State and reward roots are deterministic after valid v2 apply. | parent-state snapshot, v2 apply transition, state root, reward root, spent/carry-over updates. | Applying the same valid block to the same parent yields one child state and matching roots; failed admission has no partial mutation. | Current roots are v1/reference global-map roots and no parent-state apply theorem exists. |
| INV-012 | Vote admission imports parent-state v2 validation. | `validate_block_v2(parent_state, block)` result, vote signature, stake snapshot, duplicate rule. | Votes for invalid blocks or blocks validated against the wrong state are rejected before finality weight is counted. | Current votes check known block hash, voter stake, and signatures only. |
| INV-013 | Finalized-set mutation implies prior parent-state v2 validation. | finalized block set and validation certificate. | A block enters finalized state only through valid v2 vote quorum or valid fallback path with a certificate for the exact parent state. | Current finality can certify a reference block. |
| INV-014 | Fallback validity is explicit and reward-safe. | timeout/synchrony state, no-work evidence, validator rotation, reduced reward, no miner TWU reward, parent-state fallback certificate. | Fallback blocks preserve safety and cannot claim useful work. | Local `PowSkipFallback` implementation is partial; full synchrony/liveness proof and public runtime evidence remain blocked. |
| INV-015 | Public DA claims are evidence-linked. | retention window, observers, signed measurements, operator identities. | Public claims require enough signed evidence for the window and operator threshold. | Local/remote fetch proves only verification-time retrieval. |
| INV-016 | Reward finality is delayed and challenge-safe. | pending/challenged/invalidated/settled claim status, challenge windows, opening roots, maturity deadlines, spendable balances. | Block application creates pending claims; only direct recomputation or closed challenge windows settle them, and valid challenges claw back or invalidate dependent claims before spendability. | Local pending ledgers and challenge voiding exist for implemented paths; full interactive transcript challenge theorem remains blocked. |

## Preservation Theorems

Future proof work should name these preservation theorems explicitly:

```text
receipt_admission_preserves_receipt_identity
receipt_admission_fixes_validation_seed
settlement_preserves_eligible_receipt_pool
canonical_selection_is_deterministic
apply_v2_block_spends_only_selected_receipts
selected_receipt_root_matches_canonical_selection
check_leaf_recomputes_for_selected_receipt
checks_root_matches_all_selected_leaves
valid_useful_pow_binds_validated_header
difficulty_target_derived_from_parent_state
valid_v2_block_implies_proposer_eligible
apply_v2_block_roots_are_deterministic
vote_admission_requires_validate_block_v2
finalized_block_has_validity_certificate
fallback_block_preserves_v2_safety
reward_finality_waits_for_challenge_window
valid_challenge_invalidates_only_dependent_claims
public_da_claim_requires_external_evidence
```

Do not collapse these into a single `valid_block` theorem until each dependency is separately testable and
traceable.

## Current Counterexample Coverage

| Counterexample | Killed By Invariants |
| --- | --- |
| CEX-001 finalized block with no useful-PoW witness | INV-006, INV-008, INV-009, INV-012, INV-013 |
| CEX-002 produced block does not imply proposer eligibility | INV-010, INV-012, INV-013 |
| CEX-003 TensorWork proposer selection contradicts v2 | INV-010 |
| CEX-004 receipt map root is not canonical blockspace | INV-003, INV-004, INV-005, INV-006 |
| CEX-005 quorum is syntactic unless evidence is bound | INV-007, INV-008, INV-012 |
| CEX-006 assignment uses current beacon, not receipt-lifecycle seed | INV-002 |
| CEX-007 reference signatures are not production auth | INV-012 plus production signature assumption |
| CEX-010 finalized block pays irreversible rewards before challenge finality | INV-011, INV-016 |
| CEX-011 block uses an easy self-supplied target | INV-009, INV-009A, INV-012 |

If a counterexample still constructs after an implementation change, the related invariant is not
discharged.

## Proof Review Gates

Before any v2 consensus claim moves from `blocked-v2` to `local-proof-ready`, require:

1. The invariant is representable in committed code.
2. The transition that preserves it is named.
3. Honest and adversarial tests cover the transition.
4. The theorem statement appears in `formal_proof_manifest_v0.md`.
5. The traceability matrix links the theorem to code and evidence.
6. Any remaining cryptographic, randomness, economic, or public-evidence assumption is still visible.
7. The negative proof ledger no longer has a live counterexample for the claim.

## Current Judgment

The v2 consensus proof is blocked at the invariant layer, not just at theorem wording. Until these
invariants are implemented and preserved by block admission, block application, vote admission, and finality
mutation, the only defensible core proof remains the narrow verifier-local and syntactic current-chain
kernel.
