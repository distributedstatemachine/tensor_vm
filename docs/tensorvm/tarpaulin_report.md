# TensorVM Tarpaulin Report

Latest attempted run: June 20, 2026 from the workspace root during Iteration 72 with:

```bash
cargo tarpaulin --workspace --offline
```

Result:

```text
error: no such command: `tarpaulin`

help: view all installed commands with `cargo --list`
help: find a package to install `tarpaulin` with `cargo search cargo-tarpaulin`
```

This environment does not currently have `cargo-tarpaulin` installed, so Iteration 72 coverage could not
be regenerated. The most recent completed coverage report below remains the prior May 23, 2026 run.

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

The optional CUDA kernel feature is verified separately because the standard Tarpaulin configuration keeps
the portable default feature set:

```text
cargo test -p tensor_vm --features cuda-kernels --release
182 tensor_vm tests passed, including native CUDA field-matmul and linear-step tensor-op checks against
canonical CPU output
```

Tarpaulin reports line coverage here. Its branch coverage flag is currently listed as not implemented by the installed tool.
