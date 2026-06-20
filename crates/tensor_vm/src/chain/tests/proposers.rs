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
    chain.produce_block(validator_a, 1_000).unwrap();
    assert_ne!(seed_before, chain.validation_seed(&receipt, &validator_a));
    assert_eq!(chain.state().finalized_beacon_round(), 1);
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
