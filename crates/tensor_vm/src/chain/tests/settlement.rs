use super::*;

#[test]
fn chain_settles_valid_tensorwork_and_rewards_participants() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..5)
        .map(|i| address(format!("validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }

    let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job.clone()));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    for validator in &validators {
        chain
            .submit_attestation(ValidatorAttestation::new(
                *validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: report.result,
                    checks_root: report.checks_root,
                    data_availability_passed: report.data_availability_passed,
                },
            ))
            .unwrap();
    }

    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        receipt.tensor_work_units
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    let claimable_at_height = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == miner)
        .unwrap()
        .claimable_at_height;
    assert_eq!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.beneficiary == miner)
            .count(),
        1
    );
    let pending_validator_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == validators[0])
        .unwrap()
        .amount;
    assert!(pending_validator_reward > 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    chain.set_position_for_testing(claimable_at_height, 0);
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            beneficiary,
            amount: 1_000,
            ..
        } if *beneficiary == miner
    )));
    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
    let validator_reward = chain.state().rewards().balance(&validators[0]);
    assert!(validator_reward > 0);
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
    assert_eq!(
        chain.state().rewards().balance(&validators[0]),
        validator_reward
    );
}

#[test]
fn unavailable_data_evidence_voids_delayed_receipt_rewards_before_release() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 3,
            minimum_validators: 2,
            minimum_stake_numerator: 2,
            minimum_stake_denominator: 3,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"delayed-unavailable-miner");
    chain.register_miner(miner, 100).unwrap();
    for i in 0..8 {
        chain
            .register_validator(
                address(format!("delayed-unavailable-validator-{i}").as_bytes()),
                10_000,
            )
            .unwrap();
    }

    let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"delayed-unavailable-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assigned =
        JobScheduler::default().assign_validators(&chain, receipt.receipt_id, &assignment_seed);
    let validators = assigned.validators;
    assert_eq!(validators.len(), 3);
    for validator in validators.iter().take(2) {
        chain
            .submit_attestation(ValidatorAttestation::new(
                *validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: report.result,
                    checks_root: report.checks_root,
                    data_availability_passed: report.data_availability_passed,
                },
            ))
            .unwrap();
    }
    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    let claimable_at_height = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt.receipt_id)
        .unwrap()
        .claimable_at_height;
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id && !reward.voided_by_challenge)
    );

    chain
        .submit_attestation(ValidatorAttestation::new(
            validators[2],
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Unavailable,
                checks_root: hash_bytes(b"test", &[b"delayed-unavailable"]),
                data_availability_passed: false,
            },
        ))
        .unwrap();

    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipt.receipt_id)
            .all(|reward| reward.voided_by_challenge)
    );
    chain.set_position_for_testing(claimable_at_height, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    for validator in validators.iter().take(2) {
        assert_eq!(chain.state().rewards().balance(validator), 0);
    }
}

#[test]
fn quorum_and_agreement_helpers_reject_unknown_receipts() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"orphan-validator");
    chain.register_validator(validator, 10_000).unwrap();
    let receipt_id = hash_bytes(b"test", &[b"orphan-receipt"]);
    chain.insert_attestation_for_testing(ValidatorAttestation::new(
        validator,
        10_000,
        AttestationStatement {
            receipt_id,
            job_id: hash_bytes(b"test", &[b"orphan-job"]),
            primitive_type: PrimitiveType::TensorOp,
            result: VerificationResult::Valid,
            checks_root: hash_bytes(b"test", &[b"orphan-checks"]),
            data_availability_passed: true,
        },
    ));

    assert!(!chain.has_attestation_quorum(&receipt_id));
    assert_eq!(chain.redundant_agreement_count(&receipt_id), 0);
    assert!(!chain.has_redundant_agreement(&receipt_id));
}

#[test]
fn redundant_agreement_quorum_is_required_before_settlement() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 3,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            minimum_stake_numerator: 1,
            minimum_stake_denominator: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miners: Vec<_> = (0..3)
        .map(|i| address(format!("agreement-miner-{i}").as_bytes()))
        .collect();
    for miner in &miners {
        chain.register_miner(*miner, 100).unwrap();
    }
    let validator = address(b"agreement-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 9, 4, 4, 4, &beacon, 10);
    chain.submit_job(JobState::TensorOp(job.clone()));
    let receipts: Vec<_> = miners
        .iter()
        .map(|miner| TensorOpReceipt::from_job(&job, *miner, 1, 5).unwrap().0)
        .collect();
    for receipt in receipts.iter().take(2) {
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
                    checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
    }

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 2);
    assert!(!chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert!(chain.state().settled_receipts().is_empty());

    let receipt = &receipts[2];
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
                checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 3);
    assert!(chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().settled_receipts().len(), 3);
}

#[test]
fn conflicting_linear_training_roots_do_not_settle() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut params = ChainParams::default();
    params.freivalds.minimum_validators = 1;
    params.freivalds.minimum_stake_numerator = 1;
    params.freivalds.minimum_stake_denominator = 1;
    params.agreement_quorum = 1;
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
        deadline_block: 20,
    });
    let (receipt, mut output) =
        LinearTrainingStepReceipt::from_job(&job, miner, &weights, 1, 5).unwrap();
    let tensor_job = MatmulJob::synthetic(0, 99, 2, 2, 2, &beacon, 20);
    let (tensor_receipt, _a, _b, _c) = TensorOpReceipt::from_job(&tensor_job, miner, 1, 5).unwrap();
    output
        .weight_after
        .set2(0, 0, output.weight_after.get2(0, 0).unwrap() + 1)
        .unwrap();
    let mut conflicting = receipt.clone();
    conflicting.weight_root_after = output.weight_after.commitment_root();
    conflicting.trace_root = hash_bytes(b"test", &[b"settlement-conflict-trace"]);
    conflicting.receipt_id = conflicting.recompute_receipt_id(&job.program_hash());
    conflicting.signature = sign(&conflicting.miner, &conflicting.receipt_id);
    chain.submit_job(JobState::LinearTrainingStep(job));
    chain.submit_job(JobState::TensorOp(tensor_job));
    chain
        .submit_tensor_op_receipt(tensor_receipt.clone())
        .unwrap();
    chain.submit_linear_receipt(receipt.clone()).unwrap();
    assert!(!has_conflicting_linear_receipt(
        &chain,
        receipt.receipt_id,
        &receipt
    ));
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

    chain.settle_epoch(1_000, 500);
    assert!(chain.state().settled_receipts().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
}
