# TensorVM Formal Proof Manifest v0

Status: documentation-only manifest compiled from the current worktree.

This file is the proof inventory for the MVP core. It is intentionally blunt: a theorem is either backed by
current code and tests, blocked by an explicit assumption, or not currently provable because the consensus
object is missing.

This is not a Lean/TorchLean artifact. It is the manifest that a later mechanized proof package should be
checked against.

Related boundary documents:

- [`mvp_core_proof_completion_audit.md`](mvp_core_proof_completion_audit.md) audits the formal-proof goal
  requirement by requirement and records the current completion verdict.
- [`mvp_core_candidate_v2_block_audit.md`](mvp_core_candidate_v2_block_audit.md) audits the local
  v2-block reference path and records why it does not yet discharge consensus proof obligations.
- [`mvp_core_proof_traceability_matrix.md`](mvp_core_proof_traceability_matrix.md) maps current proof claims
  to Rust surfaces, evidence classes, allowed wording, and upgrade gates.
- [`mvp_core_assumption_discharge_plan.md`](mvp_core_assumption_discharge_plan.md) classifies proof
  assumptions by whether they are permanent, mechanizable, implementation-dischargeable, evidence-bound, or
  wording guardrails.
- [`mvp_core_adversary_model.md`](mvp_core_adversary_model.md) states the adversary capabilities and proof
  worlds that current claims are measured against.
- [`mvp_core_theorem_dependency_graph.md`](mvp_core_theorem_dependency_graph.md) shows which theorem nodes
  depend on which assumptions and where blocked v2 consensus imports must stop.
- [`mvp_core_canonical_encoding_commitment_model.md`](mvp_core_canonical_encoding_commitment_model.md)
  separates pre-hash canonical encoding, hash/Merkle binding, and consensus-object meaning.
- [`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md) defines the
  settled receipt lifecycle, cap policy, selected leaf, and carry-over rules needed for v2 blockspace.
- [`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md) separates structural
  useful-PoW header validity from economic useful-work dominance.
- [`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md) defines parent-state
  target validity, bounded retargeting, and hash-to-target comparison semantics.
- [`mvp_core_fallback_liveness_model.md`](mvp_core_fallback_liveness_model.md) specifies the zero-receipt
  and no-PoW fallback path as a separate liveness-only transition, not useful-PoW.
- [`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md) defines the missing
  recomputable/challengeable evidence bridge from signed Valid statements to semantic verifier execution.
- [`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md) defines
  delayed reward finality, challenge resolution, and clawback/nonpayment obligations for verifier-dependent
  rewards.
- [`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md) defines the
  required parent-state validation and child-state transition theorem for v2 finality.
- [`mvp_core_probabilistic_soundness_budget.md`](mvp_core_probabilistic_soundness_budget.md) records the
  verifier-local false-accept budgets and the composition rules for receipt volume.
- [`mvp_core_receipt_lifecycle_seed_model.md`](mvp_core_receipt_lifecycle_seed_model.md) defines the seed
  stability theorem needed for non-adaptive Freivalds and random-linear challenges.
- [`mvp_core_signature_authentication_boundary.md`](mvp_core_signature_authentication_boundary.md) separates
  the current reference signature relation from production authentication claims.
- [`mvp_core_v2_state_invariants.md`](mvp_core_v2_state_invariants.md) defines the state invariants that
  must be preserved before finality can imply v2 block validity.
- [`mvp_core_mechanization_checklist.md`](mvp_core_mechanization_checklist.md) maps the sound-kernel
  theorem set to future Lean/TorchLean modules, assumptions, and proof dependencies.
- [`mvp_core_v2_consensus_proof_obligations.md`](mvp_core_v2_consensus_proof_obligations.md) defines the
  blocked v2 consensus theorem spine and the objects required before those theorems can be proved.
- [`mvp_core_sound_kernel.md`](mvp_core_sound_kernel.md) defines the narrow proof kernel that is defensible
  today.
- [`mvp_core_data_availability_boundary.md`](mvp_core_data_availability_boundary.md) separates
  verification-time tensor retrieval from public data availability.
- [`mvp_core_negative_proofs.md`](mvp_core_negative_proofs.md) records current counterexamples to the full
  reviewed v2 consensus theorem.

## Status Legend

| Status | Meaning |
| --- | --- |
| `local-proof-ready` | The property has a clear theorem statement, deterministic Rust semantics, and local tests/evidence that make it ready to mechanize. |
| `assumption-bound` | The theorem can be stated only under explicit cryptographic, randomness, availability, or economic assumptions. |
| `implementation-blocked` | Current code does not expose the state or transition needed to state the theorem honestly. |
| `reference-only` | Current behavior is tested and useful locally but belongs to the superseded v1 model, not the reviewed v2 MVP. |
| `not-started` | The required proof surface is not present. |

## Manifest Summary

| Area | Current Status | Bottom Line |
| --- | --- | --- |
| TensorOp verifier | `local-proof-ready` plus `assumption-bound` soundness | Freivalds proof path is clear, but probabilistic and randomness-bound. |
| LinearTrainingStep verifier | `local-proof-ready` plus `assumption-bound` soundness | Algebraic field transition is proof-ready; real-valued ML meaning is not proven. |
| Row sampling | `local-proof-ready` as audit math | Useful as audit probability, unsafe as sole block validity. |
| Receipt binding | `assumption-bound` | Canonical encodings can be specified; cryptographic hash binding is assumed. |
| Attestation admission | `local-proof-ready` for syntactic admission, `assumption-bound` for semantic verification | Assigned-validator admission is now in the chain engine; it does not prove the verifier actually ran, and receipt-lifecycle seed stability remains weak. |
| Settlement | `local-proof-ready` for v1 syntactic-quorum settlement | Settlement/quorum behavior is testable, but semantic verifier execution and v2 blockspace pool semantics are missing. |
| Useful-verification PoW | `implementation-blocked` | Local block fields, parent snapshots, delayed rewards, and Docker evidence exist, but the full reviewed v2 consensus theorem still lacks interactive transcript disputes, public/CUDA evidence, and complete theorem discharge. |
| Difficulty retargeting | `implementation-blocked` | Parent-state target derivation, bounded retargeting, target bounds, and hash-to-target vectors are not implemented. |
| Fallback liveness | `implementation-blocked` | The local reference has `PowSkipFallback` timeout, stake-weighted rotation, reduced delayed rewards, and tests; full liveness proof, multi-validator runtime evidence, and public/CUDA evidence remain blocked. |
| Reward finality | `implementation-blocked` | Local pending receipt/proposer/challenge/credit ledgers, clawback/voiding, maturity sweep, and tests exist; full challenge-window theorem discharge and public deployed evidence remain blocked. |
| Finality | `reference-only` | Stake-threshold finality exists for current blocks, not for v2 useful-PoW validity. |
| Verification-time artifact retrieval | `local-proof-ready` for root-matched local fetch, `assumption-bound` for availability | Current worktree can check fetched tensor roots before verifier use; public DA is not proven. |
| Public availability/operator independence | `assumption-bound` | Local and request-response fetches do not prove public DA or independent operators. |

## Core Theorems

### TVM-ALG-001: Canonical Field Tensor Determinism

Statement:

```text
For a fixed dtype, shape, layout, and field element vector, TensorVM tensor operations used by consensus are
deterministic.
```

Status: `local-proof-ready`

Rust evidence:

- `crates/tensor_vm/src/tensor.rs`
- `crates/tensor_vm/src/field.rs`
- `crates/tensor_vm/src/vm.rs`
- Runtime parity tests referenced by `docs/tensorvm/coverage_matrix.md`

Formal artifact needed:

```text
formal/TensorVM/Field.lean
formal/TensorVM/Tensor.lean
theorem tensor_ops_deterministic
```

Bad assumption to reject:

```text
GPU execution is canonical because CPU/GPU tests pass locally.
```

Correct boundary:

GPU kernels can accelerate mining. Consensus validity is canonical CPU/field semantics unless a GPU kernel
is separately proven equivalent.

### TVM-MM-001: Matmul Shape Correctness

Statement:

```text
A.shape = [m,k] and B.shape = [k,n] imply matmul(A,B).shape = [m,n].
```

Status: `local-proof-ready`

Rust evidence:

- `Tensor::matmul`
- Tensor shape checks in `verify_tensor_op`
- Tensor and verifier tests listed in `docs/tensorvm/coverage_matrix.md`

Formal artifact needed:

```text
theorem matmul_shape
```

### TVM-FRV-001: Freivalds Completeness

Statement:

```text
C = A @ B -> full_freivalds(A,B,C,seed,rounds) = true
```

Status: `local-proof-ready`

Rust evidence:

- `crates/tensor_vm/src/verify.rs::full_freivalds`
- `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption`

Formal artifact needed:

```text
formal/TensorVM/Freivalds.lean
theorem freivalds_complete
```

Proof notes:

The algebraic proof is direct: for every sampled vector `r`, `A(Br) = (AB)r = Cr`.

### TVM-FRV-002: Freivalds Soundness

Statement:

```text
C != A @ B -> Pr_r[full_freivalds_round(A,B,C,r) = true] <= 1 / |F|
```

Status: `assumption-bound`

Rust evidence:

- `full_freivalds`
- `study::freivalds_security`
- corruption tests in `verify.rs`

Formal artifact needed:

```text
theorem freivalds_sound_one_round
theorem freivalds_repeated_sound
```

Assumptions:

- Hash-derived `random_field_vector` is modeled as uniform over the field.
- The miner cannot choose or mutate `C` after seeing the validation seed.
- Domain-separated hash sampling has no exploitable bias for the protocol parameters.

Bad assumption to reject:

```text
Freivalds deterministically proves all matrix cells are correct.
```

Correct claim:

Freivalds gives a false-accept probability bound.

### TVM-ROW-001: Row Sampling Detection Probability

Statement:

```text
corruption in t rows, s sampled rows without replacement:
P_detect = 1 - choose(m - t, s) / choose(m, s)
```

Status: `local-proof-ready`

Rust evidence:

- `row_sample_detection_probability`
- `verify::tests::row_sampling_probability_exposes_sparse_weakness`
- `study::tests::row_sampling_study_blocks_sparse_row_sampled_only_acceptance`

Formal artifact needed:

```text
theorem row_sample_detection_probability
```

Bad assumption to reject:

```text
Row sampling is equivalent to full-output Freivalds for block eligibility.
```

Correct claim:

Row sampling is audit evidence unless configured probability bounds are explicitly strong enough.

### TVM-LIN-001: LinearTrainingStep Algebraic Completeness

Statement:

```text
Y = XW
dY = Y - T
G = X^T dY
W_next = W - lr * G
receipt binds Y, dY, G, W_next
->
verify_linear_training_step(...).result = Valid
```

Status: `local-proof-ready`

Rust evidence:

- `crates/tensor_vm/src/verify.rs::verify_linear_training_step`
- `crates/tensor_vm/src/vm.rs`
- `vm::tests::linear_backward_and_sgd_match_equations`
- `jobs::tests::linear_receipt_commits_to_learning_step`
- `verify::tests::linear_training_verifier_rejects_metadata_and_commitment_mismatches`

Formal artifact needed:

```text
formal/TensorVM/LinearStep.lean
theorem linear_training_step_complete
```

Bad assumption to reject:

```text
The MVP proves meaningful real-valued SGD.
```

Correct claim:

The current primitive proves a deterministic finite-field algebraic training-shaped transition. A bridge to
real-valued SGD would require fixed-point semantics, range bounds, rounding rules, and approximation-error
theorems.

### TVM-LIN-002: Random-Linear Equality Soundness

Statement:

```text
L != R -> Pr_q[<q,L> = <q,R>] <= 1 / |F|
```

Status: `assumption-bound`

Rust evidence:

- `random_linear_equal`
- `verify::tests::linear_training_verifier_rejects_sparse_error_poisoning`
- `verify::tests::linear_training_verifier_rejects_sparse_weight_poisoning`

Formal artifact needed:

```text
theorem random_linear_equal_sound
```

Assumptions:

- Challenge vector is uniform enough.
- Receipt roots are committed before challenge derivation.
- Hash-derived sampling matches the random-oracle model.

### TVM-COM-001: Canonical Encoding Binding

Statement:

```text
Different canonical receipt/tensor/block preimages should not map to the same committed hash except with
negligible probability.
```

Status: `assumption-bound`

Rust evidence:

- Domain-separated hashing in `crates/tensor_vm/src/types.rs::hash_bytes`
- Receipt id recomputation checks in verifier and chain receipt admission
- Root construction in `crates/tensor_vm/src/chain/roots.rs`
- Storage encoding tests in `crates/tensor_vm/src/storage.rs`

Formal artifact needed:

```text
theorem canonical_encoding_injective_before_hash
assumption hash_collision_resistance
```

Bad assumption to reject:

```text
Lean can prove SHA-256 collision resistance for our protocol.
```

Correct claim:

Lean can prove canonical encoding properties. Hash security remains an explicit cryptographic assumption.

### TVM-SIG-001: Signature Validity

Statement:

```text
Accepted receipts, attestations, and votes are signed by the claimed actor.
```

Status: `assumption-bound`

Rust evidence:

- `verify_signature` checks are present in receipt verification, attestation admission, and block voting.
- `crates/tensor_vm/src/types.rs::sign` is a reference helper, not a production signature scheme.

Formal artifact needed:

```text
assumption signature_unforgeability
assumption key_ownership
theorem accepted_statement_has_valid_signature_under_Sig
```

Bad assumption to reject:

```text
The current reference sign() helper is production authentication.
```

Correct claim:

The reference helper tests signing flow shape. Production security requires a real signature scheme,
domain-separated messages, replay resistance, and key custody assumptions.

### TVM-ATT-001: Assigned Validator Attestation Admission

Statement:

```text
SubmitAttestation(A) succeeds ->
  A.validator is registered
  A.stake equals registered stake
  A.signature verifies
  A.validator is assigned to A.receipt_id
  A.receipt_id exists
  A.job_id and A.primitive_type match the stored receipt
  no prior attestation by A.validator exists for A.receipt_id
```

Status: `local-proof-ready` with seed-lifecycle caveat

Rust evidence:

- `crates/tensor_vm/src/chain/validation.rs::submit_attestation`
- `crates/tensor_vm/src/scheduler.rs::assign_validators`
- `chain::tests::unassigned_validator_attestations_are_rejected`
- `chain::tests::duplicate_receipts_and_validator_attestations_are_rejected`
- `chain::tests::forged_attestation_stake_is_rejected`

Formal artifact needed:

```text
theorem submit_attestation_success_implies_registered_assigned_validator
```

Remaining blocker:

Assignment is currently recomputed from current `finalized_randomness`. Delayed attestations need a
receipt-lifecycle validation seed stored at receipt admission or otherwise fixed through the validation
window.

### TVM-QUO-001: Attestation Quorum Counts Only Assigned Signed Statements

Statement:

```text
has_attestation_quorum(receipt_id) = true ->
  enough unique assigned validators signed Valid/DataAvailable statements for that stored receipt
```

Status: `local-proof-ready` for syntactic quorum, `assumption-bound` for semantic verifier execution

Rust evidence:

- `crates/tensor_vm/src/chain/validation.rs::has_attestation_quorum`
- chain tests for invalid attestations, duplicate attestations, stake mismatch, unknown receipts, and
  unavailable data.

Formal artifact needed:

```text
theorem quorum_implies_unique_assigned_valid_attestation_weight
```

Bad assumption to reject:

```text
Validator count alone is the quorum.
```

Correct claim:

The quorum depends on unique validators, registered stake, assigned validator set, valid result, data
availability flag, matching receipt metadata, and signature validity.

Semantic boundary:

The current chain admission path does not recompute `verify_tensor_op` or `verify_linear_training_step` and
does not derive `checks_root` from tensor artifacts. A quorum therefore proves signed assigned-validator
statements, not that those validators actually executed the verifier correctly. That stronger claim needs
recomputable check leaves, block-level `checks_root`, challenge openings, or another evidence-binding
surface; see [`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).

### TVM-SET-001: Local Settlement Requires Quorum And Agreement

Statement:

```text
If a receipt is newly settled, then it had syntactic attestation quorum, redundant agreement if configured,
and no blocking conflicting linear transition.
```

Status: `local-proof-ready` for the current reference settlement model

Rust evidence:

- `crates/tensor_vm/src/chain/settlement.rs`
- `chain::tests::redundant_agreement_quorum_is_required_before_settlement`
- `chain::tests::conflicting_linear_training_roots_do_not_settle`
- reward settlement tests listed in `coverage_matrix.md`

Formal artifact needed:

```text
theorem settle_epoch_only_settles_quorum_agreed_receipts
```

Boundary:

This is not the v2 settled-receipt blockspace theorem. It proves local settlement behavior over signed
attestation statements, not canonical per-block inclusion or semantic verifier execution.

### TVM-DA-001: Verification-Time Data Availability

Statement:

```text
Validators can mark receipts unavailable when required tensor roots cannot be served.
```

Status: `local-proof-ready` for local behavior, `assumption-bound` for public availability

Rust evidence:

- `validator::tests::validator_attests_unavailable_for_each_missing_receipt_root`
- tensor-server tests referenced by `coverage_matrix.md`
- local checker tensor descriptor/row/chunk/opening fetches.

Formal artifact needed:

```text
theorem unavailable_attestation_blocks_quorum_or_settlement
assumption public_data_availability_measurement
```

Bad assumption to reject:

```text
Local tensor serving proves durable public data availability.
```

Correct claim:

Local serving proves the code path. Public DA requires external measurement during active and retention
windows.

## Consensus Theorems Blocked By Implementation

### TVM-BLK-001: Canonical Settled-Receipt Blockspace

Target statement:

```text
For parent state S, beacon b, and blockspace caps C, selected_receipts(S,b,C) is deterministic, contains only
eligible settled unspent receipts, respects byte/TWU/count caps, and carries over nonincluded receipts.
```

Status: `implementation-blocked`

Why blocked:

Current state has `settled_receipts: BTreeSet<Hash>`, not a v2 settled-receipt pool with eligibility,
expiry, spent/carry-over, byte size, and TWU cap metadata.

Required code surface before proof:

```text
SettledReceipt metadata
settled_receipt_pool
block_twu_cap
block_byte_cap
block_receipt_cap
canonical selector
spent/carry-over state
```

The required blockspace lifecycle and selected leaf model is specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).

### TVM-BLK-002: Block-Level Checks Root Recomputability

Target statement:

```text
Given a v2 block and parent state, every validator can recompute the selected receipt verification
transcripts and obtain the block's checks_root.
```

Status: `implementation-blocked`

Why blocked:

Current blocks do not contain `checks_root`, do not select a canonical receipt set, and do not define a
block-level verification transcript.

Required code surface before proof:

```text
check_leaf format
checks_root aggregation
selected receipt list/root
receipt transcript recomputation
challenge opening format
```

The semantic evidence requirements for these objects are specified in
[`mvp_core_verifier_evidence_model.md`](mvp_core_verifier_evidence_model.md).

### TVM-POW-001: Useful-Verification PoW Validity

Target statement:

```text
For a non-fallback block:
  proposer is a registered validator
  H(parent_hash || settled_receipt_set_root || checks_root || beacon || proposer || nonce) < target
  settled_receipt_set_root and checks_root are valid for parent state
```

Status: `implementation-blocked`

Why blocked:

Current `TensorBlock` has no `settled_receipt_set_root`, `checks_root`, `difficulty_target`, or `nonce`.
`chain::blocks::produce` advances the chain without a PoW predicate.

Bad assumption to reject:

```text
Adding a nonce to the current block hash proves useful verification.
```

Correct requirement:

The nonce must be bound to the canonical settled-receipt set and recomputable verification transcript.
Useful-work dominance is a separate economic assumption; see
[`mvp_core_useful_pow_work_model.md`](mvp_core_useful_pow_work_model.md).

### TVM-DIFF-001: Parent-State Difficulty Target Validity

Target statement:

```text
For every valid nonfallback v2 block B at parent state S, B.difficulty_target equals the target derived
from S.difficulty_state, bounded retarget rules, and committed difficulty parameters.
```

Status: `implementation-blocked`

Why blocked:

Current/candidate evidence can check a nonce against a static or helper-derived target, but no discharged
theorem derives targets from parent consensus state or proves retarget bounds.

Paper model:

- [`mvp_core_difficulty_retarget_model.md`](mvp_core_difficulty_retarget_model.md)

Required code surface before proof:

```text
DifficultyState
DifficultyParams
expected_target(parent_difficulty_state, height)
target_valid(parent_state, block)
hash_to_uint and target encoding vectors
```

### TVM-FIN-001: Finality Implies v2 Block Validity

Target statement:

```text
If a v2 block is finalized, then enough validator stake signed a block that passes parent, canonical
blockspace, checks_root, PoW, reward, and state-transition validation.
```

Status: `implementation-blocked`

Why blocked:

Current finality only checks signed stake over an existing block hash. It does not validate v2 block
semantics.

Required code surface before proof:

```text
validate_block_v2
submit_block_vote requires validate_block_v2
finality over valid blocks only
fallback block validity rules
```

### TVM-FALLBACK-001: Zero-Receipt / No-PoW Liveness Fallback

Target statement:

```text
If no valid useful-verification PoW block appears within the timeout, a stake-weighted validator fallback can
produce a reduced-reward PoW-skip block without miner TensorWork rewards.
```

Status: `implementation-blocked`

Why blocked:

The local reference now implements a `PowSkipFallback` path with timeout checks, deterministic
stake-weighted validator selection, reduced delayed proposer rewards, and focused tests. Formal proof is
still blocked on complete liveness/synchrony assumptions, multi-validator runtime evidence, public/CUDA
evidence, and theorem discharge.

Paper model:

- [`mvp_core_fallback_liveness_model.md`](mvp_core_fallback_liveness_model.md)

Required code surface before proof:

```text
valid_fallback_block(parent_state, block)
timeout/no-work evidence root
deterministic validator fallback rotation
reduced fallback reward transition
fallback vote admission gate
```

### TVM-REWARD-001: Delayed Reward Finality And Challenge Settlement

Target statement:

```text
Finalized v2 blocks create pending verifier-dependent reward claims; those claims become spendable only
after direct recomputation or challenge-window finality with no unresolved valid challenge.
```

Status: `implementation-blocked`

Why blocked:

The local reference now exposes pending receipt, proposer, challenge, and credit reward ledgers plus
voiding, clawback/nonpayment, and maturity transitions for implemented challenge paths. Formal proof remains
blocked on full challenge-window theorem discharge, complete interactive transcript dispute semantics, and
public deployed evidence.

Paper model:

- [`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md)

Required code surface before proof:

```text
RewardClaim status encoding
valid_challenge(parent_state, challenge)
resolve_challenge(parent_state, challenge)
settle_reward_claim(parent_state, claim)
reward_root over pending/challenged/invalidated/settled states
```

## Bad Assumptions Register

| Bad Assumption | Why It Is Bad | Correct Framing |
| --- | --- | --- |
| Gate 0 proves MVP core soundness | Gate 0 proves local reference behavior, not v2 useful-PoW consensus. | Gate 0 is necessary local evidence, not full proof. |
| Freivalds proves all cells deterministically | Freivalds is probabilistic. | State false-accept bounds and randomness assumptions. |
| Row sampling is block-validity security | Sparse corruption can evade small samples. | Row sampling is audit unless configured bounds are strong enough. |
| Field training proves useful ML training | Field SGD-shaped algebra is not real-valued ML convergence. | Claim deterministic algebraic transition only. |
| Receipt map root is canonical blockspace | A global map root does not define selected eligible receipts. | Need deterministic settled-receipt selector and blockspace caps. |
| Per-receipt checks_root proves block proposer verified | Blocks do not aggregate or validate a canonical transcript. | Need block-level checks_root and challenge path. |
| Aggregate checks_root proves verifier execution | A root over signed check claims is still a root over claims. | Need recomputable or challengeable verifier evidence. |
| Stake finality implies useful-PoW validity | Current votes only check known block hash and signature/stake. | Votes must require v2 block validation. |
| Block target validates itself | A nonce below a candidate-supplied target can be trivial. | Derive target from parent difficulty state and bounded retarget rules. |
| Fallback or empty blocks prove useful work | Fallback is a liveness transition for zero-receipt, below-floor, or timeout cases. | Keep fallback disjoint from useful-PoW and give it reduced rewards. |
| Block finality means reward finality | Verifier-dependent rewards remain challengeable after block ordering finality. | Use pending reward claims until direct recomputation or challenge-window settlement. |
| Reference signatures imply production authentication | `sign` is a hash helper. | Use real signature assumptions and production crypto. |
| Local containers are independent operators | They are separate local participants, not independent principals. | Public evidence must prove independent operators. |
| Local tensor serving is durable DA | It proves a path, not public retention. | DA needs active/retention-window external measurement. |
| Produced block implies eligible proposer | `produce_block` accepts a supplied address and finality checks voters, not proposer eligibility. | v2 block admission must validate a registered validator useful-PoW winner. |
| Valid attestation means verifier ran | The chain counts signed Valid/DataAvailable statements without recomputing verifier transcripts. | Phrase quorum syntactically until checks are recomputable or challengeable. |

## Mechanization Package Shape

The first mechanized proof package should be deliberately narrow:

```text
formal/TensorVM/Field.lean
formal/TensorVM/Tensor.lean
formal/TensorVM/Freivalds.lean
formal/TensorVM/RandomLinear.lean
formal/TensorVM/LinearStep.lean
formal/TensorVM/Manifest.lean
```

Initial theorem names:

```text
tensor_ops_deterministic
matmul_shape
matmul_deterministic
freivalds_complete
freivalds_sound_one_round
freivalds_repeated_sound
row_sample_detection_probability
random_linear_equal_complete
random_linear_equal_sound
linear_training_step_complete
canonical_encoding_injective_before_hash
```

Do not attempt to mechanize these until the implementation exposes the required object:

```text
canonical_settled_receipt_blockspace
block_checks_root_recomputable
useful_verification_pow_valid
finality_implies_v2_block_valid
zero_receipt_pow_skip_fallback_live
```

## Release Gate For Claiming Core Soundness

Do not call the MVP core sound until all of these are true:

1. Every verifier acceptance rule maps to a theorem or explicit assumption in this manifest.
2. Every consensus block validity rule maps to a theorem or explicit assumption in this manifest.
3. Current code has v2 block fields and a canonical settled-receipt selector.
4. Finality votes reject invalid v2 blocks.
5. Block production/admission rejects ineligible proposers.
6. Attestation/quorum claims distinguish signed statements from semantic verifier execution.
7. The proof docs no longer classify useful-verification PoW as `implementation-blocked`.
8. Public claims say "probabilistic verification" unless a receipt is fully recomputed or succinctly proven.

Current result: not sound yet.
