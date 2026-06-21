# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 135 complete locally - network-visible external randomness beacon relay.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots with producer-selected receipts,
  side-branch fork storage, automatic unfinalized side-branch deep reorg, graph-backed synthetic jobs,
  and delayed challenge rewards are
  implemented locally. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner
  TensorWork activation now follows delayed miner receipt reward maturity instead of immediate settlement,
  and settled receipt rewards carry explicit awaiting-inclusion or claimable-height maturity state before claim.
  Reward maturity now makes state-rooted pending claims claimable, but spendable credit is owned by
  `ClaimReward` instead of automatic block-transition release.
  Newly emitted receipt-reward pending events now carry that maturity state directly instead of flattening
  awaiting-inclusion rewards into a synthetic claim height, and the internal receipt reward claim-height
  API now returns no height for awaiting-inclusion claims instead of a sentinel workaround.
  Block-check, invalid-output, and data-unavailability evidence now delays voided receipt claims to a
  state-rooted challenge hold height before they can be swept without credit.
  Selected-receipt block openings now expose typed block-check transcript commitments and
  submission-anchored retention deadlines. Redundancy-delayed receipts now have chain-owned state-rooted
  records when quorum-backed work cannot settle because distinct-operator agreement is missing or
  conflicting, and later pending receipt reward claims inherit those redundant reward holds. Redundant
  delay records persist both agreeing miner-address and agreeing operator counts. External randomness
  beacon records can now advance future receipt randomness through a rooted chain command and relay over
  the same bounded p2p/node ingest path used by local CPU role processes. `Fixed32`
  multiplication now rescales the signed raw product back to the lhs/output scale with round-half-to-even
  semantics in tensor, exact IR replay, and conformance vectors. Mixed-scale `Fixed32` `add`/`sub` now
  rescale the RHS to the lhs/output scale with the same half-even policy. `Fixed32` reciprocal division now
  returns to the lhs/output scale with the same half-even policy and rejects zero divisors. `Fixed32`
  matmul now accumulates signed raw products in fixed order and rescales once into the lhs/output scale.
  Packed int8 quantization now has a tensor-owned `TVQ8` payload API for bounded length calculation and
  shared encode/decode validation used by IR replay and conformance. External graph job payloads with
  missing graph bodies now stay pending through the shared node payload path, runtime ingest fetches
  missing graph bodies by request-response before retry, and miner/validator role loops fetch missing graph
  tensor artifacts, including `const_blob` tensors, before execution or attestation. Runtime block-payload
  import now tolerates producer/receiver mempool and finality-map timing drift while binding parent snapshots
  to stable chain anchors. Exact IR execution now exposes verified per-op trace openings, and libp2p can
  sample them by `trace_root` and op index.
- Current blockers:
  - `cargo tarpaulin --workspace --offline` is blocked because `cargo-tarpaulin` is not installed:
    `error: no such command: tarpaulin`.
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: preserve the local delayed multi-proposer and network-applied beacon proofs while
  broadening the same evidence into public/CUDA deployment runs, public drand/VRF verification, and full
  interactive transcript disputes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 21, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports `live_role_miner_receipts_submitted=402` | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Latest local CPU Docker proof reports `live_role_validator_block_proposer_operators=2`, `live_role_delayed_validator_block_proposer_operators=1`, `live_role_validator_useful_blocks_proposed=404`, `live_delayed_proposer_reward_claims=1`, competing proposers `validator-00 validator-01`, finalized passive-observer convergence at height 22, current-head useful competitor replacement, side-branch storage, and automatic unfinalized deep reorg | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, submission-anchored opening retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg, Docker proof | Remaining: full interactive transcript disputes |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface, packed int8 artifact APIs, role-owned graph execution, content-addressed `const_blob` replay/fetch, and p2p-sampled verified trace openings | Continue exact Tier-B verifier coverage, full interactive trace disputes, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records with miner/operator counts, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; local runtime ingests configured deterministic external beacon fixture; bounded p2p messages relay beacon records through node ingest; status/explorer/checker expose seed-domain, external beacon count/latest round, role applied counters, network-applied beacon counters, and block-hash-ban evidence | Add public drand verification, validator VRF construction, and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit receipt reward maturity state, inclusion-started receipt reward maturity, claim-owned spendability, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 135: Network-Visible External Randomness Beacon Relay

Feature capability: publish configured external randomness beacon records as bounded p2p payloads, decode
them through node message ingest, apply them only through `ChainCommand::SubmitExternalRandomnessBeacon`,
and persist the accepted chain state from network-originated events.
Readiness requirements covered: `upow.md` §10 randomness binding, §11 live role networking, and the
shortcut ban against local-only fixture mutation.
Canonical owner: `ChainCommand::SubmitExternalRandomnessBeacon` and chain validation own freshness,
recording, finalized-beacon advancement, and idempotent already-known handling.
Adapter callers: deterministic local runtime beacon tick, p2p wire codecs, node network ingest, runtime
chain persistence, and the chain-announcement publisher.
Old shortcut being removed: every role could apply the same configured fixture locally, but the local CPU
proof did not require any role to receive and apply that beacon from the network.
Regression tests that prove the shortcut is gone: `network_event_driver_applies_external_randomness_beacon_payloads`,
`external_randomness_beacon_payloads_roundtrip_and_reject_malformed_edges`,
`libp2p_mapping_separates_gossip_and_request_response`, runtime status tests, CLI status tests, and the
local CPU checker now requiring positive `role_network_external_randomness_beacons_applied`.
Behavior with local synthetic block production disabled: inbound beacon payloads apply through the node
event path without needing local job production or local block proposal.
Behavior for producer and non-producer roles: any role may observe/apply the beacon idempotently through
the chain command; producers republish current external beacon records with normal chain announcements
after peers are connected.
Structured evidence source: `role_network_external_randomness_beacons_ingested`,
`role_network_external_randomness_beacons_applied`, explorer randomness evidence, and checker output.
Finality source: unchanged signed validator block votes and block finality thresholds; the beacon only
feeds future receipt randomness anchors.
Wire-size and codec boundary: p2p tag 27 carries bounded source id, round, and payload bytes on the blocks
gossip topic with shared encode/decode helpers and mismatch/malformed rejection.
Out of scope: public drand signature verification, validator VRF construction, deployed commit-reveal
lifecycle, CUDA miner proof, and public 7-day evidence.

Validation evidence captured June 21, 2026:
- Focused checks passed: `cargo fmt --check --all`,
  `cargo test -p tensor_vm external_randomness_beacon_payloads_roundtrip_and_reject_malformed_edges -- --nocapture`,
  `cargo test -p tensor_vm network_event_driver_applies_external_randomness_beacon_payloads -- --nocapture`,
  `cargo test -p tensor_vm randomness_beacon -- --nocapture`,
  `cargo test -p tensor_vm p2p_messages_roundtrip -- --nocapture`,
  `cargo test -p tensor_vm libp2p_mapping_separates_gossip_and_request_response -- --nocapture`,
  `cargo test -p tensor_vm network_event_ingest_accumulates_runtime_counters -- --nocapture`,
  `cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces -- --nocapture`,
  `cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture`,
  and `sh -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh deploy/tensorvm/local-cpu/scripts/entrypoint.sh`.
- Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.
- Docker gate passed after rebuilding `tensorvm-local-cpu:latest`, `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up -d --wait`, and `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`. Key output: `live_role_network_external_randomness_beacons=23539`, `live_role_network_external_randomness_beacons_applied=23539`, `live_external_randomness_beacon_records=1`, `live_latest_external_randomness_beacon_round=1000`, `live_role_randomness_beacons_applied=15`, and `live_external_randomness_beacon_evidence=true`.
- Earlier Docker attempts failed with `no role applied external randomness beacon payloads from the network`; publishing current beacon records with normal chain announcements fixed the startup-gossip timing gap.
- Commit evidence: implementation committed as `e5010dd`.

### Iteration 134: Multi-Validator Proposer Competition Evidence

Feature capability: split synthetic job publication from validator block-proposer permission so multiple
validator role processes can propose useful UVPoW blocks from network-visible settled state. Result:
`validator-00` publishes jobs and proposes immediately, delayed `validator-01` proposes after
`TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS=20`, and other validators attest/vote only.
Validation evidence captured June 21, 2026: Gate 0, focused runtime/CLI/compose checks, Docker build/up/
check/down, and compose config passed. Docker output included `live_local_synthetic_job_producers=1`,
`live_role_validator_block_proposer_operators=2`, `live_role_delayed_validator_block_proposer_operators=1`,
`live_competing_validator_block_proposers=validator-00 validator-01`, and
`live_delayed_proposer_reward_claims=1`. `which tensorvm-verifier` and `which verifier` returned no
verifier binary on PATH. Commit/push evidence: implementation committed as `3655076` and pushed to
`origin/main`.

### Iteration 133: Claim-Owned Delayed Reward Release

Feature capability: keep matured rewards as state-rooted pending claims until the beneficiary submits
`ClaimReward`, removing block-transition auto-credit as the reward-release workaround.
Implementation result: `ClaimReward(beneficiary)` now sweeps matured non-void proposer, receipt, challenge,
and credit claims into `RewardState`, credits the account, and clears the reward balance in one command.
Block transition no longer auto-promotes matured non-void claims; it only prunes matured voided
proposer/challenge claims. Explicit low-level release commands remain for direct chain API/test coverage.
Validation evidence: Gate 0 first command passed; focused reward/command/transaction/settlement/challenge/
attestation/payload/runtime-role tests passed; `cargo fmt --check --all`, `git diff --check`,
`cargo test -p tensor_vm --quiet`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --release`, final Gate 0, compose config, Docker build, Docker `up --wait`,
checker, and Docker `down -v` passed. The live checker reported
`live_delayed_receipt_reward_claims=18`, `live_delayed_proposer_reward_claims=1`,
`live_delayed_challenge_reward_claims=1`, plus external beacon evidence. Coverage regeneration remained
blocked by missing `cargo-tarpaulin`.

### Iteration 132: Local External Beacon Runtime Wiring

Feature capability: local role runtimes ingest a configured deterministic drand-style beacon through
`ChainCommand::SubmitExternalRandomnessBeacon`, persist accepted state, and expose status/checker evidence.
Validation evidence: Gate 0 ultimately passed after an empty status value fix; focused runtime/status,
randomness, service-status, local CPU compose, shell syntax, and compose config checks passed; broad fmt,
diff, `cargo test -p tensor_vm --quiet`, clippy, workspace release, final Gate 0, and Docker
build/up/check/down passed with `live_external_randomness_beacon_records=1`,
`live_latest_external_randomness_beacon_round=1000`, and `live_role_randomness_beacons_applied=15`.
Coverage regeneration remained blocked by missing `cargo-tarpaulin`.

### Iteration 131: Reward Maturity Boundary Cleanup

Feature capability: make delayed rewards release at the protocol claim height during normal block
application instead of depending on a one-block/manual sweep workaround. Validation evidence: Gate 0,
focused proposer/receipt/fallback/audit reward tests, fmt, diff check, tensor_vm lib tests, clippy,
workspace release tests, and final Gate 0 passed; tarpaulin remained unavailable.

## Recent Iterations

### Iteration 130: Local Proof Status Drift Cleanup

Feature capability: align readiness, implementation, coverage, completion, workflow, and formal status docs
with the local CPU Docker proof for delayed proposer/challenge rewards and fallback while preserving public,
CUDA, drand/VRF, and full fraud-proof blockers.
Readiness requirements covered: `goal.md` stale-readiness update rule, local production evidence hygiene,
delayed reward/fallback evidence traceability, and no-overclaim status boundaries.
Files/modules likely touched: local readiness/status/coverage/completion docs, selected formal docs,
Codex workflow doc, doc guard tests, and this plan.
Parallel subagents run: readiness mapper, stale-doc explorer, and doc-test coverage explorer.
Tests/checkers/docs to add or update: doc guard tests rejecting stale `/health` blocker and obsolete
formal fallback/reward-finality wording.
Narrow validation commands: `cargo test -p tensor_vm deployment_docs --lib`.
Broad validation commands before commit: fmt, diff check, tensor_vm tests, clippy, workspace release tests,
final Gate 0, and tarpaulin attempt if status docs require coverage regeneration.
Expected observable evidence: stale Docker `/health` blocker text is gone from status docs; local Docker
proof and delayed reward evidence are named; public/CUDA/full-v0 blockers remain explicit.
Out of scope: public 7-day run, CUDA miner evidence, live drand/VRF, multi-validator Docker proposer
competition, and full interactive fraud-proof implementation.
Split trigger: formal docs require broad theorem rewrites rather than local-evidence status correction.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused doc guard passed: `cargo test -p tensor_vm deployment_docs --lib`.
- Stale-text scan found no old `/health` blocker or obsolete formal fallback/reward-state phrases in
  `docs/tensorvm` or `docs/formal`.
- Validation passed: `cargo fmt --check --all`, `git diff --check`,
  `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet`,
  `cargo test -p tensor_vm --quiet`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --release`, and final `cargo test -p tensor_vm local_testnet --release`.
- Coverage regeneration remains blocked because `cargo tarpaulin --workspace --offline` reports
  `error: no such command: tarpaulin`.

## Recent Iterations

### Iteration 129: Delayed Proposer And Challenge Rewards

Feature capability: replace proposer suppression/workarounds with protocol-level delayed proposer and
challenge rewards, producer parent-state block payloads, and local CPU Docker proof.
Readiness requirements covered: local production-ready acceptance gate, role-owned Docker runtime evidence,
and full local checker evidence for implemented miner/validator/proposer paths.
Files/modules touched: chain rewards/challenges/blocks/state, p2p block payload codecs, node payload
application, runtime proposer/miner roles, storage load recovery, local CPU checker, and this plan.
Parallel subagents run: read-only verifier agent reviewed the implementation before final validation.
Tests/checkers/docs added or updated: delayed proposer/challenge reward tests, block payload parent-snapshot
regression, runtime role tests, local CPU checker delayed reward counters, and this plan.
Narrow validation commands: focused block-payload regression, local CPU compose test, and Docker checker.
Broad validation commands before commit: fmt, diff check, tensor_vm tests, clippy, workspace release tests,
tarpaulin attempt, final Gate 0, and Docker cleanup.
Expected observable evidence: the Docker gate reports future-maturity proposer and challenge reward claims
while all operators converge on a finalized live target head.
Out of scope: public 7-day run, CUDA miner packaging, and broad protocol refactors.
Split trigger: public/CUDA proof or full interactive transcript disputes.

Validation evidence:
- First Gate 0: `cargo test -p tensor_vm local_testnet --release` passed before edits on June 21, 2026.
- Focused checks passed: `cargo test -p tensor_vm --test local_cpu_compose --quiet` and
  `cargo test -p tensor_vm block_payload_application_uses_producer_parent_snapshot_for_divergent_mempool --quiet`.
- Docker proof passed after a clean image build/up: `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
  reported `live_pending_proposer_rewards=19`, `live_delayed_proposer_reward_claims=1`,
  `live_pending_challenge_rewards=1`, `live_delayed_challenge_reward_claims=1`,
  `all_operator_common_head_height=47`, and `all_operator_target_head_convergence=true`.
- Broad checks passed: `cargo fmt --check --all`, `git diff --check`, `cargo test -p tensor_vm --quiet`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --release`.
- Coverage regeneration remains blocked because `cargo tarpaulin --workspace --offline` reports
  `error: no such command: tarpaulin`.
- Final Gate 0 passed: `cargo test -p tensor_vm local_testnet --release`.
- Commit/push evidence: implementation committed as `c8b4cf1` and pushed to `origin/main`.

## Recent Iterations

- Iteration 128 made awaiting-inclusion receipt rewards genuinely heightless in commits `9aa0841` and
  `8110a0f`.
- Iteration 127 added the Codex local-chain workflow doc in commit `94d4180`.
- Iteration 126 made `ReceiptRewardPending` events carry explicit maturity state in commits `2c5cb68` and
  `c6613cb`.
- Iteration 125 exposed explicit pending reward maturity views.
- Iteration 124 made collusion-risk study evidence operator-aware in commit `bdac46b`.
- Iteration 123 made redundant settlement quorum operator-distinct in commit `1c86e13`.
- Iteration 122 delayed voided receipt rewards through challenge holds in commit `bde7e51`.
- Iterations 120-121: trace openings and p2p trace-opening sampling landed in `b3fe556` and `f631084`.
- Iterations 116-119: packed int8 artifacts, external graph artifact fetch, and explicit receipt reward
  maturity landed in prior pushed commits.
- Iterations 110-115: fixed-point rescale semantics, packed int8 payloads, and delayed reward cleanup
  landed in pushed commits including `ce665a5`, `4de9463`, `506b020`, `4fceaeb`, and `1c65b80`.

Earlier detailed iterations are summarized in the archive to keep this plan compact.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest local validation is Iteration 129 on June 21, 2026:

```text
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
cargo fmt --check --all
git diff --check
cargo test -p tensor_vm --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --release
```

Current coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

## Archive

- Iterations 73-103: validator-audit calibration/appeal, diagnostic block-check challenges, fallback
  timeout, receipt-bound randomness, fork choice/side-branch storage, invalid-output slashing, role-owned
  graph production, typed block-check openings, retention deadlines, and inclusion-started reward delay
  landed across the archived commits listed in earlier plan revisions.
- Iterations 59-64: exact `clamp`, field `div`, split/einsum, registry/conformance guard, and graph
  verifier coverage landed across commits including `85a2956`, `d659e14`, and `b6e0887`.
