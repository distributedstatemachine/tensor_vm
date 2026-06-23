use super::*;
use std::collections::BTreeMap;
use tensor_vm::app::{
    MinerRoleWorkObservation, RuntimeRole, ServiceRuntimeConfig,
    fetch_miner_role_missing_graph_artifacts, miner_role_work_observation, runtime_node_config,
    start_runtime_services, submit_miner_role_receipt, submit_miner_role_receipt_with_device,
    tick_miner_role_work_once,
};

fn supported_cuda_graph_execution_case() -> (
    tensor_vm::TensorGraph,
    BTreeMap<String, Tensor>,
    BTreeMap<String, tensor_vm::field::Elem>,
) {
    let graph = tensor_vm::TensorGraph {
        ir_version: 1,
        inputs: vec![
            tensor_vm::TensorSpec::field("a", vec![2, 3]),
            tensor_vm::TensorSpec::field("b", vec![3, 2]),
            tensor_vm::TensorSpec::field("bias", vec![2, 2]),
        ],
        params: vec![tensor_vm::ParamSpec {
            name: "scale".to_owned(),
            type_name: "field_scalar".to_owned(),
        }],
        ops: vec![
            tensor_vm::OpNode {
                id: 0,
                op: "matmul".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Input {
                        name: "a".to_owned(),
                    },
                    tensor_vm::IrRef::Input {
                        name: "b".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("product", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 1,
                op: "add".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 0, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("biased", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 2,
                op: "sub".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 1, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("centered", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 3,
                op: "mul".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 2, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("mixed", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 4,
                op: "transpose".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 3, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("transposed", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 5,
                op: "scalar_mul".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 4, idx: 0 },
                    tensor_vm::IrRef::Param {
                        name: "scale".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("scaled", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 6,
                op: "relu".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 5, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("activated", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 7,
                op: "neg".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 6, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("negated", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 8,
                op: "abs".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 7, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("absolute", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 9,
                op: "sign".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 8, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("signed", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 10,
                op: "identity".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 9, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("identity", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 11,
                op: "eq".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec {
                    name: "equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: tensor_vm::DType::Int32,
                    scale: 0,
                }],
            },
            tensor_vm::OpNode {
                id: 12,
                op: "gt".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec {
                    name: "greater_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: tensor_vm::DType::Int32,
                    scale: 0,
                }],
            },
            tensor_vm::OpNode {
                id: 13,
                op: "lt".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec {
                    name: "less_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: tensor_vm::DType::Int32,
                    scale: 0,
                }],
            },
            tensor_vm::OpNode {
                id: 14,
                op: "ge".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec {
                    name: "greater_equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: tensor_vm::DType::Int32,
                    scale: 0,
                }],
            },
            tensor_vm::OpNode {
                id: 15,
                op: "le".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec {
                    name: "less_equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: tensor_vm::DType::Int32,
                    scale: 0,
                }],
            },
            tensor_vm::OpNode {
                id: 16,
                op: "where".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 15, idx: 0 },
                    tensor_vm::IrRef::Op { id: 10, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("selected", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 17,
                op: "div".to_owned(),
                args: vec![
                    tensor_vm::IrRef::Op { id: 16, idx: 0 },
                    tensor_vm::IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![tensor_vm::TensorSpec::field("quotient", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 18,
                op: "clamp".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 17, idx: 0 }],
                kwargs: BTreeMap::from([
                    (
                        "min".to_owned(),
                        tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Field(2)),
                    ),
                    (
                        "max".to_owned(),
                        tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Field(100)),
                    ),
                ]),
                out: vec![tensor_vm::TensorSpec::field("clamped", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 19,
                op: "sum".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 18, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(1)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("summed", vec![2])],
            },
            tensor_vm::OpNode {
                id: 20,
                op: "mean".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 18, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(1)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("meaned", vec![2])],
            },
            tensor_vm::OpNode {
                id: 21,
                op: "broadcast".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 20, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "shape".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::List(vec![
                        tensor_vm::IrLiteral::Uint(2),
                        tensor_vm::IrLiteral::Uint(2),
                    ])),
                )]),
                out: vec![tensor_vm::TensorSpec::field("broadcasted", vec![2, 2])],
            },
            tensor_vm::OpNode {
                id: 22,
                op: "reshape".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 21, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "shape".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::List(vec![
                        tensor_vm::IrLiteral::Uint(4),
                    ])),
                )]),
                out: vec![tensor_vm::TensorSpec::field("reshaped", vec![4])],
            },
            tensor_vm::OpNode {
                id: 23,
                op: "unsqueeze".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 22, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(0)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("unsqueezed", vec![1, 4])],
            },
            tensor_vm::OpNode {
                id: 24,
                op: "squeeze".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 23, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(0)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("squeezed", vec![4])],
            },
            tensor_vm::OpNode {
                id: 25,
                op: "slice".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 24, idx: 0 }],
                kwargs: BTreeMap::from([
                    (
                        "dim".to_owned(),
                        tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(0)),
                    ),
                    (
                        "start".to_owned(),
                        tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(1)),
                    ),
                    (
                        "end".to_owned(),
                        tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(3)),
                    ),
                ]),
                out: vec![tensor_vm::TensorSpec::field("sliced", vec![2])],
            },
            tensor_vm::OpNode {
                id: 26,
                op: "unsqueeze".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 25, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Uint(0)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("triangular_input", vec![1, 2])],
            },
            tensor_vm::OpNode {
                id: 27,
                op: "triu".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 26, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "diagonal".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Int(0)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("upper", vec![1, 2])],
            },
            tensor_vm::OpNode {
                id: 28,
                op: "tril".to_owned(),
                args: vec![tensor_vm::IrRef::Op { id: 27, idx: 0 }],
                kwargs: BTreeMap::from([(
                    "diagonal".to_owned(),
                    tensor_vm::IrValue::Literal(tensor_vm::IrLiteral::Int(0)),
                )]),
                out: vec![tensor_vm::TensorSpec::field("triangular", vec![1, 2])],
            },
        ],
        outputs: vec![tensor_vm::GraphOutput {
            name: "triangular".to_owned(),
            value: tensor_vm::IrRef::Op { id: 28, idx: 0 },
        }],
    };
    let inputs = BTreeMap::from([
        (
            "a".to_owned(),
            Tensor::from_vec(
                vec![2, 3],
                tensor_vm::DType::FieldElement,
                vec![1, 2, 3, 4, 5, 6],
            )
            .unwrap(),
        ),
        (
            "b".to_owned(),
            Tensor::from_vec(
                vec![3, 2],
                tensor_vm::DType::FieldElement,
                vec![7, 8, 9, 10, 11, 12],
            )
            .unwrap(),
        ),
        (
            "bias".to_owned(),
            Tensor::from_vec(
                vec![2, 2],
                tensor_vm::DType::FieldElement,
                vec![3, 5, 7, 11],
            )
            .unwrap(),
        ),
    ]);
    (graph, inputs, BTreeMap::from([("scale".to_owned(), 3)]))
}

#[test]
fn miner_role_work_observation_tracks_assigned_unreceipted_jobs() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-observation"]));
    let miner = address(b"miner-work-observation-miner");
    register_miner(&mut chain, miner);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    let job_state = tensor_vm::JobState::TensorOp(job);
    chain
        .apply_command(ChainCommand::SubmitJob(job_state.clone()))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert_eq!(observation.unreceipted_jobs, BTreeSet::from([job_id]));

    let bundle = tensor_vm::roles::CpuReferenceMinerRole::new(miner)
        .execute_job(&job_state, chain.state().height(), 1)
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .unwrap();

    let observation = miner_role_work_observation(&chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_work_observation_ignores_unassigned_miners() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-work-unassigned"]));
    let miner = address(b"miner-work-assigned");
    let unassigned = address(b"miner-work-unassigned");
    register_miner(&mut chain, miner);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();

    assert_eq!(
        miner_role_work_observation(&chain, unassigned),
        MinerRoleWorkObservation::default()
    );
}

#[test]
fn miner_role_submits_assigned_unreceipted_tensor_op_once() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-receipt-submit"]));
    let miner = address(b"miner-receipt-submit-miner");
    register_miner(&mut chain, miner);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let submission = submit_miner_role_receipt(&mut node, miner, job_id)
        .unwrap()
        .expect("assigned unreceipted job should submit a receipt");

    assert_eq!(submission.receipts_submitted, 1);
    assert_eq!(submission.tensors_inserted, 3);
    assert_eq!(submission.backend_kind, BackendKind::CpuReference);
    assert_eq!(node.chain.state().receipts().len(), 1);
    let receipt = node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("receipt should be stored");
    assert_eq!(receipt.job_id(), job_id);
    assert_eq!(receipt.miner(), miner);
    assert_tensor_count(&node, 3);
    let observation = miner_role_work_observation(&node.chain, miner);
    assert_eq!(observation.assigned_jobs, BTreeSet::from([job_id]));
    assert!(observation.unreceipted_jobs.is_empty());
}

#[test]
fn miner_role_cuda_device_selection_reaches_gpu_backend_without_cuda_feature() {
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-cuda-selection"]));
        let miner = address(b"miner-cuda-selection-miner");
        register_miner(&mut chain, miner);
        let scheduler = JobScheduler::with_small_shape((2, 2, 2));
        let job = scheduler.generate_small_matmul(
            chain.state().epoch(),
            chain.state().height(),
            &chain.state().finalized_randomness(),
            chain
                .state()
                .height()
                .saturating_add(chain.params().receipt_submission_window),
        );
        let job_id = job.job_id;
        chain
            .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
            .unwrap();
        let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

        let error = submit_miner_role_receipt_with_device(&mut node, miner, job_id, "cuda:0")
            .expect_err("default build must route cuda selection to backend failure");

        assert!(error.contains("cuda kernels not compiled"));
        assert!(node.chain.state().receipts().is_empty());
    }
}

#[test]
fn miner_role_graph_cuda_device_selection_reaches_gpu_backend_without_cuda_feature() {
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 1,
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(
            params,
            hash_bytes(b"test", &[b"miner-cuda-graph-selection"]),
        );
        let miner = address(b"miner-cuda-graph-selection-miner");
        register_miner(&mut chain, miner);
        let graph = tensor_vm::SyntheticLocalJobSource::graph_execution_graph();
        let inputs = tensor_vm::SyntheticLocalJobSource::graph_execution_inputs();
        let mut source = tensor_vm::SyntheticLocalJobSource::default();
        let job = source.next_graph_job(&chain);
        let job_id = job.job_id;
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: job.graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(
                tensor_vm::JobState::GraphExecution(job),
            ))
            .unwrap();
        let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
        for tensor in inputs.into_values() {
            node.insert_tensor(tensor);
        }

        let error = submit_miner_role_receipt_with_device(&mut node, miner, job_id, "cuda:0")
            .expect_err("default build must route cuda graph selection to backend failure");

        assert!(error.contains("cuda kernels not compiled"));
        assert!(node.chain.state().receipts().is_empty());
    }
}

#[test]
fn miner_role_supported_multi_op_graph_cuda_device_selection_reaches_gpu_backend_without_cuda_feature()
 {
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let params = ChainParams {
            replication_factor: 1,
            agreement_quorum: 1,
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(
            params,
            hash_bytes(b"test", &[b"miner-cuda-supported-graph-selection"]),
        );
        let miner = address(b"miner-cuda-supported-graph-selection-miner");
        register_miner(&mut chain, miner);
        let (graph, inputs, field_params) = supported_cuda_graph_execution_case();
        let graph_id = graph.validate_for_consensus().unwrap();
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let job = tensor_vm::jobs::GraphJob::new(0, graph_id, input_roots, field_params, 10, 1, 1);
        let job_id = job.job_id;
        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: job.graph_id,
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        chain
            .apply_command(ChainCommand::SubmitJob(
                tensor_vm::JobState::GraphExecution(job),
            ))
            .unwrap();
        let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
        for tensor in inputs.into_values() {
            node.insert_tensor(tensor);
        }

        let error = submit_miner_role_receipt_with_device(&mut node, miner, job_id, "cuda:0")
            .expect_err(
                "default build must route supported cuda graph selection to backend failure",
            );

        assert!(error.contains("cuda kernels not compiled"));
        assert!(node.chain.state().receipts().is_empty());
    }
}

#[cfg(feature = "cuda-kernels")]
#[test]
fn miner_role_submits_tensor_op_with_configured_cuda_backend() {
    if tensor_vm::cuda_device_count().unwrap_or(0) == 0 {
        return;
    }
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-cuda-tensor-op"]));
    let miner = address(b"miner-cuda-tensor-op-miner");
    register_miner(&mut chain, miner);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let mut cpu_node = RpcNode::with_faucet(chain.clone(), Faucet::new(1_000_000, 100));
    let mut cuda_node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let cpu_submission = submit_miner_role_receipt(&mut cpu_node, miner, job_id)
        .unwrap()
        .expect("cpu role should submit receipt");
    let cuda_submission =
        submit_miner_role_receipt_with_device(&mut cuda_node, miner, job_id, "cuda:0")
            .unwrap()
            .expect("cuda role should submit receipt");

    assert_eq!(cpu_submission.backend_kind, BackendKind::CpuReference);
    assert_eq!(
        cuda_submission.backend_kind,
        BackendKind::GpuMiner {
            device: "cuda:0".to_owned()
        }
    );
    assert_eq!(cuda_submission.receipts_submitted, 1);
    assert_eq!(cuda_submission.tensors_inserted, 3);
    let cpu_receipt = cpu_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cpu receipt should be stored");
    let cuda_receipt = cuda_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cuda receipt should be stored");
    assert_eq!(cuda_receipt.job_id(), job_id);
    assert_eq!(cuda_receipt.miner(), miner);
    assert_eq!(cuda_receipt.receipt_id(), cpu_receipt.receipt_id());
}

#[cfg(feature = "cuda-kernels")]
#[test]
fn miner_role_submits_graph_execution_with_configured_cuda_backend() {
    if tensor_vm::cuda_device_count().unwrap_or(0) == 0 {
        return;
    }
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-cuda-graph"]));
    let miner = address(b"miner-cuda-graph-miner");
    register_miner(&mut chain, miner);
    let graph = tensor_vm::SyntheticLocalJobSource::graph_execution_graph();
    let inputs = tensor_vm::SyntheticLocalJobSource::graph_execution_inputs();
    let mut source = tensor_vm::SyntheticLocalJobSource::default();
    let job = source.next_graph_job(&chain);
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id: job.graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::GraphExecution(job),
        ))
        .unwrap();
    let mut cpu_node = RpcNode::with_faucet(chain.clone(), Faucet::new(1_000_000, 100));
    let mut cuda_node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    for tensor in inputs.values() {
        cpu_node.insert_tensor(tensor.clone());
        cuda_node.insert_tensor(tensor.clone());
    }

    let cpu_submission = submit_miner_role_receipt(&mut cpu_node, miner, job_id)
        .unwrap()
        .expect("cpu role should submit graph receipt");
    let cuda_submission =
        submit_miner_role_receipt_with_device(&mut cuda_node, miner, job_id, "cuda:0")
            .unwrap()
            .expect("cuda role should submit graph receipt");

    assert_eq!(cpu_submission.backend_kind, BackendKind::CpuReference);
    assert_eq!(
        cuda_submission.backend_kind,
        BackendKind::GpuMiner {
            device: "cuda:0".to_owned()
        }
    );
    assert_eq!(cuda_submission.receipts_submitted, 1);
    assert_eq!(cuda_submission.tensors_inserted, 3);
    let cpu_receipt = cpu_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cpu graph receipt should be stored");
    let cuda_receipt = cuda_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cuda graph receipt should be stored");
    assert_eq!(cuda_receipt.job_id(), job_id);
    assert_eq!(cuda_receipt.miner(), miner);
    assert_eq!(cuda_receipt.receipt_id(), cpu_receipt.receipt_id());
    let ReceiptState::GraphExecution(cpu_graph_receipt) = cpu_receipt else {
        panic!("cpu receipt must be graph execution");
    };
    let ReceiptState::GraphExecution(cuda_graph_receipt) = cuda_receipt else {
        panic!("cuda receipt must be graph execution");
    };
    assert_eq!(
        cuda_graph_receipt.output_roots,
        cpu_graph_receipt.output_roots
    );
    assert_eq!(cuda_graph_receipt.trace_root, cpu_graph_receipt.trace_root);
}

#[cfg(feature = "cuda-kernels")]
#[test]
fn miner_role_submits_supported_multi_op_graph_execution_with_configured_cuda_backend() {
    if tensor_vm::cuda_device_count().unwrap_or(0) == 0 {
        return;
    }
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(
        params,
        hash_bytes(b"test", &[b"miner-cuda-supported-graph"]),
    );
    let miner = address(b"miner-cuda-supported-graph-miner");
    register_miner(&mut chain, miner);
    let (graph, inputs, field_params) = supported_cuda_graph_execution_case();
    let graph_id = graph.validate_for_consensus().unwrap();
    let input_roots = inputs
        .iter()
        .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
        .collect();
    let job = tensor_vm::jobs::GraphJob::new(0, graph_id, input_roots, field_params, 10, 1, 1);
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id: job.graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::GraphExecution(job),
        ))
        .unwrap();
    let mut cpu_node = RpcNode::with_faucet(chain.clone(), Faucet::new(1_000_000, 100));
    let mut cuda_node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    for tensor in inputs.values() {
        cpu_node.insert_tensor(tensor.clone());
        cuda_node.insert_tensor(tensor.clone());
    }

    let cpu_submission = submit_miner_role_receipt(&mut cpu_node, miner, job_id)
        .unwrap()
        .expect("cpu role should submit supported graph receipt");
    let cuda_submission =
        submit_miner_role_receipt_with_device(&mut cuda_node, miner, job_id, "cuda:0")
            .unwrap()
            .expect("cuda role should submit supported graph receipt");

    assert_eq!(cpu_submission.backend_kind, BackendKind::CpuReference);
    assert_eq!(
        cuda_submission.backend_kind,
        BackendKind::GpuMiner {
            device: "cuda:0".to_owned()
        }
    );
    assert_eq!(cuda_submission.receipts_submitted, 1);
    assert_eq!(cuda_submission.tensors_inserted, 4);
    let cpu_receipt = cpu_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cpu graph receipt should be stored");
    let cuda_receipt = cuda_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cuda graph receipt should be stored");
    assert_eq!(cuda_receipt.job_id(), job_id);
    assert_eq!(cuda_receipt.miner(), miner);
    assert_eq!(cuda_receipt.receipt_id(), cpu_receipt.receipt_id());
    let ReceiptState::GraphExecution(cpu_graph_receipt) = cpu_receipt else {
        panic!("cpu receipt must be graph execution");
    };
    let ReceiptState::GraphExecution(cuda_graph_receipt) = cuda_receipt else {
        panic!("cuda receipt must be graph execution");
    };
    assert_eq!(
        cuda_graph_receipt.output_roots,
        cpu_graph_receipt.output_roots
    );
    assert_eq!(cuda_graph_receipt.trace_root, cpu_graph_receipt.trace_root);
}

#[cfg(feature = "cuda-kernels")]
#[test]
fn miner_role_submits_linear_step_with_configured_cuda_backend() {
    if tensor_vm::cuda_device_count().unwrap_or(0) == 0 {
        return;
    }
    let mut chain = Chain::new(hash_bytes(b"test", &[b"miner-cuda-linear"]));
    let miner = address(b"miner-cuda-linear-miner");
    register_miner(&mut chain, miner);
    let mut source = tensor_vm::SyntheticLocalJobSource::default();
    let job = source.next_linear_training_job(&chain);
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::LinearTrainingStep(job),
        ))
        .unwrap();
    let mut cpu_node = RpcNode::with_faucet(chain.clone(), Faucet::new(1_000_000, 100));
    let mut cuda_node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    let cpu_submission = submit_miner_role_receipt(&mut cpu_node, miner, job_id)
        .unwrap()
        .expect("cpu role should submit receipt");
    let cuda_submission =
        submit_miner_role_receipt_with_device(&mut cuda_node, miner, job_id, "cuda:0")
            .unwrap()
            .expect("cuda role should submit receipt");

    assert_eq!(cpu_submission.backend_kind, BackendKind::CpuReference);
    assert_eq!(
        cuda_submission.backend_kind,
        BackendKind::GpuMiner {
            device: "cuda:0".to_owned()
        }
    );
    assert_eq!(cuda_submission.receipts_submitted, 1);
    assert_eq!(cuda_submission.tensors_inserted, 6);
    let cpu_receipt = cpu_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cpu receipt should be stored");
    let cuda_receipt = cuda_node
        .chain
        .state()
        .receipts()
        .values()
        .next()
        .expect("cuda receipt should be stored");
    assert_eq!(cuda_receipt.job_id(), job_id);
    assert_eq!(cuda_receipt.miner(), miner);
    assert_eq!(cuda_receipt.receipt_id(), cpu_receipt.receipt_id());
}

#[test]
fn miner_role_receipt_submission_skips_duplicate_unregistered_and_unassigned_work() {
    let params = ChainParams {
        replication_factor: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-receipt-skip"]));
    let miner_a = address(b"miner-receipt-skip-a");
    let miner_b = address(b"miner-receipt-skip-b");
    let unknown = address(b"miner-receipt-skip-unknown");
    register_miner(&mut chain, miner_a);
    register_miner(&mut chain, miner_b);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    let assignment_seed = chain.miner_assignment_seed(&job_id);
    let assignment =
        JobScheduler::with_small_shape((8, 8, 8)).assign_miners(&chain, job_id, &assignment_seed);
    let assigned = assignment.miners[0];
    let unassigned = [miner_a, miner_b]
        .into_iter()
        .find(|miner| *miner != assigned)
        .expect("replication factor one should leave one registered miner unassigned");
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_miner_role_receipt(&mut node, unknown, job_id)
            .unwrap()
            .is_none()
    );
    assert!(
        submit_miner_role_receipt(&mut node, unassigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 0);

    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_some()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
    assert!(
        submit_miner_role_receipt(&mut node, assigned, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 1);
    assert_tensor_count(&node, 3);
}

#[test]
fn miner_role_receipt_submission_skips_stale_deadline_work() {
    let params = ChainParams {
        replication_factor: 1,
        receipt_submission_window: 1,
        ..ChainParams::default()
    };
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-stale-deadline"]));
    let miner = address(b"miner-stale-deadline-assigned");
    let validator = address(b"miner-stale-deadline-validator");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, validator);
    let scheduler = JobScheduler::with_small_shape((2, 2, 2));
    let job = scheduler.generate_small_matmul(
        chain.state().epoch(),
        chain.state().height(),
        &chain.state().finalized_randomness(),
        chain.state().height(),
    );
    let job_id = job.job_id;
    chain
        .apply_command(ChainCommand::SubmitJob(tensor_vm::JobState::TensorOp(job)))
        .unwrap();
    produce_block(&mut chain, validator, 1_000);
    assert_eq!(chain.state().height(), 1);
    assert_eq!(chain.job(&job_id).unwrap().deadline_block(), 0);
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));

    assert!(
        submit_miner_role_receipt(&mut node, miner, job_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(node.chain.state().receipts().len(), 0);
    let response = node.handle(&tensor_vm::RpcRequest {
        method: "GET".to_owned(),
        path: "/tensor/latest".to_owned(),
        body: Vec::new(),
    });
    assert_eq!(response.status, 404);
}

#[test]
fn miner_role_fetches_remote_graph_inputs_and_const_blobs_before_execution() {
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let miner = address(b"miner-remote-graph-artifact");
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-remote-graph"]));
    register_miner(&mut chain, miner);
    let (graph, input, blob, job) = graph_job_with_const_blob(&chain);
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id: job.graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::GraphExecution(job.clone()),
        ))
        .unwrap();
    let mut node = RpcNode::with_faucet(chain, Faucet::new(1_000_000, 100));
    let provider_port = free_tcp_port();
    let provider = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{provider_port}")],
        identity_seed: Some(hash_bytes(b"test", &[b"miner-remote-graph-provider"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    provider.register_tensor(input.clone());
    provider.register_tensor(blob.clone());
    let requester = spawn_libp2p_service(Libp2pControlPlaneConfig {
        listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
        bootstrap_addresses: vec![format!(
            "/ip4/127.0.0.1/tcp/{provider_port}/p2p/{}",
            provider.peer_id()
        )],
        identity_seed: Some(hash_bytes(b"test", &[b"miner-remote-graph-requester"])),
        ..Libp2pControlPlaneConfig::default()
    })
    .unwrap();
    wait_for_connected_role_services(&provider, &requester);

    assert!(submit_miner_role_receipt(&mut node, miner, job.job_id).is_err());
    let store = NodeStore::open(unique_temp_data_dir("miner-remote-graph-artifacts"));
    let report =
        fetch_miner_role_missing_graph_artifacts(&store, &mut node, &requester, job.job_id)
            .unwrap();
    assert_eq!(report.successes, 2);
    assert_eq!(report.tensors_inserted, 2);

    let submission = submit_miner_role_receipt(&mut node, miner, job.job_id)
        .unwrap()
        .expect("fetched graph artifacts should let miner submit a receipt");
    assert_eq!(submission.receipts_submitted, 1);
    assert!(matches!(
        node.chain.state().receipts().values().next(),
        Some(ReceiptState::GraphExecution(_))
    ));
}

#[test]
fn miner_role_tick_keeps_missing_graph_artifacts_pending_without_exiting() {
    let data_dir = unique_temp_data_dir("miner-missing-graph-pending");
    let _ = std::fs::remove_dir_all(&data_dir);
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let params = ChainParams {
        replication_factor: 1,
        agreement_quorum: 1,
        ..ChainParams::default()
    };
    let miner = address(b"miner-missing-graph-pending");
    let mut chain = Chain::with_params(params, hash_bytes(b"test", &[b"miner-missing-graph"]));
    register_miner(&mut chain, miner);
    let (graph, _input, _blob, job) = graph_job_with_const_blob(&chain);
    chain
        .apply_command(ChainCommand::RegisterProgramBody {
            graph_id: job.graph_id,
            bytes: graph.canonical_json().into_bytes(),
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::SubmitJob(
            tensor_vm::JobState::GraphExecution(job.clone()),
        ))
        .unwrap();
    let store = NodeStore::open(data_dir.clone());
    store.persist_chain(&chain).unwrap();
    let config = ServiceRuntimeConfig {
        runtime_command: "miner_run",
        role: RuntimeRole::Miner,
        role_wallet_address: Some(miner),
        role_wallet_secret: Some("miner-missing-graph".to_owned()),
        miner_device: Some("cpu".to_owned()),
        node: runtime_node_config(
            &data_dir_text,
            RuntimeRole::Miner,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            Some(hash_bytes(b"test", &[b"miner-missing-graph-identity"])),
            "secret",
            0,
        )
        .unwrap(),
        randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
    };
    let mut services = start_runtime_services(&config).unwrap();
    let mut runtime_state = NodeRuntimeState::default();

    assert!(
        tick_miner_role_work_once(
            &config,
            &services.store,
            &mut services.server,
            &services.p2p_service,
            &mut runtime_state,
        )
        .unwrap()
    );
    assert!(
        services
            .server
            .gateway()
            .node
            .chain
            .state()
            .receipts()
            .is_empty()
    );
    assert!(runtime_state.miner_receipts_submitted() == 0);
    assert!(runtime_state.miner_tensors_inserted() == 0);

    drop(services);
    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}

fn graph_job_with_const_blob(
    chain: &Chain,
) -> (
    tensor_vm::TensorGraph,
    Tensor,
    Tensor,
    tensor_vm::jobs::GraphJob,
) {
    let input = Tensor::from_vec(vec![2], tensor_vm::DType::FieldElement, vec![5, 6]).unwrap();
    let blob = Tensor::from_vec(vec![2], tensor_vm::DType::FieldElement, vec![1, 2]).unwrap();
    let blob_uri = hex(&blob.commitment_root());
    let graph = tensor_vm::TensorGraph {
        ir_version: 1,
        inputs: vec![tensor_vm::TensorSpec::field("x", vec![2])],
        params: Vec::new(),
        ops: vec![tensor_vm::OpNode {
            id: 0,
            op: "add".to_owned(),
            args: vec![
                tensor_vm::IrRef::Input {
                    name: "x".to_owned(),
                },
                tensor_vm::IrRef::ConstBlob {
                    uri: blob_uri,
                    shape: vec![2],
                    dtype: tensor_vm::DType::FieldElement,
                },
            ],
            kwargs: BTreeMap::new(),
            out: vec![tensor_vm::TensorSpec::field("y", vec![2])],
        }],
        outputs: vec![tensor_vm::GraphOutput {
            name: "y".to_owned(),
            value: tensor_vm::IrRef::Op { id: 0, idx: 0 },
        }],
    };
    let graph_id = graph.validate_for_consensus().unwrap();
    let job = tensor_vm::jobs::GraphJob::new(
        chain.state().epoch(),
        graph_id,
        BTreeMap::from([("x".to_owned(), input.commitment_root())]),
        BTreeMap::new(),
        chain
            .state()
            .height()
            .saturating_add(chain.params().receipt_submission_window),
        1,
        2,
    );
    (graph, input, blob, job)
}
