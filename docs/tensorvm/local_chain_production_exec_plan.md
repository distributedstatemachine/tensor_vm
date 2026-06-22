# Local Chain Production Execution Plan

Durable source of truth for current status, active/recent iterations, validation evidence, blockers, and
archive commit anchors only.

## Current State

- Active feature: Iteration 180 complete: Local GraphExecution Checker Evidence.
- Current status: the local CPU checker now requires live generic GraphExecution receipt
  and finalized block evidence from the existing graph job runtime path. Graph receipt verification test
  scenarios now include arithmetic, reduction, transpose,
  unary sign/absolute, and cast coverage for admitted IR ops, with a frozen-registry drift test that fails
  when a consensus-admitted op lacks graph receipt verification scenario coverage. Network graph receipt
  payloads now wait on missing canonical program bodies instead of
  being misclassified as invalid when the graph job is already known through a direct/local state path.
  Pre-inclusion voided receipt rewards now prune directly after their explicit delayed hold even when the
  challenged receipt never reaches `included_receipts`. Automatic block-state reward pruning uses the
  receipt reward maturity policy for
  auto-prunable verifier-dependent receipt claims: voided miner claims and unavailable-data claims. Valid
  matured claims remain non-spendable pending claims until beneficiary `ClaimReward`, and voided validator
  audit claims stay on the explicit appeal-aware release path. Deterministic `F_p` conformance vectors now
  have unique identity checks, every
  consensus-admitted frozen registry op must have vector and CPU profile evidence, and any non-registry
  conformance vector/profile entry must be explicitly marked as an auxiliary verifier vector. Delayed
  proposer, receipt, challenge, validator-audit, and credit rewards are chain-owned
  pending claims. Valid matured claims remain non-spendable until the beneficiary calls `ClaimReward`;
  direct maturity-sweep commands can prune only voided/unavailable matured claims and cannot credit live
  beneficiary balances. Trace-bisection has signed sessions/rounds, bounded p2p open/expectation/round/
  referee payloads, pending-queue application, runtime session-open, challenger expectation, responder
  round generation, runtime challenger referee-witness generation from local graph replay, one-op referee
  verdicts, timeout settlement, slashing, and delayed challenger rewards.
- Current blockers:
  - Public 7-day external deployment evidence and CUDA miner evidence remain outside the local CPU proof.
- Next action: continue CUDA/public deployment evidence or remaining deployed-randomness/economic evidence
  without treating roadmap trace-bisection work as a v0 blocker.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | Iteration 178 first command `cargo test -p tensor_vm local_testnet --release` passed on June 22, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports live miner submissions | Keep Docker checker in local CPU gate |
| Role-owned validator attestations/votes/proposer tick | Implemented locally | Validator role submits attestations, block votes, and useful proposals through chain commands; local CPU proof covers convergence and delayed proposer rewards | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, blocks, votes, audits, block-check challenges, trace-bisection expectations/rounds/referees, drand, and validator reveals | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW, selected roots, checks roots, beacon binding, parent snapshots, delayed rewards, diagnostic challenges, side branches, and trace-bisection admission/economics | Add deployed public/CUDA proof or remaining fraud-proof DoS policy |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON/`graph_id`, registry validation, graph receipts, exact Tier-B replay, graph-verifier receipt scenarios for every consensus-admitted op, packed int8 APIs, const blobs, role-owned graph execution, and automatic runtime trace-bisection referee witnesses when isolated-opening roots match local replay | Require local checker evidence for live GraphExecution receipts/blocks; continue CUDA graph evidence, multi-round trace-bisection DoS policy, and incomplete-transcript handling |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant quorum, watcher flags, state-rooted redundant delay records, and delayed pending reward holds | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops, and graph-verifier receipt scenario drift coverage for every admitted op; default CUDA non-admission | Add CUDA conformance evidence and deployed CUDA profile evidence |
| Randomness commit/reveal or VRF beacon | Partial | Receipt anchors, validator reveal keys/proofs, verified local/public drand, chain-owned epoch windows, reward-release reveal gates | Add deployed full VRF construction and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, claim-owned spendability, delayed TensorWork activation, invalid-output/data-unavailability/audit/block-check/trace-bisection slashing and delayed bounties, calibration evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 180: Local GraphExecution Checker Evidence

Feature capability: the local CPU readiness checker must fail unless the live post-startup runtime exposes
generic GraphExecution evidence through the same explorer receipt and block-status surfaces already used for
TensorOp and LinearTrainingStep. This turns the existing role-owned graph job path into a local acceptance
gate instead of leaving it as indirect unit/p2p evidence.

Readiness requirements covered: `goal.md`/`upow.md` content-addressed Tensor IR graph language, generic
GraphExecution role/network evidence, and `mvp_spec.md` local checker evidence for live post-startup work.

Canonical owner: existing chain/RPC/block-status surfaces already own graph receipt/block data. This
iteration changes only the checker gate and status docs.

Adapter callers: `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` consumes
`/explorer/receipts/latest` and `tvmd node block` graph fields.

Old shortcut being removed: local readiness could pass while only proving live TensorOp and
LinearTrainingStep primitive evidence, even though generic GraphExecution jobs are part of the current IR
runtime.

Regression test that proves the shortcut is gone: targeted shell/static checks plus focused local-testnet
tests must show the checker names `graph_execution` receipt and block evidence.

Behavior with local synthetic block production disabled: unchanged. This checker evidence applies only to
the local CPU profile where the deterministic synthetic graph job source is enabled.

Behavior for producer and non-producer roles: unchanged. Producers and non-producers continue to rely on
the same chain/RPC block status and explorer receipt data.

Structured evidence source: `/explorer/receipts/latest/*` `primitive_type=graph_execution`,
`tvmd node block` `graph_execution_receipt_count`, checker output fields, and this exec plan.

Finality source: unchanged; finalized graph block evidence is read from `tvmd node block`.

Wire-size and codec boundary: unchanged; no p2p, RPC, storage, or consensus payload formats change.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; checker/docs-only acceptance evidence slice.

Tests/checkers/docs to add or update: local CPU checker, topology defaults if needed, deployment-doc tests
if a static checker assertion exists, and exec plan/readiness wording.

Narrow validation commands: `sh -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`; targeted
Rust deployment-doc/local-testnet checker tests if discoverable.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin unless the change remains shell/docs-only with no Rust coverage impact.

Expected observable evidence: checker output includes `live_graph_execution_receipts=true`,
`live_graph_execution_block_evidence=true`, graph receipt counts, and finalized graph block height/receipt
counts.

Out of scope: changing graph execution semantics, CUDA graph evidence, public deployment evidence, and new
standalone verifier binaries.

Split trigger: split if the existing local graph runtime does not produce live graph receipts reliably and
requires runtime scheduling changes beyond checker evidence.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo tarpaulin --version` passed: `cargo-tarpaulin-tarpaulin 0.35.5`.
- `sh -n deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` passed.
- `cargo test -p tensor_vm local_cpu_checker_requires_live_graph_execution_evidence --lib` passed: 1
  targeted deployment-doc test.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 546 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 561
  instrumented tests, 84.54% line coverage, 22603/26736 lines covered.
- No standalone verifier binary was used or added; validation used shell checks, Rust tests, clippy, and
  tarpaulin.
- Commit `92e5602` (`Require graph execution checker evidence`) pushed to `origin/main`.

### Iteration 179: Graph Verifier Admitted-Op Receipt Coverage

Feature capability: local CPU graph execution verification now has explicit receipt-scenario evidence for
every consensus-admitted frozen registry op. A new arithmetic/reduction/cast graph receipt covers the
previously indirect admitted ops: `matmul`, `add`, `sub`, `mul`, `scalar_mul`, `reduce_sum`, `mean`,
`transpose`, `identity`, `neg`, `abs`, `sign`, and `cast`. A companion registry-drift test fails if a
future admitted op is not represented in the graph-verifier receipt scenario set.

Readiness requirements covered: `goal.md` deterministic `F_p`/Tensor IR verifier coverage and
`mvp_spec.md` CPU local-reference graph verifier evidence for admitted Tier-A/Tier-B ops. This is local CPU
evidence only; it does not claim CUDA conformance or deployed public evidence.

Canonical owner: `crates/tensor_vm/src/verify.rs` graph execution verification tests.

Adapter callers: no runtime, p2p, RPC, storage, or consensus adapters changed.

Old shortcut being removed: graph verifier receipt coverage for several admitted ops was implicit through
lower-level IR execution and conformance tests rather than directly exercised through graph receipt
verification.

Regression test that proves the shortcut is gone:
`graph_verifier_accepts_arithmetic_reduction_and_cast_receipt` verifies an accepted graph receipt and
checks conformance-profile gating for the newly covered op group.
`graph_verifier_receipt_scenarios_cover_every_consensus_admitted_op` compares the graph-verifier receipt
scenario set against the frozen consensus-admitted registry.

Behavior with local synthetic block production disabled: unchanged; this is test-only verification
coverage.

Behavior for producer and non-producer roles: unchanged.

Structured evidence source: focused graph verifier tests, full library tests, release local-testnet gate,
clippy, tarpaulin, and this exec plan.

Finality source: unchanged.

Wire-size and codec boundary: unchanged.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to graph verifier tests.

Tests/checkers/docs to add or update: graph verifier admitted-op receipt tests and exec plan.

Narrow validation commands: `cargo test -p tensor_vm graph_verifier --lib`.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin.

Expected observable evidence: every consensus-admitted frozen IR op is named in an accepted graph verifier
receipt scenario set, and a graph receipt using the previously indirect arithmetic/reduction/cast group
verifies as valid under the CPU conformance profile.

Out of scope: CUDA conformance admission, public deployment evidence, protocol behavior changes, and new
standalone verifier binaries.

Split trigger: split only if the new graph receipt exposes an actual interpreter/verifier bug requiring
non-test behavior changes.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm graph_verifier --lib` passed: 16 graph verifier tests.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 545 library tests.
- Post-change release gate `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 560
  instrumented tests, 84.54% line coverage, 22603/26736 lines covered.

### Iteration 178: Pre-Inclusion Voided Receipt Reward Delay

Feature capability: voided receipt rewards that are challenged before block inclusion keep their explicit
delayed hold and then prune without credit once that hold matures. This removes the leftover dependency on
`included_receipts` for claims that can no longer become spendable, so the reward delay itself is the
canonical lifecycle boundary rather than a later release workaround.

Readiness requirements covered: `upow.md` §12 reward-finality delay and `mvp_spec.md` §20.3 pending
receipt-reward lifecycle: verifier-dependent receipt rewards are pending claims, invalidated claims are
voided before spendability, and matured voided claims are pruned without crediting balances.

Canonical owner: `crates/tensor_vm/src/chain/commands.rs` owns reward claim/prune policy. Challenge and
attestation paths already set the delayed hold; this iteration makes the shared release policy honor that
state directly.

Adapter callers: block child-state projection via `release_all_matured_rewards`, explicit release helpers,
and beneficiary `ClaimReward` all share the same chain-owned reward policy.

Old shortcut being removed: pre-inclusion voided receipt rewards could be delayed correctly but then remain
stuck because pruning still required `included_receipts`, effectively depending on a later inclusion path
that a challenged receipt should not need.

Regression test that proves the shortcut is gone: add focused reward tests for delayed pre-inclusion
voided miner and unavailable-data receipt claims.

Behavior with local synthetic block production disabled: unchanged; this is a deterministic chain-state
reward transition used by every profile.

Behavior for producer and non-producer roles: unchanged; both project block child state and process
reward claims through the same chain transition.

Structured evidence source: focused reward regression, settlement challenge regressions, release
local-testnet gate, tarpaulin report, and this exec plan.

Finality source: unchanged; claims remain state-rooted through the configured hold and are pruned only
after maturity.

Wire-size and codec boundary: unchanged; no p2p, RPC, storage codec, or payload format changes.

Parallel subagents to run: none. The multi-agent tool policy only permits spawning when the user
explicitly asks for delegated agent work; this pass remains single-writer.

Parallelizable implementation workstreams: not split; the slice is confined to reward release policy and
focused tests.

Tests/checkers/docs to add or update: focused reward regression and exec plan.

Narrow validation commands: `cargo test -p tensor_vm chain::tests::rewards --lib` and relevant settlement
challenge regressions.

Broad validation commands before commit: fmt/check/diff, library tests, release local-testnet, clippy, and
tarpaulin.

Expected observable evidence: a voided pre-inclusion receipt reward survives before its hold height, is
removed after the hold matures, credits no spendable balance, and does not activate miner TensorWork.

Out of scope: changing reward amounts, challenge economics, validator-audit appeal semantics, public/CUDA
deployment evidence, and new verifier binaries.

Split trigger: split only if the policy change exposes unrelated audit-appeal or TensorWork activation
failures requiring a separate ownership pass.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm chain::tests::rewards --lib` passed: 19 reward lifecycle tests.
- `cargo test -p tensor_vm unavailable_data_evidence_voids_delayed_receipt_rewards_before_release --lib`
  passed.
- `cargo test -p tensor_vm invalid_output_evidence_voids_delayed_receipt_rewards_before_release --lib`
  passed.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 543 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 558
  instrumented tests, 84.49% workspace line coverage, 22588/26736 lines covered.
- Initial parallel clippy failed because tarpaulin concurrently cleaned `target/debug`; rerun alone passed:
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Manual verifier-style review: no standalone verifier binary was used or added; the change stays inside
  chain reward-prune policy, preserves `ClaimReward` as the only live-reward credit path, and changes no
  p2p/RPC/storage wire format.
- Feature commit `638ba58` pushed to `main` on June 22, 2026:
  `git push` returned `ab26932..638ba58  main -> main`.

### Iteration 177: Graph Receipt Pending Program Boundary

Feature capability: network graph receipt payloads become order-tolerant at the same canonical program
boundary as graph job payloads. If a `GraphExecution` receipt arrives after the job/miner are known but
before the registered `program_body`, node payload application returns `Pending` so existing program-fetch
and pending-retry paths can admit the receipt once the canonical graph body is available.

Readiness requirements covered: `upow.md` §4.5-§4.6 content-addressed Tensor IR graph admission and
`mvp_spec.md` canonical runtime/transition boundary for jobs, receipts, and p2p/node ingestion.

Canonical owner: chain admission remains `ChainCommand::SubmitReceipt` and `chain::receipts` validation;
node payload application only classifies missing dependencies as pending before submitting to the chain.

Adapter callers: p2p/node event ingestion, pending-payload retry, and runtime graph artifact fetch paths
call `apply_network_receipt_payload`.

Old shortcut being removed: graph receipt payloads could be rejected as invalid merely because the
canonical graph body had not arrived yet, even though graph job payloads already used a pending dependency
boundary for the same missing program body.

Regression test that proves the shortcut is gone: add a node payload-application test where a graph job is
known, the graph receipt payload arrives first and returns `Pending`, registering the graph body makes the
same payload `Applied`, and conflicting payloads still reject.

Behavior with local synthetic block production disabled: unchanged; this is inbound network dependency
classification and applies regardless of producer capability.

Behavior for producer and non-producer roles: both producers and non-producers share the same node payload
application path and pending retry semantics.

Structured evidence source: focused node payload-application test, existing graph job pending-program
test, exec plan, and tarpaulin report after validation.

Finality source: unchanged; no block/finality mutation.

Wire-size and codec boundary: unchanged; no new p2p message or codec. Existing bounded receipt payloads
are decoded before dependency classification.

Tests/checkers/docs to add or update: focused node payload application test, exec plan, and tarpaulin
report if coverage changes.

Validation completed on June 22, 2026:
- First executable gate before edits: `cargo test -p tensor_vm local_testnet --release` passed.
- `cargo test -p tensor_vm graph_receipt_payload_waits_for_registered_program_body --lib -- --nocapture`
  passed.
- `cargo test -p tensor_vm node::payload_application --lib` passed: 19 node payload tests.
- `cargo fmt --all -- --check` passed.
- `git diff --check` passed.
- `cargo test -p tensor_vm --lib` passed: 542 tests.
- `cargo test -p tensor_vm local_testnet --release` passed: 5 release lib tests plus the filtered
  `tvmd_cli` local-testnet gateway test.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo tarpaulin --workspace --timeout 120 --out Xml --output-dir target/tarpaulin` passed: 557
  instrumented tests, 84.49% workspace line coverage, 22588/26736 lines covered.
- Manual verifier-style review: no standalone verifier binary was used or added; the change stays in node
  dependency classification, uses existing bounded receipt payload decoding, preserves
  `ChainCommand::SubmitReceipt` as canonical graph receipt admission, and adds no new wire format.
- Feature commit `1006b70` pushed to `main` on June 22, 2026:
  `git push` returned `ee329bc..1006b70  main -> main`.

Out of scope: new graph-program gossip message, CUDA graph execution evidence, public deployment evidence,
and changing graph receipt verification semantics.

## Recent Iterations

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
- Bounded p2p/node payloads remain the only network wire surface for randomness, reveal records, and
  trace-bisection expectation/round/referee evidence.
- Public 7-day evidence, CUDA evidence, deployed full VRF construction, incomplete-transcript
  final-opening automation, and multi-round DoS policy remain deployment or future-feature gates, not
  local-completion claims.

## Validation Evidence

- Iteration 178 validation passed on June 22, 2026: first executable Gate 0; focused reward and
  settlement challenge regressions; fmt/check/diff; `cargo test -p tensor_vm --lib` (543 passed);
  release local-testnet; clippy rerun after tarpaulin/target contention; tarpaulin 84.49%
  (22588/26736); manual reward-boundary review; feature commit `638ba58` pushed to `main`.
- Iteration 177 validation passed on June 22, 2026: first executable Gate 0; focused graph receipt pending
  program test; `cargo test -p tensor_vm node::payload_application --lib` (19 passed); fmt/check/diff;
  `cargo test -p tensor_vm --lib` (542 passed); release local-testnet; clippy; tarpaulin 84.49%
  (22588/26736); and manual node/chain boundary review.

## Archive

- Iteration 175 (`e45c876` plus metadata `b3c4bf9`, pushed `main` -> `main`): conformance vector/profile
  identity guard requiring unique vector IDs, registry-admitted coverage, and explicit auxiliary
  non-registry verifier vectors.
- Iteration 176 (`b96debd` plus metadata `ee329bc`, pushed `main` -> `main`): automatic block-state
  matured-reward pruning directly covers auto-prunable voided miner and unavailable-data receipt claims
  without credit while preserving `ClaimReward` as the only live-reward credit path.
- Iteration 174 (`b5bf0d9`, pushed `main` -> `main`): runtime challenger derives one-op
  trace-bisection referee witnesses from local graph replay and submits/gossips the existing bounded
  referee payload without adding any standalone verifier binary.
- Iteration 173 (`919f77f`, pushed `main` -> `main`): reward claim boundary regression hardening proved
  matured reward sweeps cannot credit verifier-dependent rewards before beneficiary `ClaimReward`.
- Iterations 171 through 172 (`6901655`, `bbb3d28` plus metadata `8b94508`, pushed `main` -> `main`):
  chain-owned challenger expected-root claims and bounded gossip prevent responder-carried branch roots
  from self-selecting trace-bisection branches.
- Iteration 170 (`d88a14d` plus metadata `04a85d4`, pushed `main` -> `main`): runtime responder round
  generation builds signed midpoint rounds from local graph evidence, applies them through
  `ChainCommand::SubmitTraceBisectionRound`, persists state, publishes bounded round payloads, and reports
  role counters.
- Iteration 169 (`091142d` plus metadata `16e9b17`, pushed `main` -> `main`): runtime roles detect local
  disputable graph evidence and open signed trace-bisection sessions through shared chain commands plus
  bounded gossip.
- Iterations 165 through 168 (`e3af101`, `bfcefa7`, `a93676b`, `54912ce`, all pushed `main` -> `main`):
  established bounded signed session-open gossip, explicit challenge reward claim boundaries,
  trace-bisection timeout slashing, and one-op referee slashing with delayed challenger rewards.
- Iteration 164 (`e42ad44`, pushed `main` -> `main`): one-op referee witness crosses node boundaries as a
  bounded gossip payload and is admitted only through shared node pending queue plus
  `ChainCommand::RefereeTraceBisection`.
- Iterations 158 through 163 (`6f6344a`, `713c6a4`, `2662d5a`, `f1372a4`, `02e288f`, `0487f77`, all
  pushed `main` -> `main`): established signed trace-bisection core, delayed block-check challenger
  rewards, bounded round wire payloads, chain session/round admission, node pending-queue round
  application, input-rooted trace openings, and chain-owned one-op referee verdicts.
- Iterations 143 through 157 established graph exact-op coverage, verified drand/network randomness,
  validator reveal proofs, finality-delayed proposer rewards, finalized side-branch convergence, durable
  restart-rehydrated tensor artifacts, deployment preflight/evidence surfaces, rolling restart evidence,
  richer IR/Tier-B execution, delayed reward maturity, claim-owned spendability, audit and challenge
  reward holds, exact trace openings, and local CPU Docker proof evidence.
