use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobScheduler, JobState, ReceiptState, RpcNode,
    SyntheticLocalJobSource, Tensor, TensorGraph,
    chain::ValidatorAuditReport,
    hash::hex,
    jobs::LinearTrainingStepOutput,
    roles::{ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle},
    types::{Address, Hash, parse_hash_hex},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleWorkObservation {
    pub assigned_receipts: BTreeSet<Hash>,
    pub unattested_receipts: BTreeSet<Hash>,
    pub artifact_ready_receipts: BTreeSet<Hash>,
    pub artifact_missing_receipts: BTreeSet<Hash>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleAuditObservation {
    pub assigned_audits: BTreeSet<Hash>,
    pub unreported_audits: BTreeSet<Hash>,
    pub artifact_ready_audits: BTreeSet<Hash>,
    pub artifact_missing_audits: BTreeSet<Hash>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleBlockProposalObservation {
    pub settled_receipts: BTreeSet<Hash>,
    pub artifact_ready_receipts: BTreeSet<Hash>,
    pub attested_receipts: BTreeSet<Hash>,
}

pub fn validator_role_work_observation(
    node: &RpcNode,
    validator: Address,
) -> ValidatorRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let mut observation = ValidatorRoleWorkObservation::default();
    for (receipt_id, receipt) in node.chain.state().receipts() {
        let assignment_seed = node.chain.validator_assignment_seed(receipt_id);
        let assignment = scheduler.assign_validators(&node.chain, *receipt_id, &assignment_seed);
        if !assignment.validators.contains(&validator) {
            continue;
        }
        observation.assigned_receipts.insert(*receipt_id);
        if validator_has_attested_for_receipt(&node.chain, validator, *receipt_id) {
            continue;
        }
        observation.unattested_receipts.insert(*receipt_id);
        if role_receipt_bundle_from_local_tensors(node, receipt).is_some() {
            observation.artifact_ready_receipts.insert(*receipt_id);
        } else {
            observation.artifact_missing_receipts.insert(*receipt_id);
        }
    }
    observation
}

pub fn validator_role_audit_observation(
    node: &RpcNode,
    auditor: Address,
) -> ValidatorRoleAuditObservation {
    let mut observation = ValidatorRoleAuditObservation::default();
    if !node.chain.state().validators().contains_key(&auditor) {
        return observation;
    }
    for (audit_id, assignment) in node.chain.state().validator_audit_assignments() {
        if assignment.auditor != auditor {
            continue;
        }
        observation.assigned_audits.insert(*audit_id);
        if node
            .chain
            .state()
            .validator_audit_results()
            .contains_key(audit_id)
            || node
                .chain
                .state()
                .validator_audit_slashes()
                .contains_key(audit_id)
            || node.chain.state().height() > assignment.deadline_height
        {
            continue;
        }
        observation.unreported_audits.insert(*audit_id);
        if let Some(receipt) = node.chain.state().receipts().get(&assignment.receipt_id) {
            if role_receipt_bundle_from_local_tensors(node, receipt).is_some() {
                observation.artifact_ready_audits.insert(*audit_id);
            } else {
                observation.artifact_missing_audits.insert(*audit_id);
            }
        } else {
            observation.artifact_missing_audits.insert(*audit_id);
        }
    }
    observation
}

pub fn validator_role_block_proposal_observation(
    node: &RpcNode,
    validator: Address,
) -> ValidatorRoleBlockProposalObservation {
    if !node.chain.state().validators().contains_key(&validator) {
        return ValidatorRoleBlockProposalObservation::default();
    }
    let mut observation = ValidatorRoleBlockProposalObservation {
        ..ValidatorRoleBlockProposalObservation::default()
    };
    for receipt_id in node.chain.state().settled_receipts() {
        if node.chain.state().included_receipts().contains(receipt_id)
            || node
                .chain
                .state()
                .data_unavailable_receipts()
                .contains(receipt_id)
        {
            continue;
        }
        observation.settled_receipts.insert(*receipt_id);
        if let Some(receipt) = node.chain.state().receipts().get(receipt_id)
            && role_receipt_bundle_from_local_tensors(node, receipt).is_some()
        {
            observation.artifact_ready_receipts.insert(*receipt_id);
        }
        if node
            .chain
            .state()
            .attestations()
            .get(receipt_id)
            .is_some_and(|attestations| !attestations.is_empty())
        {
            observation.attested_receipts.insert(*receipt_id);
        }
    }
    observation
}

fn validator_has_attested_for_receipt(chain: &Chain, validator: Address, receipt_id: Hash) -> bool {
    chain
        .state()
        .attestations()
        .get(&receipt_id)
        .is_some_and(|attestations| {
            attestations
                .iter()
                .any(|attestation| attestation.validator == validator)
        })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleAttestationSubmission {
    pub attestations_submitted: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleAuditReportSubmission {
    pub audit_reports_submitted: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleBlockVoteSubmission {
    pub block_votes_submitted: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleBlockProposal {
    pub blocks_proposed: usize,
    pub useful_blocks_proposed: usize,
    pub fallback_blocks_proposed: usize,
    pub selected_receipts: Vec<Hash>,
}

pub fn submit_validator_role_block_proposal(
    node: &mut RpcNode,
    validator: Address,
    timestamp: u64,
) -> std::result::Result<Option<ValidatorRoleBlockProposal>, String> {
    if !node.chain.state().validators().contains_key(&validator) {
        return Ok(None);
    }
    node.chain
        .prepare_block_parent_state()
        .map_err(|error| format!("validator proposer failed to prepare parent state: {error}"))?;
    let settled_receipts = node.chain.state().settled_receipts().len();
    let proposer_reward = node
        .chain
        .params()
        .reward_allocation(10_000)
        .proposer_reward;
    let block_reward = if settled_receipts > 0 {
        proposer_reward
    } else {
        reduced_fallback_proposer_reward(proposer_reward)
    };
    let command = if block_reward > 0 {
        ChainCommand::ProduceRewardedBlock {
            proposer: validator,
            timestamp,
            fixed_block_reward: block_reward,
            fee_share: 0,
        }
    } else {
        ChainCommand::ProduceBlock {
            proposer: validator,
            timestamp,
        }
    };
    node.chain
        .apply_command(command)
        .map_err(|error| format!("validator proposer failed to produce block: {error}"))?;
    let block = node
        .chain
        .blocks()
        .last()
        .cloned()
        .ok_or_else(|| "validator proposer produced no block".to_owned())?;
    let selected_receipts = node.chain.selected_receipts_for_block(&block);
    Ok(Some(ValidatorRoleBlockProposal {
        blocks_proposed: 1,
        useful_blocks_proposed: usize::from(block.production_kind.requires_pow()),
        fallback_blocks_proposed: usize::from(!block.production_kind.requires_pow()),
        selected_receipts,
    }))
}

fn reduced_fallback_proposer_reward(useful_proposer_reward: u64) -> u64 {
    useful_proposer_reward / 10
}

pub fn submit_validator_role_attestation(
    node: &mut RpcNode,
    validator: Address,
    receipt_id: Hash,
) -> std::result::Result<Option<ValidatorRoleAttestationSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = node.chain.validator_assignment_seed(&receipt_id);
    let assignment = scheduler.assign_validators(&node.chain, receipt_id, &assignment_seed);
    if !assignment.validators.contains(&validator)
        || validator_has_attested_for_receipt(&node.chain, validator, receipt_id)
    {
        return Ok(None);
    }
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(None);
    };
    let Some(job) = node.chain.state().jobs().get(&receipt.job_id()).cloned() else {
        return Ok(None);
    };
    let Some(bundle) = role_receipt_bundle_from_local_tensors(node, &receipt) else {
        return Ok(None);
    };
    let validation_seed = node.chain.validation_seed(&receipt_id, &validator);
    let attestation = ReferenceValidatorRole::new(validator, validator_stake)
        .verify_receipt(
            &job,
            &bundle,
            &validation_seed,
            &node.chain.params().freivalds,
        )
        .map_err(|error| {
            format!(
                "validator role failed to verify receipt {}: {error}",
                hex(&receipt_id)
            )
        })?;
    if attestation.receipt_id != receipt_id || attestation.validator != validator {
        return Err(
            "validator role produced attestation for the wrong receipt or validator".to_owned(),
        );
    }
    node.chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map_err(|error| {
            format!(
                "validator role failed to submit attestation {}: {error}",
                hex(&receipt_id)
            )
        })?;
    Ok(Some(ValidatorRoleAttestationSubmission {
        attestations_submitted: 1,
    }))
}

pub fn submit_validator_role_audit_report(
    node: &mut RpcNode,
    auditor: Address,
    audit_id: Hash,
) -> std::result::Result<Option<ValidatorRoleAuditReportSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&auditor) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let Some(assignment) = node
        .chain
        .state()
        .validator_audit_assignments()
        .get(&audit_id)
        .cloned()
    else {
        return Ok(None);
    };
    if assignment.validator == auditor
        || node
            .chain
            .state()
            .validator_audit_results()
            .contains_key(&audit_id)
        || node
            .chain
            .state()
            .validator_audit_slashes()
            .contains_key(&audit_id)
        || node.chain.state().height() > assignment.deadline_height
    {
        return Ok(None);
    }
    let Some(receipt) = node
        .chain
        .state()
        .receipts()
        .get(&assignment.receipt_id)
        .cloned()
    else {
        return Ok(None);
    };
    let Some(job) = node.chain.state().jobs().get(&receipt.job_id()).cloned() else {
        return Ok(None);
    };
    let Some(bundle) = role_receipt_bundle_from_local_tensors(node, &receipt) else {
        return Ok(None);
    };
    let validation_seed = node.chain.validation_seed(&assignment.receipt_id, &auditor);
    let canonical = ReferenceValidatorRole::new(auditor, validator_stake)
        .verify_receipt(
            &job,
            &bundle,
            &validation_seed,
            &node.chain.params().freivalds,
        )
        .map_err(|error| {
            format!(
                "validator role failed to audit receipt {}: {error}",
                hex(&assignment.receipt_id)
            )
        })?;
    let report = ValidatorAuditReport::new(
        audit_id,
        auditor,
        canonical.result,
        canonical.data_availability_passed,
        canonical.checks_root,
    );
    node.chain
        .apply_command(ChainCommand::SubmitValidatorAuditReport(report))
        .map_err(|error| {
            format!(
                "validator role failed to submit audit report {}: {error}",
                hex(&audit_id)
            )
        })?;
    Ok(Some(ValidatorRoleAuditReportSubmission {
        audit_reports_submitted: 1,
    }))
}

pub fn submit_validator_role_block_vote(
    node: &mut RpcNode,
    validator: Address,
) -> std::result::Result<Option<ValidatorRoleBlockVoteSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let Some(block) = node
        .chain
        .blocks()
        .iter()
        .rev()
        .find(|block| {
            let block_hash = block.hash();
            !node.chain.is_block_finalized(&block_hash)
                && !validator_has_block_vote(&node.chain, validator, block_hash)
                && node.chain.validate_block(block).is_ok()
        })
        .cloned()
    else {
        return Ok(None);
    };
    let vote = BlockVote::new(validator, validator_stake, &block);
    node.chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .map_err(|error| {
            format!(
                "validator role failed to submit block vote {}: {error}",
                hex(&block.hash())
            )
        })?;
    Ok(Some(ValidatorRoleBlockVoteSubmission {
        block_votes_submitted: 1,
    }))
}

fn validator_has_block_vote(chain: &Chain, validator: Address, block_hash: Hash) -> bool {
    chain
        .state()
        .block_votes()
        .get(&block_hash)
        .is_some_and(|votes| votes.iter().any(|vote| vote.validator == validator))
}

fn role_receipt_bundle_from_local_tensors(
    node: &RpcNode,
    receipt: &ReceiptState,
) -> Option<RoleReceiptBundle> {
    let job = node.chain.state().jobs().get(&receipt.job_id())?;
    match (job, receipt) {
        (JobState::TensorOp(_), ReceiptState::TensorOp(receipt)) => {
            let a = node
                .tensor_by_commitment_root(receipt.input_roots.first()?)?
                .clone();
            let b = node
                .tensor_by_commitment_root(receipt.input_roots.get(1)?)?
                .clone();
            let c = node
                .tensor_by_commitment_root(receipt.output_roots.first()?)?
                .clone();
            Some(RoleReceiptBundle {
                receipt: ReceiptState::TensorOp(receipt.clone()),
                artifacts: RoleReceiptArtifacts::TensorOp { a, b, c },
            })
        }
        (JobState::LinearTrainingStep(job), ReceiptState::LinearTrainingStep(receipt)) => {
            let weights_before = SyntheticLocalJobSource::linear_training_weights();
            if weights_before.commitment_root() != job.weight_root_before
                || receipt.weight_root_before != job.weight_root_before
            {
                return None;
            }
            let (x, target) = job.batch_tensors().ok()?;
            let y = node.tensor_by_commitment_root(&receipt.y_root)?.clone();
            let grad_w = node
                .tensor_by_commitment_root(&receipt.grad_w_root)?
                .clone();
            let weight_after = node
                .tensor_by_commitment_root(&receipt.weight_root_after)?
                .clone();
            let dy = y.sub(&target).ok()?;
            Some(RoleReceiptBundle {
                receipt: ReceiptState::LinearTrainingStep(receipt.clone()),
                artifacts: RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output: Box::new(LinearTrainingStepOutput {
                        x,
                        target,
                        y,
                        dy,
                        grad_w,
                        weight_after,
                        loss_commitment: receipt.loss_commitment,
                    }),
                },
            })
        }
        (JobState::GraphExecution(job), ReceiptState::GraphExecution(receipt)) => {
            let graph = graph_from_program_body(node, &job.graph_id)?;
            let mut inputs = BTreeMap::new();
            for (name, root) in &job.input_roots {
                inputs.insert(name.clone(), node.tensor_by_commitment_root(root)?.clone());
            }
            let mut outputs = BTreeMap::new();
            for (name, root) in &receipt.output_roots {
                outputs.insert(name.clone(), node.tensor_by_commitment_root(root)?.clone());
            }
            let const_blobs = graph_const_blobs_from_node(node, &graph)?;
            Some(RoleReceiptBundle {
                receipt: ReceiptState::GraphExecution(receipt.clone()),
                artifacts: RoleReceiptArtifacts::GraphExecution {
                    graph,
                    inputs,
                    const_blobs,
                    outputs,
                },
            })
        }
        _ => None,
    }
}

fn graph_const_blobs_from_node(
    node: &RpcNode,
    graph: &TensorGraph,
) -> Option<BTreeMap<String, Tensor>> {
    let mut const_blobs = BTreeMap::new();
    for (uri, _) in graph.const_blob_specs().ok()? {
        let root = parse_hash_hex(&uri).ok()?;
        const_blobs.insert(uri, node.tensor_by_commitment_root(&root)?.clone());
    }
    Some(const_blobs)
}

fn graph_from_program_body(node: &RpcNode, graph_id: &Hash) -> Option<TensorGraph> {
    let bytes = node.chain.state().program_body(graph_id)?;
    let graph = TensorGraph::from_canonical_json_bytes(bytes).ok()?;
    if graph.validate_for_consensus().ok()? != *graph_id {
        return None;
    }
    Some(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChainParams, RpcNode,
        app::miner_role::submit_miner_role_receipt,
        scheduler::SyntheticLocalJobSource,
        types::{address, hash_bytes},
        verify::FreivaldsParams,
    };

    #[test]
    fn role_runtime_submits_and_attests_graph_execution_from_local_artifacts() {
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
        let miner = address(b"app-graph-miner");
        let validator = address(b"app-graph-validator");
        let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"app-graph-role"]));
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake: chain.params().miner_min_stake,
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::RegisterValidator {
                address: validator,
                stake: chain.params().validator_min_stake,
            })
            .unwrap();
        chain.set_position_for_testing(2, 0);
        let mut source = SyntheticLocalJobSource::default();
        let job = source.next_graph_job(&chain);
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: job.graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
                job.clone(),
            )))
            .unwrap();
        let mut node = RpcNode::new(chain);
        for tensor in SyntheticLocalJobSource::graph_execution_inputs().into_values() {
            node.insert_tensor(tensor);
        }

        let submission = submit_miner_role_receipt(&mut node, miner, job.job_id)
            .unwrap()
            .expect("miner role should submit a graph receipt");
        for tensor in submission.served_tensors {
            node.insert_tensor(tensor);
        }
        let receipt_id = node
            .chain
            .state()
            .receipts()
            .keys()
            .copied()
            .next()
            .expect("graph receipt must be stored");
        let observation = validator_role_work_observation(&node, validator);
        assert!(observation.artifact_ready_receipts.contains(&receipt_id));

        let attestation = submit_validator_role_attestation(&mut node, validator, receipt_id)
            .unwrap()
            .expect("validator role should attest the graph receipt");

        assert_eq!(attestation.attestations_submitted, 1);
        assert!(matches!(
            node.chain.state().receipts().get(&receipt_id),
            Some(ReceiptState::GraphExecution(_))
        ));
        assert_eq!(
            node.chain
                .state()
                .attestations()
                .get(&receipt_id)
                .map(Vec::len),
            Some(1)
        );
    }
}
