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
