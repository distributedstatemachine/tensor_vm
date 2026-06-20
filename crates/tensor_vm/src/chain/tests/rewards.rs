use super::*;

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
    assert_eq!(block.reward_root, reward_root(chain.state().rewards()));

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
    chain.set_position_for_testing(101, 1);
    chain.release_matured_proposer_rewards().unwrap();
    assert_eq!(chain.state().rewards().balance(&proposer), 1_000);
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
