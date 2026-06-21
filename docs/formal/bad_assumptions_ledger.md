# TensorVM Bad Assumptions Ledger

Status: documentation-only risk ledger compiled from the current worktree.

Purpose: record assumptions that would make TensorVM soundness claims misleading if left implicit. This is
not a code change and not a mechanized proof. It is a guardrail for future specs, proofs, release notes, and
implementation work.

Uncommitted or build-failing candidate implementation work is not treated as discharged proof evidence in
this ledger. The local v2-block reference path is tracked separately in
[`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md).

## Severity Legend

| Severity | Meaning |
| --- | --- |
| Critical | Makes the reviewed MVP consensus claim false if asserted today. |
| High | Can make verifier, reward, or availability claims materially misleading. |
| Medium | Can produce wrong expectations or weak local evidence if not qualified. |

## Ledger

| ID | Bad Assumption | Severity | Current Evidence | Why It Is Unsound | Allowed Claim Today | Proof/Implementation Gate |
| --- | --- | --- | --- | --- | --- | --- |
| BA-001 | "The current chain already implements useful-verification PoW." | Critical | Partially discharged locally: `TensorBlock` now has `settled_receipt_set_root`, `checks_root`, `beacon`, `difficulty_target`, and `nonce`; `chain::blocks::produce` mines a nonce and block votes validate known blocks with strict parent-root checks. | The remaining proof target still needs exact parent-state snapshots, child-state apply semantics, difficulty retargeting, and challenge openings. | The current chain implements a local useful-verification PoW reference path, not a complete production consensus proof. | Add exact parent snapshots/apply semantics, settled-receipt lifecycle metadata, retargeting, and `checks_root` opening challenges. |
| BA-002 | "Settled TensorWork proposer selection is close enough to the reviewed MVP." | Critical | Discharged in the chain core: `chain::proposer` selects registered validators and ignores miner `settled_tensor_work`; miner-only block production is rejected. | Runtime topology still has a transitional single local block driver, so live validator proposer networking is not proved. | TensorWork no longer selects proposers in the shared chain engine. | Move timed local block driving from deterministic replay into live validator proposer networking. |
| BA-003 | "Finality implies useful-PoW block validity." | Critical | Partially discharged locally: `submit_block_vote` calls strict block validation before accepting votes, including state root, beacon, PoW target, proposer, selected-root, and checks-root tests. | Validation reconstructs a parent-like view from local state and selected-receipt metadata; it is not yet an exact persisted parent snapshot or child-state transition proof. | Finality votes are gated by local block validity checks for known blocks in the current reference path. | Persist/replay the exact parent state used to build the block and prove the child-state apply transition. |
| BA-004 | "A receipt map root is canonical blockspace." | Critical | Partially discharged locally: blocks commit `settled_receipt_set_root` from deterministic settled-receipt selection with count/TWU/byte caps, record selected receipts as local evidence, and mark selected receipts included once. | The selected root currently lacks full selected-leaf metadata for expiry, challenge-window state, carry-over, and proof-openable lifecycle semantics. | The block root now commits the local canonical selected receipt set. | Add settled-receipt pool metadata and parent-state selected-leaf encoding. |
| BA-005 | "Per-receipt checks roots prove the block proposer verified work." | Critical | Partially discharged locally: blocks commit an aggregate block-level `checks_root` recomputed from valid receipt attestations and votes reject mismatches. | Challenge openings and transcript-level dispute rewards are not implemented. | Block headers bind the local aggregate checks evidence. | Define check leaves, opening payloads, dispute rules, and challenge/reward integration. |
| BA-006 | "Freivalds proves every output cell." | High | `full_freivalds` is randomized and returns a boolean from sampled vectors. | Freivalds is probabilistic; false acceptance is bounded, not impossible. | Freivalds bounds false acceptance under hidden uniform challenges. | Mechanize Freivalds soundness and state sampling/randomness assumptions. |
| BA-007 | "Row sampling is good enough for block eligibility." | High | `row_sample_detection_probability` shows sparse-row detection can be low. | Sparse corruptions can evade small samples with high probability. | Row sampling is audit telemetry unless explicit bounds meet the target. | Require full-output Freivalds or a documented parameterized row-sampling bound per job shape. |
| BA-008 | "Hash-derived randomness is automatically unbiasable." | High | Validation seed uses finalized randomness and receipt id; assignment admission currently recomputes from current finalized randomness. | Hashing a biased or late-changing source does not make it unbiasable. Delayed attestations can be evaluated against the wrong lifecycle seed if the seed is not receipt-locked. | Randomness is deterministic from current chain state, with explicit beacon/commitment assumptions. | Store or derive receipt-lifecycle validation seed at receipt admission; prove miners commit before challenge derivation; see `mvp_core_receipt_lifecycle_seed_model.md`. |
| BA-009 | "Reference signatures prove production authentication." | High | `types::sign` is `hash(address, message)` and verification recomputes the hash. | This is a test helper, not a real signature scheme or key-ownership proof. | The reference core tests signature-flow plumbing. | Use a production signature scheme and state unforgeability, key custody, and replay-domain assumptions; see `mvp_core_signature_authentication_boundary.md`. |
| BA-010 | "LinearTrainingStep proves real ML training." | High | Current verifier checks finite-field algebraic relations. | Field arithmetic can model a training-shaped transition without proving real-valued SGD approximation or convergence. | LinearTrainingStep proves deterministic field-algebra transition consistency. | Define fixed-point scale, rounding, overflow/range bounds, and bridge theorem to real-valued SGD if that claim is desired. |
| BA-011 | "Local tensor serving proves public data availability." | High | Local tests/checkers can fetch descriptors, rows, chunks, and openings. | Local fetches do not prove independent retention or public availability over active/retention windows. | Local serving proves the code path and verification-time availability in local runs. | Add public DA measurement evidence with signed observations over retention windows. |
| BA-012 | "Local Compose operators are independent." | High | Compose starts multiple containers with separate identities and roles. | Separate local participants are not independent economic/security principals. | Compose proves multi-participant local shape, not public operator independence. | Require external operator attestations, disjoint ownership evidence, and public run evidence. |
| BA-013 | "Synthetic tensor jobs prove useful work." | Medium | Local synthetic job source emits deterministic matmul and LinearTrainingStep jobs. | Synthetic jobs prove verifiable compute rails, not externally demanded usefulness. | Synthetic jobs prove verifiable deterministic workload handling. | Separate synthetic-work metrics from user-valued workload metrics. |
| BA-014 | "High line coverage proves protocol soundness." | Medium | Tarpaulin reports strong line coverage for reference code. | Coverage proves lines executed, not that threat-model properties or consensus theorems hold. | Coverage is regression evidence for implemented behavior. | Tie every soundness claim to a theorem, assumption, adversarial test, or external evidence item. |
| BA-015 | "A clean local testnet gate means the MVP is complete." | Medium | Gate 0 exercises CPU local multi-participant paths. | Gate 0 is necessary local evidence, but not public-run, useful-PoW, DA, slashing, or independent-operator evidence. | Gate 0 is the first local acceptance gate. | Complete v2 consensus proof surface and public evidence gates before claiming full MVP completion. |
| BA-016 | "Produced blocks imply proposer eligibility." | Critical | Locally discharged for registered-validator eligibility: `produce_block` is fallible, rejects unknown validators, and vote admission validates the block proposer. | The local path still does not prove live network proposer admission or full useful-PoW economics. | Current block production is a validator-gated reference path. | Tie live proposer networking and block admission to the complete v2 useful-PoW predicate. |
| BA-017 | "A Valid attestation means the verifier actually ran." | High | `submit_attestation` accepts an assigned validator's signed `Valid` statement and `checks_root`; it does not recompute tensor verification. | Quorum is syntactic unless verifier evidence is independently bound, recomputed, or challengeable. | Current quorum proves assigned validators signed matching Valid/DataAvailable statements. | Bind attestations to recomputable check leaves, block-level `checks_root`, and challenge openings or direct verification evidence. |
| BA-018 | "Remote tensor fetch proves public data availability." | High | Current worktree adds root-addressed request-response fetches and validator remote-fetch counters. | A successful fetch proves one runtime retrieved a matching tensor at one time; it does not prove durable retention, public reachability, or independent hosting. | Remote fetch is verification-time artifact availability evidence. | Add signed public DA measurements across active/challenge windows and prove enough independent operators serve required artifacts. |
| BA-019 | "One Freivalds round over the current 31-bit field is cryptographic-scale soundness." | High | The current field is `2^31 - 1`, default `full_rounds` is `1`, and LinearTrainingStep has two single random-linear equality checks. | The default per-relation false-accept budget is about `1 / 2^31`, and receipt-volume union bounds can make that too weak for broad consensus claims. Validator quorum cannot be multiplied in while quorum is syntactic. | Current verifier checks have explicit finite-field probabilistic bounds under assumptions. | Set a target soundness budget, receipt-volume limit, round count, and random-linear repetition story; see `mvp_core_probabilistic_soundness_budget.md`. |
| BA-020 | "A hash root automatically proves the intended consensus object." | High | Current roots deterministically hash encoded state and block objects, including local selected receipt and aggregate check roots. | A root binds only the bytes it encodes under a hash assumption; it does not prove eligibility, selection, verifier transcripts, or v2 state transitions unless those fields are encoded with the right lifecycle semantics. | Current roots are deterministic commitments to their encoded objects. | Specify canonical leaf schemas, prove pre-hash injectivity, import hash collision resistance, and complete selected receipt/check opening semantics; see `mvp_core_canonical_encoding_commitment_model.md`. |
| BA-021 | "A nonce over a verification commitment proves useful-work dominance." | High | Current blocks have a local nonce/target predicate, but it proves only hash success unless transcript cost, target validity, and sharing/caching assumptions are modeled. | A valid nonce can be ordinary PoW over bytes. It does not prove verification work dominated nonce grinding, or that the winning proposer personally performed the verification. | Useful-verification PoW is a target theorem plus an economic assumption. | Validate target/difficulty from parent state, bind checks to selected receipt leaves, and document cost parameters; see `mvp_core_useful_pow_work_model.md`. |
| BA-022 | "Aggregating signed checks roots proves verifier execution." | High | Attestations can carry `checks_root` values, and block-level roots can aggregate those values, but the chain still needs recomputation or challenge openings to prove transcript truth. | A root over signed claims is still a root over claims. It does not prove the underlying verifier relation unless every leaf is recomputable, directly verified, or challengeable. | Aggregated check roots can be evidence commitments, not semantic verifier-execution proof. | Define `CheckLeaf`, transcript, opening, challenge-window, and reward-finality semantics; see `mvp_core_verifier_evidence_model.md`. |
| BA-023 | "A v2-shaped candidate implementation discharges the proof obligations." | Critical | The local v2-block path is build-clean and adds useful fields, strict vote validation, included-receipt exclusion, validator proposer checks, parent snapshots, delayed rewards, and local fallback evidence, but still lacks full semantic transcript disputes, difficulty economics, public/CUDA evidence, and theorem discharge. | Build-clean partial code can be useful evidence, but it cannot prove consensus soundness until every theorem gate is discharged. | The candidate is directionally aligned but not proof-sound. | Add adversarial tests and discharge each theorem gate; see `mvp_core_candidate_v2_block_audit.md`. |
| BA-024 | "Validating a block against current state proves it was valid for its parent." | Critical | The proof target requires `valid_v2_block(parent_state, block)`, while candidate validation patterns can recompute roots from mutable chain state unless an exact parent snapshot is modeled. | Current-state validation can accept or reject based on later receipts, attestations, rewards, randomness, or direct state mutation. It does not prove the block transition from its parent. | Parent-state validation is a required theorem, not an implementation detail. | Define parent-state lookup, `apply_v2_block`, child roots, finality certificates, and atomic failure semantics; see `mvp_core_parent_state_transition_model.md`. |
| BA-025 | "A settled receipt id set is canonical blockspace." | Critical | Current state has settled ids plus an included-receipt exclusion set and local block-selected receipt metadata, but the v2 theorem still needs expiry, DA-through-challenge-window status, cap accounting leaves, and carry-over semantics. | A set of ids does not prove why receipts are eligible, why omitted receipts were omitted, or how much block capacity they consumed. | Settled ids plus local inclusion tracking are inputs to the blockspace selector, not the complete theorem. | Define the settled receipt pool, selection policy, selected leaf schema, carry-over/spent rules, and omission theorem; see `mvp_core_settled_receipt_blockspace_model.md`. |
| BA-026 | "A zero-receipt or timeout block is useful-verification PoW." | Critical | The local reference has a `PowSkipFallback` implementation with timeout, deterministic validator rotation, reduced delayed rewards, and focused tests, but full liveness proof and public/multi-validator evidence remain blocked. | Fallback preserves liveness when useful-PoW is unavailable; it does not prove verification work, cannot reuse normal useful-PoW rewards, and cannot skip parent-state validation. | Fallback is a separate liveness-only proof obligation. | Discharge synchrony/liveness assumptions, multi-validator runtime evidence, public/CUDA proof, and parent-state validation theorem; see `mvp_core_fallback_liveness_model.md`. |
| BA-027 | "Block finality means reward finality." | Critical | The v2 spec separates finalized block ordering from delayed verifier reward settlement; the local reference now has pending receipt/proposer/challenge/credit claims, voiding, clawback/nonpayment, and maturity release for implemented paths, but not the full challenge-window theorem. | A finalized block can still contain wrong `checks_root` evidence or unavailable artifacts. Paying spendable rewards before direct recomputation or challenge-window finality makes clawback impossible. | Finalized blocks may create pending reward claims only. | Complete challenge openings, interactive transcript disputes, DA-through-window assumptions, public evidence, and settlement proofs; see `mvp_core_reward_finality_challenge_model.md`. |
| BA-028 | "A block target is valid because the nonce is below it." | Critical | The local v2-shaped path uses a target helper, and the proof target has no discharged parent-state difficulty, retarget, or target-bound theorem. | A proposer can satisfy a hash predicate against an invalid, trivial, or locally configured target unless the target is derived from parent consensus state. | A valid nonce proves only hash-target success against whatever target was validated. | Add versioned difficulty state, bounded retargeting, hardest/easiest target bounds, canonical hash-to-target semantics, and work-floor rules; see `mvp_core_difficulty_retarget_model.md`. |

## Non-Negotiable Wording Rules

Use these rules in specs, README text, release notes, and investor-facing summaries:

1. Say **probabilistically verified** unless every relevant tensor output is fully recomputed or succinctly
   proven.
2. Say **local reference block production** for the current block path, not useful-verification PoW.
3. Say **superseded TensorWork proposer path** for `chain::proposer` until v2 block production replaces it.
4. Say **verification-time availability** for local tensor serving, not durable DA.
5. Say **field-algebra training step**, not real-valued training, unless fixed-point bridge theorems exist.
6. Say **reference signature helper**, not production authentication, until production crypto is wired.
7. Say **local multi-participant shape**, not independent operators, for Compose.
8. Say **signed Valid/DataAvailable statements**, not verified work, when referring only to current
   attestation quorum.
9. Say **verification-time remote fetch**, not public DA, for root-addressed request-response tensor
   retrieval.
10. Say **PoW over a verification commitment** unless the useful-work dominance model is discharged with
    target, cost, and transcript-sharing assumptions.
11. Say **evidence commitment** for an aggregate checks root until verifier transcripts are recomputable or
    challengeable.
12. Say **local v2 block reference path** for the current build-clean partial implementation, not completed
    v2 consensus.
13. Say **parent-state validation** when discussing v2 block validity; do not treat current-state root
    recomputation as equivalent.
14. Say **settled receipt ids** unless eligibility, cap, carry-over, and selected-leaf semantics are
    actually represented.
15. Say **PoW-skip fallback** or **liveness fallback**, not useful-verification PoW, for zero-receipt,
    below-floor, or timeout blocks.
16. Say **pending reward claim** until direct recomputation or challenge-window finality makes the reward
    spendable.
17. Say **parent-state-derived target** when discussing PoW difficulty; do not treat a block-supplied or
    static local target as production retargeting.

## Claims We Can Make Today

These are defensible with current docs/code evidence:

- The verifier layer has proof-ready algebraic obligations for TensorOp and LinearTrainingStep.
- Freivalds and random-linear checks provide probabilistic soundness under explicit randomness and
  commitment assumptions.
- Row sampling is documented as weak for sparse corruption and should be treated as audit coverage.
- Chain attestation admission now enforces registered stake, signature validity, receipt metadata,
  deterministic assignment, and duplicate prevention.
- Current documentation clearly records that v2 useful-verification PoW consensus is not complete.
- Current attestation quorum proves syntactic assigned-validator agreement, not independent recomputation of
  verifier transcripts.
- Current root-addressed remote tensor fetch can prove payload/root matching for verifier use, not public DA.
- Current default probabilistic verifier bounds are finite-field proof budgets, not cryptographic-scale
  consensus soundness across unbounded receipt volume.
- Current roots commit their encoded objects under hash assumptions; they do not automatically commit the
  reviewed v2 consensus object.
- A future useful-PoW theorem must separately prove structural header validity and state the economic
  work-dominance assumption.
- Semantic verifier execution requires recomputable or challengeable evidence, not only signed Valid bits.
- The local v2-block reference path demonstrates implementation direction but not proof completion.
- Finality soundness requires validation against the exact parent state and deterministic child roots.
- Canonical blockspace requires a settled receipt lifecycle model, not only a set of ids.
- Zero-receipt and no-PoW liveness require a separate fallback transition; fallback is not useful work.
- Reward finality is a separate delayed transition from block finality.
- Difficulty validity requires parent-state target derivation, not only a nonce below a target field.

## Claims We Must Not Make Today

These would overstate the current MVP:

- "The MVP core is formally proven sound."
- "The chain implements useful-verification proof of work."
- "Finalized blocks prove validators verified the canonical receipt set."
- "TensorWork no longer affects proposer eligibility in implementation."
- "Local Compose evidence proves public independent operators."
- "Local tensor fetches prove durable data availability."
- "LinearTrainingStep proves real SGD correctness."
- "A finalized current block proves its proposer was eligible under v2 rules."
- "A quorum of Valid attestations proves validators actually executed the verifier."
- "Remote tensor fetch counters prove public data availability."
- "One default Freivalds round over the 31-bit field is enough for all production consensus volume."
- "A current receipt/state root proves v2 canonical blockspace or block-level verification checks."
- "A nonce over `checks_root` proves useful-work dominance."
- "An aggregate root over signed `checks_root` values proves validators executed the verifier."
- "The local v2-block reference path proves the MVP core sound."
- "A block validated against current mutable state was valid for its parent state."
- "A settled receipt id root proves canonical blockspace."
- "A zero-receipt, below-floor, or timeout block proves useful-verification PoW."
- "Finalized blocks have already paid irreversible verifier-dependent rewards."
- "A nonce below a block-supplied or static local target proves production difficulty validity."

## Proof Hygiene Checklist

Before upgrading any `implementation-blocked` item to `local-proof-ready`, require all of:

1. The theorem statement is written in `formal_proof_manifest_v0.md`.
2. The exact Rust transition or verifier surface exists.
3. Tests cover honest and adversarial cases for that surface.
4. Every cryptographic/randomness/economic dependency is listed as an assumption.
5. The relevant docs stop using superseded v1 evidence as v2 proof.
6. The claim is phrased with probabilistic bounds when randomness is involved.

## Current Judgment

The current proof docs are useful and materially more honest than the earlier MVP wording. The core is still
not sound enough for the full reviewed v2 MVP because block production and finality are still tied to the old
consensus object.

This ledger should remain open until useful-verification PoW, canonical settled-receipt blockspace, and v2
block finality validation are implemented and mapped back into the formal proof manifest.

Assumptions should be discharged only through the category-specific gates in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md); tests, local runs, and
wording changes do not discharge cryptographic, public-evidence, or missing-implementation assumptions by
themselves.
