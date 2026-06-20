use std::collections::BTreeSet;

use crate::{
    Chain, ChainCommand, ChainEngine, JobScheduler, NodeRuntimeState, NodeStore, RpcHttpServer,
    RpcNode, Tensor, TensorVmLibp2pService,
    hash::hex,
    roles::CpuReferenceMinerRole,
    types::{Address, Hash},
};

use super::{
    ServiceRuntimeConfig, chain_announcement_checkpoint, publish_new_chain_announcements,
    runtime_role_wallet_registration,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MinerRoleWorkObservation {
    pub assigned_jobs: BTreeSet<Hash>,
    pub unreceipted_jobs: BTreeSet<Hash>,
}

pub fn miner_role_work_observation(chain: &Chain, miner: Address) -> MinerRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let mut observation = MinerRoleWorkObservation::default();
    for job_id in chain.state().jobs().keys() {
        let assignment_seed = chain.miner_assignment_seed(job_id);
        let assignment = scheduler.assign_miners(chain, *job_id, &assignment_seed);
        if !assignment.miners.contains(&miner) {
            continue;
        }
        observation.assigned_jobs.insert(*job_id);
        if !miner_has_receipt_for_job(chain, miner, *job_id) {
            observation.unreceipted_jobs.insert(*job_id);
        }
    }
    observation
}

fn miner_has_receipt_for_job(chain: &Chain, miner: Address, job_id: Hash) -> bool {
    chain
        .state()
        .receipts()
        .values()
        .any(|receipt| receipt.job_id() == job_id && receipt.miner() == miner)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MinerRoleReceiptSubmission {
    pub receipts_submitted: usize,
    pub tensors_inserted: usize,
    pub served_tensors: Vec<Tensor>,
}

pub fn submit_miner_role_receipt(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
) -> std::result::Result<Option<MinerRoleReceiptSubmission>, String> {
    if !node.chain.state().miners().contains_key(&miner) {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = node.chain.miner_assignment_seed(&job_id);
    let assignment = scheduler.assign_miners(&node.chain, job_id, &assignment_seed);
    if !assignment.miners.contains(&miner) || miner_has_receipt_for_job(&node.chain, miner, job_id)
    {
        return Ok(None);
    }
    let Some(job) = node.chain.state().jobs().get(&job_id).cloned() else {
        return Ok(None);
    };
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job, node.chain.state().height(), 1)
        .map_err(|error| format!("miner role failed to execute job {}: {error}", hex(&job_id)))?;
    if bundle.receipt.job_id() != job_id || bundle.receipt.miner() != miner {
        return Err("miner role produced receipt for the wrong job or miner".to_owned());
    }
    let served_tensors = bundle.served_tensors();
    node.chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .map_err(|error| {
            format!(
                "miner role failed to submit receipt {}: {error}",
                hex(&job_id)
            )
        })?;
    let mut tensors_inserted = 0usize;
    for tensor in &served_tensors {
        node.insert_tensor(tensor.clone());
        tensors_inserted = tensors_inserted.saturating_add(1);
    }
    Ok(Some(MinerRoleReceiptSubmission {
        receipts_submitted: 1,
        tensors_inserted,
        served_tensors,
    }))
}

pub fn tick_miner_role_work_once(
    config: &ServiceRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    let Some(miner) = config.role_wallet_address else {
        return Ok(false);
    };
    if runtime_role_wallet_registration(
        config.role,
        config.role_wallet_address,
        &server.gateway().node.chain,
    ) != "miner"
    {
        return Ok(false);
    }
    let observation = miner_role_work_observation(&server.gateway().node.chain, miner);
    let job_to_submit = observation.unreceipted_jobs.iter().next().copied();
    let mut status_changed = false;
    if runtime_state
        .record_miner_work_observation(observation.assigned_jobs, observation.unreceipted_jobs)
    {
        status_changed = true;
    }
    if let Some(job_id) = job_to_submit {
        let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
        if let Some(submission) =
            submit_miner_role_receipt(&mut server.gateway_mut().node, miner, job_id)?
        {
            publish_new_chain_announcements(
                p2p_service,
                &announcement_checkpoint,
                &server.gateway().node.chain,
            )?;
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| format!("failed to persist miner receipt state: {error}"))?;
            runtime_state.record_miner_receipt_submission(
                submission.receipts_submitted,
                submission.tensors_inserted,
            );
            for tensor in submission.served_tensors {
                p2p_service.register_tensor(tensor);
            }
            let observation = miner_role_work_observation(&server.gateway().node.chain, miner);
            runtime_state.record_miner_work_observation(
                observation.assigned_jobs,
                observation.unreceipted_jobs,
            );
            status_changed = true;
        }
    }
    Ok(status_changed)
}
