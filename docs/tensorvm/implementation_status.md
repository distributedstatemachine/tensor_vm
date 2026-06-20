# TensorVM Implementation Status

This tracks the implementation of [`mvp_spec.md`](mvp_spec.md). The
acceptance-criterion test map is in [`coverage_matrix.md`](coverage_matrix.md).

The reviewed MVP consensus model is partially implemented locally. The reference core now uses
validator-owned block production with useful-verification PoW fields over deterministic settled-receipt
blockspace, selected receipts are marked included once, and block votes validate the known block with strict
parent-root checks before counting stake. Long-running validator roles can now submit and gossip explicit
block votes for unfinalized valid blocks, so block append is separated from finality in the runtime path.
Remaining consensus gaps are full verifier-transcript challenge semantics, exact parent-state snapshots and
child-state apply semantics, difficulty retargeting, zero-receipt skip fallback economics, deterministic
live invalid-block challenge generation, multi-validator proposer competition/fork-choice policy, and a
fresh full Docker proof of live validator proposer/block-assembly networking after the current `/health`
blocker clears. See
[`mvp_core_formal_proofs.md`](../formal/mvp_core_formal_proofs.md).

## Implemented In `crates/tensor_vm`

- Deterministic finite-field tensors and TensorVM operations
- TensorVM field arithmetic, SHA-256 hashing, oracle RNG primitives, and standalone consensus logic;
  `tensor_vm` does not depend on `experiments`
- Bounds-checked tensor row/cell access and invalid-index rejection
- Full direct TensorVM wrapper and program-hash variant coverage
- Content-addressed Tensor IR foundation with typed tensor specs, params, refs, ops, graph outputs,
  canonical JSON encoding, `graph_id = SHA256(canonical_json(graph))`, frozen v0 op-registry metadata,
  structural validation, Tier-C vocabulary that is carried but rejected for consensus admission, and
  canonical TensorOp/LinearTrainingStep graph constructors. Each frozen op now declares its verifier
  class: full Freivalds, random-linear, exact deterministic replay, canonical-reference-required, or
  index-consistency-required. `gather`, `scatter`, and `embedding` are present only as non-admitted
  index-consistency-gated vocabulary. Current TensorOp and LinearTrainingStep receipts bind their
  `program_hash` to the validated IR `graph_id`, and fixed canonical receipt production now derives
  TensorOp and LinearTrainingStep `trace_root` values from exact execution of those canonical graph op
  traces. Current job admission stores the canonical graph body bytes in chain state keyed by graph ID.
  Arbitrary user-submitted canonical graph bodies can also be
  registered directly through `ChainCommand::RegisterProgramBody`, which parses the JSON IR, validates it
  for consensus admission, checks that the submitted graph ID matches the validated graph, rejects
  noncanonical byte encodings, and treats matching duplicate submissions as idempotent. Registered graph
  bodies are committed in the state root, persisted through the chain-state snapshot, hydrated into the
  runtime program server at startup, and served by the existing libp2p
  `RequestProgram`/`ProgramResponse` path. `TensorGraph::execute_exact` now provides a deterministic
  interpreter foundation for validated, consensus-admitted graphs over the currently implemented exact
  tensor ops: `matmul`, broadcast-aware `add`/`sub`/`mul`, `scalar_mul`, `transpose`, explicit-dim
  `sum`/`reduce_sum`, `identity`, `neg`, signed-residue `abs`/`sign`/`relu`, fixed-point scale-aware
  half-even `round`, `reshape`, `broadcast`, comparisons `gt`/`lt`/`ge`/`le`/`eq`, `where`, `mean`,
  scale-aware `cast`, `concat`, `stack`, `full`, and `arange`. Runtime `Tensor` values now carry
  consensus-visible `scale` metadata, tensor descriptors and commitment roots bind that scale, and
  graph execution rejects bound tensors whose dtype/scale does not match `TensorSpec`. The value model now
  also carries consensus-visible `int8`, `uint8`, and `bool` dtype tags through tensor commitments,
  shared codecs, p2p tensor payloads, and canonical IR JSON. Tensor construction and deterministic random
  tensor generation enforce canonical int8, uint8, and bool ranges. The frozen registry admits exact
  deterministic `quantize_int8_per_channel` and `dequantize_int8_per_channel`: quantize takes `Fixed32`
  input plus a `dim` kwarg, returns an `Int8` tensor and rank-1 `Fixed32` scale tensor, selects
  `scale_raw = max(1, ceil(max_abs_raw / 127))` per channel, round-half-even divides by scale, and clamps
  to `[-128, 127]`; dequantize multiplies canonical int8 values by the supplied scale, infers a unique
  channel dimension from the scale length, broadcasts length-1 scales, and rejects ambiguous scale matches.
  The registry also admits `quantize_pack_int8` and `unpack_dequantize_int8` using a canonical flat
  `uint8` payload layout: `TVQ8` magic/version, rank, quantization axis, fixed-point output scale, original
  shape, per-channel signed 64-bit raw scales, and row-major signed int8 payload bytes. The interpreter
  validates bound tensors
  and field-scalar params, resolves input/op/param/const refs, returns named output tensors, records per-op
  output commitment roots, and derives a Merkle `trace_root`; Tier-C/deferred ops and admitted registry ops
  that do not yet have exact replay implementation fail closed. Registered canonical graph bodies can now
  be referenced by first-class `GraphExecution` jobs and receipts: command admission checks the registered
  graph body, input roots, params, job id, receipt digest, miner signature, and deadline; shared codec,
  p2p wrappers, state roots, storage snapshots, RPC/explorer rendering, telemetry, role verification, and
  settlement all carry the graph variant. Graph receipts replay through `TensorGraph::execute_exact` and
  settle through the same delayed pending receipt reward path after valid attestations. Role-runtime
  production for arbitrary graph jobs outside explicit graph artifacts, const-blob fetching, fixed-point
  arithmetic scale policy beyond `cast`/`round`, low-level packed tensor storage/chunking APIs, and CUDA
  generic graph execution remain open.
- Deterministic `F_p` conformance vectors for the current executable admitted op surface used by TensorOp
  and LinearTrainingStep plus field-only unary/shaping/generator coverage (`add`, `sub`, `mul`,
  `scalar_mul`, `identity`, `neg`, `abs`, `sign`, `round`, `relu`, `transpose`, `reshape`, `broadcast`,
  `sum`, `reduce_sum`, `mean`, `cast`, `concat`, `stack`, `matmul`, `full`, `arange`,
  `quantize_int8_per_channel`, `dequantize_int8_per_channel`, `quantize_pack_int8`,
  `unpack_dequantize_int8`, comparison masks (`gt`, `lt`, `ge`, `le`, `eq`), `where`, and `mse_loss`),
  including per-input and expected output dtype/scale metadata for fixed-point rescale vectors,
  multi-output expected tensors for exact quantize scale output, field-order comparison and selection
  vectors, and byte-exact packed payload vectors, with a stable suite hash, CPU reference backend pass reporting,
  default-build CUDA non-admission, a registry-derived guard that requires every consensus-admitted frozen
  op spelling to have vector and CPU profile evidence, and receipt verification gates that reject
  otherwise-valid TensorOp, LinearTrainingStep, or GraphExecution receipts when the required conformance
  profile is unavailable or missing an admitted op.
- Tensor descriptors, Merkle commitments, chunk openings, and row access
- Synthetic matmul jobs, TensorOp receipts, and trace commitments
- Full-output Freivalds verification and row-sampled audit checks
- Row-sampling sparse-corruption probability calculator
- Milestone -1 study utilities for the current UVPoW threat model, Freivalds false-accept bounds,
  randomness grindability, data withholding, collusion thresholds, TensorWork concentration, verification
  cost, the strict expected-slash-cost versus reward-from-fraud economic invariant, and zero-work liveness
  fallback
- LinearTrainingStep execution and verification
- Random-linear checks for `dY = Y - T` and `W_next = W - lr * grad_W`, backed by registry-level
  verifier classification that distinguishes random-linear relations from exact replay and deferred
  index-consistency ops
- Sparse-corruption rejection tests for TensorOp outputs, `dY`, and `W_next`
- Receipt digest/signature checks and trace-root recomputation
- Validator attestations with registered-stake quorum enforcement and deterministic assigned-validator
  admission checks
- Block assembly through the internal `chain::blocks` boundary with registered-validator proposer
  eligibility, deterministic settled-receipt blockspace selection, block-level `checks_root`, beacon,
  difficulty target, nonce search, useful-verification PoW checking, stake-weighted block-finality votes,
  duplicate-vote rejection, finalized block tracking, and finality-rate telemetry
- Duplicate registration, duplicate receipt, and duplicate validator-attestation rejection
- Account, miner, validator, job, receipt, attestation, reward, and model-state registries
- Miner hardware-class profiles with bounded reported GPU utilization for telemetry
- Content roots for jobs, receipts, attestations, rewards, and full chain state through the internal
  `chain::roots` boundary
- Receipt settlement in the internal `chain::settlement` boundary, 70/20/5/5 reward allocation,
  delayed miner and validator receipt rewards through consensus-visible pending receipt reward claims,
  delayed proposer rewards through a pending reward ledger, delayed block-check challenger rewards through
  pending challenge reward claims, delayed generic/faucet credits through a state-rooted pending credit
  reward ledger, a block `reward_root` that binds spendable rewards plus pending proposer, receipt,
  challenge, and credit ledgers, treasury rewards, reward accounting without repeated payout, and no-quorum rejection.
  Validator-owned useful proposals and empty fallback proposals now both create proposer claims with the
  explicit full reward-maturity delay, with fallback claims carrying the reduced reward amount. Pending
  proposer reward state, roots, and storage no longer carry a later-useful-block release latch.
  Receipt, challenge, and generic credit reward claims are state-rooted, persisted, and
  released only after maturity. Normal block transitions first apply the current block's receipt-inclusion
  delays and slash/audit voiding, then sweep still-matured reward claims into spendable balances through the
  shared chain transition instead of requiring adapter-side release workarounds. Voided proposer, receipt,
  and challenge claims are pruned without credit. Receipt claims are voided/pruned if a block-check
  challenge succeeds before release, and blocks with the old spendable-only reward root are rejected.
- MVP v0 penalty handling for data-unavailable receipts and mismatched attestations
- Registered-validator proposer selection through the internal `chain::proposer` boundary. Miner
  TensorWork no longer selects block proposers; TensorWork remains reward, telemetry, and blockspace input.
- Chain parameters, chain state, block/vote, job/receipt, account, miner, validator, reward, model, and
  transaction domain types through the internal `chain::state` boundary
- Genesis chain construction through the internal `chain::genesis` boundary
- Account creation, balance crediting, transfers, and reward claims through the internal
  `chain::accounts` boundary
- Miner/validator registration, stake floors, duplicate rejection, and miner hardware-profile checks through
  the internal `chain::operators` boundary
- Job submission, job lookup, and tensor/linear receipt admission through the internal `chain::receipts`
  boundary
- Model registration and linear-training model-state transitions through the internal `chain::models`
  boundary
- Challenge outcome application, miner/validator slashing, local block `checks_root` challenge admission,
  pending proposer reward invalidation, delayed pending challenger reward creation, challenged-receipt
  quarantine, and proposer throttle windows through the internal `chain::challenges` boundary
- Bounded network-visible block-check challenge payloads over the shared p2p/node event path, with
  challenge-id consistency checks, Merkle-proof sibling bounds before allocation, pending retry while the
  challenged block is missing, canonical application through `ChainCommand::SubmitBlockCheckChallenge`,
  persistence on challenge mutation, and runtime/checker status counters for ingested/applied challenges
- Profile-neutral `ChainCommand`, `ChainEvent`, and `ChainEngine` facade types through the internal
  `chain::engine` boundary
- `ChainEngine` command dispatch, event emission, and view accessors through the internal
  `chain::commands` boundary
- Transaction application through the internal `chain::transactions` boundary
- Attestation acceptance, quorum checks, validation seeds, and stake-weighted block finality checks through
  the internal `chain::validation` boundary
- Profile-neutral `ChainEngine`, file-backed `ChainStore`, and shared `ChainProfile`/`NodeConfig`
  boundaries so local CPU, public testnet, and future mainnet profiles build the same transition engine
- Receipt-bound validation randomness anchors: receipt admission persists the finalized beacon round,
  finalized randomness, and derived validator-assignment seed for each admitted receipt; validator
  assignment and `Chain::validation_seed` use the persisted anchor so later finalized-beacon advancement
  cannot change an admitted receipt's assigned validators or challenge-vector seed. Full VRF/drand and
  external commit-reveal lifecycle wiring remain open.
- Model-state transition sequencing and conflicting-root settlement delay for training steps
- Txpool with reference transaction payload parsing, receipt deduplication, and multi-validator attestation flow
- Negative-path coverage for transaction parsing, chain registration/receipt/attestation/block-vote rejection,
  verifier metadata/commitment mismatch rejection, RPC route validation, HTTP parsing/socket error responses,
  faucet exhaustion, malformed P2P payloads, and malformed peer-book records
- Full line coverage for TensorVM Merkle helpers, tensor server access, type/signature helpers, validator
  root-availability handling, tensor primitives, TensorVM wrappers, CLI parsing, runtime backends,
  faucet, miner, scheduler, storage, watcher, and local testnet/public-evidence modules
- Deterministic job scheduler, operator-separated miner replication assignment with fallback when
  diversity is insufficient, and validator assignment
- `JobSource` and `SyntheticLocalJobSource` boundaries for local CPU job generation, with deterministic
  post-startup matmul and LinearTrainingStep jobs emitted without embedding scheduler policy directly in
  the block-production adapter, and profile-controlled enablement so local CPU can generate synthetic jobs
  without public testnet or mainnet inheriting local-only job production
- The long-running service runtime consumes `TENSORVM_CHAIN_PROFILE`, builds `NodeConfig` at the CLI
  boundary, reports `chain_profile` and `role_chain_profile`, and gates timed synthetic production through
  profile, role, block-interval, and local-producer policy so local CPU, public-testnet, and mainnet profiles
  share the same runtime path while selecting different job-source policy.
- `NodeConfig` now owns typed `NetworkConfig` and `StorageConfig` values for runtime listen addresses,
  libp2p identity seed, RPC auth token, max-request limit, and node-store path, reducing the remaining
  service-loop argument plumbing to runtime command and role label.
- Local block catch-up now accepts decoded `TensorBlock` payloads through `ChainCommand::SubmitBlock` after
  prerequisite job, receipt, attestation, settlement, and model-transition state is locally available.
  `NewBlockHeader` remains an announcement/locator and no longer satisfies non-producer block-application
  evidence.
- Network block-vote catch-up now accepts decoded `BlockVote` payloads through
  `ChainCommand::SubmitBlockVote`, persists vote-only state changes, rejects conflicting duplicate
  validator votes, and exposes ingested/applied block-vote counters for service and role runtimes.
- Network validator-audit catch-up now accepts bounded signed validator audit report payloads through
  `ChainCommand::SubmitValidatorAuditReport`, queues reports whose assignments or dependencies are not yet
  available, rejects conflicting duplicate reports, publishes locally submitted reports over p2p gossip,
  and exposes validator-submitted plus network-ingested/applied audit-report counters for service and role
  runtimes.
- Validator proposer role status now distinguishes useful settled-receipt block proposals from empty
  fallback blocks. The scheduled local producer publishes deterministic jobs only; the validator role tick
  observes settled receipts with local tensor artifacts and validator attestations before submitting useful
  block proposals through the chain engine. Validator proposal is gated by the configured validator
  proposer duty, not by the local synthetic job producer path, so a validator can propose from already
  accepted settled state even when synthetic job production is disabled. Runtime status records
  settled-receipt proposer readiness, artifact-ready receipt count, attested receipt count, total proposed
  blocks, useful proposal count, fallback proposal count, and selected receipt count; `tvmd node status`
  passes those fields through and the local CPU checker requires positive useful proposal,
  proposed-receipt, artifact-ready, and attested receipt evidence instead of accepting a generic
  produced-block counter.
- `CpuReferenceMinerRole`, `ReferenceValidatorRole`, and `RoleReceiptBundle` boundaries for CPU role work,
  so local synthetic production drives miner execution and validator verification through role-owned
  components before submitting receipts and attestations through the shared chain engine
- Redundant miner-output agreement quorum before settlement, with disagreement/fewer-than-quorum receipts
  delayed rather than rewarded
- Miner node executor with receipt submission and tensor serving
- Validator node attestation flow for TensorOp and LinearTrainingStep receipts
- Server-backed TensorOp data availability verification with unavailable attestations
- Tensor server for descriptors, rows, chunks, Merkle openings, and retention-window pruning
- End-to-end local matmul round: schedule, mine, serve tensors, verify via tensor server, attest, settle, and produce block
- End-to-end local LinearTrainingStep round: register model, mine, verify, attest, settle, update model state, and produce block
- Library-owned local CPU synthetic round producer that schedules matmul and LinearTrainingStep jobs,
  executes CPU miner work, verifies, settles, advances model state for training jobs, and appends blocks
  without synthesizing finality votes in the runtime path; `tvmd node serve` now calls this shared
  protocol path
- P2P message enum, deterministic byte codec, rust-libp2p runtime dependency, TCP/TLS/Yamux swarm
  construction, Gossipsub topic subscriptions for block/job/receipt/attestation/peer announcements,
  Identify protocol wiring, Kademlia discovery/address registration, JSON request-response protocols for
  tensor chunks, tensor rows, root-addressed full tensor payloads, and program fetches,
  `tvmd node peer add` bootstrap seeding, `tvmd node check` short startup checks for the
  mandatory libp2p control-plane runtime, `tvmd node serve` long-running startup of the same runtime,
  DNS/TCP bootstrap dialing with redial after disconnect, service-level bounded request-response calls,
  local tensor registration for request serving, local job/receipt/attestation/block/block-vote payload
  announcements and height-bearing block-header announcement publishing over Gossipsub, decoded inbound message queues
  consumed by the role runtime,
  pending block/block-vote/receipt/attestation payload retry when gossip arrives before prerequisite parents,
  jobs, receipts, or blocks,
  runtime-observed consensus-gossip counters, latest observed block heights, bounded observed block-hash
  sets, network-event ingestion counters, and network-applied block counters exposed through role status,
  fixed-size bounded block and block-vote payload decoding, bounded tensor-row response lengths,
  and durable libp2p bootstrap peer-book storage with checksum validation and `/p2p/<peer-id>` dial
  multiaddr loading
- Validator role remote tensor fetch: assigned validator role loops can request missing receipt tensor
  artifacts by commitment root over libp2p request-response, decode and verify fetched tensors against the
  requested roots, insert them into local runtime storage, and submit the resulting attestation through
  `ChainCommand::SubmitAttestation`; role status exposes fetch attempts, successes, failures, bytes, and
  inserted tensors
- Documented network-stack recommendation that makes libp2p the mandatory MVP runtime for consensus
  propagation and bounded tensor/program fetches
- Node/tensor RPC route handling, state-root-bearing `/chain/head` responses, service and per-surface
  health endpoints, explorer data RPC endpoints, `/explorer/ws` WebSocket polling for browser explorers,
  telemetry/faucet RPC endpoints, browser-facing explorer/telemetry/faucet HTML pages, mutable
  transaction submission, job lookup, HTTP response formatting, generic HTTP request reading, socketed
  stdlib HTTP serving, `tvmd node init/peer add/check/serve` launch
  configuration for a `NodeStore`-backed service process with mandatory rust-libp2p listen configuration,
  and gateway auth/body-size/rate-limit enforcement
- CLI parser and `tvmd` binary entrypoint for documented miner/validator/proposer commands, with local
  stake, wallet, device where relevant, mandatory libp2p node-endpoint validation, and structured readiness
  reports
- Role-specific long-running `tvmd miner run`, `tvmd validator run`, and `tvmd proposer run` command
  surfaces that validate the role config, start the mandatory libp2p-backed service runtime, write live
  role-loop counters from a reusable node runtime state object, delegate decoded network message ordering,
  invalid event accounting, block/job/receipt/attestation/block-vote payload application, pending payload
  retry, and producer versus non-producer block-payload dispatch through a shared node runtime event driver,
  submit validator-owned block votes for unvoted valid local blocks, and report role runtime readiness plus
  local-producer mode, network-applied block counters, validator block-vote submissions,
  ingested/applied network block-vote counters, and observed job/receipt/attestation/block/block-payload/
  block-vote gossip counters through `tvmd node status`
- CPU reference backend for portable default builds, plus a CUDA-only `GpuMinerBackend` that reports
  the selected device and rejects execution unless native CUDA kernels are compiled
- Miner CLI readiness now treats `--device cpu` as the portable reference backend and requires
  `--features cuda-kernels` plus an available CUDA device before `--device cuda:N` can report GPU miner
  readiness
- Optional `cuda-kernels` feature that builds `kernels/cuda/field_matmul.cu` with `nvcc`, routes the
  `GpuMinerBackend` matmul path and LinearTrainingStep forward, backward, error, update, transpose, and
  loss substeps through native CUDA kernels, and checks CUDA outputs against canonical CPU outputs
- Restartable `NodeStore` data directory that persists chain snapshots, append-only block logs, and the
  durable peer book with fixed-format encoding, checksum validation, parent-link checks, append-only sync,
  full-chain state snapshots for restart, snapshot/block-log/state mismatch detection, and service-init
  recovery that rewrites torn snapshot/block-log state from valid `chain.state`
- Watcher tooling that scans chain evidence for invalid receipts, data withholding, validator misconduct,
  missing quorum, missing redundant agreement, and conflicting learning-state transitions
- Faucet, explorer WebSocket summaries, full local telemetry success metrics, local testnet bootstrap, and
  public-testnet evidence reporting that separates local readiness from external 7-day run proof
- Typed public-testnet run evidence evaluation for disjoint distinct miner/validator operators, one-to-one
  matching between live operator IDs and live node addresses for counted public participants,
  signature-verified node heartbeat summaries that cover the observed block count, signed wall-clock
  run-window evidence, observed block continuity, finality rate, data-availability rate, invalid-work
  rejection evidence, reward-settlement records, production libp2p runtime evidence, internally consistent
  finalized-block and available-receipt counters, and deployed RPC/explorer/faucet/telemetry service
  reachability with exactly one service-health and one service-content record per deployed service kind,
  reachable and signed health-check summaries that cover the observed block count, rejection of
  overreported reachable counts above signed health-check counts, signed content-root observations bound
  to external HTTPS service URLs and paths, requiring distinct service endpoint IDs and distinct
  service-content roots across the four deployed service kinds
- Typed public-testnet evidence-bundle evaluation that additionally requires an external public manifest
  location, exactly one verified manifest publication signature in the current manifest format, signed
  independent auditor records bound to external audit URIs, distinct from the manifest signer, and observed
  at or after the signed run-window end with an exact match to `independent_auditor_count`, a signed
  run-window record, block/finality history, signed
  operator identity attestations observed inside the signed run window and matched exactly to the
  independent operator/address pairs selected by criteria-aware one-to-one public matching, so a
  validator-satisfying match is not rejected merely because greedy role ordering or address choice consumed
  a shared address, live but uncounted nodes cannot satisfy a missing counted operator attestation, and
  missing, duplicate, extra, or overreported operator-attestation records are rejected, signed
  per-operator production libp2p network-observation records, signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary roots, signed
  external artifact locators for the raw records behind each summary root with exactly one locator for
  each required supporting-record kind, well-formed whitespace-free
  `ipfs://`/`ar://` content identifiers with traversal/query/fragment path rejection, HTTPS evidence URI
  concrete-path enforcement with root-only/query/fragment rejection, exact untrimmed URI/path manifest-field
  validation, duplicate scalar manifest-field rejection, whitespace-padded field-key and scalar-value rejection,
  whitespace-padded repeated-record value rejection, and
  exact run-derived block/finality/network-runtime/data-availability/invalid-work summary counts, distinct node-address
  counting for public operators, plus network-runtime observation rejection for missing records,
  unmatched operators, non-public listen addresses, stale timestamps, undercounts, and overcounts against
  every counted public operator before full-spec evidence can be considered
  independently checkable; the `public_evidence_full_spec`
  report flag also requires the default 7-day, 10-miner, 5-validator public-testnet criteria or stricter
  criteria, so relaxed local harness criteria cannot mark an evidence bundle full-spec
- Dependency-free public-testnet preflight manifest parsing plus a CLI launch-readiness surface for
  `tvmd public preflight <path>`, with public service endpoint checks rejecting local,
  private, link-local, special-use DNS, single-label DNS, documentation, shared-address, benchmarking,
  multicast, reserved, and malformed HTTPS authorities, rejecting service URL query strings/fragments, and
  requiring exact untrimmed service URL/path manifest fields and exact comma-separated `service=...`
  values, a `cuda_ready_miner_count` that matches the planned public miner count, a
  `libp2p_ready_node_count` that matches the planned miner plus validator count and can be derived from
  process-level `tvmd node check` checks that load the initialized node store, load the durable peer
  book, start the real rust-libp2p control plane, report `libp2p_ready=true`, and exit, plus distinct
  endpoint IDs for exactly one ready RPC, explorer, faucet, and telemetry service plan on the planned
  public content paths used by post-run evidence, with missing, duplicate, or extra preflight service plans
  rejected by the public service plan gate
- `tvmd` binary tests for the documented spec-path pending manifest commands, proving
  `tvmd public preflight docs/tensorvm/public-testnet.preflight` reads the checked
  manifest and reports `public_testnet_preflight_ready=false`, while
  a process-level generated external-addressed preflight manifest reports
  `public_testnet_preflight_ready=true`, and
  `tvmd public evidence validate docs/tensorvm/public-testnet.evidence` reads the checked
  manifest and reports `public_evidence_full_spec=false`
- Public deployment scaffold under `deploy/tensorvm/` with an environment template, systemd unit for the
  explicit `tvmd` binary target, nginx HTTPS reverse-proxy template for RPC/explorer/faucet/telemetry
  hostnames, a template guard test that requires mandatory libp2p startup, durable data-dir use,
  auth-token wiring, hardened systemd settings, TLS proxying, and the required public HTTPS surfaces, an
  operator runbook guard test that requires the preflight status flags, evidence generator commands, daily
  checkpoint requirements, post-run validation flags, publication artifacts, and explicit no-real-run
  blocker, a deployment README guard test that requires the scaffold file list, public service routes,
  minimal operator flow, evidence commands, and non-evidence boundary, a preflight manifest
  example that parses but does not report launch readiness until special-use placeholder hosts are
  replaced, checked spec-path pending manifests at `docs/tensorvm/public-testnet.preflight` and
  `docs/tensorvm/public-testnet.evidence` that parse from the documented CLI paths while intentionally
  reporting not-ready/non-full-spec until replaced by owned public infrastructure and real run records, and
  a checked post-run evidence manifest example that validates structurally while still reporting
  `public_evidence_full_spec=false`
- Dependency-free public evidence manifest parsing plus a CLI validation surface for
  `tvmd public evidence validate <path>`, plus
  `tvmd public evidence publish ...`, `tvmd public evidence audit ...`,
  `tvmd public evidence run window ...`, and
  `tvmd public evidence node heartbeat ...` generation for signed publication, independent-auditor,
  wall-clock run-window, and external-operator heartbeat fields, plus
  `tvmd public evidence run window-file ...` generation that derives the signed run-window manifest
  fields from saved contiguous per-block `run_window_observation=...` files with
  duplicate-block, gap, zero-timestamp, decreasing-timestamp, unsupported-line, and
  whitespace-padded-record rejection,
  `tvmd public evidence node heartbeat-file ...` generation that derives signed `node=...` lines
  from saved contiguous per-block `node_heartbeat_observation=...` files with duplicate-block, gap,
  identity-mismatch, unsupported-line, and whitespace-padded-record rejection,
  `tvmd public evidence node operator-attestation ...` generation for signed operator identity records bound to
  external identity URIs,
  `tvmd public evidence service health ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service=...` manifest records bound to external HTTPS health URLs and observation counts, with
  root-only, query-string, fragment, and non-exact health URL rejection, plus
  `tvmd public evidence service health-file ...` generation that derives the same signed
  `service=...` line from saved contiguous per-block `service_health_observation=...` files with
  duplicate-block, gap, unsupported-line, and whitespace-padded-record rejection,
  `tvmd public evidence service content ...` generation for exact signed RPC/explorer/faucet/telemetry
  `service_content=...` manifest records bound to external HTTPS content URLs, required content paths,
  matching service endpoint IDs, matching service-health HTTPS authorities, exact query-free URL paths,
  root-only, query-string, fragment, and non-exact content URL rejection, distinct content roots, and at
  least 64 observed bytes, plus
  `tvmd public evidence service content-bytes ...` generation that derives those content roots from
  exact captured response-body bytes and `tvmd public evidence service content-file ...` generation
  that derives them directly from captured response-body files,
  `tvmd public evidence network observation ...` generation for signed public libp2p runtime observation
  records with missing TCP listen port, zero TCP port, non-public multiaddr, malformed DNS-label, and
  single-label DNS rejection, plus `tvmd public evidence network from-service-log ...`
  generation that derives the peer ID, protocol counts, bootstrap-peer count, and DoS-control settings
  from captured `tvmd node serve` logs while still requiring a public listen multiaddr, plus manifest
  validation that binds one such signed raw record to every counted public operator and to the aggregate
  network-runtime root; the process-level `tvmd` service smoke test now derives a public-address
  observation root from the live libp2p peer/protocol/control stdout and feeds that root through
  `evidence record summary-roots`, `evidence record artifact-roots`, and the matching file-derived commands,
  `tvmd public evidence record summary ...` generation for signed
  block/finality/network-runtime/data-availability/invalid-work/reward-settlement summary fields including
  production libp2p network-observation roots,
  `tvmd public evidence record artifact ...` generation for signed external raw-record artifact locators,
  `tvmd public evidence record artifact-roots ...` generation that signs artifact locators from the
  same derived aggregate root and count as summary generation, `tvmd public evidence record
  summary-roots ...` deterministic root aggregation for post-run supporting records with
  duplicate-root and whitespace-padded root-list rejection, plus `tvmd public evidence record summary-file ...` and
  `tvmd public evidence record artifact-file ...` generation from saved raw-record files containing
  `record_root=...` lines, fully verified signed `network_runtime_observation=...` lines, or typed
  `block_history_record=...`, `finality_history_record=...`, `data_availability_measurement=...`,
  `invalid_work_rejection=...`, and `reward_settlement=...` supporting-record lines with kind-specific
  field validation, including hex reward-settlement participant IDs, exact-line hashing, and
  whitespace-padded or empty-field rejection; network-runtime file
  derivation rejects malformed peer IDs, non-public multiaddrs, zero counters, and mismatched observation
  roots or signatures before aggregation; a process-level `tvmd` integration test now assembles a short
  external-addressed evidence manifest entirely from the signed generator subcommands, validates it from
  disk, and proves it is independently checkable without allowing the default full-spec flag to pass
- Local CPU Docker Compose deployment bundle under `deploy/tensorvm/local-cpu/`, with a CPU-only
  Dockerfile, explicit 10-miner/5-validator Compose topology, one durable volume per operator, mandatory
  libp2p readiness checks for all 15 operators, stable operator-ID-derived libp2p identities, CPU miner
  readiness, role-specific `tvmd miner run` and `tvmd validator run` Compose entrypoints checked through
  `runtime_command` status, explicit role-run loop wrappers feeding a shared runtime loop
  boundary, registered role wallet address and role registration status persisted through
  `role-runtime.status` and checked through `tvmd node status`, runtime policy that prevents service,
  miner, and legacy proposer roles from becoming local block producers while allowing validator runtimes
  only with the explicit producer flag and interval, authenticated host gateway route
  checks, a seeded local CPU chain exposed through the gateway with settled matmul and LinearTrainingStep
  receipts, plus live synthetic CPU job production on `validator-00` with typed job, receipt, and
  attestation payloads gossiped, decoded, and applied through `ChainCommand::SubmitJob`,
  `ChainCommand::SubmitReceipt`, and
  `ChainCommand::SubmitAttestation` on non-producers, typed block and block-vote payloads gossiped, decoded,
  and applied through `ChainCommand::SubmitBlock` and `ChainCommand::SubmitBlockVote`, so post-startup
  blocks advance through receipts, attestations, settlement, validator block production, and role-owned
  block-vote finality instead of a static snapshot, miner role loops
  that report assigned-job and unreceipted-job readiness from loaded chain state and can submit assigned
  unreceipted receipts through the shared chain engine while inserting served tensor artifacts locally,
  validator role loops that can submit assigned attestations through the shared chain engine when local tensor
  artifacts are available, submit block votes for unvoted valid blocks, and propose useful blocks from
  ready settled state without depending on synthetic job production, miner rewards, finality, data
  availability, a standalone explorer service that polls the TensorVM `/explorer/ws`
  WebSocket endpoint, a rolling
  all-operator restart-continuity gate with node-store recovery from torn local writes, all-operator
  durable status checks, an all-operator finalized common-head checkpoint queried through
  `tvmd node block`, a local-only evidence boundary, and
  `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape` guarding the artifact shape

## Implemented In `crates/tensor_vm_explorer`

- Standalone `tensorvm-explorer serve` clap command that serves the browser explorer from
  `TENSORVM_EXPLORER_LISTEN` and publishes the TensorVM WebSocket URL configured by
  `TENSORVM_EXPLORER_WS_URL`; `tensorvm-explorer health-check` validates the running service
- Default terminal-style explorer UI shell, Ratzilla/Ratatui WASM entry point, and JSON view models for
  overview metrics, latest blocks, account lookup, miners, validators, receipts, and jobs
- Local CPU Compose integration on `127.0.0.1:8080`, configured to poll `miner-00` through
  `ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token`

## Verified Gates

Current local verification commands:

```bash
cargo test -p tensor_vm local_testnet --release
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
cargo fmt --check --all
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo tarpaulin
cargo test -p tensor_vm --features cuda-kernels --release
cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings
```

The May 20, 2026 Compose verification on this host used
`TENSORVM_LOCAL_CPU_EXPLORER_PORT=18080` for `up --wait` and local check-script runs because host port
`8080` was already allocated; the Compose default remains `8080`.

Gate 0 is the first non-skippable CPU local multi-participant testnet required before CUDA, public
preflight, public evidence, or deployment-gated work can count:

- `cargo test -p tensor_vm local_testnet --release`: 5 TensorVM tests passed, covering the local
  10-miner/5-validator bootstrap shape, separate participant identities and libp2p endpoints, live
  mandatory libp2p control-plane startup under default features, real loopback libp2p delivery across every
  TensorVM gossip topic and request-response message family, matmul settlement/rewards, LinearTrainingStep
  state transition, tensor-server availability, no simulation or local-only
  networking-shim credit, and the explicit non-public-run evidence boundary

- Local CPU Compose gate: `docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait`
  started all 15 operator containers as healthy; `deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh`
  reported `local_cpu_testnet_ready=true`, `ready_miners=10`, `ready_validators=5`,
  `distinct_operator_ids=15`, `distinct_libp2p_peer_ids=15`, `distinct_node_multiaddrs=15`,
  `libp2p_ready_node_count=15`, `cpu_ready_miner_count=10`, `cuda_required_miner_count=0`,
  `settled_receipts=10`, `matmul_settled=true`, `linear_training_settled=true`, positive
  `rewarded_miners`,
  seeded `total_reward_balance`, seeded `attestation_count`, `finality_rate_bps=10000`,
  `data_availability_bps=10000`, `public_evidence_full_spec=false`, and `independently_checkable=false`,
  with `standalone_explorer_ready=true` and
  `standalone_explorer_websocket_polling=true`; the gate now also requires
  `live_block_production=true`, `live_synthetic_jobs=true`, `live_linear_training_jobs=true`,
  `live_attestations=true`, `live_receipt_attestations=true`, `live_tensor_op_receipts=true`,
  `live_linear_training_receipts=true`, `live_tensor_op_block_evidence=true`,
  `live_linear_training_block_evidence=true`, `live_tensor_fetch=true`, and `live_rewards=true`, proving
  `/chain/head` and explorer counters advance past the seeded two-block baseline, at least one live
  LinearTrainingStep advances model state after startup, validators add attestations, `/explorer/receipts`
  exposes per-receipt validator attestation details plus named post-seed TensorOp and LinearTrainingStep
  primitive receipts for live work, and `tvmd node block` exposes finalized live block-height receipt
  IDs and primitive counts for both TensorOp and LinearTrainingStep work,
  `/tensor/latest` returns a live tensor ID whose descriptor, row, chunk, and opening are fetchable, and
  settled live work credits new rewards; the gate also runs `tvmd node status` and
  `tvmd node block` inside all 15 operator containers and requires
  `all_operator_live_block_convergence=true` plus `all_operator_common_head_convergence=true`, proving
  every durable node store advanced past the shared seed, reports the same first live finalized block
  hash, can return the same finalized common-head block hash at the bounded convergence height, and can
  catch up to validator-00's finalized local-head checkpoint after that head appears in p2p block gossip with
  matching finalized block hash and state root via
  `all_operator_network_head_convergence=true`, plus p2p observation of that same network head hash,
  `all_operator_block_log_roots_observed=true`, `all_operator_role_status=true`,
  `all_operator_role_runtime_commands=true`, `all_operator_chain_profiles=true`,
  `all_operator_role_production_policy=true`, `all_operator_role_runtime_counters=true`,
  `single_local_producer=true`, `local_validator_producer=true`,
  `all_non_producer_network_applied_blocks=true`,
  `all_non_producer_network_block_payload_ingestion=true`,
  `all_non_producer_network_block_payload_application=true`,
  `live_validator_block_vote_networking=true`,
  `all_non_producer_network_block_vote_ingestion=true`,
  `all_non_producer_network_block_vote_application=true`,
  `all_non_producer_network_event_ingestion=true`,
  `all_non_producer_network_payload_announcements=true`,
  `all_non_producer_network_job_payload_application=true`,
  `all_non_producer_network_receipt_payload_application=true`,
  `all_non_producer_network_attestation_payload_application=true`,
  `all_operator_p2p_connected_peers=true`, `all_operator_p2p_block_gossip=true`,
  `all_operator_p2p_block_payload_gossip=true`, `all_operator_p2p_block_payload_head_observed=true`,
  `all_operator_p2p_block_vote_gossip=true`,
  `all_operator_p2p_job_gossip=true`, `all_operator_p2p_receipt_gossip=true`,
  `all_operator_p2p_attestation_gossip=true`,
  `all_operator_p2p_target_head_observed=true`, `all_operator_p2p_latest_head_observed=true`, and
  `all_operator_chain_counters=true`, proving each operator status surface reports its role, runtime
  command, active chain profile, live role-loop counters, one block-production-capable runtime, one local timed producer,
  network-applied block progress from decoded block payloads on every non-producer, validator-owned block
  vote submission, network block-vote ingestion/application on every non-producer, decoded
  job/receipt/attestation event ingestion plus decoded block/job/receipt/attestation payload application on
  every non-producer, real libp2p connected-peer count,
  observed consensus gossip including block-payload gossip for the target convergence head and block-vote
  gossip, live chain counters,
  and durable block-log root;
  `check-rolling-restart-continuity.sh` is now the full local restart gate and runs the same continuity
  check one service at a time across every counted operator, proving each
  restarted service keeps a stable libp2p peer ID, preserves the pre-restart finalized common head and state
  root on every operator, advances height, block count, state-root, and block-log-root evidence, and
  continues finalizing blocks; `tvmd node init` repairs torn snapshot/block-log state from valid
  `chain.state` before a restarted service reports readiness

The workspace currently has 262 passing tests under Tarpaulin:

- 14 in `experiments`
- 247 in `tensor_vm`
- 1 in `tensor_vm_explorer`

`cargo test -p tensor_vm --tests` also runs 22 `tvmd` binary unit tests, 1 local CPU Compose integration
test, and 7 `tvmd` CLI integration tests for the documented spec-path pending manifest commands, a
generated launch-ready preflight manifest round trip, a generated short-run evidence manifest round trip
that reports `independently_checkable=true` and `public_evidence_full_spec=false`, a local CPU seed command
that persists a settled two-block local chain, a role-run command test that proves `tvmd miner run`,
`tvmd validator run`, and `tvmd proposer run` serve through role-specific loop wrappers and runtime
surfaces with mandatory libp2p startup, then proves registered local-testnet role wallet addresses are
exposed through role-run stdout and `tvmd node status`, bounded service startup can generate live
synthetic CPU jobs and append unfinalized live blocks when no validator role loop has voted, plus a supervised
`tvmd node init` / `tvmd node peer add` / `tvmd node check` / bounded `tvmd node serve`
lifecycle smoke test that starts the mandatory libp2p service path and serves authenticated `/health`, `/rpc/health`,
`/explorer/health`, `/faucet/health`, `/telemetry/health`, `/chain/head`, `/epoch/current`,
`/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, and
`/telemetry/dashboard` from the process-level service, plus authenticated mutable `/tx`, `/receipt`, and
`/attestation` submissions with reference payloads, read-back of registered miner/validator state, and
unauthenticated request rejection. The same process-level smoke test now captures the served
`/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response bodies and verifies that
`tvmd public evidence service content-bytes` and
`tvmd public evidence service content-file` emit identical signed service-content evidence for the
captured bodies, while generating signed `tvmd public evidence service health` lines from reached
RPC/explorer/faucet/telemetry health responses. It also derives the local libp2p peer ID and protocol
counts from service stdout and verifies that `tvmd public evidence network observation` rejects the
loopback listen address instead of counting local service startup as public network evidence.

The current instrumented Tarpaulin line coverage is documented in
[`tarpaulin_report.md`](tarpaulin_report.md):

- 97.29% workspace line coverage
- 11559/11881 workspace lines covered
- 97.81% `tensor_vm` crate line coverage
- 10696/10936 `tensor_vm` lines covered
- 100.00% `tensor_vm_explorer` crate line coverage
- 277/277 `tensor_vm_explorer` lines covered

The CUDA feature gate was also checked locally on an NVIDIA B200 with CUDA 12.8:

- `cargo test -p tensor_vm --features cuda-kernels --release`: 182 TensorVM tests passed, including
  `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges` and
  `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops`
- `cargo clippy -p tensor_vm --features cuda-kernels --all-targets -- -D warnings`: passed

## Still Not A Production/Public Testnet

These spec items require real deployment or non-reference infrastructure and are not complete:

- production GPU-miner packaging and a broader optimized CUDA/C++ kernel suite; the current native kernel
  coverage includes CUDA field-matmul plus linear-step sub/scalar/transpose/squared-error kernels checked
  against canonical CPU outputs
- long-running public 7-day testnet with independent external operators; current implementation exposes
  typed `PublicTestnetRunEvidence`/`PublicTestnetEvidence` so this criterion can be measured without
  treating a local test harness as public proof, and now requires a signed wall-clock run window,
  invalid-work rejection plus reward-settlement records, signed per-operator production libp2p runtime
  observation records that aggregate to the network-runtime root, signed external artifact locators for raw supporting records, deployed public-service
  reachability, and signed public-service content roots before public evidence can satisfy the gate
- published external public-testnet evidence bundle; the required bundle shape is documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), and
  `deploy/tensorvm/RUNBOOK.md` records the external collection and publication flow, while
  `docs/tensorvm/public-testnet.evidence` and
  `deploy/tensorvm/manifests/public-testnet.evidence.example` are checked as non-full-spec format
  examples, but no complete external bundle is available yet
- externally observed production libp2p operation during a public testnet; current implementation starts
  the mandatory rust-libp2p service runtime locally with bounded Gossipsub payloads, request timeouts,
  concurrent stream limits, idle connection timeouts, Kademlia discovery/address registration, and durable
  bootstrap peer-book persistence loaded as peer-ID-preserving dial multiaddrs, and the public evidence validator now requires signed
  network-observation records, but no independently checkable public-run network evidence is available yet
- production HTTP deployment and full durable database; current implementation has a stdlib socketed HTTP
  wrapper, `tvmd node init/peer add/check/serve` launch wiring, in-process auth/body-size/rate-limit enforcement, and a
  restartable reference `NodeStore` data directory with consistency-checked snapshot, append-only
  block-log, full-chain state, and peer-book persistence, plus tested deployable systemd/env/nginx templates, while
  public evidence validation now rejects local, private, special-use DNS, single-label DNS, documentation,
  shared-address, benchmarking, multicast, reserved, malformed service URLs, root-only service URLs, and
  service URLs with query strings or fragments
- deployed browser explorer, faucet, and telemetry web services; current implementation exposes node RPC
  endpoints, a local standalone WebSocket explorer, and local browser-facing HTML pages for telemetry and
  local faucet claims

The current crate is a complete deterministic reference core and local test harness, not a production
network release.
