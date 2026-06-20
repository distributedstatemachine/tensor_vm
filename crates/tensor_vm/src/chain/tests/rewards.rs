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
    assert_eq!(
        pending_reward.claimable_at_height,
        chain.state().height() + chain.params().reward_maturity_delay_blocks()
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    receipt.receipt_id
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
    assert_eq!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .amount,
        500
    );
    assert_eq!(block.reward_root, reward_root(chain.state()));
    assert_ne!(
        block.reward_root,
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
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    add_settled_receipt_for_blockspace(&mut chain, &beacon);
    chain.produce_block(proposer, 1_006).unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    let claimable_at_height = chain
        .state()
        .pending_proposer_rewards()
        .get(&block.height)
        .unwrap()
        .claimable_at_height;
    chain.set_position_for_testing(claimable_at_height, 1);
    chain.release_matured_proposer_rewards().unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 1_000);
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

    chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
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
        .claimable_at_height += 1;
    assert_ne!(full_root, reward_root(&changed_receipt));

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
    chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
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
            .cmp(&window[1].claimable_at_height)
            .then_with(|| window[0].ledger.cmp(&window[1].ledger))
            .then_with(|| window[0].claim_id.cmp(&window[1].claim_id))
            != std::cmp::Ordering::Greater
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Proposer
            && claim.claim_id == RewardClaimKey::BlockHeight(0)
            && claim.subject_id == RewardClaimKey::BlockHeight(0)
            && claim.beneficiary == proposer
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptMiner
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.amount > 0
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::ReceiptValidator
            && claim.subject_id == RewardClaimKey::Hash(receipt_id)
            && claim.amount > 0
            && !claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Challenge
            && claim.subject_id
                == RewardClaimKey::Hash(hash_bytes(b"test", &[b"claim-view-challenge"]))
            && claim.related_id == Some(RewardClaimKey::Hash(receipt_id))
            && claim.beneficiary == challenger
            && claim.voided_by_challenge
    }));
    assert!(claims.iter().any(|claim| {
        claim.ledger == RewardClaimLedger::Credit
            && claim.beneficiary == credit_beneficiary
            && claim.amount == 25
            && !claim.voided_by_challenge
    }));
}

#[test]
fn block_transition_releases_matured_rewards_without_manual_command() {
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
            .saturating_add(producer.params().reward_maturity_delay_blocks())
    );
    assert_eq!(producer.state().rewards().balance(&proposer), 0);

    let mut peer = Chain::with_params(params, beacon);
    peer.register_validator(proposer, peer.params().validator_min_stake)
        .unwrap();
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    peer.apply_command(ChainCommand::SubmitBlock(block0))
        .unwrap();
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(peer.state().pending_proposer_rewards().contains_key(&0));

    add_settled_receipt_for_blockspace(&mut producer, &beacon);
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    let block1 = producer
        .produce_block_with_rewards(proposer, 1_001, 80, 20)
        .unwrap();
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
    assert_eq!(block1.reward_root, reward_root(producer.state()));

    peer.apply_command(ChainCommand::SubmitBlock(block1))
        .unwrap();
    assert_eq!(peer.state().rewards().balance(&proposer), 0);
    assert!(peer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(peer.state(), producer.state());

    add_settled_receipt_for_blockspace(&mut producer, &beacon);
    add_settled_receipt_for_blockspace(&mut peer, &beacon);
    let block2 = producer
        .produce_block_with_rewards(proposer, 1_002, 80, 20)
        .unwrap();
    assert_eq!(producer.state().rewards().balance(&proposer), 500);
    assert!(!producer.state().pending_proposer_rewards().contains_key(&0));
    assert_eq!(block2.reward_root, reward_root(producer.state()));

    peer.apply_command(ChainCommand::SubmitBlock(block2))
        .unwrap();
    assert_eq!(peer.state().rewards().balance(&proposer), 500);
    assert!(!peer.state().pending_proposer_rewards().contains_key(&0));
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
    chain
        .state
        .pending_proposer_rewards
        .get_mut(&block.height)
        .unwrap()
        .voided_by_challenge = true;
    chain.set_position_for_testing(2, 1);

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(!chain.state().pending_proposer_rewards().contains_key(&0));
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
    let fallback_reward = chain
        .state()
        .pending_proposer_rewards()
        .get(&fallback.height)
        .unwrap();
    assert_eq!(fallback_reward.amount, 50);
    assert_eq!(fallback_reward.claimable_at_height, 2);

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&fallback.height)
    );

    chain.produce_block(proposer, 1_006).unwrap();
    assert_eq!(chain.state().height(), 2);
    let events = chain.release_matured_proposer_rewards().unwrap();
    assert!(events.contains(&ChainEvent::ProposerRewardReleased {
        block_height: fallback.height,
        proposer,
        amount: 50,
    }));
    assert_eq!(chain.state().rewards().balance(&proposer), 50);
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
