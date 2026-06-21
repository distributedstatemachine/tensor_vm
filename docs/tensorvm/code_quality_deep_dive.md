# TensorVM Code Quality Deep Dive

This is a strict implementation-quality review of the current TensorVM codebase, focused on separation of
concerns, SOLID-style ownership boundaries, DRY, idiomatic Rust, and long-term maintainability.

The codebase has a strong technical core: the tensor primitives, verification math, typed chain commands,
and local test coverage show real engineering effort. The main risk is not lack of tests. The main risk is
that large operational surfaces have grown around the core and now own too much behavior through public
state, string contracts, shell scripts, duplicate codecs, and god modules.

## Executive Verdict

The biggest design problem is that TensorVM does not yet have a hard canonical mutation boundary.
`ChainCommand` and `ChainEvent` are the right direction, but `Chain`, `ChainState`, runtime loops,
RPC, tests, and deployment scripts can still bypass or duplicate chain behavior.

That creates four recurring failure modes:

- Consensus invariants are enforced in some paths and bypassed in others.
- Runtime adapters accumulate protocol logic that belongs in `chain/*`.
- Tests certify string outputs and shell contracts instead of typed behavior.
- Manual codecs and parsers drift across p2p, storage, RPC, CLI, and shell.

The repair should be structural, not cosmetic. Splitting files before fixing ownership would only move
spaghetti around.

## Implementation Progress

- Iteration 1 completed the core abstraction rename: the protocol state machine is now `Chain`, with no
  `LocalChain` compatibility alias in the Rust source or public exports. The remaining findings below refer
  to `Chain` as the current type, while the rationale section preserves why the old name was removed.
- Iteration 2 started the encapsulation work by making `Chain`'s top-level `params`, `state`, and `blocks`
  fields crate-private and adding inherent read-only accessors. Internal modules can still mutate
  `ChainState` directly, so the larger command-boundary finding remains open.
- Iteration 3 moved local synthetic block production and the localnet test finality helper onto
  `ChainCommand::ProduceBlock` and `ChainCommand::SubmitBlockVote`. The local CPU round now uses the
  command boundary for jobs, receipts, attestations, settlement, block production, and block votes.
- Iteration 4 removed the silent `apply_transaction` success path for receipt and attestation reference
  submissions. Those transaction variants are now explicitly txpool-only via `Transaction::is_reference_submission`,
  and direct chain application returns an error instead of pretending to mutate state.
- Iteration 5 stopped runtime RPC serving from persisting chain state after read-only requests. The service
  loop now compares chain snapshots around a served RPC request, persists only when the chain changed, and
  still updates served-request runtime status for read-only traffic.
- Iteration 6 routed `LocalTestnet` participant registration, job/receipt/attestation submission,
  settlement, block production, and block votes through `ChainCommand`. Model registration and model
  transition remain direct until the command API grows model-specific variants.
- Iteration 7 added model registration and model transition variants to `ChainCommand`, with matching
  events, and routed local synthetic linear training, block parent-state preparation, and `LocalTestnet`
  model updates through that command path.
- Iteration 8 made model registration duplicate-safe. `register_model` now returns an error for an existing
  model ID, and the `ChainCommand::RegisterModel` path propagates that instead of overwriting model state.
- Iteration 9 added command variants and events for account transfer and reward claim, then routed
  non-reference `apply_transaction` writes through `ChainCommand` instead of direct imperative helpers.
- Iteration 10 stopped validator remote tensor-fetch bookkeeping from persisting unchanged chain state.
  Runtime status still records fetch failures/successes, but snapshot and chain-state files are no longer
  rewritten when no consensus data changed.
- Iteration 11 moved faucet reward credits behind `ChainCommand::CreditReward`. The faucet now owns only
  drip eligibility and faucet balance, while the RPC claim path asks the chain engine to mutate reward state.
- Iteration 12 made `apply_transaction` return the `ChainEvent`s produced by its delegated `ChainCommand`,
  so public transaction writes no longer silently discard the typed mutation effects.
- Iteration 13 added a command/event wrapper for fraud challenge outcomes. Public challenge application now
  enters through `ChainCommand::ApplyChallengeOutcome`, and rejected/slashing outcomes emit typed events.
- Iteration 14 narrowed `RewardState`'s externally visible fields and added read accessors for balances,
  total balance, and treasury so runtime/RPC/testnet reporting no longer reaches through reward internals.
- Iteration 15 made `RewardState` balances and treasury visible only inside the `chain` module, while storage
  decodes rewards through an explicit constructor and encodes them through read accessors.
- Iteration 16 made reward crediting chain-private too, moving storage and telemetry fixtures through
  `ChainCommand::CreditReward` instead of calling `RewardState::credit` directly.
- Iteration 17 moved non-chain test fixture height/epoch writes behind a crate-test-only chain position
  helper, keeping those setup mutations out of scheduler/localnet/profile tests.
- Iteration 18 moved non-chain settled-receipt fixture inserts behind a crate-test-only helper, leaving
  settlement ownership inside `chain/*` instead of scattered storage/RPC/watcher tests.
- Iteration 19 moved malformed/orphan receipt fixture inserts in telemetry and watcher tests behind a
  crate-test-only `Chain` helper, making those consensus bypasses explicit test setup.
- Iteration 20 moved the storage durability fixture's data-unavailable marker and treasury setup behind
  crate-test-only `Chain` helpers instead of reaching into state internals.
- Iteration 21 moved node network-payload dependency deletion fixtures behind crate-test-only `Chain`
  helpers, so missing job/receipt/attestation setup no longer mutates maps directly outside `chain/*`.
- Iteration 22 added read-only `ChainState` accessors and started moving the binary runtime/reporting
  surface away from direct public field reads.
- Iteration 23 moved RPC handlers and RPC tests onto the `ChainState` accessors, reducing another external
  service surface's dependence on public state fields.
- Iteration 24 moved telemetry snapshot and metric calculations onto the `ChainState` accessors, keeping
  reporting code on the same read-only boundary as RPC.
- Iteration 25 moved scheduler and synthetic job-source reads onto the `ChainState` accessors, so assignment
  and local job generation no longer depend on public state fields.
- Iteration 26 moved local synthetic round production, finality helpers, and localnet tests onto the
  `ChainState` accessors, extending the read boundary through local CPU harness code.
- Iteration 27 moved watcher production scans onto the `ChainState` accessors while leaving malformed
  attestation fixture mutations isolated to watcher tests.
- Iteration 28 moved storage snapshot metadata, chain-state encoding, and storage test temp-name reads
  onto `ChainState` accessors without changing storage fixture mutations.
- Iteration 29 moved the zero-work liveness study's proposer randomness read onto the `ChainState`
  accessor, removing another standalone production reach-through.
- Iteration 30 moved node network-payload ingress duplicate/dependency checks and their colocated
  assertions onto the `ChainState` accessors.
- Iteration 31 finished the read-only telemetry metric paths that still reached through `chain.state`,
  leaving settled-work fixture mutation helpers as a separate follow-up.
- Iteration 32 moved role-level validator stake lookup onto the `ChainState` validator accessor.
- Iteration 33 moved RPC explorer account, operator, receipt, job, and malformed-transaction
  assertion reads onto `ChainState` accessors.
- Iteration 34 moved local testnet simulation, explorer summary, and colocated assertion reads onto
  `ChainState` accessors while leaving invalid-attestation fixture insertion explicit.
- Iteration 35 moved telemetry settled-work fixtures behind a crate-test-only `Chain` helper instead
  of mutating miner state maps directly.
- Iteration 36 moved malformed attestation and optimizer-state fixtures behind crate-test-only
  `Chain` helpers, removing the remaining external test-state map mutations.
- Iteration 37 narrowed `ChainState` fields to `chain/*` internals and routed storage through an
  explicit decoded-parts constructor plus read accessors.
- Iteration 38 added an explicit crate-only `ChainParts` restore constructor and moved storage
  persistence plus storage fixtures off raw `Chain` params/block fields.
- Iteration 39 introduced shared crate-internal enum tag helpers for dtype, primitive type, and
  verification-result codecs, then routed storage, p2p, and chain roots through them.
- Iteration 40 moved fixed-length block and block-vote payload codecs behind shared crate-internal
  helpers while preserving storage and p2p error boundaries.
- Iteration 41 moved telemetry's remaining top-level `Chain` params/block reads onto the public
  accessors, keeping reporting off raw chain fields.
- Iteration 42 moved `JobState` payload encoding and streaming decode into shared crate-internal
  codec helpers while preserving p2p and storage error mapping.
- Iteration 43 moved `ReceiptState` payload encoding and streaming decode into the same shared
  codec boundary while keeping p2p hash-vector caps out of persisted storage decoding.
- Iteration 44 moved validator-attestation payload encoding and streaming decode into shared
  codec helpers, leaving p2p and storage responsible only for envelopes and error domains.
- Iteration 45 collapsed repeated codec-error boundary mapping in p2p and storage while keeping
  the existing p2p trailing-payload messages and storage error strings intact.
- Iteration 46 made block-vote vector decoding loop over the decoded count directly instead of
  relying on vector capacity as an implicit counter.
- Iteration 47 moved libp2p peer-book records, file encoding, and bootstrap multiaddr validation into
  `p2p/peer_book.rs` while preserving the public `p2p` facade exports.
- Iteration 48 moved p2p message routing, gossipsub envelopes, payload wrappers, tensor payload codec,
  and low-level wire readers/writers into `p2p/wire.rs`, leaving `p2p.rs` as the service facade.
- Iteration 49 moved the wire roundtrip and malformed-payload tests into `p2p/wire.rs`, so the parent
  `p2p.rs` test module now focuses on libp2p service behavior and peer-book persistence.
- Iteration 50 moved libp2p request-response behavior aliases, pending request bookkeeping, response
  construction, protocol dispatch, and behavior construction into `p2p/request_response.rs`.
- Iteration 51 moved libp2p connection accounting, observed gossip metrics, request-response event
  routing, and bounded observed-block hash tracking into `p2p/service_events.rs`.
- Iteration 52 moved the derived libp2p network behaviour and gossipsub/identify/kademlia
  behaviour construction into `p2p/behaviour.rs`, leaving node and service startup in the facade.
- Iteration 53 moved the libp2p service handle, public service accessors, lifecycle drop, and
  background worker loop into `p2p/service.rs`, leaving node construction and service tests in the facade.
- Iteration 54 moved libp2p node construction, transport setup, listen binding, and bootstrap dialing
  into `p2p/node.rs`, leaving the parent module as the public configuration facade plus p2p tests.
- Iteration 55 moved peer-book persistence and malformed-decoder tests into `p2p/peer_book.rs`,
  allowing peer-book codec constants and helper readers/writers to become module-private.
- Iteration 56 moved libp2p node construction tests into `p2p/node.rs`, keeping those transport and
  bootstrap assertions with the node builder.
- Iteration 57 moved the raw two-swarm gossip/request-response integration test and its async wait
  helpers into `p2p/node.rs`, leaving `p2p.rs` focused on service-level integration tests.
- Iteration 58 moved the remaining libp2p service integration tests and polling helpers into
  `p2p/service.rs`, leaving `p2p.rs` as the public p2p configuration and re-export facade.
- Iteration 59 extracted snapshot encoding, decoding, persistence, and snapshot-specific tests into
  `storage/snapshot.rs`, starting the documented storage module split behind unchanged public exports.
- Iteration 60 extracted block-log persistence, record codecs, and block-log tests into
  `storage/block_log.rs`, leaving `storage.rs` to orchestrate node-store recovery and chain-state codecs.
- Iteration 61 extracted chain-state persistence, full-chain codecs, and chain-state decoder tests into
  `storage/chain_state.rs`, leaving `storage.rs` focused on `NodeStore` orchestration.
- Iteration 62 extracted `NodeStore` orchestration, recovery/status checks, and node-store tests into
  `storage/node_store.rs`, making `storage.rs` a public facade over the storage submodules.
- Iteration 63 extracted shared primitive storage-format readers/writers into `storage/codec.rs`,
  removing duplicate snapshot readers while keeping chain-state domain codecs in `storage/chain_state.rs`.
- Iteration 64 moved the binary runtime test module out of `main.rs` into `main_tests.rs`, keeping
  test paths stable while reducing the production binary file to runtime code.
- Iteration 65 extracted `service_status` and `service_block` formatting into `main/status.rs`,
  giving the binary service-status reporting code a focused owner behind unchanged CLI dispatch.
- Iteration 66 extracted service init, peer-add, readiness, and local CPU verification command helpers
  into `main/commands.rs`, leaving `main.rs` closer to dispatch and runtime orchestration.
- Iteration 67 extracted binary network ingest and gossip-publishing helpers into `main/network.rs`,
  keeping runtime loop orchestration in `main.rs` while isolating p2p message application glue.
- Iteration 68 moved runtime status snapshot construction and `role-runtime.status` writing into
  `main/status.rs`, consolidating service and role status formatting behind the same module boundary.
- Iteration 69 extracted miner and validator role-work observation/submission helpers into
  `main/roles.rs`, leaving `main.rs` focused on CLI dispatch and runtime-loop orchestration.
- Iteration 70 moved `RoleRuntimeLoop` orchestration into `main/runtime.rs`, keeping the
  entrypoint responsible for command dispatch and runtime configuration construction.
- Iteration 71 moved local-testnet seeding into `main/commands.rs`, keeping command-specific
  testnet and storage dependencies out of the binary entrypoint.
- Iteration 72 moved status-file parsing and hash-list report formatting into `main/status.rs`,
  removing the remaining status helper ownership from `main.rs`.
- Iteration 73 moved runtime role/config construction and role command wrappers into
  `main/runtime.rs`, leaving `main.rs` close to CLI dispatch plus shared seed/identity helpers.
- Iteration 74 moved the remaining shared binary seed and p2p identity report helpers into
  `main/shared.rs`, leaving `main.rs` as CLI dispatch and module wiring.
- Iteration 75 moved runtime role/config construction, profile/env parsing, and wallet registration
  helpers into `main/runtime_config.rs`, leaving `main/runtime.rs` focused on loop orchestration.
- Iteration 76 moved role runtime service-report formatting into `main/status.rs`, reusing the
  runtime status snapshot so `main/runtime.rs` no longer owns the long report string contract.
- Iteration 77 moved service block status reporting into `main/block_status.rs`, keeping block-level
  node-store inspection separate from service and runtime status reporting.
- Iteration 78 moved miner/validator/proposer run-command wrappers into `main/runtime_commands.rs`,
  leaving `main/runtime.rs` to own service-loop startup and ticking.
- Iteration 79 moved miner work observation and receipt submission into `main/miner_role.rs`,
  separating miner role glue from validator role tensor-fetch and attestation logic.
- Iteration 80 moved validator remote tensor fetch and p2p tensor-response parsing into
  `main/validator_fetch.rs`, leaving `main/roles.rs` focused on validator observation and submissions.
- Iteration 81 moved runtime status snapshots, role-runtime status writing, and runtime report
  formatting into `main/runtime_status.rs`, leaving `main/status.rs` focused on service status reads.
- Iteration 82 moved `RoleRuntimeLoop` into `main/runtime_loop.rs`, leaving `main/runtime.rs`
  as the service runtime entrypoint and preserving the existing test-facing loop handle.
- Iteration 83 moved binary runtime persistence/status tests into `main_tests/runtime_persistence.rs`,
  starting the large `main_tests.rs` split along runtime behavior boundaries.
- Iteration 84 moved miner role observation and receipt-submission tests into
  `main_tests/miner_role.rs`, reducing the binary test module's mixed role-test surface.
- Iteration 85 moved validator role observation, attestation, block-vote, and remote tensor-fetch
  tests into `main_tests/validator_role.rs`, keeping role-specific binary tests with their behavior.
- Iteration 86 moved network payload ordering/retry tests and their local chain helpers into
  `main_tests/network_payloads.rs`, further shrinking the binary test parent module.
- Iteration 87 moved runtime role policy, role-loop config/report, wallet-registration, and profile
  parsing tests into `main_tests/runtime_roles.rs`.
- Iteration 88 moved binary service command tests for public manifests and service-store recovery into
  `main_tests/service_commands.rs`.
- Iteration 89 moved runtime network-ingest and loop-counter state tests into
  `main_tests/runtime_state.rs`, leaving `main_tests.rs` as module wiring plus shared helpers.
- Iteration 90 extracted shared binary test helpers into `main_tests/support.rs`, leaving
  `main_tests.rs` as imports and submodule wiring for the split test files.
- Iteration 91 extracted role runtime service startup and owned p2p report metadata into
  `main/runtime_services.rs`, narrowing `main/runtime_loop.rs` toward loop behavior.
- Iteration 92 moved miner role runtime ticking into `main/miner_role.rs`, keeping miner
  observation, receipt submission, persistence, p2p publication, and counters with the miner worker.
- Iteration 93 moved validator role runtime ticking into `main/roles.rs`, preserving the
  `RoleRuntimeLoop` test wrapper while keeping fetch, attestation, block-vote, persistence, p2p
  publication, and counters with the validator worker.
- Iteration 94 extracted local synthetic production timing, persistence, and produced-block
  counters into `main/runtime_production.rs`, leaving `RoleRuntimeLoop` to orchestrate tick order
  and status writes.
- Iteration 95 moved validator role worker implementation into `main/validator_role.rs` and
  updated runtime callers and binary tests to use that focused module directly.
- Iteration 96 split validator runtime ticking into `main/runtime_validator.rs`, leaving
  `main/validator_role.rs` focused on role observation, artifact assembly, attestation submission,
  and block-vote submission.
- Iteration 97 extracted runtime network ingest persistence and counter recording into
  `main/runtime_network.rs`, leaving `RoleRuntimeLoop` to call the network tick and write status
  when activity occurred.
- Iteration 98 extracted runtime RPC serving, mutation detection, chain persistence, and
  served-request counters into `main/runtime_rpc.rs`, preserving the `RoleRuntimeLoop` test wrapper
  as orchestration-only status writing.
- Iteration 99 split runtime status snapshot collection into `main/runtime_status_snapshot.rs`,
  leaving `main/runtime_status.rs` focused on the existing text renderers while preserving the
  status-file and service-report contracts.
- Iteration 100 added a cached `key=value` status-file reader for `service_status`, so
  `main/status.rs` now reads `local-cpu-ready` and `role-runtime.status` once each instead of
  reparsing the same files for every projected field.
- Iteration 101 split `NetworkEventIngest` and `NodeRuntimeState` into
  `node/runtime_state.rs`, keeping the public `node` re-exports unchanged while separating runtime
  counters from network payload application.
- Iteration 102 split `PendingNetworkPayloads` queue and retry bookkeeping into
  `node/pending_payloads.rs`, keeping the same public `node` re-export while shrinking the network
  ingestion module toward payload application only.
- Iteration 103 split network payload apply result types, processor traits, and chain/context
  processor adapters into `node/payload_processor.rs`, leaving `node.rs` to orchestrate message
  ingest and concrete payload application.
- Iteration 104 moved concrete network payload decode, validation, and chain application helpers
  into `node/payload_application.rs`, keeping the existing public `node` helper API while reducing
  `node.rs` to network message orchestration and tests.
- Iteration 105 split network message ingest ordering and event-counter orchestration into
  `node/message_ingest.rs`, leaving `node.rs` as the public node facade plus the remaining inline
  tests.
- Iteration 106 moved runtime-state-specific node tests into `node/runtime_state.rs`, starting the
  inline node test split without changing the public node facade.
- Iteration 107 moved pending-payload retry and duplicate-queue tests into
  `node/pending_payloads.rs`, keeping retry fixtures colocated with the queue implementation.
- Iteration 108 moved network ingest ordering and driver tests into `node/message_ingest.rs`, so
  ingest fixtures sit beside the ingest orchestration instead of the node facade.
- Iteration 109 moved network payload application tests into `node/payload_application.rs`, leaving
  `node.rs` with facade re-exports and the remaining payload processor retry coverage.
- Iteration 110 moved the chain payload processor retry test into `node/payload_processor.rs`, so
  `node.rs` is now a small public facade over the split node submodules.
- Iteration 111 moved the large inline CLI test module into `cli/tests.rs`, shrinking `cli.rs`
  toward parser and reference-command code while preserving private helper coverage.
- Iteration 112 extracted public network-observation address filtering into
  `cli/network_observation.rs`, giving the public-evidence path a focused owner for routability
  checks.
- Iteration 113 extracted shared CLI argument parsing and public-evidence tag helpers into
  `cli/arguments.rs`, reducing parser/helper coupling in the parent CLI module.
- Iteration 114 extracted CLI runtime validation, address derivation, CUDA readiness, and endpoint
  checks into `cli/validation.rs`, leaving the parent CLI module closer to command parsing and
  dispatch.
- Iteration 115 extracted CLI public-testnet evidence and preflight report renderers into
  `cli/reports.rs`, preserving the exported validation functions while narrowing `cli.rs`.
- Iteration 116 extracted CLI public service health and content evidence builders into
  `cli/service_evidence.rs`, keeping service observation parsing and content-root generation out
  of the parent CLI module.
- Iteration 117 extracted CLI public evidence publication and auditor record builders into
  `cli/publication_evidence.rs`, separating publication proof formatting from command dispatch.
- Iteration 118 extracted CLI public run-window evidence formatting and observation parsing into
  `cli/run_window_evidence.rs`, leaving the parent CLI module with only dispatch wiring for that
  command family.
- Iteration 119 extracted CLI public node heartbeat and operator identity evidence helpers into
  `cli/node_evidence.rs`, keeping heartbeat observation parsing beside node evidence formatting.
- Iteration 120 extracted CLI public network observation evidence and service-log parsing into
  `cli/network_evidence.rs`, giving public evidence record aggregation a focused network-root
  verifier to call into.
- Iteration 121 extracted CLI public evidence record summary, artifact, aggregation, and
  supporting-record parsing helpers into `cli/record_evidence.rs`, leaving the parent CLI module
  focused on dispatch.
- Iteration 122 extracted CLI command description rendering into `cli/descriptions.rs`, preserving
  the public `describe_cli_command` export while reducing the parent CLI module to parser and dispatch
  orchestration.
- Iteration 123 extracted CLI command execution into `cli/execution.rs`, moving status and
  public-evidence output dispatch out of the parser module while preserving the public execution API.
- Iteration 124 replaced the hand-rolled CLI parser with a typed `clap` command tree in
  `cli/parser.rs`, making the binary parse directly through `TvmdCli::parse()` instead of preserving the
  old string-slice parser API.
- Iteration 125 extracted the `TvmdCommand` data model into `cli/commands.rs`, leaving `cli.rs` as a
  small facade over command definitions, clap parsing, descriptions, execution, and evidence helpers.
- Iteration 126 split the clap public-evidence command tree into `cli/public_evidence_parser.rs` and
  moved shared clap value parsers into `cli/parser_values.rs`, leaving `cli/parser.rs` focused on the
  top-level local node, service, and public-testnet command structure.
- Iteration 127 moved the local miner, validator, proposer, service, local-testnet, and local-cpu
  clap command groups into `cli/local_parser.rs`, leaving `cli/parser.rs` as the top-level clap
  router plus public-testnet preflight parser.
- Iteration 128 moved local miner, validator, proposer, service, local-testnet, and local-cpu
  reference command execution into `cli/local_execution.rs`, leaving `cli/execution.rs` focused on
  dispatch plus public-evidence evidence generation.
- Iteration 129 moved public-evidence and public-testnet reference command execution into
  `cli/public_evidence_execution.rs`, making `cli/execution.rs` a small command-family dispatcher.
- Iteration 130 split CLI command descriptions into `cli/local_descriptions.rs` and
  `cli/public_evidence_descriptions.rs`, leaving `cli/descriptions.rs` as the exported
  command-family description dispatcher.
- Iteration 131 moved public-evidence service health/content clap argument structs and command
  conversion into `cli/public_evidence_service_parser.rs`, narrowing `cli/public_evidence_parser.rs`
  toward public-evidence family routing.
- Iteration 132 moved public-evidence record summary/artifact clap argument structs and command
  conversion into `cli/public_evidence_record_parser.rs`, and replaced the standalone explorer's
  hand-rolled argv scanner with explicit `tensorvm-explorer serve` and `health-check` clap commands.
- Iteration 133 moved public-evidence network-observation clap argument structs and command conversion
  into `cli/public_evidence_network_parser.rs`, continuing to shrink `cli/public_evidence_parser.rs`
  toward a command-family router.
- Iteration 134 moved public-evidence publication and auditor-record clap argument structs and command
  conversion into `cli/public_evidence_publication_parser.rs`, keeping publication CLI shape beside its
  evidence command family.
- Iteration 135 moved public-evidence run-window clap argument structs and command conversion into
  `cli/public_evidence_run_window_parser.rs`, keeping run-duration parser shape beside run-window evidence.
- Iteration 136 moved public-evidence node heartbeat and operator-attestation clap argument structs and
  command conversion into `cli/public_evidence_node_parser.rs`, leaving `cli/public_evidence_parser.rs`
  as a thin public-evidence command router.
- Iteration 137 moved public-evidence service health/content command execution into
  `cli/public_evidence_service_execution.rs`, narrowing `cli/public_evidence_execution.rs` toward a
  command-family dispatcher.
- Iteration 138 moved public-evidence record summary/artifact command execution into
  `cli/public_evidence_record_execution.rs`, keeping record-root aggregation orchestration beside the
  record command family instead of in the parent public-evidence dispatcher.
- Iteration 139 moved public-evidence network-observation command execution into
  `cli/public_evidence_network_execution.rs`, isolating service-log reading and network observation
  evidence dispatch from the parent public-evidence command router.
- Iteration 140 moved public-evidence publication and auditor-record command execution into
  `cli/public_evidence_publication_execution.rs`, keeping publication proof dispatch out of the
  parent public-evidence command router.
- Iteration 141 moved public-evidence run-window command execution into
  `cli/public_evidence_run_window_execution.rs`, isolating block-observation file dispatch from the
  parent public-evidence command router.
- Iteration 142 moved public-evidence node heartbeat and operator-attestation command execution into
  `cli/public_evidence_node_execution.rs`, keeping node observation file dispatch beside the node
  evidence command family instead of in the parent public-evidence router.
- Iteration 143 moved public-evidence service health/content descriptions into
  `cli/public_evidence_service_descriptions.rs`, starting the public-evidence description split along
  the same command-family boundaries as the clap parser and execution modules.
- Iteration 144 moved public-evidence record summary/artifact descriptions into
  `cli/public_evidence_record_descriptions.rs`, continuing the public-evidence description split along
  command-family boundaries already used by parser and execution modules.
- Iteration 145 replaced the legacy flat command adapter with the `clap` command tree itself,
  making parsed commands the execution and description model instead of preserving a parallel CLI
  representation.
- Iteration 146 moved public-evidence network-observation descriptions into
  `cli/public_evidence_network_descriptions.rs`, keeping network CLI text beside the network parser
  and execution command-family modules.
- Iteration 147 moved public-evidence publication and auditor-record descriptions into
  `cli/public_evidence_publication_descriptions.rs`, aligning publication CLI text with its parser
  and execution command-family modules.
- Iteration 148 moved public-evidence run-window descriptions into
  `cli/public_evidence_run_window_descriptions.rs`, keeping run-window CLI text beside its parser
  and execution command-family modules.
- Iteration 149 moved public-evidence node heartbeat and operator-attestation descriptions into
  `cli/public_evidence_node_descriptions.rs`, leaving the parent public-evidence description module
  as a small dispatcher plus validate/preflight text.
- Iteration 150 moved local miner, validator, and proposer command descriptions into
  `cli/local_role_descriptions.rs` with shared identity-seed rendering in
  `cli/local_description_values.rs`, narrowing `cli/local_descriptions.rs` to local command-family
  dispatch plus service/local-testnet/local-cpu text.
- Iteration 151 moved local service command descriptions into
  `cli/local_service_descriptions.rs`, leaving `cli/local_descriptions.rs` as a small local
  command-family dispatcher plus the tiny local-testnet and local-cpu descriptions.
- Iteration 152 moved local miner, validator, and proposer command execution into
  `cli/local_role_execution.rs` with shared identity report rendering in
  `cli/local_execution_values.rs`, narrowing `cli/local_execution.rs` toward service/local-testnet
  command execution and family dispatch.
- Iteration 153 moved local service command execution into `cli/local_service_execution.rs`,
  leaving `cli/local_execution.rs` as a compact local command-family dispatcher plus the tiny
  local-testnet and local-cpu execution handlers.
- Iteration 154 moved local miner, validator, and proposer clap command/argument structs into
  `cli/local_role_parser.rs`, aligning role parsing with the existing role description and
  execution modules while narrowing `cli/local_parser.rs` toward service/local-testnet/local-cpu
  clap shape.
- Iteration 155 moved local service clap command/argument structs into
  `cli/local_service_parser.rs`, leaving `cli/local_parser.rs` as the compact owner for shared
  local data-dir arguments plus local-testnet/local-cpu clap shape.
- Iteration 156 rewrote the local `tvmd` clap argument model around shared runtime argument groups,
  parse-time socket/multiaddr validation, and env/default-backed operator settings, so role and service
  commands no longer require every runtime flag on every invocation.
- Iteration 157 moved the CLI network-observation address-filter test into
  `cli/tests/network_observation.rs`, starting the split of the large CLI test module along focused
  behavior-family boundaries.
- Iteration 158 moved local CLI validation and CUDA-readiness tests into
  `cli/tests/local_validation.rs`, separating local command validation from the public-evidence
  invalid-argument coverage that remains in the parent CLI test module.
- Iteration 159 moved CLI public-evidence and public-testnet manifest report tests into
  `cli/tests/manifest_reports.rs`, keeping manifest report assertions separate from parser and command
  execution coverage while shrinking the parent CLI test module.
- Iteration 160 moved CLI command-description coverage into `cli/tests/command_descriptions.rs`,
  separating clap parsing/default tests from description snapshot assertions and further shrinking the
  parent CLI test module.
- Iteration 161 moved the remaining documented clap parser/default/rejection tests into
  `cli/tests/parser.rs`, leaving the parent CLI test module focused on shared fixtures and command
  execution behavior.
- Iteration 162 moved the positive CLI command execution/report coverage into
  `cli/tests/execution_reports.rs`, isolating ready-path command output assertions from the remaining
  public-evidence rejection coverage.
- Iteration 163 moved public-evidence CLI rejection coverage into
  `cli/tests/public_evidence_rejections.rs`, leaving `cli/tests.rs` as shared CLI test fixtures and
  module wiring instead of a mixed inline test body.
- Iteration 164 moved public-operator matching and network-runtime observation helper tests out of
  `testnet.rs` into `testnet/tests/network_runtime.rs`, starting the split of the large inline
  testnet test module along evidence-domain boundaries.
- Iteration 165 moved local testnet bootstrap and matmul/linear round tests into
  `testnet/tests/local_harness.rs`, separating local harness behavior from public evidence evaluation
  coverage in the large testnet test module.
- Iteration 166 moved the public-run independent external operator criteria test into
  `testnet/tests/run_evidence.rs`, further separating public evidence rule coverage from the shared
  testnet fixture module.
- Iteration 167 moved the public-run deployed service and production runtime criteria test into
  `testnet/tests/run_services.rs`, keeping service-reachability rule coverage out of the shared
  fixture module.
- Iteration 168 moved the public evidence bundle publication and auditor-record coverage into
  `testnet/tests/evidence_bundle.rs`, separating bundle-level evidence assertions from the remaining
  manifest and deployment-template tests.
- Iteration 169 moved the public evidence manifest parse, docs-example, deployed-example, and
  malformed-input tests into `testnet/tests/evidence_manifest.rs`, leaving preflight and deployment
  template coverage as the remaining inline testnet groups.
- Iteration 170 moved the public deployment scaffold, runbook, and README assertions into
  `testnet/tests/deployment_docs.rs`, separating deployment artifact documentation checks from the
  preflight manifest parser coverage.
- Iteration 171 moved the public preflight manifest readiness, pending-example, and malformed-input
  tests into `testnet/tests/preflight_manifest.rs`, leaving only the short unsigned/short-lived run
  evidence filter test inline in `testnet.rs`.
- Iteration 172 moved the remaining unsigned and short-lived public run evidence filter test into
  `testnet/tests/run_evidence.rs`, leaving `testnet.rs` with shared test fixtures and child-module
  wiring instead of inline test cases.
- Iteration 173 renamed the clap command surface to `TvmdCli`/`TvmdCommand`, replaced the
  legacy `execute_reference_cli_command` export with `execute_cli_command`, and renamed CLI test
  adapters to explicit command fixtures instead of preserving the old reference-command terminology.
- Iteration 174 moved public HTTPS endpoint, content-addressed evidence URI, and public network
  multiaddr validation helpers into `testnet/public_urls.rs`, separating public-address policy from
  the remaining public-testnet evidence and manifest orchestration code.
- Iteration 175 moved public-operator independence matching, quota search, and attestation-key
  derivation into `testnet/public_operators.rs`, keeping public-run criteria evaluation separate
  from the address/operator matching policy.
- Iteration 176 moved public evidence record kinds, signing/message-domain helpers, record-root
  aggregation, and generated network-runtime observation evidence into
  `testnet/public_evidence_crypto.rs`, separating public-evidence cryptographic framing from the
  main testnet orchestration module.
- Iteration 177 moved public preflight manifest parsing and launch-readiness manifest assembly into
  `testnet/public_preflight_manifest.rs`, separating preflight input handling from public evidence
  bundle manifest parsing.
- Iteration 178 moved shared public manifest field validation, scalar parsing, hash decoding, and
  required-field helpers into `testnet/public_manifest_fields.rs`, letting both public manifest
  parsers depend on one small parsing utility module instead of the main testnet orchestration file.
- Iteration 179 moved public evidence manifest parsing, record-line decoders, and evidence bundle
  manifest assembly into `testnet/public_evidence_manifest.rs`, leaving `testnet.rs` focused on
  evidence data types and orchestration logic rather than key-value parser state.
- Iteration 180 moved public evidence bundle construction, report evaluation, record signature
  checks, auditor validation, operator attestation matching, and network observation aggregation into
  `testnet/public_evidence_bundle.rs`, separating bundle-level evidence policy from the main
  testnet module.
- Iteration 181 moved public run evidence criteria evaluation, deployed service reachability checks,
  and service content uniqueness validation into `testnet/public_run_evidence.rs`, keeping public
  run policy separate from local testnet orchestration.
- Iteration 182 moved local testnet bootstrapping, participant endpoint validation, local round
  execution, telemetry/explorer adapters, and block finalization helpers into
  `testnet/local_harness.rs`, separating local orchestration from public testnet evidence types.
- Iteration 183 moved public preflight service-plan readiness checks and launch-readiness evaluation
  into `testnet/public_preflight_plan.rs`, keeping preflight policy beside but separate from
  preflight manifest parsing.
- Iteration 184 moved public service kind, endpoint, signed health evidence, and content evidence
  validation into `testnet/public_services.rs`, separating service proof policy from the remaining
  public evidence bundle and node evidence types.
- Iteration 185 moved production libp2p runtime evidence and signed runtime observation proof checks
  into `testnet/public_network_runtime.rs`, leaving the top-level testnet module less coupled to
  peer ID, multiaddr, and network-runtime validation details.
- Iteration 186 moved public evidence publication, auditor-record, and supporting-artifact proof
  validation into `testnet/public_evidence_publication.rs`, separating evidence publication policy
  from node/operator liveness evidence.
- Iteration 187 moved public node role, signed heartbeat evidence, and operator identity attestation
  validation into `testnet/public_node_evidence.rs`, leaving the top-level testnet module focused on
  shared public-testnet structs and aggregate report shapes.
- Iteration 188 moved public evidence and preflight manifest text/signature fixture builders into
  `testnet/tests/manifest_fixtures.rs`, reducing the parent test module to shared run/bundle fixture
  constructors and test module wiring.
- Iteration 189 moved shared public run, service, network-observation, and evidence-bundle fixture
  constructors into `testnet/tests/run_fixtures.rs`, leaving `testnet.rs` with production types,
  shared helpers, and test module declarations.
- Iteration 190 split the `tvmd` clap command definitions into `cli/local_commands.rs`,
  `cli/public_evidence_commands.rs`, and `cli/command_values.rs`, leaving `cli/commands.rs` as the
  top-level `TvmdCli`/`TvmdCommand` facade.
- Iteration 191 extracted the RPC HTTP server, response rendering, request parser, and query-token
  decoding into `rpc/http.rs`, leaving `rpc.rs` less coupled to transport framing while preserving the
  public `RpcHttpServer` and `http_response_text` exports.
- Iteration 192 extracted read-only explorer DTO projection helpers into `rpc/explorer.rs`, keeping
  account, block, miner, validator, receipt, job, and overview shaping separate from RPC route dispatch.
- Iteration 193 extracted websocket handshake, frame encoding/decoding, accept-key hashing, and
  websocket command-field parsing into `rpc/websocket.rs`, leaving `rpc.rs` closer to route dispatch
  and application state handling.
- Iteration 194 extracted job JSON, tensor numeric-array JSON, and the small telemetry/faucet HTML
  renderers into `rpc/render.rs`, keeping presentation string assembly out of RPC route logic.
- Iteration 195 moved `RpcPolicy` and `RpcGateway` auth/body-limit/rate-limit handling into
  `rpc/gateway.rs`, preserving the public RPC facade while separating gateway policy from route logic.
- Iteration 196 moved tensor descriptor, row, chunk, opening, latest, and tensor lookup route helpers
  into `rpc/tensor_routes.rs`, separating tensor data serving from the main RPC route dispatcher.
- Iteration 197 moved mutable RPC transaction, receipt-reference, attestation-reference, and faucet
  claim handlers into `rpc/mutations.rs`, isolating chain/txpool mutation paths from read routing.
- Iteration 198 moved chain block, receipt, job, miner, validator, faucet-status, and health read
  route helpers into `rpc/read_routes.rs`, narrowing `rpc.rs` toward dispatch and shared response helpers.
- Iteration 199 moved explorer account/latest collection routes and explorer websocket command
  handling into `rpc/explorer_routes.rs`, keeping explorer transport responses out of the RPC facade.
- Iteration 200 moved RPC hash/address parsing helpers into `rpc/parse.rs`, so route modules no
  longer depend on utility code living in the main RPC facade.
- Iteration 201 moved RPC request/response structs into `rpc/types.rs` and re-exported them from
  the facade, preserving the public API while reducing root-module responsibility.
- Iteration 202 moved shared RPC response constructors into `rpc/response.rs`, keeping status/body
  formatting centralized without leaving helper plumbing in the facade.
- Iteration 203 moved RPC dispatch and dynamic route matching into `rpc/dispatch.rs`, leaving the
  facade centered on node state construction and public re-exports.
- Iteration 204 moved `RpcNode` state, constructors, tensor registry access, and synthetic-round
  helpers into `rpc/node.rs`, reducing the facade to module wiring plus tests.
- Iteration 205 moved core RPC route, mutation, malformed-request, and receipt/status tests into
  `rpc/tests/routes.rs`, starting the RPC test split while keeping shared fixtures in the parent test module.
- Iteration 206 moved RPC HTTP parser/server, gateway policy, and response-format tests into
  `rpc/tests/http.rs`, continuing to reduce the RPC facade's inline test surface.
- Iteration 207 moved RPC explorer websocket, frame, and websocket JSON/query helper tests into
  `rpc/tests/websocket.rs`, leaving only tensor and synthetic-round tests inline in the facade.
- Iteration 208 moved the remaining RPC tensor route and synthetic-round tensor retention tests into
  `rpc/tests/tensors.rs`, leaving `rpc.rs` as RPC module wiring plus shared test-module imports.
- Iteration 209 moved the shared RPC test harness imports and child-module wiring into `rpc/tests.rs`,
  leaving the RPC facade as production module wiring and public re-exports only.
- Iteration 210 moved explicit chain test-state mutation helpers into `chain/test_helpers.rs`, keeping
  consensus-bypass fixture code out of the chain facade while preserving crate-test-only access.
- Iteration 211 moved the large `ChainCommand`/`ChainEvent` command-boundary test into
  `chain/tests/commands.rs`, starting the chain facade's inline test split around command ownership.
- Iteration 212 moved settlement, quorum, attestation, and conflicting-linear-root tests into
  `chain/tests/settlement.rs`, keeping receipt-settlement behavior out of the chain facade test module.
- Iteration 213 moved block production, proposer/finality, block-root, and reward-block tests into
  `chain/tests/blocks.rs`, so the block-specific proof-of-work helpers now live with block tests.
- Iteration 214 moved account, transaction, operator-root, parameter, and boundary rejection tests into
  `chain/tests/boundaries.rs`, leaving the chain facade test module focused on model/challenge edges.
- Iteration 215 moved the chain test harness root into `chain/tests.rs`, leaving `chain.rs` with only
  production facade code plus `#[cfg(test)] mod tests;` wiring.
- Iteration 216 moved the remaining model-transition and challenge-outcome tests into focused
  `chain/tests/models.rs` and `chain/tests/challenges.rs` modules, leaving the test root as prelude wiring.
- Iteration 217 moved attestation rejection, duplicate, availability, and assignment tests into
  `chain/tests/attestations.rs`, narrowing `chain/tests/settlement.rs` to settlement/quorum flows.
- Iteration 218 moved reward allocation and reward-block failure tests into `chain/tests/rewards.rs`,
  keeping `chain/tests/blocks.rs` focused on block proposal, admission, finality, and canonical roots.
- Iteration 219 moved proposer selection and validation-seed tests into `chain/tests/proposers.rs`,
  narrowing `chain/tests/blocks.rs` to block production, finality, vote admission, and root commitments.
- Iteration 220 moved chain transaction application and reference-submission rejection tests into
  `chain/tests/transactions.rs`, reducing boundary tests to account/job, parameter, root, and rejection edges.
- Iteration 221 moved account/job tracking, chain parameter retention math, and miner root commitment tests
  into focused modules, leaving `chain/tests/boundaries.rs` for rejection-boundary coverage only.
- Iteration 222 split the remaining rejection-boundary scenario into registration/transfer, receipt,
  attestation, block-vote, and model/challenge tests, preserving coverage while localizing failures.
- Iteration 223 moved public evidence and preflight manifest fixture builders into
  `cli/tests/manifest_fixtures.rs`, reducing the CLI test harness root to command fixtures and module wiring.
- Iteration 224 moved the then-existing CLI command fixture enum, parser adapter,
  execution/description adapters, and command conversion helpers into a dedicated CLI test helper
  module, leaving `cli/tests.rs` as shared imports and child-module wiring.
- Iteration 225 split local runtime/service execution-report assertions into
  `cli/tests/local_execution_reports.rs`, leaving public-evidence output assertions in
  `cli/tests/execution_reports.rs`.
- Iteration 226 moved public-evidence supporting-record summary, artifact, file-derived aggregation, and
  malformed-record report assertions into `cli/tests/public_evidence_record_reports.rs`, leaving
  `cli/tests/execution_reports.rs` focused on publication, run-window, node, service, and network output
  assertions.
- Iteration 227 moved `tvmd` public-testnet preflight and public-evidence manifest integration tests into
  `tests/tvmd_cli/public_evidence.rs`, leaving `tests/tvmd_cli.rs` focused on local service and role-runtime
  process flows plus shared integration helpers.
- Iteration 228 moved the `tvmd` service lifecycle integration test into
  `tests/tvmd_cli/service_lifecycle.rs`, keeping service init/peer/readiness/serve/public-surface evidence
  checks together while reducing the root `tvmd_cli.rs` integration harness to shared helpers and role flows.
- Iteration 229 moved public-evidence service health/content invalid-argument coverage into
  `cli/tests/public_evidence_service_rejections.rs`, separating service evidence rejection cases from the
  remaining publication, node, network, and supporting-record rejection coverage.
- Iteration 230 moved public-evidence publication and auditor-record invalid-argument coverage into
  `cli/tests/public_evidence_publication_rejections.rs`, keeping publication-bound URI/signature/auditor
  edge cases beside that command family and shrinking the remaining mixed rejection module.
- Iteration 231 moved public-evidence run-window invalid-argument and observation-file parser coverage into
  `cli/tests/public_evidence_run_window_rejections.rs`, leaving node/operator, network, parser, and
  supporting-record rejection coverage in the mixed module for subsequent splits.
- Iteration 232 replaced the Clap wrapper around the old CLI surface with an ergonomic command tree:
  `tvmd public ...` now owns preflight and evidence generation, `tvmd localnet ...` owns local CPU checks,
  manifest paths are positional arguments, and deployment docs/scripts/tests no longer invoke the retired
  `public-evidence`, `public-testnet`, `local-testnet`, or `local-cpu` top-level commands.
- Iteration 233 moved service/testnet command helpers and shared local CPU identity/seed helpers out of the
  binary-only `src/main/*` module tree into a library-owned `app` module, starting the `main.rs` collapse
  without mixing that ownership change with the larger runtime loop migration.
- Iteration 234 moved service status and block-status report builders into the same library-owned `app`
  module, leaving the binary to call application reporting APIs instead of owning the status formatter
  modules directly.
- Iteration 235 moved runtime role/config construction, wallet registration helpers, and role-service
  command config structs into the library-owned `app` module, so the remaining binary runtime modules depend
  on an application config API rather than a binary-private `runtime_config` owner.
- Iteration 236 moved role runtime status snapshot construction, runtime report formatting, and
  `role-runtime.status` writing into the library-owned `app` module, keeping runtime reporting with the
  app-owned command/status APIs instead of binary-private formatter modules.
- Iteration 237 moved runtime service startup, including node-store loading, libp2p launch, RPC binding,
  and p2p report metadata, into the library-owned `app` module so the runtime loop consumes an app startup
  API instead of another binary-private service module.
- Iteration 238 moved runtime network ingestion, p2p announcement publishing, and scheduled local synthetic
  production helpers into the library-owned `app` module, leaving role workers and the runtime loop to call
  app-level network/runtime helpers instead of binary-private network modules.
- Iteration 239 moved the runtime RPC serving helper into the library-owned `app` module, so request serving,
  mutation detection, and service-state persistence are exposed as an app runtime helper instead of another
  binary-private loop dependency.
- Iteration 240 moved validator role observation/submission helpers and remote tensor-fetch helpers into the
  library-owned `app` module, leaving the validator runtime worker to orchestrate app-level validator role
  APIs instead of owning validator-specific helper modules in the binary tree.
- Iteration 241 moved miner role work observation, receipt submission, and miner runtime ticking into the
  library-owned `app` module, removing another binary-private role worker while keeping `tvmd` runtime-loop
  code focused on orchestration.
- Iteration 242 moved validator runtime ticking into the library-owned `app` module, so both miner and
  validator role workers now sit behind app APIs and the `tvmd` runtime loop no longer depends on a
  binary-private validator worker module.
- Iteration 243 moved `RoleRuntimeLoop` into the library-owned `app` module, making the binary runtime
  entrypoint a thin command wrapper over an app-owned loop instead of the owner of service, RPC, network,
  production, and role-tick orchestration.
- Iteration 244 moved miner, validator, and proposer role-service runners into the library-owned `app`
  module, replacing the binary-private `RoleRunLoop` wrapper with an app-level `RoleServiceRunner` API and
  leaving `tvmd` to translate parsed Clap commands into app calls.
- Iteration 245 moved the service runtime wrapper into the library-owned `app` module, removing the last
  production module from the `src/main/*` tree so `tvmd` now owns Clap parsing and dispatch while app APIs
  own service startup and runtime execution.
- Iteration 246 moved the `tvmd` command-dispatch match into the library-owned `app` module as
  `execute_tvmd_command`, leaving `main.rs` as a minimal Clap parse/execute/print entrypoint instead of the
  owner of command-family routing and manifest-file handling.
- Iteration 247 moved the remaining `tvmd` runtime harness out of the binary unit-test module and into a
  `tvmd_runtime` integration-test target, removing the final `main.rs` module hook now that the runtime
  surface is library-owned.
- Iteration 248 routed `tvmd_runtime` miner and validator registration setup through `ChainCommand`
  helpers, removing another runtime-facing integration-test dependency on direct `Chain::register_*`
  mutation methods outside the chain module.
- Iteration 249 routed block-log storage fixtures through `ChainCommand`-backed registration and block
  production helpers, removing direct `Chain::register_*` and `Chain::produce_block` setup from that
  storage codec test surface.
- Iteration 250 extracted shared storage test helpers for command-backed block-producer setup and moved
  snapshot storage fixtures onto them, extending the command-boundary cleanup across another storage
  persistence test surface.
- Iteration 251 routed the remaining `tvmd_runtime` block-production setup through a `ChainCommand`
  helper, removing direct `Chain::produce_block` calls from the runtime integration harness.
- Iteration 252 reused the command-backed storage test helpers in simple node-store persistence fixtures,
  removing another set of direct registration and block-production setup calls from storage tests.
- Iteration 253 expanded the storage command test helpers and routed the command-covered portions of the
  node-store durable-chain fixture through them, reducing direct fixture calls for validator registration,
  transfers, jobs, receipts, attestations, model registration, block production, and block votes.
- Iteration 254 reused those storage command helpers in the chain-state durable fixture and added a
  `CreditReward` helper, aligning the chain-state persistence fixture with the node-store command-backed
  setup path.
- Iteration 255 routed scheduler assignment test registration setup through local `ChainCommand` helpers,
  removing direct miner and validator registration calls from that test surface.
- Iteration 256 reconciled the deep-dive CLI guidance with the completed Clap rewrite: `tvmd` now owns an
  ergonomic typed Clap command tree, and the old top-level compatibility commands are no longer preserved.
- Iteration 257 added a shared CLI test helper for parsing `key=value` reports and converted local
  runtime report assertions from substring checks to field-level expectations.
- Iteration 258 reused the parsed CLI report helper in manifest report tests, replacing another
  substring-heavy status assertion surface with explicit field checks.
- Iteration 259 reused the parsed CLI report helper for CUDA miner readiness assertions, shrinking the
  remaining generated status substring checks in local CLI validation coverage.
- Iteration 260 reused the parsed CLI report helper for public-evidence publication output assertions,
  removing another generated `key=value` substring check from CLI execution coverage.
- Iteration 261 added structured comma-record assertions for public service health/content evidence lines,
  reducing broad substring checks in CLI execution report coverage.
- Iteration 262 moved `evidence service content-bytes --content-hex` decoding into the Clap value parser,
  so command execution receives validated bytes and retired top-level command families stay rejected.
- Iteration 263 converted public node heartbeat evidence tests from prefix/suffix checks to parsed
  comma-record field assertions, matching the service evidence coverage style.
- Iteration 264 reused parsed integration-test stdout fields for the local testnet seed report, replacing
  broad substring checks with keyed numeric and boolean assertions.
- Iteration 265 converted public evidence/preflight integration checks to parsed stdout fields, removing
  substring assertions from those report-status tests.
- Iteration 266 converted service lifecycle init, peer-add, readiness, and serve report checks to keyed
  stdout assertions with numeric parsing for counters and libp2p limits.
- Iteration 267 added comma-record parsing to the `tvmd_cli` integration harness and used it for public
  service health/content evidence generated from live service responses.
- Iteration 268 converted local service gateway serve/status integration checks to parsed stdout fields,
  replacing another cluster of `key=value` substring assertions.
- Iteration 269 converted the validator local-producer integration report checks to keyed stdout fields,
  including produced-block counters.
- Iteration 270 made the service peer command description test exact by formatting the generated peer ID,
  removing the last substring assertion from CLI command description coverage.
- Iteration 271 converted role-run integration stdout checks to parsed fields, including duplicate command
  handling for the embedded `service_serve` report and numeric runtime/network counters.
- Iteration 272 added direct `serde_json` test parsing for RPC responses and converted the head, health,
  RPC health, and block route checks from raw substring matching to typed JSON field assertions.
- Iteration 273 reused parsed RPC JSON assertions for the current-job and job-lookup routes, checking
  exact job IDs, primitive types, tensor dimensions, training shapes, and deadlines.
- Iteration 274 converted the explorer, telemetry, and faucet RPC route checks to parsed JSON assertions,
  leaving only the intentionally HTML page responses as content checks.
- Iteration 275 converted the RPC receipt lookup route test to parsed JSON assertions for receipt ID,
  job ID, and tensor-work units while preserving the HTTP status-text edge coverage.
- Iteration 276 moved the RPC JSON test helpers to the shared RPC test module and converted the
  `/tensor/latest` synthetic-round assertion to parsed tensor-count and hash-field checks.
- Iteration 277 converted explorer websocket collection and error-response tests from substring scans to
  parsed JSON assertions over response types, hardware classes, primitive types, receipt fields, and errors.
- Iteration 278 reused the shared RPC JSON parser in the HTTP parser coverage for direct explorer websocket
  overview/account responses, replacing another pair of substring checks.
- Iteration 279 removed the runtime dependency on the preserved CLI execution facade: `tvmd` now parses
  through the Clap command tree, dispatches app commands directly, keeps public-evidence generation behind
  an explicit evidence executor, and moves role/service argument validation into the app boundary.
- Iteration 280 converted the RPC HTTP transport tests from response substring checks to explicit
  status-line, header, body, and websocket-frame JSON assertions.
- Iteration 281 reused parsed HTTP response helpers in the `tvmd_cli` integration harness, replacing
  live service status and JSON substring checks with exact status-line and `serde_json` assertions.
- Iteration 282 added shared keyed-report parsing to the `tvmd_runtime` integration harness and converted
  service command report checks from substring scans to field-level assertions.
- Iteration 283 removed the remaining `tvmd_runtime` report/body substring assertions, reusing keyed
  report parsing, exact HTTP status-line checks, and parsed tensor JSON counts.
- Iteration 284 converted the `tvmd_cli` role-run service-status checks from broad `role_*` substring
  scans to keyed field, numeric, boolean, hash, and list assertions.
- Iteration 285 converted service-lifecycle network evidence checks from prefix/substring scans to parsed
  comma-record, keyed summary, artifact, and exact CLI error assertions.
- Iteration 286 converted the remaining `tvmd_cli` service-block report substring checks to keyed command,
  hash, numeric, receipt-count, PoW, and canonical-blockspace assertions.
- Iteration 287 added local compose/env structure helpers and moved local CPU operator, producer,
  explorer, wallet, volume, network, and env-file assertions off broad substring scans.
- Iteration 288 reused the local CPU deployment test structure helpers for Dockerfile, dockerignore, and
  CPU-only compose guards, replacing broad build-artifact substring scans with exact line assertions.
- Iteration 289 added shell logical-line parsing to the local CPU deployment test and replaced
  entrypoint substring checks with exact command/readiness-line assertions.
- Iteration 290 reused the shell logical-line assertions for restart and rolling-restart deployment
  scripts, replacing fragment inventories with exact continuity command and status-output checks.
- Iteration 291 moved the local CPU check script setup and seeded-state gate assertions to exact
  shell logical lines, shrinking the remaining broad inventory to live/status-network checks.
- Iteration 292 moved the local CPU check script live gateway, explorer, tensor, and ready-report
  assertions to exact shell logical lines, leaving the remaining inventory focused on operator status.
- Iteration 293 added generated exact assertions for local CPU check script `status_value` reads and
  replaced operator role/runtime readiness fragments with typed status-field checks.
- Iteration 294 replaced the final local CPU check script substring inventory with exact block-status,
  convergence, network-evidence, and ready-report shell-line assertions.
- Iteration 295 added exact trimmed-line assertions for public deployment env, systemd, and nginx
  templates, replacing the template substring inventory in the deployment-docs test.
- Iteration 296 reused deployment-docs trimmed-line assertions for the public runbook preflight gate
  and evidence command list, replacing another pair of runbook substring inventories.
- Iteration 297 converted the public deployment README scaffold-artifact and public-route checks to
  exact trimmed-line assertions, leaving only prose/operator-flow inventories in that test.
- Iteration 298 removed the test-only hand-written CLI description mirror, added direct Clap help
  coverage for the `tvmd` command tree and retired top-level families, and made the binary execute through
  the parsed Clap command object instead of extracting a command for a separate call.
- Iteration 299 converted the remaining public deployment runbook and README prose inventories to exact
  trimmed-line assertions, removing the final document-wide substring loops from deployment docs coverage.
- Iteration 300 replaced the API surface test's partial CLI command substring scans with exact route and
  command-list assertions, making the documented API surface coverage fail on drift instead of fragments.
- Iteration 301 added small HTML test parsers for route and process-level service tests, replacing raw
  rendered-page substring assertions with exact title, heading, definition-list, and JavaScript line checks.
- Iteration 302 replaced the remaining local CPU Compose artifact substring checks with exact trimmed-line
  helpers, including exact topology-service names in the spec, required Dockerfile/.dockerignore lines, and
  explicit absent NVIDIA runtime/device lines.
- Iteration 303 replaced telemetry snapshot's hand-formatted JSON string builder with `serde_json`
  serialization and converted its remaining JSON substring checks to parsed field assertions.
- Iteration 304 replaced explorer WebSocket command substring routing with `serde_json` command parsing,
  preserving shorthand commands while removing the test-only hand-written JSON field extractors.
- Iteration 305 added a shared exact comma-field parser for CLI evidence records and moved network,
  node-heartbeat, service-health, and run-window observation parsers off repeated raw split/count logic.
- Iteration 306 reused the shared exact comma-field parser for public supporting-record payloads, removed
  the local split/count helper, and made empty/whitespace field rejection consistent across CLI evidence
  record parsers.
- Iteration 307 centralized public-testnet manifest line scanning, comment skipping, key whitespace
  rejection, and duplicate scalar-field checks so evidence and preflight manifests no longer maintain
  parallel `key=value` parser loops.
- Iteration 308 introduced an explicit transaction-body parser for txpool envelopes, moving command
  dispatch, argument extraction, and extra-token rejection out of the raw whitespace-scanner flow while
  preserving the existing transaction wire format.
- Iteration 309 centralized RPC HTTP request-line parsing so the socket HTTP parser and in-memory
  `handle_http_text` test/dispatch path share one method/path extraction boundary instead of maintaining
  parallel whitespace scanners.
- Iteration 310 replaced the retained top-level `tvmd` command families with a breaking Clap tree:
  `node`, `role`, `localnet`, and `public`, updating binary dispatch, deployment scripts, docs, and
  parser/help tests so old command names are rejected instead of carried as aliases.
- Iteration 311 introduced an owned role-service dispatch config so `tvmd` miner, validator, and proposer
  run dispatch share the same wallet/node/listen/data-dir/auth extraction path before validation and
  service launch.
- Iteration 312 removed the duplicate CLI public-network address filter and routed network evidence through
  the testnet-owned external multiaddr validator, moving the local/private-address edge coverage with that
  canonical owner.
- Iteration 313 deleted the test-only CLI identity-report renderer and reused the app-owned
  `p2p_identity_report` for local role and node readiness fixture output, leaving the identity seed status
  text with one formatter.
- Iteration 314 added a shared `KeyValueReport` parser for app-owned status/log text, replacing the
  separate service-status file scanner and CLI service-log field loop used by network evidence generation.
- Iteration 315 reused `KeyValueReport` in the crate-internal CLI report assertion helper, removing another
  bespoke `key=value` parser from local execution, manifest, and public-evidence report coverage.
- Iteration 316 added a shared integration-test report-field parser for `tvmd_cli` and `tvmd_runtime`,
  preserving repeated-field assertions while removing two more ad hoc stdout/status `key=value` scanners.
- Iteration 317 replaced local CPU verify JSON string formatting and manual escaping with typed
  `serde_json` serialization in both the app command and CLI fixture path, with process-level coverage
  parsing `tvmd localnet verify --json` as JSON.
- Iteration 318 removed `cargo test` execution and the Rust toolchain requirement from the local CPU
  deployment checker, keeping unit-test execution owned by CI instead of the shell readiness script.
- Iteration 319 reused the shared integration report parser for the local CPU env-file fixture checks,
  removing another one-off `KEY=value` scanner from the Compose artifact test.
- Iteration 320 added a shared integration-test comma-record parser and moved `tvmd_cli` service,
  network, and supporting-record evidence assertions off inline `split(',')` helpers.
- Iteration 321 routed crate-internal CLI evidence comma-record assertions through the same exact
  comma-field parser used by evidence readers, removing the last local `split(',')` helper from CLI
  report tests.
- Iteration 322 extracted fixed-size hash hex parsing into `types.rs`, leaving CLI arguments and public
  manifest parsing as domain-specific error wrappers instead of maintaining parallel 32-byte nibble loops.
- Iteration 323 moved CLI `--content-hex` byte decoding onto the same typed hex parser utilities, so
  fixed hashes and arbitrary byte arguments no longer maintain separate nibble decoders.
- Iteration 324 reused the shared hex nibble decoder from RPC query-token percent decoding, leaving URL
  parsing local while eliminating the last separate hex digit lookup table.
- Iteration 325 routed transaction-envelope hash token decoding through the shared fixed-hash parser while
  preserving the txpool wire format's exact 64-character token requirement.
- Iteration 326 routed RPC path hash decoding through the shared fixed-hash parser, preserving the route
  parser's exact 64-character segment requirement while removing its local hex loop.
- Iteration 327 moved `tvmd` hex hash, address, identity-seed, and byte payload CLI values onto typed
  Clap `FromStr` wrappers, keeping the Clap command tree as the argument boundary without resurrecting
  retired top-level command families.
- Iteration 328 centralized HTTPS authority slicing for public URL validation, so host extraction and
  authority comparison share the same scheme, control-character, and path-boundary handling.
- Iteration 329 extracted strict comma-field record parsing into a crate-private helper shared by CLI
  evidence readers and public manifest readers, removing another duplicated CSV-like validation loop.
- Iteration 330 moved RPC dynamic-route path splitting into a fixed-capacity parser helper, keeping
  route matching focused on method/segment dispatch instead of owning the trim/split logic inline.
- Iteration 331 moved RPC HTTP header parsing into a private header accumulator, preserving auth
  precedence while keeping request framing separate from individual header interpretation.
- Iteration 332 moved RPC job and tensor response JSON onto `serde_json::json!`, replacing manual array
  and object string assembly while expanding tensor route field assertions.
- Iteration 333 moved core RPC read, status, accepted, and error responses onto `serde_json::json!`, so
  route helpers no longer hand-build those JSON objects or own escaping details.
- Iteration 334 moved the faucet claim mutation response onto `serde_json::json!` and asserted the
  returned claim amount, account, and remaining balance, removing the last hand-built RPC JSON object.
- Iteration 335 added a shared key-value report writer beside the parser and moved p2p identity readiness
  reports onto it, starting to consolidate status rendering without changing the larger service-status shape.
- Iteration 336 moved local CPU verify key-value output, including the CLI fixture path, onto the shared
  report writer so typed verify structs no longer hand-format their text status form.
- Iteration 337 tightened the Clap command tree so incomplete nested command groups show their scoped help
  directly for the then-current node, role, localnet, and public command groups without reintroducing
  legacy aliases.
- Iteration 338 moved app-owned miner and validator registration, readiness, and status reports onto the
  shared key-value report writer, including device-readiness fields, with parseability coverage for the
  operator-check surface.
- Iteration 339 added a checked subreport append path to the key-value writer and moved local service CLI
  fixture reports onto it, so node init, peer, readiness, serve, status, and block fixture outputs no
  longer hand-format their status text.
- Iteration 340 moved local role CLI fixture reports for miner, validator, and proposer registration,
  checks, runs, and statuses onto the shared key-value report writer, including device and p2p subreports.
- Iteration 341 moved the remaining local CLI fixture seed report onto the shared key-value report writer,
  finishing the local role, service, and localnet fixture report-rendering cleanup.
- Iteration 342 promoted miner, validator, and proposer to first-class Clap subcommands, removing the
  old role grouping so operators use shorter `tvmd miner ...`, `tvmd validator ...`, and
  `tvmd proposer ...` commands.
- Iteration 343 moved the local CPU deployment checker's `tvmd localnet verify --json` readiness checks
  from substring greps to a real JSON boolean parser, keeping Docker orchestration in shell while reducing
  one policy-critical ad hoc parsing surface.
- Iteration 344 replaced the local CPU deployment checker's scalar JSON number and string extraction
  helpers with Python JSON readers, removing another sed/tr pipeline from live-chain and tensor-route
  policy checks.
- Iteration 345 replaced the local CPU deployment checker's receipt JSON field-count regex helpers with
  structured Python JSON traversal, so live receipt attestation and primitive-type policy checks no longer
  depend on grep over serialized JSON text.
- Iteration 346 moved standalone explorer health readiness and websocket URL checks from grepping JSON text
  onto the shared JSON helpers, leaving HTML page probes as text reachability checks.
- Iteration 347 replaced live tensor descriptor, row, chunk, and opening route greps in the local CPU
  checker with parsed JSON field assertions, including descriptor-root and chunk-index consistency checks.
- Iteration 348 changed local-testnet seed report checks from repeated greps over
  `local-testnet-seed.out` to one captured report per service with exact `status_value` field assertions.
- Iteration 349 changed `local-cpu-ready` readiness checks from repeated file greps to one captured
  readiness report per service, validating role, libp2p, identity, profile, and device fields exactly.
- Iteration 350 removed the local CPU checker's remaining `grep` dependency by replacing exact-line and
  explorer-page probes with shell string helpers.
- Iteration 351 removed the local CPU checker's remaining `sed` dependency by using direct service-file
  reads and a shell key-value field lookup for status reports.
- Iteration 352 removed `sed` key-value extraction from the local CPU restart-continuity checker, reusing
  one shell field reader for captured reports and persisted snapshot files.
- Iteration 353 removed the local CPU operator healthcheck's `grep` dependency by scanning
  `local-cpu-ready` with an exact shell line reader.
- Iteration 354 moved the local CPU operator topology list into one sourced deployment script so the
  check, restart, and rolling-restart gates no longer duplicate the service inventory.
- Iteration 355 derived local CPU checker service, miner, validator, settled-receipt, and CUDA-expected
  counts from the shared topology script instead of hard-coding those policy constants in the checker.
- Iteration 356 moved the local CPU checker's bootstrap service, network observer, seed height/block
  expectations, and full-rate basis-point constants into the shared topology script.
- Iteration 357 removed the last retired preflight and service-run command references from
  operator-facing docs and added a deployment-doc regression against retired top-level CLI families.
- Iteration 358 changed the local CPU checker's all-operator convergence gates to compare service
  status against topology-derived seed, operator-count, and settled-receipt expectations.
- Iteration 359 moved the local CPU checker's live TensorOp and LinearTrainingStep receipt evidence
  floor into the shared topology policy script.
- Iteration 360 moved the local CPU checker's live receipt query page limit into the shared topology
  policy script.
- Iteration 361 moved the local CPU checker's useful-PoW block scan depth into the shared topology
  policy script.
- Iteration 362 moved the local CPU checker's general retry and all-operator convergence retry limits
  into the shared topology policy script.
- Iteration 363 moved the local CPU checker's Docker exec timeout into the shared topology policy script.
- Iteration 364 moved the local CPU checker's retry sleep interval into the shared topology policy script.
- Iteration 365 moved the local CPU checker's curl HTTP timeout into the shared topology policy script.
- Iteration 366 moved the restart continuity checker's Docker exec timeout into the shared topology
  policy script.
- Iteration 367 moved the restart continuity checker's retry limit into the shared topology policy
  script.
- Iteration 368 moved the restart continuity checker's retry sleep interval into the shared topology
  policy script.
- Iteration 369 moved the restart continuity checker's seeded-height comparisons into the shared
  topology policy script.
- Iteration 370 moved the restart continuity checker's Docker restart command timeout into the shared
  topology policy script.
- Iteration 371 moved the restart continuity checker's full local CPU check timeout into the shared
  topology policy script.
- Iteration 372 moved the restart continuity checker's default restart service set into the shared
  topology policy script.
- Iteration 373 routed test-only CLI operator check fixtures through the production operator-check
  helpers instead of duplicating register/check/status report construction.
- Iteration 374 shared the test-only CLI libp2p fixture report fields across local role and service
  fixtures.
- Iteration 375 made parser tests receive the clap-produced `TvmdCommand` directly instead of
  round-tripping parsed commands back through `CommandFixture`.
- Iteration 376 added a direct `TvmdCommand` execution helper for CLI tests so execution suites can
  migrate off the parallel `CommandFixture` model incrementally.
- Iteration 377 migrated local CLI execution report tests to execute clap-parsed commands directly,
  removing the now-dead local CPU verify fixture variant.
- Iteration 378 migrated local CLI validation tests to execute clap-parsed commands directly instead
  of constructing local `CommandFixture` variants.
- Iteration 379 moved local CLI parser expectations onto direct `TvmdCommand` construction, removed
  the local command family from `CommandFixture`, and renamed the remaining public-evidence fixture
  variants without the redundant `Public` prefix.
- Iteration 380 renamed the remaining public-evidence-only CLI fixture type and executor so they no
  longer read like a parallel model for every CLI command.
- Iteration 381 moved public evidence manifest, publication, audit, and run-window parser
  expectations from `EvidenceFixture` to direct `TvmdCommand` construction, then removed the now-dead
  fixture variants and redundant `Evidence` prefixes from the remaining fixture variants.
- Iteration 382 moved public node heartbeat, heartbeat-file, and operator-attestation parser
  expectations from `EvidenceFixture` to direct `TvmdCommand` construction.
- Iteration 383 moved public service health/content parser expectations from `EvidenceFixture` to
  direct `TvmdCommand` construction.
- Iteration 384 moved public network observation parser expectations from `EvidenceFixture` to direct
  `TvmdCommand` construction.
- Iteration 385 moved public supporting-record parser expectations from `EvidenceFixture` to direct
  `TvmdCommand` construction and removed the remaining parser fixture equality adapter.
- Iteration 386 moved public publication, auditor, and run-window evidence report tests from
  `EvidenceFixture` to direct `EvidenceCommand` execution.
- Iteration 387 moved public node heartbeat, heartbeat-file, and operator-attestation report tests from
  `EvidenceFixture` to direct `EvidenceCommand` execution.
- Iteration 388 moved public service health/content report tests from `EvidenceFixture` to direct
  `EvidenceCommand` execution.
- Iteration 389 moved public network observation report tests from `EvidenceFixture` to direct
  `EvidenceCommand` execution and removed the now-dead service-log network fixture variant.
- Iteration 390 moved the public record-report suite's network observation setup from
  `EvidenceFixture` to direct `EvidenceCommand` execution.
- Iteration 391 moved public record summary and artifact report cases from `EvidenceFixture` to direct
  `EvidenceCommand` execution.
- Iteration 392 moved public record summary-roots and artifact-roots report cases from
  `EvidenceFixture` to direct `EvidenceCommand` execution.
- Iteration 393 moved public record summary-file and artifact-file report cases from
  `EvidenceFixture` to direct `EvidenceCommand` execution and removed the now-dead file-backed
  record fixture variants.
- Iteration 394 moved public publication and auditor rejection tests from `EvidenceFixture` to direct
  `EvidenceCommand` execution and removed the now-dead publication fixture variants.
- Iteration 395 moved public run-window rejection tests from `EvidenceFixture` to direct
  `EvidenceCommand` execution and removed the now-dead run-window fixture variants.
- Iteration 396 moved public node heartbeat and operator-attestation rejection tests from
  `EvidenceFixture` to direct `EvidenceCommand` execution and removed the now-dead node fixture
  variants.
- Iteration 397 moved public network observation rejection tests from `EvidenceFixture` to direct
  `EvidenceCommand` execution and removed the now-dead network fixture variant.
- Iteration 398 moved public record and service rejection tests from `EvidenceFixture` to
  direct `EvidenceCommand` execution and removed the obsolete fixture enum entirely.
- Iteration 399 removed the test-only public kind parser shims and covered invalid service
  and record kind inputs through clap parsing instead.
- Iteration 400 renamed the leftover `cli/arguments.rs` record-field helper module to
  `cli/evidence_fields.rs` and made hash/numeric helpers explicit field parsers.
- Iteration 401 renamed the CLI test command helper module away from fixture terminology now
  that tests parse and execute clap commands directly.
- Iteration 402 renamed the local libp2p CLI report helper module and functions away from
  fixture terminology and cleaned stale CLI test expectation wording.
- Iteration 403 moved proposer edge-case tests off direct `ChainState` field mutation by adding
  explicit crate-test-only `Chain` helpers for validator stake and miner tensor-work setup.
- Iteration 404 moved block finality and block-root tests off direct `ChainState`/`blocks`
  mutation by adding explicit crate-test-only helpers for injected votes, receipts, and blocks.
- Iteration 405 moved settlement orphan-attestation setup onto the crate-test-only `Chain`
  helper and switched adjacent settlement assertions to immutable state accessors.
- Iteration 406 moved command and transaction reward-claim test setup off direct reward-state
  mutation; the later delayed reward cleanup removed the interim helper and routes those tests through
  pending credit rewards plus mature release.
- Iteration 407 made the miner `--device` CLI flag a typed clap argument so invalid backend
  names are rejected at parse time instead of passing through as loose strings.
- Iteration 408 moved account, challenge, model-command, and reward test assertions onto
  `Chain`/`ChainState` accessors and used the existing model optimizer test helper for setup.
- Iteration 409 moved attestation tests off direct `ChainState` and `ChainParams` field reads,
  using the existing state and params accessors throughout the suite.
- Iteration 410 moved root-hash and block root/height assertions onto the existing chain
  accessors, removing another cluster of direct test reads from `Chain` internals.
- Iteration 411 moved boundary tests onto `Chain`/`ChainState`/`ChainParams` accessors so the
  last chain-test suite with broad direct field reads follows the same encapsulation boundary.
- Iteration 412 moved RPC route/explorer block reads and external RPC/testnet test parameter reads
  onto `Chain` accessors instead of reaching through crate-visible fields.
- Iteration 413 moved the production local testnet harness off direct `Chain.params` and
  `Chain.blocks` field reads, keeping profile/run evidence code behind the accessor boundary.
- Iteration 414 moved synthetic localnet production code and its unit tests off direct
  `Chain.params`/`Chain.blocks` reads, leaving the module on the same accessor path.
- Iteration 415 moved scheduler production reads onto `Chain::params()` and replaced scheduler
  test parameter mutations with explicit crate-test-only `Chain` helper methods.
- Iteration 416 moved the zero-work liveness study off direct `Chain.params` and `Chain.blocks`
  reads, clearing non-chain-module direct access to those fields.
- Iteration 417 tightened the rewritten clap CLI surface with root examples, argument help text,
  URL/file completion hints, hidden auth-token env values, and a command-model debug assertion.
- Iteration 418 split the public service evidence clap command family into
  `cli/public_evidence_service_commands.rs`, keeping service-specific args and value enums beside
  their command owner while preserving the public re-export surface.
- Iteration 419 split the public supporting-record clap command family into
  `cli/public_evidence_record_commands.rs`, moving record args and the record-kind value enum out
  of the mixed public evidence command module without changing external re-exports.
- Iteration 420 split the public libp2p network evidence clap command family into
  `cli/public_evidence_network_commands.rs`, isolating network args and libp2p-specific imports from
  the mixed public evidence command module.
- Iteration 421 split the public run-window evidence clap command family into
  `cli/public_evidence_run_window_commands.rs`, moving run-window args beside their subcommand owner.
- Iteration 422 split the public node/operator evidence clap command family into
  `cli/public_evidence_node_commands.rs`, moving heartbeat, operator-attestation, and node-role
  value types out of the mixed public evidence command module.
- Iteration 423 split public publication and auditor clap argument structs into
  `cli/public_evidence_publication_commands.rs`, leaving the mixed public evidence command module
  focused on top-level command routing and manifest path arguments.
- Iteration 424 split miner, validator, proposer, and shared role-runtime clap command types into
  `cli/local_role_commands.rs`, leaving `cli/local_commands.rs` focused on node, peer, runtime, and
  localnet argument groups.
- Iteration 425 split node and peer clap command types into `cli/local_node_commands.rs`, leaving
  `cli/local_commands.rs` focused on shared runtime, data-dir, and localnet argument groups.
- Iteration 426 split the localnet clap command and local CPU verify arguments into
  `cli/localnet_commands.rs`, leaving `cli/local_commands.rs` as the shared local argument module.
- Iteration 427 moved shared local runtime and data-dir clap arguments into
  `cli/local_runtime_args.rs`, turning `cli/local_commands.rs` into a thin re-export boundary for local
  command families.
- Iteration 428 extracted shared clap parser test constructors into `cli/tests/parser_support.rs`,
  shrinking the catch-all parser module and preparing command-family parser suites without duplicating
  argument fixtures.
- Iteration 429 moved local miner, validator, proposer, node, localnet, defaults, and invalid-argument
  parser coverage into `cli/tests/local_parser.rs`, leaving `cli/tests/parser.rs` focused on public
  command parsing and global retired-family rejection.
- Iteration 430 renamed the remaining public-focused clap parser suite to
  `cli/tests/public_parser.rs`, removing the last generic parser test module name after the local split.
- Iteration 431 moved the shared comma-record assertion helper out of the public evidence execution report
  suite and into `cli/tests/report_fields.rs`, preparing that oversized report test for family-level splits.
- Iteration 432 split publication and auditor public evidence execution report coverage into
  `cli/tests/public_evidence_publication_reports.rs`, reducing the monolithic execution report suite.
- Iteration 433 split public run-window execution report and observation-file equivalence coverage into
  `cli/tests/public_evidence_run_window_reports.rs`, continuing the family-level report test split.
- Iteration 434 split public node heartbeat, heartbeat-file, and operator-attestation execution report
  coverage into `cli/tests/public_evidence_node_reports.rs`, further reducing the mixed report suite.
- Iteration 435 split public service health/content execution report coverage into
  `cli/tests/public_evidence_service_reports.rs`, leaving the mixed report suite focused on network
  observation behavior.
- Iteration 436 renamed the remaining network-only execution report suite to
  `cli/tests/public_evidence_network_reports.rs`, removing the generic execution report test module.
- Iteration 437 split public network observation rejection coverage into
  `cli/tests/public_evidence_network_rejections.rs`, shrinking the remaining mixed public evidence
  rejection suite.
- Iteration 438 split public node heartbeat, heartbeat-file, and operator-attestation rejection coverage
  into `cli/tests/public_evidence_node_rejections.rs`, leaving the mixed rejection suite focused on
  service parser and record validation edges.
- Iteration 439 split public evidence record parser and direct validation rejection coverage into
  `cli/tests/public_evidence_record_rejections.rs`, leaving only service parser checks in the old
  mixed rejection module.
- Iteration 440 folded the remaining service parser rejection checks into
  `cli/tests/public_evidence_service_rejections.rs` and deleted the last mixed public evidence
  rejection module.
- Iteration 441 removed the test-only local CLI execution shims and routed CLI execution tests
  through the same `app::execute_tvmd_command` dispatcher used by the binary, leaving `cli` focused
  on the Clap command model and public evidence helpers.
- Iteration 442 split public publication and auditor Clap parser coverage into
  `cli/tests/public_evidence_publication_parser.rs`, narrowing the remaining public parser suite
  toward manifest, run-window, node, service, network, and record command families.
- Iteration 443 split public run-window Clap parser coverage into
  `cli/tests/public_evidence_run_window_parser.rs`, leaving the remaining mixed public parser suite
  focused on manifest, node, service, network, and record command families.
- Iteration 444 split public node heartbeat, heartbeat-file, and operator-attestation Clap parser
  coverage into `cli/tests/public_evidence_node_parser.rs`, leaving the mixed public parser suite
  focused on manifest, service, network, and record command families.
- Iteration 445 split public service health/content Clap parser coverage into
  `cli/tests/public_evidence_service_parser.rs`, leaving the mixed public parser suite focused on
  manifest, network, and record command families.
- Iteration 446 split public network observation and service-log Clap parser coverage into
  `cli/tests/public_evidence_network_parser.rs`, leaving the mixed public parser suite focused on
  manifest and supporting-record command families.
- Iteration 447 split public supporting-record Clap parser coverage into
  `cli/tests/public_evidence_record_parser.rs`, leaving `cli/tests/public_parser.rs` focused on
  public manifest parsing and retired top-level command rejection.
- Iteration 448 split direct public record summary/artifact report coverage into
  `cli/tests/public_evidence_record_summary_reports.rs`, narrowing the remaining record report suite
  toward aggregate-root, record-file, and supporting-record behavior.
- Iteration 449 split aggregate-root public record summary/artifact report coverage into
  `cli/tests/public_evidence_record_aggregate_reports.rs`, leaving the remaining record report suite
  focused on record-file and supporting-record behavior.
- Iteration 450 split network-runtime record-file summary/artifact report coverage into
  `cli/tests/public_evidence_record_file_reports.rs`, leaving the remaining record report suite focused
  on supporting-record file behavior and direct line-rejection edges.
- Iteration 451 split direct public record line and record-file rejection coverage into
  `cli/tests/public_evidence_record_line_rejections.rs`, leaving the remaining record report suite
  focused on successful supporting-record file summaries and artifacts.
- Iteration 452 split direct public record execution argument rejection coverage into
  `cli/tests/public_evidence_record_execution_rejections.rs`, leaving the original record rejection
  suite focused on Clap parse-time invalid argument shapes.
- Iteration 453 split miner, validator, and proposer Clap parser coverage into
  `cli/tests/local_role_parser.rs`, leaving the local parser suite focused on node, localnet,
  defaulting, and local invalid-argument parsing.
- Iteration 454 split miner Clap parser coverage into `cli/tests/local_miner_parser.rs`,
  leaving the local role parser suite focused on validator and proposer command shapes.
- Iteration 455 split validator Clap parser coverage into `cli/tests/local_validator_parser.rs`,
  leaving the local role parser suite focused on proposer command shapes.
- Iteration 456 split public service Clap parser rejection coverage into
  `cli/tests/public_evidence_service_parser_rejections.rs`, leaving the service rejection suite focused
  on execution validation and observation-file errors.
- Iteration 457 split service health execution and health-observation file rejection coverage into
  `cli/tests/public_evidence_service_health_rejections.rs`, leaving the service rejection suite focused
  on service content validation errors.
- Iteration 458 split CUDA miner readiness validation into `cli/tests/local_cuda_validation.rs`
  and added a shared CLI-argument execution helper, leaving local validation focused on invalid local
  argument paths.
- Iteration 459 removed the remaining execution convenience methods from the Clap command types and
  routed the binary through `app::run(TvmdCli::parse())`, keeping Clap as a pure parse boundary with app
  dispatch owned by the application layer.
- Iteration 460 split node-specific Clap parser and local node validation coverage into
  `cli/tests/local_node_parser.rs` and `cli/tests/local_node_validation.rs`, leaving the mixed local
  suites focused on localnet defaults and role argument validation.
- Iteration 461 split public service manifest fixture helpers into
  `cli/tests/manifest_service_fixtures.rs`, leaving `cli/tests/manifest_fixtures.rs` focused on full
  preflight/evidence manifest assembly and shared hashes/signatures.
- Iteration 462 split publication, auditor, and supporting-artifact manifest helpers into
  `cli/tests/manifest_publication_fixtures.rs`, further narrowing the full manifest fixture module to
  bundle construction and manifest text assembly.
- Iteration 463 split node heartbeat and operator-identity manifest helpers into
  `cli/tests/manifest_node_fixtures.rs`, leaving the full manifest fixture module to compose node
  fixture lines instead of owning their signature construction.
- Iteration 464 split network-runtime root aggregation and observation-line rendering into
  `cli/tests/manifest_network_fixtures.rs`, keeping full manifest assembly separate from network
  supporting-record fixture formatting.
- Iteration 465 split `manifest_bundle()` construction into `cli/tests/manifest_bundle_fixtures.rs`,
  leaving `cli/tests/manifest_fixtures.rs` focused on rendering full evidence and preflight manifest text.
- Iteration 466 split preflight manifest rendering into `cli/tests/manifest_preflight_fixtures.rs`,
  leaving `cli/tests/manifest_fixtures.rs` focused on public evidence manifest rendering and shared
  manifest hash/address helpers.
- Iteration 467 split miner device readiness parsing and readiness report fields into
  `app/miner_device_readiness.rs`, leaving `app/operator_checks.rs` focused on stake, wallet, and
  role/service validation orchestration.
- Iteration 468 split local CPU store verification and report rendering into
  `app/local_cpu_verify.rs`, leaving `app/commands.rs` focused on node-store setup, peer readiness,
  and local testnet seeding.
- Iteration 469 split public evidence record root aggregation, record-file parsing, and
  supporting-record validation into `cli/record_evidence_roots.rs`, leaving `cli/record_evidence.rs`
  focused on rendering summary and artifact evidence lines.
- Iteration 470 split node and localnet app command dispatch into `app/tvmd_node_dispatch.rs`
  and shared path rendering into `app/tvmd_path.rs`, leaving `app/tvmd_dispatch.rs` focused on
  top-level command-family routing and role/public evidence dispatch.
- Iteration 471 split shared operator wallet, runtime, and data-dir validation into
  `app/operator_validation.rs`, leaving `app/operator_checks.rs` focused on registration, start,
  and status report rendering.
- Iteration 472 split supporting-record prefixes, payload validation, and supporting-record root
  derivation into `cli/record_supporting_evidence.rs`, leaving `cli/record_evidence_roots.rs`
  focused on aggregate roots, record files, and top-level record-line routing.
- Iteration 473 split local testnet seeding into `app/local_testnet_seed.rs`, leaving
  `app/commands.rs` focused on node-store initialization, peer-book updates, and libp2p readiness checks.
- Iteration 474 split artifact-specific public evidence record Clap arguments into
  `cli/public_evidence_record_artifact_commands.rs`, leaving `cli/public_evidence_record_commands.rs`
  focused on the record command enum, summary arguments, and record-kind value enum.
- Iteration 475 flattened repeated public service endpoint and content-target Clap arguments into
  reusable `PublicServiceEndpointArgs` and `ServiceContentTargetArgs`, keeping service evidence
  subcommands on one canonical typed argument model instead of duplicating shared flags per command.
- Iteration 476 flattened repeated public node role, address, and operator-id Clap arguments into
  `PublicNodeIdentityArgs`, keeping node heartbeat, heartbeat-file, and operator-attestation commands
  on one canonical typed identity model while preserving the existing flags.
- Iteration 477 flattened repeated public evidence record kind, bundle-id, and manifest-signer Clap
  arguments into `PublicEvidenceRecordContextArgs`, keeping summary and artifact record commands on
  one canonical typed context model across direct, roots, and file-backed inputs.
- Iteration 478 flattened repeated public network operator-id, listen-address, and observed-at Clap
  arguments into `NetworkObservationTargetArgs`, keeping direct and service-log network observation
  commands on one canonical typed target model while preserving the existing flags.
- Iteration 479 flattened repeated miner, validator, and proposer wallet-path Clap arguments into
  `RoleWalletArgs`, keeping local role check/run commands on one canonical typed wallet model while
  preserving the existing `--wallet` flag.
- Iteration 480 flattened repeated miner, validator, and proposer node-address Clap arguments into
  `RoleNodeArgs`, keeping local role check/run commands on one canonical typed node target model while
  preserving the existing `--node` flag.
- Iteration 481 flattened repeated local node-store `--data-dir` Clap arguments into `DataDirArgs`
  for node peer, check, block, and localnet verify commands, and routed node/localnet dispatch
  through the typed data-dir accessor.
- Iteration 482 flattened repeated local `--p2p-listen`, `--data-dir`, and `--identity-seed`
  runtime Clap arguments into shared typed groups, keeping node check, node serve, and role runtime
  dispatch on one canonical local runtime model.
- Iteration 483 flattened repeated public publication `--bundle-id` and `--public-uri` Clap arguments
  into `PublicationBundleArgs`, keeping publish and audit evidence commands on one canonical bundle
  target model.
- Iteration 484 flattened repeated public run-window `--bundle-id` and `--manifest-signer` Clap
  arguments into `RunWindowContextArgs`, keeping direct and file-backed run-window evidence on one
  canonical signing context.
- Iteration 485 flattened repeated supporting-record artifact `--artifact-uri` Clap arguments into
  `RecordArtifactLocatorArgs`, keeping direct, roots-backed, and file-backed artifact evidence on
  one canonical locator model.
- Iteration 486 flattened repeated supporting-record `--record-root` and `--record-count` Clap
  arguments into `RecordRootArgs`, keeping direct summary and direct artifact evidence on one
  canonical record-root model.
- Iteration 487 flattened repeated supporting-record `--record-roots` Clap arguments into
  `RecordRootsArgs`, keeping summary-roots and artifact-roots evidence on one canonical aggregate
  roots model.
- Iteration 488 flattened repeated service `--health-path` Clap arguments into
  `ServiceHealthPathArgs`, keeping direct and file-backed service health evidence on one canonical
  observed health path model.
- Iteration 489 flattened repeated supporting-record `--record-file` Clap arguments into
  `RecordFileArgs`, keeping summary-file and artifact-file evidence on one canonical record file
  source model.
- Iteration 490 flattened repeated public evidence `--first-block` and `--last-block` Clap
  arguments into `BlockHeightWindowArgs`, keeping node heartbeat and service health evidence on one
  canonical block-height window model.
- Iteration 491 flattened repeated public evidence `--observed-at` Clap arguments into
  `ObservationTimestampArgs`, keeping network observations, operator attestations, auditor records,
  and service content evidence on one canonical observation timestamp model.
- Iteration 492 flattened repeated public evidence `--manifest-signer` Clap arguments into
  `ManifestSignerArgs`, keeping publication, run-window, and supporting-record evidence on one
  canonical manifest signer model.
- Iteration 493 flattened repeated public evidence `--bundle-id` Clap arguments into
  `EvidenceBundleIdArgs`, keeping publication bundles, run-window contexts, and supporting-record
  contexts on one canonical evidence bundle identifier model.
- Iteration 494 flattened local `node peer add` target arguments into `BootstrapPeerArgs`, keeping
  the `--peer-id` and `--address` flags as one typed bootstrap peer model for parsing and dispatch.
- Iteration 495 flattened repeated public evidence `--operator-id` Clap arguments into
  `OperatorIdArgs`, keeping node identity and network observation targets on one canonical operator
  identifier model.
- Iteration 496 routed public service endpoint Clap parsing through typed endpoint/target accessors,
  keeping service evidence execution out of raw parser fields and one-off hash conversions.
- Iteration 497 routed public publication and auditor Clap parsing through typed accessors, keeping
  publication evidence execution out of raw count, auditor address, and audit URI fields.
- Iteration 498 routed public service health/content source Clap parsing through typed accessors,
  keeping service execution out of raw observation-file, content-root, content-byte, and content-file
  fields.
- Iteration 499 routed public run-window Clap parsing through typed accessors, keeping run-window
  evidence execution out of raw timestamp, observed-block, and block-observation-file fields.
- Iteration 500 routed public network observation Clap parsing through typed accessors, keeping
  network evidence execution out of raw peer, protocol-count, limit, timeout, and service-log fields.
- Iteration 501 routed public node/operator evidence Clap parsing through typed accessors, keeping
  node evidence execution out of raw role, heartbeat-count, heartbeat-file, identity-URI, and
  observation fields.
- Iteration 502 routed public service health observation counters through typed Clap accessors,
  keeping service health execution out of raw reachability and signed-check count fields.
- Iteration 503 routed supporting-record kind Clap parsing through the record context accessor,
  keeping record evidence execution out of repeated raw `context.kind` conversions.
- Iteration 504 routed public auditor and service-content observation timestamps through typed
  owner accessors, keeping evidence execution out of nested raw observation fields.
- Iteration 505 routed top-level public preflight/evidence manifest Clap paths through typed path
  accessors, keeping app dispatch out of raw manifest path fields.
- Iteration 506 routed local node/localnet runtime dispatch through typed Clap accessors, keeping
  service dispatch out of raw runtime, peer, block-height, and JSON-mode fields.
- Iteration 507 routed local role command dispatch through typed Clap accessors, keeping miner,
  validator, and proposer paths out of raw stake, wallet, device, node, and runtime fields.
- Iteration 508 hid the root `TvmdCli` command field behind explicit accessors, keeping app dispatch
  and parser tests on the typed Clap root boundary instead of reaching into parser internals.
- Iteration 509 routed public node evidence execution through owning command accessors, keeping the
  generator out of nested identity and block-window parser groups.
- Iteration 510 routed public publication evidence execution through owning command accessors,
  keeping the generator out of nested bundle and signer parser groups.
- Iteration 511 routed public service evidence execution through owning command accessors, keeping
  health and content generators out of nested endpoint, path, window, and target parser groups.
- Iteration 512 routed public network evidence execution through owning command accessors, keeping
  runtime and service-log generators out of the nested observation target parser group.
- Iteration 513 routed public run-window evidence execution through owning command accessors, keeping
  direct and file-backed generators out of the nested bundle/signer context parser group.
- Iteration 514 routed public supporting-record evidence execution through owning command accessors,
  keeping summary and artifact generators out of nested context/root/artifact/file parser groups.
- Iteration 515 hid shared public evidence leaf Clap fields behind constructors and accessors, keeping
  bundle IDs, manifest signers, operator IDs, and observation timestamps out of public parser internals.
- Iteration 516 hid supporting-record leaf Clap fields behind constructors and accessors, keeping
  record roots, root lists, record files, and artifact URIs out of public parser internals.
- Iteration 517 hid local role leaf Clap fields behind constructors and accessors, keeping stake
  amounts, wallet paths, and role node multiaddrs out of public parser internals.
- Iteration 518 hid local runtime leaf Clap fields behind constructors and accessors, keeping data
  directories, P2P listen addresses, and identity seeds out of public parser internals.
- Iteration 519 hid local runtime owner Clap fields behind constructors and accessors, keeping
  node and role runtime groups out of public parser internals.
- Iteration 520 hid local node/localnet Clap fields behind constructors and accessors, keeping
  peer-add, bootstrap-peer, node-check, node-serve, block, and verify args out of public parser internals.
- Iteration 521 hid local role command Clap fields behind constructors and accessors, keeping miner,
  validator, and proposer check/run groups out of public parser internals.
- Iteration 522 hid shared public manifest and block-window Clap fields behind constructors and
  accessors, keeping manifest paths and block-height bounds out of public parser internals.
- Iteration 523 hid public run-window Clap fields behind constructors and accessors, keeping direct
  window timestamps, observed-block counts, file inputs, and bundle/signer context out of public parser internals.
- Iteration 524 hid public network observation Clap fields behind constructors and accessors, grouping
  protocol counts and transport limits into typed argument owners while keeping service logs and observation targets out of public parser internals.
- Iteration 525 hid public node/operator Clap fields behind constructors and accessors, keeping
  heartbeat windows, heartbeat files, identity URIs, and node identity triples out of public parser internals.
- Iteration 526 hid public publication/auditor Clap fields behind constructors and accessors, keeping
  bundle publication URIs, manifest signature counts, auditor IDs, audit URIs, and observation times out of public parser internals.
- Iteration 527 hid public supporting-record Clap group fields behind constructors and accessors, keeping
  record context, root, roots, file, and artifact locator groups out of public parser internals.
- Iteration 528 hid public service-health Clap fields behind constructors and accessors, keeping
  endpoint/path/window groups, reachability counters, signed-check counters, and observation files out of public parser internals.
- Iteration 529 hid public service-content Clap fields behind constructors and accessors, keeping
  endpoint triples, content targets, content roots, byte payloads, content files, and minimum byte counts out of public parser internals.
- Iteration 530 tightened the public Clap boundary so external callers keep `TvmdCli` plus `app::run`
  while direct `TvmdCommand` execution, parsed command extraction, and argument payload types remain crate-internal.
- Iteration 531 made the parsed Clap command enums and command-family re-exports crate-private, leaving
  `TvmdCli` as the only public parser entrypoint while preserving internal typed dispatch.
- Iteration 532 made Clap-only argument payload structs and value wrappers crate-private, keeping
  `TvmdCli` public while preventing parser internals from becoming a library API by accident.
- Iteration 533 made the `cli` module internal and kept only the root `TvmdCli` and manifest validators
  public, so the Clap command tree is an implementation detail behind `app::run`.
- Iteration 534 moved service-status output assembly onto `KeyValueReportWriter`, keeping node-store and
  role-runtime status fields parseable through the shared key-value report path instead of a giant format string.
- Iteration 535 collapsed service-status pass-through fields into ordered status-file field lists, keeping
  the output contract explicit without a long manual append sequence.
- Iteration 536 made service-status field lookups borrow from the cached status-file map, avoiding
  per-field `String` clones while keeping missing status fields rendered as `unknown`.
- Iteration 537 moved test-only CLI parser and evidence helper imports out of the production `cli.rs`
  root and into `cli/tests.rs`, keeping the production Clap module boundary focused on runtime exports.
- Iteration 538 moved node init, peer-add, and readiness report rendering onto `KeyValueReportWriter`,
  including production subreport appends for shared libp2p identity fields.
- Iteration 539 moved local-testnet seed report rendering onto `KeyValueReportWriter`, keeping seeded
  chain metrics parseable through the same app-owned key-value report path.
- Iteration 540 moved per-block node status rendering onto `KeyValueReportWriter`, replacing the last
  large `service_block` format string with validated key-value field assembly.
- Iteration 541 moved the printed role-runtime `service_serve` report onto `KeyValueReportWriter`,
  preserving libp2p identity subreports while eliminating another large key-value format string.
- Iteration 542 moved the persisted `role-runtime.status` writer onto `KeyValueReportWriter`, so both
  role runtime report surfaces now share validated app-owned key-value assembly.
- Iteration 543 moved public evidence and public preflight manifest validation reports onto
  `KeyValueReportWriter`, keeping the Clap-facing manifest validators on the shared key-value report path.
- Iteration 544 moved public publication and auditor evidence outputs onto `KeyValueReportWriter`,
  preserving their comma-record payloads inside validated key-value report fields.
- Iteration 545 moved public run-window evidence output onto `KeyValueReportWriter`, keeping signed
  run-window fields validated through the shared report assembler.
- Iteration 546 moved supporting-record summary and artifact evidence outputs onto
  `KeyValueReportWriter`, preserving dynamic record prefixes and artifact comma payloads in validated fields.
- Iteration 547 moved public node heartbeat and operator-attestation evidence outputs onto
  `KeyValueReportWriter`, keeping node/operator comma payloads inside validated report fields.
- Iteration 548 moved public service health and content evidence outputs onto `KeyValueReportWriter`,
  keeping service comma payloads inside validated report fields.
- Iteration 549 moved public network runtime observation evidence output onto `KeyValueReportWriter`,
  keeping the libp2p observation comma payload inside a validated report field.
- Iteration 550 removed the getter/constructor compatibility layer from the local `tvmd` Clap command
  payloads, so node, role, localnet, and top-level dispatch now use the typed Clap fields directly instead
  of treating Clap as a wrapper around an older command API.
- Iteration 551 removed the same compatibility layer from shared public manifest, publication, signing,
  observation, block-window, and run-window Clap payloads, keeping the public evidence command family on
  direct typed fields instead of constructor/getter adapters.
- Iteration 552 moved public node and operator evidence Clap payloads to direct fields, removing the
  node/operator getter constructors while keeping heartbeat, heartbeat-file, and operator-attestation
  execution on the typed command tree.
- Iteration 553 moved public network evidence Clap payloads to direct fields, removing observation,
  service-log, protocol-count, transport-limit, and target getter constructors while keeping network
  evidence execution on the typed command tree.
- Iteration 554 moved public service evidence Clap payloads to direct fields, removing service endpoint,
  health, content, byte-content, file-content, and target getter constructors while keeping service evidence
  execution on the typed command tree.
- Iteration 555 moved public supporting-record Clap payloads to direct fields, removing summary, artifact,
  aggregate-root, file, locator, and record-context getter constructors while keeping record evidence
  execution on the typed command tree.
- Iteration 556 removed the final CLI compatibility shims from value-parser test construction and identity
  seed dispatch, so Clap payload structs remain the command data boundary instead of delegating through
  legacy-style helper constructors and getters.
- Iteration 557 split top-level `tvmd` app dispatch into command-family handlers for miner, validator,
  proposer, public, node, and localnet commands, shrinking the remaining catch-all match while preserving
  the Clap command tree as the execution boundary.
- Iteration 558 split local node and localnet app dispatch into focused handlers for node init, peer add,
  readiness, serving, status, block lookup, seed, and verify flows, keeping validation at the app boundary
  while shrinking the remaining nested node command match.
- Iteration 559 consolidated public supporting-record evidence execution around one typed record context
  extractor and shared root conversion helper, removing repeated Clap-field plumbing from summary,
  artifact, roots, and file command variants.
- Iteration 560 consolidated public service evidence execution around typed endpoint and content-target
  contexts, so health, health-file, content-root, content-bytes, and content-file variants no longer repeat
  the same Clap-field extraction before calling the service evidence builders.
- Iteration 561 consolidated public network evidence execution around typed target, protocol-count, and
  transport-limit contexts, keeping observation and service-log variants from duplicating libp2p operator,
  listen address, timestamp, and runtime-limit extraction.
- Iteration 562 consolidated public node evidence execution around a typed node identity context, removing
  repeated role, address, and operator-id extraction from heartbeat, heartbeat-file, and operator-attestation
  command variants.
- Iteration 563 consolidated public run-window evidence execution around a typed bundle/signer context,
  removing repeated bundle-id and manifest-signer extraction from direct and file-backed run-window command
  variants.
- Iteration 564 narrowed public publication evidence execution to explicit publish and audit handlers,
  removing the broad `EvidenceCommand` executor plus its unreachable fallback and sharing bundle/public-URI
  extraction through a typed publication bundle context.
- Iteration 565 removed the redundant miner, validator, and proposer service wrapper functions, so `tvmd`
  role dispatch now invokes the typed `RoleServiceRunner` directly instead of preserving a second forwarding
  API for the same runtime role execution.
- Iteration 566 split RPC static route dispatch out of the dynamic route fallback path, keeping the existing
  route table behavior while separating fixed service, explorer, telemetry, faucet, and submission routes
  from path-segment-based dynamic routing.
- Iteration 567 split mutable RPC route dispatch out of the read-only fallback path, so transaction,
  reference-submission, attestation, and faucet claim routes are identified before delegating unmatched
  requests back to the static/dynamic read route pipeline.
- Iteration 568 split dynamic RPC route method handling from the dynamic GET path-segment route table,
  keeping non-GET dynamic requests on the not-found path while isolating chain, explorer, tensor, job,
  miner, validator, and receipt lookups behind a focused dynamic GET helper.
- Iteration 569 split static RPC route dispatch by HTTP method, keeping GET service/explorer/telemetry/faucet
  reads separate from immutable POST reference acknowledgements before dynamic route fallback.
- Iteration 570 split dynamic RPC GET route dispatch by first path segment, isolating chain, receipt,
  miner, validator, explorer, tensor, and job lookups into route-family helpers.
- Iteration 571 split static RPC GET dispatch into core, explorer, telemetry, and faucet route-family
  helpers, keeping fixed read routes out of a single mixed service match.
- Iteration 572 split mutable RPC dispatch into method-first routing and a POST-only mutation helper,
  removing the repeated POST guard from transaction, receipt, attestation, and faucet-claim routing.
- Iteration 573 split public service health evidence execution out of the service command match, keeping
  direct health counts and file-backed health observations behind focused helper paths.
- Iteration 574 split direct public network observation execution out of the network command match,
  keeping peer/protocol/transport evidence assembly separate from service-log-backed observation loading.

## Core Abstraction Correction: `Chain`, Not `LocalChain`

`LocalChain` is the wrong name for the core protocol state machine. It was a reasonable name when the
project was mostly a local harness, but it now leaks a deployment profile into the consensus layer. The
chain itself is not local. Local CPU, public testnet, and mainnet are profiles/configurations of the same
chain engine.

The target model should be:

```text
Chain owns consensus.
Profiles own environment policy.
Runtime owns process and network orchestration.
```

The core type should be named `Chain`. The local profile should be represented by configuration, not by a
separate chain abstraction.

Concrete target shape:

```rust
pub struct Chain {
    params: ChainParams,
    state: ChainState,
    blocks: Vec<TensorBlock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainProfile {
    LocalCpu,
    PublicTestnet,
    Mainnet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainConfig {
    pub profile: ChainProfile,
    pub genesis: GenesisConfig,
    pub params: ChainParams,
    pub job_source: JobSourceConfig,
    pub reward_policy: RewardPolicy,
    pub network_policy: NetworkPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub finalized_randomness: Hash,
    pub initial_miners: Vec<GenesisMiner>,
    pub initial_validators: Vec<GenesisValidator>,
    pub initial_accounts: Vec<GenesisAccount>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobSourceConfig {
    SyntheticLocalCpu {
        matmul_shape: (usize, usize, usize),
        linear_training_shape: (usize, usize),
    },
    NetworkOnly,
    ExternalProgrammatic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardPolicy {
    pub miner_reward_pool: u64,
    pub validator_reward_pool: u64,
    pub proposer_reward_bps: u64,
    pub treasury_reward_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkPolicy {
    pub require_libp2p: bool,
    pub allow_synthetic_jobs: bool,
    pub allow_local_block_production: bool,
}
```

The implementation should be a direct replacement, not a compatibility migration:

```rust
pub struct Chain {
    params: ChainParams,
    state: ChainState,
    blocks: Vec<TensorBlock>,
}
```

Do not add `pub type LocalChain = Chain`. Do not keep `LocalChain` as a deprecated alias. Rename the core
type and update call sites, tests, docs, and public exports in the same feature-sized refactor. If the
branch has not shipped, preserving a misleading compatibility name just keeps the wrong abstraction alive.

This rename is not cosmetic. It deletes a misleading concept. The current name encourages local-only
shortcuts to creep into consensus code. A `Chain` configured by `ChainProfile::LocalCpu` makes the boundary
clear: local CPU is a profile and deployment mode, not a different state machine.

Rules that should follow from this:

- `Chain` must not know whether it is running in Docker Compose, a public testnet, or mainnet.
- `local_cpu`, `public_testnet`, and `mainnet` must not fork block validation, receipt settlement, finality,
  codec behavior, or state roots.
- Synthetic local jobs are `JobSourceConfig`, not a separate chain type.
- Local rewards are `RewardPolicy`, not hardcoded constants in node/runtime glue.
- Local deployment readiness is evidence about a profile, not a protocol architecture.

## Priority Findings

### 1. Chain State Has No Real Encapsulation

`Chain` now hides its top-level fields from external crate users, but `ChainState` still exposes its
internals directly in `crates/tensor_vm/src/chain/state.rs`, and internal runtime modules can still bypass
the command boundary.

```rust
pub struct Chain {
    pub(crate) params: ChainParams,
    pub(crate) state: ChainState,
    pub(crate) blocks: Vec<TensorBlock>,
}
```

This defeats the purpose of `ChainCommand` and `ChainEngine`. Any caller can mutate jobs, receipts,
votes, rewards, finality, or height without going through validation.

Impact:

- Invariants in `chain/validation.rs`, `chain/receipts.rs`, `chain/settlement.rs`, and `chain/blocks.rs`
  are advisory rather than enforced.
- Runtime and tests can create states the chain engine would never admit.
- Future refactors will keep needing defensive cleanup because invalid states are representable everywhere.

Remaining fix:

- Move direct internal mutation of `Chain.state`, `Chain.blocks`, and `Chain.params` behind narrower helpers.
- Expose immutable views through `ChainView` or narrow accessor methods.
- Route production mutations through `ChainEngine::apply_command`.
- Move direct-state test setup into explicit `#[cfg(test)]` builders.

### 2. Dual Mutation APIs Make Events Optional

The command/event facade exists in `crates/tensor_vm/src/chain/engine.rs`, but the codebase still mutates
through imperative methods on `Chain`.

Remaining examples:

- runtime test setup and lower-level model tests still call direct mutation helpers in several places.
- `node.rs` and runtime paths call chain helpers directly.
- network block ingestion still prepares parent state and admits blocks directly so it can preserve
  pending/duplicate/invalid admission results.
- `apply_transaction` now routes non-reference writes through `ChainCommand`, returns command events, and
  rejects txpool-only reference submissions, but the public transaction surface still mixes immediate
  mutations with queued reference announcements.

This violates single responsibility and interface segregation: callers must know which mutation path emits
events, which one finalizes, and which one silently mutates.

Fix:

- Pick the command path as canonical.
- Keep imperative methods private to `chain/*` modules.
- Make public write paths return typed events/effects.
- Remove or implement transaction variants that currently lie about behavior.

### 3. `main.rs` Should Be Only An Entrypoint

`main.rs` should not be a module family. The binary target should be a tiny process adapter: parse CLI,
call library code, print output, convert errors to exit codes.

Target shape:

```rust
use clap::Parser;
use tensor_vm::{app, TvmdCli};

fn main() {
    let cli = TvmdCli::parse();
    match app::run(cli) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(error.exit_code());
        }
    }
}
```

The current `crates/tensor_vm/src/main/` module split improved file size, but it keeps too much
application logic conceptually owned by the binary. That is the wrong ownership boundary. Runtime,
service, status, command dispatch, local CPU verification, miner/validator workers, network glue, and
synthetic production should be library modules callable from tests and any future binary.

Fix:

- Delete the `main` module concept as an application framework.
- Move command dispatch into `app.rs` or `tvmd.rs`.
- Move long-running service code into `service/`.
- Move miner/validator/proposer workers into `service/roles/` or `node/roles/`.
- Move status snapshots and renderers into `service/status.rs` or `status.rs`.
- Move local CPU verification into a library-owned verifier module.
- Keep `src/main.rs` as a 10-20 line process entrypoint.
- Keep consensus decisions in `chain/*`, not runtime loops.

### 4. Runtime Adapters Still Own Consensus Behavior

Runtime and node adapter code still knows too much about assignment, chain state shape, block publication,
role counters, persistence, and local production. Moving files under `src/main/` reduced the size of
`main.rs`, but it did not fully fix the ownership problem.

Fix:

- Treat runtime code as adapters around library-owned services.
- Make `app::run` and service modules testable without going through the binary.
- Keep process concerns in `main.rs`; keep domain behavior in library modules.

### 5. Finality And Block Admission Need A Harder Boundary

The code now has a typed `BlockAdmission` direction, which is good. The next step is making finality
fully vote-driven in every runtime path.

Target contract:

```text
valid block payload -> append block
signed block vote payloads -> finality
```

Anything that appends and finalizes in one hidden step should be treated as test-only or removed.

Fix:

- Keep `admit_block` append-only.
- Route all finality through `SubmitBlockVote`.
- Add block-vote gossip/RPC coverage as a first-class network event.
- Keep local auto-finalization only in clearly named pure test helpers.

### 6. Shell Scripts Encode Protocol Policy

`deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh` is a large shell policy engine. It verifies
protocol claims by grepping status strings, parsing JSON with `sed`, and duplicating service lists.

This is not just unpleasant. It is an architecture problem: readiness semantics live outside the Rust
types that produce the state.

Fix:

- Move verification policy into Rust, for example `tvmd localnet verify --json`.
- Let shell orchestrate Docker only.
- Make the checker consume typed JSON, not scattered `key=value` strings.
- Remove cargo test execution from deployment scripts; CI should own unit tests.

### 7. Status Surfaces Are Duplicated And Stringly

There are multiple overlapping status emitters:

- CLI readiness output in `cli.rs`
- real service handlers in `main.rs`
- `role-runtime.status`
- `local-cpu-ready`
- `service status`
- checker expectations
- CLI integration tests

The same conceptual fields appear with different names, for example prefixed and unprefixed role fields.
This creates brittle tests and makes every new metric require edits across many files.

Fix:

- Introduce one typed `RuntimeStatusSnapshot` / `ServiceStatusV1`.
- Render text and JSON from the same struct.
- Add a schema version.
- Make tests parse the schema instead of checking substring inventories.

### 8. Duplicate Codecs Are A Drift Risk

`p2p.rs` and `storage.rs` both encode domain objects manually, including block payloads. The layouts are
similar enough to suggest shared intent, but separate enough to drift.

Current duplicated domains include:

- `TensorBlock`
- jobs
- receipts
- attestations
- block votes
- tensor payloads

Fix:

- Create a `codec/` module.
- Put canonical encode/decode for consensus payloads there.
- Make p2p and storage call the shared codec.
- Add golden, roundtrip, malformed, trailing-byte, and max-size tests.
- Keep storage-specific wrappers separate from payload encoding.

### 9. Manual Parsing Is Still Overused Outside The Clap CLI

Parsing appears in many forms:

- legacy hand-rolled CLI parsing was replaced by the typed `clap` command tree in `crates/tensor_vm/src/cli/commands.rs`
- shell `grep`/`sed` JSON extraction
- `key=value` status parsing
- hand-written JSON extraction in RPC/tests
- CSV-like manifest parsing with `split(',')`
- whitespace transaction parsing

This is brittle and non-idiomatic Rust for structured data.

Fix:

- Keep `clap` as the CLI parser boundary with typed subcommands, flattened shared args, `ValueEnum`
  evidence/profile values, env defaults, and value hints.
- Use `serde_json` for JSON.
- Use typed structs for CLI/RPC status output.
- Use the `csv` crate or a simpler line format that rejects ambiguous characters explicitly.
- Replace `stdout.contains(...)` tests with parsed assertions.

The CLI should not regress to a giant string-slice match. The current public `tvmd` parser surface is the
root `TvmdCli` plus `app::run`; the parsed command tree and Clap argument payloads are crate-internal.
There are no preserved legacy top-level `role`, `public-evidence`, `public-testnet`, `local-testnet`,
or `local-cpu` command families:

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "tvmd")]
pub struct TvmdCli {
    #[command(subcommand)]
    command: TvmdCommand,
}

#[derive(Subcommand, Debug)]
pub(crate) enum TvmdCommand {
    Node(NodeCommand),
    Miner(MinerCommand),
    Validator(ValidatorCommand),
    Proposer(ProposerCommand),
    Localnet(LocalnetCommand),
    Public(PublicCommand),
}

#[derive(Subcommand, Debug)]
pub(crate) enum NodeCommand {
    Init(DataDirArgs),
    Peer(NodePeerCommand),
    Check(NodeCheckArgs),
    Serve(NodeServeArgs),
    Status(DataDirArgs),
    Block(NodeBlockArgs),
}

#[derive(Args, Debug)]
pub(crate) struct DataDirArgs {
    #[arg(long)]
    data_dir: PathBuf,
}

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChainProfileArg {
    LocalCpu,
    PublicTestnet,
    Mainnet,
}
```

The remaining parsing cleanup is no longer about CLI argv parsing. It is about replacing ad hoc structured
data parsing in status files, shell checks, RPC tests, manifests, and transaction text with typed formats.

### 10. `TvmError` Is Too Stringly

`TvmError::InvalidReceipt(&'static str)` carries many unrelated failures:

- bad signatures
- codec length errors
- duplicate votes
- unknown blocks
- malformed transactions
- state-root mismatches

This prevents callers from making reliable decisions without matching strings.

Fix:

- Split errors by domain:
  - `ChainError`
  - `BlockAdmissionError`
  - `CodecError`
  - `StorageError`
  - `RpcError`
- Keep `TvmError` as a top-level wrapper.
- Avoid string matching in retry/admission logic.

### 11. Error Paths Have Hidden Side Effects

`validation.rs::submit_attestation` can penalize a validator while returning `Err`.

That means `Err` does not imply no state change. This is surprising and makes retries dangerous.

Fix:

- Split validation from effects:
  - `validate_attestation(...) -> AttestationDecision`
  - `apply_attestation_decision(...)`
- If rejected attestations slash or penalize, return a typed event/effect.
- Make side-effecting rejection explicit in `ChainEvent`.

### 12. `Chain::clone()` Is Too Easy

`Chain` is deeply cloneable. That encourages speculative mutation by cloning the whole chain and
replacing it on success.

This is convenient but not scalable or conceptually clean.

Fix:

- Replace clone-based admission with dry-run validation or a small rollback journal.
- Avoid cloning `ChainState` to reconstruct parent views.
- Consider removing or narrowing `Clone` for `Chain` outside tests.

### 13. God Modules Hide Ownership

Several files are far past a healthy size boundary:

| File | Problem |
| --- | --- |
| `crates/tensor_vm/src/cli.rs` | CLI parsing, validation, public evidence, docs tests, string output |
| `crates/tensor_vm/src/testnet.rs` | testnet orchestration, manifests, public evidence, validation |
| `crates/tensor_vm/src/p2p.rs` | p2p public configuration facade and integration-style tests |
| `crates/tensor_vm/src/main.rs` and `crates/tensor_vm/src/main/*` | binary dispatch and application/service framework logic that should live in library modules |
| `crates/tensor_vm/src/rpc.rs` | HTTP parsing, routing, explorer, websocket, chain reads |
| `crates/tensor_vm/src/storage.rs` | snapshots, block log, state codec, recovery |
| `crates/tensor_vm/src/node.rs` | network ingest, payload apply, pending queues, runtime counters |

Fix after boundaries are cleaner:

- `p2p/{service,codec,peer_book,request_response}.rs`
- `app.rs` or `tvmd.rs` for command dispatch
- `service/{runtime,roles,status}.rs`
- `rpc/{server,routes,explorer,websocket}.rs`
- `storage/{snapshot,block_log,chain_state,codec}.rs`
- `cli/{parse,reference,evidence,commands}.rs`

Do not split files first. Split after the canonical owners are clear.

## SOLID Review

### Single Responsibility

Weak in:

- `main.rs` and `main/*`
- `cli.rs`
- `p2p.rs`
- `rpc.rs`
- `storage.rs`
- `testnet.rs`

Strong in:

- `tensor.rs`
- `field.rs`
- `merkle.rs`
- parts of `chain/*`
- `miner.rs`
- `validator.rs`

Recommendation: keep core math and chain modules focused; extract operational code aggressively.

### Open/Closed

The system is not open for extension without editing many places. Adding a new payload/status field often
touches:

- API enum
- p2p codec
- lib re-exports
- node ingest
- main status strings
- checker script
- CLI tests
- compose artifact tests

Recommendation: route extensions through typed snapshots and shared codecs.

### Liskov Substitution

This is less relevant because there is little trait hierarchy. The notable issue is that the
`ChainEngine` trait promises a mutation boundary, but `Chain` exposes bypasses. Implementations can
not be substituted if callers rely on public fields.

Recommendation: make `ChainEngine` meaningful by hiding direct mutation.

### Interface Segregation

`lib.rs` re-exports too much. Consumers get chain, p2p, CLI, testnet, telemetry, and deployment-adjacent
types from one crate surface.

Recommendation:

- expose `tensor_vm::core`
- expose `tensor_vm::node`
- expose `tensor_vm::ops`
- or split into `tensor_vm_core` and `tensor_vm_node` later.

### Dependency Inversion

Runtime logic depends directly on concrete `RpcHttpServer`, `TensorVmLibp2pService`, `Chain`,
`NodeStore`, and status files.

Recommendation:

- introduce narrow traits for runtime dependencies:
  - `ChainMutator`
  - `TensorStore`
  - `NetworkPublisher`
  - `RuntimeStatusSink`
- use concrete types at the binary boundary.

## DRY Findings

### Duplicated Domain Logic

High-priority duplicates:

- block commit path in `blocks::produce` and `blocks::admit`
- p2p and storage payload codecs
- public endpoint validation across CLI/testnet code
- attestation validation in write path, quorum checks, and watcher logic
- receipt submission for TensorOp and LinearTrainingStep
- status string rendering across CLI/main/checker/tests
- identity seed reports in CLI/main
- CLI argument parsing and command descriptions in `cli.rs`
- JSON/key-value parsing in shell/tests/RPC

Fix one domain at a time. The highest leverage is shared status snapshot and shared codecs.

## Idiomatic Rust Findings

### Prefer Private Fields And Smart Constructors

Many domain structs have all-public fields. That is reasonable for plain data transfer, but not for
consensus state.

Make these private or crate-private:

- `Chain`
- `ChainState`
- possibly `TensorBlock`
- possibly `BlockVote`

Keep DTOs public only at serialization boundaries.

### Prefer Typed Results Over Strings

Replace:

```rust
Result<T, String>
InvalidReceipt(&'static str)
```

with typed errors that callers can match without string parsing.

### Prefer Shared Parsers Over Hand-Rolled Splits

Manual parsing is acceptable for small, consensus-critical binary codecs if centralized and tested. It is
not appropriate for every JSON/status/CSV surface.

### Avoid Deep Clone As Control Flow

Cloning a whole chain to attempt admission is easy but expensive and hides which changes are speculative.
Prefer dry-run validation or a journal.

### Avoid Massive Match Functions

App command dispatch, CLI parsing, and RPC route dispatch should be split into typed command handlers.

## Tests Review

Strengths:

- high number of focused unit tests
- good coverage of malformed p2p payloads
- good coverage of chain validation edges
- integration tests exercise CLI flows

Weaknesses:

- too many substring tests
- huge inline test modules in already-large source files
- checker script tests assert script text rather than behavior
- no property/fuzz tests for manual codecs
- repeated doc/manifest tests across multiple files

Fix:

- add test support helpers in `crates/tensor_vm/tests/support`
- parse structured outputs instead of using `contains`
- move binary-runtime tests out of `main.rs`
- add proptest or table-driven malformed codec tests
- reduce duplicate manifest tests to one integration surface

## Recommended Refactor Order

1. Encapsulate `Chain` and `ChainState`.
2. Enforce `ChainEngine` / `ChainTransition` as the only production mutation path.
3. Split block append from finality everywhere.
4. Consolidate p2p/storage codecs.
5. Introduce typed status snapshots and JSON outputs.
6. Move local CPU checker policy into Rust.
7. Keep the completed Clap CLI as the parser boundary and remove the remaining non-CLI manual parsers.
8. Stop persisting chain state on read-only runtime activity. Read-only RPC was completed in Iteration 5,
   and validator remote tensor-fetch status bookkeeping was completed in Iteration 10.
9. Collapse `main.rs` to a thin binary entrypoint and move `src/main/*` application logic into library-owned
   `app`/`service` modules.
10. Split `cli.rs`, `p2p.rs`, `rpc.rs`, and `storage.rs` by ownership.
11. Replace stringly errors with typed domain errors.
12. Move large inline tests into focused module or integration test files.

## Positive Notes

The following pieces are directionally good and should be preserved:

- `ChainCommand` / `ChainEvent` is the right abstraction; it needs enforcement.
- `BlockAdmission` is the right shape; carry that pattern further.
- `Tensor`, `TxPool`, and `NodeRuntimeState` show better encapsulation than chain state.
- `tensor.rs`, `field.rs`, `merkle.rs`, `miner.rs`, and `validator.rs` are comparatively focused.
- The verification math is relatively isolated from I/O.
- Test density is high; the problem is test shape and duplication, not total lack of tests.

## Approval Bar For Future Changes

Do not accept future changes that:

- add consensus behavior to runtime adapters
- add more `key=value` fields without a typed snapshot owner
- add another codec for an existing domain type
- expose more mutable state publicly
- match consensus errors by string
- extend the shell checker as the protocol authority
- push an already-large file further past a clear split boundary

The codebase needs fewer branches in adapters and stronger ownership in the core.
