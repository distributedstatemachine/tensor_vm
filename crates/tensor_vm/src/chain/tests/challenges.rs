use super::*;
use crate::{
    challenge::{BlockCheckChallenge, BlockCheckChallengeInput},
    merkle::{build_proof, merkle_root},
    types::sign,
};

fn finalize_challenge_test_block(chain: &mut Chain, block: &TensorBlock) {
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
fn observed_block_check_challenge_records_evidence_without_punishing_canonical_proposer() {
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
    assert!(chain.state().pending_proposer_rewards().is_empty());
    finalize_challenge_test_block(&mut chain, &block);
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
    let diagnostic = chain
        .deterministic_bad_block_check_challenge(&block, challenger)
        .unwrap();
    assert_eq!(diagnostic.challenge.receipt_id, receipt.receipt_id);
    assert_ne!(diagnostic.observed_block.checks_root, block.checks_root);
    chain
        .install_diagnostic_observed_block(&diagnostic)
        .unwrap();
    assert!(
        chain
            .blocks()
            .iter()
            .any(|stored| stored.hash() == block.hash())
    );
    assert!(
        !chain
            .blocks()
            .iter()
            .any(|stored| stored.hash() == diagnostic.observed_block.hash())
    );

    let events = chain
        .submit_block_check_challenge(diagnostic.challenge.clone())
        .unwrap();

    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        ChainEvent::BlockCheckChallengeProven {
            proposer: event_proposer,
            challenger: event_challenger,
            proposer_reward_clawback: 0,
            challenger_reward: 0,
            ..
        } if *event_proposer == proposer && *event_challenger == challenger
    ));
    let challenge_id = crate::chain::challenges::block_check_challenge_id(
        &diagnostic.observed_block.hash(),
        &receipt.receipt_id,
    );
    let claimable_at_height = chain
        .state()
        .height()
        .saturating_add(chain.params().reward_maturity_delay_blocks());
    let block_hash = diagnostic.observed_block.hash();
    assert_eq!(block_hash, diagnostic.observed_block.hash());
    assert_eq!(
        chain
            .state()
            .block_check_challenges()
            .get(&challenge_id)
            .unwrap()
            .challenger,
        challenger
    );
    assert!(
        !chain
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
            .all(|reward| !reward.voided_by_challenge)
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    assert_eq!(chain.state().rewards().treasury(), 0);
    assert!(
        !chain
            .state()
            .challenged_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id)
    );
    assert_eq!(chain.state().proposer_penalty_until().get(&proposer), None);
    assert!(chain.proposer_challenge_throttle_ready(proposer));
    assert!(
        chain
            .release_matured_challenge_rewards()
            .unwrap()
            .is_empty()
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    chain.set_position_for_testing(claimable_at_height, 1);
    let release_events = chain.release_matured_challenge_rewards().unwrap();
    assert!(release_events.is_empty());
    assert!(chain.state().pending_challenge_rewards().is_empty());
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    assert!(
        chain
            .release_matured_challenge_rewards()
            .unwrap()
            .is_empty()
    );
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipt.receipt_id)
            .all(|reward| !reward.voided_by_challenge)
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
}

#[test]
fn canonical_block_check_challenge_materializes_and_delays_reward_in_chain() {
    let beacon = hash_bytes(b"test", &[b"canonical-block-check-delay-beacon"]);
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
    let miner = address(b"canonical-block-check-miner");
    let proposer = address(b"canonical-block-check-proposer");
    let challenger = address(b"canonical-block-check-watcher");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(proposer, 10_000).unwrap();
    chain.register_validator(challenger, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let good_block = chain
        .produce_block_with_rewards(proposer, 1_000, 900, 100)
        .unwrap();
    let parent_state = chain
        .block_parent_state_for_payload(&good_block.hash())
        .unwrap()
        .clone();
    let outcome = chain.block_apply_outcome(&good_block).unwrap();
    let opening = outcome.selected_openings.first().unwrap();
    let mut observed_leaves = outcome
        .selected_openings
        .iter()
        .map(|opening| opening.check_leaf)
        .collect::<Vec<_>>();
    let observed_check_leaf = hash_bytes(
        b"test",
        &[
            b"canonical-block-check-observed-leaf",
            &good_block.hash(),
            &opening.receipt_id,
        ],
    );
    observed_leaves[opening.check_leaf_index as usize] = observed_check_leaf;

    chain.pop_block_for_testing();
    let mut bad_block = good_block.clone();
    bad_block.checks_root = merkle_root(&observed_leaves);
    let bad_hash = bad_block.hash();
    bad_block.proposer_signature = sign(&bad_block.proposer, &bad_hash);
    bad_block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&bad_hash]);
    chain.push_block_for_testing(bad_block.clone());
    chain.set_block_parent_state_for_admission(bad_hash, parent_state);
    chain.set_block_selected_receipts_for_admission(bad_hash, outcome.selected_receipt_ids.clone());
    chain.state.finalized_blocks.insert(bad_hash);
    assert!(
        !chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&bad_block.height)
    );

    let challenge = BlockCheckChallenge::new(BlockCheckChallengeInput {
        challenger,
        block_hash: bad_hash,
        receipt_id: opening.receipt_id,
        expected_check_leaf: opening.check_leaf,
        observed_check_leaf,
        check_leaf_index: opening.check_leaf_index,
        check_leaf_proof: build_proof(&observed_leaves, opening.check_leaf_index).unwrap(),
        recomputed_checks_root: outcome.checks_root,
    });
    let events = chain.submit_block_check_challenge(challenge).unwrap();

    let challenger_reward = 500;
    assert!(events.iter().any(|event| matches!(
        event,
        ChainEvent::BlockCheckChallengeProven {
            block_hash,
            receipt_id,
            proposer: event_proposer,
            challenger: event_challenger,
            proposer_reward_clawback: 1_000,
            challenger_reward: event_reward,
            ..
        } if *block_hash == bad_hash
            && *receipt_id == receipt.receipt_id
            && *event_proposer == proposer
            && *event_challenger == challenger
            && *event_reward == challenger_reward
    )));
    let pending_proposer = chain
        .state()
        .pending_proposer_rewards()
        .get(&bad_block.height)
        .unwrap();
    assert!(pending_proposer.voided_by_challenge);
    assert_eq!(pending_proposer.amount, 1_000);
    let pending_challenge = chain
        .state()
        .pending_challenge_rewards()
        .values()
        .next()
        .unwrap();
    assert_eq!(pending_challenge.amount, challenger_reward);
    assert_eq!(pending_challenge.challenger, challenger);
    let challenge_claimable_at_height = pending_challenge.claimable_at_height;
    assert_eq!(
        challenge_claimable_at_height,
        chain
            .state()
            .height()
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);

    chain.set_position_for_testing(challenge_claimable_at_height, 1);
    assert!(
        chain
            .release_matured_challenge_rewards()
            .unwrap()
            .is_empty()
    );
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(challenger))
        .unwrap();
    assert!(claim_events.iter().any(|event| matches!(
        event,
        ChainEvent::ChallengeRewardReleased {
            challenger: event_challenger,
            amount,
            ..
        } if *event_challenger == challenger && *amount == challenger_reward
    )));
    assert_eq!(
        chain.state().accounts().get(&challenger).unwrap().balance,
        challenger_reward
    );
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
    assert!(chain.state().pending_proposer_rewards().is_empty());
    finalize_challenge_test_block(&mut chain, &block);
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
            .saturating_add(chain.params().proposer_reward_maturity_delay_blocks())
    );
    let claimable_at_height = pending.claimable_at_height;
    assert_eq!(chain.state().rewards().balance(&proposer), 0);

    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    chain.set_position_for_testing(claimable_at_height, 1);
    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .contains_key(&block.height)
    );
    let events = chain
        .apply_command(ChainCommand::ClaimReward(proposer))
        .unwrap();
    assert!(events.contains(&ChainEvent::ProposerRewardReleased {
        block_height: block.height,
        proposer,
        amount: 500,
    }));
    assert_eq!(chain.state().rewards().balance(&proposer), 0);
    assert_eq!(
        chain.state().accounts().get(&proposer).unwrap().balance,
        500
    );
    assert!(
        chain
            .state()
            .pending_proposer_rewards()
            .get(&block.height)
            .is_none()
    );
}

#[test]
fn diagnostic_block_check_challenge_uses_full_observed_check_tree() {
    let beacon = hash_bytes(b"test", &[b"multi-block-check-challenge-beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        challenge_window_epochs: 1,
        epoch_length: 4,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"multi-block-check-challenge-miner");
    let proposer = address(b"multi-block-check-challenge-proposer");
    let challenger = address(b"multi-block-check-challenge-watcher");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(proposer, 10_000).unwrap();
    chain.register_validator(challenger, 10_000).unwrap();

    for label in [
        b"first".as_slice(),
        b"second".as_slice(),
        b"third".as_slice(),
    ] {
        let job = MatmulJob::synthetic(0, label[0] as u64, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
        chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    }

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 900, 100)
        .unwrap();
    let outcome = chain.block_apply_outcome(&block).unwrap();
    assert_eq!(outcome.selected_openings.len(), 3);
    finalize_challenge_test_block(&mut chain, &block);

    let diagnostic = chain
        .deterministic_bad_block_check_challenge(&block, challenger)
        .unwrap();
    assert_ne!(diagnostic.observed_block.checks_root, block.checks_root);
    assert_eq!(
        diagnostic.challenge.check_leaf_proof.leaf_index,
        diagnostic.challenge.check_leaf_index
    );
    assert!(verify_proof(
        &diagnostic.observed_block.checks_root,
        diagnostic.challenge.observed_check_leaf,
        &diagnostic.challenge.check_leaf_proof,
    ));

    chain
        .install_diagnostic_observed_block(&diagnostic)
        .unwrap();
    let events = chain
        .submit_block_check_challenge(diagnostic.challenge.clone())
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        ChainEvent::BlockCheckChallengeProven {
            block_hash,
            receipt_id,
            challenger: event_challenger,
            proposer_reward_clawback: 0,
            challenger_reward: 0,
            ..
        } if *block_hash == diagnostic.challenge.block_hash
            && *receipt_id == diagnostic.challenge.receipt_id
            && *event_challenger == challenger
    )));
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, ChainEvent::ChallengeRewardPending { .. }))
    );
}
