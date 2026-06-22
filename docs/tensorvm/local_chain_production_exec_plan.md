# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 197 validation complete; commit/push pending: Public VRF Lifecycle Raw-Record Evidence Gate.
- Current status: post-run public evidence requires `cuda_verified_miner_count` to cover counted public
  miners, positive `cuda_graph_execution_receipts` within checked/available receipt counts, and
  `validator_vrf_lifecycle_records` covering checked receipts exactly. This iteration adds signed
  validator-VRF-lifecycle summary roots and matching raw lifecycle records so the scalar count must be
  derived from independently checkable deployed commit-reveal records before `public_evidence_full_spec=true`
  can pass.
- Current blockers:
  - Public 7-day external deployment evidence and real CUDA miner/runtime evidence remain outside the
    local CPU proof.
  - Real deployed full VRF construction and public commit-reveal lifecycle artifacts remain open.
- Next action: commit and push Iteration 197, then continue real public VRF/CUDA/deployed-run artifact work.

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
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates, and public full-spec evidence now requires checked-receipt VRF lifecycle coverage | Add real deployed full VRF construction artifacts and public lifecycle records |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 197: Public VRF Lifecycle Raw-Record Evidence Gate

Feature capability: require full-spec public evidence to include signed validator VRF lifecycle summary
evidence and raw deployed lifecycle records that aggregate to the signed lifecycle root.

Readiness requirements covered: `upow.md` §10 and `mvp_spec.md` require deployed validator VRF
commit-to-reveal lifecycle evidence for checked receipts, not only a copied scalar lifecycle count.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec evidence admission; the manifest
parser owns typed raw lifecycle syntax; `tvmd public evidence record ...` owns signed summary/artifact
generation from raw lifecycle files.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment runbooks, and
public evidence docs consume the bundle report.

Old shortcut being removed: an otherwise complete full-spec public evidence bundle could pass with
`validator_vrf_lifecycle_records` set to the checked receipt count but without raw lifecycle records or a
signed lifecycle summary root behind that count.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec` will prove
missing lifecycle summaries, missing raw lifecycle records, and mismatched lifecycle roots keep otherwise
complete evidence non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: `validator_vrf_lifecycle_records`, `validator_vrf_lifecycle_root`,
`validator_vrf_lifecycle_signature`, and typed raw
`validator_vrf_lifecycle=<receipt-root>,<validator-id>,<beacon-round>,committed|revealed,<block>` records.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain separate
gates.

Wire-size and codec boundary: no p2p/consensus wire changes; public evidence manifest and record-kind CLI
gain one supporting-record kind.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle/manifest/report tests, record-file summary
tests, checked evidence manifests, public evidence docs/status/coverage/tarpaulin docs, and this exec plan.

Narrow validation commands: focused public evidence raw lifecycle tests, manifest parser tests, and record
summary-file tests.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`.

Expected observable evidence: otherwise complete full-spec public evidence passes only when the signed
validator VRF lifecycle summary count/root matches raw lifecycle records covering the checked receipts.

Out of scope: generating real deployed validator VRF artifacts, replacing the local chain VRF
implementation, or claiming a 7-day public run in this workspace.

Split trigger: split if adding the supporting-record kind requires unrelated deployment-template or
process-runner refactors beyond fixture updates.

Validation evidence, June 22, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused public-evidence validation: `cargo test -p tensor_vm public_testnet_evidence --lib` and
  `cargo test -p tensor_vm public_evidence_record --lib` passed, including
  `public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec`.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Broad validation: `cargo test --workspace --release` passed.
- Lint validation: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Coverage validation: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`
  passed, 574 instrumented tests, 84.77% line coverage, 23329/27520 lines covered.
- Commit: pending.
- Push: pending.

### Iteration 196: Public Detection Measurement Evidence Gate

Feature capability: require full-spec public evidence to include positive signed deployed
detection-measurement records and raw detection records that aggregate to the signed detection summary.

Readiness requirements covered: the public economics/slashing invariant must be backed by deployed-run
detection measurements, not only local calculator estimates or signed reward-settlement summaries.

Canonical owner: `PublicTestnetRunEvidence::evaluate` owns run-level positive detection-measurement
evidence; `PublicTestnetEvidenceBundle::evaluate` owns independently checkable/full-spec admission and raw
record root matching; the manifest parser and `tvmd public evidence record ...` commands own typed
syntax/signing.

Old shortcut being removed: otherwise complete full-spec public evidence could pass without any deployed
detection-measurement record set behind the economics/detection claims.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec` proves missing
run counts, missing signed summaries, missing raw detection records, and mismatched raw detection roots keep
otherwise complete evidence non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; this iteration only changes public evidence
validation and CLI/documented manifest generation.

Structured evidence source: `detection_measurement_records`,
`detection_measurement_root`, `detection_measurement_signature`, and typed raw
`detection_measurement=<mechanism>,<subject-root>,<sample-count>,<detected-count>,<block>` records.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Validation passed on June 22, 2026: first executable Gate 0 was attempted first and exposed the expected
in-progress compile gap before the new record-kind CLI argument was wired; after implementation,
`cargo fmt --all -- --check`, `git diff --check`,
`public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec`,
`public_testnet_evidence_manifest_parses_into_bundle`,
`validate_public_evidence_manifest_reports_default_criteria_status`,
`execute_public_evidence_record_reports_outputs`, `deployment_docs`, `cargo test -p tensor_vm --lib`,
`cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed.

Tarpaulin passed with 573 instrumented tests and 84.81% line coverage, 23261/27428 lines covered.

Commit `a65f247` (`Require deployed detection evidence for full spec`) prepared on June 22, 2026.

### Iteration 195: Reward Delay Spec Alignment

Feature capability: remove the stale miner reward-curve wording from `upow.md` and state the implemented
chain-owned delayed reward plus delayed TensorWork activation rule directly.

Readiness requirements covered: rewards must be distributed by verified settled TensorWork, but spendable
credit must remain delayed until canonical inclusion, challenge/audit holds, maturity, and beneficiary
`ClaimReward`.

Canonical owner: existing chain reward ledgers own this behavior through pending receipt, proposer,
challenge, and credit claims. This iteration changes documentation only because the implementation and
focused reward tests already enforce the delayed path.

Old shortcut being removed: the spec no longer describes a diminishing-return `sqrt(miner_epoch_twu)`
reward curve as the miner reward defense. The implemented defense is delayed, voidable reward escrow plus
delayed TensorWork activation.

Validation passed on June 22, 2026: first executable Gate 0
`cargo test -p tensor_vm local_testnet --release`, focused reward-delay regressions
`generic_credit_rewards_claim_only_after_maturity`,
`reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim`, and
`miner_rewards_delay_tensorwork_activation_until_reward_release`, plus `cargo fmt --all -- --check` and
`git diff --check`.

Coverage impact: docs-only; no coverage change, so tarpaulin was not rerun for this iteration. Installed
coverage tool confirmed as `cargo-tarpaulin-tarpaulin 0.35.5`.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

### Iteration 194: Public VRF Lifecycle Evidence Gate

Feature capability: require full-spec public evidence to include receipt-level validator VRF
commit→reveal lifecycle coverage before a bundle can report `public_evidence_full_spec=true`.

Readiness requirements covered: `upow.md` §10 and `mvp_spec.md` require validation randomness to be
unbiasable after receipt roots are committed and require a deployed commit-reveal or VRF lifecycle, not
only a positive randomness-beacon summary.

Canonical owner: `testnet::PublicTestnetRunEvidence::evaluate` owns run-level evidence consistency;
`PublicTestnetEvidenceBundle::evaluate` owns full-spec admission; the manifest parser owns only syntactic
decoding of the new scalar.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment runbooks, and docs
consume the bundle report.

Old shortcut being removed: an otherwise complete full-spec public evidence bundle with accepted public
`validator-vrf-v1` randomness records could still set `public_evidence_full_spec=true` without proving that
checked receipts had deployed commit→reveal lifecycle records.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec` proves missing, zero,
undercounted, and overcounted lifecycle evidence keeps otherwise complete evidence non-full-spec.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: `validator_vrf_lifecycle_records`, `checked_receipts`, and the
`tvmd public evidence validate` report fields `validator_vrf_lifecycle_evidence`,
`validator_vrf_lifecycle_records`, and `validator_vrf_lifecycle_record_evidence`.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain separate
gates.

Wire-size and codec boundary: no p2p/consensus wire changes; public evidence manifest gains one scalar
field.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation,
and this is a single-writer public evidence/test/docs slice.

Tests/checkers/docs updated: public evidence bundle/manifest/report tests, checked evidence manifests,
process-level generated evidence manifest test, public evidence docs/status/coverage/tarpaulin docs, and
this exec plan.

Narrow validation passed:
`cargo test -p tensor_vm public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec --lib`,
`cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`, and
`cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.

Broad validation passed: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, and `cargo clippy --workspace --all-targets -- -D warnings`.

Tarpaulin passed:
`cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with 572 instrumented
tests and 84.82% line coverage, 23183/27332 lines covered.

Commit `5a101b5` (`Require VRF lifecycle evidence for full spec`) pushed to `origin/main` on
June 22, 2026.

Expected observable evidence: otherwise complete full-spec public evidence passes only when
`validator_vrf_lifecycle_records` covers every checked receipt exactly; missing, zero, malformed,
undercounted, or overcounted lifecycle evidence keeps `public_evidence_full_spec=false`.

Out of scope: generating real deployed validator VRF artifacts, replacing the local chain VRF
implementation, or claiming a 7-day public run in this workspace.

Split trigger: split if the manifest shape change requires unrelated CLI generation refactors or broad
tests expose existing fixture drift outside this evidence gate. The only broad-test drift found was the
process-level generated evidence manifest fixture, now updated and covered.

## Recent Iterations

### Iteration 193: Public CUDA Graph Execution Evidence Gate

Feature capability: require full-spec public evidence to include positive CUDA graph-execution receipt
coverage that does not overclaim beyond checked or available public-run receipts.

Evidence: `public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec` proves missing,
zero, or overcounted CUDA graph receipt evidence keeps otherwise complete evidence non-full-spec. Manifest
parser/report tests and checked docs/examples include `cuda_graph_execution_receipts`.

Validation passed: focused public evidence tests, `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`,
`cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`, and
tarpaulin with 571 instrumented tests and 84.81% line coverage, 23169/27318 lines covered.

Commit `a56a2c2` (`Require CUDA graph evidence for full spec`) pushed to `origin/main` on June 22, 2026.
Metadata commit `cb20432` (`Record CUDA graph evidence gate push`) also pushed on June 22, 2026.

### Iteration 192: Public Randomness Run Coverage Gate

Feature capability: require signed public randomness-beacon summary evidence to cover the full observed
run window before a public evidence bundle can become independently checkable or full-spec.

Evidence: `public_testnet_evidence_bundle_requires_randomness_records_for_full_run` proves undercounted or
overcounted randomness summaries fail randomness evidence and independently checkable evidence;
`public_testnet_evidence_bundle_requires_raw_randomness_records` proves local deterministic fixture records
cannot satisfy the full-spec public randomness gate.

Commit `6a50ad6` (`Require full-run randomness evidence`) pushed to `origin/main` on June 22, 2026.

## Decision Log

- `tensorvm-verifier` is not a repository binary. Validation uses tests, clippy, tarpaulin, and manual
  verifier-style review only.
- Do not spawn subagents unless the user explicitly asks for delegation.
- Public/CUDA/deployed evidence remains blocked until real external infrastructure and CUDA kernels exist.
- Reward claims remain delayed and chain-owned; valid matured claims become spendable only through
  beneficiary `ClaimReward`, while voided/prunable claims may be swept without credit.

## Validation Evidence

- Current Iteration 196 first executable Gate 0 was run first on June 22, 2026 and initially exposed the
  in-progress compile gap for the new public record kind; after the CLI enum wiring, the release local
  testnet gate passed:
  `cargo test -p tensor_vm local_testnet --release`.
- Current Iteration 196 focused validation passed on June 22, 2026:
  `public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec`,
  `public_testnet_evidence_manifest_parses_into_bundle`,
  `validate_public_evidence_manifest_reports_default_criteria_status`,
  `execute_public_evidence_record_reports_outputs`, and `deployment_docs`.
- Current Iteration 196 broad validation passed on June 22, 2026:
  `cargo fmt --all -- --check`, `git diff --check`, `cargo test -p tensor_vm --lib`,
  `cargo test -p tensor_vm local_testnet --release`, `cargo test --workspace --release`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Current Iteration 196 tarpaulin passed on June 22, 2026:
  `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` with
  573 instrumented tests and 84.81% line coverage, 23261/27428 lines covered.
- Current Iteration 196 feature commit `a65f247`
  (`Require deployed detection evidence for full spec`) prepared on June 22, 2026.

## Archive

- Iteration 191: Public Evidence CUDA Miner Gate. Full-spec public evidence now requires
  `cuda_verified_miner_count` to cover counted public miners. Commit `322857a`
  (`Require CUDA miner evidence for full spec`) pushed to `origin/main` on June 22, 2026.
- Iteration 190: Raw Public Operational Evidence Gate. Full-spec public evidence now requires raw public
  block/finality/data-availability/invalid-work/reward records behind signed summaries. Commit `74ef9fe`
  (`Require raw public operational evidence`) pushed to `origin/main` on June 22, 2026.
- Iteration 189: Public Chain History Raw-Record Gate. Commit `306ad18` pushed to `origin/main` on
  June 22, 2026.
- Iteration 188: Public Network Runtime Raw-Observation Gate. Commit `8eac676` pushed to `origin/main` on
  June 22, 2026.
- Iteration 187 and earlier: Chain-owned verifier bandwidth evidence, public randomness evidence raw-record
  gate, mixed-dtype conformance vectors, trace-bisection DoS admission bounds, isolated trace-bisection
  timeout policy, and reward sweep boundary naming are implemented and documented in prior commits.
