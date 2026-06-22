# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 193 complete and pushed: Public CUDA Graph Execution Evidence Gate.
- Current status: post-run public evidence requires `cuda_verified_miner_count` to cover counted public
  miners and `cuda_graph_execution_receipts` to prove positive CUDA graph receipt coverage before a bundle
  can report `public_evidence_full_spec=true`; signed randomness-beacon summary evidence also requires an
  explicit run-coverage count match so an undercounted or overcounted beacon record set cannot satisfy
  independently checkable public evidence.
- Current blockers:
  - Public 7-day external deployment evidence and real CUDA miner/runtime evidence remain outside the
    local CPU proof.
  - Deployed full VRF construction and deployed commit-reveal lifecycle evidence remain open.
- Next action: continue deployed public/CUDA evidence work, especially deployed VRF/commit-reveal lifecycle
  evidence.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Current iteration first command `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection messages, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, delayed pending reward holds, and state-rooted proposer reward release tombstones | Continue Tier-C committee policy and deployed public-operator evidence |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 193: Public CUDA Graph Execution Evidence Gate

Feature capability: require full-spec public evidence to include positive CUDA graph-execution receipt
coverage that does not overclaim beyond checked or available public-run receipts.

Readiness requirements covered: `upow.md` and `mvp_spec.md` keep CUDA/deployed graph execution as a
full-spec blocker; public evidence must not promote a public run with CUDA-capable miners but no CUDA graph
receipt coverage to `public_evidence_full_spec=true`.

Canonical owner: `testnet::PublicTestnetEvidenceBundle::evaluate` owns public-run evidence admission;
`PublicTestnetRunEvidence::evaluate` owns run-level consistency checks; the manifest parser owns only
syntactic decoding of the new scalar.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment runbooks, and docs
consume the bundle report.

Old shortcut being removed: an otherwise complete public evidence bundle with CUDA-verified miners could
still satisfy `public_evidence_full_spec=true` without proving any CUDA-backed graph-execution receipt was
checked and available during the public run.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec` proves missing, zero, or
overcounted CUDA graph receipt evidence keeps otherwise complete evidence non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: `cuda_graph_execution_receipts`, `checked_receipts`, `available_receipts`, and
the `tvmd public evidence validate` report fields `cuda_graph_execution_evidence` and
`cuda_graph_execution_receipt_evidence`.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; public evidence manifest gains one scalar
field.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation,
and this is a single-writer public evidence/test/docs slice.

Parallelizable implementation workstreams: single-writer slice across public evidence structs/parser,
focused tests, report output, fixtures, and docs/status.

Tests/checkers/docs to add or update: public evidence bundle/manifest/report tests, checked evidence
manifests, public evidence docs/status/coverage/tarpaulin docs, and this exec plan.

Narrow validation commands:
`cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec --lib`,
`cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`, and
`cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, and tarpaulin
because public evidence tests/reportable coverage change.

Expected observable evidence: otherwise complete full-spec public evidence passes only when
`cuda_graph_execution_receipts > 0` and the count does not exceed checked or available receipts; missing,
zero, malformed, or overcounted graph evidence keeps `public_evidence_full_spec=false`.

Out of scope: generating real CUDA deployment artifacts, implementing CUDA kernels, or claiming a 7-day
public run in this workspace.

Split trigger: split if the manifest shape change requires unrelated CLI generation refactors or broad
tests expose existing fixture drift outside this evidence gate.

Validation evidence so far:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Focused evidence passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`, and
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.
- Broad gates passed:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  571 instrumented tests and 84.81% line coverage, 23169/27318 lines covered.
- Commit `a56a2c2` (`Require CUDA graph evidence for full spec`) pushed to `origin/main` on
  June 22, 2026.

### Iteration 192: Public Randomness Run Coverage Gate

Feature capability: require signed public randomness-beacon summary evidence to cover the full observed
run window before a public evidence bundle can become independently checkable or full-spec.

Readiness requirements covered: `mvp_spec.md` requires run-derived block, finality, randomness-beacon,
data-availability, invalid-work, and reward-settlement summary counts to match signed run counters exactly;
`upow.md` §10 requires unbiasable randomness evidence after receipt commitments.

Canonical owner: `testnet::PublicTestnetEvidenceBundle::evaluate` owns public-run evidence admission; the
manifest parser owns only syntactic record decoding and must not infer run coverage from a nonzero summary.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, and deployment runbooks
consume the bundle report.

Old shortcut being removed: a signed positive randomness-beacon summary could make
`has_randomness_beacon_evidence=true` even when its record count underreported or overreported the signed
run window's observed block count.

Regression test that proves the shortcut is gone: focused public evidence bundle coverage will resign
under-counted and over-counted randomness summaries and prove they no longer satisfy randomness evidence or
independently checkable evidence while the rest of the public bundle remains otherwise valid.

Behavior with local synthetic block production disabled: unchanged; this only validates public evidence
after a run.

Behavior for producer and non-producer roles: unchanged; the gate is evaluated at evidence-bundle level,
not in role runtime mutation.

Structured evidence source: `randomness_beacon_records`, `observed_blocks`, signed randomness summary root,
and manifest-level raw randomness records.

Finality source: unchanged; the run window and finality-history summaries remain separately signed.

Wire-size and codec boundary: no p2p/consensus wire changes; this tightens public evidence validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation,
and this is a single-writer public evidence/test/docs slice.

Parallelizable implementation workstreams: single-writer slice across bundle evaluation, focused tests,
and docs/status.

Tests/checkers/docs to add or update: public evidence bundle tests, public evidence docs/status/coverage,
tarpaulin report, and this exec plan.

Narrow validation commands:
`cargo test -p tensor_vm public_testnet_evidence_bundle_requires_randomness_records_for_full_run --lib`
and `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_randomness_records --lib`.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, and
tarpaulin because public evidence tests/reportable coverage change.

Expected observable evidence: signed randomness-beacon summaries only pass public evidence when their count
equals the signed observed block count; undercounts or overcounts fail even with valid signatures and
artifact locators.

Out of scope: generating real public drand/VRF deployment records or claiming a 7-day public run.

Split trigger: split if enforcing the count uncovers unrelated manifest generation drift outside public
randomness evidence.

Validation evidence:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Focused evidence passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_randomness_records_for_full_run --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_randomness_records --lib`, and
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_publication_and_audit_records --lib`.
- Broad gates passed:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  570 instrumented tests and 84.80% line coverage, 23154/27303 lines covered.
- Commit `6a50ad6` (`Require full-run randomness evidence`) pushed to `origin/main` on
  June 22, 2026.

### Iteration 191: Public Evidence CUDA Miner Gate

Feature capability: require independently checkable post-run evidence to include CUDA-verified counted
miner evidence before a public evidence bundle can satisfy the full-spec flag.

Readiness requirements covered: `mvp_spec.md` full-spec completion requires real CUDA/C++ kernels where GPU
mining is claimed, public preflight requires CUDA-ready miners, and public evidence must not promote a
CPU-only/unspecified public run to full-spec completion.

Canonical owner: `testnet::PublicTestnetEvidenceBundle::evaluate` and the public evidence manifest parser
own public-run evidence admission; CLI/report surfaces render the resulting evidence fields.

Adapter callers: `tvmd public evidence validate` and docs/deployment examples consume the manifest and
report output. They must not infer CUDA readiness from local preflight or from the mere presence of miners.

Old shortcut being removed: post-run evidence could satisfy `public_evidence_full_spec=true` through public
run/service/operator/raw-record gates without a public evidence field proving the counted miners were
CUDA-verified.

Regression test that proves the shortcut is gone: focused public evidence bundle and manifest tests will
show full-spec evidence fails when `cuda_verified_miner_count` is missing or lower than the counted miner
requirement, while relaxed local/public evidence remains non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this only affects public evidence
validation after a run.

Behavior for producer and non-producer roles: unchanged; counted public miner evidence is evaluated at the
bundle/report layer, not by role runtime mutation.

Structured evidence source: `cuda_verified_miner_count` in `PublicTestnetRunEvidence` and public evidence
manifest output/report fields.

Finality source: unchanged; signed run-window and finality-history records remain the finality evidence.

Wire-size and codec boundary: no p2p/consensus wire changes; public evidence manifest gains one scalar
field.

Parallel subagents to run: none. The user corrected the nonexistent verifier assumption, and the decision
log says not to spawn subagents without explicit delegation.

Parallelizable implementation workstreams: single-writer slice across public evidence structs/parser,
fixtures/tests, and docs/status.

Tests/checkers/docs to add or update: public evidence bundle tests, manifest parser/report tests, checked
public evidence example manifests, coverage/status/tarpaulin docs, and this exec plan.

Narrow validation commands:
`cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec --lib`,
`cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`, and
`cargo test -p tensor_vm docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec --lib`.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, and
tarpaulin because public evidence tests/reportable coverage change.

Expected observable evidence: a full-spec public evidence fixture passes only when its CUDA-verified miner
count covers the counted miners; removing or undercounting that field keeps all other public evidence true
but makes `public_evidence_full_spec=false`.

Out of scope: generating real CUDA deployment artifacts or claiming a 7-day public run in this workspace.

Split trigger: split if the manifest format change requires unrelated CLI generation refactors or if broad
tests expose existing public evidence fixture drift outside this gate.

Validation evidence:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Focused evidence passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`,
  `cargo test -p tensor_vm docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec --lib`,
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`, and
  `cargo test -p tensor_vm public_testnet_evidence_manifest_rejects_malformed_input --lib`.
- Broad gates passed:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  568 instrumented tests and 84.80% line coverage, 23153/27302 lines covered.
- Commit `a0697dc` (`Require CUDA miner evidence for full spec`) pushed to `origin/main` on
  June 22, 2026.

### Iteration 190: Proposer Reward Delay Tombstone

Feature capability: replace the finalized proposer reward height-cutoff workaround with explicit,
state-rooted released proposer reward block tracking.

Readiness requirements covered: `goal.md`/`upow.md` delayed reward maturity, claim-owned spendability, and
avoiding adapter/workaround reward release paths.

Canonical owner: `chain::blocks` materializes finalized proposer rewards; `chain::commands` releases
claims through beneficiary `ClaimReward`; `ChainState` owns pending and released proposer reward state.

Adapter callers: runtime/status/RPC/explorer surfaces consume chain state and must not infer proposer
reward finality from block height cutoffs.

Old shortcut being removed: `materialize_finalized_proposer_rewards` skipped creating a pending proposer
reward when `state.height > claimable_at_height`, which prevented rematerialization after claim but also
made late-finalized rewards disappear instead of becoming delayed claims.

Regression tests that prove the shortcut is gone:
`late_finalized_proposer_reward_materializes_as_delayed_claim_once`, proposer-reward focused tests, and the
chain-state snapshot roundtrip.

Behavior with local synthetic block production disabled: unchanged; finalized rewarded blocks use the same
chain-owned pending/released proposer reward ledgers.

Behavior for producer and non-producer roles: producer and peers recompute the same reward/state roots
because released proposer reward block heights are included in state snapshots and roots.

Structured evidence source: `pending_proposer_rewards`, `released_proposer_reward_blocks`, `reward_root`,
`state_root`, and chain-state snapshot roundtrip.

Finality source: unchanged; proposer rewards materialize after block finality, then become spendable only
through beneficiary `ClaimReward`.

Wire-size and codec boundary: no p2p/RPC wire format changes; chain-state snapshot encoding changes.

Parallel subagents to run: none. The available subagent tool policy requires explicit user delegation; this
slice is confined to chain reward state, tests, and docs/status alignment.

Narrow validation commands:
`cargo test -p tensor_vm late_finalized_proposer_reward_materializes_as_delayed_claim_once --lib`,
`cargo test -p tensor_vm proposer_reward --lib`,
`cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib`,
`cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`,
and `cargo test -p tensor_vm block_transition_preserves_matured_rewards_until_claim --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, workspace
release tests, clippy, and tarpaulin because reward tests and reportable coverage change.

Expected observable evidence: a rewarded block finalized after its claimable height still creates a
pending proposer reward claim, `ClaimReward` releases it, and later materialization does not recreate that
block's claim.

Validation evidence:

- Required first executable on this resume, before implementation:
  `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026.
- Narrow evidence passed so far:
  `cargo test -p tensor_vm late_finalized_proposer_reward_materializes_as_delayed_claim_once --lib`,
  `cargo test -p tensor_vm proposer_reward --lib`,
  `cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering --lib`,
  `cargo test -p tensor_vm reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim --lib`,
  and `cargo test -p tensor_vm block_transition_preserves_matured_rewards_until_claim --lib`.
- Broad gates passed:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Tarpaulin passed:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  567 instrumented tests and 84.80% line coverage, 23142/27291 lines covered.
- Commit `b1b368b` (`Delay proposer rewards without height workaround`) pushed to `origin/main` on
  June 22, 2026. Metadata commit `5659058` (`Record proposer reward delay push`) was also pushed to
  `origin/main` on June 22, 2026.

## Recent Completed Iterations

- Iteration 193: Public CUDA Graph Execution Evidence Gate. Commit `a56a2c2`
  (`Require CUDA graph evidence for full spec`) pushed to `origin/main` on June 22, 2026.
- Iteration 192: Public Randomness Run Coverage Gate. Commit `6a50ad6`
  (`Require full-run randomness evidence`) pushed to `origin/main` on June 22, 2026.
- Iteration 191: Public Evidence CUDA Miner Gate. Commit `a0697dc`
  (`Require CUDA miner evidence for full spec`) pushed to `origin/main` on June 22, 2026.
- Iteration 190: Proposer Reward Delay Tombstone. Commit `b1b368b` (`Delay proposer rewards without height
  workaround`) pushed to `origin/main` on June 22, 2026; metadata commit `5659058` recorded/pushed the
  evidence anchor.
- Iteration 189: Public Evidence Raw Chain History Record Gate. Commit `8f84062`
  (`Require raw public chain history evidence`) pushed to `origin/main` on June 22, 2026; metadata commit
  `581f87a` recorded/pushed the evidence anchor.
- Iteration 188: Public Evidence Raw Operational Record Gate. Commit `e4c599f`
  (`Require raw public operational evidence`) pushed to `origin/main` on June 22, 2026.
- Iteration 187: Chain-Owned Verifier Bandwidth Evidence. Commit history before Iteration 188 contains the
  detailed anchor; local verifier-bandwidth evidence is implemented and documented.
- Iteration 186: Public Randomness Evidence Raw-Record Gate.
- Iteration 185: Mixed-Dtype Conformance Vector Coverage.
- Iteration 184: Trace-Bisection DoS Admission Bounds.
- Iteration 183: Isolated Trace-Bisection Timeout Policy.
- Iteration 182: Reward Sweep Boundary Naming.

## Decision Log

- `tensorvm-verifier` is not a repository binary. Validation uses tests, clippy, tarpaulin, and manual
  verifier-style review only.
- Do not spawn subagents unless the user explicitly asks for delegation.
- Public/CUDA/deployed evidence remains blocked until real external infrastructure and CUDA kernels exist.
- Reward claims remain delayed and chain-owned; valid matured claims become spendable only through
  beneficiary `ClaimReward`, while voided/prunable claims may be swept without credit.

## Validation Evidence

- Current Iteration 193 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 193 focused validation passed on June 22, 2026:
  `public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec`,
  `public_testnet_evidence_manifest_parses_into_bundle`, and
  `validate_public_evidence_manifest_reports_default_criteria_status`.
- Current Iteration 193 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 193 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  571 instrumented tests and 84.81% line coverage, 23169/27318 lines covered.
- Commit `a56a2c2` pushed to `origin/main` on June 22, 2026.
- Current Iteration 192 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 192 focused validation passed on June 22, 2026:
  `public_testnet_evidence_bundle_requires_randomness_records_for_full_run`,
  `public_testnet_evidence_bundle_requires_raw_randomness_records`, and
  `public_testnet_evidence_bundle_requires_publication_and_audit_records`.
- Current Iteration 192 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 192 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  570 instrumented tests and 84.80% line coverage, 23154/27303 lines covered.
- Commit `6a50ad6` pushed to `origin/main` on June 22, 2026.
- Current Iteration 191 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 191 focused validation passed on June 22, 2026:
  `public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec`,
  `public_testnet_evidence_manifest_parses_into_bundle`,
  `docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec`,
  `validate_public_evidence_manifest_reports_default_criteria_status`, and
  `public_testnet_evidence_manifest_rejects_malformed_input`.
- Current Iteration 191 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 191 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  568 instrumented tests and 84.80% line coverage, 23153/27302 lines covered.
- Commit `a0697dc` pushed to `origin/main` on June 22, 2026.
- Current Iteration 190 first executable Gate 0 passed on June 22, 2026.
- Current Iteration 190 focused validation passed on June 22, 2026:
  `late_finalized_proposer_reward_materializes_as_delayed_claim_once`, `proposer_reward`,
  `chain_state_store_roundtrips_full_chain_and_detects_tampering`,
  `reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim`, and
  `block_transition_preserves_matured_rewards_until_claim`.
- Current Iteration 190 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 190 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  567 instrumented tests and 84.80% line coverage, 23142/27291 lines covered.
- Commits `b1b368b` and `5659058` pushed to `origin/main` on June 22, 2026.

## Archive

Older detailed iteration notes were compacted on June 22, 2026 after the plan exceeded 300 lines. Durable
commit anchors and status are preserved above; detailed historical notes remain available in git history.
