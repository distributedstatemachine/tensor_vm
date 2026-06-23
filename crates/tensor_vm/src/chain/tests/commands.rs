use super::*;
use crate::canonical_matmul_graph;
use crate::ir::{GraphOutput, IrLiteral, IrRef, IrValue, OpNode, TensorGraph, TensorSpec};
use crate::jobs::{GraphJob, GraphReceipt};
use crate::tensor::{DType, Tensor};
use std::collections::BTreeMap;

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
    let miner_claimable_at_height = pending_credit_reward(&mut chain, miner, 7);
    assert_eq!(
        chain.apply_command(ChainCommand::ClaimReward(miner)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );
    assert!(miner_claimable_at_height > chain.state().height());
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().accounts().get(&miner).unwrap().balance, 38);
    let credit_events = chain
        .apply_command(ChainCommand::CreditReward {
            address: receiver,
            amount: 9,
        })
        .unwrap();
    let ChainEvent::CreditRewardPending {
        claim_id,
        beneficiary,
        amount,
        claimable_at_height,
    } = credit_events[0]
    else {
        panic!("credit reward should create a pending claim");
    };
    assert_eq!(beneficiary, receiver);
    assert_eq!(amount, 9);
    assert_eq!(chain.state().rewards().balance(&receiver), 0);
    assert_eq!(chain.state().pending_credit_rewards().len(), 2);
    assert_eq!(
        chain.apply_command(ChainCommand::ClaimReward(receiver)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );
    assert_eq!(chain.state().rewards().balance(&receiver), 0);
    assert_eq!(
        chain.state().pending_credit_rewards()[&claim_id].claim_id,
        claim_id
    );
    assert!(claimable_at_height > chain.state().height());

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
            claimable_at_height: None,
            awaiting_inclusion: true,
            ..
        } if *receipt_id == receipt.receipt_id && *beneficiary == miner
    )));
    assert!(settlement_events.iter().any(|event| matches!(
        event,
        ChainEvent::ReceiptRewardPending {
            receipt_id,
            beneficiary,
            amount: 500,
            claimable_at_height: None,
            awaiting_inclusion: true,
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

fn pending_credit_reward(chain: &mut Chain, beneficiary: Address, amount: u64) -> u64 {
    let events = chain
        .apply_command(ChainCommand::CreditReward {
            address: beneficiary,
            amount,
        })
        .unwrap();
    let ChainEvent::CreditRewardPending {
        claimable_at_height,
        ..
    } = events[0]
    else {
        panic!("credit reward should create a pending claim");
    };
    claimable_at_height
}

#[test]
fn chain_engine_admits_graph_jobs_and_receipts_from_registered_program_body() {
    let beacon = hash_bytes(b"test", &[b"graph-command"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"graph-command-miner");
    chain
        .apply_command(ChainCommand::RegisterMiner {
            address: miner,
            stake: 100,
        })
        .unwrap();

    let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
    let graph_id = graph.validate_for_consensus().unwrap();
    let a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 0, 0, 1]).unwrap();
    let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![2, 3, 4, 5]).unwrap();
    let inputs = BTreeMap::from([("a".to_owned(), a.clone()), ("b".to_owned(), b.clone())]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);

    assert_eq!(
        chain.apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
            job.clone()
        ))),
        Err(TvmError::InvalidReceipt("unknown tensor ir graph body"))
    );

    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(
            job.clone(),
        )))
        .unwrap();

    let (receipt, _outputs) =
        GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 6).unwrap();
    let events = chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();

    assert_eq!(
        events,
        vec![ChainEvent::ReceiptAccepted(receipt.receipt_id)]
    );
    assert!(chain.state().receipts().contains_key(&receipt.receipt_id));
}

#[test]
fn chain_engine_registers_valid_canonical_program_body_without_job() {
    let beacon = hash_bytes(b"test", &[b"chain-engine-program-body"]);
    let mut chain = Chain::new(beacon);
    let graph = canonical_matmul_graph(2, 3, 4, DType::FieldElement);
    let graph_id = graph.graph_id();
    let bytes = graph.canonical_json().into_bytes();

    assert_eq!(
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id,
                bytes: bytes.clone(),
            })
            .unwrap(),
        vec![ChainEvent::ProgramBodyRegistered { graph_id }]
    );
    assert_eq!(
        chain.state().program_body(&graph_id),
        Some(bytes.as_slice())
    );
    assert!(chain.state().jobs().is_empty());

    assert_eq!(
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id,
                bytes: bytes.clone(),
            })
            .unwrap(),
        vec![ChainEvent::ProgramBodyRegistered { graph_id }]
    );
    assert_eq!(chain.state().program_bodies().len(), 1);
}

#[test]
fn chain_engine_rejects_invalid_or_conflicting_program_bodies() {
    let beacon = hash_bytes(b"test", &[b"chain-engine-program-body-reject"]);
    let mut chain = Chain::new(beacon);
    let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
    let graph_id = graph.graph_id();
    let bytes = graph.canonical_json().into_bytes();

    assert_eq!(
        chain.apply_command(ChainCommand::RegisterProgramBody {
            graph_id: hash_bytes(b"test", &[b"wrong-graph-id"]),
            bytes: bytes.clone(),
        }),
        Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"))
    );

    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert_ne!(noncanonical, bytes);
    assert_eq!(
        chain.apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: noncanonical,
        }),
        Err(TvmError::InvalidReceipt(
            "noncanonical tensor ir graph body"
        ))
    );

    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: bytes.clone(),
        })
        .unwrap();
    chain
        .state
        .program_bodies
        .insert(graph_id, b"stale".to_vec());
    assert_eq!(
        chain.apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: bytes.clone(),
        }),
        Err(TvmError::InvalidReceipt("conflicting tensor ir graph body"))
    );
    let conflicting = canonical_matmul_graph(2, 2, 3, DType::FieldElement)
        .canonical_json()
        .into_bytes();
    assert_eq!(
        chain.apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: conflicting,
        }),
        Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"))
    );
}

#[test]
fn chain_engine_rejects_index_consistency_ops_at_program_registration() {
    let beacon = hash_bytes(b"test", &[b"chain-engine-index-consistency-program"]);
    let mut chain = Chain::new(beacon);
    let graph = TensorGraph {
        ir_version: 1,
        inputs: vec![
            TensorSpec {
                name: "a".to_owned(),
                shape: vec![2, 3],
                dtype: DType::FieldElement,
                scale: 0,
            },
            TensorSpec {
                name: "index".to_owned(),
                shape: vec![2, 3],
                dtype: DType::Int64,
                scale: 0,
            },
        ],
        params: Vec::new(),
        ops: vec![OpNode {
            id: 0,
            op: "gather".to_owned(),
            args: vec![
                IrRef::Input {
                    name: "a".to_owned(),
                },
                IrRef::Input {
                    name: "index".to_owned(),
                },
            ],
            kwargs: BTreeMap::from([("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1)))]),
            out: vec![TensorSpec {
                name: "gathered".to_owned(),
                shape: vec![2, 3],
                dtype: DType::FieldElement,
                scale: 0,
            }],
        }],
        outputs: vec![GraphOutput {
            name: "gathered".to_owned(),
            value: IrRef::Op { id: 0, idx: 0 },
        }],
    };
    assert!(graph.validate(false).is_ok());
    let graph_id = graph.graph_id();

    assert_eq!(
        chain.apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        }),
        Err(TvmError::InvalidReceipt(
            "tensor ir op is not consensus admitted"
        ))
    );
    assert!(chain.state().program_bodies().is_empty());
}

#[test]
fn generic_credit_rewards_claim_only_after_maturity() {
    let beacon = hash_bytes(b"test", &[b"delayed-credit-reward"]);
    let mut chain = Chain::with_params(
        ChainParams {
            reward_settlement_delay_epochs: 1,
            challenge_window_epochs: 1,
            epoch_length: 5,
            ..ChainParams::default()
        },
        beacon,
    );
    let beneficiary = address(b"credit-beneficiary");

    let events = chain
        .apply_command(ChainCommand::CreditReward {
            address: beneficiary,
            amount: 25,
        })
        .unwrap();
    let ChainEvent::CreditRewardPending {
        claim_id,
        claimable_at_height,
        ..
    } = events[0]
    else {
        panic!("expected pending credit event");
    };
    assert_eq!(claimable_at_height, 10);
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
    assert_eq!(
        chain.apply_command(ChainCommand::ClaimReward(beneficiary)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );
    chain.set_position_for_testing(claimable_at_height, 0);
    let claim_events = chain
        .apply_command(ChainCommand::ClaimReward(beneficiary))
        .unwrap();
    assert!(claim_events.contains(&ChainEvent::CreditRewardReleased {
        claim_id,
        beneficiary,
        amount: 25,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardCredited {
        address: beneficiary,
        amount: 25,
    }));
    assert!(claim_events.contains(&ChainEvent::RewardClaimed {
        address: beneficiary,
        amount: 25,
    }));
    assert_eq!(chain.state().pending_credit_rewards().len(), 0);
    assert_eq!(chain.state().rewards().balance(&beneficiary), 0);
    assert_eq!(
        chain.state().accounts().get(&beneficiary).unwrap().balance,
        25
    );
}
