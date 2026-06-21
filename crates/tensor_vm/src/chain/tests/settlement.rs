use super::*;
use crate::canonical_matmul_graph;
use crate::jobs::{GraphJob, GraphReceipt};
use crate::verify::verify_graph_execution;
use std::collections::BTreeMap;

#[test]
fn chain_settles_valid_tensorwork_and_rewards_participants() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    let validators: Vec<_> = (0..5)
        .map(|i| address(format!("validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }

    let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
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
    for validator in &validators {
        chain
            .submit_attestation(ValidatorAttestation::new(
                *validator,
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
    }

    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        receipt.tensor_work_units
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    let pending_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == miner)
        .unwrap();
    assert!(pending_claim.awaiting_inclusion());
    assert_eq!(pending_claim.claimable_at_height(), None);
    assert_eq!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.beneficiary == miner)
            .count(),
        1
    );
    let pending_validator_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == validators[0])
        .unwrap()
        .amount;
    assert!(pending_validator_reward > 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id)
    );

    let block = chain.produce_block(validators[0], 1_000).unwrap();
    assert!(
        chain
            .state()
            .included_receipts()
            .contains(&receipt.receipt_id)
    );
    let claimable_at_height = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == miner)
        .unwrap()
        .claimable_at_height()
        .expect("receipt reward should have inclusion-derived maturity");
    assert_eq!(
        claimable_at_height,
        block
            .height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    chain.set_position_for_testing(claimable_at_height, 0);
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            beneficiary,
            amount: 1_000,
            ..
        } if *beneficiary == miner
    )));
    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        receipt.tensor_work_units
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        0
    );
    let validator_reward = chain.state().rewards().balance(&validators[0]);
    assert!(validator_reward > 0);
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
    assert_eq!(
        chain.state().rewards().balance(&validators[0]),
        validator_reward
    );
}

#[test]
fn chain_settles_valid_graph_execution_and_delays_rewards() {
    let beacon = hash_bytes(b"test", &[b"graph-settlement"]);
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
    let miner = address(b"graph-settlement-miner");
    let validator = address(b"graph-settlement-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
    let inputs = BTreeMap::from([("a".to_owned(), a.clone()), ("b".to_owned(), b.clone())]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);
    let (receipt, _outputs) =
        GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
    let report = verify_graph_execution(
        &job,
        &receipt,
        &graph,
        &inputs,
        &hash_bytes(b"test", &[b"graph-validation"]),
    )
    .unwrap();

    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
            job.clone(),
        )))
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::GraphExecution,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();

    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        receipt.tensor_work_units
    );
    let pending_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == miner)
        .unwrap();
    assert!(pending_claim.awaiting_inclusion());
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);

    let block = chain.produce_block(validator, 1_000).unwrap();
    let inclusion_claimable_at_height = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.beneficiary == miner)
        .unwrap()
        .claimable_at_height()
        .expect("receipt reward should have inclusion-derived maturity");
    assert_eq!(
        inclusion_claimable_at_height,
        block
            .height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    chain.set_position_for_testing(inclusion_claimable_at_height, 0);
    chain.release_matured_receipt_rewards().unwrap();

    assert_eq!(chain.state().rewards().balance(&miner), 1_000);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        receipt.tensor_work_units
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        0
    );
}

#[test]
fn miner_rewards_delay_tensorwork_activation_until_reward_release() {
    let beacon = hash_bytes(b"test", &[b"reward-tensorwork-delay"]);
    let params = ChainParams {
        agreement_quorum: 1,
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
    let dominant = address(b"dominant-tensorwork-miner");
    let minority = address(b"minority-tensorwork-miner");
    let validator = address(b"reward-curve-validator");
    chain.register_miner(dominant, 100).unwrap();
    chain.register_miner(minority, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let graph = canonical_matmul_graph(1, 1, 1, DType::FieldElement);
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let a = Tensor::from_vec(vec![1, 1], DType::FieldElement, vec![3]).unwrap();
    let b = Tensor::from_vec(vec![1, 1], DType::FieldElement, vec![5]).unwrap();
    let inputs = BTreeMap::from([("a".to_owned(), a), ("b".to_owned(), b)]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect::<BTreeMap<_, _>>();

    let submit_graph_receipt =
        |chain: &mut Chain, miner: Address, epoch: u64, work: u64| -> GraphReceipt {
            let job = GraphJob::new(
                epoch,
                graph_id,
                input_roots.clone(),
                BTreeMap::new(),
                10,
                1,
                work,
            );
            let (receipt, _outputs) =
                GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
            chain
                .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
                    job.clone(),
                )))
                .unwrap();
            chain
                .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
                    receipt.clone(),
                )))
                .unwrap();
            chain
                .submit_attestation(ValidatorAttestation::new(
                    validator,
                    10_000,
                    AttestationStatement {
                        receipt_id: receipt.receipt_id,
                        job_id: receipt.job_id,
                        primitive_type: PrimitiveType::GraphExecution,
                        result: VerificationResult::Valid,
                        checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                        data_availability_passed: true,
                    },
                ))
                .unwrap();
            receipt
        };

    let dominant_receipt = submit_graph_receipt(&mut chain, dominant, 0, 10_000);
    let minority_receipt = submit_graph_receipt(&mut chain, minority, 1, 100);

    chain.settle_epoch(1_100, 0);

    let dominant_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == dominant_receipt.receipt_id)
        .unwrap()
        .amount;
    let minority_reward = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == minority_receipt.receipt_id)
        .unwrap()
        .amount;
    let dominant_reward_maturity = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == dominant_receipt.receipt_id)
        .unwrap()
        .maturity;

    assert_eq!(dominant_reward + minority_reward, 1_100);
    assert_eq!(dominant_reward, 1_089);
    assert_eq!(minority_reward, 11);
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&dominant)
            .unwrap()
            .settled_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&minority)
            .unwrap()
            .settled_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&dominant)
            .unwrap()
            .pending_tensor_work,
        10_000
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&minority)
            .unwrap()
            .pending_tensor_work,
        100
    );

    let block = chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        dominant_reward_maturity,
        ReceiptRewardMaturity::AwaitingInclusion
    );
    assert_eq!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .find(|reward| reward.receipt_id == dominant_receipt.receipt_id)
            .unwrap()
            .claimable_at_height()
            .expect("receipt reward should have inclusion-derived maturity"),
        block
            .height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    chain.set_position_for_testing(
        block
            .height
            .saturating_add(chain.params().reward_maturity_delay_blocks()),
        0,
    );
    chain.release_matured_receipt_rewards().unwrap();
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&dominant)
            .unwrap()
            .settled_tensor_work,
        10_000
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&minority)
            .unwrap()
            .settled_tensor_work,
        100
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&dominant)
            .unwrap()
            .pending_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&minority)
            .unwrap()
            .pending_tensor_work,
        0
    );
}

#[test]
fn receipt_rewards_use_minimum_reward_maturity_delay_when_epochs_are_zero() {
    let beacon = hash_bytes(b"test", &[b"zero-epoch-receipt-delay"]);
    let params = ChainParams {
        agreement_quorum: 1,
        epoch_length: 7,
        reward_settlement_delay_epochs: 0,
        challenge_window_epochs: 0,
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
    let miner = address(b"zero-epoch-delay-miner");
    let validator = address(b"zero-epoch-delay-validator");
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
                checks_root: hash_bytes(b"test", &[b"zero-epoch-delay-checks"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    chain.settle_epoch(1_000, 500);
    let pending_claim = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .find(|reward| reward.receipt_id == receipt.receipt_id && reward.beneficiary == miner)
        .unwrap();
    assert!(pending_claim.awaiting_inclusion());
    assert_eq!(chain.params().tensor_retention_window_blocks(), 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
}

#[test]
fn unavailable_data_evidence_voids_delayed_receipt_rewards_before_release() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 3,
            minimum_validators: 2,
            minimum_stake_numerator: 2,
            minimum_stake_denominator: 3,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"delayed-unavailable-miner");
    chain.register_miner(miner, 100).unwrap();
    for i in 0..8 {
        chain
            .register_validator(
                address(format!("delayed-unavailable-validator-{i}").as_bytes()),
                10_000,
            )
            .unwrap();
    }

    let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"delayed-unavailable-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assigned =
        JobScheduler::default().assign_validators(&chain, receipt.receipt_id, &assignment_seed);
    let validators = assigned.validators;
    assert_eq!(validators.len(), 3);
    for validator in validators.iter().take(2) {
        chain
            .submit_attestation(ValidatorAttestation::new(
                *validator,
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
    }
    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id && !reward.voided_by_challenge)
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        receipt.tensor_work_units
    );

    chain
        .submit_attestation(ValidatorAttestation::new(
            validators[2],
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Unavailable,
                checks_root: hash_bytes(b"test", &[b"delayed-unavailable"]),
                data_availability_passed: false,
            },
        ))
        .unwrap();

    let reward_hold_until_height = chain
        .state()
        .height()
        .saturating_add(chain.params().reward_maturity_delay_blocks());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipt.receipt_id)
            .all(|reward| reward.voided_by_challenge
                && reward
                    .claimable_at_height()
                    .expect("receipt reward should have inclusion-derived maturity")
                    == reward_hold_until_height)
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        0
    );
    chain.set_position_for_testing(reward_hold_until_height.saturating_sub(1), 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id)
    );
    chain.set_position_for_testing(reward_hold_until_height, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    for validator in validators.iter().take(2) {
        assert_eq!(chain.state().rewards().balance(validator), 0);
    }
}

#[test]
fn invalid_output_evidence_voids_delayed_receipt_rewards_before_release() {
    let beacon = hash_bytes(b"test", &[b"invalid-output-delay"]);
    let params = ChainParams {
        agreement_quorum: 1,
        freivalds: FreivaldsParams {
            validators_per_job: 3,
            minimum_validators: 2,
            minimum_stake_numerator: 2,
            minimum_stake_denominator: 3,
            ..FreivaldsParams::default()
        },
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"delayed-invalid-output-miner");
    chain.register_miner(miner, 100).unwrap();
    for i in 0..8 {
        chain
            .register_validator(
                address(format!("delayed-invalid-output-validator-{i}").as_bytes()),
                10_000,
            )
            .unwrap();
    }

    let job = MatmulJob::synthetic(0, 0, 8, 8, 8, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"delayed-invalid-output-validation"]),
        &chain.params().freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();

    let assignment_seed = chain.validator_assignment_seed(&receipt.receipt_id);
    let assigned =
        JobScheduler::default().assign_validators(&chain, receipt.receipt_id, &assignment_seed);
    let validators = assigned.validators;
    assert_eq!(validators.len(), 3);
    for validator in validators.iter().take(2) {
        chain
            .submit_attestation(ValidatorAttestation::new(
                *validator,
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
    }
    assert!(chain.has_attestation_quorum(&receipt.receipt_id));
    chain.settle_epoch(1_000, 500);
    assert!(
        chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id && !reward.voided_by_challenge)
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        receipt.tensor_work_units
    );

    chain
        .submit_attestation(ValidatorAttestation::new(
            validators[2],
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Invalid,
                checks_root: hash_bytes(b"test", &[b"delayed-invalid-output"]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    let reward_hold_until_height = chain
        .state()
        .height()
        .saturating_add(chain.params().reward_maturity_delay_blocks());
    assert!(
        !chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        chain
            .state()
            .challenged_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipt.receipt_id)
            .all(|reward| reward.voided_by_challenge
                && reward
                    .claimable_at_height()
                    .expect("receipt reward should have inclusion-derived maturity")
                    == reward_hold_until_height)
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .pending_tensor_work,
        0
    );
    assert_eq!(
        chain
            .state()
            .miners()
            .get(&miner)
            .unwrap()
            .settled_tensor_work,
        0
    );
    chain.set_position_for_testing(reward_hold_until_height.saturating_sub(1), 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .any(|reward| reward.receipt_id == receipt.receipt_id)
    );
    chain.set_position_for_testing(reward_hold_until_height, 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    for validator in validators.iter().take(2) {
        assert_eq!(chain.state().rewards().balance(validator), 0);
    }
}

#[test]
fn quorum_and_agreement_helpers_reject_unknown_receipts() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"orphan-validator");
    chain.register_validator(validator, 10_000).unwrap();
    let receipt_id = hash_bytes(b"test", &[b"orphan-receipt"]);
    chain.insert_attestation_for_testing(ValidatorAttestation::new(
        validator,
        10_000,
        AttestationStatement {
            receipt_id,
            job_id: hash_bytes(b"test", &[b"orphan-job"]),
            primitive_type: PrimitiveType::TensorOp,
            result: VerificationResult::Valid,
            checks_root: hash_bytes(b"test", &[b"orphan-checks"]),
            data_availability_passed: true,
        },
    ));

    assert!(!chain.has_attestation_quorum(&receipt_id));
    assert_eq!(chain.redundant_agreement_count(&receipt_id), 0);
    assert!(!chain.has_redundant_agreement(&receipt_id));
}

#[test]
fn redundant_agreement_quorum_is_required_before_settlement() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let params = ChainParams {
        agreement_quorum: 3,
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
    let miners: Vec<_> = (0..3)
        .map(|i| address(format!("agreement-miner-{i}").as_bytes()))
        .collect();
    for miner in &miners {
        chain.register_miner(*miner, 100).unwrap();
    }
    let validator = address(b"agreement-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 9, 4, 4, 4, &beacon, 10);
    chain.submit_job(JobState::TensorOp(job.clone()));
    let receipts: Vec<_> = miners
        .iter()
        .map(|miner| TensorOpReceipt::from_job(&job, *miner, 1, 5).unwrap().0)
        .collect();
    for receipt in receipts.iter().take(2) {
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
                    checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
    }

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 2);
    assert!(!chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert!(chain.state().settled_receipts().is_empty());
    let delay = chain
        .state()
        .redundant_settlement_delays()
        .get(&receipts[0].receipt_id)
        .expect("quorum-backed receipt should record delayed redundant settlement");
    assert_eq!(delay.receipt_id, receipts[0].receipt_id);
    assert_eq!(delay.job_id, receipts[0].job_id);
    assert_eq!(delay.primitive_type, PrimitiveType::TensorOp);
    assert_eq!(delay.observed_agreeing_miners, 2);
    assert_eq!(delay.observed_agreeing_operators, 2);
    assert_eq!(delay.required_agreement_quorum, 3);
    assert_eq!(delay.conflicting_quorum_receipts, 0);
    assert_eq!(
        delay.reward_delay_until_height,
        delay
            .recorded_at_height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    let redundant_reward_delay_until_height = delay.reward_delay_until_height;
    assert!(chain.state().pending_receipt_rewards().is_empty());
    assert_eq!(
        delay.reason,
        "awaiting redundant independent operator agreement quorum"
    );

    let receipt = &receipts[2];
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
                checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                data_availability_passed: true,
            },
        ))
        .unwrap();

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 3);
    assert_eq!(
        chain.redundant_agreement_operator_count(&receipts[0].receipt_id),
        3
    );
    assert!(chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().settled_receipts().len(), 3);
    let delayed_claims = chain
        .state()
        .pending_receipt_rewards()
        .values()
        .filter(|reward| reward.receipt_id == receipts[0].receipt_id)
        .collect::<Vec<_>>();
    assert_eq!(delayed_claims.len(), 2);
    assert!(
        delayed_claims
            .iter()
            .any(|reward| reward.kind == ReceiptRewardKind::Miner)
    );
    assert!(
        delayed_claims
            .iter()
            .any(|reward| reward.kind == ReceiptRewardKind::Validator)
    );
    assert!(delayed_claims.iter().all(|reward| {
        reward
            .claimable_at_height()
            .expect("receipt reward should have inclusion-derived maturity")
            == redundant_reward_delay_until_height
    }));
    drop(delayed_claims);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&receipts[0].miner), 0);
    let block = chain.produce_block(validator, 1_000).unwrap();
    assert!(
        chain
            .state()
            .included_receipts()
            .contains(&receipts[0].receipt_id)
    );
    let inclusion_reward_delay_until_height = block
        .height
        .saturating_add(chain.params().reward_maturity_delay_blocks());
    assert!(
        chain
            .state()
            .pending_receipt_rewards()
            .values()
            .filter(|reward| reward.receipt_id == receipts[0].receipt_id)
            .all(|reward| reward
                .claimable_at_height()
                .expect("receipt reward should have inclusion-derived maturity")
                == inclusion_reward_delay_until_height)
    );
    chain.set_position_for_testing(inclusion_reward_delay_until_height.saturating_sub(1), 0);
    assert!(chain.release_matured_receipt_rewards().unwrap().is_empty());
    assert_eq!(chain.state().rewards().balance(&receipts[0].miner), 0);
    chain.set_position_for_testing(inclusion_reward_delay_until_height, 0);
    let release_events = chain.release_matured_receipt_rewards().unwrap();
    assert!(release_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardReleased {
            receipt_id,
            beneficiary,
            ..
        } if *receipt_id == receipts[0].receipt_id && *beneficiary == receipts[0].miner
    )));
    assert!(chain.state().rewards().balance(&receipts[0].miner) > 0);
    assert!(
        chain
            .state()
            .redundant_settlement_delays()
            .get(&receipts[0].receipt_id)
            .is_none()
    );
}

#[test]
fn redundant_agreement_requires_distinct_miner_operators() {
    let beacon = hash_bytes(b"test", &[b"operator-quorum-beacon"]);
    let params = ChainParams {
        agreement_quorum: 3,
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
    let shared_operator = address(b"redundant-shared-operator");
    let miners: Vec<_> = (0..5)
        .map(|i| address(format!("operator-quorum-miner-{i}").as_bytes()))
        .collect();
    for miner in miners.iter().take(3) {
        chain
            .register_miner_with_operator(*miner, 100, shared_operator)
            .unwrap();
    }
    let distinct_operator = address(b"redundant-distinct-operator-a");
    chain
        .register_miner_with_operator(miners[3], 100, distinct_operator)
        .unwrap();
    chain
        .register_miner_with_operator(miners[4], 100, address(b"redundant-distinct-operator-b"))
        .unwrap();
    let validator = address(b"operator-quorum-validator");
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 10, 4, 4, 4, &beacon, 10);
    chain.submit_job(JobState::TensorOp(job.clone()));
    let receipts: Vec<_> = miners
        .iter()
        .map(|miner| TensorOpReceipt::from_job(&job, *miner, 1, 5).unwrap().0)
        .collect();
    for receipt in receipts.iter().take(3) {
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
                    checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
    }

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 3);
    assert_eq!(
        chain.redundant_agreement_operator_count(&receipts[0].receipt_id),
        1
    );
    assert!(!chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert!(chain.state().settled_receipts().is_empty());
    let delay = chain
        .state()
        .redundant_settlement_delays()
        .get(&receipts[0].receipt_id)
        .expect("same-operator agreement should remain delayed");
    assert_eq!(delay.observed_agreeing_miners, 3);
    assert_eq!(delay.observed_agreeing_operators, 1);
    assert_eq!(delay.required_agreement_quorum, 3);

    for distinct_receipt in receipts.iter().skip(3) {
        chain
            .submit_tensor_op_receipt(distinct_receipt.clone())
            .unwrap();
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: distinct_receipt.receipt_id,
                    job_id: distinct_receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[&distinct_receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
    }

    assert_eq!(chain.redundant_agreement_count(&receipts[0].receipt_id), 5);
    assert_eq!(
        chain.redundant_agreement_operator_count(&receipts[0].receipt_id),
        3
    );
    assert!(chain.has_redundant_agreement(&receipts[0].receipt_id));
    chain.settle_epoch(1_000, 500);
    assert_eq!(chain.state().settled_receipts().len(), 5);
}

#[test]
fn conflicting_linear_training_roots_do_not_settle() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut params = ChainParams::default();
    params.freivalds.minimum_validators = 1;
    params.freivalds.minimum_stake_numerator = 1;
    params.freivalds.minimum_stake_denominator = 1;
    params.agreement_quorum = 1;
    let mut chain = Chain::with_params(params, beacon);
    let miner = address(b"miner");
    let validator = address(b"validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"model"]),
        step: 0,
        batch_seed: hash_bytes(b"test", &[b"batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![3, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![3, 2],
        lr: 2,
        deadline_block: 20,
    });
    let (receipt, mut output) =
        LinearTrainingStepReceipt::from_job(&job, miner, &weights, 1, 5).unwrap();
    let tensor_job = MatmulJob::synthetic(0, 99, 2, 2, 2, &beacon, 20);
    let (tensor_receipt, _a, _b, _c) = TensorOpReceipt::from_job(&tensor_job, miner, 1, 5).unwrap();
    output
        .weight_after
        .set2(0, 0, output.weight_after.get2(0, 0).unwrap() + 1)
        .unwrap();
    let mut conflicting = receipt.clone();
    conflicting.weight_root_after = output.weight_after.commitment_root();
    conflicting.trace_root = hash_bytes(b"test", &[b"settlement-conflict-trace"]);
    conflicting.receipt_id = conflicting.recompute_receipt_id(&job.program_hash());
    conflicting.signature = sign(&conflicting.miner, &conflicting.receipt_id);
    chain.submit_job(JobState::LinearTrainingStep(job));
    chain.submit_job(JobState::TensorOp(tensor_job));
    chain
        .submit_tensor_op_receipt(tensor_receipt.clone())
        .unwrap();
    chain.submit_linear_receipt(receipt.clone()).unwrap();
    assert!(!has_conflicting_linear_receipt(
        &chain,
        receipt.receipt_id,
        &receipt
    ));
    chain.submit_linear_receipt(conflicting.clone()).unwrap();

    for receipt in [&receipt, &conflicting] {
        chain
            .submit_attestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::LinearTrainingStep,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[&receipt.receipt_id]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
    }

    chain.settle_epoch(1_000, 500);
    assert!(chain.state().settled_receipts().is_empty());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    let delay = chain
        .state()
        .redundant_settlement_delays()
        .get(&receipt.receipt_id)
        .expect("conflicting quorum-backed transition should record delayed settlement");
    assert_eq!(delay.primitive_type, PrimitiveType::LinearTrainingStep);
    assert_eq!(delay.observed_agreeing_miners, 1);
    assert_eq!(delay.observed_agreeing_operators, 1);
    assert_eq!(delay.required_agreement_quorum, 1);
    assert_eq!(delay.conflicting_quorum_receipts, 1);
    assert_eq!(
        delay.reward_delay_until_height,
        delay
            .recorded_at_height
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    assert!(chain.state().pending_receipt_rewards().is_empty());
    assert_eq!(
        delay.reason,
        "conflicting quorum-backed linear training transition"
    );
}
