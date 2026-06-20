use super::*;
use tensor_vm::app::{
    MinerRoleWorkObservation, miner_role_work_observation, submit_miner_role_receipt,
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
