use crate::conformance::{ConformanceProfile, cpu_reference_conformance_profile};
use crate::error::{Result, TvmError};
#[cfg(feature = "cuda-kernels")]
use crate::field::Elem;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepOutput, MatmulJob};
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
}

#[cfg(feature = "cuda-kernels")]
impl GpuMinerBackend {
    fn cuda_device_index(&self) -> Result<u32> {
        let index = self.device.strip_prefix("cuda:").unwrap_or(&self.device);
        index
            .parse::<u32>()
            .map_err(|_| TvmError::InvalidReceipt("invalid cuda device"))
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
        cpu_reference_conformance_profile()
    }
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let _ = backend;
        Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
    }
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

    fn cuda_error(code: i32) -> TvmError {
        match code {
            -1 => TvmError::InvalidReceipt("cuda kernel received null pointer"),
            -2 => TvmError::InvalidReceipt("cuda device unavailable or invalid shape"),
            -3 => TvmError::InvalidReceipt("cuda allocation failed"),
            -4 => TvmError::InvalidReceipt("cuda host-device copy failed"),
            -5 => TvmError::InvalidReceipt("cuda kernel execution failed"),
            -6 => TvmError::InvalidReceipt("cuda device index out of range"),
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
        assert_eq!(
            backend_conformance_profile(&gpu).unwrap().suite_hash,
            crate::conformance::conformance_suite_hash()
        );
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
