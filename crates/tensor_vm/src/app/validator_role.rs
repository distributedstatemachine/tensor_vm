use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use crate::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobScheduler, JobState, ReceiptState, RpcNode,
    SyntheticLocalJobSource, Tensor, TensorGraph,
    chain::{ValidatorAuditReport, validator_vrf_ed25519_public_key_from_secret},
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatorRoleVrfKeyRegistration {
    pub vrf_public_key: Hash,
    pub registered_new_key: bool,
}

pub fn ensure_validator_role_vrf_key(
    node: &mut RpcNode,
    validator: Address,
    wallet_secret: Option<&str>,
) -> std::result::Result<Option<ValidatorRoleVrfKeyRegistration>, String> {
    let Some(secret) = wallet_secret else {
        return Ok(None);
    };
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let public_key = validator_vrf_ed25519_public_key_from_secret(secret);
    if validator_state.vrf_public_key == Some(public_key) {
        return Ok(Some(ValidatorRoleVrfKeyRegistration {
            vrf_public_key: public_key,
            registered_new_key: false,
        }));
    }
    node.chain
        .apply_command(ChainCommand::RegisterValidatorVrfKey {
            validator,
            vrf_public_key: public_key,
        })
        .map_err(|error| format!("validator role failed to register vrf key: {error}"))?;
    Ok(Some(ValidatorRoleVrfKeyRegistration {
        vrf_public_key: public_key,
        registered_new_key: true,
    }))
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
    let has_unincluded_settled_receipts = node.chain.state().settled_receipts().iter().any(|id| {
        !node.chain.state().included_receipts().contains(id)
            && !node.chain.state().data_unavailable_receipts().contains(id)
    });
    let proposer_reward = node
        .chain
        .params()
        .reward_allocation(10_000)
        .proposer_reward;
    let block_reward = if has_unincluded_settled_receipts {
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
    wallet_secret: Option<&str>,
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
    let reveal = if let Some(secret) = wallet_secret {
        ensure_validator_role_vrf_key(node, validator, wallet_secret)?;
        node.chain
            .validator_vrf_reveal_record_with_secret(receipt_id, validator, 0, secret)
            .map_err(|error| {
                format!(
                    "validator role failed to build vrf reveal {}: {error}",
                    hex(&receipt_id)
                )
            })?
    } else {
        node.chain
            .validator_vrf_reveal_record(receipt_id, validator, 0)
            .map_err(|error| {
                format!(
                    "validator role failed to build vrf reveal {}: {error}",
                    hex(&receipt_id)
                )
            })?
    };
    if !node
        .chain
        .state()
        .validator_vrf_reveals()
        .contains_key(&reveal.reveal_id)
    {
        node.chain
            .apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal))
            .map_err(|error| {
                format!(
                    "validator role failed to submit vrf reveal {}: {error}",
                    hex(&receipt_id)
                )
            })?;
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
    let Some((block, _canonical)) = node
        .chain
        .blocks()
        .iter()
        .map(|block| (block, true))
        .chain(
            node.chain
                .side_branch_blocks()
                .values()
                .map(|block| (block, false)),
        )
        .filter(|(block, _canonical)| {
            let block_hash = block.hash();
            !node.chain.is_block_finalized(&block_hash)
                && !validator_has_block_vote(&node.chain, validator, block_hash)
                && node.chain.validate_block(block).is_ok()
        })
        .min_by(validator_vote_candidate_order)
        .map(|(block, canonical)| (block.clone(), canonical))
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

fn validator_vote_candidate_order(
    left: &(&crate::chain::TensorBlock, bool),
    right: &(&crate::chain::TensorBlock, bool),
) -> Ordering {
    let (left_block, left_canonical) = left;
    let (right_block, right_canonical) = right;
    left_block
        .height
        .cmp(&right_block.height)
        .then_with(|| useful_pow_vote_order(left_block, right_block))
        .then_with(|| right_canonical.cmp(left_canonical))
        .then_with(|| left_block.hash().cmp(&right_block.hash()))
}

fn useful_pow_vote_order(
    left: &crate::chain::TensorBlock,
    right: &crate::chain::TensorBlock,
) -> Ordering {
    if left.production_kind.requires_pow() && right.production_kind.requires_pow() {
        left.pow_hash()
            .cmp(&right.pow_hash())
            .then_with(|| left.hash().cmp(&right.hash()))
    } else {
        Ordering::Equal
    }
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
            // Verification re-executes the graph from its inputs and compares the
            // recomputed roots against the receipt's claimed output roots, so the
            // miner's output *tensors* are never needed to attest. Tier-C committee
            // outputs are served only by the few miners that computed them, so
            // requiring them here would block the validator from ever attesting a
            // committee receipt. Only fetch outputs for strict Tier-A/B graphs
            // (where they are reliably available) and skip them for committee ones.
            let mut outputs = BTreeMap::new();
            if !graph.requires_committee_verification() {
                for (name, root) in &receipt.output_roots {
                    outputs.insert(name.clone(), node.tensor_by_commitment_root(root)?.clone());
                }
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
    // Admit Tier-C committee graphs so this node can reconstruct, serve, and
    // re-execute committee receipts on the live path.
    if graph.validate_for_committee().ok()? != *graph_id {
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
        jobs::{MatmulJob, TensorOpReceipt},
        scheduler::SyntheticLocalJobSource,
        types::{address, hash_bytes},
        verify::FreivaldsParams,
    };

    #[test]
    fn validator_role_votes_valid_side_branch_after_canonical_vote() {
        let seed = hash_bytes(b"test", &[b"validator-role-side-branch-vote"]);
        let validators = [
            address(b"side-vote-validator-0"),
            address(b"side-vote-validator-1"),
            address(b"side-vote-validator-2"),
            address(b"side-vote-validator-3"),
            address(b"side-vote-validator-4"),
        ];
        let voter = validators[0];
        let mut chain = Chain::new(seed);
        for validator in validators {
            chain
                .apply_command(ChainCommand::RegisterValidator {
                    address: validator,
                    stake: chain.params().validator_min_stake,
                })
                .unwrap();
        }
        let miner = address(b"side-vote-miner");
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake: chain.params().miner_min_stake,
            })
            .unwrap();
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &seed, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let receipt_id = receipt.receipt_id;
        chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt));
        chain.mark_receipt_settled_for_testing(receipt_id);

        let mut branch_a = chain.clone();
        let mut branch_b = chain.clone();
        let block_a = branch_a.produce_block(validators[1], 1_000).unwrap();
        let block_b = branch_b.produce_block(validators[2], 1_000).unwrap();
        let (preferred, nonpreferred) = if block_a
            .pow_hash()
            .cmp(&block_b.pow_hash())
            .then_with(|| block_a.hash().cmp(&block_b.hash()))
            .is_lt()
        {
            (block_a, block_b)
        } else {
            (block_b, block_a)
        };
        chain.admit_block(preferred).unwrap();
        chain.admit_block(nonpreferred).unwrap();
        let side_hash = chain
            .side_branch_blocks()
            .keys()
            .copied()
            .next()
            .expect("competing block should be stored as a side branch");
        let side_block = chain.side_branch_blocks().get(&side_hash).unwrap().clone();
        let canonical = chain.blocks().last().unwrap().clone();
        assert_ne!(side_hash, canonical.hash());

        let voter_stake = chain.state().validators().get(&voter).unwrap().stake;
        chain
            .apply_command(ChainCommand::SubmitBlockVote(BlockVote::new(
                voter,
                voter_stake,
                &canonical,
            )))
            .unwrap();
        assert!(!chain.is_block_finalized(&canonical.hash()));

        let mut node = RpcNode::new(chain);
        let submission = submit_validator_role_block_vote(&mut node, voter)
            .unwrap()
            .expect("validator role should vote for a valid side branch");

        assert_eq!(submission.block_votes_submitted, 1);
        assert!(validator_has_block_vote(&node.chain, voter, side_hash));
        assert!(!node.chain.is_block_finalized(&side_hash));
        assert_eq!(
            node.chain.side_branch_blocks().get(&side_hash),
            Some(&side_block)
        );
    }

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

        let attestation = submit_validator_role_attestation(&mut node, validator, receipt_id, None)
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
