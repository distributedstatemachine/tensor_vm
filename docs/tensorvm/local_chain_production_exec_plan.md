# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 222 in progress: Verified Public Randomness Gates Full Spec.
- Current status: public evidence remains deployment-gated; Iteration 222 separates signed raw
  randomness/validator-VRF lifecycle record shape from chain-accepted drand/validator-VRF evidence so shaped
  placeholder records cannot set `public_evidence_full_spec=true`. Iteration 221 requires deployed public service
  health/content evidence before a public evidence bundle can report `independently_checkable=true`.
  Iteration 220 tightens public full-spec raw
  block/finality history so signed records must cover the exact observed block range rather than a shifted
  range with the same count. Iteration 219 tightens public full-spec raw
  operational, detection-measurement, and validator-VRF-lifecycle evidence so signed records cannot satisfy
  full-spec evidence with `observed_block` values outside the signed run's `observed_blocks` range.
  Iteration 218 tightens public full-spec
  validator VRF lifecycle evidence so checked available receipts require both deployed `committed` and
  `revealed` lifecycle records with matching validator IDs and beacon rounds. Iteration 217 tightens validator reward
  release so a validator with a registered VRF key cannot release a pending receipt reward using an earlier
  legacy unkeyed reveal. Iteration 216 keeps the full-spec public
  evidence criteria intact while reducing the raw record cardinality of test-only full-spec fixtures so
  coverage instrumentation can complete. Iteration 215 ties raw validator-VRF lifecycle
  records to the same checked available receipt roots proven by raw data-availability records. Iteration
  214 tightens the scalar
  validator-VRF lifecycle evidence flag so it cannot pass when checked receipts exceed available receipt
  artifacts. Index-consistency Tensor IR ops are registry vocabulary only; Iteration 213 adds
  chain-command boundary evidence that `gather`/`scatter`/`embedding` cannot be registered as consensus
  program bodies until their index-consistency proofs exist. Reward-delay work is implemented locally for
  receipt, proposer, and challenge rewards;
  Iteration 212 closes the pre-finality block-check edge so a successful canonical block-check challenge
  prevents later proposer reward materialization after finality instead of relying on adapter-side or timing
  workarounds. Post-run public evidence requires `cuda_verified_miner_count` to cover counted public
  miners, positive `cuda_graph_execution_receipts` within checked/available receipt counts, and
  committed/revealed `validator_vrf_lifecycle_records` covering each checked receipt. Iteration 207 tightened the
  independently checkable supporting-artifact gate so signed raw-record artifact URIs must be distinct
  across required public record kinds before `public_evidence_full_spec=true` can pass.
  Iteration 198 tightened the VRF-lifecycle gate so raw lifecycle records must cover distinct checked receipt
  roots rather than padding the count with multiple records for the same receipt. Iteration 199 applies the
  same checked-receipt coverage rule to raw public data-availability measurement records. Iteration 200
  extends semantic receipt coverage checks to invalid-work and reward-settlement raw records. Iteration 201
  aligns direct bundle validation for raw deployed detection measurements with the manifest parser's field
  checks. Iteration 202 adds semantic consistency checks for raw public block/finality history records.
  Iteration 203 requires raw public randomness-beacon records to cover distinct observed blocks and beacon
  rounds exactly. Iteration 204 requires counted public network-runtime observations to use distinct peer
  IDs and public listen multiaddrs. Iteration 205 requires deployed public service evidence to use
  distinct signed service-health and service-content URLs across service kinds. Iteration 208 extends the
  same URL-diversity rule to pre-run public service launch plans so reused service URLs cannot pass
  public preflight. Iteration 209 scopes local CPU runtime production knobs to the local CPU profile so
  public/mainnet profiles cannot inherit local synthetic producer/proposer behavior from a shared
  environment. Iteration 210 adds runtime-configured bootstrap peers so startup/readiness can merge
  `TENSORVM_BOOTSTRAP_PEERS` with durable peer-book records.
- Current blockers:
  - Public 7-day external deployment evidence and real CUDA miner/runtime evidence remain outside the
    local CPU proof.
  - Real deployed full VRF construction and public commit-reveal lifecycle artifacts remain open.
- Next action: continue real public VRF/CUDA/deployed-run artifact work.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Current iteration release gate `cargo test -p tensor_vm local_testnet --release` passed on June 23, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, runtime profile env-scope tests, Gate 0 | Preserve one transition engine while adding runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection messages, drand, validator reveals, and runtime/peer-book bootstrap config | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof and deployed dispute evidence |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, receipt verification scenarios, packed int8 APIs, const blobs, role-owned graph execution, local checker graph evidence, and explorer API graph rendering | Continue CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, delayed pending reward holds, and state-rooted proposer reward release tombstones | Continue Tier-C committee policy and deployed public-operator evidence |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates, and public evidence now distinguishes signed raw record shape from chain-accepted drand/validator-VRF evidence | Add real deployed full VRF construction artifacts and public lifecycle records |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence, and chain-owned verifier bandwidth estimates | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 222: Verified Public Randomness Gates Full Spec

Feature capability: prevent shaped public randomness-beacon and validator-VRF lifecycle records from
setting `public_evidence_full_spec=true` until the public evidence schema is tied to chain-accepted
drand records or chain-accepted validator VRF reveal records from the deployed run.

Readiness requirements covered: `upow.md` §10 and `mvp_spec.md` §31.4/§35 require unbiasable deployed
randomness and deployed commit-to-reveal lifecycle evidence, not only signed hashes with public-looking
proof-kind labels.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns public full-spec evidence admission.

Adapter callers: manifest validation, `tvmd public evidence validate`, supporting-record summary/artifact
generation, public evidence docs/tests, and implementation status reports.

Old shortcut being removed: placeholder `drand-v1`/`validator-vrf-v1` raw records whose proof material was
only a nonzero hash could satisfy the full-spec public evidence gate.

Regression test that proves the shortcut is gone: otherwise complete public evidence with shaped raw
randomness and lifecycle records reports those raw records present but keeps chain-accepted evidence flags
false and `public_evidence_full_spec=false`; zero beacon rounds are rejected by manifest parsing,
supporting-record line validation, and direct bundle evaluation.

Behavior with local synthetic block production disabled: unchanged; this is post-run public evidence
validation.

Behavior for producer and non-producer roles: unchanged; role behavior is observed through generated public
evidence records.

Structured evidence source: signed `randomness_beacon_record=...` and
`validator_vrf_lifecycle=...` records, plus future chain-accepted drand/validator-VRF evidence exported
from the deployed run.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: readiness mapper, codebase explorer, and test-coverage explorer.

Parallelizable implementation workstreams: read-only mapping/test discovery ran in parallel; code/docs
edits remain single-writer.

Tests/checkers/docs to add or update: public evidence bundle regressions, manifest malformed-input
regression, CLI raw-record line rejection, manifest report fields, public evidence/status/coverage/tarpaulin
notes, and this exec plan.

Narrow validation commands: focused public randomness/lifecycle bundle tests, manifest malformed-input
test, raw-record line rejection test, and manifest report test.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, diff check, and tarpaulin/report update if coverage changes.

Validation evidence:

- Gate 0 first executable passed: `cargo test -p tensor_vm local_testnet --release`.
- Focused public-evidence regressions passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_randomness_records --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_rejects_malformed_input --lib`,
  `cargo test -p tensor_vm direct_public_record_line_rejections_report_invalid_receipts --lib`,
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle --lib`, and
  `cargo test -p tensor_vm public_testnet_evidence_manifest --lib`.
- Reward-delay audit passed: `cargo test -p tensor_vm reward --lib` ran 42 focused reward tests and
  confirmed live matured rewards stay pending until beneficiary `ClaimReward`.
- Broad validation passed: `cargo test -p tensor_vm --lib`, `cargo clippy -p tensor_vm --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin`
  reported 582 instrumented tests passing and 85.08% workspace coverage, 23,675/27,826 lines.

Expected observable evidence: `public_evidence_full_spec=true` is impossible without chain-accepted
public randomness and validator-VRF lifecycle evidence exported from the deployed run; signed raw record
shape remains separately reported.

Out of scope: adding a new drand proof schema, changing p2p randomness payload codecs, generating real
public deployment records, or deploying public infrastructure.

Split trigger: split if chain-accepted deployed randomness/VRF evidence requires changing public manifest
schema or runtime drand/VRF payload formats.

### Iteration 221: Public Service Evidence Required For Independent Bundles

Feature capability: require independently checkable public evidence bundles to include deployed public
service health and content evidence for RPC, explorer, faucet, and telemetry, using the existing exact
endpoint/path/authority/count/content checks.

Readiness requirements covered: `mvp_spec.md` §31.4 and §35 require public service health and
service-content roots as independently checkable deployment evidence, not only as a full-spec/public
criterion side gate.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns bundle-level independent checkability;
`PublicTestnetRunEvidence::evaluate` owns detailed service health/content validation.

Adapter callers: manifest validation, `tvmd public evidence validate`, deployment docs/tests, and public
evidence reports.

Old shortcut being removed: a bundle with missing deployed service-health or service-content records could
still report `independently_checkable=true` when other signed record summaries and artifacts were present.

Regression test that proves the shortcut is gone: clearing service-health or service-content evidence from
an otherwise complete bundle must now make both `independently_checkable=false` and
`public_evidence_full_spec=false`; parsed manifest service-content mismatches must likewise fail the
service evidence gate.

Behavior with local synthetic block production disabled: unchanged; this is post-run public evidence
validation.

Behavior for producer and non-producer roles: unchanged; this consumes deployed public service records.

Structured evidence source: signed `service=...` health records and signed `service_content=...` content
records already parsed into `PublicTestnetRunEvidence`.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: read-only service-evidence mapper and test-coverage explorer.

Parallelizable implementation workstreams: code/docs edits remain single-writer; read-only mapping and
test discovery ran in parallel.

Tests/checkers/docs to add or update: public evidence bundle service regressions, manifest mismatch
regression if needed, coverage/status/tarpaulin notes, and this exec plan.

Narrow validation commands: focused public evidence bundle and manifest service tests.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, diff check, and tarpaulin/report update if coverage changes.

Expected observable evidence: `independently_checkable=true` is impossible without exact deployed service
health and content evidence for all four public service kinds.

Out of scope: generating real deployed service records, changing service CLI signature formats, or changing
public URL validation rules.

Split trigger: split if making service evidence independently checkable requires changing manifest schema
or public report output formats beyond a boolean report field.

Validation evidence (June 23, 2026):

- Implementation commit: `0545df6` (`Require public service evidence for independent bundles`).
- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused public evidence checks passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib` and
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`.
- CLI report regression passed:
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.
- Broad lib validation passed: `cargo test -p tensor_vm --lib` reported 567 passed, 0 failed.
- Hygiene passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p tensor_vm
  --all-targets -- -D warnings`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
  target/tarpaulin` produced 85.04% workspace line coverage, 23641/27800 lines covered.

### Iteration 220: Public Chain History Covers Observed Block Range

Feature capability: require full-spec public raw block-history and finality-history evidence to cover the
exact signed observed block range `0..observed_blocks`.

Readiness requirements covered: public evidence integrity for `mvp_spec.md` post-run block/finality
artifacts and `upow.md` §11 finality/history evidence.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns raw public full-spec evidence admission.

Adapter callers: manifest validation, `tvmd public evidence validate`, public evidence docs/reports.

Old shortcut being removed: signed block-history/finality-history summaries could use distinct matching
block numbers shifted outside the signed observed range while keeping the expected record counts and roots.

Regression test that proves the shortcut is gone: a complete bundle with re-signed shifted block and
finality history records stays independently checkable but reports `public_evidence_full_spec=false`.

Behavior with local synthetic block production disabled: unchanged; this is post-run public evidence
validation.

Behavior for producer and non-producer roles: unchanged; this consumes deployed evidence records only.

Structured evidence source: raw `block_history_record=<block>,<block-root>` and
`finality_history_record=<block>,<block-root>,finalized|unfinalized` records.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate deployed evidence artifacts.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection was parallelized; code/docs edits are
single-writer.

Tests/checkers/docs to add or update: public evidence bundle chain-history regressions, public
evidence/MVP docs, coverage/status/tarpaulin notes, and this exec plan.

Narrow validation commands: focused public raw chain-history bundle test.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, diff check, and tarpaulin/report update.

Expected observable evidence: `public_evidence_full_spec=true` rejects otherwise signed block/finality
history records unless their block numbers are exactly the signed observed run range.

Out of scope: generating real deployed public records, changing live chain finality behavior, or changing
p2p payload encoding.

Split trigger: split if exact block-range validation requires a manifest schema change.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused public evidence check passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_chain_history_records --lib`.
- Broad lib validation passed: `cargo test -p tensor_vm --lib` reported 567 passed, 0 failed.
- Hygiene passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p tensor_vm
  --all-targets -- -D warnings`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
  target/tarpaulin` produced 85.04% workspace line coverage, 23637/27796 lines covered.
- Implementation commit: `f27f858` (`Require public chain history range coverage`).

### Iteration 219: Public Raw Records Bound Observed Blocks To Run Window

Feature capability: require full-spec public raw operational, deployed detection-measurement, and
validator-VRF-lifecycle evidence to use observed block indexes inside the signed run window.

Readiness requirements covered: public evidence integrity for `mvp_spec.md` post-run artifacts and
`upow.md` §10/§12 deployed randomness/economics evidence.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns raw public full-spec evidence admission.

Adapter callers: manifest validation, `tvmd public evidence validate`, public evidence docs/reports.

Old shortcut being removed: a signed raw-record summary could aggregate otherwise well-formed records with
`observed_block >= observed_blocks`, making evidence from outside the signed run appear to support the
full-spec public gate.

Regression test that proves the shortcut is gone: focused public evidence tests re-sign out-of-window raw
data-availability, invalid-work, reward-settlement, detection-measurement, and validator-VRF-lifecycle
records and prove `public_evidence_full_spec=false`.

Behavior with local synthetic block production disabled: unchanged; this is post-run public evidence
validation.

Behavior for producer and non-producer roles: unchanged; this consumes deployed evidence records only.

Structured evidence source: raw
`data_availability_measurement=<receipt-root>,available|unavailable,<block>`,
`invalid_work_rejection=<receipt-root>,rejected,<block>`,
`reward_settlement=<receipt-root>,<miner-id>,<validator-id>,<block>`,
`detection_measurement=<mechanism>,<subject-root>,<sample-count>,<detected-count>,<block>`, and
`validator_vrf_lifecycle=<receipt-root>,<validator-id>,<beacon-round>,committed|revealed,<block>`
records.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection and focused validation were parallelized;
code/docs edits are single-writer.

Tests/checkers/docs to add or update: public evidence bundle regressions, public evidence/MVP docs,
coverage/status/tarpaulin notes, and this exec plan.

Narrow validation commands: focused public raw operational, detection-measurement, and validator VRF
lifecycle bundle tests.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, diff check, and tarpaulin/report update.

Expected observable evidence: `public_evidence_full_spec=true` rejects otherwise signed raw records whose
observation block lies outside the signed observed run.

Out of scope: generating real deployed public records, changing live local VRF/reward behavior, or changing
p2p payload encoding.

Split trigger: split if binding observed blocks requires a manifest schema change.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused public evidence checks passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec --lib`,
  and
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib`.
- Broad lib validation passed: `cargo test -p tensor_vm --lib` reported 567 passed, 0 failed.
- Hygiene passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p tensor_vm
  --all-targets -- -D warnings`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
  target/tarpaulin` produced 85.04% workspace line coverage, 23627/27785 lines covered.
- Implementation commit: `ef2024b` (`Bind public raw records to run window`).

### Iteration 218: Public VRF Lifecycle Requires Commit/Reveal Pairs

Feature capability: require full-spec public validator VRF lifecycle evidence to prove both deployed
`committed` and `revealed` phases for each checked available receipt.

Readiness requirements covered: `upow.md` §10 commit-to-reveal randomness binding and public evidence
requirements in `mvp_spec.md`.

Canonical owner: `PublicTestnetRunEvidence::evaluate` owns scalar run evidence, and
`PublicTestnetEvidenceBundle::evaluate` owns raw public full-spec admission.

Adapter callers: manifest validation, `tvmd public evidence validate`, public evidence docs/reports.

Old shortcut being removed: one revealed raw lifecycle line per receipt could satisfy full-spec lifecycle
evidence without any committed-phase evidence.

Regression test that proves the shortcut is gone: an otherwise full-spec bundle with only revealed
lifecycle records, properly re-signed and artifact-bound, no longer reaches `public_evidence_full_spec=true`.

Behavior with local synthetic block production disabled: unchanged; this is post-run public evidence
validation.

Behavior for producer and non-producer roles: unchanged; this consumes deployed evidence records only.

Structured evidence source:
`validator_vrf_lifecycle_records`, `validator_vrf_lifecycle_root`, and raw
`validator_vrf_lifecycle=<receipt-root>,<validator-id>,<beacon-round>,committed|revealed,<block>`
records.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection was parallelized; code/docs edits are
single-writer.

Tests/checkers/docs to add or update: public run evidence count gate, raw lifecycle pair gate, public
manifest/CLI fixtures, MVP/public evidence docs, coverage/status/tarpaulin notes, and this exec plan.

Narrow validation commands: focused public lifecycle, manifest, and run-evidence tests.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, diff check, and tarpaulin/report update.

Expected observable evidence: `public_evidence_full_spec=true` requires two validator VRF lifecycle
records per checked available receipt and rejects reveal-only lifecycle evidence.

Out of scope: generating real deployed VRF records, changing live local VRF behavior, or changing p2p
payload encoding.

Split trigger: split if deployed lifecycle pair validation requires a manifest schema change.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused public evidence checks passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`,
  `cargo test -p tensor_vm public_testnet_run_evidence_requires_independent_external_operators --lib`,
  and `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.
- Broad lib validation passed: `cargo test -p tensor_vm --lib` reported 567 passed, 0 failed.
- Hygiene passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p tensor_vm
  --all-targets -- -D warnings`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
  target/tarpaulin` produced 85.03% workspace line coverage, 23616/27775 lines covered.
- Implementation commit: `dc9e05b` (`Require public VRF lifecycle commit reveal pairs`).

### Iteration 217: Keyed VRF Reveals Gate Validator Reward Release

Feature capability: require validator receipt reward release to honor the validator's current registered
VRF key, so keyed validators must provide matching keyed reveal evidence before their delayed validator
reward can become spendable.

Readiness requirements covered: §10 commit→reveal randomness binding and §12 delayed reward release must
use chain-owned keyed reveal evidence instead of accepting an older local fallback reveal as a release
workaround.

Canonical owner: `chain::commands` owns pending reward release, while `chain::validation` owns reveal
admission.

Adapter callers: `ChainCommand::ClaimReward`, automatic matured receipt reward sweeps, runtime validator
role reward release, status/explorer pending reward evidence.

Old shortcut being removed: once a legacy unkeyed reveal existed for a receipt/validator pair, the reward
release gate accepted it even if that validator later registered a production VRF public key.

Regression test that proves the shortcut is gone: a validator reward with an earlier legacy reveal remains
pending after key registration until a keyed Ed25519 reveal matching the registered key is submitted.

Behavior with local synthetic block production disabled: unchanged; reward release remains chain-owned and
driven by `ClaimReward`/sweep commands.

Behavior for producer and non-producer roles: unchanged; all roles observe the same pending reward ledger.

Structured evidence source: `validator_vrf_reveals`, `validators[*].vrf_public_key`, and pending receipt
reward maturity state.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only source inspection was parallelized; code/docs edits
are single-writer.

Tests/checkers/docs to add or update: focused reward/VRF regression, coverage/status notes if needed, and
this exec plan.

Narrow validation commands: focused reward release regression plus existing keyed validator VRF reveal
test.

Broad validation commands before commit: Gate 0 first, `cargo test -p tensor_vm --lib`, clippy with
warnings denied, rustfmt check, and diff check.

Expected observable evidence: registered-key validators cannot claim delayed validator receipt rewards
from legacy fallback reveal records.

Out of scope: removing the local fallback helper entirely, changing p2p reveal payload encoding, or
generating real deployed public VRF lifecycle artifacts.

Split trigger: split if release gating requires profile-specific chain params or storage migrations.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused regression passed: `cargo test -p tensor_vm
  registered_validator_vrf_key_requires_keyed_reveal_for_reward_release --lib`.
- Focused VRF/reward checks passed: `cargo test -p tensor_vm
  keyed_validator_vrf_reveal_requires_production_proof --lib`, `cargo test -p tensor_vm
  validator_receipt_reward_waits_for_vrf_reveal_after_maturity --lib`, and `cargo test -p tensor_vm
  service_status_exports_randomness_binding_evidence --lib`.
- Broad lib validation passed: `cargo test -p tensor_vm --lib` reported 567 passed, 0 failed.
- Hygiene passed: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy -p tensor_vm
  --all-targets -- -D warnings`.
- Coverage refresh passed: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
  target/tarpaulin` produced 85.02% workspace line coverage, 23593/27749 lines covered.
- Implementation commit: `d185f28` (`Delay keyed validator rewards until keyed VRF reveal`).

### Iteration 216: Compact Full-Spec Evidence Fixtures For Coverage Runs

Feature capability: keep full-spec public evidence tests semantically full-spec while avoiding
coverage-time fixture explosions from 100,800 raw public evidence records per full-spec bundle.

Readiness requirements covered: coverage validation must exercise the same public evidence gates without
turning fixture construction into the blocker.

Canonical owner: `crates/tensor_vm/src/testnet/tests/run_fixtures.rs` owns test-only public evidence
fixture cardinality.

Adapter callers: public evidence bundle tests only.

Old shortcut being removed: none in production behavior; this removes a test harness scale problem that
made tarpaulin unable to refresh after Iteration 215.

Regression test that proves behavior is preserved: full-spec public evidence bundle tests still evaluate
default `PublicTestnetCriteria` with `full_spec_evidence_met=true`.

Behavior with local synthetic block production disabled: unchanged; this is test fixture construction only.

Behavior for producer and non-producer roles: unchanged.

Structured evidence source: unchanged public evidence raw records, with fewer test fixture records derived
from a test-only block time.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection was parallelized; code/docs edits are
single-writer.

Tests/checkers/docs to add or update: compact full-spec fixture helper, public evidence bundle call sites,
tarpaulin report if coverage can complete, and this exec plan.

Narrow validation commands: focused full-spec public evidence bundle tests covering raw randomness and raw
validator VRF lifecycle records.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, diff check, and tarpaulin/report update.

Expected observable evidence: `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir
target/tarpaulin` completes instead of stalling in the instrumented public evidence bundle tail.

Out of scope: changing production `PublicTestnetCriteria`, public evidence manifest requirements, or
weakening any public evidence full-spec gate.

Split trigger: split if tarpaulin still stalls after compacting fixture cardinality.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused full-spec fixture regressions passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_randomness_records --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib`,
  and
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_publication_and_audit_records --lib`.
- `cargo test -p tensor_vm --lib` passed: 566 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` completed
  successfully after the fixture compaction: 581 instrumented tests passed, 85.00% workspace line
  coverage, 23,562/27,721 lines covered.
- Implementation commit: `12dffe6` (`Compact full-spec evidence fixtures for coverage`).
- Validation metadata recorded in the follow-up docs commit before push.
- Push range from prior remote anchor: `026929c..HEAD`.

### Iteration 215: VRF Lifecycle Records Must Match Available Receipt Roots

Feature capability: require full-spec raw validator VRF lifecycle evidence to cover the same checked
available receipt roots as raw data-availability measurements.

Readiness requirements covered: public full-spec evidence must prove deployed validator VRF lifecycle
coverage for the checked receipts whose artifacts are actually available, rather than any unrelated set of
distinct revealed receipt roots.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns cross-record public evidence consistency for
full-spec bundle validation.

Adapter callers: `tvmd public evidence validate`, manifest bundle validation, public evidence reports, and
public evidence docs consume the bundle full-spec result.

Old shortcut being removed: raw validator lifecycle records could satisfy full-spec evidence with
distinct, revealed receipt roots that did not match the raw data-availability measurement receipt roots.

Regression test that proves the shortcut is gone: an otherwise full-spec evidence bundle whose lifecycle
records aggregate and reveal successfully but use a different receipt-root set than data-availability
records keeps summary evidence true while clearing full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; role behavior is observed through public evidence.

Structured evidence source: raw `data_availability_measurement=...` and `validator_vrf_lifecycle=...`
records.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection was parallelized; code/docs edits are
single-writer.

Tests/checkers/docs to add or update: public bundle raw lifecycle regression, full-spec fixtures, public
evidence docs, MVP wording, coverage/status/tarpaulin notes, and this exec plan.

Narrow validation commands: focused raw validator VRF lifecycle bundle test and manifest parser/report
tests.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, diff check, and tarpaulin/report update if feasible for coverage-changing tests.

Expected observable evidence: `public_evidence_full_spec=true` requires revealed lifecycle records whose
receipt-root set exactly equals the available checked receipt-root set.

Out of scope: generating real deployed VRF records, changing manifest syntax, or adding a new raw record
field.

Split trigger: split if cross-record consistency requires manifest schema changes or deployed artifact
format changes.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused raw lifecycle regression passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib`.
- Affected manifest/report regressions passed:
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib` and
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.
- `cargo test -p tensor_vm --lib` passed: 566 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed after formatting.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` was attempted
  twice. Both attempts rebuilt the workspace, passed the `experiments` and `tensor_vm_explorer`
  instrumented tests, reached the `tensor_vm` public-evidence bundle tail, then made no progress for more
  than ten minutes with the instrumented `tensor_vm` test binary still running. Both attempts were
  interrupted with exit code 130, so no fresh coverage summary was produced. The latest completed
  tarpaulin run remains the June 22, 2026 report in `docs/tensorvm/tarpaulin_report.md`.
- Iteration status: implementation and non-coverage validation are complete locally; coverage refresh is
  blocked by the reproducible tarpaulin instrumentation stall, so this iteration is not marked complete.
- Implementation/blocker-record commit pushed to `main`: `cc009ad`
  (`Require VRF lifecycle roots to match available receipts`), range `3e9311b..cc009ad`.

### Iteration 214: Validator VRF Lifecycle Requires Available Receipts

Feature capability: require complete data availability for checked receipts before public scalar run
evidence can claim validator VRF lifecycle coverage for those receipts.

Readiness requirements covered: public full-spec evidence must not claim complete deployed validator VRF
commit-to-reveal/reward-delay lifecycle coverage when any checked receipt lacks available artifacts.

Canonical owner: `PublicTestnetRunEvidence::evaluate` owns scalar public run evidence flags consumed by
bundle validation and CLI reports.

Adapter callers: `tvmd public evidence validate`, manifest bundle validation, public evidence docs, and
readiness reports consume the scalar flag.

Old shortcut being removed: `has_validator_vrf_lifecycle_evidence` could be true when
`validator_vrf_lifecycle_records == checked_receipts` even if `available_receipts < checked_receipts`.

Regression test that proves the shortcut is gone: an otherwise full-spec evidence bundle with one
unavailable checked receipt keeps public criteria/independent records where applicable but clears
`has_validator_vrf_lifecycle_evidence`, validator lifecycle record summary, and full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run evidence gate.

Behavior for producer and non-producer roles: unchanged; role behavior is observed through public evidence.

Structured evidence source: `checked_receipts`, `available_receipts`, and
`validator_vrf_lifecycle_records`.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only inspection was parallelized; code/docs edits are
single-writer.

Tests/checkers/docs to add or update: public run/bundle evidence regressions, public evidence docs, and
this exec plan.

Narrow validation commands: focused run-evidence/bundle lifecycle tests.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, and diff check.

Expected observable evidence: validator VRF lifecycle evidence remains false unless checked receipts are
all available and lifecycle records exactly cover those checked receipts.

Out of scope: generating real deployed VRF records, changing manifest syntax, or changing raw lifecycle
record hashing.

Split trigger: split if the stricter scalar flag requires changing raw record schema or public manifest
parsing.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused run-evidence regression passed:
  `cargo test -p tensor_vm public_testnet_run_evidence_requires_independent_external_operators --lib`.
- Focused lifecycle bundle regression passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec --lib`.
- Affected evidence fixture/regression tests passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_publication_and_audit_records --lib`,
  `cargo test -p tensor_vm public_testnet_evidence_manifest_parses_into_bundle --lib`, and
  `cargo test -p tensor_vm validate_public_evidence_manifest_reports_default_criteria_status --lib`.
- `cargo test -p tensor_vm --lib` passed: 566 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `b0040b7` (`Require availability for VRF lifecycle evidence`).
- Validation metadata commit: `d6b3348` (`Record VRF lifecycle availability validation`).
- Pushed to `main`: `18774f0..d6b3348`.

### Iteration 213: Index-Consistency Ops Rejected at Program Registration

Feature capability: prove at the chain command boundary that index-consistency-gated Tensor IR ops remain
non-admitted vocabulary and cannot enter state-rooted registered program bodies.

Readiness requirements covered: `upow.md` §7 requires `gather`/`scatter`/`embedding` to stay out of v0
consensus until index-consistency arguments exist.

Canonical owner: `TensorGraph::validate_for_consensus` and `chain::receipts::register_program_body` own
consensus graph admission.

Adapter callers: `ChainCommand::RegisterProgramBody`, p2p graph job hydration, RPC submission, and role
startup program hydration all reach the same chain registration path.

Old shortcut being removed: only the IR unit test proved the vocabulary classification; the chain command
surface did not have a direct regression showing non-admitted index ops cannot be registered as program
bodies.

Regression test that proves the shortcut is gone: a graph body containing `gather` parses and validates in
non-consensus mode, but `ChainCommand::RegisterProgramBody` rejects it with
`tensor ir op is not consensus admitted` and leaves `program_bodies` empty.

Behavior with local synthetic block production disabled: unchanged; graph admission is independent of
synthetic job production.

Behavior for producer and non-producer roles: unchanged; all roles use the same registered program-body
state.

Structured evidence source: chain command result and `ChainState::program_bodies`.

Finality source: unchanged; no block/finality behavior changes.

Wire-size and codec boundary: no wire format changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only source/test/doc inspection was parallelized; edits are
single-writer in chain tests and docs.

Tests/checkers/docs to add or update: chain command regression, coverage matrix evidence, and this exec
plan.

Narrow validation commands: focused command regression.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, and diff check.

Expected observable evidence: index-consistency-gated ops remain parseable vocabulary but cannot be
state-registered through the chain consensus command path.

Out of scope: implementing index-consistency proofs, admitting `gather`/`scatter`/`embedding`, changing
graph canonical JSON, or changing p2p codecs.

Split trigger: split if command-boundary coverage requires changing graph admission semantics rather than
adding the missing regression.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused command-boundary regression passed:
  `cargo test -p tensor_vm chain_engine_rejects_index_consistency_ops_at_program_registration --lib`.
- `cargo test -p tensor_vm --lib` passed: 566 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `a12d4d9` (`Reject index-consistency graphs at registration`).
- Validation metadata commit: `079b679` (`Record index registration validation`).
- Pushed to `main`: `33be5c3..18774f0`.

### Iteration 212: Block-Check Challenges Block Late Proposer Reward Materialization

Feature capability: make canonical proposer reward materialization consult block-check challenge records so
a challenged block cannot later create a non-voided pending proposer reward when finality arrives after the
challenge.

Readiness requirements covered: `upow.md` §12 and `mvp_spec.md` §20.7 require delayed reward finality and
block-check clawback to be consensus-state behavior, not a runtime/admission workaround.

Canonical owner: `crates/tensor_vm/src/chain/blocks.rs::materialize_finalized_proposer_rewards` owns
creation of delayed proposer reward claims from finalized blocks.

Adapter callers: block-vote finality, side-branch promotion, block production/admission helpers, and chain
maintenance call the shared materialization path.

Old shortcut being removed: if a canonical block-check challenge was accepted before the block's proposer
reward was materialized, later finality/materialization could ignore the challenge record and create a fresh
non-voided pending proposer reward for the challenged block.

Regression test that proves the shortcut is gone: submit a canonical block-check challenge before finality,
then finalize the block and assert proposer reward materialization remains absent/non-spendable while the
challenge record remains the state-rooted evidence.

Behavior with local synthetic block production disabled: unchanged; this is a shared chain-state reward
materialization rule.

Behavior for producer and non-producer roles: unchanged; all roles that ingest votes or promoted blocks use
the same chain materialization path.

Structured evidence source: `block_check_challenges`, `pending_proposer_rewards`, `finalized_blocks`, and
`released_proposer_reward_blocks` in `ChainState`.

Finality source: existing block-vote finality and manual test finality state; reward finality remains
separate from block finality.

Wire-size and codec boundary: no p2p/consensus wire format changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Parallelizable implementation workstreams: read-only test/code inspection was parallelized; code edits are
single-writer in chain state and tests.

Tests/checkers/docs to add or update: chain block-check challenge regression and this exec plan.

Narrow validation commands: focused challenge regression.

Broad validation commands before commit: Gate 0 first, focused challenge tests, full tensor_vm lib tests,
clippy with warnings denied, rustfmt check, and diff check.

Expected observable evidence: a pre-finality successful block-check challenge prevents later pending
proposer reward creation for the challenged block.

Out of scope: changing verifier transcript schemas, public deployment evidence, p2p message formats, or
hard stake slashing.

Split trigger: split if delaying late proposer materialization requires changing finality vote admission,
challenge IDs, or canonical block storage.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused pre-finality block-check reward regression passed:
  `cargo test -p tensor_vm pre_finality_block_check_challenge_delays_and_voids_late_proposer_reward --lib`.
- Adjacent finalized block-check reward regression passed:
  `cargo test -p tensor_vm canonical_block_check_challenge_materializes_and_delays_reward_in_chain --lib`.
- `cargo test -p tensor_vm --lib` passed: 565 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `c4ec55a` (`Delay pre-finality block-check rewards`).
- Validation metadata commit: `bbfabac` (`Record block-check reward validation`).
- Pushed to `main`: `635e97f..33be5c3`.

### Iteration 211: CUDA Graph Evidence Requires CUDA Miner Coverage

Feature capability: make the CUDA graph-execution evidence flag depend on counted CUDA-verified public
miner coverage as well as positive, checked, and available CUDA graph receipt counts.

Readiness requirements covered: public full-spec evidence must prove real CUDA miner/runtime evidence, not
only scalar CUDA graph receipt counts disconnected from counted CUDA-capable public miners.

Canonical owner: `PublicTestnetRunEvidence::evaluate` owns public run scalar evidence admission.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the report.

Old shortcut being removed: `has_cuda_graph_execution_evidence` could report true when
`cuda_graph_execution_receipts` was positive and bounded by checked/available receipts even if
`cuda_verified_miner_count` did not cover counted public miners.

Regression test that proves the shortcut is gone: mutate an otherwise full-spec public evidence bundle so
CUDA graph receipts stay positive but `cuda_verified_miner_count` is zero, then assert both CUDA miner and
CUDA graph evidence flags are false and full-spec evidence remains false.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence.

Structured evidence source: `cuda_verified_miner_count`, counted public miner operators, and
`cuda_graph_execution_receipts`.

Finality source: unchanged.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence
evaluation.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Tests/checkers/docs to add or update: public run evidence/bundle regression, public evidence docs/status,
and this exec plan.

Narrow validation commands: focused public CUDA evidence tests.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, and diff check.

Expected observable evidence: positive CUDA graph receipt counts do not set
`cuda_graph_execution_evidence=true` unless the same public run also proves CUDA verified miner coverage.

Out of scope: generating real CUDA receipts, CUDA kernel changes, or external public deployment evidence.

Split trigger: split if coupling the evidence flags requires changing public manifest syntax or adding raw
CUDA receipt identities rather than scalar consistency.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused run-evidence regression passed:
  `cargo test -p tensor_vm public_testnet_run_evidence_requires_independent_external_operators --lib`.
- Focused CUDA miner/graph bundle regressions passed:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec --lib`
  and
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec --lib`.
- `cargo test -p tensor_vm --lib` passed: 564 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `511f6d6` (`Require CUDA miner coverage for graph evidence`).
- Validation metadata commit: `b8f3906` (`Record CUDA graph evidence validation`).
- Push result: `git push origin main` succeeded on June 23, 2026, updating `main` from `f215cdb` to
  `b8f3906`.

### Iteration 210: Runtime Bootstrap Peer Policy

Feature capability: allow node runtime adapters to accept profile/runtime-scoped bootstrap libp2p
multiaddrs from `TENSORVM_BOOTSTRAP_PEERS`, validate them through the shared bootstrap multiaddr rules, and
merge them with the durable peer book when starting or checking libp2p.

Readiness requirements covered: shared profiles and runtime adapters should own public bootstrap policy;
bootstrap loading should not depend only on a pre-seeded persisted peer book.

Canonical owner: `NetworkConfig` owns runtime network bootstrap policy; `p2p::peer_book` owns bootstrap
multiaddr validation and normalization.

Adapter callers: `tvmd node serve`, `tvmd miner run`, `tvmd validator run`, `tvmd proposer run`, and
`tvmd node check`.

Old shortcut being removed: runtime bootstrap peers could only come from persisted peer-book state, so
public/testnet runtime launch policy had to be staged through `tvmd node peer add` even when an operator
provided bootstrap peers in the process environment.

Regression test that proves the shortcut is gone: set `TENSORVM_BOOTSTRAP_PEERS` with a valid full
`/p2p/<peer-id>` multiaddr and assert runtime node config carries it; set malformed, zero-TCP, or
peer-id-less values and assert config construction rejects them.

Behavior with local synthetic block production disabled: unchanged; bootstrap policy is independent of
local production.

Behavior for producer and non-producer roles: all roles use the same bootstrap merge path.

Structured evidence source: readiness/runtime status `p2p_bootstrap_peers` count from merged runtime
bootstrap addresses and peer-book records.

Finality source: unchanged.

Wire-size and codec boundary: no consensus wire change; libp2p bootstrap multiaddrs remain bounded by
normal `Multiaddr` parsing before service startup.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Tests/checkers/docs to add or update: runtime config env tests, service lifecycle readiness coverage,
readiness/status docs, and this exec plan.

Narrow validation commands: focused runtime-config and service lifecycle tests.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, selected tvmd CLI/runtime
tests, clippy with warnings denied, rustfmt check, and diff check.

Expected observable evidence: `TENSORVM_BOOTSTRAP_PEERS` can start/check libp2p with nonzero
`p2p_bootstrap_peers` even without a peer-book file, while persisted peer-book bootstrap peers still work.

Out of scope: public DNS ownership, external reachability probing, Kademlia rendezvous, or deployed
7-day evidence.

Split trigger: split if runtime bootstrap env support requires changing libp2p discovery behavior rather
than config validation and startup merging.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused bootstrap multiaddr validation passed:
  `cargo test -p tensor_vm p2p::peer_book::tests::bootstrap_multiaddr_normalization_requires_tcp_and_peer_id --lib`.
- Focused runtime bootstrap env config passed:
  `cargo test -p tensor_vm app::runtime_config::tests::runtime_node_config_ --lib`.
- Focused process readiness bootstrap env path passed:
  `cargo test -p tensor_vm --test tvmd_cli service_readiness_loads_runtime_bootstrap_peers_from_env_without_peer_book`.
- `cargo test -p tensor_vm --lib` passed: 564 passed, 0 failed.
- `cargo test -p tensor_vm --test tvmd_cli` passed: 9 passed, 0 failed.
- `cargo test -p tensor_vm --test tvmd_runtime` passed: 46 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `34ce0da` (`Add runtime bootstrap peer policy`).
- Validation metadata commit: `2065652` (`Record runtime bootstrap validation`).
- Push result: `git push origin main` succeeded on June 23, 2026, updating `main` from `58c1c3f` to
  `2065652`.

### Iteration 209: Profile-Scoped Local Runtime Knobs

Feature capability: keep `TENSORVM_LOCAL_CPU_*` runtime producer/proposer knobs scoped to the local CPU
profile so public-testnet and mainnet node configs cannot inherit local-only synthetic job production,
local validator block proposal, block interval, proposer delay, or local proposer cooldown behavior from a
shared operator environment.

Readiness requirements covered: shared profiles must drive runtime policy, and public service exposure
policy must be wired through runtime adapters rather than existing only as profile metadata.

Canonical owner: `NodeConfig` owns profile/role production policy; `runtime_node_config` owns environment
adapter mapping into node config.

Adapter callers: `tvmd node serve`, `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run`.

Old shortcut being removed: local CPU environment knobs were applied while building every runtime profile,
letting public/mainnet node configs report or run local-only production modes when an operator reused a
local testnet environment.

Regression test that proves the shortcut is gone: build runtime node configs with
`TENSORVM_CHAIN_PROFILE=public_testnet` plus all local CPU producer/proposer knobs set, then assert the
resulting config has no local interval, synthetic producer, local block proposer, proposer delay, or local
cooldown; also assert `local_cpu` still honors the same knobs.

Behavior with local synthetic block production disabled: public/mainnet profiles remain disabled even if
local env flags are present; local CPU behavior is unchanged.

Behavior for producer and non-producer roles: only local CPU validators can enable the local proposer and
synthetic producer paths; non-validator roles remain non-producers.

Structured evidence source: `NodeConfig` accessors and runtime status snapshots derived from those
accessors.

Finality source: unchanged; this iteration only scopes local runtime policy.

Wire-size and codec boundary: no p2p/consensus wire changes; this is runtime config/profile policy.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Tests/checkers/docs to add or update: profile policy tests, runtime environment adapter tests, readiness
wording, and this exec plan.

Narrow validation commands: focused profile/runtime-config tests.

Broad validation commands before commit: Gate 0 first, full tensor_vm lib tests, clippy with warnings
denied, rustfmt check, and diff check.

Expected observable evidence: public/mainnet runtime configs ignore local CPU environment production knobs,
while local CPU validators still honor them.

Out of scope: public endpoint reachability, nginx/systemd topology, external deployment evidence, or CUDA
runtime evidence.

Split trigger: split if enforcing profile scope requires changing consensus block production or p2p
ingestion instead of config/accessor policy.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- Focused profile policy passed:
  `cargo test -p tensor_vm profile::tests::node_config_drives_local_runtime_policy_without_changing_chain_base --lib`.
- Focused runtime adapter policy passed:
  `cargo test -p tensor_vm app::runtime_config::tests:: --lib`.
- `cargo test -p tensor_vm --lib` passed: 561 passed, 0 failed.
- `cargo test -p tensor_vm --test tvmd_runtime` passed: 46 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `b56a674` (`Scope local runtime knobs to local profile`).
- Validation metadata commit: `5a14dd9` (`Record profile runtime knob validation`).
- Push result: `git push origin main` succeeded on June 23, 2026, updating `main` from `bc4bd0d` to
  `5a14dd9`.

### Iteration 208: Public Preflight Service URL Diversity Gate

Feature capability: require public-testnet launch preflight plans to use distinct public service health
URLs and distinct public service content URLs across RPC, explorer, faucet, and telemetry service kinds.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_preflight.md` require exactly one ready
public service plan per surface before a public run starts. Launch readiness should reject a plan that
points multiple service kinds at the same externally signed URL and only differs by endpoint ID.

Canonical owner: `PublicTestnetPreflightPlan::evaluate` owns public launch-plan readiness.

Adapter callers: `tvmd public preflight`, checked preflight manifests, deployment docs, and operator
runbooks consume the preflight report.

Old shortcut being removed: a preflight manifest could reuse one public service health or content URL
across multiple public service kinds while still reporting distinct endpoint IDs and per-kind ready
records.

Regression test that proves the shortcut is gone: mutate a complete preflight manifest so explorer reuses
the RPC health/content URLs, then assert the individual ready records are not enough to satisfy the public
service plan or launch readiness.

Behavior with local synthetic block production disabled: unchanged; this is a pre-run deployment evidence
gate.

Behavior for producer and non-producer roles: unchanged; public service launch planning does not mutate
role behavior.

Structured evidence source: repeated preflight `service=...` manifest records.

Finality source: unchanged; preflight does not prove a run or finality.

Wire-size and codec boundary: no p2p/consensus wire changes; only manifest readiness validation changes.

Parallel subagents to run: none. Tooling requires explicit delegation authorization, so the parent remains
the single writer.

Tests/checkers/docs updated: preflight manifest regression, public preflight docs, deployment
README/runbook/status wording, deployment-doc assertions, and this exec plan.

Narrow validation passed: focused public preflight manifest test and
`cargo test -p tensor_vm public_deployment --lib`.

Broad validation passed: Gate 0 was first (`cargo test -p tensor_vm local_testnet --release`),
`cargo test -p tensor_vm --lib` with 559 passing tests, clippy with warnings denied,
`cargo fmt --all -- --check`, and `git diff --check`.

Expected observable evidence: otherwise launch-ready public preflight manifests report
`public_services_planned=false`, `deployment_plan_ready=false`, and `public_testnet_preflight_ready=false`
when any required service kind reuses another kind's health or content URL.

Out of scope: proving live deployed service reachability, changing nginx/systemd topology, or fetching
service content.

Split trigger: split if enforcing URL diversity requires active network probes instead of manifest-level
URL validation.

Commit `3d4789f` (`Validate public preflight service URLs`) prepared on June 23, 2026.
Metadata commit `2e52ef5` (`Record public preflight URL validation`) pushed to `origin/main` on
June 23, 2026.

### Iteration 207: Public Supporting Artifact URI Diversity Gate

Feature capability: require independently checkable public evidence bundles to use distinct signed
supporting raw-record artifact URIs across every required public record kind.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require signed external raw
supporting-record artifact locators behind summary roots. Those locators should prove separate
kind-specific raw artifacts, not one reused URI attached to multiple signed kind/root/count tuples.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns independently checkable supporting-artifact
admission.

Adapter callers: `tvmd public evidence validate`, manifest parsing, deployment docs, and public evidence
templates consume the bundle report.

Old shortcut being removed: a bundle could reuse the same external artifact URI across multiple public
record kinds while presenting valid per-kind signatures, roots, and counts, so the supporting-artifact gate
proved signed metadata but not distinct raw artifact locators.

Regression test that proves the shortcut is gone: mutate a complete bundle so the finality-history
supporting artifact reuses the block-history artifact URI, re-sign that artifact for its own kind/root/count,
and assert supporting artifacts, independent checkability, and full-spec evidence fail.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: signed `record_artifact=...` manifest records.

Finality source: unchanged; signed block/finality records remain separate evidence gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence
validation.

Parallel subagents to run: none. The file set is small and edits would collide.

Tests/checkers/docs updated: public evidence bundle regression, deployment-doc assertions, public evidence
docs/status/audit wording, and stale verifier-wording cleanup so docs do not imply a nonexistent
service-content verifier.

Narrow validation passed: focused public evidence bundle publication/artifact test, focused public
deployment-doc test, and stale-wording `rg` checks for reused-artifact, seven-supporting-record, and
nonexistent verifier phrasing.

Broad validation passed: Gate 0 was first (`cargo test -p tensor_vm local_testnet --release`),
`cargo test -p tensor_vm --lib` with 559 passing tests, clippy with warnings denied,
`cargo fmt --all -- --check`, and `git diff --check`.

Expected observable evidence: otherwise complete signed public evidence cannot satisfy independently
checkable or full-spec gates when two required raw supporting-record kinds reuse the same artifact URI.

Out of scope: requiring canonical hosted paths or fetching artifact content.

Split trigger: split if URI diversity requires active artifact download or content hashing rather than
validating signed locator metadata.

Commit `452c57d` (`Validate public artifact URIs`) prepared on June 23, 2026.
Metadata commit `9195b3c` (`Record public artifact URI validation`) pushed to `origin/main` on
June 23, 2026.

### Iteration 206: Reward-Finality Formal Status Alignment

Feature capability: align formal proof-status documents with the current local reward-finality runtime:
pending receipt, proposer, challenge, audit-related, and credit reward ledgers exist locally with maturity,
voiding, and prune/nonpayment paths for implemented disputes, while the complete theorem remains blocked
on full verifier-transcript challenges, DA-through-window evidence, public deployed dispute propagation,
and formal proof discharge.

Readiness requirements covered: `upow.md` §12 and `mvp_spec.md` reward-finality wording require delayed
verifier-dependent settlement instead of immediate spendable rewards. Formal docs must not say that reward
state is "not started" when current code has pending reward ledgers, but they also must not over-claim the
full v0 theorem.

Canonical owner: formal proof-status docs under `docs/formal/` describe theorem readiness; chain reward
state remains owned by `crates/tensor_vm/src/chain/state.rs` and maturity release by
`crates/tensor_vm/src/chain/commands.rs`.

Adapter callers: reviewers, readiness docs, and future proof work consume these formal status docs.

Old shortcut being removed: stale formal wording described delayed reward finality as paper-only or
unimplemented, encouraging workaround thinking instead of acknowledging the existing pending-claim
runtime boundary.

Regression test that proves the shortcut is gone: documentation search rejects stale reward-finality
"not implemented"/"paper-only" status phrases for the current local reward state while preserving blocked
theorem wording.

Behavior with local synthetic block production disabled: unchanged; documentation-only iteration.

Behavior for producer and non-producer roles: unchanged; documentation-only iteration.

Structured evidence source: `PendingProposerReward`, `PendingReceiptReward`, `PendingChallengeReward`,
`PendingCreditReward`, reward release functions, and existing reward tests.

Finality source: unchanged; this iteration only distinguishes block ordering finality from reward
maturity/finality in proof docs.

Wire-size and codec boundary: no wire or codec changes.

Parallel subagents to run: none. This is a docs/status alignment slice with a small file set.

Tests/checkers/docs to add or update: formal proof obligations, adversary model, theorem dependency graph,
assumption discharge plan, and this exec plan.

Narrow validation commands: stale-phrase `rg` checks and `git diff --check`.

Broad validation commands before commit: Gate 0 was first; docs-only slice uses targeted doc hygiene unless
code changes appear.

Expected observable evidence: formal docs state the existing local pending-claim implementation accurately
and keep the full reward-finality theorem blocked until transcript/DA/public proof obligations are met.

Out of scope: adding new reward-state code, new verifier services, or proving full reward-finality theorem
completion.

Split trigger: split if inspection finds a real immediate-credit runtime path for verifier-dependent
rewards that requires code changes rather than documentation correction.

Validation evidence (June 23, 2026):

- Gate 0 first command passed: `cargo test -p tensor_vm local_testnet --release` ran five release lib
  local-testnet tests plus `local_testnet_service_gateway_does_not_produce_local_blocks`.
- `cargo test -p tensor_vm formal_status_docs_record_local_fallback_and_delayed_reward_evidence --lib`
  passed after extending the formal-status doc regression to cover the touched proof-status files and stale
  reward-finality phrases.
- Stale-phrase search passed:
  `rg -n "reward finality is paper-specified only|reward-finality state and challenge resolution are not implemented|RewardFinalityState.*Paper-specified.*implementation not started|miner/validator reward finality.*remain incomplete|Block proposer reward finality is partially discharged locally" docs/formal docs/tensorvm -g '*.md'`
  returned no matches.
- `cargo test -p tensor_vm --lib` passed: 559 passed, 0 failed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `b7c2ee3` (`Align reward finality proof status`).
- Validation metadata commit: `fda3a5a` (`Record reward finality status validation`).
- Push result: `git push origin main` succeeded on June 23, 2026, updating `main` from `225c215` to
  `fda3a5a`.

### Iteration 205: Public Service URL Diversity Gate

Feature capability: require deployed RPC, explorer, faucet, and telemetry service evidence to use distinct
signed public service-health URLs and service-content URLs before the public service gate can pass.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require deployed public
service evidence for the four public surfaces. The raw records should prove distinct public service
surfaces, not only distinct endpoint IDs and content roots attached to reused URLs.

Canonical owner: `PublicTestnetRunEvidence::evaluate` owns deployed public service admission for run
evidence.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the run/bundle report.

Old shortcut being removed: deployed service records could reuse the same public service-health URL across
multiple service kinds while retaining distinct endpoint IDs, content roots, signatures, and matching
content authorities.

Regression test that proves the shortcut is gone:
`public_testnet_run_evidence_requires_production_runtime_and_reachable_services` includes a recomputed
Explorer service/content pair that reuses the RPC public health URL and matching authority; individual
Explorer service evidence still passes, but deployed public services and public criteria fail.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: signed `service=...` and `service_content=...` manifest records.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public run evidence
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public run service regression and public evidence docs/status
wording.

Narrow validation commands: focused public run deployed-service test and public evidence manifest
round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise signed public evidence cannot satisfy deployed public services
when service kinds reuse the same public health/content URL.

Out of scope: proving real deployed service reachability or generating public run artifacts.

Split trigger: split if service URL diversity requires active HTTP probing rather than validating signed
records.

Validation evidence (June 23, 2026):

- `cargo fmt --all && cargo test -p tensor_vm public_testnet_run_evidence_requires_production_runtime_and_reachable_services --lib`
  passed; the focused regression covers reused public service URLs while individual Explorer evidence still
  validates.
- `cargo test -p tensor_vm --lib` passed: 559 passed, 0 failed.
- `cargo test -p tensor_vm local_testnet --release` passed: five release lib local-testnet tests and
  `local_testnet_service_gateway_does_not_produce_local_blocks` passed.
- `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release`
  passed.
- `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Implementation commit: `acad8dc` (`Validate public service URLs`).
- Validation metadata commit: `ebf5324` (`Record public service URL validation`).
- Push result: `git push origin main` succeeded on June 23, 2026, updating `main` from `7f09155` to
  `ebf5324`.

### Iteration 204: Public Network Runtime Endpoint Diversity Gate

Feature capability: require signed public network-runtime observations for counted public operators to use
distinct peer IDs and distinct public listen multiaddrs before they can satisfy the independently checkable
public evidence gate.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require exactly one valid
production-libp2p network observation per counted independent public miner/validator operator. The raw
records should prove distinct public node endpoints, not only distinct operator IDs with reusable endpoint
metadata.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns independently checkable public evidence
admission and network-runtime observation matching.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: counted public operators could present separate signed observation roots while
reusing the same public listen multiaddr, so the network-runtime gate proved operator IDs but not distinct
public libp2p endpoints.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_operational_records` includes recomputed signed
network-runtime summaries where two counted operators reuse one public listen multiaddr or one peer ID;
supporting artifacts still match, but network-runtime evidence and independent checkability fail.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: signed `network_runtime_observation=...` records with operator ID, peer ID,
public listen multiaddr, runtime counters, record root, and observation signature.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle network-runtime regression and public evidence
docs/status wording.

Narrow validation commands: focused public evidence publication/audit/network-runtime test and public
evidence manifest round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise signed public evidence is not independently checkable when counted
operators reuse a public peer ID or listen multiaddr.

Out of scope: proving real deployed peer reachability, changing libp2p runtime behavior, or generating
public run artifacts.

Split trigger: split if endpoint diversity requires active network probing rather than validating signed
raw observation records.

Validation evidence, June 23, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regressions:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_publication_and_audit_records --lib`
  passed, and
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib`
  passed after adding duplicate public listen multiaddr and duplicate peer ID cases.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `882b3e7` (`Validate public network endpoints`).
- Validation metadata commit: `b11d8cf` (`Record public network validation`).
- Push: `git push origin main` succeeded on June 23, 2026 (`127e542..b11d8cf  main -> main`).

### Iteration 203: Public Randomness Beacon Coverage Gate

Feature capability: require full-spec raw randomness-beacon evidence to cover each observed block exactly
once with distinct accepted public beacon rounds.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require the signed
randomness-beacon summary count to equal `observed_blocks`; raw records should prove per-block public
randomness coverage, not only a matching aggregate root.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
randomness record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: signed randomness summaries could aggregate records that repeated an
`observed_block` or beacon round while omitting another observed block, as long as the raw record count/root
matched.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_randomness_records` will include recomputed signed summaries
for duplicate observed blocks, skipped observed block coverage, and duplicate beacon rounds that still fail
full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw
`randomness_beacon_record=<source-id>,<round>,<randomness-root>,<proof-root>,drand-v1|validator-vrf-v1,<observed-block>,accepted`
records.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain separate
gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle raw randomness regressions and public evidence
docs/status wording.

Narrow validation commands: focused public evidence raw randomness test and public evidence manifest
round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when raw
randomness records repeat observed blocks or beacon rounds instead of covering the observed run.

Out of scope: proving live drand/VRF network availability, changing chain randomness, or generating public
run artifacts.

Split trigger: split if randomness validation requires deployed proof cryptography beyond the existing
proof-kind/root evidence.

### Iteration 202: Public Chain History Consistency Gate

Feature capability: require full-spec raw block-history and finality-history evidence to cover distinct
nonzero block roots, bind finality records to the same block roots, and match the finalized block count in
run evidence.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require public chain-history
records to support observed/finalized block counts, not just arbitrary raw record roots with matching
summary signatures.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
chain-history record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: signed block/finality summaries could aggregate distinct raw record roots that
duplicated block numbers, used zero block roots, disagreed between block-history and finality roots, or
reported finalized status counts inconsistent with `finalized_blocks`.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_chain_history_records` will include recomputed signed summaries
for duplicate block numbers, zero block roots, block/finality root mismatch, and finalized-count mismatch
that still fail full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw `block_history_record=<block>,<block-root>` and
`finality_history_record=<block>,<block-root>,finalized|unfinalized` records.

Finality source: unchanged; this checks public evidence consistency, not consensus finality rules.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle chain-history regressions and public evidence
docs/status wording.

Narrow validation commands: focused public evidence raw chain-history test and public evidence manifest
round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when
signed raw chain-history records do not prove distinct, root-consistent observed/finalized blocks.

Out of scope: proving real deployed block availability, changing chain finality, or generating public run
artifacts.

Split trigger: split if chain-history validation requires replaying deployed block bodies rather than raw
record consistency.

Validation evidence, June 23, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regression:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_chain_history_records --lib` passed.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `78e4981` (`Validate public chain history records`).
- Validation metadata commit: `aee3083` (`Record chain history validation`).
- Push: `git push origin main` succeeded on June 23, 2026 (`49783e0..aee3083  main -> main`).

### Iteration 201: Public Detection Measurement Field Gate

Feature capability: require full-spec raw deployed detection-measurement records to have a valid mechanism
label, nonzero subject root, nonzero sample count, and `detected_count <= sample_count` before their signed
aggregate can satisfy public evidence.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require deployed detection
measurements to be independently checkable raw records. Direct bundle construction should enforce the same
field semantics as line-oriented manifest parsing.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
operational record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: a signed detection-measurement summary could aggregate raw records with an
empty/unknown mechanism, zero subject root, zero sample count, or detected count exceeding sample count
when the bundle was constructed directly instead of through the manifest parser.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec` will include
malformed raw detection records whose recomputed signed summaries still fail full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw
`detection_measurement=<mechanism>,<subject-root>,<sample-count>,<detected-count>,<block>` records with
manifest-equivalent field validation.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle detection-measurement regressions and public
evidence docs/status wording.

Narrow validation commands: focused public evidence deployed detection-measurement test and public evidence
manifest round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when signed
raw detection-measurement records have malformed fields.

Out of scope: proving the real deployed detection-measurement source, generating external artifacts, or
changing chain reward-delay mechanics.

Split trigger: split if detection measurement validation needs deployed-run trace replay rather than raw
record field validation.

Validation evidence, June 23, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regression:
  `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec --lib` passed.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `134b0bb` (`Validate public detection measurement records`).
- Validation metadata commit: `03e2f70` (`Record detection measurement validation`).
- Push: `git push origin main` succeeded on June 23, 2026 (`33297c0..03e2f70  main -> main`).

### Iteration 200: Unique Public Settlement Receipt Coverage Gate

Feature capability: require full-spec raw invalid-work rejection and reward-settlement evidence to use
distinct nonzero receipt roots, and require reward-settlement raw records to bind nonzero miner and validator
IDs.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require invalid-work
rejection and reward-settlement supporting counts to represent deployed receipt events, not duplicated raw
record hashes over the same receipt.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
operational record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: signed invalid-work or reward-settlement summaries could aggregate distinct
raw record roots that repeated the same receipt root, or reward-settlement records with zero participant
IDs, while still satisfying count/root checks.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_operational_records` will include duplicate-receipt-root
invalid-work and reward-settlement cases whose recomputed signed summaries still fail full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw `invalid_work_rejection=<receipt-root>,rejected,<block>` and
`reward_settlement=<receipt-root>,<miner-id>,<validator-id>,<block>` records with distinct nonzero receipt
roots.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle raw operational regressions and public evidence
docs/status wording.

Narrow validation commands: focused public evidence raw operational test and public evidence manifest
round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when
signed raw invalid-work or reward-settlement records repeat the same receipt root, or when reward-settlement
participants are zero IDs.

Out of scope: proving the real deployed receipt set, generating external artifacts, or changing chain
reward-delay mechanics.

Split trigger: split if the check requires introducing a deployed receipt identity registry rather than
validating distinct raw operational receipt roots.

Validation evidence, June 23, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regression: `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib` passed.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `c655f62` (`Require unique public settlement receipts`).
- Validation metadata commit: `95d60da` (`Record public settlement receipt validation`).
- Push: `git push origin main` succeeded on June 23, 2026 (`91cb5e1..95d60da  main -> main`).

### Iteration 199: Unique Data Availability Receipt Coverage Gate

Feature capability: require full-spec public data-availability measurement evidence to cover distinct
nonzero receipt roots so checked-receipt coverage cannot be padded by multiple raw records for the same
receipt.

Readiness requirements covered: `mvp_spec.md` and `public_testnet_evidence.md` require public
data-availability measurement counts to match checked receipts. Coverage is per checked receipt, not only
per unique raw record hash.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
operational record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: a signed data-availability summary could aggregate distinct raw record roots
that repeated the same receipt root with different observed blocks, satisfying count/root checks without
proving checked-receipt coverage.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_operational_records` will include a duplicate-receipt-root
data-availability case whose recomputed signed summary still fails full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw
`data_availability_measurement=<receipt-root>,available|unavailable,<block>` records with distinct nonzero
`receipt-root` values.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain
separate gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle raw data-availability regression and public
evidence docs/status wording.

Narrow validation commands: focused public evidence raw operational test and public evidence manifest
round-trip.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, targeted release CLI
validation, and `cargo clippy -p tensor_vm --all-targets -- -D warnings`.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when
signed raw data-availability records repeat the same receipt root.

Out of scope: proving the real deployed receipt set, generating external artifacts, or changing chain
reward-delay mechanics.

Split trigger: split if the check requires introducing a deployed receipt identity registry rather than
validating distinct raw data-availability receipt roots.

Validation evidence, June 22, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regression: `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_operational_records --lib` passed.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `9bd11cb` (`Require unique data availability receipts`).
- Validation metadata commit: `8cf0f75` (`Record data availability receipt validation`).
- Push: `git push origin main` succeeded on June 22, 2026 (`2ccf292..8cf0f75  main -> main`).

### Iteration 198: Unique VRF Lifecycle Receipt Coverage Gate

Feature capability: require full-spec public validator VRF lifecycle evidence to cover distinct receipt
roots so lifecycle records cannot pad checked-receipt coverage with multiple revealed records for the same
receipt.

Readiness requirements covered: `upow.md` §10 and `mvp_spec.md` public evidence require deployed
commit-to-reveal lifecycle records covering checked receipts. Coverage is per receipt, not just per unique
record hash.

Canonical owner: `PublicTestnetEvidenceBundle::evaluate` owns full-spec public evidence admission and raw
lifecycle record checks.

Adapter callers: `tvmd public evidence validate`, checked evidence manifests, deployment examples, and
public evidence docs consume the bundle report.

Old shortcut being removed: a signed lifecycle summary could aggregate distinct raw record roots that all
referenced the same receipt root, satisfying count/root checks without proving checked-receipt coverage.

Regression test that proves the shortcut is gone:
`public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec` will include a
duplicate-receipt-root case whose recomputed signed lifecycle summary still fails full-spec evidence.

Behavior with local synthetic block production disabled: unchanged; this is a post-run public evidence
gate.

Behavior for producer and non-producer roles: unchanged; counted role behavior is observed through public
run evidence, not mutated by this validator.

Structured evidence source: typed raw
`validator_vrf_lifecycle=<receipt-root>,<validator-id>,<beacon-round>,revealed,<block>` records with
distinct nonzero `receipt-root` values.

Finality source: unchanged; signed run-window, block-history, and finality-history evidence remain separate
gates.

Wire-size and codec boundary: no p2p/consensus wire changes; this only tightens public evidence bundle
validation.

Parallel subagents to run: none. The decision log says not to spawn subagents without explicit delegation.

Tests/checkers/docs to add or update: public evidence bundle raw lifecycle regression and public evidence
docs/status wording.

Narrow validation commands: focused public evidence raw lifecycle test and public evidence manifest tests.

Broad validation commands before commit: `cargo fmt --all -- --check`, `git diff --check`,
`cargo test -p tensor_vm --lib`, `cargo test -p tensor_vm local_testnet --release`, and targeted release
CLI validation; broader workspace/clippy/tarpaulin if the implementation touches shared paths beyond the
bundle gate.

Expected observable evidence: otherwise complete full-spec public evidence remains non-full-spec when
signed raw lifecycle records repeat the same receipt root.

Out of scope: proving the real deployed receipt set, generating external artifacts, or changing chain
reward-delay mechanics.

Split trigger: split if the check requires introducing a deployed receipt identity registry rather than
validating distinct raw lifecycle receipt roots.

Validation evidence, June 22, 2026:
- Gate 0 first command: `cargo test -p tensor_vm local_testnet --release` passed.
- Focused regression: `cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec --lib` passed.
- Full library validation: `cargo test -p tensor_vm --lib` passed, 559 tests.
- Formatting and diff hygiene: `cargo fmt --all -- --check` and `git diff --check` passed.
- Release local-testnet validation: `cargo test -p tensor_vm local_testnet --release` passed after the patch.
- Release CLI evidence validation:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release` passed.
- Lint validation: `cargo clippy -p tensor_vm --all-targets -- -D warnings` passed.
- Commit: `08d8e57` (`Require unique VRF lifecycle receipts`).
- Push: `git push origin main` succeeded on June 22, 2026 (`2056689..9ff1804  main -> main`).

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
- Commit: `f320a4b` (`Require raw VRF lifecycle evidence`).
- Push: `git push origin main` succeeded on June 22, 2026 (`8534c6b..b069b2b  main -> main`).

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

- `tensorvm-verifier` is not a repository binary. Validation uses the `tvmd` CLI surfaces
  (`public evidence validate`, `localnet verify`), tests, clippy, and tarpaulin.
- Do not spawn subagents unless the user explicitly asks for delegation.
- Public/CUDA/deployed evidence remains blocked until real external infrastructure and CUDA kernels exist.
- Reward claims remain delayed and chain-owned; valid matured claims become spendable only through
  beneficiary `ClaimReward`, while voided/prunable claims may be swept without credit.

## Validation Evidence

- Current Iteration 208 Gate 0 release local-testnet validation passed first on June 23, 2026:
  `cargo test -p tensor_vm local_testnet --release` with the five release lib `local_testnet` tests and
  `local_testnet_service_gateway_does_not_produce_local_blocks` passing.
- Current Iteration 208 focused validation passed on June 23, 2026:
  focused public preflight manifest test covering the reused preflight service-URL regression.
- Current Iteration 208 deployment-doc validation passed on June 23, 2026:
  `cargo test -p tensor_vm public_deployment --lib` with three deployment-doc tests passing.
- Current Iteration 208 broad validation passed on June 23, 2026:
  `cargo test -p tensor_vm --lib` with 559 passing tests.
- Current Iteration 208 lint and hygiene validation passed on June 23, 2026:
  `cargo clippy -p tensor_vm --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Current Iteration 208 feature commit `3d4789f`
  (`Validate public preflight service URLs`) prepared on June 23, 2026.
- Current Iteration 208 validation metadata commit `2e52ef5`
  (`Record public preflight URL validation`) pushed to `origin/main` on June 23, 2026.
- Current Iteration 208 push result on June 23, 2026: `c822222..2e52ef5 main -> main`.
- Current Iteration 207 Gate 0 release local-testnet validation passed first on June 23, 2026:
  `cargo test -p tensor_vm local_testnet --release` with the five release lib `local_testnet` tests and
  `local_testnet_service_gateway_does_not_produce_local_blocks` passing.
- Current Iteration 207 focused validation passed on June 23, 2026:
  focused public evidence bundle publication/artifact test covering the reused supporting-artifact URI
  regression.
- Current Iteration 207 deployment-doc validation passed on June 23, 2026:
  `cargo test -p tensor_vm public_deployment --lib` with three deployment-doc tests passing.
- Current Iteration 207 stale wording validation passed on June 23, 2026:
  `rg` found no stale seven-supporting-record, reused-artifact, auditor/verifier, or service-content
  verifier wording in the checked docs/testnet paths.
- Current Iteration 207 broad validation passed on June 23, 2026:
  `cargo test -p tensor_vm --lib` with 559 passing tests.
- Current Iteration 207 lint and hygiene validation passed on June 23, 2026:
  `cargo clippy -p tensor_vm --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Current Iteration 207 feature commit `452c57d`
  (`Validate public artifact URIs`) prepared on June 23, 2026.
- Current Iteration 207 validation metadata commit `9195b3c`
  (`Record public artifact URI validation`) pushed to `origin/main` on June 23, 2026.
- Current Iteration 207 push result on June 23, 2026: `e1df583..9195b3c main -> main`.
- Current Iteration 203 focused validation passed on June 23, 2026:
  `cargo fmt --all && cargo test -p tensor_vm public_testnet_evidence_bundle_requires_raw_randomness_records --lib`.
- Current Iteration 203 hygiene validation passed on June 23, 2026:
  `cargo fmt --all -- --check` and `git diff --check`.
- Current Iteration 203 broad validation passed on June 23, 2026:
  `cargo test -p tensor_vm --lib` with 559 passing tests.
- Current Iteration 203 release local-testnet validation passed on June 23, 2026:
  `cargo test -p tensor_vm local_testnet --release` with the five release lib `local_testnet` tests and
  `local_testnet_service_gateway_does_not_produce_local_blocks` passing.
- Current Iteration 203 targeted release CLI validation passed on June 23, 2026:
  `cargo test -p tensor_vm --test tvmd_cli generated_public_evidence_manifest_round_trips_through_tvmd_validator --release`.
- Current Iteration 203 lint validation passed on June 23, 2026:
  `cargo clippy -p tensor_vm --all-targets -- -D warnings`.
- Current Iteration 203 feature commit `81e673c`
  (`Validate public randomness coverage`) prepared on June 23, 2026.
- Current Iteration 203 validation metadata commit `f80c181`
  (`Record public randomness validation`) pushed to `origin/main` on June 23, 2026.
- Current Iteration 203 push result on June 23, 2026: `2a1559a..f80c181 main -> main`.

## Archive

- Iteration 203: Public Randomness Beacon Coverage Gate. Full-spec raw randomness-beacon evidence now must
  cover each observed block exactly once with distinct accepted public beacon rounds. Commit `81e673c`
  (`Validate public randomness coverage`) pushed to `origin/main` on June 23, 2026.
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
