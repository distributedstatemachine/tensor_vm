# Useful-Work Chain Specification

**Status:** Design draft (v0). Not a production security proof.
**Scope:** A standalone blockchain whose native proof-of-work is the *verification of useful tensor computation*. Miners execute deterministic tensor workloads; validators verify them with cheaper-than-recompute checks and mine blocks over the verification result; the chain's validity rule *is* the correctness of that work.

This spec unifies three things:

1. A fully-specified, content-addressed tensor IR as the workload language (§4).
2. A useful-verification PoW over deterministic settled-receipt blockspace as the consensus model.
3. An interactive fraud-proof layer over execution-trace commitments, which extends verification beyond linear algebra to general nonlinear workloads.

Throughout, the load-bearing idea is stated once and reused:

> **Security comes from the asymmetry between doing the work and checking the work, anchored to stake.** A miner must do `O(expensive)` tensor compute; any honest validator can detect a false result for `O(cheap)`; a false result that survives is an economic loss (slashing) larger than the reward for faking it. The chain only finalizes work that passed cheap verification, so *block validity and work-correctness are the same predicate*.

---

## 0. Glossary

| Term | Meaning |
|---|---|
| **Workload / Job** | A deterministic tensor program: an IR graph + bound inputs + verification policy. |
| **Receipt** | A miner's signed claim that it executed a job, committing to outputs and an execution trace. |
| **Attestation** | A validator's signed result of verifying a receipt (pass / fail / inconclusive + evidence). |
| **`checks_root`** | Merkle root over the canonical set of validator checks for a block's settled receipts. |
| **Useful-Verification PoW (UVPoW)** | The block-production puzzle: a nonce search bound to a validator's `checks_root` commitment. |
| **Settled receipt** | A receipt that has accumulated the required attestations and is eligible for blockspace. |
| **Trace root** | Merkle root over per-op output commitments of an execution, enabling interactive fraud proofs. |
| **Field `F_p`** | The prime field all consensus-critical arithmetic happens in. |
| **TensorWork** | A reward/telemetry metric for miners. **Never** selects proposers or influences consensus. |
| **Normalization** | The deterministic, semantics-preserving pass pipeline applied to a graph before `graph_id` so the id addresses the computation, not its syntax (§4.5.1). |
| **Reduction-order class** | A per-op, dtype-derived tag (`OrderFree` for `field`, `AscendingFixedPoint` for `fixedNN`) fixing whether accumulation order is observable (§3.4). |
| **Verification class / region fusion** | The per-op verifier kind (Freivalds / random-linear / replay / index-consistency / redundancy) and the lattice-join rule that lets contiguous regions be checked once (§4.11). |
| **Prover/verifier seam** | The single asymmetry boundary: miners compute offline, validators check cheaply on-chain; the IR is the contract across it (§1.3). |

---

## 1. Thesis & Non-Goals

### 1.1 Thesis
Traditional PoW proves energy was burned on hash preimage search. This chain proves, within explicit soundness bounds, that **tensor state transitions were computed according to canonical deterministic semantics** — and recycles that proof as the block-production mechanism.

### 1.2 Non-Goals (v0)
- Arbitrary floating-point output as consensus state.
- Full Transformer training as a single consensus step.
- General smart contracts / a Turing-complete VM as consensus.
- On-chain storage of full tensors.
- ZKML for the whole workload.
- Subjective "usefulness" scoring.

These are roadmap items (§13), not v0.

### 1.3 Execution model: offline prover, on-chain verifier
The chain is built around exactly one asymmetry boundary. Heavy tensor compute happens **offline** on miner hardware; consensus only ever performs **cheap checks** on-chain. Nothing about block validity requires any node to redo the miner's FLOPs — that would make the work redundant rather than useful.

| Concern | Where it lives | Cost | Reference |
|---|---|---|---|
| Program (IR graph), input commitments, job body | on-chain consensus state | hashes | §4, §5 |
| Full tensors, intermediate activations, the actual execution | **offline, miner box** | GB / `O(expensive)` | §5, §9 |
| Receipt: output commitments + `trace_root` + bond + sig | on-chain | hashes | §5.3 |
| Freivalds / random-linear checks, availability sampling, `checks_root` | **validator, on-chain** | sublinear (§6–§7) | §6, §7, §11 |
| Full execution trace | offline; served via DA **only if disputed** | `O(expensive)`, served lazily | §8.2, §9 |
| One-op re-execution after dispute bisection | on-chain / referee committee | `O(one op)` | §8.2 |

The IR (§4) is the **contract** across this boundary: `graph_id` fixes the program both sides agree on before any FLOP is spent, and every verifier check (§6–§8) is defined relative to that fixed program. Consequently the compiler-style machinery introduced in §4 (canonical encoding, structural normalization §4.5.1, the op DAG as an execution trace §4.10, verification-class composition §4.11) is judged **solely** by whether it (a) makes the offline→on-chain contract unambiguous, (b) shrinks the on-chain verifier, or (c) makes disputes bisectable — never by raw execution speed. Speed is the miner's private concern; correctness-under-cheap-checking is the protocol's.

---

## 2. System Roles (hard process boundaries)

Roles are **separate long-running processes** with distinct keys, durable state, and network identities. Jobs, receipts, attestations, tensor fetches, blocks, and votes cross process boundaries over p2p/RPC before they affect another node. No single process may mutate multiple counted roles in memory.

- **Miner.** Subscribes to jobs, executes the IR on its hardware (GPU allowed), publishes receipts, and serves tensor data for verification.
- **Validator.** Verifies settled receipts (the actual "work"), builds `checks_root`, performs UVPoW, proposes blocks, and casts stake-weighted finality votes. **Block proposal is validator-owned.** There is no separate miner-proposer role.
- **Full node.** Validates and relays blocks; can challenge (see §8); does not necessarily stake.
- **Job source.** Emits deterministic, content-addressed jobs (synthetic in v0; externally-useful workloads later). Job emission must be deterministic and verifiable so no party can grind job content to advantage a miner.

```mermaid
flowchart LR
  JS[Job Source] -->|signed jobs| P2P((libp2p))
  M[Miner] -->|receipt + trace_root| P2P
  M -->|serve tensors| V
  V[Validator] -->|verify -> attestation| P2P
  V -->|checks_root + UVPoW nonce| BLK[TensorBlock]
  V -->|BFT vote| BLK
  FN[Full Node / Challenger] -->|fraud proof| P2P
```

---

## 3. Determinism Contract (the foundation)

Consensus-critical computation MUST be bit-exact reproducible across machines. Verification is by **equality of commitments**, never by floating-point tolerance.

### 3.1 Arithmetic domain
- All consensus-critical values are elements of a prime field **`F_p`** (e.g. a 61-bit or 64-bit-friendly prime; final choice in §12).
- Real-valued workloads use **fixed-point**: a value `x` is encoded as `round(x · 2^s) mod p` for a per-tensor scale `s` declared in the IR. All scale handling (rescale after multiply, rounding mode, saturation) is canonical and part of the op semantics.
- Integer workloads map directly into `F_p`.
- Overflow, rounding (round-half-to-even), and tensor memory layout (row-major, canonical dtype tags) are fixed by spec.

### 3.2 Hardware policy
- Miners MAY use GPU/CUDA kernels for speed, but the **committed output must equal the canonical `F_p` semantics**. A GPU kernel that produces a near-but-not-equal result yields an invalid receipt.
- Banned from the consensus path: nondeterministic CUDA reductions, fused kernels whose reduction order is unspecified, any fp16/bf16/fp32 value as committed state.

### 3.3 Why this matters
Equality-of-commitment verification (Freivalds, hash equality, fraud-proof bisection) is only sound if all honest nodes agree on every bit. The fixed-point/field contract is what makes "anyone can recompute one op and rule on it" possible. **This is the single hardest engineering constraint in the system; everything else depends on it.**

> Status: the local reference now publishes a conformance vector suite for the current executable exact
> `F_p` ops used by TensorOp and LinearTrainingStep, and receipt verification gates those current jobs on
> the matching suite profile. The suite now carries per-input and expected output dtype/scale metadata for
> fixed-point `cast`/`round` half-even rescale vectors, `Fixed32` `mul` half-even rescale vectors
> including mixed-scale product rescale, exact field modular-inverse and `Fixed32` reciprocal `div`,
> exact Tier-A matrix-contraction `einsum`, plus multi-output expected tensors for exact per-channel int8
> quantize scale output. `int8`, `uint8`, and `bool` dtype tags are implemented, exact
> `quantize_int8_per_channel`/`dequantize_int8_per_channel` vectors are CPU-conformance covered, and
> `quantize_pack_int8`/`unpack_dequantize_int8` use a byte-exact flat `uint8` payload vector. Fixed-scale
> comparison masks and int8 selection are also conformance covered. Local A100 CUDA pass evidence now
> covers the current executable CUDA TensorOp/LinearTrainingStep runtime paths: CUDA field matmul edge
> cases, exact linear tensor ops, canonical matmul backend parity, linear-step backend parity, and live
> miner-role receipt submission for those two paths through `tvmd miner run --device cuda:N` runtime
> config pass under `--features cuda-kernels`. CUDA graph receipt evidence now covers the current local
> synthetic GraphExecution shape (`add` -> `relu`) plus a focused supported-op CUDA graph
> (`matmul`/`add`/`sub`/`mul`/`div`/`clamp`/`sum`/`mean`/`reshape`/`squeeze`/`unsqueeze`/`slice`/`split`/`tril`/`triu`/`concat`/`stack`/`broadcast`/`transpose`/`scalar_mul`/`relu`/`identity`/`neg`/`abs`/`sign`/`eq`/`gt`/`lt`/`ge`/`le`/`where`)
> through the CUDA miner backend with bit-exact CPU/GPU receipt roots. CUDA conformance reporting is
> limited to the exercised field subset instead of over-claiming binary op broadcasting, vector clamp
> bounds, fixed-point division/comparisons/selection/clamp/broadcast/mean/reshape/squeeze/unsqueeze/slice/split/tril/triu/concat/stack, fixed-point or broadcast
> reductions, bool masks, or the full CPU reference profile. The full frozen-registry CUDA vector suite and
> Tier-C/transcendental vector references remain TODO before claiming complete §3.3 coverage for every
> runtime.

### 3.4 Where determinism is free, and where it is earned
A subtle but load-bearing consequence of working in `F_p`: **for any computation that stays inside the field, the result is a unique field element independent of evaluation/reduction order.** Field addition and multiplication are exactly associative and commutative, so a matmul `C = A·B`, a `sum`, or a `mean` numerator has one canonical value no matter what order a runtime accumulates it in.

This splits the determinism problem cleanly:

- **Inside the field, cross-hardware determinism is automatic.** Two correct runtimes agree on `field`-dtype results even if one accumulates serially and another uses a parallel reduction tree. Freivalds (§6) and exact-replay committees (§8.1) verify the *value*, which is order-invariant. A fixed serial reduction order is therefore **not** required for `field` dtype, and reduction order is left free (a balanced tree is admissible and GPU-friendly).
- **At the field boundary, determinism must be specified explicitly.** Order and representation become observable only when a step leaves exact field arithmetic:
  - **Fixed-point (`fixedNN_s`)**: intermediate magnitude can exceed the logical scale and each rescale rounds, so reduction order *is* observable. For `fixedNN` reductions the canonical order is **fixed ascending index order** with round-half-to-even at each declared rescale point (§4.8). A tree reorder is a fault.
  - **Tier-C approximations** (transcendental, order statistics): determinism comes from a canonical integer reference algorithm (§4.8), never from arithmetic associativity.

> Normative refinement: each reduction/contraction op carries a canonical **reduction-order class** that is a pure function of its output dtype — `OrderFree` for `field`, `AscendingFixedPoint` for `fixedNN`. Because it is derived (not authored), it does not affect `graph_id`; it constrains conformant runtimes and is exactly what the §3.3 conformance vectors pin. This is the only place a computation's "schedule" is consensus-relevant: it is earned precisely at the field boundary and free everywhere inside it. The earlier blanket "fixed ascending reduction order" framing is superseded by this dtype-scoped rule.

### 3.5 Datatype definition framework
A consensus datatype is defined not by a host-language type but by a triple: **(value embedding into `F_p`, per-op lowering rules, conformance vector set)**. Adding or changing a dtype is a protocol-version change that ships all three:

- **Embedding** — how values map into/out of `F_p` (raw `field`; `round(v·2^s) mod p` for `fixedNN_s`; sign-extended modular embedding for ints; §4.1).
- **Lowering** — how each admitting op consumes/produces the dtype: rescale-after-multiply, division mode (modular inverse for `field` vs. signed reciprocal for `fixedNN`), reduction-order class (§3.4), rounding (round-half-to-even), saturation/clamp.
- **Conformance vectors** — the §3.3 suite is the *normative definition* of the dtype's behavior, not merely a test of it; a runtime is conformant for a dtype iff it reproduces every vector bit-exactly on every backend (CPU reference and any GPU path).

> This makes "is this committed value correct?" a closed question: the answer is "does it equal the canonical lowering," and the conformance suite *is* the canonical lowering made executable. It is also the clean extension path — a new exact dtype (another modulus, a different fixed-point scale) is a new triple, not a kernel rewrite.

---

## 4. The Workload Primitive: Tensor IR (full specification)

Workloads are expressed as a **content-addressed tensor IR graph**. This section specifies the IR completely and self-containedly: its value model, node structure, canonical encoding, structural validity rules, the frozen op registry, and the determinism obligations each op must satisfy. Nothing in the IR refers to any external implementation; a conformant runtime is defined entirely by this section plus the determinism contract (§3).

### 4.1 Value model
Every IR value is a **tensor**: a dense, row-major array of field elements with a declared shape and a numeric encoding.

- **`dtype`** is one of:
  - `field` — a raw element of `F_p`.
  - `fixedNN_s` — a real number encoded as a fixed-point field element with `NN` logical bits and binary scale `s` (value `v` represents `v·2^{−s}`, stored as `round(v·2^s) mod p`).
  - `int64`, `int32`, `int8`, `uint8`, `bool` — exact integers embedded in `F_p` (used for indices, masks, packed bytes). Integer values MUST lie in `[0, p)` after embedding; negative integers embed as `x mod p`.
- **Shape** is a list of non-negative integers. A declared shape dimension of `-1` in an *input* `TensorSpec` means "unconstrained at definition time, fixed by the bound input." All other shapes are concrete.
- There is **no floating-point dtype** in the consensus path. Floating point may exist only inside a miner's accelerator as an optimization, never as a committed value (§3.2).

```text
TensorSpec { name: string, shape: [int], dtype: DType, scale: int }   // scale used iff dtype is fixedNN
ParamSpec  { name: string, type: string }                              // declares a bound parameter slot
```

### 4.2 Refs (how an op names its inputs)
An op argument or keyword value is either a literal or a **ref**. A ref is a tagged object with a `kind`:

| `kind` | Fields | Resolves to |
|---|---|---|
| `input` | `name` | the bound graph input tensor `name` |
| `op` | `id`, `idx` (default 0) | output `idx` of the op with that `id` |
| `param` | `name` | the bound parameter `name` |
| `const` | `value` | an inline literal (scalar, list, or small tensor of field/int values) |
| `const_blob` | `uri`, `shape`, `dtype` | a large constant tensor fetched by content address (`uri` is a tensor commitment, §5.1), shape/dtype asserted on load |

Refs make data flow explicit and pure: an op's inputs are fully determined by graph inputs, params, consts, and the outputs of strictly-earlier ops. This SSA-like, side-effect-free structure is what makes a single op re-executable in isolation during a fraud proof (§8.2).

### 4.3 Op
```text
Op {
  id:     int           // equals the op's index in Graph.ops (0-based, gap-free)
  op:     string        // a name in the frozen registry (§4.7)
  args:   [Ref]         // positional inputs, count fixed by the op's arity
  kwargs: { string: (Ref | literal) }   // keys restricted to the op's declared kwarg set
  out:    [TensorSpec]  // declared outputs; length matches the op's output count
}
```
Multi-output ops (e.g. `split`, `topk`, `qr`) declare multiple `out` specs and are referenced via `{kind:"op", id, idx}`.

### 4.4 Graph
```text
Graph {
  ir_version: int
  inputs:  [TensorSpec]                 // names unique
  params:  [ParamSpec]                  // names unique
  ops:     [Op]                         // topologically ordered: op i may only ref ops < i
  outputs: [{ name: string, ref: Ref }] // named graph outputs, each pointing at a ref
}
```

### 4.5 Canonical encoding & `graph_id`
- The graph serializes to **canonical JSON**: object keys sorted lexicographically, no insignificant whitespace, UTF-8, integers as decimal, field elements as fixed-width lowercase hex.
- `graph_id = SHA256(canonical_json(graph))`.
- Jobs reference a graph **only** by `graph_id`; the program is therefore immutable and content-addressed. Two graphs with identical `graph_id` are identical programs by construction.

> Status: current TensorOp and LinearTrainingStep jobs store their validated canonical graph bodies in
> chain state keyed by `graph_id`, persist those bodies through node-store snapshots, commit them in the
> state root, and can serve them through the existing `RequestProgram`/`ProgramResponse` libp2p path.
> Current canonical TensorOp and LinearTrainingStep receipts derive `trace_root` from exact execution of
> their canonical TensorGraph op traces. Generic `GraphExecution` jobs and receipts can reference
> registered canonical graph bodies, execute locally through miner role loops, attest through validator
> role loops, cross the shared p2p/node payload path, settle through delayed receipt rewards, and surface
> through explorer HTTP/WebSocket plus local checker evidence. CUDA graph execution now covers the current
> local synthetic GraphExecution shape and a supported same-shape field-op graph including elementwise
> `mul`/`div`, scalar-bounds field `clamp`, deterministic field `sum`, field `mean`, field
> `reshape`/`squeeze`/`unsqueeze`/`slice`/`split`/`tril`/`triu`/`concat`/`stack`, unary field `broadcast`, exact field `identity`/`neg`/`abs`/`sign`, field comparison masks `eq`/`gt`/`lt`/`ge`/`le`, and mask-fed
> field `where` through miner-role `cuda:N` backend selection;
> broader CUDA graph op coverage
> and public deployed graph evidence remain TODO.

### 4.5.1 Canonical normalization before `graph_id`
`graph_id` content-addresses the *program*, and rewards, blockspace accounting, and work-dedup all key off it (§11–§12). Syntactic hashing alone lets an adversary mint many distinct `graph_id`s for the **same computation** — reordered commutative operands, dead nodes, renamed intermediates, permuted independent ops — and thereby claim the same useful work multiple times, occupy multiple blockspace slots, or evade caching. To make `graph_id` address semantics rather than syntax, a graph is **normalized** before hashing by a fixed, deterministic pass pipeline:

1. **Dead-op elimination** — drop ops not reachable from `outputs`.
2. **Common-subexpression elimination** — merge ops with identical `(op, args, kwargs, out)` after their predecessors are normalized.
3. **Commutative operand canonicalization** — for declared-commutative ops (`add`, `mul`, `eq`; `concat`/`stack` only when the spec marks the axis order-insensitive) sort `args` by a canonical ref key.
4. **Canonical topological renumbering** — assign `id`s by a deterministic topological order (e.g. Kahn with a total tie-break on `(op, canonical-arg-keys)`), preserving the §4.6 dense, gap-free invariant.

`graph_id = SHA256(canonical_json(normalize(graph)))`. Normalization MUST be **idempotent** (`normalize(normalize(g)) == normalize(g)`) and **semantics-preserving** (it never changes any output value for any admissible input); both properties are conformance-tested. Two graphs computing the same function therefore share a `graph_id` by construction, which is the anti-grinding property §10/§12 rely on.

> Scope note (design): normalization is purely *structural* — it never reorders a reduction's internal accumulation (that is governed by §3.4). v0 MAY ship a reduced pipeline (DCE + canonical renumbering) and treat CSE / commutative canonicalization as a hardening follow-up, provided the implemented subset is still idempotent and semantics-preserving. Until the full pipeline lands, the residual grinding surface is tracked in §14/§16.

### 4.6 Structural validity rules
A graph is **structurally valid** iff all of the following hold (checked before any execution; an invalid graph cannot appear in a job):
1. `ops[i].id == i` for all `i` (dense, ordered ids).
2. Every `{kind:"op", id}` ref satisfies `id < i` for the referencing op `i` (acyclic, topologically ordered).
3. Every `{kind:"op", id, idx}` ref has `idx < len(ops[id].out)`.
4. Every `input`/`param` ref names a declared input/param.
5. `len(args)` equals the op's arity, or the op is variadic (arity `n`) and `len(args) ≥ 1`.
6. Every key in `kwargs` is in the op's declared kwarg set; required kwargs are present.
7. Every declared output `ref` resolves to a defined value.
8. All `inputs` and `params` names are unique; output `name`s are unique.
9. Shapes and dtypes are consistent with each op's typing rule (shape inference must succeed and match declared `out` specs).

> The flat, `id`-indexed, pure-ref DAG is exactly what makes interactive fraud proofs (§8) tractable: any op's inputs are fully determined by earlier committed op outputs, so a single op can be re-executed in isolation.

### 4.7 Frozen op registry
The op set is **frozen per protocol version**. Each op has a fixed arity, a fixed kwarg set, an output count, a verification **tier** (§4.8), and a **canonical `F_p` semantics** (the determinism obligation). Arity `n` denotes variadic. Outputs `1` unless noted.

**Elementwise binary** (Tier B) — arity 2, no kwargs, broadcasting per §4.8:
`add`, `sub`, `mul`, `div`*, `pow`*

**Elementwise unary** (arity 1, no kwargs):
- Tier B (exact in `F_p`): `neg`, `abs`, `sign`, `identity`, `round`, `relu`†
- Tier C (transcendental — require canonical fixed-point approximation, §4.8): `exp`, `log`, `sqrt`*, `sin`, `cos`, `sigmoid`, `tanh`, `gelu`, `silu`

**Comparison** (Tier B) — arity 2, no kwargs, output `bool`:
`gt`, `lt`, `ge`, `le`, `eq`

**Selection / shaping** (Tier B) — exact, structural:
| op | arity | kwargs | notes |
|---|---|---|---|
| `where` | 3 | — | `cond` is `bool` |
| `clamp` | 1 | `min`, `max` | |
| `cast` | 1 | `dtype` | re-encodes; scale changes are canonical rounding |
| `reshape` | 1 | `shape` | |
| `transpose` | 1 | `dims` | permutation |
| `broadcast` | 1 | `shape` | |
| `squeeze` | 1 | `dim` | |
| `unsqueeze` | 1 | `dim` | |
| `slice` | 1 | `dim`, `start`, `end` | |
| `concat` | n | `dim` | |
| `stack` | n | `dim` | |
| `split` | 1 | `sizes`, `dim` | outputs = len(`sizes`) |
| `tril` | 1 | `diagonal` | |
| `triu` | 1 | `diagonal` | |

**Linear / bilinear** (Tier A — Freivalds, §6):
| op | arity | kwargs | notes |
|---|---|---|---|
| `matmul` | 2 | — | the canonical Tier-A op |
| `einsum` | 2 | `equation` | **only** contraction/permutation equations are Tier A; others reduce to Tier B/C |

**Reductions**:
| op | arity | kwargs | tier | notes |
|---|---|---|---|---|
| `sum` | 1 | `dim`, `keepdim` | B | exact; random-linear checkable |
| `mean` | 1 | `dim`, `keepdim` | B | fixed-point division by count is canonical |
| `max` | 1 | `dim`, `keepdim` | C | data-dependent |
| `min` | 1 | `dim`, `keepdim` | C | data-dependent |

**Indexing / gather-scatter** (Tier C — need index-consistency arguments, §7 TODO):
| op | arity | kwargs | notes |
|---|---|---|---|
| `gather` | 2 | `dim` | index tensor is `int64` |
| `scatter` | 3 | `dim` | |
| `embedding` | 2 | — | `embedding(weight, ids)`; Tier B w/ index-consistency |

**Generators** (arity 0):
| op | arity | kwargs | tier | notes |
|---|---|---|---|---|
| `arange` | 0 | `start`, `end`, `step`, `dtype` | B | exact, deterministic |
| `full` | 0 | `shape`, `value`, `dtype` | B | exact |
| `normal` | 0 | `seed`, `shape`, `dtype` | C | **canonical PRNG over `F_p` required** (§4.8) |
| `uniform` | 0 | `seed`, `shape`, `dtype` | C | canonical PRNG required |

**Order statistics & normalization** (Tier C):
| op | arity | kwargs | outputs | notes |
|---|---|---|---|---|
| `sort` | 1 | `dim`, `descending` | 1 | stable order canonical |
| `topk` | 1 | `k`, `dim` | 2 (values, indices) | tie-break canonical |
| `softmax` | 1 | `dim` | 1 | transcendental → fixed-point approx |
| `log_softmax` | 1 | `dim` | 1 | transcendental |
| `layer_norm` | 3 | `eps` | 1 | `(x, weight, bias)`; involves `sqrt` |
| `rmsnorm` | 2 | `eps` | 1 | `(x, weight)`; involves `rsqrt` |
| `cross_entropy` | 2 | `ignore_index` | 1 | `(logits, targets)`; transcendental |

**Quantization / packing** (Tier B/C, exact integer arithmetic):
| op | arity | kwargs | outputs | notes |
|---|---|---|---|---|
| `quantize_int8_per_channel` | 1 | `dim` | 2 (q, scale) | round-half-even canonical |
| `dequantize_int8_per_channel` | 2 | — | 1 | exact |
| `quantize_pack_int8` | 1 | `dim` | 1 | byte-exact packing |
| `unpack_dequantize_int8` | 1 | `dim`, `shape`, `scale_dim` | 1 | byte-exact |

**Linear algebra & data** (Tier C):
| op | arity | kwargs | outputs | notes |
|---|---|---|---|---|
| `qr` | 1 | — | 2 (Q, R) | iterative → **excluded from v0 consensus** until a canonical fixed-point algorithm + tolerance-free check is specified |
| `data_indexer` | 1 | `B`, `T`, `mb_seed` | 2 (x, y) | canonical PRNG selects minibatch windows from a token stream |

\* `div`, `pow`, `sqrt`: division in `F_p` is either modular inverse (exact, for `field` dtype) or canonical signed fixed-point reciprocal division (exact integer quotient rescale for `Fixed32`). Roots remain Tier C until their canonical fixed-point references are published. The IR distinguishes the modes by operand dtype; mixing is a typing error.
† `relu` is exact in fixed-point (`max(x,0)`), so it is Tier B despite living in the activation family.

> Permissionless deployment of new ops is **not** allowed: a malicious or ill-defined op could be unverifiable. New ops are added only by protocol upgrade, each shipping (a) a canonical `F_p` semantics, (b) a verification tier + verifier, and (c) a soundness bound.

### 4.8 Determinism obligations per op class
Each op MUST have a single canonical `F_p` result for given inputs, identical on every conformant runtime:
- **Exact ops** (Tier A/B integer & fixed-point arithmetic, shaping, comparisons): the field/fixed-point operation is exact; the only freedom is rounding after scale changes, which is **round-half-to-even** by spec, and broadcasting, which follows the canonical NumPy-style rule with a fixed alignment.
- **Reductions**: the reduction-order class is set by output dtype (§3.4). For `field` dtype the result is order-invariant, so any accumulation order — including hardware reduction trees — is admissible. For `fixedNN` dtype the order is **fixed ascending index order** with round-half-to-even at each rescale, and hardware reduction-tree reordering is a fault.
- **Transcendental ops** (`exp`, `log`, `sqrt`, trig, `sigmoid`, `tanh`, `gelu`, `softmax`, `log_softmax`, `cross_entropy`, `rmsnorm`, `layer_norm`): defined by a **canonical fixed-point reference algorithm** (specified polynomial/LUT + fixed iteration count + canonical rounding), not by an IEEE library call. Two runtimes agree because they run the *same* integer approximation, not because their floats happen to match. Until that reference is published per op, the op is **not consensus-eligible** (Tier C, deferred).
- **PRNG ops** (`normal`, `uniform`, `data_indexer`): use a **canonical, seeded, integer PRNG over `F_p`** (e.g. a counter-based PRF), never a platform RNG. Output is a pure function of `(seed, shape, dtype)`.
- **Order-dependent ops** (`sort`, `topk`, `max`, `min`, `argmax`-like): ties are broken by **lowest index**, making the result a total function of the input.

> TODO: publish the canonical fixed-point reference for each transcendental op with its error bound and a conformance vector set (§3.3). This is the gating work item for admitting Tier-C ops to consensus.

### 4.8.1 Canonical fixed-point reference: the exp-family
This is the first published canonical reference for transcendental ops, covering `exp` and everything that reduces to it: `sigmoid`, `tanh`, `silu`, `gelu`, and `softmax`. It is what lets a miner train on **any** float hardware while the chain verifies a hardware-independent fixed-point recomputation (§1.3, §3.4): the reference is pure integer arithmetic, so every conformant runtime (CPU reference, CUDA) produces identical bits and can be checked by the §3.3 conformance suite.

**Evaluation domain.** All intermediate math runs in a fixed **Q-format**: a signed integer `X` represents the real value `X · 2^{−F}` with `F = 32` fractional bits. Inputs in `fixedNN_s` are converted to Q-format by a round-half-to-even rescale (`s → F`), and outputs are rescaled back (`F → s`). Every multiply rounds half-to-even immediately after the product (`q_mul(a,b) = round½even(a·b / 2^F)`); every divide computes `round½even(a·2^F / b)`. There is no floating point anywhere in the reference.

**`exp(x)` (the primitive).** Defined for all `x`; the core is the non-positive branch:
- Underflow: if `x ≤ −64`, `exp(x) := 0` (canonical; `exp(−64) ≈ 1.6·10⁻²⁸` is sub-ulp at any consensus-sane scale).
- Range reduction by squaring: let `r = round½even(x / 2^{M})` with **`M = 10`**, so `|r| ≤ 2^{−4}`. Then `exp(x) = exp(r)^{2^{M}}`, computed by squaring `M` times.
- Reduced evaluation: `exp(r)` is the **degree-5 Taylor polynomial** `1 + r + r²/2 + r³/6 + r⁴/24 + r⁵/120` in Horner form. On `|r| ≤ 2^{−4}` the truncation error is `< r⁶/720 ≈ 10⁻⁹`, far below Q-format resolution.
- Positive branch: `exp(x) = 1 / exp(−x)` for `x > 0` (overflow if `exp(−x)` underflowed to 0 — saturation is a TODO).

**Derived ops** (all from `exp`, branching on sign so `exp` is only ever evaluated on a non-positive argument):
- `sigmoid(x) = 1/(1+exp(−x))` for `x ≥ 0`, else `exp(x)/(1+exp(x))`.
- `tanh(z) = 2·sigmoid(2z) − 1`.
- `silu(x) = x · sigmoid(x)`.
- `gelu(x) = ½·x·(1 + tanh( c·(x + a·x³) ))` with canonical Q-format constants `c = round(√(2/π)·2^{32}) = 3 426 888 095` and `a = round(0.044715·2^{32}) = 192 049 463` (the standard tanh approximation).
- `softmax(x, dim)`: subtract the per-group max (order-stable, exact), exponentiate each non-positive shifted logit, sum in **ascending index order** (the `AscendingFixedPoint` reduction class, §3.4), then divide each by the sum. Shift-invariant by construction.

**Two distinct gates — determinism vs. accuracy.** These are deliberately separate, because conflating them is a mistake:
- **Determinism (consensus-relevant).** The suite (§3.3) carries a Fixed32 vector per op whose expected output is the canonical reference's own output, checked by **exact equality**. This pins CPU == CUDA == every runtime and freezes the reference via the suite hash, so any change to the algorithm or a constant is a detectable protocol change. This is the only gate consensus needs: for the chain, "correct" *is* "equals the canonical reference," so expected values **must** be reference-derived. (Exact-match "golden" vectors against an external truth are impossible here — a transcendental reference is itself an approximation and will not equal `round(true value)` to sub-ulp everywhere.)
- **Accuracy (usefulness / pre-freeze diligence).** A separate, **tolerance-based** harness (test-only `f64`, never in the consensus path) measures the reference's max error against the `f64` evaluation of the same algorithm, isolating quantization error. This does not affect consensus security; it validates that the approximation is good enough to be worth freezing as immutable protocol vocabulary, and that miners training against it get faithful gradients.

**Worst-case error decomposition (toward a full-domain bound).** The error at output scale `s` splits into three independent, separately-bounded parts:

```
error(scale s)  ≤  0.5·2^{−s}            (output requantization Q32 → s; ½ ulp, exact by construction)
                +  E_q32                  (internal algorithm error in Q32; SCALE-INDEPENDENT)
                +  input-representation    (inherent to a fixed-point input; not the op's error)
```

Two facts make this close to a full-domain proof rather than a finite sweep:
- **Scale is not a real axis.** The algorithm runs entirely in Q32 regardless of `s`; only the in/out conversions touch `s`, and each is `±½ ulp` by construction. So bounding all scales reduces to bounding the single scale-independent constant `E_q32` plus the closed-form `0.5·2^{−s}` term.
- **The active input domain is bounded.** Range reduction caps the reduced argument to `|r| ≤ 2^{−4}` (analytic Taylor tail `< r⁶/720 ≈ 10⁻⁹`), and outside `|x| ≳ 64` the functions saturate *exactly* (`exp→0`, `sigmoid→{0,1}`, `tanh→±1`, `gelu→{0,x}`), so the tail is exact and only `|x| ≤ 64` carries approximation.

`E_q32` is **exhaustively proven** at the conformance scale: because the field bounds both the scale and the value range, the representable input set over `|x| ≤ 64` at `s = 16` is *finite* (all `8.4·10⁶` `Fixed32` values), so enumerating every one of them is a complete proof for that scale — there is no continuum to sample. The oracle is `f64` evaluation of the exact formula; `f64` libm error here is `≤ ~10⁻¹¹` absolute (`~10⁻¹⁵` relative on `|value| ≤ 16384`), `≥ 3` orders of magnitude below the result, and is carried as an explicit margin in the proof:

| op | `E_q32` | metric |
|---|---|---|
| `exp` | `≤ 1·10⁻⁵` | relative, over its representable range (`exp(x) ≥ 2^{−16}`) |
| `sigmoid` / `silu` / `gelu` | `≤ 1·10⁻⁷` | absolute (`≈ 2^{−24}`) |
| `tanh` | `≤ 1.5·10⁻⁷` | absolute |

So e.g. at `s = 16` (ulp `≈ 1.5·10⁻⁵`) the bounded ops are dominated by the `0.5·2^{−16} ≈ 7.6·10⁻⁶` requantization term, with internal error only `~5·10⁻⁸` on top — matching the directly-measured scale-16 figure.

**`softmax` (multi-input)** is not exhaustively enumerable, so its bound is by composition: each `exp` term carries `E_q32(exp)` relative error, the ascending-order sum of `n` terms adds `≤ (n−1)` accumulation roundings, and the final division adds `≤ ½ ulp`; the normalized result is therefore within `≈ n·E_q32(exp) + O(2^{−s})`.

**Admission.** These ops remain **Tier C and gated out of consensus** (`consensus_admitted = false`): a deterministic, accurate reference satisfies the §3.3 *determinism* obligation but not the §6–§8 *verifiability* obligation. They become consensus-eligible only once a verifier exists — redundancy/committee (§8.1) now, interactive fraud proofs over the trace (§8.2) for 1-of-N honesty.

**`log` and `sqrt`** are also provided as canonical Q-format references: `sqrt(x)` by round-to-nearest integer square root (`sqrt(x)·2^F = isqrt(x_q·2^F)`); `ln(x)` by range reduction `x = m·2^e` (`m ∈ [1,2)`) with `ln(m) = 2·atanh((m−1)/(m+1))` evaluated as an odd-power series to `t^13` plus `e·ln2`.

**Compositions (transformer building blocks).** `layer_norm` (`y = (x−mean)/sqrt(var+eps)·weight + bias`) and `rmsnorm` (`y = x/sqrt(mean(x²)+eps)·weight`) are now assembled as canonical references **composed from the primitives** — mean/variance via exact round-half-to-even integer division, plus center, square, `sqrt`, normalize (`div`), and the affine `mul`/`add`. This proves the small primitive set assembles into a real transformer block. They are frozen-registry ops (arity 3 / 2, `eps` kwarg; Tier-C, **not** consensus-admitted) and are well-formed, stably-addressed (`graph_id`) graph citizens, verified within `3·10⁻³` of an `f64` reference and bit-exact-deterministic. The IR `mean` op now performs exact round-half-to-even integer averaging for `fixedNN` (previously a field inverse, which is the modular inverse — correct only for `field` dtype), so `layer_norm` is also expressible and executes correctly as a **raw multi-op graph** of primitives (`mean`/`broadcast`/`sub`/`mul`/`mean`/`add`/`sqrt`/`div`/`mul`/`add`), verified within `5·10⁻³` of an `f64` reference. (Executing such a Tier-C graph through the consensus interpreter still awaits Tier-C admission via a verifier.) `log_softmax`/`cross_entropy` remain.

**GPU determinism (CUDA).** The exp-family (`exp`/`sigmoid`/`tanh`/`silu`/`gelu`/`softmax`) is also implemented as CUDA device kernels in the exact same Q-format integer arithmetic (`__int128`, round-half-to-even), and the GPU backend's conformance profile now requires each GPU kernel to be **bit-exact** with the CPU reference before it is admitted to `passed_ops` — i.e. the GPU path has joined the determinism gate. Verified on the local A100×8 box: `cpu_and_gpu_backends_match_fixed_exp_family` and `gpu_conformance_profile_includes_exp_family` pass under `--features cuda-kernels`.

> Status (implemented): `exp`, `log`, `sqrt`, `sigmoid`, `tanh`, `silu`, `gelu` (arity-1), `softmax` (`dim`), and the `layer_norm`/`rmsnorm` compositions (`eps` kwarg) execute through the exact interpreter on `Fixed32` tensors via the canonical Q-format references above, are present in the frozen registry as Tier-C `CanonicalReferenceRequired` ops, and (for the non-composite ops) are covered by Fixed32 determinism vectors gated through the CPU reference profile; the exp-family additionally has bit-exact CUDA kernels gated through the GPU profile. Unit-tested: sanity goldens (`exp(0)=1`, `sigmoid(0)=½`, `tanh(0)=0`, `gelu(0)=0`, `sqrt(4)=2`, `ln(1)=0`, softmax uniform/shift-invariance/monotone-sigmoid), CPU/GPU bit-exact determinism, a fast scale-16 accuracy check, a dense cross-scale `E_q32` sweep, and an `#[ignore]`d **exhaustive** scale-16 proof enumerating all `8.4·10⁶` representable inputs (complete for that scale) producing the bounds tabled above.
> TODO: (a) extend the exhaustive proof from the conformance scale to every field-representable scale (`s ≤ ~24`, `~2·10⁹` inputs) and add a symbolic bound that removes even the negligible bounded `f64`-oracle margin, plus a formal `softmax` composition proof; (b) input saturation instead of overflow errors; (c) CUDA `log`/`sqrt` kernels and exhaustive `E_q32`/error bounds for `log`/`sqrt`; (d) `log_softmax`/`cross_entropy` compositions (`layer_norm`/`rmsnorm` done, and now expressible as raw multi-op graphs after the Fixed32 `mean` fix); (e) consensus admission via the §8.1 committee path is now live end to end (admission + committee execution + deterministic agreement root + honest-majority `settle_epoch` gate on `redundancy_k`, §8.1 status note), so Tier-C receipts can settle under honest-majority committee agreement; §8.2 fraud proofs remain the planned strengthening.

### 4.9 Canonical jobs (v0)
1. **`TensorOp`** — a single `matmul` `C = A·B` over `F_p`. The minimal verifiable unit, fully Freivalds-checkable.
2. **`LinearTrainingStep`** — forward (`X·W`), fixed-point loss, backward (`dW = Xᵀ·dY`), optimizer update (`W' = W − η·dW`). A real learning step whose pieces are all matmul-like → Freivalds-verifiable.

**v0 admits only ops whose canonical `F_p` semantics are fully specified and exactly verifiable:** Tier A (`matmul`, contraction `einsum`), the exact Tier B ops (elementwise integer/fixed-point arithmetic, `relu`, shaping, `sum`/`mean`, comparisons, exact quantization), plus whatever minimal set `LinearTrainingStep` requires. Transcendental and order-dependent ops are carried in the registry as the workload vocabulary but are gated out of consensus until §4.8 references and their verifiers exist (§13 roadmap).

### 4.10 The op DAG as the canonical execution trace
The IR is deliberately a flat, dense-`id`, gap-free, SSA-like, side-effect-free DAG (§4.3–§4.6). That structure *is* a linear execution program: op `id` is a program counter, each op reads only already-defined values and writes its `out` slots, and `trace_root = MerkleRoot([op_output_commit(i)])` (§5.2) commits the full instruction-by-instruction execution. **No separate bytecode is required — the canonical graph is the canonical trace**, which is why the verification ladder needs no per-op-type dispute machinery.

Consequences the rest of the spec depends on:

- **Uniform dispute granularity.** Every step has the same shape `(pc = i, op, input refs → committed inputs, committed outputs)`, so the §8.2 bisection game binary-searches a single integer `pc` and isolates exactly one op regardless of which ops a graph uses.
- **Single-step re-execution is well-defined.** Because inputs are pure refs to earlier committed outputs, the §8.2 step-4 referee re-runs one op on agreed inputs with no hidden state and rules `O(one op)`.
- **DA addresses the trace.** Each `op_output_commit(i)` is independently openable (§5.2), so availability sampling and dispute openings address execution at op granularity (§9).

> Multi-output ops commit a vector `op_output_commit(i) = MerkleRoot([commit(out_j)])`. The bisection Merkle layout pads `#ops` to a power of two (§16 edge case). This subsection is descriptive of the existing IR + `trace_root` design, not a new artifact: it names the property that makes §8.2 tractable.

### 4.11 Verification-class algebra and region fusion
Each op has a verification class — Tier A → full Freivalds (§6); affine Tier B → random-linear (§7); exact non-affine Tier B → deterministic replay; index ops → index-consistency; Tier C → redundancy/fraud-proof (§8). A validator that checked every op independently would pay per-op verifier cost growing with `#ops`. Because the classes compose, contiguous regions can be checked **once**:

- **Affine ∘ affine = affine.** A maximal connected region of random-linear-checkable ops (`add`, `sub`, const `mul`, `reshape`/`transpose`/`broadcast`, `sum`/`mean`) collapses to a single affine relation verified with one random-linear check (§7), soundness still `≥ 1 − 1/p` per rep over the region.
- **Freivalds absorbs surrounding affine.** A `matmul` composed with affine pre/post-ops is verified as one linear check on the fused relation.
- **Poisoning classes don't fuse.** Any op requiring index-consistency or canonical-reference / redundancy verification poisons its region: the region splits at that op and the poisoning op is verified by its own class. Fusion never upgrades a weaker class into a stronger guarantee.

The fused class of a region is the **lattice join** of its members over the order `Freivalds/RandomLinear ⊑ DeterministicReplay ⊑ IndexConsistency ⊑ CanonicalReference/Redundancy`. `checks_root` (§11) commits the *canonical region decomposition* used, so every other validator recomputes the same regions deterministically from the normalized graph (§4.5.1).

> Verifier-cost relevance: region fusion is what keeps on-chain verification sublinear in `#ops` for affine-heavy graphs, directly improving the §12 `bond ≥ gain-from-fraud` margin (cheaper honest verification) and block throughput. The decomposition is a deterministic pass over the normalized graph, so it cannot be ground to any party's advantage. Design status: the per-op verification class exists in the frozen registry today (§7 status); the *fusion/region* algebra is a design addition tracked in §16.

---

## 5. Commitments & Records (commitments on-chain, tensors off-chain)

The chain never stores full tensors. On-chain: job defs, receipts, attestations, block metadata, reward/stake/slash state. Off-chain (served by miners, sampled by validators): tensor data, activations, traces.

### 5.1 Tensor commitment
A tensor commits as a **Merkle root over fixed-size chunks** of its canonical `F_p` byte encoding: `tensor_commit = MerkleRoot(chunks)`. This supports chunk-level availability sampling and selective disclosure.

### 5.2 Trace commitment
An execution produces a **trace root**: `trace_root = MerkleRoot([ op_output_commit(i) for i in topo_order ])`, where `op_output_commit(i)` commits the output tensor(s) of op `i`. The trace root is the anchor for interactive fraud proofs (§8). The local reference now derives this root from exact IR execution, verifies per-op Merkle openings for TensorOp, LinearTrainingStep, and GraphExecution receipts, and carries those openings over bounded libp2p request-response for sampling.

### 5.3 Records (all asymmetrically signed; sr25519/ed25519)

```text
Job {
  graph_id: Hash
  input_commitments: [TensorCommit]
  params: { scales, op-tier policy, ... }
  verification_policy: { freivalds_reps, sample_rate, redundancy_k, ... }
  deadline, created, job_id
  source_sig
}

Receipt {
  job_id, graph_id
  miner_pubkey
  input_commitments:  [TensorCommit]   // must equal Job's
  output_commitments: [TensorCommit]
  trace_root: Hash                       // §5.2
  compute_metadata: { claimed_flops, time }   // telemetry only, never consensus
  bond: Stake                            // slashable
  miner_sig
}

Attestation {
  receipt_id
  validator_pubkey
  result: Pass | Fail | Inconclusive
  method: Freivalds | RandomLinear | Redundant | FraudProof
  evidence: { challenge_vectors, sampled_indices, ... }  // reproducible
  validator_sig
}
```

Every record's signed body is canonical JSON / SSZ; `*_id = SHA256(canonical(body))`.

> Status: the local reference computes current canonical TensorOp and LinearTrainingStep receipt
> `trace_root` values from the exact TensorGraph execution trace for the corresponding graph ID. Registered
> canonical graph bodies can also be used by `GraphExecution` jobs/receipts, and local role-owned synthetic
> production now executes and attests an exact Tier-B graph from node-local artifacts. The shared node
> payload path now keeps external graph jobs pending until their program bodies are registered, and focused
> libp2p evidence fetches externally supplied graph bodies plus input tensor artifacts before applying the
> graph job payload. Runtime ingest now fetches missing pending graph-program bodies over the bounded
> `RequestProgram` path before retrying queued graph jobs; miner role loops fetch missing graph input and
> `const_blob` tensors before execution; and validator role loops fetch graph input, output, and
> `const_blob` tensors before attestation. Local CPU graph receipt verification now has direct receipt
> scenarios for every consensus-admitted frozen-registry op, but CUDA/deployed graph verification evidence
> and broader public-runtime measurements remain open.

> **Crypto is asymmetric, full stop.** No HMAC/shared-secret signing anywhere in the consensus path. Identities are on-chain accounts.

---

## 6. Verification Ladder — Level 1: Freivalds (linear/bilinear)

For `C = A·B` (`A: m×k`, `B: k×n`), instead of recomputing (`O(mnk)`), a validator checks with a random vector `r ∈ F_p^n`:

```text
A·(B·r)  ==  C·r        cost: O(mn + nk + mk) = O(n^2)-class
```

- Soundness per repetition over `F_p`: a wrong `C` passes with probability `≤ 1/p` for a uniformly random `r` (Freivalds). With `t` independent reps, failure `≤ p^{-t}`. Pick `t` so `p^{-t}` is negligible (§12).
- **Full-output Freivalds** (checks all rows/cols) is mandatory for **block-eligible** receipts.
- **Row-sampled Freivalds** is cheaper but does **not** soundly catch sparse single-row corruption; it is allowed only for *telemetry/large-job triage*, never as the sole validity check for block eligibility. (Documented soundness gap — do not over-claim.)

Forward and backward passes of `LinearTrainingStep` are matmul-like and use Freivalds directly.

---

## 7. Verification Ladder — Level 2: Random-Linear Checks (affine/elementwise)

For elementwise/affine relations `Y = f(X)` where `f` is affine over `F_p` (e.g. `add`, `mul`-by-const, `reshape`, fixed-point `layer_norm` linear part), a random linear combination of the asserted equations gives algebraic coverage analogous to Freivalds:

```text
draw r; check  <r, Y - f(X)>  == 0   over F_p
```

This catches any nonzero error with probability `≥ 1 − 1/p` per rep. Used for Tier B ops that are not bilinear. Adjacent affine ops are fused into a single region and checked once (§4.11), keeping per-receipt verifier cost sublinear in `#ops`.

> Status: the frozen IR registry now carries an executable verifier classification for every current op.
> Tier-A `matmul` and the admitted rank-2 matrix-contraction `einsum` subset use full Freivalds;
> affine/reduction Tier-B relations used by the current
> LinearTrainingStep verifier (`add`/`sub`, scalar multiplication, `sum`/`mean`) are classified as
> random-linear-checkable; exact but non-affine structural/pointwise Tier-B ops require deterministic
> replay through the graph verifier and conformance profile; and `gather`/`scatter`/`embedding` are
> present only as non-admitted index-consistency-gated vocabulary. Runtime tensors now carry scale
> metadata, exact replay enforces `TensorSpec.scale`, fixed-point `cast`/`round`, mixed-scale `add`/`sub`,
> mixed-scale `mul`, `Fixed32` reciprocal `div`, and `Fixed32` `matmul` use canonical round-half-even
> rescale, and exact per-channel int8
> quantize/dequantize replay is conformance covered.
> Byte-packed int8 quantization now uses a tensor-owned canonical flat `uint8` payload API with explicit
> magic/version, shape, axis, scale metadata, per-channel raw scales, row-major int8 bytes, bounded length
> calculation, and shared encode/decode validation for IR replay and conformance. Field `div` is
> admitted as exact modular-inverse replay, and `Fixed32` `div` is admitted as signed reciprocal
> division that returns to the lhs/output scale with round-half-even semantics.
> Packed tensor chunking/public-artifact APIs exist for packed payload artifacts, and local CPU graph
> receipt verification scenarios cover every current consensus-admitted exact op. Fixed-scale comparison
> masks and int8 selection are also conformance covered; CUDA/public deployment evidence remains TODO.

---

## 8. Verification Ladder — Level 3: Redundancy + Interactive Fraud Proofs (nonlinear / general)

Tier C ops (`softmax`, `gelu`, `topk`, `cross_entropy`, `data_indexer`, quantization, …) have no cheap algebraic check. Two complementary mechanisms:

### 8.1 Redundancy + agreement (v0 baseline)
Assign each Tier-C-containing receipt to `k` independent validators (selection via §10 randomness). They each re-execute the relevant op(s) (or full job for small jobs) and commit results. Agreement among `k` honest-majority validators settles the receipt; disagreement triggers **delayed settlement** and escalation to §8.2 or full re-execution. Soundness rests on honest-majority *within the sampled committee* — explicitly weaker than Levels 1–2; redundancy `k` and selection randomness are the security parameters.

> Status (implemented — end to end): a **committee admission policy** (`TensorGraph::validate_for_committee`, `requires_committee_verification`) admits Tier-C canonical-reference graphs (the §4.8.1 exp-family, `layer_norm`/`rmsnorm`) but still rejects index-consistency ops; a **committee execution path** (`execute_committee`) re-executes them exactly. The **committee verifier** (`verify_graph_execution_committee`) performs the same exact-replay check as the strict path but commits a **seed-independent agreement root** (`committee_checks_root`) so honest committee validators produce identical roots, and **`committee_agreement`** counts agreement only when ≥ `k` distinct assigned validators share one root (a disagreeing minority or split cannot reach the threshold). This is now **wired into consensus**: program-body **registration** admits committee graphs (`validate_for_committee`); the **validator role** routes committee receipts to the committee verifier; **`settle_epoch`** gates a committee-class receipt on `committee_agreement(redundancy_k)` instead of the Freivalds quorum, recording a **delayed-settlement** record (`reason: "awaiting tier-c committee agreement"`) on insufficient agreement; and **`ChainParams.redundancy_k`** (`= k`, default 3) is the committee size. End-to-end chain tests prove a Tier-C `gelu` receipt settles only at ≥ `k` agreement and that a 2-vs-1 split blocks settlement. Soundness is honest-majority *within the committee* — weaker than §6–§7 — and is the v0 path by which Tier-C work enters consensus; §8.2 fraud proofs are the planned strengthening.
>
> Status (audit deterrent): because the agreement root is a deterministic function of the receipt's claimed outputs, a lazy validator could in principle commit it *without* re-executing; the honest-majority assumption is therefore enforced economically by the §12 mandatory randomized **validator audits**. Audit assignment is verification-class-agnostic (it samples every attestation, committee receipts included), the auditor re-verifies a Tier-C receipt through the committee verifier, and a `Valid` attestation contradicted by the auditor's canonical `Invalid` is **slashed** — proven end to end (a lazy committee validator who rubber-stamps a wrong Tier-C receipt loses stake). The detection-probability evidence (§12 calibration) no longer reports committee receipts under deterministic 100% `graph_exact_replay`; they appear under a `committee_agreement` mechanism whose enforceable detection is the validator-audit sample rate, and the `bond ≥ gain-from-fraud` invariant covers the committee validator reward via the `validator_audit` fraud path.

### 8.2 Interactive fraud proofs (the general, asymptotically-cheap mechanism)
This is where the chain becomes secure for *arbitrary* workloads, and it is built directly on the IR DAG + `trace_root`. The bisection treats the op DAG as a canonical instruction stream (§4.10): op `id` is the program counter the parties binary-search over, which is why a single dispute protocol covers every op in the registry with no per-op-type special-casing.

Setup: the miner's receipt commits `trace_root` over per-op output commitments. A **challenger** (any node) asserts the result is wrong.

Protocol (interactive bisection / refutation game):
1. Challenger and miner hold the same `trace_root`. They disagree about the final output → by a pigeonhole over the trace, they disagree about *some op's* output commitment.
2. They **binary-search the op DAG**: at each round the responder reveals the committed output of the midpoint op (with a Merkle proof against `trace_root`). The parties recurse into the half where their commitments first diverge.
3. After `⌈log2(#ops)⌉` rounds they isolate **one op** `i` and agree on its committed *inputs* (outputs of already-agreed predecessor ops) but disagree on its *output*.
4. The chain (or a small committee) **re-executes only op `i`** on its agreed inputs using the canonical `F_p` semantics — `O(one op)`, cheap and bounded — and rules. The party whose commitment disagrees with canonical re-execution **loses**.
5. Loser is slashed; winner is rewarded from the loser's bond.

Properties:
- **Verifier work is `O(log #ops)` interaction + one op replay**, independent of total job cost → true verify ≪ compute for nonlinear workloads.
- **1-of-N honest** safety: a single honest challenger can punish any false result. No honest majority of *compute* required.
- Data availability is enforced by the game: a miner who withholds the data needed to reveal the disputed op's inputs/outputs **loses by timeout** (§9).

> v0 ships §8.1 (redundancy). §8.2 is the priority post-v0 upgrade because it removes the honest-majority-committee assumption and unlocks Tier-C-heavy real training. The receipt `trace_root` field exists in v0 specifically to make this a non-breaking addition.
>
> Status (implemented — Tier-C disputes): the trace-bisection game (open → signed midpoint rounds → isolate one op → one-op referee verdict → slash loser + delayed challenger bounty, with responder/challenger timeouts) is implemented and **now resolves Tier-C committee receipts**. The one-op referee (`TensorGraph::referee_op` / `referee_witness`) and the runtime challenger's local replay (`execute_committee` / `validate_for_committee`) admit committee graphs, so a **single honest challenger** can punish a wrong Tier-C (e.g. `gelu`) receipt by isolating the disputed op and having the chain re-execute just that op against the canonical reference — 1-of-N honesty, independent of committee honesty. Trace openings are Merkle-only and were already admission-agnostic; the committee receipt's `trace_root` (from `execute_committee`) is bisected identically. Proven end to end: a fraudulent `gelu` receipt is refereed, the miner is slashed, and a delayed challenger bounty is recorded. **Not yet wired:** auto-escalation from a §8.1 committee disagreement to *automatically opening* a dispute — disputes remain challenger-initiated (the standard optimistic model: any 1-of-N honest node opens with a bond). That auto-open trigger is a separate, optional policy.

### 8.3 Level 4 (future): ZK proofs
Per-op or per-segment SNARK/STARK proofs replace interaction for the most expensive disputes. Out of scope for v0; the IR/trace structure is ZK-friendly (uniform op semantics over `F_p`).

---

## 9. Data Availability

Verification-availability ≠ durable DA. v0 is explicit about this.

- **v0:** miners serve tensor chunks and exact-IR trace openings on request over p2p; validators perform **availability sampling** (request random chunks/openings against the committed Merkle root). Trace opening requests are keyed by `(trace_root, op_index)` and return either a verified encoded opening or an explicit missing response. A receipt whose required evidence cannot be served within a deadline is **not finalizable** and the miner's bond is at risk. This guarantees availability *for verification at settlement time*, not long-term retention.
- **Roadmap:** erasure-code chunks + distributed custody, or anchor to an external DA layer, for durable availability and light-client guarantees.

> TODO: erasure-coding parameters vs. activation tensor sizes (multi-GB); decide rate and custody set size.

---

## 10. Unbiasable Randomness

Randomness is used for (a) Freivalds/random-linear challenge vectors `r`, (b) which receipts/elements get sampled audits, (c) committee assignment in §8.1, and (d) anti-grinding in block production.

- Challenge vectors and audit selection MUST be **unpredictable until after the miner commits**, otherwise a miner can compute correctly only where it knows it will be checked (a sample/seed that is visible at commit time is directly exploitable).
- Source: a **VRF per validator** seeded by finalized chain state, and/or an external **drand-style randomness beacon**. Block-hash-derived randomness is **banned** for these purposes because a proposer can grind the block hash.

> Status: the local reference exposes chain-owned randomness binding evidence through service status and
> explorer overview. Admitted receipts persist a finalized-beacon anchor, assignment seed, and validation
> seed commitment; `ChainState::randomness_binding_evidence` reports the exact local seed domains,
> commit→reveal ordering (`receipt_id + finalized_beacon_round` committed before validator/job/round seed
> reveal), zero current-block-hash anchors, consistency counts for persisted receipt anchors, and
> state-rooted local validator VRF reveal records. Validators with registered reveal public keys must now
> provide bounded Ed25519 proof bytes over the committed receipt seed input; the chain verifies those proof
> bytes against the registered key before releasing validator receipt rewards, while the old unkeyed helper
> remains only as a local fallback for validators with no registered key. Local CPU validator role runtimes
> now derive and register wallet-backed reveal public keys before receipt work and expose checker-gated key
> lifecycle evidence. Local CPU role runtimes can now ingest a configured
> deterministic drand-style external beacon fixture through `ChainCommand::SubmitExternalRandomnessBeacon`,
> persist the state-rooted beacon record, submit validator reveal records before validator reward release,
> relay bounded reveal payloads over p2p/node ingest, retry out-of-order reveal payloads until receipt
> anchors arrive, and expose observed/applied counters plus external-beacon/reveal count evidence through
> status, explorer JSON, and the local checker. The chain and bounded p2p/node ingest path can now admit
> `pedersen-bls-unchained` drand evidence by verifying the signature, deriving randomness from that
> signature, and storing typed `DrandPedersenBlsUnchainedV1` proof metadata. Public drand mode now polls
> the default-chain v2 HTTP endpoint, verifies `pedersen-bls-chained` responses using
> `previous_signature`, applies only strictly newer finalized beacon rounds through the same chain
> command, skips stale rounds, computes endpoint-observed expected latest round and chain-epoch mapping
> evidence, rejects locally fetched public rounds outside the configured freshness lag, and exposes
> poll/backoff/freshness counters. Accepted chained drand records now anchor the current chain epoch to
> a public drand round, reject later chained records outside the deterministic chain-owned epoch window,
> and expose the rooted/persisted anchor plus current window through status and explorer evidence.
> v0 randomness decision (owner override, 2026-06-23, see `goal.md` "v0 Scope Decisions"): drand is the
> canonical v0 beacon. §10 is satisfied for v0 by the verified drand round bound into the chain epoch plus
> validator commit→reveal anchored to `(receipt_id, beacon_round)`; this section already admits "an external
> drand-style randomness beacon" as a valid source. A bespoke per-validator VRF construction is roadmap, not
> a v0 gate. The local reference now defaults the runtime randomness-beacon config to public drand when no
> override is supplied, and block application preserves an accepted verified drand beacon as the finalized
> consensus randomness instead of replacing it with a synthetic post-block beacon. Deterministic fixture
> beacons remain explicit local overrides; deployed public commit-reveal lifecycle evidence is part of the
> roadmap public run.

---

## 11. Consensus — Useful-Verification PoW + BFT Finality

The novel part: **the act of verifying useful work is the block-production proof of work.**

### 11.1 Settled-receipt blockspace
Receipts that have accumulated the required attestations (full Freivalds for Tier A, the policy's `k`/fraud-proof outcome for Tier B/C) become **settled**. Settled receipts form a **deterministic blockspace**: given finalized state + beacon round, all honest validators compute the *same* canonical candidate receipt set (deterministic selection, caps, duplicate/spent handling). Jobs do **not** advance blocks directly.

### 11.2 The UVPoW puzzle
A validator that wants to produce a block:
1. Verifies the canonical settled-receipt set (does the actual tensor-verification work).
2. Commits to its checks as `checks_root` = MerkleRoot(its attestations over the canonical set).
3. Searches for a nonce such that `H(checks_root ‖ prev_block ‖ beacon ‖ validator_pubkey ‖ nonce) < target`.

The PoW is *bound to* a valid `checks_root`: you cannot mine without having a verification commitment over the canonical receipt set. A block is **invalid** if its `checks_root` doesn't correspond to correct verification of the claimed settled receipts (other validators recompute/verify it). Difficulty `target` retargets to a block interval (§12).

> This binds "energy spent" to "useful verification performed." It is closer to classic PoW than PoS in liveness, but the wasted-hash component is minimized — the dominant cost is the verification, which is useful.

### 11.3 Finality
Block admission and finality are separate:
- A valid UVPoW block is **admitted** (longest-valid-chain tip).
- **Stake-weighted validator BFT votes** finalize it once votes exceed the stake threshold (e.g. 2/3). Finalized blocks are irreversible.

This gives PoW-style permissionless block proposal + BFT economic finality. TensorWork (miner output volume) affects **rewards and blockspace capacity only** — never proposer selection (that would be circular: current receipts must not influence who validates current receipts).

> Status: the local reference now applies a narrow current-head fork-choice policy for validator
> competition. A valid same-parent useful UVPoW block can replace the unfinalized current useful head only
> when its PoW hash is strictly better; finalized heads and accepted fallback heads are not replaced.
> Valid known-parent side branches are now retained in chain-owned fork-tree storage with parent and child
> state snapshots, and the chain-state codec persists that branch evidence without mutating the canonical
> head until a strictly longer unfinalized branch promotes through chain-owned fork choice. Public Docker
> evidence remains open.

### 11.4 Block structure

```text
TensorBlock {
  height, prev_hash, beacon_round
  settled_receipts: [ReceiptId]        // canonical deterministic set
  checks_root: Hash                    // proposer's verification commitment
  pow: { nonce, target }
  reward_allocations: [...]            // miners + winning validator
  proposer_pubkey, proposer_sig
  finality_votes: [StakeWeightedVote]  // appended at/after admission
}
```

---

## 12. Economics & Parameters

### 12.1 Incentives
- **Miners** earn rewards from settled, verified TensorWork through delayed chain-owned claims: the local
  reference aggregates newly settled TensorWork per miner, allocates the miner pool proportionally by raw
  receipt TWU, and keeps the matching miner TensorWork pending until the non-voided receipt reward survives
  canonical inclusion, challenge/audit holds, and beneficiary `ClaimReward`. Reward requires surviving
  verification; a slashed or voided receipt forfeits the pending claim and clears the pending TensorWork
  before it can activate.
- **Validators** earn (a) the block reward for the winning UVPoW block and (b) attestation fees / a share of slashed bonds for catching fraud. Verifying correctly is the paid job.
- **Challengers** (§8.2) earn a share of the loser's slashed bond → bounty for finding fraud.
- Miner and validator rewards derived from verifier-dependent receipt settlement are pending claims first.
  They become spendable only after the reward-settlement delay plus the challenge window, and a successful
  challenge or unavailable-data evidence before maturity voids the affected pending claims. This
  reward-finality delay includes an explicit fraud-window hold: the configured challenge window and,
  when mandatory audit sampling is enabled, the validator-audit window. Audited validator rewards remain
  escrowed long enough for assignment and dispute. The delay is distinct from, but must be at least as
  long as, the tensor/trace retention window needed for verification, audit, and challenge data
  availability.
- Successful block-check challenger bounties are also pending consensus claims before spendability. The
  challenge record proves the dispute, while the pending challenger reward claim is state-rooted, persisted,
  and claimable only after its maturity height. Canonical block-check challenge admission materializes any
  missing finalized proposer reward claim before computing the clawback and bounty, so p2p/node payload
  adapters do not pre-release or otherwise prepare rewards as a workaround.
- Proposer rewards use an additional proposer-specific hold after the normal reward maturity delay. This
  keeps block rewards escrowed past the block-check fraud window so a disproven block voids the pending
  proposer claim before it can become spendable.
- Local generic/faucet reward credits also use a state-rooted pending credit claim before spendability, so
  the shared `CreditReward` command cannot bypass the maturity boundary.

### 12.2 Slashing
- Miner: committing a receipt that fails verification (Freivalds/random-linear/fraud-proof) → slash bond.
- Validator: signing an attestation contradicted by canonical re-verification, or failing a *mandatory* (randomly-assigned) audit → slash stake. This closes the "lazy validator that rubber-stamps" attack.
- Withholding data needed to settle/dispute → slash (timeout loss).

> Status: the local reference now applies a state-rooted miner bond slash for data-unavailable receipts.
> An assigned validator's unavailable-data attestation marks the receipt non-finalizable, and the next
> canonical block child-state transition records a `DataUnavailabilitySlashRecord`, reduces the miner stake
> once for that receipt, and credits treasury. If receipt rewards were already pending, the unavailable-data
> evidence voids those delayed claims before spendability. A late assigned `Invalid` attestation likewise
> contests an already settled receipt, removes it from the settled set, marks it challenged, and voids the
> delayed miner and validator receipt reward claims before release instead of relying on spendable-balance
> clawback. The local reference also state-roots validator
> audit assignments, signed audit results, and validator audit slash records when audit sampling is
> configured. Base receipt-reward maturity now exposes a canonical fraud hold covering the challenge
> window and, when active, the audit window, so miner and validator receipt rewards remain delayed before
> spendability instead of relying on economic-calibration filtering. Audit assignment names a deterministic
> registered auditor distinct from the audited validator,
> keeps the audited validator's pending receipt reward held through the audit deadline, and rejects
> reports from non-assigned auditors; a missed audit or contradictory audit result slashes that validator
> once, credits treasury, voids that delayed validator reward, and holds the voided pending claim through
> the audit appeal deadline before it can be pruned without credit. A slashed validator can now submit a
> signed, bounded appeal that is tied to the audit slash, state-rooted, and persisted for adjudication.
> Appeal resolution is also chain-owned for the reward and stake sides: an upheld appeal keeps the delayed
> validator receipt reward voided for normal pruning, while a reversed reward-void outcome reinstates the
> pending claim, refunds the recorded stake slash from treasury back to validator stake, and still releases
> the reward only through the normal beneficiary `ClaimReward` maturity sweep.
> Chain state also exposes live economic calibration from current params and pending reward exposure. The
> validator-audit view reports configured audit sampling probability, slash amount, non-voided pending
> validator receipt reward exposure, required slashable bond, and pass/fail invariant. The broader
> fraud-path view covers implemented validator-audit, miner data-unavailability, invalid-output, and
> block-check/proposer clawback paths with aggregate worst-required-bond and all-path pass/fail status; delayed, non-voided
> receipt and proposer rewards are treated as slashable/voidable escrow, and fraud proceeds are counted
> only after the claim is actually spendable. Chain state also exposes structured detection-probability
> evidence for full-output Freivalds, sparse row-sampling audits, LinearTrainingStep random-linear checks,
> exact graph replay, replicated data availability, validator audits, data-unavailability evidence, and
> block-check challenges, derived from current params, live job shapes, and chain-state counters.
> Registered validator roles now observe
> only their assigned local audit work, submit signed audit reports through the shared chain command path,
> gossip bounded audit-report payloads, and expose submitted plus network-applied report counters.
> Deployed-run measured detection records and remaining fraud paths remain TODO before claiming the full §12
> economics invariant.

### 12.3 Parameters (to be fixed before testnet)

| Parameter | Symbol | v0 placeholder | Notes |
|---|---|---|---|
| Field modulus | `p` | 61-bit Mersenne-friendly prime | TODO: balance fixed-point range vs. wraparound |
| Freivalds reps (block-eligible) | `t` | such that `p^{-t} ≤ 2^{-80}` | full-output only |
| Redundancy committee | `k` | 3–7 | Tier C, §8.1 |
| Audit sample rate | `ρ` | beacon-derived, unpredictable | §10 |
| Block interval | — | target ~ seconds–minutes | retarget like classic PoW |
| Finality threshold | — | 2/3 stake | BFT |
| Challenge window | — | bounded; ≥ max fraud-proof game length | §8.2 |
| Miner bond | — | ≥ expected reward × safety factor | must exceed gain-from-fraud |

> The economic invariant that must hold: **expected cost of getting caught (bond × P(detection)) > reward from faking work.** Detection probability is driven by §6–§8 soundness + §10 unpredictability. State and re-verify this whenever a parameter changes.

---

## 13. Roadmap (verification maturity ladder)

| Level | Mechanism | Covers | Status |
|---|---|---|---|
| 1 | Freivalds | matmul / bilinear | v0 |
| 2 | Random-linear | affine / elementwise | v0 (partial) |
| 3a | Redundancy + agreement | nonlinear (honest-majority committee) | v0 |
| 3b | Interactive fraud proofs over `trace_root` | **arbitrary ops, 1-of-N honest** | implemented for Tier-A/B and Tier-C committee disputes; auto-escalation from §8.1 disagreement still challenger-initiated |
| 4 | ZK proofs of op/segment execution | expensive disputes, light clients | future |
| — | Durable erasure-coded DA | data availability | future |
| — | Externally-useful workloads (real training/inference) | usefulness | future |

The v0 → 3b transition is the most important: it removes the honest-majority-of-compute assumption and is what lets the chain secure **real nonlinear training**, not just matmul. It is designed to be non-breaking because `trace_root` ships in the v0 receipt.

Three cross-cutting IR refinements harden the verifier rather than extending the ladder, and slot in without a consensus break:

| Refinement | Buys | Status |
|---|---|---|
| Canonical normalization before `graph_id` (§4.5.1) | anti-grinding / work-dedup on `graph_id` | design; v0 may ship a reduced pipeline |
| Verification-class region fusion (§4.11) | sublinear on-chain verification for affine-heavy graphs | per-op classes exist; region algebra is design |
| Dtype definition framework (§3.5) + reduction-order class (§3.4) | closed correctness question per value; field-boundary determinism | partial (fixed-point/int dtypes + conformance vectors landing) |

These are all consequences of treating the IR as a *compilation contract* between the offline prover and the on-chain verifier (§1.3): each one either tightens the contract, shrinks the verifier, or closes a grinding surface — none of them change what work is useful, only how unambiguously and cheaply it is checked.

---

## 14. Threat Model & Soundness Summary

| Attack | Mitigation | Residual risk |
|---|---|---|
| Miner fakes matmul output | Full Freivalds, `p^{-t}` soundness | negligible w/ enough reps |
| Miner correct only on sampled elements | Unpredictable beacon-bound challenge vectors (§10); full-output for block eligibility | row-sampled triage only |
| Miner fakes nonlinear op | Redundancy (v0) → fraud proofs (3b) | v0: committee honest-majority |
| Lazy validator rubber-stamps | Mandatory randomized audits + slashing | needs enough honest auditors |
| Validator–validator collusion | Stake slashing + 1-of-N challenger (3b) | v0 weaker (committee) |
| Proposer grinds randomness | Beacon/VRF, block-hash randomness banned (§10) | beacon liveness dependency |
| Data withholding | Availability sampling + timeout slashing | durable DA is future work |
| Sybil / monopoly | Stake + bond + reward concentration analysis | economic, ongoing |
| Graph-id grinding (same work, many `graph_id`s) | Canonical normalization before `graph_id` (§4.5.1) | residual: v0 may ship a reduced normalization pipeline |
| Reduction-order divergence across hardware | Order-free inside `F_p`; fixed ascending for fixed-point (§3.4); conformance vectors (§3.3) | residual: fixed-point / Tier-C vector coverage |
| Circular proposer selection | TensorWork barred from proposer choice (§11.3) | by construction |

**Honest framing:** v0 is a *probabilistically verified tensor-work testnet under a bounded adversarial model* — Tier A is cryptographically strong; Tier C rests on committee honesty until fraud proofs (3b) land. Do not claim base-layer economic security until 3b, unbiasable randomness, durable DA, and slashing are all live.

---

## 15. Implementation Notes

This section is non-normative guidance on how the spec components partition into build work.

| Spec component | Nature |
|---|---|
| Tensor IR (§4): structure, registry, canonical encoding, `graph_id` | self-contained; implementable directly from §4 |
| Deterministic op semantics (§3, §4.8) | the core engineering effort: fixed-point `F_p` reference kernels + optional GPU accel path that must match them bit-for-bit |
| Records — Job/Receipt/Attestation (§5) | asymmetric (sr25519/ed25519) signatures throughout; no shared-secret signing |
| Randomness beacon (§10) | VRF + external drand-style beacon; commit→reveal binding |
| Chain / consensus / p2p / runtime (§11) | a useful-verification-PoW + BFT-finality chain with deterministic settled-receipt blockspace; the heaviest infra component, well-suited to a Rust node implementation |
| Interactive fraud proofs (§8.2) | a dedicated `dispute`/`referee` subsystem keyed off the receipt `trace_root` (the general-verification upgrade; ships after the redundancy baseline) |

> Build order: (1) determinism contract + conformance vectors (§3, §4.8) — everything depends on it; (2) IR + Tier-A/B exact ops + Freivalds, with canonical normalization before `graph_id` (§4.5.1); (3) records, p2p, settled-receipt blockspace, UVPoW + BFT; (4) randomness beacon binding; (5) redundancy (§8.1); (6) interactive fraud proofs (§8.2) over the op-DAG instruction stream (§4.10); (7) verification-class region fusion (§4.11), durable DA, and transcendental-op references.

> Design lineage (non-normative): the IR/verification design borrows structure from ML compilers — a content-addressed graph-level IR, structural normalization/CSE, op classes used to reason about whole regions, and a flat SSA instruction stream as the execution form. The borrow is deliberately partial: those systems optimize for *speed on one device*, whereas here the same machinery is repurposed for an unambiguous prover/verifier contract (§1.3), sublinear on-chain verification (§4.11), and bisectable disputes (§4.10, §8.2). Numerically the borrow is *inverted* — a compiler's freedom to reassociate floating point is exactly what the §3 determinism contract forbids; here reassociation is admissible only where the field makes it exact (§3.4).

---

## 16. Open Problems / TODO

- [~] **Determinism conformance suite**: current executable TensorOp/LinearTrainingStep exact-op `F_p`
  vectors exist and gate those receipt validation paths. The first canonical Tier-C transcendental
  reference (the `exp`-family: `exp`/`sigmoid`/`tanh`/`silu`/`gelu`/`softmax`, §4.8.1) now executes through
  the exact interpreter in fixed Q-format integer math and is CPU-conformance gated as auxiliary (Tier-C,
  not consensus-admitted). The exp-family carries both gates: exact-match determinism vectors (the
  consensus-relevant gate) plus a tolerance-based accuracy harness measuring max error vs. `f64`
  (`≤ 2.5e-4` for `exp`, `≤ 1e-5` for the rest at scale 16, `|x| ≤ 8`). Also landed: bit-exact CUDA
  kernels for the exp-family (`__int128` Q-format) gated through the GPU conformance profile and verified
  on the local A100×8 box; CPU `log`/`sqrt` references; and a full-domain accuracy decomposition
  `error(s) ≤ 0.5·2^{−s} + E_q32` with closed-form requant/saturated-tail terms and `E_q32` proven
  exhaustively at the conformance scale (s=16, 8.4M inputs; `exp` rel `≤ 1e-5`, `sigmoid`/`silu`/`gelu`
  `≤ 1e-7`, `tanh` `≤ 1.5e-7`). Remaining for full §3.3 safety: extend the exhaustive `E_q32` proof to all
  field-representable scales (s ≤ ~24) plus a formal `softmax` composition proof; CUDA `log`/`sqrt` kernels
  and their bounds; `log_softmax`/`cross_entropy` compositions (`layer_norm`/`rmsnorm` done, and now
  expressible as raw multi-op graphs after the Fixed32 `mean` fix); and a §8.1/§8.2 verifier before any
  consensus admission.
- [~] Exact `F_p` choice and fixed-point scale discipline: runtime tensor scale metadata, input-scale
  enforcement, fixed-point `cast`/`round` round-half-even rescale, mixed-scale `Fixed32` `add`/`sub`
  RHS-to-lhs/output rescale, mixed-scale `Fixed32` `mul` rescale from product scale back to the declared
  output scale, and
  canonical `int8`/`uint8`/`bool` dtype tags are implemented; exact
  per-channel int8 quantize/dequantize scale selection and saturation are conformance covered;
  byte-packed quantization has a conformance-covered tensor-owned flat `uint8` payload API; packed payloads
  can now be constructed and decoded as first-class `Uint8` tensor artifacts with normal descriptor,
  chunk, and Merkle-opening evidence; fixed-point reciprocal division is implemented for `Fixed32` `div`;
  `Fixed32` `matmul` now accumulates signed raw products in fixed order and rescales once into the
  lhs/output scale.
- [~] Which Tier-B ops have *sound* random-linear checks vs. deterministic replay/fraud proofs: current
  frozen-registry metadata classifies every op and keeps `gather`/`scatter`/`embedding` non-admitted until
  index-consistency proofs exist; graph-backed exact replay now covers the current unary, structural,
  comparison, generator, reduction, fixed-point `cast`/`round`, mixed-scale `add`/`sub`, mixed-scale
  `mul`, and `Fixed32` `matmul` rescale surface with conformance gating where the vector schema fits,
  plus exact per-channel and byte-packed int8 quantization, fixed-scale comparison masks, and int8
  selection. CUDA evidence now covers same-shape field modular division, scalar-bounds field clamp,
  deterministic field sum/mean, unary field reshape/squeeze/unsqueeze/slice/tril/triu/broadcast, and field mask
  selection only;
  broader CUDA/public deployment evidence remains TODO (§7).
- [~] Fraud-proof game: signed trace-bisection session and round state now provide a deterministic
  message/hash boundary over verified `IrTraceOpening`s, with response deadlines and challenger/responder
  bond envelope fields. Bounded p2p round payloads now reuse the trace-opening codec, verify responder
  signatures, and reject announcement/payload mismatches before gossip delivery. Chain command admission
  now records trace-bisection sessions, signed midpoint rounds, transcript-root advancement, isolated-op
  outcomes, and responder timeouts in state-rooted challenge records. Node payload application now routes
  bounded trace-bisection round gossip through the shared pending queue and canonical
  `ChainCommand::SubmitTraceBisectionRound` path, with status counters for ingested/applied rounds. Trace
  openings now bind resolved input roots as well as output roots, and isolated disputes can record a
  chain-owned one-op referee verdict from explicit witness values through
  `ChainCommand::RefereeTraceBisection`. Referee verdicts and responder timeouts now also settle the
  state-rooted economic side: the losing registered miner or validator stake is slashed from the session
  bond envelope, affected receipt rewards and pending TensorWork are voided when the responder/miner loses,
  treasury receives the net slash, and a winning challenger receives only a delayed
  `PendingChallengeReward` claim that remains non-spendable until the normal reward maturity plus
  beneficiary `ClaimReward` boundary. Isolated sessions that pass the response deadline without a
  referee witness now time out against the challenger through the same chain command, slash the challenger
  bond to treasury, leave the responder/miner receipt path unvoided, and close the transcript instead of
  leaving bonds unresolved.
  Bounded p2p referee-witness payloads now route through the shared node pending queue into the same chain
  command, with dedicated ingest/application counters. Runtime challenger nodes now derive one-op referee
  witnesses from local graph replay for isolated sessions whose stored opening input roots match the
  generated witness, submit them through `ChainCommand::RefereeTraceBisection`, persist the resulting
  chain state, publish the existing bounded referee payload, and report validator referee-submission
  counters.
  Focused tests prove midpoint narrowing, final-op isolation, one-op referee verdicts, timeout reporting
  with slashing and delayed challenger rewards, round-budget admission rejection, conflicting pending
  expectation overwrite rejection with duplicate replay idempotence, tamper rejection, malformed wire edges,
  duplicate rejection, round, expectation, and referee pending retry, runtime session-open generation/gossip from local
  evidence, runtime challenger expected-root generation/gossip from local replay, runtime responder round
  generation from committed local traces, runtime challenger referee-witness generation/gossip from local
  replay, challenger-signed expected midpoint roots enforced by chain admission before responder rounds can
  advance, isolated-timeout challenger forfeiture, and snapshot persistence. Deployed public/CUDA dispute
  evidence remains TODO (§8.2).
- [~] Block-check transcript openings: selected-receipt block openings now expose typed transcript fields
  (beacon, parent, check seed, selected receipt leaf, receipt checks root, and receipt metadata) whose
  commitment is the Merkle-proven `check_leaf`; the full interactive fraud-proof game remains TODO.
- [~] Beacon binding: local finalized-beacon receipt anchors now expose chain-owned seed-domain,
  commit→reveal, and current-block-hash-ban evidence through status/explorer; local role runtimes now
  ingest a deterministic drand-style external beacon fixture and expose checker-gated applied-record
  evidence. Chain and p2p/node admission can now verify bounded `pedersen-bls-unchained` drand evidence.
  Public drand mode now polls and verifies newer default-chain `pedersen-bls-chained` rounds with stale
  skip, backoff, endpoint expected-round, chain-epoch, and freshness-lag evidence. Full-spec public
  evidence validation now requires raw accepted `drand-v1` or `validator-vrf-v1` randomness records whose
  aggregate root matches the signed randomness summary; local deterministic fixture records cannot satisfy
  that public randomness gate. Consensus-level drand round ↔ epoch mapping and validator VRF construction
  remain TODO (§10).
- [ ] DA: erasure-coding rate, custody set size, light-client sampling guarantees (§9).
- [~] Retention evidence: selected-receipt block openings now anchor `expires_at_block` to receipt
  submission height plus the configured tensor-retention window, so delayed inclusion cannot extend the
  reported verification/challenge availability deadline. Durable erasure-coded DA remains TODO (§9).
- [~] Economic calibration: live calibration now reports the configured validator-audit detection
  probability, current pending validator reward exposure, implemented miner data-unavailability and
  block-check/proposer clawback paths, required slashable bonds, aggregate worst-required-bond, and
  pass/fail invariants. Reward maturity includes an explicit fraud-window hold before spendability;
  status/explorer now expose structured detection-probability evidence for implemented verifier and fraud
  mechanisms plus verifier-bandwidth evidence derived from live job and receipt shapes. Deployed-run
  measurements, CUDA bandwidth evidence, and remaining fraud paths remain open (§12.2).
- [~] Reward concentration / TensorWork activation delay: the local reference now keeps newly settled miner
  TensorWork pending until the matching delayed miner receipt reward survives inclusion, challenge windows,
  and maturity. Receipt rewards now store awaiting-inclusion and claimable-height maturity as explicit
  chain state instead of a sentinel height. Invalid-output, data-unavailability, and block-check challenge paths clear pending work
  before it can activate, while telemetry/study reporting still tracks raw concentration. Deployed-run
  concentration measurements and governance tuning remain open.
- [ ] Canonical normalization pipeline (§4.5.1): fix the declared-commutative op set, the canonical ref-key/tie-break ordering, and CSE/renumbering determinism; prove (and conformance-test) idempotence and semantics-preservation; decide the v0 subset vs. full pipeline and record the residual grinding surface.
- [ ] Verification-class region fusion (§4.11): canonical region decomposition over the normalized graph, lattice-join correctness, soundness of the fused affine/Freivalds checks over a region, and committing the decomposition into `checks_root` so it is recomputable.
- [~] Reduction-order class (§3.4): `field` `OrderFree` vs. `fixedNN` `AscendingFixedPoint`. Fixed ascending accumulation is implemented for `Fixed32` `matmul`; broader fixed-point reductions plus their conformance vectors, and the explicit admission of free reduction order for `field` reductions on the GPU path, remain.
- [ ] Datatype definition framework (§3.5): make `(embedding, lowering, conformance vectors)` the single source of truth for each dtype so a new exact dtype is a data change, not a kernel change; audit current dtypes against this triple.
- [ ] Defining "externally useful" jobs without introducing subjective scoring or grindable job content (§2 job-source determinism).
- [~] Edge case: jobs with `#ops` not a power of two in bisection; multi-output ops; ops with
  `const_blob` inputs. Exact graph execution now loads `const_blob` tensors by content URI from local
  tensor artifacts and checks shape/dtype/root before replay; availability of those blobs during a future
  interactive dispute remains open.
- [ ] Edge case: floating-point miners producing off-by-one-ULP fixed-point results — define the canonical rounding so this is a *fault*, not noise.
