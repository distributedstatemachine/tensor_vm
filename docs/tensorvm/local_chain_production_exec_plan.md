# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 145 complete - chain-owned delayed reward and restart convergence cleanup.
- Current status: delayed proposer, receipt, challenge, validator-audit, and credit rewards are
  state-rooted pending claims. Validator-owned proposal, block votes, audit-report gossip, observed
  malformed block-check challenge handling, parent-state snapshots with producer-selected receipts,
  side-branch fork storage, automatic unfinalized side-branch deep reorg, graph-backed synthetic jobs,
  and canonical delayed challenge rewards are
  implemented locally. Pre-inclusion fraud/redundancy reward holds now remain explicit
  awaiting-inclusion delayed pending receipt rewards until a canonical block includes the receipt, and
  voided reveal-held claims can be pruned after the rooted hold height without waiting for a validator
  reveal. Miner and validator role helpers can execute and attest `GraphExecution` jobs from
  registered graph bodies, local tensor artifacts, and content-addressed `const_blob` tensors. Miner
  TensorWork activation now follows delayed miner receipt reward maturity instead of immediate settlement,
  and settled receipt rewards carry explicit awaiting-inclusion or claimable-height maturity state before claim.
  Validator receipt rewards included before their validator reveal exists now carry an explicit
  `AwaitingValidatorVrfReveal` maturity state in chain state, reward roots, storage, service status, and
  explorer JSON until `SubmitValidatorVrfReveal` converts the same pending claim back to its original
  claimable height.
  Later challenge, audit, or redundant-settlement reward delays now extend that same reveal-held
  maturity height instead of converting it to a plain claimable reward.
  Selected `LinearTrainingStep` receipt inclusion now applies the model-state transition inside the
  deterministic block child-state transition, including deterministic registration of the selected
  job's missing model state before the first transition. Proposer parent-state preparation no longer
  advances linear models as a pre-block side effect. Status and explorer summaries expose
  `model_step_total`, and the local CPU checker verifies a rooted model transition separately from
  live linear receipt block evidence.
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
  the same bounded p2p/node ingest path used by local CPU role processes. Validator VRF reveal records are
  now chain-verified, state-rooted, p2p-relayed, retried when received before receipt anchors, and required
  before positive validator receipt reward credit can release.
  Public evidence bundles now require a typed signed `randomness-beacon` supporting-record summary and
  artifact locator, and record-file derivation validates exact `randomness_beacon_record=` lines before
  hashing. This creates a public evidence surface for drand/validator-VRF records without claiming live
  production drand BLS verification yet. `Fixed32`
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
- Next action: continue public/CUDA deployment runs, production drand/VRF verification, and full
  interactive transcript disputes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 21, 2026 for Iteration 140; post-edit rerun also passed | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports `live_role_miner_receipts_submitted=402` | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Latest local CPU Docker proof reports delayed proposer rewards, all-operator finalized-head convergence, current-head useful competitor replacement, side-branch storage, automatic unfinalized deep reorg, and five-validator chain-visible proposer cadence state | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, submission-anchored opening retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg, Docker proof | Remaining: full interactive transcript disputes |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface, packed int8 artifact APIs, role-owned graph execution, content-addressed `const_blob` replay/fetch, and p2p-sampled verified trace openings | Continue exact Tier-B verifier coverage, full interactive trace disputes, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records with miner/operator counts, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; local runtime ingests configured deterministic external beacon fixture; bounded p2p messages relay beacon and validator reveal records through node ingest; status/explorer/checker expose seed-domain, external beacon count/latest round, validator reveal count, role applied counters, network-applied beacon/reveal counters, and block-hash-ban evidence | Add public drand verification, production validator VRF signatures, and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit receipt reward maturity state, explicit pre-inclusion delayed receipt maturity, inclusion-started receipt reward maturity, explicit `AwaitingValidatorVrfReveal` validator receipt maturity, claim-owned spendability, validator receipt reward release gated by accepted reveal records, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 145: Chain-Owned Delayed Rewards And Restart Convergence Cleanup

Feature capability: delayed rewards are enforced by canonical chain state instead of checker/runtime
workarounds. Receipt rewards stay pending until block inclusion starts the maturity clock, validator
receipt rewards stay reveal-held until `SubmitValidatorVrfReveal`, proposer rewards materialize only after
the proposed block reaches finality, and finalized side branches promote through the same block-vote path
so restarted/passive operators converge without synthetic reward or fork compensation.
Readiness requirements covered: `upow.md` §11 finality-separated block admission, `upow.md` §12 reward
escrow/challenge-window economics, `mvp_spec.md` §§20.3/20.4/25.5 delayed spendability, local CPU restart
continuity, and the shortcut ban against checker-only reward/fork readiness evidence.
Files/modules touched: `crates/tensor_vm/src/chain/blocks.rs`,
`crates/tensor_vm/src/chain/validation.rs`, `crates/tensor_vm/src/chain/challenges.rs`,
`crates/tensor_vm/src/chain/receipts.rs`, reward/finality/audit/block tests, node/runtime restart status
surfaces, local CPU checker scripts, local CPU Compose spec tests, and readiness docs.
Parallel subagents run: reward/finality mapper, runtime/restart counter explorer, and test-coverage
explorer from the prior delayed-reward/restart pass; parent owned the final integration edits.
Parallelizable implementation workstreams: read-only mapping and focused validation ran in parallel; parent
owned consensus, checker, and docs edits because reward roots, finality, and restart gates must remain
one coherent transition.
Tests/checkers/docs added or updated: proposer reward finality-delay coverage, receipt reward
inclusion-started maturity coverage, validator VRF reveal-held reward coverage, finalized useful side-branch
promotion coverage, audit appeal reward-delay assertions, restart plateau checker shape, rolling restart
script-shape coverage, and readiness/local CPU docs.
Narrow validation commands: `cargo test -p tensor_vm chain::tests::blocks --lib -- --nocapture`,
`cargo test -p tensor_vm chain::tests::attestations --lib -- --nocapture`,
`cargo test -p tensor_vm chain::tests::rewards --lib -- --nocapture`, and
`cargo test -p tensor_vm chain::tests::settlement --lib -- --nocapture`.
Broad validation commands before commit: `cargo test -p tensor_vm local_testnet --release`,
`cargo test -p tensor_vm --test local_cpu_compose -- --nocapture`, `cargo fmt --check --all`, and
`git diff --check`.
Expected observable evidence: producing or admitting a block does not create a spendable proposer reward
until signed votes finalize that block; included receipt rewards derive maturity from inclusion height;
validator rewards remain pending with `AwaitingValidatorVrfReveal` until a verified reveal is accepted;
finality votes for a stored useful side branch can promote it and preserve vote/finality/reward state; and
restart checks preserve finalized common head/state while reporting plateau versus fresh advancement.
Out of scope: public/CUDA deployment evidence, production drand/BLS verification, production validator VRF
key management, and full interactive trace disputes.
Split trigger: if the delayed reward root changes had required a storage schema migration or public
evidence format change, land chain reward timing first and defer adapters; no split was required.
Canonical owner: chain block application, block-vote finality, pending reward ledgers, and node-store
restart recovery. Checkers only observe structured state.
Adapter callers: validator proposer ticks, p2p block/vote ingest, restart and rolling-restart checkers,
status/explorer pending-claim views, and local CPU Compose evidence tests.
Old shortcut removed: adapters no longer need to compensate for proposer rewards appearing before finality,
receipt rewards becoming claimable before block inclusion/reveal, or restart gates requiring fresh block
advancement when all operators already preserve and reconverge on a finalized common head.
Regression test that proves the shortcut is gone: block/reward tests assert no proposer pending claim before
finality and materialization after finality, receipt rewards preserve inclusion/reveal holds, audit appeal
reversal does not bypass VRF reveal, and finalized side-branch votes promote through chain state.
Behavior with local synthetic block production disabled: reward maturity and side-branch finality are
chain-state rules driven by submitted blocks/votes/reveals; they do not depend on local synthetic job
production.
Behavior for producer and non-producer roles: producers and passive observers ingest the same block,
vote, and reveal payloads and derive the same pending reward/finality state.
Structured evidence source: finalized block set, block votes, pending proposer/receipt/challenge reward
ledgers, reward roots, node-store block-log roots, role runtime status, and local CPU checker output.
Finality source: unchanged signed validator block-vote threshold; no runtime vote synthesis.
Wire-size and codec boundary: no new p2p payloads or unbounded wire reads; existing block, vote, challenge,
and reveal codecs carry the evidence.

Latest validation evidence: `cargo test -p tensor_vm chain::tests::blocks --lib -- --nocapture`,
`cargo test -p tensor_vm chain::tests::attestations --lib -- --nocapture`,
`cargo test -p tensor_vm chain::tests::rewards --lib -- --nocapture`, and
`cargo test -p tensor_vm chain::tests::settlement --lib -- --nocapture` passed on June 22, 2026 after the
final reward/finality assertions. `cargo fmt --check --all`, `git diff --check`, and
`cargo test -p tensor_vm local_testnet --release` also passed on June 22, 2026. After aligning runtime
tests with finality-delayed reward materialization, `cargo test -p tensor_vm --test tvmd_runtime -- --nocapture`
and `cargo test -p tensor_vm --test local_cpu_compose -- --nocapture` also passed on June 22, 2026.
Docker full local CPU and rolling-restart evidence remains the previously recorded local proof unless
rerun before this commit.

### Iteration 144: Typed Public Randomness Evidence Records

Feature capability: public evidence bundles must bind randomness-beacon/drand/VRF evidence as a typed
supporting-record class instead of relying on generic network-runtime observations. The record kind covers
public beacon source, round, randomness, proof root, verification scheme, observed block, and accepted/rejected
status so later production drand/VRF verification has a stable signed evidence surface.
Readiness requirements covered: `upow.md` §10 beacon/VRF randomness evidence, `mvp_spec.md` public evidence
manifest completeness, AC13 independently checkable supporting artifacts, and the shortcut ban against
hardcoded checker booleans as readiness evidence.
Files/modules likely touched: `crates/tensor_vm/src/testnet/public_evidence_crypto.rs`,
`crates/tensor_vm/src/testnet/public_evidence_bundle.rs`,
`crates/tensor_vm/src/testnet/public_evidence_manifest.rs`,
`crates/tensor_vm/src/cli/public_evidence_record_commands.rs`,
`crates/tensor_vm/src/cli/record_supporting_evidence.rs`, public evidence fixtures/tests, checked example
manifests, coverage/status docs, and this execution plan.
Parallel subagents to run: readiness/evidence mapper, public-evidence code-path explorer, and test-coverage
explorer.
Parallelizable implementation workstreams: read-only mapping runs in parallel; parent owns all edits because
the record enum, manifest parser, CLI args, bundle completeness report, and fixtures must remain in lockstep.
Tests/checkers/docs to add or update: record-kind parser coverage, raw supporting-line validation/rejection,
summary/artifact CLI output for the new kind, bundle completeness requiring the randomness artifact, public
evidence manifest fixtures/examples, and coverage/readiness documentation.
Narrow validation commands: `cargo test -p tensor_vm public_evidence_record --lib -- --nocapture`,
`cargo test -p tensor_vm public_evidence --test tvmd_cli -- --nocapture`, and focused testnet evidence bundle
tests if touched.
Broad validation commands before commit: `cargo test -p tensor_vm local_testnet --release`,
`cargo fmt --check --all`, `git diff --check`, and the public evidence CLI integration tests.
Expected observable evidence: `tvmd public evidence record ... --kind randomness-beacon` can generate signed
summary and artifact lines; `record summary-file` accepts typed randomness-beacon raw records and rejects
malformed/tampered lines; full public evidence reports require exactly one public randomness-beacon artifact
whose root/count/signature match the manifest signer.
Out of scope: live drand HTTP fetching, BLS signature verification, production validator VRF key management,
CUDA evidence, 7-day public deployment, and interactive transcript disputes.
Split trigger: if adding the evidence kind requires changing chain state or p2p beacon payloads, land the
public evidence surface first and keep chain-admission verification for a separate randomness iteration.
Canonical owner: public evidence bundle evaluation, manifest parsing, typed record-kind crypto messages, and
CLI supporting-record derivation.
Adapter callers: `tvmd public evidence record ...`, checked evidence manifests, deployment runbook evidence
generation, and public evidence validators.
Old shortcut being removed: public randomness evidence can no longer be represented only as untyped
network-runtime observations or a documentation claim; it must have its own signed summary and artifact.
Regression test that proves the shortcut is gone: full-spec public evidence fixture/report tests fail without
the randomness-beacon artifact and CLI record-file tests reject malformed randomness-beacon lines.
Behavior with local synthetic block production disabled: record generation/validation works from captured
public evidence files and does not require local fixture beacon ticks.
Behavior for producer and non-producer roles: producers and observers can contribute the same signed
randomness-beacon records; the public bundle validates the aggregated artifact independently of role.
Structured evidence source: signed `randomness_beacon_record=` lines, aggregated record root, manifest
summary signature, and public artifact locator signature.
Finality source: observed block/finality fields inside the typed record plus the existing signed finality
history artifact; this feature does not change chain finality.
Wire-size and codec boundary: no p2p payload change; public evidence record files remain line-oriented,
bounded by existing file/line parsing and exact comma-field validation.

Latest validation evidence: Gate 0 first executable command for this resume,
`cargo test -p tensor_vm local_testnet --release`, passed on June 22, 2026. Focused validation passed:
`cargo test -p tensor_vm public_evidence_record --lib -- --nocapture`,
`cargo test -p tensor_vm public_testnet_evidence_manifest --lib -- --nocapture`,
`cargo test -p tensor_vm public_testnet_evidence_bundle --lib -- --nocapture`,
`cargo test -p tensor_vm --test tvmd_cli public_evidence -- --nocapture`,
`cargo test -p tensor_vm public_evidence --lib -- --nocapture`,
`cargo test -p tensor_vm public_deployment --lib -- --nocapture`, and
`cargo fmt --check --all`.

### Iteration 143: Finality-Delayed Proposer Reward Accrual and Finalized Branch Convergence

Feature capability: useful/fallback block proposer rewards are recorded only after the block reaches
chain finality, while spendability remains claim-owned and delayed through the existing maturity window.
Finalized useful side branches must also converge through the same chain-owned block/vote path, so passive
operators advance past the seeded height without checker-side reward or fork workarounds.
Readiness requirements covered: `upow.md` reward-finality delay, `mvp_spec.md` pending reward maturity,
local CPU all-operator convergence, and the shortcut ban against papering over fork drift in Docker
checkers.
Files/modules likely touched: `crates/tensor_vm/src/chain/blocks.rs`,
`crates/tensor_vm/src/chain/validation.rs`, reward/fork-choice tests, local CPU checker evidence, and
this execution plan.
Parallel subagents run: readiness mapper, restart/runtime counter path explorer, and test-coverage
explorer.
Parallelizable implementation workstreams: read-only mapping in parallel; parent owns consensus/reward
edits because reward roots, finality, and fork admission must stay one chain-owned transition.
Tests/checkers/docs to add or update: proposer reward finality-delay regression, current-head competitor
admission with branch-local parent snapshots, side-branch/finality convergence coverage, passive-operator
height-convergence evidence, and rolling restart evidence output.
Narrow validation commands: `cargo test -p tensor_vm proposer_reward -- --nocapture`,
`cargo test -p tensor_vm competing_useful_head -- --nocapture`, and local CPU compose script-shape tests.
Broad validation commands before commit: `cargo test -p tensor_vm local_testnet --release`,
`cargo fmt --check --all`, `git diff --check`, and rebuilt local CPU Docker checkers including rolling
restart continuity.
Expected observable evidence: competing unfinalized branches no longer earn rooted proposer claims before
votes finalize a block; after finality, the canonical block exposes a pending proposer reward with the same
maturity delay and claim-only spendability as before. If a useful competing branch reaches finality before
the current unfinalized head, block-vote application promotes that branch through consensus state instead of
leaving observers parked at the seeded height.
Out of scope: public/CUDA deployment evidence, production drand/VRF signatures, and interactive transcript
disputes.
Split trigger: if finality-delayed accrual requires broader storage/status schema changes, first land the
chain-state reward timing and fork-admission tests, then extend explorer/status formatting in a follow-up.
Canonical owner: block-vote finality and chain reward state. The checker must observe delayed rewards; it
must not compensate for pre-finality reward-root drift.
Adapter callers: validator proposer ticks, p2p block and vote ingest, status/explorer pending reward views,
and local CPU restart evidence scripts.
Old shortcut removed: unfinalized competing blocks no longer create immediately rooted proposer reward
claims that adapters then have to work around during convergence.
Regression evidence: tests must show no proposer pending reward before finality, reward creation when the
block first finalizes, idempotent duplicate votes, side-branch vote acceptance/promotion, and unchanged
claim-owned spendability after maturity.
Behavior with local synthetic block production disabled: finality-delayed rewards are a chain-state rule
triggered by signed block votes and do not depend on local job synthesis.
Behavior for producer and non-producer roles: producers and passive observers ingest the same finalized
block votes and derive the same pending proposer reward state.
Structured evidence source: block votes, finalized block set, pending proposer reward claims, reward roots,
and all-operator finalized-head status.
Finality source: unchanged signed validator block-vote threshold.
Wire-size/codec impact: no new p2p payload; existing block and vote codecs carry all required evidence.

Latest validation evidence: Gate 0 first executable command for this resume,
`cargo test -p tensor_vm local_testnet --release`, passed on June 21, 2026. On June 22, 2026, the rebuilt
local CPU Docker proof passed `check-local-testnet.sh`, `check-restart-continuity.sh validator-02`, and a
sampled `check-rolling-restart-continuity.sh miner-03 validator-02`. On June 22, 2026, the rolling matrix
covered all 15 counted operators: `miner-00` through `miner-06` passed in the default rolling run before a
transient `miner-01` status-read timeout during `miner-07`; `miner-07` and `miner-08` then passed in a
continuation run; and `miner-09` plus `validator-00` through `validator-04` passed in the final
continuation. The rolling evidence reported delayed proposer reward claims, applied diagnostic block-check
evidence before restart counter reset, all 15 operators in the finalized common-head quorum, preserved peer
identities and pre-restart finalized common head/state root, preserved tensor artifacts, and plateaued
advancement flags (`restart_blocks_continue=false`, `rolling_restart_blocks_continue=false`) because no
additional finalized block was needed after the stable common-head snapshot.

### Iteration 142: Durable Restart-Rehydrated Tensor Artifacts

Feature capability: tensors served before a process restart are persisted as node artifacts and rehydrated
into both RPC gateway tensor routes and libp2p request-response serving on startup.
Readiness requirements covered: `upow.md` tensor-serving process boundary and retention model, `mvp_spec.md`
validator tensor fetches through node RPC/request-response, and the local CPU rolling-restart requirement
that tensor artifacts remain fetchable after service restart.
Files/modules likely touched: `crates/tensor_vm/src/storage/node_store.rs`,
`crates/tensor_vm/src/app/runtime_services.rs`, tensor insertion paths in `app/network.rs`,
`app/miner_role.rs`, and `app/validator_fetch.rs`, local CPU restart checker/tests, and this execution
plan.
Parallel subagents run: readiness mapper, durable artifact codebase explorer, and restart/storage test
coverage explorer.
Parallelizable implementation workstreams: read-only mapping in parallel; parent owns storage/runtime/checker
edits because persistence, hydration, and observable restart evidence must stay aligned.
Tests/checkers/docs to add or update: tensor artifact storage roundtrip/corruption coverage, startup
hydration coverage for both serving maps, restart checker probes for the same pre-restart tensor id/root,
and local CPU compose static script assertions.
Narrow validation commands: `cargo test -p tensor_vm node_store --lib`,
`cargo test -p tensor_vm runtime_services --lib`, and local CPU compose script-shape tests.
Broad validation commands before commit: `cargo test -p tensor_vm local_testnet --release`,
`cargo fmt --check --all`, and `git diff --check`.
Expected observable evidence: `/tensor/latest`, `/tensor/:id/descriptor`, `/row/0`, `/chunk/0`, and
`/opening/0` can serve a pre-restart tensor after restart, with the same tensor id and commitment root.
Out of scope: role runtime counter restoration, public/CUDA evidence, production drand/VRF signatures, and
interactive transcript disputes.
Split trigger: if libp2p remote-by-root restart proof requires multi-node Docker orchestration beyond local
artifact hydration, land storage/startup hydration first and keep peer-fetch proof as a follow-up.
Canonical owner: `NodeStore` tensor artifact persistence and runtime startup hydration; chain state roots
remain the owner of receipts/jobs/rewards, not artifact bytes.
Adapter callers: `RpcNode`, `TensorVmLibp2pService`, miner/validator runtime insertion paths, and local
CPU restart checkers only observe or write through the canonical store.
Old shortcut removed: restart evidence no longer relies on fresh synthetic work repopulating in-memory
gateway tensors or on checker sleeps that tolerate missing tensor routes. The same artifact must survive
process restart.
Regression evidence to add: persistence roundtrip including nonzero tensor scale, malformed artifact
rejection, startup hydration into RPC and libp2p stores, and restart checker assertions for descriptor,
row, chunk, and opening routes.
Behavior with local synthetic block production disabled: already persisted tensor artifacts still hydrate
and serve through RPC/libp2p after startup without requiring a new synthetic round.
Behavior for producer and non-producer roles: any role that inserts or fetches tensors writes the same
artifact format, and startup hydration is independent of producer election.
Structured evidence source: durable files under the node data directory, RPC tensor routes, libp2p
request-response tensor store, and local CPU rolling-restart checker output.
Finality source: unchanged block finality; tensor retention is artifact availability policy, not consensus
finality.
Wire-size and codec boundary: durable storage uses its own bounded tensor artifact codec including shape,
dtype, layout tag, scale, values, tensor id, and commitment root; it must not reuse the current p2p tensor
payload codec that omits scale.

Validation:

```bash
cargo test -p tensor_vm node_store --lib
cargo test -p tensor_vm runtime_services --lib
cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture
cargo test -p tensor_vm --test tvmd_runtime miner_role -- --nocapture
cargo test -p tensor_vm --test tvmd_runtime validator_role -- --nocapture
cargo test -p tensor_vm rpc::tests::tensors --lib
cargo test -p tensor_vm local_testnet --release
cargo fmt --check --all
git diff --check
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
```

### Iteration 141: Explicit Pre-Inclusion Receipt Reward Delay

Feature capability: receipt rewards that inherit a fraud/redundancy hold before block inclusion remain in
an explicit awaiting-inclusion delayed state until a canonical block includes the receipt, instead of
appearing height-claimable and relying on release-time inclusion filtering.
Readiness requirements covered: `upow.md` §12 reward-finality delay, `mvp_spec.md` receipt reward
maturity, redundant delayed settlement, and the shortcut ban against adapter/checker-only reward policy.
Files/modules likely touched: `crates/tensor_vm/src/chain/state.rs`,
`crates/tensor_vm/src/chain/settlement.rs`, `crates/tensor_vm/src/chain/blocks.rs`,
`crates/tensor_vm/src/chain/roots.rs`, `crates/tensor_vm/src/storage/chain_state.rs`, focused chain/RPC
tests, and this execution plan.
Parallel subagents to run: readiness mapper, reward-state codebase explorer, and reward test-coverage
explorer.
Parallelizable implementation workstreams: read-only exploration in parallel; parent owns code/docs edits
because the reward enum, storage codec, roots, status, and tests share the same files.
Tests/checkers/docs to add or update: focused reward maturity regression, storage/root coverage where the
new state is encoded, and exec-plan evidence.
Narrow validation commands: `cargo test -p tensor_vm reward -- --nocapture`,
`cargo test -p tensor_vm settlement -- --nocapture`, and targeted storage/RPC tests if touched.
Broad validation commands before commit: `cargo test -p tensor_vm local_testnet --release`,
`cargo fmt --check --all`, and `git diff --check`.
Expected observable evidence: pending receipt reward views expose awaiting inclusion with a future
claimable height, and inclusion converts that claim into the existing claimable or validator-reveal-held
maturity before `ClaimReward` can release it.
Out of scope: rolling restart tensor artifact persistence, public/CUDA evidence, production drand/VRF
signatures, and interactive transcript disputes.
Split trigger: if making the new maturity state requires broad checker or explorer schema migration beyond
the unified pending reward claim view, split display/schema work from chain-state semantics.
Canonical owner: `ReceiptRewardMaturity::AwaitingInclusionUntil`, `PendingReceiptReward`,
`apply_block_to_parent_state`, `release_matured_receipt_rewards_with_policy`, reward roots, and the
chain-state storage codec.
Adapter callers: status, explorer JSON, local checkers, and role/runtime code only observe the unified
pending reward claim view.
Old shortcut removed: redundant/challenge/audit holds applied before inclusion no longer make receipt
rewards look height-claimable while relying on `included_receipts` in the release filter. The claim stays
awaiting inclusion with its rooted hold height until a block includes the receipt.
Regression evidence: `pre_inclusion_reward_delay_stays_awaiting_inclusion_until_block_inclusion` covers
the maturity state directly; `redundant_agreement_quorum_is_required_before_settlement` covers redundant
holds composing with inclusion; block-check, invalid-output, and unavailable-data tests cover voided
claims held without credit; reward-root and storage roundtrip tests cover persistence/rooting.
Behavior with local synthetic block production disabled: pending reward maturity is chain state and is
converted only by canonical block inclusion or chain-owned reveal/challenge commands.
Behavior for producer and non-producer roles: imported blocks apply the same child-state transition and
reward-root checks as producers, including pre-inclusion delayed claims.
Structured evidence source: `ChainState::pending_reward_claims`, service status pending receipt reward
claim tuples, explorer pending rewards, reward roots, and persisted chain snapshots.
Finality source: unchanged signed validator block votes and finality threshold; reward delay controls
spendability and TensorWork activation only.
Wire-size and codec boundary: no new p2p payload; the existing chain-state storage codec adds a new
pending receipt reward maturity tag.

Validation:

```bash
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm pre_inclusion_reward_delay_stays_awaiting_inclusion_until_block_inclusion -- --nocapture
cargo test -p tensor_vm redundant_agreement_quorum_is_required_before_settlement -- --nocapture
cargo test -p tensor_vm produced_blocks_delay_receipt_rewards_from_inclusion_height -- --nocapture
cargo test -p tensor_vm reward -- --nocapture
cargo test -p tensor_vm settlement -- --nocapture
cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture
cargo test -p tensor_vm service_status_exports_pending_reward_claim_maturity_details -- --nocapture
cargo test -p tensor_vm explorer_overview_exports_validator_audit_economic_calibration -- --nocapture
cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract -- --nocapture
cargo fmt --check --all
git diff --check
```

### Iteration 140: Block-Applied Linear Model Transitions

Feature capability: selected `LinearTrainingStep` receipts now advance canonical model state during
deterministic block child-state application, not during proposer parent-state preparation.
Readiness requirements covered: `upow.md` §11 deterministic blockspace/state transition, `mvp_spec.md`
§20.3/§23 model state, and the local CPU evidence requirement that linear work has chain-visible model
state rather than checker-only settlement inference.
Canonical owner: `apply_block_to_parent_state`, `apply_selected_linear_model_transitions`, `ModelState`,
state roots, service status, explorer summary, and the local CPU checker.
Old shortcut removed: `prepare_parent_state` no longer walks settled receipts and mutates model state
before block production. The checker no longer treats `model_count` alone as a settlement proxy; it also
surfaces `model_step_total` and separately scans finalized blocks for live linear receipt evidence.
Regression evidence: `block_application_registers_missing_linear_job_model_before_transition` covers the
missing-model registration path, and existing block tests assert block/state-root recomputation after
linear transitions.
Docker evidence: local CPU compose checker passed on June 21, 2026 with `live_model_step_total=1`,
`live_delayed_receipt_reward_claims=18`, `live_delayed_proposer_reward_claims=1`, and finalized passive
operator convergence. Rolling restart continuity was attempted after restarting `miner-03` and remains a
separate blocker because restarted role runtime counters and gateway tensor artifacts are not yet stable
post-restart invariants.

Validation:

```bash
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm chain::tests::blocks -- --nocapture
cargo test -p tensor_vm explorer -- --nocapture
cargo test -p tensor_vm service_status -- --nocapture
cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture
cargo test -p tensor_vm_explorer explorer_json_and_shell_include_live_websocket_contract -- --nocapture
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
cargo fmt --check --all
git diff --check
```

### Iteration 139: Preserve Validator Reveal Holds Through Later Reward Delays

Feature capability: additional receipt reward delays extend a validator reveal-held reward without
unlocking it into a plain `ClaimableAt` maturity.
Readiness requirements covered: `upow.md` §12 reward-finality delay, `mvp_spec.md` §20.3 receipt reward
maturity, and the shortcut ban against compensating for reward policy at release/checker time.
Canonical owner: `ReceiptRewardMaturity::delayed_until`, `PendingReceiptReward`, block inclusion,
challenge/audit delay callers, reward release, and reward roots.
Old shortcut removed: a reward already held as `AwaitingValidatorVrfReveal(height)` is no longer converted
to `ClaimableAt(max(height, later_delay))` when a later fraud-window or redundant-settlement delay applies.
It remains `AwaitingValidatorVrfReveal(max(height, later_delay))` until the accepted reveal converts it.
Regression evidence: `extending_reward_delay_preserves_validator_vrf_reveal_hold` covers the state
transition directly, and the redundant-settlement test now separately asserts miner inclusion-delay
maturity and validator reveal-held maturity.
Docker evidence: the release image built and all 15 local CPU services became healthy, but
`check-local-testnet.sh` failed before this code change because canonical `model_count` remained 1 while a
minority validator view had already observed 2. The full Docker proof remains the next local evidence item.

Validation:

```bash
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm extending_reward_delay_preserves_validator_vrf_reveal_hold -- --nocapture
cargo test -p tensor_vm validator_receipt_reward_waits_for_vrf_reveal_after_maturity -- --nocapture
cargo test -p tensor_vm rewards -- --nocapture
cargo test -p tensor_vm settlement -- --nocapture
```

### Iteration 138: Explicit Validator Reveal Reward Hold

Feature capability: validator receipt rewards are delayed by a canonical pending-claim maturity state while
the matching validator VRF reveal is absent, instead of looking height-claimable and being filtered only at
release time.
Readiness requirements covered: `upow.md` §10 commit/reveal evidence, `upow.md` §12 reward-finality delay,
`mvp_spec.md` §20.3 receipt reward maturity, and the shortcut ban against adapter/checker-only reward
policy.
Canonical owner: `ReceiptRewardMaturity`, `PendingReceiptReward`, block child-state inclusion transition,
`SubmitValidatorVrfReveal`, reward roots, and storage codec.
Adapter callers: `ClaimReward`, release helpers, status, explorer JSON, p2p/node reveal ingest, and
validator-role reveal submission all observe or call the chain-owned state.
Old shortcut removed: a validator receipt reward no longer appears as normally claimable by height while
the reveal is missing; it is rooted as `AwaitingValidatorVrfReveal(height)` until reveal submission.
Regression test that proves the shortcut is gone:
`validator_receipt_reward_waits_for_vrf_reveal_after_maturity` now asserts the explicit reveal-wait
maturity and the beneficiary `ClaimReward` path.
Behavior with local synthetic block production disabled: inbound reveal payloads still apply through
`SubmitValidatorVrfReveal`; the reward hold is state-owned and independent of producer scheduling.
Behavior for producer and non-producer roles: producers and non-producers persist the same pending reward
root; a reveal accepted on either path unlocks the same pending claim.
Structured evidence source: pending reward claim views now expose `awaiting_validator_vrf_reveal`,
service status includes the reveal-wait slot in receipt-claim tuples, and explorer JSON emits
`awaiting_validator_vrf_reveal`.
Finality source: unchanged signed validator block votes and finality threshold; reveal maturity gates
reward spendability only.
Wire-size and codec boundary: no new p2p message; the existing bounded validator reveal payload unlocks the
chain-owned pending claim.
Out of scope: public drand signature verification, production validator VRF signatures, public/CUDA
evidence, Docker checker rerun, and interactive trace disputes.

Validation:

```bash
cargo test -p tensor_vm local_testnet --release
cargo check -p tensor_vm
cargo test -p tensor_vm validator_receipt_reward_waits_for_vrf_reveal_after_maturity -- --nocapture
cargo test -p tensor_vm reward -- --nocapture
cargo test -p tensor_vm service_status_exports_pending_reward_claim_maturity_details -- --nocapture
cargo test -p tensor_vm_explorer pending_reward -- --nocapture
cargo test -p tensor_vm explorer_json_and_shell_include_live_websocket_contract -- --nocapture
cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture
cargo test -p tensor_vm reward_root_commits_to_all_pending_reward_ledgers -- --nocapture
cargo test -p tensor_vm explorer_overview_exports_validator_audit_economic_calibration -- --nocapture
cargo test -p tensor_vm_explorer -- --nocapture
cargo fmt --check --all
git diff --check
```

### Iteration 137: Validator VRF Reveal Records

Feature capability: chain-verified local validator reveal records are rooted, persisted, bounded on the
p2p wire, retried through node ingest, surfaced in status/explorer/checker evidence, and required before
positive validator receipt rewards can release.
Readiness requirements covered: `upow.md` §10 commit/reveal randomness binding, `upow.md` §12 delayed
reward finality, `mvp_spec.md` Gate 0 status evidence, and the shortcut ban against checker-only policy.
Canonical owner: `ChainState`, `ChainCommand::SubmitValidatorVrfReveal`, chain validation, state roots,
reward release, and storage codec.
Adapter callers: validator role builds a reveal through the chain helper after local verification; p2p/node
runtime relays, queues, and applies bounded reveal payloads through the same chain command.
Old shortcut removed: validator receipt rewards can no longer become spendable by height/inclusion alone.
Regression test that proves the shortcut is gone: `validator_receipt_reward_waits_for_vrf_reveal_after_maturity`.
Behavior with local synthetic block production disabled: inbound reveal payloads still decode, queue, retry,
apply, and persist through the shared network path when receipt anchors arrive.
Behavior for producer and non-producer roles: producers publish accepted reveals; non-producers apply or
queue them without classifying out-of-order dependencies as invalid network events.
Structured evidence source: `randomness_validator_vrf_reveal_count`, role network reveal counters,
explorer JSON, local checker `live_validator_vrf_reveals`, and pending reward claim tests.
Finality source: unchanged signed validator block votes and finality threshold; reveal records gate reward
release, not block finality.
Wire-size and codec boundary: `NewValidatorVrfRevealPayload` uses the shared bounded p2p codec and maps to
the attestations gossip topic with no request-response protocol.
Out of scope: public drand signature verification, production validator VRF signatures, public/CUDA
evidence, and interactive trace disputes.

## Recent Iterations

- Iteration 136 enforced chain-visible validator proposer cadence in commit `22464c7`.
- Iteration 135 relayed external randomness beacon records over bounded p2p/node ingest in commit
  `e5010dd`; docs evidence landed in `92e28c9`.
- Iteration 134 proved multi-validator proposer competition with a runtime-delayed second proposer in
  commit `3655076`; Iteration 136 is replacing that runtime-delay proof with chain-visible cooldown state.
- Iteration 133 made delayed rewards claim-owned in normal block state in prior pushed commits.
- Iteration 132 wired local external beacon runtime evidence; Iteration 131 cleaned reward maturity
  boundary behavior; Iteration 130 cleaned local proof status drift.
- Iterations 120-129 cover delayed proposer/challenge rewards, heightless awaiting-inclusion receipt
  rewards, workflow docs, redundant settlement, operator-aware collusion evidence, and trace openings.
- Iterations 110-119 cover fixed-point rescale semantics, packed int8 payloads, external graph artifact
  fetch, and explicit receipt reward maturity.

## Decision Log

- `upow.md` is canonical; `mvp_spec.md` wins where `upow.md` is silent.
- Gate 0 command `cargo test -p tensor_vm local_testnet --release` must be the first executable
  acceptance command of every new/resumed implementation iteration.
- TensorWork is never proposer selection input; block proposal is validator-owned useful-verification PoW.
- Consensus mutation belongs in shared chain/IR/verifier layers, not `tvmd`, p2p/RPC adapters,
  deployment scripts, or checker-only branches.
- Multi-agent writer work is not used unless explicitly requested and file ownership is non-overlapping.

## Validation Evidence

Latest local validation is Iteration 146 on June 22, 2026:

```text
cargo test -p tensor_vm local_testnet --release
cargo test -p tensor_vm block_check_challenge -- --nocapture
cargo test -p tensor_vm network_event_driver_applies_observed_block_check_challenge_without_punishing_canonical_reward -- --nocapture
cargo test -p tensor_vm selected_validator_proposer_emits_idle_fallback_block -- --nocapture
cargo test -p tensor_vm validator_role -- --nocapture
cargo test -p tensor_vm validator_role_votes_valid_side_branch_after_canonical_vote -- --nocapture
cargo test -p tensor_vm block_vote_payload_finalizes_and_promotes_useful_side_branch -- --nocapture
cargo test -p tensor_vm competing_useful_head_uses_candidate_parent_snapshot -- --nocapture
cargo test -p tensor_vm competing -- --nocapture
cargo test -p tensor_vm reward -- --nocapture
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
sh -n deploy/tensorvm/local-cpu/scripts/entrypoint.sh
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
cargo fmt --check --all
git diff --check
```

Docker evidence during this iteration:

```text
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
```

Earlier Iteration 144 Docker runs reached a main-checker pass before later delayed-reward changes exposed
the diagnostic observed-block punishment gap. Those earlier runs are superseded by the Iteration 146 Docker
status below.

Iteration 146 replaces the prior immediate proposer-reward workaround with actual delayed proposer
reward materialization: pending proposer rewards are inserted only after the canonical block is finalized
and claimable-at height is computed from the configured settlement/challenge/hold windows. Block
production, network admission, side-branch promotion, vote finality, reorg, and challenge application now
materialize finalized proposer rewards through the shared chain path. The local CPU profile was also
changed to seed every operator before role startup, use a single deterministic selected validator proposer
(`validator-02`) to avoid 3-of-5 early fork deadlock, and publish diagnostic block-check challenges after
pending proposer rewards actually exist. Observed diagnostic challenge payloads now verify against the
full observed checks tree and tolerate receiver-local parent anchor differences for observed invalid blocks.

Current Docker readiness status for image `sha256:0669aaf0113669f80a4e1df3c761e9f08a88e1f25b1e153b32d0fe39ba9a613d`:

```text
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
local_cpu_testnet_ready=true
live_pending_proposer_rewards=1
live_delayed_proposer_reward_claims=1
live_pending_challenge_rewards=1
live_delayed_challenge_reward_claims=1
live_role_network_block_check_challenges_applied=70
```

Rolling restart continuity now separates restart safety from post-restart advancement. The latest rolling
coverage exercised every counted operator across the default run and continuations, with `miner-09` plus
all five validators passing in the final continuation while reporting preserved peer IDs, pre-restart
finalized common head/state root, tensor artifact preservation, and all-operator common-head convergence.
Because the local chain was already at a stable finalized common head when the snapshots were taken, the
advancement booleans were reported as false instead of failing the gate:

```text
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh miner-09 validator-00 validator-01 validator-02 validator-03 validator-04
local_cpu_rolling_restart_continuity_ready=true
rolling_restart_services=miner-09,validator-00,validator-01,validator-02,validator-03,validator-04
rolling_restart_service_count=6
rolling_restart_blocks_continue=false
rolling_restart_previous_common_head_preserved=true
rolling_restart_previous_common_state_root_preserved=true
rolling_restart_common_head_convergence=true
```

Current coverage blocker:

```text
cargo tarpaulin --workspace --offline
error: no such command: `tarpaulin`
```

Iteration 147 adds a production-aligned drand randomness admission boundary instead of treating
`proof_hash` as the proof. The chain now exposes `SubmitVerifiedDrandBeacon` for bounded
`pedersen-bls-unchained` drand evidence, verifies the BLS signature with `drand-verify`, derives the
admitted randomness from the signature, and commits typed proof metadata into the external randomness
record, durable state encoding, and state root. The existing `SubmitExternalRandomnessBeacon` command is
kept as the explicit `local-deterministic-fixture-v1` path for local testnet fixtures only.

Iteration 147 validation checkpoint:

```text
cargo test -p tensor_vm chain::tests::proposers --lib -- --nocapture
cargo test -p tensor_vm storage::chain_state --lib -- --nocapture
cargo test -p tensor_vm app::randomness_beacon --lib -- --nocapture
cargo test -p tensor_vm p2p::wire --lib -- --nocapture
```

## Archive

- Iterations 73-103: validator-audit calibration/appeal, diagnostic block-check challenges, fallback
  timeout, receipt-bound randomness, fork choice/side-branch storage, invalid-output slashing, role-owned
  graph production, typed block-check openings, retention deadlines, and inclusion-started reward delay
  landed across the archived commits listed in earlier plan revisions.
- Iterations 59-64: exact `clamp`, field `div`, split/einsum, registry/conformance guard, and graph
  verifier coverage landed across commits including `85a2956`, `d659e14`, and `b6e0887`.
