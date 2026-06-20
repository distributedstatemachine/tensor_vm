use super::{Chain, JobState, ReceiptRandomnessAnchor, ReceiptState, validation};
use crate::error::{Result, TvmError};
use crate::ir::{GraphId, TensorGraph};
use crate::jobs::{GraphJob, GraphReceipt, LinearTrainingStepReceipt, TensorOpReceipt};
use crate::types::Hash;

pub fn register_program_body(chain: &mut Chain, graph_id: GraphId, bytes: Vec<u8>) -> Result<()> {
    let graph = TensorGraph::from_canonical_json_bytes(&bytes)?;
    let validated_graph_id = graph.validate_for_consensus()?;
    if validated_graph_id != graph_id {
        return Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"));
    }
    if graph.canonical_json().as_bytes() != bytes.as_slice() {
        return Err(TvmError::InvalidReceipt(
            "noncanonical tensor ir graph body",
        ));
    }
    if let Some(existing) = chain.state.program_bodies.get(&graph_id) {
        if existing.as_slice() != bytes.as_slice() {
            return Err(TvmError::InvalidReceipt("conflicting tensor ir graph body"));
        }
        return Ok(());
    }
    chain.state.program_bodies.insert(graph_id, bytes);
    Ok(())
}

pub fn submit_job(chain: &mut Chain, job: JobState) {
    if let Some(graph) = job.tensor_ir_graph()
        && graph.validate_for_consensus().is_ok()
    {
        let graph_id = graph.graph_id();
        chain
            .state
            .program_bodies
            .entry(graph_id)
            .or_insert_with(|| graph.canonical_json().into_bytes());
    }
    chain.state.jobs.insert(job.job_id(), job);
}

pub fn submit_graph_job(chain: &mut Chain, job: &GraphJob) -> Result<()> {
    let graph = registered_graph(chain, &job.graph_id)?;
    validate_graph_job_bindings(&graph, job)?;
    if job.job_id != job.recompute_job_id() {
        return Err(TvmError::InvalidReceipt("graph job id mismatch"));
    }
    Ok(())
}

pub fn job<'a>(chain: &'a Chain, job_id: &Hash) -> Option<&'a JobState> {
    chain.state.jobs.get(job_id)
}

pub fn submit_tensor_op(chain: &mut Chain, receipt: TensorOpReceipt) -> Result<()> {
    if !chain.state.miners.contains_key(&receipt.miner) {
        return Err(TvmError::UnknownMiner);
    }
    if !chain.state.jobs.contains_key(&receipt.job_id) {
        return Err(TvmError::InvalidReceipt("unknown job"));
    }
    if chain.state.receipts.contains_key(&receipt.receipt_id) {
        return Err(TvmError::InvalidReceipt("duplicate receipt"));
    }
    let receipt_id = receipt.receipt_id;
    chain
        .state
        .receipts
        .insert(receipt_id, ReceiptState::TensorOp(receipt));
    anchor_receipt_randomness(chain, receipt_id);
    Ok(())
}

pub fn submit_linear_training_step(
    chain: &mut Chain,
    receipt: LinearTrainingStepReceipt,
) -> Result<()> {
    if !chain.state.miners.contains_key(&receipt.miner) {
        return Err(TvmError::UnknownMiner);
    }
    if !chain.state.jobs.contains_key(&receipt.job_id) {
        return Err(TvmError::InvalidReceipt("unknown job"));
    }
    if chain.state.receipts.contains_key(&receipt.receipt_id) {
        return Err(TvmError::InvalidReceipt("duplicate receipt"));
    }
    let receipt_id = receipt.receipt_id;
    chain
        .state
        .receipts
        .insert(receipt_id, ReceiptState::LinearTrainingStep(receipt));
    anchor_receipt_randomness(chain, receipt_id);
    Ok(())
}

pub fn submit_graph_execution(chain: &mut Chain, receipt: GraphReceipt) -> Result<()> {
    if !chain.state.miners.contains_key(&receipt.miner) {
        return Err(TvmError::UnknownMiner);
    }
    let job = match chain.state.jobs.get(&receipt.job_id) {
        Some(JobState::GraphExecution(job)) => job,
        Some(_) => return Err(TvmError::InvalidReceipt("job primitive mismatch")),
        None => return Err(TvmError::InvalidReceipt("unknown job")),
    };
    registered_graph(chain, &job.graph_id)?;
    if receipt.submitted_at_block > job.deadline_block {
        return Err(TvmError::InvalidReceipt("receipt submitted after deadline"));
    }
    if receipt.graph_id != job.graph_id {
        return Err(TvmError::InvalidReceipt("graph id mismatch"));
    }
    if receipt.input_roots != job.input_roots {
        return Err(TvmError::InvalidReceipt("input roots mismatch"));
    }
    if receipt.tensor_work_units != job.tensor_work_units() {
        return Err(TvmError::InvalidReceipt("tensor work mismatch"));
    }
    if receipt.receipt_id != receipt.recompute_receipt_id() {
        return Err(TvmError::InvalidReceipt("receipt digest mismatch"));
    }
    if !crate::types::verify_signature(&receipt.miner, &receipt.receipt_id, &receipt.signature) {
        return Err(TvmError::InvalidReceipt("bad receipt signature"));
    }
    if chain.state.receipts.contains_key(&receipt.receipt_id) {
        return Err(TvmError::InvalidReceipt("duplicate receipt"));
    }
    let receipt_id = receipt.receipt_id;
    chain
        .state
        .receipts
        .insert(receipt_id, ReceiptState::GraphExecution(receipt));
    anchor_receipt_randomness(chain, receipt_id);
    Ok(())
}

fn registered_graph(chain: &Chain, graph_id: &GraphId) -> Result<TensorGraph> {
    let bytes = chain
        .state
        .program_bodies
        .get(graph_id)
        .ok_or(TvmError::InvalidReceipt("unknown tensor ir graph body"))?;
    let graph = TensorGraph::from_canonical_json_bytes(bytes)?;
    if graph.validate_for_consensus()? != *graph_id {
        return Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"));
    }
    Ok(graph)
}

fn validate_graph_job_bindings(graph: &TensorGraph, job: &GraphJob) -> Result<()> {
    if graph.graph_id() != job.graph_id {
        return Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"));
    }
    if graph.inputs.len() != job.input_roots.len() {
        return Err(TvmError::InvalidReceipt("graph input root mismatch"));
    }
    for input in &graph.inputs {
        if !job.input_roots.contains_key(&input.name) {
            return Err(TvmError::InvalidReceipt("missing graph input root"));
        }
    }
    if graph.params.len() != job.field_params.len() {
        return Err(TvmError::InvalidReceipt("graph param mismatch"));
    }
    for param in &graph.params {
        if !job.field_params.contains_key(&param.name) {
            return Err(TvmError::InvalidReceipt("missing graph param"));
        }
    }
    Ok(())
}

fn anchor_receipt_randomness(chain: &mut Chain, receipt_id: Hash) {
    let beacon_round = chain.state.finalized_beacon_round;
    let finalized_randomness = chain.state.finalized_randomness;
    let assignment_seed =
        validation::assignment_seed(beacon_round, &finalized_randomness, &receipt_id);
    let validation_seed_commitment =
        validation::validation_seed_commitment(beacon_round, &finalized_randomness, &receipt_id);
    chain.state.receipt_randomness_anchors.insert(
        receipt_id,
        ReceiptRandomnessAnchor {
            receipt_id,
            beacon_round,
            finalized_randomness,
            assignment_seed,
            validation_seed_commitment,
        },
    );
}
