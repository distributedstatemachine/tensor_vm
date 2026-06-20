# TensorVM MVP Core Assumption Discharge Plan

Status: documentation-only discharge plan compiled from the current worktree.

Purpose: turn the proof assumptions and bad-assumption ledger into explicit gates. This document does not
prove the full MVP core sound. It says which assumptions are permanent model assumptions, which can be
mechanized, which require implementation changes, and which require external public evidence before claims
can be upgraded.

The adversary model for these assumptions is defined in
[`mvp_core_adversary_model.md`](mvp_core_adversary_model.md).
Verifier-local probability budgets are recorded in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
The receipt-lifecycle seed model needed to discharge challenge non-adaptivity is defined in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
The signature/authentication boundary is defined in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
The canonical encoding and commitment boundary is defined in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
The settled-receipt blockspace model is defined in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
The useful-PoW work and economics model is defined in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
The verifier evidence model is defined in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
The parent-state transition model is defined in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).

## Discharge Categories

| Category | Meaning | What Discharges It | What Does Not Discharge It |
| --- | --- | --- | --- |
| `permanent-assumption` | A cryptographic, randomness, or economic model assumption that remains visible in theorem statements. | A precise model, parameters, and accepted threat-model language. | Unit tests, local testnets, or mechanized algebra over an idealized primitive. |
| `formalizable` | A property that can become a theorem once the definitions are stable. | Mechanized proof or tightly reviewed paper proof over the exact model. | Implementation existence alone. |
| `implementation-dischargeable` | A gap caused by missing or wrong protocol state/transition code. | Implemented surface, adversarial tests, and traceability back to a theorem. | Documentation saying the intended behavior. |
| `evidence-dischargeable` | A claim about public operation, availability, or independence. | Signed external observations, retention-window measurements, or operator evidence. | Local Compose, local counters, or one successful request-response fetch. |
| `wording-guardrail` | A claim that should be excluded or phrased narrowly unless the protocol scope changes. | Correct wording in specs, READMEs, release notes, and proof docs. | More tests for the narrower current behavior. |

Discharge means "the specific avoidable gap is closed." It does not remove permanent cryptographic,
randomness, economic, or honesty assumptions from the theorem.

## Assumption Matrix

| ID | Assumption Or Gap | Category | Current State | Discharge Gate | Claim Until Discharged |
| --- | --- | --- | --- | --- | --- |
| AD-001 | Rust verifier semantics match the formal model. | `formalizable`, `implementation-dischargeable` | The proof docs use Rust surfaces as evidence, but no mechanized model or conformance harness exists. | Define the formal model for field, tensor, encoding, and verifier functions; add conformance vectors from Rust to the formal artifacts. | "Proof-ready under modeled Rust semantics." |
| AD-002 | Canonical tensor and receipt encodings are injective before hashing. | `formalizable` | Encoding/root claims are listed, but a complete formal encoding theorem is not present. | Specify every leaf and container encoding, prove decode/encode injectivity where applicable, and link roots to those encodings. | "Canonical encoding is an explicit proof obligation." |
| AD-003 | Hash binding is collision resistant. | `permanent-assumption` | The docs rely on hash-bound commitments and roots. | State the hash function, domains, security level, and collision-resistance assumption in every affected theorem. | "Hash binding holds under collision resistance." |
| AD-004 | Hash-derived Freivalds and random-linear challenges are uniform enough. | `permanent-assumption`, `formalizable` | Freivalds/random-linear soundness is assumption-bound. | Model hash-to-field sampling, domain separation, rejection/modulo behavior, and independence requirements; keep the random-oracle or PRF assumption explicit. | "Probabilistic soundness under hidden uniform-enough challenges." |
| AD-005 | The miner commits to outputs before seeing verifier challenges. | `implementation-dischargeable` | The kernel assumes a hidden receipt-bound validation seed, but assignment currently has a seed-lifecycle caveat. | Store or derive an immutable receipt-lifecycle validation seed at receipt admission and prove challenge derivation cannot be changed after receipt commitment. | "Soundness requires committed outputs before challenge derivation." |
| AD-006 | Challenge rounds and check domains are independent enough. | `formalizable`, `permanent-assumption` | Domain separation is named, not fully enumerated. | Define domain tags for job id, receipt id, primitive, round, tensor role, and check type; mechanize the algebra assuming independent samples. | "Repeated checks multiply bounds under explicit independence assumptions." |
| AD-007 | Verifier artifacts are available while verification runs. | `implementation-dischargeable`, `evidence-dischargeable` | Root-matched local/remote fetch supports verification-time retrieval, but not durable public availability. | Keep root-match validation in the verifier path; add failure semantics, retention policy, and public signed availability measurements if public DA is claimed. | "Verification-time artifact retrieval, not public DA." |
| AD-008 | A signed `Valid` attestation means the verifier actually ran correctly. | `implementation-dischargeable`, `wording-guardrail` | Current admission proves assigned signed statements only. | Bind attestations to recomputable check leaves, transcript openings, or challengeable verifier evidence before semantic wording is allowed; see `mvp_core_verifier_evidence_model.md`. | "Assigned validators signed matching Valid/DataAvailable statements." |
| AD-009 | Validator signatures prove production authentication and key ownership. | `implementation-dischargeable`, `permanent-assumption` | Current signature helper is a hash relation. | Replace/wrap the helper with a production signature scheme, replay domains, key registration, and an unforgeability assumption. | "Reference signatures test message-flow shape." |
| AD-010 | Canonical settled-receipt blockspace exists. | `implementation-dischargeable` | Current block roots commit global maps, and candidate selected roots still need eligibility, lifecycle, cap, and selected-leaf semantics. | Add settled receipt pool metadata, deterministic ordering, caps, expiry, spent/carry-over rules, omission theorem, and root over selected leaves; see `mvp_core_settled_receipt_blockspace_model.md`. | "Receipt roots or settled id sets are not v2 blockspace." |
| AD-011 | Block-level `checks_root` binds proposer verification work. | `implementation-dischargeable` | Selected receipt check leaves are locally openable and challengeable, but they still summarize attestation check roots rather than full verifier transcripts. | Define full transcript leaves, aggregate root format, recomputation rules, network/RPC challenge propagation, and openings/challenges tied to selected receipts; see `mvp_core_verifier_evidence_model.md`. | "Local check-leaf challenge evidence, not full semantic transcript evidence." |
| AD-012 | Useful-verification PoW exists and dominates nonce grinding. | `implementation-dischargeable`, `permanent-assumption`, `evidence-dischargeable` | Current blocks lack nonce, target, selected receipt root, and checks root; work dominance also needs transcript-acquisition and nonce-cost modeling. | Add v2 PoW predicate and validation; measure verification work versus nonce grinding under chosen parameters; keep economic assumptions explicit; see `mvp_core_useful_pow_work_model.md`. | "Useful-verification PoW is a target, not current behavior." |
| AD-012A | Difficulty target is derived from parent consensus state. | `implementation-dischargeable`, `formalizable` | Current/candidate target evidence is static/helper-derived and does not discharge retarget bounds or target encoding semantics. | Add versioned difficulty state, bounded retargeting, hardest/easiest target bounds, work-floor params, hash-to-target vectors, and tests; see `mvp_core_difficulty_retarget_model.md`. | "Nonce validity is checked against a parent-state-derived target." |
| AD-013 | Produced blocks imply proposer eligibility. | `implementation-dischargeable` | Current production can accept a caller-supplied proposer and the finality path validates voters, not proposer eligibility. | Make block admission fallible on v2 proposer eligibility: registered validator, valid useful-PoW predicate, target, and no superseded TensorWork proposer path. | "Current production is reference append behavior." |
| AD-014 | Finality implies v2 block validity. | `implementation-dischargeable` | Current votes are over known current block hashes. | Require `validate_block_v2` before accepting or counting finality votes; prove finalized-set mutation only follows validated v2 votes or validated fallback. | "Current finality is stake-threshold finality for current blocks." |
| AD-014A | V2 block validation uses the exact parent state and deterministic child transition. | `implementation-dischargeable`, `formalizable` | Current proof targets name parent state, but no committed parent-state transition theorem exists. | Define parent-state lookup, `apply_v2_block`, child state/reward roots, validation certificates, and atomic failure semantics; see `mvp_core_parent_state_transition_model.md`. | "Parent-state validation is still a proof target." |
| AD-015 | Zero-receipt or timeout fallback preserves consensus safety. | `implementation-dischargeable`, `formalizable` | The reviewed fallback object is specified in `mvp_core_fallback_liveness_model.md`, but it is not implemented as a v2 transition. | Add timeout/no-work evidence, validator rotation, reduced rewards, no miner TWU rewards, exact parent-state fallback validity predicate, and tests. | "Fallback is a documented liveness-only proof obligation, not current useful-PoW." |
| AD-015A | Verifier-dependent reward finality waits for challenges or direct recomputation. | `implementation-dischargeable`, `formalizable`, `evidence-dischargeable` | Local proposer rewards now use pending claims, challenge-window release, and block-check invalidation, but miner/validator reward finality, networked disputes, direct recomputation evidence, and public DA-through-window evidence remain incomplete. | Extend pending/challenged/invalidated/settled states to all verifier-dependent rewards, add networked challenge openings, deterministic resolution, DA-through-window evidence, and settlement tests. | "Block proposer reward finality is partially discharged locally; full reward finality remains open." |
| AD-016 | TensorWork no longer selects proposers. | `implementation-dischargeable` | The current proposer selector still uses settled TensorWork when total work is nonzero. | Remove or quarantine the v1 TensorWork proposer path from normal v2 block production; keep TensorWork only as reward/blockspace metric if retained. | "TensorWork proposer selection is superseded reference behavior." |
| AD-017 | LinearTrainingStep proves real-valued SGD. | `wording-guardrail`, `formalizable` | Current verifier proves finite-field algebraic relations only. | Either keep the claim out of scope or define fixed-point semantics, rounding, range bounds, overflow behavior, and an approximation theorem. | "Field-algebra training-shaped transition." |
| AD-018 | Local multi-participant evidence proves independent public operators. | `evidence-dischargeable`, `wording-guardrail` | Compose proves local shape with multiple identities, not independent operators. | Collect signed public operator attestations, disjoint ownership/hosting evidence, and public run observations. | "Local multi-participant shape." |
| AD-019 | Remote tensor fetch proves public DA. | `evidence-dischargeable`, `wording-guardrail` | A successful request-response fetch proves one matching payload retrieval at one time. | Add signed retention-window measurements from independent observers and define public reachability and durability thresholds. | "Root-matched verification-time remote fetch." |
| AD-020 | Coverage and local testnet gates prove protocol soundness. | `wording-guardrail` | Coverage and Gate 0 are regression/runtime evidence. | Keep them in evidence matrices as support for implemented behavior only; require theorem mapping for soundness claims. | "Regression evidence, not proof of consensus soundness." |

## Discharge Rules

1. Do not discharge cryptographic assumptions with tests. Tests can catch regressions in use of the primitive,
   but they do not prove collision resistance, unforgeability, or random-oracle behavior.
2. Do not discharge public DA or operator independence with local Compose evidence. Local evidence can prove
   runtime shape only.
3. Do not upgrade signed `Valid` statements to semantic verifier execution unless the statement is bound to
   recomputable, challengeable, or directly verified evidence.
4. Do not move a blocked v2 consensus theorem into the sound kernel until the implementation exposes the
   state object, adversarial tests cover it, and the traceability matrix maps it back to a theorem.
5. Do not treat wording guardrails as missing tests. Some claims are simply outside the current proof
   boundary and should remain excluded.
6. Every discharge must update `formal_proof_manifest_v0.md`, `mvp_core_proof_traceability_matrix.md`, and
   `bad_assumptions_ledger.md` in the same proof-review pass.

## Immediate Discharge Targets When Code Work Resumes

The next implementation work should discharge the assumptions that currently block the reviewed v2
consensus theorem:

1. Immutable receipt-lifecycle validation seed (`AD-005`).
2. Canonical settled-receipt blockspace and selected receipt root (`AD-010`).
3. Block-level checks root and recomputable check leaves (`AD-011`).
4. Useful-verification PoW header, target, nonce, and validation predicate (`AD-012`).
5. Proposer eligibility and removal of TensorWork proposer selection from v2 production (`AD-013`,
   `AD-016`).
6. Finality vote admission over `validate_block_v2` (`AD-014`).
7. Production signature model (`AD-009`).
8. Public DA and operator-independence evidence if those claims remain in scope (`AD-018`, `AD-019`).

## Current Judgment

The assumptions are now categorized, but the full core is still not sound. The defensible kernel remains the
finite-field verifier and syntactic chain-admission/settlement story under explicit assumptions. The
reviewed v2 consensus theorem remains blocked until the implementation makes the current counterexamples
impossible.
