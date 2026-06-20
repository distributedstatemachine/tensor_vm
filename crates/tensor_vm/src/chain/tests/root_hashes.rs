use super::*;

#[test]
fn miner_root_commits_to_operator_identity() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"operator-root-miner");
    chain
        .register_miner_with_operator(
            miner,
            chain.params().miner_min_stake,
            address(b"operator-root-a"),
        )
        .unwrap();

    let original_root = miner_root(chain.state().miners());
    let mut changed_miners = chain.state().miners().clone();
    changed_miners.get_mut(&miner).unwrap().operator_id = address(b"operator-root-b");

    assert_ne!(original_root, miner_root(&changed_miners));
}

#[test]
fn state_root_commits_to_data_unavailability_slash_records() {
    let beacon = hash_bytes(b"test", &[b"slash-root-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"slash-root-miner");
    let validator = address(b"slash-root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    chain.mark_receipt_data_unavailable_for_testing(receipt.receipt_id);

    let before = chain.state_root();
    chain.produce_block(validator, 1_000).unwrap();

    assert!(
        chain
            .state()
            .data_unavailability_slashes()
            .contains_key(&receipt.receipt_id)
    );
    assert_ne!(before, chain.state_root());
}

#[test]
fn state_root_commits_to_validator_audit_records() {
    let beacon = hash_bytes(b"test", &[b"audit-root-beacon"]);
    let params = ChainParams {
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 1,
        validator_audit_window_blocks: 3,
        validator_audit_slash_amount: 25,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"audit-root-miner");
    let candidate_auditor = address(b"audit-root-auditor");
    let validator = address(b"audit-root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(candidate_auditor, 10_000).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let audited = JobScheduler::default()
        .assign_validators(&chain, receipt.receipt_id, &assignment_seed)
        .validators[0];
    chain
        .submit_attestation(ValidatorAttestation::new(
            audited,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"audit-root-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    let before_assignment = chain.state_root();
    chain.produce_block(audited, 1_000).unwrap();
    let after_assignment = chain.state_root();
    assert_ne!(before_assignment, after_assignment);
    let audit_id = *chain
        .state()
        .validator_audit_assignments()
        .keys()
        .next()
        .unwrap();
    let auditor = chain.state().validator_audit_assignments()[&audit_id].auditor;
    assert_ne!(auditor, audited);

    chain
        .submit_validator_audit_report(ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Invalid,
            true,
            hash_bytes(b"test", &[b"audit-root-canonical"]),
        ))
        .unwrap();
    assert!(
        chain
            .state()
            .validator_audit_results()
            .contains_key(&audit_id)
    );
    assert!(
        chain
            .state()
            .validator_audit_slashes()
            .contains_key(&audit_id)
    );
    assert_ne!(after_assignment, chain.state_root());
}
