# TensorVM MVP Core Candidate v2 Block Audit

Status: documentation-only audit of the local v2-block reference path observed on May 23, 2026.

Purpose: evaluate whether the local v2-shaped block changes discharge the proof obligations in the MVP core
proof corpus. They do not. The reference path is meaningful progress and is build-clean locally, but it does
not yet prove the reviewed v2 consensus theorem.

Existing proof statuses remain conservative until exact parent snapshots, child-state apply semantics,
selected-leaf lifecycle metadata, challenge openings, difficulty retargeting, and fallback semantics are
implemented and mapped back into the formal proof manifest.

The parent-state transition theorem that this candidate still needs is specified in
[`mvp_core_parent_state_transition_model.md`](mvp_core_parent_state_transition_model.md).
The settled-receipt lifecycle and blockspace theorem that this candidate still needs is specified in
[`mvp_core_settled_receipt_blockspace_model.md`](mvp_core_settled_receipt_blockspace_model.md).

## Evidence Scope

Observed implementation files:

```text
crates/tensor_vm/src/chain.rs
crates/tensor_vm/src/chain/blocks.rs
crates/tensor_vm/src/chain/commands.rs
crates/tensor_vm/src/chain/proposer.rs
crates/tensor_vm/src/chain/roots.rs
crates/tensor_vm/src/chain/state.rs
crates/tensor_vm/src/chain/validation.rs
crates/tensor_vm/src/lib.rs
crates/tensor_vm/src/localnet.rs
crates/tensor_vm/src/storage.rs
crates/tensor_vm/src/testnet.rs
docs/tensorvm/local_chain_production_exec_plan.md
```

Build check:

```text
cargo check -p tensor_vm --all-targets
```

Result: passed in the Iteration 11 validation run.

Additional broad evidence:

```text
cargo test -p tensor_vm --lib
cargo test -p tensor_vm local_testnet --release
cargo tarpaulin --workspace --offline
```

Consequence: build status no longer blocks local evidence. The remaining blockers are proof-surface gaps,
not compile failures.

## Candidate Progress

The candidate does make real movement toward the reviewed v2 shape:

- `TensorBlock` has v2-shaped fields: `settled_receipt_set_root`, `checks_root`, `beacon`,
  `difficulty_target`, and `nonce`.
- `produce_block` now returns `Result<TensorBlock>` and rejects non-validator proposers.
- `proposer_for_next_epoch` selects among validators and ignores miner TensorWork.
- `submit_block_vote` looks up the block and calls block validation before counting a vote.
- `canonical_blockspace` derives a selected receipt list from settled receipts, beacon, parent hash, and
  static caps.
- `pow_hash` and `pow_valid` bind nonce search to a candidate header.
- Block storage has started moving from old roots to v2-shaped block fields.

These changes attack several live counterexamples. They are still a candidate slice, not a completed proof
surface.

## Non-Discharged Findings

### CV2-001: Build-Clean Is Only The First Gate

The first proof gate is a build-clean implementation. The local reference path now passes `cargo check -p
tensor_vm --all-targets`, but that only proves the candidate compiles.

Required next repair:

- Keep broad validation evidence attached to each semantic upgrade.
- Add adversarial tests for every remaining theorem gate before upgrading proof status.

### CV2-002: Block Validation Uses A Reconstructed Parent View, Not An Explicit Parent Snapshot

`blocks::validate(chain, block, strict_state_root)` now reconstructs a parent-like state view for known
blocks by rewinding height, epoch, finalized randomness, block votes, finalized status, and selected
included receipts. For finality and replay the theorem still needs an explicit parent snapshot or replayable
transition, not a best-effort reconstruction from mutable node state.

The local path partially avoids the old current-state bug for immediate production and vote admission, but
the proof statement must still be:

```text
valid_v2_block(parent_state, block)
```

not:

```text
valid_v2_block(current_node_state, block)
```

Required repair:

- Define the parent-state snapshot used for validation.
- Validate every root and transition against that parent state.
- Prove vote admission imports the parent-state validation result, not a later mutable state.

### CV2-003: The Beacon Is Self-Consistent But Not Parent-State Validated

The candidate block stores `beacon`, and validation recomputes canonical blockspace using `block.beacon`.
The local path now checks the beacon against genesis randomness for height 0 and against the parent block's
finalized-randomness transition for later heights. This is still not a complete production randomness model
or parent-state snapshot theorem.

The remaining selection-grinding surface is in the randomness model and replay proof, not in the immediate
local `block.beacon` equality check.

Required repair:

- Add `beacon_valid(parent_state, block.beacon)`.
- Bind beacon evolution into the block transition theorem.
- Add adversarial tests for altered beacons that preserve all other roots.

### CV2-004: Selected Receipt Root Does Not Bind The Full Selected Receipt Leaf

`selected_receipt_root` currently hashes a set of receipt ids. The v2 proof target needs a canonical selected
receipt leaf that binds at least receipt id, receipt hash, TensorWork units, and any blockspace-accounting
fields used by caps.

Receipt id may indirectly bind some content if its construction is proven, but the selected-root theorem
should not rely on an unstated transitive assumption.

Required repair:

- Define `SelectedReceiptLeaf`.
- Prove leaf encoding injectivity before hashing.
- Include enough fields to support blockspace and reward theorems.

### CV2-005: Canonical Selection Skips Over Cap-Exceeding Receipts

The reviewed spec says the selector appends receipts in deterministic order until adding the next receipt
would exceed a cap. The candidate continues past a cap-exceeding receipt and may include later smaller
receipts.

That may be a valid design choice, but it is a different theorem. It changes omission/carry-over semantics
and can alter censorship, fairness, and congestion proofs.

Required repair:

- Either change the selector to the spec's stop-before-first-exceed rule, or update the spec and proof
  statement to the skip-over-large-receipts rule.
- Add adversarial tests for one large receipt followed by many small receipts.
- State carry-over and expiry behavior for skipped receipts.

### CV2-006: Eligibility Lacks Unspent, Expiry, And Challenge-Window State

The candidate filters settled receipts, included receipts, and data-unavailable receipts. The v2 proof target
also needs expiry, challenge-window availability, and carry-over semantics.

Selected receipts are now marked included after local production so the selector will not select them again.
That is a local one-shot guard, not the full settled-receipt lifecycle theorem.

Required repair:

- Promote included/spent metadata into the canonical selected-receipt lifecycle.
- Add expiry and challenge-window availability predicates.
- Prove nonselected eligible receipts carry over and selected receipts cannot be included twice.

### CV2-007: `checks_root` Aggregates Signed Claims, Not Recomputed Check Leaves

`block_checks_root` aggregates per-attestation `checks_root` values that are signed, valid, and
data-available. It does not recompute verifier transcripts from selected receipt artifacts. It also does not
define `CheckLeaf`, `VerifierTranscript`, or challenge openings.

This is exactly the bad assumption rejected by the verifier evidence model: a root over signed claims is
still a root over claims.

Required repair:

- Define check leaves over receipt id, seed anchor, primitive, verifier parameters, artifact roots, result,
  and transcript roots.
- Either recompute those leaves during block/vote validation or make them challengeable with consensus
  openings.
- Tie reward settlement to direct recomputation or challenge-window finality.

### CV2-008: `block_checks_root` Does Not Itself Prove Assignment Or Stake

The root helper checks attestation signature relation, `Valid`, DA bit, and receipt id. It does not itself
prove the attesting validator was registered, assigned, had the stated stake, or was accepted by
`submit_attestation`.

This can be acceptable only if the theorem imports the chain admission invariant:

```text
all attestations in state were admitted through submit_attestation
```

That invariant is not automatic while `ChainState` fields are public and tests or internal code can mutate
state directly.

Required repair:

- State the admitted-state invariant explicitly.
- Keep root helpers private to admitted state, or validate assignment/stake in the check-leaf construction.
- Add tests for forged direct state entries if public mutation remains in scope.

### CV2-009: Difficulty Is A Static Helper, Not Parent-State Difficulty

`useful_pow_difficulty_target` returns a fixed target. Validation checks equality to that helper. There is
no parent-state difficulty, retarget rule, floor/ceiling theorem, or useful-work cost relationship.

This may be enough for a tiny local static-difficulty slice. It is not enough for the reviewed useful-PoW
economics theorem.

Required repair:

- Define difficulty state and validate `block.difficulty_target` from parent state.
- Add retarget bounds, target floor/ceiling, and work-floor rules.
- State that static difficulty is local evidence only unless the economic model is discharged.
- Use the target convention and discharge gate in `mvp_core_difficulty_retarget_model.md`.

### CV2-010: Empty Or Tiny Blocks Can Still Be Normal PoW Blocks

There is no nonfallback work floor. If the selected set is empty, the block can still satisfy `pow_valid`
and be produced as a normal block.

The v2 model needs explicit fallback semantics for zero-receipt or no-PoW periods. Empty normal blocks must
not be marketed as useful-verification PoW.

Required repair:

- Add `nonfallback_work_floor(parent_state, block)`.
- Add a separate fallback block validity predicate with reduced rewards and no miner TWU rewards.
- Prove fallback blocks do not claim useful work.

### CV2-011: Reward Mutation Can Precede Block Validity Failure

Status: discharged for the local `produce_block_with_rewards` wrapper.

`produce_block_with_rewards` now rejects unknown validators before crediting and restores the reward state
if the fallible production path returns an error.

This is a state-transition proof failure: failed block admission must not mutate rewards.

Remaining repair:

- Add adversarial tests for invalid proposer or invalid block production with nonzero rewards.
- Move verifier-dependent rewards into the pending/challenge-window reward model.

### CV2-012: Vote Validation Does Not Prove Full State Transition Roots

`submit_block_vote` now calls `blocks::validate(chain, &block, true)`, so local vote admission validates
`block.state_root` against the reconstructed parent-like state view before counting stake.

The finality theorem still needs finalized blocks to imply a valid v2 child-state transition, not only a
valid parent-root/header view.

Required repair:

- Split validation into explicit predicates: header validity, selected blockspace validity, checks-root
  validity, state-transition validity, and reward-transition validity.
- Require the right complete predicate before finality votes count.

### CV2-013: PoW Hash Ordering Needs A Consensus Integer Model

`hash_below_target` compares byte arrays directly. That may be acceptable if the consensus model defines
hashes as big-endian byte strings and target comparison uses lexicographic order. It is not acceptable if the
intended theorem is over a `U256` with different byte order.

Required repair:

- State the hash-to-integer interpretation.
- Add test vectors around boundary targets.
- Keep the proof model and Rust comparison identical.

## Proof Status Matrix

| Obligation | Candidate Movement | Remaining Status |
| --- | --- | --- |
| V2-BLK-001 canonical selection | Partial selector exists with included-receipt exclusion. | Still blocked: expiry, carry-over, challenge-window eligibility, and spec mismatch remain. |
| V2-BLK-002 selected root | Field exists. | Still blocked: root over ids only, no selected leaf theorem. |
| V2-CHK-001 check leaf recomputability | No semantic leaf. | Still blocked. |
| V2-CHK-002 block checks root | Aggregate root exists. | Still blocked: aggregates signed claims, not recomputed/challengeable evidence. |
| V2-DIFF-001 parent-state difficulty | Static target helper exists. | Still blocked: no parent difficulty state, retarget bounds, target vectors, or work-floor theorem. |
| V2-POW-001 useful-PoW validity | Nonce/target/hash predicate exists and local beacon equality is checked. | Still blocked: static target, no full randomness model, no work floor, no economics. |
| V2-PROP-001 proposer eligibility | Significant progress: non-validator rejection, validator selector, and local CPU Docker proof for the single configured validator proposer. | Not fully discharged until multi-validator/public proposer networking and complete validation are in place. |
| V2-STATE-001 valid transition | Roots exist and vote admission checks parent-root validity. | Still blocked: no child-state apply theorem or exact parent snapshot. |
| V2-REWARD-001 delayed reward finality | Local pending receipt/proposer/challenge/credit ledgers, voiding, and maturity release exist for implemented paths. | Still blocked: full challenge-window theorem, interactive transcript disputes, and public deployed evidence are incomplete. |
| V2-FIN-001 vote admission validates block | Partial: vote path calls strict `validate`. | Still blocked: validation predicate lacks full child-state transition and replay model. |
| V2-FIN-002 finality implies v2 validity | Partial path. | Still blocked until V2-FIN-001 and V2-STATE-001 are complete. |
| V2-FALLBACK-001 PoW-skip fallback | Local `PowSkipFallback` timeout, deterministic selected proposer, reduced delayed reward, and rejection tests exist. | Still blocked for full synchrony/liveness theorem, multi-validator runtime evidence, and public/CUDA proof. |

## Minimum Next Gates

Before this candidate can upgrade any proof status:

1. `cargo check -p tensor_vm --all-targets` passes.
2. All block-production callers handle `Result` explicitly.
3. `valid_v2_block(parent_state, block)` is factored as a named predicate.
4. Beacon, difficulty, selected receipt root, checks root, state root, reward root, and proposer eligibility
   are validated from parent state.
5. Selected receipt lifecycle state prevents double inclusion.
6. `CheckLeaf` is recomputable or challengeable, not merely an aggregated signed `checks_root`.
7. Empty normal blocks are separated from fallback blocks.
8. Rewards are atomic with successful block admission.
9. Adversarial tests cover wrong beacon, wrong selected root, wrong checks root, invalid nonce, invalid
   target, non-validator proposer, cap-boundary selection, repeated receipt inclusion, and failed production
   with rewards.
10. The formal proof manifest and traceability matrix are updated only after the build and tests establish
    the implementation surface.

## Current Judgment

The candidate is directionally aligned with v2 but not proof-sound. It closes the most obvious old shape gap
by adding v2-looking block fields and validator proposer checks, but it still leaves the hard theorem gaps:
parent-state validation, semantic verifier evidence, challenge-window reward finality, selected receipt
lifecycle, difficulty economics, fallback, and build-clean evidence.

Do not claim this candidate proves the MVP core sound.
