use std::collections::BTreeSet;

use crate::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobScheduler, JobState, ReceiptState, RpcNode,
    SyntheticLocalJobSource,
    chain::ValidatorAuditReport,
    hash::hex,
    jobs::LinearTrainingStepOutput,
    roles::{ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle},
    types::{Address, Hash},
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
        if assignment.validator == auditor {
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidatorRoleBlockProposal {
    pub blocks_proposed: usize,
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
    node.chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer: validator,
            timestamp,
        })
        .map_err(|error| format!("validator proposer failed to produce block: {error}"))?;
    Ok(Some(ValidatorRoleBlockProposal { blocks_proposed: 1 }))
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
        _ => None,
    }
}
