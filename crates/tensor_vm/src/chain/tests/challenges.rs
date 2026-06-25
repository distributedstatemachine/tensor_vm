use super::*;
use crate::{
    canonical_linear_training_step_graph,
    challenge::{
        BlockCheckChallenge, BlockCheckChallengeInput, TraceBisectionConfig,
        TraceBisectionExpectation, TraceBisectionRound,
    },
    ir::{
        GraphOutput, IrOpRefereeWitness, IrOpWitnessValue, IrRef, OpNode, TensorGraph, TensorSpec,
    },
    jobs::{GraphJob, GraphReceipt},
    merkle::{build_proof, merkle_root},
    storage::{decode_chain_state_snapshot, encode_chain_state_snapshot},
    tensor::{DType, Tensor},
    types::sign,
};
use std::collections::BTreeMap;

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

fn trace_bisection_fixture() -> (Chain, GraphReceipt, crate::ir::IrExecution) {
    let beacon = hash_bytes(b"test", &[b"trace-bisection-chain"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"trace-bisection-miner");
    chain.register_miner(miner, 100).unwrap();

    let graph =
        canonical_linear_training_step_graph(&[2, 2], &[2, 2], &[2, 2], DType::FieldElement);
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let x = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let w = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
    let target = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 1, 1, 1]).unwrap();
    let inputs = BTreeMap::from([
        ("target".to_owned(), target),
        ("w".to_owned(), w),
        ("x".to_owned(), x),
    ]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let field_params = BTreeMap::from([("lr".to_owned(), 1)]);
    let job = GraphJob::new(0, graph_id, input_roots, field_params, 10, 1, 16);
    let (receipt, _) = GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
    let execution = job.exact_ir_execution(&graph, &inputs).unwrap();

    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(job)))
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();
    (chain, receipt, execution)
}

fn two_op_trace_bisection_fixture() -> (Chain, GraphReceipt, crate::ir::IrExecution, Tensor, Tensor)
{
    let beacon = hash_bytes(b"test", &[b"trace-bisection-referee-chain"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"trace-bisection-referee-miner");
    chain.register_miner(miner, 100).unwrap();

    let graph = TensorGraph {
        ir_version: 1,
        inputs: vec![
            TensorSpec::field("x", vec![2, 2]),
            TensorSpec::field("y", vec![2, 2]),
        ],
        params: Vec::new(),
        ops: vec![
            OpNode {
                id: 0,
                op: "add".to_owned(),
                args: vec![
                    IrRef::Input {
                        name: "x".to_owned(),
                    },
                    IrRef::Input {
                        name: "y".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("sum", vec![2, 2])],
            },
            OpNode {
                id: 1,
                op: "identity".to_owned(),
                args: vec![IrRef::Op { id: 0, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("out", vec![2, 2])],
            },
        ],
        outputs: vec![GraphOutput {
            name: "out".to_owned(),
            value: IrRef::Op { id: 1, idx: 0 },
        }],
    };
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let x = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let y = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
    let inputs = BTreeMap::from([("x".to_owned(), x.clone()), ("y".to_owned(), y.clone())]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);
    let (receipt, _) = GraphReceipt::from_execution(&job, &graph, miner, &inputs, 1, 3).unwrap();
    let execution = job.exact_ir_execution(&graph, &inputs).unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(job)))
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();
    (chain, receipt, execution, x, y)
}

fn submit_trace_bisection_expectation(
    chain: &mut Chain,
    challenge_id: Hash,
    expected_output_roots: Vec<Hash>,
) {
    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let expectation = TraceBisectionExpectation::new(&state, expected_output_roots).unwrap();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionExpectation(expectation))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionExpectationAccepted { .. }]
    ));
}

#[test]
fn trace_bisection_rounds_are_chain_admitted_and_state_rooted() {
    let (mut chain, receipt, execution) = trace_bisection_fixture();
    let challenger = address(b"trace-bisection-challenger");
    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let ChainEvent::TraceBisectionOpened {
        challenge_id,
        low_op,
        high_op,
        ..
    } = open_events[0]
    else {
        panic!("expected trace bisection open event");
    };
    assert_eq!(low_op, 0);
    assert_eq!(high_op, 5);
    let opened_root = chain.state_root();

    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = execution.trace_opening(state.midpoint()).unwrap();
    let expected_output_roots = opening.op_trace.output_roots.clone();
    let round = TraceBisectionRound::new(&state, expected_output_roots.clone(), opening).unwrap();
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitTraceBisectionRound(round.clone())),
        Err(TvmError::InvalidReceipt(
            "trace bisection expectation missing"
        ))
    );
    submit_trace_bisection_expectation(&mut chain, challenge_id, expected_output_roots.clone());
    let wrong_opening = execution.trace_opening(state.midpoint()).unwrap();
    let wrong_round = TraceBisectionRound::new(
        &state,
        vec![hash_bytes(b"test", &[b"wrong-round-expected-root"])],
        wrong_opening,
    )
    .unwrap();
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitTraceBisectionRound(wrong_round)),
        Err(TvmError::InvalidReceipt(
            "trace bisection expectation mismatch"
        ))
    );
    let pending_record = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap();
    assert!(!pending_record.pending_expected_output_roots.is_empty());
    assert!(pending_record.pending_expectation_leaf.is_some());
    let pending_encoded = encode_chain_state_snapshot(chain.state());
    let pending_decoded = decode_chain_state_snapshot(&pending_encoded).unwrap();
    assert_eq!(
        pending_decoded.trace_bisection_challenges(),
        chain.state().trace_bisection_challenges()
    );
    let narrowed_events = chain
        .apply_command(ChainCommand::SubmitTraceBisectionRound(round.clone()))
        .unwrap();
    assert!(matches!(
        narrowed_events.as_slice(),
        [ChainEvent::TraceBisectionNarrowed {
            low_op: 3,
            high_op: 5,
            matched_midpoint: true,
            ..
        }]
    ));
    assert_ne!(chain.state_root(), opened_root);
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitTraceBisectionRound(round)),
        Err(TvmError::InvalidReceipt(
            "trace bisection round state mismatch"
        ))
    );

    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = execution.trace_opening(state.midpoint()).unwrap();
    let expected_output_roots = opening.op_trace.output_roots.clone();
    let final_round =
        TraceBisectionRound::new(&state, expected_output_roots.clone(), opening).unwrap();
    submit_trace_bisection_expectation(&mut chain, challenge_id, expected_output_roots);
    let isolated_events = chain
        .apply_command(ChainCommand::SubmitTraceBisectionRound(final_round))
        .unwrap();
    assert!(matches!(
        isolated_events.as_slice(),
        [ChainEvent::TraceBisectionIsolated { op_index: 5, .. }]
    ));
    let record = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap();
    assert_eq!(record.opened_rounds, 2);
    assert!(record.pending_expected_output_roots.is_empty());
    assert!(record.pending_expectation_leaf.is_none());
    assert!(matches!(
        record.status,
        TraceBisectionStatus::Isolated { op_index: 5 }
    ));

    let encoded = encode_chain_state_snapshot(chain.state());
    let decoded = decode_chain_state_snapshot(&encoded).unwrap();
    assert_eq!(
        decoded.trace_bisection_challenges(),
        chain.state().trace_bisection_challenges()
    );
}

#[test]
fn isolated_trace_bisection_referee_records_one_op_verdict() {
    let (mut chain, receipt, execution, x, y) = two_op_trace_bisection_fixture();
    let challenger = address(b"trace-bisection-referee-challenger");
    chain.register_validator(challenger, 10_000).unwrap();
    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
        panic!("expected trace bisection open event");
    };
    let challenge_id = *challenge_id;
    let witness = IrOpRefereeWitness {
        op_index: 0,
        input_values: vec![
            IrOpWitnessValue::Tensor(x.clone()),
            IrOpWitnessValue::Tensor(y.clone()),
        ],
    };
    assert_eq!(
        chain.apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness: witness.clone(),
        }),
        Err(TvmError::InvalidReceipt("trace bisection is not isolated"))
    );

    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = execution.trace_opening(0).unwrap();
    let wrong_expected = vec![hash_bytes(b"test", &[b"wrong-referee-expected-root"])];
    submit_trace_bisection_expectation(&mut chain, challenge_id, wrong_expected.clone());
    let round = TraceBisectionRound::new(&state, wrong_expected, opening.clone()).unwrap();
    let isolated_events = chain
        .apply_command(ChainCommand::SubmitTraceBisectionRound(round))
        .unwrap();
    assert!(matches!(
        isolated_events.as_slice(),
        [ChainEvent::TraceBisectionIsolated { op_index: 0, .. }]
    ));
    let record = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap();
    assert_eq!(
        record.last_opening_input_roots,
        opening.op_trace.input_roots
    );
    assert_eq!(
        record.last_opening_output_roots,
        opening.op_trace.output_roots
    );

    let bad_witness = IrOpRefereeWitness {
        op_index: 0,
        input_values: vec![
            IrOpWitnessValue::Tensor(y.clone()),
            IrOpWitnessValue::Tensor(x.clone()),
        ],
    };
    assert_eq!(
        chain.apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness: bad_witness,
        }),
        Err(TvmError::InvalidReceipt(
            "trace bisection referee input root mismatch"
        ))
    );

    let refereed_events = chain
        .apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness,
        })
        .unwrap();
    let expected_sum = x.add(&y).unwrap().commitment_root();
    assert_eq!(refereed_events.len(), 1);
    assert!(matches!(
        refereed_events.as_slice(),
        [ChainEvent::TraceBisectionRefereed {
            op_index: 0,
            dishonest_party,
            canonical_output_roots,
            disputed_output_roots,
            ..
        }] if *dishonest_party == challenger
            && canonical_output_roots == &vec![expected_sum]
            && disputed_output_roots == &opening.op_trace.output_roots
    ));
    assert_eq!(
        chain.state().validators().get(&challenger).unwrap().stake,
        9_993
    );
    assert_eq!(chain.state().rewards().treasury(), 7);
    assert!(chain.state().pending_challenge_rewards().is_empty());
    let record = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap();
    assert!(matches!(
        &record.status,
        TraceBisectionStatus::Refereed {
            op_index: 0,
            dishonest_party,
            canonical_output_roots,
            disputed_output_roots,
        } if *dishonest_party == challenger
            && canonical_output_roots == &vec![expected_sum]
            && disputed_output_roots == &opening.op_trace.output_roots
    ));
    assert_eq!(
        chain.apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness: IrOpRefereeWitness {
                op_index: 0,
                input_values: vec![IrOpWitnessValue::Tensor(x), IrOpWitnessValue::Tensor(y)],
            },
        }),
        Err(TvmError::InvalidReceipt("trace bisection is not isolated"))
    );

    let encoded = encode_chain_state_snapshot(chain.state());
    let decoded = decode_chain_state_snapshot(&encoded).unwrap();
    assert_eq!(
        decoded.trace_bisection_challenges(),
        chain.state().trace_bisection_challenges()
    );
}

#[test]
fn trace_bisection_referee_resolves_tier_c_committee_dispute() {
    // A single honest challenger punishes a wrong Tier-C (gelu) receipt via the
    // §8.2 game — 1-of-N honesty, independent of committee honesty (§8.1).
    let beacon = hash_bytes(b"test", &[b"tier-c-bisection-chain"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"tier-c-bisection-miner");
    let challenger = address(b"tier-c-bisection-challenger");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(challenger, 10_000).unwrap();

    let fixed = |name: &str| TensorSpec {
        name: name.to_owned(),
        shape: vec![2, 2],
        dtype: DType::Fixed32,
        scale: 16,
    };
    let graph = TensorGraph {
        ir_version: 1,
        inputs: vec![fixed("x")],
        params: Vec::new(),
        ops: vec![
            OpNode {
                id: 0,
                op: "gelu".to_owned(),
                args: vec![IrRef::Input {
                    name: "x".to_owned(),
                }],
                kwargs: BTreeMap::new(),
                out: vec![fixed("activated")],
            },
            OpNode {
                id: 1,
                op: "identity".to_owned(),
                args: vec![IrRef::Op { id: 0, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![fixed("out")],
            },
        ],
        outputs: vec![GraphOutput {
            name: "out".to_owned(),
            value: IrRef::Op { id: 1, idx: 0 },
        }],
    };
    // Tier-C: only committee admission accepts this graph.
    assert!(graph.validate_for_consensus().is_err());
    let graph_id = graph.validate_for_committee().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();

    let x =
        Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 16, vec![0, 32768, 65536, 98304])
            .unwrap();
    let inputs = BTreeMap::from([("x".to_owned(), x.clone())]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 4);
    let execution = job
        .committee_ir_execution_with_const_blobs(&graph, &inputs, &BTreeMap::new())
        .unwrap();
    // Miner commits a wrong trace: tamper the gelu op output.
    let mut bad_execution = execution.clone();
    let bad_output_root = hash_bytes(b"test", &[b"tier-c-bisection-bad-gelu-output"]);
    bad_execution.op_traces[0].output_roots = vec![bad_output_root];
    bad_execution.trace_root = merkle_root(&bad_execution.trace_leaves());
    let output_roots = execution
        .outputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let receipt =
        GraphReceipt::from_roots(&job, miner, output_roots, bad_execution.trace_root, 1, 3);
    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(job)))
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();

    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: bad_execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
        panic!("expected trace bisection open event");
    };
    let challenge_id = *challenge_id;
    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = bad_execution.trace_opening(0).unwrap();
    let expected_output_roots = execution.op_traces[0].output_roots.clone();
    submit_trace_bisection_expectation(&mut chain, challenge_id, expected_output_roots.clone());
    let round = TraceBisectionRound::new(&state, expected_output_roots.clone(), opening).unwrap();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionRound(round))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionIsolated { op_index: 0, .. }]
    ));

    // The referee re-executes the isolated gelu op on agreed inputs and rules.
    let witness = IrOpRefereeWitness {
        op_index: 0,
        input_values: vec![IrOpWitnessValue::Tensor(x)],
    };
    let events = chain
        .apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness,
        })
        .unwrap();
    assert!(matches!(
        events.as_slice(),
        [
            ChainEvent::TraceBisectionRefereed {
                dishonest_party,
                canonical_output_roots,
                disputed_output_roots,
                ..
            },
            ChainEvent::ChallengeRewardPending { amount: 5, .. }
        ] if *dishonest_party == miner
            && canonical_output_roots == &expected_output_roots
            && disputed_output_roots == &vec![bad_output_root]
    ));
    assert_eq!(chain.state().miners().get(&miner).unwrap().stake, 89);
}

#[test]
fn trace_bisection_referee_slashes_miner_and_delays_challenger_reward() {
    let beacon = hash_bytes(b"test", &[b"trace-bisection-bounty-chain"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"trace-bisection-bounty-miner");
    let challenger = address(b"trace-bisection-bounty-challenger");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(challenger, 10_000).unwrap();

    let graph = TensorGraph {
        ir_version: 1,
        inputs: vec![
            TensorSpec::field("x", vec![2, 2]),
            TensorSpec::field("y", vec![2, 2]),
        ],
        params: Vec::new(),
        ops: vec![
            OpNode {
                id: 0,
                op: "add".to_owned(),
                args: vec![
                    IrRef::Input {
                        name: "x".to_owned(),
                    },
                    IrRef::Input {
                        name: "y".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("sum", vec![2, 2])],
            },
            OpNode {
                id: 1,
                op: "identity".to_owned(),
                args: vec![IrRef::Op { id: 0, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("out", vec![2, 2])],
            },
        ],
        outputs: vec![GraphOutput {
            name: "out".to_owned(),
            value: IrRef::Op { id: 1, idx: 0 },
        }],
    };
    let graph_id = graph.validate_for_consensus().unwrap();
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    let x = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let y = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
    let inputs = BTreeMap::from([("x".to_owned(), x.clone()), ("y".to_owned(), y.clone())]);
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 10, 1, 8);
    let execution = job.exact_ir_execution(&graph, &inputs).unwrap();
    let mut bad_execution = execution.clone();
    let bad_output_root = hash_bytes(b"test", &[b"trace-bisection-bad-op-output"]);
    bad_execution.op_traces[0].output_roots = vec![bad_output_root];
    bad_execution.trace_root = merkle_root(&bad_execution.trace_leaves());
    let output_roots = execution
        .outputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let receipt =
        GraphReceipt::from_roots(&job, miner, output_roots, bad_execution.trace_root, 1, 3);
    chain
        .apply_command(ChainCommand::SubmitJob(JobState::GraphExecution(job)))
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(ReceiptState::GraphExecution(
            receipt.clone(),
        )))
        .unwrap();

    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: bad_execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
        panic!("expected trace bisection open event");
    };
    let challenge_id = *challenge_id;
    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = bad_execution.trace_opening(0).unwrap();
    let expected_output_roots = execution.op_traces[0].output_roots.clone();
    submit_trace_bisection_expectation(&mut chain, challenge_id, expected_output_roots.clone());
    let round = TraceBisectionRound::new(&state, expected_output_roots.clone(), opening).unwrap();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionRound(round))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionIsolated { op_index: 0, .. }]
    ));

    let witness = IrOpRefereeWitness {
        op_index: 0,
        input_values: vec![IrOpWitnessValue::Tensor(x), IrOpWitnessValue::Tensor(y)],
    };
    let events = chain
        .apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness,
        })
        .unwrap();
    assert!(matches!(
        events.as_slice(),
        [
            ChainEvent::TraceBisectionRefereed {
                dishonest_party,
                canonical_output_roots,
                disputed_output_roots,
                ..
            },
            ChainEvent::ChallengeRewardPending {
                challenge_id: event_challenge_id,
                block_hash,
                receipt_id,
                challenger: event_challenger,
                amount: 5,
                ..
            }
        ] if *dishonest_party == miner
            && canonical_output_roots == &expected_output_roots
            && disputed_output_roots == &vec![bad_output_root]
            && *event_challenge_id == challenge_id
            && *block_hash == [0; 32]
            && *receipt_id == receipt.receipt_id
            && *event_challenger == challenger
    ));
    assert_eq!(chain.state().miners().get(&miner).unwrap().stake, 89);
    assert_eq!(chain.state().rewards().treasury(), 6);
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    let pending = chain
        .state()
        .pending_challenge_rewards()
        .values()
        .next()
        .unwrap();
    assert_eq!(pending.challenge_id, challenge_id);
    assert_eq!(pending.receipt_id, receipt.receipt_id);
    assert_eq!(pending.challenger, challenger);
    assert_eq!(pending.amount, 5);
    assert_eq!(
        pending.claimable_at_height,
        chain
            .state()
            .height()
            .saturating_add(chain.params().reward_maturity_delay_blocks())
    );

    let claimable_at_height = pending.claimable_at_height;
    chain.set_position_for_testing(claimable_at_height, 1);
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
            challenge_id: event_challenge_id,
            challenger: event_challenger,
            amount: 5,
            ..
        } if *event_challenge_id == challenge_id && *event_challenger == challenger
    )));
    assert_eq!(
        chain.state().accounts().get(&challenger).unwrap().balance,
        5
    );
}

#[test]
fn trace_bisection_chain_admission_rejects_mismatch_and_records_timeout() {
    let (mut chain, receipt, execution) = trace_bisection_fixture();
    let challenger = address(b"trace-bisection-timeout-challenger");
    chain.register_validator(challenger, 10_000).unwrap();
    let pending_receipt_claim = hash_bytes(b"test", &[b"trace-timeout-pending-receipt-reward"]);
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: pending_receipt_claim,
        receipt_id: receipt.receipt_id,
        beneficiary: receipt.miner,
        amount: 13,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(0),
        voided_by_challenge: false,
    });
    let open = ChainCommand::OpenTraceBisection(TraceBisectionConfig {
        receipt_id: receipt.receipt_id,
        trace_root: receipt.trace_root,
        challenger,
        responder: receipt.miner,
        op_count: execution.op_traces.len() as u64,
        response_deadline_height: 2,
        challenger_bond: 7,
        responder_bond: 11,
    });
    chain.apply_command(open.clone()).unwrap();
    assert_eq!(
        chain.apply_command(open),
        Err(TvmError::InvalidReceipt("duplicate trace bisection"))
    );
    assert_eq!(
        chain.apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: hash_bytes(b"test", &[b"wrong-trace-root"]),
            challenger: address(b"wrong-trace-challenger"),
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 2,
            challenger_bond: 7,
            responder_bond: 11,
        })),
        Err(TvmError::InvalidReceipt(
            "trace bisection receipt trace root mismatch"
        ))
    );

    let challenge_id = *chain
        .state()
        .trace_bisection_challenges()
        .keys()
        .next()
        .unwrap();
    assert_eq!(
        chain.apply_command(ChainCommand::RecordTraceBisectionTimeout { challenge_id }),
        Err(TvmError::InvalidReceipt("trace bisection deadline pending"))
    );
    chain.set_position_for_testing(3, 0);
    let timeout_events = chain
        .apply_command(ChainCommand::RecordTraceBisectionTimeout { challenge_id })
        .unwrap();
    assert!(matches!(
        timeout_events.as_slice(),
        [
            ChainEvent::TraceBisectionTimedOut {
            forfeiting_party,
            ..
            },
            ChainEvent::ChallengeRewardPending {
                challenge_id: event_challenge_id,
                receipt_id,
                challenger: event_challenger,
                amount: 5,
                ..
            }
        ] if *forfeiting_party == receipt.miner
            && *event_challenge_id == challenge_id
            && *receipt_id == receipt.receipt_id
            && *event_challenger == challenger
    ));
    assert!(matches!(
        chain
            .state()
            .trace_bisection_challenges()
            .get(&challenge_id)
            .unwrap()
            .status,
        TraceBisectionStatus::TimedOut { forfeiting_party } if forfeiting_party == receipt.miner
    ));
    assert_eq!(
        chain.state().miners().get(&receipt.miner).unwrap().stake,
        89
    );
    assert_eq!(chain.state().rewards().treasury(), 6);
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
    let voided_reward = chain
        .state()
        .pending_receipt_rewards()
        .get(&pending_receipt_claim)
        .unwrap();
    assert!(voided_reward.voided_by_challenge);
    assert_eq!(
        voided_reward.maturity,
        ReceiptRewardMaturity::ClaimableAt(
            3_u64.saturating_add(chain.params().reward_maturity_delay_blocks())
        )
    );
    let pending = chain
        .state()
        .pending_challenge_rewards()
        .values()
        .next()
        .unwrap();
    assert_eq!(pending.challenge_id, challenge_id);
    assert_eq!(pending.receipt_id, receipt.receipt_id);
    assert_eq!(pending.challenger, challenger);
    assert_eq!(pending.amount, 5);
    assert_eq!(
        pending.claimable_at_height,
        3_u64.saturating_add(chain.params().reward_maturity_delay_blocks())
    );
    let claimable_at_height = pending.claimable_at_height;
    assert_eq!(chain.state().rewards().balance(&challenger), 0);
    chain.set_position_for_testing(claimable_at_height, 1);
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
            challenge_id: event_challenge_id,
            challenger: event_challenger,
            amount: 5,
            ..
        } if *event_challenge_id == challenge_id && *event_challenger == challenger
    )));
    assert_eq!(
        chain.state().accounts().get(&challenger).unwrap().balance,
        5
    );
}

#[test]
fn trace_bisection_admission_enforces_round_budget_and_pending_expectation_policy() {
    let (mut chain, receipt, execution) = trace_bisection_fixture();
    let challenger = address(b"trace-bisection-dos-policy-challenger");
    chain.register_validator(challenger, 10_000).unwrap();
    assert_eq!(
        chain.apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: (1_u64 << 32) + 1,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        })),
        Err(TvmError::InvalidReceipt(
            "trace bisection round cap exceeded"
        ))
    );
    assert!(chain.state().trace_bisection_challenges().is_empty());

    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
        panic!("expected trace bisection open event");
    };
    let challenge_id = *challenge_id;
    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = execution.trace_opening(state.midpoint()).unwrap();
    let expected_output_roots = opening.op_trace.output_roots.clone();
    let expectation =
        TraceBisectionExpectation::new(&state, expected_output_roots.clone()).unwrap();
    let expectation_leaf = expectation.expectation_leaf();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionExpectation(
                expectation.clone()
            ))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionExpectationAccepted {
            expectation_leaf: event_leaf,
            ..
        }] if *event_leaf == expectation_leaf
    ));
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionExpectation(
                expectation.clone()
            ))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionExpectationAccepted {
            expectation_leaf: event_leaf,
            ..
        }] if *event_leaf == expectation_leaf
    ));
    let conflicting =
        TraceBisectionExpectation::new(&state, vec![hash_bytes(b"test", &[b"conflicting-root"])])
            .unwrap();
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitTraceBisectionExpectation(conflicting)),
        Err(TvmError::InvalidReceipt(
            "trace bisection expectation already pending"
        ))
    );

    let round = TraceBisectionRound::new(&state, expected_output_roots, opening).unwrap();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionRound(round))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionNarrowed { .. }]
    ));
    let record = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap();
    assert!(record.pending_expected_output_roots.is_empty());
    assert!(record.pending_expectation_leaf.is_none());
}

#[test]
fn isolated_trace_bisection_timeout_slashes_incomplete_challenger() {
    let (mut chain, receipt, execution, x, y) = two_op_trace_bisection_fixture();
    let challenger = address(b"trace-bisection-isolated-timeout-challenger");
    chain.register_validator(challenger, 10_000).unwrap();
    let pending_receipt_claim = hash_bytes(b"test", &[b"isolated-timeout-pending-receipt-reward"]);
    chain.mark_receipt_settled_for_testing(receipt.receipt_id);
    chain.insert_pending_receipt_reward_for_testing(PendingReceiptReward {
        claim_id: pending_receipt_claim,
        receipt_id: receipt.receipt_id,
        beneficiary: receipt.miner,
        amount: 13,
        kind: ReceiptRewardKind::Miner,
        maturity: ReceiptRewardMaturity::ClaimableAt(0),
        voided_by_challenge: false,
    });
    let open_events = chain
        .apply_command(ChainCommand::OpenTraceBisection(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger,
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 9,
            challenger_bond: 7,
            responder_bond: 11,
        }))
        .unwrap();
    let [ChainEvent::TraceBisectionOpened { challenge_id, .. }] = open_events.as_slice() else {
        panic!("expected trace bisection open event");
    };
    let challenge_id = *challenge_id;

    let state = chain
        .state()
        .trace_bisection_challenges()
        .get(&challenge_id)
        .unwrap()
        .state
        .clone();
    let opening = execution.trace_opening(0).unwrap();
    let wrong_expected = vec![hash_bytes(
        b"test",
        &[b"isolated-timeout-wrong-expected-root"],
    )];
    submit_trace_bisection_expectation(&mut chain, challenge_id, wrong_expected.clone());
    let round = TraceBisectionRound::new(&state, wrong_expected, opening).unwrap();
    assert!(matches!(
        chain
            .apply_command(ChainCommand::SubmitTraceBisectionRound(round))
            .unwrap()
            .as_slice(),
        [ChainEvent::TraceBisectionIsolated { op_index: 0, .. }]
    ));
    assert_eq!(
        chain.apply_command(ChainCommand::RecordTraceBisectionTimeout { challenge_id }),
        Err(TvmError::InvalidReceipt("trace bisection deadline pending"))
    );

    chain.set_position_for_testing(10, 0);
    let timeout_events = chain
        .apply_command(ChainCommand::RecordTraceBisectionTimeout { challenge_id })
        .unwrap();
    assert!(matches!(
        timeout_events.as_slice(),
        [ChainEvent::TraceBisectionTimedOut {
            forfeiting_party,
            ..
        }] if *forfeiting_party == challenger
    ));
    assert!(matches!(
        chain
            .state()
            .trace_bisection_challenges()
            .get(&challenge_id)
            .unwrap()
            .status,
        TraceBisectionStatus::TimedOut { forfeiting_party } if forfeiting_party == challenger
    ));
    assert_eq!(
        chain.state().validators().get(&challenger).unwrap().stake,
        9_993
    );
    assert_eq!(chain.state().rewards().treasury(), 7);
    assert!(chain.state().pending_challenge_rewards().is_empty());
    assert!(
        chain
            .state()
            .settled_receipts()
            .contains(&receipt.receipt_id)
    );
    assert!(
        !chain
            .state()
            .challenged_receipts()
            .contains(&receipt.receipt_id)
    );
    let receipt_reward = chain
        .state()
        .pending_receipt_rewards()
        .get(&pending_receipt_claim)
        .unwrap();
    assert!(!receipt_reward.voided_by_challenge);
    assert_eq!(
        receipt_reward.maturity,
        ReceiptRewardMaturity::ClaimableAt(0)
    );
    assert_eq!(
        chain.apply_command(ChainCommand::RefereeTraceBisection {
            challenge_id,
            witness: IrOpRefereeWitness {
                op_index: 0,
                input_values: vec![IrOpWitnessValue::Tensor(x), IrOpWitnessValue::Tensor(y)],
            },
        }),
        Err(TvmError::InvalidReceipt("trace bisection is not isolated"))
    );

    let encoded = encode_chain_state_snapshot(chain.state());
    let decoded = decode_chain_state_snapshot(&encoded).unwrap();
    assert_eq!(
        decoded.trace_bisection_challenges(),
        chain.state().trace_bisection_challenges()
    );
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
fn pre_finality_block_check_challenge_delays_and_voids_late_proposer_reward() {
    let beacon = hash_bytes(b"test", &[b"pre-finality-block-check-delay-beacon"]);
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
    let miner = address(b"pre-finality-block-check-miner");
    let proposer = address(b"pre-finality-block-check-proposer");
    let challenger = address(b"pre-finality-block-check-watcher");
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
            b"pre-finality-block-check-observed-leaf",
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
    assert!(!chain.state().finalized_blocks().contains(&bad_hash));
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
            proposer: event_proposer,
            challenger: event_challenger,
            proposer_reward_clawback: 1_000,
            challenger_reward: event_reward,
            ..
        } if *block_hash == bad_hash
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
    assert_eq!(
        pending_proposer.claimable_at_height,
        bad_block
            .height
            .saturating_add(chain.params().proposer_reward_maturity_delay_blocks())
    );
    assert_eq!(
        chain
            .state()
            .pending_challenge_rewards()
            .values()
            .next()
            .unwrap()
            .amount,
        challenger_reward
    );

    chain.state.finalized_blocks.insert(bad_hash);
    crate::chain::blocks::materialize_finalized_proposer_rewards(
        &mut chain.state,
        &chain.blocks,
        &chain.params,
    );
    let pending_after_late_finality = chain
        .state()
        .pending_proposer_rewards()
        .get(&bad_block.height)
        .unwrap();
    assert!(pending_after_late_finality.voided_by_challenge);
    assert_eq!(pending_after_late_finality.amount, 1_000);

    let claimable_at_height = pending_after_late_finality.claimable_at_height;
    chain.set_position_for_testing(claimable_at_height, 1);
    assert!(chain.release_matured_proposer_rewards().unwrap().is_empty());
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
