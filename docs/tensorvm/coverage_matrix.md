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
graphs over the currently implemented tensor runtime ops (`matmul`, exact Tier-A matrix-contraction `einsum`, broadcast-aware `add`/`sub`/`mul`,
exact field modular-inverse and `Fixed32` reciprocal `div`, `scalar_mul`, `transpose`, explicit-dim `sum`/`reduce_sum`, `identity`, `neg`, signed-residue
`abs`/`sign`/`relu`, mixed-scale fixed-point `add`/`sub` with RHS-to-lhs/output half-even rescale, mixed-scale fixed-point `mul` with half-even product rescale, fixed-point reciprocal `div` with lhs/output scale half-even quotient rescale, `Fixed32` `matmul` with signed fixed-order accumulation and one final lhs/output scale rescale, fixed-point scale-aware half-even `round`, `reshape`, `broadcast`, single-output
structural `squeeze`/`unsqueeze`/`slice`/`tril`/`triu`, comparisons `gt`/`lt`/`ge`/`le`/`eq`, `where`,
field-order `clamp`, `mean`, `cast`, `concat`, `stack`, `full`, and `arange`). Tensor
metadata now also carries canonical `int8`, `uint8`, and `bool` dtype tags through tensor commitments,
shared codecs, p2p tensor payloads, and canonical IR JSON. The frozen registry now admits exact
`quantize_int8_per_channel` and `dequantize_int8_per_channel` execution: quantize uses deterministic
per-channel integer scales, round-half-even division, and int8 clamping, while dequantize multiplies by a
rank-1 scale tensor and rejects ambiguous inferred channel dimensions. Byte-packed
`quantize_pack_int8`/`unpack_dequantize_int8` are also admitted with a tensor-owned canonical flat `uint8`
payload API containing `TVQ8` magic/version bytes, rank, quantization axis, output scale, original shape,
per-channel signed 64-bit scales, and row-major int8 payload bytes, with bounded length calculation and
shared encode/decode validation used by IR replay and conformance. The tensor layer now also exposes
packed payload construction/decode as a first-class `Uint8` tensor artifact API, so those payloads carry
normal descriptor, chunk, and Merkle-opening evidence for public tensor serving. Interpreter output includes named
output tensors, per-op output commitment roots, a Merkle `trace_root`, and per-op trace openings verified
against that root. The libp2p control plane now includes a bounded trace-opening request-response stream
for sampling openings by `(trace_root, op_index)`; deferred Tier-C ops and
admitted registry ops without implemented exact replay return explicit execution errors instead of being
silently accepted. Local synthetic production now emits a deterministic graph-backed exact Tier-B job
(`add` then `relu`), registers the graph body and input tensors, and miner/validator role helpers can
submit and attest the `GraphExecution` receipt from node-local program/tensor artifacts. Focused
evidence: `ir::tests::matmul_graph_has_stable_canonical_json_and_graph_id`,
`ir::tests::graph_validation_rejects_bad_structure`,
`ir::tests::graph_validation_rejects_op_metadata_mismatches`,
`ir::tests::tier_c_vocabulary_is_carried_but_not_consensus_admitted`,
`ir::tests::frozen_registry_declares_verifier_class_for_every_op`,
`ir::tests::index_ops_require_index_consistency_and_are_not_consensus_admitted`,
`ir::tests::exact_interpreter_executes_hand_built_graph_and_commits_trace`,
`ir::tests::exact_interpreter_executes_unary_tier_b_ops`,
`ir::tests::exact_interpreter_executes_fixed32_add_sub_with_mixed_scales`,
`ir::tests::exact_interpreter_executes_fixed32_mul_with_scale_rescale`,
`ir::tests::exact_interpreter_executes_fixed32_mul_with_mixed_scales`,
`ir::tests::exact_interpreter_executes_fixed32_matmul_with_mixed_scales`,
`ir::tests::exact_interpreter_executes_fixed32_div_with_scale_rescale`,
`ir::tests::graph_validation_rejects_unsupported_matmul_dtype`,
`ir::tests::exact_interpreter_executes_shaping_comparison_generators_and_where`,
`ir::tests::exact_interpreter_executes_clamp`,
`ir::tests::exact_interpreter_executes_single_output_structural_ops`,
`ir::tests::exact_interpreter_executes_mean_cast_concat_and_stack`,
`ir::tests::exact_interpreter_supports_field_scalar_params`,
`ir::tests::exact_interpreter_rejects_deferred_ops`,
`ir::tests::graph_validation_rejects_inconsistent_exact_tier_b_shapes`,
`ir::tests::graph_json_roundtrips_narrow_integer_dtypes`,
`ir::tests::quantization_vocabulary_admits_exact_quantization_ops`,
`ir::tests::exact_interpreter_executes_per_channel_int8_quantize_dequantize`,
`ir::tests::exact_interpreter_executes_packed_int8_quantize_dequantize`,
`tensor::tests::packed_int8_tensor_artifact_exposes_descriptor_chunks_and_openings`,
`tensor::tests::narrow_integer_tensors_enforce_canonical_ranges_and_commit_dtype`,
`tensor::tests::random_narrow_integer_tensors_are_canonical`,
`tensor::tests::fixed32_add_sub_rescale_rhs_to_lhs_scale_half_even`,
`tensor::tests::fixed32_multiply_rescales_to_input_scale_half_even`,
`tensor::tests::fixed32_multiply_rescales_mixed_scales_to_lhs_scale_half_even`,
`tensor::tests::fixed32_matmul_accumulates_then_rescales_to_lhs_scale_half_even`,
`tensor::tests::fixed32_division_rescales_to_lhs_scale_half_even`,
`ir::tests::linear_training_step_graph_validates_and_commits_shapes`,
`jobs::tests::matmul_receipt_commits_to_outputs`, and
`jobs::tests::linear_receipt_commits_to_learning_step`,
`chain::tests::chain_engine_applies_profile_neutral_commands`,
`chain::tests::chain_engine_registers_valid_canonical_program_body_without_job`,
`chain::tests::chain_engine_rejects_invalid_or_conflicting_program_bodies`,
`localnet::tests::synthetic_cpu_round_settles_work_and_advances_finalized_chain`,
`app::validator_role::tests::role_runtime_submits_and_attests_graph_execution_from_local_artifacts`,
`app::runtime_services::tests::startup_program_hydration_registers_state_rooted_program_bodies`,
`storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, and
`p2p::service::tests::libp2p_service_fetches_registered_program_body`. Inbound external graph jobs whose
program bodies have not arrived locally now stay pending in the shared node payload path instead of being
counted invalid, and focused libp2p evidence fetches the graph body plus input tensor artifacts by
request-response before applying the same external graph job payload. Evidence:
`node::payload_application::tests::graph_job_payload_waits_for_registered_program_body`,
`node::message_ingest::tests::network_event_driver_queues_graph_job_until_program_body_arrives`, and
`p2p::service::tests::libp2p_service_propagates_external_graph_job_artifacts`. Role/runtime graph
artifact resolution is now automatic at the remaining runtime boundaries: pending graph job payloads fetch
their missing program body through bounded `RequestProgram` before retry, miner roles fetch missing graph
input and `const_blob` tensor artifacts before execution, and validator roles fetch missing graph input,
output, and `const_blob` tensor artifacts before attestation. Evidence:
`network_payloads::network_ingest_fetches_pending_graph_job_program_before_retry`,
`miner_role::miner_role_fetches_remote_graph_inputs_and_const_blobs_before_execution`, and
`validator_role::validator_role_fetches_remote_graph_const_blobs_before_attesting`. Trace-opening sampling
evidence: `p2p::wire::tests::trace_opening_payloads_roundtrip_and_reject_malformed_edges`,
`p2p::service::tests::libp2p_service_fetches_trace_opening`, and
`p2p::node::tests::local_testnet_libp2p_swarms_exchange_gossip_and_request_response`. Explorer API
evidence: `rpc::tests::websocket::explorer_websocket_views_cover_chain_collections_and_bad_commands`
now covers `graph_execution` in WebSocket jobs and receipts alongside TensorOp and LinearTrainingStep.

The local reference also has a deterministic `F_p` conformance vector gate for the current executable
admitted op surface used by TensorOp and LinearTrainingStep: field `add`, `sub`, `mul`, `div`, `scalar_mul`,
`identity`, `neg`, signed-residue `abs`, `sign`, `relu`, field/integer and fixed-point `round`, `transpose`,
`reshape`, `broadcast`, `sum`, `reduce_sum`, `mean`, `clamp`, `squeeze`, `unsqueeze`, `slice`, `split`,
`tril`, `triu`, `concat`, `stack`, `matmul`, `einsum`, `full`, `arange`, and
`quantize_int8_per_channel`, `dequantize_int8_per_channel`, `quantize_pack_int8`,
`unpack_dequantize_int8`, comparison masks (`gt`, `lt`, `ge`, `le`, `eq`), and `where`, plus the explicit
auxiliary LinearTrainingStep verifier vector for `mse_loss`, plus
scale-aware fixed-point `cast`/`round`, mixed-scale `add`/`sub`, mixed-scale `mul`, `Fixed32` reciprocal `div`, and `Fixed32` `matmul` vectors using per-input and expected output dtype/scale metadata,
multi-output expected tensors for exact quantize scale output and dynamic-output `split`, exact
field modular-inverse and `Fixed32` reciprocal `div`, Tier-A matrix-contraction `einsum`, field-order comparison, selection, and clamp
vectors, row-major structural vectors, and byte-exact packed payload vectors. The suite has a stable hash, requires unique vector IDs, derives its admitted-op guard
from `ir::frozen_op_registry()`, requires every consensus-admitted op spelling to have vector and CPU
profile evidence, rejects non-admitted vector/profile entries unless they are explicitly marked auxiliary,
the CPU reference backend must pass it through `runtime::backend_conformance_profile`,
and `verify_tensor_op` / `verify_linear_training_step` reject otherwise-valid receipts when their required
conformance profile is unavailable or missing an op.
Focused evidence:
`conformance::tests::conformance_vectors_are_stable_and_cover_current_ops`,
`conformance::tests::conformance_vectors_cover_every_consensus_admitted_op`,
`conformance::tests::conformance_vectors_only_cover_admitted_or_auxiliary_ops`,
`conformance::tests::cpu_reference_passes_all_vectors`,
`conformance::tests::cpu_reference_passes_all_admitted_ops`,
`conformance::tests::cpu_reference_profile_matches_registry_and_auxiliary_boundary`,
`conformance::tests::required_conformance_gates_current_jobs`,
`verify::tests::graph_verifier_accepts_fixed_point_rescale_receipt`,
`verify::tests::graph_verifier_accepts_quantize_dequantize_receipt`,
`verify::tests::graph_verifier_accepts_packed_quantize_dequantize_receipt`,
`verify::tests::graph_verifier_accepts_comparison_where_receipt`,
`verify::tests::graph_verifier_accepts_clamp_receipt`,
`verify::tests::graph_verifier_accepts_single_output_structural_receipt`,
`verify::tests::graph_verifier_accepts_sum_receipt`,
`runtime::tests::cpu_backend_reports_passing_conformance_profile`,
`runtime::tests::gpu_backend_reports_device_and_requires_cuda_kernels`,
`verify::tests::tensor_op_verifier_requires_conformance_profile`, and
`verify::tests::linear_training_verifier_requires_conformance_profile`.

Remaining Tensor IR/conformance gaps: index-consistency proofs for `gather`/`scatter`/`embedding` and
CUDA conformance evidence when `cuda-kernels` is not compiled in this environment; fixed-scale comparison
masks and int8 selection now have mixed dtype/scale vectors. Automatic runtime referee witness generation
is covered locally for isolated
transcripts whose stored opening input roots match canonical graph replay, isolated transcripts that pass
their deadline without a referee witness now close by challenger-forfeit timeout without voiding the
responder receipt path, and trace-bisection admission now bounds worst-case midpoint rounds while rejecting
conflicting pending expectation overwrites.
Content-addressed `const_blob` refs now resolve through local tensor artifacts keyed by the declared
commitment URI, with shape/dtype/root checks during exact graph replay. Evidence:
`ir::tests::exact_interpreter_executes_const_blob_by_content_uri`,
`jobs::tests::graph_receipt_replay_supports_const_blob_artifacts`, and
`roles::tests::cpu_roles_execute_and_verify_graph_jobs_with_const_blob`.
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
all validators to run `tvmd validator run`, `validator-00` to be the single local timed synthetic job
producer, and all five validators to be validator block proposers with chain-visible cooldown status as reported
by readiness and role status, requires live role-loop counters, the `local_cpu` chain
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
the seeded count of `tensor_op`, `linear_training_step`, and `graph_execution` primitive receipts, requires
finalized live `tvmd node block` views to expose block-height receipt IDs and primitive counts for TensorOp,
LinearTrainingStep, and GraphExecution work, fetches a live tensor
descriptor, row, chunk, and opening through the TensorVM node, reruns Gate 0 from the checker,
verifies the local-only evidence boundary, requires all 15 operator stores to report the same finalized
common-head block hash through `tvmd node block`, selects a non-producer's latest finalized p2p-observed
block-payload head from the block-payload gossip set and requires every operator to return the matching finalized block hash and state
root while reporting block-vote finality evidence, a nonempty block-log root, observed block-vote gossip,
and structured pending reward claim samples with future claimable heights for live receipt and proposer
rewards, and uses
`check-rolling-restart-continuity.sh` to run the restart-continuity gate one service at a time across every
counted operator, proving each restarted service keeps its libp2p peer ID, preserves the pre-restart
finalized common head and state root, avoids height/block-count/state-root/block-log-root regression,
preserves a sampled tensor artifact, reconverges on a finalized common head, and reports whether
post-restart finalized blocks advanced. The restart gate marks the just-restarted services for the local
checker so a stable post-restart plateau can rely on preserved post-seed chain state and live peer
connectivity instead of fresh volatile gossip counters, and process-lifetime role/network totals are not
re-required after those processes have restarted, without relaxing normal local readiness checks.

## Acceptance Criteria

| # | Criterion | Evidence |
| --- | --- | --- |
| 1 | Miners execute deterministic tensor jobs. | Current TensorOp and LinearTrainingStep jobs are backed by validated content-addressed IR graph IDs, and their receipts now derive `trace_root` from exact execution of the canonical TensorGraph op traces. The exact IR interpreter foundation can execute hand-built validated graphs over the currently implemented deterministic tensor ops, including Tier-A `einsum` matrix contractions, exact field and `Fixed32` `div`, `Fixed32` `matmul`, shaping, generators, comparisons, `where`, field-order `clamp`, structural `squeeze`/`unsqueeze`/`slice`/`split`/`tril`/`triu`, and commits per-op output roots plus a trace root. Local synthetic production includes a graph-backed exact Tier-B job, and miner/validator role helpers submit and attest the graph receipt from registered graph bodies plus node-local tensor artifacts. Evidence: `ir::tests::matmul_graph_has_stable_canonical_json_and_graph_id`, `ir::tests::linear_training_step_graph_validates_and_commits_shapes`, `ir::tests::exact_interpreter_executes_hand_built_graph_and_commits_trace`, `ir::tests::exact_interpreter_executes_fixed32_matmul_with_mixed_scales`, `ir::tests::graph_validation_rejects_unsupported_matmul_dtype`, `ir::tests::exact_interpreter_executes_field_div`, `ir::tests::exact_interpreter_executes_fixed32_div_with_scale_rescale`, `ir::tests::graph_validation_rejects_unsupported_div_dtype`, `ir::tests::exact_interpreter_executes_einsum_matrix_contraction`, `ir::tests::graph_validation_rejects_unsupported_einsum_equations`, `ir::tests::exact_interpreter_executes_shaping_comparison_generators_and_where`, `ir::tests::exact_interpreter_executes_clamp`, `ir::tests::exact_interpreter_executes_single_output_structural_ops`, `ir::tests::exact_interpreter_executes_split_multi_output_structural_op`, `ir::tests::graph_validation_rejects_split_size_mismatch`, `ir::tests::exact_interpreter_supports_field_scalar_params`, `ir::tests::exact_interpreter_rejects_deferred_ops`, `verify::tests::graph_verifier_accepts_field_div_receipt`, `verify::tests::graph_verifier_accepts_einsum_receipt`, `verify::tests::graph_verifier_accepts_clamp_receipt`, `verify::tests::graph_verifier_accepts_single_output_structural_receipt`, `verify::tests::graph_verifier_accepts_split_receipt`, `jobs::tests::matmul_receipt_commits_to_outputs`, `jobs::tests::linear_receipt_commits_to_learning_step`, `localnet::tests::synthetic_cpu_round_settles_work_and_advances_finalized_chain`, `app::validator_role::tests::role_runtime_submits_and_attests_graph_execution_from_local_artifacts`, `miner::tests::miner_solves_matmul_and_serves_tensors`, `miner::tests::miner_solves_linear_step_and_serves_intermediates`, and `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul` |
| 2 | Validators verify block-eligible matmul jobs with full-output Freivalds or bounded equivalent. | `verify::full_freivalds`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption`, `verify::tests::tensor_op_verifier_rejects_metadata_and_shape_mismatches`, `validator::tests::validator_verifies_matmul_from_tensor_server` |
| 3 | Row-sampled checks are audits unless false-accept bounds are documented. | `verify::row_sample_detection_probability`, `study::row_sampling_study`, `study::tests::row_sampling_study_blocks_sparse_row_sampled_only_acceptance` |
| 4 | Blocks are produced by validators winning useful-verification PoW over deterministic settled-receipt blockspace. | Partially implemented locally. `TensorBlock` now commits `settled_receipt_set_root`, block-level `checks_root`, beacon, proposer reward amount, difficulty target, and nonce; `chain::proposer` selects registered validators and ignores miner TensorWork; selected receipts are marked included once; historical block evidence stores exact parent `ChainState` snapshots keyed by block hash and persists them through the chain-state codec, so `BlockApplyOutcome`, selected-receipt openings, typed block-check transcripts, checks roots, and child roots remain stable after later receipts/blocks and after restart; empty fallback block validation requires the deterministic stake-weighted proposer selected from parent state and beacon plus the configured `pow_timeout_blocks * block_time_seconds` delay after the parent for non-genesis fallback blocks, while useful UVPoW blocks remain validator-competitive and are not fallback-delayed; current-head admission replaces an unfinalized useful head only when a same-parent useful competitor has a strictly better PoW hash, finalized or fallback heads remain stable, and valid known-parent side branches are retained with parent and child state snapshots without mutating canonical head state, and strictly longer unfinalized side branches automatically reorganize canonical head state; `submit_block_vote` validates known blocks with strict parent-root checks before counting votes; bounded network-visible block-check challenge payloads can disprove a committed check leaf through the shared p2p/node event path, retry while the challenged block or observed-block parent context is missing, cache an observed malformed block outside canonical chain state, apply through `ChainCommand::SubmitBlockCheckChallenge`, recording observed-invalid diagnostic evidence without punishing canonical proposer rewards while reserving proposer throttling and delayed challenger rewards for canonical block-check failures; selected-receipt openings now expose the typed transcript fields that hash into the expected check leaf before a block-check challenge can prove a mismatch. `Chain::deterministic_bad_block_check_challenge` derives a reproducible diagnostic observed block plus signed challenge from a produced useful block and canonical selected-receipt opening without weakening normal malformed-block rejection; validator proposers publish that observed diagnostic challenge after useful proposal gossip, and non-producers apply it through the normal observed-diagnostic path; validator role loops submit and gossip explicit block votes so append and finality are separate runtime events; local synthetic scheduling publishes deterministic jobs only and no longer forces empty fallback blocks through the role wallet, while the validator role tick observes not-yet-included settled receipts with local tensor artifacts and attestations before submitting useful proposals. Validator proposal is no longer gated by `local_synthetic_producer`; a configured validator proposer can build a useful block from already accepted settled state even when synthetic job production is disabled. Validator runtime status separates useful settled-receipt block proposals from empty fallback proposals, and the local checker requires positive useful proposal, proposed-receipt, artifact-ready, attested-receipt, delayed pending proposer reward evidence, and applied diagnostic block-check challenge evidence. Evidence: `chain::tests::proposer_selection_ignores_tensorwork`, `chain::tests::fallback_blocks_require_stake_weighted_selected_validator`, `chain::tests::non_genesis_fallback_requires_pow_timeout`, `chain::tests::useful_pow_blocks_do_not_require_fallback_selected_validator`, `chain::tests::historical_block_apply_outcome_uses_stored_parent_snapshot_after_future_receipts`, `chain::tests::block_apply_outcome_exposes_parent_child_and_check_openings`, `chain::tests::block_roots_commit_to_canonical_receipts_checks_attestations_and_state_values`, `chain::tests::block_votes_reject_invalid_useful_pow_and_checks_root`, `chain::tests::produced_blocks_mark_selected_settled_receipts_included_once`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, `chain::tests::observed_block_check_challenge_records_evidence_without_punishing_canonical_proposer`, `p2p::wire::tests::block_check_challenge_payloads_roundtrip_and_reject_malformed_edges`, `app::network::tests::observed_block_check_challenge_messages_carry_delayed_reward_evidence_payload`, `node::message_ingest::tests::network_event_driver_applies_observed_block_check_challenge_without_punishing_canonical_reward`, `node::payload_application::tests::block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges`, `node::payload_application::tests::observed_block_check_challenge_payload_caches_observation_and_applies`, `node::pending_payloads::tests::pending_payloads_retry_keeps_pending_payloads`, `node::tests::block_payload_application_admits_next_head_and_rejects_bad_edges`, `node::payload_application::tests::block_payload_application_replaces_current_head_with_better_useful_pow`, `chain::tests::longer_side_branch_reorganizes_unfinalized_canonical_suffix`, `chain::tests::side_branch_reorg_does_not_replace_finalized_canonical_suffix`, `node::payload_application::tests::block_payload_application_reorganizes_to_longer_side_branch`, `chain::tests::historical_parent_side_branch_is_stored_without_replacing_canonical_head`, `p2p::tests::block_vote_payloads_roundtrip_and_reject_malformed_edges`, `node::runtime_state::tests::runtime_state_tracks_loop_counters`, `tvmd` binary `tests::validator_role_block_vote_submission_finalizes_only_through_votes`, `tvmd` binary `tests::producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks`, `tvmd` binary `tests::validator_proposer_tick_runs_without_synthetic_producer_gate`, `tvmd` binary `tests::network_applied_receipt_and_attestation_make_validator_proposal_useful`, `tvmd` CLI `role_run_commands_serve_through_role_specific_surfaces`, `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape`, `storage::tests::block_log_store_appends_loads_and_detects_tampering`, `localnet::tests::synthetic_cpu_round_settles_work_and_advances_finalized_chain`, and service-block evidence fields. Remaining gaps: public deployment evidence, CUDA miner evidence, deployed public/CUDA dispute evidence, and public drand/VRF randomness verification; the latest local CPU Docker proof covers three validator block proposers, chain-visible proposer cooldown state, delayed proposer reward evidence, applied diagnostic block-check evidence, trace-bisection admission DoS bounds, isolated trace-bisection timeout against incomplete challengers, and passive observer finalized-head convergence. |
| 5 | Rewards are distributed by verified settled TensorWork. | Miner receipt rewards are chain-allocated proportionally from verified receipt TensorWork as delayed pending claims; newly settled miner TWU is recorded as `pending_tensor_work` and only becomes `settled_tensor_work` when the matching non-voided miner receipt reward is claimed after canonical blockspace inclusion plus the explicit reward-settlement and challenge-window maturity delay. Invalid-output, data-unavailability, and block-check challenge paths clear pending miner work when they void delayed receipt rewards, so invalid work cannot activate later; telemetry still reports total observed work as settled plus pending. Miner and validator receipt rewards are pending claims whose state-rooted `ReceiptRewardMaturity` is either `AwaitingInclusion` or `ClaimableAt(height)`; canonical block inclusion converts awaiting claims into inclusion-derived delayed heights, validator receipt rewards also require a matching chain-accepted validator VRF reveal before positive credit, and receipt claims remain pending until included, mature claims are swept by beneficiary `ClaimReward`. Proposer rewards are delayed, successful block-check challenger bounties become pending challenge reward claims before spendability and become claimable only after the same full maturity delay rather than the shorter proposer throttle height, configured mandatory validator audits deterministically select a registered auditor distinct from the audited validator, delay the audited validator's reward until the audit deadline, reject non-assigned audit reports, and void the delayed reward when audits are missed or contradictory while holding that voided claim through the audit appeal deadline before pruning without credit, and generic/faucet reward credits now enter a state-rooted pending credit ledger before any spendable reward balance is credited. Late-finalized proposer rewards now materialize as delayed pending claims even when finality arrives after their claimable height, and claimed or voided proposer rewards record their block heights in `released_proposer_reward_blocks` so later materialization cannot recreate them. Block `reward_root` now commits the child state's spendable rewards plus pending proposer, released proposer reward block heights, receipt, challenge, and credit ledgers, so delayed reward-finality claims are block-root-bound instead of visible only through the broader state root; blocks carrying the old spendable-only root are rejected. Normal block transitions now apply the current block's receipt-inclusion delays and slash/audit voiding first, then preserve still-mature non-voided proposer, included receipt, challenge, and credit reward claims as pending state until beneficiary `ClaimReward`; producer and non-producer block application recompute the same claimable-but-not-spendable child state, while voided proposer/challenge claims are pruned without credit. `ChainState::pending_reward_claims` now exposes a unified read-only pending reward claim view for proposer, receipt miner, receipt validator, challenge, and credit ledgers, including related IDs such as the receipt affected by a challenge claim, and service status plus explorer overview consume that chain-owned view for bounded claim samples with claimable heights and voided status. The local CPU checker requires live non-voided receipt and proposer pending reward claims whose `claimable_at_height` is greater than the observed live height, plus applied diagnostic block-check challenge evidence, so local evidence proves delayed rewards directly from state rather than inferring maturity from aggregate counts. Validator-owned useful block proposals and empty fallback proposals both enter the delayed proposer reward ledger with the explicit full reward-maturity delay before any spendable proposer balance is credited; fallback blocks remain distinguishable and carry the reduced proposer claim amount, and pending proposer reward state/root/storage no longer carry a later-useful-block unlock latch. Registered validator roles now observe only their assigned audits, submit signed audit reports, and non-producers ingest/retry those bounded p2p payloads; slashed audited validators can submit signed, bounded appeal records through the shared chain command path, and those records are committed in the state root and persisted in chain-state snapshots. Appeal resolution now works through the same delayed reward ledger and recorded stake-slash path: upheld outcomes keep the validator receipt reward voided for normal pruning, while reversed reward-void outcomes reinstate the pending claim without immediate spendable credit, refund the recorded slash from treasury back to validator stake, and spendable credit still requires beneficiary `ClaimReward` after maturity. Block-check challenge payloads now make the existing canonical challenge clawback/pending-bounty path reachable through shared network ingestion, while observed malformed block payloads cache evidence without replacing canonical blocks or punishing canonical proposer rewards. Deterministic diagnostic challenge generation plus validator-proposer emission proves that observed diagnostic path without admitting malformed blocks normally. The `study::economic_invariant_study` helper computes the strict `slashable_bond * P(detection) > reward_from_fraud` margin and required slashable bond, `ChainState::validator_audit_economic_calibration` exposes the current validator-audit detection probability, slash amount, non-voided pending validator reward exposure, required slashable bond, and pass/fail invariant through service status and explorer overview, and `ChainState::fraud_path_economic_calibration` adds implemented-path calibration for validator audit, miner data-unavailability, invalid-output, and block-check/proposer clawback paths with aggregate worst-required-bond and all-path pass/fail evidence. Reward maturity now exposes an explicit fraud-window hold through `ChainParams::fraud_reward_hold_blocks`, covering the challenge window and active audit window before spendability. Delayed non-voided receipt and proposer rewards are treated as slashable/voidable escrow, so reward-from-fraud is counted only after claimability. Late assigned invalid-output attestations now contest already settled receipts by removing them from the settled set, marking them challenged, recording a state-rooted miner stake slash, crediting treasury, voiding delayed pending receipt reward claims before claim, and pruning those claims without spendable credit at maturity. `ChainState::detection_probability_evidence` now derives structured detection evidence for implemented verifier and fraud mechanisms from current params, live TensorOp and LinearTrainingStep job shapes, graph-job counts, and chain-state fraud counters; service status and explorer overview render per-mechanism detection bps, false-accept bps, sample sizes, source labels, and live subject counts. Deployed-run measurements and remaining fraud paths remain outside the local economics invariant. Evidence: `chain::tests::miner_rewards_delay_tensorwork_activation_until_reward_release`, `chain::tests::chain_settles_valid_tensorwork_and_rewards_participants`, `chain::tests::invalid_output_evidence_voids_delayed_receipt_rewards_before_release`, `chain::tests::invalid_output_attestation_slashes_receipt_miner_once_and_voids_rewards`, `chain::tests::chain_settles_valid_graph_execution_and_delays_rewards`, `chain::tests::receipt_rewards_use_minimum_reward_maturity_delay_when_epochs_are_zero`, `chain::tests::generic_credit_rewards_claim_only_after_maturity`, `chain::tests::produced_blocks_delay_receipt_rewards_from_inclusion_height`, `chain::tests::late_finalized_proposer_reward_materializes_as_delayed_claim_once`, `chain::tests::observed_block_check_challenge_records_evidence_without_punishing_canonical_proposer`, `chain::tests::reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury`, `chain::tests::fallback_proposer_reward_uses_explicit_maturity_delay`, `chain::tests::block_reward_root_rejects_spendable_only_root_when_pending_rewards_exist`, `chain::tests::reward_root_commits_to_all_pending_reward_ledgers`, `chain::tests::pending_reward_claim_view_covers_all_ledgers`, `chain::tests::block_transition_preserves_matured_rewards_until_claim`, `chain::tests::block_transition_preserves_matured_receipt_rewards_until_claim`, `chain::tests::release_matured_proposer_rewards_sweeps_voided_claims_without_credit`, `chain::tests::matured_proposer_reward_releases_after_full_maturity_delay`, `chain::tests::validator_audit_result_slashes_contradicted_attestation_and_voids_reward`, `chain::tests::mandatory_validator_audit_assignment_requires_separate_auditor`, `chain::tests::state_root_commits_to_validator_audit_records`, `app::status::tests::service_status_exports_pending_reward_claim_maturity_details`, `app::network::tests::observed_block_check_challenge_messages_carry_delayed_reward_evidence_payload`, `node::message_ingest::tests::network_event_driver_applies_observed_block_check_challenge_without_punishing_canonical_reward`, `rpc::tests::node_rpc_serves_explorer_telemetry_and_faucet_routes`, `tensor_vm_explorer::tests::explorer_json_and_shell_include_live_websocket_contract`, `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape`, `study::tests::economic_invariant_study_reports_required_slash_margin`, `study::tests::economic_invariant_study_clamps_detection_probability`, `p2p::tests::validator_audit_report_payloads_roundtrip_and_reject_malformed_edges`, `p2p::wire::tests::block_check_challenge_payloads_roundtrip_and_reject_malformed_edges`, `node::tests::validator_audit_report_payload_application_reports_pending_applied_and_invalid_edges`, `node::payload_application::tests::block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges`, `node::payload_application::tests::observed_block_check_challenge_payload_caches_observation_and_applies`, `node::tests::network_event_driver_applies_validator_audit_report_payloads_for_non_producers`, `pending_payloads::tests::pending_payloads_retry_keeps_pending_payloads`, `tvmd` binary `tests::producer_job_is_receipted_attested_and_proposed_by_role_owned_ticks`, `tvmd` binary `tests::validator_role_audit_report_submission_observes_assignments_and_skips_duplicates`, `tvmd` binary `runtime_persistence::role_runtime_mutating_rpc_persists_chain`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, and `tvmd_cli::local_testnet_service_gateway_does_not_produce_local_blocks`. |
| 6 | Validation randomness is unbiasable after receipt roots are committed. | Partial. Admitted receipts persist a `ReceiptRandomnessAnchor` with receipt-time finalized beacon round/randomness, assignment seed, and validation seed commitment, and attestation admission rejects stored receipts missing that anchor. `ChainCommand::SubmitExternalRandomnessBeacon` accepts strictly newer externally observed beacon rounds, stores state-rooted `ExternalRandomnessBeaconRecord` entries, advances the finalized beacon for future receipt anchors, persists records through chain-state snapshots, and exposes count/latest-round evidence through status and explorer JSON. Local CPU role runtimes now ingest the configured deterministic drand-style fixture, configured verified drand evidence, and public default-chain chained drand polling through verified chain commands before network/role work, persist accepted records, expose observed/applied/skipped/failure plus public-drand attempts/successes/stale/backoff plus expected-latest-round, fetched-lag, max-lag, rounds-per-chain-epoch, chain-epoch, and freshness counters through `role-runtime.status` and `tvmd node status`; locally fetched public rounds outside the configured lag are skipped before chain mutation, and accepted chained drand rounds are constrained by a rooted/persisted chain-owned epoch window; accepted beacons are relayed as bounded p2p payloads, apply network-originated beacon payloads idempotently through the same chain command, derive and register wallet-backed validator reveal public keys before receipt work, submit state-rooted validator VRF reveal records before validator receipt rewards can become spendable, require registered-key validators to provide bounded Ed25519 proof bytes over the committed receipt seed before reward release, relay bounded reveal payloads over p2p/node ingest, retry out-of-order reveal payloads until receipt anchors arrive, and the local checker gates positive external-beacon record evidence, positive validator-reveal record evidence, validator reveal key lifecycle evidence, production-vs-legacy reveal count evidence, public-drand epoch-window evidence, network-applied beacon/reveal evidence, current-block-hash-ban evidence, and receipt-anchor consistency evidence. Full-spec public evidence validation now also requires the signed randomness-beacon summary count to equal `observed_blocks`, plus manifest-level raw accepted `drand-v1` or `validator-vrf-v1` randomness records whose aggregate root matches the signed randomness summary; local deterministic fixture records cannot satisfy the full-spec public randomness gate. Evidence: `chain::Chain::validation_seed`, `chain::tests::validation_seed_is_bound_to_finalized_randomness_and_receipt`, `chain::tests::external_randomness_beacon_command_advances_receipt_anchor_source`, `chain::tests::external_randomness_beacon_command_rejects_stale_and_empty_records`, `chain::tests::verified_chained_drand_beacon_respects_chain_epoch_mapping`, `chain::tests::validator_vrf_reveal_records_are_chain_verified_and_state_rooted`, `chain::tests::validator_vrf_reveal_rejects_tampered_binding_fields`, `chain::tests::keyed_validator_vrf_reveal_requires_production_proof`, `tvmd` binary `validator_role::validator_role_vrf_key_registration_is_keyed_and_idempotent`, `tvmd` binary `runtime_roles::selected_validator_proposer_emits_idle_fallback_block`, `app::status::tests::service_status_forwards_role_randomness_beacon_evidence`, `chain::tests::admitted_receipt_validation_randomness_is_anchored_at_submission`, `chain::tests::admitted_receipt_attestation_requires_randomness_anchor`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, `chain::tests::randomness_binding_evidence_reports_receipt_bound_finalized_beacon_policy`, `app::status::tests::service_status_exports_randomness_binding_evidence`, `p2p::wire::tests::external_randomness_beacon_payloads_roundtrip_and_reject_malformed_edges`, `p2p::wire::tests::validator_vrf_reveal_payloads_roundtrip_and_reject_malformed_edges`, `node::payload_application::tests::validator_vrf_reveal_payload_application_reports_pending_applied_and_invalid_edges`, `node::message_ingest::tests::network_event_driver_applies_external_randomness_beacon_payloads`, `node::message_ingest::tests::network_event_driver_applies_validator_vrf_reveal_payloads`, `tvmd` binary `runtime_persistence::role_runtime_external_randomness_beacon_tick_persists_chain_and_status`, `tvmd` binary `runtime_persistence::role_runtime_public_drand_fetch_tick_persists_chain_and_status`, `tvmd` binary `runtime_persistence::role_runtime_public_drand_polling_skips_stale_rounds_and_backs_off_failures`, `tvmd` binary `runtime_persistence::role_runtime_public_drand_skips_newer_round_outside_freshness_window`, `testnet::tests::public_testnet_evidence_bundle_requires_randomness_records_for_full_run`, `testnet::tests::public_testnet_evidence_bundle_requires_raw_randomness_records`, `testnet::tests::public_testnet_evidence_manifest_parses_into_bundle`, `local_cpu_compose::local_cpu_compose_bundle_matches_spec_artifact_shape`, `rpc::tests::routes::explorer_overview_exports_validator_audit_economic_calibration`, `tensor_vm_explorer::tests::explorer_json_and_shell_include_live_websocket_contract`, and `study::assess_randomness`. Remaining gap: deployed full VRF construction and deployed commit-reveal lifecycle. |
| 7 | Invalid tensor outputs are rejected in dense and sparse corruption tests. | `verify::tests::tensor_op_verifier_rejects_bad_output`, `verify::tests::full_freivalds_accepts_honest_and_rejects_corruption` |
| 8 | LinearTrainingStep receipts validate forward/backward/error/update structure. | `verify::verify_linear_training_step`, `verify::tests::linear_training_verifier_rejects_metadata_and_commitment_mismatches`, `ir::tests::frozen_registry_declares_verifier_class_for_every_op`, `vm::tests::linear_backward_and_sgd_match_equations`, `jobs::tests::linear_receipt_commits_to_learning_step` |
| 9 | Sparse corruptions in `dY` and `W_next` are rejected with stated probability. | Current LinearTrainingStep verification uses random-linear checks for `dY = Y - T` and `W_next = W - lr * grad_W`; registry metadata now distinguishes random-linear Tier-B relations from deterministic replay and deferred index-consistency ops. Evidence: `verify::tests::linear_training_verifier_rejects_sparse_error_poisoning`, `verify::tests::linear_training_verifier_rejects_sparse_weight_poisoning`, `ir::tests::frozen_registry_declares_verifier_class_for_every_op`, and `ir::tests::index_ops_require_index_consistency_and_are_not_consensus_admitted`. |
| 10 | Honest miners produce identical output roots. | Redundant settlement now records state-rooted `RedundantSettlementDelayRecord` entries when quorum-backed receipts cannot settle because the configured independent-operator agreement quorum is still missing or conflicting quorum-backed linear-transition receipts exist. The agreement gate counts distinct registered miner `operator_id` values rather than miner addresses alone, and delay records include both agreeing miner count and agreeing operator count plus `reward_delay_until_height`, derived from the chain's reward maturity rule. Any later settled receipt reward claims inherit that height as an explicit lower-bound hold before block inclusion can start the ordinary maturity clock. Evidence: `runtime::tests::gpu_backend_reports_device_and_requires_cuda_kernels`, `runtime::tests::cpu_and_gpu_backends_match_canonical_matmul`, `runtime::tests::cpu_and_gpu_backends_match_linear_step`, `runtime::tests::cuda_kernel_matches_canonical_field_matmul_edges`, and `runtime::tests::cuda_kernels_match_canonical_linear_tensor_ops` under `--features cuda-kernels`, `chain::tests::redundant_agreement_quorum_is_required_before_settlement`, `chain::tests::redundant_agreement_requires_distinct_miner_operators`, `chain::tests::conflicting_linear_training_roots_do_not_settle`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, `scheduler::tests::miner_assignment_prefers_operator_separation`, and `scheduler::tests::miner_assignment_falls_back_when_operator_diversity_is_insufficient` |
| 11 | Validators spend materially less compute than full recomputation. | Chain state now derives verifier-bandwidth evidence from live job and receipt shapes, including per-primitive max execution ops, max verification ops, estimated verification bytes, estimated per-validator bandwidth, and verification-to-execution bps surfaced through service status and explorer overview. Evidence: `study::matmul_verification_cost_study`, `study::tests::matmul_verification_cost_is_lower_than_execution_for_mvp_shape`, `telemetry::estimated_verification_to_execution_ratio`, `chain::tests::verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes`, `app::status::tests::service_status_exports_validator_audit_economic_calibration`, `rpc::tests::explorer_overview_exports_validator_audit_economic_calibration`, and `tensor_vm_explorer::tests::explorer_json_and_shell_include_live_websocket_contract`. CUDA and public deployed bandwidth measurements remain deployment-gated. |
| 12 | Tensor data availability exceeds 95% during active and retention windows. | Selected-receipt openings expose submission-anchored retention deadlines for included receipts, so block apply evidence no longer extends the apparent retention window as parent height advances. Evidence: `chain::tests::selected_receipt_opening_retention_deadline_is_submission_anchored`, `validator::tests::validator_attests_unavailable_when_server_lacks_tensor_roots`, `tensor_server::tests::tensor_server_retains_through_deadline_and_prunes_afterward`, `telemetry::data_availability_rate`; public-network measurement remains deployment-gated |
| 13 | Network runs for 7 consecutive days with independent nodes. | Not locally complete; `testnet::tests::local_testnet_bootstraps_required_public_shape`, `testnet::tests::public_testnet_preflight_manifest_reports_launch_readiness`, `testnet::tests::deployed_public_testnet_preflight_example_rejects_placeholder_domains`, `testnet::tests::docs_public_testnet_preflight_manifest_rejects_placeholder_domains`, `testnet::tests::public_testnet_preflight_manifest_rejects_malformed_input`, `cli::tests::execute_reference_cli_command_reports_miner_and_validator_readiness`, `cli::tests::validate_public_testnet_preflight_manifest_reports_launch_readiness`, `tvmd` binary `tests::docs_public_testnet_preflight_command_reports_pending_status`, `tvmd` binary `tests::docs_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::documented_public_testnet_preflight_command_reports_pending_status`, `tvmd_cli::generated_public_testnet_preflight_manifest_reports_ready`, `tvmd_cli::documented_public_testnet_evidence_command_reports_non_full_spec_status`, `tvmd_cli::generated_public_evidence_manifest_round_trips_through_tvmd_validator`, `tvmd_cli::service_cli_lifecycle_starts_libp2p_and_serves_public_surfaces`, `p2p::tests::peer_book_store_upserts_bootstrap_records_with_peer_ids`, `rpc::tests::node_rpc_serves_head_and_blocks`, `rpc::tests::node_rpc_serves_explorer_telemetry_and_faucet_routes`, `testnet::tests::public_testnet_run_evidence_requires_independent_external_operators`, `testnet::tests::public_testnet_run_evidence_requires_production_runtime_and_reachable_services`, `testnet::tests::public_testnet_evidence_bundle_requires_publication_and_audit_records`, `testnet::tests::public_testnet_evidence_bundle_requires_randomness_records_for_full_run`, `testnet::tests::public_testnet_evidence_bundle_requires_raw_randomness_records`, `testnet::tests::public_testnet_evidence_bundle_requires_raw_chain_history_records`, `testnet::tests::public_testnet_evidence_bundle_requires_raw_operational_records`, `testnet::tests::public_testnet_evidence_manifest_parses_into_bundle`, `testnet::tests::deployed_public_testnet_evidence_example_is_parseable_but_not_full_spec`, `testnet::tests::docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec`, `testnet::tests::public_testnet_evidence_manifest_rejects_malformed_input`, and `testnet::tests::public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes` validate the local launch preflight plus service-launch config and health/content endpoints, checked spec-path pending manifests and deploy preflight/evidence examples with planned public content paths, actual `tvmd` file-reading and process invocation behavior for the documented pending-manifest commands, process-generated launch-ready external-addressed preflight manifest validation from disk, process-generated short-run evidence-manifest assembly from signed `tvmd public evidence ...` generator commands that validates from disk as independently checkable without setting the full-spec flag, bounded process-level service init/peer-add/readiness/serve lifecycle with mandatory libp2p startup from the initialized node store and durable peer book, unauthenticated request rejection, authenticated `/health`, `/rpc/health`, `/explorer/health`, `/faucet/health`, `/telemetry/health`, process-level signed service-health generation from reached RPC/explorer/faucet/telemetry health responses, state-root-bearing `/chain/head`, `/epoch/current`, `/jobs/current`, the empty-chain `/chain/block/0` route response, `/explorer`, `/faucet/page`, `/telemetry/dashboard`, mutable `/tx`, `/receipt`, and `/attestation` submissions, registered miner/validator state read-back, captured `/chain/head`, `/explorer`, `/faucet/page`, and `/telemetry/dashboard` response-body evidence generation through matching `evidence service content-bytes` and `evidence service content-file` CLI outputs, process-derived local libp2p peer/protocol data accepted only when bound to an external public multiaddr and then summarized/artifact-bound from its network-runtime observation root, the same process-derived data rejected as public network-observation evidence when bound to loopback, exact query-free service URL path enforcement, and placeholder-domain rejection, signed publication/auditor-record/run-window/node-heartbeat/operator-attestation CLI generation and invalid argument rejection, service peer-book bootstrap seeding with peer-ID-preserving `/p2p/<peer-id>` dial addresses, service-health and service-content CLI manifest-line generation, byte-derived and file-derived service-content root generation, plus invalid argument rejection, signed production-libp2p network-observation CLI generation and invalid argument rejection including malformed DNS-label and single-label DNS multiaddrs, signed supporting-record summary generation, signed external supporting-record artifact locator generation, signed artifact locator generation from derived aggregate roots, plus deterministic root aggregation for block/finality/network-runtime/randomness-beacon/data-availability/invalid-work/reward-settlement/detection-measurement/validator-vrf-lifecycle evidence, evidence gate for signed 7-day wall-clock run-window evidence, expected block count, distinct external operators, signature-verified heartbeat summaries, run continuity, finality, data availability, invalid-work rejection, reward-settlement records, production libp2p runtime use, signed per-operator production libp2p network-observation records exactly matching counted public operators and aggregating to signed network-runtime summary roots, deployed RPC/explorer/faucet/telemetry service reachability with signed health summaries and signed content roots bound to external HTTPS URLs, matching and distinct endpoint IDs, distinct service-content roots, and the required content paths, external public evidence publication URI validation including special-use DNS and single-label DNS rejection, verified manifest publication signatures, signed independent-auditor records, signed block/finality/randomness-beacon/data-availability/invalid-work/reward-settlement/detection-measurement/validator-vrf-lifecycle summary roots, manifest-level raw accepted public randomness records plus raw block-history, finality-history, data-availability, invalid-work, reward-settlement, detection-measurement, and revealed validator-VRF-lifecycle records that aggregate to their signed summaries before full-spec evidence can pass, signed operator-attestation-derived external-operator evidence, independently checkable evidence-bundle publication, and manifest parsing |
| 14 | Zero-receipt epochs have a tested stake-weighted PoW-skip fallback path. | Partial. Empty canonical blockspace produces explicit `PowSkipFallback` blocks that validate only when signed by the deterministic stake-weighted fallback proposer selected from parent state and beacon, and non-genesis fallback blocks must wait at least `pow_timeout_blocks * block_time_seconds` after the parent; non-selected or early fallback production and non-producer block admission reject the payload, while useful UVPoW blocks remain open to validator competition. Validator role fallback proposals carry reduced proposer claims with the full reward-maturity delay before beneficiary claim, and validator-owned zero-work liveness is covered by `study::tests::zero_work_liveness_study_produces_blocks_from_validators`, `chain::tests::fallback_blocks_require_stake_weighted_selected_validator`, `chain::tests::non_genesis_fallback_requires_pow_timeout`, `chain::tests::useful_pow_blocks_do_not_require_fallback_selected_validator`, and `chain::tests::fallback_proposer_reward_uses_explicit_maturity_delay`. Remaining gap: public/CUDA runtime evidence beyond the local CPU chain-cadence proposer proof. |
| 15 | Reward concentration, validator disagreement, and data withholding are reported. | `telemetry::TelemetrySnapshot`, `study::tensorwork_concentration`, `study::data_withholding_study`, operator-aware `study::collusion_risk_assessment`, `study::economic_invariant_study`, live `ChainState::validator_audit_economic_calibration`, live `ChainState::fraud_path_economic_calibration`, live `ChainState::verifier_bandwidth_evidence`, `watcher::ChainWatcher`, state-rooted `RedundantSettlementDelayRecord` entries whose agreeing miner/operator counts and `reward_delay_until_height` are copied into later pending miner and validator receipt rewards for the delayed receipt, and chain-owned delayed TensorWork activation that keeps miner work pending until the matching reward claim matures. Collusion-risk evidence reports colluding miner-address quorum separately from colluding operator quorum and only treats redundant agreement as satisfiable when the colluding operator count reaches quorum. Evidence: `chain::tests::miner_rewards_delay_tensorwork_activation_until_reward_release`, `chain::tests::redundant_agreement_quorum_is_required_before_settlement`, `chain::tests::redundant_agreement_requires_distinct_miner_operators`, `chain::tests::conflicting_linear_training_roots_do_not_settle`, `chain::tests::verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes`, `storage::chain_state::tests::chain_state_store_roundtrips_full_chain_and_detects_tampering`, `telemetry::tests::telemetry_reports_block_timing_and_concentration`, `telemetry::tests::telemetry_reports_security_compute_and_economic_success_metrics`, `telemetry::tests::telemetry_reports_hardware_classes_and_gpu_utilization`, `telemetry::tests::telemetry_reports_linear_receipt_bandwidth_and_missing_job_edges`, `study::tests::collusion_risk_assessment_reports_threshold_crossings`, `study::tests::economic_invariant_study_reports_required_slash_margin`, `study::tests::economic_invariant_study_clamps_detection_probability`, `chain::tests::validator_audit_economic_calibration_uses_live_at_risk_validator_rewards`, `chain::tests::fraud_path_economic_calibration_covers_pending_reward_fraud_paths`, `app::status::tests::service_status_exports_validator_audit_economic_calibration`, `rpc::tests::explorer_overview_exports_validator_audit_economic_calibration`, `watcher::tests::watcher_reports_invalid_receipts_and_data_withholding`, `watcher::tests::watcher_flags_validator_misconduct_in_audited_state`, `watcher::tests::watcher_flags_malformed_attestation_evidence`, `watcher::tests::watcher_reports_conflicting_linear_transitions` |

Public evidence CUDA graph gate note:
`testnet::tests::public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec` now proves
that otherwise complete full-spec public evidence remains non-full-spec unless
`cuda_verified_miner_count` covers the counted public miners.
`testnet::tests::public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec` now also
proves that otherwise complete full-spec public evidence remains non-full-spec unless
`cuda_graph_execution_receipts` is positive and does not exceed checked or available receipt counts. The
`testnet::tests::public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec` now proves
that otherwise complete full-spec public evidence remains non-full-spec unless signed
`validator_vrf_lifecycle_records` exactly cover checked receipts,
`testnet::tests::public_testnet_evidence_bundle_requires_raw_operational_records` now proves raw
data-availability, invalid-work, and reward-settlement records cannot repeat receipt roots to pad deployed
receipt coverage and that reward settlements cannot use zero participant IDs, and
`testnet::tests::public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec`
now proves raw detection-measurement records cannot bypass manifest field validation when directly bundled, and
`testnet::tests::public_testnet_evidence_bundle_requires_raw_chain_history_records` now proves raw
block/finality history must use distinct nonzero block roots, matching block/finality roots, and finalized
status counts matching the run evidence, and
`testnet::tests::public_testnet_evidence_bundle_requires_raw_randomness_records` now proves raw public
randomness records must cover each observed block exactly once with distinct source/round pairs, and
`testnet::tests::public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec`
proves the full-spec gate also requires raw revealed lifecycle records that aggregate to the signed
lifecycle summary root and cannot repeat receipt roots to pad checked-receipt coverage.
`testnet::tests::public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec`
now proves that otherwise complete full-spec public evidence remains non-full-spec unless it has positive
signed deployed detection-measurement records and raw detection records that aggregate to the signed
summary. The public evidence manifest parser requires all four fields, and CLI validation reports the
counts and boolean CUDA miner/graph, signed/raw VRF lifecycle, and deployed detection evidence gates.

## Non-Local Gaps

Reward-delay evidence note: `chain::tests::block_transition_preserves_matured_receipt_rewards_until_claim`
now proves included receipt rewards remain state-rooted pending claims through canonical block child-state
application on both producer and non-producer peers, and become spendable only through beneficiary
`ClaimReward`.
`chain::tests::produced_blocks_delay_receipt_rewards_from_inclusion_height`,
`chain::tests::chain_settles_valid_tensorwork_and_rewards_participants`, and
`chain::tests::chain_settles_valid_graph_execution_and_delays_rewards` now also prove receipt reward claims
explicitly await canonical blockspace inclusion before their reward-maturity clock starts.
`chain::tests::pending_reward_claim_view_covers_all_ledgers`,
`app::status::tests::service_status_exports_pending_reward_claim_maturity_details`,
`rpc::tests::routes::explorer_overview_exports_validator_audit_economic_calibration`, and
`tensor_vm_explorer::tests::explorer_json_and_shell_include_live_websocket_contract` now prove pending
reward views expose awaiting-inclusion receipt rewards explicitly, with no synthetic far-future claim
height workaround. `chain::tests::commands::chain_engine_applies_profile_neutral_commands` now also proves
new `ReceiptRewardPending` settlement events expose `claimable_at_height=None` and
`awaiting_inclusion=true` for newly pending miner and validator receipt rewards.

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
  validation with traversal/query/fragment path rejection, signed deployed detection-measurement evidence
  with raw records whose aggregate root matches the signed summary, and deployed public-service reachability with exactly
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
  evidence commands, and non-evidence boundary,
  `testnet::tests::codex_local_chain_workflow_records_required_iteration_flow` guarding the Codex
  local-chain workflow artifact for Gate 0, context refresh, Docker gate, validation, blockers, and
  commit/push evidence flow, signed public
  libp2p network-observation CLI generation rejects missing or zero TCP listen ports plus non-public and
  single-label DNS multiaddrs, `evidence network from-service-log` derives signed observation records
  from captured `tvmd node serve` logs while still requiring public listen multiaddrs, process-level
  network-runtime observation roots can be summarized and artifact-bound from external-addressed records or
  saved raw-record files with exact unpadded root-list parsing and full signed network-observation line
  validation before aggregation, and
  file-derived block/finality/randomness-beacon/data-availability/invalid-work/reward/detection supporting record summaries validate
  typed raw-record fields, including reward-settlement participant IDs and bounded detection
  sample/detected counts, before exact-line hashing while
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
