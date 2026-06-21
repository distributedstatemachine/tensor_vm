use std::collections::BTreeMap;

use crate::error::{Result, TvmError};
use crate::field::Elem;
use crate::ir::{
    IrExecution, IrExecutionInputs, TensorGraph, canonical_linear_training_step_graph,
    canonical_matmul_graph,
};
use crate::tensor::{DType, Tensor};
use crate::types::{Address, Hash, Signature, hash_bytes, sign};
use crate::vm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    TensorOp,
    LinearTrainingStep,
    GraphExecution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphJob {
    pub job_id: Hash,
    pub epoch: u64,
    pub graph_id: Hash,
    pub input_roots: BTreeMap<String, Hash>,
    pub field_params: BTreeMap<String, Elem>,
    pub deadline_block: u64,
    pub reward_weight: u64,
    pub declared_tensor_work_units: u64,
}

impl GraphJob {
    pub fn new(
        epoch: u64,
        graph_id: Hash,
        input_roots: BTreeMap<String, Hash>,
        field_params: BTreeMap<String, Elem>,
        deadline_block: u64,
        reward_weight: u64,
        declared_tensor_work_units: u64,
    ) -> Self {
        let job_id = graph_job_id(GraphJobIdInput {
            epoch,
            graph_id: &graph_id,
            input_roots: &input_roots,
            field_params: &field_params,
            deadline_block,
            reward_weight,
            declared_tensor_work_units,
        });
        Self {
            job_id,
            epoch,
            graph_id,
            input_roots,
            field_params,
            deadline_block,
            reward_weight,
            declared_tensor_work_units,
        }
    }

    pub fn recompute_job_id(&self) -> Hash {
        graph_job_id(GraphJobIdInput {
            epoch: self.epoch,
            graph_id: &self.graph_id,
            input_roots: &self.input_roots,
            field_params: &self.field_params,
            deadline_block: self.deadline_block,
            reward_weight: self.reward_weight,
            declared_tensor_work_units: self.declared_tensor_work_units,
        })
    }

    pub fn tensor_work_units(&self) -> u64 {
        self.declared_tensor_work_units
            .saturating_mul(self.reward_weight)
    }

    pub fn program_hash(&self) -> Hash {
        self.graph_id
    }

    pub fn exact_ir_execution(
        &self,
        graph: &TensorGraph,
        tensors: &BTreeMap<String, Tensor>,
    ) -> Result<IrExecution> {
        self.exact_ir_execution_with_const_blobs(graph, tensors, &BTreeMap::new())
    }

    pub fn exact_ir_execution_with_const_blobs(
        &self,
        graph: &TensorGraph,
        tensors: &BTreeMap<String, Tensor>,
        const_blobs: &BTreeMap<String, Tensor>,
    ) -> Result<IrExecution> {
        for (name, expected_root) in &self.input_roots {
            let Some(tensor) = tensors.get(name) else {
                return Err(TvmError::InvalidReceipt("missing graph input tensor"));
            };
            if tensor.commitment_root() != *expected_root {
                return Err(TvmError::InvalidReceipt("graph input root mismatch"));
            }
        }
        if tensors.len() != self.input_roots.len() {
            return Err(TvmError::InvalidReceipt("unexpected graph input tensor"));
        }
        let mut execution_tensors = tensors.clone();
        for (uri, tensor) in const_blobs {
            if execution_tensors
                .insert(uri.clone(), tensor.clone())
                .is_some()
            {
                return Err(TvmError::InvalidReceipt("graph const_blob input collision"));
            }
        }
        graph.execute_exact(&IrExecutionInputs {
            tensors: execution_tensors,
            field_params: self.field_params.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphReceipt {
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub miner: Address,
    pub graph_id: Hash,
    pub input_roots: BTreeMap<String, Hash>,
    pub output_roots: BTreeMap<String, Hash>,
    pub trace_root: Hash,
    pub tensor_work_units: u64,
    pub execution_time_ms: u64,
    pub submitted_at_block: u64,
    pub signature: Signature,
}

impl GraphReceipt {
    pub fn from_execution(
        job: &GraphJob,
        graph: &TensorGraph,
        miner: Address,
        tensors: &BTreeMap<String, Tensor>,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(Self, BTreeMap<String, Tensor>)> {
        Self::from_execution_with_const_blobs(
            job,
            graph,
            miner,
            tensors,
            &BTreeMap::new(),
            submitted_at_block,
            execution_time_ms,
        )
    }

    pub fn from_execution_with_const_blobs(
        job: &GraphJob,
        graph: &TensorGraph,
        miner: Address,
        tensors: &BTreeMap<String, Tensor>,
        const_blobs: &BTreeMap<String, Tensor>,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(Self, BTreeMap<String, Tensor>)> {
        let execution = job.exact_ir_execution_with_const_blobs(graph, tensors, const_blobs)?;
        let output_roots = execution
            .outputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect::<BTreeMap<_, _>>();
        let receipt = Self::from_roots(
            job,
            miner,
            output_roots,
            execution.trace_root,
            submitted_at_block,
            execution_time_ms,
        );
        Ok((receipt, execution.outputs))
    }

    pub fn from_roots(
        job: &GraphJob,
        miner: Address,
        output_roots: BTreeMap<String, Hash>,
        trace_root: Hash,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Self {
        let unsigned = graph_receipt_digest(GraphReceiptDigestInput {
            job_id: &job.job_id,
            miner: &miner,
            graph_id: &job.graph_id,
            input_roots: &job.input_roots,
            output_roots: &output_roots,
            trace_root: &trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
        });
        Self {
            receipt_id: unsigned,
            job_id: job.job_id,
            miner,
            graph_id: job.graph_id,
            input_roots: job.input_roots.clone(),
            output_roots,
            trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
            signature: sign(&miner, &unsigned),
        }
    }

    pub fn recompute_receipt_id(&self) -> Hash {
        graph_receipt_digest(GraphReceiptDigestInput {
            job_id: &self.job_id,
            miner: &self.miner,
            graph_id: &self.graph_id,
            input_roots: &self.input_roots,
            output_roots: &self.output_roots,
            trace_root: &self.trace_root,
            tensor_work_units: self.tensor_work_units,
            execution_time_ms: self.execution_time_ms,
            submitted_at_block: self.submitted_at_block,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatmulJob {
    pub job_id: Hash,
    pub epoch: u64,
    pub m: usize,
    pub k: usize,
    pub n: usize,
    pub dtype: DType,
    pub modulus: Option<Elem>,
    pub seed_a: Hash,
    pub seed_b: Hash,
    pub deadline_block: u64,
    pub reward_weight: u64,
}

impl MatmulJob {
    pub fn synthetic(
        epoch: u64,
        nonce: u64,
        m: usize,
        k: usize,
        n: usize,
        beacon: &Hash,
        deadline_block: u64,
    ) -> Self {
        let job_id = hash_bytes(
            b"tensor-vm-matmul-job-v1",
            &[
                beacon,
                &epoch.to_le_bytes(),
                &nonce.to_le_bytes(),
                &(m as u64).to_le_bytes(),
                &(k as u64).to_le_bytes(),
                &(n as u64).to_le_bytes(),
            ],
        );
        let seed_a = hash_bytes(b"tensor-vm-matmul-a-v1", &[&job_id]);
        let seed_b = hash_bytes(b"tensor-vm-matmul-b-v1", &[&job_id]);
        Self {
            job_id,
            epoch,
            m,
            k,
            n,
            dtype: DType::FieldElement,
            modulus: Some(crate::field::MODULUS),
            seed_a,
            seed_b,
            deadline_block,
            reward_weight: 1,
        }
    }

    pub fn input_tensors(&self) -> Result<(Tensor, Tensor)> {
        let a = Tensor::random(&self.seed_a, vec![self.m, self.k], self.dtype)?;
        let b = Tensor::random(&self.seed_b, vec![self.k, self.n], self.dtype)?;
        Ok((a, b))
    }

    pub fn execute(&self) -> Result<(Tensor, Tensor, Tensor)> {
        let (a, b) = self.input_tensors()?;
        let c = a.matmul(&b)?;
        Ok((a, b, c))
    }

    pub fn tensor_work_units(&self) -> u64 {
        2_u64
            .saturating_mul(self.m as u64)
            .saturating_mul(self.k as u64)
            .saturating_mul(self.n as u64)
            .saturating_mul(self.reward_weight)
    }

    pub fn program_hash(&self) -> Hash {
        self.tensor_ir_graph().graph_id()
    }

    pub fn tensor_ir_graph(&self) -> TensorGraph {
        canonical_matmul_graph(self.m, self.k, self.n, self.dtype)
    }

    pub fn exact_ir_execution(&self, a: &Tensor, b: &Tensor) -> Result<IrExecution> {
        self.tensor_ir_graph().execute_exact(&IrExecutionInputs {
            tensors: BTreeMap::from([("a".to_owned(), a.clone()), ("b".to_owned(), b.clone())]),
            field_params: BTreeMap::new(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorOpReceipt {
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub miner: Address,
    pub program_hash: Hash,
    pub input_roots: Vec<Hash>,
    pub output_roots: Vec<Hash>,
    pub trace_root: Hash,
    pub tensor_work_units: u64,
    pub execution_time_ms: u64,
    pub submitted_at_block: u64,
    pub signature: Signature,
}

impl TensorOpReceipt {
    pub fn from_job(
        job: &MatmulJob,
        miner: Address,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(Self, Tensor, Tensor, Tensor)> {
        let (a, b, c) = job.execute()?;
        let receipt = Self::from_output(
            job,
            miner,
            submitted_at_block,
            execution_time_ms,
            &a,
            &b,
            &c,
        )?;
        Ok((receipt, a, b, c))
    }

    pub fn from_output(
        job: &MatmulJob,
        miner: Address,
        submitted_at_block: u64,
        execution_time_ms: u64,
        a: &Tensor,
        b: &Tensor,
        c: &Tensor,
    ) -> Result<Self> {
        let input_roots = vec![a.commitment_root(), b.commitment_root()];
        let output_roots = vec![c.commitment_root()];
        let execution = job.exact_ir_execution(a, b)?;
        if execution.outputs.get("c") != Some(c) {
            return Err(TvmError::InvalidReceipt("tensor ir output mismatch"));
        }
        let trace_root = execution.trace_root;
        let unsigned = receipt_digest(ReceiptDigestInput {
            domain: b"tensor-vm-tensorop-receipt-v1",
            job_id: &job.job_id,
            miner: &miner,
            program_hash: &job.program_hash(),
            input_roots: &input_roots,
            output_roots: &output_roots,
            trace_root: &trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
        });
        Ok(Self {
            receipt_id: unsigned,
            job_id: job.job_id,
            miner,
            program_hash: job.program_hash(),
            input_roots,
            output_roots,
            trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
            signature: sign(&miner, &unsigned),
        })
    }

    pub fn recompute_receipt_id(&self) -> Hash {
        receipt_digest(ReceiptDigestInput {
            domain: b"tensor-vm-tensorop-receipt-v1",
            job_id: &self.job_id,
            miner: &self.miner,
            program_hash: &self.program_hash,
            input_roots: &self.input_roots,
            output_roots: &self.output_roots,
            trace_root: &self.trace_root,
            tensor_work_units: self.tensor_work_units,
            execution_time_ms: self.execution_time_ms,
            submitted_at_block: self.submitted_at_block,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearTrainingStepJob {
    pub job_id: Hash,
    pub model_id: Hash,
    pub step: u64,
    pub batch_seed: Hash,
    pub weight_root_before: Hash,
    pub input_shape: Vec<usize>,
    pub weight_shape: Vec<usize>,
    pub target_shape: Vec<usize>,
    pub lr: Elem,
    pub dtype: DType,
    pub deadline_block: u64,
    pub reward_weight: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearTrainingStepSpec {
    pub model_id: Hash,
    pub step: u64,
    pub batch_seed: Hash,
    pub weight_root_before: Hash,
    pub input_shape: Vec<usize>,
    pub weight_shape: Vec<usize>,
    pub target_shape: Vec<usize>,
    pub lr: Elem,
    pub deadline_block: u64,
}

impl LinearTrainingStepJob {
    pub fn from_spec(spec: LinearTrainingStepSpec) -> Self {
        let job_id = hash_bytes(
            b"tensor-vm-linear-step-job-v1",
            &[
                &spec.model_id,
                &spec.step.to_le_bytes(),
                &spec.batch_seed,
                &spec.weight_root_before,
                &encode_usizes(&spec.input_shape),
                &encode_usizes(&spec.weight_shape),
                &encode_usizes(&spec.target_shape),
                &spec.lr.to_le_bytes(),
            ],
        );
        Self {
            job_id,
            model_id: spec.model_id,
            step: spec.step,
            batch_seed: spec.batch_seed,
            weight_root_before: spec.weight_root_before,
            input_shape: spec.input_shape,
            weight_shape: spec.weight_shape,
            target_shape: spec.target_shape,
            lr: spec.lr,
            dtype: DType::FieldElement,
            deadline_block: spec.deadline_block,
            reward_weight: 1,
        }
    }

    pub fn batch_tensors(&self) -> Result<(Tensor, Tensor)> {
        let x_seed = hash_bytes(b"tensor-vm-linear-x-v1", &[&self.batch_seed]);
        let t_seed = hash_bytes(b"tensor-vm-linear-target-v1", &[&self.batch_seed]);
        let x = Tensor::random(&x_seed, self.input_shape.clone(), self.dtype)?;
        let target = Tensor::random(&t_seed, self.target_shape.clone(), self.dtype)?;
        Ok((x, target))
    }

    pub fn execute(&self, weights: &Tensor) -> Result<LinearTrainingStepOutput> {
        if weights.commitment_root() != self.weight_root_before {
            return Err(TvmError::InvalidReceipt("weight root mismatch"));
        }
        let (x, target) = self.batch_tensors()?;
        let y = x.matmul(weights)?;
        let dy = y.sub(&target)?;
        let grad_w = x.transpose()?.matmul(&dy)?;
        let weight_after = weights.sub(&grad_w.scalar_mul(self.lr)?)?;
        let loss_commitment = vm::mse_loss(&y, &target)?;
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

    pub fn tensor_work_units(&self) -> u64 {
        let batch = *self.input_shape.first().unwrap_or(&0) as u64;
        let input_dim = *self.input_shape.get(1).unwrap_or(&0) as u64;
        let output_dim = *self.weight_shape.get(1).unwrap_or(&0) as u64;
        let forward = 2_u64
            .saturating_mul(batch)
            .saturating_mul(input_dim)
            .saturating_mul(output_dim);
        let backward = forward;
        let update = input_dim.saturating_mul(output_dim).saturating_mul(2);
        forward
            .saturating_add(backward)
            .saturating_add(update)
            .saturating_mul(self.reward_weight)
    }

    pub fn program_hash(&self) -> Hash {
        self.tensor_ir_graph().graph_id()
    }

    pub fn tensor_ir_graph(&self) -> TensorGraph {
        canonical_linear_training_step_graph(
            &self.input_shape,
            &self.weight_shape,
            &self.target_shape,
            self.dtype,
        )
    }

    pub fn exact_ir_execution(
        &self,
        weights: &Tensor,
        output: &LinearTrainingStepOutput,
    ) -> Result<IrExecution> {
        self.tensor_ir_graph().execute_exact(&IrExecutionInputs {
            tensors: BTreeMap::from([
                ("x".to_owned(), output.x.clone()),
                ("w".to_owned(), weights.clone()),
                ("target".to_owned(), output.target.clone()),
            ]),
            field_params: BTreeMap::from([("lr".to_owned(), self.lr)]),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearTrainingStepOutput {
    pub x: Tensor,
    pub target: Tensor,
    pub y: Tensor,
    pub dy: Tensor,
    pub grad_w: Tensor,
    pub weight_after: Tensor,
    pub loss_commitment: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearTrainingStepReceipt {
    pub receipt_id: Hash,
    pub job_id: Hash,
    pub miner: Address,
    pub model_id: Hash,
    pub step: u64,
    pub weight_root_before: Hash,
    pub batch_root: Hash,
    pub y_root: Hash,
    pub loss_commitment: Hash,
    pub grad_w_root: Hash,
    pub weight_root_after: Hash,
    pub trace_root: Hash,
    pub tensor_work_units: u64,
    pub execution_time_ms: u64,
    pub submitted_at_block: u64,
    pub signature: Signature,
}

impl LinearTrainingStepReceipt {
    pub fn from_job(
        job: &LinearTrainingStepJob,
        miner: Address,
        weights: &Tensor,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(Self, LinearTrainingStepOutput)> {
        let output = job.execute(weights)?;
        let receipt = Self::from_output(
            job,
            miner,
            weights,
            &output,
            submitted_at_block,
            execution_time_ms,
        )?;
        Ok((receipt, output))
    }

    pub fn from_output(
        job: &LinearTrainingStepJob,
        miner: Address,
        weights_before: &Tensor,
        output: &LinearTrainingStepOutput,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<Self> {
        let batch_root = hash_bytes(
            b"tensor-vm-linear-batch-root-v1",
            &[
                &output.x.commitment_root(),
                &output.target.commitment_root(),
            ],
        );
        let execution = job.exact_ir_execution(weights_before, output)?;
        if execution.outputs.get("y") != Some(&output.y)
            || execution.outputs.get("dy") != Some(&output.dy)
            || execution.outputs.get("grad_w") != Some(&output.grad_w)
            || execution.outputs.get("weight_after") != Some(&output.weight_after)
        {
            return Err(TvmError::InvalidReceipt("tensor ir output mismatch"));
        }
        let trace_root = execution.trace_root;
        let unsigned = receipt_digest(ReceiptDigestInput {
            domain: b"tensor-vm-linear-receipt-v1",
            job_id: &job.job_id,
            miner: &miner,
            program_hash: &job.program_hash(),
            input_roots: &[job.weight_root_before, batch_root],
            output_roots: &[
                output.y.commitment_root(),
                output.grad_w.commitment_root(),
                output.weight_after.commitment_root(),
            ],
            trace_root: &trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
        });
        Ok(Self {
            receipt_id: unsigned,
            job_id: job.job_id,
            miner,
            model_id: job.model_id,
            step: job.step,
            weight_root_before: job.weight_root_before,
            batch_root,
            y_root: output.y.commitment_root(),
            loss_commitment: output.loss_commitment,
            grad_w_root: output.grad_w.commitment_root(),
            weight_root_after: output.weight_after.commitment_root(),
            trace_root,
            tensor_work_units: job.tensor_work_units(),
            execution_time_ms,
            submitted_at_block,
            signature: sign(&miner, &unsigned),
        })
    }

    pub fn recompute_receipt_id(&self, program_hash: &Hash) -> Hash {
        receipt_digest(ReceiptDigestInput {
            domain: b"tensor-vm-linear-receipt-v1",
            job_id: &self.job_id,
            miner: &self.miner,
            program_hash,
            input_roots: &[self.weight_root_before, self.batch_root],
            output_roots: &[self.y_root, self.grad_w_root, self.weight_root_after],
            trace_root: &self.trace_root,
            tensor_work_units: self.tensor_work_units,
            execution_time_ms: self.execution_time_ms,
            submitted_at_block: self.submitted_at_block,
        })
    }
}

struct ReceiptDigestInput<'a> {
    domain: &'a [u8],
    job_id: &'a Hash,
    miner: &'a Address,
    program_hash: &'a Hash,
    input_roots: &'a [Hash],
    output_roots: &'a [Hash],
    trace_root: &'a Hash,
    tensor_work_units: u64,
    execution_time_ms: u64,
    submitted_at_block: u64,
}

struct GraphJobIdInput<'a> {
    epoch: u64,
    graph_id: &'a Hash,
    input_roots: &'a BTreeMap<String, Hash>,
    field_params: &'a BTreeMap<String, Elem>,
    deadline_block: u64,
    reward_weight: u64,
    declared_tensor_work_units: u64,
}

fn graph_job_id(input: GraphJobIdInput<'_>) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&input.epoch.to_le_bytes());
    encoded.extend_from_slice(input.graph_id);
    encode_named_hashes(&mut encoded, input.input_roots);
    encoded.extend_from_slice(&(input.field_params.len() as u64).to_le_bytes());
    for (name, value) in input.field_params {
        encode_name(&mut encoded, name);
        encoded.extend_from_slice(&value.to_le_bytes());
    }
    encoded.extend_from_slice(&input.deadline_block.to_le_bytes());
    encoded.extend_from_slice(&input.reward_weight.to_le_bytes());
    encoded.extend_from_slice(&input.declared_tensor_work_units.to_le_bytes());
    hash_bytes(b"tensor-vm-graph-job-v1", &[&encoded])
}

struct GraphReceiptDigestInput<'a> {
    job_id: &'a Hash,
    miner: &'a Address,
    graph_id: &'a Hash,
    input_roots: &'a BTreeMap<String, Hash>,
    output_roots: &'a BTreeMap<String, Hash>,
    trace_root: &'a Hash,
    tensor_work_units: u64,
    execution_time_ms: u64,
    submitted_at_block: u64,
}

fn graph_receipt_digest(input: GraphReceiptDigestInput<'_>) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(input.job_id);
    encoded.extend_from_slice(input.miner);
    encoded.extend_from_slice(input.graph_id);
    encode_named_hashes(&mut encoded, input.input_roots);
    encode_named_hashes(&mut encoded, input.output_roots);
    encoded.extend_from_slice(input.trace_root);
    encoded.extend_from_slice(&input.tensor_work_units.to_le_bytes());
    encoded.extend_from_slice(&input.execution_time_ms.to_le_bytes());
    encoded.extend_from_slice(&input.submitted_at_block.to_le_bytes());
    hash_bytes(b"tensor-vm-graph-receipt-v1", &[&encoded])
}

fn receipt_digest(input: ReceiptDigestInput<'_>) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(input.job_id);
    encoded.extend_from_slice(input.miner);
    encoded.extend_from_slice(input.program_hash);
    encoded.extend_from_slice(&(input.input_roots.len() as u64).to_le_bytes());
    for root in input.input_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(&(input.output_roots.len() as u64).to_le_bytes());
    for root in input.output_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(input.trace_root);
    encoded.extend_from_slice(&input.tensor_work_units.to_le_bytes());
    encoded.extend_from_slice(&input.execution_time_ms.to_le_bytes());
    encoded.extend_from_slice(&input.submitted_at_block.to_le_bytes());
    hash_bytes(input.domain, &[&encoded])
}

fn encode_named_hashes(out: &mut Vec<u8>, values: &BTreeMap<String, Hash>) {
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for (name, root) in values {
        encode_name(out, name);
        out.extend_from_slice(root);
    }
}

fn encode_name(out: &mut Vec<u8>, name: &str) {
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
}

fn encode_usizes(values: &[usize]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + values.len() * 8);
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        out.extend_from_slice(&(*value as u64).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{GraphOutput, IrRef, OpNode, canonical_matmul_graph};
    use crate::types::address;

    #[test]
    fn matmul_receipt_commits_to_outputs() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 1, 4, 3, 2, &beacon, 10);
        assert_eq!(
            job.program_hash(),
            job.tensor_ir_graph().validate_for_consensus().unwrap()
        );
        let miner = address(b"miner");
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 7).unwrap();
        assert_eq!(receipt.output_roots, vec![c.commitment_root()]);
        assert_eq!(
            receipt.trace_root,
            job.exact_ir_execution(&a, &b).unwrap().trace_root
        );
        assert_eq!(receipt.tensor_work_units, 48);
        assert_eq!(receipt.program_hash, job.tensor_ir_graph().graph_id());
    }

    #[test]
    fn linear_receipt_commits_to_learning_step() {
        let seed = hash_bytes(b"test", &[b"batch"]);
        let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let model = hash_bytes(b"test", &[b"model"]);
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: model,
            step: 0,
            batch_seed: seed,
            weight_root_before: weights.commitment_root(),
            input_shape: vec![3, 2],
            weight_shape: vec![2, 2],
            target_shape: vec![3, 2],
            lr: 2,
            deadline_block: 20,
        });
        assert_eq!(
            job.program_hash(),
            job.tensor_ir_graph().validate_for_consensus().unwrap()
        );
        let (receipt, output) =
            LinearTrainingStepReceipt::from_job(&job, address(b"miner"), &weights, 3, 9).unwrap();
        assert_eq!(receipt.y_root, output.y.commitment_root());
        assert_eq!(receipt.grad_w_root, output.grad_w.commitment_root());
        assert_eq!(
            receipt.weight_root_after,
            output.weight_after.commitment_root()
        );
        assert_eq!(
            receipt.trace_root,
            job.exact_ir_execution(&weights, &output)
                .unwrap()
                .trace_root
        );
        assert_eq!(
            receipt.receipt_id,
            receipt.recompute_receipt_id(&job.program_hash())
        );

        let wrong_weights =
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![4, 3, 2, 1]).unwrap();
        assert_eq!(
            job.execute(&wrong_weights),
            Err(TvmError::InvalidReceipt("weight root mismatch"))
        );
    }

    #[test]
    fn graph_receipt_commits_to_exact_graph_execution() {
        let graph = canonical_matmul_graph(2, 2, 2, DType::FieldElement);
        let graph_id = graph.validate_for_consensus().unwrap();
        let a = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let b = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![5, 6, 7, 8]).unwrap();
        let inputs = BTreeMap::from([("a".to_owned(), a.clone()), ("b".to_owned(), b.clone())]);
        let input_roots = inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.commitment_root()))
            .collect();
        let job = GraphJob::new(3, graph_id, input_roots, BTreeMap::new(), 20, 2, 8);

        let (receipt, outputs) =
            GraphReceipt::from_execution(&job, &graph, address(b"graph-miner"), &inputs, 4, 9)
                .unwrap();
        let execution = job.exact_ir_execution(&graph, &inputs).unwrap();

        assert_eq!(receipt.graph_id, graph_id);
        assert_eq!(receipt.output_roots["c"], outputs["c"].commitment_root());
        assert_eq!(
            receipt.output_roots["c"],
            execution.outputs["c"].commitment_root()
        );
        assert_eq!(receipt.trace_root, execution.trace_root);
        assert_eq!(receipt.tensor_work_units, 16);
        assert_eq!(receipt.receipt_id, receipt.recompute_receipt_id());
        assert_eq!(
            job.exact_ir_execution(&graph, &BTreeMap::new()),
            Err(TvmError::InvalidReceipt("missing graph input tensor"))
        );
    }

    #[test]
    fn graph_receipt_replay_supports_const_blob_artifacts() {
        let input = Tensor::from_vec(vec![2], DType::FieldElement, vec![3, 4]).unwrap();
        let blob = Tensor::from_vec(vec![2], DType::FieldElement, vec![7, 8]).unwrap();
        let blob_uri = crate::hash::hex(&blob.commitment_root());
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![crate::ir::TensorSpec::field("x", vec![2])],
            params: Vec::new(),
            ops: vec![OpNode {
                id: 0,
                op: "add".to_owned(),
                args: vec![
                    IrRef::Input {
                        name: "x".to_owned(),
                    },
                    IrRef::ConstBlob {
                        uri: blob_uri.clone(),
                        shape: vec![2],
                        dtype: DType::FieldElement,
                    },
                ],
                kwargs: BTreeMap::new(),
                out: vec![crate::ir::TensorSpec::field("y", vec![2])],
            }],
            outputs: vec![GraphOutput {
                name: "y".to_owned(),
                value: IrRef::Op { id: 0, idx: 0 },
            }],
        };
        let graph_id = graph.validate_for_consensus().unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let const_blobs = BTreeMap::from([(blob_uri, blob)]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let job = GraphJob::new(4, graph_id, input_roots, BTreeMap::new(), 30, 1, 2);

        let (receipt, outputs) = GraphReceipt::from_execution_with_const_blobs(
            &job,
            &graph,
            address(b"graph-blob-miner"),
            &inputs,
            &const_blobs,
            5,
            6,
        )
        .unwrap();

        assert_eq!(
            outputs["y"],
            Tensor::from_vec(vec![2], DType::FieldElement, vec![10, 12]).unwrap()
        );
        assert_eq!(
            job.exact_ir_execution_with_const_blobs(&graph, &inputs, &const_blobs)
                .unwrap()
                .trace_root,
            receipt.trace_root
        );
        assert_eq!(
            job.exact_ir_execution(&graph, &inputs),
            Err(TvmError::InvalidReceipt("missing tensor ir const_blob"))
        );
    }
}
