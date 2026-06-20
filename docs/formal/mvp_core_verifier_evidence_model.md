# TensorVM MVP Core Verifier Evidence Model

Status: documentation-only model for the blocked semantic verifier-execution bridge.

Purpose: define the evidence needed before TensorVM can honestly upgrade from:

```text
assigned validators signed Valid/DataAvailable statements
```

to:

```text
the selected receipt verification relation held for the block
```

The current sound kernel can support syntactic attestation and quorum theorems. It cannot yet support a
semantic verifier-execution theorem. A signed `Valid` statement, a per-attestation `checks_root`, or an
aggregate root over signed `checks_root` values is still a statement about bytes unless the protocol can
recompute, open, or challenge the verifier transcript.

Reward finality for challengeable evidence is specified separately in
[`mvp_core_reward_finality_challenge_model.md`](mvp_core_reward_finality_challenge_model.md). This evidence
model is not enough by itself unless the reward state waits for that direct recomputation or challenge
finality.

## Current Verdict

The semantic bridge remains blocked.

Current proof-safe statement:

```text
quorum(receipt) means enough assigned registered validators signed matching Valid/DataAvailable statements.
```

Unsafe statement:

```text
quorum(receipt) means enough validators actually executed the verifier correctly.
```

The repository has verifier functions, reference challenge helpers, attestation admission checks, and
emerging block-check-root surfaces. Those are useful ingredients. They do not discharge the semantic bridge
until the chain has an accepted evidence object whose leaves can be recomputed from committed artifacts or
challenged with a consensus-valid opening.

## Evidence Objects

The v2 proof should name these objects explicitly:

| Object | Meaning | Required Binding |
| --- | --- | --- |
| `ReceiptCommitment` | Miner-committed job, output roots, metadata, and receipt id. | Receipt id, primitive type, input/output roots, model/code version, miner signature. |
| `ValidationSeed` | Lifecycle-stable challenge seed for the receipt. | Fixed before verifier challenges and included in transcript anchors. |
| `VerifierTranscript` | The exact checks performed for the primitive. | Freivalds vectors/results, random-linear checks, DA root checks, failure reason if any. |
| `CheckLeaf` | Canonical committed summary of one selected receipt's verifier transcript. | Receipt id, receipt hash, primitive, seed anchor, transcript roots, result, DA evidence root, parameter version. |
| `AttestationEvidence` | Validator statement about a receipt and check leaf. | Validator id, stake snapshot, assignment proof, statement domain, signature, and `CheckLeaf` hash. |
| `BlockChecksRoot` | Aggregate root over the canonical selected receipt check leaves. | Selected receipt order, leaf schema version, and no extra or omitted leaves. |
| `ChallengeOpening` | Data that can prove a leaf or transcript is wrong during the challenge window. | Merkle opening, receipt artifacts, recomputed transcript, challenger signature, timeout context. |

If any of these objects is replaced by an opaque hash with no recomputation or opening rule, the proof must
remain syntactic.

## Theorem Split

The semantic bridge needs separate theorems. Collapsing them into "validators attested" hides the hard
parts.

| ID | Target Theorem | Status Today | Why It Matters |
| --- | --- | --- | --- |
| EVID-001 | Receipt commitments bind the exact artifacts used by verification. | Assumption-bound / partially represented. | Prevents checking one output while rewarding another. |
| EVID-002 | The validation seed is lifecycle-stable and challenge domains are fixed. | Implementation-blocked. | Prevents adaptive outputs or delayed-attestation seed changes. |
| EVID-003 | `VerifierTranscript` recomputes from committed artifacts for each primitive. | Formalizable for verifier kernels, not chain-bound. | Connects Rust verifier relations to chain evidence. |
| EVID-004 | `CheckLeaf` is an injective encoding of receipt id, seed anchor, result, and transcript roots before hashing. | Formalizable after schema definition. | Prevents one leaf from meaning multiple verification events. |
| EVID-005 | `AttestationEvidence` binds a validator signature to a specific `CheckLeaf`. | Implementation-dischargeable plus signature assumptions. | Prevents a signed Valid bit from floating free of verifier evidence. |
| EVID-006 | `BlockChecksRoot` is exactly the ordered root of selected receipt `CheckLeaf` values. | Implementation-blocked. | Prevents arbitrary or incomplete aggregate check roots. |
| EVID-007 | A valid challenge opening can disprove a wrong leaf within the challenge window. | Implementation/evidence-blocked. | Makes hidden or summarized transcript commitments accountable. |
| EVID-008 | Reward settlement waits for the challenge window or imports direct recomputation. | Paper-specified in the reward-finality model, implementation-blocked. | Prevents paying rewards before evidence can be contested. |
| EVID-009 | No accepted invalid verifier evidence after the challenge window, under explicit assumptions. | Assumption-bound. | Requires honest/challenger availability, DA, timeout, and verifier soundness assumptions. |

The current syntactic quorum theorem can stay in the sound kernel. A semantic theorem must import the
evidence theorems above plus the probabilistic verifier bounds.

## Acceptable Evidence Designs

There are three defensible designs. The implementation can choose one, but proof docs must say which one is
being used.

| Design | Safe Claim | Main Cost Or Assumption |
| --- | --- | --- |
| Direct recomputation at block/vote admission | Every validating node recomputes selected receipt check leaves before accepting the block. | Higher validation cost; still needs artifact availability and seed stability. |
| Challengeable transcript commitments | Blocks commit to check leaves; rewards wait while challengers can open and disprove wrong leaves. | Requires challenge liveness, DA through the window, timeout rules, and delayed/clawback rewards. |
| Proof-carrying verifier evidence | Blocks include succinct or independently checkable proofs for verifier transcripts. | Requires a proof system, circuit/model definition, and proof-verifier soundness assumptions. |

A fourth option, "validators are honest because they signed," is not a proof design. It is an honesty
assumption and must remain visible in theorem statements.

## Check Leaf Minimum Schema

A future `CheckLeaf` should include, or be injectively derived from, at least:

```text
leaf_version
receipt_id
receipt_hash
primitive_type
validation_seed_anchor
verifier_params_version
artifact_commitment_roots
data_availability_root
verification_result
freivalds_transcript_root
random_linear_transcript_root
failure_code_or_zero
```

The leaf should not depend on validator identity unless the design intentionally makes verifier evidence
validator-specific. If validator-specific leaves are used, the block-level theorem must define how multiple
validator leaves compose into one receipt-level result.

## Challenge Opening Minimum Schema

A future challenge opening should include:

```text
block_hash
receipt_id
selected_receipt_index
claimed_check_leaf
checks_root_merkle_path
receipt_artifact_openings
validation_seed_anchor
recomputed_transcript
expected_result
observed_result
challenger
challenge_height_or_time
challenge_signature
```

The opening must be enough for any node to distinguish:

1. Bad Merkle opening.
2. Leaf does not match the selected receipt.
3. Transcript does not recompute from artifacts.
4. Transcript recomputes and proves the claimed leaf/result wrong.
5. Challenge is late or outside the allowed window.

Without these cases, challenge resolution becomes a policy statement rather than a proof obligation.

## Probability Composition Boundary

Semantic verifier evidence does not remove probabilistic verifier assumptions.

If a check leaf records one Freivalds round over a small field, the evidence proves only that the recorded
probabilistic check accepted. It does not prove deterministic all-cell correctness. To compose semantic
evidence with verifier soundness, the theorem must import:

- field size,
- round count,
- challenge sampling model,
- receipt-volume union bound,
- lifecycle seed non-adaptivity,
- independence or domain-separation assumptions,
- and the maximum number of accepted receipts covered by the claim.

Validator multiplicity cannot be multiplied into the false-accept budget unless each validator's evidence
uses independent, lifecycle-stable challenge material and the quorum-to-evidence theorem proves those
checks were actually performed or challengeable.

## Current Code Evidence Boundary

These are useful current or emerging ingredients:

- Verifier functions return reports with `checks_root` values.
- Attestation admission checks assignment, registration, signatures, receipt metadata, and duplicate
  prevention.
- Challenge helper code can recompute TensorOp verification for a supplied receipt/artifact bundle.
- Block-level root work can aggregate check-root-shaped data.

These ingredients do not yet prove semantic verifier execution:

- Attestation admission can accept a signed `Valid` statement without recomputing the verifier.
- A root over attestation `checks_root` fields can aggregate claims without proving the claims are true.
- A challenge helper is not a consensus challenge window unless it is tied to block validity, openings,
  deadlines, reward delay, and state transitions.
- A valid block `checks_root` is not enough unless every leaf is recomputable or challengeable from the
  selected receipt artifacts.

## Bad Assumptions Rejected

The proof corpus must reject these assumptions:

1. A signed `Valid` statement proves verifier execution.
2. An aggregate root over signed `checks_root` values proves the underlying transcripts.
3. A challenge helper proves challenge-window soundness without timeout and reward-state integration.
4. Challenge absence proves correctness without challenger availability and DA assumptions.
5. Multiple validators multiply Freivalds security unless independent challenge evidence is bound.
6. Hidden check details are safe if they can never be opened.
7. A `checks_root` can omit seed, parameter version, primitive type, or artifact roots.
8. Reward settlement can happen before direct recomputation or challenge finality.

Current local implementation status: selected receipt check leaves can be opened and block-check
challenges can disprove a committed `checks_root` against canonical local recomputation, with pending
proposer reward invalidation. This is challenge-window plumbing over current check leaves, not proof that
the aggregated attestation `checks_root` values are full verifier transcripts.

## Discharge Gate

Do not upgrade `K-QUO-001`, `V2-CHK-001`, or `V2-CHK-002` to semantic verifier evidence until all of these
are true:

1. `CheckLeaf` and `VerifierTranscript` schemas are committed and versioned.
2. The leaf encoding is injective before hashing.
3. The leaf binds receipt id, receipt hash, primitive type, validation seed anchor, verifier parameters,
   artifact roots, result, and transcript roots.
4. The chain can recompute leaves directly or validate challenge openings against `checks_root`.
5. Challenge windows, deadlines, reward delay, clawback, and state transitions are part of consensus state;
   see `mvp_core_reward_finality_challenge_model.md`.
6. DA assumptions cover every artifact needed through the challenge window.
7. The probabilistic verifier budget is imported into the semantic theorem.
8. The traceability matrix maps the theorem to committed code, adversarial tests, and remaining assumptions.

## Current Judgment

The present proof boundary should remain strict:

```text
signed quorum -> syntactic assigned-validator agreement
```

is defensible.

```text
signed quorum -> validators executed the verifier correctly
```

is not yet defensible. The missing object is not another root by itself; it is a recomputable or
challengeable evidence surface that binds signatures, selected receipts, verifier transcripts, seeds,
artifacts, block roots, and reward finality into one auditable chain transition.
