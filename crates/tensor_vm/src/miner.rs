use crate::error::Result;
use crate::jobs::{
    LinearTrainingStepJob, LinearTrainingStepOutput, LinearTrainingStepReceipt, MatmulJob,
    TensorOpReceipt,
};
use crate::runtime::ExecutionBackend;
use crate::tensor::Tensor;
use crate::tensor_server::TensorServer;
use crate::types::Address;

#[derive(Clone, Debug)]
pub struct MinerNode<B> {
    pub address: Address,
    pub backend: B,
    pub tensor_server: TensorServer,
}

impl<B: ExecutionBackend> MinerNode<B> {
    pub fn new(address: Address, backend: B) -> Self {
        Self {
            address,
            backend,
            tensor_server: TensorServer::default(),
        }
    }

    pub fn solve_matmul_job(
        &mut self,
        job: &MatmulJob,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(TensorOpReceipt, Tensor, Tensor, Tensor)> {
        let (a, b, c) = self.backend.execute_matmul(job)?;
        self.tensor_server.insert(a.clone());
        self.tensor_server.insert(b.clone());
        self.tensor_server.insert(c.clone());
        let receipt = TensorOpReceipt::from_output(
            job,
            self.address,
            submitted_at_block,
            execution_time_ms,
            &a,
            &b,
            &c,
        )?;
        Ok((receipt, a, b, c))
    }

    pub fn solve_linear_training_step(
        &mut self,
        job: &LinearTrainingStepJob,
        weights: &Tensor,
        submitted_at_block: u64,
        execution_time_ms: u64,
    ) -> Result<(LinearTrainingStepReceipt, LinearTrainingStepOutput)> {
        let output = self.backend.execute_linear_training_step(job, weights)?;
        self.tensor_server.insert(output.x.clone());
        self.tensor_server.insert(output.target.clone());
        self.tensor_server.insert(output.y.clone());
        self.tensor_server.insert(output.dy.clone());
        self.tensor_server.insert(output.grad_w.clone());
        self.tensor_server.insert(output.weight_after.clone());
        let receipt = LinearTrainingStepReceipt::from_output(
            job,
            self.address,
            weights,
            &output,
            submitted_at_block,
            execution_time_ms,
        )?;
        Ok((receipt, output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
    use crate::runtime::CpuReferenceBackend;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};

    #[test]
    fn miner_solves_matmul_and_serves_tensors() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, a, b, c) = miner.solve_matmul_job(&job, 1, 5).unwrap();
        assert_eq!(receipt.output_roots, vec![c.commitment_root()]);
        assert!(miner.tensor_server.get(&a.tensor_id()).is_some());
        assert!(miner.tensor_server.get(&b.tensor_id()).is_some());
        assert!(miner.tensor_server.get(&c.tensor_id()).is_some());
    }

    #[test]
    fn miner_solves_linear_step_and_serves_intermediates() {
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
        let mut miner = MinerNode::new(address(b"miner"), CpuReferenceBackend);
        let (receipt, output) = miner
            .solve_linear_training_step(&job, &weights, 1, 5)
            .unwrap();
        assert_eq!(
            receipt.weight_root_after,
            output.weight_after.commitment_root()
        );
        assert!(miner.tensor_server.len() >= 6);
    }
}
