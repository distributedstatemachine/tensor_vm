# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 153 in progress - production validator reveal proofs.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  chain-owned pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots, side-branch fork storage,
  automatic unfinalized side-branch deep reorg, graph-backed synthetic jobs, and claim-owned reward
  spendability are implemented locally. Receipt reward maturity is explicit for awaiting-inclusion,
  claimable-height, and validator-VRF-reveal-held states; later challenge, audit, or redundant-settlement
  holds extend the same rooted pending claims. Miner TensorWork activation follows delayed miner receipt
  reward maturity. Selected `LinearTrainingStep` receipt inclusion applies the model-state transition in
  the deterministic child-state transition. Public evidence bundles require typed signed
  `randomness-beacon` supporting records. Chain, bounded p2p/node ingest, and role runtime paths verify
  `pedersen-bls-unchained` drand and public default-chain `pedersen-bls-chained` drand evidence through
  typed proof metadata. Public drand mode now polls continuously, applies only verified newer rounds,
  skips stale finalized rounds, backs off after failures, computes endpoint-observed expected-latest-round
  and chain-epoch evidence, and rejects locally fetched public rounds outside the configured lag. Keyed
  validators now register reveal public keys and must provide chain-verified bounded Ed25519 proof bytes
  over the committed receipt seed before validator receipt rewards are released. Consensus-level public
  drand epoch mapping, deployed validator reveal key lifecycle/full VRF construction, and deployed
  lifecycle evidence remain open.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: continue deployed validator reveal key lifecycle/full VRF construction, consensus-level
  public drand epoch mapping, public/CUDA deployment runs, and full interactive transcript disputes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Local CPU Docker proof covers proposer cadence, delayed proposer reward evidence, side-branch storage, and passive convergence | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, block-check challenges, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, competitor policy, side-branch storage, deep reorg, Docker proof | Remaining: full interactive transcript disputes |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph receipts, exact replay for current core and broad Tier-B surface, packed int8 APIs, role-owned graph execution, `const_blob`, p2p trace openings | Continue exact Tier-B verifier coverage, full interactive trace disputes, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors bind finalized beacon randomness and validation seed commitments. Local runtimes ingest deterministic fixtures, configured verified drand, and public default-chain chained drand through verified chain commands. P2p/node paths relay bounded fixture, verified drand, public chained drand, and validator reveal payloads. Public drand polling exposes attempts/successes/stale/failure/backoff plus expected latest round, fetched lag, max lag, rounds per chain epoch, chain epoch, and freshness, and stale-by-policy local public rounds are skipped before chain mutation. Keyed validator reveals register public keys and require bounded Ed25519 proof bytes before reward release. Status/explorer/checker expose seed-domain, external beacon count/latest round, validator reveal count, production-vs-legacy reveal counts, role counters, network-applied beacon/reveal counters, and block-hash-ban evidence | Add consensus-level public drand epoch mapping, deployed validator reveal key lifecycle/full VRF construction, and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit reward maturity, VRF reveal holds, claim-owned spendability, delayed miner TensorWork activation, late invalid-output voiding/slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 153: Production Validator Reveal Proofs

Feature capability: validator receipt rewards no longer rely on the old address-hash reveal helper for
keyed validators. Validators can register a reveal public key; reveal records carry bounded Ed25519 proof
bytes over the committed receipt seed input; chain validation verifies those bytes against the registered
key, roots/stores/relays the proof material, and only then unlocks the matching validator receipt reward.
Status and explorer evidence split production keyed reveals from legacy local fallback reveals.

Readiness requirements covered: `upow.md` §10 validator reveal randomness, reward-release reveal holds,
and the goal shortcut ban against working around randomness by accepting unverifiable local helper output.

Files/modules touched: chain validation/state/roots/storage/commands, p2p wire and node payload
application, validator role runtime secret plumbing, status/explorer evidence, focused chain/runtime/RPC
tests, coverage/readiness docs, and this execution plan.

Parallel subagents run: readiness mapper, code-path explorer, and test coverage explorer; parent owned
the final code/docs integration.

Expected observable evidence: keyed validators reject legacy helper reveals, accept Ed25519 proof reveals,
unlock validator receipt rewards only after accepted proof reveal, bounded p2p payloads reject oversized
proof bytes, role runtime submits a registered-key proof reveal from its wallet secret, and status/explorer
show registered-key plus production/legacy reveal counts.

Canonical owner: chain validation/state/rooting own registered keys, reveal proof verification,
reward-release gating, and persisted evidence. Runtime and node adapters only derive, relay, queue, or
display bounded payloads.

Out of scope: SR25519 or deployed validator VRF key management, consensus-level public drand epoch mapping,
public endpoint quorum/failover, 7-day public deployment evidence, CUDA evidence, and full deployed
commit-reveal lifecycle.

First executable gate this iteration: `cargo test -p tensor_vm local_testnet --release` passed on
June 22, 2026 before code edits.

Narrow validation commands: `cargo test -p tensor_vm validator_vrf --lib -- --nocapture`,
`cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture`,
`cargo test -p tensor_vm app::status::tests::service_status_exports_randomness_binding_evidence --lib -- --nocapture`,
`cargo test -p tensor_vm rpc::tests::routes::explorer_overview_exports_validator_audit_economic_calibration --lib -- --nocapture`,
and `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture`.

Broad validation commands before commit: `cargo fmt --check --all`, `git diff --check`, and
`cargo test -p tensor_vm local_testnet --release`.

## Recent Iterations

### Iteration 152: Public Drand Freshness And Chain-Epoch Mapping Evidence

Commit: `19cb3ba`.

Feature capability: public drand polling now treats endpoint freshness as part of local runtime evidence
instead of accepting any newer verified public round. The fetch path records the drand `/info` genesis
time and period, computes the expected latest round from the local observed time, calculates fetched-round
lag, derives drand rounds per chain epoch from chain params, records the current chain epoch, and skips
locally fetched public rounds whose lag exceeds `TENSORVM_RANDOMNESS_BEACON_DRAND_MAX_ROUND_LAG`.

Readiness requirements covered: `upow.md` §10 external beacon freshness, `mvp_spec.md` randomness evidence,
the goal shortcut ban against working around randomness, and operator-visible evidence that stale public
rounds are delayed at the runtime ingress boundary instead of admitted into chain state.

Files/modules touched: `crates/tensor_vm/src/app/randomness_beacon.rs`,
`crates/tensor_vm/src/node/runtime_state.rs`, `crates/tensor_vm/src/app/runtime_status_snapshot.rs`,
`crates/tensor_vm/src/app/runtime_status.rs`, `crates/tensor_vm/src/app/status.rs`,
`crates/tensor_vm/tests/tvmd_runtime/runtime_persistence.rs`, coverage/readiness docs, and this execution
plan.

Parallel subagents run: readiness mapper, code-path explorer, and test coverage explorer; parent owned
the final code/docs integration.

Expected observable evidence: public drand status includes expected latest round, fetched lag, max lag,
rounds per chain epoch, chain epoch, and freshness. A scripted valid-but-stale public chained round newer
than finalized state increments observed/skipped/stale counters, leaves finalized randomness unchanged,
and reports `fresh=false`.

Canonical owner: chain validation still owns deterministic signature verification, randomness derivation,
typed proof metadata, monotonic accepted beacon records, and receipt/block seed use. Runtime owns
wall-clock freshness observation for locally fetched public endpoints until a consensus genesis-time
round-to-epoch parameter is introduced.

Out of scope: consensus-level drand epoch mapping, public endpoint quorum/failover, deployed validator
reveal key lifecycle/full VRF construction, 7-day public deployment evidence, CUDA evidence, and full
deployed commit-reveal lifecycle.

Narrow validation commands: `cargo test -p tensor_vm public_drand --lib -- --nocapture`,
`cargo test -p tensor_vm --test tvmd_runtime public_drand -- --nocapture`,
`cargo test -p tensor_vm node::runtime_state::tests::runtime_state_tracks_loop_counters --lib -- --nocapture`,
and `cargo test -p tensor_vm app::status::tests::service_status_forwards_role_randomness_beacon_evidence --lib -- --nocapture`.

Broad validation commands before commit: `cargo fmt --check --all`, `git diff --check`, and
`cargo test -p tensor_vm local_testnet --release`.

### Iteration 151: Continuous Public Drand Polling And Backoff

Commit: `139cfe2`.

`TENSORVM_RANDOMNESS_BEACON_MODE=public_drand` became runtime polling instead of a startup shortcut. The
role loop fetches the latest public default-chain chained drand beacon on cooldown, verifies and applies
only strictly newer rounds through `SubmitVerifiedChainedDrandBeacon`, records stale finalized rounds
without mutating finalized randomness, and backs off after fetch/verification failures. Validation passed:
first-command Gate 0 `cargo test -p tensor_vm local_testnet --release`, public drand focused tests,
runtime persistence tests, runtime-state/status focused tests, `cargo fmt --check --all`,
`git diff --check`, and final Gate 0.

### Iteration 150: Public Default-Chain Drand Fetch

Commit: `9d87df0`.

Role runtimes can use `TENSORVM_RANDOMNESS_BEACON_MODE=public_drand` to fetch the latest public drand
default-chain beacon from the v2 HTTP API, validate `pedersen-bls-chained` proof bytes with
`previous_signature`, derive randomness from the verified signature, persist typed
`DrandPedersenBlsChainedV1` metadata, and publish/ingest bounded chained drand p2p payloads. Validation
passed: first-command Gate 0, verified chained drand tests, public drand tests, network ingest tests,
runtime persistence tests, `cargo fmt --check --all`, `git diff --check`, and final Gate 0.

### Iteration 149: Configured Verified Drand Runtime Mode

Configured role runtimes can apply `pedersen-bls-unchained` drand evidence through
`ChainCommand::SubmitVerifiedDrandBeacon` and publish bounded `NewVerifiedDrandBeaconPayload` messages,
removing the runtime-facing local fixture shortcut for production-style drand evidence.

### Iteration 148: Verified Drand Network Admission

Bounded p2p/node runtime paths admit verified drand evidence through the verified chain command, so
network-originated drand no longer uses caller-supplied randomness plus proof hashes.

### Iteration 145: Chain-Owned Delayed Rewards And Restart Convergence Cleanup

Delayed rewards are enforced by canonical chain state. Receipt rewards wait for block inclusion to start
the maturity clock, validator receipt rewards wait for `SubmitValidatorVrfReveal`, proposer rewards
materialize only after finality, and finalized side branches promote through the same block-vote path.

### Iteration 144: Typed Public Randomness Evidence Records

Public evidence bundles gained a typed signed `randomness-beacon` supporting-record class, giving later
production drand/VRF verification a stable public evidence surface.

## Decision Log

- Gate 0 remains `cargo test -p tensor_vm local_testnet --release` and must be the first executable command
  on every resume before edits.
- Chain validation is the canonical owner for accepted randomness proof verification, typed proof metadata,
  state-rooted records, finalized beacon advancement, and seed derivation.
- Runtime may observe wall-clock public endpoint freshness only for locally fetched public drand. Network
  ingested public drand remains deterministic signature plus monotonicity until consensus carries a
  chain-genesis-time round mapping.
- Reward delays, reveal holds, and spendability are chain-owned pending-claim state. Checkers and runtime
  surfaces only observe these states.
- Bounded p2p/node payloads remain the only network wire surface for randomness and reveal records.
- Public 7-day evidence, CUDA evidence, deployed validator reveal key lifecycle/full VRF construction, and
  full interactive transcript disputes remain deployment or future-feature gates, not local-completion
  claims.

## Validation Evidence

- June 22, 2026 Iteration 153 first executable command passed before code edits:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 153 focused validation passed on June 22, 2026:
  `cargo fmt --check --all`;
  `cargo test -p tensor_vm validator_vrf --lib -- --nocapture`;
  `cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture`;
  `cargo test -p tensor_vm app::status::tests::service_status_exports_randomness_binding_evidence --lib -- --nocapture`;
  `cargo test -p tensor_vm rpc::tests::routes::explorer_overview_exports_validator_audit_economic_calibration --lib -- --nocapture`;
  `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture`;
  `cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract -- --nocapture`;
  and `git diff --check`.
- Iteration 153 final release gate passed on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.
- June 22, 2026 Iteration 152 first executable command passed:
  `cargo test -p tensor_vm local_testnet --release`.
- Iteration 152 focused validation passed on June 22, 2026:
  `cargo fmt --check --all`;
  `cargo test -p tensor_vm node::runtime_state::tests::runtime_state_tracks_loop_counters --lib -- --nocapture`;
  `cargo test -p tensor_vm app::status::tests::service_status_forwards_role_randomness_beacon_evidence --lib -- --nocapture`;
  `cargo test -p tensor_vm public_drand --lib -- --nocapture`;
  `cargo test -p tensor_vm --test tvmd_runtime public_drand -- --nocapture`;
  and `git diff --check`.
- Iteration 152 final release gate passed on June 22, 2026:
  `cargo test -p tensor_vm local_testnet --release`.

## Archive

- Iterations 143 and earlier established finality-delayed proposer rewards, finalized side-branch
  convergence, durable restart-rehydrated tensor artifacts, deployment preflight/evidence surfaces, rolling
  restart evidence, richer IR/Tier-B execution, delayed reward maturity, claim-owned spendability, audit and
  challenge reward holds, exact trace openings, and related local CPU Docker proof evidence.
- Keep detailed historical command transcripts in git history rather than this active plan.
