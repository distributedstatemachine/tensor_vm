# Local Chain Production Readiness And Chain-Core Refactor Plan

## Superseded Assumptions (v2)

The MVP spec now uses useful-verification PoW with deterministic blockspace. Gaps tied to
TensorWork-weighted proposer selection or job-rooted blocks are obsolete. New gaps:

- Implement `useful_verification_pow` puzzle and difficulty retargeting.
- Extend the local settled-receipt pool and deterministic canonical selection into the remaining
  selected-leaf, expiry, carry-over, and challenge-window lifecycle model. Exact block parent snapshots
  are now stored by block hash for replay-stable apply evidence.
- Implement verification challenge window with cross-validator dispute path.
- Keep local checker assertions for PoW block validity, canonical blockspace evidence, and BFT finality
  independent as the live proposer network path is upgraded.

This document records the current local-chain readiness gaps and the refactor path for making TensorVM's
local chain production-grade while keeping it local-only. It combines the local setup review with an
architecture plan for using one shared chain base across local, testnet, and mainnet profiles.

The target is not public infrastructure. The target is a real local chain where all Docker Compose
participants run the same protocol code paths that a public testnet or mainnet profile would use.

## Architecture Decision

TensorVM should have one chain base. Local CPU, public testnet, and mainnet must share the same deterministic
state-transition engine, validation rules, settlement rules, proposer selection, block application, storage
contract, and libp2p message handling. They may differ only through profile configuration and deployment
adapters.

Accepted profile differences:

```text
chain ID and genesis state
operator set and bootstrap peers
job source policy
block and epoch timing
reward and faucet policy
service exposure and authentication policy
evidence requirements
storage paths and retention policy
```

Rejected differences:

```text
separate local-only chain transition logic
simulation shortcuts in production paths
in-memory propagation instead of the shared node event path
optional libp2p for any counted operator
testnet/mainnet-only validation or settlement code
role processes that bypass the chain engine
```

The repository boundary should stay simple: protocol, runtime, storage, networking, and local deployment
support live in the `tensor_vm` crate and deploy tree; non-protocol experiments, studies, and exploratory
tools live in the `experiments` crate. A feature can be experimental, but it should not require production
chain code to import experiment-only modules.

## Scope

Local production-ready means:

```text
CPU-only default execution remains supported
all 10 miners and 5 validators are real long-running participants
jobs, receipts, attestations, blocks, votes, and tensor fetches move through libp2p/RPC boundaries
all operators persist and sync chain state
the explorer reads live chain data from the node API
restart and rollback behavior is checked locally
the implementation remains explicitly non-public evidence
```

It does not mean:

```text
CUDA requirement
public DNS or TLS
systemd/nginx deployment
external independent operators
7-day public-run evidence
mainnet security claims
```

## Current State

The local bundle is useful and should remain the first operational target:

- `deploy/tensorvm/local-cpu/docker-compose.yml` starts 10 miner containers, 5 validator containers, and
  the standalone explorer.
- Each counted operator has a stable operator ID, stable libp2p identity seed, distinct volume, and
  mandatory libp2p readiness check.
- The libp2p runtime resolves Docker DNS bootstrap multiaddrs, preserves `/p2p/<peer-id>` dial targets, and
  redials bootstrap peers after disconnects so local peer counts recover across restarts.
- `miner-00` exposes local RPC, explorer data, faucet, telemetry, and the host-facing WebSocket endpoint.
- The current live producer keeps `/chain/head` advancing past the seeded two-block baseline.
- `check-local-testnet.sh` now fails if live jobs, receipts, settled receipts, height, and block count do
  not advance.
- Every operator now starts from the same deterministic local CPU seed and exposes durable node-store status
  through `tvmd node status`.
- The checker fails unless all 15 operator node stores advance past the seed, report role-specific status
  and live chain counters, report the same first live finalized block hash, and return the same finalized
  common-head block hash through `tvmd node block` before and after restart checks. It also selects
  a non-producer's latest finalized p2p-observed head from the block-payload gossip set, then fails unless
  every operator catches up to that same finalized block hash and state root, with a nonempty block-log root reported from
  every node store.
- Compose now marks only `validator-00` as the local timed synthetic job producer. Miners are never local
  block producers. All five validators are configured as validator block proposers under a shared
  chain-visible proposer cooldown, while every counted operator keeps the same seeded chain base and
  advances live blocks only after a block payload is decoded and verified against the shared chain path.
- `check-restart-continuity.sh` captures pre/post peer IDs, heights, block counts, state roots, block-log
  roots, and finalized common heads around actual Compose restarts, and fails unless restarted services
  keep identity, avoid durable-state regression, preserve the pre-restart finalized common head and state
  root, preserve the sampled tensor artifact, and reconverge on a finalized common head. It reports
  post-restart height/root advancement separately; a stable plateau is allowed when preservation and
  convergence hold. The restart gate passes the restarted service list into `check-local-testnet.sh`, so
  only those just-restarted services may satisfy volatile runtime gossip counters from preserved post-seed
  chain state and live peer connectivity. Process-lifetime role/network totals that reset across a rolling
  restart are also scoped to normal local readiness, while fresh local checks still require fresh
  gossip/runtime counters and role totals.
- `check-rolling-restart-continuity.sh` runs that continuity gate one service at a time across every
  counted miner and validator by default, turning the selected restart checks into a rolling all-operator
  matrix.
- `tvmd node init` validates the complete node store on restart and repairs torn snapshot/block-log
  state from `chain.state` before readiness is allowed.
- Compose now execs role-specific runtime commands for counted operators: all miners run `tvmd miner run`,
  all validators run `tvmd validator run`, `validator-00` carries the single local synthetic job producer flag,
  all five validators carry validator block-proposer flags with chain-visible proposer cooldown status,
  `tvmd node status` reports `runtime_command`, and the checker fails unless all 15 operators report the
  role command expected for their Compose service.
- Counted role runtimes now derive a chain address from their configured wallet label, persist
  `role_wallet_address`, `role_wallet_registration`, and `role_wallet_registered` in role runtime status,
  and expose those fields through `tvmd node status`. Compose wallet labels now match the deterministic
  seeded `LocalTestnet` miner and validator addresses, and the checker fails unless every counted operator
  reports a registered role wallet for its service class.
- Miner role loops now scan the loaded chain state for jobs assigned to their registered miner wallet,
  persist `role_miner_work_ready`, `role_miner_assigned_jobs_seen`, and
  `role_miner_unreceipted_jobs` in role runtime status, and expose those fields through
  `tvmd node status`. Miner role loops can now submit receipts for assigned unreceipted jobs through
  `ChainCommand::SubmitReceipt`, insert served tensor artifacts into their local node, publish receipt
  announcements through the existing p2p announcement path, and report `role_miner_receipts_submitted` plus
  `role_miner_tensors_inserted`. After the job-only producer split, the local checker now rejects a run
  unless at least one live miner role reports positive receipt submission and tensor insert counters.
- Validator role loops now scan the loaded chain state for receipts assigned to their registered validator
  wallet, distinguish unattested receipts with local tensor artifacts from receipts still missing local
  artifacts, submit assigned attestations through `ChainCommand::SubmitAttestation`, publish attestation
  announcements through the existing p2p announcement path, and report `role_validator_work_ready`,
  `role_validator_artifact_ready_receipts`, `role_validator_artifact_missing_receipts`, and
  `role_validator_attestations_submitted`. Validator role loops can now issue bounded libp2p
  request-response fetches for missing receipt tensor roots, verify the fetched tensor payloads against the
  requested commitment roots before inserting them locally, and report remote fetch attempts, successes,
  failures, bytes, and inserted tensor counters through role status. After the job-only producer split, the
  local checker now rejects a run unless at least one live validator role reports positive validator-owned
  attestation submissions.
- Validator role loops now also submit explicit block votes for unfinalized valid blocks through
  `ChainCommand::SubmitBlockVote`, publish block-vote payloads through the shared p2p announcement path,
  and report `role_validator_block_votes_submitted`, `role_network_block_votes_ingested`,
  `role_network_block_votes_applied`, and `role_p2p_observed_block_votes`. Local synthetic block production
  appends blocks but no longer fabricates finality votes in the runtime path.
- Scheduled local block production remains split from validator block proposal. The scheduled producer
  publishes only deterministic local jobs; it does not force empty fallback blocks through the role wallet.
  The validator role tick observes only not-yet-included settled receipts with local tensor artifacts and
  validator attestations, prepares chain-owned parent state, stores exact parent-state snapshots by block
  hash for replay-stable `BlockApplyOutcome`/status evidence, applies a rewarded block command only when
  useful settled receipt blockspace is available, and publishes the resulting block payload/header/hash.
  Validator block proposal is no longer gated by the local synthetic job producer path: a configured
  validator proposer can assemble a useful block from already accepted settled state even when synthetic
  job production is disabled. Producer-local
  receipt and attestation synthesis is no longer on the scheduled runtime path; the Docker checker now
  consumes structured role status and fails unless live miner and validator role loops produce positive
  receipt/tensor and attestation counters from those jobs, unless the validator producer reports positive
  useful block proposals with selected receipt, artifact-ready, and attested-receipt counts, and unless
  the live overview exposes a pending proposer reward claim for useful block production. Useful and empty
  fallback block production both release proposer claims only after the explicit full reward-settlement plus
  challenge-window maturity height is reached, while fallback blocks remain distinguishable from useful
  UVPoW, carry only the reduced proposer claim amount, and validate only for the deterministic
  stake-weighted fallback proposer selected from parent state and beacon after the configured
  `pow_timeout_blocks * block_time_seconds` delay for non-genesis empty fallback blocks. Pending proposer
  reward state, roots, and storage no longer carry a later useful-block unlock latch. A fresh local CPU
  Docker pass now proves delayed proposer claims, applied observed-diagnostic block-check evidence without
  punishing canonical proposer rewards, all-operator finalized-head convergence, and the role-owned useful
  proposal path; public/CUDA evidence remains outside that proof.
- Long-running node runtime now consumes `TENSORVM_CHAIN_PROFILE`, defaults local Compose to `local_cpu`,
  builds a typed `NodeConfig` at the CLI boundary, and exposes `chain_profile`/`role_chain_profile` in
  readiness, serve, and status output. Only the local CPU profile enables deterministic synthetic block
  production; public-testnet and mainnet profiles use the same chain engine with local synthetic jobs
  disabled. `NodeConfig` now carries typed network listen/auth/identity/max-request settings and storage
  paths for the runtime.
- Each long-running role command now writes live role-loop counters to the node data directory, and
  `tvmd node status` exposes `role_runtime_command`, `role_loop_ready`, `role_loop_role`,
  `role_chain_profile`, `role_can_produce_blocks`, `role_local_producer`, `role_produced_blocks`,
  `role_validator_proposer_work_ready`, `role_validator_useful_blocks_proposed`,
  `role_validator_fallback_blocks_proposed`, `role_validator_receipts_proposed`,
  `role_network_applied_blocks`, decoded `role_network_*_ingested` event counters,
  block/job/receipt/attestation/block-check-challenge payload apply counters,
  `role_network_invalid_events`,
  `role_latest_height`, `role_p2p_connected_peers`,
  `role_p2p_observed_jobs`, `role_p2p_observed_receipts`, `role_p2p_observed_attestations`,
  `role_p2p_observed_blocks`, `role_p2p_observed_block_votes`, `role_p2p_latest_observed_block_height`,
  `role_p2p_latest_observed_block_hash`, and
  `role_p2p_observed_block_hashes`, and block-payload gossip counters/hashes; the checker fails unless every counted operator reports a live role
  loop, validator operators report block-production capability, only `validator-00` reports timed
  produced-block progress, miners report no block-production capability, every non-producer reports
  network-applied block progress from decoded block payloads, every non-producer has ingested decoded
  block-payload/header/block-vote/job/receipt/attestation/block-check-challenge events with zero invalid network events, every non-producer has
  accepted decoded block, block-vote, job, receipt, attestation, and block-check-challenge payloads through the chain engine when such payloads are present, at least one real libp2p
  connection, job/receipt/attestation/block/block-vote announcements observed through Gossipsub, and an observed network
  announcement for the selected finalized p2p-observed head hash. Validator operators report block-production
  capability, but only `validator-00` reports `role_local_producer=true` and positive timed produced-block
  progress backed by positive useful proposal, proposed-receipt, artifact-ready, and attested-receipt
  counters; miners report no block-production capability. The checker also emits exact
  `live_role_miner_receipt_operators`, `live_role_miner_receipts_submitted`,
  `live_role_miner_tensors_inserted`, `live_role_validator_attestation_operators`, and
  `live_role_validator_attestations_submitted`, `live_role_validator_useful_blocks_proposed`, and
  `live_role_validator_proposed_receipts` evidence fields after convergence.
- The checker now requires `/explorer/receipts/latest/500` to name more than the seeded count of both
  `tensor_op` and `linear_training_step` receipts, so live post-startup primitive evidence is visible by
  receipt type instead of only by aggregate model-count growth.
- `tvmd node block` now exposes per-height receipt IDs, settled receipt IDs, and TensorOp versus
  LinearTrainingStep receipt counts, and the checker fails unless finalized live blocks expose both
  primitive types through that block view.
- Chain state now records data-unavailability miner bond slashes: an unavailable-data attestation marks the
  receipt non-finalizable, and canonical block application slashes the receipt miner once, credits treasury,
  commits the slash record in the state root, and persists/exposes slashing counts. Mandatory validator audit
  records now also exist in the local reference when audit sampling is configured: base receipt-reward
  maturity and tensor retention are extended to at least the validator-audit window, assignment names a
  deterministic registered auditor distinct from the audited validator, keeps the audited validator's
  pending receipt reward held through the audit deadline, and a missed or contradicted audit slashes that
  validator once, voids the delayed validator reward, holds the voided claim through the appeal deadline
  before pruning without credit, credits treasury, and persists/exposes audit counts.
  Registered validator roles now observe only their assigned local audit work, submit signed audit reports
  through the shared chain command path, gossip bounded audit-report payloads, let non-producers apply or
  retry those payloads through node ingest, and expose submitted plus network-applied audit-report
  counters. Slashed audited validators can now submit signed, bounded appeal records that are rooted and
  persisted with the audit slash state, and appeal resolution mutates the delayed validator reward claim
  plus the recorded stake slash: upheld outcomes keep the reward voided for normal pruning, while reversed
  reward-void outcomes reinstate the pending claim without immediate spendable credit and refund the slash
  from treasury back to validator stake. Chain state, service status, and explorer overview now
  expose live validator-audit economic calibration plus a broader implemented-path fraud calibration for
  validator-audit, miner data-unavailability, invalid-output, and block-check/proposer clawback paths, including required
  slashable bonds, aggregate worst-required-bond, and pass/fail invariant status. Proposer rewards now
  carry an extra proposer-specific maturity hold, and the fraud-path calibration treats held receipt and
  proposer claims as slashable/voidable escrow while counting fraud proceeds only after claimability.
  Late assigned invalid-output attestations now contest already settled receipts in chain validation:
  the receipt leaves the settled set, enters the challenged set, the miner's bonded stake is slashed into
  treasury through a state-rooted invalid-output slash record, and delayed miner plus validator receipt
  rewards are voided and held through an explicit challenge maturity height instead of requiring
  spendable-balance clawback. Data-unavailability and block-check challenges now use the same
  voided-but-delayed receipt-claim behavior, so affected claims remain state-rooted until the hold clears.
  Redundant settlement now counts distinct registered miner operator IDs for agreement quorum instead of
  miner addresses alone, and delay records persist both agreeing miner and agreeing operator counts before
  any delayed claim can later settle. The collusion-risk study now reports miner-address quorum and
  operator quorum separately, so redundant-agreement collusion evidence follows the same operator-distinct
  rule as settlement.
  Chain state, service status, and explorer overview now also expose structured detection-probability
  evidence for the implemented verifier and fraud mechanisms. Local CPU role runtimes now ingest the
  configured deterministic drand-style external beacon fixture through the shared chain command before
  network/role work, persist the accepted beacon record, relay it as a bounded p2p payload, apply
  network-originated beacon payloads through the same chain command, submit chain-verified local validator
  VRF reveal records before validator receipt rewards can become spendable, relay/retry bounded reveal
  payloads through p2p/node ingest, and expose checker-gated role counters plus external-beacon
  count/latest-round and validator-reveal count evidence. Chain admission now supports registered
  validator reveal public keys and requires bounded Ed25519 proof bytes for keyed validators before
  releasing validator receipt rewards; unkeyed reveal records remain local fallback evidence only. Chain
  and p2p/node paths now admit bounded `pedersen-bls-unchained` drand evidence through signature
  verification and typed proof metadata. Public
  drand mode now polls the default-chain v2 endpoint, verifies chained proofs, skips stale rounds, backs
  off after failures, computes endpoint expected-round and chain-epoch freshness evidence, rejects locally
  fetched rounds beyond the configured lag, and exposes status counters for
  attempts/successes/stale/failure backoff/freshness. Consensus-level public drand epoch mapping,
  deployed validator reveal key lifecycle/full VRF construction, and deployed lifecycle evidence remain
  open.
  Deployed-run measured detection records and remaining fraud paths remain open
  economics work.

That is enough for a useful local demonstration. It is not enough for a production-grade local chain.

## Refactor Progress

The first chain-core cleanup slices are already in the tree:

- The core state machine is exposed as profile-neutral `Chain`; the old `LocalChain` compatibility alias is
  gone, and `ChainEngine` remains the command/event facade.
- `NodeStore` implements a `ChainStore` boundary for loading and persisting chain state.
- `ChainProfile` and `NodeConfig` let local CPU, public testnet, and future mainnet construct the same
  deterministic chain engine from profile values.
- Local CPU synthetic production moved into the `tensor_vm` library instead of remaining private binary code.
- `JobSource` and `SyntheticLocalJobSource` separate deterministic local job generation from scheduler and
  block-production code.
- `CpuReferenceMinerRole`, `ReferenceValidatorRole`, and `RoleReceiptBundle` separate CPU miner execution,
  validator verification, served tensor artifacts, and receipt/attestation submission from local round
  orchestration.
- `NodeRuntimeState`, `NetworkEventIngest`, `PendingNetworkPayloads`, and `NetworkPayloadProcessor` now
  live behind a reusable node runtime boundary instead of being private `tvmd` binary state, so role-owned
  loops can share the same counters and out-of-order payload retry semantics.
- Decoded network job, receipt, and attestation payload application now lives behind chain-centric node
  runtime helpers, so future role loops can apply accepted payloads through `ChainCommand` without depending
  on private `tvmd` helpers.
- Network event ordering, invalid event accounting, decoded payload ingestion, pending-payload retry, and
  block-payload admission now live in the reusable node runtime driver. `NewBlockHeader` remains an
  announcement/locator; non-producers apply block progress from decoded `TensorBlock` payloads through the
  shared chain engine.
- Block-vote payload admission now also lives in the reusable node runtime driver. Validators submit
  explicit role-owned block votes for locally valid unfinalized blocks, non-producers ingest and apply
  decoded block-vote payloads through `ChainCommand::SubmitBlockVote`, and vote-only finality progress is
  persisted to the node store.
- Role runtimes now bind their configured wallet to a deterministic chain address and report whether that
  address is registered as a miner or validator in the loaded chain state. Local CPU Compose uses seeded
  wallet labels for counted miner and validator operators, and the checker requires those registrations
  before accepting operator readiness.
- `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run` now construct explicit role-run loop
  wrappers before entering the shared runtime. The runtime loop has named steps for status writes, RPC
  serving, network ingestion, role-owned miner receipt submission, role-owned validator attestation
  submission, and optional validator proposer work, preserving current consensus behavior. The local CPU
  Docker checker now covers the single configured validator proposer path; public/CUDA and multi-proposer
  deployment evidence remain open.

These are foundation pieces, not completion. Miner receipts, validator attestations, validator block votes,
and configured validator proposer ticks now have role-owned submission paths for locally available work, and
validators can fetch missing tensors remotely. The chain core now has a current-head competition policy
that replaces an unfinalized useful head only with a same-parent useful head carrying a strictly better
PoW hash while keeping finalized and fallback heads stable. Valid known-parent non-canonical blocks are
now retained in chain-owned side-branch fork storage with parent and child state snapshots, strictly longer
unfinalized branches automatically reorganize canonical head state, and branch maps persist through
chain-state snapshots. The local runtime now has a fresh local CPU Docker proof of multi-validator
proposer competition, chain-visible proposer cooldown state, delayed proposer rewards, applied diagnostic
block-check evidence, and all-operator convergence. Remaining proof work is public/CUDA deployment evidence and full interactive
transcript disputes.

## Highest-Priority Gaps

### 1. Local Production Still Needs Public Evidence

Current live production now runs inside validator runtimes. Deterministic local job publication remains a
single `validator-00` synthetic-producer duty, but receipt execution, attestation, settlement preparation,
block proposal, and finality voting run through role-owned ticks and shared chain commands. Finality votes
come from explicit validator role block-vote submissions. The latest local CPU Docker gate proves five
validator block proposers, chain-visible proposer cadence state, delayed proposer rewards, observed
diagnostic block-check evidence, and all-operator finalized-head convergence. Public deployment evidence,
CUDA evidence, and public
drand/VRF randomness verification remain open.
The chain core requires registered-validator useful-verification PoW blocks, and block append/finality are
separate chain commands.

The local CPU proof no longer depends on a single service mutating counted roles in memory, proves local
multi-validator proposal competition, and proves network-applied external randomness beacon payloads, but
it still does not prove public multi-operator deployment behavior or CUDA-backed miners.

Required fix:

- Extend proposer, beacon, receipt, attestation, and reward evidence into public/CUDA deployment runs.
- Replace deterministic local beacon fixtures in public evidence with deployed public-drand round mapping,
  deployed validator reveal key lifecycle/full VRF construction, or the deployed commit-reveal lifecycle
  selected for the public profile.
- Continue tightening protocol-level proposer eligibility with deployed evidence rather than runtime-only
  scheduling gates.

### 2. Miner And Validator Containers Still Delegate Internals To The Service Runtime

`tvmd miner check` and `tvmd validator check` prove local readiness. Containers now exec the matching
long-running `tvmd miner run`, `tvmd validator run`, or `tvmd proposer run` surface. Those role commands
still delegate their inner serving path to the shared service runtime, so they prove the command surface
and Compose contract but not independent role ownership yet.

Required fix:

- Keep `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run` as counted operator entrypoints.
- Move miner, validator, and proposer internals out of the generic service loop so each role loop owns
  only its role responsibilities.
- Keep readiness commands as preflight checks, not the runtime.

### 3. Libp2p Runs But Does Not Drive Chain State

The libp2p control plane subscribes to TensorVM topics and supports request-response protocols, but
production state changes still happen through local memory in the gateway process.

Required fix:

- Implement a node event loop that ingests libp2p messages:
  - `NewJob`
  - `NewReceipt`
  - `NewAttestation`
  - `NewBlock`
  - `NewBlockPayload`
  - `NewBlockHeader`
  - `PeerInfo`
- Validate message payloads before applying them.
- Persist accepted events through the shared chain engine.
- Publish local events back out through libp2p.

### 4. Non-Bootstrap Operators Do Not Prove Chain Sync

The checker validates that all operators are running and libp2p-ready, and now checks every node store for
role status, live chain counters, the same first live finalized block hash, the same finalized common-head
block hash, non-producer network-applied block counters, decoded job/receipt/attestation payload
application counters, and a finalized local-head checkpoint/state root that has also been observed through
p2p block-payload gossip via `tvmd node block`. It still does not prove every block is assembled from
network-derived role-owned miner and validator work instead of service-owned timed production, or that every
operator is executing a distinct fully independent production loop.

Required fix:

- Extend `tvmd node status` or the local node API to include real connected peer count and role-specific
  work counters sourced from role loops.
- Move the convergence assertion from deterministic same-seed first-live/common-head equality to the
  shared network event path.
- The checker must eventually fail unless all 15 operators converge on the same network-derived latest
  finalized head within a bounded time.

Status: started for role-loop and network counters. `tvmd node status` now exposes role-runtime
command, role-loop readiness, role, local-producer mode, produced-block, network-applied block,
decoded network-event ingestion counters, decoded job/receipt/attestation payload application counters,
role wallet address and registration status, miner-assigned work readiness counters, miner receipt
submission/tensor-insertion counters, latest-height, real libp2p connected-peer counters, and
runtime-observed job, receipt, attestation, and block gossip counters from the long-running command. Local
block production now publishes typed
`NewJobPayload`, `NewReceiptPayload`, `NewAttestationPayload`, and `NewBlockPayload` messages, legacy
`NewJob`, `NewReceipt`, `NewAttestation`, and `NewBlock` hash announcements, and height-bearing
`NewBlockHeader` announcements over Gossipsub. The libp2p worker queues decoded inbound messages for the runtime loop;
non-producers validate and apply job payloads through `ChainCommand::SubmitJob`, receipt payloads through
`ChainCommand::SubmitReceipt`, attestation payloads through `ChainCommand::SubmitAttestation`, and block
payloads through `ChainCommand::SubmitBlock`. Pending block, receipt, and attestation payloads are retained
and retried once prerequisite parents, jobs, receipts, or attestations arrive. Only `validator-00` is allowed to drive timed local block production, while the chain block
itself must be proposed by a registered validator and pass useful-verification PoW checks. The role loop processes block
payload dependencies before block payloads through the reusable node runtime event driver, which also owns
decoded payload application, pending retry, invalid event accounting, and producer versus non-producer
block-payload dispatch. The remaining gap is replacing service-owned timed production with role-owned
miner, validator, and validator proposer loops that assemble blocks from network-visible state.

### 5. Restart Gate Now Has A Rolling Matrix

The local spec requires restarted operators to reuse durable state and libp2p identity, rejoin the network,
and avoid chain rollback. The current restart-continuity script records pre-restart and post-restart
continuity for selected restarts, and `check-rolling-restart-continuity.sh` now runs that gate across every
counted operator by default.

Current assertion:

- The rolling gate fails unless every requested miner or validator keeps its peer ID, avoids height,
  block-count, state-root, and block-log-root regression, preserves the pre-restart finalized common head
  and state root on every operator, preserves the sampled tensor artifact, and reconverges on a finalized
  common head. It reports post-restart advancement separately, so a stable common-head plateau is visible
  without weakening the restart-preservation gate. During this restart-only check, a just-restarted service
  may prove plateau readiness with preserved post-seed chain state plus live peer connectivity instead of
  newly incremented volatile gossip counters, and process-lifetime role/network totals are not re-proved
  after every service has restarted; normal local readiness still requires the fresh counters and totals.
- Focused Rust tests cover block-log replacement, node-store recovery from `chain.state`, and service-init
  recovery for torn snapshot/block-log state.
- The full local spec uses the default all-operator rolling matrix. Passing a smaller service list is only a
  smoke check.

### 6. Live Primitive Coverage Needs Stronger Evidence

The seed covers both TensorOp and LinearTrainingStep. Live post-startup production now uses
`SyntheticLocalJobSource` for both matmul and LinearTrainingStep jobs, and the checker requires
`model_count` to advance past the seeded baseline plus receipt details to name more than the seeded count
of both primitive types. The service block view now reports per-height receipt IDs and primitive counts, and
the local checker requires finalized live TensorOp and LinearTrainingStep block evidence near the current
head.

Required fix:

- Keep the deterministic local `JobSource` emitting both:
  - TensorOp matmul jobs
  - LinearTrainingStep jobs
- Extend this from per-receipt primitive evidence to per-block primitive evidence once block views expose
  included receipt IDs by block.

Status: complete for the current local block view. Receipt ownership is still not role-owned end to end,
but block-height receipt evidence is now queryable and gated.

### 7. The Checker Does Not Prove All Local-Spec Acceptance Items

The local spec requires validator attestations, rewards, data availability, telemetry, and tensor-server
availability evidence. The checker currently verifies some seed strings and aggregate live counters.

Required fix:

- Query live receipt details and prove at least one new post-startup receipt has validator attestations.
- Query pending miner and validator receipt reward claims after live jobs, and separately verify they remain
  pending until a beneficiary `ClaimReward` after the reward-settlement delay plus challenge window.
- Query pending block-check challenger reward claims after a successful live challenge scenario, and
  separately verify mature claimability through `ClaimReward` instead of immediate bounty credit. The shared
  p2p/node payload path, status counters, chain-owned full reward-maturity delay for challenge bounties,
  deterministic local diagnostic bad-block challenge generation, validator-proposer diagnostic emission,
  observed malformed-block side-cache ingestion, and hard checker gate for future-maturity challenge reward
  claims now exist. The remaining gap is a fresh Docker proof of the full scenario after the `/health`
  blocker clears.
- Perform a live tensor row/chunk/opening fetch through the local tensor-server path.
- Assert telemetry counters advance with the live chain.
- Record exact observed values in checker output.

## Shared Chain-Core Refactor

The core architectural goal is:

```text
local, testnet, and mainnet use the same deterministic chain engine
```

The profiles should differ by configuration, adapters, and launch topology, not by separate chain logic.

### Current Coupling To Reduce

`Chain` still owns state, parameters, registration, transaction application, receipt submission,
attestation validation, and finality helpers in one type. Settlement, proposer selection, deterministic
commitment roots, and block assembly have been split into internal `chain::settlement`, `chain::proposer`,
`chain::roots`, and `chain::blocks` modules, with the public `Chain`/`ChainEngine` API preserved.

That is practical for a reference core, but it makes it easy for local/testnet helpers to bypass real
runtime boundaries.

### Target Module Shape

Refactor toward these boundaries:

```text
chain::state
  ChainState, ChainParams, account/miner/validator/job/receipt/block state types

chain::engine
  ChainEngine, deterministic state transitions, command application, event emission

chain::validation
  receipt, attestation, block-vote, and transaction validation

chain::settlement
  epoch settlement, reward accounting, model-state transition settlement

chain::proposer
  v1 proposer compatibility path; v2 useful-verification PoW block validation should replace this surface

node::runtime
  event loop joining network, store, txpool, chain engine, clock, and role services

node::roles
  miner, validator, proposer, watcher

node::profiles
  local, testnet, mainnet runtime profiles

network
  libp2p adapter, message codec, gossip/request-response routing

storage
  ChainStore trait, NodeStore implementation, recovery and consistency checks
```

### Shared Profile Model

Use a single profile type instead of environment-specific branches:

```rust
pub enum ChainProfile {
    Local(LocalProfile),
    Testnet(TestnetProfile),
    Mainnet(MainnetProfile),
}

pub struct NodeConfig {
    pub chain: ChainParams,
    pub profile: ChainProfile,
    pub role: NodeRole,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
}
```

Local/testnet/mainnet should select different values for:

- genesis state
- chain ID
- job source policy
- block interval
- peer discovery/bootstrap
- auth/exposure policy
- reward caps
- persistence paths
- telemetry/evidence requirements

They should not select different state-transition code.

### Engine API Direction

The chain engine should expose a small command/event boundary:

```rust
pub enum ChainCommand {
    RegisterMiner(...),
    RegisterValidator(...),
    SubmitJob(JobState),
    SubmitReceipt(ReceiptState),
    SubmitAttestation(ValidatorAttestation),
    SubmitBlock(TensorBlock),
    SubmitBlockVote(BlockVote),
    SettleEpoch,
}

pub enum ChainEvent {
    JobAccepted(Hash),
    ReceiptAccepted(Hash),
    AttestationAccepted(Hash),
    ReceiptSettled(Hash),
    BlockAccepted(Hash),
    BlockFinalized(Hash),
    RewardCredited { address: Address, amount: u64 },
}

pub trait ChainEngine {
    fn apply(&mut self, command: ChainCommand) -> Result<Vec<ChainEvent>>;
    fn view(&self) -> &ChainState;
}
```

This makes tests, local Compose, public testnet, and future mainnet run the same transition path while
still allowing different runtimes to drive it.

### Traits To Introduce

Keep traits narrow and role-specific:

```rust
pub trait ChainStore {
    fn load_chain(&self) -> Result<ChainSnapshot>;
    fn persist_events(&self, events: &[ChainEvent]) -> Result<()>;
    fn persist_snapshot(&self, state: &ChainState) -> Result<()>;
}

pub trait Network {
    fn publish(&self, message: P2pMessage) -> Result<()>;
    fn recv(&mut self) -> Result<NetworkEvent>;
    fn request(&self, peer: PeerId, request: P2pMessage) -> Result<P2pMessage>;
}

pub trait JobSource {
    fn next_job(&mut self, state: &ChainState) -> Option<JobState>;
}

pub trait MinerExecutor {
    fn execute(&mut self, job: &JobState, context: &ExecutionContext) -> Result<ReceiptBundle>;
}

pub trait ReceiptVerifier {
    fn verify(&self, receipt: &ReceiptState, context: &ValidationContext) -> Result<ValidatorAttestation>;
}
```

Concrete implementations:

- `NodeStore` implements `ChainStore`.
- `Libp2pNetwork` implements `Network`.
- `SyntheticLocalJobSource` implements `JobSource`.
- `CpuReferenceMiner` implements `MinerExecutor`.
- `TensorVmReceiptVerifier` implements `ReceiptVerifier`.

### SOLID/Rust Guidelines

Use SOLID as a practical constraint, not as ceremony:

- Single responsibility: chain transition logic should not know Docker, HTTP, CLI, or libp2p details.
- Open/closed: adding `MainnetProfile` should not require editing settlement or validation internals.
- Liskov substitution: tests should run against the same `ChainEngine` trait as local Compose.
- Interface segregation: miners should not depend on proposer APIs; validators should not depend on faucet APIs.
- Dependency inversion: `node::runtime` depends on `Network` and `ChainStore` traits, not concrete libp2p or file-store types.

Rust-specific practices:

- Prefer explicit domain types over `String`/`usize` plumbing at module boundaries.
- Keep `Result<T, TvmError>` for fallible domain paths and avoid stringly errors in core logic.
- Make command application deterministic and side-effect-free except through returned events.
- Keep IO at adapter edges: storage, network, CLI, RPC.
- Avoid large `impl` blocks that mix registration, execution, settlement, and API concerns.
- Prefer small structs with explicit ownership over shared mutable globals.
- Use `#[cfg(test)]` helpers only for tests; do not let production code call testnet-only shortcuts.

## Role Runtime Design

### Miner Loop

Responsibilities:

```text
subscribe to jobs
check assignment
execute with CPU reference backend
serve tensor rows/chunks/openings
submit receipts
gossip receipt announcements
track local work metrics
```

### Validator Loop

Responsibilities:

```text
subscribe to jobs and receipts
check validation assignment
request tensor data from assigned miner
verify TensorOp and LinearTrainingStep receipts
submit attestations
gossip attestation announcements
vote on valid blocks
track validation metrics
```

### Proposer Loop

Responsibilities:

```text
watch the canonical settled-receipt blockspace
verify the selected receipt set and derive checks_root
search or validate useful-verification PoW over the v2 block header
assemble blocks from accepted state and canonical blockspace
publish blocks
collect block votes
track finality metrics
```

In local mode, the single timed local producer is a validator runtime; it must still consume network-visible
jobs, receipts, attestations, and votes.

## Proposed Implementation Phases

### Phase 1: Document And Harden The Gate

- Add this document.
- Update the local checker to emit exact live counters.
- Update `coverage_matrix.md` so it describes live post-startup jobs, not only seeded state.
- Add checker assertions for live pending reward claims, mature reward release, live attestations, live tensor data fetch, and all-operator
  finalized-head convergence.

Status: partially complete. The document exists and the checker gates live post-startup height, blocks,
jobs, model-count advancement, attestation-count growth, pending receipt-reward growth, receipts, and settled
receipts, per-receipt validator-attestation details, live tensor descriptor/row/chunk/opening fetches, all
15 operator node stores reporting role status, live chain counters, finalized live TensorOp and
  LinearTrainingStep block-view evidence, the single local synthetic job producer, chain-cadence
  multi-validator proposer participation, network
applied block progress on every non-producer, accepted job, receipt, attestation, and live diagnostic block-check-challenge payload application
through the shared chain engine on every non-producer, validator-owned block-vote submission,
non-producer block-vote ingestion/application, pending receipt/attestation/block-vote/block-check-challenge retry for out-of-order
p2p payloads, the same first live finalized block hash, the same finalized common-head block hash, and a
finalized local-head checkpoint/state root that was also observed through p2p block gossip via
`tvmd node block`, plus named post-seed TensorOp and LinearTrainingStep receipt evidence, real libp2p
connected-peer counts, job/receipt/attestation/block/block-vote gossip observations from every role runtime,
positive validator useful block proposal and selected-receipt counters from at least two proposer-enabled
validators, positive pending proposer reward count for useful proposals, positive delayed proposer reward
claims, applied diagnostic block-check challenge evidence that does not punish the canonical proposer
reward path, and nonempty block-log roots from every node store. The
restart-continuity script also captures
pre/post peer IDs, heights, block counts, state roots, block-log roots, and finalized common heads for
selected restart gates, and the rolling wrapper applies that gate to every counted operator by default.
Fully assembling blocks from shared network-derived state and role-owned miner/validator/proposer loops
still needs hard checker assertions.

### Phase 2: Extract Chain Engine Boundaries

- Keep the profile-neutral `Chain` core type and continue moving write access behind `ChainEngine`.
- Move validation, settlement, proposer selection, and state views into separate modules.
- Preserve all existing behavior and tests.
- Do not reintroduce a `LocalChain` compatibility alias.

Status: complete for the core rename and current production chain-core split. `Chain`, `ChainEngine`,
`ChainCommand`, and `ChainEvent` exist. Proposer selection now lives behind `chain::proposer`,
epoch settlement/redundant-agreement logic now lives behind `chain::settlement`, deterministic
content/state roots now live behind `chain::roots`, block assembly now lives behind `chain::blocks`, and
chain parameters/state/domain view types now live behind `chain::state` while preserving the
profile-neutral chain API. Attestation, validation-seed, quorum, and block-finality checks now live behind
`chain::validation`, and account creation/transfer/reward-claim logic now lives behind `chain::accounts`.
Genesis construction now lives behind `chain::genesis`. Miner/validator registration and hardware-profile
checks now live behind `chain::operators`, job/receipt admission now lives behind `chain::receipts`, and
model registration plus transition checks now live behind `chain::models`. Challenge outcome and slashing
mutation now lives behind `chain::challenges`, profile-neutral command/event facade types now live behind
`chain::engine`, `ChainEngine` command routing now lives behind `chain::commands`, and transaction
application now lives behind `chain::transactions`. `chain.rs` is now a profile-neutral facade over the
smaller chain modules and the existing test module.

### Phase 3: Add Role Loops Without Changing Consensus Semantics

- Add long-running miner, validator, and proposer/node commands.
- Initially run them against the existing RPC endpoints.
- Then move gossip/request-response ingestion into the node runtime.

Status: started. `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run` are long-running
role-specific command surfaces. Compose uses `tvmd miner run` for all counted miners and
`tvmd validator run` for all validators, with `validator-00` carrying the single local timed synthetic job
producer flag and all five validators carrying validator block-proposer flags under the shared chain-visible
proposer cooldown;
the local checker verifies those runtime commands through ready files and `tvmd node status`. The status path also
exposes live role-loop counters, local-producer mode, network-applied block counters, real libp2p
connected-peer counts, job/receipt/attestation/block/block-payload/block-vote gossip observations, and target-head block-payload gossip
observations for every counted operator. The service runtime now keeps served-request counts,
produced-block counts, network-applied block counts, aggregate network-event counters, pending
out-of-order network payloads, and decoded block/job/receipt/attestation/block-vote payload application in reusable
node runtime helpers instead of private binary state. Message ordering, invalid network-event accounting,
pending retry integration, and block-payload application dispatch now also go through the shared node runtime
event driver. The role
commands now enter explicit role-run loop wrappers and a named runtime loop boundary instead of constructing
the generic service loop inline. CPU miner execution and validator verification now live behind role-owned
library components, miner receipt submission and validator attestation submission have role-loop paths for
locally available work, validators can fetch missing receipt tensors over the libp2p request-response path
before submitting attestations, validators submit explicit block votes for locally valid unfinalized blocks,
and the local validator producer assembles useful proposals from the validator-owned role tick after seeing
settled, artifact-ready, attested receipts. Runtime role policy now prevents service, miner, and legacy
proposer roles from becoming local block producers even if they inherit local block-interval configuration;
validators require the explicit block-proposer flag for proposal, while the interval and local synthetic job
source only control deterministic local job publication.

### Phase 4: Make Compose Participants Actually Participate

- `miner-*` containers run miner role loops.
- `validator-*` containers run validator role loops.
- `validator-00` runs the single local synthetic job producer duty: the scheduler publishes jobs, while
  miner, validator, and validator-proposer role ticks handle receipts, attestations, settlement, and useful
  block proposals.
- All five validators run validator block-proposer duties under chain-visible cooldown; the local checker
  counts proposer-capable validators in the finalized-head convergence proof.
- The checker requires all operators to converge on the same finalized head.

### Phase 5: Shared Profiles

- Introduce `NodeConfig` and `ChainProfile`.
- Express local, testnet, and future mainnet as config profiles.
- Remove profile-specific chain transition branches.
- Ensure all profile tests instantiate the same engine.

Status: partially complete. `ChainProfile`, `NodeConfig`, `NetworkConfig`, and `StorageConfig` exist and
tests prove all profiles build the same engine.
`ChainProfile` now also owns optional synthetic-job scheduling: the local CPU profile enables the
deterministic matmul/LinearTrainingStep source, while public testnet and mainnet profiles disable local-only
synthetic production. The long-running node runtime now reads `TENSORVM_CHAIN_PROFILE`, reports the active
profile in serve/status surfaces, and gates synthetic production through `NodeConfig` role policy, block
interval, local-producer settings, network listen/auth/identity/max-request settings, and storage path.
Bootstrap peer loading still comes from the persisted peer book; profile-specific public exposure policy
still needs to be wired through runtime adapters rather than documented profile fields only.

### Phase 6: Restart And Recovery

- Restart miner, validator, and proposer/gateway roles independently.
- Verify no rollback.
- Verify catch-up from persisted block log and peer state.
- Verify restart non-regression and report whether block production continues after restart.

Status: complete for the current local-store model. `check-restart-continuity.sh` proves stable libp2p peer
IDs, non-regressing height/block count/state-root evidence, preservation of the pre-restart finalized common
head and state root on every operator, nonzero block-log roots, sampled tensor artifact preservation, and
common-head reconvergence for each requested service. It reports whether post-restart blocks advanced
without failing a stable plateau. `check-rolling-restart-continuity.sh` now applies that gate one operator
at a time across the full 15-service matrix by default, and service init repairs torn snapshot/block-log
state from `chain.state`
before a restarted operator can report readiness.

## Local Production-Ready Acceptance Gate

The local chain should not be called production-ready until this command sequence passes:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

And the checker must prove:

```text
all 15 counted operators are running real role loops
all 15 operators have stable identities after restart
all 15 operators converge on the same finalized head
restart checks preserve the pre-restart finalized head/state and report whether new blocks appeared
jobs are delivered through libp2p or the shared node event path
receipts are produced by miner containers
attestations are produced by validator containers
blocks are produced from network-visible receipts and attestations
block finality votes are produced by validator containers and gossiped/applied by non-producers
TensorOp and LinearTrainingStep live jobs both settle after startup
tensor rows/chunks/openings are fetched through the local tensor-server path
live pending reward claims accrue to miners, validators, and observed successful block-check challengers,
and matured claims become spendable only through beneficiary `ClaimReward`
telemetry reflects live post-startup work
local evidence remains explicitly non-public
```

## Recommended Next Commit Sequence

Keep this incremental:

1. Broaden chain-cadence multi-proposer, delayed proposer reward evidence, and diagnostic block-check evidence from the local CPU proof into public/CUDA deployment
   runs.
2. Replace deterministic local beacon fixtures in public deployment evidence with round-mapped public
   drand polling or deployed validator reveal key lifecycle/full VRF evidence.
3. Continue full interactive transcript dispute work over the trace-opening path.

This sequence keeps the local chain usable at every step while moving it toward the same base runtime that
testnet and mainnet profiles should use.
