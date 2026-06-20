# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Active feature: Iteration 55, uniform proposer reward maturity delay, implemented and validated;
  commit/push pending.
- Current status: Iteration 52 admits exact deterministic `quantize_int8_per_channel`,
  `dequantize_int8_per_channel`, `quantize_pack_int8`, and `unpack_dequantize_int8`
  execution/conformance. Packed quantization uses a flat `uint8` payload with explicit `TVQ8`
  magic/version bytes, rank, axis, output scale, shape, per-channel signed 64-bit scales, and row-major
  int8 payload bytes. Iteration 52 feature commit `1b86f7f` and evidence commit `0387246` are pushed.
  Iteration 53 feature commit `72e16b8` and evidence commit `fae9faf` are pushed. Iteration 54 feature
  commit `f5dd68b` is pushed and adds mixed-dtype comparison and `where` coverage to the conformance suite
  and verifier profile-gating tests. Iteration 55 will make useful and fallback proposer rewards use the
  same full reward-settlement plus challenge-window maturity delay.
- Current blockers:
  - `docs/tensorvm/codex_5_5_local_chain_workflow.md` is referenced by `goal.md` but is missing from the
    worktree.
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Full Docker runtime verification remains unresolved from the prior recorded run: gateway `/health`
    timed out with `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Next action: commit/push Iteration 55, then select the next goal-aligned implementation slice.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing for current iteration | Iteration 54 first and final `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker requires positive live counters | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches missing tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Implemented in Rust runtime; Docker proof pending | `validator_proposer_tick_runs_without_synthetic_producer_gate`; useful proposal counters; delayed proposer rewards | Rerun full Docker checker after `/health`; add multi-validator proposer competition/fork-choice policy |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, block votes, validator audit reports, and block-check challenges | Continue extending only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, checks roots, beacon binding, fallback mode, delayed rewards, and network-visible block-check challenges | Remaining: full transcript disputes, exact replayable snapshots/apply theorem, deterministic live bad-block challenge generation |
| Tensor IR graph language | Partial; Iteration 52 byte-packed quantization implemented | `ir::TensorGraph`, canonical JSON, `graph_id`, registry validation, state/storage/runtime program serving, exact interpreter for current core plus Iteration 44 shaping/generator/comparison coverage, Iteration 45 `mean`/`cast`/`concat`/`stack` replay, Iteration 46 current TensorOp/LinearTrainingStep receipt trace roots from canonical graph execution, Iteration 47 graph-backed jobs/receipts, Iteration 48 exact unary Tier-B replay for `identity`, `neg`, `abs`, `sign`, `round`, and `relu`, Iteration 49 tensor scale metadata plus half-even fixed-point `cast`/`round` rescale, Iteration 50 `int8`/`uint8`/`bool` dtype tags plus gated quantization registry vocabulary, Iteration 51 admitted exact per-channel quantize/dequantize replay, and Iteration 52 admitted flat `uint8` packed quantize/dequantize replay | Continue toward remaining exact Tier-B verifier coverage and role-runtime arbitrary graph production |
| Per-op `F_p` conformance vectors | Partial; Iteration 54 mixed-dtype comparison/`where` vectors implemented | Deterministic vectors for current executable ops plus Iteration 44 field-only shaping/generator vectors, Iteration 45 `mean`/`concat`/`stack` vectors, Iteration 48 unary vectors for `identity`, `neg`, `abs`, `sign`, `round`, and `relu`, Iteration 49 fixed-point scale-aware `cast`/`round` vectors, Iteration 51 multi-output exact per-channel quantize plus dequantize vectors, Iteration 52 byte-exact pack/unpack vectors, and Iteration 54 comparison/selection vectors; CPU pass profile; default CUDA non-admission | Add remaining admitted-op vectors and CUDA conformance evidence |
| Randomness commit/reveal or VRF beacon | Partial | Admitted receipts persist receipt-time finalized beacon randomness/assignment seed | Remaining: full VRF/drand construction and external commit-reveal ordering |
| Economics and slashing invariant | Partial; Iteration 55 uniform proposer reward delay implemented | Delayed proposer, reduced delayed fallback proposer, receipt, challenge, and credit rewards; reward-root binding; block-transition mature release; useful and fallback proposer claims both use full reward maturity; data-unavailability and validator-audit slashing; no proposer reward useful-successor latch | Add auditor-selection policy, appeal paths, unified formal reward-claim objects, and broader invariant calibration |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 55: Uniform Proposer Reward Maturity Delay

Feature capability: all proposer rewards, including useful-verification blocks and zero-receipt fallback
blocks, enter the state-rooted pending proposer reward ledger with the same full reward-settlement plus
challenge-window maturity delay. This removes the remaining reward-timing workaround where useful proposer
claims could mature after only the challenge-window component.

Readiness requirements covered:
- `upow.md` §12: verifier-dependent rewards are delayed state claims and become spendable only after the
  reward-settlement delay plus challenge window.
- `upow.md` §11.4: block reward allocations are committed by reward roots before spendability.
- `local_chain_production_readiness.md`: proposer rewards should release after the explicit full
  reward-maturity height.

Canonical owner: `chain::blocks::BlockRewardContext` owns proposer reward claim creation and maturity
height; `chain::commands::release_matured_proposer_rewards` owns spendable release; `chain::roots` and
state storage own commitment/encoding of pending reward fields.
Adapter callers: validator role block production and tests call the shared chain transition; runtime/status
surfaces only observe pending/released rewards.
Old shortcut being removed: useful proposer rewards used only `challenge_window_blocks` while fallback
proposer rewards used the full reward-settlement plus challenge-window delay.
Regression test that proves the shortcut is gone: useful proposer rewards remain pending until the full
reward maturity height, not merely the challenge-window height; fallback reward delay tests continue to
prove the same rule for empty blockspace.
Behavior with local synthetic block production disabled: unchanged; validator proposer rewards still enter
the same pending ledger through role-owned block production.
Behavior for producer and non-producer roles: unchanged; both recompute the same child reward root and
release matured pending rewards through block application or explicit release commands.
Structured evidence source: reward tests, block tests that observe delayed release in block transitions,
coverage/status docs, and local-testnet Gate 0.
Finality source: unchanged stake-weighted block votes; reward finality remains delayed beyond block
finality.
Wire-size and codec boundary: no codec or storage schema change; only the existing
`claimable_at_height` value changes for useful proposer rewards.

Parallel subagents:
- Not used for this narrow consensus-timing slice; parent owns code, tests, and docs.

Implementation workstreams:
- Update proposer reward maturity calculation to always use the full reward-settlement plus
  challenge-window delay.
- Update reward/block tests and status docs that encode the shorter useful reward challenge-window path.

Narrow validation commands:
- `cargo test -p tensor_vm chain::tests::rewards`
- `cargo test -p tensor_vm chain::tests::blocks::block_transition_releases_matured_rewards_without_manual_command`
- `cargo test -p tensor_vm chain::tests::challenges::matured_proposer_reward_releases_after_full_maturity_delay`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: useful and fallback proposer rewards share one full maturity delay, reward
roots remain deterministic, voiding before maturity still prevents credit, and docs no longer describe
useful proposer rewards as challenge-window-only.

Out of scope: reward amount calibration, receipt/challenge/credit reward ledger unification, auditor
selection, appeals, public evidence, Docker `/health`, and storage schema changes.

Implementation summary:
- Removed the selected-receipt special case from `BlockRewardContext::proposer_claimable_at_height`; all
  block proposer rewards now use `reward_settlement_delay_epochs + challenge_window_epochs`.
- Updated `settle_epoch_rewards` proposer claim top-ups/direct inserts to use the same full maturity delay
  anchored to the reward claim block height.
- Updated reward, challenge, and role-runtime tests so useful proposer rewards remain pending until full
  maturity and validator wallets can receive proposer plus receipt rewards at the same release boundary.
- Updated status/coverage/readiness docs to describe useful and fallback proposer rewards as sharing the
  same full delayed maturity rule.

Validation evidence:
- First gate: `cargo test -p tensor_vm local_testnet --release` passed before implementation.
- Focused reward tests: `cargo test -p tensor_vm chain::tests::rewards` passed.
- Focused challenge test: `cargo test -p tensor_vm chain::tests::challenges::matured_proposer_reward_releases_after_full_maturity_delay` passed.
- Focused runtime regression: `cargo test -p tensor_vm --test tvmd_runtime runtime_roles::producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks` passed.
- Format/diff: `cargo fmt --check --all` and `git diff --check` passed.
- Broad debug: `cargo test -p tensor_vm` passed 379 library tests plus integration/doc checks.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Broad release: `cargo test --workspace --release` passed.
- Final gate: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is
  not installed (`error: no such command: tarpaulin`).

### Iteration 54: Mixed-DType Comparison And Where Conformance Vectors

Feature capability: the conformance suite covers exact Tier-B comparison and selection semantics with
mixed dtypes: canonical field-order comparison emits deterministic integer mask tensors, and `where`
consumes a mask plus fixed-point branches while preserving output dtype/scale. This moves the current
IR-only evidence for comparisons and `where` into the per-op `F_p` conformance gate.

Readiness requirements covered:
- `upow.md` §3.3 and §16: admitted exact ops need deterministic conformance vectors before runtimes can
  claim receipt eligibility.
- `upow.md` §4.7-§4.9: Tier-B comparison and selection ops are exact deterministic replay, not
  implementation-local behavior.
- `mvp_spec.md` §35: validators must reject otherwise-valid receipts when required op conformance is
  missing.

Canonical owner: `conformance` owns per-op vector execution and suite hashing; `ir::TensorGraph` owns exact
runtime semantics; `verify` consumes conformance profiles for receipt admission.
Adapter callers: CPU/CUDA runtime profile reporting and graph/TensorOp/LinearTrainingStep verifiers consume
the conformance profile but do not define vector semantics.
Old shortcut being removed: mixed-dtype comparison and `where` behavior was proven by IR tests only, leaving
the runtime conformance suite unable to name those admitted ops as pass-gated vectors.
Regression test that proves the shortcut is gone: conformance tests pass vectors for `gt`, `lt`, `ge`,
`le`, `eq`, and `where`, and the CPU profile's passed-op set includes those ops.
Behavior with local synthetic block production disabled: unchanged; this is deterministic runtime
admission metadata only.
Behavior for producer and non-producer roles: unchanged; all roles consume the same suite hash/profile.
Structured evidence source: conformance vector tests, CPU runtime profile tests, and graph verifier tests
for profile gating.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no p2p or storage codec changes; only the suite hash and vector set change.

Parallel subagents:
- Required read-only mapper/explorer/test-coverage agent launches were attempted and all failed with
  `agent thread limit reached`; parent performs the mapping and implementation directly.

Parallelizable implementation workstreams:
- Parent/integrator owns all edits because vector definitions, reference execution, suite hash, tests, and
  docs must agree.
- No parallel writers.

Tests/checkers/docs to add or update:
- Conformance vectors for field-order comparison results and `where` over fixed-point branches.
- Reference conformance executor support for comparison/selection ops.
- Status docs replacing mixed-dtype comparison/`where` as IR-only evidence with conformance-covered
  evidence.

Narrow validation commands:
- `cargo test -p tensor_vm conformance::tests -- --nocapture`
- `cargo test -p tensor_vm runtime::tests::cpu_backend_reports_passing_conformance_profile -- --nocapture`
- `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_comparison_where_receipt -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: the conformance suite includes comparison and `where` vectors, CPU reference
passes them, suite hashing commits them, and docs no longer list mixed-dtype comparison/`where` as
IR-only vector coverage.

Out of scope: CUDA runtime pass evidence, new IR op admission, `gather`/`scatter`/`embedding`
index-consistency proofs, VRF/drand, fork-choice, Docker `/health`, public evidence, and arbitrary runtime
role changes.

Split trigger: if `where` conformance requires broad graph verifier or vector schema changes, stop after
comparison vectors and leave `where` vector schema work for a separate iteration.

Implementation summary:
- Added conformance vectors for canonical field-order `gt`, `lt`, `ge`, `le`, boolean `eq`, and
  fixed-point `where` with broadcasted mask/branch shapes.
- Added reference conformance executor support for comparison and selection ops, including shared
  broadcasting helpers for vector replay.
- Added `verify::tests::graph_verifier_accepts_comparison_where_receipt`, proving graph receipts using
  comparison plus `where` verify under the CPU conformance profile and fail when `where` is removed from
  the passed-op set.
- Updated status/coverage docs so mixed-dtype comparison and `where` are no longer described as IR-only
  evidence.

Validation evidence:
- First gate: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused conformance: `cargo test -p tensor_vm conformance::tests` passed.
- Focused verifier: `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_comparison_where_receipt` passed.
- Focused runtime profile: `cargo test -p tensor_vm runtime::tests::cpu_backend_reports_passing_conformance_profile` passed.
- Format/diff: `cargo fmt --check --all` and `git diff --check` passed.
- Broad debug: `cargo test -p tensor_vm` passed 379 library tests plus integration/doc checks.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Broad release: `cargo test --workspace --release` passed.
- Final gate: `cargo test -p tensor_vm local_testnet --release` passed.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is
  not installed (`error: no such command: tarpaulin`).

### Iteration 53: Proposer Reward Delay Cleanup

Feature capability: proposer rewards use one explicit state-rooted delay rule: pending claims become
spendable only after `claimable_at_height`, unless voided by a challenge. The old useful-successor latch is
removed so fallback and useful proposer rewards share the same delayed-maturity path instead of carrying a
dead workaround field.

Readiness requirements covered:
- `goal.md` §12: rewards are delayed state claims and the economic invariant is easier to reason about when
  spendability depends on one maturity rule.
- `upow.md` §11.4 and §12: block reward allocations are committed through reward roots and released only
  after the settlement/challenge delay.
- `local_chain_production_readiness.md`: fallback proposer rewards release after the explicit full
  reward-settlement plus challenge-window delay without a later-useful-block unlock latch.

Canonical owner: `chain::state::PendingProposerReward` owns consensus-visible proposer reward claim state;
`chain::commands::release_matured_proposer_rewards` owns spendable release; `chain::blocks` owns block
transition claim creation; `chain::roots::pending_proposer_reward_root` owns commitment encoding.
Adapter callers: validator role reward sizing, runtime status, block status, and tests observe pending
claims but do not own release policy.
Old shortcut being removed: `requires_useful_successor` is a legacy latch that can keep a mature proposer
claim unavailable for reasons unrelated to the explicit reward delay. Current production paths already set
it to `false`; the field should not remain part of consensus state.
Regression test that proves the shortcut is gone: fallback proposer rewards release at maturity without a
useful successor, and no test or root encoding references `requires_useful_successor`.
Behavior with local synthetic block production disabled: unchanged; validator proposer rewards still enter
pending claims and mature by height.
Behavior for producer and non-producer roles: unchanged; all nodes validate the same reward root and release
rules.
Structured evidence source: reward tests, challenge tests, block-root tests, local-testnet Gate 0, and
status docs.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no p2p payload enum changes; chain state root encoding drops the obsolete
pending proposer reward latch byte.

Parallel subagents:
- Read-only subagent launch was not available in the prior iteration because the agent thread limit was
  reached; parent performs the focused mapping directly unless capacity becomes available.

Parallelizable implementation workstreams:
- Parent/integrator owns all edits because this touches consensus state/root encoding and tests.
- No parallel writers.

Tests/checkers/docs to add or update:
- Reward tests covering fallback proposer reward maturity without useful successor.
- Root/reward tests updated so `pending_proposer_reward_root` commits only live proposer claim fields.
- Status docs reflecting that proposer reward delay is height-only plus challenge voiding.

Narrow validation commands:
- `cargo test -p tensor_vm chain::tests::rewards -- --nocapture`
- `cargo test -p tensor_vm chain::tests::challenges -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: pending proposer rewards have no useful-successor state, mature proposer
claims release solely by `claimable_at_height` unless voided, fallback proposer reward tests still pass, and
reward roots remain deterministic.

Out of scope: changing reward amounts, receipt/challenge/credit reward ledgers, auditor-selection policy,
appeals, VRF/drand, fork-choice, Docker `/health`, public evidence, and arbitrary runtime role changes.

Split trigger: if removing the field exposes serialized-state migration requirements beyond local
reference genesis/test fixtures, stop after tests/docs and leave migration design for a separate iteration.

Implementation summary:
- Removed `requires_useful_successor` from `PendingProposerReward`.
- Proposer reward release now filters only by `claimable_at_height`; voided claims are still pruned without
  credit.
- Pending proposer reward roots and chain-state storage encode only block height, proposer, amount,
  claimable height, and challenge-voiding state.
- Existing useful and fallback proposer reward constructors now create the same height-delayed pending
  claim shape.
- Reward tests no longer assert or depend on a useful-successor latch, and the fallback block test helper
  preserves zero-nonce fallback validity while isolating reward-root mismatches.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026.
- Focused reward tests: `cargo test -p tensor_vm chain::tests::rewards -- --nocapture` passed 7 tests.
- Focused challenge tests: `cargo test -p tensor_vm chain::tests::challenges -- --nocapture` passed 3
  tests.
- Focused chain-state storage tests: `cargo test -p tensor_vm storage::chain_state::tests -- --nocapture`
  passed 2 tests.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- Final gate: `cargo test -p tensor_vm local_testnet --release` passed with 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.
- Feature commit `72e16b8` pushed `0387246..72e16b8 main -> main`.

### Iteration 52: Canonical Byte-Packed Int8 Quantization Layout

Feature capability: Tensor IR admits deterministic byte-packed int8 quantization/dequantization by pinning
one canonical `uint8` payload layout for `quantize_pack_int8` and `unpack_dequantize_int8`. The packed
layout is a flat row-major byte tensor with an explicit header, original shape, channel dimension,
fixed-point scale metadata, per-channel signed 64-bit raw scales, and signed int8 payload bytes. This
closes the current byte-packing gap without changing tensor storage internals.

Readiness requirements covered:
- `upow.md` §3 and §4.8: byte-packed quantization has one bit-exact deterministic representation.
- `upow.md` §4.7-§4.9 and §16: the remaining quantization/packing Tier-B vocabulary can move from gated
  registry entries to exact deterministic replay once conformance vectors exist.
- `mvp_spec.md` §7.3 and §35: canonical dtype/layout metadata is pinned for packed int8 tensors.

Files/modules likely touched:
- `crates/tensor_vm/src/ir.rs` for typing, exact pack/unpack execution, registry admission, and IR tests.
- `crates/tensor_vm/src/conformance.rs` for packed vectors and CPU profile coverage.
- `crates/tensor_vm/src/verify.rs` for graph verifier admission coverage if needed.
- Status docs under `docs/tensorvm/`.

Parallel subagents to run:
- Attempted read-only explorer launch for requirements/code/test mapping, but the agent thread limit is
  currently reached; parent performs the mapping and implementation directly.

Parallelizable implementation workstreams:
- Parent/integrator owns all edits because the layout, IR inference, execution, and conformance vectors
  must agree byte-for-byte.
- No parallel writers.

Tests/checkers/docs to add or update:
- IR tests proving pack output bytes, unpack/dequantize reconstruction, graph trace roots, malformed packed
  payload rejection, and registry admission.
- Conformance vectors for `quantize_pack_int8` and `unpack_dequantize_int8`.
- Status docs replacing the current pack/unpack-gated wording with the new layout boundary.

Narrow validation commands:
- `cargo test -p tensor_vm ir::tests -- --nocapture`
- `cargo test -p tensor_vm conformance::tests -- --nocapture`
- Optional focused graph verifier test if verifier coverage is added.

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: `quantize_pack_int8` and `unpack_dequantize_int8` validate for consensus,
execute exactly, produce stable conformance vectors, and appear in the CPU conformance profile. Malformed
packed payloads and mismatched shape/dim metadata are rejected.

Out of scope: changing low-level tensor commitment chunking, CUDA quantization kernels, VRF/drand,
fork-choice, Docker `/health`, public evidence, and arbitrary runtime role changes.

Split trigger: if packed layout admission requires changing tensor commitment/storage APIs or graph receipt
schemas, stop after pinning layout docs/tests and leave storage/API refactors for a separate iteration.

Implementation summary:
- `quantize_pack_int8` now validates for consensus when its input is `Fixed32` and its `dim` kwarg selects
  a valid channel axis. It returns a rank-1 `Uint8` tensor whose bytes encode the canonical packed payload.
- The packed payload begins with `TVQ8`, version `1`, rank, axis, reserved byte, output fixed-point scale
  metadata, original shape dimensions, per-channel signed 64-bit raw scales, and row-major two's-complement
  int8 payload bytes.
- `unpack_dequantize_int8` accepts only the canonical rank-1 `Uint8` packed payload, validates the expected
  `dim`, `shape`, and output scale metadata, and dequantizes through the existing exact per-channel path.
- The CPU conformance profile includes byte-exact vectors for packed quantize and unpack/dequantize.
- Graph verification accepts packed quantize/dequantize receipts only when the CPU conformance profile
  admits the packed op vocabulary.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026.
- Focused IR tests: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 19 tests.
- Focused conformance tests: `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3 tests.
- Focused graph verifier test:
  `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_packed_quantize_dequantize_receipt -- --nocapture`
  passed.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- Final gate: `cargo test -p tensor_vm local_testnet --release` passed with 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.
- Feature commit `1b86f7f` pushed `3df902d..1b86f7f main -> main`.

Out-of-scope items remain: low-level packed tensor storage/chunking API changes, CUDA quantization kernels,
VRF/drand, fork-choice, Docker `/health`, public evidence, and arbitrary runtime role changes.

### Iteration 51: Exact Per-Channel Int8 Quantize/Dequantize Admission

Feature capability: Tensor IR admits exact per-channel int8 quantize/dequantize execution for fixed-point
input tensors using deterministic integer scale selection, round-half-even division, canonical int8 range
checks, multi-output trace commitments, and conformance vectors. Byte packing remains carried but
non-admitted until storage-layout semantics are pinned.

Readiness requirements covered:
- `upow.md` §3 and §4.8: exact quantization becomes deterministic `F_p` integer/fixed-point semantics
  rather than registry vocabulary only.
- `upow.md` §4.7-§4.9 and §16: exact quantization moves into the admitted Tier-B surface while
  byte-packing remains gated.
- `upow.md` §3.3 and `mvp_spec.md` §35: CPU conformance covers the admitted quantization semantics.

Files/modules likely touched:
- `crates/tensor_vm/src/ir.rs` for typing, exact execution, and registry admission.
- `crates/tensor_vm/src/conformance.rs` for multi-output vectors and quantization/dequantization vectors.
- Status docs under `docs/tensorvm/`.

Parallel subagents to run:
- Requested read-only explorers could not be launched because the agent thread limit is currently reached;
  parent performs the mapping and implementation directly.

Parallelizable implementation workstreams:
- Parent/integrator owns all edits because IR typing/execution and conformance vectors must agree exactly.
- No parallel writers.

Tests/checkers/docs to add or update:
- IR tests for quantize/dequantize exact output, shape/type inference, trace-root commitment, and continued
  non-admission for pack/unpack.
- Conformance tests proving CPU profile includes admitted quantize/dequantize vectors and suite hash
  changes deterministically.
- Exec/status docs distinguishing admitted per-channel quantize/dequantize from gated byte packing.

Narrow validation commands:
- `cargo test -p tensor_vm ir::tests -- --nocapture`
- `cargo test -p tensor_vm conformance::tests -- --nocapture`
- `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_quantize_dequantize_receipt -- --nocapture`
  if a graph verifier regression is added.

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: `quantize_int8_per_channel` and `dequantize_int8_per_channel` validate for
consensus, execute exactly, produce stable conformance vectors, and appear in the CPU conformance profile;
`quantize_pack_int8` and `unpack_dequantize_int8` remain non-admitted.

Out of scope: byte-packed tensor storage layout, pack/unpack consensus admission, CUDA quantization kernels,
VRF/drand, fork-choice, Docker `/health`, public evidence, and arbitrary runtime role changes.

Split trigger: if multi-output conformance requires broad receipt/profile schema changes beyond
conformance-vector execution, admit dequantize plus IR quantize execution first and leave quantize
conformance for the next iteration with explicit docs.

Implementation summary:
- `quantize_int8_per_channel` now validates for consensus when its input is `Fixed32` and its `dim` kwarg
  selects a valid channel axis; it returns an `Int8` tensor with the input shape plus a rank-1 `Fixed32`
  scale tensor whose length is the channel count.
- Scale selection is deterministic per channel: `scale_raw = max(1, ceil(max_abs_raw / 127))`; values are
  divided by scale using round-half-even integer division and clamped to `[-128, 127]`.
- `dequantize_int8_per_channel` accepts an `Int8` tensor plus a rank-1 `Fixed32` scale tensor, infers the
  channel axis from the unique matching tensor dimension, broadcasts length-1 scales, and rejects ambiguous
  matches.
- The conformance vector schema now supports multiple expected outputs, and the CPU reference profile
  includes exact quantize/dequantize vectors.
- Graph verification accepts exact quantize/dequantize receipts only when the CPU conformance profile
  includes the admitted quantization op.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026.
- Focused IR tests: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 18 tests.
- Focused conformance tests: `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3 tests.
- Focused graph verifier test:
  `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_quantize_dequantize_receipt -- --nocapture`
  passed.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- Final gate: `cargo test -p tensor_vm local_testnet --release` passed with 5 local-testnet library tests
  plus `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.

Out-of-scope items remain: byte-packed tensor storage layout, pack/unpack consensus admission, CUDA
quantization kernels, VRF/drand, fork-choice, Docker `/health`, public evidence, and arbitrary runtime role
changes.

### Iteration 50: Quantization DType And Gated Registry Foundation

Feature capability: the IR and tensor metadata can represent the spec's exact integer dtypes needed by
quantization (`int8`, `uint8`, and `bool`), and the frozen op registry carries the four quantization/packing
op names with arity/kwarg/output metadata while keeping them non-consensus-admitted until their per-channel
scale, saturation, and byte-packing semantics have executable conformance vectors.

Readiness requirements covered:
- `upow.md` §4.1: the value model includes `int8`, `uint8`, and `bool`; the code currently cannot parse or
  commit those dtypes.
- `upow.md` §4.7 and §16: quantization/packing ops are part of the frozen vocabulary but exact
  quantization semantics and conformance vectors are still TODO.
- `mvp_spec.md` §4.1, §7.2, §8.3, and §35: deterministic integer/fixed-point arithmetic requires
  canonical dtype/layout metadata before quantization execution can be admitted.

Canonical owner: `tensor::DType` owns consensus-visible dtype tags and tensor commitment metadata;
`ir::FROZEN_OP_REGISTRY` owns quantization op vocabulary/admission state; shared codecs own dtype tag
roundtrips.
Adapter callers: job payload codecs, p2p tensor payloads, storage snapshots, graph canonical JSON, and
runtime tensor serving consume dtype tags/names; no adapter gets quantization execution policy.
Old shortcut being removed: the spec's `int8`/`uint8`/`bool` value model can no longer be silently absent
from graph parsing and tensor commitments, and quantization ops can no longer be unknown rather than
explicitly carried but gated.
Regression test that proves the shortcut is gone: dtype tag/name/canonical JSON tests accept the new dtypes;
registry tests find all four quantization ops with exact metadata; consensus graph validation rejects those
ops because they are deliberately non-admitted.
Behavior with local synthetic block production disabled: unchanged; this is metadata and validation
vocabulary only.
Behavior for producer and non-producer roles: unchanged; all roles decode the same dtype tags and continue
rejecting unadmitted quantization graphs.
Structured evidence source: dtype roundtrip tests, tensor commitment tests, codec/p2p decode tests, IR
registry/validation tests, and docs status.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: shared dtype tags gain new values; no block/job/receipt enum variants or
payload length formats change.

Parallel subagents:
- Socrates mapped quantization requirements and recommended a foundation slice.
- Bohr inspected IR/conformance/code paths and hazards.
- Euler mapped focused tests and validation targets.

Parallelizable implementation workstreams:
- Parent/integrator owns edits because dtype tags touch shared tensor, codec, p2p, and IR parsing code.
- Subagents are read-only; no parallel writers.

Tests/checkers/docs to add or update:
- Tensor dtype tag/range/commitment tests for `int8`, `uint8`, and `bool`.
- Shared codec and p2p tensor payload dtype roundtrip tests.
- IR dtype JSON/canonical graph tests and quantization registry-gating tests.

Implemented scope:
- Added `DType::Int8`, `DType::Uint8`, and `DType::Bool` with stable tags `5`, `6`, and `7`.
- Tensor construction validates canonical int8, uint8, and bool ranges before normalizing field elements.
- `Tensor::random` now emits in-range random elements for the new narrow dtypes.
- Shared codec and p2p tensor payload decoding accept the new dtype tags and reject malformed bool payloads.
- Tensor IR canonical JSON parses/renders `int8`, `uint8`, and `bool`, and graph IDs commit those dtype names.
- `quantize_int8_per_channel`, `dequantize_int8_per_channel`, `quantize_pack_int8`, and
  `unpack_dequantize_int8` are present in the frozen registry with exact metadata but
  `consensus_admitted: false`.

Validation:
- First and final `cargo test -p tensor_vm local_testnet --release` passed on June 20, 2026.
- Focused tests passed: `cargo test -p tensor_vm tensor::tests -- --nocapture`,
  `cargo test -p tensor_vm codec::tests -- --nocapture`,
  `cargo test -p tensor_vm p2p::wire::tests -- --nocapture`, and
  `cargo test -p tensor_vm ir::tests -- --nocapture`.
- Broad gates passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- `cargo tarpaulin --workspace --offline` remains blocked by missing `cargo-tarpaulin`.
- Feature commit `b89bb18` pushed `42365a2..b89bb18 main -> main`; evidence commit `4c4d527`
  pushed `b89bb18..4c4d527 main -> main`.
- Status/coverage/exec docs clarifying that dtype/registry foundation is implemented while exact
  quantization execution/conformance remains open.

Narrow validation commands:
- `cargo test -p tensor_vm tensor::tests -- --nocapture`
- `cargo test -p tensor_vm codec::tests -- --nocapture`
- `cargo test -p tensor_vm p2p::wire::tests -- --nocapture`
- `cargo test -p tensor_vm ir::tests -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Expected observable evidence: canonical Tensor IR accepts and commits `int8`/`uint8`/`bool` metadata, wire
codecs roundtrip those dtype tags, quantization ops appear in the frozen registry with deterministic metadata,
and consensus validation rejects quantization graphs until executable exact semantics and conformance vectors
exist.

Out of scope: executing quantization ops, per-channel scale tensors, byte-packed tensor storage layout,
multi-output conformance vectors, CUDA quantization evidence, VRF/drand, fork-choice, runtime role changes,
and Docker `/health`.

Split trigger: if adding dtype tags requires broad codec/storage migrations beyond additive tag parsing,
split before registry entries and keep this iteration to dtype support only.

### Iteration 49: Fixed-Point Scale Metadata And Round-Half-Even Rescale Foundation

Feature capability: runtime tensors carry consensus-visible scale metadata, execution inputs enforce
`TensorSpec.scale`, and the exact IR interpreter/conformance suite use one canonical signed fixed-point
round-half-even rescale primitive for `cast` and fixed-point `round`.

Readiness requirements covered:
- `upow.md` §3.1 and §4.8: fixed-point scale changes and rounding are consensus semantics, not adapter
  metadata.
- `upow.md` §3.3 and §16: conformance vectors must be able to express dtype/scale for fixed-point
  operations before runtimes can be admitted.
- `mvp_spec.md` §8.3, §14, and §35: fixed-point/integer training-shaped transitions require explicit
  rounding/scale rules before stronger real-valued claims or quantization admission.

Canonical owner: `tensor::Tensor` owns runtime dtype/scale metadata; `ir::TensorGraph::execute_exact` owns
scale-aware execution and validation; `conformance` owns scale-aware pass vectors.
Adapter callers: jobs, graph verification, runtime backend profile checks, storage/tensor serving, and role
execution consume `Tensor` values; no adapter gets new rounding or scale policy.
Old shortcut being removed: `TensorSpec.scale` can no longer be declaration-only while runtime tensors and
conformance vectors silently ignore it, and `cast`/`round` can no longer be only raw field copies for
fixed-point scale changes.
Regression test that proves the shortcut is gone: fixed-point execution rejects input scale mismatches,
`cast`/`round` golden tests cover positive and negative half-even ties, and conformance vectors include
per-input/per-output dtype and scale.
Behavior with local synthetic block production disabled: unchanged; this is pure IR/tensor/conformance
capability.
Behavior for producer and non-producer roles: unchanged; all roles consume the same `Tensor` metadata and
graph verifier conformance gate.
Structured evidence source: tensor scale accessors/commitment roots, `IrExecution` outputs/traces,
scale-aware conformance vectors, and focused tests.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: no consensus payload enum changes; tensor bytes/commitments become
scale-sensitive through canonical tensor metadata.

Parallel subagents:
- Volta mapped readiness requirements and split triggers for fixed-point/quantization.
- Schrodinger mapped dtype/scale implementation paths and graph-verifier coupling.
- Einstein mapped tests and conformance schema gaps.

Parallelizable implementation workstreams:
- Parent/integrator owns code edits because `Tensor` scale metadata touches shared constructors and tests.
- Subagents are read-only; no parallel writers.

Tests/checkers/docs to add or update:
- Tensor tests for scale metadata and scale-sensitive commitment roots.
- IR tests for input scale enforcement and fixed-point half-even `cast`/`round`.
- Conformance schema/vector tests for fixed-point scale-aware vectors.
- Graph verifier test proving a fixed-point cast/round graph receipt is admitted only with matching
  conformance.
- Update status/coverage/exec/tarpaulin docs after validation.

Narrow validation commands:
- `cargo test -p tensor_vm tensor::tests -- --nocapture`
- `cargo test -p tensor_vm ir::tests -- --nocapture`
- `cargo test -p tensor_vm conformance::tests -- --nocapture`
- `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_fixed_point_rescale_receipt -- --nocapture`

Broad validation commands before commit:
- `cargo fmt --check --all`
- `git diff --check`
- `cargo test -p tensor_vm`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --release`
- `cargo test -p tensor_vm local_testnet --release`
- `cargo tarpaulin --workspace --offline` expected blocked while `cargo-tarpaulin` is missing.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused tensor module: `cargo test -p tensor_vm tensor::tests -- --nocapture` passed 8 tests.
- Focused IR module: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 15 tests.
- Focused conformance module: `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3
  tests.
- Focused graph verifier: `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_fixed_point_rescale_receipt -- --nocapture`
  passed.
- Scale-bound tensor IDs changed a challenge-test receipt assignment; the fixture now derives the assigned
  validator from `JobScheduler` instead of assuming the proposer remains assigned.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed 370 library tests plus integration tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- Final `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.

Commit evidence:
- Feature commit: `a14ba9b` (`Add fixed point scale rescale foundation`), pushed to `main` on June 20,
  2026 (`d8295ab..a14ba9b`).
- Evidence commit: `ca856f6` (`Record fixed point scale validation evidence`), pushed to `main` on
  June 20, 2026 (`a14ba9b..ca856f6`).

Expected observable evidence: a fixed-point graph with nonzero scale can exact-execute with deterministic
round-half-even scale changes, graph verification rejects missing conformance, and tensor commitments bind
scale metadata.

Out of scope: exact quantization ops and packing, fixed-point transcendental references, CUDA fixed-point
pass evidence, VRF/drand, fork-choice, runtime role changes, codec enum changes, and Docker `/health`.

Split trigger: split before quantization if adding scale metadata requires broad tensor/storage/commitment
updates or breaks existing current-job receipt roots.

## Recent Iterations

### Iteration 48: Exact Unary Tier-B IR Replay And Conformance

Feature capability: consensus-admitted exact unary Tier-B ops `identity`, `neg`, `abs`, `sign`, `round`,
and `relu` execute
through `TensorGraph::execute_exact`, receive deterministic `F_p` conformance vectors, and can be used by
graph-backed receipts without failing the conformance gate.

Readiness requirements covered:
- `upow.md` §3.3 and §16: the CPU reference conformance suite must cover admitted deterministic `F_p`
  semantics before receipts are accepted.
- `upow.md` §4.7-§4.9: exact Tier-B unary ops carried as consensus-admitted registry entries must have
  executable deterministic semantics rather than validate-but-fail interpreter behavior.
- `mvp_spec.md` §8 and §35: deterministic TensorVM operation semantics and cross-runtime conformance
  evidence for block-eligible receipts.

Canonical owner: `ir::TensorGraph::execute_exact` owns exact unary op execution; `conformance` owns the
runtime pass profile and suite hash used by graph receipt verification.
Adapter callers: `verify_graph_execution`, role graph verification, and graph receipt tests consume the
conformance profile; no adapter gets new consensus logic.
Old shortcut being removed: registry admission can no longer list these unary Tier-B ops as admitted while
the exact interpreter rejects them or the conformance profile lacks pass evidence.
Regression test that proves the shortcut is gone: IR tests execute a graph containing unary ops and graph
verification tests accept a graph receipt using one of the newly covered ops only when the CPU conformance
profile includes it.
Behavior with local synthetic block production disabled: unchanged; this is pure IR/conformance capability.
Behavior for producer and non-producer roles: unchanged; validators that see graph receipts use the same
conformance profile and exact replay path.
Structured evidence source: `IrExecution` outputs/traces, conformance vector suite hash/pass set, graph
verification report, and focused tests.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: unchanged; no payload format changes.

Parallel subagents:
- Meitner mapped exact unary Tier-B coverage to `upow.md`/MVP requirements and risks.
- Linnaeus inspected `ir`, `conformance`, and verifier touch points.
- Dewey identified focused tests and stale docs.

Parallelizable implementation workstreams:
- Parent/integrator owns edits because the slice is small and centered on shared IR/conformance modules.
- Subagents remain read-only support; no parallel writers.

Tests/checkers/docs to add or update:
- Focused IR execution test for unary ops.
- Conformance vector coverage/pass tests for `identity`, `neg`, `abs`, `sign`, `round`, and `relu`.
- Graph verification test proving a graph receipt using a newly covered op is accepted.
- Update status/exec/tarpaulin docs after validation.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR module: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 14 tests.
- Focused unary IR: `cargo test -p tensor_vm ir::tests::exact_interpreter_executes_unary_tier_b_ops -- --nocapture`
  passed.
- Focused conformance: `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3 tests.
- Focused graph verifier: `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_unary_tier_b_graph_receipt -- --nocapture`
  passed.
- After clippy's `manual_div_ceil` finding was fixed, `cargo test -p tensor_vm ir::tests::exact_interpreter_executes_unary_tier_b_ops -- --nocapture`
  and `cargo test -p tensor_vm conformance::tests::cpu_reference_passes_all_vectors -- --nocapture` passed.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed 367 library tests plus integration tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- Final `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.

Expected observable evidence: a consensus-valid graph containing exact unary Tier-B ops can exact-execute,
its receipt can be verified through the graph verifier using the CPU conformance profile, and missing
conformance still rejects it.

Out of scope: exact quantization, fixed-point rescale/round-half-even semantics beyond identity rounding for
current field/integer tensors, Tier-C/transcendental admission, runtime role changes, codec changes,
VRF/drand, fork-choice, and Docker `/health`.

Commit evidence:
- Feature commit: `46050d2` (`Add exact unary IR conformance`).
- Evidence commit: `e12ab32` (`Record exact unary validation evidence`), pushed to `main` on June 20,
  2026 (`1ac2197..e12ab32`).

### Iteration 47: Graph-Backed Exact Job And Receipt Admission

Feature capability: registered canonical Tensor IR graph bodies can be referenced by first-class
`GraphExecution` jobs and receipts, encoded over the shared payload codec, rooted/persisted in chain state,
exact-replay verified through `TensorGraph::execute_exact`, and settled/rewarded through the existing receipt
machinery when validators attest.

Readiness requirements covered:
- `upow.md` §4.4-§4.6: jobs reference content-addressed graphs by `graph_id` and require registered,
  consensus-valid canonical bodies.
- `upow.md` §5: graph receipts commit to named input roots, output roots, `trace_root`, miner identity,
  deterministic receipt id, and signature.
- `upow.md` §11.1: graph receipts enter the same canonical receipt/attestation/settlement/blockspace path
  as fixed current jobs.

Canonical owner: `ir::TensorGraph::execute_exact` remains the owner of exact graph execution and trace-root
construction; `chain::receipts` owns job/receipt admission checks.
Adapter callers: shared payload codec, storage, p2p wire wrappers, node payload application, and role tests
consume the new `JobState`/`ReceiptState` variants without adding consensus decisions.
Old shortcut being removed: arbitrary registered graph bodies can no longer only exist as inert program
bodies; they can be bound to executable jobs and receipt records.
Regression test that proves the shortcut is gone: graph job/receipt chain tests, codec/storage/root tests,
role exact replay tests, and settlement tests for graph receipts.
Behavior with local synthetic block production disabled: unchanged; graph jobs can be admitted through
shared commands/payloads, but the local synthetic job source keeps emitting only current canonical jobs.
Behavior for producer and non-producer roles: producer policy is unchanged; validators verify graph receipts
through the same role verifier once graph tensors/artifacts are available.
Structured evidence source: `GraphJob`, `GraphReceipt`, state roots, shared payload roundtrips, receipt
settlement events, and focused tests.
Finality source: unchanged stake-weighted block votes.
Wire-size and codec boundary: add bounded map/string encodings to the existing shared job/receipt payload
codec; p2p wire continues delegating to that shared codec.

Parallel subagents:
- Copernicus mapped the readiness slice and scope boundaries.
- Dalton mapped the data model, admission checks, codec/storage/p2p/role paths.
- Lorentz mapped existing coverage and missing tests.

Parallelizable implementation workstreams:
- Parent/integrator owns code changes because `JobState`/`ReceiptState` variants touch shared files.
- Read-only explorers remain background support only; no parallel writers.

Tests/checkers/docs to add or update:
- Added focused `jobs`, `codec`, `chain`, `storage`, `roles`, and settlement tests.
- Updated status/tarpaulin evidence after validation.

Narrow validation commands:
- `cargo test -p tensor_vm graph -- --nocapture` passed 10 focused graph tests.
- `cargo test -p tensor_vm codec::tests` passed 6 codec/storage-codec tests.
- `cargo test -p tensor_vm storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`
  passed.

Broad validation commands before commit:
- First required gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- `cargo fmt --all` applied formatting; `git diff --check` passed.
- `cargo test -p tensor_vm` passed 365 library tests plus integration tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed.
- Final `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` remains blocked because `cargo-tarpaulin` is not installed.
- Feature commit: `decdf91` (`Add graph execution jobs and receipts`).
- Evidence commit: `1ac2197` (`Record graph execution validation evidence`), pushed to `main` on June 20,
  2026 (`c3706cc..1ac2197`).

Observable evidence: a registered non-fixed graph can be submitted as a graph job, produce an
exact trace-root receipt, survive codec/storage/root paths, receive a valid graph attestation, and settle
through the same delayed reward path.

Out of scope: Tier-C consensus admission, redundancy committee redesign, fraud games, `const_blob` fetching,
CUDA generic graph execution, mixed-dtype conformance vectors, exact quantization/signed fixed-point unary
completion, VRF/drand lifecycle, multi-validator fork-choice, and Docker `/health`.

Split trigger: split smaller if the app role runtime or p2p payload admission changes require unrelated
status/checker rewrites beyond compiling the new variants and proving shared-codec roundtrips.

### Iteration 46: Canonical Current-Job IR Trace Roots

Feature capability: current canonical TensorOp and LinearTrainingStep receipts derive and verify
`trace_root` from their canonical `TensorGraph::execute_exact` op traces instead of parallel handcrafted
trace-hash shortcuts.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph::execute_exact` remains the owner of exact graph execution and trace
  root construction.
- Adapter callers: `jobs` and `verify` consume canonical graph execution results for current fixed job
  records only; role/runtime/p2p adapters stay unchanged.
- Old shortcut removed: current receipt constructors and verifiers no longer build separate
  receipt-specific trace roots that can diverge from the canonical IR DAG trace.
- Regression tests: current job receipt tests assert trace roots equal exact graph execution roots; verifier
  mismatch tests continue rejecting altered trace commitments.
- Synthetic production disabled: unchanged; current canonical job execution semantics only.
- Producer/non-producer roles: unchanged; arbitrary graph-backed role execution remains later work.
- Structured evidence source: receipt `trace_root`, `IrExecution.trace_root`, focused jobs/verify tests,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused jobs tests: `cargo test -p tensor_vm --lib jobs::tests -- --nocapture` passed 2 tests.
- Focused verifier tests: `cargo test -p tensor_vm --lib verify::tests -- --nocapture` passed 13 tests.
- Focused challenge/settlement/watcher/reward regressions passed:
  `challenge::tests::fraud_challenge_proves_invalid_tensorop_and_resolves_slash`,
  `chain::tests::settlement::conflicting_linear_training_roots_do_not_settle`, `watcher::tests`, and
  `chain::tests::rewards::reward_root_commits_to_all_pending_reward_ledgers`.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 361 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final local-testnet gate: `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet
  library tests plus the filtered service-gateway integration test.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `9aaf2c9` (`Use canonical IR trace roots for receipts`).
- Push evidence: pushed to `main` on June 20, 2026 (`37d1446..9aaf2c9`).

Out of scope: arbitrary graph-backed job/receipt record types, chain/runtime receipt production for
arbitrary registered graphs, p2p/codec changes, generic graph verifier economics, const-blob fetching,
signed/fixed-point unary semantics, exact quantization, and Docker `/health`.

### Iteration 45: Remaining Exact Tier-B Shape/Reduction IR Replay

Feature capability: validated Tensor IR graphs can exact-execute `mean`, `cast`, `concat`, and `stack`;
concat/stack validation uses real axis shape rules instead of the previous same-shape placeholder.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph` validation/exact execution and conformance suite metadata.
- Adapter callers: current runtime/job verification paths consume accepted graph IDs/profiles; no adapter
  gains consensus mutation.
- Old shortcut being removed: admitted Tier-B graph ops no longer fail closed or validate with placeholder
  same-shape typing solely because deterministic exact replay was missing.
- Regression tests: new IR execution tests for mean/cast/concat/stack and conformance vector tests where
  the current same-dtype vector schema fits.
- Synthetic production disabled: unchanged; this is pure IR execution capability.
- Producer/non-producer roles: unchanged until graph-backed job admission is wired later.
- Structured evidence source: `IrExecution` named outputs, `IrOpTrace` roots, conformance profile pass set,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Validation target:
- Focused: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` and
  `cargo test -p tensor_vm --lib conformance::tests -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`,
  final `cargo test -p tensor_vm local_testnet --release`, and the expected tarpaulin blocked check.

Current validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR tests: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` passed 13 tests.
- Focused conformance tests: `cargo test -p tensor_vm --lib conformance::tests -- --nocapture` passed 3
  tests.
- Formatting/whitespace: `cargo fmt --check --all` and `git diff --check` passed.
- TensorVM crate: `cargo test -p tensor_vm` passed 361 library tests plus integration tests.
- Lints: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Release workspace: `cargo test --workspace --release` passed.
- Final local-testnet gate: `cargo test -p tensor_vm local_testnet --release` passed 5 local-testnet
  library tests plus the filtered service-gateway integration test.
- Coverage attempt: `cargo tarpaulin --workspace --offline` remains blocked by `error: no such command:
  tarpaulin`.
- Feature commit: `7154f6a` (`Complete exact Tier-B IR replay`).
- Push evidence: pushed to `main` on June 20, 2026 (`da008c3..7154f6a`).

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, signed/fixed-point unary op semantics, exact
quantization, mixed-dtype conformance-vector schema, and CUDA generic graph execution.

### Iteration 44: Wider Exact Tier-B IR Interpreter Coverage

Feature capability: validated Tensor IR graphs can exact-execute a broader admitted Tier-B surface:
broadcast-aware `add`/`sub`/`mul`, `reshape`, `broadcast`, comparisons `gt`/`lt`/`ge`/`le`/`eq`, `where`,
`full`, and `arange`. Field-only conformance vectors now cover `reshape`, `broadcast`, `full`, and
`arange`.

Architecture shortcut answers:
- Canonical owner: `ir::TensorGraph` validation/exact execution and conformance suite metadata.
- Adapter callers: current runtime/job verification paths consume accepted graph IDs/profiles; no adapter
  gains consensus mutation.
- Old shortcut being removed: admitted Tier-B graph ops no longer fail closed solely because deterministic
  exact replay was missing.
- Regression tests: `exact_interpreter_executes_shaping_comparison_generators_and_where`,
  `graph_validation_rejects_inconsistent_exact_tier_b_shapes`, and conformance vector tests.
- Synthetic production disabled: unchanged; this is pure IR execution capability.
- Producer/non-producer roles: unchanged until graph-backed job admission is wired later.
- Structured evidence source: `IrExecution` named outputs, `IrOpTrace` roots, conformance profile pass set,
  docs matrix/status entries.
- Finality source: unchanged stake-weighted block votes.
- Wire-size and codec boundary: no wire codec changes.

Implementation target:
- Add deterministic exact replay helpers inside `ir.rs` rather than broadening the public tensor API.
- Tighten validation for reshape element counts and arange declared output length.
- Add graph-level execution tests for shaping, comparison, generator, and selection ops.
- Add field-only conformance vectors where the current single-dtype vector schema fits.
- Update coverage/status/readiness/tarpaulin docs and keep this plan compact.

Validation target:
- Focused: `cargo test -p tensor_vm --lib ir::tests -- --nocapture` and
  `cargo test -p tensor_vm --lib conformance::tests -- --nocapture`.
- Broad: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --release`,
  final `cargo test -p tensor_vm local_testnet --release`, and the expected tarpaulin blocked check.

Validation evidence:
- Required first gate: `cargo test -p tensor_vm local_testnet --release` passed before edits.
- Focused IR tests passed: 12 `ir::tests`.
- Focused conformance tests passed: 3 `conformance::tests`.
- `cargo fmt --check --all` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm` passed: 360 library tests, 1 local CPU Compose integration test, 8
  `tvmd_cli` tests, 31 `tvmd_runtime` tests, and doc-tests.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --release` passed: 14 `experiments` tests, 360 `tensor_vm` library tests,
  integration tests, 1 `tensor_vm_explorer` library test, 2 explorer CLI tests, and doc-tests.
- Final `cargo test -p tensor_vm local_testnet --release` passed: 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo tarpaulin --workspace --offline` failed as expected because `cargo-tarpaulin` is not installed:
  `error: no such command: tarpaulin`.
- Feature commit: `ce3deea` (`Widen exact Tensor IR replay coverage`).
- Feature push: `git push` to `github.com:distributedstatemachine/tensor_vm.git` updated `main -> main`
  from `699193e` to `ce3deea`.

Out of scope: arbitrary graph-backed job/receipt records, role/runtime receipt production through
`TensorGraph::execute_exact`, const-blob fetching, mixed-dtype conformance-vector schema, exact replay for
`mean`, `cast`, exact unary fixed-point ops, concat/stack, exact quantization, and CUDA generic graph
execution.

### Iteration 43: Explicit Fallback Reward Maturity Delay

Empty `PowSkipFallback` proposer claims now mature by height using the full reward-settlement plus
challenge-window delay, instead of remaining blocked behind a later-useful-block unlock latch. Evidence:
first and final `cargo test -p tensor_vm local_testnet --release` passed; focused reward, command, and
attestation/audit tests passed; `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm`,
`cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release` passed;
`cargo tarpaulin --workspace --offline` remained blocked by missing `cargo-tarpaulin`. Feature commit
`b0fd68e` pushed `2ee2340..b0fd68e main -> main`; evidence commit `699193e` pushed
`b0fd68e..699193e main -> main`.

### Iteration 42: State-Rooted Arbitrary Tensor IR Graph-Body Admission

Arbitrary canonical Tensor IR graph bodies can enter canonical chain state independently of fixed job
constructors, survive node-store persistence, and hydrate the runtime program server for the existing
bounded `RequestProgram`/`ProgramResponse` path. Evidence: first/final Gate 0 passed; focused chain, IR,
runtime hydration, and chain-state tests passed; broad format, unit, clippy, workspace release gates
passed; tarpaulin remained blocked by missing `cargo-tarpaulin`. Feature commit `9a32039` pushed
`b5fd81d..9a32039 main -> main`; evidence commit `2ee2340` pushed `9a32039..2ee2340 main -> main`.

## Decision Log

- `upow.md` is canonical where docs conflict; update stale readiness text when touched.
- Validators own useful-verification block proposal; TensorWork affects rewards, blockspace, telemetry, and
  concentration analysis only.
- `tvmd` remains an adapter/process launcher; counted roles must communicate through libp2p/RPC before
  affecting another node.
- Reward spendability is delayed through state-rooted pending claims. Fallback proposer rewards now use an
  explicit reward-maturity height and no longer require a useful-block unlock.
- Full Docker local CPU readiness remains blocked by gateway `/health`; do not claim full local production
  readiness until that gate passes.
- `cargo tarpaulin --workspace --offline` cannot regenerate coverage in this environment until
  `cargo-tarpaulin` is installed.

## Validation Evidence

- Latest current-iteration Gate 0: `cargo test -p tensor_vm local_testnet --release` passed first on
  June 20, 2026 with 5 local-testnet library tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- Latest focused tests: `cargo test -p tensor_vm ir::tests -- --nocapture` passed 18 IR tests;
  `cargo test -p tensor_vm conformance::tests -- --nocapture` passed 3 conformance tests; and
  `cargo test -p tensor_vm verify::tests::graph_verifier_accepts_quantize_dequantize_receipt -- --nocapture`
  passed.
- Latest broad gates: `git diff --check`, `cargo test -p tensor_vm`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`
  passed after `cargo fmt --check --all`.
- Latest feature commit: `c04af93` (`Admit exact int8 quantize dequantize`) pushed
  `8c1323a..c04af93 main -> main`.
- Current tarpaulin blocker:

```text
error: no such command: `tarpaulin`
```

- Current Docker blocker:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
```

## Archive

- Iteration 41, `e86258e` / `b5fd81d`: added the generic exact-IR interpreter foundation with named
  outputs, per-op output roots, and `trace_root` over the exact ops already available at the time.
- Iterations 39-40, `3001501`, `7652f13`, `6cea749`, `b3637fc`: anchored receipt validation randomness
  to receipt-time finalized beacon state and added reduced delayed fallback proposer claims, later refined
  by Iteration 43.
- Iterations 35-38, `f53700c`, `587b111`, `584e5d4`, `4984e6f`: bound reward roots to all pending reward
  ledgers and moved mature reward release through normal block transitions.
- Iterations 30-34, `5664acb` and related evidence commits: delayed proposer, receipt, challenger, and
  generic credit rewards as state-rooted pending claims.
- Iterations 26-29: added challenge rewards, unavailable-receipt slashing/reward voiding, validator audit
  slashing, and network-visible audit/challenge ingestion.
- Earlier iterations established role-owned local miner/validator work, network event ingestion,
  TensorBlock/UVPoW foundations, Tensor IR graph IDs, frozen registry metadata, conformance gates for
  current jobs, public-evidence validators/templates, finalized-beacon randomness binding, block apply
  openings, retarget/fallback mode, and checker evidence for role-owned local work.
