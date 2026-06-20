use super::*;

fn resign_challenge_test_block(block: &mut TensorBlock) {
    let block_hash = block.hash();
    block.proposer_signature = sign(&block.proposer, &block_hash);
    block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
}

#[test]
fn challenge_outcome_slashes_miner_and_credits_treasury() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyChallengeOutcome(
                ChallengeOutcome::ProvenInvalid {
                    dishonest_party: miner,
                    slash_amount: 25,
                    reason: "invalid receipt".to_owned(),
                },
            ))
            .unwrap(),
        vec![ChainEvent::ChallengeProvenInvalid {
            dishonest_party: miner,
            slash_amount: 25,
            reason: "invalid receipt".to_owned(),
        }]
    );
    chain
        .apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
            dishonest_party: miner,
            slash_amount: 5,
            reason: "invalid receipt again".to_owned(),
        })
        .unwrap();
    assert_eq!(chain.state().miners().get(&miner).unwrap().stake, 70);
    assert_eq!(chain.state().miners().get(&miner).unwrap().reputation, -20);
    assert_eq!(chain.state().rewards().treasury(), 30);
}

#[test]
fn block_check_challenge_voids_pending_reward_and_throttles_proposer() {
    let beacon = hash_bytes(b"test", &[b"block-check-challenge-beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        challenge_window_epochs: 1,
        epoch_length: 4,
        freivalds: FreivaldsParams {
            minimum_validators: 1,
            validators_per_job: 1,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"block-check-challenge-miner");
    let proposer = address(b"block-check-challenge-proposer");
    let challenger = address(b"block-check-challenge-watcher");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(proposer, 10_000).unwrap();
    chain.register_validator(challenger, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"block-check-challenge-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assigned_validator = JobScheduler::default()
        .assign_validators(&chain, receipt.receipt_id, &assignment_seed)
        .validators
        .into_iter()
        .next()
        .unwrap();
    chain.insert_attestation_for_testing(ValidatorAttestation::new(
        assigned_validator,
        10_000,
        AttestationStatement {
            receipt_id: receipt.receipt_id,
            job_id: receipt.job_id,
            primitive_type: PrimitiveType::TensorOp,
            result: report.result,
            checks_root: report.checks_root,
            data_availability_passed: report.data_availability_passed,
        },
    ));
    chain.settle_epoch(1_000, 500);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id && !reward.voided_by_challenge)
    );

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 900, 100)
        .unwrap();
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
    let outcome = chain.block_apply_outcome(&block).unwrap();
    let opening = outcome.selected_openings.first().unwrap();

    let observed_check_leaf = hash_bytes(b"test", &[b"bad-observed-check-leaf"]);
    let observed_root = merkle_root(&[observed_check_leaf]);
    let mut bad_block = block.clone();
    bad_block.checks_root = observed_root;
    resign_challenge_test_block(&mut bad_block);
    chain.pop_block_for_testing();
    chain.push_block_for_testing(bad_block.clone());
    chain
        .state
        .block_selected_receipts
        .insert(bad_block.hash(), vec![receipt.receipt_id]);

    let challenge =
        crate::challenge::BlockCheckChallenge::new(crate::challenge::BlockCheckChallengeInput {
            challenger,
            block_hash: bad_block.hash(),
            receipt_id: receipt.receipt_id,
            expected_check_leaf: opening.check_leaf,
            observed_check_leaf,
            check_leaf_index: opening.check_leaf_index,
            check_leaf_proof: build_proof(&[observed_check_leaf], 0).unwrap(),
            recomputed_checks_root: outcome.checks_root,
        });
    let events = chain.submit_block_check_challenge(challenge).unwrap();

    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        ChainEvent::BlockCheckChallengeProven {
            proposer: event_proposer,
            challenger: event_challenger,
            proposer_reward_clawback: 1000,
            challenger_reward: 500,
            ..
        } if *event_proposer == proposer && *event_challenger == challenger
    ));
    let ChainEvent::ChallengeRewardPending {
        claim_id,
        challenge_id,
        block_hash,
        receipt_id,
        challenger: pending_challenger,
        amount,
        claimable_at_height,
    } = events[1]
    else {
        panic!("expected pending challenge reward event");
    };
    assert_eq!(block_hash, bad_block.hash());
    assert_eq!(receipt_id, receipt.receipt_id);
    assert_eq!(pending_challenger, challenger);
    assert_eq!(amount, 500);
    assert_eq!(claimable_at_height, 5);
    assert_eq!(
        chain
            .state()
            .block_check_challenges()
            .get(&challenge_id)
            .unwrap()
            .challenger,
        challenger
    );
    assert!(matches!(
        chain.state().pending_challenge_rewards().get(&claim_id),
        Some(reward)
            if reward.challenge_id == challenge_id
                && reward.block_hash == bad_block.hash()
                && reward.receipt_id == receipt.receipt_id
                && reward.challenger == challenger
                && reward.amount == 500
                && reward.claimable_at_height == 5
                && !reward.voided_by_challenge
    ));
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .unwrap()
            .voided_by_challenge
    );
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipt.receipt_id)
            .all(|reward| reward.voided_by_challenge)
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    assert_eq!(chain.state().rewards().treasury(), 500);
    assert!(
        chain
            .state()
            .challenged_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        !chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id)
    );
    assert_eq!(
        chain.state().proposer_penalty_until().get(&proposer),
        Some(&5)
    );
    assert_eq!(
        chain.produce_block(proposer, 1_006),
        Err(TvmError::InvalidReceipt("proposer is challenge-throttled"))
    );
    assert!(
        chain
            .release_matured_challenge_rewards()
            .unwrap()
            .is_empty()
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    chain.set_position_for_testing(5, 1);
    let release_events = chain.release_matured_challenge_rewards().unwrap();
    assert!(
        release_events.contains(&ChainEvent::ChallengeRewardReleased {
            claim_id,
            challenge_id,
            challenger,
            amount: 500,
        })
    );
    assert!(release_events.contains(&ChainEvent::RewardCredited {
        address: challenger,
        amount: 500,
    }));
    assert_eq!(chain.state().rewards().balance(&challenger), 500);
    assert!(
        chain
            .state()
            .pending_challenge_rewards()
            .get(&claim_id)
            .is_none()
    );
    assert!(
        chain
            .release_matured_challenge_rewards()
            .unwrap()
            .is_empty()
    );
    chain.set_position_for_testing(100, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .all(|reward| reward.receipt_id != receipt.receipt_id)
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
}

#[test]
fn matured_proposer_reward_releases_after_full_maturity_delay() {
    let beacon = hash_bytes(b"test", &[b"pending-proposer-reward"]);
    let params = ChainParams {
        challenge_window_epochs: 1,
        epoch_length: 2,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let proposer = address(b"pending-proposer");
    let miner = address(b"pending-proposer-miner");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(proposer, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    let pending = chain
        .state()
        .pending_proposer_rewards()
        .get(&block.height)
        .unwrap();
    assert_eq!(pending.amount, 500);
    assert_eq!(
        pending.claimable_at_height,
        block
            .height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    let claimable_at_height = pending.claimable_at_height;
    assert_eq!(chain.state().rewards().balance(&proposer), 0);

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    chain.set_position_for_testing(claimable_at_height, 1);
    let events = chain.release_matured_proposer_rewards().unwrap();
    assert!(events.contains(&ChainEvent::ProposerRewardReleased {
        block_height: block.height,
        proposer,
        amount: 500,
    }));
    assert_eq!(chain.state().rewards().balance(&proposer), 500);
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .is_none()
    );
}
