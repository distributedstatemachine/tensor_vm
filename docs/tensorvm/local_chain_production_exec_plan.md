# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 188 complete: Public Evidence Raw Operational Record Gate.
- Current status: full-spec public evidence evaluation now requires raw data-availability, invalid-work,
  and reward-settlement operational records whose aggregate roots match the signed public evidence
  summaries. The validator runtime also delays empty fallback proposer rewards while a local synthetic
  job producer has no settled receipts, and VRF key registration is scoped to real attestation/proposal
  work so failed remote tensor fetch status updates do not persist chain state.
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
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates from live job/receipt shapes | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 188: Public Evidence Raw Operational Record Gate

Feature capability: require full-spec public evidence bundles to include raw accepted data-availability,
invalid-work, and reward-settlement records whose aggregate roots match their signed summaries, instead of
letting signed counters and artifact locators alone satisfy those operational gates.

Readiness requirements covered: `goal.md`/`mvp_spec.md` Acceptance Criterion 13 independently checkable
public-run evidence, raw supporting records behind summary roots, and the local/full boundary that no
public run has happened yet.

Canonical owner: `testnet` public evidence bundle evaluation and manifest parsing own public-run evidence
claims.

Adapter callers: `tvmd public evidence validate` and docs/deploy examples consume the parsed
`PublicTestnetEvidenceBundle`; adapters may not promote summary-only operational evidence to full-spec
completion.

Old shortcut being removed: full-spec evaluation could accept signed data-availability, invalid-work, and
reward-settlement summary roots without inspecting raw manifest-level records for those operational
evidence kinds.

Regression tests that prove the shortcut is gone: add public evidence bundle and manifest tests proving
full-spec evidence fails without raw operational records, fails when they do not aggregate to the signed
roots, and still parses the documented pending manifests as non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this is post-run evidence validation.

Behavior for producer and non-producer roles: unchanged; the validation concerns public evidence bundles,
not runtime role logic.

Structured evidence source: repeated `data_availability_measurement`, `invalid_work_rejection`, and
`reward_settlement` manifest lines, their aggregate roots, and the signed public evidence bundle report.

Finality source: unchanged; this validates post-run evidence for finalized public runs.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC wire format changes.

Parallel subagents to run: none. The available subagent tool policy requires explicit user delegation; this
slice is confined to public evidence parsing/evaluation, tests, and docs/status alignment.

Parallelizable implementation workstreams: not split; one writer owns the manifest/evidence structs and
fixture updates.

Tests/checkers/docs to add or update: public evidence bundle/manifest tests, public evidence docs,
`coverage_matrix.md`, `implementation_status.md`, `tarpaulin_report.md` if coverage changes, and this exec
plan.

Narrow validation commands: `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib`
and `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because evidence tests and reportable coverage change.

Expected observable evidence: `public_evidence_full_spec` remains false unless raw DA, invalid-work, and
reward-settlement records are present in the manifest and aggregate to their signed summary roots.

Validation evidence:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Narrow evidence gates passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires --lib`, and
  `cargo test -p tensor_vm public_testnet_evidence_manifest --lib`.
- Release smoke gates passed after delaying empty fallback proposer rewards for local synthetic producers:
  `cargo test -p tensor_vm --test tvmd_cli validator_run_with_synthetic_job_producer_publishes_jobs_without_empty_fallback_blocks --release -- --nocapture`,
  `cargo test -p tensor_vm --test tvmd_runtime runtime_persistence::validator_remote_tensor_fetch_status_does_not_persist_chain --release -- --nocapture`, and
  `cargo test -p tensor_vm --test tvmd_runtime runtime_roles::selected_validator_proposer_emits_idle_fallback_block --release -- --nocapture`.
- Broad gate passed:
  `cargo fmt --all -- --check && git diff --check && cargo test -p tensor_vm --lib && cargo test -p tensor_vm local_testnet --release && cargo test --workspace --release && cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  565 instrumented tests and 84.74% line coverage, 23056/27207 lines covered.
- Commit `e4c599f` (`Require raw public operational evidence`) pushed to `origin/main` on June 22, 2026.

Out of scope: generating real public operational evidence, CUDA evidence, changing runtime evidence
admission beyond the local synthetic-producer fallback reward delay, or changing local CPU checker
behavior.

Split trigger: split only if manifest compatibility requires a larger evidence schema migration.

### Iteration 187: Chain-Owned Verifier Bandwidth Evidence

Feature capability: report verifier bandwidth and verification-to-execution evidence from live job and
receipt shapes through the chain-owned status/explorer surfaces.

Readiness requirements covered: `goal.md`/`mvp_spec.md` bounded verifier bandwidth per job shape,
Acceptance Criterion 11 cheaper-than-recompute evidence, and local telemetry evidence for active verifier
bandwidth.

Canonical owner: `ChainState` owns the computed verifier-bandwidth summary; service status and explorer RPC
only render it.

Adapter callers: `tvmd node status`, explorer overview JSON/WebSocket, and local/public readiness checkers
may consume the rendered fields but must not recompute protocol evidence independently.

Old shortcut being removed: verifier bandwidth was available only as telemetry estimates and study tests,
not as a chain-owned evidence summary tied to live job and receipt shapes.

Regression tests that prove the shortcut is gone:
`verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes`,
`service_status_exports_validator_audit_economic_calibration`,
`explorer_overview_exports_validator_audit_economic_calibration`, and
`tensor_vm_explorer::tests::explorer_json_and_shell_include_live_websocket_contract`.

Behavior with local synthetic block production disabled: unchanged; the evidence is derived from chain
state after jobs/receipts are admitted.

Behavior for producer and non-producer roles: unchanged; all nodes rendering the same chain state report
the same verifier-bandwidth evidence.

Structured evidence source: `ChainState::verifier_bandwidth_evidence`, status key-value fields, and
explorer overview JSON.

Finality source: unchanged; this is verifier-cost evidence, not block finality logic.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC payload codec changes.

Parallel subagents to run: none. The slice is confined to chain evidence, status/explorer rendering, tests,
and docs.

Parallelizable implementation workstreams: not split; one writer owns the state/status/explorer type
surface.

Tests/checkers/docs to add or update: chain/status/RPC/explorer tests, `coverage_matrix.md`,
`implementation_status.md`, `tarpaulin_report.md` if coverage changes, and this exec plan.

Narrow validation commands: `cargo test -p tensor_vm verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes --lib`,
`cargo test -p tensor_vm service_status_exports_validator_audit_economic_calibration --lib`,
`cargo test -p tensor_vm explorer_overview_exports_validator_audit_economic_calibration --lib`, and
`cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because coverage and explorer JSON changed.

Expected observable evidence: status and explorer overview expose live job/receipt counts, estimated
verification bytes, estimated per-validator bandwidth, and verification-to-execution bps for TensorOp,
LinearTrainingStep, and GraphExecution.

Out of scope: CUDA performance evidence, public deployed bandwidth measurements, changing verifier
semantics, or claiming full-spec public evidence.

Split trigger: split only if verifier bandwidth needs to become a signed public evidence record format.

Validation evidence on June 22, 2026:
- First executable Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused checks passed: `cargo test -p tensor_vm verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes --lib`,
  `cargo test -p tensor_vm service_status_exports_validator_audit_economic_calibration --lib`,
  `cargo test -p tensor_vm explorer_overview_exports_validator_audit_economic_calibration --lib`, and
  `cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract --lib`.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 549 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release library local-testnet tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 564
  instrumented tests, 84.74% workspace line coverage, 22957/27091 lines covered.
- Manual ownership-boundary review: verifier bandwidth evidence is computed in `ChainState` and only
  rendered by status/RPC adapters; no verifier semantics, p2p/storage codec, CUDA path, or public evidence
  claim changed.
- Commit `c6a44e3` (`Expose verifier bandwidth evidence`) pushed to `origin/main`.

### Iteration 186: Public Randomness Evidence Raw-Record Gate

Feature capability: require full-spec public evidence bundles to include raw accepted public randomness
records, so a signed randomness summary alone or a local deterministic fixture cannot satisfy the §10
public randomness gate.

Readiness requirements covered: `goal.md`/`upow.md` §10 unbiasable randomness, public evidence
gatekeeping, and the local/full boundary that deployed public randomness evidence is still absent.

Canonical owner: `testnet` public evidence evaluation owns public-run evidence claims and manifest parsing.

Adapter callers: `tvmd public evidence validate` and docs/examples consume the parsed
`PublicTestnetEvidenceBundle`; adapters may not promote summary-only randomness evidence to full-spec
completion.

Old shortcut being removed: full-spec evaluation accepted any correctly signed randomness summary root and
record count without inspecting raw record kinds, so a local deterministic fixture summary could look like
public randomness evidence at the manifest layer.

Regression tests that prove the shortcut is gone: public evidence bundle tests will keep relaxed
independently-checkable summaries working while proving full-spec evidence fails without raw public
randomness records and fails when those records are local fixtures.

Behavior with local synthetic block production disabled: unchanged; this is post-run evidence validation.

Behavior for producer and non-producer roles: unchanged; the validation concerns public evidence bundles,
not runtime role logic.

Structured evidence source: repeated `randomness_beacon_record` manifest lines, their aggregate root, the
signed randomness summary, and the public evidence bundle report.

Finality source: unchanged; this validates evidence for finalized public runs rather than producing blocks.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC wire format changes.

Parallel subagents to run: none. The slice is confined to public evidence parsing/evaluation, tests, and
docs/status alignment.

Parallelizable implementation workstreams: not split; one writer owns the manifest/evidence structs and
fixture updates.

Tests/checkers/docs to add or update: public evidence bundle/manifest tests, public evidence docs,
`coverage_matrix.md`, `implementation_status.md`, `tarpaulin_report.md` if coverage changes, and this exec
plan.

Narrow validation commands: `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_publication_and_audit_records --lib`
and `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because evidence tests and reportable coverage change.

Expected observable evidence: full-spec public evidence remains true only with accepted `drand-v1` or
`validator-vrf-v1` raw randomness records whose aggregate root matches the signed summary; local fixture
randomness records no longer satisfy full-spec evidence.

Out of scope: generating public drand evidence, CUDA evidence, changing runtime randomness admission, or
changing local CPU checker behavior.

Split trigger: split only if manifest backward compatibility requires a larger evidence schema migration.

Validation evidence on June 22, 2026:
- First executable Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused public-evidence checks passed:
  `public_testnet_evidence_bundle_requires_publication_and_audit_records`,
  `public_testnet_evidence_manifest_parses_into_bundle`, `public_testnet_evidence`, and
  `public_evidence_record`.
- Manifest malformed-input coverage now rejects unknown `randomness_beacon_record` proof kinds and
  statuses.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 548 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release library local-testnet tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 563
  instrumented tests, 84.62% workspace line coverage, 22726/26858 lines covered.
- Manual ownership-boundary review: no runtime randomness admission changed and no public evidence was
  generated; this is a manifest/evidence validation gate. No standalone verifier binary exists or was
  added; validation used Rust tests, shell checks, clippy, tarpaulin, and manual ownership-boundary review.

Commit `4e3f97a` (`Gate full spec evidence on public randomness records`) pushed to `origin/main`.

### Iteration 185: Mixed-Dtype Conformance Vector Coverage

Feature capability: strengthen the canonical `F_p` conformance suite with additional mixed dtype/scale
vectors for already-admitted exact ops, without changing consensus semantics or admitting new Tier-C
vocabulary.

Readiness requirements covered: `goal.md`/`upow.md` determinism contract, per-op conformance vectors, and
the local/full boundary that CPU reference evidence is not CUDA evidence.

Canonical owner: `conformance` owns the vector corpus, suite hash, CPU reference profile, and receipt
gating evidence.

Adapter callers: receipt validation and runtime reporting paths that consume `ConformanceProfile`; no
adapter may bypass the suite hash or per-op pass set.

Old shortcut being removed: the suite covered the admitted op spelling surface, but mixed dtype/scale
coverage still relied on a small set of vectors around fixed-point arithmetic, quantization, equality, and
selection.

Regression tests that prove the shortcut is gone:
`conformance::tests::conformance_vectors_are_stable_and_cover_current_ops` now requires the new fixed-scale
comparison and int8 selection vectors, while `conformance::tests::cpu_reference_passes_all_vectors` proves
the CPU reference executes the enlarged suite.

Behavior with local synthetic block production disabled: unchanged; conformance vectors are local
verification inputs and do not synthesize chain work.

Behavior for producer and non-producer roles: unchanged; receipt verification uses the same suite hash and
profile regardless of role.

Structured evidence source: vector IDs, suite hash, CPU reference profile, receipt conformance gate tests,
and this exec plan.

Finality source: unchanged; this is pre-admission deterministic execution evidence.

Wire-size and codec boundary: unchanged; no p2p/storage/RPC wire format changes.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to conformance vectors,
focused tests, and docs/status alignment.

Tests/checkers/docs to add or update: conformance vectors/tests, `upow.md`, `coverage_matrix.md`,
`implementation_status.md`, `tarpaulin_report.md`, and this exec plan.

Narrow validation commands: `cargo test -p tensor_vm conformance_vectors_are_stable_and_cover_current_ops --lib`
and `cargo test -p tensor_vm cpu_reference_passes_all_vectors --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin because the vector corpus and suite hash change.

Expected observable evidence: suite hash changes, CPU reference passes the enlarged vector set, required
op profile gating still covers all admitted ops, and CUDA remains explicitly unproven in default builds.

Out of scope: CUDA pass evidence, Tier-C/transcendental admission, changing comparison ordering semantics,
and new validation binaries or runtime surfaces.

Split trigger: split only if a new vector reveals an execution semantic mismatch requiring IR/tensor
behavior changes.

Validation evidence on June 22, 2026:
- First executable Gate 0: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused conformance checks passed: `conformance_vectors_are_stable_and_cover_current_ops`,
  `cpu_reference_passes_all_vectors`, and `required_conformance_gates_current_jobs`.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 548 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release library local-testnet tests plus
  `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 563
  instrumented tests, 84.61% workspace line coverage, 22662/26784 lines covered.
- Manual ownership-boundary review: no standalone verifier binary exists or was added; validation used
  Rust tests, shell checks, clippy, tarpaulin, and receipt conformance gate tests.

Commit `a9991aa` (`Expand mixed dtype conformance vectors`) pushed to `origin/main`.

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
new validation binaries.

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
- Commit `c8b76fc` (`Bound trace bisection admission`) pushed to `origin/main`.

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
execution, public deployment evidence, and new validation binaries.

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
