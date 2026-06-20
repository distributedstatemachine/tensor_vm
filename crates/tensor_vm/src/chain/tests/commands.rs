use super::*;

#[test]
fn chain_engine_applies_profile_neutral_commands() {
    let beacon = hash_bytes(b"test", &[b"chain-engine"]);
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
    let miner = address(b"engine-miner");
    let validator = address(b"engine-validator");
    let receiver = address(b"engine-receiver");

    assert_eq!(chain.params().agreement_quorum, 1);
    assert_eq!(
        chain
            .apply_command(ChainCommand::RegisterMiner {
                address: miner,
                stake: 100,
            })
            .unwrap(),
        vec![ChainEvent::MinerRegistered(miner)]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::RegisterValidator {
                address: validator,
                stake: 10_000,
            })
            .unwrap(),
        vec![ChainEvent::ValidatorRegistered(validator)]
    );
    chain.credit_account(miner, 50);
    assert_eq!(
        chain
            .apply_command(ChainCommand::Transfer {
                from: miner,
                to: receiver,
                amount: 12,
            })
            .unwrap(),
        vec![ChainEvent::AccountTransferred {
            from: miner,
            to: receiver,
            amount: 12,
        }]
    );
    assert_eq!(chain.state().accounts().get(&receiver).unwrap().balance, 12);
    chain.credit_reward_for_testing(miner, 7);
    assert_eq!(
        chain
            .apply_command(ChainCommand::ClaimReward(miner))
            .unwrap(),
        vec![ChainEvent::RewardClaimed {
            address: miner,
            amount: 7,
        }]
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().accounts().get(&miner).unwrap().balance, 45);
    assert_eq!(
        chain
            .apply_command(ChainCommand::CreditReward {
                address: receiver,
                amount: 9,
            })
            .unwrap(),
        vec![ChainEvent::RewardCredited {
            address: receiver,
            amount: 9,
        }]
    );
    assert_eq!(chain.state().rewards().balance(&receiver), 9);

    let matmul_job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let matmul_program_hash = matmul_job.program_hash();
    let matmul_program_body = matmul_job.tensor_ir_graph().canonical_json().into_bytes();
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&matmul_job, miner, 0, 3).unwrap();
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::TensorOp(
                matmul_job.clone()
            )))
            .unwrap(),
        vec![ChainEvent::JobAccepted(matmul_job.job_id)]
    );
    assert_eq!(
        chain.state().program_body(&matmul_program_hash),
        Some(matmul_program_body.as_slice())
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitReceipt(ReceiptState::TensorOp(
                receipt.clone()
            )))
            .unwrap(),
        vec![ChainEvent::ReceiptAccepted(receipt.receipt_id)]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitAttestation(ValidatorAttestation::new(
                validator,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"engine-checks"]),
                    data_availability_passed: true,
                },
            )))
            .unwrap(),
        vec![ChainEvent::AttestationAccepted {
            receipt_id: receipt.receipt_id,
            validator,
        }]
    );

    let settlement_events = chain
        .apply_command(ChainCommand::SettleEpoch {
            miner_reward_pool: 1_000,
            validator_reward_pool: 500,
        })
        .unwrap();
    assert!(settlement_events.contains(&ChainEvent::ReceiptSettled(receipt.receipt_id)));
    assert!(settlement_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardPending {
            receipt_id,
            beneficiary,
            amount: 1_000,
            ..
        } if *receipt_id == receipt.receipt_id && *beneficiary == miner
    )));
    assert!(settlement_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardPending {
            receipt_id,
            beneficiary,
            amount: 500,
            ..
        } if *receipt_id == receipt.receipt_id && *beneficiary == validator
    )));
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().rewards().balance(&validator), 0);

    let block_events = chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer: validator,
            timestamp: 6,
        })
        .unwrap();
    let block = chain.blocks().last().unwrap().clone();
    assert_eq!(
        block_events,
        vec![ChainEvent::BlockProduced {
            height: 0,
            hash: block.hash(),
        }]
    );
    assert_eq!(chain.view().height, 1);
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitBlockVote(BlockVote::new(
                validator, 10_000, &block
            )))
            .unwrap(),
        vec![
            ChainEvent::BlockVoteAccepted {
                block_hash: block.hash(),
                validator,
            },
            ChainEvent::BlockFinalized(block.hash()),
        ]
    );

    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let model_id = hash_bytes(b"test", &[b"engine-model"]);
    let architecture = hash_bytes(b"test", &[b"engine-architecture"]);
    let config = hash_bytes(b"test", &[b"engine-config"]);
    assert_eq!(
        chain
            .apply_command(ChainCommand::RegisterModel {
                model_id,
                architecture_hash: architecture,
                weight_root: weights.commitment_root(),
                config_hash: config,
            })
            .unwrap(),
        vec![ChainEvent::ModelRegistered(model_id)]
    );
    let registered_model = chain.state().model_states().get(&model_id).unwrap().clone();
    assert_eq!(
        chain.apply_command(ChainCommand::RegisterModel {
            model_id,
            architecture_hash: architecture,
            weight_root: weights.commitment_root(),
            config_hash: config,
        }),
        Err(TvmError::InvalidReceipt("duplicate model"))
    );
    assert_eq!(
        chain.state().model_states().get(&model_id),
        Some(&registered_model)
    );
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id,
        step: 0,
        batch_seed: hash_bytes(b"test", &[b"engine-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![2, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![2, 2],
        lr: 1,
        deadline_block: 20,
    });
    let (linear_receipt, _) =
        LinearTrainingStepReceipt::from_job(&linear_job, miner, &weights, 1, 4).unwrap();
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitJob(JobState::LinearTrainingStep(
                linear_job.clone()
            )))
            .unwrap(),
        vec![ChainEvent::JobAccepted(linear_job.job_id)]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::SubmitReceipt(
                ReceiptState::LinearTrainingStep(linear_receipt.clone())
            ))
            .unwrap(),
        vec![ChainEvent::ReceiptAccepted(linear_receipt.receipt_id)]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyModelTransition {
                model_id,
                step: 0,
                weight_root_before: weights.commitment_root(),
                weight_root_after: linear_receipt.weight_root_after,
            })
            .unwrap(),
        vec![ChainEvent::ModelTransitionApplied {
            model_id,
            step: 0,
            weight_root_after: linear_receipt.weight_root_after,
        }]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyChallengeOutcome(
                ChallengeOutcome::Rejected {
                    reason: "honest receipt".to_owned(),
                }
            ))
            .unwrap(),
        vec![ChainEvent::ChallengeRejected {
            reason: "honest receipt".to_owned(),
        }]
    );
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyChallengeOutcome(
                ChallengeOutcome::ProvenInvalid {
                    dishonest_party: miner,
                    slash_amount: 3,
                    reason: "invalid receipt".to_owned(),
                }
            ))
            .unwrap(),
        vec![ChainEvent::ChallengeProvenInvalid {
            dishonest_party: miner,
            slash_amount: 3,
            reason: "invalid receipt".to_owned(),
        }]
    );
    assert_eq!(chain.state().miners().get(&miner).unwrap().stake, 97);
    assert_eq!(chain.state().rewards().treasury(), 3);
}
