use super::*;

#[test]
fn invalid_attestations_do_not_create_quorum() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    let validator = address(b"validator");
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job.clone()));
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
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();
    assert!(!chain.has_attestation_quorum(&receipt.receipt_id));
    assert_ne!(attestation_root(chain.state().attestations()), [0; 32]);
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 0);
}

#[test]
fn unavailable_data_attestation_penalizes_receipt_miner_once() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"unavailable-miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..2)
        .map(|i| address(format!("unavailable-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job));
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
                    result: VerificationResult::Unavailable,
                    checks_root: hash_bytes(b"test", &[b"unavailable"]),
                    data_availability_passed: false,
                },
            ))
            .unwrap();
    }

    assert_eq!(
        chain.state().miners().get(&miner).unwrap().reputation,
        -1,
        "availability penalty is per receipt, not per validator"
    );
    assert!(
        chain
            .state()
            .data_unavailable_receipts()
            .contains(&receipt.receipt_id)
    );
    assert_ne!(attestation_root(chain.state().attestations()), [0; 32]);
    assert!(!chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 0);
}

#[test]
fn mismatched_attestation_metadata_penalizes_validator_and_is_rejected() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"mismatch-miner");
    let validator = address(b"mismatch-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
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

    assert_eq!(
        chain.submit_attestation(bad_attestation),
        Err(TvmError::InvalidReceipt("attestation receipt mismatch"))
    );
    assert_eq!(
        chain
            .state()
            .validators()
            .get(&validator)
            .unwrap()
            .reputation,
        -1
    );
    assert!(
        !chain
            .state()
            .attestations()
            .contains_key(&receipt.receipt_id)
    );
}

#[test]
fn duplicate_receipts_and_validator_attestations_are_rejected() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    let validator = address(b"validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    assert_eq!(
        chain.register_miner(miner, 100),
        Err(TvmError::InvalidReceipt("miner already registered"))
    );
    assert_eq!(
        chain.register_validator(validator, 10_000),
        Err(TvmError::InvalidReceipt("validator already registered"))
    );

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
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
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    assert_eq!(
        chain.submit_tensor_op_receipt(receipt.clone()),
        Err(TvmError::InvalidReceipt("duplicate receipt"))
    );

    let attestation = ValidatorAttestation::new(
        validator,
        10_000,
        AttestationStatement {
            receipt_id: receipt.receipt_id,
            job_id: receipt.job_id,
            primitive_type: PrimitiveType::TensorOp,
            result: report.result,
            checks_root: report.checks_root,
            data_availability_passed: report.data_availability_passed,
        },
    );
    chain.submit_attestation(attestation.clone()).unwrap();
    assert_eq!(
        chain.submit_attestation(attestation),
        Err(TvmError::InvalidReceipt("duplicate validator attestation"))
    );
    assert_eq!(
        chain
            .state()
            .attestations()
            .get(&receipt.receipt_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn forged_attestation_stake_is_rejected() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    let validator = address(b"validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job.clone()));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

    let result = chain.submit_attestation(ValidatorAttestation::new(
        validator,
        1_000_000,
        AttestationStatement {
            receipt_id: receipt.receipt_id,
            job_id: receipt.job_id,
            primitive_type: PrimitiveType::TensorOp,
            result: VerificationResult::Valid,
            checks_root: hash_bytes(b"test", &[b"checks"]),
            data_availability_passed: true,
        },
    ));

    assert!(matches!(
        result,
        Err(TvmError::InvalidReceipt("attestation stake mismatch"))
    ));
}

#[test]
fn unassigned_validator_attestations_are_rejected() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            minimum_stake_numerator: 1,
            minimum_stake_denominator: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"assignment-miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..6)
        .map(|i| address(format!("assignment-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assignment =
        JobScheduler::default().assign_validators(&chain, receipt.receipt_id, &assignment_seed);
    let assigned = assignment.validators[0];
    let unassigned = validators
        .iter()
        .copied()
        .find(|validator| *validator != assigned)
        .expect("single-validator assignment should leave an unassigned validator");
    let statement = AttestationStatement {
        receipt_id: receipt.receipt_id,
        job_id: receipt.job_id,
        primitive_type: PrimitiveType::TensorOp,
        result: VerificationResult::Valid,
        checks_root: hash_bytes(b"test", &[b"checks"]),
        data_availability_passed: true,
    };

    assert_eq!(
        chain.submit_attestation(ValidatorAttestation::new(
            unassigned,
            10_000,
            statement.clone(),
        )),
        Err(TvmError::InvalidReceipt(
            "validator not assigned to receipt"
        ))
    );
    assert!(
        !chain
            .state()
            .attestations()
            .contains_key(&receipt.receipt_id)
    );
    chain
        .submit_attestation(ValidatorAttestation::new(assigned, 10_000, statement))
        .unwrap();
    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
}
