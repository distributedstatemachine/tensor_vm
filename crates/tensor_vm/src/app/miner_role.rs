use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Chain, ChainCommand, ChainEngine, JobScheduler, JobState, NodeRuntimeState, NodeStore,
    RpcHttpServer, RpcNode, Tensor, TensorGraph, TensorVmLibp2pService,
    error::TvmError,
    hash::hex,
    roles::CpuReferenceMinerRole,
    types::{Address, Hash, parse_hash_hex},
};

use super::{
    ServiceRuntimeConfig, chain_announcement_checkpoint, persist_runtime_tensor,
    publish_new_chain_announcements, runtime_role_wallet_registration,
    validator_fetch::fetch_miner_role_missing_graph_artifacts,
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
    let bundle = execute_miner_role_job(node, miner, &job, job_id)?;
    if bundle.receipt.job_id() != job_id || bundle.receipt.miner() != miner {
        return Err("miner role produced receipt for the wrong job or miner".to_owned());
    }
    let served_tensors = bundle.served_tensors();
    if let Err(error) = node
        .chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
    {
        if matches!(
            error,
            TvmError::InvalidReceipt("receipt submitted after deadline")
        ) {
            return Ok(None);
        }
        return Err(format!(
            "miner role failed to submit receipt {}: {error}",
            hex(&job_id)
        ));
    }
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

fn execute_miner_role_job(
    node: &RpcNode,
    miner: Address,
    job: &JobState,
    job_id: Hash,
) -> std::result::Result<crate::RoleReceiptBundle, String> {
    let role = CpuReferenceMinerRole::new(miner);
    match job {
        JobState::GraphExecution(graph_job) => {
            let graph = graph_from_program_body(node, &graph_job.graph_id)?;
            let mut inputs = std::collections::BTreeMap::new();
            for (name, root) in &graph_job.input_roots {
                let Some(tensor) = node.tensor_by_commitment_root(root).cloned() else {
                    return Err(format!(
                        "miner role missing graph input tensor {} for job {}",
                        name,
                        hex(&job_id)
                    ));
                };
                inputs.insert(name.clone(), tensor);
            }
            let const_blobs = graph_const_blobs_from_node(node, &graph)?;
            role.execute_graph_job(
                graph_job,
                &graph,
                &inputs,
                &const_blobs,
                node.chain.state().height(),
                1,
            )
        }
        JobState::TensorOp(_) | JobState::LinearTrainingStep(_) => {
            role.execute_job(job, node.chain.state().height(), 1)
        }
    }
    .map_err(|error| format!("miner role failed to execute job {}: {error}", hex(&job_id)))
}

fn graph_const_blobs_from_node(
    node: &RpcNode,
    graph: &TensorGraph,
) -> std::result::Result<BTreeMap<String, Tensor>, String> {
    let mut const_blobs = BTreeMap::new();
    for (uri, _) in graph
        .const_blob_specs()
        .map_err(|error| format!("miner role invalid graph const_blob spec: {error}"))?
    {
        let root = parse_hash_hex(&uri)
            .map_err(|_| format!("miner role invalid graph const_blob uri {uri}"))?;
        let Some(tensor) = node.tensor_by_commitment_root(&root).cloned() else {
            return Err(format!("miner role missing graph const_blob tensor {uri}"));
        };
        const_blobs.insert(uri, tensor);
    }
    Ok(const_blobs)
}

fn graph_from_program_body(
    node: &RpcNode,
    graph_id: &Hash,
) -> std::result::Result<TensorGraph, String> {
    let Some(bytes) = node.chain.state().program_body(graph_id) else {
        return Err(format!(
            "miner role missing graph program body {}",
            hex(graph_id)
        ));
    };
    TensorGraph::from_canonical_json_bytes(bytes)
        .map_err(|error| format!("miner role failed to parse graph program body: {error}"))
}

fn graph_artifacts_available_for_job(
    node: &RpcNode,
    job_id: Hash,
) -> std::result::Result<bool, String> {
    let Some(JobState::GraphExecution(graph_job)) = node.chain.state().jobs().get(&job_id) else {
        return Ok(true);
    };
    let graph = match graph_from_program_body(node, &graph_job.graph_id) {
        Ok(graph) => graph,
        Err(error) if error.starts_with("miner role missing graph program body") => {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    for root in graph_job.input_roots.values() {
        if !node.contains_tensor_commitment_root(root) {
            return Ok(false);
        }
    }
    for (uri, _) in graph
        .const_blob_specs()
        .map_err(|error| format!("miner role invalid graph const_blob spec: {error}"))?
    {
        let root = parse_hash_hex(&uri)
            .map_err(|_| format!("miner role invalid graph const_blob uri {uri}"))?;
        if !node.contains_tensor_commitment_root(&root) {
            return Ok(false);
        }
    }
    Ok(true)
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
        let fetch_report = fetch_miner_role_missing_graph_artifacts(
            store,
            &mut server.gateway_mut().node,
            p2p_service,
            job_id,
        )?;
        if fetch_report.has_activity() {
            if fetch_report.programs_registered > 0 {
                store
                    .persist_chain(&server.gateway().node.chain)
                    .map_err(|error| format!("failed to persist fetched graph program: {error}"))?;
            }
            status_changed = true;
        }
        if !graph_artifacts_available_for_job(&server.gateway().node, job_id)? {
            return Ok(status_changed);
        }
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
                persist_runtime_tensor(store, &server.gateway().node.chain, &tensor)?;
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
