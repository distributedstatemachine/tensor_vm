use crate::chain::{Chain, ReceiptState};
use crate::types::{Address, Hash};
use crate::verify::VerificationResult;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchEventKind {
    InvalidReceipt,
    DataUnavailable,
    ValidatorMisconduct,
    MissingAttestationQuorum,
    MissingRedundantAgreement,
    ConflictingLinearTransition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchEvent {
    pub kind: WatchEventKind,
    pub receipt_id: Hash,
    pub job_id: Option<Hash>,
    pub actor: Option<Address>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatcherConfig {
    pub flag_missing_quorum: bool,
    pub flag_missing_redundant_agreement: bool,
    pub flag_conflicting_linear_transitions: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            flag_missing_quorum: true,
            flag_missing_redundant_agreement: true,
            flag_conflicting_linear_transitions: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct WatchReport {
    pub events: Vec<WatchEvent>,
    pub invalid_receipts: usize,
    pub data_withholding_incidents: usize,
    pub validator_misconduct_events: usize,
    pub missing_quorum_receipts: usize,
    pub missing_redundant_agreement_receipts: usize,
    pub conflicting_linear_transitions: usize,
}

impl WatchReport {
    pub fn has_findings(&self) -> bool {
        !self.events.is_empty()
    }

    pub fn events_by_kind(&self, kind: WatchEventKind) -> Vec<&WatchEvent> {
        self.events
            .iter()
            .filter(|event| event.kind == kind)
            .collect()
    }

    fn record(&mut self, event: WatchEvent) {
        match event.kind {
            WatchEventKind::InvalidReceipt => self.invalid_receipts += 1,
            WatchEventKind::DataUnavailable => self.data_withholding_incidents += 1,
            WatchEventKind::ValidatorMisconduct => self.validator_misconduct_events += 1,
            WatchEventKind::MissingAttestationQuorum => self.missing_quorum_receipts += 1,
            WatchEventKind::MissingRedundantAgreement => {
                self.missing_redundant_agreement_receipts += 1;
            }
            WatchEventKind::ConflictingLinearTransition => {
                self.conflicting_linear_transitions += 1;
            }
        }
        self.events.push(event);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChainWatcher {
    pub config: WatcherConfig,
}

impl ChainWatcher {
    pub fn new(config: WatcherConfig) -> Self {
        Self { config }
    }

    pub fn scan(&self, chain: &Chain) -> WatchReport {
        let mut report = WatchReport::default();
        self.scan_attestations(chain, &mut report);
        self.scan_receipt_settlement_blockers(chain, &mut report);
        if self.config.flag_conflicting_linear_transitions {
            self.scan_linear_transition_conflicts(chain, &mut report);
        }
        report
    }

    fn scan_attestations(&self, chain: &Chain, report: &mut WatchReport) {
        let mut invalid_receipts = BTreeSet::new();
        let mut unavailable_receipts = BTreeSet::new();

        for (receipt_id, attestations) in chain.state().attestations() {
            let receipt = chain.state().receipts().get(receipt_id);
            for attestation in attestations {
                let validator = chain.state().validators().get(&attestation.validator);
                if validator.is_none() {
                    report.record(WatchEvent {
                        kind: WatchEventKind::ValidatorMisconduct,
                        receipt_id: *receipt_id,
                        job_id: Some(attestation.job_id),
                        actor: Some(attestation.validator),
                        message: "attestation from unknown validator".to_owned(),
                    });
                }
                if validator.is_some_and(|validator| validator.stake != attestation.stake) {
                    report.record(WatchEvent {
                        kind: WatchEventKind::ValidatorMisconduct,
                        receipt_id: *receipt_id,
                        job_id: Some(attestation.job_id),
                        actor: Some(attestation.validator),
                        message: "attestation stake does not match registered stake".to_owned(),
                    });
                }
                if !attestation.verify_signature() {
                    report.record(WatchEvent {
                        kind: WatchEventKind::ValidatorMisconduct,
                        receipt_id: *receipt_id,
                        job_id: Some(attestation.job_id),
                        actor: Some(attestation.validator),
                        message: "attestation signature does not verify".to_owned(),
                    });
                }

                match receipt {
                    Some(receipt)
                        if attestation.job_id != receipt.job_id()
                            || attestation.primitive_type != receipt.primitive_type() =>
                    {
                        report.record(WatchEvent {
                            kind: WatchEventKind::ValidatorMisconduct,
                            receipt_id: *receipt_id,
                            job_id: Some(attestation.job_id),
                            actor: Some(attestation.validator),
                            message: "attestation references different receipt metadata".to_owned(),
                        });
                    }
                    None => report.record(WatchEvent {
                        kind: WatchEventKind::ValidatorMisconduct,
                        receipt_id: *receipt_id,
                        job_id: Some(attestation.job_id),
                        actor: Some(attestation.validator),
                        message: "attestation references unknown receipt".to_owned(),
                    }),
                    Some(_) => {}
                }

                if matches!(attestation.result, VerificationResult::Invalid)
                    && invalid_receipts.insert(*receipt_id)
                {
                    report.record(WatchEvent {
                        kind: WatchEventKind::InvalidReceipt,
                        receipt_id: *receipt_id,
                        job_id: receipt.map(ReceiptState::job_id),
                        actor: receipt.map(ReceiptState::miner),
                        message: "validator reported invalid tensor work".to_owned(),
                    });
                }
                if (matches!(attestation.result, VerificationResult::Unavailable)
                    || !attestation.data_availability_passed)
                    && unavailable_receipts.insert(*receipt_id)
                {
                    report.record(WatchEvent {
                        kind: WatchEventKind::DataUnavailable,
                        receipt_id: *receipt_id,
                        job_id: receipt.map(ReceiptState::job_id),
                        actor: receipt.map(ReceiptState::miner),
                        message: "validator could not retrieve required tensor data".to_owned(),
                    });
                }
            }
        }
    }

    fn scan_receipt_settlement_blockers(&self, chain: &Chain, report: &mut WatchReport) {
        for (receipt_id, receipt) in chain.state().receipts() {
            if chain.state().settled_receipts().contains(receipt_id) {
                continue;
            }
            if self.config.flag_missing_quorum && !chain.has_attestation_quorum(receipt_id) {
                report.record(WatchEvent {
                    kind: WatchEventKind::MissingAttestationQuorum,
                    receipt_id: *receipt_id,
                    job_id: Some(receipt.job_id()),
                    actor: Some(receipt.miner()),
                    message: "receipt has not reached validator attestation quorum".to_owned(),
                });
                continue;
            }
            if self.config.flag_missing_redundant_agreement
                && !chain.has_redundant_agreement(receipt_id)
            {
                report.record(WatchEvent {
                    kind: WatchEventKind::MissingRedundantAgreement,
                    receipt_id: *receipt_id,
                    job_id: Some(receipt.job_id()),
                    actor: Some(receipt.miner()),
                    message: "receipt has quorum but not enough matching independent miner roots"
                        .to_owned(),
                });
            }
        }
    }

    fn scan_linear_transition_conflicts(&self, chain: &Chain, report: &mut WatchReport) {
        let mut reported = BTreeSet::new();
        for (left_id, left) in chain.state().receipts() {
            let ReceiptState::LinearTrainingStep(left) = left else {
                continue;
            };
            if !chain.has_attestation_quorum(left_id) {
                continue;
            }
            for (right_id, right) in chain.state().receipts() {
                if left_id >= right_id {
                    continue;
                }
                let ReceiptState::LinearTrainingStep(right) = right else {
                    continue;
                };
                if !chain.has_attestation_quorum(right_id) {
                    continue;
                }
                if left.model_id == right.model_id
                    && left.step == right.step
                    && left.weight_root_before == right.weight_root_before
                    && left.weight_root_after != right.weight_root_after
                    && reported.insert((*left_id, *right_id))
                {
                    report.record(WatchEvent {
                        kind: WatchEventKind::ConflictingLinearTransition,
                        receipt_id: *left_id,
                        job_id: Some(left.job_id),
                        actor: Some(left.miner),
                        message: "multiple quorum-backed receipts claim different model roots"
                            .to_owned(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, ChainParams, JobState};
    use crate::field;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        PrimitiveType, TensorOpReceipt,
    };
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes, sign};
    use crate::verify::{AttestationStatement, FreivaldsParams, ValidatorAttestation};

    #[test]
    fn watcher_reports_invalid_receipts_and_data_withholding() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        let validator_a = address(b"validator-a");
        let validator_b = address(b"validator-b");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator_a, 10_000).unwrap();
        chain.register_validator(validator_b, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        chain
            .submit_attestation(ValidatorAttestation::new(
                validator_a,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Invalid,
                    checks_root: hash_bytes(b"test", &[b"invalid"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator_b,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Unavailable,
                    checks_root: hash_bytes(b"test", &[b"unavailable"]),
                    data_availability_passed: false,
                },
            ))
            .unwrap();

        let report = ChainWatcher::default().scan(&chain);
        assert!(report.has_findings());
        assert_eq!(report.invalid_receipts, 1);
        assert_eq!(report.data_withholding_incidents, 1);
        assert_eq!(
            report.events_by_kind(WatchEventKind::InvalidReceipt)[0].actor,
            Some(miner)
        );
        assert_eq!(
            report.events_by_kind(WatchEventKind::DataUnavailable)[0].actor,
            Some(miner)
        );
    }

    #[test]
    fn watcher_flags_validator_misconduct_in_audited_state() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        let bad_attestation = ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: hash_bytes(b"test", &[b"wrong-job"]),
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );
        chain.insert_attestation_for_testing(bad_attestation);

        let report = ChainWatcher::default().scan(&chain);
        assert_eq!(report.validator_misconduct_events, 1);
        assert_eq!(
            report.events_by_kind(WatchEventKind::ValidatorMisconduct)[0].actor,
            Some(validator)
        );
    }

    #[test]
    fn watcher_reports_receipts_blocked_by_redundant_agreement() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let params = ChainParams {
            agreement_quorum: 2,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
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
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"valid"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();

        assert!(chain.has_attestation_quorum(&receipt.receipt_id));
        let report = ChainWatcher::default().scan(&chain);
        assert_eq!(report.missing_quorum_receipts, 0);
        assert_eq!(report.missing_redundant_agreement_receipts, 1);
        assert_eq!(
            report.events_by_kind(WatchEventKind::MissingRedundantAgreement)[0].receipt_id,
            receipt.receipt_id
        );
    }

    #[test]
    fn watcher_flags_malformed_attestation_evidence() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        let unknown_validator = address(b"unknown-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        let stake_mismatch = ValidatorAttestation::new(
            validator,
            9_999,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"stake-mismatch"]),
                data_availability_passed: true,
            },
        );
        let mut bad_signature = ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"bad-signature"]),
                data_availability_passed: true,
            },
        );
        bad_signature.signature = [7; 32];
        let unknown_receipt_id = hash_bytes(b"test", &[b"unknown-receipt"]);
        let unknown_receipt = ValidatorAttestation::new(
            unknown_validator,
            10_000,
            AttestationStatement {
                receipt_id: unknown_receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"unknown-receipt"]),
                data_availability_passed: true,
            },
        );
        chain.insert_attestation_for_testing(stake_mismatch);
        chain.insert_attestation_for_testing(bad_signature);
        chain.insert_attestation_for_testing(unknown_receipt);

        let report = ChainWatcher::default().scan(&chain);
        assert!(report.validator_misconduct_events >= 4);
        assert!(
            report
                .events_by_kind(WatchEventKind::ValidatorMisconduct)
                .iter()
                .any(|event| event.message == "attestation references unknown receipt")
        );
        assert!(
            report
                .events_by_kind(WatchEventKind::ValidatorMisconduct)
                .iter()
                .any(|event| event.message == "attestation signature does not verify")
        );
    }

    #[test]
    fn watcher_reports_missing_quorum_and_respects_config_flags() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let mut chain = Chain::new(beacon);
        let miner = address(b"miner");
        chain.register_miner(miner, 100).unwrap();
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

        let report = ChainWatcher::new(WatcherConfig {
            flag_missing_quorum: true,
            flag_missing_redundant_agreement: false,
            flag_conflicting_linear_transitions: false,
        })
        .scan(&chain);
        assert_eq!(report.missing_quorum_receipts, 1);

        let quiet = ChainWatcher::new(WatcherConfig {
            flag_missing_quorum: false,
            flag_missing_redundant_agreement: false,
            flag_conflicting_linear_transitions: false,
        })
        .scan(&chain);
        assert!(!quiet.has_findings());
    }

    #[test]
    fn watcher_skips_settled_receipts_and_non_quorum_linear_conflict_candidates() {
        let beacon = hash_bytes(b"test", &[b"watcher-skip-beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"watcher-skip-miner");
        let validator = address(b"watcher-skip-validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();

        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"watcher-skip-model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"watcher-skip-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 2,
            deadline_block: 10,
        });
        let (mut quorum_linear, mut output) =
            LinearTrainingStepReceipt::from_job(&linear_job, miner, &weights, 1, 5).unwrap();
        quorum_linear.receipt_id = [1; 32];
        output
            .weight_after
            .set2(0, 0, field::add(output.weight_after.get2(0, 0).unwrap(), 1))
            .unwrap();
        let mut no_quorum_linear = quorum_linear.clone();
        no_quorum_linear.weight_root_after = output.weight_after.commitment_root();
        no_quorum_linear.trace_root = hash_bytes(b"test", &[b"no-quorum-linear-trace"]);
        no_quorum_linear.execution_time_ms = 6;
        no_quorum_linear.receipt_id =
            no_quorum_linear.recompute_receipt_id(&linear_job.program_hash());
        no_quorum_linear.signature = sign(&no_quorum_linear.miner, &no_quorum_linear.receipt_id);
        no_quorum_linear.receipt_id = [3; 32];

        let tensor_job = MatmulJob::synthetic(0, 2, 2, 2, 2, &beacon, 10);
        let (mut tensor_receipt, _a, _b, _c) =
            TensorOpReceipt::from_job(&tensor_job, miner, 1, 5).unwrap();
        tensor_receipt.receipt_id = [2; 32];

        chain.submit_job(JobState::LinearTrainingStep(linear_job));
        chain.submit_job(JobState::TensorOp(tensor_job));
        chain.insert_receipt_for_testing(ReceiptState::LinearTrainingStep(quorum_linear.clone()));
        chain.insert_receipt_for_testing(ReceiptState::TensorOp(tensor_receipt.clone()));
        chain.insert_receipt_for_testing(ReceiptState::LinearTrainingStep(no_quorum_linear));
        chain.mark_receipt_settled_for_testing(tensor_receipt.receipt_id);

        for (receipt_id, job_id, primitive_type) in [
            (
                quorum_linear.receipt_id,
                quorum_linear.job_id,
                PrimitiveType::LinearTrainingStep,
            ),
            (
                tensor_receipt.receipt_id,
                tensor_receipt.job_id,
                PrimitiveType::TensorOp,
            ),
        ] {
            chain
                .submit_attestation(ValidatorAttestation::new(
                    validator,
                    10_000,
                    AttestationStatement {
                        receipt_id,
                        job_id,
                        primitive_type,
                        result: VerificationResult::Valid,
                        checks_root: hash_bytes(b"test", &[&receipt_id]),
                        data_availability_passed: true,
                    },
                ))
                .unwrap();
        }

        let report = ChainWatcher::new(WatcherConfig {
            flag_missing_quorum: false,
            flag_missing_redundant_agreement: false,
            flag_conflicting_linear_transitions: true,
        })
        .scan(&chain);
        assert!(!report.has_findings());
    }

    #[test]
    fn watcher_reports_conflicting_linear_transitions() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                minimum_stake_numerator: 1,
                minimum_stake_denominator: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"miner");
        let validator = address(b"validator");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(validator, 10_000).unwrap();
        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 2,
            deadline_block: 10,
        });
        let (receipt, mut output) =
            LinearTrainingStepReceipt::from_job(&job, miner, &weights, 1, 5).unwrap();
        output
            .weight_after
            .set2(0, 0, field::add(output.weight_after.get2(0, 0).unwrap(), 1))
            .unwrap();
        let mut conflicting = receipt.clone();
        conflicting.weight_root_after = output.weight_after.commitment_root();
        conflicting.trace_root = hash_bytes(b"test", &[b"conflicting-linear-trace"]);
        conflicting.execution_time_ms = 6;
        conflicting.receipt_id = conflicting.recompute_receipt_id(&job.program_hash());
        conflicting.signature = sign(&conflicting.miner, &conflicting.receipt_id);
        chain.submit_job(JobState::LinearTrainingStep(job.clone()));
        chain.submit_linear_receipt(receipt.clone()).unwrap();
        chain.submit_linear_receipt(conflicting.clone()).unwrap();

        for receipt in [&receipt, &conflicting] {
            chain
                .submit_attestation(ValidatorAttestation::new(
                    validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::LinearTrainingStep,
                        result: VerificationResult::Valid,
                        checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                        data_availability_passed: true,
                    },
                ))
                .unwrap();
        }

        let report = ChainWatcher::default().scan(&chain);
        assert_eq!(report.conflicting_linear_transitions, 1);
        assert_eq!(
            report.events_by_kind(WatchEventKind::ConflictingLinearTransition)[0].job_id,
            Some(job.job_id)
        );
    }
}
