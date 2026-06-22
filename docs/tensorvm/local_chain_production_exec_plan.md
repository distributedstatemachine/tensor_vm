# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 184 complete: Trace-Bisection DoS Admission Bounds.
- Current status: the local CPU checker requires live TensorOp, LinearTrainingStep, and GraphExecution
  receipt/block evidence. Graph receipt verification test scenarios cover every consensus-admitted frozen
  registry op locally, and explorer WebSocket jobs/receipts now expose the same `graph_execution`
  primitive already carried by chain/RPC/checker paths.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
  - Deployed full VRF construction, deployed commit-reveal lifecycle evidence, and public/CUDA graph
    execution evidence remain open.
- Next action: continue CUDA/public deployment evidence or remaining deployed-randomness/economic evidence.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Current iteration first command `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection expectations/rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios for every consensus-admitted op, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops, receipt verification scenario drift coverage for every admitted op; default CUDA non-admission | Add CUDA conformance evidence and deployed CUDA profile evidence |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 184: Trace-Bisection DoS Admission Bounds

Feature capability: bound interactive trace-bisection resource use in the canonical chain path. Opening a
trace dispute now must fit the protocol's maximum admitted bisection depth, and challenger midpoint
expectations are idempotent only for exact duplicates while conflicting pending expectation overwrites are
rejected.

Readiness requirements covered: `goal.md`/`upow.md` fraud-proof game liveness, multi-round trace-bisection
DoS policy, and chain-owned challenge admission.

Canonical owner: `challenge` defines the bisection-round budget and cap; `chain::challenges` enforces
admission and pending-expectation state transitions.

Adapter callers: existing open/expectation command callers, p2p payload handlers, and runtime challenger
generation; no new wire format is required.

Old shortcut being removed: any nonempty `op_count` was admissible regardless of worst-case bisection
depth, and a challenger could repeatedly overwrite a pending expectation for the same midpoint before the
responder round arrived.

Regression tests that prove the shortcut is gone:
`chain::tests::trace_bisection_admission_enforces_round_budget_and_pending_expectation_policy` rejects
oversized disputes, accepts duplicate expectation replay idempotently, and rejects conflicting pending
expectation overwrites.

Behavior with local synthetic block production disabled: unchanged; the policy is a chain-state admission
rule over submitted challenge commands.

Behavior for producer and non-producer roles: unchanged; any role that submits or ingests the shared
commands observes the same canonical rejection/acceptance rules.

Structured evidence source: chain command errors/events, trace-bisection pending expectation state,
state-rooted challenge records, and this exec plan.

Finality source: unchanged; this is pre-finality challenge admission and bounded transcript progression.

Wire-size and codec boundary: unchanged; existing config, expectation, and event fields are reused.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to challenge budget helpers,
chain admission, focused tests, and docs/status alignment.

Tests/checkers/docs to add or update: chain admission test, `upow.md`, `coverage_matrix.md`,
`implementation_status.md`, `tarpaulin_report.md`, and this exec plan.

Narrow validation commands: `cargo test -p tensor_vm
trace_bisection_admission_enforces_round_budget_and_pending_expectation_policy --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because the test count/coverage changes.

Expected observable evidence: oversized bisection sessions fail before state insertion, exact duplicate
expectation replay stays accepted, conflicting pending expectation overwrites fail, and normal
round-response progression still clears the pending expectation.

Out of scope: public/CUDA deployed dispute evidence, new p2p payload types, per-profile cap tuning, and
new standalone verifier binaries.

Split trigger: split only if the cap must become a persisted chain parameter or wire field.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm trace_bisection_admission_enforces_round_budget_and_pending_expectation_policy --lib`
  passed.
- `cargo test -p tensor_vm trace_bisection_rounds_are_chain_admitted_and_state_rooted --lib` passed.
- `cargo test -p tensor_vm trace_bisection_rounds_narrow_to_disputed_op --lib` passed.
- `cargo test -p tensor_vm trace_bisection_expectation_payload_application_reports_pending_applied_and_invalid_edges --lib`
  passed.
- `cargo test -p tensor_vm network_event_driver_applies_and_retries_trace_bisection_expectation_payloads --lib`
  passed.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 548 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 563
  workspace tests under instrumentation, 84.59% line coverage, 22635/26757 lines covered.
- Manual ownership-boundary review: no standalone verifier binary was used or added; bisection budget
  math lives in the protocol challenge module, chain admission rejects oversized sessions before state
  insertion, duplicate expectation replay remains idempotent for p2p retries, conflicting expectation
  overwrites are rejected while a midpoint response is pending, and no p2p, storage, or RPC wire format
  changed.

### Iteration 183: Isolated Trace-Bisection Timeout Policy

Feature capability: close incomplete trace-bisection transcripts after the final op is isolated. Active
sessions still time out against a non-responsive responder, while isolated sessions that pass their
deadline without a referee witness time out against the challenger so bonds and receipt state cannot remain
stuck indefinitely.

Readiness requirements covered: `goal.md`/`upow.md` fraud-proof game liveness, incomplete-transcript
handling, delayed challenge rewards, and chain-owned challenge finality.

Canonical owner: `chain::challenges::record_trace_bisection_timeout` owns timeout settlement for both
active and isolated trace-bisection records.

Adapter callers: existing `ChainCommand::RecordTraceBisectionTimeout` callers; no new runtime, p2p, RPC,
or storage command surface is required.

Old shortcut being removed: an isolated record could only be refereed; if the challenger did not submit the
single-op witness after the responder supplied the final opening, the challenge could stay isolated with
unsettled bonds.

Regression test that proves the shortcut is gone:
`chain::tests::isolated_trace_bisection_timeout_slashes_incomplete_challenger` isolates a one-op dispute,
rejects timeout before the deadline, advances past the deadline, then requires the challenger to forfeit
without voiding the responder/miner receipt reward path or issuing a challenger bounty.

Behavior with local synthetic block production disabled: unchanged; the timeout is a chain-state command
over an existing challenge record.

Behavior for producer and non-producer roles: unchanged; any role that submits the shared chain command
observes the same canonical state transition.

Structured evidence source: trace-bisection record status, stake/treasury changes, pending reward ledgers,
state root/snapshot persistence, and this exec plan.

Finality source: chain height relative to the trace-bisection response deadline.

Wire-size and codec boundary: unchanged; reuses the existing `TimedOut` status and command/event surface.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to chain timeout policy,
focused chain tests, and docs/status alignment.

Tests/checkers/docs to add or update: chain timeout test, `upow.md`, `coverage_matrix.md`,
`implementation_status.md`, `tarpaulin_report.md`, and this exec plan.

Narrow validation commands: `cargo test -p tensor_vm
isolated_trace_bisection_timeout_slashes_incomplete_challenger --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because the test count/coverage changes.

Expected observable evidence: isolated trace-bisection timeout emits `TraceBisectionTimedOut` with the
challenger as forfeiting party, slashes the challenger bond to treasury, leaves receipt rewards unvoided,
and creates no pending challenger reward.

Out of scope: public/CUDA deployed dispute evidence, new p2p payloads, max-round admission caps, and new
standalone verifier binaries.

Split trigger: split only if isolated timeout requires new storage or wire variants; the current design
should reuse existing state variants.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm isolated_trace_bisection_timeout_slashes_incomplete_challenger --lib`
  passed.
- `cargo test -p tensor_vm trace_bisection_chain_admission_rejects_mismatch_and_records_timeout --lib`
  passed.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 547 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 562
  workspace tests under instrumentation, 84.59% line coverage, 22621/26741 lines covered.
- Manual ownership-boundary review: no standalone verifier binary was used or added; timeout finality
  remains chain-owned through `ChainCommand::RecordTraceBisectionTimeout`; active sessions still punish
  non-responsive responders; isolated sessions that pass the deadline punish incomplete challengers without
  voiding the responder/miner receipt reward path; no p2p, storage, or RPC wire format changed.
- Commit `ff3bf9a` (`Close isolated trace bisection timeouts`) pushed to `origin/main`.

### Iteration 182: Reward Sweep Boundary Naming

Feature capability: make the chain reward command surface explicit that delayed rewards are not paid by
maintenance release/sweep commands; non-voided proposer, receipt, challenge, and credit rewards stay
pending until the beneficiary submits `ClaimReward`.

Readiness requirements covered: `goal.md`/`upow.md` economics, delayed reward maturity, claim-owned
spendability, and the local/full boundary that rewards are chain-owned pending claims rather than adapter
workarounds.

Canonical owner: `chain::commands` owns delayed reward claim release, voided/prunable maintenance sweeps,
and spendable reward crediting.

Adapter callers: transaction submission, node/runtime command callers, tests, and status/explorer readers
that observe pending reward ledgers.

Old shortcut being removed: ambiguous internal helper names made the public `ReleaseMatured*` commands look
like a payout path, even though live matured rewards already remain pending for `ClaimReward`.

Regression test that proves the shortcut is gone:
`chain::tests::reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim` covers
non-voided proposer, receipt, challenge, and credit rewards and requires every `ReleaseMatured*` command to
leave those claims pending until `ClaimReward`.

Behavior with local synthetic block production disabled: unchanged; reward release is chain state only and
does not depend on synthetic production.

Behavior for producer and non-producer roles: unchanged; producer and non-producer block application both
preserve non-voided mature claims until the beneficiary claim command.

Structured evidence source: chain command events, pending reward ledgers, reward root/state root, and this
exec plan.

Finality source: unchanged; finalized/admitted block state may mature claims, but spendability still
requires `ClaimReward`.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC wire format changes.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to command naming/docs and
focused reward-boundary tests already in the chain suite.

Tests/checkers/docs to add or update: command enum docs, private helper names, this exec plan, and reward
boundary validation.

Narrow validation commands: `cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin if coverage-affecting tests change.

Expected observable evidence: release/sweep commands return no payout events for live matured claims,
pending ledgers remain populated, and `ClaimReward` emits the reward release plus claim events.

Out of scope: changing maturity heights, public/CUDA deployment evidence, VRF construction, or reward
amount formulas.

Split trigger: split only if helper renaming uncovers a runtime call site that still depends on immediate
crediting of non-voided rewards.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`
  passed.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 546 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Tarpaulin was not rerun because this iteration only added command documentation, renamed private helpers,
  and reused existing reward-boundary coverage without changing executable test count or coverage scope.
- Manual ownership-boundary review: no standalone verifier binary was used or added; reward claim release
  remains chain-owned, release/sweep commands are maintenance-only for voided/prunable ledgers, adapters do
  not credit rewards directly, and no p2p, storage, or RPC wire format changed.
- Commit `d185b02` (`Clarify reward sweep claim boundary`) pushed to `origin/main`.

### Iteration 181: Explorer WebSocket GraphExecution Evidence

Feature capability: the explorer WebSocket jobs and receipts views must expose `graph_execution` evidence
through the same JSON contract as TensorOp and LinearTrainingStep, preventing the browser-facing API from
silently regressing to a two-primitive view while chain/RPC/checker paths support first-class
GraphExecution.

Readiness requirements covered: `goal.md`/`upow.md` content-addressed Tensor IR graph language,
`mvp_spec.md` Node RPC/explorer WebSocket evidence, and local readiness API evidence for live graph work.

Canonical owner: `rpc::explorer` remains the typed renderer for jobs/receipts; the test now exercises the
existing GraphExecution branch with a real registered graph body and graph receipt.

Adapter callers: `/explorer/ws`, the standalone browser explorer, and deployment checkers consuming
explorer JSON.

Old shortcut being removed: WebSocket regression coverage could pass with only TensorOp and
LinearTrainingStep jobs/receipts even though GraphExecution is a first-class primitive in chain state,
codecs, settlement, checker output, and explorer HTTP rendering.

Regression test that proves the shortcut is gone:
`rpc::tests::websocket::explorer_websocket_views_cover_chain_collections_and_bad_commands` now creates a
registered graph body, a GraphExecution job, and a GraphExecution receipt, then requires WebSocket jobs and
receipts to include `graph_execution`.

Behavior with local synthetic block production disabled: unchanged; this is a read-surface regression over
chain state and does not synthesize work.

Behavior for producer and non-producer roles: unchanged. Any node exposing explorer WebSocket data renders
the local chain view through the same RPC renderer.

Structured evidence source: WebSocket JSON response, docs status/coverage, and this exec plan.

Finality source: unchanged; the test uses settled TensorOp evidence for the block view and direct receipt
view evidence for GraphExecution.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC wire format changes.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to RPC regression evidence and
docs/status alignment.

Tests/checkers/docs to add or update: WebSocket RPC test, `upow.md`, `mvp_spec.md`,
`coverage_matrix.md`, `implementation_status.md`, `tarpaulin_report.md`, and this compact exec plan.

Narrow validation commands: `cargo test -p tensor_vm
explorer_websocket_views_cover_chain_collections_and_bad_commands --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because the test count/coverage changes.

Expected observable evidence: the WebSocket jobs response includes `graph_execution`, the WebSocket
receipts response includes a GraphExecution receipt, and docs no longer show a two-variant
`PrimitiveType` contract.

Out of scope: changing graph execution semantics, block selection, checker thresholds, CUDA graph
execution, public deployment evidence, and new standalone verifier binaries.

Split trigger: split only if the WebSocket renderer itself cannot expose graph receipts without changing
RPC schemas or explorer UI contracts.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm explorer_websocket_views_cover_chain_collections_and_bad_commands --lib`
  passed after adding the graph WebSocket fixture.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 546 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 561
  instrumented tests, 84.58% line coverage, 22613/26736 lines covered.
- Manual ownership-boundary review: no standalone verifier binary was used or added; the change is limited
  to explorer WebSocket regression coverage and docs/status alignment, with no p2p, storage, or consensus
  wire-format changes.
- Commit `932c69c` (`Expose graph execution in explorer websocket`) pushed to `origin/main`.

## Recent Iterations

### Iteration 180: Local GraphExecution Checker Evidence

Feature capability: the local CPU readiness checker now fails unless live post-startup runtime exposes
generic GraphExecution evidence through explorer receipt and block-status surfaces used for TensorOp and
LinearTrainingStep.

Evidence and validation on June 22, 2026: first executable Gate 0; `cargo tarpaulin --version`
(`cargo-tarpaulin-tarpaulin 0.35.5`); shell syntax; focused deployment-doc regression; fmt/check/diff;
`cargo test -p tensor_vm --lib` (546 passed); release local-testnet; clippy; tarpaulin 84.54%
(22603/26736, 561 instrumented tests). No standalone verifier binary was used or added.

Commit `92e5602` (`Require graph execution checker evidence`) pushed to `origin/main`; follow-up metadata
commit `7a1c41b` (`Record graph checker evidence push`) pushed to `origin/main`.

### Iteration 179: Graph Receipt Verification Admitted-Op Coverage

Feature capability: local CPU graph execution verification has explicit receipt-scenario evidence for
every consensus-admitted frozen registry op, including arithmetic, reduction, transpose, unary
sign/absolute, and cast coverage.

Evidence and validation on June 22, 2026: first executable Gate 0; focused graph verification tests;
fmt/check/diff; `cargo test -p tensor_vm --lib` (545 passed); release local-testnet; clippy; tarpaulin
84.54% (22603/26736, 560 instrumented tests).

Commit `be4af33` (`Cover admitted graph verifier ops`) pushed to `origin/main`.

## Decision Log

- Gate 0 remains `cargo test -p tensor_vm local_testnet --release` and must be the first executable command
  on every resume before edits.
- Chain validation is the canonical owner for accepted randomness proof verification, typed proof metadata,
  state-rooted records, finalized beacon advancement, seed derivation, rewards, slashing, and challenge
  settlement. Runtime/checkers only observe or submit commands.
- Runtime may observe wall-clock public endpoint freshness only for locally fetched public drand. Chain
  validation/state own the accepted public drand anchor and deterministic chain-epoch round window.
- Reward delays, reveal holds, slashing, challenge settlement, and spendability are chain-owned pending
  claim/state transitions. Valid matured claims become spendable only through beneficiary `ClaimReward`.
- Bounded p2p/node payloads remain the only network wire surface for randomness, reveal records, graph
  jobs/receipts, and trace-bisection expectation/round/referee evidence.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, and deployed dispute evidence
  remain deployment gates, not local-completion claims.
- There is no standalone `tensorvm-verifier` binary. Validation uses shell checks, Rust tests, clippy,
  tarpaulin, and manual ownership-boundary review.

## Validation Evidence

- Current Iteration 182 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 182 validation passed on June 22, 2026: focused reward-boundary test; fmt/check/diff;
  `cargo test -p tensor_vm --lib` (546 passed); release local-testnet; clippy. Tarpaulin was not rerun
  because executable coverage scope did not change. Commit `d185b02` pushed to `origin/main`.

## Archive

- Iterations 177-178 (`1006b70`, `638ba58`, pushed `main` -> `main`): graph receipt payloads wait for
  missing registered program bodies; voided pre-inclusion receipt rewards prune directly after their
  explicit delayed hold without credit.
- Iterations 175-176 (`e45c876`, `b3c4bf9`, `b96debd`, `ee329bc`, pushed `main` -> `main`): conformance
  vector/profile identity guard and automatic block-state matured-reward pruning for auto-prunable voided
  receipt claims.
- Iterations 169-174 (`091142d`, `d88a14d`, `04a85d4`, `6901655`, `bbb3d28`, `8b94508`, `919f77f`,
  `b5bf0d9`, pushed `main` -> `main`): runtime trace-bisection session open, responder rounds,
  challenger expected roots, reward claim boundary regressions, and one-op referee witness generation.
- Iterations 158-168 (`6f6344a`, `713c6a4`, `2662d5a`, `f1372a4`, `02e288f`, `0487f77`, `e42ad44`,
  `e3af101`, `bfcefa7`, `a93676b`, `54912ce`, pushed `main` -> `main`): signed trace-bisection core,
  delayed block-check challenger rewards, bounded round/referee wire payloads, chain admission, pending
  queues, input-rooted trace openings, and one-op referee economics.
- Iterations 143-157: graph exact-op coverage, verified drand/network randomness, validator reveal proofs,
  finality-delayed proposer rewards, side-branch convergence, durable restart-rehydrated tensor artifacts,
  deployment preflight/evidence surfaces, rolling restart evidence, richer IR/Tier-B execution, delayed
  reward maturity, claim-owned spendability, audit/challenge reward holds, exact trace openings, and local
  CPU Docker proof evidence.
