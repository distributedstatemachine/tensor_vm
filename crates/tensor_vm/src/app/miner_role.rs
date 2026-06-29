use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Chain, ChainCommand, ChainEngine, JobScheduler, JobState, NodeRuntimeState, NodeStore,
    ReceiptState, RpcHttpServer, RpcNode, Tensor, TensorGraph, TensorVmLibp2pService,
    api::P2pMessage,
    encode_tensor_payload,
    error::TvmError,
    hash::hex,
    jobs::GraphReceipt,
    merkle::merkle_root,
    roles::{
        GraphJobExecution, RoleReceiptArtifacts, RoleReceiptBundle, execute_graph_job_with_backend,
        execute_job_with_backend,
    },
    runtime::{BackendKind, CpuReferenceBackend, GpuMinerBackend},
    types::{Address, Hash, hash_bytes, parse_hash_hex},
};

use super::{
    ServiceRuntimeConfig, chain_announcement_checkpoint,
    network::{publish_runtime_trace_bisection_round, submit_runtime_trace_bisection_round},
    persist_runtime_tensor, publish_new_chain_announcements, runtime_role_wallet_registration,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinerRoleReceiptSubmission {
    pub receipts_submitted: usize,
    pub tensors_inserted: usize,
    pub served_tensors: Vec<Tensor>,
    pub backend_kind: BackendKind,
}

pub fn submit_miner_role_receipt(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
) -> std::result::Result<Option<MinerRoleReceiptSubmission>, String> {
    submit_miner_role_receipt_with_device(node, miner, job_id, "cpu", false)
}

pub fn submit_miner_role_receipt_with_device(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
    device: &str,
    malicious_committee_miner: bool,
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
    let (bundle, backend_kind) = execute_miner_role_job_with_device(
        node,
        miner,
        &job,
        job_id,
        device,
        malicious_committee_miner,
    )?;
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
        backend_kind,
    }))
}

fn execute_miner_role_job_with_device(
    node: &RpcNode,
    miner: Address,
    job: &JobState,
    job_id: Hash,
    device: &str,
    malicious_committee_miner: bool,
) -> std::result::Result<(crate::RoleReceiptBundle, BackendKind), String> {
    let device = device.trim();
    // Fault injection: a malicious miner submits a non-canonical Tier-C committee
    // receipt (honest inputs + claimed outputs, tampered op trace) so honest
    // challengers open a live §8.2 trace-bisection dispute that slashes it.
    if malicious_committee_miner
        && device == "cpu"
        && let Some(result) = try_malicious_committee_bundle(node, miner, job, job_id)?
    {
        return Ok(result);
    }
    let result = match job {
        JobState::GraphExecution(graph_job) if device == "cpu" || device.starts_with("cuda:") => {
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
            if device == "cpu" {
                execute_graph_job_with_backend(
                    miner,
                    CpuReferenceBackend,
                    GraphJobExecution {
                        job: graph_job,
                        graph: &graph,
                        inputs: &inputs,
                        const_blobs: &const_blobs,
                        submitted_at_block: node.chain.state().height(),
                        execution_time_ms: 1,
                    },
                )
                .map(|bundle| (bundle, BackendKind::CpuReference))
            } else {
                execute_graph_job_with_backend(
                    miner,
                    GpuMinerBackend::new(device),
                    GraphJobExecution {
                        job: graph_job,
                        graph: &graph,
                        inputs: &inputs,
                        const_blobs: &const_blobs,
                        submitted_at_block: node.chain.state().height(),
                        execution_time_ms: 1,
                    },
                )
                .map(|bundle| {
                    (
                        bundle,
                        BackendKind::GpuMiner {
                            device: device.to_owned(),
                        },
                    )
                })
            }
        }
        JobState::GraphExecution(_) => Err(TvmError::InvalidReceipt("unsupported miner device")),
        JobState::TensorOp(_) | JobState::LinearTrainingStep(_) if device == "cpu" => {
            execute_job_with_backend(
                miner,
                CpuReferenceBackend,
                job,
                node.chain.state().height(),
                1,
            )
            .map(|bundle| (bundle, BackendKind::CpuReference))
        }
        JobState::TensorOp(_) | JobState::LinearTrainingStep(_) if device.starts_with("cuda:") => {
            execute_job_with_backend(
                miner,
                GpuMinerBackend::new(device),
                job,
                node.chain.state().height(),
                1,
            )
            .map(|bundle| {
                (
                    bundle,
                    BackendKind::GpuMiner {
                        device: device.to_owned(),
                    },
                )
            })
        }
        JobState::TensorOp(_) | JobState::LinearTrainingStep(_) => {
            Err(TvmError::InvalidReceipt("unsupported miner device"))
        }
    };
    result.map_err(|error| format!("miner role failed to execute job {}: {error}", hex(&job_id)))
}

/// Fault-injection bundle: for a Tier-C committee graph job, compute the honest
/// execution but commit a tampered op trace (a wrong op output root + recomputed
/// trace root) while keeping the honest claimed output roots. The receipt is thus
/// internally a lie — its trace disagrees with a faithful re-execution — so an
/// honest challenger opens a §8.2 trace-bisection dispute, the referee re-executes
/// the isolated op on-chain, and this miner is slashed. The honest output tensors
/// are still served so challengers can detect the disagreement.
fn try_malicious_committee_bundle(
    node: &RpcNode,
    miner: Address,
    job: &JobState,
    job_id: Hash,
) -> std::result::Result<Option<(RoleReceiptBundle, BackendKind)>, String> {
    let JobState::GraphExecution(graph_job) = job else {
        return Ok(None);
    };
    let graph = graph_from_program_body(node, &graph_job.graph_id)?;
    if !graph.requires_committee_verification() {
        return Ok(None);
    }
    let mut inputs = BTreeMap::new();
    for (name, root) in &graph_job.input_roots {
        let Some(tensor) = node.tensor_by_commitment_root(root).cloned() else {
            return Ok(None);
        };
        inputs.insert(name.clone(), tensor);
    }
    let const_blobs = graph_const_blobs_from_node(node, &graph)?;
    let execution = graph_job
        .committee_ir_execution_with_const_blobs(&graph, &inputs, &const_blobs)
        .map_err(|error| {
            format!(
                "malicious miner failed to execute committee job {}: {error}",
                hex(&job_id)
            )
        })?;
    if execution.op_traces.is_empty() {
        return Ok(None);
    }
    let honest_output_roots = execution
        .outputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let mut tampered = execution.clone();
    let bad_op_output_root = hash_bytes(b"tensor-vm-malicious-committee-miner-v1", &[&job_id]);
    tampered.op_traces[0].output_roots = vec![bad_op_output_root];
    tampered.trace_root = merkle_root(&tampered.trace_leaves());
    let receipt = GraphReceipt::from_roots(
        graph_job,
        miner,
        honest_output_roots,
        tampered.trace_root,
        node.chain.state().height(),
        1,
    );
    let bundle = RoleReceiptBundle {
        receipt: ReceiptState::GraphExecution(receipt),
        artifacts: RoleReceiptArtifacts::GraphExecution {
            graph,
            inputs,
            const_blobs,
            outputs: execution.outputs,
        },
    };
    Ok(Some((bundle, BackendKind::CpuReference)))
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
        if let Some(submission) = submit_miner_role_receipt_with_device(
            &mut server.gateway_mut().node,
            miner,
            job_id,
            config.miner_device.as_deref().unwrap_or("cpu"),
            config.node.profile.malicious_committee_miner,
        )? {
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
            let interim_tensor_gossip = config.node.profile.interim_tensor_gossip;
            for tensor in submission.served_tensors {
                persist_runtime_tensor(store, &server.gateway().node.chain, &tensor)?;
                // Interim belt-and-suspenders: gossip-relay served tensors so
                // committee validators / challengers can detect a bad-trace receipt
                // multi-hop. Content routing (register_tensor's provider record
                // below) is the canonical path; disabled once pure-DHT is validated.
                if interim_tensor_gossip {
                    p2p_service
                        .publish_gossip(P2pMessage::NewJobInputTensorPayload {
                            commitment_root: tensor.commitment_root(),
                            payload: encode_tensor_payload(&tensor),
                        })
                        .map_err(|error| {
                            format!("failed to publish miner served tensor gossip: {error}")
                        })?;
                }
                // Advertise a Kademlia provider record for content routing.
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
    if let Some(round) =
        submit_runtime_trace_bisection_round(&mut server.gateway_mut().node, miner)?
    {
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist trace-bisection round state: {error}"))?;
        publish_runtime_trace_bisection_round(p2p_service, &round)?;
        runtime_state.record_miner_trace_bisection_round_submission(1);
        status_changed = true;
    }
    Ok(status_changed)
}

#[cfg(test)]
mod malicious_tests {
    use super::*;
    use crate::types::address;
    use crate::{ChainParams, jobs::GraphJob, scheduler::SyntheticLocalJobSource};
    use crate::{chain::Chain, verify::FreivaldsParams};

    #[test]
    fn malicious_committee_miner_submits_a_disputable_tier_c_receipt() {
        // A malicious miner's Tier-C committee receipt must claim the honest
        // output roots but commit a tampered trace, so an honest re-execution
        // disagrees on the trace root (the trigger for a §8.2 dispute) — while a
        // normal (honest) miner produces the canonical trace.
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
        let miner = address(b"malicious-committee-miner");
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"malicious-committee"]));
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake: chain.params().miner_min_stake,
            })
            .unwrap();
        chain.set_position_for_testing(2, 0);

        let graph = SyntheticLocalJobSource::committee_graph_execution_graph();
        let graph_id = graph.validate_for_committee().unwrap();
        let inputs = SyntheticLocalJobSource::committee_graph_execution_inputs();
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 4);
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
                job.clone(),
            )))
            .unwrap();
        let mut node = RpcNode::new(chain);
        for tensor in inputs.values() {
            node.insert_tensor(tensor.clone());
        }

        let honest_trace_root = job
            .committee_ir_execution_with_const_blobs(&graph, &inputs, &BTreeMap::new())
            .unwrap()
            .trace_root;

        submit_miner_role_receipt_with_device(&mut node, miner, job.job_id, "cpu", true)
            .unwrap()
            .expect("malicious miner should still submit a (lying) committee receipt");

        let ReceiptState::GraphExecution(receipt) = node
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("malicious committee receipt must be stored")
            .clone()
        else {
            panic!("expected a graph execution receipt");
        };
        // Honest claimed inputs/outputs (so it is accepted and detectable) ...
        assert_eq!(receipt.input_roots, job.input_roots);
        // ... but a tampered trace that disagrees with a faithful re-execution.
        assert_ne!(receipt.trace_root, honest_trace_root);
    }
}
