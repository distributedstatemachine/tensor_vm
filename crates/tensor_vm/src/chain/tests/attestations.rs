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
fn unavailable_data_attestation_slashes_receipt_miner_once_on_block_apply() {
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
    let starting_state_root = chain.state_root();
    let starting_stake = chain.state().miners().get(&miner).unwrap().stake;
    let starting_treasury = chain.state().rewards().treasury();

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
    assert_eq!(
        chain.state().miners().get(&miner).unwrap().stake,
        starting_stake,
        "attestation admission marks evidence; block application performs the slash"
    );
    assert!(chain.state().data_unavailability_slashes().is_empty());
    assert_ne!(attestation_root(chain.state().attestations()), [0; 32]);
    assert!(!chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 0);

    let slash_amount = chain.params().data_unavailability_miner_slash_amount;
    chain.produce_block(validators[0], 1_000).unwrap();
    let slashed = chain
        .state()
        .data_unavailability_slashes()
        .get(&receipt.receipt_id)
        .expect("unavailable receipt must have a slash record");
    assert_eq!(slashed.receipt_id, receipt.receipt_id);
    assert_eq!(slashed.miner, miner);
    assert_eq!(slashed.evidence_validator, validators[0]);
    assert_eq!(slashed.amount, slash_amount);
    assert_eq!(slashed.slashed_at_height, 0);
    assert_eq!(
        chain.state().miners().get(&miner).unwrap().stake,
        starting_stake - slash_amount
    );
    assert_eq!(
        chain.state().rewards().treasury(),
        starting_treasury + slash_amount
    );
    assert_ne!(chain.state_root(), starting_state_root);

    let proposer = chain
        .proposer_for_next_epoch(&chain.state().finalized_randomness())
        .unwrap();
    chain.produce_block(proposer, 1_024).unwrap();
    assert_eq!(chain.state().data_unavailability_slashes().len(), 1);
    assert_eq!(
        chain.state().miners().get(&miner).unwrap().stake,
        starting_stake - slash_amount
    );
    assert_eq!(
        chain.state().rewards().treasury(),
        starting_treasury + slash_amount
    );
}

#[test]
fn mandatory_validator_audit_assignment_missed_slashes_once_on_block_apply() {
    let beacon = hash_bytes(b"test", &[b"audit-missed-beacon"]);
    let params = ChainParams {
        epoch_length: 1,
        reward_settlement_delay_epochs: 0,
        challenge_window_epochs: 0,
        agreement_quorum: 1,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 1,
        validator_audit_window_blocks: 1,
        validator_audit_slash_amount: 77,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"audit-missed-miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("audit-missed-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assigned = JobScheduler::default()
        .assign_validators(&chain, receipt.receipt_id, &assignment_seed)
        .validators[0];
    chain
        .submit_attestation(ValidatorAttestation::new(
            assigned,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"audit-missed-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    chain.settle_epoch(100, 10);
    let pending_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == assigned
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("validator reward should be pending before audit assignment");
    assert_eq!(
        pending_validator_claim.claimable_at_height,
        chain
            .state()
            .height()
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    assert!(!pending_validator_claim.voided_by_challenge);

    let starting_treasury = chain.state().rewards().treasury();
    let starting_stake = chain.state().validators().get(&assigned).unwrap().stake;
    chain.produce_block(validators[0], 1_000).unwrap();
    let audit_id = *chain
        .state()
        .validator_audit_assignments()
        .keys()
        .next()
        .expect("mandatory audit must be assigned");
    let assignment = chain
        .state()
        .validator_audit_assignments()
        .get(&audit_id)
        .unwrap();
    let assigned_auditor = assignment.auditor;
    assert_eq!(assignment.receipt_id, receipt.receipt_id);
    assert_eq!(assignment.validator, assigned);
    assert_ne!(assigned_auditor, assigned);
    assert!(chain.state().validators().contains_key(&assigned_auditor));
    assert_eq!(assignment.assigned_at_height, 0);
    assert_eq!(assignment.deadline_height, 1);
    let delayed_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == assigned
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("validator reward should remain pending through audit deadline");
    assert_eq!(delayed_validator_claim.claimable_at_height, 1);
    assert!(!delayed_validator_claim.voided_by_challenge);
    assert!(chain.state().validator_audit_slashes().is_empty());

    chain.produce_block(validators[0], 1_012).unwrap();
    let slash = chain
        .state()
        .validator_audit_slashes()
        .get(&audit_id)
        .expect("missed audit must slash");
    assert_eq!(slash.validator, assigned);
    assert_eq!(slash.auditor, assigned_auditor);
    assert_eq!(slash.amount, 77);
    assert_eq!(slash.slashed_at_height, 1);
    assert_eq!(slash.reason, "validator missed mandatory audit");
    assert_eq!(
        chain.state().validators().get(&assigned).unwrap().stake,
        starting_stake - 77
    );
    assert_eq!(
        chain
            .state()
            .validators()
            .get(&assigned)
            .unwrap()
            .missed_assignments,
        1
    );
    assert_eq!(chain.state().rewards().treasury(), starting_treasury + 77);
    let voided_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == assigned
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("slashed validator reward should remain pending through appeal deadline");
    assert!(voided_validator_claim.voided_by_challenge);
    assert_eq!(
        voided_validator_claim.claimable_at_height,
        slash
            .slashed_at_height
            .saturating_add(chain.params().validator_audit_window_blocks.max(1))
    );
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(!release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            beneficiary,
            ..
        } if *beneficiary == assigned
    )));
    assert_eq!(chain.state().rewards().balance(&assigned), 0);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .all(|reward| reward.beneficiary != assigned || reward.receipt_id != receipt.receipt_id),
        "voided validator reward should be pruned without credit once appeal hold matures"
    );

    let proposer = chain
        .proposer_for_next_epoch(&chain.state().finalized_randomness())
        .unwrap();
    chain.produce_block(proposer, 1_024).unwrap();
    assert_eq!(chain.state().validator_audit_slashes().len(), 1);
    assert_eq!(
        chain.state().validators().get(&assigned).unwrap().stake,
        starting_stake - 77
    );
}

#[test]
fn mandatory_validator_audit_assignment_requires_separate_auditor() {
    let beacon = hash_bytes(b"test", &[b"audit-separate-auditor"]);
    let params = ChainParams {
        epoch_length: 1,
        agreement_quorum: 1,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 1,
        validator_audit_window_blocks: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"audit-separate-miner");
    let validator = address(b"audit-separate-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
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
                checks_root: hash_bytes(b"test", &[b"audit-separate-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    chain.produce_block(validator, 1_000).unwrap();
    assert!(chain.state().validator_audit_assignments().is_empty());
}

#[test]
fn validator_audit_report_slashes_contradicted_attestation_and_accepts_matching_result() {
    let beacon = hash_bytes(b"test", &[b"audit-report-beacon"]);
    let params = ChainParams {
        epoch_length: 1,
        reward_settlement_delay_epochs: 0,
        challenge_window_epochs: 0,
        agreement_quorum: 1,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 1,
        validator_audit_window_blocks: 3,
        validator_audit_slash_amount: 55,
        freivalds: FreivaldsParams {
            validators_per_job: 1,
            minimum_validators: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"audit-report-miner");
    let candidate_auditor = address(b"audit-report-auditor");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(candidate_auditor, 10_000).unwrap();
    let validators: Vec<_> = (0..4)
        .map(|i| address(format!("audit-report-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
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
                checks_root: hash_bytes(b"test", &[b"audit-report-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();
    chain.settle_epoch(100, 10);
    let pending_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == audited
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("audited validator reward should be pending before assignment");
    assert_eq!(
        pending_validator_claim.claimable_at_height,
        chain
            .state()
            .height()
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    let starting_stake = chain.state().validators().get(&audited).unwrap().stake;
    let starting_treasury = chain.state().rewards().treasury();
    chain.produce_block(validators[0], 1_000).unwrap();
    let audit_id = *chain
        .state()
        .validator_audit_assignments()
        .keys()
        .next()
        .expect("mandatory audit must be assigned");
    let auditor = chain.state().validator_audit_assignments()[&audit_id].auditor;
    assert_ne!(auditor, audited);
    let unauthorized_auditor = validators
        .iter()
        .copied()
        .find(|validator| *validator != audited && *validator != auditor)
        .expect("non-selected registered auditor should exist");
    assert_eq!(
        chain.submit_validator_audit_report(ValidatorAuditReport::new(
            audit_id,
            unauthorized_auditor,
            VerificationResult::Invalid,
            true,
            hash_bytes(b"test", &[b"unauthorized-audit-report"]),
        )),
        Err(TvmError::InvalidReceipt("validator audit auditor mismatch"))
    );
    let delayed_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == audited
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("audited validator reward should be delayed by assignment");
    assert_eq!(delayed_validator_claim.claimable_at_height, 3);
    assert!(!delayed_validator_claim.voided_by_challenge);

    let report = ValidatorAuditReport::new(
        audit_id,
        auditor,
        VerificationResult::Invalid,
        true,
        hash_bytes(b"test", &[b"audit-report-canonical"]),
    );
    let events = chain.submit_validator_audit_report(report).unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        ChainEvent::ValidatorAuditAccepted {
            audit_id: event_audit_id,
            auditor: event_auditor,
            validator: event_validator,
            passed: false,
        } if *event_audit_id == audit_id
            && *event_auditor == auditor
            && *event_validator == audited
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ChainEvent::ValidatorAuditSlashApplied {
            audit_id: event_audit_id,
            validator: event_validator,
            amount: 55,
            reason,
        } if *event_audit_id == audit_id
            && *event_validator == audited
            && reason == "validator audit contradicted attestation"
    )));
    let result = chain
        .state()
        .validator_audit_results()
        .get(&audit_id)
        .unwrap();
    assert!(!result.passed);
    assert_eq!(result.attested_result, VerificationResult::Valid);
    assert_eq!(result.canonical_result, VerificationResult::Invalid);
    let slash = chain
        .state()
        .validator_audit_slashes()
        .get(&audit_id)
        .cloned()
        .unwrap();
    assert_eq!(slash.validator, audited);
    assert_eq!(slash.auditor, auditor);
    assert_eq!(slash.amount, 55);
    assert_eq!(
        chain.state().validators().get(&audited).unwrap().stake,
        starting_stake - 55
    );
    assert_eq!(chain.state().rewards().treasury(), starting_treasury + 55);
    assert_eq!(
        chain.submit_validator_audit_appeal(ValidatorAuditAppeal::new(
            hash_bytes(b"test", &[b"unknown-audit-appeal"]),
            audited,
            "audit evidence references the wrong receipt",
        )),
        Err(TvmError::InvalidReceipt("unknown validator audit slash"))
    );
    assert_eq!(
        chain.submit_validator_audit_appeal(ValidatorAuditAppeal::new(
            audit_id,
            unauthorized_auditor,
            "only the slashed validator may appeal",
        )),
        Err(TvmError::InvalidReceipt(
            "validator audit appeal signer mismatch"
        ))
    );
    let mut bad_signature = ValidatorAuditAppeal::new(
        audit_id,
        audited,
        "signature does not match the appeal body",
    );
    bad_signature.signature = [0; 32];
    assert_eq!(
        chain.submit_validator_audit_appeal(bad_signature),
        Err(TvmError::InvalidReceipt(
            "bad validator audit appeal signature"
        ))
    );
    let mut expired_chain = chain.clone();
    expired_chain.set_position_for_testing(
        slash
            .slashed_at_height
            .saturating_add(expired_chain.params().validator_audit_window_blocks.max(1))
            .saturating_add(1),
        0,
    );
    assert_eq!(
        expired_chain.submit_validator_audit_appeal(ValidatorAuditAppeal::new(
            audit_id,
            audited,
            "appeal after the audit appeal window",
        )),
        Err(TvmError::InvalidReceipt("validator audit appeal expired"))
    );
    let appeal_events = chain
        .submit_validator_audit_appeal(ValidatorAuditAppeal::new(
            audit_id,
            audited,
            "auditor recomputation omitted the served output chunk",
        ))
        .unwrap();
    let appeal = chain
        .state()
        .validator_audit_appeals()
        .get(&audit_id)
        .unwrap();
    assert_eq!(appeal.receipt_id, receipt.receipt_id);
    assert_eq!(appeal.validator, audited);
    assert_eq!(appeal.auditor, auditor);
    assert_eq!(appeal.slash_amount, 55);
    assert_eq!(appeal.appealed_at_height, chain.state().height());
    assert_eq!(
        appeal.deadline_height,
        slash
            .slashed_at_height
            .saturating_add(chain.params().validator_audit_window_blocks.max(1))
    );
    assert_eq!(
        appeal.reason,
        "auditor recomputation omitted the served output chunk"
    );
    assert!(appeal_events.iter().any(|event| matches!(
        event,
        ChainEvent::ValidatorAuditAppealAccepted {
            audit_id: event_audit_id,
            validator: event_validator,
            deadline_height,
        } if *event_audit_id == audit_id
            && *event_validator == audited
            && *deadline_height == appeal.deadline_height
    )));
    assert_eq!(
        chain.submit_validator_audit_appeal(ValidatorAuditAppeal::new(
            audit_id,
            audited,
            "duplicate appeal",
        )),
        Err(TvmError::InvalidReceipt("duplicate validator audit appeal"))
    );
    let voided_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == audited
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("contradicted validator reward should stay pending until release");
    assert!(voided_validator_claim.voided_by_challenge);
    assert_eq!(
        voided_validator_claim.claimable_at_height,
        slash
            .slashed_at_height
            .saturating_add(chain.params().validator_audit_window_blocks.max(1))
    );
    let claimable_at_height = voided_validator_claim.claimable_at_height;
    let mut upheld_chain = chain.clone();
    let upheld_events = upheld_chain
        .resolve_validator_audit_appeal(audit_id, ValidatorAuditAppealResolution::UpholdSlash)
        .unwrap();
    assert!(upheld_events.iter().any(|event| matches!(
        event,
        ChainEvent::ValidatorAuditAppealResolved {
            audit_id: event_audit_id,
            validator: event_validator,
            resolution: ValidatorAuditAppealResolution::UpholdSlash,
            receipt_reward_reinstated: false,
        } if *event_audit_id == audit_id && *event_validator == audited
    )));
    assert!(
        upheld_chain
            .state()
            .validator_audit_appeals()
            .get(&audit_id)
            .unwrap()
            .resolution
            .is_some()
    );
    assert!(
        upheld_chain
            .state()
            .pending_receipt_rewards()
            .values()
            .find(|reward| {
                reward.receipt_id == receipt.receipt_id
                    && reward.beneficiary == audited
                    && reward.kind == ReceiptRewardKind::Validator
            })
            .unwrap()
            .voided_by_challenge
    );
    assert_eq!(
        upheld_chain.resolve_validator_audit_appeal(
            audit_id,
            ValidatorAuditAppealResolution::ReverseRewardVoid,
        ),
        Err(TvmError::InvalidReceipt(
            "validator audit appeal already resolved"
        ))
    );

    let reverse_events = chain
        .resolve_validator_audit_appeal(audit_id, ValidatorAuditAppealResolution::ReverseRewardVoid)
        .unwrap();
    assert!(reverse_events.iter().any(|event| matches!(
        event,
        ChainEvent::ValidatorAuditAppealResolved {
            audit_id: event_audit_id,
            validator: event_validator,
            resolution: ValidatorAuditAppealResolution::ReverseRewardVoid,
            receipt_reward_reinstated: true,
        } if *event_audit_id == audit_id && *event_validator == audited
    )));
    let reinstated_validator_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| {
            reward.receipt_id == receipt.receipt_id
                && reward.beneficiary == audited
                && reward.kind == ReceiptRewardKind::Validator
        })
        .expect("reversed appeal should keep the delayed validator reward claim pending");
    assert!(!reinstated_validator_claim.voided_by_challenge);
    assert_eq!(
        reinstated_validator_claim.claimable_at_height,
        claimable_at_height
    );
    assert_eq!(chain.state().rewards().balance(&audited), 0);
    chain.set_position_for_testing(claimable_at_height.saturating_sub(1), 0);
    let early_release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(!early_release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            receipt_id: event_receipt_id,
            beneficiary,
            ..
        } if *event_receipt_id == receipt.receipt_id && *beneficiary == audited
    )));
    assert_eq!(chain.state().rewards().balance(&audited), 0);
    assert_eq!(
        chain.submit_validator_audit_report(ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Invalid,
            true,
            hash_bytes(b"test", &[b"duplicate-audit-report"]),
        )),
        Err(TvmError::InvalidReceipt("duplicate validator audit result"))
    );
    chain.set_position_for_testing(claimable_at_height, 0);
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            receipt_id: event_receipt_id,
            beneficiary,
            ..
        } if *event_receipt_id == receipt.receipt_id && *beneficiary == audited
    )));
    assert_eq!(chain.state().rewards().balance(&audited), 10);

    let mut passing_chain = Chain::with_params(chain.params().clone(), beacon);
    passing_chain.register_miner(miner, 100).unwrap();
    passing_chain
        .register_validator(candidate_auditor, 10_000)
        .unwrap();
    for validator in &validators {
        passing_chain
            .register_validator(*validator, 10_000)
            .unwrap();
    }
    let job = MatmulJob::synthetic(0, 1, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    passing_chain.submit_job(JobState::TensorOp(job));
    passing_chain
        .submit_tensor_op_receipt(receipt.clone())
        .unwrap();
    let assignment_seed = passing_chain.validator_assignment_seed(&receipt.receipt_id);
    let audited = JobScheduler::default()
        .assign_validators(&passing_chain, receipt.receipt_id, &assignment_seed)
        .validators[0];
    passing_chain
        .submit_attestation(ValidatorAttestation::new(
            audited,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"passing-audit-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();
    let proposer = passing_chain.proposer_for_next_epoch(&beacon).unwrap();
    passing_chain.produce_block(proposer, 1_000).unwrap();
    let audit_id = *passing_chain
        .state()
        .validator_audit_assignments()
        .keys()
        .next()
        .expect("mandatory audit must be assigned");
    let auditor = passing_chain.state().validator_audit_assignments()[&audit_id].auditor;
    let events = passing_chain
        .submit_validator_audit_report(ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Valid,
            true,
            hash_bytes(b"test", &[b"passing-audit-canonical"]),
        ))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(passing_chain.state().validator_audit_results()[&audit_id].passed);
    assert!(passing_chain.state().validator_audit_slashes().is_empty());
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
