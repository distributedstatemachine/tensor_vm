# TensorVM Tarpaulin Report

Latest attempted run: June 20, 2026 from the workspace root during Iteration 89 with:

```bash
cargo tarpaulin --workspace --offline
```

Result:

```text
error: no such command: `tarpaulin`

help: view all installed commands with `cargo --list`
help: find a package to install `tarpaulin` with `cargo search cargo-tarpaulin`
```

This environment does not currently have `cargo-tarpaulin` installed, so Iteration 89 coverage could not
be regenerated. Iterations 78 through 89 rechecked the same command and hit the same missing-binary blocker.
The most recent completed coverage report below remains the prior May 23, 2026 run.

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
require applied diagnostic challenges plus future-maturity pending challenge reward claims. Focused
app/network, node ingest, and compose contract tests cover the emitted bounded payload, noncanonical
observed-block application, and delayed reward evidence.
Iteration 79 added durable block-parent `ChainState` snapshots for replay-stable historical
`BlockApplyOutcome` evidence. Focused chain and storage tests cover old useful-block apply outcomes after
future receipts/blocks and after chain-state save/load.
Iteration 80 added chain-level PoW-skip fallback timeout validation. Focused fallback and retarget tests
cover producer and non-producer rejection of early empty fallback blocks while preserving useful UVPoW.
Iteration 81 added inclusion-gated receipt reward release. Focused reward and settlement tests cover
height-mature settled receipt claims staying pending before blockspace inclusion, inclusion extending
claim maturity from the block height, and spendable release only after the inclusion-based maturity height.
Iteration 82 added chain-owned delayed receipt reward release evidence. Focused reward tests cover producer
and peer block transitions releasing included matured receipt rewards through canonical child-state
application, without a manual adapter-side release command.
Iteration 83 added receipt-bound validation challenge seed evidence. Focused randomness/proposer and
storage tests cover persisted validation seed commitments, stable challenge-vector seeds after later beacon
advancement, and attestation rejection when stored receipts are missing their randomness anchor.
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

The optional CUDA kernel feature is verified separately because the standard Tarpaulin configuration keeps
the portable default feature set:

```text
cargo test -p tensor_vm --features cuda-kernels --release
182 tensor_vm tests passed, including native CUDA field-matmul and linear-step tensor-op checks against
canonical CPU output
```

Tarpaulin reports line coverage here. Its branch coverage flag is currently listed as not implemented by the installed tool.
