use crate::chain::{Chain, HardwareClass, JobState, ReceiptState};
use crate::field::Elem;
use crate::jobs::PrimitiveType;
use crate::study::matmul_verification_cost_study;
use crate::types::Hash;
use crate::verify::VerificationResult;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TelemetrySnapshot {
    pub block_finality_rate: f64,
    pub average_block_time: f64,
    pub average_receipt_age_blocks: f64,
    pub receipt_count: usize,
    pub settled_receipt_count: usize,
    pub data_availability_rate: f64,
    pub invalid_receipts_submitted: usize,
    pub invalid_receipts_accepted: u64,
    pub invalid_receipt_detection_rate: f64,
    pub validator_disagreement_rate: f64,
    pub data_withholding_incidents: usize,
    pub total_tensor_work: u64,
    pub tensor_work_per_epoch: f64,
    pub max_miner_work_share: f64,
    pub state_entries_per_epoch: f64,
    pub estimated_bandwidth_per_validator_bytes: f64,
    pub estimated_gpu_utilization: f64,
    pub estimated_verification_to_execution_ratio: f64,
    pub redundant_compute_overhead: f64,
    pub miner_reward_per_twu: f64,
    pub validator_reward_per_attestation: f64,
    pub hardware_class_participation: usize,
    pub estimated_cost_to_attack_one_epoch: u64,
}

impl TelemetrySnapshot {
    pub fn from_chain(chain: &Chain) -> Self {
        let block_count = chain.blocks().len();
        let average_block_time = if block_count <= 1 {
            0.0
        } else {
            let first = chain
                .blocks()
                .first()
                .map(|block| block.timestamp)
                .unwrap_or(0);
            let last = chain
                .blocks()
                .last()
                .map(|block| block.timestamp)
                .unwrap_or(first);
            (last.saturating_sub(first) as f64) / (block_count.saturating_sub(1) as f64)
        };
        let receipt_count = chain.state().receipts().len();
        let settled_receipt_count = chain.state().settled_receipts().len();
        let receipt_age_sum: u64 = chain
            .state()
            .receipts()
            .values()
            .map(|receipt| {
                chain
                    .state()
                    .height()
                    .saturating_sub(receipt_submitted_at_block(receipt))
            })
            .sum();
        let average_receipt_age_blocks = ratio_u64(receipt_age_sum, receipt_count as u64);
        let total_attestations = chain
            .state()
            .attestations()
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let unavailable = chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .filter(|att| !att.data_availability_passed)
            .count();
        let invalid = chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .filter(|att| !matches!(att.result, crate::verify::VerificationResult::Valid))
            .count();
        let invalid_receipt_ids: BTreeSet<Hash> = chain
            .state()
            .attestations()
            .iter()
            .filter(|(_, items)| {
                items
                    .iter()
                    .any(|att| matches!(att.result, VerificationResult::Invalid))
            })
            .map(|(receipt_id, _)| *receipt_id)
            .collect();
        let invalid_receipts_submitted = invalid_receipt_ids.len();
        let invalid_receipts_accepted = invalid_receipt_ids
            .iter()
            .filter(|receipt_id| chain.state().settled_receipts().contains(*receipt_id))
            .count() as u64;
        let data_availability_rate = ratio(
            total_attestations.saturating_sub(unavailable),
            total_attestations,
        );
        let validator_disagreement_rate = ratio(invalid, total_attestations);
        let valid_attestations = chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .filter(|att| {
                matches!(att.result, VerificationResult::Valid) && att.data_availability_passed
            })
            .count();
        let total_tensor_work: u64 = chain
            .state()
            .miners()
            .values()
            .map(|miner| miner.settled_tensor_work)
            .sum();
        let max_miner_work = chain
            .state()
            .miners()
            .values()
            .map(|miner| miner.settled_tensor_work)
            .max()
            .unwrap_or(0);
        let epochs_seen = chain.state().epoch().saturating_add(1);
        let state_entries = chain
            .state()
            .accounts()
            .len()
            .saturating_add(chain.state().miners().len())
            .saturating_add(chain.state().validators().len())
            .saturating_add(chain.state().jobs().len())
            .saturating_add(chain.state().receipts().len())
            .saturating_add(chain.state().attestations().len())
            .saturating_add(chain.state().block_votes().len())
            .saturating_add(chain.state().finalized_blocks().len())
            .saturating_add(chain.state().model_states().len());
        let miner_rewards: u64 = chain
            .state()
            .miners()
            .keys()
            .map(|address| chain.state().rewards().balance(address))
            .sum();
        let validator_rewards: u64 = chain
            .state()
            .validators()
            .keys()
            .map(|address| chain.state().rewards().balance(address))
            .sum();

        Self {
            block_finality_rate: ratio(chain.state().finalized_blocks().len(), block_count),
            average_block_time,
            average_receipt_age_blocks,
            receipt_count,
            settled_receipt_count,
            data_availability_rate,
            invalid_receipts_submitted,
            invalid_receipts_accepted,
            invalid_receipt_detection_rate: ratio_u64(
                invalid_receipts_submitted as u64 - invalid_receipts_accepted,
                invalid_receipts_submitted as u64,
            ),
            validator_disagreement_rate,
            data_withholding_incidents: chain.state().data_unavailable_receipts().len(),
            total_tensor_work,
            tensor_work_per_epoch: ratio_u64(total_tensor_work, epochs_seen),
            max_miner_work_share: ratio(max_miner_work as usize, total_tensor_work as usize),
            state_entries_per_epoch: ratio(state_entries, epochs_seen as usize),
            estimated_bandwidth_per_validator_bytes: estimated_bandwidth_per_validator(chain),
            estimated_gpu_utilization: estimated_gpu_utilization(chain),
            estimated_verification_to_execution_ratio: average_verification_cost_ratio(chain),
            redundant_compute_overhead: redundant_compute_overhead(chain),
            miner_reward_per_twu: ratio_u64(miner_rewards, total_tensor_work),
            validator_reward_per_attestation: ratio_u64(
                validator_rewards,
                valid_attestations as u64,
            ),
            hardware_class_participation: hardware_class_participation(chain),
            estimated_cost_to_attack_one_epoch: estimated_cost_to_attack_one_epoch(chain),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("telemetry snapshot should serialize to JSON")
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn ratio_u64(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn receipt_submitted_at_block(receipt: &ReceiptState) -> u64 {
    receipt.submitted_at_block()
}

fn average_verification_cost_ratio(chain: &Chain) -> f64 {
    let mut total_ratio = 0.0;
    let mut counted = 0_usize;
    for receipt in chain.state().receipts().values() {
        let ReceiptState::TensorOp(receipt) = receipt else {
            continue;
        };
        let Some(crate::chain::JobState::TensorOp(job)) = chain.state().jobs().get(&receipt.job_id)
        else {
            continue;
        };
        total_ratio += matmul_verification_cost_study(
            job.m,
            job.k,
            job.n,
            chain.params().freivalds.full_rounds,
        )
        .verification_to_execution_ratio;
        counted += 1;
    }
    if counted == 0 {
        0.0
    } else {
        total_ratio / counted as f64
    }
}

fn estimated_bandwidth_per_validator(chain: &Chain) -> f64 {
    let total_bytes: u64 = chain
        .state()
        .receipts()
        .values()
        .map(|receipt| estimate_receipt_verification_bytes(chain, receipt))
        .sum::<u64>()
        .saturating_mul(chain.params().freivalds.validators_per_job as u64);
    ratio_u64(total_bytes, chain.state().validators().len() as u64)
}

fn estimate_receipt_verification_bytes(chain: &Chain, receipt: &ReceiptState) -> u64 {
    let elem_bytes = std::mem::size_of::<Elem>() as u64;
    match receipt {
        ReceiptState::TensorOp(receipt) => {
            let Some(JobState::TensorOp(job)) = chain.state().jobs().get(&receipt.job_id) else {
                return 0;
            };
            (job.m as u64)
                .saturating_mul(job.k as u64)
                .saturating_add((job.k as u64).saturating_mul(job.n as u64))
                .saturating_add((job.m as u64).saturating_mul(job.n as u64))
                .saturating_mul(elem_bytes)
        }
        ReceiptState::LinearTrainingStep(receipt) => {
            let Some(JobState::LinearTrainingStep(job)) = chain.state().jobs().get(&receipt.job_id)
            else {
                return 0;
            };
            let inputs = tensor_elements(&job.input_shape);
            let weights = tensor_elements(&job.weight_shape);
            let targets = tensor_elements(&job.target_shape);
            inputs
                .saturating_add(targets)
                .saturating_add(weights.saturating_mul(3))
                .saturating_add(targets.saturating_mul(2))
                .saturating_mul(elem_bytes)
        }
        ReceiptState::GraphExecution(receipt) => {
            receipt.tensor_work_units.saturating_mul(elem_bytes)
        }
    }
}

fn tensor_elements(shape: &[usize]) -> u64 {
    shape
        .iter()
        .fold(1_u64, |acc, value| acc.saturating_mul(*value as u64))
}

fn estimated_gpu_utilization(chain: &Chain) -> f64 {
    let mut weighted_utilization = 0_u64;
    let mut total_gpu_weight = 0_u64;
    for miner in chain.state().miners().values() {
        if !miner.hardware_class.is_gpu() {
            continue;
        }
        let weight = miner.settled_tensor_work.max(miner.stake).max(1);
        weighted_utilization =
            weighted_utilization.saturating_add(miner.gpu_utilization_bps.saturating_mul(weight));
        total_gpu_weight = total_gpu_weight.saturating_add(weight);
    }
    ratio_u64(
        weighted_utilization,
        total_gpu_weight.saturating_mul(10_000),
    )
}

fn hardware_class_participation(chain: &Chain) -> usize {
    let classes: BTreeSet<HardwareClass> = chain
        .state()
        .miners()
        .values()
        .map(|miner| miner.hardware_class)
        .collect();
    classes.len()
}

fn redundant_compute_overhead(chain: &Chain) -> f64 {
    let mut groups = BTreeSet::new();
    for receipt in chain.state().receipts().values() {
        groups.insert(receipt_agreement_key(receipt));
    }
    ratio(chain.state().receipts().len(), groups.len())
}

fn receipt_agreement_key(receipt: &ReceiptState) -> Vec<u8> {
    let mut encoded = Vec::new();
    match receipt {
        ReceiptState::TensorOp(receipt) => {
            encoded.push(PrimitiveType::TensorOp as u8);
            encoded.extend_from_slice(&receipt.job_id);
            encoded.extend_from_slice(&receipt.program_hash);
            encode_hashes(&mut encoded, &receipt.input_roots);
            encode_hashes(&mut encoded, &receipt.output_roots);
            encoded.extend_from_slice(&receipt.trace_root);
        }
        ReceiptState::LinearTrainingStep(receipt) => {
            encoded.push(PrimitiveType::LinearTrainingStep as u8);
            encoded.extend_from_slice(&receipt.job_id);
            encoded.extend_from_slice(&receipt.model_id);
            encoded.extend_from_slice(&receipt.step.to_le_bytes());
            encoded.extend_from_slice(&receipt.weight_root_before);
            encoded.extend_from_slice(&receipt.batch_root);
            encoded.extend_from_slice(&receipt.y_root);
            encoded.extend_from_slice(&receipt.loss_commitment);
            encoded.extend_from_slice(&receipt.grad_w_root);
            encoded.extend_from_slice(&receipt.weight_root_after);
            encoded.extend_from_slice(&receipt.trace_root);
        }
        ReceiptState::GraphExecution(receipt) => {
            encoded.push(PrimitiveType::GraphExecution as u8);
            encoded.extend_from_slice(&receipt.job_id);
            encoded.extend_from_slice(&receipt.graph_id);
            encode_named_hashes(&mut encoded, &receipt.input_roots);
            encode_named_hashes(&mut encoded, &receipt.output_roots);
            encoded.extend_from_slice(&receipt.trace_root);
        }
    }
    encoded
}

fn encode_hashes(out: &mut Vec<u8>, hashes: &[Hash]) {
    out.extend_from_slice(&(hashes.len() as u64).to_le_bytes());
    for hash in hashes {
        out.extend_from_slice(hash);
    }
}

fn encode_named_hashes(out: &mut Vec<u8>, hashes: &BTreeMap<String, Hash>) {
    out.extend_from_slice(&(hashes.len() as u64).to_le_bytes());
    for (name, hash) in hashes {
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(hash);
    }
}

fn estimated_cost_to_attack_one_epoch(chain: &Chain) -> u64 {
    let total_stake: u64 = chain
        .state()
        .validators()
        .values()
        .map(|validator| validator.stake)
        .sum();
    if total_stake == 0 {
        return 0;
    }
    let numerator = chain.params().finality_stake_numerator;
    let denominator = chain.params().finality_stake_denominator.max(1);
    total_stake.saturating_mul(numerator).div_ceil(denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainParams, JobState};
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, PrimitiveType,
        TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{AttestationStatement, ValidatorAttestation};

    fn snapshot_json(snapshot: &TelemetrySnapshot) -> serde_json::Value {
        serde_json::from_str(&snapshot.to_json()).expect("telemetry snapshot JSON must parse")
    }

    #[test]
    fn telemetry_reports_block_timing_and_concentration() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::with_params(
            ChainParams {
                pow_timeout_blocks: 1,
                ..ChainParams::default()
            },
            beacon,
        );
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        chain.produce_block(validator, 10).unwrap();
        chain.produce_block(validator, 16).unwrap();
        chain
            .set_miner_settled_tensor_work_for_testing(miner, 500)
            .unwrap();
        let snapshot = TelemetrySnapshot::from_chain(&chain);
        assert_eq!(snapshot.average_block_time, 6.0);
        assert_eq!(snapshot.max_miner_work_share, 1.0);
        assert_eq!(snapshot.tensor_work_per_epoch, 500.0);
        assert!(snapshot.state_entries_per_epoch >= 2.0);
        assert_eq!(snapshot.hardware_class_participation, 1);
        let json = snapshot_json(&snapshot);
        assert_eq!(json["average_block_time"].as_f64(), Some(6.0));
        assert_eq!(json["total_tensor_work"].as_u64(), Some(500));
        assert_eq!(
            json["estimated_cost_to_attack_one_epoch"].as_u64(),
            Some(snapshot.estimated_cost_to_attack_one_epoch)
        );
    }

    #[test]
    fn telemetry_reports_receipt_age_and_verification_cost_ratio() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::with_params(
            ChainParams {
                pow_timeout_blocks: 1,
                ..ChainParams::default()
            },
            beacon,
        );
        let miner = address(b"miner");
        let validator = address(b"receipt-age-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = crate::jobs::MatmulJob::synthetic(0, 0, 16, 16, 16, &beacon, 10);
        let (receipt, _a, _b, _c) =
            crate::jobs::TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt).unwrap();
        chain.produce_block(validator, 10).unwrap();
        chain.produce_block(validator, 16).unwrap();

        let snapshot = TelemetrySnapshot::from_chain(&chain);
        assert_eq!(snapshot.receipt_count, 1);
        assert_eq!(snapshot.average_receipt_age_blocks, 1.0);
        assert!(snapshot.estimated_verification_to_execution_ratio > 0.0);
        assert!(snapshot.estimated_verification_to_execution_ratio < 1.0);
        assert!(snapshot.estimated_bandwidth_per_validator_bytes > 0.0);
        assert_eq!(snapshot.redundant_compute_overhead, 1.0);
    }

    #[test]
    fn telemetry_reports_linear_receipt_bandwidth_and_missing_job_edges() {
        let beacon = hash_bytes(b"test", &[b"linear-telemetry-beacon"]);
        let mut chain = Chain::with_params(
            ChainParams {
                pow_timeout_blocks: 1,
                ..ChainParams::default()
            },
            beacon,
        );
        let miner = address(b"linear-telemetry-miner");
        let validator = address(b"linear-telemetry-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"linear-telemetry-model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"linear-telemetry-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 3,
            deadline_block: 10,
        });
        let (linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&job, miner, &weights, 1, 5).unwrap();
        chain.submit_job(JobState::LinearTrainingStep(job.clone()));
        chain.submit_linear_receipt(linear_receipt).unwrap();

        let orphan_tensor_job = crate::jobs::MatmulJob::synthetic(0, 9, 2, 2, 2, &beacon, 10);
        let (orphan_tensor_receipt, _a, _b, _c) =
            TensorOpReceipt::from_job(&orphan_tensor_job, miner, 1, 5).unwrap();
        chain.insert_receipt_for_testing(ReceiptState::TensorOp(orphan_tensor_receipt));

        let orphan_linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"orphan-linear-model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"orphan-linear-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![2, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![2, 2],
            lr: 1,
            deadline_block: 10,
        });
        let (orphan_linear_receipt, _output) =
            LinearTrainingStepReceipt::from_job(&orphan_linear_job, miner, &weights, 1, 5).unwrap();
        chain.insert_receipt_for_testing(ReceiptState::LinearTrainingStep(orphan_linear_receipt));

        chain.produce_block(validator, 10).unwrap();
        chain.produce_block(validator, 16).unwrap();

        let snapshot = TelemetrySnapshot::from_chain(&chain);
        assert_eq!(snapshot.receipt_count, 3);
        assert!(snapshot.average_receipt_age_blocks > 0.0);
        assert!(snapshot.estimated_bandwidth_per_validator_bytes > 0.0);
        assert_eq!(snapshot.estimated_verification_to_execution_ratio, 0.0);
        assert_eq!(snapshot.redundant_compute_overhead, 1.0);
    }

    #[test]
    fn telemetry_reports_security_compute_and_economic_success_metrics() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"telemetry-miner");
        let validator = address(b"telemetry-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = crate::jobs::MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Invalid,
                    checks_root: hash_bytes(b"test", &[b"invalid-checks"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
        chain.credit_reward_for_testing(miner, 64);
        chain.credit_reward_for_testing(validator, 8);
        chain
            .set_miner_settled_tensor_work_for_testing(miner, 64)
            .unwrap();

        let snapshot = TelemetrySnapshot::from_chain(&chain);
        assert_eq!(snapshot.invalid_receipts_submitted, 1);
        assert_eq!(snapshot.invalid_receipts_accepted, 0);
        assert_eq!(snapshot.invalid_receipt_detection_rate, 1.0);
        assert_eq!(snapshot.validator_disagreement_rate, 1.0);
        assert_eq!(snapshot.miner_reward_per_twu, 1.0);
        assert_eq!(snapshot.validator_reward_per_attestation, 0.0);
        assert_eq!(snapshot.estimated_cost_to_attack_one_epoch, 6_667);
    }

    #[test]
    fn telemetry_reports_hardware_classes_and_gpu_utilization() {
        let beacon = hash_bytes(b"test", &[b"hardware-beacon"]);
        let mut chain = Chain::new(beacon);
        let cpu = address(b"hardware-cpu");
        let consumer_gpu = address(b"hardware-consumer-gpu");
        let datacenter_gpu = address(b"hardware-datacenter-gpu");
        chain
            .register_miner_with_profile(cpu, 100, HardwareClass::Cpu, 0)
            .unwrap();
        chain
            .register_miner_with_profile(consumer_gpu, 100, HardwareClass::ConsumerGpu, 5_000)
            .unwrap();
        chain
            .register_miner_with_profile(datacenter_gpu, 300, HardwareClass::DatacenterGpu, 9_000)
            .unwrap();
        chain
            .set_miner_settled_tensor_work_for_testing(consumer_gpu, 100)
            .unwrap();
        chain
            .set_miner_settled_tensor_work_for_testing(datacenter_gpu, 300)
            .unwrap();

        let snapshot = TelemetrySnapshot::from_chain(&chain);
        assert_eq!(snapshot.hardware_class_participation, 3);
        assert_eq!(snapshot.estimated_gpu_utilization, 0.8);

        assert_eq!(
            chain.register_miner_with_profile(address(b"bad-cpu"), 100, HardwareClass::Cpu, 1,),
            Err(crate::error::TvmError::InvalidReceipt(
                "non-gpu miner cannot report gpu utilization",
            ))
        );
        assert_eq!(
            chain.register_miner_with_profile(
                address(b"bad-gpu"),
                100,
                HardwareClass::ConsumerGpu,
                10_001,
            ),
            Err(crate::error::TvmError::InvalidReceipt(
                "gpu utilization exceeds 100%",
            ))
        );
    }
}
