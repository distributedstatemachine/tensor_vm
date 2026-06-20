use super::*;

fn resign_test_block(block: &mut TensorBlock) {
    let block_hash = block.hash();
    block.proposer_signature = sign(&block.proposer, &block_hash);
    block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
}

fn mine_test_block(block: &mut TensorBlock) {
    if block.production_kind.requires_pow() {
        while !block.pow_valid() {
            block.nonce = block.nonce.saturating_add(1);
        }
    }
    resign_test_block(block);
}

fn add_settled_test_receipt(chain: &mut Chain, beacon: &Hash, label: &[u8]) -> Hash {
    let miner = address(format!("{}-miner", String::from_utf8_lossy(label)).as_bytes());
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

fn useful_head_preference(left: &TensorBlock, right: &TensorBlock) -> std::cmp::Ordering {
    left.pow_hash()
        .cmp(&right.pow_hash())
        .then_with(|| left.hash().cmp(&right.hash()))
}

#[test]
fn blocks_advance_height_and_commit_state() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"proposer");
    chain.register_validator(proposer, 10_000).unwrap();
    let block = chain.produce_block(proposer, 1_000).unwrap();
    assert_eq!(block.height, 0);
    assert_eq!(chain.state().height(), 1);
    assert_eq!(chain.blocks().len(), 1);
}

#[test]
fn competing_useful_head_with_better_pow_replaces_unfinalized_head() {
    let beacon = hash_bytes(b"test", &[b"competing-useful-head"]);
    let mut parent = Chain::new(beacon);
    let validator_a = address(b"competing-useful-validator-a");
    let validator_b = address(b"competing-useful-validator-b");
    parent
        .register_validator(validator_a, parent.params().validator_min_stake)
        .unwrap();
    parent
        .register_validator(validator_b, parent.params().validator_min_stake)
        .unwrap();
    let receipt_id = add_settled_test_receipt(&mut parent, &beacon, b"competing-useful");

    let mut branch_a = parent.clone();
    let mut branch_b = parent.clone();
    let block_a = branch_a.produce_block(validator_a, 1_000).unwrap();
    let block_b = branch_b.produce_block(validator_b, 1_000).unwrap();
    assert_eq!(block_a.parent_hash, block_b.parent_hash);
    assert_eq!(
        block_a.production_kind,
        BlockProductionKind::UsefulVerificationPow
    );
    assert_eq!(
        block_b.production_kind,
        BlockProductionKind::UsefulVerificationPow
    );

    let (better, better_state, worse) = if useful_head_preference(&block_a, &block_b).is_lt() {
        (block_a.clone(), branch_a.state().clone(), block_b.clone())
    } else {
        (block_b.clone(), branch_b.state().clone(), block_a.clone())
    };

    let mut peer = parent;
    assert_eq!(
        peer.admit_block(worse.clone()).unwrap(),
        BlockAdmission::Applied {
            height: worse.height,
            hash: worse.hash()
        }
    );
    assert_eq!(
        peer.admit_block(better.clone()).unwrap(),
        BlockAdmission::Replaced {
            height: better.height,
            old_hash: worse.hash(),
            hash: better.hash()
        }
    );
    assert_eq!(
        peer.blocks().last().map(TensorBlock::hash),
        Some(better.hash())
    );
    assert_eq!(peer.state(), &better_state);
    assert_eq!(peer.selected_receipts_for_block(&better), vec![receipt_id]);
}

#[test]
fn competing_head_does_not_replace_finalized_head() {
    let beacon = hash_bytes(b"test", &[b"finalized-competing-head"]);
    let mut parent = Chain::new(beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("finalized-competing-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        parent
            .register_validator(*validator, parent.params().validator_min_stake)
            .unwrap();
    }
    add_settled_test_receipt(&mut parent, &beacon, b"finalized-competing");

    let mut branches = validators
        .iter()
        .take(2)
        .map(|validator| {
            let mut branch = parent.clone();
            branch.produce_block(*validator, 1_000).unwrap()
        })
        .collect::<Vec<_>>();
    branches.sort_by(useful_head_preference);
    let better = branches[0].clone();
    let worse = branches[1].clone();

    let mut peer = parent;
    peer.admit_block(worse.clone()).unwrap();
    peer.submit_block_vote(BlockVote::new(validators[0], 10_000, &worse))
        .unwrap();
    peer.submit_block_vote(BlockVote::new(validators[1], 10_000, &worse))
        .unwrap();
    assert!(peer.is_block_finalized(&worse.hash()));

    assert_eq!(
        peer.admit_block(better.clone()).unwrap(),
        BlockAdmission::Invalid {
            height: better.height,
            hash: better.hash(),
            reason: BlockInvalidReason::FinalizedConflict,
        }
    );
    assert_eq!(
        peer.blocks().last().map(TensorBlock::hash),
        Some(worse.hash())
    );
}

#[test]
fn competing_fallback_head_does_not_replace_accepted_fallback() {
    let beacon = hash_bytes(b"test", &[b"competing-fallback-head"]);
    let mut parent = Chain::new(beacon);
    let proposer = address(b"competing-fallback-validator");
    parent
        .register_validator(proposer, parent.params().validator_min_stake)
        .unwrap();

    let mut branch_a = parent.clone();
    let mut branch_b = parent.clone();
    let fallback_a = branch_a.produce_block(proposer, 1_000).unwrap();
    let fallback_b = branch_b.produce_block(proposer, 1_006).unwrap();
    assert_eq!(
        fallback_a.production_kind,
        BlockProductionKind::PowSkipFallback
    );
    assert_eq!(
        fallback_b.production_kind,
        BlockProductionKind::PowSkipFallback
    );

    let mut peer = parent;
    peer.admit_block(fallback_a.clone()).unwrap();
    assert_eq!(
        peer.admit_block(fallback_b.clone()).unwrap(),
        BlockAdmission::Invalid {
            height: fallback_b.height,
            hash: fallback_b.hash(),
            reason: BlockInvalidReason::NonPreferredCompetingHead,
        }
    );
    assert_eq!(
        peer.blocks().last().map(TensorBlock::hash),
        Some(fallback_a.hash())
    );
}

#[test]
fn produced_blocks_use_parent_finalized_beacon_not_own_hash() {
    let beacon = hash_bytes(b"test", &[b"parent-finalized-beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"parent-beacon-proposer");
    chain.register_validator(proposer, 10_000).unwrap();

    let block = chain.produce_block(proposer, 1_000).unwrap();

    assert_eq!(block.beacon_round, 0);
    assert_eq!(block.beacon, beacon);
    assert_ne!(block.beacon, block.hash());
    assert_ne!(block.beacon, block.pow_hash());
    assert_eq!(chain.state().finalized_beacon_round(), 1);
    assert_ne!(chain.state().finalized_randomness(), block.hash());
}

#[test]
fn block_validation_rejects_block_hash_beacon_randomness() {
    let beacon = hash_bytes(b"test", &[b"block-hash-beacon-ban"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"bad-beacon-proposer");
    chain.register_validator(proposer, 10_000).unwrap();
    let block = chain.produce_block(proposer, 1_000).unwrap();

    let mut bad_block = block.clone();
    bad_block.beacon = block.hash();
    mine_test_block(&mut bad_block);

    assert_eq!(
        chain.validate_block(&bad_block),
        Err(TvmError::InvalidReceipt("block beacon mismatch"))
    );
}

#[test]
fn block_finality_requires_two_thirds_validator_stake() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("finality-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
    let block = chain.produce_block(proposer, 1_000).unwrap();
    let block_hash = block.hash();

    assert!(!chain.has_block_finality(&block_hash));
    chain
        .submit_block_vote(BlockVote::new(validators[0], 10_000, &block))
        .unwrap();
    assert!(!chain.has_block_finality(&block_hash));
    chain
        .submit_block_vote(BlockVote::new(validators[1], 10_000, &block))
        .unwrap();

    assert!(chain.has_block_finality(&block_hash));
    assert!(chain.is_block_finalized(&block_hash));
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validators[1], 10_000, &block)),
        Err(TvmError::InvalidReceipt("duplicate block vote"))
    );
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validators[2], 1, &block)),
        Err(TvmError::InvalidReceipt("block vote stake mismatch"))
    );
}

#[test]
fn block_finality_ignores_invalid_direct_vote_records() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    assert!(!Chain::new(beacon).has_block_finality(&hash_bytes(b"test", &[b"no-stake"])));

    let mut chain = Chain::new(beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("invalid-finality-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
    let block = chain.produce_block(proposer, 1_000).unwrap();
    let block_hash = block.hash();

    let unknown = BlockVote::new(address(b"unknown-direct-validator"), 10_000, &block);
    let wrong_stake = BlockVote::new(validators[0], 1, &block);
    let valid = BlockVote::new(validators[0], 10_000, &block);
    let duplicate = BlockVote::new(validators[0], 10_000, &block);
    let mut bad_signature = BlockVote::new(validators[1], 10_000, &block);
    bad_signature.signature = [9; 32];
    chain.insert_block_votes_for_testing(
        block_hash,
        vec![unknown, wrong_stake, valid, duplicate, bad_signature],
    );

    assert!(!chain.has_block_finality(&block_hash));
    assert!(!chain.is_block_finalized(&block_hash));
}

#[test]
fn block_votes_reject_invalid_useful_pow_and_checks_root() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"block-validity-validator");
    chain.register_validator(validator, 10_000).unwrap();
    let block = chain.produce_block(validator, 1_000).unwrap();

    let mut bad_target = block.clone();
    bad_target.difficulty_target = [0; 32];
    resign_test_block(&mut bad_target);
    chain.push_block_for_testing(bad_target.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_target)),
        Err(TvmError::InvalidReceipt("block difficulty target mismatch"))
    );
    chain.pop_block_for_testing();

    let mut bad_checks = block.clone();
    bad_checks.checks_root = hash_bytes(b"test", &[b"bad-block-checks"]);
    mine_test_block(&mut bad_checks);
    chain.push_block_for_testing(bad_checks.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_checks)),
        Err(TvmError::InvalidReceipt("block checks root mismatch"))
    );
    chain.pop_block_for_testing();

    let mut bad_state_root = block.clone();
    bad_state_root.state_root = hash_bytes(b"test", &[b"bad-block-state-root"]);
    mine_test_block(&mut bad_state_root);
    chain.push_block_for_testing(bad_state_root.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_state_root)),
        Err(TvmError::InvalidReceipt("block state root mismatch"))
    );
    chain.pop_block_for_testing();

    let mut bad_receipts = block.clone();
    bad_receipts.settled_receipt_set_root = hash_bytes(b"test", &[b"bad-receipt-set"]);
    mine_test_block(&mut bad_receipts);
    chain.push_block_for_testing(bad_receipts.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_receipts)),
        Err(TvmError::InvalidReceipt("noncanonical settled receipt set"))
    );
}

#[test]
fn block_checks_root_is_bound_to_finalized_beacon_round() {
    let beacon = hash_bytes(b"test", &[b"checks-beacon-binding"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"checks-beacon-miner");
    let validator = address(b"checks-beacon-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    let block = chain.produce_block(validator, 1_000).unwrap();
    let wrong_beacon = block.hash();
    let wrong_checks_root = block_checks_root(
        &chain.selected_receipts_for_block(&block),
        chain.state().receipts(),
        chain.state().attestations(),
        block.beacon_round,
        &wrong_beacon,
        &block.parent_hash,
    );
    assert_ne!(block.checks_root, wrong_checks_root);

    let mut bad_block = block.clone();
    bad_block.checks_root = wrong_checks_root;
    mine_test_block(&mut bad_block);

    assert_eq!(
        chain.validate_block(&bad_block),
        Err(TvmError::InvalidReceipt("block checks root mismatch"))
    );
}

#[test]
fn produced_blocks_mark_selected_settled_receipts_included_once() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"included-receipt-miner");
    let validator = address(b"included-receipt-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let first = chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        chain.selected_receipts_for_block(&first),
        vec![receipt.receipt_id]
    );
    assert!(
        chain
            .state()
            .included_receipts()
            .contains(&receipt.receipt_id)
    );

    let second = chain.produce_block(validator, 2_000).unwrap();
    assert!(chain.selected_receipts_for_block(&second).is_empty());
    assert_eq!(second.production_kind, BlockProductionKind::PowSkipFallback);
    assert_eq!(
        second.settled_receipt_set_root,
        selected_receipt_root(&BTreeSet::new())
    );
}

#[test]
fn zero_receipt_parent_produces_explicit_fallback_block() {
    let beacon = hash_bytes(b"test", &[b"fallback-beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"fallback-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let block = chain.produce_block(validator, 1_000).unwrap();

    assert_eq!(block.production_kind, BlockProductionKind::PowSkipFallback);
    assert_eq!(block.nonce, 0);
    assert!(chain.selected_receipts_for_block(&block).is_empty());
    assert_eq!(chain.validate_block(&block), Ok(()));
}

#[test]
fn non_genesis_fallback_requires_pow_timeout() {
    let beacon = hash_bytes(b"test", &[b"fallback-timeout-beacon"]);
    let mut producer = Chain::new(beacon);
    let mut peer = Chain::new(beacon);
    let validator = address(b"fallback-timeout-validator");
    producer.register_validator(validator, 10_000).unwrap();
    peer.register_validator(validator, 10_000).unwrap();

    let genesis_fallback = producer.produce_block(validator, 1_000).unwrap();
    peer.apply_command(ChainCommand::SubmitBlock(genesis_fallback.clone()))
        .unwrap();

    assert_eq!(
        producer.produce_block(validator, 1_006),
        Err(TvmError::InvalidReceipt("fallback before pow timeout"))
    );

    let timed_fallback = producer.produce_block(validator, 1_012).unwrap();
    let mut early_payload = timed_fallback.clone();
    early_payload.timestamp = 1_006;
    resign_test_block(&mut early_payload);

    assert_eq!(
        peer.validate_block(&early_payload),
        Err(TvmError::InvalidReceipt("fallback before pow timeout"))
    );
    assert_eq!(
        peer.apply_command(ChainCommand::SubmitBlock(early_payload)),
        Err(TvmError::InvalidReceipt("fallback before pow timeout"))
    );
    assert_eq!(
        peer.apply_command(ChainCommand::SubmitBlock(timed_fallback)),
        Ok(vec![ChainEvent::BlockAccepted {
            height: 1,
            hash: producer.blocks().last().unwrap().hash()
        }])
    );
}

#[test]
fn fallback_blocks_require_stake_weighted_selected_validator() {
    let beacon = hash_bytes(b"test", &[b"fallback-selected-proposer"]);
    let mut producer = Chain::new(beacon);
    let mut peer = Chain::new(beacon);
    let validators: Vec<_> = (0..4)
        .map(|i| address(format!("fallback-selected-validator-{i}").as_bytes()))
        .collect();
    for (i, validator) in validators.iter().enumerate() {
        let stake = 10_000 + i as u64;
        producer.register_validator(*validator, stake).unwrap();
        peer.register_validator(*validator, stake).unwrap();
    }
    let selected = producer.proposer_for_next_epoch(&beacon).unwrap();
    let other = validators
        .iter()
        .copied()
        .find(|validator| *validator != selected)
        .unwrap();

    assert_eq!(
        producer.produce_block(other, 1_000),
        Err(TvmError::InvalidReceipt(
            "fallback proposer is not selected"
        ))
    );

    let block = producer.produce_block(selected, 1_000).unwrap();
    assert_eq!(block.production_kind, BlockProductionKind::PowSkipFallback);
    assert_eq!(block.proposer, selected);

    let mut bad_payload = block.clone();
    bad_payload.proposer = other;
    resign_test_block(&mut bad_payload);
    assert_eq!(
        peer.validate_block(&bad_payload),
        Err(TvmError::InvalidReceipt(
            "fallback proposer is not selected"
        ))
    );
    assert_eq!(
        peer.apply_command(ChainCommand::SubmitBlock(bad_payload)),
        Err(TvmError::InvalidReceipt(
            "fallback proposer is not selected"
        ))
    );
    assert!(peer.blocks().is_empty());
}

#[test]
fn useful_pow_blocks_do_not_require_fallback_selected_validator() {
    let beacon = hash_bytes(b"test", &[b"useful-open-proposer"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"useful-open-miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..4)
        .map(|i| address(format!("useful-open-validator-{i}").as_bytes()))
        .collect();
    for (i, validator) in validators.iter().enumerate() {
        chain
            .register_validator(*validator, 10_000 + i as u64)
            .unwrap();
    }
    let fallback_selected = chain.proposer_for_next_epoch(&beacon).unwrap();
    let useful_proposer = validators
        .iter()
        .copied()
        .find(|validator| *validator != fallback_selected)
        .unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let block = chain.produce_block(useful_proposer, 1_000).unwrap();

    assert_eq!(
        block.production_kind,
        BlockProductionKind::UsefulVerificationPow
    );
    assert_eq!(block.proposer, useful_proposer);
    assert_ne!(block.proposer, fallback_selected);
    assert_eq!(
        chain.selected_receipts_for_block(&block),
        vec![receipt.receipt_id]
    );
}

#[test]
fn produced_blocks_delay_receipt_rewards_from_inclusion_height() {
    let beacon = hash_bytes(b"test", &[b"delayed-receipt-reward-block"]);
    let params = ChainParams {
        agreement_quorum: 1,
        reward_settlement_delay_epochs: 1,
        challenge_window_epochs: 1,
        epoch_length: 5,
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
    let miner = address(b"delayed-receipt-reward-miner");
    let validator = address(b"delayed-receipt-reward-validator");
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
                checks_root: hash_bytes(b"test", &[b"delayed-reward-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    chain.settle_epoch(1_000, 500);
    let initial_claimable = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt.receipt_id)
        .unwrap()
        .claimable_at_height;
    chain.set_pending_receipt_reward_claimable_for_testing(receipt.receipt_id, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id)
    );

    let block = chain.produce_block(validator, 1_000).unwrap();
    assert!(
        chain
            .state()
            .included_receipts()
            .contains(&receipt.receipt_id)
    );
    let inclusion_delayed_claimable = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt.receipt_id)
        .unwrap()
        .claimable_at_height;
    assert_eq!(
        inclusion_delayed_claimable,
        block.height.saturating_add(
            chain
                .params()
                .reward_settlement_delay_epochs
                .saturating_add(chain.params().challenge_window_epochs)
                .saturating_mul(chain.params().epoch_length)
        )
    );
    assert_eq!(inclusion_delayed_claimable, initial_claimable);
    assert!(inclusion_delayed_claimable > 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    chain.set_position_for_testing(inclusion_delayed_claimable, 0);
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            receipt_id,
            beneficiary,
            ..
        } if *receipt_id == receipt.receipt_id && *beneficiary == miner
    )));
    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
}

#[test]
fn historical_parent_side_branch_is_stored_without_replacing_canonical_head() {
    let beacon = hash_bytes(b"test", &[b"historical-side-branch"]);
    let params = ChainParams {
        pow_timeout_blocks: 1,
        ..ChainParams::default()
    };
    let mut parent = Chain::with_params(params, beacon);
    let proposer = address(b"historical-side-branch-validator");
    parent
        .register_validator(proposer, parent.params().validator_min_stake)
        .unwrap();
    parent.produce_block(proposer, 1_000).unwrap();

    let mut canonical = parent.clone();
    let canonical_one = canonical.produce_block(proposer, 1_006).unwrap();
    let canonical_two = canonical.produce_block(proposer, 1_012).unwrap();
    let canonical_state = canonical.state().clone();

    let mut branch = parent.clone();
    let side_one = branch.produce_block(proposer, 1_007).unwrap();
    let side_two = branch.produce_block(proposer, 1_013).unwrap();
    assert_eq!(side_one.parent_hash, canonical_one.parent_hash);
    assert_ne!(side_one.hash(), canonical_one.hash());
    assert_eq!(side_two.parent_hash, side_one.hash());

    let mut peer = parent;
    assert_eq!(
        peer.admit_block(canonical_one.clone()).unwrap(),
        BlockAdmission::Applied {
            height: canonical_one.height,
            hash: canonical_one.hash(),
        }
    );
    assert_eq!(
        peer.admit_block(canonical_two.clone()).unwrap(),
        BlockAdmission::Applied {
            height: canonical_two.height,
            hash: canonical_two.hash(),
        }
    );
    assert_eq!(peer.state(), &canonical_state);

    assert_eq!(
        peer.admit_block(side_one.clone()).unwrap(),
        BlockAdmission::SideBranchStored {
            height: side_one.height,
            parent_hash: side_one.parent_hash,
            hash: side_one.hash(),
        }
    );
    assert_eq!(peer.state(), &canonical_state);
    assert_eq!(
        peer.blocks().last().map(TensorBlock::hash),
        Some(canonical_two.hash())
    );
    assert!(peer.side_branch_blocks().contains_key(&side_one.hash()));
    assert!(
        peer.side_branch_child_states()
            .contains_key(&side_one.hash())
    );

    assert_eq!(
        peer.admit_block(side_two.clone()).unwrap(),
        BlockAdmission::SideBranchStored {
            height: side_two.height,
            parent_hash: side_one.hash(),
            hash: side_two.hash(),
        }
    );
    assert_eq!(peer.state(), &canonical_state);
    assert!(peer.side_branch_blocks().contains_key(&side_two.hash()));
    assert!(
        peer.side_branch_child_states()
            .contains_key(&side_two.hash())
    );
}

#[test]
fn longer_side_branch_reorganizes_unfinalized_canonical_suffix() {
    let beacon = hash_bytes(b"test", &[b"side-branch-reorg"]);
    let params = ChainParams {
        pow_timeout_blocks: 1,
        ..ChainParams::default()
    };
    let mut parent = Chain::with_params(params, beacon);
    let proposer = address(b"side-branch-reorg-validator");
    parent
        .register_validator(proposer, parent.params().validator_min_stake)
        .unwrap();
    let base = parent.produce_block(proposer, 1_000).unwrap();

    let mut canonical = parent.clone();
    let canonical_one = canonical.produce_block(proposer, 1_006).unwrap();
    let canonical_two = canonical.produce_block(proposer, 1_012).unwrap();

    let mut branch = parent.clone();
    let side_one = branch.produce_block(proposer, 1_007).unwrap();
    let side_two = branch.produce_block(proposer, 1_013).unwrap();
    let side_three = branch.produce_block(proposer, 1_019).unwrap();
    let branch_state = branch.state().clone();
    assert_eq!(side_one.parent_hash, base.hash());
    assert_eq!(side_two.parent_hash, side_one.hash());
    assert_eq!(side_three.parent_hash, side_two.hash());

    let mut peer = parent;
    assert!(matches!(
        peer.admit_block(canonical_one.clone()).unwrap(),
        BlockAdmission::Applied { .. }
    ));
    assert!(matches!(
        peer.admit_block(canonical_two.clone()).unwrap(),
        BlockAdmission::Applied { .. }
    ));
    let old_head = canonical_two.hash();

    assert!(matches!(
        peer.admit_block(side_one.clone()).unwrap(),
        BlockAdmission::SideBranchStored { .. }
    ));
    assert!(matches!(
        peer.admit_block(side_two.clone()).unwrap(),
        BlockAdmission::SideBranchStored { .. }
    ));
    assert_eq!(
        peer.admit_block(side_three.clone()).unwrap(),
        BlockAdmission::Reorganized {
            height: side_three.height,
            old_head,
            hash: side_three.hash(),
        }
    );

    assert_eq!(peer.state(), &branch_state);
    assert_eq!(
        peer.blocks()
            .iter()
            .map(TensorBlock::hash)
            .collect::<Vec<_>>(),
        vec![
            base.hash(),
            side_one.hash(),
            side_two.hash(),
            side_three.hash()
        ]
    );
    assert!(
        peer.side_branch_blocks()
            .contains_key(&canonical_one.hash())
    );
    assert!(
        peer.side_branch_blocks()
            .contains_key(&canonical_two.hash())
    );
    assert!(
        peer.side_branch_child_states()
            .contains_key(&canonical_two.hash())
    );
    assert!(!peer.side_branch_blocks().contains_key(&side_three.hash()));
}

#[test]
fn side_branch_reorg_does_not_replace_finalized_canonical_suffix() {
    let beacon = hash_bytes(b"test", &[b"finalized-side-branch-reorg"]);
    let params = ChainParams {
        pow_timeout_blocks: 1,
        ..ChainParams::default()
    };
    let mut parent = Chain::with_params(params, beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("finalized-side-branch-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        parent
            .register_validator(*validator, parent.params().validator_min_stake)
            .unwrap();
    }
    let base_proposer = parent
        .proposer_for_next_epoch(&parent.state().finalized_randomness())
        .unwrap();
    let base = parent.produce_block(base_proposer, 1_000).unwrap();

    let mut canonical = parent.clone();
    let canonical_one_proposer = canonical
        .proposer_for_next_epoch(&canonical.state().finalized_randomness())
        .unwrap();
    let canonical_one = canonical
        .produce_block(canonical_one_proposer, 1_006)
        .unwrap();
    let mut branch = parent.clone();
    let side_one_proposer = branch
        .proposer_for_next_epoch(&branch.state().finalized_randomness())
        .unwrap();
    let side_one = branch.produce_block(side_one_proposer, 1_007).unwrap();
    let side_two_proposer = branch
        .proposer_for_next_epoch(&branch.state().finalized_randomness())
        .unwrap();
    let side_two = branch.produce_block(side_two_proposer, 1_013).unwrap();
    let side_three_proposer = branch
        .proposer_for_next_epoch(&branch.state().finalized_randomness())
        .unwrap();
    let side_three = branch.produce_block(side_three_proposer, 1_019).unwrap();
    assert_eq!(side_one.parent_hash, base.hash());

    let mut peer = parent;
    peer.admit_block(canonical_one.clone()).unwrap();
    peer.submit_block_vote(BlockVote::new(validators[0], 10_000, &canonical_one))
        .unwrap();
    peer.submit_block_vote(BlockVote::new(validators[1], 10_000, &canonical_one))
        .unwrap();
    assert!(peer.is_block_finalized(&canonical_one.hash()));
    let mut canonical_after_finality = peer.clone();
    let canonical_two_proposer = canonical_after_finality
        .proposer_for_next_epoch(&canonical_after_finality.state().finalized_randomness())
        .unwrap();
    let canonical_two = canonical_after_finality
        .produce_block(canonical_two_proposer, 1_012)
        .unwrap();
    peer.admit_block(canonical_two.clone()).unwrap();
    let canonical_state = peer.state().clone();

    assert_eq!(
        peer.admit_block(side_one.clone()).unwrap(),
        BlockAdmission::Invalid {
            height: side_one.height,
            hash: side_one.hash(),
            reason: BlockInvalidReason::FinalizedConflict,
        }
    );
    assert!(matches!(
        peer.admit_block(side_two.clone()).unwrap(),
        BlockAdmission::PendingParent { .. }
    ));
    assert!(matches!(
        peer.admit_block(side_three.clone()).unwrap(),
        BlockAdmission::PendingParent { .. }
    ));
    assert_eq!(peer.state(), &canonical_state);
    assert_eq!(
        peer.blocks().last().map(TensorBlock::hash),
        Some(canonical_two.hash())
    );
}

#[test]
fn block_kind_cannot_masquerade_across_empty_and_nonempty_blockspace() {
    let beacon = hash_bytes(b"test", &[b"kind-beacon"]);
    let mut empty_chain = Chain::new(beacon);
    let validator = address(b"kind-validator");
    empty_chain.register_validator(validator, 10_000).unwrap();
    let fallback = empty_chain.produce_block(validator, 1_000).unwrap();

    let mut bad_useful = fallback.clone();
    bad_useful.production_kind = BlockProductionKind::UsefulVerificationPow;
    mine_test_block(&mut bad_useful);
    assert_eq!(
        empty_chain.validate_block(&bad_useful),
        Err(TvmError::InvalidReceipt(
            "useful pow requires selected receipts"
        ))
    );

    let mut useful_chain = Chain::new(beacon);
    let miner = address(b"kind-miner");
    useful_chain.register_miner(miner, 100).unwrap();
    useful_chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    useful_chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    useful_chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    let useful = useful_chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        useful.production_kind,
        BlockProductionKind::UsefulVerificationPow
    );

    let mut bad_fallback = useful.clone();
    bad_fallback.production_kind = BlockProductionKind::PowSkipFallback;
    bad_fallback.nonce = 0;
    resign_test_block(&mut bad_fallback);
    assert_eq!(
        useful_chain.validate_block(&bad_fallback),
        Err(TvmError::InvalidReceipt(
            "fallback requires zero selected receipts"
        ))
    );
}

#[test]
fn uvpow_retarget_boundary_updates_target_with_bounded_adjustment() {
    let beacon = hash_bytes(b"test", &[b"retarget-beacon"]);
    let mut params = ChainParams {
        difficulty_retarget_epoch_length: 2,
        difficulty_target_block_time_seconds: 6,
        difficulty_retarget_max_ratio: 4,
        pow_timeout_blocks: 1,
        ..ChainParams::default()
    };
    params.difficulty_floor_target = [1; 32];
    params.difficulty_ceiling_target = [0xff; 32];
    let mut chain = Chain::with_params(params, beacon);
    let validator = address(b"retarget-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let first = chain.produce_block(validator, 1_000).unwrap();
    let second = chain.produce_block(validator, 1_006).unwrap();
    assert_eq!(first.difficulty_target, second.difficulty_target);
    let expected = chain.expected_difficulty_target(2);
    assert!(expected < second.difficulty_target);

    let third = chain.produce_block(validator, 1_012).unwrap();
    assert_eq!(third.difficulty_target, expected);
}

#[test]
fn uvpow_non_retarget_heights_reuse_parent_target() {
    let beacon = hash_bytes(b"test", &[b"non-retarget-beacon"]);
    let params = ChainParams {
        difficulty_retarget_epoch_length: 3,
        difficulty_target_block_time_seconds: 6,
        pow_timeout_blocks: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let validator = address(b"non-retarget-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let first = chain.produce_block(validator, 1_000).unwrap();
    let second = chain.produce_block(validator, 1_006).unwrap();

    assert_eq!(first.difficulty_target, second.difficulty_target);
    assert_eq!(
        chain.expected_difficulty_target(2),
        second.difficulty_target
    );
}

#[test]
fn block_roots_commit_to_canonical_receipts_checks_attestations_and_state_values() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"root-miner");
    let validator = address(b"root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
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

    chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    let parent_hash = chain
        .blocks()
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let expected_selection = chain.canonical_blockspace(
        &parent_hash,
        chain.state().finalized_beacon_round(),
        &chain.state().finalized_randomness(),
    );
    let expected_settled_receipt_set_root =
        selected_receipt_commitment_root(&expected_selection.receipt_ids, chain.state().receipts());
    let expected_checks_root = block_checks_root(
        &expected_selection.receipt_ids,
        chain.state().receipts(),
        chain.state().attestations(),
        chain.state().finalized_beacon_round(),
        &chain.state().finalized_randomness(),
        &parent_hash,
    );
    let expected_attestation_root = attestation_root(chain.state().attestations());
    let block = chain.produce_block(validator, 1_000).unwrap();
    let outcome = chain.block_apply_outcome(&block).unwrap();
    assert_eq!(
        block.settled_receipt_set_root,
        expected_settled_receipt_set_root
    );
    assert_eq!(block.checks_root, expected_checks_root);
    assert_eq!(block.attestation_root, expected_attestation_root);
    assert_eq!(block.state_root, outcome.child_state_root);
    assert_eq!(block.state_root, chain.state_root());
    assert!(block.pow_valid());

    let mut altered_miners = chain.state().miners().clone();
    altered_miners.get_mut(&miner).unwrap().stake += 1;
    assert_ne!(
        miner_root(chain.state().miners()),
        miner_root(&altered_miners)
    );

    let mut altered_receipts = chain.state().receipts().clone();
    match altered_receipts.get_mut(&receipt.receipt_id).unwrap() {
        ReceiptState::TensorOp(receipt) => receipt.execution_time_ms += 1,
        ReceiptState::LinearTrainingStep(_) | ReceiptState::GraphExecution(_) => {
            unreachable!("test inserts tensor op receipt")
        }
    }
    assert_ne!(
        receipt_root(chain.state().receipts()),
        receipt_root(&altered_receipts)
    );
}

#[test]
fn block_apply_outcome_exposes_parent_child_and_check_openings() {
    let beacon = hash_bytes(b"test", &[b"opening-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"opening-miner");
    let validator = address(b"opening-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 3, 3, 3, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"opening-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
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
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let parent_root = chain.state_root();
    let block = chain.produce_block(validator, 1_000).unwrap();
    let outcome = chain.block_apply_outcome(&block).unwrap();

    assert_eq!(outcome.parent_snapshot.state_root, parent_root);
    assert_eq!(outcome.child_state_root, block.state_root);
    assert_eq!(outcome.child_state_root, chain.state_root());
    assert_eq!(outcome.selected_receipt_ids, vec![receipt.receipt_id]);
    assert_eq!(outcome.selected_openings.len(), 1);
    let opening = &outcome.selected_openings[0];
    assert_eq!(opening.receipt_id, receipt.receipt_id);
    assert!(opening.settled);
    assert!(!opening.included_before_parent);
    assert!(opening.data_available);
    assert_eq!(opening.primitive_type, Some(PrimitiveType::TensorOp));
    assert_eq!(opening.tensor_work_units, receipt.tensor_work_units);
    assert!(
        opening
            .receipt_leaf_proof
            .as_ref()
            .is_some_and(|proof| verify_proof(
                &outcome.selected_receipt_root,
                opening.receipt_leaf,
                proof
            ))
    );
    assert!(
        opening
            .check_leaf_proof
            .as_ref()
            .is_some_and(|proof| verify_proof(&outcome.checks_root, opening.check_leaf, proof))
    );
}

#[test]
fn historical_block_apply_outcome_uses_stored_parent_snapshot_after_future_receipts() {
    let beacon = hash_bytes(b"test", &[b"historical-apply-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"historical-apply-miner");
    let validator = address(b"historical-apply-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let first_job = MatmulJob::synthetic(0, 0, 3, 3, 3, &beacon, 10);
    let (first_receipt, first_a, first_b, first_c) =
        TensorOpReceipt::from_job(&first_job, miner, 1, 5).unwrap();
    let first_report = verify_tensor_op(
        &first_job,
        &first_receipt,
        &first_a,
        &first_b,
        &first_c,
        &hash_bytes(b"test", &[b"historical-first-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(first_job.clone()));
    chain
        .submit_tensor_op_receipt(first_receipt.clone())
        .unwrap();
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: first_receipt.receipt_id,
                job_id: first_receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: first_report.result,
                checks_root: first_report.checks_root,
                data_availability_passed: first_report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.mark_receipt_settled_for_testing(first_receipt.receipt_id);

    let first_parent_root = chain.state_root();
    let first_block = chain.produce_block(validator, 1_000).unwrap();
    let first_child_root = first_block.state_root;

    let second_job = MatmulJob::synthetic(0, 1, 2, 2, 2, &beacon, 10);
    let (second_receipt, second_a, second_b, second_c) =
        TensorOpReceipt::from_job(&second_job, miner, 2, 5).unwrap();
    let second_report = verify_tensor_op(
        &second_job,
        &second_receipt,
        &second_a,
        &second_b,
        &second_c,
        &hash_bytes(b"test", &[b"historical-second-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(second_job.clone()));
    chain
        .submit_tensor_op_receipt(second_receipt.clone())
        .unwrap();
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: second_receipt.receipt_id,
                job_id: second_receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: second_report.result,
                checks_root: second_report.checks_root,
                data_availability_passed: second_report.data_availability_passed,
            },
        ))
        .unwrap();
    chain.mark_receipt_settled_for_testing(second_receipt.receipt_id);
    let second_block = chain.produce_block(validator, 2_000).unwrap();

    let historical = chain.block_apply_outcome(&first_block).unwrap();

    assert_eq!(historical.parent_snapshot.state_root, first_parent_root);
    assert_eq!(historical.child_state_root, first_child_root);
    assert_eq!(
        historical.selected_receipt_ids,
        vec![first_receipt.receipt_id]
    );
    assert!(
        !historical
            .selected_receipt_ids
            .contains(&second_receipt.receipt_id)
    );
    assert_eq!(chain.validate_block(&first_block), Ok(()));
    assert_eq!(chain.validate_block(&second_block), Ok(()));
}

#[test]
fn block_validation_rejects_parent_root_disguised_as_child_state_root() {
    let beacon = hash_bytes(b"test", &[b"child-root-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"child-root-miner");
    let validator = address(b"child-root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);

    let parent_root = chain.state_root();
    let block = chain.produce_block(validator, 1_000).unwrap();
    let mut bad_block = block.clone();
    bad_block.state_root = parent_root;
    mine_test_block(&mut bad_block);

    assert_eq!(
        chain.validate_block(&bad_block),
        Err(TvmError::InvalidReceipt("block state root mismatch"))
    );
}
