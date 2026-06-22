use super::*;
use std::collections::BTreeMap;
use tensor_vm::app::{
    ValidatorRemoteTensorResponse, fetch_validator_role_missing_tensors,
    submit_validator_role_attestation, submit_validator_role_audit_report,
    submit_validator_role_block_proposal, submit_validator_role_block_vote,
    validator_remote_tensor_response, validator_role_audit_observation,
    validator_role_work_observation,
};

#[test]
fn validator_role_work_observation_tracks_assigned_unattested_receipts() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"validator-work-observation"]));
    let miner = address(b"validator-work-miner");
    let validator = address(b"validator-work-validator");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert_eq!(
        observation.unattested_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_ready_receipts.is_empty());
    assert_eq!(
        observation.artifact_missing_receipts,
        BTreeSet::from([receipt_id])
    );

    insert_bundle_tensors(&mut node, &bundle);
    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert_eq!(
        observation.unattested_receipts,
        BTreeSet::from([receipt_id])
    );
    assert_eq!(
        observation.artifact_ready_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_missing_receipts.is_empty());
}

#[test]
fn validator_role_attestation_submission_skips_missing_unregistered_unassigned_and_duplicates() {
    let params = ChainParams {
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(
        params,
        hash_bytes(b"test", &[b"validator-attestation-skip"]),
    );
    let miner = address(b"validator-attestation-miner");
    let validator_a = address(b"validator-attestation-a");
    let validator_b = address(b"validator-attestation-b");
    let unknown = address(b"validator-attestation-unknown");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator_a);
    register_validator(&mut chain, validator_b);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt_id);
    let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_validators(
        &chain,
        receipt_id,
        &assignment_seed,
    );
    let assigned = assignment.validators[0];
    let unassigned = [validator_a, validator_b]
        .into_iter()
        .find(|validator| *validator != assigned)
        .expect("one-validator assignment should leave one validator unassigned");
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_validator_role_attestation(&mut node, unknown, receipt_id, None)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_validator_role_attestation(&mut node, unassigned, receipt_id, None)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_validator_role_attestation(&mut node, assigned, receipt_id, None)
            .unwrap()
            .is_none()
    );
    assert!(!node.chain.state().attestations().contains_key(&receipt_id));

    insert_bundle_tensors(&mut node, &bundle);
    let submission = submit_validator_role_attestation(
        &mut node,
        assigned,
        receipt_id,
        Some("validator-role-vrf-secret"),
    )
    .unwrap()
    .expect("assigned validator with local tensors should submit attestation");
    assert_eq!(submission.attestations_submitted, 1);
    let attestations = node
        .chain
        .state()
        .attestations()
        .get(&receipt_id)
        .expect("attestation should be stored");
    assert_eq!(attestations.len(), 1);
    assert_eq!(attestations[0].validator, assigned);
    assert_eq!(attestations[0].result, VerificationResult::Valid);
    let reveal = node
        .chain
        .state()
        .validator_vrf_reveals()
        .values()
        .find(|reveal| reveal.receipt_id == receipt_id && reveal.validator == assigned)
        .expect("validator role should submit a reveal");
    assert_ne!(reveal.vrf_public_key, [0; 32]);
    assert_eq!(reveal.vrf_proof.len(), 64);
    assert!(
        submit_validator_role_attestation(&mut node, assigned, receipt_id, None)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().attestations()[&receipt_id].len(), 1);
    let observation = validator_role_work_observation(&node, assigned);
    assert_eq!(observation.assigned_receipts, BTreeSet::from([receipt_id]));
    assert!(observation.unattested_receipts.is_empty());
    assert!(observation.artifact_ready_receipts.is_empty());
    assert!(observation.artifact_missing_receipts.is_empty());
}

#[test]
fn validator_role_block_vote_submission_finalizes_only_through_votes() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"validator-block-vote"]));
    let validators = [
        address(b"validator-block-vote-a"),
        address(b"validator-block-vote-b"),
        address(b"validator-block-vote-c"),
    ];
    for validator in validators {
        register_validator(&mut chain, validator);
    }
    let block = produce_block(&mut chain, validators[0], 1_000);
    let block_hash = block.hash();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(!node.chain.is_block_finalized(&block_hash));
    assert!(!node.chain.state().block_votes().contains_key(&block_hash));
    assert!(
        submit_validator_role_block_vote(&mut node, address(b"unknown-block-voter"))
            .unwrap()
            .is_none()
    );

    let first = submit_validator_role_block_vote(&mut node, validators[0])
        .unwrap()
        .expect("registered validator should vote on an unfinalized block");
    assert_eq!(first.block_votes_submitted, 1);
    assert!(!node.chain.is_block_finalized(&block_hash));
    assert_eq!(node.chain.state().block_votes()[&block_hash].len(), 1);
    assert!(
        submit_validator_role_block_vote(&mut node, validators[0])
            .unwrap()
            .is_none()
    );

    let second = submit_validator_role_block_vote(&mut node, validators[1])
        .unwrap()
        .expect("second validator should reach the finality threshold");
    assert_eq!(second.block_votes_submitted, 1);
    assert!(node.chain.is_block_finalized(&block_hash));
    assert!(
        submit_validator_role_block_vote(&mut node, validators[2])
            .unwrap()
            .is_none()
    );
}

#[test]
fn validator_role_audit_report_submission_observes_assignments_and_skips_duplicates() {
    let params = ChainParams {
        agreement_quorum: 1,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 1,
        validator_audit_window_blocks: 3,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(
        params,
        hash_bytes(b"test", &[b"validator-role-audit-report"]),
    );
    let miner = address(b"validator-role-audit-miner");
    let proposer = address(b"validator-role-audit-proposer");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, proposer);
    let validators: Vec<_> = (0..5)
        .map(|index| address(format!("validator-role-audit-validator-{index}").as_bytes()))
        .collect();
    for validator in &validators {
        register_validator(&mut chain, *validator);
    }
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let assignment = JobScheduler::with_small_shape((8, 8, 8)).assign_validators(
        &chain,
        receipt_id,
        &chain.validator_assignment_seed(&receipt_id),
    );
    let audited = assignment.validators[0];
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    insert_bundle_tensors(&mut node, &bundle);
    submit_validator_role_attestation(&mut node, audited, receipt_id, None)
        .unwrap()
        .expect("assigned validator should attest before audit assignment");
    let proposer = node
        .chain
        .proposer_for_next_epoch(&node.chain.state().finalized_randomness())
        .unwrap_or(proposer);
    node.chain.produce_block(proposer, 1_000).unwrap();
    let audit_id = *node
        .chain
        .state()
        .validator_audit_assignments()
        .keys()
        .next()
        .expect("audit assignment should be created");
    let auditor = node.chain.state().validator_audit_assignments()[&audit_id].auditor;
    assert_ne!(auditor, audited);
    let non_selected_auditor = validators
        .iter()
        .copied()
        .find(|validator| *validator != audited && *validator != auditor)
        .expect("separate non-selected auditor should be available");
    assert!(
        validator_role_audit_observation(&node, non_selected_auditor)
            .assigned_audits
            .is_empty()
    );

    let observation = validator_role_audit_observation(&node, auditor);
    assert_eq!(observation.assigned_audits, BTreeSet::from([audit_id]));
    assert_eq!(observation.unreported_audits, BTreeSet::from([audit_id]));
    assert_eq!(
        observation.artifact_ready_audits,
        BTreeSet::from([audit_id])
    );
    assert!(observation.artifact_missing_audits.is_empty());

    let submission = submit_validator_role_audit_report(&mut node, auditor, audit_id)
        .unwrap()
        .expect("registered auditor should submit report");
    assert_eq!(submission.audit_reports_submitted, 1);
    assert!(node.chain.state().validator_audit_results()[&audit_id].passed);
    assert!(
        submit_validator_role_audit_report(&mut node, auditor, audit_id)
            .unwrap()
            .is_none()
    );
    let observation = validator_role_audit_observation(&node, auditor);
    assert!(observation.unreported_audits.is_empty());
    assert!(observation.artifact_ready_audits.is_empty());
}

#[test]
fn validator_role_block_proposal_uses_settled_state_without_synthesizing_finality() {
    let params = ChainParams {
        replication_factor: 2,
        agreement_quorum: 2,
        freivalds: FreivaldsParams {
            validators_per_job: 2,
            minimum_validators: 2,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"validator-block-proposal"]));
    let validators = [
        address(b"validator-block-proposal-a"),
        address(b"validator-block-proposal-b"),
    ];
    for index in 0..2 {
        register_miner(
            &mut chain,
            address(format!("validator-block-proposal-miner-{index}").as_bytes()),
        );
    }
    for validator in validators {
        register_validator(&mut chain, validator);
    }
    tensor_vm::localnet::produce_synthetic_cpu_work_with_profile(
        &mut chain,
        &ChainProfile::local_cpu(),
    )
    .unwrap()
    .expect("synthetic work should settle before validator proposal");
    assert_eq!(chain.blocks().len(), 0);
    assert!(!chain.state().settled_receipts().is_empty());
    let settled_before = chain.state().settled_receipts().clone();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_validator_role_block_proposal(&mut node, address(b"unknown-block-proposer"), 1_000,)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.blocks().len(), 0);

    let proposal = submit_validator_role_block_proposal(&mut node, validators[0], 1_000)
        .unwrap()
        .expect("registered validator should propose a block");
    assert_eq!(proposal.blocks_proposed, 1);
    assert_eq!(node.chain.blocks().len(), 1);
    assert_eq!(node.chain.state().height(), 1);
    assert_eq!(node.chain.state().settled_receipts(), &settled_before);
    let block = node.chain.blocks().last().unwrap();
    let block_hash = block.hash();
    assert_eq!(block.proposer, validators[0]);
    assert!(node.chain.validate_block(block).is_ok());
    assert_eq!(
        node.chain
            .selected_receipts_for_block(block)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        settled_before
    );
    assert!(!node.chain.state().block_votes().contains_key(&block_hash));
    assert!(!node.chain.is_block_finalized(&block_hash));
}

#[test]
fn validator_remote_tensor_response_rejects_corrupt_or_mismatched_payloads() {
    let tensor =
        Tensor::from_vec(vec![2, 2], tensor_vm::DType::FieldElement, vec![1, 3, 5, 7]).unwrap();
    let requested_root = tensor.commitment_root();
    let payload = tensor_vm::encode_tensor_payload(&tensor);
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(payload.clone()),
            },
        ),
        ValidatorRemoteTensorResponse::Found {
            tensor: tensor.clone(),
            bytes: payload.len(),
        }
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: None,
            },
        ),
        ValidatorRemoteTensorResponse::Missing
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: hash_bytes(b"test", &[b"wrong-response-root"]),
                payload: Some(payload.clone()),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(vec![255, 0, 1]),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
    let other_tensor =
        Tensor::from_vec(vec![2, 2], tensor_vm::DType::FieldElement, vec![2, 3, 5, 7]).unwrap();
    assert_eq!(
        validator_remote_tensor_response(
            requested_root,
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: requested_root,
                payload: Some(tensor_vm::encode_tensor_payload(&other_tensor)),
            },
        ),
        ValidatorRemoteTensorResponse::Invalid
    );
}

#[test]
fn validator_role_fetches_remote_tensors_before_attesting() {
    let params = ChainParams {
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"validator-remote-fetch"]));
    let miner = address(b"validator-remote-fetch-miner");
    let validator = address(b"validator-remote-fetch-validator");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-fetch-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    for tensor in bundle.served_tensors() {
        provider.register_tensor(tensor);
    }
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-fetch-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(
        observation.artifact_missing_receipts,
        BTreeSet::from([receipt_id])
    );
    let store = NodeStore::open(unique_temp_data_dir("validator-remote-fetch"));
    let report =
        fetch_validator_role_missing_tensors(&store, &mut node, &requester, receipt_id).unwrap();
    assert_eq!(report.successes, 3);
    assert_eq!(report.failures, 0);
    assert_eq!(report.tensors_inserted, 3);
    assert!(report.attempts >= 3);
    assert!(report.bytes > 0);
    assert_tensor_count(&node, 3);

    let observation = validator_role_work_observation(&node, validator);
    assert_eq!(
        observation.artifact_ready_receipts,
        BTreeSet::from([receipt_id])
    );
    assert!(observation.artifact_missing_receipts.is_empty());
    let submission = submit_validator_role_attestation(&mut node, validator, receipt_id, None)
        .unwrap()
        .expect("remote-fetched tensors should allow attestation");
    assert_eq!(submission.attestations_submitted, 1);
    assert_eq!(
        node.chain.state().attestations()[&receipt_id][0].result,
        VerificationResult::Valid
    );
}

#[test]
fn validator_role_fetches_remote_graph_const_blobs_before_attesting() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let miner = address(b"validator-remote-graph-miner");
    let validator = address(b"validator-remote-graph-validator");
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"validator-remote-graph"]));
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let (graph, input, blob, job) = graph_job_with_const_blob(&chain);
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id: job.graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::GraphExecution(job.clone()),
        ))
        .unwrap();
    let const_blobs = BTreeMap::from([(hex(&blob.commitment_root()), blob.clone())]);
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_graph_job(
            &job,
            &graph,
            &BTreeMap::from([("x".to_owned(), input.clone())]),
            &const_blobs,
            chain.state().height(),
            1,
        )
        .unwrap();
    let receipt_id = bundle.receipt_id();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt.clone()))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let provider_port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{provider_port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-graph-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    provider.register_tensor(input.clone());
    provider.register_tensor(blob.clone());
    for tensor in bundle.served_tensors() {
        provider.register_tensor(tensor);
    }
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{provider_port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"validator-remote-graph-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);

    assert!(
        submit_validator_role_attestation(&mut node, validator, receipt_id, None)
            .unwrap()
            .is_none()
    );
    let store = NodeStore::open(unique_temp_data_dir("validator-remote-graph-fetch"));
    let report =
        fetch_validator_role_missing_tensors(&store, &mut node, &requester, receipt_id).unwrap();
    assert_eq!(report.successes, 3);
    assert_eq!(report.tensors_inserted, 3);

    let attestation = submit_validator_role_attestation(&mut node, validator, receipt_id, None)
        .unwrap()
        .expect("remote graph tensors and const_blob should allow attestation");
    assert_eq!(attestation.attestations_submitted, 1);
}

fn graph_job_with_const_blob(
    chain: &Chain,
) -> (
    tensor_vm::TensorGraph,
    Tensor,
    Tensor,
    tensor_vm::jobs::GraphJob,
) {
    let input = Tensor::from_vec(vec![2], tensor_vm::DType::FieldElement, vec![5, 6]).unwrap();
    let blob = Tensor::from_vec(vec![2], tensor_vm::DType::FieldElement, vec![1, 2]).unwrap();
    let blob_uri = hex(&blob.commitment_root());
    let graph = tensor_vm::TensorGraph {
        ir_version: 1,
        inputs: vec![tensor_vm::TensorSpec::field("x", vec![2])],
        params: Vec::new(),
        ops: vec![tensor_vm::OpNode {
            id: 0,
            op: "add".to_owned(),
            args: vec![
                tensor_vm::IrRef::Input {
                    name: "x".to_owned(),
                },
                tensor_vm::IrRef::ConstBlob {
                    uri: blob_uri,
                    shape: vec![2],
                    dtype: tensor_vm::DType::FieldElement,
                },
            ],
            kwargs: BTreeMap::new(),
            out: vec![tensor_vm::TensorSpec::field("y", vec![2])],
        }],
        outputs: vec![tensor_vm::GraphOutput {
            name: "y".to_owned(),
            value: tensor_vm::IrRef::Op { id: 0, idx: 0 },
        }],
    };
    let graph_id = graph.validate_for_consensus().unwrap();
    let job = tensor_vm::jobs::GraphJob::new(
        chain.state().epoch(),
        graph_id,
        BTreeMap::from([("x".to_owned(), input.commitment_root())]),
        BTreeMap::new(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
        1,
        2,
    );
    (graph, input, blob, job)
}
