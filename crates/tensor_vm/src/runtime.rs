use std::collections::BTreeMap;
#[cfg(feature = "cuda-kernels")]
use std::collections::BTreeSet;

#[cfg(feature = "cuda-kernels")]
use crate::conformance::conformance_suite_hash;
use crate::conformance::{ConformanceProfile, cpu_reference_conformance_profile};
use crate::error::{Result, TvmError};
#[cfg(feature = "cuda-kernels")]
use crate::field::Elem;
#[cfg(feature = "cuda-kernels")]
use crate::ir::{GraphOutput, IrOpTrace, IrRef, OpNode, ParamSpec, TensorSpec};
use crate::ir::{IrExecution, TensorGraph};
use crate::jobs::{GraphJob, LinearTrainingStepJob, LinearTrainingStepOutput, MatmulJob};
#[cfg(feature = "cuda-kernels")]
use crate::merkle::merkle_root;
#[cfg(feature = "cuda-kernels")]
use crate::tensor::DType;
use crate::tensor::Tensor;
#[cfg(feature = "cuda-kernels")]
use crate::types::{Hash, hash_bytes};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendKind {
    CpuReference,
    GpuMiner { device: String },
}

pub trait ExecutionBackend {
    fn kind(&self) -> BackendKind;

    fn execute_matmul(&self, job: &MatmulJob) -> Result<(Tensor, Tensor, Tensor)> {
        job.execute()
    }

    fn execute_linear_training_step(
        &self,
        job: &LinearTrainingStepJob,
        weights: &Tensor,
    ) -> Result<LinearTrainingStepOutput> {
        job.execute(weights)
    }

    fn execute_graph_exact(
        &self,
        job: &GraphJob,
        graph: &TensorGraph,
        tensors: &BTreeMap<String, Tensor>,
        const_blobs: &BTreeMap<String, Tensor>,
    ) -> Result<IrExecution> {
        job.exact_ir_execution_with_const_blobs(graph, tensors, const_blobs)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CpuReferenceBackend;

impl ExecutionBackend for CpuReferenceBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::CpuReference
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuMinerBackend {
    device: String,
}

impl GpuMinerBackend {
    pub fn new(device: impl Into<String>) -> Self {
        Self {
            device: device.into(),
        }
    }

    pub fn device(&self) -> &str {
        &self.device
    }
}

impl ExecutionBackend for GpuMinerBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::GpuMiner {
            device: self.device.clone(),
        }
    }

    fn execute_matmul(&self, job: &MatmulJob) -> Result<(Tensor, Tensor, Tensor)> {
        #[cfg(feature = "cuda-kernels")]
        {
            let (a, b) = job.input_tensors()?;
            let c = cuda::field_matmul(self.cuda_device_index()?, &a, &b)?;
            Ok((a, b, c))
        }
        #[cfg(not(feature = "cuda-kernels"))]
        {
            let _ = job;
            Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
        }
    }

    fn execute_linear_training_step(
        &self,
        job: &LinearTrainingStepJob,
        weights: &Tensor,
    ) -> Result<LinearTrainingStepOutput> {
        #[cfg(feature = "cuda-kernels")]
        {
            if weights.commitment_root() != job.weight_root_before {
                return Err(TvmError::InvalidReceipt("weight root mismatch"));
            }
            let (x, target) = job.batch_tensors()?;
            let device_index = self.cuda_device_index()?;
            let y = cuda::field_matmul(device_index, &x, weights)?;
            let dy = cuda::field_sub(device_index, &y, &target)?;
            let x_t = cuda::field_transpose(device_index, &x)?;
            let grad_w = cuda::field_matmul(device_index, &x_t, &dy)?;
            let scaled_grad = cuda::field_scalar_mul(device_index, &grad_w, job.lr)?;
            let weight_after = cuda::field_sub(device_index, weights, &scaled_grad)?;
            let loss_commitment = cuda::field_mse_loss(device_index, &y, &target)?;
            Ok(LinearTrainingStepOutput {
                x,
                target,
                y,
                dy,
                grad_w,
                weight_after,
                loss_commitment,
            })
        }
        #[cfg(not(feature = "cuda-kernels"))]
        {
            let _ = (job, weights);
            Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
        }
    }

    fn execute_graph_exact(
        &self,
        job: &GraphJob,
        graph: &TensorGraph,
        tensors: &BTreeMap<String, Tensor>,
        const_blobs: &BTreeMap<String, Tensor>,
    ) -> Result<IrExecution> {
        #[cfg(feature = "cuda-kernels")]
        {
            self.execute_graph_exact_cuda(job, graph, tensors, const_blobs)
        }
        #[cfg(not(feature = "cuda-kernels"))]
        {
            let _ = (job, graph, tensors, const_blobs);
            Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
        }
    }
}

#[cfg(feature = "cuda-kernels")]
impl GpuMinerBackend {
    fn cuda_device_index(&self) -> Result<u32> {
        let index = self.device.strip_prefix("cuda:").unwrap_or(&self.device);
        index
            .parse::<u32>()
            .map_err(|_| TvmError::InvalidReceipt("invalid cuda device"))
    }

    fn execute_graph_exact_cuda(
        &self,
        job: &GraphJob,
        graph: &TensorGraph,
        tensors: &BTreeMap<String, Tensor>,
        const_blobs: &BTreeMap<String, Tensor>,
    ) -> Result<IrExecution> {
        let graph_id = graph.validate_for_consensus()?;
        if graph_id != job.graph_id {
            return Err(TvmError::InvalidReceipt("tensor ir graph id mismatch"));
        }
        for (name, expected_root) in &job.input_roots {
            let Some(tensor) = tensors.get(name) else {
                return Err(TvmError::InvalidReceipt("missing graph input tensor"));
            };
            if tensor.commitment_root() != *expected_root {
                return Err(TvmError::InvalidReceipt("graph input root mismatch"));
            }
        }
        if tensors.len() != job.input_roots.len() {
            return Err(TvmError::InvalidReceipt("unexpected graph input tensor"));
        }

        let mut graph_inputs = tensors.clone();
        for (uri, tensor) in const_blobs {
            if graph_inputs.insert(uri.clone(), tensor.clone()).is_some() {
                return Err(TvmError::InvalidReceipt("graph const_blob input collision"));
            }
        }

        let device_index = self.cuda_device_index()?;
        let mut op_outputs = Vec::<Vec<GraphRuntimeValue>>::new();
        let mut op_traces = Vec::with_capacity(graph.ops.len());
        let mut trace_leaves = Vec::with_capacity(graph.ops.len());

        for op in &graph.ops {
            let args = op
                .args
                .iter()
                .map(|arg| {
                    resolve_graph_runtime_ref(arg, &graph_inputs, &job.field_params, &op_outputs)
                })
                .collect::<Result<Vec<_>>>()?;
            let input_roots = args
                .iter()
                .map(GraphRuntimeValue::commitment_root)
                .collect::<Vec<Hash>>();
            let outputs = execute_cuda_graph_op(device_index, op.op.as_str(), &args)?;
            let output_roots = outputs
                .iter()
                .map(|value| match value {
                    GraphRuntimeValue::Tensor(tensor) => Ok(tensor.commitment_root()),
                    GraphRuntimeValue::Field(_) => Err(TvmError::InvalidReceipt(
                        "tensor ir op produced scalar output",
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            trace_leaves.push(graph_trace_op_leaf(op.id, &input_roots, &output_roots));
            op_traces.push(IrOpTrace {
                op_id: op.id,
                input_roots,
                output_roots,
            });
            op_outputs.push(outputs);
        }

        let mut outputs = BTreeMap::new();
        for output in &graph.outputs {
            let value = resolve_graph_runtime_ref(
                &output.value,
                &graph_inputs,
                &job.field_params,
                &op_outputs,
            )?;
            match value {
                GraphRuntimeValue::Tensor(tensor) => {
                    outputs.insert(output.name.clone(), tensor);
                }
                GraphRuntimeValue::Field(_) => {
                    return Err(TvmError::InvalidReceipt(
                        "tensor ir graph output resolves to scalar",
                    ));
                }
            }
        }

        Ok(IrExecution {
            graph_id,
            outputs,
            op_traces,
            trace_root: merkle_root(&trace_leaves),
        })
    }
}

#[cfg(feature = "cuda-kernels")]
#[derive(Clone, Debug, Eq, PartialEq)]
enum GraphRuntimeValue {
    Tensor(Tensor),
    Field(Elem),
}

#[cfg(feature = "cuda-kernels")]
impl GraphRuntimeValue {
    fn commitment_root(&self) -> Hash {
        match self {
            Self::Tensor(tensor) => tensor.commitment_root(),
            Self::Field(value) => {
                hash_bytes(b"tensor-vm-ir-field-value-v1", &[&value.to_le_bytes()])
            }
        }
    }
}

#[cfg(feature = "cuda-kernels")]
fn graph_trace_op_leaf(op_id: usize, input_roots: &[Hash], output_roots: &[Hash]) -> Hash {
    let mut encoded = Vec::with_capacity(24 + input_roots.len() * 32 + output_roots.len() * 32);
    encoded.extend_from_slice(&(op_id as u64).to_le_bytes());
    encoded.extend_from_slice(&(input_roots.len() as u64).to_le_bytes());
    for root in input_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(&(output_roots.len() as u64).to_le_bytes());
    for root in output_roots {
        encoded.extend_from_slice(root);
    }
    hash_bytes(b"tensor-vm-ir-trace-op-v1", &[&encoded])
}

#[cfg(feature = "cuda-kernels")]
fn resolve_graph_runtime_ref(
    value: &IrRef,
    inputs: &BTreeMap<String, Tensor>,
    params: &BTreeMap<String, Elem>,
    op_outputs: &[Vec<GraphRuntimeValue>],
) -> Result<GraphRuntimeValue> {
    match value {
        IrRef::Input { name } | IrRef::ConstBlob { uri: name, .. } => inputs
            .get(name)
            .cloned()
            .map(GraphRuntimeValue::Tensor)
            .ok_or(TvmError::InvalidReceipt(
                "unknown tensor ir execution input",
            )),
        IrRef::Op { id, idx } => op_outputs
            .get(*id)
            .and_then(|outputs| outputs.get(*idx))
            .cloned()
            .ok_or(TvmError::InvalidReceipt(
                "unknown tensor ir execution op ref",
            )),
        IrRef::Param { name } => params
            .get(name)
            .copied()
            .map(GraphRuntimeValue::Field)
            .ok_or(TvmError::InvalidReceipt(
                "unknown tensor ir execution param",
            )),
        IrRef::Const { value } => match value {
            crate::ir::IrLiteral::Field(value) => Ok(GraphRuntimeValue::Field(*value)),
            crate::ir::IrLiteral::Int(value) if *value >= 0 => {
                Ok(GraphRuntimeValue::Field(*value as Elem))
            }
            crate::ir::IrLiteral::Uint(value) => Ok(GraphRuntimeValue::Field(*value as Elem)),
            _ => Err(TvmError::InvalidReceipt(
                "unsupported tensor ir execution literal",
            )),
        },
    }
}

#[cfg(feature = "cuda-kernels")]
fn execute_cuda_graph_op(
    device_index: u32,
    op: &str,
    args: &[GraphRuntimeValue],
) -> Result<Vec<GraphRuntimeValue>> {
    let output = match op {
        "add" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_add(device_index, lhs, rhs)?
        }
        "sub" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_sub(device_index, lhs, rhs)?
        }
        "mul" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_mul(device_index, lhs, rhs)?
        }
        "div" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_div(device_index, lhs, rhs)?
        }
        "eq" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_eq(device_index, lhs, rhs)?
        }
        "gt" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_gt(device_index, lhs, rhs)?
        }
        "lt" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_lt(device_index, lhs, rhs)?
        }
        "ge" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_ge(device_index, lhs, rhs)?
        }
        "le" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_le(device_index, lhs, rhs)?
        }
        "where" => {
            let [cond, when_true, when_false] = three_graph_tensor_values(args)?;
            require_graph_int32_tensor(cond)?;
            require_graph_field_tensor(when_true)?;
            require_graph_field_tensor(when_false)?;
            cuda::field_where(device_index, cond, when_true, when_false)?
        }
        "matmul" => {
            let [lhs, rhs] = two_graph_tensor_values(args)?;
            require_graph_field_tensor(lhs)?;
            require_graph_field_tensor(rhs)?;
            cuda::field_matmul(device_index, lhs, rhs)?
        }
        "transpose" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_transpose(device_index, tensor)?
        }
        "relu" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_relu(device_index, tensor)?
        }
        "identity" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_identity(device_index, tensor)?
        }
        "neg" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_neg(device_index, tensor)?
        }
        "abs" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_abs(device_index, tensor)?
        }
        "sign" => {
            let tensor = one_graph_tensor_value(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_sign(device_index, tensor)?
        }
        "scalar_mul" => {
            let (tensor, scalar) = graph_tensor_and_scalar_values(args)?;
            require_graph_field_tensor(tensor)?;
            cuda::field_scalar_mul(device_index, tensor, scalar)?
        }
        _ => return Err(TvmError::InvalidReceipt("cuda graph op not supported")),
    };
    Ok(vec![GraphRuntimeValue::Tensor(output)])
}

#[cfg(feature = "cuda-kernels")]
fn one_graph_tensor_value(values: &[GraphRuntimeValue]) -> Result<&Tensor> {
    match values {
        [GraphRuntimeValue::Tensor(tensor)] => Ok(tensor),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor argument",
        )),
    }
}

#[cfg(feature = "cuda-kernels")]
fn two_graph_tensor_values(values: &[GraphRuntimeValue]) -> Result<[&Tensor; 2]> {
    match values {
        [
            GraphRuntimeValue::Tensor(lhs),
            GraphRuntimeValue::Tensor(rhs),
        ] => Ok([lhs, rhs]),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor arguments",
        )),
    }
}

#[cfg(feature = "cuda-kernels")]
fn three_graph_tensor_values(values: &[GraphRuntimeValue]) -> Result<[&Tensor; 3]> {
    match values {
        [
            GraphRuntimeValue::Tensor(first),
            GraphRuntimeValue::Tensor(second),
            GraphRuntimeValue::Tensor(third),
        ] => Ok([first, second, third]),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor arguments",
        )),
    }
}

#[cfg(feature = "cuda-kernels")]
fn graph_tensor_and_scalar_values(values: &[GraphRuntimeValue]) -> Result<(&Tensor, Elem)> {
    match values {
        [
            GraphRuntimeValue::Tensor(tensor),
            GraphRuntimeValue::Field(scalar),
        ] => Ok((tensor, *scalar)),
        _ => Err(TvmError::InvalidReceipt(
            "tensor ir expected tensor and scalar arguments",
        )),
    }
}

#[cfg(feature = "cuda-kernels")]
fn require_graph_int32_tensor(tensor: &Tensor) -> Result<()> {
    if tensor.dtype() == DType::Int32 && tensor.scale() == 0 {
        Ok(())
    } else {
        Err(TvmError::InvalidReceipt(
            "cuda graph op only supports int32 mask tensors",
        ))
    }
}

#[cfg(feature = "cuda-kernels")]
fn require_graph_field_tensor(tensor: &Tensor) -> Result<()> {
    if tensor.dtype() == DType::FieldElement && tensor.scale() == 0 {
        Ok(())
    } else {
        Err(TvmError::InvalidReceipt(
            "cuda graph op only supports field tensors",
        ))
    }
}

pub fn cuda_kernels_compiled() -> bool {
    cfg!(feature = "cuda-kernels")
}

pub fn cuda_device_count() -> Result<u32> {
    #[cfg(feature = "cuda-kernels")]
    {
        cuda::device_count()
    }
    #[cfg(not(feature = "cuda-kernels"))]
    {
        Ok(0)
    }
}

pub fn backend_conformance_profile<B: ExecutionBackend>(backend: &B) -> Result<ConformanceProfile> {
    match backend.kind() {
        BackendKind::CpuReference => cpu_reference_conformance_profile(),
        BackendKind::GpuMiner { .. } => gpu_backend_conformance_profile(backend),
    }
}

fn gpu_backend_conformance_profile<B: ExecutionBackend>(backend: &B) -> Result<ConformanceProfile> {
    #[cfg(feature = "cuda-kernels")]
    {
        let mut passed_ops = BTreeSet::new();
        let beacon = hash_bytes(b"tensor-vm-conformance-runtime-v1", &[b"matmul"]);
        let matmul = MatmulJob::synthetic(0, 0, 2, 3, 2, &beacon, 10);
        let cpu = CpuReferenceBackend;
        let (_, _, expected_matmul) = cpu.execute_matmul(&matmul)?;
        let (_, _, actual_matmul) = backend.execute_matmul(&matmul)?;
        if expected_matmul != actual_matmul {
            return Err(TvmError::VerificationFailed(
                "gpu matmul conformance failed",
            ));
        }
        passed_ops.insert("matmul");

        let weights = Tensor::from_vec(
            vec![3, 2],
            crate::tensor::DType::FieldElement,
            vec![1, 2, 3, 4, 5, 6],
        )?;
        let linear = LinearTrainingStepJob::from_spec(crate::jobs::LinearTrainingStepSpec {
            model_id: hash_bytes(b"tensor-vm-conformance-runtime-v1", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-conformance-runtime-v1", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![2, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![2, 2],
            lr: 2,
            deadline_block: 10,
        });
        let expected_linear = cpu.execute_linear_training_step(&linear, &weights)?;
        let actual_linear = backend.execute_linear_training_step(&linear, &weights)?;
        if expected_linear != actual_linear {
            return Err(TvmError::VerificationFailed(
                "gpu linear-step conformance failed",
            ));
        }
        passed_ops.extend(["sub", "scalar_mul", "transpose", "mse_loss"]);

        let (graph, inputs, field_params) = supported_cuda_graph_conformance_case()?;
        let graph_id = graph.validate_for_consensus()?;
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let graph_job = GraphJob::new(0, graph_id, input_roots, field_params.clone(), 10, 1, 1);
        let expected_graph =
            cpu.execute_graph_exact(&graph_job, &graph, &inputs, &BTreeMap::new())?;
        let actual_graph =
            backend.execute_graph_exact(&graph_job, &graph, &inputs, &BTreeMap::new())?;
        if expected_graph != actual_graph {
            return Err(TvmError::VerificationFailed("gpu graph conformance failed"));
        }
        passed_ops.extend([
            "add", "mul", "div", "eq", "gt", "lt", "ge", "le", "where", "identity", "neg", "abs",
            "sign", "relu",
        ]);

        Ok(ConformanceProfile {
            suite_hash: conformance_suite_hash(),
            passed_ops,
        })
    }
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let _ = backend;
        Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
    }
}

#[cfg(feature = "cuda-kernels")]
type CudaGraphConformanceCase = (
    TensorGraph,
    BTreeMap<String, Tensor>,
    BTreeMap<String, Elem>,
);

#[cfg(feature = "cuda-kernels")]
fn supported_cuda_graph_conformance_case() -> Result<CudaGraphConformanceCase> {
    let graph = TensorGraph {
        ir_version: 1,
        inputs: vec![
            TensorSpec::field("a", vec![2, 3]),
            TensorSpec::field("b", vec![3, 2]),
            TensorSpec::field("bias", vec![2, 2]),
        ],
        params: vec![ParamSpec {
            name: "scale".to_owned(),
            type_name: "field_scalar".to_owned(),
        }],
        ops: vec![
            OpNode {
                id: 0,
                op: "matmul".to_owned(),
                args: vec![
                    IrRef::Input {
                        name: "a".to_owned(),
                    },
                    IrRef::Input {
                        name: "b".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("product", vec![2, 2])],
            },
            OpNode {
                id: 1,
                op: "add".to_owned(),
                args: vec![
                    IrRef::Op { id: 0, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("biased", vec![2, 2])],
            },
            OpNode {
                id: 2,
                op: "sub".to_owned(),
                args: vec![
                    IrRef::Op { id: 1, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("centered", vec![2, 2])],
            },
            OpNode {
                id: 3,
                op: "mul".to_owned(),
                args: vec![
                    IrRef::Op { id: 2, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("mixed", vec![2, 2])],
            },
            OpNode {
                id: 4,
                op: "transpose".to_owned(),
                args: vec![IrRef::Op { id: 3, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("transposed", vec![2, 2])],
            },
            OpNode {
                id: 5,
                op: "scalar_mul".to_owned(),
                args: vec![
                    IrRef::Op { id: 4, idx: 0 },
                    IrRef::Param {
                        name: "scale".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("scaled", vec![2, 2])],
            },
            OpNode {
                id: 6,
                op: "relu".to_owned(),
                args: vec![IrRef::Op { id: 5, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("activated", vec![2, 2])],
            },
            OpNode {
                id: 7,
                op: "neg".to_owned(),
                args: vec![IrRef::Op { id: 6, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("negated", vec![2, 2])],
            },
            OpNode {
                id: 8,
                op: "abs".to_owned(),
                args: vec![IrRef::Op { id: 7, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("absolute", vec![2, 2])],
            },
            OpNode {
                id: 9,
                op: "sign".to_owned(),
                args: vec![IrRef::Op { id: 8, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("signed", vec![2, 2])],
            },
            OpNode {
                id: 10,
                op: "identity".to_owned(),
                args: vec![IrRef::Op { id: 9, idx: 0 }],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("identity", vec![2, 2])],
            },
            OpNode {
                id: 11,
                op: "eq".to_owned(),
                args: vec![
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: DType::Int32,
                    scale: 0,
                }],
            },
            OpNode {
                id: 12,
                op: "gt".to_owned(),
                args: vec![
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "greater_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: DType::Int32,
                    scale: 0,
                }],
            },
            OpNode {
                id: 13,
                op: "lt".to_owned(),
                args: vec![
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "less_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: DType::Int32,
                    scale: 0,
                }],
            },
            OpNode {
                id: 14,
                op: "ge".to_owned(),
                args: vec![
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "greater_equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: DType::Int32,
                    scale: 0,
                }],
            },
            OpNode {
                id: 15,
                op: "le".to_owned(),
                args: vec![
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec {
                    name: "less_equal_mask".to_owned(),
                    shape: vec![2, 2],
                    dtype: DType::Int32,
                    scale: 0,
                }],
            },
            OpNode {
                id: 16,
                op: "where".to_owned(),
                args: vec![
                    IrRef::Op { id: 15, idx: 0 },
                    IrRef::Op { id: 10, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("selected", vec![2, 2])],
            },
            OpNode {
                id: 17,
                op: "div".to_owned(),
                args: vec![
                    IrRef::Op { id: 16, idx: 0 },
                    IrRef::Input {
                        name: "bias".to_owned(),
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![TensorSpec::field("quotient", vec![2, 2])],
            },
        ],
        outputs: vec![GraphOutput {
            name: "quotient".to_owned(),
            value: IrRef::Op { id: 17, idx: 0 },
        }],
    };
    graph.validate_for_consensus()?;
    let inputs = BTreeMap::from([
        (
            "a".to_owned(),
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])?,
        ),
        (
            "b".to_owned(),
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![7, 8, 9, 10, 11, 12])?,
        ),
        (
            "bias".to_owned(),
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![3, 5, 7, 11])?,
        ),
    ]);
    Ok((graph, inputs, BTreeMap::from([("scale".to_owned(), 3)])))
}

#[cfg(feature = "cuda-kernels")]
mod cuda {
    use super::*;

    unsafe extern "C" {
        fn tensor_vm_cuda_device_count(out: *mut u32) -> i32;
        fn tensor_vm_cuda_field_matmul(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            rows: u64,
            inner: u64,
            cols: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_sub(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_add(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_mul(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_div(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_eq(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_gt(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_lt(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_ge(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_le(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_where(
            device_index: u32,
            cond: *const u64,
            when_true: *const u64,
            when_false: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_relu(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_identity(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_neg(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_abs(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_sign(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_scalar_mul(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            len: u64,
            scalar: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_transpose(
            device_index: u32,
            input: *const u64,
            out: *mut u64,
            rows: u64,
            cols: u64,
        ) -> i32;
        fn tensor_vm_cuda_field_squared_error_sum(
            device_index: u32,
            lhs: *const u64,
            rhs: *const u64,
            out: *mut u64,
            len: u64,
        ) -> i32;
    }

    pub fn device_count() -> Result<u32> {
        let mut count = 0;
        let code = unsafe { tensor_vm_cuda_device_count(&mut count) };
        if code == 0 {
            Ok(count)
        } else {
            Err(cuda_error(code))
        }
    }

    pub fn field_matmul(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        let rows = lhs.rows()?;
        let inner = lhs.cols()?;
        rhs.require_rank_for_cuda_matmul()?;
        if inner != rhs.shape()[0] {
            return Err(TvmError::DimensionMismatch {
                left: lhs.shape().to_vec(),
                right: rhs.shape().to_vec(),
            });
        }
        let cols = rhs.shape()[1];
        let mut out = vec![0; rows * cols];
        let code = unsafe {
            tensor_vm_cuda_field_matmul(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                rows as u64,
                inner as u64,
                cols as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(vec![rows, cols], lhs.dtype(), out)
    }

    pub fn field_add(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        require_same_shape(lhs, rhs)?;
        require_field_element_tensor(lhs)?;
        require_field_element_tensor(rhs)?;
        let mut out = vec![0; lhs.len()];
        let code = unsafe {
            tensor_vm_cuda_field_add(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(lhs.shape().to_vec(), lhs.dtype(), out)
    }

    pub fn field_sub(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        require_same_shape(lhs, rhs)?;
        let mut out = vec![0; lhs.len()];
        let code = unsafe {
            tensor_vm_cuda_field_sub(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(lhs.shape().to_vec(), lhs.dtype(), out)
    }

    pub fn field_mul(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        require_same_shape(lhs, rhs)?;
        require_field_element_tensor(lhs)?;
        require_field_element_tensor(rhs)?;
        let mut out = vec![0; lhs.len()];
        let code = unsafe {
            tensor_vm_cuda_field_mul(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(lhs.shape().to_vec(), lhs.dtype(), out)
    }

    pub fn field_div(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        require_same_shape(lhs, rhs)?;
        require_field_element_tensor(lhs)?;
        require_field_element_tensor(rhs)?;
        let mut out = vec![0; lhs.len()];
        let code = unsafe {
            tensor_vm_cuda_field_div(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(lhs.shape().to_vec(), lhs.dtype(), out)
    }

    pub fn field_eq(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        field_compare(device_index, lhs, rhs, tensor_vm_cuda_field_eq)
    }

    pub fn field_gt(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        field_compare(device_index, lhs, rhs, tensor_vm_cuda_field_gt)
    }

    pub fn field_lt(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        field_compare(device_index, lhs, rhs, tensor_vm_cuda_field_lt)
    }

    pub fn field_ge(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        field_compare(device_index, lhs, rhs, tensor_vm_cuda_field_ge)
    }

    pub fn field_le(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor> {
        field_compare(device_index, lhs, rhs, tensor_vm_cuda_field_le)
    }

    pub fn field_where(
        device_index: u32,
        cond: &Tensor,
        when_true: &Tensor,
        when_false: &Tensor,
    ) -> Result<Tensor> {
        require_same_shape(cond, when_true)?;
        require_same_shape(when_true, when_false)?;
        require_int32_tensor(cond)?;
        require_field_element_tensor(when_true)?;
        require_field_element_tensor(when_false)?;
        let mut out = vec![0; when_true.len()];
        let code = unsafe {
            tensor_vm_cuda_field_where(
                device_index,
                cond.as_slice().as_ptr(),
                when_true.as_slice().as_ptr(),
                when_false.as_slice().as_ptr(),
                out.as_mut_ptr(),
                when_true.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(when_true.shape().to_vec(), when_true.dtype(), out)
    }

    pub fn field_relu(device_index: u32, input: &Tensor) -> Result<Tensor> {
        require_field_element_tensor(input)?;
        let mut out = vec![0; input.len()];
        let code = unsafe {
            tensor_vm_cuda_field_relu(
                device_index,
                input.as_slice().as_ptr(),
                out.as_mut_ptr(),
                input.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(input.shape().to_vec(), input.dtype(), out)
    }

    pub fn field_identity(device_index: u32, input: &Tensor) -> Result<Tensor> {
        field_unary(device_index, input, tensor_vm_cuda_field_identity)
    }

    pub fn field_neg(device_index: u32, input: &Tensor) -> Result<Tensor> {
        field_unary(device_index, input, tensor_vm_cuda_field_neg)
    }

    pub fn field_abs(device_index: u32, input: &Tensor) -> Result<Tensor> {
        field_unary(device_index, input, tensor_vm_cuda_field_abs)
    }

    pub fn field_sign(device_index: u32, input: &Tensor) -> Result<Tensor> {
        field_unary(device_index, input, tensor_vm_cuda_field_sign)
    }

    pub fn field_scalar_mul(device_index: u32, input: &Tensor, scalar: Elem) -> Result<Tensor> {
        let mut out = vec![0; input.len()];
        let code = unsafe {
            tensor_vm_cuda_field_scalar_mul(
                device_index,
                input.as_slice().as_ptr(),
                out.as_mut_ptr(),
                input.len() as u64,
                scalar,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(input.shape().to_vec(), input.dtype(), out)
    }

    pub fn field_transpose(device_index: u32, input: &Tensor) -> Result<Tensor> {
        let rows = input.rows()?;
        let cols = input.cols()?;
        let mut out = vec![0; input.len()];
        let code = unsafe {
            tensor_vm_cuda_field_transpose(
                device_index,
                input.as_slice().as_ptr(),
                out.as_mut_ptr(),
                rows as u64,
                cols as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(vec![cols, rows], input.dtype(), out)
    }

    pub fn field_squared_error_sum(device_index: u32, lhs: &Tensor, rhs: &Tensor) -> Result<Elem> {
        require_same_shape(lhs, rhs)?;
        let mut out = 0;
        let code = unsafe {
            tensor_vm_cuda_field_squared_error_sum(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                &mut out,
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Ok(out)
    }

    fn field_unary(
        device_index: u32,
        input: &Tensor,
        kernel: unsafe extern "C" fn(u32, *const u64, *mut u64, u64) -> i32,
    ) -> Result<Tensor> {
        require_field_element_tensor(input)?;
        let mut out = vec![0; input.len()];
        let code = unsafe {
            kernel(
                device_index,
                input.as_slice().as_ptr(),
                out.as_mut_ptr(),
                input.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(input.shape().to_vec(), input.dtype(), out)
    }

    fn field_compare(
        device_index: u32,
        lhs: &Tensor,
        rhs: &Tensor,
        kernel: unsafe extern "C" fn(u32, *const u64, *const u64, *mut u64, u64) -> i32,
    ) -> Result<Tensor> {
        require_same_shape(lhs, rhs)?;
        require_field_element_tensor(lhs)?;
        require_field_element_tensor(rhs)?;
        let mut out = vec![0; lhs.len()];
        let code = unsafe {
            kernel(
                device_index,
                lhs.as_slice().as_ptr(),
                rhs.as_slice().as_ptr(),
                out.as_mut_ptr(),
                lhs.len() as u64,
            )
        };
        if code != 0 {
            return Err(cuda_error(code));
        }
        Tensor::from_vec(lhs.shape().to_vec(), DType::Int32, out)
    }

    fn require_int32_tensor(tensor: &Tensor) -> Result<()> {
        if tensor.dtype() == DType::Int32 && tensor.scale() == 0 {
            Ok(())
        } else {
            Err(TvmError::InvalidReceipt("cuda tensor must be int32"))
        }
    }

    pub fn field_mse_loss(device_index: u32, y: &Tensor, target: &Tensor) -> Result<Hash> {
        let sum = field_squared_error_sum(device_index, y, target)?;
        Ok(hash_bytes(
            b"tensor-vm-mse-loss-v1",
            &[&sum.to_le_bytes(), &(y.len() as u64).to_le_bytes()],
        ))
    }

    trait CudaMatmulShape {
        fn require_rank_for_cuda_matmul(&self) -> Result<()>;
    }

    impl CudaMatmulShape for Tensor {
        fn require_rank_for_cuda_matmul(&self) -> Result<()> {
            if self.shape().len() == 2 {
                Ok(())
            } else {
                Err(TvmError::UnsupportedRank {
                    rank: self.shape().len(),
                })
            }
        }
    }

    fn require_same_shape(lhs: &Tensor, rhs: &Tensor) -> Result<()> {
        if lhs.shape() == rhs.shape() {
            Ok(())
        } else {
            Err(TvmError::DimensionMismatch {
                left: lhs.shape().to_vec(),
                right: rhs.shape().to_vec(),
            })
        }
    }

    fn require_field_element_tensor(tensor: &Tensor) -> Result<()> {
        if tensor.dtype() == DType::FieldElement && tensor.scale() == 0 {
            Ok(())
        } else {
            Err(TvmError::InvalidReceipt(
                "cuda graph op only supports field tensors",
            ))
        }
    }

    fn cuda_error(code: i32) -> TvmError {
        match code {
            -1 => TvmError::InvalidReceipt("cuda kernel received null pointer"),
            -2 => TvmError::InvalidReceipt("cuda device unavailable or invalid shape"),
            -3 => TvmError::InvalidReceipt("cuda allocation failed"),
            -4 => TvmError::InvalidReceipt("cuda host-device copy failed"),
            -5 => TvmError::InvalidReceipt("cuda kernel execution failed"),
            -6 => TvmError::InvalidReceipt("cuda device index out of range"),
            -7 => TvmError::InvalidReceipt("cuda field division by zero"),
            _ => TvmError::InvalidReceipt("cuda kernel failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "cuda-kernels")]
    use crate::field::MODULUS;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
    use crate::tensor::{DType, Tensor};
    use crate::types::hash_bytes;

    #[test]
    fn cuda_kernel_feature_flag_reports_availability() {
        assert_eq!(CpuReferenceBackend.kind(), BackendKind::CpuReference);
        assert_eq!(cuda_kernels_compiled(), cfg!(feature = "cuda-kernels"));
        #[cfg(not(feature = "cuda-kernels"))]
        assert_eq!(cuda_device_count().unwrap(), 0);
        #[cfg(feature = "cuda-kernels")]
        assert!(cuda_device_count().unwrap() > 0);
    }

    #[test]
    fn cpu_backend_reports_passing_conformance_profile() {
        let profile = backend_conformance_profile(&CpuReferenceBackend).unwrap();
        assert!(profile.passes("matmul"));
        assert!(profile.passes("sub"));
        assert!(profile.passes("scalar_mul"));
        assert!(profile.passes("transpose"));
        assert!(profile.passes("mse_loss"));
    }

    #[test]
    fn gpu_backend_reports_device_and_requires_cuda_kernels() {
        let gpu = GpuMinerBackend::new("cuda:0");
        assert_eq!(
            gpu.kind(),
            BackendKind::GpuMiner {
                device: "cuda:0".to_owned()
            }
        );
        assert_eq!(gpu.device(), "cuda:0");

        #[cfg(not(feature = "cuda-kernels"))]
        {
            let beacon = hash_bytes(b"test", &[b"beacon"]);
            let job = MatmulJob::synthetic(0, 0, 8, 4, 5, &beacon, 10);
            assert!(matches!(
                gpu.execute_matmul(&job),
                Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
            ));
            let weights =
                Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
            let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"model"]),
                step: 0,
                batch_seed: hash_bytes(b"test", &[b"batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 10,
            });
            assert!(matches!(
                gpu.execute_linear_training_step(&linear_job, &weights),
                Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
            ));
            assert_eq!(
                backend_conformance_profile(&gpu),
                Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
            );
            let graph = crate::scheduler::SyntheticLocalJobSource::graph_execution_graph();
            let inputs = crate::scheduler::SyntheticLocalJobSource::graph_execution_inputs();
            let chain = crate::chain::Chain::new(hash_bytes(b"test", &[b"graph-chain"]));
            let mut source = crate::scheduler::SyntheticLocalJobSource::default();
            let graph_job = source.next_graph_job(&chain);
            assert!(matches!(
                gpu.execute_graph_exact(&graph_job, &graph, &inputs, &BTreeMap::new()),
                Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
            ));
        }
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cpu_and_gpu_backends_match_canonical_matmul() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 8, 4, 5, &beacon, 10);
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");
        let (_, _, cpu_out) = cpu.execute_matmul(&job).unwrap();
        let (_, _, gpu_out) = gpu.execute_matmul(&job).unwrap();
        assert_eq!(cpu.kind(), BackendKind::CpuReference);
        assert_eq!(
            gpu.kind(),
            BackendKind::GpuMiner {
                device: "cuda:0".to_owned()
            }
        );
        assert_eq!(gpu.device(), "cuda:0");
        assert_eq!(cpu_out.commitment_root(), gpu_out.commitment_root());
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cpu_and_gpu_backends_match_linear_step() {
        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"model"]),
            step: 0,
            batch_seed: hash_bytes(b"test", &[b"batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 10,
        });
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");
        let cpu_out = cpu.execute_linear_training_step(&job, &weights).unwrap();
        let gpu_out = gpu.execute_linear_training_step(&job, &weights).unwrap();
        assert_eq!(cpu_out.y.commitment_root(), gpu_out.y.commitment_root());
        assert_eq!(cpu_out.dy.commitment_root(), gpu_out.dy.commitment_root());
        assert_eq!(
            cpu_out.grad_w.commitment_root(),
            gpu_out.grad_w.commitment_root()
        );
        assert_eq!(
            cpu_out.weight_after.commitment_root(),
            gpu_out.weight_after.commitment_root()
        );
        assert_eq!(cpu_out.loss_commitment, gpu_out.loss_commitment);
        let gpu_profile = backend_conformance_profile(&gpu).unwrap();
        assert_eq!(
            gpu_profile.suite_hash,
            crate::conformance::conformance_suite_hash()
        );
        for op in [
            "add",
            "sub",
            "matmul",
            "mul",
            "div",
            "eq",
            "gt",
            "lt",
            "ge",
            "le",
            "identity",
            "neg",
            "abs",
            "sign",
            "transpose",
            "relu",
            "scalar_mul",
            "mse_loss",
        ] {
            assert!(gpu_profile.passes(op), "gpu profile missing {op}");
        }
        for op in [
            "sum",
            "mean",
            "einsum",
            "quantize_int8_per_channel",
            "reshape",
        ] {
            assert!(
                !gpu_profile.passes(op),
                "gpu profile must not overclaim {op}"
            );
        }
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cpu_and_gpu_backends_match_synthetic_graph_execution() {
        if cuda_device_count().unwrap_or(0) == 0 {
            return;
        }
        let graph = crate::scheduler::SyntheticLocalJobSource::graph_execution_graph();
        let inputs = crate::scheduler::SyntheticLocalJobSource::graph_execution_inputs();
        let chain = crate::chain::Chain::new(hash_bytes(b"test", &[b"graph-chain"]));
        let mut source = crate::scheduler::SyntheticLocalJobSource::default();
        let job = source.next_graph_job(&chain);
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");

        let cpu_execution = cpu
            .execute_graph_exact(&job, &graph, &inputs, &BTreeMap::new())
            .unwrap();
        let gpu_execution = gpu
            .execute_graph_exact(&job, &graph, &inputs, &BTreeMap::new())
            .unwrap();

        assert_eq!(gpu_execution.graph_id, cpu_execution.graph_id);
        assert_eq!(gpu_execution.trace_root, cpu_execution.trace_root);
        assert_eq!(gpu_execution.op_traces, cpu_execution.op_traces);
        assert_eq!(gpu_execution.outputs, cpu_execution.outputs);
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cpu_and_gpu_backends_match_supported_cuda_graph_ops() {
        if cuda_device_count().unwrap_or(0) == 0 {
            return;
        }
        let (graph, inputs, field_params) = supported_cuda_graph_conformance_case().unwrap();
        let graph_id = graph.validate_for_consensus().unwrap();
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let job = GraphJob::new(0, graph_id, input_roots, field_params, 10, 1, 1);
        let cpu = CpuReferenceBackend;
        let gpu = GpuMinerBackend::new("cuda:0");

        let cpu_execution = cpu
            .execute_graph_exact(&job, &graph, &inputs, &BTreeMap::new())
            .unwrap();
        let gpu_execution = gpu
            .execute_graph_exact(&job, &graph, &inputs, &BTreeMap::new())
            .unwrap();

        assert_eq!(gpu_execution.graph_id, cpu_execution.graph_id);
        assert_eq!(gpu_execution.trace_root, cpu_execution.trace_root);
        assert_eq!(gpu_execution.op_traces, cpu_execution.op_traces);
        assert_eq!(gpu_execution.outputs, cpu_execution.outputs);
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cuda_graph_backend_rejects_unsupported_consensus_ops_explicitly() {
        if cuda_device_count().unwrap_or(0) == 0 {
            return;
        }
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec::field("x", vec![2, 2])],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "sum".to_owned(),
                args: vec![IrRef::Input {
                    name: "x".to_owned(),
                }],
                kwargs: BTreeMap::from([(
                    "dim".to_owned(),
                    crate::ir::IrValue::Literal(crate::ir::IrLiteral::Uint(0)),
                )]),
                out: vec![TensorSpec::field("summed", vec![2])],
            }],
            outputs: vec![GraphOutput {
                name: "summed".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let input = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let job = GraphJob::new(
            0,
            graph_id,
            BTreeMap::from([("x".to_owned(), input.commitment_root())]),
            BTreeMap::new(),
            10,
            1,
            1,
        );

        assert!(matches!(
            GpuMinerBackend::new("cuda:0").execute_graph_exact(
                &job,
                &graph,
                &inputs,
                &BTreeMap::new()
            ),
            Err(TvmError::InvalidReceipt("cuda graph op not supported"))
        ));
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cuda_kernel_matches_canonical_field_matmul_edges() {
        assert!(cuda_kernels_compiled());
        assert!(cuda_device_count().unwrap() > 0);
        let lhs = Tensor::from_vec(
            vec![2, 3],
            DType::FieldElement,
            vec![MODULUS - 1, 2, 3, 4, MODULUS - 2, 6],
        )
        .unwrap();
        let rhs = Tensor::from_vec(
            vec![3, 2],
            DType::FieldElement,
            vec![7, 8, MODULUS - 3, 10, 11, MODULUS - 4],
        )
        .unwrap();
        let expected = lhs.matmul(&rhs).unwrap();
        let actual = cuda::field_matmul(0, &lhs, &rhs).unwrap();
        assert_eq!(actual.as_slice(), expected.as_slice());
        assert_eq!(actual.commitment_root(), expected.commitment_root());
    }

    #[cfg(feature = "cuda-kernels")]
    #[test]
    fn cuda_kernels_match_canonical_linear_tensor_ops() {
        let lhs = Tensor::from_vec(
            vec![2, 3],
            DType::FieldElement,
            vec![MODULUS - 1, 0, 5, 11, MODULUS - 3, 9],
        )
        .unwrap();
        let rhs = Tensor::from_vec(
            vec![2, 3],
            DType::FieldElement,
            vec![2, 3, MODULUS - 2, 7, 8, MODULUS - 5],
        )
        .unwrap();

        let sub = cuda::field_sub(0, &lhs, &rhs).unwrap();
        assert_eq!(sub, lhs.sub(&rhs).unwrap());

        let add = cuda::field_add(0, &lhs, &rhs).unwrap();
        assert_eq!(add, lhs.add(&rhs).unwrap());

        let multiplied = cuda::field_mul(0, &lhs, &rhs).unwrap();
        assert_eq!(multiplied, lhs.mul(&rhs).unwrap());

        let divided = cuda::field_div(0, &lhs, &rhs).unwrap();
        assert_eq!(divided, lhs.div(&rhs).unwrap());

        let zero_divisor =
            Tensor::from_vec(rhs.shape().to_vec(), rhs.dtype(), vec![1, 2, 0, 4, 5, 6]).unwrap();
        assert!(matches!(
            cuda::field_div(0, &lhs, &zero_divisor),
            Err(TvmError::InvalidReceipt("cuda field division by zero"))
        ));

        let expected_compare = |predicate: fn(Elem, Elem) -> bool| {
            Tensor::from_vec(
                lhs.shape().to_vec(),
                DType::Int32,
                lhs.as_slice()
                    .iter()
                    .zip(rhs.as_slice())
                    .map(|(left, right)| {
                        if predicate(*left % MODULUS, *right % MODULUS) {
                            1
                        } else {
                            0
                        }
                    })
                    .collect(),
            )
            .unwrap()
        };
        assert_eq!(
            cuda::field_eq(0, &lhs, &rhs).unwrap(),
            expected_compare(|left, right| left == right)
        );
        assert_eq!(
            cuda::field_gt(0, &lhs, &rhs).unwrap(),
            expected_compare(|left, right| left > right)
        );
        assert_eq!(
            cuda::field_lt(0, &lhs, &rhs).unwrap(),
            expected_compare(|left, right| left < right)
        );
        assert_eq!(
            cuda::field_ge(0, &lhs, &rhs).unwrap(),
            expected_compare(|left, right| left >= right)
        );
        assert_eq!(
            cuda::field_le(0, &lhs, &rhs).unwrap(),
            expected_compare(|left, right| left <= right)
        );

        let mask =
            Tensor::from_vec(lhs.shape().to_vec(), DType::Int32, vec![1, 0, 2, 0, 1, 0]).unwrap();
        let expected_where = Tensor::from_vec(
            lhs.shape().to_vec(),
            lhs.dtype(),
            mask.as_slice()
                .iter()
                .zip(lhs.as_slice())
                .zip(rhs.as_slice())
                .map(|((cond, when_true), when_false)| {
                    if *cond == 0 {
                        *when_false % MODULUS
                    } else {
                        *when_true % MODULUS
                    }
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(
            cuda::field_where(0, &mask, &lhs, &rhs).unwrap(),
            expected_where
        );

        let relu = cuda::field_relu(0, &lhs).unwrap();
        let expected_relu = Tensor::from_vec(
            lhs.shape().to_vec(),
            lhs.dtype(),
            lhs.as_slice()
                .iter()
                .map(|value| if *value > MODULUS / 2 { 0 } else { *value })
                .collect(),
        )
        .unwrap();
        assert_eq!(relu, expected_relu);

        let identity = cuda::field_identity(0, &lhs).unwrap();
        assert_eq!(identity, lhs);

        let neg = cuda::field_neg(0, &lhs).unwrap();
        let expected_neg = Tensor::from_vec(
            lhs.shape().to_vec(),
            lhs.dtype(),
            lhs.as_slice()
                .iter()
                .map(|value| if *value == 0 { 0 } else { MODULUS - *value })
                .collect(),
        )
        .unwrap();
        assert_eq!(neg, expected_neg);

        let abs = cuda::field_abs(0, &lhs).unwrap();
        let expected_abs = Tensor::from_vec(
            lhs.shape().to_vec(),
            lhs.dtype(),
            lhs.as_slice()
                .iter()
                .map(|value| {
                    if *value > MODULUS / 2 {
                        MODULUS - *value
                    } else {
                        *value
                    }
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(abs, expected_abs);

        let sign = cuda::field_sign(0, &lhs).unwrap();
        let expected_sign = Tensor::from_vec(
            lhs.shape().to_vec(),
            lhs.dtype(),
            lhs.as_slice()
                .iter()
                .map(|value| {
                    if *value == 0 {
                        0
                    } else if *value > MODULUS / 2 {
                        MODULUS - 1
                    } else {
                        1
                    }
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(sign, expected_sign);

        let scaled = cuda::field_scalar_mul(0, &lhs, MODULUS + 2).unwrap();
        assert_eq!(scaled, lhs.scalar_mul(MODULUS + 2).unwrap());

        let transposed = cuda::field_transpose(0, &lhs).unwrap();
        assert_eq!(transposed, lhs.transpose().unwrap());

        let squared_error_sum = cuda::field_squared_error_sum(0, &lhs, &rhs).unwrap();
        assert_eq!(squared_error_sum, lhs.squared_error_sum(&rhs).unwrap());

        let loss = cuda::field_mse_loss(0, &lhs, &rhs).unwrap();
        assert_eq!(loss, crate::vm::mse_loss(&lhs, &rhs).unwrap());
    }
}
