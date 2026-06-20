use super::*;

#[test]
fn chain_params_define_tensor_retention_deadline() {
    let params = ChainParams {
        epoch_length: 50,
        reward_settlement_delay_epochs: 2,
        challenge_window_epochs: 3,
        ..ChainParams::default()
    };
    assert_eq!(params.tensor_retention_window_blocks(), 250);
    assert_eq!(params.tensor_retention_deadline(10), 260);
}

#[test]
fn audit_sampling_extends_reward_maturity_and_tensor_retention() {
    let inactive = ChainParams {
        epoch_length: 7,
        reward_settlement_delay_epochs: 0,
        challenge_window_epochs: 0,
        validator_audit_sample_numerator: 0,
        validator_audit_window_blocks: 11,
        ..ChainParams::default()
    };
    assert_eq!(inactive.validator_audit_reward_hold_blocks(), 0);
    assert_eq!(inactive.fraud_reward_hold_blocks(), 7);
    assert_eq!(inactive.reward_maturity_delay_blocks(), 7);
    assert_eq!(inactive.proposer_reward_hold_blocks(), 7);
    assert_eq!(inactive.proposer_reward_maturity_delay_blocks(), 14);
    assert_eq!(inactive.tensor_retention_window_blocks(), 0);

    let active = ChainParams {
        validator_audit_sample_numerator: 1,
        ..inactive
    };
    assert_eq!(active.validator_audit_reward_hold_blocks(), 11);
    assert_eq!(active.fraud_reward_hold_blocks(), 11);
    assert_eq!(active.reward_maturity_delay_blocks(), 11);
    assert_eq!(active.proposer_reward_maturity_delay_blocks(), 18);
    assert_eq!(active.tensor_retention_window_blocks(), 11);
    assert_eq!(active.tensor_retention_deadline(5), 16);
}
