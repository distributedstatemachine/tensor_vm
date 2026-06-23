# TensorVM (TVM) MVP Specification (Reviewed Draft)

## 0. Review Status

This document is a reviewed design draft, not a production security proof.

The MVP is viable as a research testnet if it is framed as **probabilistically verified tensor work under a
bounded adversarial model**. It is not yet a complete base-layer security design. The original version
overstated several points:

- row-sampled Freivalds checks do not, by themselves, give high-probability detection for sparse row
  corruptions
- sampled optimizer checks can miss poisoned model-state entries
- block-hash-derived validator randomness can be grindable if the proposer influences the block hash
- TensorWork-weighted proposer selection is circular if current receipts influence current proposer choice
- serving sampled chunks to validators is availability for verification, not durable data availability
- synthetic random tensor jobs prove verifiable compute, not externally useful work

The spec below keeps the architecture but tightens the MVP around these constraints.

## 0.1 One-Line Definition

TensorVM (TVM) is a blockchain testnet where **validator-side useful verification of miner tensor work** is
the native proof-of-work primitive. Miners produce deterministic tensor receipts, validators verify settled
receipts with reproducible algebraic checks, and the winning validator proves that verification work by
mining a block over the checked receipt set.

---

## 1. MVP Thesis

Traditional Proof-of-Work proves that energy was spent on hash search.

TensorVM should prove, within explicit soundness parameters, that tensor computation was performed
according to canonical deterministic semantics.

The long-term vision is a blockchain where the core commodity is not hashpower, gas, or generic computation, but:

```text
verified tensor state transitions
```

The MVP should test the smallest verifiable version of this idea:

```text
A decentralized network can generate deterministic tensor jobs, have miners execute them, have validators
verify them with cheaper-than-recompute checks, and use validator-side useful verification of settled
receipt blockspace as the block-production proof of work.
```

The MVP should not attempt full decentralized LLM training immediately. It should build the verification and
incentive rails that make that possible later. It should also avoid claiming production economic security
until slashing, unbiasable randomness, and data availability are implemented.

---

## 2. Reviewed MVP Scope

The final MVP has two execution primitives and one consensus primitive:

1. **TensorOp Primitive**
   - deterministic tensor operation verification
   - initial canonical job: matrix multiplication
   - validator verification via Freivalds-style checks

2. **LinearTrainingStep Primitive**
   - minimal forward pass
   - minimal loss
   - minimal backward pass
   - minimal optimizer update
   - validates an actual learning step without full Transformer complexity

3. **Useful-Verification Proof-of-Work**
   - miners submit receipts for completed tensor work
   - settled receipts form deterministic blockspace
   - validators verify the canonical receipt set, commit to the resulting `checks_root`, and search for a
     proof-of-work nonce over that verification commitment
   - stake-weighted validator finality confirms the winning PoW block

The MVP should use TensorOp and LinearTrainingStep receipts to reward miners, test validator verification,
and produce blocks through validator useful-verification PoW. Jobs no longer advance blocks directly, and
TensorWork no longer selects proposers.

---

## 3. MVP Non-Goals

The MVP must not include:

- arbitrary PyTorch execution as consensus
- full Transformer training as consensus
- floating-point consensus-critical outputs
- general smart contracts
- large tensor storage on-chain
- full ZKML verification
- subjective usefulness scoring
- permissionless arbitrary TensorVM op deployment
- full fraud-proof games
- model parallel distributed training
- decentralized optimizer aggregation for large models

These belong in later versions.

---

## 4. Core Design Principles

### 4.1 Determinism First

Consensus-critical computation must be deterministic across machines.

Use:

```text
integer arithmetic
fixed-point arithmetic
finite-field arithmetic
canonical rounding
canonical overflow behavior
canonical tensor layouts
```

Avoid:

```text
fp16/bf16/fp32 consensus outputs
non-deterministic CUDA reductions
hardware-specific fused kernels as canonical state
```

GPU kernels may be used by miners for acceleration, but their final committed outputs must match canonical deterministic semantics.

---

### 4.2 Commitments On-Chain, Tensors Off-Chain

The chain should never store full tensors.

On-chain:

```text
tensor commitments
job definitions
execution receipts
validator attestations
block metadata
reward state
slashing/reputation state
```

Off-chain:

```text
full tensor data
intermediate activations
execution traces
model weights
training batches
large outputs
```

---

### 4.3 Cheap Verification

The miner should do expensive tensor work.

The validator should verify cheaply.

Target asymmetry:

```text
miner cost:     O(n^3) or large tensor execution
validator cost: O(n^2) full-output checks, plus optional sampled audits
```

Freivalds-style verification is the first canonical verifier for matrix-heavy workloads.

---

### 4.4 Useful Work Later, Verifiable Work First

The MVP should not over-optimize for subjective usefulness.

The first target is:

```text
verifiably correct tensor work
```

Then the protocol can move toward:

```text
inference
training
RL rollouts
model evaluation
architecture search
scientific workloads
```

---

### 4.5 Gate 0: First Executable Gate - CPU Multi-Participant Local Testnet

Gate 0 is the first non-skippable MVP acceptance gate, the first executable gate in the MVP acceptance
sequence, and the first command that may produce acceptance evidence in every new or resumed MVP
implementation iteration: a local CPU multi-participant testnet. Before any other local acceptance claim,
CUDA acceleration, production deployment, public-run evidence, or adversarial public-testnet claim can
count, the default-feature CPU reference path must run a local TensorVM testnet with multiple
independently instantiated miners and validators. This gate must use canonical CPU semantics and the
mandatory libp2p node runtime; it cannot be satisfied by CUDA kernels, simulations, local-only networking
shims, in-memory propagation substitutes, or single-participant shortcuts.

For this gate, CPU means the default reference backend without `cuda-kernels` or GPU acceleration.
Multi-participant means more than one independently instantiated local participant with separate node
state, libp2p identity, endpoint, and role; single-process helper assertions do not count unless they
stand up the same participant shape with distinct local node identities, endpoints, and role-specific
state. Local testnet means a local-only harness is allowed, but protocol messages must flow through the
mandatory libp2p node paths rather than mocks or direct in-memory propagation.

The focused implementation target for this gate is defined in
[`local_cpu_testnet_spec.md`](local_cpu_testnet_spec.md). That spec narrows the first milestone to a full
local CPU testnet where all 10 miner operators and 5 validator operators are deployable with Docker
Compose, use durable per-operator state, and communicate through the mandatory libp2p runtime. It does
not include CUDA, public infrastructure, or 7-day public-run evidence.

The local, public-testnet, and future mainnet modes must share the same chain engine, store boundary,
network event loop, and profile-driven configuration model. Local-only code may supply deterministic CPU
job generation and Compose wiring, but it must not fork consensus, storage, networking, or role-loop logic
away from the base TensorVM runtime.

Gate 0 is also the first executable gate for every new or resumed MVP implementation iteration. Context
gathering may happen before it, but the first acceptance command for the iteration must be the CPU local
multi-participant testnet command below. No later gate, implementation slice, or completion claim counts
until this command passes against the current codebase.

Required Gate 0 command from the repository root:

```bash
cargo test -p tensor_vm local_testnet --release
```

The Gate 0 run must cover:

```text
at least two miners and one validator, with the default local target remaining 10 miners and 5 validators
separate local participant identities, node endpoints, and operator roles for miners and validators
block production from the local testnet harness
matmul receipt submission, full Freivalds validation, attestation, settlement, and rewards
LinearTrainingStep receipt validation and state transition
data availability through the local tensor server path
mandatory libp2p node startup and propagation paths under default features
real loopback libp2p gossipsub exchange for every TensorVM gossip topic and request-response exchange for
every TensorVM request family
explicit proof that this CPU gate is not public-run evidence and does not satisfy the 7-day deployment gate
```

Gate 0 must be executed first when starting or resuming MVP work. Any completion transcript must list this
command before later acceptance-gate commands. Later acceptance criteria may be implemented and tested
only after this local CPU multi-participant gate is passing in the current codebase.

The MVP must pass these gates before it is treated as a public adversarial testnet:

```text
Gate 0 local CPU multi-participant testnet passes without simulations, local-only networking shims, or
single-participant shortcuts
explicit threat model for miners, validators, proposers, and data servers
unbiasable validation randomness from a finalized beacon or commit-reveal protocol
useful-verification PoW over deterministic settled-receipt blockspace
full-output Freivalds for block-eligible matmul receipts, or a documented row-sampling soundness bound
random-linear checks for full elementwise training relations
bounded verifier bandwidth per job shape
data retention through settlement and challenge windows
adversarial parameter studies for sparse corruptions, colluding validators, and data withholding
```

Until these gates pass, v0 rewards should be capped and the network should be described as a research
testnet, not as an economically secure proof-of-work chain.

---

### 4.6 Canonical Runtime And Transition Boundary

The MVP has one consensus state-transition boundary. Runtime loops, RPC routes, libp2p handlers,
localnet harnesses, testnet harnesses, deployment scripts, and checkers may request or observe
transitions, but they must not own consensus mutation.

The chain transition layer owns:

```text
job admission
receipt admission
attestation admission
pre-block settlement and model updates
block validation and append
block-vote admission
finality
proposer rules
reward allocation
challenge outcomes
```

Adapters must not perform hidden consensus work. In particular, `tvmd` runtime code, node event
ingestion, p2p glue, RPC glue, shell checkers, and deployment scripts must not synthesize settlement,
model transitions, block votes, finality, proposer selection, reward allocation, or block validity
outside the canonical chain transition path.

Every long-running role must continuously drain and apply inbound network events. P2P ingest must not
depend on synthetic job production, local block production, blocking RPC mode, profile-local jobs, or
whether the node can produce blocks. A producer-capable node may decide whether to create outbound
blocks, but it must still accept valid inbound block payloads and signed votes through the same network
path as every other node.

Block admission and finality are separate protocol steps:

```text
valid block payload -> append block
signed validator block votes -> finality once stake threshold is met
```

Appending a valid block must not fabricate local validator votes. Auto-finalization is allowed only in
clearly named pure test helpers, never in runtime block admission, node ingest, p2p handling, RPC
handling, or deployment checker code.

Consensus payload decoding must be bounded before allocation. Network and storage payloads for blocks,
jobs, receipts, attestations, block votes, and tensor artifacts should share canonical codecs. If a
network/storage format split is intentionally preserved, the boundary must be documented and covered by
parity, roundtrip, malformed-input, and size-bound tests.

Local checker evidence must come from structured runtime and chain state. Shell scripts may orchestrate
Docker and invoke verification commands, but they must not become the source of truth for protocol
readiness through hand-written grep/sed parsing, duplicated service lists, hardcoded certificate
booleans, or status-field archaeology.

---

## 5. System Actors

### 5.1 Miner

A miner provides tensor compute. Miners do not produce blocks in the v2 MVP. Their consensus-facing output is
a signed execution receipt plus enough tensor data availability for validators to verify that receipt.

Responsibilities:

```text
register on-chain
listen for tensor workload requests
execute TensorVM programs
commit to output tensors
serve tensor chunks to validators
submit execution receipts
earn rewards for valid tensor work
```

MVP miner hardware:

```text
CPU supported for reference/small jobs
GPU recommended for competitive mining
```

---

### 5.2 Validator

A validator verifies tensor work, performs useful-verification proof-of-work, proposes winning blocks, and
finalizes blocks through stake-weighted voting.

Responsibilities:

```text
register stake
receive validation assignments
request tensor chunks/openings
perform Freivalds checks
perform sampled row/cell checks
verify training-step consistency
derive checks_root for the canonical settled-receipt set
search for a proof-of-work nonce over the verification commitment
propose the winning block when the PoW target is met
submit attestations
vote on block validity
earn validator rewards
```

---

### 5.3 Proposer (Validator-Elected By Useful-Verification PoW)

A proposer is the validator that first publishes a valid proof-of-work block for the current parent and
canonical settled-receipt set. This is not a separate miner role, and it is not selected by TensorWork score.

Responsibilities:

```text
select the canonical settled-receipt set from deterministic blockspace
verify that receipt set and commit to checks_root
find nonce such that H(header || nonce) < difficulty_target
propose block
collect stake-weighted finality votes
earn the validator proposer reward
```

In the MVP, all registered validators may attempt useful-verification PoW. Miner TensorWork affects miner
rewards and blockspace accounting only; it does not grant block-production eligibility.

---

### 5.4 Watcher / Challenger

A watcher audits the network.

Responsibilities:

```text
monitor invalid receipts
recompute sampled jobs
flag validator misconduct
flag miner data withholding
```

MVP v0 can include watcher tooling without full slashing-based fraud games.

---

## 6. Chain Architecture

The MVP has six layers:

```text
Application Layer
  synthetic tensor workload requests, linear training steps

Tensor Job Layer
  workload generation, assignment, deadlines, miner rewards

TensorVM Layer
  deterministic tensor execution semantics

Verification Layer
  Freivalds checks, sampled checks, redundant agreement

Blockspace Layer
  settled receipt pool, deterministic receipt ordering, TWU/byte/count caps

Consensus/Settlement Layer
  useful-verification PoW, blocks, attestations, rewards, BFT finality
```

---

## 7. Tensor Object Primitive

### 7.1 Tensor Descriptor

```rust
struct TensorDescriptor {
    tensor_id: TensorId,
    shape: Vec<u64>,
    dtype: DType,
    layout: Layout,
    chunk_shape: Vec<u64>,
    commitment: Commitment,
    byte_size: u64,
}
```

### 7.2 Supported MVP DTypes

Consensus-critical:

```text
int32
int64
fixed32
field_element
```

Optional non-consensus execution:

```text
fp16
bf16
fp32
```

Recommendation:

```text
Use finite-field or integer modular arithmetic for the first matmul jobs.
Use fixed-point only when needed for training-step semantics.
```

---

### 7.3 Tensor Layout

MVP supported layouts:

```text
row_major
chunked_row_major
```

Deferred:

```text
tiled
sparse_csr
sparse_coo
quantized_grouped
```

---

### 7.4 Tensor Commitment

Full tensors are chunked and Merkle-committed.

```text
chunk_size = 1 MiB
leaf_i = H(domain || tensor_id || chunk_index || chunk_bytes)
root = MerkleRoot(leaves)
```

The chain stores:

```text
tensor_id
shape
dtype
layout
commitment root
```

---

### 7.5 Tensor Opening

Validators request chunks or rows.

```rust
struct TensorOpening {
    tensor_id: TensorId,
    chunk_index: u64,
    chunk_bytes: Vec<u8>,
    merkle_proof: Vec<Hash>,
}
```

A row opening can be encoded as one or more chunk openings.

---

## 8. TensorVM

### 8.1 Purpose

TensorVM is a deterministic execution environment for tensor programs.

It defines:

```text
valid operations
shape rules
dtype rules
rounding rules
overflow rules
cost accounting
trace commitment format
verification semantics
```

---

### 8.2 MVP Operation Set

Required ops:

```text
random_tensor(seed, shape, dtype)
matmul(A, B)
transpose(A)
add(A, B)
sub(A, B)
mul(A, B)
reduce_sum(A, axis)
scalar_mul(A, scalar)
commit_tensor(A)
hash_tensor(A)
```

Training-step ops:

```text
mse_loss(Y, T)
linear_backward(X, dY)
sgd_update(W, grad_W, lr)
```

Deferred ops:

```text
softmax
layernorm
attention
embedding_lookup
topk
cross_entropy
adam_update
moe_routing
```

---

### 8.3 Canonical Arithmetic

For matmul verification, prefer finite-field arithmetic:

```text
C = A @ B mod p
```

Where `p` is a protocol-defined prime.

For fixed-point training:

```text
fixed32 = signed int32 with global scale S
value = raw_int / S
```

MVP recommendation:

```text
TensorOp jobs use field arithmetic.
LinearTrainingStep jobs use fixed-point or integer-scaled arithmetic.
```

---

### 8.4 Program Hashing

A TensorVM program is canonicalized before hashing.

```text
program_hash = H(canonical_program_encoding)
```

This prevents ambiguity in execution receipts.

---

## 9. Primitive 1: TensorOp Workload

### 9.1 Purpose

TensorOp workloads verify raw tensor compute. A workload request can be generated by the local profile,
submitted by a user in future versions, or scheduled by a workload policy, but it is not itself the block
production primitive. The on-chain primitive that enters blockspace is the miner receipt after it has
enough validator evidence to become settled.

The canonical MVP job is matrix multiplication:

```text
C = A @ B
```

### 9.2 Synthetic Matmul Workload Request

```rust
struct MatmulWorkload {
    job_id: JobId,
    epoch: u64,
    m: u64,
    k: u64,
    n: u64,
    dtype: DType,
    modulus: Option<FieldModulus>,
    seed_a: Hash,
    seed_b: Hash,
    deadline_block: u64,
    reward_weight: u64,
}
```

Inputs are generated from public randomness:

```text
seed = H(finalized_epoch_beacon || epoch || job_id)
A = random_tensor(seed_a, [m, k])
B = random_tensor(seed_b, [k, n])
```

The miner computes:

```text
C = A @ B mod p
```

The miner submits:

```text
commitment_C
receipt
```

The receipt becomes blockspace only after validator attestations settle it. Blocks contain hashes and roots
for settled receipts, not raw job requests.

Current local-reference status: canonical TensorOp receipts bind `program_hash` to the validated
single-matmul TensorGraph `graph_id`, and their `trace_root` is derived from exact execution of that
canonical graph's op trace rather than from a receipt-specific shortcut hash.

---

### 9.3 TensorOp Receipt

```rust
struct TensorOpReceipt {
    receipt_id: ReceiptId,
    job_id: JobId,
    miner: Address,
    program_hash: Hash,
    input_roots: Vec<Hash>,
    output_roots: Vec<Hash>,
    trace_root: Hash,
    tensor_work_units: u64,
    execution_time_ms: u64,
    signature: Signature,
}
```

For synthetic jobs, `input_roots` can be derived from seeds and may not need full submitted input tensors.

---

## 10. Primitive 2: LinearTrainingStep Workload

### 10.1 Purpose

The LinearTrainingStep primitive tests the full shape of learning:

```text
forward pass
loss computation
backward pass
optimizer update
```

without requiring Transformer complexity.

This is the smallest learning-shaped training primitive.

As with TensorOp, the workload request defines what a miner should compute. The chain's blockspace primitive
is the settled receipt proving the miner computed and served enough data for validators to verify the
transition.

---

### 10.2 Linear Training Equations

Given:

```text
X: input batch
W_t: current weights
T: target tensor
lr: learning rate
```

Forward:

```text
Y = X W_t
```

Loss:

```text
L = mean((Y - T)^2)
```

Backward:

```text
dY = Y - T
grad_W = X^T dY
```

Consensus convention:

```text
dY = Y - T is the MVP gradient signal.
```

This is equivalent to using a half-squared-error sum, or to absorbing the constant factor from mean-squared
error into `lr`. Do not claim exact mean-MSE gradient semantics unless the protocol defines the scale factor,
division rule, and rounding rule.

Optimizer:

```text
W_{t+1} = W_t - lr * grad_W
```

This primitive is intentionally simple because both the forward and backward passes are matmul-like and can be verified with Freivalds-style checks.

---

### 10.3 LinearTrainingStep Workload Request

```rust
struct LinearTrainingStepWorkload {
    job_id: JobId,
    model_id: ModelId,
    step: u64,
    batch_seed: Hash,
    weight_root_before: Hash,
    input_shape: Vec<u64>,
    weight_shape: Vec<u64>,
    target_shape: Vec<u64>,
    lr: FixedPoint,
    dtype: DType,
    deadline_block: u64,
    reward_weight: u64,
}
```

Inputs:

```text
X generated from batch_seed or committed dataset batch
T generated from batch_seed or committed dataset target
W_t committed by weight_root_before
```

Outputs:

```text
Y_root
loss_commitment
grad_W_root
weight_root_after
```

---

### 10.4 LinearTrainingStep Receipt

```rust
struct LinearTrainingStepReceipt {
    receipt_id: ReceiptId,
    job_id: JobId,
    miner: Address,
    model_id: ModelId,
    step: u64,
    weight_root_before: Hash,
    batch_root: Hash,
    y_root: Hash,
    loss_commitment: Hash,
    grad_w_root: Hash,
    weight_root_after: Hash,
    trace_root: Hash,
    tensor_work_units: u64,
    execution_time_ms: u64,
    signature: Signature,
}
```

---

### 10.5 Why Include This in the MVP?

TensorOp demonstrates:

```text
miners can do verifiable tensor compute
```

LinearTrainingStep demonstrates:

```text
miners can do verifiable learning state transitions
```

This creates a path from synthetic compute to real training.

---

## 11. Verification Layer

The MVP verification stack is:

```text
Level 0: data availability checks
Level 1: redundant miner agreement
Level 2: Freivalds-style checks
Level 3: sampled row/cell checks
Level 4: training-step consistency checks
Level 5: future fraud proofs / ZK proofs
```

---

## 12. Freivalds Verification

### 12.1 Core Identity

For a claimed matrix multiplication:

```text
C = A @ B
```

sample a random vector `r` and check:

```text
C r = A (B r)
```

If `C` is correct, the equality always holds.

If `C` is incorrect, one random check catches the error with high probability.

Repeated checks reduce false acceptance probability exponentially.

This statement is true for a full Freivalds check over the whole output matrix. It is not true for a
row-sampled variant unless the sampling probability is included in the soundness bound.

---

### 12.2 Full Freivalds Check

Validator computes:

```text
x = B r
y = A x
z = C r
accept if y == z
```

Cost:

```text
full matmul verification: O(n^3)
Freivalds verification:   O(n^2)
```

This gives the desired asymmetry while still touching the full committed output. A validator can stream `C`
from chunks and compute `C r` without storing the full tensor. For MVP block eligibility, this is the default
verification path.

---

### 12.3 Row-Sampled Freivalds Check

For large tensors, validators may not download all of `C`.

Instead they check sampled rows.

For sampled row `i`:

```text
(C r)_i = A_i (B r)
```

Validator procedure:

```text
1. derive random vector r
2. compute x = B r
3. sample rows i_1 ... i_k
4. request committed rows of C
5. verify Merkle openings
6. check dot(C_i, r) == dot(A_i, x)
```

This reduces validator bandwidth, but it weakens soundness against sparse corruptions.

If an adversary corrupts `t` rows out of `m` and validators sample `s` distinct rows, row sampling catches
the corruption with probability:

```text
P_detect = 1 - C(m - t, s) / C(m, s)
```

For a one-row corruption this is only:

```text
P_detect = s / m
```

Therefore row-sampled Freivalds must not be the only validity check for block-eligible receipts unless the
protocol publishes a target false-accept probability and chooses `s` accordingly. Row sampling is acceptable
as:

```text
extra audit coverage
bandwidth-reduced monitoring
large-job telemetry before fraud proofs/ZK are available
```

It is not a substitute for full-output verification in the first MVP.

---

### 12.4 MVP Freivalds Parameters

Initial suggested parameters:

```text
validators_per_job: 8
full_freivalds_rounds_per_validator: 1
audit_rows_per_validator: 16
minimum_valid_attestations: max(5 validators, 2/3 assigned validator stake)
```

This yields:

```text
at least 5 full-output Freivalds attestations for block eligibility
up to 128 additional sampled-row audit checks per job
```

For small MVP jobs, full-output Freivalds should be mandatory. For larger jobs, row-sampled-only acceptance
requires a separate parameter study and must state the expected false-accept probability for sparse and dense
corruptions.

---

### 12.5 Randomness

Validator randomness must be unbiasable by the miner, proposer, and assigned validators.

Do not derive validation randomness from a block hash that the current proposer can grind after seeing
receipt roots. Use a finalized beacon or commit-reveal sequence:

```text
1. miner commits output root
2. receipt root is finalized or locked
3. validation beacon is revealed from prior validator commitments or prior finalized randomness
4. validators derive vectors and sampled rows
```

Recommended derivation:

```text
r_seed = H(finalized_validation_beacon || receipt_root || job_id || validator_address || round_id)
row_seed = H(r_seed || "rows")
```

Validators must not reveal sampled rows before the miner has committed the output root.

---

## 13. Verification of TensorOp Jobs

For `C = A @ B`, validator checks:

```text
A and B are generated from correct seeds, or input tensor openings are available
C commitment is available
input availability is sufficient to compute B r and A(B r)
full-output Freivalds check passes for block-eligible receipts
sampled C rows open correctly under Merkle root for additional audits
receipt signature is valid
receipt deadline is valid
```

Acceptance rule:

```text
A TensorOp receipt is valid if:
  - required input tensors are generated or available
  - output tensor data is available
  - required full-output Freivalds attestations pass
  - enough validators attest valid
  - enough redundant miners agree on output root, if redundancy is enabled
```

---

## 14. Verification of LinearTrainingStep Jobs

Validators verify the learning transition in pieces.

MVP training verification should use one algebraic domain for all consensus checks. Prefer a prime field.
If fixed-point values are used, represent scaled integers inside the field and define all scale factors as
public constants. Avoid saturating arithmetic and hardware rounding in consensus-critical identities.

### 14.1 Forward Check

Check:

```text
Y = X W_t
```

using Freivalds-style verification.

### 14.2 Error Tensor Check

Do not check only sampled entries. A miner could corrupt unsampled `dY` entries and still pass while poisoning
the backward pass or future state.

Use a random-linear check over the flattened tensor:

```text
<q, dY> = <q, Y> - <q, T>
```

where `q` is derived from the validation randomness after the miner commits `dY_root`.

### 14.3 Backward Check

Check:

```text
grad_W = X^T dY
```

using Freivalds-style verification.

### 14.4 Optimizer Check

Do not check only sampled entries. `W_{t+1}` is consensus state, so sparse corruptions are dangerous even if
most entries are correct.

Use a random-linear check over the flattened tensors:

```text
<q, W_{t+1}> = <q, W_t> - lr * <q, grad_W>
```

For small MVP shapes, a validator may also fully stream the tensors and check every update entry. For larger
shapes, the random-linear check gives algebraic coverage analogous to Freivalds for elementwise relations.

### 14.5 Loss Check

For MVP, loss should be auxiliary, not primary consensus.

Validators can sample entries and verify partial MSE consistency:

```text
loss_sample = mean((Y_sample - T_sample)^2)
```

But block validity should rely on the structural checks:

```text
forward correctness
full-tensor error relation
backward correctness
full-tensor optimizer relation
data availability
```

not exact global loss.

---

## 15. Redundant Agreement

For early MVP robustness, each job can be assigned to multiple miners.

Suggested parameters:

```text
replication_factor: 5
agreement_quorum: 3
```

A result root becomes a candidate if at least three independent miners produce the same root.
In the local reference, this quorum is enforced by distinct registered miner `operator_id` values rather
than by miner addresses alone, and redundant settlement delay records persist both the agreeing miner count
and the agreeing operator count for state-rooted audit evidence.

This is a robustness tool, not a proof of correctness. It only helps if the miners are actually independent.
With weak identity controls, a single operator can register multiple miners and satisfy the quorum.

MVP requirements:

```text
replicated miners must be sampled from stake- or identity-separated operators
agreement does not replace validator verification
disagreement triggers delayed settlement and additional full Freivalds checks
agreement quorum must be excluded from formal soundness unless Sybil resistance is specified
```

This mitigates:

```text
validator sampling weakness
single-miner cheating
implementation bugs
transient hardware faults
```

Over time, redundancy can be reduced as fraud proofs/ZK mature.

---

## 16. Data Availability

A miner’s receipt is invalid unless validators can retrieve requested tensor chunks before the verification deadline.

This is verification availability, not durable network data availability. A miner serving sampled rows during
verification can still disappear after settlement. The MVP must define the retention window explicitly.

Data availability check:

```text
validator requests row/chunk
miner returns bytes + Merkle proof
validator verifies proof against tensor root
```

Unavailable data means:

```text
receipt invalid
no spendable reward; existing pending receipt rewards are voided before maturity
reputation penalty
local v0 reference: miner bond slash once canonical block state applies the unavailable-data evidence
```

MVP retention rule:

```text
output tensors and required traces must remain retrievable until reward_settlement_delay + challenge_window
validators attest only to chunks they actually retrieved
receipts cannot be finalized if required full Freivalds streams cannot be served
large tensors should use external content-addressed storage or replication before public testnet rewards
```

---

## 17. Execution Receipts

### 17.1 Common Receipt Header

```rust
struct ReceiptHeader {
    receipt_id: ReceiptId,
    job_id: JobId,
    miner: Address,
    primitive_type: PrimitiveType,
    tensor_work_units: u64,
    execution_time_ms: u64,
    submitted_at_block: u64,
    signature: Signature,
}
```

### 17.2 Primitive Type

```rust
enum PrimitiveType {
    TensorOp,
    LinearTrainingStep,
    GraphExecution,
}
```

### 17.3 Trace Root

All receipts commit to an execution trace root.

```text
trace_root = MerkleRoot([
  H(op_0 || output_root_0),
  H(op_1 || output_root_1),
  ...
])
```

Trace roots prepare the system for later interactive fraud proofs.

Current local-reference status: fixed canonical TensorOp and LinearTrainingStep receipt constructors and
first-class graph-backed `GraphExecution` receipts derive this root from `TensorGraph::execute_exact`, and
verifiers require the same canonical trace root for otherwise-valid receipts. Exact IR execution now exposes
per-op `IrTraceOpening` Merkle proofs, and TensorOp, LinearTrainingStep, and GraphExecution receipts reject
trace openings from executions whose root differs from the receipt. The libp2p request-response surface
now samples those openings by `(trace_root, op_index)` using a bounded encoded payload and explicit missing
responses. The full interactive fraud-proof game over those openings remains future work.

---

## 18. Validator Attestations

```rust
struct ValidatorAttestation {
    validator: Address,
    receipt_id: ReceiptId,
    job_id: JobId,
    primitive_type: PrimitiveType,
    result: VerificationResult,
    checks_root: Hash,
    data_availability_passed: bool,
    signature: Signature,
}
```

```rust
enum VerificationResult {
    Valid,
    Invalid,
    Unavailable,
}
```

`checks_root` commits to the validator’s verification checks without necessarily revealing all check details
immediately.

For auditability, validators should reveal check details after the receipt settlement delay or during a
challenge. Hidden check details are useful against adaptive miners, but permanent secrecy makes validator
misconduct hard to prove.

---

## 19. Workload, Receipt, And Blockspace Lifecycle

```text
1. A local profile, user, or workload policy creates a TensorVM workload request.
2. Miners observe the workload request.
3. Miners execute the TensorVM program.
4. Miners commit output roots.
5. Miners submit receipts before the receipt deadline.
6. Receipt roots are locked for the validation window.
7. Validation randomness is revealed from an unbiasable finalized beacon.
8. Validators derive random checks for each receipt.
9. Validators request tensor openings.
10. Validators perform full-output Freivalds checks, random-linear checks, and sampled audits as required.
11. Validators submit attestations.
12. A receipt with sufficient valid attestations and data-availability evidence enters the settled-receipt pool.
13. The next block's canonical blockspace is selected deterministically from the settled-receipt pool.
14. Validators verify the canonical receipt set, commit to checks_root, and race to find a useful-verification
    PoW nonce over that checks_root.
15. The winning validator proposes a block containing the settled receipt set root and checks_root.
16. Stake-weighted validator finality confirms the block.
17. Miner, validator, PoW proposer, and treasury rewards are calculated after the verification challenge
    window.
```

---

## 20. Consensus Model

### 20.1 Hybrid Consensus

The MVP uses:

```text
validator useful-verification PoW
+
stake-weighted validator finality
```

Miners earn rewards by producing settled tensor receipts. Validators earn rewards by verifying receipts,
producing valid useful-verification PoW blocks, and voting on finality.

TensorWork is no longer a proposer-eligibility signal. It is used for miner reward weighting, blockspace TWU
accounting, and network metrics. This removes the circularity where current or recent receipts can influence
who gets to propose blocks.

The chain must also have a liveness fallback. Genesis, zero-receipt epochs, and network partitions cannot
halt block production merely because no useful receipt set is available.

Fallback rule:

```text
if no valid useful-verification PoW block appears within pow_timeout_blocks:
  a stake-weighted validator rotation may produce a PoW-skip fallback block
  the block may include no new settled receipts
  the proposer reward is reduced
  no miner TensorWork rewards are paid for empty blockspace
  workload generation continues with smaller fallback workloads
```

This keeps the chain live while making useful verification the normal block-production path.

---

### 20.2 Epochs

Suggested parameters:

```text
block_time: 6 seconds
epoch_length: 100 blocks
approx_epoch_duration: 10 minutes
```

Each epoch has:

```text
challenge generation
receipt submission
verification
settled-receipt pool update
useful-verification PoW difficulty retargeting
reward settlement
```

---

### 20.3 TensorWork Score

Each valid receipt contributes TensorWork Units.

```text
score_miner(epoch E) = sum(settled_valid_receipt.tensor_work_units from epoch E)
```

Miner reward weight is proportional to verified receipt TensorWork. Raw TensorWork drives blockspace
accounting and telemetry, but miner `settled_tensor_work` is not activated at receipt settlement time.
Newly settled receipt work is recorded as pending TensorWork until the corresponding miner receipt reward
claim survives inclusion, challenge windows, and maturity:

```text
reward_weight_miner(epoch E) =
  score_miner(epoch E) / sum(score_miner(epoch E))
```

Receipt-derived miner and validator rewards are first recorded as pending claims. A pending receipt reward
claim stores an explicit awaiting-inclusion maturity state until the receipt has been included in
blockspace, then converts to an inclusion-derived claimable height after at least
`reward_settlement_delay + verification_challenge_window` has elapsed from that inclusion point. Miner
TensorWork activates only when the non-voided miner receipt claim is claimed after maturity. A valid block-check,
invalid-output, or data-unavailability challenge before maturity voids the affected miner and validator
receipt claims, extends those claims to a state-rooted challenge hold height, and clears pending miner
TensorWork; matured voided claims are pruned without crediting spendable reward balances.

TensorWork does not affect block proposer selection. Validators produce blocks by proving useful verification
of the canonical settled-receipt set.

---

### 20.4 Finality

A block is finalized if:

```text
>= 2/3 validator stake signs the block
```

Validators check:

```text
useful-verification PoW target
canonical settled-receipt set root
checks_root recomputation
receipt validity
attestation quorum
reward calculation
state transition
parent validity
```

Block finality can be immediate under the BFT rule, but reward finality is delayed until the verification
challenge window closes. A finalized block can still have its proposer reward clawed back if its `checks_root`
is disproven.

---

### 20.5 Useful-Verification PoW

For a candidate block, validators build the canonical settled-receipt set from the parent state:

```text
receipt_set = select(finalized_beacon, settled_receipt_pool, blockspace_caps)
settled_receipt_set_root = MerkleRoot(receipt_set.receipt_id || receipt_hash || tensor_work_units)
```

Selection is deterministic:

```text
1. filter receipts that are settled, unspent, unexpired, and data-available through the challenge window
2. sort by H(finalized_beacon || parent_hash || receipt_id)
3. append receipts in order until adding the next receipt would exceed any blockspace cap
4. produce an empty set if the pool is empty
```

Validators then execute the required verification checks for every selected receipt and commit to the result:

```text
check_seed = H(finalized_beacon || parent_hash || receipt_id || "checks")
check_leaf = H(receipt_id || primitive_type || freivalds_transcript_root || random_linear_root || da_root)
checks_root = MerkleRoot(check_leaf_0, check_leaf_1, ...)
```

The useful-verification PoW header is:

```text
pow_header = H(parent_hash || settled_receipt_set_root || checks_root || finalized_beacon || validator_id)
```

A block satisfies PoW if:

```text
H(pow_header || nonce) < difficulty_target
```

A valid PoW block must satisfy all of:

```text
nonce meets difficulty_target
settled_receipt_set_root matches canonical selection from parent state
checks_root recomputes from the selected receipts and finalized beacon
proposer is a registered validator eligible for the epoch
block has stake-weighted finality signatures
```

This makes the proof of work useful only if verification dominates the candidate construction cost. The
initial parameter target should keep expected nonce-search time per validator less than or equal to expected
verification time for the canonical receipt set; otherwise validators could skip verification and brute-force
nonces.

Difficulty retargeting is bounded per epoch:

```text
observed_time = median_time(last_epoch_blocks)
ratio = clamp(observed_time / target_epoch_time, 1 / difficulty_retarget_max_ratio, difficulty_retarget_max_ratio)
new_target = old_target * ratio
```

Difficulty must also have a floor and ceiling so local CPU testnets remain live and public testnets cannot
collapse into trivial nonce search.

---

### 20.6 Blockspace

Blockspace is the bounded capacity for settled receipts in a block. The MVP has no fee market and no
validator-selected mempool ordering. Inclusion is deterministic.

The three independent caps are:

```text
block_twu_cap
block_byte_cap
block_receipt_cap
```

The canonical selector stops before the first receipt that would exceed any cap. This prevents both
single-mega-receipt and many-tiny-receipt attacks. TensorWork Units are a blockspace dimension and miner
reward metric, not a proposer-selection primitive.

Because v0 has no fee market, congestion is handled by deterministic carry-over. Receipts not included in
one block remain in the settled pool until included, expired, challenged, or pruned by retention policy.

---

### 20.7 Verification Challenge Window

Any registered validator may challenge a block's `checks_root` during `verification_challenge_window`.

A challenge provides:

```text
block_hash
receipt_id
expected_check_leaf
observed_check_leaf
opening against checks_root
recomputed verification transcript
challenger_signature
```

Current local-reference status: selected-receipt block openings expose a typed block-check transcript
containing the finalized beacon round/randomness, parent hash, check seed, selected-receipt leaf,
canonical receipt checks root, primitive type, TensorWork, and byte metadata. The transcript hash is the
expected `check_leaf` proven against `checks_root`, and block-check challenge admission verifies that
recomputed transcript commitment before accepting the mismatch proof. The multi-round interactive fraud
game over receipt `trace_root` remains future work.

If the challenge proves the block proposer committed to a wrong verification result:

```text
proposer block reward is clawed back
challenger receives a pending challenge reward claim that releases only after maturity
proposer's future PoW eligibility is throttled for a penalty window
the affected receipt is removed from reward settlement until reverified
```

MVP v0 may use reward clawback and throttling instead of hard stake slashing. Hard slashing should wait
until verifier correctness, transcript formats, and appeal paths are battle-tested.

---

## 21. Block Structure

```rust
struct TensorBlock {
    height: u64,
    parent_hash: Hash,
    epoch: u64,
    proposer: Address, // winning validator
    settled_receipt_set_root: Hash,
    checks_root: Hash,
    attestation_root: Hash,
    state_root: Hash,
    reward_root: Hash,
    beacon: Hash,
    difficulty_target: U256,
    nonce: u64,
    timestamp: u64,
    proposer_signature: Signature,
    validator_signature_aggregate: Signature,
}
```

---

## 22. Chain State

```rust
struct SettledReceipt {
    receipt_id: ReceiptId,
    receipt_hash: Hash,
    miner: Address,
    primitive_type: PrimitiveType,
    tensor_work_units: u64,
    byte_size: u64,
    settled_at_block: u64,
    expires_at_block: u64,
    attestation_quorum_root: Hash,
}

struct ChainState {
    accounts: Map<Address, Account>,
    miners: Map<Address, MinerState>,
    validators: Map<Address, ValidatorState>,
    jobs: Map<JobId, JobState>,
    receipts: Map<ReceiptId, ReceiptState>,
    settled_receipt_pool: BTreeMap<ReceiptId, SettledReceipt>,
    attestations: Map<ReceiptId, Vec<ValidatorAttestation>>,
    model_states: Map<ModelId, ModelState>,
    rewards: RewardState,
    difficulty: U256,
    last_retarget_epoch: u64,
}
```

---

## 23. Model State for Training Primitive

```rust
struct ModelState {
    model_id: ModelId,
    architecture_hash: Hash,
    weight_root: Hash,
    optimizer_state_root: Option<Hash>,
    step: u64,
    config_hash: Hash,
}
```

For MVP LinearTrainingStep, optimizer state can be empty because SGD does not require momentum.

Model-state transition rule:

```text
only one weight_root_after can be accepted for a given (model_id, step, weight_root_before)
duplicate receipts with the same valid weight_root_after may be rewarded as redundant execution
conflicting valid-looking roots trigger delayed settlement and expanded verification
receipts against stale weight_root_before are invalid for state transition, but may be kept as non-state audits
```

Deferred:

```text
momentum
Adam m/v states
scheduler state
compressed gradient state
```

---

## 24. Transactions

MVP transactions:

```rust
enum Transaction {
    RegisterMiner,
    RegisterValidator,
    SubmitTensorOpReceipt,
    SubmitLinearTrainingStepReceipt,
    SubmitAttestation,
    Transfer,
    ClaimReward,
}
```

Optional MVP+:

```rust
SubmitUserTensorJob
SubmitModelRegistration
SubmitDatasetRegistration
ChallengeReceipt
```

---

## 25. Rewards and Economics

### 25.1 Reward Sources

```text
protocol emissions
transaction fees
future user job fees
```

### 25.2 Reward Split

Recommended MVP split:

```text
60% miners
20% validators
15% useful-verification PoW proposer
5% treasury
```

### 25.3 Miner Rewards

The local reference allocates the miner reward pool by raw verified receipt TensorWork from §20.3:

```text
miner_reward = miner_valid_tensorwork / sum(valid_tensorwork_by_miner) * miner_reward_pool
```

Each receipt-level miner claim remains traceable and delayed. The miner's pending TensorWork becomes
settled TensorWork only when that delayed claim is claimed after maturity.

### 25.4 Validator Rewards

```text
validator_reward = validator_valid_attestations / total_valid_attestations * validator_reward_pool
```

### 25.5 Proposer Rewards

```text
proposer_reward = fixed_block_reward_for_valid_useful_verification_pow
```

The proposer is the winning validator, not a miner selected by TensorWork. In v0 there is no fee market and
no miner inclusion bid. The proposer reward is first recorded as a pending claim and becomes spendable only
after the reward-settlement delay plus `verification_challenge_window` closes without a successful
`checks_root` challenge.

---

## 26. Penalties

### 26.1 MVP v0 Penalties

```text
invalid receipt: no reward
unavailable data: void pending receipt rewards + reputation penalty + state-rooted miner bond slash in the local reference
invalid attestation: no reward + reputation penalty; local reference can slash and void delayed validator receipt reward when a configured mandatory audit contradicts the attestation
missed validation assignment: no reward; local reference can slash and void delayed validator receipt reward when a configured mandatory audit is missed
```

### 26.2 MVP v1 Penalties

```text
invalid receipt: miner stake slash
invalid attestation: validator stake slash
data withholding: miner stake slash
collusion proof: slash + ban window
```

Recommendation:

```text
Hard slashing should remain narrowly scoped until verification code is battle-tested. The local reference
currently hard-slashes miner bond for data-unavailable receipts and can hard-slash validators for missed or
contradicted mandatory audits when audit sampling is configured. It also records invalid-output slashing,
runtime audit-report submission, appeal records/resolution, and live fraud-path bond calibration for the
implemented local paths. Public deployed-run measurements, remaining fraud paths, and adversarial evidence
still require battle-tested proof before public rewards are uncapped.
```

Consequence:

```text
v0 still has incomplete economic security.
Most invalid work is deterred mainly by non-payment, reputation loss, redundancy, and audits; data
withholding and invalid-output evidence now also carry state-rooted miner bond slashes in the local
reference, and validator audits can slash/appeal through delayed reward state.
Public rewards should be capped until those flows have public deployed-run evidence and the remaining fraud
paths are implemented.
```

---

## 27. TensorWork Units

TensorWork Units represent normalized verified tensor computation. In v2, TensorWork Units are used for miner
reward weighting, blockspace capacity, telemetry, and concentration analysis. They do not select block
proposers.

Initial simple model:

```text
matmul(m,k,n) = 2 * m * k * n base units
add(numel) = numel
sub(numel) = numel
mul(numel) = numel
reduce_sum(numel) = numel
sgd_update(numel) = 2 * numel
```

LinearTrainingStep cost:

```text
forward_matmul + backward_matmul + optimizer_update + auxiliary ops
```

The cost model should be governance/config upgradeable. Any update must preserve deterministic TWU
calculation from receipt contents so every validator computes the same blockspace cap usage.

---

## 28. MVP Parameters

Suggested genesis parameters:

```text
block_time: 6 seconds
epoch_length: 100 blocks
receipt_submission_window: 20 blocks
verification_window: 40 blocks
reward_settlement_delay: 1 epoch
replication_factor: 5
agreement_quorum: 3
validators_per_job: 8
full_freivalds_rounds_per_validator: 1
audit_rows_per_validator: 16
minimum_valid_attestations: max(5 validators, 2/3 assigned validator stake)
block_twu_cap: 10_000_000_000 base units
block_byte_cap: 4 MiB
block_receipt_cap: 512 receipts
difficulty_target_block_time: 6 seconds
difficulty_retarget_epoch_length: 100 blocks
difficulty_retarget_max_ratio: 4
pow_timeout_blocks: 2
verification_challenge_window: 1 epoch
miner_min_stake: 100 tokens
validator_min_stake: 10,000 tokens
chunk_size: 1 MiB
tensor_retention_epochs: 2
challenge_window: 1 epoch
```

The initial local reference treats the effective receipt-reward maturity delay as
`max(reward_settlement_delay + challenge_window, validator_audit_window when audit sampling is enabled)`
from the block that includes the settled receipt. Tensor/trace retention must cover the same or longer
period so auditors and challengers can still retrieve the data needed to dispute the included receipt.

Initial TensorOp shapes:

```text
small_matmul: 1024 x 1024 x 1024
medium_matmul: 4096 x 4096 x 4096
```

Initial LinearTrainingStep shape:

```text
batch: 1024 x 1024
weights: 1024 x 1024
target: 1024 x 1024
```

Start smaller if network bandwidth is constrained.

---

## 29. Networking

The network boundary is consensus-critical. In the MVP, every counted miner, validator, and validator
proposer is a separate long-running node process with its own durable state, libp2p identity, listen
address, peer connections, and role loop. Consensus progress must be observable as interprocess messages
between these nodes, not as one `tvmd` process mutating shared memory on behalf of multiple roles.

Required P2P messages:

```text
NewBlock
NewJob
NewReceipt
NewAttestation
RequestTensorChunk
TensorChunkResponse
RequestTensorRow
TensorRowResponse
RequestProgram
ProgramResponse
PeerInfo
```

Required process-boundary rules:

```text
jobs move from workload source to miners through libp2p gossip or node RPC submission
miner receipts move from miner processes to validators through libp2p gossip or node RPC submission
validator tensor fetches use request-response or node RPC against the serving miner process
validator attestations move from validator processes to the network through libp2p gossip or node RPC submission
validator-proposed blocks move through libp2p block gossip
finality votes move through validator node messages
node stores are updated only after the local node validates an inbound network/RPC event through the shared chain engine
```

Rejected shortcuts:

```text
one tvmd process creating jobs, receipts, attestations, blocks, and votes for all roles
direct in-memory propagation between counted operators
service-loop callbacks that mutate another role's state without a network/RPC event
single-process tests counted as multi-participant acceptance evidence
checker-only status fields that are not backed by interprocess node messages
compatibility paths that preserve the old service-owned local producer
```

Pure unit tests may call the chain engine directly to test deterministic state transitions. Any local-chain,
Gate 0, public-testnet, or production-readiness acceptance claim must use separate node processes and
interprocess libp2p/RPC messages.

---

## 30. APIs

### 30.1 Node RPC

```text
GET  /health
GET  /rpc/health
GET  /chain/head
GET  /chain/block/:height
GET  /epoch/current
GET  /jobs/current
GET  /jobs/:job_id
GET  /receipts/:receipt_id
GET  /miners/:address
GET  /validators/:address
GET  /explorer/health
GET  /telemetry/health
GET  /faucet/health
POST /tx
POST /receipt
POST /attestation
```

### 30.2 Tensor Data RPC

```text
GET /tensor/:tensor_id/descriptor
GET /tensor/:tensor_id/chunk/:chunk_index
GET /tensor/:tensor_id/row/:row_index
GET /tensor/:tensor_id/opening/:chunk_index
```

---

## 31. CLI

`tvmd` is a process launcher, operator CLI, and local node adapter. It is not the consensus orchestrator.
The binary may parse config, initialize durable storage, start a role node, expose operator RPC surfaces, and
submit messages to the local node. It must not privately perform cross-role consensus work that should be
done by separate miner and validator processes.

`tvmd` commands must respect this boundary:

```text
tvmd miner run      starts exactly one miner node role
tvmd validator run  starts exactly one validator node role, including useful-verification PoW eligibility
tvmd node serve  starts exactly one node service surface for the configured role/profile
tvmd node init   initializes one node data directory
tvmd node peer   edits one node's peer book
```

No `tvmd` command may satisfy production-readiness by internally simulating multiple counted operators,
injecting receipts or attestations for other roles, or assembling blocks from local-only deterministic replay.
If a role needs data produced by another role, it must obtain it through libp2p or node RPC, even in local
CPU mode.

### 31.1 Miner CLI

```bash
tvmd miner register --stake 100

tvmd miner check \
  --wallet miner.key \
  --device cuda:0 \
  --node /ip4/127.0.0.1/tcp/4001

tvmd miner run \
  --wallet miner.key \
  --device cpu \
  --node /ip4/127.0.0.1/tcp/4001 \
  --listen 0.0.0.0:8545 \
  --p2p-listen /ip4/0.0.0.0/tcp/4001 \
  --data-dir /var/lib/tensorvm \
  --identity-seed <32-byte-hex-seed> \
  --auth-token service-token \
  --max-requests 0

tvmd miner status
```

`--device cpu` selects the deterministic CPU reference backend used by Gate 0. `--device cuda:N` is a
GPU-miner claim and must fail unless the binary was built with `--features cuda-kernels` and device `N`
is present; CUDA readiness cannot be satisfied by a default CPU build. `miner start` is the preflight
readiness surface; `miner run` is the long-running role entrypoint used by the local CPU Compose gate.

### 31.2 Validator CLI

```bash
tvmd validator register --stake 10000

tvmd validator check \
  --wallet validator.key \
  --node /ip4/127.0.0.1/tcp/4001

tvmd validator run \
  --wallet validator.key \
  --node /ip4/127.0.0.1/tcp/4001 \
  --listen 0.0.0.0:8545 \
  --p2p-listen /ip4/0.0.0.0/tcp/4001 \
  --data-dir /var/lib/tensorvm \
  --identity-seed <32-byte-hex-seed> \
  --auth-token service-token \
  --max-requests 0

tvmd validator status
```

`validator start` is the preflight readiness surface; `validator run` is the long-running role entrypoint
used by the local CPU Compose gate.

### 31.3 Service CLI

```bash
tvmd node init \
  --data-dir /var/lib/tensorvm

tvmd node peer add \
  --data-dir /var/lib/tensorvm \
  --peer-id "$BOOTSTRAP_PEER_ID" \
  --address /dns/bootstrap.tensorvm.net/tcp/4001

tvmd node serve \
  --listen 0.0.0.0:8545 \
  --p2p-listen /ip4/0.0.0.0/tcp/4001 \
  --data-dir /var/lib/tensorvm \
  --auth-token service-token \
  --max-requests 0
```

Miner and validator `--node` values are libp2p multiaddrs. The RPC listener remains HTTP for
operator APIs, but `tvmd node serve` must start the mandatory rust-libp2p control plane for node
discovery, Gossipsub propagation, and tensor/program request-response protocols.
`tvmd node peer add` persists durable bootstrap peer records under the node data directory; service
startup must load those records, preserve the peer IDs in `/p2p/<peer-id>` dial multiaddrs, and pass them
to the mandatory libp2p Kademlia/bootstrap path. Non-bootstrap public nodes must seed at least one
reachable TCP bootstrap peer before starting the public run.
libp2p is a normal, unconditional TensorVM runtime dependency: no TensorVM feature flag may disable it,
no shim or in-memory substitute may satisfy node propagation, and `default-features = false` on the
upstream `libp2p` crate is only allowed to narrow the explicit protocol surface compiled into TensorVM.
`--max-requests 0` means serve continuously; positive values are for supervised smoke tests and controlled
rollout checks.

The `tvmd` executable must be an explicit Cargo binary target. Public deployment templates live under
`deploy/tensorvm/` and must use `tvmd node serve` with a required libp2p listen multiaddr, external TLS
termination, plus the health and content endpoints required by the public preflight and post-run evidence
validators.
Those templates include checked preflight and non-full-spec post-run evidence example manifests; the
post-run example is only a signature-domain and parser shape check and cannot substitute for the required
7-day external public-run evidence.
The preflight manifest must include a `cuda_ready_miner_count` equal to the planned `miner_count`, derived
from successful `tvmd miner check --device cuda:N` readiness checks on the planned public miner hosts.
It must also include a `libp2p_ready_node_count` equal to `miner_count + validator_count`, derived from
successful mandatory-libp2p node readiness checks on the planned public miners and validators.
The `tvmd node check --p2p-listen <multiaddr> --data-dir <path>` command loads the initialized
node store and durable peer book, starts the real rust-libp2p control plane with the same bounded runtime
configuration used by `tvmd node serve`, reports `libp2p_ready=true`, and exits; this command is the
intended per-node preflight source for `libp2p_ready_node_count`.
Each repeated preflight `service=...` plan must contain exactly eight comma-separated, nonempty,
untrimmed values; leading or trailing whitespace in any service value is a manifest parse error.
Public preflight deployment readiness requires exactly one ready `service=...` plan for each RPC, explorer,
faucet, and telemetry surface. Missing, duplicate, or extra service plans do not satisfy the public service
plan gate.
The external operator flow, daily evidence collection, post-run validation, and publication steps are
recorded in `deploy/tensorvm/RUNBOOK.md`.

### 31.4 Public Testnet Evidence CLI

The `docs/tensorvm/public-testnet.preflight` and `docs/tensorvm/public-testnet.evidence` paths must exist
as checked pending manifests for the commands below. They are parser/readiness fixtures until replaced
with owned public endpoint IDs, externally reachable HTTPS services, and real records from a 7-day public
run; they must not be counted as full public-testnet evidence while they contain placeholder hosts or short
run data.

```bash
tvmd public preflight \
  docs/tensorvm/public-testnet.preflight

tvmd public evidence validate \
  docs/tensorvm/public-testnet.evidence

tvmd public evidence validate \
  deploy/tensorvm/manifests/public-testnet.evidence.example

tvmd node check \
  --p2p-listen /ip4/0.0.0.0/tcp/4001 \
  --data-dir /var/lib/tensorvm

tvmd public evidence publish \
  --bundle-id <bundle-id-hex> \
  --public-uri https://tensorvm.net/tensorvm/public-evidence.json \
  --manifest-signer <manifest-signer-address-hex> \
  --manifest-signature-count 1 \
  --independent-auditor-count 1

tvmd public evidence audit \
  --bundle-id <bundle-id-hex> \
  --public-uri https://tensorvm.net/tensorvm/public-evidence.json \
  --auditor-id <auditor-address-hex> \
  --audit-uri https://auditor.tensorvm.net/tensorvm/audit.json \
  --observed-at <unix-seconds>

tvmd public evidence run window \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --started-at <unix-seconds> \
  --ended-at <unix-seconds-plus-at-least-604800> \
  --observed-blocks 100800

tvmd public evidence run window-file \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --block-observation-file artifacts/block-observations.records

tvmd public evidence node heartbeat \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --first-block 0 \
  --last-block 100799 \
  --heartbeat-count 100800

tvmd public evidence node heartbeat-file \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --heartbeat-file artifacts/miner-a-heartbeats.records

tvmd public evidence node operator-attestation \
  --role miner \
  --address <node-address-hex> \
  --operator-id <operator-id-hex> \
  --identity-uri https://operator-a.tensorvm.net/tensorvm.json \
  --observed-at <unix-seconds>

tvmd public evidence service health \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/health \
  --health-path /health \
  --first-block 0 \
  --last-block 100799 \
  --reachable-count 100800 \
  --signed-health-check-count 100800

tvmd public evidence service health-file \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/health \
  --health-path /health \
  --observation-file artifacts/rpc-health.records

tvmd public evidence service content \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --content-root <content-root-hex> \
  --observed-at <unix-seconds> \
  --min-content-bytes 64

tvmd public evidence service content-bytes \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --observed-at <unix-seconds> \
  --content-hex <captured-response-body-hex>

tvmd public evidence service content-file \
  --kind rpc \
  --endpoint-id <endpoint-id-hex> \
  --public-url https://rpc.tensorvm.net/chain/head \
  --content-path /chain/head \
  --observed-at <unix-seconds> \
  --content-file artifacts/rpc-chain-head.body

tvmd public evidence network observation \
  --operator-id <operator-id-hex> \
  --peer-id <libp2p-peer-id> \
  --listen-address /dns/node-a.tensorvm.net/tcp/4001 \
  --observed-at <unix-seconds> \
  --gossip-topics 5 \
  --request-response-protocols 4 \
  --bootstrap-peers 2 \
  --max-transmit-bytes 1048576 \
  --request-timeout-seconds 10 \
  --max-concurrent-streams 128 \
  --idle-timeout-seconds 60

tvmd public evidence network from-service-log \
  --operator-id <operator-id-hex> \
  --listen-address /dns/node-a.tensorvm.net/tcp/4001 \
  --observed-at <unix-seconds> \
  --service-log artifacts/node-a-tvmd-service.log

tvmd public evidence record summary \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-root <network-runtime-root-hex> \
  --record-count <operator-count>

tvmd public evidence record artifact \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-root <network-runtime-root-hex> \
  --record-count <operator-count>

tvmd public evidence record artifact-roots \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-roots <comma-separated-record-roots>

tvmd public evidence record artifact-file \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --artifact-uri https://evidence.tensorvm.net/tensorvm/network-runtime.json \
  --record-file artifacts/network-runtime.records

tvmd public evidence record summary-roots \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-roots <comma-separated-record-roots>

tvmd public evidence record summary-file \
  --kind network-runtime \
  --bundle-id <bundle-id-hex> \
  --manifest-signer <manifest-signer-address-hex> \
  --record-file artifacts/network-runtime.records
```

The `publication`, `auditor-record`, `run-window`, `node-heartbeat`, and `operator-attestation` commands
emit the signed manifest fields for the public evidence location, independent auditor records, wall-clock
run duration, external operator node heartbeats, and operator identity attestations. Counted node
heartbeats must cover the manifest's full observed block count, counted operators must also have distinct
node addresses through a one-to-one matching of live operator IDs to live node addresses, and counted miner
and validator operator/address sets must be disjoint. Unique operator and address totals without such a
matching do not satisfy public operator independence. Public operator matching must be criteria-aware:
when greedy role ordering or address choice would miss a satisfiable set for the requested miner and
validator minima, the evidence gate must count a satisfying independent operator/address set. The current evidence
manifest has exactly one `manifest_signature` field, so
`manifest_signature_count` must be `1`. Public evidence and preflight manifests must reject duplicate
scalar fields, whitespace-padded field keys, and whitespace-padded scalar values. Public evidence
repeated-record fields and preflight `service=...` records must also reject whitespace-padded
comma-separated values; only explicitly repeated record fields such as `node=`, `service=`, `auditor=`,
`record_artifact=`, `operator=`, `network_runtime_observation=`, and `service_content=` may appear more
than once.
For `record_artifact=`, the independently checkable public evidence gate still requires exactly one valid
line for each required supporting-record kind and rejects extra artifact locators.
The `auditor-record` command emits the exact `auditor=...` manifest line for an external audit artifact
bound to the evidence bundle ID, public evidence URI, auditor ID, and observation time; counted auditor
IDs must differ from the manifest signer and must be observed at or after the signed run-window end. The
independently checkable evidence gate requires the number of valid signed `auditor=` lines to match
`independent_auditor_count` exactly; missing, invalid, or extra auditor records do not satisfy the gate.
Content-addressed `ipfs://` and `ar://` evidence, auditor, artifact, or operator-identity URIs must start
with a well-formed identifier segment using only ASCII alphanumerics, `-`, or `_`, with no raw whitespace
or control characters. Optional path segments after the identifier must use only ASCII alphanumerics,
`.`, `-`, or `_`, must not be empty, must not be `.` or `..`, and must not include query strings,
fragments, backslashes, raw whitespace, or control characters.
The `evidence run window-file` command derives the signed run-window record from saved
`run_window_observation=<block>,<unix-seconds>` lines. Blank lines and `#` comments are ignored;
duplicate blocks, non-contiguous block observations, zero timestamps, decreasing timestamps, unsupported
lines, and whitespace-padded records are rejected so the signed start time, end time, and observed block
count come from raw block observations instead of hand-copied counters.
The `operator-attestation` command emits the exact `operator=...` manifest line for an external operator
identity URI bound to a node address, role, operator ID, and observation time. Counted operator
attestations must be observed inside the signed run window, and the
`operator_identity_attestation_records` manifest count must exactly match the number of repeated
`operator=...` lines and the number of valid signed operator-attestation records for the exact independent
operator/address pairs selected by the public operator matching. A valid attestation for a live but
uncounted node does not satisfy a missing counted operator attestation. Missing, invalid, duplicate,
uncounted, or extra operator-attestation records do not satisfy external-operator evidence.
The `evidence node heartbeat-file` command derives the same signed `node=...` line from a saved
line-oriented heartbeat observation file using
`node_heartbeat_observation=<role>,<node-address-hex>,<operator-id-hex>,<block>` records. Blank lines and
`#` comments are ignored; duplicate blocks, non-contiguous block observations, identity mismatches,
unsupported lines, and whitespace-padded records are rejected so first block, last block, and heartbeat
count are derived rather than copied by hand.
The `evidence service health` command emits the exact `service=...` manifest line for RPC, explorer, faucet, or
telemetry evidence. The signature is bound to the service kind, endpoint ID, external HTTPS URL, health
path, observed block range, reachable observation count, and signed health-check count; the public URL path
must match the signed health path. Counted service-health records must cover the manifest's full observed
block count with both reachable observations and signed health checks, and the reachable count must not
exceed the signed health-check count.
The `evidence service health-file` command derives the same signed line from a saved line-oriented observation
file using `service_health_observation=<block>,reachable` or
`service_health_observation=<block>,unreachable` records. Blank lines and `#` comments are ignored;
duplicate blocks, non-contiguous block observations, unsupported lines, and whitespace-padded records are
rejected so the derived first block, last block, reachable count, and signed health-check count are not
manually copied.
The `evidence service content` command emits the exact `service_content=...` manifest line for RPC, explorer,
faucet, or telemetry content evidence. The signature is bound to the service kind, endpoint ID, external
HTTPS URL, content path, content root, observation time, and minimum observed content bytes. Counted
service-content records must be observed inside the signed run window and prove at least 64 observed bytes.
The `evidence service content-bytes` and `evidence service content-file` commands derive the content root from the
exact captured response body bytes using the TensorVM service-content-root domain and emit the same signed
`service_content=...` manifest line with `min_content_bytes` set to the captured byte length. Operators
should prefer the file form when turning raw public service observations into evidence, and keep the exact
captured files as raw supporting artifacts.
Deployed public service evidence must include exactly one signed health record and exactly one signed
content record for each RPC, explorer, faucet, and telemetry service surface, with matching endpoint IDs
and matching HTTPS authorities for `/chain/head`, `/explorer`, `/faucet/page`, and
`/telemetry/dashboard`. Extra service-health or service-content records, reused endpoint IDs, or reused
service-content roots do not satisfy the public service gate.
Public service health and content URLs must use concrete non-root paths that exactly match the signed path,
with no query string or fragment.
External public URLs must use well-formed HTTPS authorities and must not use userinfo, whitespace, invalid
DNS host labels, single-label DNS hosts, invalid ports, malformed bracketed IPv6 authorities, localhost,
private, link-local, documentation, shared-address, benchmarking, multicast, reserved IP literals, or
special-use DNS names such as `.localhost`, `.local`, `.test`, `.example`, `.invalid`, `example.com`,
`example.net`, and `example.org`.
The `evidence network observation` command emits a signed `network_runtime_observation=...` record line for a
public libp2p multiaddr, observed peer ID, discovery peer count, Gossipsub/request-response protocol
counts, and DoS-control limits. The public libp2p multiaddr must include a nonzero TCP listen port and
must not use localhost, `.local`, loopback, unspecified, private, link-local, special-use DNS names,
malformed DNS labels, single-label DNS hosts, documentation, shared-address, benchmarking, multicast, or
reserved IP hosts. Full-spec evidence must include exactly one valid signed network-runtime observation
record for every counted public miner and validator operator. Each record must be bound to that operator
ID, a valid libp2p peer ID, a public listen multiaddr, an observation timestamp inside the signed run
window, nonzero discovery/gossip/request-response/DoS-control counters, a matching observation root, and a
matching observation signature. The `network-runtime` summary root must aggregate those raw observation
roots; a signed summary root without the corresponding per-operator `network_runtime_observation` manifest
lines does not satisfy the public evidence gate.
The `evidence network from-service-log` command derives the peer ID, protocol counts, bootstrap-peer
count, and DoS-control settings from an exact captured `tvmd node serve` log. It must reject logs that
do not show `command=service_serve` and `p2p_runtime=libp2p`, and it still requires the observer-supplied
public listen multiaddr above; a loopback or private service log cannot be promoted into public evidence.
The `evidence record summary` command emits the exact `<record>_records`, `<record>_root`, and
`<record>_signature` manifest lines for block history, finality history, production libp2p network
observations, randomness-beacon evidence, data-availability measurements, invalid-work rejections,
reward settlements, deployed detection measurements, or validator VRF lifecycle records. Supported record kinds are `block-history`,
`finality-history`, `network-runtime`, `randomness-beacon`, `data-availability`, `invalid-work`,
`reward-settlement`, `detection-measurement`, and `validator-vrf-lifecycle`.
The `network-runtime` record count must equal the counted independent public miner plus validator operator
total; signed undercounts or overcounts cannot satisfy independently checkable evidence.
The `evidence record artifact` command emits a signed `record_artifact=...` line that binds an external raw-record
artifact URI to the record kind, root, and count; independently checkable public evidence requires one
valid artifact locator for every required supporting-record summary root and exactly nine supporting
artifact locators total: block history, finality history, network runtime, randomness beacon, data
availability, invalid work, reward settlement, detection measurement, and validator VRF lifecycle.
The `evidence record artifact-roots` command derives the same aggregate root and count as
`evidence record summary-roots` before signing the artifact locator, so the summary line and artifact locator
can be generated from the same raw record-root list.
The `evidence record summary-roots` command deterministically aggregates unique comma-separated
supporting-record roots, rejects duplicate roots that would pad the derived count, rejects empty or
whitespace-padded root entries, and emits the same signed manifest summary fields so operators do not need
an out-of-band root-signing tool for post-run bundles.
The `evidence record summary-file` and `evidence record artifact-file` commands derive the same aggregate root and
count from a saved line-oriented raw-record file. Blank lines and `#` comments are ignored; generic
supporting-record root files use `record_root=<hex>` lines, and network-runtime files may contain the
exact signed `network_runtime_observation=...` lines emitted by `evidence network observation` or
`evidence network from-service-log`. Non-network supporting-record files may contain exact
`block_history_record=...`, `finality_history_record=...`, `randomness_beacon_record=...`,
`data_availability_measurement=...`, `invalid_work_rejection=...`, `reward_settlement=...`, or
`detection_measurement=...`, or `validator_vrf_lifecycle=...` raw record lines. Those typed raw lines are
validated against the selected record kind before hashing with the record kind and exact line bytes, so
operators can derive summary roots and artifact locators from captured records without precomputing each
`record_root=<hex>` by hand. The accepted typed fields are
`block_history_record=<block>,<block-root-hex>`,
`finality_history_record=<block>,<block-root-hex>,finalized|unfinalized`,
`randomness_beacon_record=<source-id-hex>,<round>,<randomness-root-hex>,<proof-root-hex>,drand-v1|validator-vrf-v1|local-deterministic-fixture-v1,<observed-block>,accepted|rejected`,
`data_availability_measurement=<receipt-root-hex>,available|unavailable,<block>`,
`invalid_work_rejection=<receipt-root-hex>,rejected,<block>`,
`reward_settlement=<receipt-root-hex>,<miner-id-hex>,<validator-id-hex>,<block>`,
`detection_measurement=<mechanism>,<subject-root-hex>,<sample-count>,<detected-count>,<block>`, and
`validator_vrf_lifecycle=<receipt-root-hex>,<validator-id-hex>,<beacon-round>,committed|revealed,<block>`.
Whitespace-padded record lines or empty fields are rejected, reward-settlement participant IDs must be
valid 64-character hex IDs, detection mechanisms must be lowercase ASCII letters, digits, or `-`,
sample count must be nonzero, and detected count cannot exceed sample count.
Run-level counters must be internally consistent before the public evidence gate can pass: finalized
blocks cannot exceed observed blocks, and available tensor receipts cannot exceed checked tensor receipts.
The post-run evidence manifest must also include `cuda_verified_miner_count`, derived from CUDA kernel and
device checks for the counted public miner set, `cuda_graph_execution_receipts`, derived from CUDA
graph-execution receipt checks in the public run, and signed `validator_vrf_lifecycle_records` plus raw
`validator_vrf_lifecycle=...` records derived from the direct deployed validator VRF commit-to-reveal
reward-delay lifecycle for checked receipts with distinct receipt roots. Full-spec evidence requires
the CUDA miner count to cover the counted public miners, the CUDA graph-execution receipt count to be
positive without exceeding checked or available receipt counts, raw data-availability measurement records
to equal the checked receipt count with distinct nonzero receipt roots, raw invalid-work and
reward-settlement records to use distinct nonzero receipt roots and nonzero settlement participant IDs,
signed validator VRF lifecycle
records to equal the checked receipt count, use distinct receipt roots, and aggregate to the signed lifecycle root,
and positive signed deployed detection-measurement records with raw
records that aggregate to the signed detection summary; a public run with otherwise valid services,
operators, network observations, and raw supporting records but missing, undercounted, or overcounted
CUDA/graph/VRF-lifecycle/detection evidence must report
`public_evidence_full_spec=false`.
`public_evidence_full_spec=true` is reserved for default-or-stricter public-testnet criteria; shortened or
relaxed local harness criteria may exercise the validator but must not mark evidence as full-spec.

---

## 32. Reference Implementation

The reference implementation should live in a Cargo workspace so multiple chain designs can be developed
side-by-side without mixing protocol experiments.

Required repository structure for this workspace:

```text
chain/
  Cargo.toml
  README.md
  tarpaulin.toml
  crates/
    experiments/
      Cargo.toml
      README.md
      docs/
        README.md
        pearl/
          README.md
          pearl.pdf
        ambient/
          README.md
          Ambient_Litepaper_V1.pdf
        reviews/
          README.md
        attacks/
          README.md
      src/
    tensor_vm/
      Cargo.toml
      README.md
      src/
    tensor_vm_explorer/
      Cargo.toml
      README.md
      src/
  deploy/
    tensorvm/
      README.md
      env/
      manifests/
      nginx/
      systemd/
  docs/
    README.md
    tensorvm/
      README.md
      mvp_spec.md
      coverage_matrix.md
      implementation_status.md
      networking_choice.md
      tarpaulin_report.md
      torchlean_verification_analysis.md
```

Workspace requirements:

```text
the repository root must be a Cargo workspace
TensorVM implementation code must live under crates/tensor_vm/
TensorVM explorer UI code must live under crates/tensor_vm_explorer/ and ship as a default feature
non-TensorVM protocol experiments must live under crates/experiments/
each crate must have its own README
the root README must describe the workspace and link to crate-level READMEs
TensorVM specs, reports, and evidence docs must live under docs/tensorvm/
experiment papers, reviews, and attack probes must live under crates/experiments/docs/
deployment templates and launch manifests must live under deploy/<name>/
crates/tensor_vm/ must be self-contained and must not depend on the experiments crate
```

Recommended internal module structure for TensorVM:

```text
api
chain
challenge
cli
error
faucet
jobs
merkle
miner
p2p
rpc
runtime
scheduler
storage
study
telemetry
tensor
tensor_server
testnet
txpool
types
validator
verify
vm
watcher
```

Recommended languages:

```text
Rust: node, state, commitments, consensus, P2P
Rust/C++/CUDA: optimized TensorVM kernels
Python: research harness
```

Must include:

```text
CPU reference backend
GPU miner backend
cross-machine determinism tests
invalid-output test harness
restartable node storage
libp2p P2P runtime, P2P codec, RPC codec, and RPC socket tests
TensorVM node explorer WebSocket endpoint and standalone explorer crate with a default terminal-style
Ratzilla/Ratatui UI path
```

Required local verification commands before sharing changes:

```bash
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
```

Test and coverage requirements:

```text
all workspace tests must pass
TensorVM library tests must cover every line in crates/tensor_vm/src
coverage reports must be generated from the workspace root with cargo tarpaulin
any remaining uncovered lines must be outside TensorVM or explicitly documented
branch coverage should be reported when the installed coverage tool supports it
```

### 32.1 Autonomous Agent Completion Contract

This spec is intended to be executable by an autonomous coding agent. The agent must treat the document as
a completion contract, not as background reading. It must continue implementation, verification, and
documentation until every local and deployment-gated acceptance item below is either complete with evidence
or explicitly marked blocked by missing external infrastructure.

Codex goal command from the repository root:

```bash
codex exec -C . --sandbox workspace-write --ask-for-approval on-request \
  "Goal: fully complete docs/tensorvm/mvp_spec.md for TensorVM (TVM). Implement every incomplete local reference item in crates/tensor_vm, keep tensor_vm self-contained and independent of experiments, update docs/tensorvm/coverage_matrix.md, docs/tensorvm/implementation_status.md, and docs/tensorvm/tarpaulin_report.md, run every relevant test and experiment before claiming each iteration complete, run cargo fmt --check --all, cargo test --workspace --release, cargo clippy --workspace --all-targets -- -D warnings, and cargo tarpaulin from the workspace root, and do not declare full-spec completion unless real CUDA/C++ kernels, production libp2p runtime, deployed RPC/explorer/faucet/telemetry services, and independently checkable 7-day public-testnet evidence are present and documented."
```

If the local wrapper is named `docex` instead of `codex`, use the same command with only the binary name
changed.

Autonomous agent operating rules:

```text
read this spec, docs/tensorvm/coverage_matrix.md, docs/tensorvm/implementation_status.md, and
docs/tensorvm/tarpaulin_report.md before editing code
preserve the Cargo workspace structure and keep each chain design in its own crate
do not make crates/tensor_vm depend on crates/experiments
prefer deterministic Rust reference implementations before optimized kernels
add tests for every new consensus-critical branch
run all relevant tests, study harnesses, and experiments before claiming any iteration is
complete
run Gate 0 as the first executable acceptance gate in every new or resumed MVP implementation iteration
update the coverage matrix when acceptance criteria move from incomplete to complete
update the implementation status whenever the local/full-spec boundary changes
update the Tarpaulin report after every coverage-changing implementation pass
leave unrelated experiment docs, papers, and probes intact
```

The agent should execute this loop until the contract is satisfied:

```text
1. read this spec and the linked status/coverage docs
2. run and record the Gate 0 CPU local-testnet command as the first executable acceptance gate before any
   other local, CUDA, public preflight, public evidence, or deployment-gated work can count
3. compare code and docs against Sections 32, 33, and 35
4. list missing local-reference items and missing deployment-gated items
5. implement one coherent missing local-reference slice
6. add or update focused tests for that slice
7. run cargo fmt --check --all
8. run cargo test --workspace --release
9. run cargo clippy --workspace --all-targets -- -D warnings
10. run every relevant experiment or study command for the slice, including randomness, Freivalds,
   row-sampling, data-withholding, collusion, TensorWork concentration, zero-work liveness,
   local-testnet, public-evidence, networking, persistence, and telemetry experiments when touched
11. run cargo tarpaulin when line coverage or test count changes
12. update docs/tensorvm/coverage_matrix.md, docs/tensorvm/implementation_status.md, and
   docs/tensorvm/tarpaulin_report.md
13. commit the completed iteration with a message that names the implemented slice
14. push the completed iteration to the configured upstream branch
15. repeat until no local-reference gaps remain
16. only then attempt deployment-gated items if infrastructure, credentials, and public endpoints are
    available
```

An iteration is not complete until the agent has recorded:

```text
the exact tests and experiments that apply to the changed slice
the exact commands executed from the workspace root
whether each command passed, failed, or was blocked
where the resulting evidence is documented
the commit hash for the completed iteration
the upstream remote and branch that received the pushed iteration
whether the push passed, failed, or was blocked
```

If any required test or experiment cannot be run, the agent must mark the iteration as blocked or
incomplete, record the command and failure reason, and avoid claiming that the iteration is complete.
If commit or push cannot be completed because no upstream exists, credentials are unavailable, network
access is blocked, or policy forbids publishing the current changes, the agent must record the exact
blocker and avoid claiming that the iteration is complete.

Local reference completion requires:

```text
all modules listed in the recommended internal module structure exist or have documented replacements
Gate 0 CPU local multi-participant testnet is the first satisfied local-reference gate and passes with
cargo test -p tensor_vm local_testnet --release, using canonical CPU semantics, mandatory libp2p node
paths, separate participant identities/endpoints, and no simulations or local-only networking shims
all local behavior needed by Acceptance Criteria 1-12, 14, and 15 has passing tests
Acceptance Criterion 13 has an evidence validator and local preflight harness, even if the public run is
not yet complete
crates/tensor_vm has no dependency on crates/experiments
cargo tree -p tensor_vm shows tensor_vm without experiments beneath it; external runtime dependencies such
as rust-libp2p are allowed when they are part of the TensorVM implementation
cargo fmt --check --all passes
cargo test --workspace --release passes
cargo clippy --workspace --all-targets -- -D warnings passes
cargo tarpaulin passes
crates/tensor_vm/src has 100% line coverage or every uncovered TensorVM line is justified in
docs/tensorvm/tarpaulin_report.md
```

Full-spec completion requires all local reference completion items plus externally verifiable deployment
evidence:

```text
real CUDA/C++ kernels exist for any claimed GPU mining path and are checked against canonical CPU outputs
production libp2p runtime is used for node discovery, gossip, and request/response propagation
production libp2p evidence includes one signed network-observation record for every counted public
operator, covering discovery, gossip, request/response, and DoS controls, and those records aggregate to
the signed network-runtime root
RPC, explorer, faucet, and telemetry services are deployed outside the local test harness
public service evidence includes external HTTPS URLs, signed health-check summaries, and signed
service-content roots for those services, with distinct endpoint IDs and distinct content roots across
RPC, explorer, faucet, and telemetry
a public testnet runs for 7 consecutive days with independent external miner and validator operators
the evidence bundle includes a signed wall-clock run window proving the 7-day duration, not only expected
block-count evidence
the evidence bundle includes post-run signed independent auditor records, signed node heartbeats,
block/finality history, operator identity attestations that exactly match the counted public operators,
data-availability measurements, invalid-work rejection evidence, reward-settlement records, and deployed
detection-measurement records
the evidence bundle includes signed external artifact locators for every raw supporting-record summary root
the evidence bundle is stored or linked from docs/tensorvm/implementation_status.md
```

The agent must not report "fully complete" if any of these are missing:

```text
GPU miner execution lacks real CUDA/C++ kernels for any claimed GPU path
independently checkable public-run evidence does not prove that the mandatory rust-libp2p runtime carried
node discovery, gossip, request/response, and DoS-controlled network operation
independently checkable public-run evidence lacks signed external artifact locators for raw supporting
records behind summary roots
independently checkable public-run evidence lacks signed service-content roots for deployed public
services
browser-facing services are still local-only handlers or static HTML responses
the 7-day public testnet evidence has not actually happened
the verification commands were not executed from the workspace root after the final change
```

---

## 33. Development Milestones

### Milestone -1: Threat Model and Parameter Study

Deliverables:

```text
miner/validator/proposer threat model
row-sampling detection probability calculator
Freivalds false-accept test harness
validator randomness grindability analysis
data withholding availability study
TensorWork concentration assessment
liveness fallback harness for zero-work epochs
```

Success criteria:

```text
documented false-accept targets per job shape
no row-sampled-only block eligibility unless target bounds are met
validation randomness cannot be biased by the current proposer
useful-verification PoW is implemented over canonical settled-receipt sets
genesis and zero-receipt epochs still produce blocks via stake-weighted PoW-skip fallback
```

---

### Milestone 0: Local Reference Execution

Deliverables:

```text
job generator
CPU TensorVM
miner reference harness
validator reference harness
Freivalds checker
reward accounting harness
```

Success criteria:

```text
honest miners agree on roots
invalid matmul outputs are detected
reward distribution works
```

---

### Milestone 1: TensorVM Reference Runtime

Deliverables:

```text
TensorDescriptor
Merkle tensor commitments
finite-field matmul
fixed-point linear training step
canonical program hashing
CPU execution backend
```

Success criteria:

```text
same input produces same output root across machines
```

---

### Milestone 2: Freivalds Validator

Deliverables:

```text
full Freivalds check
row-sampled Freivalds check
Merkle row openings
validator attestation format
soundness calculator for full and row-sampled checks
```

Success criteria:

```text
validators detect corrupted outputs with expected probability
sparse row corruptions are tested separately from dense corruptions
block-eligible receipts require full-output Freivalds or documented equivalent soundness
```

---

### Milestone 3: LinearTrainingStep Verifier

Deliverables:

```text
forward check: Y = XW
backward check: grad_W = X^T dY
random-linear error check: dY = Y - T
random-linear optimizer check: W_next = W - lr * grad_W
sampled loss check
```

Success criteria:

```text
invalid forward/backward/update receipts are rejected
sparse corruptions in dY and W_next are rejected with stated probability
```

---

### Milestone 4: Minimal Chain

Deliverables:

```text
accounts
miner registry
validator registry
job registry
receipt registry
attestation registry
reward state
settled-receipt pool
deterministic blockspace selector
useful-verification PoW block production
```

Success criteria:

```text
local testnet produces blocks from deterministic settled-receipt blockspace
validators can recompute checks_root for the block receipt set
TensorWork affects miner rewards and blockspace caps, not proposer selection
```

---

### Milestone 5: Public Testnet v0

Deliverables:

```text
multi-node network
production libp2p runtime
miner CLI
validator CLI
explorer
faucet
telemetry dashboard
public docs
deployed public services for RPC, explorer, faucet, and telemetry
signed service-health evidence bound to external HTTPS public service URLs
signed service-content evidence bound to external HTTPS public service URLs and content roots
signed production libp2p network-observation evidence
real CUDA/C++ miner kernels where GPU acceleration is claimed
external public-testnet evidence bundle
signed public run-window evidence for the 7-day duration
signed external raw supporting-record artifact locators
```

Success criteria:

```text
10+ miners
5+ validators
7 days continuous block production
signed wall-clock run window covering the full 7-day duration
independent external operator evidence
invalid work rejected
rewards paid by verified TensorWork and useful-verification PoW proposer success
validators produce blocks by useful-verification PoW over canonical settled receipts
production libp2p networking used for node propagation
production libp2p operation is evidenced by signed network-observation records
deployed services remain reachable during the public run
deployed service reachability is evidenced by signed health checks bound to external HTTPS service URLs
deployed service content is evidenced by signed content roots bound to external HTTPS service URLs, with
distinct endpoint IDs and distinct content roots across RPC, explorer, faucet, and telemetry
raw supporting-record artifacts are externally linked and signed against their summary roots
GPU kernel outputs match canonical deterministic CPU semantics
```

---

## 34. Success Metrics

Technical:

```text
block finality rate
average block time
receipt inclusion latency
verification latency
data availability rate
invalid receipt detection rate
state growth per epoch
bandwidth per validator
```

Compute:

```text
valid TensorWork Units per epoch
GPU utilization
verification cost / execution cost ratio
redundant compute overhead
```

Security:

```text
invalid receipts submitted
invalid receipts accepted
validator disagreement rate
data withholding incidents
collusion risk assessment results
```

Economic:

```text
miner reward per TWU
validator reward per attestation
reward concentration
hardware-class participation
cost to attack one epoch
```

---

## 35. Acceptance Criteria

The MVP succeeds if:

```text
1. Miners execute deterministic tensor workload requests and submit receipts.
2. Validators verify block-eligible matmul receipts with full-output Freivalds or an explicitly bounded equivalent.
3. Row-sampled checks are treated as audits unless their false-accept bounds are documented.
4. Blocks are produced by validators winning useful-verification PoW over deterministically ordered settled
   receipts.
5. Rewards are distributed according to verified settled TensorWork.
6. Validation randomness is unbiasable after receipt roots are committed.
7. Invalid tensor outputs are rejected in controlled dense and sparse corruption tests.
8. LinearTrainingStep receipts validate forward/backward/error/update structure.
9. Sparse corruptions in dY and W_next are rejected with stated probability.
10. Honest miners produce identical output roots.
11. Validators spend materially less compute than full recomputation.
12. Tensor data availability exceeds 95% during active windows and required retention windows.
13. The network runs for 7 consecutive days with independent nodes.
14. Zero-receipt epochs have a tested stake-weighted PoW-skip fallback path.
15. Reward concentration, validator disagreement, and data withholding are reported.
```

Full-spec completion additionally requires deployment evidence, not only a local deterministic reference
implementation:

```text
Gate 0 CPU local multi-participant testnet passes before any deployment evidence is counted, using
canonical CPU semantics, mandatory libp2p node paths, separate participant identities/endpoints, and no
simulations or local-only networking shims
real CUDA/C++ kernels exist where GPU mining is claimed
production libp2p runtime is used for network propagation
production libp2p operation is supported by signed per-operator network-observation records exactly
matching every counted public miner and validator operator and aggregating to the signed network-runtime
root
RPC, explorer, faucet, and telemetry services are deployed outside the local test harness
public service evidence includes externally reachable HTTPS URLs, signed health-check summaries, and
signed service-content roots with distinct endpoint IDs and distinct content roots across RPC, explorer,
faucet, and telemetry
the public testnet runs for 7 consecutive days with independent external operators
evidence for the 7-day run is published and independently checkable
the evidence bundle includes exactly one manifest publication signature in the current manifest format
HTTPS public evidence URIs include concrete query-free paths
the evidence includes a signed wall-clock run window; expected block count alone is not sufficient
the evidence includes signed external artifact locators for the raw supporting records behind summary roots
run-derived block/finality/randomness-beacon/data-availability/invalid-work/detection summary counts match the signed run counters exactly
the required Cargo workspace structure is present
the required verification commands have been executed and their results are documented
```

Do not count the full spec as complete if these remain true:

```text
GPU mining is claimed without real CUDA/C++ kernels
public-run evidence does not prove that the mandatory rust-libp2p runtime carried network propagation
public-run evidence summarizes supporting records without signed external raw-record artifact locators
public-run evidence lacks signed service-content roots for deployed public service URLs
browser/RPC surfaces exist only as local handlers or local HTML pages
durable state is only a reference file store rather than a production database/service deployment
7-day public-testnet evidence is inferred from a local harness rather than an external run
```

Do not count the MVP as successful if:

```text
current-epoch receipts affect proposer eligibility
validators can choose arbitrary receipts instead of the canonical blockspace set
checks_root cannot be recomputed by other validators
row-sampled checks are the only validity check without a parameterized soundness bound
training-step state updates are only sampled entry-by-entry
validation randomness can be influenced after miners commit receipt roots
data disappears before reward settlement or challenge windows close
```

---

## 36. Pros of This MVP Design

### 36.1 It Tests the Core Primitive

The MVP demonstrates probabilistically verified tensor work as a block-production commodity.

### 36.2 It Avoids PyTorch Consensus Complexity

PyTorch can become a frontend later, but the MVP remains deterministic and auditable.

### 36.3 Freivalds Gives Cheap Verification

Matrix-heavy tensor work can be verified much more cheaply than full recomputation when validators perform
full-output Freivalds checks or a row-sampling scheme with explicit soundness bounds.

### 36.4 LinearTrainingStep Creates a Bridge to Real Training

The MVP is not limited to synthetic random matmul. It includes the smallest complete learning step.

### 36.5 It Builds Toward Templar-Like Training

The path is clear:

```text
TensorOp
→ LinearTrainingStep
→ MLPTrainingStep
→ TransformerBlockTrainingStep
→ TrainingWindowReceipt
→ decentralized model training
```

---

## 37. Cons / Risks

### 37.1 Verification Is Probabilistic

Freivalds and row sampling are probabilistic. They reduce cheating probability but do not eliminate it absolutely.

Mitigation:

```text
multiple validators
multiple rounds
redundant agreement
random audits
future fraud proofs/ZK
```

Critical caveat:

```text
row sampling primarily detects corrupted sampled rows
it is weak against sparse row corruptions unless sample counts are large
full-output Freivalds should remain the block-eligibility path in v0
```

### 37.2 Data Availability Can Become a Bottleneck

Validators need tensor rows/chunks.

Mitigation:

```text
small initial shapes
chunked commitments
strict deadlines
reputation penalties
future DA layer
```

### 37.3 Training-Step Verification Is More Complex Than Matmul

Even linear training introduces more moving parts.

Mitigation:

```text
start with LinearTrainingStep only
avoid Transformer backward initially
make loss auxiliary
use SGD not Adam
use random-linear checks for elementwise relations
```

### 37.4 Hardware Centralization

Large jobs favor high-end GPUs.

Mitigation later:

```text
small/medium/large lanes
consumer GPU lane
bandwidth-heavy lane
sparse workload lane
```

### 37.5 Incentives Can Be Gamed

Miners may exploit scoring, withhold data, or attempt partial invalid outputs.

Mitigation:

```text
commit before validation randomness
validator sampling
redundancy
attestation audits
settlement delay
future slashing
```

### 37.6 Synthetic Usefulness Gap

Synthetic random matmul jobs are useful for benchmarking and security testing, but they are not external user
work.

Mitigation:

```text
describe v0 as verifiable tensor work, not full useful work
add user-submitted jobs only after deterministic execution and verification are stable
track how much TensorWork comes from synthetic versus user-valued jobs
```

### 37.7 TensorWork Consensus Circularity

If current receipts influence proposer eligibility, block producers can bias inclusion and timing.

Mitigation:

```text
do not use TensorWork for proposer eligibility
use useful-verification PoW over deterministic settled-receipt blockspace
settle rewards after validation and challenge windows
make validator stake the immediate finality security source in v0
```

### 37.8 No-Slashing v0 Is Not Economically Secure

Without hard slashing, invalid behavior is punished by non-payment and reputation only.

Mitigation:

```text
cap rewards
publish v0 as a research testnet
add slashing only after verifier correctness and appeal flows are tested
```

### 37.9 Edge Cases For The PoW + Blockspace Model

Verification grinding: validators may try different receipt subsets until they find an easier nonce.
Mitigation: the receipt set is canonical, sorted by `H(finalized_beacon || parent_hash || receipt_id)`, and
truncated only by blockspace caps. A block with a non-canonical set is invalid.

Fake verification: a validator may skip Freivalds or random-linear checks and search nonces over a bogus
`checks_root`. Mitigation: `checks_root` is reproducible from selected receipts, the parent, and the finalized
beacon. Other validators can recompute it and challenge during `verification_challenge_window`.

Empty blocks: if no settled receipts are available, the chain still needs liveness. Mitigation: empty
blockspace is valid, proposer reward is reduced, and no miner TensorWork rewards are paid.

Stalling: a validator may find a nonce and withhold the block. Mitigation: after `pow_timeout_blocks`,
stake-weighted fallback can produce a PoW-skip block; future versions can also throttle validators that
repeatedly win and withhold.

Censorship: validators may try to exclude a miner's receipts. Mitigation: deterministic selection makes
omission from the canonical set invalid rather than discretionary.

Validator/miner self-dealing: the same operator may run both roles and favor their own receipts. Mitigation:
receipts still need independent attestation quorum, and v1 should require operator-group diversity for
attestations that support proposer rewards.

Difficulty death spiral: low verification throughput can reduce difficulty until one validator dominates.
Mitigation: bounded retargeting, floor and ceiling targets, and a per-validator block cap per epoch.

No fee market: v0 deterministic inclusion has no congestion pricing. Mitigation: carry over unspent settled
receipts and treat v0 blockspace as a fairness and security harness, not a fee-market design.

Dimension gaming: attackers may use one huge receipt or many tiny receipts to distort block construction.
Mitigation: cap TWU, bytes, and receipt count independently.

Liveness fallback: zero-receipt or no-PoW periods must not halt the chain. Mitigation: stake-weighted
PoW-skip fallback block with reduced rewards and explicit telemetry.

### 37.10 Critique Of v2

The v2 design replaces a single measurable economic primitive, TensorWork-weighted proposer eligibility,
with two coupled primitives: validator verification cost and nonce search. That is cleaner for block
semantics but harder to model economically.

Validators now bear proof-of-work cost in addition to verification cost. The 15% proposer reward is a
placeholder; it must be tuned against observed verification cost, nonce-search cost, and validator
participation.

The verification challenge window means block finality and reward finality are different. BFT can finalize a
block immediately, but proposer reward and affected receipt settlement remain challengeable until the window
closes.

Useful-verification PoW only remains useful if verification cost dominates or at least materially gates the
nonce race. If nonce search dominates, validators can skip verification and brute force headers. The MVP
therefore targets expected nonce-search time per validator less than or equal to expected verification time
for the canonical receipt set.

The design may centralize around validators with both high stake and high verification throughput.
Mitigations include stake caps for finality weighting, bounded difficulty retargeting, per-validator block
caps per epoch, and public telemetry for validator reward concentration.

---

## 38. Recommended Future Versions

### v1: Fraud Proofs

Add interactive trace bisection.

```text
receipt commits trace_root
challenger disputes
protocol bisects trace
single invalid op proven
slash dishonest party
```

### v2: MLPTrainingStep

Add nonlinear activation and two-layer training.

### v3: TransformerBlockTrainingStep

Add attention, MLP, residual, norm, and backward checks.

### v4: TrainingWindowReceipt

Validate windows of local training rather than individual steps.

### v5: Real Model Training Protocol

Support distributed miners contributing verified gradients or updates to a shared model.

### v6: ZK / Proof-Carrying ML

Use ZK proofs for small inference, private inference, high-value settlement, and compact checkpoint proofs.

---

## 39. Final Recommendation

The final MVP should be:

```text
A Tensor Proof-of-Work research testnet with:
  - deterministic TensorVM
  - finite-field/int tensor arithmetic
  - Merkle-committed tensors
  - synthetic matmul TensorOp workload requests
  - full-output Freivalds validator checks for block-eligible receipts
  - row/chunk audits and verification-time availability checks
  - redundant miner agreement
  - deterministic settled-receipt blockspace
  - validator useful-verification PoW over checks_root
  - stake-weighted validator finality
  - reward distribution by settled valid TensorWork and PoW proposer success
  - LinearTrainingStep primitive with forward/backward/random-linear update validation
```

The MVP should test two claims:

```text
1. Validator verification of tensor computation can be used as a useful proof-of-work resource in a testnet.
2. Simple learning steps can be probabilistically verified as deterministic state transitions.
```

Do not begin with full LLM training.

Begin with:

```text
C = A @ B
```

Then immediately add:

```text
Y = XW
loss = MSE(Y, T)
grad_W = X^T(Y - T)
W_next = W - lr * grad_W
```

This is the cleanest path from tensor computation to verifiable decentralized training.

The strategic message is:

```text
Bitcoin proved energy was spent.
TensorVM proves verification of tensor work was performed.
```

That is the MVP worth building.

---

## Appendix A: v1 Assumptions Superseded By v2

This appendix preserves the prior v1 consensus assumptions that were replaced by the useful-verification
PoW and deterministic blockspace design. They remain here for auditability and migration planning, not as
active requirements.

### A.1 Superseded Section 5.3: Block Proposer

> Superseded by v2. In v2, the proposer is the validator that wins useful-verification PoW.

```text
### 5.3 Block Proposer

A proposer assembles blocks from valid tensor receipts.

Responsibilities:

collect receipts
collect validator attestations
compute reward updates
propose block
include settled TensorWork score

In the MVP, proposers are selected from miners according to settled prior-epoch TensorWork score.
```

### A.2 Superseded Section 20: Consensus Model

> Superseded by v2. In v2, TensorWork does not select block proposers. Validators produce blocks by useful-verification PoW.

```text
### 20.1 Hybrid Consensus

The MVP uses:

settled TensorWork-weighted proposer selection
+
validator finality

Miners earn future proposer eligibility by producing valid tensor work.

Validators finalize blocks through stake-weighted voting. Validator stake is the immediate consensus security
source in v0; TensorWork is a proposer-eligibility and reward signal until slashing and stronger fraud proofs
exist.

The chain must also have a liveness fallback. Genesis and zero-work epochs cannot rely on prior TensorWork.

Fallback rule:

if total_valid_tensor_work(epoch E) == 0:
  proposer selection for epoch E+1 falls back to stake-weighted validator/proposer rotation
  no miner TensorWork rewards are paid for epoch E
  job generation continues with smaller fallback jobs

This keeps the chain live while still making TensorWork the normal proposer-eligibility path.

### 20.2 Epochs

Suggested parameters:

block_time: 6 seconds
epoch_length: 100 blocks
approx_epoch_duration: 10 minutes

Each epoch has:

challenge generation
receipt submission
verification
next-epoch proposer selection
reward settlement

### 20.3 TensorWork Score

Each valid receipt contributes TensorWork Units.

score_miner(epoch E) = sum(settled_valid_receipt.tensor_work_units from epoch E)

Block proposer probability:

P(miner for epoch E+1) = score_miner(epoch E) / total_valid_tensor_work(epoch E)

Use weighted randomness rather than deterministic top-miner selection to reduce monopolization.

Do not let receipts from the current block or current epoch affect the current proposer selection. That would
let proposers bias inclusion, validation timing, and their own future eligibility within the same decision
cycle.

### 20.4 Finality

A block is finalized if:

>= 2/3 validator stake signs the block

Validators check:

receipt validity
attestation quorum
reward calculation
state transition
parent validity
```

### A.3 Superseded Section 21: Block Structure

> Superseded by v2. In v2, `job_root` is removed and blocks commit to `settled_receipt_set_root`, `checks_root`, `difficulty_target`, and `nonce`.

```rust
struct TensorBlock {
    height: u64,
    parent_hash: Hash,
    epoch: u64,
    proposer: Address,
    job_root: Hash,
    receipt_root: Hash,
    attestation_root: Hash,
    state_root: Hash,
    reward_root: Hash,
    randomness: Hash,
    timestamp: u64,
    proposer_signature: Signature,
    validator_signature_aggregate: Signature,
}
```

### A.4 Superseded Section 25.5: Proposer Rewards

> Superseded by v2. In v2, the proposer reward is paid to the winning validator after the verification challenge window.

```text
### 25.5 Proposer Rewards

proposer_reward = fixed_block_reward + fee_share
```

### A.5 Superseded Acceptance Criteria

> Superseded by v2. In v2, block production is useful-verification PoW over deterministic settled-receipt blockspace.

```text
4. Blocks are produced using settled prior-epoch TensorWork.
14. Genesis and zero-work epochs have a tested fallback proposer path.
```
