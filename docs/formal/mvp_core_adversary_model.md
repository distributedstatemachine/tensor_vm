# TensorVM MVP Core Adversary Model

Status: documentation-only adversary model compiled from the current worktree.

Purpose: define the adversary that current MVP proof claims are measured against. A proof without this model
is ambiguous: "sound" could mean algebraic verifier soundness, syntactic chain-admission safety, public DA,
or finalized v2 useful-PoW validity. Today only the first two have a defensible proof boundary, and even
those require explicit assumptions.

## Proof Worlds

| World | What It Covers | Current Status | Main Exclusion |
| --- | --- | --- | --- |
| Verifier-local world | Malicious miner submits incorrect tensor outputs or metadata to TensorOp/LinearTrainingStep verifiers. | Defensible under probabilistic assumptions. | Does not prove block proposer eligibility or finality. |
| Syntactic chain-admission world | Validators submit signed attestations and current settlement counts syntactic quorum statements. | Defensible for syntax only. | Does not prove validators actually ran the verifier. |
| Reviewed v2 consensus world | Finalized blocks prove useful-verification PoW over canonical settled-receipt blockspace. | Implementation-blocked. | Current blocks/finality cannot express this theorem. |
| Public operation world | Public DA, independent operators, retention windows, and production authentication. | Evidence-blocked. | Local Compose and local fetches are insufficient. |

Any public or release-facing claim must say which world it belongs to. Moving a claim from one world to a
stronger world requires the discharge gates in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md).
Verifier-local probability budgets are tracked in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
The seed model for delayed attestations and non-adaptive challenges is tracked in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The signature/authentication boundary is tracked in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The canonical encoding and commitment boundary is tracked in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The useful-PoW work and economics boundary is tracked in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
The verifier evidence boundary is tracked in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).

## Adversary Capabilities

| Area | Adversary May Do | Current Defense | Still Missing |
| --- | --- | --- | --- |
| Tensor outputs | Submit malformed shapes, wrong roots, corrupted outputs, or inconsistent LinearTrainingStep tensors. | Deterministic metadata/root checks plus Freivalds and random-linear checks. | Mechanized proof and exact challenge model. |
| Challenge timing | Try to choose outputs after learning validation challenges. | The proof assumes committed outputs before hidden challenge derivation. | Immutable receipt-lifecycle seed and proof that outputs cannot adapt after the seed. |
| Randomness | Grind or bias public randomness and exploit weak domain separation. | Current docs name randomness assumptions. | Full seed lifecycle, domain tags, and grindability analysis tied to consensus state. |
| Signatures | Forge statements, replay statements, or claim another actor's authority. | Current code exercises a reference signature relation. | Production signature scheme, replay domains, and key-ownership model. |
| Validator behavior | Sign `Valid` without running the verifier, withhold attestations, duplicate attempts, or disagree. | Admission prevents duplicates and enforces current assignment syntax. | Semantic verifier-execution evidence, Byzantine stake bound, slashing/challenge rule, or explicit honesty assumption. |
| Block proposal | Produce blocks with arbitrary proposers or superseded TensorWork selection. | None for reviewed v2; current behavior is reference-only. | v2 proposer eligibility and useful-PoW validation. |
| Useful-PoW economics | Grind nonces over cheap, empty, cached, shared, or easy-target verification commitments and still claim useful work. | None for reviewed v2; current behavior has no useful-PoW predicate. | Parent-state target validation, work floor, retarget bounds, transcript-acquisition model, and conservative wording. |
| Blockspace | Omit, reorder, or substitute settled receipts. | None for reviewed v2. | Canonical selected-receipt pool, caps, expiry, spent/carry-over rules, and selected root. |
| Verification transcript | Mine over arbitrary, incomplete, or merely signed checks roots. | Per-attestation checks roots exist as statements only. | Block-level checks root, check leaves, recomputation, openings, and challenge-window reward semantics. |
| Reward settlement | Rush spendable rewards before challenges, hide artifacts until the challenge window closes, or exploit clawback gaps. | The local runtime models pending receipt, proposer, challenge, audit-related, and credit reward claims with maturity, voiding, and nonpayment/prune paths for implemented disputes. | Full verifier-transcript challenge openings, DA-through-window evidence, public dispute propagation, and theorem discharge. |
| Finality | Vote for a known current block that is not a valid v2 useful-PoW block. | Current vote path checks voter syntax and stake. | Finality vote admission must require `validate_block_v2`. |
| Data availability | Withhold tensors, serve only one peer, drop data after local tests, or serve unavailable artifacts publicly. | Root-matched fetch can reject wrong payloads. | Public retention measurements and independent serving evidence. |
| Network scheduling | Delay messages, reorder attestations, partition local peers, or exploit timeout ambiguity. | Not modeled in the current proof kernel; fallback is paper-specified only. | Explicit synchrony/partial-synchrony assumptions, timeout/no-work evidence, and fallback rules. |

## Minimum Parameters For Future Theorems

Future proof statements must declare these parameters instead of relying on prose:

| Parameter | Needed For | Current Status |
| --- | --- | --- |
| `F` and `|F|` | Freivalds and random-linear false-accept probabilities. | Present in proof sketches, not mechanized. |
| `rounds` and challenge domains | Repeated probabilistic soundness bounds. | Named, not fully enumerated. |
| Receipt-lifecycle seed | Non-adaptivity of miner outputs. | Missing as stable consensus object. |
| Byzantine validator bound or challenge rule | Upgrading quorum from syntactic to semantic. | Missing. |
| Quorum threshold and stake model | Settlement and finality safety. | Current v1/reference behavior only. |
| Network synchrony and timeout model | Fallback and liveness claims. | Paper-specified in `mvp_core_fallback_liveness_model.md`, not implemented. |
| DA retention window and observer threshold | Public availability claims. | Missing external evidence. |
| Signature scheme and domains | Actor authentication. | Reference helper only. |
| Hash model and security level | Commitment binding and PoW. | Assumed, not parameterized in theorems. |
| Difficulty and retarget parameters | Useful-PoW target validity and liveness. | Paper-specified in `mvp_core_difficulty_retarget_model.md`, not implemented. |
| Verification, transcript acquisition, and nonce costs | Useful-work dominance claims. | Missing as a discharged model. |
| Verifier evidence and challenge-window liveness | Semantic attestation and checks-root claims. | Paper-specified across verifier-evidence and reward-finality docs, not implemented. |

## Claims The Current Model Supports

Under collision-resistant hashes, hidden uniform-enough challenges, committed outputs before challenge
derivation, available verifier artifacts, and the current signature relation:

1. Malformed TensorOp and LinearTrainingStep receipts face the documented probabilistic verifier checks.
2. Successful attestation admission implies an assigned registered validator signed a matching statement.
3. Current local settlement can be described as syntactic quorum settlement under current rules.
4. Root-matched tensor fetch can protect verifier execution from accepting a payload with the wrong
   commitment root.

These are not the same as public DA, production authentication, semantic validator honesty, or v2 finality.

## Claims The Current Model Does Not Support

The current model must reject these claims:

1. A finalized current block is evidence of useful-verification PoW.
2. A `Valid` quorum proves validators executed the verifier.
3. Produced blocks prove registered-validator proposer eligibility.
4. TensorWork has been removed from proposer selection in implementation.
5. Remote fetch counters prove public durable DA.
6. Local Compose proves independent operators.
7. LinearTrainingStep proves real-valued ML training.

## Required Negative Tests Or Proof Cases

When code work resumes, every v2 upgrade should include adversarial tests or proof cases for:

1. Incorrect output accepted only with the stated Freivalds/random-linear probability model.
2. Delayed attestation evaluated against the original receipt-lifecycle seed, not current finalized
   randomness.
3. Attestation signed by unassigned, unregistered, duplicate, or wrong-metadata validators rejected.
4. Block with wrong selected receipt root rejected.
5. Block with unrecomputable checks root rejected.
6. Block with nonce satisfying a header that differs from the validated header rejected.
7. Block from non-eligible proposer rejected before votes count.
8. Finality vote for invalid v2 block rejected.
9. DA payload with wrong root rejected and unavailable payload handled without pretending public DA exists.

## Current Judgment

The adversary model exposes the same core conclusion as the completion audit: the verifier-local kernel can
be made precise, but the full reviewed MVP core is not sound today. The consensus layer still lacks the
objects needed to defend against malicious proposers, syntactic-but-dishonest validator quorums, invalid
blockspace selection, and finality over the wrong block object.
