#include <stdint.h>
#include <cuda_runtime.h>
#include <new>

namespace {

constexpr uint64_t kModulus = 2147483647ULL;

__device__ uint64_t field_mul(uint64_t lhs, uint64_t rhs) {
    return ((lhs % kModulus) * (rhs % kModulus)) % kModulus;
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

__global__ void field_relu_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        out[index] = field_relu(input[index]);
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

extern "C" int tensor_vm_cuda_field_relu(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len) {
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
        field_relu_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
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

    cudaFree(device_input);
    cudaFree(device_out);
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
