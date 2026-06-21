# TensorVM MVP Core Proof Completion Audit

Status: documentation-only completion audit for the formal-proof objective.

Audit date: May 23, 2026.

Objective audited:

```text
create formal proofs for the core of mvp. keep upgrading core until it is sound.
be critical, call our bad assumptions and create a doc. do not write code, just compile findings in an md.
```

This audit uses the current worktree as evidence. It does not mark the goal complete. The proof corpus is
materially stronger than before, but the full reviewed v2 MVP core is still not sound.

The local v2-block reference path is audited separately in
[`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md). That implementation is useful
direction and build-clean locally, but it still does not discharge the full proof obligations.

## Verdict

Current result: **not complete**.

The repository now has a defensible paper-proof kernel for verifier-local algebra and syntactic chain
admission. It also has explicit negative proofs and bad-assumption ledgers. It does not have a completed
formal proof for the full MVP core because the current consensus object still admits states outside the
reviewed v2 theorem.

The core blocker is not documentation polish. It is that the implementation still lacks the v2 consensus
objects required to state and prove the theorem honestly:

```text
canonical settled-receipt blockspace
block-level checks_root
useful-verification PoW target and nonce
v2 block validity predicate
finality over valid v2 blocks only
receipt-lifecycle validation seed
production signature model
public DA and operator-independence evidence
```

## Audit Status Legend

| Status | Meaning |
| --- | --- |
| Proven locally | The paper theorem maps to current deterministic Rust semantics and local tests/evidence. |
| Assumption-bound | The theorem can be stated, but depends on explicit crypto, randomness, availability, economic, or honesty assumptions. |
| Contradicted | The current implementation admits a counterexample to the claimed theorem. |
| Implementation-blocked | The code does not expose the state/transition object needed to state the theorem honestly. |
| Missing evidence | The claim may be intended, but the current worktree lacks enough evidence to prove it. |

## Objective Requirements Audit

| Requirement | Evidence Inspected | Status | Completion Finding |
| --- | --- | --- | --- |
| Create formal proofs for MVP core | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md`, `mvp_core_formal_proofs.md` | Partial | Paper proof statements exist for the narrow kernel. No mechanized Lean/TorchLean proof exists. Full consensus proof is blocked. |
| Keep upgrading core until sound | Current docs and Rust consensus surfaces | Not complete | The user has restricted this pass to Markdown only. The docs identify required implementation gates, but the core itself remains unsound for v2. |
| Be critical | `bad_assumptions_ledger.md`, `mvp_core_negative_proofs.md`, `mvp_core_data_availability_boundary.md` | Met for docs | The current docs call out false assumptions directly, including finality, proposer eligibility, quorum semantics, public DA, and signatures. |
| Call bad assumptions | `bad_assumptions_ledger.md` | Met for current audit | The ledger now names 28 bad assumptions and wording rules. It should remain open. |
| Create docs only | Git history and current staged policy | Met for this proof-work stream | Recent proof commits are Markdown-only. Dirty code changes in the worktree are not part of this proof-doc audit. |
| Prove full reviewed MVP core sound | v2 spec plus current chain code | Not complete | Current blocks and finality cannot express the reviewed useful-verification PoW theorem. |

## Proof Area Audit

| Area | Evidence | Status | What Is Proven | What Is Not Proven |
| --- | --- | --- | --- | --- |
| Canonical field tensor semantics | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md`, `tensor.rs`, `field.rs`, `vm.rs` | Proven locally | Deterministic finite-field tensor operations are a plausible kernel base. | GPU equivalence and external framework equivalence remain separate obligations. |
| TensorOp completeness | `K-TOP-001`, `TVM-FRV-001`, verifier tests | Proven locally | Honest matmul receipts accepted under matching roots, shapes, metadata, and signature relation. | Production signature security and artifact availability are assumptions. |
| TensorOp soundness | `K-TOP-002`, `TVM-FRV-002` | Assumption-bound | Freivalds false-accept bound under hidden uniform-enough challenges and committed outputs. | Deterministic all-cell correctness is false. Randomness and hash assumptions remain. |
| Row sampling | `K-ROW-001`, `TVM-ROW-001` | Proven locally as audit math | Hypergeometric detection probability is documented. | Row sampling is not sufficient block-validity security. |
| LinearTrainingStep completeness | `K-LIN-001`, `TVM-LIN-001` | Proven locally | Deterministic finite-field training-step equations can be checked. | Real-valued SGD, convergence, fixed-point approximation, and usefulness are not proven. |
| Random-linear equality soundness | `K-LIN-002`, `TVM-LIN-002` | Assumption-bound | False-accept bound for non-equal tensors under uniform challenge. | Random-oracle and hidden-challenge assumptions remain. |
| Receipt/root binding | `TVM-COM-001` | Assumption-bound | Canonical encodings can be specified and hash-bound. | Hash collision resistance is assumed, not proven. |
| Signature validity | `TVM-SIG-001`, `types::sign` | Assumption-bound | Current helper tests message-flow shape. | Production authentication, key ownership, anti-replay, and unforgeability are not proven. |
| Attestation admission | `K-ATT-001`, `TVM-ATT-001` | Proven locally with caveat | Successful admission implies registered assigned validator, matching receipt metadata, valid statement signature, and no duplicate. | Assignment seed is not receipt-lifecycle stable. Semantic verifier execution is not proven by admission. |
| Attestation quorum | `K-QUO-001`, `TVM-QUO-001` | Proven locally only syntactically | Quorum counts unique assigned signed Valid/DataAvailable statements under current rules. | Quorum does not prove validators actually ran the verifier. |
| Local settlement | `K-SET-001`, `TVM-SET-001` | Proven locally for v1 behavior | Newly settled receipts had syntactic quorum, redundant agreement if configured, and no current conflict blocker. | v2 blockspace inclusion, challenge-window reward finality, and semantic verification are not proven. |
| Verification-time artifact retrieval | `mvp_core_data_availability_boundary.md` | Proven locally for root matching, assumption-bound for availability | Fetched tensor payloads are checked against requested commitment roots before verifier use. | Public DA, durable retention, miner-specific serving, and challenge-window availability are not proven. |
| Canonical settled-receipt blockspace | `TVM-BLK-001`, `mvp_spec.md` | Partial / implementation-blocked | Local selected roots, inclusion tracking, parent snapshots, and Docker evidence exist. | Full selected-receipt lifecycle, cap/carry-over theorem, and public proof are not discharged. |
| Block-level `checks_root` | `TVM-BLK-002`, `mvp_core_negative_proofs.md` | Partial / implementation-blocked | Block-level roots and typed local check transcripts exist. | Full recomputable verifier transcript dispute theorem is not discharged. |
| Useful-verification PoW | `TVM-POW-001`, current `TensorBlock` | Partial / implementation-blocked | Local blocks carry selected roots, `checks_root`, target, nonce, and validator proposer evidence. | Full useful-work dominance, retargeting, interactive dispute, and public/CUDA proof remain incomplete. |
| Difficulty retargeting | `TVM-DIFF-001`, `mvp_core_difficulty_retarget_model.md` | Implementation-blocked | The paper model now defines parent-state target derivation and bounded retargeting. | No v2 difficulty state, retarget window, target-bound theorem, or hash-to-target vectors are implemented. |
| Proposer eligibility | `CEX-002`, `chain::blocks::produce` | Partial | Local block production rejects nonvalidators and Docker proves the single configured validator proposer path. | Multi-validator/public proposer competition and full useful-PoW winner proof remain incomplete. |
| TensorWork proposer removal | `CEX-003`, `chain::proposer` | Locally addressed / proof incomplete | Current selector ignores TensorWork for proposer eligibility. | Public/multi-validator proposer proof and full theorem discharge remain incomplete. |
| Finality implies v2 validity | `TVM-FIN-001`, `CEX-001` | Partial / implementation-blocked | Votes validate known blocks with strict parent-root checks before counting. | Exact parent-state validation certificate and full v2 validity theorem remain incomplete. |
| Zero-receipt fallback | `TVM-FALLBACK-001`, `mvp_core_fallback_liveness_model.md` | Partial / implementation-blocked | Local `PowSkipFallback` covers timeout/no-work, selected validator rotation, reduced rewards, and no useful-PoW claim. | Full synchrony/liveness proof, multi-validator runtime evidence, and public/CUDA proof remain incomplete. |
| Reward finality and challenges | `TVM-REWARD-001`, `mvp_core_reward_finality_challenge_model.md` | Partial / implementation-blocked | Local pending reward ledgers, challenge reward claims, voiding/clawback, and maturity release exist for implemented paths. | Full interactive transcript challenge theorem and public deployed evidence remain incomplete. |
| Public operator independence | `bad_assumptions_ledger.md`, `completion_audit.md` | Missing evidence | Local Compose proves local multi-participant shape. | Independent public operators are not proven. |
| Public DA | `mvp_core_data_availability_boundary.md`, public evidence docs | Missing evidence | Local/remote fetch paths can be exercised. | Public retention and signed external DA measurements are missing. |

## Completion Blockers By Severity

### Critical

1. **Consensus theorem cannot be stated over current block type.**
   Current `TensorBlock` lacks the v2 fields required by `mvp_spec.md`: `settled_receipt_set_root`,
   `checks_root`, `difficulty_target`, and `nonce`.

2. **Finality can certify the wrong object.**
   Current finality counts votes over known v1 block hashes. It does not require v2 block validation.

3. **Proposer eligibility is not proven by block production.**
   The block append path accepts a supplied proposer address; the vote path validates voters, not proposer
   eligibility.

4. **TensorWork proposer selection is still present.**
   The current proposer selector is a superseded v1 object, not reviewed v2 useful-verification PoW.

### High

5. **Attestation quorum is syntactic.**
   It proves assigned validators signed statements, not that they ran the verifier correctly.

6. **Validation assignment seed is not receipt-lifecycle stable.**
   Assignment is recomputed from current finalized randomness, so delayed attestations can be judged against
   the wrong beacon.

7. **Reference signatures are not production authentication.**
   The current helper is a hash relation, not a key-ownership proof.

8. **Remote tensor fetch is not public DA.**
   Root-matched retrieval helps verification-time availability; it does not prove durable public retention.

### Medium

9. **Field training is not real training.**
   LinearTrainingStep proves finite-field algebraic consistency only.

10. **Coverage and local gates do not prove protocol soundness.**
    Tests are useful regression evidence, but they do not replace consensus theorems and assumptions.

## Required Evidence To Mark Complete

Do not mark the formal-proof goal complete until all of the following are true:

1. The proof manifest classifies no full-MVP consensus theorem as `implementation-blocked`.
2. The current block type or successor exposes v2 block fields and an honest validation predicate.
3. A canonical settled-receipt selector exists and is mapped to a theorem with deterministic ordering,
   eligibility, caps, spent/carry-over, expiry, and omission rules.
4. Block-level `checks_root` is recomputable from selected receipt transcripts and has a challenge/opening
   story.
5. Useful-verification PoW ties the nonce to parent, selected receipt root, checks root, beacon, proposer,
   and target.
6. Finality vote admission rejects blocks that fail v2 validity.
7. Proposer eligibility is a theorem over registered validators and useful-PoW success, not caller input or
   TensorWork.
8. Receipt assignment uses a receipt-lifecycle validation seed or equivalent immutable anchor.
9. Attestation/quorum theorem language either stays syntactic or is upgraded with recomputable/challengeable
   verifier evidence.
10. Signature assumptions are explicitly production-grade or the reference-signature boundary remains in
    every theorem.
11. Public DA/operator-independence claims have signed external evidence, or they are excluded from the core
    soundness claim.
12. Every public-facing summary uses the language rules from `bad_assumptions_ledger.md`.

The blocked v2 consensus proof obligations are expanded in
[`mvp_core_v2_consensus_proof_obligations.md`](mvp_core_v2_consensus_proof_obligations.md).
The proof assumptions and their discharge gates are classified in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md).
The adversary model for the current proof boundary is stated in
[`mvp_core_adversary_model.md`](mvp_core_adversary_model.md).
The theorem import/dependency cut line is documented in
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md).
Verifier-local probability budgets are recorded in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
The receipt-lifecycle seed model for non-adaptive verifier challenges is specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The signature/authentication boundary is specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The canonical encoding and commitment boundary is specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The settled-receipt blockspace model is specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
The useful-PoW work model is specified in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
The verifier evidence model is specified in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
The parent-state transition model is specified in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).
The blocked v2 state invariants are listed in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).
The proof-to-implementation claim boundary is summarized in
[`mvp_core_proof_traceability_matrix.md`](mvp_core_proof_traceability_matrix.md).

## Current Safe Summary

The strongest safe statement today is:

```text
TensorVM has a documented proof-ready verifier kernel for finite-field TensorOp and LinearTrainingStep
checks, plus syntactic chain-admission and settlement invariants under explicit assumptions. The reviewed
v2 consensus core is not formally sound yet because block production and finality still use the wrong
consensus object.
```

The unsafe statement is:

```text
The MVP core is formally proven sound.
```

## Next Documentation Move

The next docs-only artifact has been created as [`mvp_core_mechanization_checklist.md`](mvp_core_mechanization_checklist.md).
If this remains docs-only after that, the next useful move is to keep the checklist synchronized with code
changes and refuse to move any blocked consensus theorem into the sound kernel until the implementation
exposes the required objects.
