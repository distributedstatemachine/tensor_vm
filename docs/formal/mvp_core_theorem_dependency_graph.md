# TensorVM MVP Core Theorem Dependency Graph

Status: documentation-only theorem dependency graph compiled from the current worktree.

Purpose: make proof imports explicit. The current proof corpus has theorem statements, assumptions,
negative cases, and v2 obligations; this document shows which claims depend on which other claims and where
the proof must stop until implementation or evidence catches up.

This is not a mechanized graph. It is the paper-proof import map that future Lean/TorchLean modules and
review checklists should follow.

## Current Verdict

The sound-kernel graph is acyclic and defensible only if its assumption leaves remain visible. The reviewed
v2 consensus graph is not closed: several required nodes are blocked or contradicted by the current
implementation.

The critical cut line is:

```text
SoundKernel -> verifier-local algebra + syntactic current-chain invariants + explicit assumptions
SoundKernel -/-> v2 finality, useful-verification PoW, public DA, production authentication
```

No current theorem may import a blocked v2 node and still be called part of the current sound kernel.

## Node Classes

| Class | Meaning | Examples |
| --- | --- | --- |
| `assumption-leaf` | A model assumption that can be named but not proved in this repository. | Hash collision resistance, random-oracle-like sampling, production signature unforgeability. |
| `proof-ready` | A theorem with stable statement and current Rust evidence, ready for mechanization. | Field determinism, matmul shape, Freivalds completeness, row-sampling probability. |
| `assumption-bound` | A theorem whose proof is meaningful only under named assumption leaves. | Freivalds soundness, random-linear soundness, receipt binding. |
| `syntactic-current-chain` | A theorem over current chain syntax, not semantic verifier truth or v2 consensus. | Attestation admission, quorum counting, v1 settlement. |
| `blocked-v2` | A target theorem whose required state or transition is missing. | Useful-verification PoW, canonical blockspace, finality implies v2 validity. |
| `evidence-bound` | A claim that requires external public evidence, not local proof. | Public DA and independent operators. |

## Sound-Kernel Dependency Graph

| Node | Class | Depends On | Must Not Import |
| --- | --- | --- | --- |
| `K-FIELD-001 tensor_ops_deterministic` | `proof-ready` | `A-RUST-EQ`, fixed field definition, tensor layout definition. | GPU equivalence, external framework semantics. |
| `K-MM-001 matmul_shape` | `proof-ready` | `K-FIELD-001`, tensor shape definitions. | Runtime test coverage as proof. |
| `K-MM-002 matmul_dot_associative_for_freivalds` | `proof-ready` | `K-FIELD-001`, `K-MM-001`, dot/vector orientation definitions. | Any floating-point semantics. |
| `K-FRV-001 freivalds_complete` | `proof-ready` | `K-MM-002`. | Randomness assumptions; completeness holds for every vector. |
| `K-FRV-002 freivalds_sound_one_round` | `assumption-bound` | `K-MM-002`, `A-RAND-UNIFORM`, `A-COMMIT-BEFORE-CHALLENGE`. | Deterministic all-cell correctness. |
| `K-FRV-003 freivalds_repeated_sound` | `assumption-bound` | `K-FRV-002`, `A-ROUND-INDEPENDENCE`. | Unstated independence between rounds. |
| `K-ROW-001 row_sample_detection_probability` | `proof-ready` | finite sampling without replacement. | Block eligibility unless parameter bounds are separately accepted. |
| `K-RLIN-001 random_linear_equal_complete` | `proof-ready` | tensor dot product over `K-FIELD-001`. | Real-valued approximation claims. |
| `K-RLIN-002 random_linear_equal_sound` | `assumption-bound` | nonzero linear polynomial lemma, `A-RAND-UNIFORM`, `A-COMMIT-BEFORE-CHALLENGE`. | Hash-derived randomness without a model. |
| `K-LIN-001 linear_training_step_complete` | `proof-ready` | `K-FIELD-001`, `K-MM-001`, `K-FRV-001`, `K-RLIN-001`, receipt metadata/root checks. | Real-valued SGD or convergence. |
| `K-COM-001 canonical_encoding_injective_before_hash` | `assumption-bound` | encoding definitions, `A-HASH-COLLISION-RESISTANCE` for hash binding. | Claim that Lean proves hash security. |
| `K-SIG-001 accepted_statement_has_valid_signature_under_Sig` | `assumption-bound` | signature relation, `A-SIG-UNFORGEABLE` if production actor control is claimed. | Current reference helper as production auth. |
| `K-ATT-001 submit_attestation_success_implies_registered_assigned_validator` | `syntactic-current-chain` | registry state, assignment function, `K-SIG-001`, receipt metadata match, duplicate-prevention rule. | Verifier execution semantics. |
| `K-QUO-001 quorum_implies_unique_assigned_valid_statement_weight` | `syntactic-current-chain` | `K-ATT-001`, unique-validator counting, threshold rule, data-availability bit. | Semantic truth of `Valid`. |
| `K-SET-001 settle_epoch_only_settles_quorum_agreed_receipts` | `syntactic-current-chain` | `K-QUO-001`, unavailable/conflict/redundant-agreement checks. | v2 block inclusion or challenge-window reward finality. |
| `K-DA-001 root_matched_fetch_inserts_matching_tensor` | `assumption-bound` | commitment-root computation, decode relation, `A-ARTIFACT-AVAILABLE`. | Public DA, durable retention, or independent hosting. |
| `SOUND-KERNEL-001 current_sound_kernel` | `assumption-bound` | all included `K-*` nodes plus visible assumption leaves. | Any `V2-*`, public DA, production auth, real SGD. |

## Assumption Leaves

| Leaf | Required By | Discharge Class |
| --- | --- | --- |
| `A-RUST-EQ` Rust/formal equivalence | All code-backed theorems. | Formalizable plus implementation conformance evidence. |
| `A-HASH-COLLISION-RESISTANCE` | Receipt/root binding, checks roots, PoW headers. | Permanent cryptographic assumption. |
| `A-RAND-UNIFORM` | Freivalds and random-linear soundness. | Permanent randomness model plus formal sampling definition. |
| `A-COMMIT-BEFORE-CHALLENGE` | Freivalds/random-linear soundness. | Implementation-dischargeable through receipt-lifecycle seed. |
| `A-ROUND-INDEPENDENCE` | Repeated probabilistic bounds. | Formalizable under domain separation and sampling assumptions. |
| `A-SIG-UNFORGEABLE` | Production actor-control claims. | Production crypto assumption plus implementation change. |
| `A-ARTIFACT-AVAILABLE` | Semantic verifier execution. | Implementation/evidence-dischargeable depending on claim. |
| `A-VALIDATOR-HONEST-OR-CHALLENGEABLE` | Quorum-to-semantics bridge. | Missing for current semantic quorum claims. |
| `A-PUBLIC-DA-EVIDENCE` | Public availability claims. | Evidence-dischargeable only. |
| `A-OPERATOR-INDEPENDENCE` | Public operator claims. | Evidence-dischargeable only. |

Assumption discharge rules live in
[`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md).
Verifier-local false-accept budgets live in
[`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md).
Receipt-lifecycle seed requirements live in
[`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md).
Signature/authentication boundaries live in
[`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md).
Canonical encoding and commitment boundaries live in
[`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md).
Settled-receipt blockspace boundaries live in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).
Useful-PoW work and economics boundaries live in
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).
Difficulty retarget boundaries live in
[`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md).
Verifier evidence boundaries live in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).
Parent-state transition boundaries live in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).
V2 state invariants live in
[`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md).

## Blocked V2 Dependency Graph

| Node | Class | Required Dependencies | Current Blocker |
| --- | --- | --- | --- |
| `V2-BLK-001 canonical_selected_receipts` | `blocked-v2` | settled-receipt pool, eligibility, expiry, caps, spent/carry-over, deterministic order, omission theorem. | Required blockspace lifecycle state and cap-policy theorem are missing. |
| `V2-BLK-002 selected_receipt_root_binding` | `blocked-v2` | `V2-BLK-001`, canonical selected receipt leaf encoding, hash binding. | Current/candidate roots do not yet bind full selected receipt leaves and lifecycle semantics. |
| `V2-CHK-001 check_leaf_recomputable` | `blocked-v2` | verifier kernel, artifact availability, transcript formats, challenge openings, verifier evidence model. | No committed semantic check leaf format or recomputation/challenge transition. |
| `V2-CHK-002 block_checks_root_binding` | `blocked-v2` | `V2-BLK-001`, `V2-CHK-001`, canonical check leaf order, verifier evidence model. | Aggregate roots over statements do not prove transcript truth without recomputation or challenge openings. |
| `V2-DIFF-001 parent_state_difficulty_target_valid` | `blocked-v2` | difficulty retarget model, parent `DifficultyState`, bounded retarget window, target bounds, hash-to-target encoding, work floor. | Paper model exists; difficulty state and validation are not implemented. |
| `V2-POW-001 useful_verification_pow_valid` | `blocked-v2` | `V2-BLK-002`, `V2-CHK-002`, `V2-DIFF-001`, nonce, beacon, proposer, hash model, useful-work cost model for economic claims. | Current block has no target/nonce and no useful-PoW predicate; work dominance is also unmodeled. |
| `V2-PROP-001 proposer_eligible` | `blocked-v2` | validator registry, `V2-POW-001`, removal of TensorWork proposer path. | Current path can use caller-supplied proposer or TensorWork selection. |
| `V2-STATE-001 valid_v2_block_transition` | `blocked-v2` | `V2-BLK-*`, `V2-CHK-*`, `V2-POW-001`, parent-state transition model, reward/state root transition. | No v2 parent-state apply transition or child-root theorem. |
| `V2-REWARD-001 delayed_reward_finality` | `blocked-v2` | reward finality model, challenge openings, DA-through-window assumption, pending/challenged/settled reward state, clawback transition. | Local pending reward ledgers, maturity release, voiding, and nonpayment/prune transitions exist for implemented paths; full theorem remains blocked by verifier-transcript challenges, DA-through-window evidence, public dispute propagation, and proof discharge. |
| `V2-FIN-001 vote_admission_requires_validate_block_v2` | `blocked-v2` | complete `validate_block_v2(parent_state, block)`, stake/signature vote checks. | Current votes do not have a committed parent-state validation certificate. |
| `V2-FIN-002 finality_implies_v2_block_valid` | `blocked-v2` | `V2-FIN-001`, `V2-STATE-001`, stake threshold theorem, finality certificate model. | Current finality can certify a v1/reference block hash or lack parent-state proof. |
| `V2-FALLBACK-001 pow_skip_fallback_valid` | `blocked-v2` | fallback liveness model, timeout/synchrony model, validator rotation, parent-state fallback validation, reduced rewards, no miner TWU rewards. | Paper model exists; v2 fallback object is not implemented. |

The v2 graph has no honest path to a completed top-level theorem until every row above is backed by code,
tests, assumptions, and traceability.

## Forbidden Edges

These imports would create false proof claims:

| Forbidden Edge | Why It Is Invalid Today |
| --- | --- |
| `SOUND-KERNEL-001 -> V2-FIN-002` | The sound kernel is current verifier/chain syntax; v2 finality is blocked. |
| `K-QUO-001 -> semantic verifier execution` | Current quorum counts signed statements, not recomputed verifier transcripts. |
| `K-DA-001 -> public DA` | Root-matched retrieval is not public retention or independent serving. |
| `K-SIG-001 -> production authentication` | The reference signature relation is not production unforgeability. |
| `K-LIN-001 -> real-valued SGD correctness` | Current arithmetic is finite-field and has no fixed-point bridge theorem. |
| `V2-POW-001 -> current TensorBlock` | Current blocks lack the required witness fields. |
| `block.difficulty_target -> target_valid` | A candidate-supplied or static target does not prove parent-state difficulty validity. |
| `V2-PROP-001 -> settled TensorWork` | The reviewed v2 spec excludes TensorWork from proposer selection. |
| `V2-FIN-002 -> current submit_block_vote` | Current vote admission does not require v2 block validation. |
| `valid nonce -> useful-work dominance` | A nonce proves hash-target success, not that verification work dominated nonce grinding. |
| `aggregate checks_root -> verifier execution` | A root over signed check claims does not prove the verifier relation without transcript recomputation or challenge openings. |
| `current-state validation -> parent-state validity` | A block must be validated against its exact parent state, not whatever mutable state exists when a node checks it. |
| `settled_receipt_ids -> canonical blockspace` | A bare id set lacks eligibility, cap accounting, selected leaf, spent/carry-over, expiry, and omission semantics. |
| `fallback_block -> useful-PoW` | A fallback block proves only liveness under timeout/no-work assumptions; it must not import useful-work dominance or normal rewards. |
| `block_finality -> reward_finality` | Block ordering finality can create pending claims; verifier-dependent rewards still need direct recomputation or challenge-window settlement. |

## Mechanization Import Rule

Future mechanized modules should preserve this split:

```text
SoundKernel.lean imports:
  Field, Tensor, Matmul, RandomOracle assumptions, Freivalds, RandomLinear,
  LinearStep, Encoding assumptions, Signature relation, Attestation syntax,
  Settlement v1 syntax, DataAvailability root-match.

SoundKernel.lean does not import:
  BlockedConsensus, v2 finality, useful-PoW, public DA, operator independence,
  production authentication, real-valued SGD bridge.
```

If a theorem needs a blocked node, it belongs in `BlockedConsensus.lean` or a future v2 module, not in the
current sound kernel.

## Upgrade Rule

When an implementation change claims to unblock a v2 node, the proof review must update all of:

1. This dependency graph.
2. `formal_proof_manifest_v0.md`.
3. `mvp_core_proof_traceability_matrix.md`.
4. `mvp_core_v2_consensus_proof_obligations.md`.
5. `mvp_core_negative_proofs.md`, removing or revising any counterexample that no longer constructs.
6. `bad_assumptions_ledger.md` and `mvp_core_assumption_discharge_plan.md`.

If any of these still classify the node as blocked or contradicted, the top-level v2 theorem remains
unproved.

## Current Judgment

The dependency graph supports a narrow sound kernel and blocks the tempting but wrong import from current
finality to useful-verification PoW. The MVP core is still not fully sound: the graph for reviewed v2
consensus has missing implementation nodes, and public operation claims remain evidence-bound.
