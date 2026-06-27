use crate::chain::{Chain, HardwareClass, JobState, RewardClaimKey, RewardClaimLedger};
use crate::hash::hex;
use crate::jobs::PrimitiveType;
use crate::types::Address;
use tensor_vm_explorer::{
    ExplorerAccount, ExplorerBlock, ExplorerDetectionProbabilityEvidence,
    ExplorerDetectionProbabilityEvidenceSummary, ExplorerFraudPathEconomicCalibration,
    ExplorerFraudPathEconomicCalibrationSummary, ExplorerJob, ExplorerMiner, ExplorerOverview,
    ExplorerPendingReward, ExplorerRandomnessBindingEvidence, ExplorerReceipt, ExplorerSummary,
    ExplorerValidator, ExplorerValidatorAuditEconomicCalibration,
    ExplorerVerifierBandwidthEvidence, ExplorerVerifierBandwidthEvidenceSummary,
};

pub(super) fn explorer_summary(chain: &Chain) -> ExplorerSummary {
    ExplorerSummary {
        height: chain.state().height(),
        epoch: chain.state().epoch(),
        block_count: chain.blocks().len(),
        miner_count: chain.state().miners().len(),
        validator_count: chain.state().validators().len(),
        job_count: chain.state().jobs().len(),
        model_count: chain.state().model_states().len(),
        model_step_total: chain
            .state()
            .model_states()
            .values()
            .map(|model| model.step)
            .sum(),
        attestation_count: chain.state().attestations().values().map(Vec::len).sum(),
        receipt_count: chain.state().receipts().len(),
        settled_receipt_count: chain.state().settled_receipts().len(),
        committee_receipt_count: chain.committee_receipt_count(),
        settled_committee_receipt_count: chain.settled_committee_receipt_count(),
        escalated_committee_dispute_count: chain.escalated_committee_dispute_count(),
        data_unavailable_receipt_count: chain.state().data_unavailable_receipts().len(),
        data_unavailability_slash_count: chain.state().data_unavailability_slashes().len(),
        data_unavailability_slashed_amount_total: chain
            .state()
            .data_unavailability_slashes()
            .values()
            .map(|slash| slash.amount)
            .sum(),
        validator_audit_assignment_count: chain.state().validator_audit_assignments().len(),
        validator_audit_result_count: chain.state().validator_audit_results().len(),
        validator_audit_slash_count: chain.state().validator_audit_slashes().len(),
        validator_audit_slashed_amount_total: chain
            .state()
            .validator_audit_slashes()
            .values()
            .map(|slash| slash.amount)
            .sum(),
        finalized_block_count: chain.state().finalized_blocks().len(),
        treasury_balance: chain.state().rewards().treasury(),
        pending_receipt_reward_count: chain.state().pending_receipt_rewards().len(),
        pending_proposer_reward_count: chain.state().pending_proposer_rewards().len(),
        pending_challenge_reward_count: chain.state().pending_challenge_rewards().len(),
        pending_credit_reward_count: chain.state().pending_credit_rewards().len(),
        total_reward_balance: chain.state().rewards().total_balance(),
    }
}

pub(super) fn explorer_account(chain: &Chain, address: &Address) -> ExplorerAccount {
    let state = chain.state();
    let miner = state.miners().get(address);
    let validator = state.validators().get(address);
    let balance = state
        .accounts()
        .get(address)
        .map(|account| account.balance)
        .unwrap_or_default();
    ExplorerAccount {
        address: hex(address),
        is_miner: miner.is_some(),
        is_validator: validator.is_some(),
        balance,
        reward_balance: state.rewards().balance(address),
        stake: miner
            .map(|miner| miner.stake)
            .or_else(|| validator.map(|validator| validator.stake))
            .unwrap_or_default(),
        reputation: miner
            .map(|miner| miner.reputation)
            .or_else(|| validator.map(|validator| validator.reputation))
            .unwrap_or_default(),
        settled_tensor_work: miner
            .map(|miner| miner.settled_tensor_work)
            .unwrap_or_default(),
        pending_tensor_work: miner
            .map(|miner| miner.pending_tensor_work)
            .unwrap_or_default(),
    }
}

pub(super) fn explorer_blocks(chain: &Chain, limit: usize) -> Vec<ExplorerBlock> {
    chain
        .blocks()
        .iter()
        .rev()
        .take(limit)
        .map(|block| ExplorerBlock {
            height: block.height,
            epoch: block.epoch,
            hash: hex(&block.hash()),
            proposer: hex(&block.proposer),
            state_root: hex(&block.state_root),
            timestamp: block.timestamp,
        })
        .collect()
}

pub(super) fn explorer_miners(chain: &Chain) -> Vec<ExplorerMiner> {
    let state = chain.state();
    state
        .miners()
        .values()
        .map(|miner| ExplorerMiner {
            address: hex(&miner.address),
            operator_id: hex(&miner.operator_id),
            stake: miner.stake,
            reputation: miner.reputation,
            settled_tensor_work: miner.settled_tensor_work,
            pending_tensor_work: miner.pending_tensor_work,
            hardware_class: hardware_class_label(miner.hardware_class).to_owned(),
            gpu_utilization_bps: miner.gpu_utilization_bps,
            reward_balance: state.rewards().balance(&miner.address),
        })
        .collect()
}

pub(super) fn explorer_validators(chain: &Chain) -> Vec<ExplorerValidator> {
    let state = chain.state();
    state
        .validators()
        .values()
        .map(|validator| ExplorerValidator {
            address: hex(&validator.address),
            stake: validator.stake,
            reputation: validator.reputation,
            valid_attestations: validator.valid_attestations,
            missed_assignments: validator.missed_assignments,
            reward_balance: state.rewards().balance(&validator.address),
        })
        .collect()
}

pub(super) fn explorer_receipts(chain: &Chain, limit: usize) -> Vec<ExplorerReceipt> {
    let state = chain.state();
    state
        .receipts()
        .iter()
        .rev()
        .take(limit)
        .map(|(receipt_id, receipt)| {
            let validator_attestations: Vec<_> = chain
                .state()
                .attestations()
                .get(receipt_id)
                .into_iter()
                .flat_map(|attestations| attestations.iter())
                .map(|attestation| hex(&attestation.validator))
                .collect();
            ExplorerReceipt {
                receipt_id: hex(receipt_id),
                job_id: hex(&receipt.job_id()),
                primitive_type: primitive_label(receipt.primitive_type()).to_owned(),
                miner: hex(&receipt.miner()),
                tensor_work_units: receipt.tensor_work_units(),
                attestation_count: validator_attestations.len(),
                validator_attestations,
                settled: state.settled_receipts().contains(receipt_id),
            }
        })
        .collect()
}

pub(super) fn explorer_pending_rewards(chain: &Chain, limit: usize) -> Vec<ExplorerPendingReward> {
    let claims = chain.state().pending_reward_claims();
    sample_pending_reward_claims(&claims, limit)
        .into_iter()
        .map(|claim| ExplorerPendingReward {
            ledger: claim.ledger.label().to_owned(),
            claim_id: reward_claim_key_label(claim.claim_id),
            subject_id: reward_claim_key_label(claim.subject_id),
            beneficiary: hex(&claim.beneficiary),
            amount: claim.amount,
            claimable_at_height: claim.claimable_at_height,
            awaiting_inclusion: claim.awaiting_inclusion,
            awaiting_validator_vrf_reveal: claim.awaiting_validator_vrf_reveal,
            voided_by_challenge: claim.voided_by_challenge,
        })
        .collect()
}

fn sample_pending_reward_claims(
    claims: &[crate::chain::RewardClaimView],
    limit: usize,
) -> Vec<crate::chain::RewardClaimView> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected: Vec<_> = claims.iter().take(limit).cloned().collect();
    for ledger in [
        RewardClaimLedger::Proposer,
        RewardClaimLedger::ReceiptMiner,
        RewardClaimLedger::ReceiptValidator,
        RewardClaimLedger::Challenge,
        RewardClaimLedger::Credit,
    ] {
        if selected.iter().any(|claim| claim.ledger == ledger) {
            continue;
        }
        let Some(claim) = claims.iter().find(|claim| claim.ledger == ledger) else {
            continue;
        };
        if selected.len() < limit {
            selected.push(claim.clone());
            continue;
        }
        if let Some(index) = replacement_pending_reward_sample_index(&selected) {
            selected[index] = claim.clone();
        }
    }
    selected.sort_by(|left, right| {
        left.claimable_at_height
            .unwrap_or(u64::MAX)
            .cmp(&right.claimable_at_height.unwrap_or(u64::MAX))
            .then_with(|| left.ledger.cmp(&right.ledger))
            .then_with(|| left.claim_id.cmp(&right.claim_id))
    });
    selected
}

fn replacement_pending_reward_sample_index(
    claims: &[crate::chain::RewardClaimView],
) -> Option<usize> {
    claims
        .iter()
        .enumerate()
        .rev()
        .find(|(_, claim)| {
            claims
                .iter()
                .filter(|other| other.ledger == claim.ledger)
                .count()
                > 1
        })
        .map(|(index, _)| index)
        .or_else(|| claims.len().checked_sub(1))
}

pub(super) fn explorer_validator_audit_economic_calibration(
    chain: &Chain,
) -> ExplorerValidatorAuditEconomicCalibration {
    let calibration = chain
        .state()
        .validator_audit_economic_calibration(chain.params());
    ExplorerValidatorAuditEconomicCalibration {
        detection_numerator: calibration.detection_numerator,
        detection_denominator: calibration.detection_denominator,
        detection_probability_bps: calibration.detection_probability_bps,
        slashable_bond: calibration.slashable_bond,
        reward_from_fraud: calibration.reward_from_fraud,
        at_risk_validator_reward_claim_count: calibration.at_risk_validator_reward_claim_count,
        required_slashable_bond: calibration.required_slashable_bond,
        invariant_holds: calibration.invariant_holds,
    }
}

pub(super) fn explorer_fraud_path_economic_calibration(
    chain: &Chain,
) -> ExplorerFraudPathEconomicCalibrationSummary {
    let calibration = chain
        .state()
        .fraud_path_economic_calibration(chain.params());
    ExplorerFraudPathEconomicCalibrationSummary {
        path_count: calibration.path_count,
        all_invariants_hold: calibration.all_invariants_hold,
        max_required_slashable_bond: calibration.max_required_slashable_bond,
        worst_path: calibration.worst_path.to_owned(),
        paths: calibration
            .paths
            .into_iter()
            .map(|path| ExplorerFraudPathEconomicCalibration {
                path: path.path.to_owned(),
                detection_numerator: path.detection_numerator,
                detection_denominator: path.detection_denominator,
                detection_probability_bps: path.detection_probability_bps,
                slashable_bond: path.slashable_bond,
                reward_from_fraud: path.reward_from_fraud,
                at_risk_reward_claim_count: path.at_risk_reward_claim_count,
                required_slashable_bond: path.required_slashable_bond,
                invariant_holds: path.invariant_holds,
            })
            .collect(),
    }
}

pub(super) fn explorer_detection_probability_evidence(
    chain: &Chain,
) -> ExplorerDetectionProbabilityEvidenceSummary {
    let evidence = chain.state().detection_probability_evidence(chain.params());
    ExplorerDetectionProbabilityEvidenceSummary {
        mechanism_count: evidence.mechanism_count,
        minimum_detection_probability_bps: evidence.minimum_detection_probability_bps,
        maximum_false_accept_probability_bps: evidence.maximum_false_accept_probability_bps,
        live_subject_count: evidence.live_subject_count,
        mechanisms: evidence
            .mechanisms
            .into_iter()
            .map(|mechanism| ExplorerDetectionProbabilityEvidence {
                mechanism: mechanism.mechanism.to_owned(),
                source: mechanism.source.to_owned(),
                sample_numerator: mechanism.sample_numerator,
                sample_denominator: mechanism.sample_denominator,
                detection_probability_bps: mechanism.detection_probability_bps,
                false_accept_probability_bps: mechanism.false_accept_probability_bps,
                live_subject_count: mechanism.live_subject_count,
            })
            .collect(),
    }
}

pub(super) fn explorer_verifier_bandwidth_evidence(
    chain: &Chain,
) -> ExplorerVerifierBandwidthEvidenceSummary {
    let evidence = chain.state().verifier_bandwidth_evidence(chain.params());
    ExplorerVerifierBandwidthEvidenceSummary {
        record_count: evidence.record_count,
        live_job_count: evidence.live_job_count,
        live_receipt_count: evidence.live_receipt_count,
        estimated_total_verification_bytes: evidence.estimated_total_verification_bytes,
        estimated_bandwidth_per_validator_bytes: evidence.estimated_bandwidth_per_validator_bytes,
        max_verification_to_execution_bps: evidence.max_verification_to_execution_bps,
        has_live_bounded_evidence: evidence.has_live_bounded_evidence,
        records: evidence
            .records
            .into_iter()
            .map(|record| ExplorerVerifierBandwidthEvidence {
                primitive: record.primitive.to_owned(),
                source: record.source.to_owned(),
                live_job_count: record.live_job_count,
                live_receipt_count: record.live_receipt_count,
                max_execution_ops: record.max_execution_ops,
                max_verification_ops: record.max_verification_ops,
                max_verification_bytes_per_receipt: record.max_verification_bytes_per_receipt,
                estimated_total_verification_bytes: record.estimated_total_verification_bytes,
                max_verification_to_execution_bps: record.max_verification_to_execution_bps,
            })
            .collect(),
    }
}

pub(super) fn explorer_randomness_binding_evidence(
    chain: &Chain,
) -> ExplorerRandomnessBindingEvidence {
    let evidence = chain
        .state()
        .randomness_binding_evidence_for_params(chain.params());
    ExplorerRandomnessBindingEvidence {
        beacon_source: evidence.beacon_source.to_owned(),
        drand_round_mapping: evidence.drand_round_mapping.to_owned(),
        vrf_construction: evidence.vrf_construction.to_owned(),
        assignment_seed_domain: evidence.assignment_seed_domain.to_owned(),
        validation_seed_commitment_domain: evidence.validation_seed_commitment_domain.to_owned(),
        validation_seed_reveal_domain: evidence.validation_seed_reveal_domain.to_owned(),
        commit_reveal_ordering: evidence.commit_reveal_ordering.to_owned(),
        current_block_hash_randomness_allowed: evidence.current_block_hash_randomness_allowed,
        receipt_anchor_count: evidence.receipt_anchor_count,
        finalized_beacon_anchor_count: evidence.finalized_beacon_anchor_count,
        finalized_beacon_round_mapping_count: evidence.finalized_beacon_round_mapping_count,
        validator_vrf_seed_count: evidence.validator_vrf_seed_count,
        validator_vrf_registered_key_count: evidence.validator_vrf_registered_key_count,
        validator_vrf_reveal_count: evidence.validator_vrf_reveal_count,
        validator_vrf_production_reveal_count: evidence.validator_vrf_production_reveal_count,
        validator_vrf_legacy_reveal_count: evidence.validator_vrf_legacy_reveal_count,
        receipt_bound_anchor_count: evidence.receipt_bound_anchor_count,
        consistent_anchor_count: evidence.consistent_anchor_count,
        current_block_hash_anchor_count: evidence.current_block_hash_anchor_count,
        external_beacon_record_count: evidence.external_beacon_record_count,
        latest_external_beacon_round: evidence.latest_external_beacon_round,
        public_drand_anchor_epoch: evidence.public_drand_anchor_epoch,
        public_drand_anchor_round: evidence.public_drand_anchor_round,
        public_drand_rounds_per_epoch: evidence.public_drand_rounds_per_epoch,
        public_drand_epoch_start_round: evidence.public_drand_epoch_start_round,
        public_drand_epoch_end_round: evidence.public_drand_epoch_end_round,
        all_receipt_anchors_consistent: evidence.all_receipt_anchors_consistent,
    }
}

fn reward_claim_key_label(key: RewardClaimKey) -> String {
    match key {
        RewardClaimKey::BlockHeight(height) => height.to_string(),
        RewardClaimKey::Hash(hash) => hex(&hash),
    }
}

pub(super) fn explorer_jobs(chain: &Chain, limit: usize) -> Vec<ExplorerJob> {
    chain
        .state()
        .jobs()
        .values()
        .rev()
        .take(limit)
        .map(|job| match job {
            JobState::TensorOp(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "tensor_op".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("matmul {}x{}x{}", job.m, job.k, job.n),
            },
            JobState::LinearTrainingStep(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "linear_training_step".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!("model step {} input {:?}", job.step, job.input_shape),
            },
            JobState::GraphExecution(job) => ExplorerJob {
                job_id: hex(&job.job_id),
                primitive_type: "graph_execution".to_owned(),
                deadline_block: job.deadline_block,
                detail: format!(
                    "graph {} inputs {} params {}",
                    hex(&job.graph_id),
                    job.input_roots.len(),
                    job.field_params.len()
                ),
            },
        })
        .collect()
}

pub(super) fn explorer_overview(
    chain: &Chain,
    block_limit: usize,
    receipt_limit: usize,
    job_limit: usize,
) -> ExplorerOverview {
    ExplorerOverview {
        summary: explorer_summary(chain),
        blocks: explorer_blocks(chain, block_limit),
        miners: explorer_miners(chain),
        validators: explorer_validators(chain),
        receipts: explorer_receipts(chain, receipt_limit),
        pending_rewards: explorer_pending_rewards(chain, receipt_limit),
        validator_audit_economic_calibration: explorer_validator_audit_economic_calibration(chain),
        fraud_path_economic_calibration: explorer_fraud_path_economic_calibration(chain),
        detection_probability_evidence: explorer_detection_probability_evidence(chain),
        verifier_bandwidth_evidence: explorer_verifier_bandwidth_evidence(chain),
        randomness_binding_evidence: explorer_randomness_binding_evidence(chain),
        jobs: explorer_jobs(chain, job_limit),
    }
}

pub(super) fn primitive_label(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::TensorOp => "tensor_op",
        PrimitiveType::LinearTrainingStep => "linear_training_step",
        PrimitiveType::GraphExecution => "graph_execution",
    }
}

pub(super) fn hardware_class_label(hardware_class: HardwareClass) -> &'static str {
    match hardware_class {
        HardwareClass::Cpu => "cpu",
        HardwareClass::ConsumerGpu => "consumer_gpu",
        HardwareClass::DatacenterGpu => "datacenter_gpu",
        HardwareClass::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{RewardClaimKey, RewardClaimView};

    fn reward_claim(
        ledger: RewardClaimLedger,
        claim_height: u64,
        claim_id: u64,
    ) -> RewardClaimView {
        RewardClaimView {
            ledger,
            claim_id: RewardClaimKey::BlockHeight(claim_id),
            subject_id: RewardClaimKey::BlockHeight(claim_id),
            related_id: None,
            beneficiary: [claim_id as u8; 32],
            amount: 1,
            claimable_at_height: Some(claim_height),
            awaiting_inclusion: false,
            awaiting_validator_vrf_reveal: false,
            voided_by_challenge: false,
        }
    }

    #[test]
    fn pending_reward_sample_keeps_non_receipt_ledgers_visible() {
        let mut claims = Vec::new();
        for id in 0..10 {
            claims.push(reward_claim(RewardClaimLedger::ReceiptMiner, 10 + id, id));
        }
        claims.push(reward_claim(RewardClaimLedger::Proposer, 300, 100));
        claims.push(reward_claim(RewardClaimLedger::Challenge, 301, 101));

        let sample = sample_pending_reward_claims(&claims, 10);

        assert_eq!(sample.len(), 10);
        assert!(
            sample
                .iter()
                .any(|claim| claim.ledger == RewardClaimLedger::Proposer)
        );
        assert!(
            sample
                .iter()
                .any(|claim| claim.ledger == RewardClaimLedger::Challenge)
        );
    }
}
