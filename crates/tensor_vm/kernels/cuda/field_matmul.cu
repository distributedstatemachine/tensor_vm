#include <stdint.h>
#include <cuda_runtime.h>
#include <new>

namespace {

constexpr uint64_t kModulus = 2147483647ULL;

__device__ uint64_t field_mul(uint64_t lhs, uint64_t rhs) {
    return ((lhs % kModulus) * (rhs % kModulus)) % kModulus;
}

__device__ uint64_t field_pow(uint64_t base, uint64_t exponent) {
    uint64_t acc = 1;
    base %= kModulus;
    while (exponent > 0) {
        if ((exponent & 1ULL) == 1ULL) {
            acc = field_mul(acc, base);
        }
        base = field_mul(base, base);
        exponent >>= 1;
    }
    return acc;
}

__device__ uint64_t field_inverse(uint64_t value) {
    return field_pow(value, kModulus - 2);
}

__device__ uint64_t field_sub(uint64_t lhs, uint64_t rhs) {
    return ((lhs % kModulus) + kModulus - (rhs % kModulus)) % kModulus;
}

__device__ uint64_t field_add(uint64_t lhs, uint64_t rhs) {
    uint64_t sum = (lhs % kModulus) + (rhs % kModulus);
    return sum >= kModulus ? sum - kModulus : sum;
}

__device__ uint64_t field_relu(uint64_t value) {
    uint64_t normalized = value % kModulus;
    return normalized > kModulus / 2 ? 0 : normalized;
}

__device__ uint64_t field_neg(uint64_t value) {
    uint64_t normalized = value % kModulus;
    return normalized == 0 ? 0 : kModulus - normalized;
}

__device__ uint64_t field_abs(uint64_t value) {
    uint64_t normalized = value % kModulus;
    return normalized > kModulus / 2 ? field_neg(normalized) : normalized;
}

__device__ uint64_t field_sign(uint64_t value) {
    uint64_t normalized = value % kModulus;
    if (normalized == 0) {
        return 0;
    }
    return normalized > kModulus / 2 ? kModulus - 1 : 1;
}

__global__ void field_matmul_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t rows,
    uint64_t inner,
    uint64_t cols) {
    uint64_t cell = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t total = rows * cols;
    if (cell >= total) {
        return;
    }

    uint64_t row = cell / cols;
    uint64_t col = cell % cols;
    uint64_t acc = 0;
    for (uint64_t k = 0; k < inner; ++k) {
        acc += field_mul(lhs[row * inner + k], rhs[k * cols + col]);
        acc %= kModulus;
    }
    out[cell] = acc;
}

__global__ void field_sub_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_sub(lhs[index], rhs[index]);
    }
}

__global__ void field_add_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_add(lhs[index], rhs[index]);
    }
}

__global__ void field_mul_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_mul(lhs[index], rhs[index]);
    }
}

__global__ void field_div_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    unsigned int* zero_divisor,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        uint64_t divisor = rhs[index] % kModulus;
        if (divisor == 0) {
            atomicExch(zero_divisor, 1U);
            out[index] = 0;
        } else {
            out[index] = field_mul(lhs[index], field_inverse(divisor));
        }
    }
}

__global__ void field_eq_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = (lhs[index] % kModulus) == (rhs[index] % kModulus) ? 1 : 0;
    }
}

__global__ void field_gt_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = (lhs[index] % kModulus) > (rhs[index] % kModulus) ? 1 : 0;
    }
}

__global__ void field_lt_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = (lhs[index] % kModulus) < (rhs[index] % kModulus) ? 1 : 0;
    }
}

__global__ void field_ge_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = (lhs[index] % kModulus) >= (rhs[index] % kModulus) ? 1 : 0;
    }
}

__global__ void field_le_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = (lhs[index] % kModulus) <= (rhs[index] % kModulus) ? 1 : 0;
    }
}

__global__ void field_where_kernel(
    const uint64_t* cond,
    const uint64_t* when_true,
    const uint64_t* when_false,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        uint64_t selected = cond[index] == 0 ? when_false[index] : when_true[index];
        out[index] = selected % kModulus;
    }
}

__global__ void field_relu_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_relu(input[index]);
    }
}

__global__ void field_identity_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = input[index] % kModulus;
    }
}

__global__ void field_neg_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_neg(input[index]);
    }
}

__global__ void field_abs_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_abs(input[index]);
    }
}

__global__ void field_sign_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_sign(input[index]);
    }
}

__global__ void field_scalar_mul_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t scalar) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_mul(input[index], scalar);
    }
}

__global__ void field_clamp_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t min,
    uint64_t max) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        uint64_t value = input[index] % kModulus;
        if (value < min) {
            out[index] = min;
        } else if (value > max) {
            out[index] = max;
        } else {
            out[index] = value;
        }
    }
}

__global__ void field_sum_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t rows,
    uint64_t cols,
    uint32_t mode) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (mode == 0) {
        if (index >= cols) {
            return;
        }
        uint64_t acc = 0;
        for (uint64_t row = 0; row < rows; ++row) {
            acc = field_add(acc, input[row * cols + index]);
        }
        out[index] = acc;
        return;
    }
    if (mode == 1) {
        if (index >= rows) {
            return;
        }
        uint64_t acc = 0;
        for (uint64_t col = 0; col < cols; ++col) {
            acc = field_add(acc, input[index * cols + col]);
        }
        out[index] = acc;
        return;
    }
    if (index == 0) {
        uint64_t acc = 0;
        for (uint64_t offset = 0; offset < len; ++offset) {
            acc = field_add(acc, input[offset]);
        }
        out[0] = acc;
    }
}

__global__ void field_mean_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t rows,
    uint64_t cols,
    uint64_t reduce_count,
    uint32_t mode) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t inverse = field_inverse(reduce_count);
    if (mode == 0) {
        if (index >= cols) {
            return;
        }
        uint64_t acc = 0;
        for (uint64_t row = 0; row < rows; ++row) {
            acc = field_add(acc, input[row * cols + index]);
        }
        out[index] = field_mul(acc, inverse);
        return;
    }
    if (mode == 1) {
        if (index >= rows) {
            return;
        }
        uint64_t acc = 0;
        for (uint64_t col = 0; col < cols; ++col) {
            acc = field_add(acc, input[index * cols + col]);
        }
        out[index] = field_mul(acc, inverse);
        return;
    }
    if (index == 0) {
        uint64_t acc = 0;
        for (uint64_t offset = 0; offset < len; ++offset) {
            acc = field_add(acc, input[offset]);
        }
        out[0] = field_mul(acc, inverse);
    }
}

__global__ void field_broadcast_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t out_len,
    const uint64_t* input_shape,
    const uint64_t* output_shape,
    uint64_t input_rank,
    uint64_t output_rank) {
    uint64_t output_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (output_index >= out_len) {
        return;
    }

    uint64_t rank_offset = output_rank - input_rank;
    uint64_t remainder = output_index;
    uint64_t input_flat = 0;
    uint64_t input_stride = 1;
    for (uint64_t axis_from_end = 0; axis_from_end < output_rank; ++axis_from_end) {
        uint64_t output_axis = output_rank - 1 - axis_from_end;
        uint64_t output_dim = output_shape[output_axis];
        uint64_t coord = output_dim == 0 ? 0 : remainder % output_dim;
        remainder = output_dim == 0 ? 0 : remainder / output_dim;
        if (output_axis >= rank_offset) {
            uint64_t input_axis = output_axis - rank_offset;
            uint64_t input_dim = input_shape[input_axis];
            uint64_t input_coord = input_dim == 1 ? 0 : coord;
            input_flat += input_coord * input_stride;
            input_stride *= input_dim;
        }
    }
    out[output_index] = input[input_flat] % kModulus;
}

__global__ void field_transpose_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t rows,
    uint64_t cols) {
    uint64_t cell = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t total = rows * cols;
    if (cell >= total) {
        return;
    }
    uint64_t row = cell / cols;
    uint64_t col = cell % cols;
    out[col * rows + row] = input[cell] % kModulus;
}

__global__ void field_squared_error_sum_kernel(
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* partials,
    uint64_t len) {
    extern __shared__ uint64_t shared[];
    uint64_t local = 0;
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t stride = gridDim.x * blockDim.x;
    while (index < len) {
        uint64_t diff = field_sub(lhs[index], rhs[index]);
        local += field_mul(diff, diff);
        local %= kModulus;
        index += stride;
    }

    shared[threadIdx.x] = local;
    __syncthreads();

    for (uint32_t offset = blockDim.x / 2; offset > 0; offset >>= 1) {
        if (threadIdx.x < offset) {
            shared[threadIdx.x] += shared[threadIdx.x + offset];
            shared[threadIdx.x] %= kModulus;
        }
        __syncthreads();
    }

    if (threadIdx.x == 0) {
        partials[blockIdx.x] = shared[0];
    }
}

int fail(cudaError_t status, int code) {
    return status == cudaSuccess ? 0 : code;
}

int select_device(uint32_t device_index) {
    int device_count = 0;
    cudaError_t status = cudaGetDeviceCount(&device_count);
    if (status != cudaSuccess || device_index >= static_cast<uint32_t>(device_count)) {
        return -6;
    }
    status = cudaSetDevice(static_cast<int>(device_index));
    if (status != cudaSuccess) {
        return -6;
    }
    return 0;
}

uint64_t block_count(uint64_t elements) {
    constexpr uint64_t threads_per_block = 256;
    return (elements + threads_per_block - 1) / threads_per_block;
}

enum class UnaryOp {
    Identity,
    Neg,
    Abs,
    Sign,
    Relu,
};

enum class CompareOp {
    Eq,
    Gt,
    Lt,
    Ge,
    Le,
};

int launch_unary_kernel(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    UnaryOp op) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        switch (op) {
            case UnaryOp::Identity:
                field_identity_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_input, device_out, len);
                break;
            case UnaryOp::Neg:
                field_neg_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_input, device_out, len);
                break;
            case UnaryOp::Abs:
                field_abs_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_input, device_out, len);
                break;
            case UnaryOp::Sign:
                field_sign_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_input, device_out, len);
                break;
            case UnaryOp::Relu:
                field_relu_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_input, device_out, len);
                break;
        }
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

int launch_compare_kernel(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len,
    CompareOp op) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        switch (op) {
            case CompareOp::Eq:
                field_eq_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_lhs, device_rhs, device_out, len);
                break;
            case CompareOp::Gt:
                field_gt_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_lhs, device_rhs, device_out, len);
                break;
            case CompareOp::Lt:
                field_lt_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_lhs, device_rhs, device_out, len);
                break;
            case CompareOp::Ge:
                field_ge_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_lhs, device_rhs, device_out, len);
                break;
            case CompareOp::Le:
                field_le_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
                    device_lhs, device_rhs, device_out, len);
                break;
        }
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return fail(status, -5);
}

int launch_where_kernel(
    uint32_t device_index,
    const uint64_t* cond,
    const uint64_t* when_true,
    const uint64_t* when_false,
    uint64_t* out,
    uint64_t len) {
    if (cond == nullptr || when_true == nullptr || when_false == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_cond = nullptr;
    uint64_t* device_true = nullptr;
    uint64_t* device_false = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_cond, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_true, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_cond);
        return -3;
    }
    status = cudaMalloc(&device_false, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_cond);
        cudaFree(device_true);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_cond);
        cudaFree(device_true);
        cudaFree(device_false);
        return -3;
    }

    status = cudaMemcpy(device_cond, cond, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_true, when_true, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_false, when_false, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_where_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_cond,
            device_true,
            device_false,
            device_out,
            len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_cond);
    cudaFree(device_true);
    cudaFree(device_false);
    cudaFree(device_out);
    return fail(status, -5);
}

}  // namespace

extern "C" int tensor_vm_cuda_device_count(uint32_t* out) {
    if (out == nullptr) {
        return -1;
    }
    int count = 0;
    cudaError_t status = cudaGetDeviceCount(&count);
    if (status != cudaSuccess) {
        *out = 0;
        return -2;
    }
    *out = static_cast<uint32_t>(count);
    return 0;
}

extern "C" int tensor_vm_cuda_field_matmul(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t rows,
    uint64_t inner,
    uint64_t cols) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (rows != 0 && cols > UINT64_MAX / rows) {
        return -2;
    }
    if (inner != 0 && rows > UINT64_MAX / inner) {
        return -2;
    }
    if (cols != 0 && inner > UINT64_MAX / cols) {
        return -2;
    }

    uint64_t lhs_len = rows * inner;
    uint64_t rhs_len = inner * cols;
    uint64_t out_len = rows * cols;
    if (out_len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    cudaError_t status = cudaSuccess;
    size_t lhs_bytes = static_cast<size_t>(lhs_len * sizeof(uint64_t));
    size_t rhs_bytes = static_cast<size_t>(rhs_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));

    status = cudaMalloc(&device_lhs, lhs_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, rhs_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, lhs_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, rhs_bytes, cudaMemcpyHostToDevice);
    }
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        cudaFree(device_out);
        return -4;
    }

    constexpr uint64_t threads_per_block = 256;
    uint64_t blocks = block_count(out_len);
    field_matmul_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
        device_lhs,
        device_rhs,
        device_out,
        rows,
        inner,
        cols);
    status = cudaGetLastError();
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);

    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_sub(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_sub_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_lhs,
            device_rhs,
            device_out,
            len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_add(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_add_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_lhs,
            device_rhs,
            device_out,
            len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_mul(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_mul_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_lhs,
            device_rhs,
            device_out,
            len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_div(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_out = nullptr;
    unsigned int* device_zero_divisor = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }
    status = cudaMalloc(&device_zero_divisor, sizeof(unsigned int));
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        cudaFree(device_out);
        return -3;
    }

    unsigned int zero = 0;
    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(
            device_zero_divisor,
            &zero,
            sizeof(unsigned int),
            cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_div_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_lhs,
            device_rhs,
            device_out,
            device_zero_divisor,
            len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    unsigned int zero_divisor = 0;
    if (status == cudaSuccess) {
        status = cudaMemcpy(
            &zero_divisor,
            device_zero_divisor,
            sizeof(unsigned int),
            cudaMemcpyDeviceToHost);
    }
    if (status == cudaSuccess && zero_divisor != 0) {
        status = cudaErrorInvalidValue;
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_out);
    cudaFree(device_zero_divisor);
    if (zero_divisor != 0) {
        return -7;
    }
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_eq(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    return launch_compare_kernel(device_index, lhs, rhs, out, len, CompareOp::Eq);
}

extern "C" int tensor_vm_cuda_field_gt(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    return launch_compare_kernel(device_index, lhs, rhs, out, len, CompareOp::Gt);
}

extern "C" int tensor_vm_cuda_field_lt(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    return launch_compare_kernel(device_index, lhs, rhs, out, len, CompareOp::Lt);
}

extern "C" int tensor_vm_cuda_field_ge(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    return launch_compare_kernel(device_index, lhs, rhs, out, len, CompareOp::Ge);
}

extern "C" int tensor_vm_cuda_field_le(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    return launch_compare_kernel(device_index, lhs, rhs, out, len, CompareOp::Le);
}

extern "C" int tensor_vm_cuda_field_where(
    uint32_t device_index,
    const uint64_t* cond,
    const uint64_t* when_true,
    const uint64_t* when_false,
    uint64_t* out,
    uint64_t len) {
    return launch_where_kernel(device_index, cond, when_true, when_false, out, len);
}

extern "C" int tensor_vm_cuda_field_relu(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Relu);
}

extern "C" int tensor_vm_cuda_field_identity(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Identity);
}

extern "C" int tensor_vm_cuda_field_reshape(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Identity);
}

extern "C" int tensor_vm_cuda_field_neg(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Neg);
}

extern "C" int tensor_vm_cuda_field_abs(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Abs);
}

extern "C" int tensor_vm_cuda_field_sign(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    return launch_unary_kernel(device_index, input, out, len, UnaryOp::Sign);
}

extern "C" int tensor_vm_cuda_field_clamp(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t min,
    uint64_t max) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_clamp_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            len,
            min,
            max);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_sum(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t rows,
    uint64_t cols,
    uint32_t mode) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (mode > 2) {
        return -2;
    }
    uint64_t out_len = 1;
    if (mode == 0) {
        if (rows != 0 && cols > UINT64_MAX / rows) {
            return -2;
        }
        if (rows * cols != len) {
            return -2;
        }
        out_len = cols;
    } else if (mode == 1) {
        if (rows != 0 && cols > UINT64_MAX / rows) {
            return -2;
        }
        if (rows * cols != len) {
            return -2;
        }
        out_len = rows;
    }
    if (len == 0 || out_len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t input_bytes = static_cast<size_t>(len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = mode == 2 ? 1 : block_count(out_len);
        field_sum_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            len,
            rows,
            cols,
            mode);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_mean(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t rows,
    uint64_t cols,
    uint64_t reduce_count,
    uint32_t mode) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (mode > 2 || reduce_count == 0) {
        return -2;
    }
    uint64_t out_len = 1;
    if (mode == 0) {
        if (rows != 0 && cols > UINT64_MAX / rows) {
            return -2;
        }
        if (rows * cols != len || reduce_count != rows) {
            return -2;
        }
        out_len = cols;
    } else if (mode == 1) {
        if (rows != 0 && cols > UINT64_MAX / rows) {
            return -2;
        }
        if (rows * cols != len || reduce_count != cols) {
            return -2;
        }
        out_len = rows;
    } else if (reduce_count != len) {
        return -2;
    }
    if (len == 0 || out_len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t input_bytes = static_cast<size_t>(len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = mode == 2 ? 1 : block_count(out_len);
        field_mean_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            len,
            rows,
            cols,
            reduce_count,
            mode);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_broadcast(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t input_len,
    uint64_t out_len,
    const uint64_t* input_shape,
    const uint64_t* output_shape,
    uint64_t input_rank,
    uint64_t output_rank) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    if ((input_rank > 0 && input_shape == nullptr) ||
        (output_rank > 0 && output_shape == nullptr) ||
        input_rank > output_rank) {
        return -2;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (out_len == 0) {
        return 0;
    }
    if (input_len == 0) {
        return -2;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    uint64_t* device_input_shape = nullptr;
    uint64_t* device_output_shape = nullptr;
    size_t input_bytes = static_cast<size_t>(input_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    size_t input_shape_bytes = static_cast<size_t>(input_rank * sizeof(uint64_t));
    size_t output_shape_bytes = static_cast<size_t>(output_rank * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }
    if (input_rank > 0) {
        status = cudaMalloc(&device_input_shape, input_shape_bytes);
        if (status != cudaSuccess) {
            cudaFree(device_input);
            cudaFree(device_out);
            return -3;
        }
    }
    if (output_rank > 0) {
        status = cudaMalloc(&device_output_shape, output_shape_bytes);
        if (status != cudaSuccess) {
            cudaFree(device_input);
            cudaFree(device_out);
            cudaFree(device_input_shape);
            return -3;
        }
    }

    status = cudaMemcpy(device_input, input, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess && input_rank > 0) {
        status = cudaMemcpy(
            device_input_shape,
            input_shape,
            input_shape_bytes,
            cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess && output_rank > 0) {
        status = cudaMemcpy(
            device_output_shape,
            output_shape,
            output_shape_bytes,
            cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(out_len);
        field_broadcast_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            out_len,
            device_input_shape,
            device_output_shape,
            input_rank,
            output_rank);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    cudaFree(device_input_shape);
    cudaFree(device_output_shape);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_scalar_mul(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    uint64_t scalar) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_scalar_mul_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            len,
            scalar);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_transpose(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t rows,
    uint64_t cols) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (rows != 0 && cols > UINT64_MAX / rows) {
        return -2;
    }
    uint64_t len = rows * cols;
    if (len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }

    status = cudaMemcpy(device_input, input, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(len);
        field_transpose_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            rows,
            cols);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_input);
    cudaFree(device_out);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_squared_error_sum(
    uint32_t device_index,
    const uint64_t* lhs,
    const uint64_t* rhs,
    uint64_t* out,
    uint64_t len) {
    if (lhs == nullptr || rhs == nullptr || out == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (len == 0) {
        *out = 0;
        return 0;
    }

    uint64_t* device_lhs = nullptr;
    uint64_t* device_rhs = nullptr;
    uint64_t* device_partials = nullptr;
    size_t bytes = static_cast<size_t>(len * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_lhs, bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_rhs, bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        return -3;
    }

    constexpr uint64_t threads_per_block = 256;
    uint64_t blocks = block_count(len);
    if (blocks > 4096) {
        blocks = 4096;
    }
    size_t partial_bytes = static_cast<size_t>(blocks * sizeof(uint64_t));
    status = cudaMalloc(&device_partials, partial_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        return -3;
    }
    uint64_t* host_partials = new (std::nothrow) uint64_t[blocks];
    if (host_partials == nullptr) {
        cudaFree(device_lhs);
        cudaFree(device_rhs);
        cudaFree(device_partials);
        return -3;
    }

    status = cudaMemcpy(device_lhs, lhs, bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_rhs, rhs, bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        field_squared_error_sum_kernel<<<
            static_cast<unsigned int>(blocks),
            threads_per_block,
            threads_per_block * sizeof(uint64_t)>>>(
                device_lhs,
                device_rhs,
                device_partials,
                len);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(host_partials, device_partials, partial_bytes, cudaMemcpyDeviceToHost);
    }

    uint64_t acc = 0;
    if (status == cudaSuccess) {
        for (uint64_t index = 0; index < blocks; ++index) {
            acc += host_partials[index] % kModulus;
            acc %= kModulus;
        }
        *out = acc;
    }

    delete[] host_partials;
    cudaFree(device_lhs);
    cudaFree(device_rhs);
    cudaFree(device_partials);
    return fail(status, -5);
}
