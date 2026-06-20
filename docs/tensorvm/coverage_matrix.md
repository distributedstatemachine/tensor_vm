# TensorVM Coverage Matrix

This maps [`mvp_spec.md`](mvp_spec.md) acceptance criteria to concrete
implementation artifacts and tests.

## Gate 0

The first non-skippable spec gate is the default-feature CPU local multi-participant testnet. It is
checked with:

```bash
cargo test -p tensor_vm local_testnet --release
```

That filtered test run covers:

- `testnet::tests::local_testnet_can_bootstrap_from_shared_profile`
- `testnet::tests::local_testnet_bootstraps_required_public_shape`
- `testnet::tests::local_testnet_runs_full_matmul_receipt_attestation_settlement_round`
- `testnet::tests::local_testnet_runs_linear_training_receipt_state_transition_round`
- `p2p::tests::local_testnet_libp2p_swarms_exchange_gossip_and_request_response`

These tests exercise the CPU reference path with the default local 10-miner/5-validator shape, separate
local participant identities and libp2p endpoints, a live mandatory libp2p control-plane startup under
default features, real loopback libp2p delivery across every TensorVM gossip topic and request-response
message family, service-level root-addressed tensor request-response fetches, validator role remote tensor
fetch before attestation, local block production, matmul receipt validation/attestation/settlement/rewards,
LinearTrainingStep validation and state transition, local tensor-server availability, no simulation or
local-only networking-shim credit, and the explicit separation between local evidence and the 7-day public
deployment gate.

## Tensor IR Foundation

The local reference now includes a content-addressed Tensor IR foundation for `upow.md` §4:
`ir::TensorGraph` canonical JSON encoding, `graph_id`, frozen op-registry metadata, structural validation,
verifier-class metadata for every frozen op, Tier-C vocabulary gating, explicit non-admission of
index-consistency ops, and canonical graph constructors for the current TensorOp matmul and
LinearTrainingStep primitives. The current fixed job structs derive their receipt `program_hash` from the
validated graph ID, and current canonical TensorOp/LinearTrainingStep receipt constructors derive their
`trace_root` from exact execution of that canonical graph's op traces. Verifiers require otherwise-valid
current-job receipts to carry the same canonical IR trace root. Submitted current jobs register their
canonical graph body bytes in chain state keyed by graph ID, and arbitrary user-submitted canonical graph
bodies can now be registered directly through
`ChainCommand::RegisterProgramBody` after parsing, consensus validation, graph-id matching, and
byte-for-byte canonical encoding checks. The registry is committed in the state root, persisted by the
node-store snapshot, hydrated into the runtime program server at startup, and servable through the existing
`RequestProgram`/`ProgramResponse` libp2p request-response path. The IR module now also exposes a
deterministic exact interpreter foundation for validated, consensus-admitted
graphs over the currently implemented tensor runtime ops (`matmul`, broadcast-aware `add`/`sub`/`mul`,
`scalar_mul`, `transpose`, explicit-dim `sum`/`reduce_sum`, `identity`, `neg`, signed-residue
`abs`/`sign`/`relu`, fixed-point scale-aware half-even `round`, `reshape`, `broadcast`, comparisons
`gt`/`lt`/`ge`/`le`/`eq`, `where`, `mean`, `cast`, `concat`, `stack`, `full`, and `arange`). Tensor
metadata now also carries canonical `int8`, `uint8`, and `bool` dtype tags through tensor commitments,
shared codecs, p2p tensor payloads, and canonical IR JSON. The frozen registry now admits exact
`quantize_int8_per_channel` and `dequantize_int8_per_channel` execution: quantize uses deterministic
per-channel integer scales, round-half-even division, and int8 clamping, while dequantize multiplies by a
rank-1 scale tensor and rejects ambiguous inferred channel dimensions. Byte-packed
`quantize_pack_int8`/`unpack_dequantize_int8` are also admitted with a canonical flat `uint8` payload
layout containing `TVQ8` magic/version bytes, rank, quantization axis, output scale, original shape,
per-channel signed 64-bit scales, and row-major int8 payload bytes. Interpreter output includes named
output tensors, per-op output commitment roots, and a Merkle `trace_root`; deferred Tier-C ops and
admitted registry ops without implemented exact replay return explicit execution errors instead of being
silently accepted. Focused
evidence: `ir::tests::matmul_graph_has_stable_canonical_json_and_graph_id`,
`ir::tests::graph_validation_rejects_bad_structure`,
`ir::tests::graph_validation_rejects_op_metadata_mismatches`,
`ir::tests::tier_c_vocabulary_is_carried_but_not_consensus_admitted`,
`ir::tests::frozen_registry_declares_verifier_class_for_every_op`,
`ir::tests::index_ops_require_index_consistency_and_are_not_consensus_admitted`,
`ir::tests::exact_interpreter_executes_hand_built_graph_and_commits_trace`,
`ir::tests::exact_interpreter_executes_unary_tier_b_ops`,
`ir::tests::exact_interpreter_executes_shaping_comparison_generators_and_where`,
`ir::tests::exact_interpreter_executes_mean_cast_concat_and_stack`,
`ir::tests::exact_interpreter_supports_field_scalar_params`,
`ir::tests::exact_interpreter_rejects_deferred_ops`,
`ir::tests::graph_validation_rejects_inconsistent_exact_tier_b_shapes`,
`ir::tests::graph_json_roundtrips_narrow_integer_dtypes`,
`ir::tests::quantization_vocabulary_admits_exact_quantization_ops`,
`ir::tests::exact_interpreter_executes_per_channel_int8_quantize_dequantize`,
`ir::tests::exact_interpreter_executes_packed_int8_quantize_dequantize`,
`tensor::tests::narrow_integer_tensors_enforce_canonical_ranges_and_commit_dtype`,
`tensor::tests::random_narrow_integer_tensors_are_canonical`,
`ir::tests::linear_training_step_graph_validates_and_commits_shapes`,
`jobs::tests::matmul_receipt_commits_to_outputs`, and
`jobs::tests::linear_receipt_commits_to_learning_step`,
`chain::tests::chain_engine_applies_profile_neutral_commands`,
`chain::tests::chain_engine_registers_valid_canonical_program_body_without_job`,
`chain::tests::chain_engine_rejects_invalid_or_conflicting_program_bodies`,
`app::runtime_services::tests::startup_program_hydration_registers_state_rooted_program_bodies`,
`storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, and
`p2p::service::tests::libp2p_service_fetches_registered_program_body`.

The local reference also has a deterministic `F_p` conformance vector gate for the current executable
admitted op surface used by TensorOp and LinearTrainingStep: field `add`, `sub`, `mul`, `scalar_mul`,
`identity`, `neg`, signed-residue `abs`, `sign`, `relu`, field/integer and fixed-point `round`, `transpose`,
`reshape`, `broadcast`, `reduce_sum`, `mean`, `concat`, `stack`, `matmul`, `full`, `arange`, and
`quantize_int8_per_channel`, `dequantize_int8_per_channel`, `quantize_pack_int8`,
`unpack_dequantize_int8`, comparison masks (`gt`, `lt`, `ge`, `le`, `eq`), `where`, and `mse_loss`, plus
scale-aware fixed-point `cast`/`round` vectors using per-input and expected output dtype/scale metadata,
multi-output expected tensors for exact quantize scale output, field-order comparison and selection
vectors, and byte-exact packed payload vectors. The suite has a stable hash, the CPU reference backend must pass it through
`runtime::backend_conformance_profile`, and `verify_tensor_op` / `verify_linear_training_step` reject
otherwise-valid receipts when their required conformance profile is unavailable or missing an op.
Focused evidence:
`conformance::tests::conformance_vectors_are_stable_and_cover_current_ops`,
`conformance::tests::cpu_reference_passes_all_vectors`,
`conformance::tests::required_conformance_gates_current_jobs`,
`verify::tests::graph_verifier_accepts_fixed_point_rescale_receipt`,
`verify::tests::graph_verifier_accepts_quantize_dequantize_receipt`,
`verify::tests::graph_verifier_accepts_packed_quantize_dequantize_receipt`,
`verify::tests::graph_verifier_accepts_comparison_where_receipt`,
`runtime::tests::cpu_backend_reports_passing_conformance_profile`,
`runtime::tests::gpu_backend_reports_device_and_requires_cuda_kernels`,
`verify::tests::tensor_op_verifier_requires_conformance_profile`, and
`verify::tests::linear_training_verifier_requires_conformance_profile`.

Remaining Tensor IR/conformance gaps: role-runtime production for arbitrary graph-backed jobs,
const-blob fetching, fixed-point arithmetic scale policy beyond `cast`/`round`, low-level packed tensor
storage/chunking APIs, index-consistency proofs for `gather`/`scatter`/`embedding`, additional mixed-dtype
conformance vectors, and CUDA conformance evidence when `cuda-kernels` is not compiled in this environment.
Tier-C, index-consistency, transcendental, and order-dependent ops remain registry vocabulary only and are
still gated out of consensus when their verifier class is deferred.

## Local CPU Compose Gate

[`local_cpu_testnet_spec.md`](local_cpu_testnet_spec.md) maps the first local deployment milestone to the
checked `deploy/tensorvm/local-cpu/` bundle. The bundle is guarded by
`local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape`, and the runnable gate is:

```bash
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml config --quiet
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml build
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml up --wait
deploy/tensorvm/local-cpu/scripts/check-local-testnet.sh
deploy/tensorvm/local-cpu/scripts/check-rolling-restart-continuity.sh
docker compose -f deploy/tensorvm/local-cpu/docker-compose.yml down -v
```

The checked run starts 10 miner containers and 5 validator containers, verifies 15 distinct operator IDs,
15 distinct stable libp2p peer IDs, and 15 distinct node multiaddrs, requires 15 libp2p-ready nodes,
requires 10 CPU-ready miners and zero CUDA-required miners, requires all miners to run `tvmd miner run`,
all validators to run `tvmd validator run`, and `validator-00` to be the single local timed producer as
reported by `runtime_command` and role status, requires live role-loop counters, the `local_cpu` chain
profile, decoded network-event ingestion, decoded block, job, receipt, and attestation payload application, and
network-applied block counters for every non-producer, plus observed job/receipt/attestation/block payload gossip
and block-vote gossip counters for every counted operator, validator-owned block-vote submission, and
non-producer block-vote ingestion/application, verifies the seeded local CPU
chain has 10 settled receipts, settled matmul work, settled LinearTrainingStep work, positive pending
receipt reward claims for miners, full finality and data availability,
checks that the host gateway exposes the seeded chain head, checks the host gateway routes with the local
auth token, checks the standalone explorer service on port 8080, verifies the
explorer page opens a WebSocket to the TensorVM `/explorer/ws` data endpoint, waits for live post-startup
height, block, job, receipt, settled-receipt, model-count, attestation-count, and pending receipt-reward
advancement so the live producer must settle at least one LinearTrainingStep and create new validator/miner
claims after the seed, requires live receipt details to expose validator attestation counts and more than
the seeded count of both `tensor_op` and `linear_training_step` primitive receipts, requires finalized
live `tvmd node block` views to expose block-height receipt IDs and primitive counts for both TensorOp
and LinearTrainingStep work, fetches a live tensor
descriptor, row, chunk, and opening through the TensorVM node, reruns Gate 0 from the checker,
verifies the local-only evidence boundary, requires all 15 operator stores to report the same finalized
common-head block hash through `tvmd node block`, selects a non-producer's latest finalized p2p-observed
block-payload head from the block-payload gossip set and requires every operator to return the matching finalized block hash and state
root while reporting block-vote finality evidence, a nonempty block-log root, and observed block-vote
gossip, and uses
`check-rolling-restart-continuity.sh` to run the restart-continuity gate one service at a time across every
counted operator, proving each restarted service keeps its libp2p peer ID, preserves the pre-restart
finalized common head and state root, advances height/block-count/state-root/block-log-root evidence, and
continues finalizing blocks after restart.

## Acceptance Criteria

| # | Criterion | Evidence |
| --- | --- | --- |
| 1 | Miners execute deterministic tensor jobs. | Current TensorOp and LinearTrainingStep jobs are backed by validated content-addressed IR graph IDs, and their receipts now derive `trace_root` from exact execution of the canonical TensorGraph op traces. The exact IR interpreter foundation can execute hand-built validated graphs over the currently implemented deterministic tensor ops, including shaping, generators, comparisons, and `where`, and commits per-op output roots plus a trace root, but role-owned chain admission still uses the current canonical job types rather than arbitrary graph-backed jobs. Evidence: `ir::tests::matmul_graph_has_stable_canonical_json_and_graph_id`, `ir::tests::linear_training_step_graph_validates_and_commits_shapes`, `ir::tests::exact_interpreter_executes_hand_built_graph_and_commits_trace`, `ir::tests::exact_interpreter_executes_shaping_comparison_generators_and_where`, `ir::tests::exact_interpreter_supports_field_scalar_params`, `ir::tests::exact_interpreter_rejects_deferred_ops`, `jobs::tests::matmul_receipt_commits_to_outputs`, `jobs::tests::linear_receipt_commits_to_learning_step`, `miner::tests::miner_solves_matmul_and_serves_tensors`, `miner::tests::miner_solves_linear_step_and_serves_intermediates`, `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul` |
| 2 | Validators verify block-eligible matmul jobs with full-output Freivalds or bounded equivalent. | `verify::full_freivalds`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption`, `verify::tests::tensor_op_verifier_rejects_metadata_and_shape_mismatches`, `validator::tests::validator_verifies_matmul_from_tensor_server` |
| 3 | Row-sampled checks are audits unless false-accept bounds are documented. | `verify::row_sample_detection_probability`, `study::row_sampling_study`, `study::tests::row_sampling_study_blocks_sparse_row_sampled_only_acceptance` |
| 4 | Blocks are produced by validators winning useful-verification PoW over deterministic settled-receipt blockspace. | Partially implemented locally. `TensorBlock` now commits `settled_receipt_set_root`, block-level `checks_root`, beacon, proposer reward amount, difficulty target, and nonce; `chain::proposer` selects registered validators and ignores miner TensorWork; selected receipts are marked included once; `submit_block_vote` validates known blocks with strict parent-root checks before counting votes; bounded network-visible block-check challenge payloads can disprove a committed check leaf through the shared p2p/node event path, retry while the challenged block is missing, apply through `ChainCommand::SubmitBlockCheckChallenge`, throttle the proposer, and enqueue delayed challenger reward claims; validator role loops submit and gossip explicit block votes so append and finality are separate runtime events; local synthetic scheduling now publishes deterministic jobs only, while the validator role tick observes settled receipts with local tensor artifacts and attestations before submitting useful proposals. Validator proposal is no longer gated by `local_synthetic_producer`; a configured validator proposer can build a useful block from already accepted settled state even when synthetic job production is disabled. Validator runtime status separates useful settled-receipt block proposals from empty fallback proposals, and the local checker requires positive useful proposal, proposed-receipt, artifact-ready, attested-receipt, and delayed pending proposer reward evidence from the sole validator producer. Evidence: `chain::tests::proposer_selection_ignores_tensorwork`, `chain::tests::block_roots_commit_to_canonical_receipts_checks_attestations_and_state_values`, `chain::tests::block_votes_reject_invalid_useful_pow_and_checks_root`, `chain::tests::produced_blocks_mark_selected_settled_receipts_included_once`, `chain::tests::block_check_challenge_voids_pending_reward_and_throttles_proposer`, `p2p::wire::tests::block_check_challenge_payloads_roundtrip_and_reject_malformed_edges`, `node::payload_application::tests::block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges`, `node::pending_payloads::tests::pending_payloads_retry_keeps_pending_payloads`, `node::tests::block_payload_application_admits_next_head_and_rejects_bad_edges`, `p2p::tests::block_vote_payloads_roundtrip_and_reject_malformed_edges`, `node::runtime_state::tests::runtime_state_tracks_loop_counters`, `tvmd` binary `tests::validator_role_block_vote_submission_finalizes_only_through_votes`, `tvmd` binary `tests::producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks`, `tvmd` binary `tests::validator_proposer_tick_runs_without_synthetic_producer_gate`, `tvmd` binary `tests::network_applied_receipt_and_attestation_make_validator_proposal_useful`, `tvmd` CLI `role_run_commands_serve_through_role_specific_surfaces`, `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape`, `storage::tests::block_log_store_appends_loads_and_detects_tampering`, `localnet::tests::synthetic_cpu_round_settles_work_and_advances_finalized_chain`, and service-block evidence fields. Remaining gaps: full verifier-transcript fraud proofs, exact parent-state snapshots and child-state apply theorem, deterministic live bad-block challenge generation, multi-validator proposer competition/fork-choice policy, and a fresh full Docker proof after the current `/health` blocker clears. |
| 5 | Rewards are distributed by verified settled TensorWork. | Miner and validator receipt rewards are pending claims until maturity, proposer rewards are delayed, successful block-check challenger bounties become pending challenge reward claims before spendability, configured mandatory validator audits delay the audited validator's reward until the audit deadline while missed or contradictory audits void that delayed reward, and generic/faucet reward credits now enter a state-rooted pending credit ledger before any spendable reward balance is credited. Block `reward_root` now commits the child state's spendable rewards plus pending proposer, receipt, challenge, and credit ledgers, so delayed reward-finality claims are block-root-bound instead of visible only through the broader state root; blocks carrying the old spendable-only root are rejected. Normal block transitions now apply the current block's receipt-inclusion delays and slash/audit voiding first, then sweep all still-mature proposer, receipt, challenge, and credit reward claims through the shared chain transition before adding the new block's proposer reward claim; producer and non-producer block application recompute the same matured-release child state, while voided proposer/receipt/challenge claims are pruned without credit. Validator-owned useful block proposals and empty fallback proposals both enter the delayed proposer reward ledger with the explicit full reward-maturity delay before any spendable proposer balance is credited; fallback blocks remain distinguishable and carry the reduced proposer claim amount, and pending proposer reward state/root/storage no longer carry a later-useful-block unlock latch. Registered validator roles now submit signed audit reports and non-producers ingest/retry those bounded p2p payloads; block-check challenge payloads now make the existing challenge clawback/pending-bounty path reachable through shared network ingestion. Full auditor-selection policy, appeals, unified formal reward-claim object/status, and bond calibration remain outside the local economics invariant. Evidence: `chain::tests::chain_settles_valid_tensorwork_and_rewards_participants`, `chain::tests::generic_credit_rewards_release_only_after_maturity`, `chain::tests::produced_blocks_delay_receipt_rewards_from_inclusion_height`, `chain::tests::block_check_challenge_voids_pending_reward_and_throttles_proposer`, `chain::tests::reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury`, `chain::tests::fallback_proposer_reward_uses_explicit_maturity_delay`, `chain::tests::block_reward_root_rejects_spendable_only_root_when_pending_rewards_exist`, `chain::tests::reward_root_commits_to_all_pending_reward_ledgers`, `chain::tests::block_transition_releases_matured_rewards_without_manual_command`, `chain::tests::release_matured_proposer_rewards_sweeps_voided_claims_without_credit`, `chain::tests::matured_proposer_reward_releases_after_full_maturity_delay`, `chain::tests::validator_audit_result_slashes_contradicted_attestation_and_voids_reward`, `rpc::tests::node_rpc_serves_explorer_telemetry_and_faucet_routes`, `p2p::tests::validator_audit_report_payloads_roundtrip_and_reject_malformed_edges`, `p2p::wire::tests::block_check_challenge_payloads_roundtrip_and_reject_malformed_edges`, `node::tests::validator_audit_report_payload_application_reports_pending_applied_and_invalid_edges`, `node::payload_application::tests::block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges`, `node::tests::network_event_driver_applies_validator_audit_report_payloads_for_non_producers`, `pending_payloads::tests::pending_payloads_retry_keeps_pending_payloads`, `tvmd` binary `tests::producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks`, `tvmd` binary `tests::validator_role_audit_report_submission_observes_assignments_and_skips_duplicates`, `tvmd` binary `runtime_persistence::role_runtime_mutating_rpc_persists_chain`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, and `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`. |
| 6 | Validation randomness is unbiasable after receipt roots are committed. | Partial. Admitted receipts now persist a `ReceiptRandomnessAnchor` with receipt-time finalized beacon round/randomness and assignment seed, and validator assignment plus `Chain::validation_seed` use that anchor even after later blocks advance the finalized beacon. Evidence: `chain::Chain::validation_seed`, `chain::tests::validation_seed_is_bound_to_finalized_randomness_and_receipt`, `chain::tests::admitted_receipt_validation_randomness_is_anchored_at_submission`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, and `study::assess_randomness`. Remaining gap: full VRF/drand construction and external commit-reveal lifecycle. |
| 7 | Invalid tensor outputs are rejected in dense and sparse corruption tests. | `verify::tests::tensor_op_verifier_rejects_bad_output`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption` |
| 8 | LinearTrainingStep receipts validate forward/backward/error/update structure. | `verify::verify_linear_training_step`, `verify::tests::linear_training_verifier_rejects_metadata_and_commitment_mismatches`, `ir::tests::frozen_registry_declares_verifier_class_for_every_op`, `vm::tests::linear_backward_and_sgd_match_equations`, `jobs::tests::linear_receipt_commits_to_learning_step` |
| 9 | Sparse corruptions in `dY` and `W_next` are rejected with stated probability. | Current LinearTrainingStep verification uses random-linear checks for `dY = Y - T` and `W_next = W - lr * grad_W`; registry metadata now distinguishes random-linear Tier-B relations from deterministic replay and deferred index-consistency ops. Evidence: `verify::tests::linear_training_verifier_rejects_sparse_error_poisoning`, `verify::tests::linear_training_verifier_rejects_sparse_weight_poisoning`, `ir::tests::frozen_registry_declares_verifier_class_for_every_op`, and `ir::tests::index_ops_require_index_consistency_and_are_not_consensus_admitted`. |
| 10 | Honest miners produce identical output roots. | `runtime::tests::gpu_backend_reports_device_and_requires_cuda_kernels`, `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul`, `runtime::tests::cpu_and_gpu_backends_match_linear_step`, `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, and `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops` under `--features cuda-kernels`, `chain::tests::redundant_agreement_quorum_is_required_before_settlement`, `scheduler::tests::miner_assignment_prefers_operator_separation`, `scheduler::tests::miner_assignment_falls_back_when_operator_diversity_is_insufficient` |
| 11 | Validators spend materially less compute than full recomputation. | `study::matmul_verification_cost_study`, `study::tests::matmul_verification_cost_is_lower_than_execution_for_mvp_shape`, `telemetry::estimated_verification_to_execution_ratio` |
| 12 | Tensor data availability exceeds 95% during active and retention windows. | `validator::tests::validator_attests_unavailable_when_server_lacks_tensor_roots`, `tensor_server::tests::tensor_server_retains_through_deadline_and_prunes_afterward`, `telemetry::data_availability_rate`; public-network measurement remains deployment-gated |
| 13 | Network runs for 7 consecutive days with independent nodes. | Not locally complete; `testnet::tests::local_testnet_bootstraps_required_public_shape`, `testnet::tests::public_testnet_preflight_manifest_reports_launch_readiness`, `testnet::tests::deployed_public_testnet_preflight_example_rejects_placeholder_domains`, `testnet::tests::docs_public_testnet_preflight_manifest_rejects_placeholder_domains`, `testnet::tests::public_testnet_preflight_manifest_rejects_malformed_input`, `cli::tests::execute_reference_cli_command_reports_miner_and_validator_readiness`, `cli::tests::validate_public_testnet_preflight_manifest_reports_launch_readiness`, `tvmd` binary `tests::docs_public_testnet_preflight_command_reports_pending_status`, `tvmd` binary `tests::docs_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::documented_public_testnet_preflight_command_reports_pending_status`, `tvmd_cli::generated_public_testnet_preflight_manifest_reports_ready`, `tvmd_cli::documented_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::generated_public_evidence_manifest_round_trips_through_tvmd_validator`, `tvmd_cli::service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces`, `p2p::tests::peer_book_store_upserts_bootstrap_records_with_peer_ids`, `rpc::tests::node_rpc_serves_head_and_blocks`, `rpc::tests::node_rpc_serves_explorer_telemetry_and_faucet_routes`, `testnet::tests::public_testnet_run_evidence_requires_independent_external_operators`, `testnet::tests::public_testnet_run_evidence_requires_production_runtime_and_reachable_services`, `testnet::tests::public_testnet_evidence_bundle_requires_publication_and_audit_records`, `testnet::tests::public_testnet_evidence_manifest_parses_into_bundle`, `testnet::tests::deployed_public_testnet_evidence_example_is_parseable_but_not_full_spec`, `testnet::tests::docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec`, `testnet::tests::public_testnet_evidence_manifest_rejects_malformed_input`, and `testnet::tests::public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes` validate the local launch preflight plus service-launch config and health/content endpoints, checked spec-path pending manifests and deploy preflight/evidence examples with planned public content paths, actual `tvmd` file-reading and process invocation behavior for the documented pending-manifest commands, process-generated launch-ready external-addressed preflight manifest validation from disk, process-generated short-run evidence-manifest assembly from signed `tvmd public evidence ...` generator commands that validates from disk as independently checkable without setting the full-spec flag, bounded process-level service init/peer-add/readiness/serve lifecycle with mandatory libp2p startup from the initialized node store and durable peer book, unauthenticated request rejection, authenticated `/health`, `/rpc/health`, `/explorer/health`, `/faucet/health`, `/telemetry/health`, process-level signed service-health generation from reached RPC/explorer/faucet/telemetry health responses, state-root-bearing `/chain/head`, `/epoch/current`, `/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, `/telemetry/dashboard`, mutable `/tx`, `/receipt`, and `/attestation` submissions, registered miner/validator state read-back, captured `/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response-body evidence generation through matching `evidence service content-bytes` and `evidence service content-file` CLI outputs, process-derived local libp2p peer/protocol data accepted only when bound to an external public multiaddr and then summarized/artifact-bound from its network-runtime observation root, the same process-derived data rejected as public network-observation evidence when bound to loopback, exact query-free service URL path enforcement, and placeholder-domain rejection, signed publication/auditor-record/run-window/node-heartbeat/operator-attestation CLI generation and invalid argument rejection, service peer-book bootstrap seeding with peer-ID-preserving `/p2p/<peer-id>` dial addresses, service-health and service-content CLI manifest-line generation, byte-derived and file-derived service-content root generation, plus invalid argument rejection, signed production-libp2p network-observation CLI generation and invalid argument rejection including malformed DNS-label and single-label DNS multiaddrs, signed supporting-record summary generation, signed external supporting-record artifact locator generation, signed artifact locator generation from derived aggregate roots, plus deterministic root aggregation for block/finality/network-runtime/data-availability/invalid-work/reward-settlement evidence, evidence gate for signed 7-day wall-clock run-window evidence, expected block count, distinct external operators, signature-verified heartbeat summaries, run continuity, finality, data availability, invalid-work rejection, reward-settlement records, production libp2p runtime use, signed per-operator production libp2p network-observation records exactly matching counted public operators and aggregating to signed network-runtime summary roots, deployed RPC/explorer/faucet/telemetry service reachability with signed health summaries and signed content roots bound to external HTTPS URLs, matching and distinct endpoint IDs, distinct service-content roots, and the required content paths, external public evidence publication URI validation including special-use DNS and single-label DNS rejection, verified manifest publication signatures, signed independent-auditor records, signed block/finality/data-availability/invalid-work/reward-settlement summary roots, signed operator-attestation-derived external-operator evidence, independently checkable evidence-bundle publication, and manifest parsing |
| 14 | Zero-receipt epochs have a tested stake-weighted PoW-skip fallback path. | Partial. Empty canonical blockspace produces explicit `PowSkipFallback` blocks, validator role fallback proposals now carry reduced proposer claims with the full reward-maturity delay before release, and validator-owned zero-work liveness is covered by `study::tests::zero_work_liveness_study_produces_blocks_from_validators` plus `chain::tests::fallback_proposer_reward_uses_explicit_maturity_delay`. Remaining gap: full stake-weighted fallback rotation/timeout policy for empty canonical blockspace. |
| 15 | Reward concentration, validator disagreement, and data withholding are reported. | `telemetry::TelemetrySnapshot`, `study::tensorwork_concentration`, `study::data_withholding_study`, `study::collusion_risk_assessment`, `watcher::ChainWatcher`, `telemetry::tests::telemetry_reports_block_timing_and_concentration`, `telemetry::tests::telemetry_reports_security_compute_and_economic_success_metrics`, `telemetry::tests::telemetry_reports_hardware_classes_and_gpu_utilization`, `telemetry::tests::telemetry_reports_linear_receipt_bandwidth_and_missing_job_edges`, `watcher::tests::watcher_reports_invalid_receipts_and_data_withholding`, `watcher::tests::watcher_flags_validator_misconduct_in_audited_state`, `watcher::tests::watcher_flags_malformed_attestation_evidence`, `watcher::tests::watcher_reports_conflicting_linear_transitions` |

## Non-Local Gaps

- Optional native CUDA kernel support exists behind `--features cuda-kernels` and covers field matmul plus
  linear-step sub/scalar/transpose/squared-error kernels checked against canonical CPU outputs locally.
  Miner CLI startup reports CPU reference readiness for `--device cpu` and rejects `--device cuda:N`
  unless CUDA kernels are compiled and the requested device is available.
  Production GPU miner packaging and a broader optimized kernel suite remain outside the local reference
  crate.
- Public 7-day independent-node testnet evidence is not available in this repository; typed evidence
  validation exists for checking it when a real external run is available, including signed wall-clock
  run-window evidence, invalid-work rejection evidence, reward-settlement records, production libp2p
  runtime use with signed per-operator network-observation records exactly matching counted public
  operators and aggregating to the signed network-runtime root,
  an exact one-signature manifest publication count for the current manifest format, exactly one signed
  external artifact locator for each required raw supporting-record kind, one-to-one live operator/address
  matching for counted public participants with criteria-aware quota selection, disjoint
  miner/validator operator IDs and node addresses, auditor IDs distinct from the manifest signer with
  auditor observations at or after the signed run end and valid signed
  auditor-record counts exactly matching `independent_auditor_count`,
  operator-attestation and service-content timestamps inside the signed run window, observed-block
  coverage for node heartbeat and service health counts, internally consistent finality/data-availability
  counters, exact run-derived supporting-record summary counts, non-public IP literal rejection,
  special-use DNS and single-label DNS rejection, plus
  malformed HTTPS authority rejection for public endpoints, raw-whitespace rejection for external evidence
  URLs and content-addressed identifiers including exact untrimmed manifest URI/path fields, HTTPS evidence
  URI concrete-path enforcement with root-only, query, and fragment rejection, duplicate scalar manifest-field rejection,
  whitespace-padded field-key and scalar-value rejection, duplicate supporting-record root rejection, repeated node-address count rejection, exact service URL path matching with root-only, query, and fragment rejection, exact operator-attestation counts with no missing, duplicate, extra, or overreported records,
  full-spec flag rejection for relaxed local harness criteria, well-formed `ipfs://`/`ar://` identifier
  validation with traversal/query/fragment path rejection, and deployed public-service reachability with exactly
  one service-health and one service-content record per deployed service kind, distinct endpoint IDs, and distinct content
  roots with at least 64 observed bytes bound to external HTTPS URLs. A local launch
  preflight manifest is documented in
  [`public_testnet_preflight.md`](public_testnet_preflight.md), requires a CUDA-ready miner count matching
  the planned miner count plus a libp2p-ready node count matching planned miners and validators before
  deployment readiness can pass, rejects whitespace-padded preflight `service=...` comma-separated values,
  requires exactly one ready RPC, explorer, faucet, and telemetry preflight service plan, rejects duplicate
  or extra preflight service plans, and deployment templates plus checked preflight and non-full-spec post-run evidence example manifests
  live under `deploy/tensorvm/`, with
  `testnet::tests::public_deployment_templates_require_libp2p_and_https_surfaces` guarding the env,
  systemd, and nginx templates for mandatory libp2p startup, durable data-dir use, auth-token wiring,
  TLS proxying, and the required public HTTPS surfaces,
  `testnet::tests::public_deployment_runbook_records_required_evidence_flow` guarding
  `deploy/tensorvm/RUNBOOK.md` coverage of preflight status flags, evidence generator commands, daily
  checkpoint requirements, post-run validation flags, publication artifacts, and the explicit no-real-run
  blocker, `testnet::tests::public_deployment_readme_records_scaffold_boundary_and_operator_flow`
  guarding the deployment README's scaffold file list, public service routes, minimal operator flow,
  evidence commands, and non-evidence boundary, signed public
  libp2p network-observation CLI generation rejects missing or zero TCP listen ports plus non-public and
  single-label DNS multiaddrs, `evidence network from-service-log` derives signed observation records
  from captured `tvmd node serve` logs while still requiring public listen multiaddrs, process-level
  network-runtime observation roots can be summarized and artifact-bound from external-addressed records or
  saved raw-record files with exact unpadded root-list parsing and full signed network-observation line
  validation before aggregation, and
  file-derived block/finality/data-availability/invalid-work/reward supporting record summaries validate
  typed raw-record fields, including reward-settlement participant IDs, before exact-line hashing while
  rejecting whitespace-padded records and empty fields,
  `evidence run window-file` derives signed run-window manifest lines from saved
  contiguous per-block observation files while rejecting duplicate blocks, gaps, zero timestamps,
  decreasing timestamps, unsupported lines, and whitespace-padded records, `evidence node heartbeat-file`
  derives signed node-heartbeat manifest lines from saved contiguous per-block observation files while
  rejecting duplicate blocks, gaps, identity mismatches, unsupported lines, and whitespace-padded records,
  `evidence service health-file` derives signed
  service-health manifest lines from saved contiguous per-block observation files while rejecting duplicate
  blocks, gaps, unsupported lines, and whitespace-padded records, service-health evidence rejects
  reachable counts above signed health-check counts, repeated public-evidence manifest records reject
  whitespace-padded comma-separated values, service health/content evidence must use matching HTTPS
  authorities for each endpoint ID and reject extra service-health or service-content records, and the
  required post-run evidence-bundle shape is
  documented in
  [`public_testnet_evidence.md`](public_testnet_evidence.md), but no complete external bundle is linked yet.
- Public production libp2p run evidence, HTTP deployment, full durable database, and deployed browser web
  services remain outside the local reference crate. The crate has mandatory rust-libp2p runtime wiring with
  TCP/TLS/Yamux swarm construction, Gossipsub subscriptions, Identify, Kademlia discovery/address
  registration, JSON request-response protocols, `tvmd node peer add` bootstrap seeding,
  `tvmd node check` startup checks for the mandatory libp2p control-plane runtime,
  `tvmd node serve` startup of the same runtime, `tvmd miner run` and `tvmd validator run`
  role-specific surfaces that Compose uses for counted operators, durable bootstrap peer-book persistence
  with peer-ID-preserving DNS/TCP dial multiaddrs and bootstrap redial, generic HTTP request reading, a socketed stdlib RPC server with auth/body/rate-limit policy checks,
  explorer data RPC endpoints, `/explorer/ws` WebSocket polling for browser explorers,
  `tvmd node status` durable node-store reporting,
  telemetry/faucet RPC endpoints, local browser-facing explorer/telemetry/faucet HTML pages,
  `tvmd node init/peer add/check/serve` launch validation with required libp2p listen multiaddrs, checked deployable
  systemd/env/nginx templates, a documented mandatory-libp2p networking choice, and a restartable reference
  `NodeStore` data
  directory with consistency-checked snapshot, append-only block-log, full-chain state, and peer-book
  persistence. The local CPU checker now also requires all 15 operator node stores to report role status,
  runtime command, live role-loop counters, local-producer mode, decoded network-event ingestion, decoded
  job/receipt/attestation/block-vote payload application, network-applied block counters for non-producers,
  validator block-vote submission, block-vote ingestion/application for non-producers, real libp2p
  connected-peer counts, active chain profile, live chain counters,
  advancement past the shared seed, finalized live TensorOp and LinearTrainingStep block-view evidence,
  the same first live finalized block hash, and the same finalized common-head block hash through
  `tvmd node block`. The restart-continuity gate captures
  pre/post peer IDs, heights, block counts, and common-head hashes around actual Compose restarts, while
  service init validates full node-store consistency and repairs torn snapshot/block-log state from
  `chain.state`.
- Instrumented line coverage has been generated with Tarpaulin; see `tarpaulin_report.md`.
  Branch coverage is not reported because the installed Tarpaulin version lists branch coverage as not implemented.
