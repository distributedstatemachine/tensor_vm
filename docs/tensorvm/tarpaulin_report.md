# TensorVM Tarpaulin Report

Latest completed run: June 24, 2026 from the workspace root during Iteration 244 with:

```bash
cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin
```

Result:

```text
588 tests passed under instrumentation:
- 14 experiments library tests
- 573 tensor_vm library tests
- 1 tensor_vm_explorer library test

84.95% workspace line coverage
23831/28053 workspace lines covered
```

The report was written under `target/tarpaulin/`. `cargo-tarpaulin` is installed in this environment as
`/home/ubuntu/.cargo/bin/cargo-tarpaulin` version `0.35.5`.

Iteration 244 coverage-changing work added CUDA-feature-only scale-0 field `split(dim,sizes)` multi-output
graph execution support and conformance coverage for the supported CUDA subset. The default Tarpaulin run
passed under the portable feature set and does not instrument the native CUDA path, so the new CUDA
structural-partition evidence is recorded through separate `--features cuda-kernels` runtime and miner-role
tests in the execution plan. The portable coverage numbers are 84.95%, 23831/28053 lines, with 588
instrumented tests.

Iteration 243 coverage-changing work added CUDA-feature-only scale-0 field `concat`/`stack` graph
execution support and conformance coverage for the supported CUDA subset. The default Tarpaulin run
passed under the portable feature set and does not instrument the native CUDA path, so the new CUDA
structural-join evidence is recorded through separate `--features cuda-kernels` runtime and miner-role
tests in the execution plan. The portable coverage numbers are 84.95%, 23831/28052 lines, with 588
instrumented tests.

Iteration 242 coverage-changing work added CUDA-feature-only scale-0 field `tril`/`triu` graph execution
support and conformance coverage for the supported CUDA subset. The default Tarpaulin run passed under
the portable feature set and does not instrument the native CUDA path, so the new CUDA triangular evidence
is recorded through separate `--features cuda-kernels` runtime and miner-role tests in the execution
plan. The portable coverage numbers are 84.96%, 23831/28050 lines, with 588 instrumented tests.

Iteration 241 coverage-changing work added CUDA-feature-only scale-0 field `slice` graph execution support
and conformance coverage for the supported CUDA subset. The default Tarpaulin run passed under the
portable feature set and does not instrument the native CUDA path, so the new CUDA `slice` evidence is
recorded through separate `--features cuda-kernels` runtime and miner-role tests in the execution plan.
The portable coverage numbers are 84.97%, 23831/28048 lines, with 588 instrumented tests.

Iteration 240 coverage-changing work added CUDA-feature-only scale-0 field `squeeze`/`unsqueeze` graph
execution support and conformance coverage for the supported CUDA subset. The default Tarpaulin run
passed under the portable feature set and does not instrument the native CUDA path, so the new CUDA
`squeeze`/`unsqueeze` evidence is recorded through separate `--features cuda-kernels` runtime and
miner-role tests in the execution plan. The portable coverage numbers are 84.97%, 23831/28047 lines,
with 588 instrumented tests.

Iteration 239 coverage-changing work added CUDA-feature-only scale-0 field `reshape` graph execution
support and conformance coverage for the supported CUDA subset. The default Tarpaulin run passed under
the portable feature set and does not instrument the native CUDA path, so the new CUDA `reshape`
evidence is recorded through separate `--features cuda-kernels` runtime and miner-role tests in the
execution plan. The portable coverage numbers are 84.97%, 23831/28045 lines, with 588 instrumented
tests.

Iteration 238 coverage-changing work added CUDA-feature-only scale-0 field `mean` graph execution support
and conformance coverage for the supported CUDA subset. The default Tarpaulin run passed under the
portable feature set and does not instrument the native CUDA path, so the new CUDA `mean` evidence is
recorded through separate `--features cuda-kernels` runtime and miner-role tests in the execution plan.
The portable coverage numbers are 84.98%, 23831/28044 lines, with 588 instrumented tests.

Iteration 237 coverage-changing work added CUDA-feature-only scale-0 field `broadcast` graph execution
support and conformance coverage for the supported CUDA subset. The default Tarpaulin run passed under
the portable feature set and does not instrument the native CUDA path, so the new CUDA `broadcast`
evidence is recorded through separate `--features cuda-kernels` runtime and miner-role tests in the
execution plan. The portable coverage numbers are 84.98%, 23831/28043 lines, with 588 instrumented
tests.

Iteration 236 coverage-changing work added CUDA-feature-only deterministic scale-0 field `sum` graph
execution support and conformance coverage for the supported CUDA subset. The default Tarpaulin run
passed under the portable feature set and does not instrument the native CUDA path, so the new CUDA
`sum` evidence is recorded through separate `--features cuda-kernels` runtime and miner-role tests in the
execution plan. The portable coverage numbers remained 85.03%, 23831/28028 lines, with 588 instrumented
tests.

Iteration 235 coverage-changing work added CUDA-feature-only same-shape scalar-bounds field `clamp` graph
execution support and conformance coverage. The default Tarpaulin run passed under the portable feature
set and does not instrument the native CUDA path, so the new CUDA `clamp` evidence is recorded through
separate `--features cuda-kernels` runtime and miner-role tests in the execution plan. The portable
coverage numbers remained 85.03%, 23831/28028 lines, with 588 instrumented tests.

Iteration 234 coverage-changing work added CUDA-feature-only same-shape field `div` graph execution
support and conformance coverage with explicit CUDA zero-denominator error propagation. The default
Tarpaulin run passed under the portable feature set and does not instrument the native CUDA path, so the
new CUDA `div` evidence is recorded through separate `--features cuda-kernels` runtime and miner-role
tests in the execution plan.

Iteration 233 coverage-changing work added CUDA-feature-only same-shape field `where` graph execution
support and conformance coverage for `Int32` masks selecting between field tensors. The default
Tarpaulin run passed under the portable feature set and does not instrument the native CUDA path, so the
new CUDA `where` evidence is recorded through separate `--features cuda-kernels` runtime and miner-role
tests in the execution plan. The extra CUDA-gated Rust glue increased the portable denominator by one
line while remaining intentionally uncovered by the default-feature Tarpaulin run.

Iteration 232 coverage-changing work added CUDA-feature-only same-shape field comparison `eq`, `gt`, `lt`,
`ge`, and `le` graph execution support and conformance coverage. The default Tarpaulin run passed under
the portable feature set and does not instrument the native CUDA path, so the new CUDA comparison evidence
is recorded through separate `--features cuda-kernels` runtime and miner-role tests in the execution plan.

Iteration 231 coverage-changing work added CUDA-feature-only same-shape field `identity`, `neg`, `abs`,
and `sign` graph execution support and conformance coverage. The default Tarpaulin run passed under the
portable feature set and does not instrument the native CUDA path, so the new CUDA unary evidence is
recorded through separate `--features cuda-kernels` runtime and miner-role tests in the execution plan.

Iteration 230 coverage-changing work added CUDA-feature-only field `mul` graph execution support and
conformance coverage.

Iteration 222 coverage-changing work split public randomness/validator-VRF raw record shape from
chain-accepted deployed evidence for full-spec admission. The new regressions prove otherwise complete
full-spec-shaped bundles keep `public_evidence_full_spec=false` until the deployed evidence path is tied
to chain-accepted drand and validator-VRF reveal records, and zero beacon rounds are rejected by manifest,
record-line, and direct bundle validation.

Iteration 219 coverage-changing work tightened full-spec public raw-record evidence so otherwise signed
data-availability, invalid-work, reward-settlement, detection-measurement, and validator-VRF-lifecycle
records must use observed block indexes inside the signed run window before `public_evidence_full_spec=true`
can pass.

Iteration 220 coverage-changing work tightened full-spec public block/finality history evidence so signed
raw chain-history records must cover the exact signed observed block range instead of any distinct shifted
range with matching block roots and counts.

Iteration 218 coverage-changing work tightened full-spec public validator VRF lifecycle evidence so signed
and raw lifecycle records must include one `committed` and one `revealed` record with matching validator ID
and beacon round for each checked available receipt. Reveal-only lifecycle records no longer satisfy the
public full-spec gate.

Iteration 217 coverage-changing work tightened validator reward release for registered-key validators. A
legacy unkeyed reveal recorded before key registration no longer makes a validator receipt reward
spendable; key registration re-holds any already-claimable matching validator reward until a keyed
Ed25519 reveal matching the registered key is submitted.

Iteration 216 compacted full-spec public evidence bundle fixtures for coverage runs. Those fixtures still
evaluate default `PublicTestnetCriteria` and exercise the full raw-record shape, but later evidence
hardening keeps `public_evidence_full_spec=false` until chain-accepted deployed public randomness and
validator-VRF evidence exists. The fixture uses a test-only block time that keeps the seven-day signed run window while
reducing generated raw record count from 100,800 to 20 per full-spec-shaped bundle. This cleared the
Iteration 215 instrumentation stall and refreshed workspace coverage with the command above.

Iteration 215 coverage-changing work tightened public validator VRF lifecycle evidence so raw revealed
`validator_vrf_lifecycle=...` receipt roots must exactly match the raw data-availability measurement
receipt-root set before full-spec evidence can pass. The new regression proves internally valid,
re-signed lifecycle records over a different receipt set still leave `public_evidence_full_spec=false`.
Fresh workspace tarpaulin refresh was initially blocked by the pre-Iteration 216 full-spec fixture scale;
Iteration 216 resolved that test harness issue and produced the June 23, 2026 coverage summary above.

Iteration 196 coverage-changing work added a public deployed detection-measurement evidence gate for
full-spec evidence. The new regression proves otherwise complete public evidence cannot set
`public_evidence_full_spec=true` unless it has positive signed deployed detection-measurement records and
raw detection records that aggregate to the signed summary.

Iteration 192 coverage-changing work added a public-evidence randomness run-coverage gate. The new
regression proves signed public randomness-beacon summaries must cover the full observed run block count;
valid signatures and artifact locators no longer make undercounted or overcounted randomness summaries
independently checkable.

Iteration 193 coverage-changing work added a public-evidence CUDA graph-execution gate for full-spec
evidence. The new regression proves otherwise complete public evidence cannot set
`public_evidence_full_spec=true` unless `cuda_graph_execution_receipts` is positive and does not exceed
checked or available receipt counts, and manifest parsing/report output now exposes the CUDA graph receipt
count and boolean gate.

Iteration 194 coverage-changing work added a public-evidence validator VRF lifecycle gate for full-spec
evidence. The new regression proves otherwise complete public evidence cannot set
`public_evidence_full_spec=true` unless signed `validator_vrf_lifecycle_records` exactly cover the
checked receipt count, and manifest parsing/report output now exposes the lifecycle count and boolean
gate.

Iteration 197 coverage-changing work extended that gate from a scalar count to signed and raw
validator-VRF-lifecycle evidence. The new regression proves otherwise complete public evidence cannot set
`public_evidence_full_spec=true` unless raw revealed `validator_vrf_lifecycle=...` records aggregate to
the signed lifecycle summary root.

Iteration 191 coverage-changing work added a public-evidence CUDA miner gate for full-spec evidence. The
new regression proves otherwise complete public evidence cannot set `public_evidence_full_spec=true`
unless `cuda_verified_miner_count` covers the counted public miners, and manifest parsing/report output
now exposes the CUDA miner count and boolean gate.

Iteration 190 coverage-changing work added state-rooted released proposer reward block tracking for
late-finalized proposer rewards. The new reward regression proves late finality still creates a delayed
claim and later materialization cannot recreate an already claimed proposer reward.

Iteration 189 coverage-changing work added raw public chain-history evidence records for full-spec public
evidence: block-history and finality-history records must now aggregate to their signed summary roots
before full-spec evidence can pass.

Iteration 188 coverage-changing work added raw public operational evidence records for full-spec public
evidence: data-availability measurements, invalid-work rejections, and reward-settlement records must now
aggregate to their signed summary roots before full-spec evidence can pass.

Iteration 187 coverage-changing work added chain-owned verifier bandwidth evidence from live job and
receipt shapes, rendered through status and explorer overview.

Iteration 186 coverage-changing work added public randomness raw-record gate coverage for full-spec public
evidence, including manifest parsing for accepted public beacon records and malformed proof-kind/status
rejections.

Iteration 185 coverage-changing work added mixed dtype/scale conformance vector coverage for fixed-scale
comparison masks and int8 selection, while keeping CUDA evidence explicitly out of the default CPU proof.

Iteration 184 coverage-changing work added trace-bisection DoS admission coverage for oversized
worst-case bisection depth, duplicate pending expectation replay, and conflicting pending expectation
overwrite rejection.

Iteration 183 coverage-changing work added isolated trace-bisection timeout coverage for incomplete
challenger transcripts, proving that isolated sessions close by challenger forfeiture without voiding the
responder receipt path or issuing a challenger bounty.

Iteration 181 coverage-changing work extended explorer WebSocket regression coverage to first-class
`GraphExecution` jobs and receipts, raising `crates/tensor_vm/src/rpc/explorer.rs` coverage to 247/250
lines. The line-coverage percentage remains lower than the old May 23, 2026 report because the current
workspace includes substantially more runtime, deployment, public-evidence, and libp2p surface area in the
denominator.

Historical report:

Generated on May 23, 2026 from the workspace root with:

```bash
cargo tarpaulin --workspace --offline
```

The root [`tarpaulin.toml`](../../tarpaulin.toml) expands that to workspace library coverage,
LLVM instrumentation, stdout output, and a force-clean build.

Host notes:

- `cargo-tarpaulin` must be at least `0.35.4` for the current Rust toolchain.
- `--engine Llvm` is used by the root `tarpaulin.toml` for stable instrumentation on this host.
- The older `cargo-tarpaulin 0.30.0` failed to parse Rust `1.94.1` / LLVM `21.1.8` profile data with `consistency check for reading counts failed`.

Result:

```text
262 tests passed under instrumentation:
- 14 experiments library tests
- 247 tensor_vm library tests
- 1 tensor_vm_explorer library test

97.29% workspace line coverage
11559/11881 workspace lines covered

97.81% tensor_vm crate line coverage
10696/10936 tensor_vm lines covered
100.00% tensor_vm_explorer crate line coverage
277/277 tensor_vm_explorer lines covered
```

The remaining uncovered `tensor_vm` lines are concentrated in block-admission rejection branches, pending
block and block-vote payload retry edges, and p2p request/response unhappy paths. Focused node and p2p
tests cover the main block/block-vote payload happy paths, malformed payload rejection, invalid
signature/root rejection, duplicate admission behavior, and bounded wire-length rejection.
Iteration 75 added deterministic diagnostic block-check challenge generation in `chain::challenges`;
focused chain and node payload tests cover deriving an observed malformed block from a produced useful
block, applying the signed challenge through the shared command/payload path, and delaying the challenger
reward.
Iteration 76 added the bounded observed-invalid-block cache and `NewObservedBlockCheckChallengePayload`
wire/node path; focused p2p, chain, node payload, and pending-retry tests cover carrying the observed
malformed block without replacing the canonical block list and resolving the challenge through delayed
pending challenger rewards.
Iteration 77 added live validator-proposer diagnostic challenge emission and hardened the local checker to
require applied diagnostic challenge evidence. Focused app/network, node ingest, and compose contract tests
cover the emitted bounded payload, noncanonical observed-block application, and the invariant that observed
diagnostics do not punish the canonical proposer reward path.
Iteration 79 added durable block-parent `ChainState` snapshots for replay-stable historical
`BlockApplyOutcome` evidence. Focused chain and storage tests cover old useful-block apply outcomes after
future receipts/blocks and after chain-state save/load.
Iteration 80 added chain-level PoW-skip fallback timeout validation. Focused fallback and retarget tests
cover producer and non-producer rejection of early empty fallback blocks while preserving useful UVPoW.
Iteration 81 added inclusion-gated receipt reward claimability. Focused reward and settlement tests cover
height-mature settled receipt claims staying pending before blockspace inclusion, inclusion extending
claim maturity from the block height, and spendable credit only after the inclusion-based maturity height
through beneficiary claim.
Iteration 82 added chain-owned delayed receipt reward evidence. Focused reward tests cover producer and
peer block transitions preserving included matured receipt rewards as pending child-state claims until
beneficiary claim, without an adapter-side release workaround.
Iteration 83 added receipt-bound validation challenge seed evidence. Focused randomness/proposer and
storage tests cover persisted validation seed commitments, stable challenge-vector seeds after later beacon
advancement, and attestation rejection when stored receipts are missing their randomness anchor.
Iteration 90 added chain-owned randomness binding evidence for the local finalized-beacon construction.
Focused randomness, status, RPC, and explorer tests cover seed-domain labels, commit-reveal ordering,
receipt-anchor consistency counts, and the current-block-hash randomness ban.
Iteration 84 added governed validator-audit stake-slash reversal. Focused audit and storage tests cover
upheld appeals retaining the slash, reversed appeals refunding treasury back to validator stake, recording
the refunded amount in state-rooted appeal records, and preserving the reversal across chain-state
roundtrip.
Iteration 86 added implemented fraud-path economic calibration for validator-audit, miner
data-unavailability, and block-check/proposer clawback paths. Focused chain, status, and RPC tests cover
path-specific required slashable bonds, at-risk claim counts, aggregate worst-required-bond, and all-path
pass/fail evidence.
Iteration 87 added a proposer-specific reward hold and changed block-check economic calibration to count
held proposer claims as slashable escrow rather than immediate fraud proceeds. Focused reward, storage,
status, and RPC tests cover proposer claim timing, codec persistence, and rendered invariant evidence.
Iteration 88 added current-head competing useful-block admission and withholding policy. Focused chain and
node payload tests cover better same-parent useful UVPoW replacement, finalized-head rejection, fallback
head stability, and network payload application of a better competing head.
Iteration 89 changed receipt-path economic calibration to treat immature pending miner and validator
receipt rewards as slashable/voidable escrow rather than immediate fraud proceeds. Focused reward, status,
and RPC tests cover claim-maturity-sensitive fraud exposure.
Iteration 91 added an explicit fraud-window reward hold to `ChainParams::reward_maturity_delay_blocks`.
Focused params and reward tests cover challenge/audit hold selection and prove pending miner rewards do
not become spendable fraud proceeds before that hold expires.
Iteration 92 added chain-owned detection probability evidence. Focused chain, status, RPC, and explorer
tests cover per-mechanism detection bps, false-accept bps, sample sizes, source labels, and live subject
counts for Freivalds, row-sampling, random-linear, graph replay, data availability, validator-audit, and
block-check paths.
Iteration 93 added late invalid-output delayed reward voiding. Focused settlement tests cover assigned
`Invalid` attestations contesting already settled receipts, voiding pending miner and validator receipt
claims before maturity, and preventing mature claim from crediting spendable rewards.
Iteration 94 added side-branch fork-tree storage. Focused chain, node payload, and storage tests cover
known-parent side branch admission, side-branch grandchildren, canonical-state preservation, and
chain-state roundtrip of side-branch block plus child-state maps.
Iteration 96 added automatic unfinalized side-branch deep reorg. Focused chain and node payload tests cover
longer-branch promotion, canonical suffix side-branch preservation, finalized-block protection, and
non-producer payload convergence.
Iteration 95 added state-rooted invalid-output miner stake slashing. Focused attestation, economic
calibration, status/explorer, and storage tests cover one-shot miner slash records, treasury credit,
delayed reward voiding, fraud-path calibration, and chain-state persistence.
Iteration 97 added role-owned local graph execution production. Focused scheduler, localnet, graph-job,
role, and app validator-role tests cover synthetic graph job emission, graph work settlement, and miner plus
validator role submission from registered graph bodies and node-local tensors.
Iteration 98 added content-addressed Tensor IR `const_blob` execution. Focused IR, graph-job, and role
tests cover blob URI/root/shape/dtype checks, graph receipt replay with blob artifacts, role bundle
serving, validator replay, and missing-blob rejection.
Iteration 100 replaced the TensorWork reward-curve workaround with delayed TensorWork activation. Focused
settlement coverage checks that newly settled miner work remains pending until the matching non-voided
miner receipt reward is released, and that unavailable or invalid receipts clear pending work before it can
become settled.
Iteration 101 added typed block-check transcript openings. Focused block apply and block-check challenge
coverage checks that selected-receipt openings expose the beacon, parent hash, check seed, selected receipt
leaf, receipt checks root, and receipt metadata that hash into the Merkle-proven check leaf before a
challenge is admitted.
Iteration 102 anchored selected-receipt opening retention deadlines to receipt submission height. Focused
block apply coverage checks that delayed inclusion after fallback production does not extend the reported
tensor retention deadline.
Iteration 103 made settled receipt rewards explicitly await canonical blockspace inclusion before their
reward maturity clock starts. Focused settlement, reward, block, and audit tests cover the
awaiting-inclusion maturity state, inclusion-derived claimable heights, reward-root sensitivity, and continued
voiding/slashing behavior.
Iteration 104 adds local finalized-beacon randomness construction evidence. Focused randomness, status,
RPC, and explorer tests cover non-placeholder local finalized-beacon round mapping, local validator VRF
seed derivation labels, round-mapping counts, validator VRF seed counts, and the current-block-hash ban.
Iteration 105 adds state-rooted redundant settlement delay records. Focused settlement and storage tests
cover missing redundant agreement quorum, conflicting quorum-backed linear training transitions, record
clearing on settlement, state-root sensitivity, and chain-state roundtrip persistence.
Iteration 106 adds state-rooted external randomness beacon records. Focused chain, status, RPC, explorer,
and storage tests cover external beacon admission, stale/empty rejection, future receipt anchor binding,
status/explorer evidence fields, state-root sensitivity, and chain-state roundtrip persistence.
Iteration 107 adds explicit reward-delay height evidence to redundant settlement delay records. Focused
settlement and storage tests cover missing redundant agreement quorum, conflicting quorum-backed linear
training transitions, reward-delay height derivation from the chain maturity policy, state-root
sensitivity, and chain-state roundtrip persistence.
Iteration 108 threads that delay into the normal pending receipt reward ledger once a delayed receipt
eventually settles. Focused settlement tests cover delayed miner and validator receipt claims inheriting
the redundant reward hold before inclusion-based maturity can release them.
Iteration 109 adds same-scale `Fixed32` multiply rescale semantics. Focused tensor, IR, and conformance
tests cover signed raw-product multiplication, round-half-even rescale back to the declared tensor scale,
broadcasted exact graph replay, and a CPU conformance vector for the admitted `mul` op.
Iteration 110 adds mixed-scale `Fixed32` add/sub semantics. Focused tensor, IR, and conformance tests cover
RHS-to-lhs/output scale rescale with round-half-to-even semantics, exact graph replay, and CPU conformance
vectors for the admitted `add` and `sub` ops.
Iteration 111 adds mixed-scale `Fixed32` multiplication semantics. Focused tensor, IR, and conformance
tests cover signed raw-product scale derivation from `lhs_scale + rhs_scale`, half-even rescale back to
the lhs/output scale, exact graph replay, and CPU conformance vectors for the admitted `mul` op.
Iteration 114 adds tensor-owned packed int8 payload APIs. Focused tensor, IR, verifier, and conformance
tests cover the byte-exact `TVQ8` payload, malformed payload rejection, shared encode/decode ownership, and
unchanged CPU conformance profile behavior.
Iteration 115 removes the remaining direct spendable reward-credit test helper. Focused chain and
telemetry tests cover generic rewards entering pending credit claims, failing claim attempts before
maturity, mature beneficiary claim through the chain command, and only then spendable reward accounting.
Iteration 116 adds first-class packed int8 tensor artifact APIs. Focused tensor, IR, and conformance tests
cover construction of packed `TVQ8` payloads as `Uint8` tensors, descriptor/chunk/opening verification,
decode validation, exact replay, and unchanged conformance vectors.
Iteration 175 strengthens admitted-op conformance identity evidence. Focused conformance tests now require
unique vector IDs, registry-derived coverage for every consensus-admitted frozen op, and an explicit
auxiliary boundary for non-registry verifier vectors such as LinearTrainingStep `mse_loss`.
Iteration 176 strengthens direct delayed-reward pruning evidence. Focused reward and validator-audit tests
cover automatic pruning of voided miner receipt claims without spendable credit, preservation of live
matured receipt claims until beneficiary `ClaimReward`, and preservation of voided validator-audit claims
for the explicit appeal-aware release path.
Iteration 177 strengthens graph receipt dependency classification. Focused node payload tests cover
`GraphExecution` receipt payloads waiting for missing registered program bodies, graph-id mismatches
remaining invalid, and successful admission once the canonical graph body is registered.
Iteration 117 adds external graph artifact propagation evidence. Focused node and libp2p tests cover
valid graph job payloads staying pending while their graph body is missing, runtime retry after program
registration, and loopback request-response fetching of graph program plus input tensor artifacts before
the same external graph job payload applies.
Iteration 118 adds automatic external graph artifact fetching at the runtime role boundaries. Focused
runtime tests cover pending graph job payloads fetching missing program bodies before retry, miner roles
fetching graph input plus `const_blob` tensors before execution, and validator roles fetching graph input,
output, and `const_blob` tensors before attestation.
Iteration 119 replaces the receipt reward awaiting-inclusion height sentinel with an explicit
`ReceiptRewardMaturity` state committed by reward roots and chain-state storage. Focused reward,
settlement, attestation, and storage tests cover awaiting-inclusion claims, inclusion-derived delayed
heights, mature beneficiary claim, audit delay/voiding, and roundtrip persistence.
Iteration 123 makes redundant settlement quorum operator-distinct. Focused settlement and storage tests
cover same-operator miner-address agreement staying delayed, distinct-operator agreement settling, delay
records preserving both agreeing miner and operator counts, and chain-state roundtrip persistence.
Iteration 124 makes collusion-risk study evidence operator-aware. Focused study coverage checks that
colluding miner-address count reaching quorum is insufficient when those miners are controlled by too few
operators, while colluding operators at quorum can satisfy redundant agreement.
Iteration 125 exposes awaiting-inclusion receipt reward claims explicitly in chain pending-claim views,
service status, and explorer JSON instead of presenting them as a synthetic far-future claim height.
Focused chain, status, RPC, and explorer tests cover null/no concrete claim height before inclusion and
normal concrete delayed heights for already scheduled claims.
Iteration 126 extends that explicit delay state to `ReceiptRewardPending` settlement events, replacing the
remaining synthetic `u64::MAX` pending-event height with `claimable_at_height=None` plus
`awaiting_inclusion=true`. Focused command tests cover newly pending miner and validator receipt reward
events.
Iteration 127 adds the checked Codex 5.5 local-chain workflow document. Focused deployment-doc coverage
guards the Gate 0-first rule, context refresh list, local Docker gate, broad validation sequence,
tarpaulin/Docker blockers, and commit/push evidence flow.
Iteration 221 makes deployed public service health/content evidence part of bundle independent
checkability. Focused public evidence bundle and manifest tests cover missing service-health or
service-content records and re-signed service-content records whose endpoint/authority do not match the
corresponding service-health record.

The optional CUDA kernel feature is verified separately because the standard Tarpaulin configuration keeps
the portable default feature set:

```text
cargo test -p tensor_vm --features cuda-kernels --release
580 tensor_vm library tests and 54 tvmd runtime tests passed, including native CUDA field-matmul,
same-shape field graph, field unary, field comparison, field division, scalar-bounds field clamp, and linear-step tensor-op checks against canonical CPU output
```

Tarpaulin reports line coverage here. Its branch coverage flag is currently listed as not implemented by the installed tool.
