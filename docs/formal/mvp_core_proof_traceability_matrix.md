# TensorVM MVP Core Proof Traceability Matrix

Status: documentation-only traceability matrix compiled from the current worktree.

Purpose: tie every current proof claim to the Rust surface, evidence class, allowed wording, bad assumption,
and next gate. This document is a control surface for avoiding accidental overclaiming. It is not a proof by
itself and it does not upgrade any blocked theorem.

The current local v2-block reference path is audited in
[`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md). Build-clean code upgrades
traceability status only for the specific gates it implements and tests.

Assumption categories and discharge gates are tracked in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md). A traceability row can
move to a stronger status only when its assumption-discharge category has been satisfied.
The adversary model used to interpret those assumptions is stated in
[`mvp_core_adversary_model.md`](mvp_core_adversary_model.md).
The theorem dependency graph is maintained in
[`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md).
Verifier-local probability budgets are recorded in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
Receipt-lifecycle seed requirements are specified in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
Signature/authentication boundaries are specified in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
Canonical encoding and commitment boundaries are specified in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
Settled-receipt blockspace boundaries are specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
Useful-PoW structural and economic boundaries are specified in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
Difficulty retarget boundaries are specified in
[`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md).
Fallback liveness boundaries are specified in
[`mvp_core_fallback_liveness_model.md`](mvp_core_fallback_liveness_model.md).
Verifier evidence boundaries are specified in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
Reward finality and challenge-window boundaries are specified in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md).
Parent-state transition boundaries are specified in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).
V2 state invariants are tracked in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).

## Status Key

| Status | Meaning |
| --- | --- |
| Defensible kernel | The claim is inside the current sound kernel under stated assumptions. |
| Assumption-bound | The claim can be stated only with explicit cryptographic, randomness, availability, or honesty assumptions. |
| Syntactic only | The chain proves a signed/structured statement, not the semantic truth of the statement. |
| Reference-only | Current behavior is useful local/v1 behavior, not reviewed v2 MVP consensus. |
| Blocked | Required code object or transition does not exist. |
| Contradicted | Current implementation admits a counterexample to the stronger claim. |
| Missing evidence | External, public, or mechanized evidence is absent. |

## Sound-Kernel Traceability

| Claim ID | Current Claim | Rust Surface | Evidence Docs | Status | Allowed Wording | Bad Assumption To Reject | Next Gate |
| --- | --- | --- | --- | --- | --- | --- | --- |
| K-FIELD-001 | Consensus tensor operations are deterministic finite-field operations under the modeled CPU semantics. | `tensor.rs`, `field.rs`, `vm.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel | "Canonical CPU/field semantics are deterministic." | "GPU or framework execution is automatically canonical." | Mechanize field/tensor definitions and prove Rust/formal equivalence. |
| K-TOP-001 | Honest TensorOp matmul receipts are accepted when roots, shapes, metadata, and signature relation match. | `verify.rs::verify_tensor_op`, `verify.rs::full_freivalds` | `mvp_core_sound_kernel.md`, `mvp_core_formal_proofs.md` | Defensible kernel | "TensorOp completeness is proof-ready." | "Acceptance proves production authentication." | Mechanize matmul/Freivalds completeness and keep signature as an explicit relation. |
| K-TOP-002 | Invalid TensorOp matmul outputs are caught with Freivalds false-accept probability bound. | `verify.rs::full_freivalds`, `tensor.rs::random_field_vector` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Freivalds gives a probabilistic bound under hidden uniform-enough challenges." | "Freivalds proves every cell deterministically." | Mechanize one-round and repeated Freivalds soundness with randomness assumptions visible. |
| K-ROW-001 | Row sampling detection probability is hypergeometric audit math. | `verify.rs::row_sample_detection_probability` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel as audit math | "Row sampling is audit telemetry." | "Row sampling alone is block-validity security." | Keep row sampling outside block eligibility unless parameters meet explicit bounds. |
| K-LIN-001 | LinearTrainingStep acceptance proves deterministic finite-field training-step relations. | `verify.rs::verify_linear_training_step`, `vm.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Defensible kernel | "LinearTrainingStep proves field-algebra transition consistency." | "This proves real-valued SGD or useful ML training." | Mechanize field algebra equations and separately define any fixed-point/real bridge if wanted. |
| K-LIN-002 | Random-linear equality checks have a false-accept bound under uniform challenge. | `verify.rs::random_linear_equal` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Random-linear checks are probabilistically sound under explicit assumptions." | "A hash-derived vector is automatically unbiasable." | Mechanize nonzero linear polynomial lemma and imported randomness assumptions. |
| K-COM-001 | Canonical encodings can be hash-bound to receipt/root preimages. | `types.rs::hash_bytes`, `chain/roots.rs`, receipt recompute functions | `formal_proof_manifest_v0.md`, `mvp_core_mechanization_checklist.md` | Assumption-bound | "Encoding injectivity is mechanizable before hash; hash binding is assumed." | "Lean proves SHA-256 collision resistance for this protocol." | Define canonical encodings and import collision-resistance assumption. |
| K-SIG-001 | Accepted statements satisfy the current signature relation. | `types.rs::sign`, `types.rs::verify_signature`, verifier/admission/vote checks | `bad_assumptions_ledger.md`, `formal_proof_manifest_v0.md` | Assumption-bound | "Reference signatures test message-flow shape." | "Reference `sign` is production authentication." | Replace or wrap with production signature model before making actor-control claims. |
| K-ATT-001 | Attestation admission success implies registered assigned validator, matching receipt metadata, signature relation, and no duplicate. | `chain/validation.rs::submit_attestation`, `scheduler.rs::assign_validators` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Syntactic only | "Admission proves an assigned validator signed a matching statement." | "Admission proves the validator ran the verifier." | Store receipt-lifecycle seed and bind attestation evidence to recomputable checks before semantic upgrade. |
| K-QUO-001 | Quorum counts unique assigned signed Valid/DataAvailable statements under current rules. | `chain/validation.rs::has_attestation_quorum` | `mvp_core_sound_kernel.md`, `mvp_core_negative_proofs.md`, `mvp_core_verifier_evidence_model.md` | Syntactic only | "Quorum proves assigned signed agreement." | "Quorum proves verified tensor work." | Add recomputable/challengeable check leaves or keep theorem explicitly syntactic. |
| K-SET-001 | Current settlement follows syntactic quorum, redundant-agreement, unavailable, and conflict checks. | `chain/settlement.rs` | `mvp_core_sound_kernel.md`, `formal_proof_manifest_v0.md` | Reference-only for v1 settlement | "Local v1 settlement follows syntactic quorum rules." | "Settlement proves v2 block inclusion." | Add v2 settled-receipt pool and blockspace transition before claiming v2 settlement. |
| K-DA-001 | Root-matched remote/local tensor fetch can support verifier-time artifact use. | `p2p.rs` request-response path, `main.rs::fetch_validator_role_missing_tensors`, `rpc.rs::tensor_by_commitment_root` | `mvp_core_data_availability_boundary.md` | Defensible kernel for local retrieval, assumption-bound for availability | "Verification-time artifact retrieval checks requested roots." | "Remote fetch proves public DA." | Add signed public retention measurements before public DA claims. |

## Blocked V2 Consensus Traceability

| Claim ID | Target Claim | Current Rust Surface | Evidence Docs | Status | Why Not Proven | Gate To Upgrade |
| --- | --- | --- | --- | --- | --- | --- |
| V2-BLK-001 | Canonical settled-receipt selection is deterministic and cap-respecting. | Current `ChainState` has `settled_receipts`, `included_receipts`, local block-selected receipt evidence, and deterministic caps. | `mvp_core_v2_consensus_proof_obligations.md`, `mvp_core_settled_receipt_blockspace_model.md` | Partial / blocked-v2 | No full settled-receipt pool metadata, expiry, challenge-window eligibility, carry-over, cap policy theorem, or omission theorem is discharged. | Add selector state and prove deterministic inclusion/omission/carry-over. |
| V2-BLK-002 | Block commits `settled_receipt_set_root` for canonical selected receipts. | Current blocks commit a selected receipt-id root; full selected receipt leaves are not bound. | `mvp_core_negative_proofs.md`, `formal_proof_manifest_v0.md`, `mvp_core_settled_receipt_blockspace_model.md` | Partial / blocked-v2 | Receipt id roots do not prove eligibility, cap accounting, reward fields, or lifecycle state. | Add selected receipt root over canonical selected leaves and prove leaf encoding. |
| V2-CHK-001 | Validators can recompute check leaves for selected receipts. | Verifier reports and attestation `checks_root` exist, but semantic evidence needs transcript objects or openings. | `mvp_core_v2_consensus_proof_obligations.md`, `mvp_core_verifier_evidence_model.md` | Blocked | No committed check leaf schema, transcript root format, or opening/challenge path discharges semantic evidence. | Define check leaves and transcript recomputation or challenge openings. |
| V2-CHK-002 | Block-level `checks_root` binds all selected receipt checks. | Block-check roots can aggregate claims, but semantic binding requires recomputable selected receipt check leaves. | `mvp_core_negative_proofs.md`, `mvp_core_verifier_evidence_model.md` | Blocked | Per-attestation roots or aggregate roots over them do not prove verifier execution. | Add block-level checks root with recomputation/challenge validation. |
| V2-DIFF-001 | Difficulty target is derived from parent consensus state with bounded retargeting and canonical hash-to-target semantics. | Local v2-shaped work has a target helper; current committed proof status has no v2 difficulty state. | `mvp_core_difficulty_retarget_model.md`, `mvp_core_useful_pow_work_model.md`, `mvp_core_candidate_v2_block_audit.md` | Blocked | No parent `DifficultyState`, retarget window, target bounds, work-floor params, or hash boundary vectors discharge target validity. | Add difficulty state, target derivation, bounded retarget, target encoding vectors, fallback policy, and adversarial tests. |
| V2-POW-001 | Useful-PoW nonce is bound to parent, selected receipt root, checks root, beacon, proposer, and parent-state-derived target. | `chain/blocks.rs::produce` mines and validates a nonce over the local block header. | `mvp_core_v2_consensus_proof_obligations.md`, `mvp_core_useful_pow_work_model.md`, `mvp_core_difficulty_retarget_model.md` | Partial / blocked-v2 | Static target, no parent difficulty state, no work floor, and useful-work dominance is not modeled. | Add parent-state difficulty target, retargeting, work floor, and cost model. |
| V2-PROP-001 | Proposer is registered validator useful-PoW winner. | `produce_block` rejects nonvalidators; `chain/proposer.rs` selects registered validators by stake and ignores TensorWork; local CPU Docker proof covers the single configured validator proposer path. | `mvp_core_negative_proofs.md` | Partial / blocked-v2 | Local proposer networking is proven only for the single configured local producer; multi-validator/public proposer competition and complete useful-PoW winner semantics are not discharged. | Tie normal proposer networking and admission to complete useful-PoW eligibility and broaden runtime evidence. |
| V2-STATE-001 | Valid v2 block transition determines state and reward roots. | Current roots include local v2 block inputs, and votes check parent-root validity. | `mvp_core_v2_consensus_proof_obligations.md`, `mvp_core_parent_state_transition_model.md` | Blocked | No exact parent snapshot, child-state `apply_v2_block` theorem, full selected-receipt lifecycle, challenge-window reward semantics, or atomicity proof. | Define parent-state block apply transition and child root checks. |
| V2-REWARD-001 | Verifier-dependent rewards remain pending until direct recomputation or challenge-window finality; valid challenges invalidate dependent claims before spendability. | Local reward roots and status views expose pending receipt/proposer/challenge/credit claims, voiding, clawback/nonpayment, and maturity release for implemented paths. | `mvp_core_reward_finality_challenge_model.md`, `mvp_core_verifier_evidence_model.md`, `mvp_core_parent_state_transition_model.md`, `mvp_core_negative_proofs.md` | Partial / blocked-v2 | Full consensus challenge openings, DA-through-window settlement theorem, interactive transcript disputes, and public deployed evidence are not discharged. | Complete challenge admission/resolution theorem, root binding proof, and adversarial/public evidence. |
| V2-FIN-001 | Vote admission requires `validate_block_v2(parent_state, block)`. | `chain/validation.rs::submit_block_vote` calls strict block validation before counting votes. | `mvp_core_negative_proofs.md`, `formal_proof_manifest_v0.md`, `mvp_core_parent_state_transition_model.md` | Partial / blocked-v2 | It validates a reconstructed parent-like view, not an exact parent snapshot with child-state transition roots. | Add exact parent-state `validate_block_v2` and child-state apply validation before counting votes. |
| V2-FIN-002 | Finality implies v2 block validity. | `state.finalized_blocks` updates only after strict local block validation and stake threshold. | `mvp_core_proof_completion_audit.md`, `mvp_core_parent_state_transition_model.md` | Partial / blocked-v2 | Current finality lacks a certificate tying finalized hashes to exact parent-state v2 validation and fallback rules. | Restrict finalized-set mutation to exact parent-state validated v2 votes or validated fallback certificates. |
| V2-FALLBACK-001 | PoW-skip fallback has explicit timeout/no-work evidence, validator rotation, parent-state validation, reduced reward, and no miner TWU rewards. | Local `PowSkipFallback` covers timeout, deterministic stake-weighted proposer selection, reduced delayed proposer reward, and focused rejection tests. | `formal_proof_manifest_v0.md`, `mvp_core_v2_consensus_proof_obligations.md`, `mvp_core_fallback_liveness_model.md` | Partial / blocked-v2 | Full liveness/synchrony proof, multi-validator runtime competition, public/CUDA evidence, and mechanized theorem discharge are not complete. | Add public/multi-validator evidence and discharge the fallback liveness theorem. |

## Evidence Class Matrix

| Evidence Class | Counts For | Does Not Count For |
| --- | --- | --- |
| Rust unit/integration tests | Regression evidence for implemented behavior. | Formal proof of consensus theorem. |
| Paper proof docs | Claim boundary, theorem statements, assumptions, negative cases. | Mechanized proof or implemented behavior. |
| Mechanization checklist | Future proof work planning. | Current proof completion. |
| Local Compose evidence | Local multi-participant/runtime shape. | Public operator independence or public DA. |
| Remote tensor fetch counters | Verification-time retrieval observability. | Durable public data availability. |
| Tarpaulin/coverage | Lines exercised by tests. | Threat-model soundness. |
| Current v1 finality tests | Stake-signature threshold for current block hashes. | v2 useful-verification PoW validity. |

## Claim Approval Rules

A claim may be used in public or release-facing text only if all of these are true:

1. It appears in this matrix or the proof manifest.
2. Its status is not `Blocked` or `Contradicted`.
3. Its assumptions are stated in the same document or linked boundary doc.
4. Its wording matches the "Allowed Wording" column.
5. It does not depend on uncommitted code unless that change is explicitly cited as uncommitted evidence.
6. Any related assumption-discharge gate is satisfied or still stated as an assumption.

If any condition fails, phrase it as a gap or target, not as a property.

## Worktree Evidence Note

This matrix treats inspected local files as current-state evidence only after the associated implementation
changes are build-clean, tested, and mapped back into the matrix. The proof status remains conservative
until each stronger claim has an explicit validation gate.

## Current Judgment

The traceability picture is simple: the verifier-local kernel has a credible proof path under assumptions;
the reviewed v2 consensus layer remains blocked or contradicted by current v1/reference surfaces. No claim
that "the MVP core is formally proven sound" should pass review until every blocked v2 consensus row has a
real implementation surface, tests, and formal proof mapping.
