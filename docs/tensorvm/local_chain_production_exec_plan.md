# Local Chain Production Execution Plan

This file is the durable source of truth for local-chain production-readiness work. Keep it compact:
current status, active/recent iterations, validation evidence, blockers, and archive commit anchors only.

## Current State

- Active feature: Iteration 136 in progress - chain-visible validator proposer cadence.
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
- Next action: finish the local CPU Docker cadence proof, then continue public/CUDA deployment runs,
  public drand/VRF verification, and full interactive transcript disputes.

## Readiness Matrix

| Capability | Status | Evidence | Next action |
| --- | --- | --- | --- |
| Gate 0 local CPU testnet | Passing | First command this iteration: `cargo test -p tensor_vm local_testnet --release` passed on June 21, 2026 | Keep as first executable gate on every resume |
| Shared chain engine/profile-neutral API | Complete for current core | Shared `ChainEngine`, `ChainCommand`, profile tests, local-testnet Gate 0 | Preserve one transition engine while adding IR/runtime features |
| Role-owned miner receipts | Implemented locally | Miner role submits receipts through `ChainCommand::SubmitReceipt`; Docker checker reports `live_role_miner_receipts_submitted=402` | Keep Docker checker in local CPU gate |
| Role-owned validator attestations | Implemented locally | Validator role verifies assigned receipts, fetches tensors remotely, submits attestations | Keep as input path for IR-backed jobs |
| Role-owned validator block votes | Implemented locally | Validator role submits/gossips `SubmitBlockVote`; non-producers ingest/apply votes | Preserve append/finality separation |
| Role-owned validator proposer tick | Docker-proven locally | Latest local CPU Docker proof reports delayed proposer rewards, finalized passive-observer convergence, current-head useful competitor replacement, side-branch storage, automatic unfinalized deep reorg, and three-validator chain-visible proposer cooldown state | Continue public/CUDA evidence |
| Network-visible event ingestion | Implemented locally | Node runtime ingests decoded jobs, receipts, attestations, block payloads, votes, audits, and block-check challenges | Extend only through shared codecs/events |
| Canonical useful-verification block validity | Partial | UVPoW target/nonce, selected roots, typed check transcripts/leaves, submission-anchored opening retention deadlines, checks roots, beacon binding, fallback eligibility/timeout, parent snapshots, delayed rewards, diagnostic block-check challenges, current-head competitor policy, persisted side-branch fork storage, automatic unfinalized side-branch reorg, Docker proof | Remaining: full interactive transcript disputes |
| Tensor IR graph language | Partial | `TensorGraph`, canonical JSON, `graph_id`, registry validation, program storage/serving, graph jobs/receipts, exact replay for current core and broad Tier-B surface, packed int8 artifact APIs, role-owned graph execution, content-addressed `const_blob` replay/fetch, and p2p-sampled verified trace openings | Continue exact Tier-B verifier coverage, full interactive trace disputes, and CUDA graph evidence |
| Redundancy and delayed settlement | Partial | Independent miner assignment, operator-distinct redundant agreement quorum, watcher flags, state-rooted redundant settlement delay records with miner/operator counts, and delayed pending reward claims after redundant holds clear to settlement | Continue Tier-C committee policy and deployed public-operator evidence |
| Per-op `F_p` conformance vectors | Partial | Registry guard, CPU profile evidence, vectors for current admitted ops; default CUDA non-admission | Add CUDA conformance evidence and remaining exact Tier-B vectors |
| Randomness commit/reveal or VRF beacon | Partial | Receipts persist receipt-time finalized beacon randomness, assignment seed, validation seed commitment; attestations require anchor; local runtime ingests configured deterministic external beacon fixture; bounded p2p messages relay beacon records through node ingest; status/explorer/checker expose seed-domain, external beacon count/latest round, role applied counters, network-applied beacon counters, and block-hash-ban evidence | Add public drand verification, validator VRF construction, and deployed commit-reveal lifecycle |
| Economics and slashing invariant | Partial | Delayed rewards, reward-root binding, explicit receipt reward maturity state, inclusion-started receipt reward maturity, claim-owned spendability, delayed miner TensorWork activation, late invalid-output reward/work voiding and miner stake slashing, audit/data-unavailability slashing, appeal reversal, pending claim view, study helper, validator-audit/fraud-path calibration, and structured detection-probability evidence | Add deployed-run detection measurements and remaining fraud paths |
| Public deployment evidence | Not complete | Public evidence validators/templates exist; no real 7-day external run | Keep deployment-gated and do not claim full spec |

## Active Feature Iteration

### Iteration 136: Chain-Visible Validator Proposer Cadence

Feature capability: enforce a state-rooted proposer cooldown after a validator wins a block, so local
multi-proposer participation is evidenced by protocol-visible cadence instead of only a runtime-local
startup delay.
Readiness requirements covered: `upow.md` §11 validator-owned useful-verification PoW, `mvp_spec.md`
§4.6 canonical transition boundary, and the shortcut ban against runtime-only proposer eligibility.
Canonical owner: `ChainParams`, `ChainState`, chain block production/validation, and state roots.
Adapter callers: validator role runtime may poll on its local interval, but chain eligibility decides
whether a proposer can submit a block.
Old shortcut removed: `TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS` as the primary
local evidence for delayed proposer participation.
Regression test that proves the shortcut is gone: a validator inside the cooldown is rejected by chain
block production/admission, another registered validator can still propose, and local runtime/checker
status exposes positive chain-cadence evidence.
Behavior with local synthetic block production disabled: cadence applies only to validator block proposal;
inbound jobs, receipts, attestations, blocks, votes, and beacon payloads still ingest normally.
Behavior for producer and non-producer roles: any proposer-enabled validator may attempt proposal, but a
validator that recently won a block must wait for the chain cooldown; non-producers only observe/apply
valid blocks and status evidence.
Structured evidence source: typed status fields for proposer cooldown blocks and chain-cadence readiness,
checker aggregates, block status, and state-rooted proposer-cadence records.
Finality source: unchanged signed validator block votes and finality threshold; cadence affects admission
eligibility, not vote synthesis.
Wire-size and codec boundary: no new p2p payload; existing bounded block payloads carry blocks rejected or
accepted by chain cadence validation.
Out of scope: public drand signature verification, validator VRF construction, public/CUDA evidence, and
interactive trace disputes.

## Recent Iterations

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

Latest local validation is Iteration 136 on June 21, 2026:

```text
cargo check -p tensor_vm
cargo test -p tensor_vm proposer_cadence_cooldown_is_chain_visible_and_state_rooted -- --nocapture
cargo test -p tensor_vm chain_state_store_roundtrips_full_chain_and_detects_tampering -- --nocapture
cargo test -p tensor_vm --test tvmd_cli role_run_commands_serve_through_role_specific_surfaces -- --nocapture
cargo test -p tensor_vm --test local_cpu_compose local_cpu_compose_bundle_matches_spec_artifact_shape -- --nocapture
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
cargo fmt --check --all
git diff --check
```

Docker proof highlights: `live_role_validator_block_proposer_operators=3`,
`live_role_chain_cadence_validator_block_proposer_operators=3`,
`live_competing_validator_block_proposers=validator-00 validator-01 validator-02`,
`live_delayed_proposer_reward_claims=1`, `live_delayed_challenge_reward_claims=1`,
and `all_operator_p2p_target_head_observed=true`.

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
