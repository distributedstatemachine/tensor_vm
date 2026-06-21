use super::*;
use std::collections::BTreeMap;
use tensor_vm::app::{
    MinerRoleWorkObservation, RuntimeRole, ServiceRuntimeConfig,
    fetch_miner_role_missing_graph_artifacts, miner_role_work_observation, runtime_node_config,
    start_runtime_services, submit_miner_role_receipt, tick_miner_role_work_once,
};

#[test]
fn miner_role_work_observation_tracks_assigned_unreceipted_jobs() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-observation"]));
    let miner = address(b"miner-work-observation-miner");
    register_miner(&mut chain, miner);
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
    let job_id = job.job_id;
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert_eq!(observation.unreceipted_jobs, BTreeSet::from([job_id]));

    let bundle = tensor_vm::roles::CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_work_observation_ignores_unassigned_miners() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-unassigned"]));
    let miner = address(b"miner-work-assigned");
    let unassigned = address(b"miner-work-unassigned");
    register_miner(&mut chain, miner);
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
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();

    assert_eq!(
        miner_role_work_observation(&chain, unassigned),
        MinerRoleWorkObservation::default()
    );
}

#[test]
fn miner_role_submits_assigned_unreceipted_tensor_op_once() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-receipt-submit"]));
    let miner = address(b"miner-receipt-submit-miner");
    register_miner(&mut chain, miner);
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
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let submission = submit_miner_role_receipt(&mut node, miner, job_id)
        .unwrap()
        .expect("assigned unreceipted job should submit a receipt");

    assert_eq!(submission.receipts_submitted, 1);
    assert_eq!(submission.tensors_inserted, 3);
    assert_eq!(node.chain.state().receipts().len(), 1);
    let receipt = node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("receipt should be stored");
    assert_eq!(receipt.job_id(), job_id);
    assert_eq!(receipt.miner(), miner);
    assert_tensor_count(&node, 3);
    let observation = miner_role_work_observation(&node.chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_receipt_submission_skips_duplicate_unregistered_and_unassigned_work() {
    let params = ChainParams {
        replication_factor: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-receipt-skip"]));
    let miner_a = address(b"miner-receipt-skip-a");
    let miner_b = address(b"miner-receipt-skip-b");
    let unknown = address(b"miner-receipt-skip-unknown");
    register_miner(&mut chain, miner_a);
    register_miner(&mut chain, miner_b);
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
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let assignment_seed = chain.miner_assignment_seed(&job_id);
    let assignment =
        JobScheduler::with_small_shape((8, 8, 8)).assign_miners(&chain, job_id, &assignment_seed);
    let assigned = assignment.miners[0];
    let unassigned = [miner_a, miner_b]
        .into_iter()
        .find(|miner| *miner != assigned)
        .expect("replication factor one should leave one registered miner unassigned");
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_miner_role_receipt(&mut node, unknown, job_id)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_miner_role_receipt(&mut node, unassigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 0);

    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
}

#[test]
fn miner_role_fetches_remote_graph_inputs_and_const_blobs_before_execution() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let miner = address(b"miner-remote-graph-artifact");
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-remote-graph"]));
    register_miner(&mut chain, miner);
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
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let provider_port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{provider_port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"miner-remote-graph-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    provider.register_tensor(input.clone());
    provider.register_tensor(blob.clone());
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{provider_port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"miner-remote-graph-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);

    assert!(submit_miner_role_receipt(&mut node, miner, job.job_id).is_err());
    let report =
        fetch_miner_role_missing_graph_artifacts(&mut node, &requester, job.job_id).unwrap();
    assert_eq!(report.successes, 2);
    assert_eq!(report.tensors_inserted, 2);

    let submission = submit_miner_role_receipt(&mut node, miner, job.job_id)
        .unwrap()
        .expect("fetched graph artifacts should let miner submit a receipt");
    assert_eq!(submission.receipts_submitted, 1);
    assert!(matches!(
        node.chain.state().receipts().values().next(),
        Some(ReceiptState::GraphExecution(_))
    ));
}

#[test]
fn miner_role_tick_keeps_missing_graph_artifacts_pending_without_exiting() {
    let data_dir = unique_temp_data_dir("miner-missing-graph-pending");
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let miner = address(b"miner-missing-graph-pending");
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-missing-graph"]));
    register_miner(&mut chain, miner);
    let (graph, _input, _blob, job) = graph_job_with_const_blob(&chain);
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
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let config = ServiceRuntimeConfig {
        runtime_command: "miner_run",
        role: RuntimeRole::Miner,
        role_wallet_address: Some(miner),
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Miner,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[b"miner-missing-graph-identity"])),
            "secret",
            0,
        )
        .unwrap(),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };
    let mut services = start_runtime_services(&config).unwrap();
    let mut runtime_state = NodeRuntimeState::default();

    assert!(
        tick_miner_role_work_once(
            &config,
            &services.store,
            &mut services.server,
            &services.p2p_service,
            &mut runtime_state,
        )
        .unwrap()
    );
    assert!(
        services
            .server
            .gateway()
            .node
            .chain
            .state()
            .receipts()
            .is_empty()
    );
    assert!(runtime_state.miner_receipts_submitted() == 0);
    assert!(runtime_state.miner_tensors_inserted() == 0);

    drop(services);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
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
