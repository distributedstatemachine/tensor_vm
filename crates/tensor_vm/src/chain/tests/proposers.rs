use super::*;

#[test]
fn validation_seed_is_bound_to_finalized_randomness_and_receipt() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let chain = Chain::new(beacon);
    let receipt_a = hash_bytes(b"test", &[b"receipt-a"]);
    let receipt_b = hash_bytes(b"test", &[b"receipt-b"]);
    let validator = address(b"seed-validator");
    assert_ne!(
        chain.validation_seed(&receipt_a, &validator),
        chain.validation_seed(&receipt_b, &validator)
    );

    let other_chain = Chain::new(hash_bytes(b"test", &[b"other-beacon"]));
    assert_ne!(
        chain.validation_seed(&receipt_a, &validator),
        other_chain.validation_seed(&receipt_a, &validator)
    );
}

#[test]
fn validation_seed_is_bound_to_validator_and_beacon_round() {
    let beacon = hash_bytes(b"test", &[b"round-beacon"]);
    let mut chain = Chain::new(beacon);
    let validator_a = address(b"round-validator-a");
    let validator_b = address(b"round-validator-b");
    chain.register_validator(validator_a, 10_000).unwrap();
    chain.register_validator(validator_b, 10_000).unwrap();
    let receipt = hash_bytes(b"test", &[b"round-receipt"]);

    assert_ne!(
        chain.validation_seed(&receipt, &validator_a),
        chain.validation_seed(&receipt, &validator_b)
    );

    let seed_before = chain.validation_seed(&receipt, &validator_a);
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
    chain.produce_block(proposer, 1_000).unwrap();
    assert_ne!(seed_before, chain.validation_seed(&receipt, &validator_a));
    assert_eq!(chain.state().finalized_beacon_round(), 1);
}

#[test]
fn admitted_receipt_validation_randomness_is_anchored_at_submission() {
    let beacon = hash_bytes(b"test", &[b"anchored-receipt-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"anchored-receipt-miner");
    let validator_a = address(b"anchored-receipt-validator-a");
    let validator_b = address(b"anchored-receipt-validator-b");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator_a, 10_000).unwrap();
    chain.register_validator(validator_b, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let job_id = job.job_id;
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let anchor = chain
        .state()
        .receipt_randomness_anchors()
        .get(&receipt_id)
        .expect("receipt admission should anchor validation randomness");
    assert_eq!(anchor.receipt_id, receipt_id);
    assert_eq!(anchor.beacon_round, 0);
    assert_eq!(anchor.finalized_randomness, beacon);
    assert_eq!(
        anchor.assignment_seed,
        chain.validator_assignment_seed(&receipt_id)
    );
    assert_eq!(
        anchor.validation_seed_commitment,
        validation::validation_seed_commitment(0, &beacon, &receipt_id)
    );
    let assigned_before = JobScheduler::default()
        .assign_validators(
            &chain,
            receipt_id,
            &chain.validator_assignment_seed(&receipt_id),
        )
        .validators;
    let seed_before = chain.validation_seed(&receipt_id, &validator_a);
    assert_eq!(
        seed_before,
        validation::committed_seed(
            &anchor.validation_seed_commitment,
            &receipt_id,
            &job_id,
            &validator_a,
            0
        )
    );

    chain.produce_block(validator_a, 1_000).unwrap();
    assert_eq!(chain.state().finalized_beacon_round(), 1);
    assert_ne!(chain.state().finalized_randomness(), beacon);

    assert_eq!(
        chain.validation_seed(&receipt_id, &validator_a),
        seed_before
    );
    assert_eq!(
        JobScheduler::default()
            .assign_validators(
                &chain,
                receipt_id,
                &chain.validator_assignment_seed(&receipt_id)
            )
            .validators,
        assigned_before
    );
    let later_job = MatmulJob::synthetic(1, 0, 4, 4, 4, &chain.state().finalized_randomness(), 10);
    let (later_receipt, _a, _b, _c) = TensorOpReceipt::from_job(&later_job, miner, 1, 3).unwrap();
    let later_receipt_id = later_receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(later_job));
    chain.submit_tensor_op_receipt(later_receipt).unwrap();
    assert_eq!(
        chain
            .state()
            .receipt_randomness_anchors()
            .get(&later_receipt_id)
            .unwrap()
            .beacon_round,
        1
    );
    assert_ne!(
        chain.validator_assignment_seed(&receipt_id),
        chain.validator_assignment_seed(&later_receipt_id)
    );
}

#[test]
fn admitted_receipt_attestation_requires_randomness_anchor() {
    let beacon = hash_bytes(b"test", &[b"missing-anchor-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"missing-anchor-miner");
    let validator = address(b"missing-anchor-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    let job_id = receipt.job_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let anchored_seed = chain.validation_seed(&receipt_id, &validator);
    chain.remove_receipt_randomness_anchor_for_testing(&receipt_id);

    assert_ne!(
        chain.validation_seed(&receipt_id, &validator),
        anchored_seed
    );
    assert_eq!(
        chain.submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id,
                job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"missing-anchor-checks"]),
                data_availability_passed: true,
            },
        )),
        Err(TvmError::InvalidReceipt(
            "receipt randomness anchor missing"
        ))
    );
}

#[test]
fn randomness_binding_evidence_reports_receipt_bound_finalized_beacon_policy() {
    let beacon = hash_bytes(b"test", &[b"randomness-binding-evidence"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"randomness-binding-miner");
    let validator = address(b"randomness-binding-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let evidence = chain.state().randomness_binding_evidence();
    assert_eq!(evidence.beacon_source, validation::RANDOMNESS_BEACON_SOURCE);
    assert_eq!(
        evidence.drand_round_mapping,
        validation::RANDOMNESS_DRAND_ROUND_MAPPING
    );
    assert_ne!(
        evidence.drand_round_mapping,
        "not_configured_local_finalized_beacon"
    );
    assert_eq!(
        evidence.vrf_construction,
        validation::RANDOMNESS_VRF_CONSTRUCTION
    );
    assert_ne!(
        evidence.vrf_construction,
        "not_configured_local_finalized_beacon"
    );
    assert_eq!(
        evidence.assignment_seed_domain,
        validation::ASSIGNMENT_SEED_DOMAIN
    );
    assert_eq!(
        evidence.validation_seed_commitment_domain,
        validation::VALIDATION_SEED_COMMITMENT_DOMAIN
    );
    assert_eq!(
        evidence.validation_seed_reveal_domain,
        validation::VALIDATION_SEED_REVEAL_DOMAIN
    );
    assert!(!evidence.current_block_hash_randomness_allowed);
    assert_eq!(evidence.receipt_anchor_count, 1);
    assert_eq!(evidence.finalized_beacon_anchor_count, 1);
    assert_eq!(evidence.finalized_beacon_round_mapping_count, 1);
    assert_eq!(evidence.validator_vrf_seed_count, 1);
    assert_eq!(evidence.receipt_bound_anchor_count, 1);
    assert_eq!(evidence.consistent_anchor_count, 1);
    assert_eq!(evidence.current_block_hash_anchor_count, 0);
    assert!(evidence.all_receipt_anchors_consistent);

    chain.produce_block(validator, 1_000).unwrap();
    let later_evidence = chain.state().randomness_binding_evidence();
    assert_eq!(later_evidence.receipt_anchor_count, 1);
    assert_eq!(later_evidence.finalized_beacon_round_mapping_count, 1);
    assert_eq!(later_evidence.validator_vrf_seed_count, 1);
    assert_eq!(later_evidence.consistent_anchor_count, 1);
    assert!(later_evidence.all_receipt_anchors_consistent);
}

#[test]
fn proposer_selection_uses_validator_stake() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"validator");
    chain.register_validator(validator, 10_000).unwrap();
    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn fallback_proposer_handles_zero_stake_validator_records() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"zero-stake-validator");
    chain.register_validator(validator, 10_000).unwrap();
    chain.set_validator_stake_for_testing(validator, 0).unwrap();

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn proposer_selection_ignores_tensorwork() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"settled-miner");
    let validator = address(b"validator-proposer");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    chain
        .set_miner_tensor_work_for_testing(miner, 1_000_000, 1_000_000)
        .unwrap();

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    assert_eq!(
        chain.produce_block(miner, 1_000),
        Err(TvmError::UnknownValidator)
    );
}
