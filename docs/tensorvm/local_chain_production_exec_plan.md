# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. It is kept compact:
feature-sized iterations are summarized after validation and push, and older details move to Archive.

## Current State

- Latest completed feature: Iteration 20, finalized-beacon consensus randomness binding, is implemented and
  validated as `1f2b74d` (`Bind consensus randomness to finalized beacon`). Blocks now carry a persisted
  `beacon_round`, chain state commits to finalized/genesis beacon rounds, validator/miner assignments and
  Freivalds validation seeds are derived from chain-owned finalized-beacon seed helpers, block check leaves
  include the finalized beacon round/value and parent commitment, and status exposes parent/child beacon
  rounds plus a `block_uses_parent_finalized_beacon` evidence field.
- Previous completed feature: Iteration 19, canonical child-state block roots and block-opening evidence, is
  implemented and validated as `232256d` (`Add canonical block apply openings`). Useful-verification blocks
  now commit to the child state root, use Merkle-openable selected-receipt and checks roots, expose parent
  snapshot plus child apply outcome evidence, and reject the old parent-root-as-block-state shortcut.
- Previous completed feature: Iteration 18, UVPoW retargeting and explicit zero-receipt fallback mode, is
  implemented and validated as `af33fe1` (`Add UVPoW retarget fallback mode`). Blocks now carry an explicit
  production kind, validators derive the expected difficulty target from parent block history and consensus
  parameters, zero selected receipts produce a PoW-skip fallback block instead of masquerading as useful
  PoW, and `tvmd node block` exposes block kind, expected target, retarget params, and fallback evidence
  fields.
- Previous completed feature: Iteration 17, role-owned live counter checker hardening, is implemented,
  validated, and pushed as `d4a6182d19bb1e1ea1f63174d8df7eb657cd6dd4`
  (`Harden role-owned local checker evidence`) on `origin/main`. The local Docker checker now requires
  hard evidence that at least one miner role reports positive live receipt/tensor submissions and at least
  one validator role reports positive live attestation submissions. This consumes existing structured
  `tvmd node status` fields; it does not move consensus logic into the shell checker.
- Previous completed feature: Iteration 16, role-owned live work before validator proposal, is implemented,
  validated, and pushed as `e18d5b3d5e87ee3d0eb71266bb1f50b11ce42171`
  (`Publish local jobs before role-owned work`) on `origin/main`. The scheduled local producer now
  publishes synthetic jobs only. Receipts and attestations are left to miner/validator role paths before
  validator block proposal. Focused runtime tests prove a producer tick does not create receipts or
  attestations and prove the job -> miner receipt -> validator attestation -> validator proposal path with
  existing role-owned helpers.
- Previous completed feature: Iteration 15, validator-owned local block proposal tick, is implemented,
  validated, and pushed as `0d7debcdb94ef50493e2f2926d4f3dc5983fcbd4`
  (`Add validator-owned block proposal tick`) on `origin/main`. This iteration splits local
  synthetic work generation from scheduled validator block proposal. The runtime scheduled producer now
  calls a validator-role helper that prepares parent state, applies `ChainCommand::ProduceBlock`, publishes
  the resulting block payload/header/hash, and leaves finality to explicit validator block votes. The
  remaining shortcut is synthetic local job/receipt/attestation generation in the producer process; full
  role-owned miner/validator work remains the next gap.
- Latest completed feature: Iteration 14, validator-owned local timed producer topology, is implemented,
  validated, and pushed as `1d556efafd1443809406dcaa54bdc3aa63c68b9e`
  (`Move local producer to validator runtime`) on `origin/main`. This iteration moves the single local
  timed producer from `miner-00/proposer_run` to `validator-00/validator_run`, makes local timed producer
  capability validator-only, and keeps full role-owned validator block assembly as the next gap.
- Required resumed Gate 0 was run first: `cargo test -p tensor_vm local_testnet --release` passed with
  5 release local-testnet library tests and the seed CLI integration test.
- Iteration 11 feature and evidence commits are on `origin/main`: `e6129d1915562a1e865579e347d8cfb85855089e`
  and `800b031edea9b0b268cfe1fb487c9628cb2c782c`.
- Iteration 10 was implemented and pushed as `2d6609e Add remote validator tensor fetch`, with follow-up
  evidence commit `1687f86 Record iteration 10 push evidence`. Later proof/doc commits landed on top:
  `e20a879`, `41a20aa`, and `07f2b05`.
- Standing blockers:
  - The full Docker runtime gate remains unresolved. The latest recorded `check-local-testnet.sh` run
    against an already-running Compose cluster failed at the bounded gateway `/health` probe with
    `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.

## Readiness Matrix

| Capability | Status | Current evidence | Next action |
| --- | --- | --- | --- |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Keep one transition engine while replacing block validity |
| Role-owned miner receipts | Checker hardening implemented/pushed | Miner role submits receipts through `ChainCommand::SubmitReceipt` and publishes receipt announcements; `d4a6182` makes the Docker checker fail unless live miner receipt/tensor counters are positive | Rerun full Docker checker after `/health` blocker clears |
| Role-owned validator attestations | Checker hardening implemented/pushed | Validator role verifies assigned receipts, fetches missing tensors remotely, and submits attestations; `d4a6182` makes the Docker checker fail unless live validator attestation counters are positive | Keep as input path for canonical blockspace and rerun full Docker checker after `/health` blocker clears |
| Role-owned validator block votes | Implemented/pushed | `fb0feb0`; validator role submits `SubmitBlockVote`, gossips block-vote payloads, and status/checker fields expose submitted/ingested/applied vote counters | Rerun full Docker checker after `/health` blocker clears |
| Remote tensor availability | Implemented/pushed | `2d6609e`; root-addressed tensor request-response and validator fetch counters | Reuse for block-check evidence; revisit slow-peer bounds later |
| Network-visible event ingestion | Implemented/pushed | `fb0feb0`; node runtime ingests decoded jobs, receipts, attestations, block payloads, and block-vote payloads; headers/hashes are announcements only | Rerun full Docker checker after `/health` blocker clears |
| Proposer/block production | Validator proposal tick started | Iteration 15; scheduled runtime production uses `submit_validator_role_block_proposal`, publishes block payload/header/hash, and tests prove proposal does not synthesize finality | Keep validator-owned proposal while removing remaining producer-local work synthesis |
| Role-owned live work before proposal | Implemented/pushed | `e18d5b3`; scheduled production publishes jobs only, role-owned runtime tests cover miner receipt and validator attestation before proposal | Rerun full Docker checker after `/health` blocker clears |
| Canonical useful-verification block validity | Finalized-beacon binding implemented locally | Blocks carry selected-root/checks-root/beacon-round/beacon/target/nonce plus explicit production kind; strict vote validation checks child state root, parent finalized beacon, parent-derived target, useful-PoW or fallback mode, proposer, selected receipts, Merkle-openable finalized-beacon-bound checks, attestation, and child reward roots; `1f2b74d` rejects block-hash beacon randomness | Add VRF/drand or commit-reveal beacon messages, persisted historical parent snapshots, challenge transaction/dispute flow, timeout rotation, and full fallback reward economics |
| Checker evidence | Updated/pushed | `tvmd node block` exposes PoW, canonical blockspace, checks-root, validator-proposer, finality-validation, and block-vote stake/validator evidence; `d4a6182` adds exact role-owned miner/validator live counter gates | Full Docker checker still awaits `/health` blocker resolution |
| Restart/recovery matrix | Complete for current storage model | Rolling restart checker covers durable state/common head for current block model | Rerun after block serialization changes |
| Public deployment evidence | Not started | Public evidence fields still report incomplete independently-checkable status | Keep out of scope until local canonical path is stable |

## Recent Iterations

### Iteration 20: Finalized-Beacon Consensus Randomness Binding

Feature capability:
Make the consensus randomness source explicit and chain-owned for the current local v0 path. Blocks now
carry a persisted finalized `beacon_round`, state roots commit to finalized/genesis beacon rounds, validator
and miner assignment seeds come from canonical chain helpers, validation seeds bind the finalized beacon
round/value, receipt id, job id, validator id, and validation round, and block check leaves include the
finalized beacon commitment.

Checkpoint before edits:
- Canonical owner: `chain::validation`, `chain::blocks`, `chain::roots`, `ChainState`, and `TensorBlock`
  own finalized-beacon seed derivation, blockspace ordering, checks-root binding, block admission, and
  persistence of beacon round state.
- Adapter callers: miner role, validator role, localnet, local harness, P2P, storage, and status call
  chain-owned seed helpers or encode canonical block/state fields; they do not derive assignment or
  validation randomness from raw local state.
- Old shortcut removed: validator assignment, miner assignment, validation seed derivation, block checks,
  and child beacon advancement no longer use generic raw finalized randomness or block-hash-derived beacon
  advancement in the consensus path.
- Regression tests: produced blocks use the parent finalized beacon rather than their own hash; mutated
  block-hash beacons are rejected; checks roots are bound to the finalized beacon round; validation seeds
  are bound to validator identity and beacon round.
- Local synthetic disabled behavior: unchanged; profiles without synthetic jobs still do not synthesize
  local work.
- Producer/non-producer behavior: producers and non-producers validate the same beacon round/value,
  selected receipt set, and checks-root commitments through the chain path.
- Structured evidence source: `tvmd node block` reports `beacon_round`, `beacon`, `parent_beacon_round`,
  `parent_beacon`, `child_beacon_round`, `child_beacon`, and `block_uses_parent_finalized_beacon`.
- Finality source: signed validator `BlockVote`s through `SubmitBlockVote`, unchanged.
- Wire-size and codec boundary: fixed `TensorBlock` payload and chain-state/snapshot codecs now include
  beacon round fields; P2P wire and storage roundtrip tests were updated.

Implementation summary:
- Added persisted finalized/genesis beacon rounds to `ChainState` and a `beacon_round` field to
  `TensorBlock`, with state root, fixed block payload, chain-state store, and snapshot encoding updates.
- Replaced child beacon advancement with `next_finalized_beacon(round, beacon, height, epoch)`, avoiding
  parent/current block hash as beacon entropy.
- Added chain-owned assignment and validation seed helpers; runtime and local harness paths now use those
  helpers for miner assignment, validator assignment, and Freivalds validation.
- Bound block check leaves to finalized beacon round/value and parent commitment through a canonical
  `block_check_seed`.
- Extended block status and CLI tests with parent/child beacon round evidence and parent finalized-beacon
  verification.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `cargo fmt && cargo test -p tensor_vm --lib block -- --nocapture`: 39 filtered block/codec/storage/P2P
  tests passed.
- `cargo test -p tensor_vm --lib proposer -- --nocapture`: 8 filtered proposer/reward/parser tests passed.
- `cargo test -p tensor_vm --test tvmd_cli`: 8 integration tests passed.
- `cargo test -p tensor_vm --lib`: 318 library tests passed.
- `cargo test -p tensor_vm`: 318 library tests, 1 local CPU Compose test, 8 `tvmd_cli` integration tests,
  28 `tvmd_runtime` integration tests, and doctests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.

Push evidence:
- Feature commit `1f2b74d` (`Bind consensus randomness to finalized beacon`) is recorded for push with
  this evidence update.

Out of scope:
- Full VRF, drand, or commit-reveal network message lifecycle and timeout handling.
- Challenge transaction/dispute flow, reward clawback, slashing, and fallback rotation economics.
- Persisted historical parent snapshot storage for replaying arbitrary old blocks from disk.
- Full Docker checker rerun while the standing gateway `/health` timeout blocker remains unresolved.

### Iteration 19: Canonical Child-State Block Roots and Block-Opening Evidence

Feature capability:
Move useful-verification block validity from parent-root and aggregate-check shortcuts toward replayable
parent-basis validation. Blocks now commit to the child state root after applying the canonical selected
receipt set, and status exposes parent snapshot, child roots, selected receipt openings, and checks-root
openings.

Checkpoint before edits:
- Canonical owner: `chain::blocks`, `chain::roots`, `ChainParams`, and `TensorBlock` own parent-basis
  validation, selected receipt commitments, checks-root commitments, child apply, block admission, and
  finality preconditions.
- Adapter callers: validator runtime, P2P, storage, and status continue to pass through canonical blocks;
  `tvmd node block` observes parent/child/opening evidence without deciding validity.
- Old shortcut removed: useful-verification block `state_root` can no longer be the parent state root, and
  `checks_root` is no longer a non-openable aggregate over selected receipt ids.
- Regression tests: block apply outcome exposes parent/child roots and verifies selected receipt plus check
  Merkle proofs; block validation rejects a useful block whose `state_root` is reset to the parent root.
- Local synthetic disabled behavior: unchanged; profiles without synthetic jobs still do not synthesize
  local work.
- Producer/non-producer behavior: producers and non-producers both validate through the same
  `chain::blocks` path and derive expected roots from the current parent basis.
- Structured evidence source: `tvmd node block` reports parent snapshot roots, child state/reward roots,
  recomputation booleans, opening counts, selected receipt leaf ids, selected receipt leaf roots, and checks
  leaf roots.
- Finality source: signed validator `BlockVote`s through `SubmitBlockVote`, unchanged.
- Wire-size and codec boundary: no block wire/body codec change in this slice; evidence is reconstructed
  from canonical chain state and status paths.

Implementation summary:
- Added `BlockApplyOutcome`, `BlockParentSnapshot`, and `SelectedReceiptOpening` to expose the canonical
  parent snapshot, selected receipt commitment, checks-root commitment, child apply result, and Merkle proof
  metadata.
- Replaced block-level selected receipt and checks commitments with Merkle roots over receipt/check leaves
  that include receipt metadata and canonical receipt checks.
- Block production and admission now compute the block `state_root` from the child state after applying
  selected receipts, and validation rejects parent-root-as-child-state blocks.
- Block status now reports parent snapshot roots, child roots, recomputation flags, opening counts, and leaf
  roots for independent local inspection.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `cargo fmt && cargo test -p tensor_vm --lib block -- --nocapture`: 36 filtered block/codec/storage/P2P
  tests passed.
- `cargo test -p tensor_vm --lib`: 314 library tests passed.
- `cargo fmt && cargo test -p tensor_vm --test tvmd_cli`: 8 integration tests passed.
- `cargo test -p tensor_vm`: 314 library tests, 1 local CPU Compose test, 8 `tvmd_cli` integration tests,
  28 `tvmd_runtime` integration tests, and doctests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `git diff --check`

Push evidence:
- Feature commit `232256d` (`Add canonical block apply openings`) is recorded for push with this evidence
  update.

Out of scope:
- Persisted historical parent snapshot storage for replaying arbitrary old blocks from disk.
- Cross-validator challenge transaction flow, dispute networking, reward clawback, and slashing.
- Timeout-based stake rotation after withheld useful-PoW proposals and full fallback reward economics.
- Full Docker checker rerun while the standing gateway `/health` timeout blocker remains unresolved.

### Iteration 18: UVPoW Retargeting and Zero-Receipt Fallback Mode

Feature capability:
Remove the fixed global useful-PoW target shortcut and stop treating empty selected receipt sets as normal
useful work. Validators now derive each block target from parent block history and configured retarget
parameters, and zero-receipt blocks use an explicit PoW-skip fallback production kind.

Checkpoint before edits:
- Canonical owner: `ChainEngine`, `chain::blocks`, `ChainParams`, and `TensorBlock` own target derivation,
  fallback mode validity, block admission, and finality preconditions.
- Adapter callers: validator runtime, P2P, storage, and status pass through canonical block payloads and
  display evidence; they do not decide fallback validity.
- Old shortcut removed: `useful_pow_difficulty_target()` as a fixed production and validation policy; empty
  blockspace no longer claims normal useful-PoW.
- Regression tests: retarget boundary changes target, non-boundary heights reuse parent target, zero
  receipts produce explicit fallback, and useful/fallback kinds cannot be swapped across empty/nonempty
  canonical blockspace.
- Local synthetic disabled behavior: public/mainnet-style profiles still have no synthetic jobs; zero
  selected receipts use fallback liveness, not synthetic work.
- Producer/non-producer behavior: validators produce blocks; all nodes validate target and production kind
  through the same block admission path.
- Structured evidence source: `tvmd node block` reports `block_kind`, `pow_skip_fallback`,
  `fallback_valid`, `expected_difficulty_target`, retarget params, `pow_required`, raw PoW fields, selected
  receipts, checks root, and finality fields.
- Finality source: signed validator `BlockVote`s through `SubmitBlockVote`, unchanged.
- Wire-size and codec boundary: block payloads add a one-byte production-kind field; fixed block codec,
  block log, P2P wire fixtures, and chain-state parameter persistence were updated.

Implementation summary:
- Added consensus difficulty parameters to `ChainParams` and persisted them in `chain.state`.
- Added `BlockProductionKind::{UsefulVerificationPow, PowSkipFallback}` to block headers and hash/Pow
  domains, with fixed payload codec and P2P fixture updates.
- Replaced the fixed target helper with parent-history retargeting using configured target block time,
  epoch length, maximum ratio, and target floor/ceiling.
- Production emits fallback blocks with nonce zero when canonical blockspace selects no receipts; validation
  rejects useful-PoW blocks with empty selection, fallback blocks with nonempty selection, wrong dynamic
  targets, and fallback blocks with nonzero nonce.
- Block status now distinguishes useful-PoW from fallback and reports expected target/retarget fields.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `cargo test -p tensor_vm --lib block -- --nocapture`: 34 filtered block/codec/storage/P2P tests passed.
- `cargo test -p tensor_vm --lib`: 312 library tests passed.
- `cargo test -p tensor_vm --test tvmd_cli`: 8 integration tests passed.
- `cargo test -p tensor_vm`: 312 library tests, 1 local CPU Compose test, 8 `tvmd_cli` integration tests,
  28 `tvmd_runtime` integration tests, and doctests passed.
- `cargo clippy --workspace --all-targets -- -D warnings`
- Final release Gate 0: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `git diff --check`

Push evidence:
- Feature commit `af33fe1` (`Add UVPoW retarget fallback mode`) is recorded for push with this evidence
  update.

Out of scope:
- Timeout-based stake rotation after withheld useful-PoW proposals; this slice implements deterministic
  zero-receipt fallback eligibility.
- Full reduced proposer reward policy for fallback blocks.
- Exact historical parent snapshots and child-state apply theorem.
- Full Docker checker rerun while the standing gateway `/health` timeout blocker remains unresolved.

### Iteration 17: Role-Owned Live Counter Checker Hardening

Feature capability:
Make the local Docker checker reject zero role-owned miner receipt/tensor counters and zero role-owned
validator attestation counters after the Iteration 16 job-only producer split. This turns existing runtime
status counters into hard local-readiness evidence instead of accepting that the fields exist.

Checkpoint before edits:
- Canonical owner: role loops and `NodeRuntimeState` own receipt, tensor, and attestation counters; the
  chain engine owns admission of jobs, receipts, attestations, blocks, and finality.
- Adapter callers: `check-local-testnet.sh` reads `tvmd node status` and emits evidence fields only after
  convergence; it does not mutate chain state.
- Old shortcut removed from evidence: present-but-zero `role_miner_receipts_submitted`,
  `role_miner_tensors_inserted`, and `role_validator_attestations_submitted` no longer satisfy local
  readiness.
- Regression test: `local_cpu_compose_bundle_matches_spec_artifact_shape` asserts the checker contains the
  hard-fail gates and exact output fields.
- Local synthetic disabled behavior: profiles without synthetic jobs still have no outbound synthetic work;
  inbound status fields remain typed and numeric.
- Producer/non-producer behavior: producer policy remains unchanged; the checker counts miner and validator
  role-owned work independently from `validator-00` timed block production.
- Structured evidence source: `role_miner_receipts_submitted`, `role_miner_tensors_inserted`, and
  `role_validator_attestations_submitted` from `tvmd node status`.
- Finality source: signed validator `BlockVote`s through `SubmitBlockVote`, unchanged.
- Wire-size and codec boundary: no codec or wire-size changes.

Implementation summary:
- The local checker now accumulates live miner receipt/tensor operator counts and totals plus live
  validator attestation operator counts and totals during the existing all-operator convergence loop.
- The checker fails if no miner role reports positive receipt submissions, no miner role reports positive
  tensor inserts, no validator role reports positive attestation submissions, or any corresponding total
  remains zero.
- The checker emits exact `live_role_*` evidence fields and boolean role-owned work gates.
- The Compose artifact-shape test now asserts these hard gates and output fields exist.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `bash -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --exact`
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tensor_vm --test tvmd_runtime runtime_roles`: 7 tests passed.
- `cargo test -p tensor_vm --tests`: 308 library tests, 1 local CPU Compose test, 8 `tvmd_cli`
  integration tests, and 28 `tvmd_runtime` integration tests passed.
- `cargo test --workspace --release`: 14 `experiments` tests, 308 `tensor_vm` library tests, 1
  `local_cpu_compose` test, 8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, 1
  `tensor_vm_explorer` library test, 2 `tensorvm_explorer` CLI tests, and doctests passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `git diff --check`
- `cargo tarpaulin --version` remains unavailable in this environment (`cargo` reports no such command:
  `tarpaulin`), so coverage was not collected.

Push evidence:
- Feature commit `d4a6182d19bb1e1ea1f63174d8df7eb657cd6dd4` (`Harden role-owned local checker
  evidence`) was pushed to `origin/main`.

Out of scope:
- Retargeting and zero-receipt PoW-skip fallback.
- Full Docker checker rerun while the standing gateway `/health` timeout blocker remains unresolved.
- Requiring all 10 miners or all 5 validators to report positive role-owned work in one window; the current
  hard gate requires positive live role-owned work from at least one miner role and at least one validator
  role.

### Iteration 16: Role-Owned Live Work Before Validator Proposal

Feature capability:
Replace the scheduled producer-local synthetic work path with job-only publication. The local producer may
create and gossip deterministic local jobs, but it no longer creates miner receipts, validator
attestations, settlement, or model transitions before proposing. Existing miner and validator role helpers
own receipt and attestation creation before validator proposal.

Checkpoint before edits:
- Canonical owner: `ChainEngine` owns job admission, receipt/attestation admission, settlement
  preparation, block proposal, and finality.
- Adapter callers: runtime production can submit one synthetic local job and request validator proposal;
  miner/validator role loops submit receipts and attestations.
- Old shortcut removed from scheduled runtime: `produce_and_publish_synthetic_work` is no longer called by
  `LocalProductionSchedule`.
- Regression tests: scheduled production adds jobs but no receipts/attestations; a role-owned pipeline test
  creates the receipt and attestation afterward and proposes over the settled receipt.
- Local synthetic disabled behavior: profiles without synthetic jobs still skip outbound job production;
  inbound network and role loops remain unchanged.
- Producer/non-producer behavior: producer policy controls outbound job/block creation only.
- Structured evidence source: role miner/validator counters, chain job/receipt/attestation counts, and
  block selected-receipt evidence.
- Finality source: signed validator `BlockVote`s through `SubmitBlockVote`.
- Wire-size/codec boundary: existing bounded job/receipt/attestation/block payload codecs are reused.

Implementation summary:
- Added `produce_and_publish_synthetic_job`, which uses the profile job source to submit only a job and
  publish job announcements. For LinearTrainingStep jobs it registers only the required synthetic model
  metadata so later chain-owned settlement can apply the transition.
- Changed `LocalProductionSchedule` to call the job-only helper before validator block proposal.
- Added focused runtime tests proving scheduled production does not create producer-local receipts or
  attestations and proving a producer-published job can be receipted, attested, and proposed by role-owned
  helpers.

Validation:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tensor_vm --test tvmd_runtime runtime_roles`: 7 tests passed.
- `cargo test -p tensor_vm --test tvmd_runtime miner_role`: 4 tests passed.
- `cargo test -p tensor_vm --test tvmd_runtime validator_role`: 6 tests passed.
- `cargo test -p tensor_vm --test tvmd_cli validator_run_with_local_producer_advances_cpu_chain`: passed.
- `cargo test -p tensor_vm --tests`: 308 library tests, 1 local CPU Compose test, 8 `tvmd_cli`
  integration tests, and 28 `tvmd_runtime` integration tests passed.
- `cargo test --workspace --release`: 14 `experiments` tests, 308 `tensor_vm` library tests, 1
  `local_cpu_compose` test, 8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, 1
  `tensor_vm_explorer` library test, 2 `tensorvm_explorer` CLI tests, and doctests passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `git diff --check`
- `cargo tarpaulin --version` remains unavailable in this environment (`cargo` reports no such command:
  `tarpaulin`), so coverage was not collected.

Push evidence:
- Feature commit `e18d5b3d5e87ee3d0eb71266bb1f50b11ce42171` (`Publish local jobs before role-owned
  work`) was pushed to `origin/main`.

Out of scope:
- Full Docker checker hardening for positive live role-owned counters.
- Removing localnet/reference full synthetic round helpers.
- Difficulty retargeting, zero-receipt fallback, public deployment evidence, CUDA, seven-day run, and the
  full Docker `/health` blocker.

### Iteration 15: Validator-Owned Local Block Proposal Tick

Feature capability:
Split scheduled local production so synthetic work generation and validator block proposal are separate
steps. The timed validator runtime no longer calls the all-in-one synthetic round helper for block
assembly; after optional local synthetic work generation it calls a validator-role block proposal helper
that prepares chain-owned parent state and applies `ChainCommand::ProduceBlock`.

Checkpoint before edits:
- Canonical owner: `ChainEngine`/`chain::blocks` own settlement preparation, canonical blockspace,
  `checks_root`, useful-verification PoW, block append, and finality validation.
- Adapter callers: the validator runtime schedule may trigger local synthetic work generation and may ask
  the validator role to propose one block. P2P adapters only publish the resulting canonical block payload,
  header, and hash.
- Old shortcut narrowed: scheduled runtime production no longer uses
  `produce_synthetic_cpu_round_with_profile` to create work and block in one call.
- Regression tests: validator role block proposal works from settled state and leaves blocks unfinalized
  until explicit `SubmitBlockVote`; localnet reference round behavior remains covered.
- Local synthetic disabled behavior: profiles without synthetic jobs still skip synthetic work generation;
  inbound network ingest and role work are unchanged.
- Producer/non-producer behavior: producer capability controls outbound scheduled proposal only.
- Structured evidence source: `role_produced_blocks`, `role_local_producer`, block PoW/canonical fields,
  and block-vote finality fields.
- Finality source: signed validator `BlockVote`s admitted through `SubmitBlockVote`.
- Wire-size/codec boundary: existing bounded block payload codec and gossip messages are reused.

Implementation summary:
- Added `SyntheticCpuWorkResult` and `produce_synthetic_cpu_work_with_profile` so local synthetic work can
  stop before block assembly while the older full synthetic round helper remains available for reference
  tests.
- Added `submit_validator_role_block_proposal`, which requires a registered validator wallet, calls
  `prepare_block_parent_state`, and applies `ChainCommand::ProduceBlock`.
- Changed `LocalProductionSchedule` to publish optional synthetic work first, then produce and publish the
  block through the validator-role proposal helper.
- Added a validator runtime test proving proposal uses settled state and does not synthesize finality votes.

Validation so far:
- Required Gate 0 first: `cargo test -p tensor_vm local_testnet --release` passed with 5 release
  local-testnet library tests and the seed CLI integration test.
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tensor_vm --test tvmd_runtime validator_role`: 6 tests passed.
- `cargo test -p tensor_vm --lib localnet::tests`: 9 tests passed.
- `cargo test -p tensor_vm --test tvmd_cli validator_run_with_local_producer_advances_cpu_chain`: passed.
- `cargo test -p tensor_vm --tests`: 308 library tests, 1 local CPU Compose test, 8 `tvmd_cli`
  integration tests, and 26 `tvmd_runtime` integration tests passed.
- `cargo test --workspace --release`: 14 `experiments`, 308 `tensor_vm`, 8 `tvmd_cli`, 26
  `tvmd_runtime`, 1 local CPU Compose, 3 `tensor_vm_explorer`, and doc-test targets passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `git diff --check`
- `cargo tarpaulin --workspace --offline` was blocked because this environment does not have the
  `cargo-tarpaulin` subcommand installed (`error: no such command: tarpaulin`).

Push evidence:
- Feature commit: `0d7debcdb94ef50493e2f2926d4f3dc5983fcbd4`
  (`Add validator-owned block proposal tick`).
- Remote/branch: `origin/main`.
- Push result: `6d551f1..0d7debc  main -> main`.

Out of scope:
- Requiring positive live Compose miner/validator-owned submissions.
- Replacing local synthetic job/receipt/attestation generation with fully role-owned interprocess work.
- Difficulty retargeting, zero-receipt fallback, public deployment evidence, CUDA, seven-day run, and the
  full Docker `/health` blocker.

### Iteration 14: Validator-Owned Local Timed Producer

Feature capability:
Move the single local timed producer away from the miner/proposer shortcut and onto a validator runtime
running `validator_run`, while preserving the existing shared chain APIs and network-visible block/vote
surfaces. This is an incremental topology/policy slice; it does not yet replace the remaining synthetic
round helper with a fully role-owned block assembly tick.

Checkpoint before edits:
- Canonical owner: `ChainEngine`/chain modules still own settlement, validator proposer checks,
  useful-verification PoW block production, block admission, and block-vote finality.
- Adapter callers: role loops may call shared runtime helpers and publish through the existing p2p/event
  path; `tvmd` must not mark finality or bypass chain validation.
- Old shortcut narrowed: Compose and checker evidence must stop blessing `miner-00` as a `proposer_run`
  block producer. The single live local producer should be a registered validator running `validator_run`.
- Regression tests: miners cannot produce local blocks; validators can only become local producers when the
  local CPU producer flag and interval are enabled; non-producer roles still ingest block/vote payloads.
- Local synthetic disabled behavior: inbound network/RPC work remains independent of the local timed
  producer flag. No new synthetic work is enabled for miners.
- Producer/non-producer behavior: producer capability controls outbound timed production only. Miners
  remain non-producers; validators and non-producers continue to vote/apply blocks through role/network
  paths.
- Structured evidence source: use existing status fields (`role_loop_role`, `role_wallet_registration`,
  `role_can_produce_blocks`, `role_local_producer`, `role_produced_blocks`) and block-view fields
  (`proposer_role`, `proposer_registered`, `tensorwork_proposer_selection`, `pow_valid`,
  `canonical_blockspace_valid`, block-vote stake/validators) without adding unsupported ownership claims.
- Finality source: finality remains signed validator `BlockVote`s admitted through `SubmitBlockVote`, not
  block append or producer-local synthesis.
- Wire-size/codec boundary: reuse existing bounded block and block-vote payload codecs.

Files/modules likely touched:
- `crates/tensor_vm/src/profile.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- `crates/tensor_vm/tests/local_cpu_compose.rs`
- `deploy/tensorvm/local-cpu/docker-compose.yml`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- Readiness/status docs.

Parallel subagents launched before implementation:
- Readiness mapper, codebase explorer, test coverage explorer, and checker/status explorer completed
  read-only passes and confirmed the current miner/proposer shortcut, status evidence limits, and safest
  incremental validator-producer scope.

Out of scope:
- Replacing `produce_synthetic_cpu_round_with_profile` with a clone-and-commit proposer tick.
- Adding new proposer ownership counters for exact block/wallet correlation.
- Public deployment evidence, challenge openings, retargeting, CUDA, seven-day run, and the full Docker
  `/health` blocker.

Validation plan:
- Focused: profile/main role policy tests, `tvmd_cli` role and service-surface tests, and
  `local_cpu_compose_bundle_matches_spec_artifact_shape`.
- Broad before commit: `cargo fmt --check --all`, `cargo check -p tensor_vm --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p tensor_vm local_testnet
  --release`, `cargo test -p tensor_vm --tests`, Compose config, `cargo tarpaulin --workspace --offline`
  if coverage remains stable, and `git diff --check`.

Implementation summary:
- `NodeConfig::can_produce_local_blocks` is validator-only. `service serve`, miners, and the legacy
  `proposer_run` surface no longer become local timed producers from a block interval.
- The runtime now requires the explicit `TENSORVM_LOCAL_CPU_ROLE_PRODUCER=true` flag in addition to a local
  CPU block interval; interval-only service runs remain non-producing.
- Compose moved the single local timed producer env from `miner-00/proposer_run` to
  `validator-00/validator_run`. Miners all run `miner_run`; validators all run `validator_run`.
- Checker and artifact tests now require `validator-00` as the only local producer, miners with no
  block-production capability, non-producer network application, `local_validator_producer=true`, and
  `local_proposer_runtime=false`.
- CLI coverage now proves `service serve` does not produce local blocks even with producer env vars, and
  `validator run` with the producer flag advances the seeded local CPU chain.
- Docs now state that this removes the miner/proposer topology shortcut but leaves full role-owned
  validator block assembly as a remaining gap.

Validation passed:
- `sh -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Focused `tvmd` binary tests for role policy, loop config binding, and wallet registration.
- Focused `tvmd_cli` tests:
  `local_testnet_service_gateway_does_not_produce_local_blocks`,
  `validator_run_with_local_producer_advances_cpu_chain`, and
  `role_run_commands_serve_through_role_specific_surfaces`.
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `cargo test -p tensor_vm --test tvmd_cli`: 8 tests passed.
- `cargo test -p tensor_vm --bin tvmd`: 22 tests passed.
- `cargo test -p tensor_vm --tests`: 247 library tests, 22 `tvmd` binary tests, 1 local CPU Compose
  integration test, and 8 `tvmd_cli` integration tests passed.
- `cargo test -p tensor_vm local_testnet --release`: 5 release local-testnet library tests and the
  `local_testnet_service_gateway_does_not_produce_local_blocks` CLI integration passed.
- `cargo test --workspace --release`: 14 `experiments`, 247 `tensor_vm`, 22 `tvmd`, 1 local CPU Compose,
  8 `tvmd_cli`, 1 `tensor_vm_explorer`, and doc-test targets passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: passed with 262 instrumented workspace tests and 97.29%
  workspace line coverage (11,559/11,881 lines).
- `git diff --check`
- Full Docker checker was not rerun because the standing gateway `/health` blocker remains unresolved.

Push evidence:
- Feature commit: `1d556efafd1443809406dcaa54bdc3aa63c68b9e`
  (`Move local producer to validator runtime`).
- Remote/branch: `origin/main`.
- Push result: `6e29e14..1d556ef  main -> main`; GitHub also printed the repository-moved notice:
  `git@github.com:distributedstatemachine/tensor_vm.git`.

### Iteration 13: Role-Owned Block Vote Finality

Feature capability:
Separate block payload append from finality by removing synthetic producer-owned finality votes from the
runtime path and adding validator role-owned block vote submission/gossip/evidence.

Checkpoint before edits:
- Canonical owner: `ChainEngine` owns `SubmitBlock` append and `SubmitBlockVote` vote/finality admission.
- Adapter callers: p2p/node runtime and role loops may submit block/vote commands and publish payloads;
  they must not mark finality directly.
- Old shortcut removed: local synthetic production must stop fabricating validator `BlockVote`s as part of
  block production. `finalize_local_cpu_block` may remain only as a test helper.
- Regression tests: block append remains unfinalized until enough explicit votes arrive; validator role
  submits a block vote for an unvoted valid block; network vote payloads finalize after quorum.
- Local synthetic disabled behavior: inbound block/vote ingest still works; no jobs, blocks, or votes are
  synthesized.
- Producer/non-producer behavior: producer capability only controls outbound block creation. Producers and
  non-producers both ingest blocks/votes; validators vote from role state.
- Structured evidence source: role runtime/status counters expose local validator block-vote submissions
  and network block-vote ingestion/application.
- Finality source: signed validator `BlockVote`s admitted by `SubmitBlockVote` and stake-weighted by
  `has_block_finality`, not block append or aggregate payload admission.
- Wire-size/codec boundary: existing bounded `NewBlockVotePayload`/`encode_block_vote_payload` codec is
  reused; this iteration adds evidence/tests rather than a new wire format.

Files/modules likely touched:
- `crates/tensor_vm/src/localnet.rs`
- `crates/tensor_vm/src/main.rs`
- `crates/tensor_vm/src/node.rs`
- `crates/tensor_vm/src/p2p.rs`
- `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
- `crates/tensor_vm/tests/local_cpu_compose.rs`
- `crates/tensor_vm/tests/tvmd_cli.rs`
- Readiness/status docs.

Parallel subagents launched before implementation:
- Readiness mapper completed and mapped canonical owner/finality/evidence requirements.
- Codebase explorer, test coverage explorer, and p2p/checker explorer completed and mapped the current
  producer-owned finality shortcut, coverage needs, and p2p/checker evidence updates.

Out of scope:
- Moving proposer block assembly fully out of the local synthetic producer.
- Public deployment evidence, CUDA, seven-day run, challenge openings, retargeting, and zero-receipt
  fallback.

Validation plan:
- Focused: `cargo fmt --check`, `cargo check -p tensor_vm --all-targets`,
  `cargo test -p tensor_vm --lib localnet::tests`, `cargo test -p tensor_vm --lib node::tests`,
  `cargo test -p tensor_vm --lib p2p::tests`, `cargo test -p tensor_vm --test tvmd_cli
  role_run_commands_serve_through_role_specific_surfaces`, and
  `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`.
- Broad before commit: `cargo test -p tensor_vm local_testnet --release`, `cargo test -p tensor_vm --tests`,
  `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`, `cargo tarpaulin
  --workspace --offline` if coverage changes are stable, and `git diff --check`.

Implementation summary:
- Local synthetic production now appends blocks without runtime-synthesized block votes; the old
  `finalize_local_cpu_block` shortcut is test-only.
- Validator role loops submit and gossip explicit block votes for locally valid unfinalized blocks, persist
  vote-only state changes, and expose `validator_block_votes_submitted`,
  `network_block_votes_ingested`, `network_block_votes_applied`, and p2p observed block-vote counters.
- Block-vote p2p payloads are covered by bounded codec tests, duplicate conflicting validator votes are
  rejected, and `TensorRowResponse` rejects oversized row lengths before allocation.
- Local Compose checker artifacts now require block-vote finality evidence, non-producer vote
  ingestion/application, and observed block-vote gossip.

Validation passed:
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Focused localnet/node/p2p library tests, the role-run CLI integration, and the local CPU Compose artifact
  test.
- `cargo test -p tensor_vm --tests`: 247 library tests, 22 `tvmd` binary tests, 1 local CPU Compose
  integration test, and 7 `tvmd_cli` integration tests.
- `cargo test -p tensor_vm local_testnet --release`: 5 release local-testnet library tests and the seed
  CLI integration test.
- `cargo test --workspace --release`: 14 `experiments`, 247 `tensor_vm`, 22 `tvmd`, 1 local CPU Compose,
  7 `tvmd_cli`, 1 `tensor_vm_explorer`, and doc-test targets passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: 262 instrumented workspace tests, 97.29% workspace line
  coverage (11,559/11,881 lines).
- `git diff --check`

Push evidence:
- Feature commit: `fb0feb02c3cebf6b9e4f0e00f7efb01fec275320`
  (`Add role-owned block vote finality`).
- Remote/branch: `origin/main`.
- Push result: `27d9bf8..fb0feb0  main -> main`; GitHub also printed the repository-moved notice:
  `git@github.com:distributedstatemachine/tensor_vm.git`.

### Iteration 12: Network-Visible Block Payload Admission

- Feature: replaced header-triggered deterministic replay with full `TensorBlock` payload gossip and
  strict chain admission through `SubmitBlock`.
- Follow-up on `origin/main`: `27d9bf8afb555d3c3c95ae2fd24524a62272fe6b` added block-vote payload
  plumbing, typed block admission outcomes, and removed remote-admission vote synthesis.
- Follow-up gap resolved by Iteration 13: local synthetic production no longer finalizes blocks by
  generating validator votes inside the producer path.
- Validation recorded: Gate 0, focused p2p/node/chain tests, workspace tests, Compose config, Tarpaulin,
  and `git diff --check`; full Docker checker remains blocked at gateway `/health`.

### Iteration 11: Canonical Useful-Verification Block Validity

- Feature: validator-owned useful-verification PoW over deterministic settled-receipt blockspace, replacing
  the prior settled-TensorWork proposer model.
- Main changes: canonical selected-receipt roots, `checks_root`, beacon, difficulty target, nonce, validator
  proposer checks, strict block-vote validation, selected-receipt inclusion tracking, and service-block/checker
  evidence for useful-PoW finality.
- Validation passed: formatting, `cargo check`, focused chain/storage/localnet/testnet/CLI/Compose gates,
  `cargo test -p tensor_vm local_testnet --release`, Compose config, Tarpaulin, and `git diff --check`.
- Full Docker gate: still blocked at gateway `/health`.
- Commits: `e6129d1915562a1e865579e347d8cfb85855089e`; `800b031edea9b0b268cfe1fb487c9628cb2c782c`.

### Iteration 10: Remote Validator Tensor Fetch

- Feature: validator role loops fetch missing receipt tensor artifacts from connected peers over libp2p
  request-response, verify tensors against requested commitment roots, insert/register tensors, and submit
  validator-owned attestations.
- Main changes: root-addressed tensor request/response messages, protocol-specific request-response
  dispatch, service-level tensor registration/fetch, validator role remote fetch counters, status/checker
  fields, and protocol count docs.
- Verifier: initial findings on malformed-payload loop abort and non-specific protocol dispatch were fixed;
  re-review reported no findings in scope.
- Validation passed: `cargo fmt --check`, `cargo check -p tensor_vm --all-targets`, focused p2p/node/role
  tests, CLI/Compose artifact tests, `cargo test -p tensor_vm local_testnet --release`,
  Compose config, `cargo tarpaulin --workspace --offline` with 254 tests and 98.73% workspace coverage,
  and `git diff --check`.
- Full Docker gate: still blocked at gateway `/health`.
- Commits: `2d6609e Add remote validator tensor fetch`; `1687f86 Record iteration 10 push evidence`.

### Iteration 9: Formalize MVP Core Soundness Boundary

- Feature: formal proof/audit docs for the MVP core and receipt-bound validator assignment enforcement in
  the shared chain engine.
- Main changes: assignment draw includes `receipt_id`; `SubmitAttestation` rejects unassigned validators;
  soundness findings/proof docs separate proved invariants from current consensus gaps.
- Validation passed: formatting, `cargo check`, scheduler/chain/role/CLI/Compose/local-testnet targeted
  tests, and `git diff --check`.
- Commits: `c42235c Add validator attestations and proof boundary`; `c916b19 Compile MVP core soundness
  findings`.

## Decision Log

- Keep the missing workflow document visible as a standing blocker; do not treat the readiness doc as a
  substitute.
- Preserve one shared chain engine. Deployment profiles can vary, but transition logic should not fork.
- Role-owned miner and validator work must mutate chain state through `ChainCommand` and publish through the
  shared p2p/event path.
- Do not require positive live Compose miner/validator-owned submissions yet while deterministic local replay
  can pre-apply jobs, receipts, attestations, or blocks before role loops observe unhandled work.
- Validator assignment is receipt-bound and enforced in the chain engine; persisting per-receipt assignment
  seed/provenance remains future work.
- For Iteration 11, replace active behavior directly with canonical names and fields. Do not add
  compatibility shims, legacy aliases, or parallel consensus modes.

## Validation Evidence

Resumed Iteration 13 checkpoint:
- Starting `HEAD`/`origin/main`: `27d9bf8afb555d3c3c95ae2fd24524a62272fe6b`.
- `git status --short`: untracked `docs/tensorvm/code_quality_deep_dive.md` was present before this
  iteration and was left untouched.
- First executable gate before exploration or edits:
  `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests and
  the seed CLI integration test.
- Subagents completed: readiness mapper, code-path explorer, test coverage explorer, and p2p/checker
  explorer.

Iteration 13 post-implementation validation passed:
- `cargo fmt --check --all`
- `cargo check -p tensor_vm --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tensor_vm --lib localnet::tests`: 9 tests passed.
- `cargo test -p tensor_vm --lib node::tests`: 17 tests passed.
- `cargo test -p tensor_vm --lib p2p::tests`: 28 tests passed.
- `cargo test -p tensor_vm --lib`: 247 tests passed.
- `cargo test -p tensor_vm --tests`: 247 library tests, 22 `tvmd` binary tests, 1 local CPU Compose
  integration test, and 7 `tvmd_cli` integration tests passed.
- `cargo test -p tensor_vm local_testnet --release`: 5 release local-testnet library tests and the seed
  CLI integration test passed.
- `cargo test --workspace --release`: 14 `experiments`, 247 `tensor_vm`, 22 `tvmd`, 1 local CPU Compose,
  7 `tvmd_cli`, 1 `tensor_vm_explorer`, and doc-test targets passed.
- `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces`
- `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape`
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: passed with 262 instrumented workspace tests and 97.29%
  workspace line coverage (11,559/11,881 lines).
- `git diff --check`
- Full Docker checker was not rerun because the standing gateway `/health` blocker remains unresolved:
  `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Feature commit after validation:
  `fb0feb02c3cebf6b9e4f0e00f7efb01fec275320` (`Add role-owned block vote finality`).
- Feature push result: `origin/main` accepted `27d9bf8..fb0feb0  main -> main`; remote printed a
  repository-moved notice pointing to `git@github.com:distributedstatemachine/tensor_vm.git`.

Resumed Iteration 12 checkpoint:
- `git status --short --branch`: `## main...origin/main` plus untracked `goal.md`.
- Starting `HEAD`/`origin/main`: `800b031edea9b0b268cfe1fb487c9628cb2c782c`.
- First executable gate before exploration or edits:
  `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests and
  the seed CLI integration test.
- Subagents completed: readiness mapper, code-path explorer, test coverage explorer, checker/docs explorer,
  and one read-only verifier.
- Verifier fixes applied: semantic invalid block payloads now count invalid instead of staying pending;
  remote block admission records modeled `BlockVote`s before finalization; dormant header replay mutation was
  removed; docs now keep the full Docker `/health` blocker visible.

Iteration 12 post-implementation validation passed:
- `cargo fmt --check`
- `cargo check -p tensor_vm --all-targets`
- `git diff --check`
- `cargo test -p tensor_vm --lib p2p::tests`
- `cargo test -p tensor_vm --lib node::tests`
- `cargo test -p tensor_vm --lib chain::tests`
- `cargo test -p tensor_vm --lib`
- `cargo test -p tensor_vm --tests`: 245 library tests, 21 `tvmd` binary tests, 1 local CPU Compose
  integration test, and 7 `tvmd_cli` integration tests passed.
- `cargo test -p tensor_vm local_testnet --release`: 5 release local-testnet library tests and the seed CLI
  integration test passed.
- `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`
- `cargo tarpaulin --workspace --offline`: passed with 260 instrumented workspace tests and 98.14%
  workspace line coverage (11,495/11,713 lines).
- `cargo fmt --check`, `cargo check`, and `git diff --check` were re-run after the verifier fixes.
- Full Docker checker was not rerun because the standing gateway `/health` blocker remains unresolved:
  `curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received`.
- Feature commit after validation:
  `f6f95074821a1ab5c0e320e0645c330ff88dde7d` (`Add network-visible block payload admission`).
- Validation evidence commit:
  `133fbcb6e1471261214d273415574cf9febef199` (`Record iteration 12 validation evidence`), confirmed on
  `origin/main`.

Previous Iteration 11 evidence:
- Feature commit: `e6129d1915562a1e865579e347d8cfb85855089e`.
- Evidence commit: `800b031edea9b0b268cfe1fb487c9628cb2c782c`, confirmed on `origin/main`.

Latest unresolved full-gate output:

```text
curl: (28) Operation timed out after 15002 milliseconds with 0 bytes received
local CPU testnet check failed: gateway route is not reachable: /health
```

Resumed Iteration 21 checkpoint:
- Starting branch state: `## main...origin/main`.
- First executable gate before this slice:
  `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests and
  the local-testnet service gateway integration test.
- Scope: implement a minimal MVP §20.7 block `checks_root` challenge path using the Merkle-openable
  selected receipt/check leaves added in Iteration 19. The slice should prove a mismatched check leaf,
  delay proposer rewards through the challenge window, void pending proposer rewards on proven block-check
  challenges, credit a challenger reward from the voided pending reward, throttle future proposer
  eligibility during a penalty window, and remove the affected receipt from reward settlement until
  reverified.
- Parallel explorers launched before implementation: readiness mapper, codebase explorer, and
  test-coverage explorer.
- Implemented locally so far:
  - `TensorBlock` carries a network-visible `proposer_reward` amount.
  - Rewarded block production and aggregate proposer rewards create `PendingProposerReward` records instead
    of immediately spendable proposer balances.
  - `ReleaseMaturedProposerRewards` moves unchallenged pending rewards into spendable balances only after
    the configured challenge window.
  - `BlockCheckChallenge` verifies challenger signatures, check-leaf Merkle openings, canonical
    recomputed `checks_root`, selected receipt/index pairing, duplicate prevention, and challenge-window
    expiry.
  - A successful block-check challenge voids the pending proposer reward for the challenged block height,
    pays the challenger from that pending amount, routes the remainder to treasury, quarantines the
    affected receipt, and sets a proposer throttle window.
  - This is not full verifier-transcript fraud-proof completion; check leaves still summarize canonical
    attestation check roots, and network/RPC challenge propagation is deferred.
- Validation run during implementation:
  - `cargo check -p tensor_vm --all-targets` passed.
  - `cargo test -p tensor_vm chain::tests::challenges -- --nocapture` passed.
  - `cargo test -p tensor_vm chain::tests::rewards -- --nocapture` passed after updating immediate-reward
    expectations to pending reward release.
  - `cargo test -p tensor_vm chain::tests::proposers -- --nocapture` passed.
  - `cargo test -p tensor_vm storage::chain_state -- --nocapture` passed.
  - `cargo test -p tensor_vm chain::tests::commands -- --nocapture` passed.
  - `cargo test -p tensor_vm chain::tests::blocks -- --nocapture` passed.
  - `cargo test -p tensor_vm --lib` passed with 320 tests.
  - `cargo fmt --check --all` passed after applying formatting.
  - `cargo test -p tensor_vm` passed with 320 library tests, 1 local CPU Compose integration test,
    8 `tvmd_cli` integration tests, 28 `tvmd_runtime` integration tests, and doc-test targets.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed after replacing high-arity helpers
    with typed context structs.
  - `cargo test -p tensor_vm local_testnet --release` passed with 5 release local-testnet library tests
    and the service gateway integration test.
  - `cargo test --workspace --release` passed with 14 `experiments`, 320 `tensor_vm`, 1 local CPU
    Compose integration test, 8 `tvmd_cli`, 28 `tvmd_runtime`, 3 `tensor_vm_explorer`, and doc-test
    targets.
  - `git diff --check` passed.
  - `cargo tarpaulin --workspace --offline` was attempted but blocked because this environment does not
    have the `cargo-tarpaulin` subcommand installed.

## Archive

- Iteration 1, `56da38a Extract reusable node runtime state`: extracted reusable node runtime state,
  pending payloads, event ingest, and counters.
- Iteration 2, `1b9a104 Move network payload application into node runtime`: moved decoded job, receipt,
  and attestation payload application into chain-centric helpers using `ChainCommand`.
- Iteration 3, `0b19f62 Extract reusable network event driver`: moved event ordering, invalid accounting,
  pending retry, and block-header dispatch into the reusable node runtime driver.
- Iteration 4, `8f24509 Bind role runtimes to chain identities`: role commands derive wallet addresses,
  check registration, persist identity status, and expose checker evidence.
- Iteration 5, `286ef9a Extract role runtime loop boundary`: added named role loop wrappers with RPC serving,
  network ingestion, local production, and status output.
- Iteration 6, `7262aaa Track miner work readiness in role loop`: miner role readiness counters detect
  assigned, unreceipted jobs; full Docker checker timed out at gateway health.
- Iteration 7, `ac7e6eb Submit miner receipts from role loop`: miner role executes assigned work, inserts
  tensors, submits receipts, publishes announcements, and exposes counters.
- Iteration 8: validator role submits assigned receipt attestations through the shared chain engine when
  local tensor artifacts are present; remote fetching was deferred to Iteration 10.
