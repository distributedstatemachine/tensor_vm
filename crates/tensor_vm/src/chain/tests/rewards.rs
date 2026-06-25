use super::*;

fn mine_reward_test_block(block: &mut TensorBlock) {
    if !block.production_kind.requires_pow() {
        let block_hash = block.hash();
        block.proposer_signature = sign(&block.proposer, &block_hash);
        block.validator_signature_aggregate =
            hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
        return;
    }
    for nonce in 0..=u64::MAX {
        block.nonce = nonce;
        if block.pow_valid() {
            let block_hash = block.hash();
            block.proposer_signature = sign(&block.proposer, &block_hash);
            block.validator_signature_aggregate =
                hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
            return;
        }
    }
    unreachable!("nonzero proof target must have a solution")
}

fn finalize_reward_test_block(chain: &mut Chain, block: &TensorBlock) {
    let validators = chain
        .state()
        .validators()
        .iter()
        .map(|(address, validator)| (*address, validator.stake))
        .collect::<Vec<_>>();
    for (validator, stake) in validators {
        if chain.is_block_finalized(&block.hash()) {
            break;
        }
        chain
            .submit_block_vote(BlockVote::new(validator, stake, block))
            .unwrap();
    }
    assert!(chain.is_block_finalized(&block.hash()));
}

fn add_pending_receipt_reward(chain: &mut Chain, beacon: &Hash) -> Hash {
    let miner = address(b"reward-root-miner");
    let validator = address(b"reward-root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let validator = JobScheduler::default()
        .assign_validators(chain, receipt.receipt_id, &assignment_seed)
        .validators[0];
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"reward-root-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
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
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.settle_epoch(1_000, 500);
    let pending_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt.receipt_id)
        .expect("settled receipt should enqueue a delayed reward");
    assert!(pending_reward.awaiting_inclusion());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    receipt.receipt_id
}

#[test]
fn extending_reward_delay_preserves_validator_vrf_reveal_hold() {
    let mut reward = PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"preserve-vrf-delay-claim"]),
        receipt_id: hash_bytes(b"test", &[b"preserve-vrf-delay-receipt"]),
        beneficiary: address(b"preserve-vrf-delay-validator"),
        amount: 25,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::AwaitingValidatorVrfReveal(7),
        voided_by_challenge: false,
    };

    reward.delay_until(11);
    assert_eq!(
        reward.maturity,
        ReceiptRewardMaturity::AwaitingValidatorVrfReveal(11)
    );
    assert!(!reward.is_mature_at(11));
    assert_eq!(reward.claimable_at_height(), None);

    reward.mark_validator_vrf_revealed();
    assert_eq!(reward.maturity, ReceiptRewardMaturity::ClaimableAt(11));
    assert!(reward.is_mature_at(11));
}

#[test]
fn pre_inclusion_reward_delay_stays_awaiting_inclusion_until_block_inclusion() {
    let claim_id = hash_bytes(b"test", &[b"pre-inclusion-delay-claim"]);
    let receipt_id = hash_bytes(b"test", &[b"pre-inclusion-delay-receipt"]);
    let beneficiary = address(b"pre-inclusion-delay-miner");
    let mut reward = PendingReceiptReward {
        claim_id,
        receipt_id,
        beneficiary,
        amount: 25,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::AwaitingInclusion,
        voided_by_challenge: false,
    };

    reward.delay_until(9);
    assert_eq!(
        reward.maturity,
        ReceiptRewardMaturity::AwaitingInclusionUntil(9)
    );
    assert!(reward.awaiting_inclusion());
    assert_eq!(reward.claimable_at_height(), None);
    assert!(!reward.is_mature_at(9));

    reward.include_with_delay(7);
    assert_eq!(reward.maturity, ReceiptRewardMaturity::ClaimableAt(9));
    assert!(reward.is_mature_at(9));

    let mut validator_reward = PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"pre-inclusion-validator-delay-claim"]),
        receipt_id,
        beneficiary: address(b"pre-inclusion-delay-validator"),
        amount: 10,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::AwaitingInclusionUntil(11),
        voided_by_challenge: false,
    };
    validator_reward.include_with_validator_vrf_reveal_delay(8);
    assert_eq!(
        validator_reward.maturity,
        ReceiptRewardMaturity::AwaitingValidatorVrfReveal(11)
    );
    assert_eq!(validator_reward.claimable_at_height(), None);
    assert!(!validator_reward.is_mature_at(11));
}

fn add_settled_receipt_for_blockspace(chain: &mut Chain, beacon: &Hash) -> Hash {
    let miner = address(b"reward-blockspace-miner");
    if !chain.state().miners().contains_key(&miner) {
        chain.register_miner(miner, 100).unwrap();
    }
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt));
    chain.mark_receipt_settled_for_testing(receipt_id);
    receipt_id
}

#[test]
fn reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"reward-proposer");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();

    let allocation = chain.params().reward_allocation(10_000);
    assert_eq!(
        allocation,
        RewardAllocation {
            miner_reward_pool: 7_000,
            validator_reward_pool: 2_000,
            proposer_reward: 500,
            treasury_reward: 500,
        }
    );

    add_settled_receipt_for_blockspace(&mut chain, &beacon);
    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(chain.state().pending_proposer_rewards().is_empty());
    assert_eq!(block.reward_root, reward_root(chain.state()));
    finalize_reward_test_block(&mut chain, &block);
    assert_eq!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .amount,
        500
    );
    assert_ne!(
        reward_root(chain.state()),
        spendable_reward_root(chain.state().rewards())
    );

    chain.settle_epoch_rewards(allocation, proposer);
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert_eq!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .amount,
        1_000
    );
    assert_eq!(chain.state().rewards().treasury(), 500);
    assert_eq!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .claimable_at_height,
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .block_height
            .saturating_add(chain.params().proposer_reward_maturity_delay_blocks())
    );
    add_settled_receipt_for_blockspace(&mut chain, &beacon);
    chain.produce_block(proposer, 1_012).unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    let claimable_at_height = chain
        .state()
        .pending_proposer_rewards()
        .get(&block.height)
        .unwrap()
        .claimable_at_height;
    chain.set_position_for_testing(claimable_at_height, 1);
    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );
    chain
        .apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert_eq!(
        chain.state().accounts().get(&proposer).unwrap().balance,
        1_000
    );
}

#[test]
fn validator_receipt_reward_waits_for_vrf_reveal_after_maturity() {
    let beacon = hash_bytes(b"test", &[b"validator-reward-vrf-delay"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"validator-reward-vrf-miner");
    let validator = address(b"validator-reward-vrf-validator");
    chain.register_miner(miner, 100).unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();

    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"validator-reward-vrf-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let assigned_validator = JobScheduler::default()
        .assign_validators(
            &chain,
            receipt_id,
            &chain.validator_assignment_seed(&receipt_id),
        )
        .validators[0];
    assert_eq!(assigned_validator, validator);
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            chain.params().validator_min_stake,
            AttestationStatement {
                receipt_id,
                job_id: chain.state().receipts().get(&receipt_id).unwrap().job_id(),
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.settle_epoch(1_000, 500);
    chain.produce_block(validator, 1_000).unwrap();

    let validator_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt_id && reward.beneficiary == validator)
        .unwrap();
    let claimable_at_height = match validator_reward.maturity {
        ReceiptRewardMaturity::AwaitingValidatorVrfReveal(height) => height,
        other => panic!("expected reveal-delayed validator reward, got {other:?}"),
    };
    chain.set_position_for_testing(claimable_at_height, 0);
    assert!(chain.state().pending_reward_claims().iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptValidator
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.beneficiary == validator
            && claim.claimable_at_height == Some(claimable_at_height)
            && !claim.awaiting_inclusion
            && claim.awaiting_validator_vrf_reveal
    }));
    assert_eq!(
        chain.apply_command(ChainCommand::ClaimReward(validator)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );
    let events = chain.release_matured_receipt_rewards().unwrap();
    assert!(events.iter().all(|event| !matches!(
        event,
        ChainEvent::ReceiptRewardReleased { beneficiary, .. } if *beneficiary == validator
    )));
    assert_eq!(chain.state().rewards().balance(&validator), 0);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| { reward.receipt_id == receipt_id && reward.beneficiary == validator })
    );

    let reveal = validation::validator_vrf_reveal_record(&chain, receipt_id, validator, 0).unwrap();
    chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal))
        .unwrap();
    assert_eq!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .find(|reward| reward.receipt_id == receipt_id && reward.beneficiary == validator)
            .unwrap()
            .maturity,
        ReceiptRewardMaturity::ClaimableAt(claimable_at_height)
    );
    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(validator))
        .unwrap();
    assert!(claim_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased { beneficiary, amount, .. }
            if *beneficiary == validator && *amount == 500
    )));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: validator,
        amount: 500,
    }));
    assert_eq!(chain.state().rewards().balance(&validator), 0);
    assert_eq!(
        chain.state().accounts().get(&validator).unwrap().balance,
        500
    );
}

#[test]
fn registered_validator_vrf_key_requires_keyed_reveal_for_reward_release() {
    let beacon = hash_bytes(b"test", &[b"registered-vrf-reward-delay"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"registered-vrf-reward-miner");
    let validator = address(b"registered-vrf-reward-validator");
    let secret = "registered-vrf-reward-secret";
    let public_key = validation::validator_vrf_ed25519_public_key_from_secret(secret);
    chain.register_miner(miner, 100).unwrap();
    chain
        .register_validator(validator, chain.params().validator_min_stake)
        .unwrap();

    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"registered-vrf-reward-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let assigned_validator = JobScheduler::default()
        .assign_validators(
            &chain,
            receipt_id,
            &chain.validator_assignment_seed(&receipt_id),
        )
        .validators[0];
    assert_eq!(assigned_validator, validator);
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            chain.params().validator_min_stake,
            AttestationStatement {
                receipt_id,
                job_id: chain.state().receipts().get(&receipt_id).unwrap().job_id(),
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.settle_epoch(1_000, 500);
    chain.produce_block(validator, 1_000).unwrap();
    let claimable_at_height = match chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt_id && reward.beneficiary == validator)
        .unwrap()
        .maturity
    {
        ReceiptRewardMaturity::AwaitingValidatorVrfReveal(height) => height,
        other => panic!("expected reveal-delayed validator reward, got {other:?}"),
    };

    let legacy_reveal =
        validation::validator_vrf_reveal_record(&chain, receipt_id, validator, 0).unwrap();
    chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(legacy_reveal))
        .unwrap();
    chain
        .apply_command(ChainCommand::RegisterValidatorVrfKey {
            validator,
            vrf_public_key: public_key,
        })
        .unwrap();
    chain.set_position_for_testing(claimable_at_height, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(
        chain.apply_command(ChainCommand::ClaimReward(validator)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );
    assert!(chain.state().pending_reward_claims().iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptValidator
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.beneficiary == validator
            && claim.awaiting_validator_vrf_reveal
    }));

    let keyed_reveal = validation::validator_vrf_reveal_record_with_secret(
        &chain, receipt_id, validator, 0, secret,
    )
    .unwrap();
    chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(keyed_reveal))
        .unwrap();
    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(validator))
        .unwrap();
    assert!(claim_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased { beneficiary, amount, .. }
            if *beneficiary == validator && *amount == 500
    )));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: validator,
        amount: 500,
    }));
    assert_eq!(
        chain.state().accounts().get(&validator).unwrap().balance,
        500
    );
}

#[test]
fn block_reward_root_rejects_spendable_only_root_when_pending_rewards_exist() {
    let beacon = hash_bytes(b"test", &[b"reward-root-rejects-spendable"]);
    let mut parent = Chain::new(beacon);
    let proposer = address(b"reward-root-old-proposer");
    parent
        .register_validator(proposer, parent.params().validator_min_stake)
        .unwrap();

    let mut producer = parent.clone();
    let mut block = producer
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    assert_eq!(producer.state().rewards().balance(&proposer), 0);
    assert_eq!(block.reward_root, reward_root(producer.state()));
    assert_ne!(
        block.reward_root,
        spendable_reward_root(producer.state().rewards())
    );

    block.reward_root = spendable_reward_root(producer.state().rewards());
    mine_reward_test_block(&mut block);
    assert_eq!(
        parent.apply_command(ChainCommand::SubmitBlock(block)),
        Err(TvmError::InvalidReceipt("block reward root mismatch"))
    );
}

#[test]
fn reward_root_commits_to_all_pending_reward_ledgers() {
    let beacon = hash_bytes(b"test", &[b"reward-root-pending-ledgers"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"reward-root-proposer");
    let credit_beneficiary = address(b"reward-root-credit");
    let challenger = address(b"reward-root-challenger");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();
    chain
        .register_validator(challenger, chain.params().validator_min_stake)
        .unwrap();

    let fallback_proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
    let block = chain
        .produce_block_with_rewards(fallback_proposer, 1_000, 400, 100)
        .unwrap();
    finalize_reward_test_block(&mut chain, &block);
    add_pending_receipt_reward(&mut chain, &beacon);
    chain
        .apply_command(ChainCommand::CreditReward {
            address: credit_beneficiary,
            amount: 25,
        })
        .unwrap();
    let challenge_claim = hash_bytes(b"test", &[b"reward-root-challenge-claim"]);
    chain.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
        claim_id: challenge_claim,
        challenge_id: hash_bytes(b"test", &[b"reward-root-challenge"]),
        block_hash: chain.blocks().last().unwrap().hash(),
        receipt_id: hash_bytes(b"test", &[b"reward-root-challenge-receipt"]),
        challenger,
        amount: 50,
        claimable_at_height: chain.state().height() + 10,
        voided_by_challenge: false,
    });

    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert_eq!(chain.state().rewards().balance(&credit_beneficiary), 0);
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    assert!(!chain.state().pending_proposer_rewards().is_empty());
    assert!(!chain.state().pending_receipt_rewards().is_empty());
    assert!(!chain.state().pending_credit_rewards().is_empty());
    assert!(!chain.state().pending_challenge_rewards().is_empty());

    let full_root = reward_root(chain.state());
    assert_ne!(full_root, spendable_reward_root(chain.state().rewards()));

    let mut changed_proposer = chain.state().clone();
    changed_proposer
        .pending_proposer_rewards
        .values_mut()
        .next()
        .unwrap()
        .amount += 1;
    assert_ne!(full_root, reward_root(&changed_proposer));

    let mut changed_receipt = chain.state().clone();
    changed_receipt
        .pending_receipt_rewards
        .values_mut()
        .next()
        .unwrap()
        .maturity = ReceiptRewardMaturity::ClaimableAt(0);
    assert_ne!(full_root, reward_root(&changed_receipt));

    let mut changed_awaiting_inclusion_delay = chain.state().clone();
    changed_awaiting_inclusion_delay
        .pending_receipt_rewards
        .values_mut()
        .next()
        .unwrap()
        .maturity = ReceiptRewardMaturity::AwaitingInclusionUntil(19);
    assert_ne!(full_root, reward_root(&changed_awaiting_inclusion_delay));

    let mut changed_credit = chain.state().clone();
    changed_credit
        .pending_credit_rewards
        .values_mut()
        .next()
        .unwrap()
        .amount += 1;
    assert_ne!(full_root, reward_root(&changed_credit));

    let mut changed_challenge = chain.state().clone();
    changed_challenge
        .pending_challenge_rewards
        .values_mut()
        .next()
        .unwrap()
        .voided_by_challenge = true;
    assert_ne!(full_root, reward_root(&changed_challenge));
}

#[test]
fn pending_reward_claim_view_covers_all_ledgers() {
    let beacon = hash_bytes(b"test", &[b"pending-reward-claim-view"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"claim-view-proposer");
    let challenger = address(b"claim-view-challenger");
    let credit_beneficiary = address(b"claim-view-credit");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();
    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    finalize_reward_test_block(&mut chain, &block);
    let receipt_id = add_pending_receipt_reward(&mut chain, &beacon);
    chain.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
        claim_id: hash_bytes(b"test", &[b"claim-view-challenge-claim"]),
        challenge_id: hash_bytes(b"test", &[b"claim-view-challenge"]),
        block_hash: chain.blocks().last().unwrap().hash(),
        receipt_id,
        challenger,
        amount: 50,
        claimable_at_height: chain.state().height() + 10,
        voided_by_challenge: true,
    });
    chain
        .apply_command(ChainCommand::CreditReward {
            address: credit_beneficiary,
            amount: 25,
        })
        .unwrap();

    let claims = chain.state().pending_reward_claims();
    assert_eq!(claims.len(), 5);
    assert!(claims.windows(2).all(|window| {
        window[0]
            .claimable_at_height
            .unwrap_or(u64::MAX)
            .cmp(&window[1].claimable_at_height.unwrap_or(u64::MAX))
            .then_with(|| window[0].ledger.cmp(&window[1].ledger))
            .then_with(|| window[0].claim_id.cmp(&window[1].claim_id))
            != std::cmp::Ordering::Greater
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Proposer
            && claim.claim_id == RewardClaimKey::BlockHeight(0)
            && claim.subject_id == RewardClaimKey::BlockHeight(0)
            && claim.beneficiary == proposer
            && claim.claimable_at_height.is_some()
            && !claim.awaiting_inclusion
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptMiner
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.amount > 0
            && claim.claimable_at_height.is_none()
            && claim.awaiting_inclusion
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptValidator
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.amount > 0
            && claim.claimable_at_height.is_none()
            && claim.awaiting_inclusion
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Challenge
            && claim.subject_id
                == RewardClaimKey::Hash(hash_bytes(b"test", &[b"claim-view-challenge"]))
            && claim.related_id == Some(RewardClaimKey::Hash(receipt_id))
            && claim.beneficiary == challenger
            && claim.claimable_at_height.is_some()
            && !claim.awaiting_inclusion
            && claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Credit
            && claim.beneficiary == credit_beneficiary
            && claim.amount == 25
            && claim.claimable_at_height.is_some()
            && !claim.awaiting_inclusion
            && !claim.voided_by_challenge
    }));
}

#[test]
fn validator_audit_economic_calibration_delays_immature_validator_reward_exposure() {
    let beacon = hash_bytes(b"test", &[b"audit-economic-calibration"]);
    let params = ChainParams {
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 4,
        validator_audit_slash_amount: 200,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let validator = address(b"audit-economic-validator");
    let miner = address(b"audit-economic-miner");
    let receipt = hash_bytes(b"test", &[b"audit-economic-receipt"]);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"audit-economic-validator-claim-a"]),
        receipt_id: receipt,
        beneficiary: validator,
        amount: 40,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::ClaimableAt(0),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"audit-economic-validator-claim-b"]),
        receipt_id: hash_bytes(b"test", &[b"audit-economic-receipt-b"]),
        beneficiary: validator,
        amount: 80,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::ClaimableAt(10),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"audit-economic-voided-validator-claim"]),
        receipt_id: hash_bytes(b"test", &[b"audit-economic-voided-receipt"]),
        beneficiary: validator,
        amount: 10_000,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::ClaimableAt(11),
        voided_by_challenge: true,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"audit-economic-miner-claim"]),
        receipt_id: receipt,
        beneficiary: miner,
        amount: 10_000,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(11),
        voided_by_challenge: false,
    });

    let calibration = chain
        .state()
        .validator_audit_economic_calibration(chain.params());
    assert_eq!(calibration.detection_numerator, 1);
    assert_eq!(calibration.detection_denominator, 4);
    assert_eq!(calibration.detection_probability_bps, 2_500);
    assert_eq!(calibration.slashable_bond, 200);
    assert_eq!(calibration.reward_from_fraud, 40);
    assert_eq!(calibration.at_risk_validator_reward_claim_count, 2);
    assert_eq!(calibration.required_slashable_bond, 161);
    assert!(calibration.invariant_holds);

    let empty = Chain::new(beacon);
    let empty_calibration = empty
        .state()
        .validator_audit_economic_calibration(empty.params());
    assert_eq!(empty_calibration.reward_from_fraud, 0);
    assert_eq!(empty_calibration.required_slashable_bond, 0);
    assert!(empty_calibration.invariant_holds);
}

#[test]
fn fraud_path_economic_calibration_covers_pending_reward_fraud_paths() {
    let beacon = hash_bytes(b"test", &[b"fraud-path-economic-calibration"]);
    let params = ChainParams {
        data_unavailability_miner_slash_amount: 150,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 2,
        validator_audit_slash_amount: 300,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"fraud-path-proposer");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();
    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    finalize_reward_test_block(&mut chain, &block);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"fraud-path-miner-claim"]),
        receipt_id: hash_bytes(b"test", &[b"fraud-path-miner-receipt"]),
        beneficiary: address(b"fraud-path-miner"),
        amount: 7,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(
            chain
                .state()
                .height()
                .saturating_add(chain.params().fraud_reward_hold_blocks()),
        ),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"fraud-path-immature-miner-claim"]),
        receipt_id: hash_bytes(b"test", &[b"fraud-path-immature-miner-receipt"]),
        beneficiary: address(b"fraud-path-miner"),
        amount: 149,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(9),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: hash_bytes(b"test", &[b"fraud-path-validator-claim"]),
        receipt_id: hash_bytes(b"test", &[b"fraud-path-validator-receipt"]),
        beneficiary: address(b"fraud-path-validator"),
        amount: 120,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::ClaimableAt(10),
        voided_by_challenge: false,
    });

    let calibration = chain
        .state()
        .fraud_path_economic_calibration(chain.params());
    assert_eq!(calibration.path_count, 4);
    assert!(calibration.all_invariants_hold);
    assert_eq!(calibration.worst_path, "block_check");
    assert_eq!(calibration.max_required_slashable_bond, 0);

    let validator_audit = calibration
        .paths
        .iter()
        .find(|path| path.path == "validator_audit")
        .unwrap();
    assert_eq!(validator_audit.detection_numerator, 1);
    assert_eq!(validator_audit.detection_denominator, 2);
    assert_eq!(validator_audit.slashable_bond, 300);
    assert_eq!(validator_audit.reward_from_fraud, 0);
    assert_eq!(validator_audit.required_slashable_bond, 0);
    let invalid_output = calibration
        .paths
        .iter()
        .find(|path| path.path == "invalid_output")
        .unwrap();
    assert_eq!(
        invalid_output.slashable_bond,
        chain.params().invalid_output_miner_slash_amount
    );
    assert_eq!(invalid_output.reward_from_fraud, 0);
    assert_eq!(invalid_output.required_slashable_bond, 0);
    assert!(validator_audit.invariant_holds);

    let data_unavailability = calibration
        .paths
        .iter()
        .find(|path| path.path == "data_unavailability")
        .unwrap();
    assert_eq!(data_unavailability.detection_probability_bps, 10_000);
    assert_eq!(data_unavailability.slashable_bond, 150);
    assert_eq!(data_unavailability.reward_from_fraud, 0);
    assert_eq!(data_unavailability.at_risk_reward_claim_count, 2);
    assert_eq!(data_unavailability.required_slashable_bond, 0);
    assert!(data_unavailability.invariant_holds);

    let block_check = calibration
        .paths
        .iter()
        .find(|path| path.path == "block_check")
        .unwrap();
    assert_eq!(block_check.detection_probability_bps, 10_000);
    assert_eq!(block_check.slashable_bond, 500);
    assert_eq!(block_check.reward_from_fraud, 0);
    assert_eq!(block_check.at_risk_reward_claim_count, 1);
    assert_eq!(block_check.required_slashable_bond, 0);
    assert!(block_check.invariant_holds);
}

#[test]
fn detection_probability_evidence_uses_live_jobs_and_params() {
    let beacon = hash_bytes(b"test", &[b"detection-probability-evidence"]);
    let params = ChainParams {
        replication_factor: 2,
        validator_audit_sample_numerator: 1,
        validator_audit_sample_denominator: 4,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let tensor_job = MatmulJob::synthetic(0, 0, 32, 8, 16, &beacon, 20);
    chain.submit_job(JobState::TensorOp(tensor_job));
    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"detection-model"]),
        step: 0,
        batch_seed: hash_bytes(b"test", &[b"detection-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![2, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![2, 2],
        lr: 1,
        deadline_block: 20,
    });
    chain.submit_job(JobState::LinearTrainingStep(linear_job));

    let evidence = chain.state().detection_probability_evidence(chain.params());
    assert_eq!(evidence.mechanism_count, 10);
    assert!(evidence.live_subject_count >= 2);

    let full_freivalds = evidence
        .mechanisms
        .iter()
        .find(|mechanism| mechanism.mechanism == "full_freivalds")
        .unwrap();
    assert_eq!(full_freivalds.sample_numerator, 1);
    assert_eq!(full_freivalds.sample_denominator, crate::field::MODULUS);
    assert_eq!(full_freivalds.detection_probability_bps, 10_000);
    assert_eq!(full_freivalds.false_accept_probability_bps, 0);
    assert_eq!(full_freivalds.live_subject_count, 1);

    let row_sampling = evidence
        .mechanisms
        .iter()
        .find(|mechanism| mechanism.mechanism == "row_sampling_sparse_audit")
        .unwrap();
    assert_eq!(row_sampling.sample_numerator, 16);
    assert_eq!(row_sampling.sample_denominator, 32);
    assert_eq!(row_sampling.detection_probability_bps, 5_000);
    assert_eq!(row_sampling.false_accept_probability_bps, 5_000);

    let audit = evidence
        .mechanisms
        .iter()
        .find(|mechanism| mechanism.mechanism == "validator_audit")
        .unwrap();
    assert_eq!(audit.detection_probability_bps, 2_500);
    assert_eq!(audit.false_accept_probability_bps, 7_500);

    let data_availability = evidence
        .mechanisms
        .iter()
        .find(|mechanism| mechanism.mechanism == "data_availability_replication")
        .unwrap();
    assert_eq!(data_availability.detection_probability_bps, 9_975);
    assert_eq!(data_availability.false_accept_probability_bps, 25);
}

#[test]
fn verifier_bandwidth_evidence_uses_live_job_and_receipt_shapes() {
    let beacon = hash_bytes(b"test", &[b"verifier-bandwidth-evidence"]);
    let params = ChainParams {
        freivalds: FreivaldsParams {
            full_rounds: 1,
            audit_rows: 16,
            validators_per_job: 2,
            minimum_validators: 1,
            minimum_stake_numerator: 1,
            minimum_stake_denominator: 1,
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"verifier-bandwidth-miner");
    let validator_a = address(b"verifier-bandwidth-validator-a");
    let validator_b = address(b"verifier-bandwidth-validator-b");
    chain.register_miner(miner, 100).unwrap();
    chain
        .register_validator(validator_a, chain.params().validator_min_stake)
        .unwrap();
    chain
        .register_validator(validator_b, chain.params().validator_min_stake)
        .unwrap();

    let tensor_job = MatmulJob::synthetic(0, 0, 32, 8, 16, &beacon, 20);
    let (tensor_receipt, _a, _b, _c) = TensorOpReceipt::from_job(&tensor_job, miner, 0, 1).unwrap();
    chain.submit_job(JobState::TensorOp(tensor_job));
    chain.submit_tensor_op_receipt(tensor_receipt).unwrap();

    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"verifier-bandwidth-model"]),
        step: 0,
        batch_seed: hash_bytes(b"test", &[b"verifier-bandwidth-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![2, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![2, 2],
        lr: 1,
        deadline_block: 20,
    });
    let (linear_receipt, _) =
        LinearTrainingStepReceipt::from_job(&linear_job, miner, &weights, 0, 1).unwrap();
    chain.submit_job(JobState::LinearTrainingStep(linear_job));
    chain.submit_linear_receipt(linear_receipt).unwrap();

    let evidence = chain.state().verifier_bandwidth_evidence(chain.params());
    assert_eq!(evidence.record_count, 3);
    assert_eq!(evidence.live_job_count, 2);
    assert_eq!(evidence.live_receipt_count, 2);
    assert!(evidence.has_live_bounded_evidence);
    assert_eq!(
        evidence.estimated_bandwidth_per_validator_bytes,
        evidence.estimated_total_verification_bytes / 2
    );

    let tensor = evidence
        .records
        .iter()
        .find(|record| record.primitive == "tensor_op")
        .unwrap();
    assert_eq!(tensor.live_job_count, 1);
    assert_eq!(tensor.live_receipt_count, 1);
    assert_eq!(tensor.max_execution_ops, 8192);
    assert_eq!(tensor.max_verification_ops, 896);
    assert!(tensor.max_verification_bytes_per_receipt > 0);
    assert!(tensor.max_verification_to_execution_bps < 2_000);

    let linear = evidence
        .records
        .iter()
        .find(|record| record.primitive == "linear_training_step")
        .unwrap();
    assert_eq!(linear.live_job_count, 1);
    assert_eq!(linear.live_receipt_count, 1);
    assert!(linear.max_verification_bytes_per_receipt > 0);
    assert!(linear.max_verification_to_execution_bps > 0);
}

#[test]
fn block_transition_preserves_matured_rewards_until_claim() {
    let beacon = hash_bytes(b"test", &[b"reward-block-transition-release"]);
    let params = ChainParams {
        epoch_length: 1,
        challenge_window_epochs: 1,
        ..ChainParams::default()
    };
    let mut producer = Chain::with_params(params.clone(), beacon);
    let proposer = address(b"reward-transition-proposer");
    producer
        .register_validator(proposer, producer.params().validator_min_stake)
        .unwrap();
    add_settled_receipt_for_blockspace(&mut producer, &beacon);

    let block0 = producer
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    assert!(producer.state().pending_proposer_rewards().is_empty());
    finalize_reward_test_block(&mut producer, &block0);
    let block0_claim = producer
        .state()
        .pending_proposer_rewards()
        .get(&block0.height)
        .unwrap()
        .clone();
    assert_eq!(
        block0_claim.claimable_at_height,
        block0
            .height
            .saturating_add(producer.params().proposer_reward_maturity_delay_blocks())
    );
    assert_eq!(producer.state().rewards().balance(&proposer), 0);

    let mut peer = Chain::with_params(params, beacon);
    peer.register_validator(proposer, peer.params().validator_min_stake)
        .unwrap();
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    peer.apply_command(ChainCommand::SubmitBlock(block0.clone()))
        .unwrap();
    finalize_reward_test_block(&mut peer, &block0);
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(peer.state().pending_proposer_rewards().contains_key(&0));

    add_settled_receipt_for_blockspace(&mut producer, &beacon);
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    let block1 = producer
        .produce_block_with_rewards(proposer, 1_012, 80, 20)
        .unwrap();
    assert_eq!(block1.reward_root, reward_root(producer.state()));
    finalize_reward_test_block(&mut producer, &block1);
    assert_eq!(producer.state().rewards().balance(&proposer), 0);
    assert!(producer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(
        producer
            .state()
            .pending_proposer_rewards()
            .get(&block1.height)
            .unwrap()
            .amount,
        100
    );

    peer.apply_command(ChainCommand::SubmitBlock(block1.clone()))
        .unwrap();
    finalize_reward_test_block(&mut peer, &block1);
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(peer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(peer.state(), producer.state());

    add_settled_receipt_for_blockspace(&mut producer, &beacon);
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    let block2 = producer
        .produce_block_with_rewards(proposer, 1_024, 80, 20)
        .unwrap();
    assert_eq!(block2.reward_root, reward_root(producer.state()));
    finalize_reward_test_block(&mut producer, &block2);
    assert_eq!(producer.state().rewards().balance(&proposer), 0);
    assert!(producer.state().pending_proposer_rewards().contains_key(&0));

    peer.apply_command(ChainCommand::SubmitBlock(block2.clone()))
        .unwrap();
    finalize_reward_test_block(&mut peer, &block2);
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(peer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(peer.state(), producer.state());

    let claim_events = producer
        .apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::ProposerRewardReleased {
        block_height: 0,
        proposer,
        amount: 500,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: proposer,
        amount: 500,
    }));
    assert_eq!(
        producer.state().accounts().get(&proposer).unwrap().balance,
        500
    );
    assert!(!producer.state().pending_proposer_rewards().contains_key(&0));
    peer.apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert_eq!(peer.state(), producer.state());

    add_settled_receipt_for_blockspace(&mut producer, &beacon);
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    let block3 = producer
        .produce_block_with_rewards(proposer, 1_036, 80, 20)
        .unwrap();
    assert_eq!(block3.reward_root, reward_root(producer.state()));
    finalize_reward_test_block(&mut producer, &block3);
    assert_eq!(producer.state().rewards().balance(&proposer), 0);
    assert!(!producer.state().pending_proposer_rewards().contains_key(&0));

    peer.apply_command(ChainCommand::SubmitBlock(block3.clone()))
        .unwrap();
    finalize_reward_test_block(&mut peer, &block3);
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(!peer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(peer.state(), producer.state());
}

#[test]
fn late_finalized_proposer_reward_materializes_as_delayed_claim_once() {
    let beacon = hash_bytes(b"test", &[b"late-finalized-reward-delay"]);
    let params = ChainParams {
        epoch_length: 1,
        reward_settlement_delay_epochs: 1,
        challenge_window_epochs: 1,
        proposer_reward_hold_epochs: 0,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"late-finalized-reward-proposer");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();
    add_settled_receipt_for_blockspace(&mut chain, &beacon);

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    assert!(chain.state().pending_proposer_rewards().is_empty());
    let claimable_at_height = block
        .height
        .saturating_add(chain.params().proposer_reward_maturity_delay_blocks());
    chain.set_position_for_testing(claimable_at_height.saturating_add(3), 0);

    finalize_reward_test_block(&mut chain, &block);
    let pending = chain
        .state()
        .pending_proposer_rewards()
        .get(&block.height)
        .expect("late finality must still materialize a delayed claim");
    assert_eq!(pending.claimable_at_height, claimable_at_height);
    assert_eq!(pending.amount, 500);
    assert_eq!(chain.state().rewards().balance(&proposer), 0);

    chain
        .apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert!(
        chain
            .state()
            .released_proposer_reward_blocks()
            .contains(&block.height)
    );
    assert!(
        !chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );

    crate::chain::blocks::materialize_finalized_proposer_rewards(
        &mut chain.state,
        &chain.blocks,
        &chain.params,
    );
    assert!(
        !chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );
}

#[test]
fn block_transition_preserves_matured_receipt_rewards_until_claim() {
    let beacon = hash_bytes(b"test", &[b"receipt-reward-block-transition-release"]);
    let params = ChainParams {
        agreement_quorum: 1,
        epoch_length: 1,
        challenge_window_epochs: 1,
        pow_timeout_blocks: 0,
        ..ChainParams::default()
    };
    let mut producer = Chain::with_params(params.clone(), beacon);
    let proposer = address(b"receipt-reward-transition-proposer");
    let miner = address(b"reward-blockspace-miner");
    producer
        .register_validator(proposer, producer.params().validator_min_stake)
        .unwrap();
    let receipt_id = add_settled_receipt_for_blockspace(&mut producer, &beacon);
    let claim_id = hash_bytes(b"test", &[b"receipt-reward-transition-claim"]);
    producer.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id,
        receipt_id,
        beneficiary: miner,
        amount: 1_000,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(0),
        voided_by_challenge: false,
    });

    let mut peer = Chain::with_params(params, beacon);
    peer.register_validator(proposer, peer.params().validator_min_stake)
        .unwrap();
    let peer_receipt_id = add_settled_receipt_for_blockspace(&mut peer, &beacon);
    assert_eq!(peer_receipt_id, receipt_id);
    peer.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id,
        receipt_id,
        beneficiary: miner,
        amount: 1_000,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(0),
        voided_by_challenge: false,
    });

    let block0 = producer
        .produce_block_with_rewards(proposer, 1_000, 80, 20)
        .unwrap();
    assert!(producer.state().included_receipts().contains(&receipt_id));
    let claimable_at_height = producer
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt_id && reward.beneficiary == miner)
        .unwrap()
        .claimable_at_height()
        .expect("receipt reward should have inclusion-derived maturity");
    assert_eq!(
        claimable_at_height,
        block0
            .height
            .saturating_add(producer.params().reward_maturity_delay_blocks())
    );
    assert_eq!(producer.state().rewards().balance(&miner), 0);

    peer.apply_command(ChainCommand::SubmitBlock(block0))
        .unwrap();
    assert_eq!(peer.state().rewards().balance(&miner), 0);
    assert_eq!(peer.state(), producer.state());

    while producer.state().height() <= claimable_at_height {
        add_settled_receipt_for_blockspace(&mut producer, &beacon);
        add_settled_receipt_for_blockspace(&mut peer, &beacon);
        let timestamp = producer.blocks().last().map_or(1_012, |block| {
            block
                .timestamp
                .saturating_add(producer.params().block_time_seconds.max(1))
        });
        let block = producer
            .produce_block_with_rewards(proposer, timestamp, 80, 20)
            .unwrap();
        peer.apply_command(ChainCommand::SubmitBlock(block))
            .unwrap();
    }

    assert_eq!(producer.state().rewards().balance(&miner), 0);
    assert!(
        producer
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt_id)
    );
    let claim_events = producer
        .apply_command(ChainCommand::ClaimReward(miner))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::ReceiptRewardReleased {
        claim_id,
        receipt_id,
        beneficiary: miner,
        amount: 1_000,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: miner,
        amount: 1_000,
    }));
    assert_eq!(
        producer.state().accounts().get(&miner).unwrap().balance,
        1_000
    );
    assert!(
        producer
            .state()
            .pending_receipt_rewards()
            .values()
            .all(|reward| reward.receipt_id != receipt_id)
    );
    assert_eq!(peer.state().rewards().balance(&miner), 0);
    peer.apply_command(ChainCommand::ClaimReward(miner))
        .unwrap();
    assert_eq!(peer.state(), producer.state());
}

#[test]
fn block_transition_preserves_matured_challenge_rewards_until_claim() {
    let beacon = hash_bytes(b"test", &[b"challenge-reward-block-transition-release"]);
    let params = ChainParams {
        epoch_length: 1,
        challenge_window_epochs: 1,
        pow_timeout_blocks: 0,
        ..ChainParams::default()
    };
    let mut producer = Chain::with_params(params.clone(), beacon);
    let proposer = address(b"challenge-reward-transition-proposer");
    let challenger = address(b"challenge-reward-transition-challenger");
    producer
        .register_validator(proposer, producer.params().validator_min_stake)
        .unwrap();
    let receipt_id = add_settled_receipt_for_blockspace(&mut producer, &beacon);
    let challenge_id = hash_bytes(b"test", &[b"challenge-reward-transition-challenge"]);
    let claim_id = hash_bytes(b"test", &[b"challenge-reward-transition-claim"]);
    producer.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
        claim_id,
        challenge_id,
        block_hash: [0; 32],
        receipt_id,
        challenger,
        amount: 250,
        claimable_at_height: 0,
        voided_by_challenge: false,
    });

    let mut peer = Chain::with_params(params, beacon);
    peer.register_validator(proposer, peer.params().validator_min_stake)
        .unwrap();
    let peer_receipt_id = add_settled_receipt_for_blockspace(&mut peer, &beacon);
    assert_eq!(peer_receipt_id, receipt_id);
    peer.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
        claim_id,
        challenge_id,
        block_hash: [0; 32],
        receipt_id,
        challenger,
        amount: 250,
        claimable_at_height: 0,
        voided_by_challenge: false,
    });

    let release_events = producer
        .apply_command(ChainCommand::ReleaseMaturedChallengeRewards)
        .unwrap();
    assert!(release_events.is_empty());
    assert!(
        producer
            .state()
            .pending_challenge_rewards()
            .contains_key(&claim_id)
    );
    assert_eq!(producer.state().rewards().balance(&challenger), 0);

    let block0 = producer
        .produce_block_with_rewards(proposer, 1_000, 80, 20)
        .unwrap();
    assert_eq!(block0.reward_root, reward_root(producer.state()));
    finalize_reward_test_block(&mut producer, &block0);
    assert!(
        producer
            .state()
            .pending_challenge_rewards()
            .contains_key(&claim_id)
    );
    assert_eq!(producer.state().rewards().balance(&challenger), 0);

    peer.apply_command(ChainCommand::SubmitBlock(block0.clone()))
        .unwrap();
    finalize_reward_test_block(&mut peer, &block0);
    assert_eq!(peer.state().rewards().balance(&challenger), 0);
    assert_eq!(peer.state(), producer.state());

    let claim_events = producer
        .apply_command(ChainCommand::ClaimReward(challenger))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::ChallengeRewardReleased {
        claim_id,
        challenge_id,
        challenger,
        amount: 250,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: challenger,
        amount: 250,
    }));
    assert_eq!(
        producer
            .state()
            .accounts()
            .get(&challenger)
            .unwrap()
            .balance,
        250
    );
    assert!(
        !producer
            .state()
            .pending_challenge_rewards()
            .contains_key(&claim_id)
    );
    assert_eq!(producer.state().rewards().balance(&challenger), 0);

    peer.apply_command(ChainCommand::ClaimReward(challenger))
        .unwrap();
    assert_eq!(peer.state(), producer.state());
}

#[test]
fn release_matured_proposer_rewards_sweeps_voided_claims_without_credit() {
    let beacon = hash_bytes(b"test", &[b"reward-voided-proposer-sweep"]);
    let params = ChainParams {
        epoch_length: 1,
        challenge_window_epochs: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"voided-proposer-sweep");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    finalize_reward_test_block(&mut chain, &block);
    chain
        .state
        .pending_proposer_rewards
        .get_mut(&block.height)
        .unwrap()
        .voided_by_challenge = true;
    chain.set_position_for_testing(chain.params().proposer_reward_maturity_delay_blocks(), 1);

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(!chain.state().pending_proposer_rewards().contains_key(&0));
}

#[test]
fn automatic_matured_reward_prune_removes_only_auto_prunable_receipt_claims() {
    let beacon = hash_bytes(b"test", &[b"reward-voided-receipt-auto-prune"]);
    let mut chain = Chain::new(beacon);
    let beneficiary = address(b"reward-voided-receipt-auto-prune-beneficiary");
    let live_receipt_id = hash_bytes(b"test", &[b"reward-live-receipt"]);
    let voided_miner_receipt_id = hash_bytes(b"test", &[b"reward-voided-miner-receipt"]);
    let voided_validator_receipt_id = hash_bytes(b"test", &[b"reward-voided-validator-receipt"]);
    let live_claim_id = hash_bytes(b"test", &[b"reward-live-receipt-claim"]);
    let voided_miner_claim_id = hash_bytes(b"test", &[b"reward-voided-miner-receipt-claim"]);
    let voided_validator_claim_id =
        hash_bytes(b"test", &[b"reward-voided-validator-receipt-claim"]);

    chain.set_position_for_testing(20, 0);
    chain.state.included_receipts.insert(live_receipt_id);
    chain
        .state
        .included_receipts
        .insert(voided_miner_receipt_id);
    chain
        .state
        .included_receipts
        .insert(voided_validator_receipt_id);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: live_claim_id,
        receipt_id: live_receipt_id,
        beneficiary,
        amount: 13,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(5),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: voided_miner_claim_id,
        receipt_id: voided_miner_receipt_id,
        beneficiary,
        amount: 17,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(5),
        voided_by_challenge: true,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: voided_validator_claim_id,
        receipt_id: voided_validator_receipt_id,
        beneficiary,
        amount: 19,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::AwaitingValidatorVrfReveal(5),
        voided_by_challenge: true,
    });

    let events = crate::chain::commands::release_all_matured_rewards(&mut chain.state);
    assert!(events.is_empty());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&live_claim_id),
        "live matured verifier-dependent rewards stay pending until ClaimReward"
    );
    assert!(
        !chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&voided_miner_claim_id),
        "voided matured miner receipt rewards are pruned without credit"
    );
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&voided_validator_claim_id),
        "voided validator receipt rewards keep their appeal-aware explicit release path"
    );
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
}

#[test]
fn automatic_matured_reward_prune_removes_pre_inclusion_voided_claims_after_hold() {
    let beacon = hash_bytes(b"test", &[b"reward-pre-inclusion-voided-prune"]);
    let mut chain = Chain::new(beacon);
    let beneficiary = address(b"reward-pre-inclusion-voided-beneficiary");
    let live_receipt_id = hash_bytes(b"test", &[b"reward-pre-inclusion-live-receipt"]);
    let voided_miner_receipt_id =
        hash_bytes(b"test", &[b"reward-pre-inclusion-voided-miner-receipt"]);
    let unavailable_receipt_id =
        hash_bytes(b"test", &[b"reward-pre-inclusion-unavailable-receipt"]);
    let live_claim_id = hash_bytes(b"test", &[b"reward-pre-inclusion-live-claim"]);
    let voided_miner_claim_id = hash_bytes(b"test", &[b"reward-pre-inclusion-voided-miner-claim"]);
    let unavailable_validator_claim_id = hash_bytes(
        b"test",
        &[b"reward-pre-inclusion-unavailable-validator-claim"],
    );

    chain.set_position_for_testing(20, 0);
    chain
        .state
        .data_unavailable_receipts
        .insert(unavailable_receipt_id);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: live_claim_id,
        receipt_id: live_receipt_id,
        beneficiary,
        amount: 13,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::AwaitingInclusionUntil(5),
        voided_by_challenge: false,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: voided_miner_claim_id,
        receipt_id: voided_miner_receipt_id,
        beneficiary,
        amount: 17,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::AwaitingInclusionUntil(5),
        voided_by_challenge: true,
    });
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: unavailable_validator_claim_id,
        receipt_id: unavailable_receipt_id,
        beneficiary,
        amount: 19,
        kind: ReceiptRewardKind::Validator,
        maturity: ReceiptRewardMaturity::AwaitingInclusionUntil(5),
        voided_by_challenge: true,
    });

    let events = crate::chain::commands::release_all_matured_rewards(&mut chain.state);
    assert!(events.is_empty());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&live_claim_id),
        "un-included live rewards stay pending and cannot be swept"
    );
    assert!(
        !chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&voided_miner_claim_id),
        "pre-inclusion voided miner rewards prune after their explicit hold"
    );
    assert!(
        !chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&unavailable_validator_claim_id),
        "pre-inclusion unavailable-data rewards prune after their explicit hold"
    );
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
}

#[test]
fn reward_release_commands_preserve_live_matured_claims_until_beneficiary_claim() {
    let beacon = hash_bytes(b"test", &[b"reward-claim-boundary"]);
    let mut chain = Chain::new(beacon);
    let beneficiary = address(b"reward-claim-boundary-beneficiary");
    let receipt_id = hash_bytes(b"test", &[b"reward-claim-boundary-receipt"]);
    let receipt_claim = hash_bytes(b"test", &[b"reward-claim-boundary-receipt-claim"]);
    let challenge_id = hash_bytes(b"test", &[b"reward-claim-boundary-challenge"]);
    let challenge_claim = hash_bytes(b"test", &[b"reward-claim-boundary-challenge-claim"]);
    let credit_claim = hash_bytes(b"test", &[b"reward-claim-boundary-credit-claim"]);

    chain.set_position_for_testing(20, 0);
    chain.state.pending_proposer_rewards.insert(
        3,
        PendingProposerReward {
            block_height: 3,
            proposer: beneficiary,
            amount: 11,
            claimable_at_height: 5,
            voided_by_challenge: false,
        },
    );
    chain.state.included_receipts.insert(receipt_id);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: receipt_claim,
        receipt_id,
        beneficiary,
        amount: 13,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(5),
        voided_by_challenge: false,
    });
    chain.insert_pending_challenge_reward_for_testing(PendingChallengeReward {
        claim_id: challenge_claim,
        challenge_id,
        block_hash: hash_bytes(b"test", &[b"reward-claim-boundary-block"]),
        receipt_id,
        challenger: beneficiary,
        amount: 17,
        claimable_at_height: 5,
        voided_by_challenge: false,
    });
    chain.state.pending_credit_rewards.insert(
        credit_claim,
        PendingCreditReward {
            claim_id: credit_claim,
            beneficiary,
            amount: 19,
            claimable_at_height: 5,
        },
    );

    assert!(
        chain
            .apply_command(ChainCommand::ReleaseMaturedProposerRewards)
            .unwrap()
            .is_empty()
    );
    assert!(
        chain
            .apply_command(ChainCommand::ReleaseMaturedReceiptRewards)
            .unwrap()
            .is_empty()
    );
    assert!(
        chain
            .apply_command(ChainCommand::ReleaseMaturedChallengeRewards)
            .unwrap()
            .is_empty()
    );
    assert!(
        chain
            .apply_command(ChainCommand::ReleaseMaturedCreditRewards)
            .unwrap()
            .is_empty()
    );
    assert!(chain.state().pending_proposer_rewards().contains_key(&3));
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .contains_key(&receipt_claim)
    );
    assert!(
        chain
            .state()
            .pending_challenge_rewards()
            .contains_key(&challenge_claim)
    );
    assert!(
        chain
            .state()
            .pending_credit_rewards()
            .contains_key(&credit_claim)
    );
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
    assert_eq!(
        chain
            .state()
            .accounts()
            .get(&beneficiary)
            .map(|account| account.balance)
            .unwrap_or_default(),
        0
    );

    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(beneficiary))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::ProposerRewardReleased {
        block_height: 3,
        proposer: beneficiary,
        amount: 11,
    }));
    assert!(claim_events.contains(&ChainEvent::ReceiptRewardReleased {
        claim_id: receipt_claim,
        receipt_id,
        beneficiary,
        amount: 13,
    }));
    assert!(claim_events.contains(&ChainEvent::ChallengeRewardReleased {
        claim_id: challenge_claim,
        challenge_id,
        challenger: beneficiary,
        amount: 17,
    }));
    assert!(claim_events.contains(&ChainEvent::CreditRewardReleased {
        claim_id: credit_claim,
        beneficiary,
        amount: 19,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: beneficiary,
        amount: 60,
    }));
    assert!(chain.state().pending_proposer_rewards().is_empty());
    assert!(chain.state().pending_receipt_rewards().is_empty());
    assert!(chain.state().pending_challenge_rewards().is_empty());
    assert!(chain.state().pending_credit_rewards().is_empty());
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
    assert_eq!(
        chain.state().accounts().get(&beneficiary).unwrap().balance,
        60
    );
}

#[test]
fn fallback_proposer_reward_uses_explicit_maturity_delay() {
    let beacon = hash_bytes(b"test", &[b"fallback-reward-delay"]);
    let params = ChainParams {
        epoch_length: 1,
        reward_settlement_delay_epochs: 1,
        challenge_window_epochs: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"fallback-delay-proposer");
    chain
        .register_validator(proposer, chain.params().validator_min_stake)
        .unwrap();

    let fallback = chain
        .produce_block_with_rewards(proposer, 1_000, 40, 10)
        .unwrap();
    assert!(chain.state().pending_proposer_rewards().is_empty());
    finalize_reward_test_block(&mut chain, &fallback);
    let fallback_reward = chain
        .state()
        .pending_proposer_rewards()
        .get(&fallback.height)
        .unwrap();
    let fallback_claimable_at_height = fallback_reward.claimable_at_height;
    assert_eq!(fallback_reward.amount, 50);
    assert_eq!(
        fallback_reward.claimable_at_height,
        fallback
            .height
            .saturating_add(chain.params().proposer_reward_maturity_delay_blocks())
    );

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&fallback.height)
    );

    while chain.state().height() < fallback_claimable_at_height {
        let timestamp = chain.blocks().last().map_or(1_012, |block| {
            block.timestamp.saturating_add(
                chain
                    .params()
                    .pow_timeout_blocks
                    .max(1)
                    .saturating_mul(chain.params().block_time_seconds.max(1)),
            )
        });
        chain.produce_block(proposer, timestamp).unwrap();
    }
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&fallback.height)
    );
    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: proposer,
        amount: 50,
    }));
    assert!(
        !chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&fallback.height)
    );
}

#[test]
fn reward_block_production_failure_does_not_credit_proposer() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"unknown-reward-proposer");
    let rewards_before = chain.state().rewards().clone();

    assert_eq!(
        chain.produce_block_with_rewards(proposer, 1_000, 400, 100),
        Err(TvmError::UnknownValidator)
    );
    assert_eq!(chain.state().rewards(), &rewards_before);
    assert!(chain.blocks().is_empty());
}
