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

__global__ void field_slice_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t out_len,
    const uint64_t* input_shape,
    const uint64_t* output_shape,
    uint64_t rank,
    uint64_t dim,
    uint64_t start) {
    uint64_t output_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (output_index >= out_len) {
        return;
    }

    uint64_t remainder = output_index;
    uint64_t input_flat = 0;
    uint64_t input_stride = 1;
    for (uint64_t axis_from_end = 0; axis_from_end < rank; ++axis_from_end) {
        uint64_t axis = rank - 1 - axis_from_end;
        uint64_t output_dim = output_shape[axis];
        uint64_t coord = output_dim == 0 ? 0 : remainder % output_dim;
        remainder = output_dim == 0 ? 0 : remainder / output_dim;
        uint64_t input_coord = axis == dim ? coord + start : coord;
        input_flat += input_coord * input_stride;
        input_stride *= input_shape[axis];
    }
    out[output_index] = input[input_flat] % kModulus;
}

__global__ void field_concat_kernel(
    const uint64_t* inputs,
    uint64_t* out,
    uint64_t out_len,
    const uint64_t* input_offsets,
    const uint64_t* input_dim_sizes,
    const uint64_t* output_shape,
    uint64_t input_count,
    uint64_t rank,
    uint64_t dim) {
    uint64_t output_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (output_index >= out_len) {
        return;
    }

    uint64_t remainder = output_index;
    uint64_t source_flat = 0;
    uint64_t source_stride = 1;
    uint64_t dim_coord = 0;
    uint64_t source_index = 0;
    uint64_t source_dim_start = 0;
    for (uint64_t axis_from_end = 0; axis_from_end < rank; ++axis_from_end) {
        uint64_t axis = rank - 1 - axis_from_end;
        uint64_t output_dim = output_shape[axis];
        uint64_t coord = output_dim == 0 ? 0 : remainder % output_dim;
        remainder = output_dim == 0 ? 0 : remainder / output_dim;
        if (axis == dim) {
            dim_coord = coord;
            uint64_t next_start = 0;
            for (uint64_t input_index = 0; input_index < input_count; ++input_index) {
                uint64_t next_end = next_start + input_dim_sizes[input_index];
                if (dim_coord >= next_start && dim_coord < next_end) {
                    source_index = input_index;
                    source_dim_start = next_start;
                    break;
                }
                next_start = next_end;
            }
        }
    }

    remainder = output_index;
    for (uint64_t axis_from_end = 0; axis_from_end < rank; ++axis_from_end) {
        uint64_t axis = rank - 1 - axis_from_end;
        uint64_t output_dim = output_shape[axis];
        uint64_t coord = output_dim == 0 ? 0 : remainder % output_dim;
        remainder = output_dim == 0 ? 0 : remainder / output_dim;
        uint64_t source_coord = axis == dim ? coord - source_dim_start : coord;
        uint64_t source_dim = axis == dim ? input_dim_sizes[source_index] : output_dim;
        source_flat += source_coord * source_stride;
        source_stride *= source_dim;
    }
    out[output_index] = inputs[input_offsets[source_index] + source_flat] % kModulus;
}

__global__ void field_stack_kernel(
    const uint64_t* inputs,
    uint64_t* out,
    uint64_t out_len,
    const uint64_t* input_offsets,
    const uint64_t* output_shape,
    uint64_t rank,
    uint64_t dim) {
    uint64_t output_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (output_index >= out_len) {
        return;
    }

    uint64_t remainder = output_index;
    uint64_t source_index = 0;
    uint64_t source_flat = 0;
    uint64_t source_stride = 1;
    for (uint64_t axis_from_end = 0; axis_from_end < rank; ++axis_from_end) {
        uint64_t axis = rank - 1 - axis_from_end;
        uint64_t output_dim = output_shape[axis];
        uint64_t coord = output_dim == 0 ? 0 : remainder % output_dim;
        remainder = output_dim == 0 ? 0 : remainder / output_dim;
        if (axis == dim) {
            source_index = coord;
        } else {
            source_flat += coord * source_stride;
            source_stride *= output_dim;
        }
    }
    out[output_index] = inputs[input_offsets[source_index] + source_flat] % kModulus;
}

__global__ void field_triangular_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t rows,
    uint64_t cols,
    int64_t diagonal,
    uint32_t lower) {
    uint64_t cell = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t total = rows * cols;
    if (cell >= total) {
        return;
    }
    uint64_t row = cell / cols;
    uint64_t col = cell % cols;
    int64_t boundary = static_cast<int64_t>(row) + diagonal;
    bool keep = lower != 0
        ? static_cast<int64_t>(col) <= boundary
        : static_cast<int64_t>(col) >= boundary;
    out[cell] = keep ? input[cell] % kModulus : 0;
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

// ---- Canonical fixed-point exp-family (Q-format, bit-exact with the CPU
// reference in tensor.rs; see upow.md §4.8.1). All integer, round-half-to-even.
typedef __int128 i128;

constexpr uint32_t kQFrac = 32;
constexpr i128 kQOne = (i128)1 << kQFrac;
constexpr i128 kQExpUnderflow = (i128)64 << kQFrac;
constexpr uint32_t kQExpSquarings = 10;
constexpr i128 kGeluC = (i128)3426888095LL; // round(sqrt(2/pi) * 2^32)
constexpr i128 kGeluA = (i128)192049463LL;  // round(0.044715  * 2^32)

__device__ i128 q_signed_elem_to_i128(uint64_t v) {
    uint64_t n = v % kModulus;
    return n > kModulus / 2 ? (i128)n - (i128)kModulus : (i128)n;
}

__device__ uint64_t q_signed_i128_to_elem(i128 v) {
    i128 m = (i128)kModulus;
    i128 r = v % m;
    if (r < 0) {
        r += m;
    }
    return (uint64_t)r;
}

__device__ i128 q_round_div_pow2_half_even(i128 value, uint32_t shift) {
    if (shift == 0) {
        return value;
    }
    i128 divisor = (i128)1 << shift;
    i128 magnitude = value < 0 ? -value : value;
    i128 quotient = magnitude / divisor;
    i128 remainder = magnitude % divisor;
    i128 half = divisor / 2;
    bool round_up = remainder > half || (remainder == half && (quotient & 1) == 1);
    i128 rounded = round_up ? quotient + 1 : quotient;
    return value < 0 ? -rounded : rounded;
}

__device__ i128 q_round_div_half_even(i128 value, i128 divisor) {
    bool negative = (value < 0) ^ (divisor < 0);
    i128 num = value < 0 ? -value : value;
    i128 den = divisor < 0 ? -divisor : divisor;
    i128 quotient = num / den;
    i128 remainder = num % den;
    i128 twice = remainder * 2;
    bool round_up = twice > den || (twice == den && (quotient & 1) == 1);
    i128 rounded = round_up ? quotient + 1 : quotient;
    return negative ? -rounded : rounded;
}

__device__ i128 q_rescale_half_even(i128 value, int64_t from_scale, int64_t to_scale) {
    int64_t delta = to_scale - from_scale;
    if (delta >= 0) {
        return value * ((i128)1 << (uint32_t)delta);
    }
    return q_round_div_pow2_half_even(value, (uint32_t)(-delta));
}

__device__ i128 q_from_elem(uint64_t v, int64_t scale) {
    return q_rescale_half_even(q_signed_elem_to_i128(v), scale, (int64_t)kQFrac);
}

__device__ uint64_t q_to_elem(i128 v, int64_t scale) {
    return q_signed_i128_to_elem(q_rescale_half_even(v, (int64_t)kQFrac, scale));
}

__device__ i128 q_mul(i128 a, i128 b) {
    return q_round_div_pow2_half_even(a * b, kQFrac);
}

__device__ i128 q_div(i128 a, i128 b) {
    return q_round_div_half_even(a * kQOne, b);
}

__device__ i128 q_exp_nonpos(i128 dq) {
    if (dq > 0) {
        return 0;
    }
    if (dq <= -kQExpUnderflow) {
        return 0;
    }
    i128 r = q_round_div_pow2_half_even(dq, kQExpSquarings);
    i128 inv2 = kQOne / 2;
    i128 inv6 = q_round_div_half_even(kQOne, 6);
    i128 inv24 = q_round_div_half_even(kQOne, 24);
    i128 inv120 = q_round_div_half_even(kQOne, 120);
    i128 p = inv120;
    p = q_mul(p, r) + inv24;
    p = q_mul(p, r) + inv6;
    p = q_mul(p, r) + inv2;
    p = q_mul(p, r) + kQOne;
    p = q_mul(p, r) + kQOne;
    for (uint32_t i = 0; i < kQExpSquarings; ++i) {
        p = q_mul(p, p);
    }
    return p;
}

__device__ i128 q_exp(i128 xq) {
    if (xq <= 0) {
        return q_exp_nonpos(xq);
    }
    i128 e = q_exp_nonpos(-xq);
    if (e == 0) {
        return 0; // overflow sentinel; CPU reference errors, so unused in-domain
    }
    return q_div(kQOne, e);
}

__device__ i128 q_sigmoid(i128 xq) {
    if (xq >= 0) {
        i128 e = q_exp_nonpos(-xq);
        return q_div(kQOne, kQOne + e);
    }
    i128 e = q_exp_nonpos(xq);
    return q_div(e, kQOne + e);
}

__device__ i128 q_tanh(i128 zq) {
    i128 s = q_sigmoid(zq * 2);
    return s * 2 - kQOne;
}

__device__ i128 q_silu(i128 xq) {
    return q_mul(xq, q_sigmoid(xq));
}

__device__ i128 q_gelu(i128 xq) {
    i128 x2 = q_mul(xq, xq);
    i128 x3 = q_mul(x2, xq);
    i128 inner = q_mul(kGeluC, xq + q_mul(kGeluA, x3));
    i128 t = q_tanh(inner);
    i128 scaled = q_mul(xq, kQOne + t);
    return q_round_div_pow2_half_even(scaled, 1);
}

// 0=exp 1=sigmoid 2=tanh 3=silu 4=gelu
__device__ i128 q_fixed_unary(uint32_t op, i128 xq) {
    switch (op) {
        case 0: return q_exp(xq);
        case 1: return q_sigmoid(xq);
        case 2: return q_tanh(xq);
        case 3: return q_silu(xq);
        default: return q_gelu(xq);
    }
}

__global__ void fixed_unary_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    int64_t scale,
    uint32_t op) {
    uint64_t index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index < len) {
        i128 xq = q_from_elem(input[index], scale);
        out[index] = q_to_elem(q_fixed_unary(op, xq), scale);
    }
}

__global__ void fixed_softmax_kernel(
    const uint64_t* input,
    uint64_t* out,
    uint64_t outer,
    uint64_t axis,
    uint64_t inner,
    int64_t scale) {
    uint64_t gid = blockIdx.x * blockDim.x + threadIdx.x;
    uint64_t groups = outer * inner;
    if (gid >= groups || axis == 0) {
        return;
    }
    uint64_t o = gid / inner;
    uint64_t i = gid % inner;
    i128 max_q = q_from_elem(input[(o * axis) * inner + i], scale);
    for (uint64_t l = 1; l < axis; ++l) {
        i128 q = q_from_elem(input[((o * axis) + l) * inner + i], scale);
        if (q > max_q) {
            max_q = q;
        }
    }
    i128 sum = 0;
    for (uint64_t l = 0; l < axis; ++l) {
        i128 q = q_from_elem(input[((o * axis) + l) * inner + i], scale);
        sum += q_exp_nonpos(q - max_q);
    }
    for (uint64_t l = 0; l < axis; ++l) {
        uint64_t idx = ((o * axis) + l) * inner + i;
        i128 q = q_from_elem(input[idx], scale);
        i128 e = q_exp_nonpos(q - max_q);
        out[idx] = q_to_elem(q_div(e, sum), scale);
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

extern "C" int tensor_vm_cuda_field_slice(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t input_len,
    uint64_t out_len,
    const uint64_t* input_shape,
    const uint64_t* output_shape,
    uint64_t rank,
    uint64_t dim,
    uint64_t start) {
    if (input == nullptr || out == nullptr || input_shape == nullptr || output_shape == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (dim >= rank) {
        return -2;
    }
    if (out_len == 0) {
        return 0;
    }

    uint64_t* device_input = nullptr;
    uint64_t* device_out = nullptr;
    uint64_t* device_input_shape = nullptr;
    uint64_t* device_output_shape = nullptr;
    size_t input_bytes = static_cast<size_t>(input_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    size_t shape_bytes = static_cast<size_t>(rank * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_input, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        return -3;
    }
    status = cudaMalloc(&device_input_shape, shape_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        cudaFree(device_out);
        return -3;
    }
    status = cudaMalloc(&device_output_shape, shape_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_input);
        cudaFree(device_out);
        cudaFree(device_input_shape);
        return -3;
    }

    status = cudaMemcpy(device_input, input, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_input_shape, input_shape, shape_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_output_shape, output_shape, shape_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(out_len);
        field_slice_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            out_len,
            device_input_shape,
            device_output_shape,
            rank,
            dim,
            start);
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

extern "C" int tensor_vm_cuda_field_concat(
    uint32_t device_index,
    const uint64_t* inputs,
    uint64_t* out,
    uint64_t input_len,
    uint64_t out_len,
    const uint64_t* input_offsets,
    const uint64_t* input_dim_sizes,
    const uint64_t* output_shape,
    uint64_t input_count,
    uint64_t rank,
    uint64_t dim) {
    if (inputs == nullptr || out == nullptr || input_offsets == nullptr ||
        input_dim_sizes == nullptr || output_shape == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (input_count == 0 || dim >= rank) {
        return -2;
    }
    if (out_len == 0) {
        return 0;
    }

    uint64_t* device_inputs = nullptr;
    uint64_t* device_out = nullptr;
    uint64_t* device_input_offsets = nullptr;
    uint64_t* device_input_dim_sizes = nullptr;
    uint64_t* device_output_shape = nullptr;
    size_t input_bytes = static_cast<size_t>(input_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    size_t input_count_bytes = static_cast<size_t>(input_count * sizeof(uint64_t));
    size_t shape_bytes = static_cast<size_t>(rank * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_inputs, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        return -3;
    }
    status = cudaMalloc(&device_input_offsets, input_count_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        cudaFree(device_out);
        return -3;
    }
    status = cudaMalloc(&device_input_dim_sizes, input_count_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        cudaFree(device_out);
        cudaFree(device_input_offsets);
        return -3;
    }
    status = cudaMalloc(&device_output_shape, shape_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        cudaFree(device_out);
        cudaFree(device_input_offsets);
        cudaFree(device_input_dim_sizes);
        return -3;
    }

    status = cudaMemcpy(device_inputs, inputs, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_input_offsets, input_offsets, input_count_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_input_dim_sizes, input_dim_sizes, input_count_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_output_shape, output_shape, shape_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(out_len);
        field_concat_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_inputs,
            device_out,
            out_len,
            device_input_offsets,
            device_input_dim_sizes,
            device_output_shape,
            input_count,
            rank,
            dim);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_inputs);
    cudaFree(device_out);
    cudaFree(device_input_offsets);
    cudaFree(device_input_dim_sizes);
    cudaFree(device_output_shape);
    return fail(status, -5);
}

extern "C" int tensor_vm_cuda_field_stack(
    uint32_t device_index,
    const uint64_t* inputs,
    uint64_t* out,
    uint64_t input_len,
    uint64_t out_len,
    const uint64_t* input_offsets,
    const uint64_t* output_shape,
    uint64_t input_count,
    uint64_t rank,
    uint64_t dim) {
    if (inputs == nullptr || out == nullptr || input_offsets == nullptr || output_shape == nullptr) {
        return -1;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    if (input_count == 0 || dim >= rank) {
        return -2;
    }
    if (out_len == 0) {
        return 0;
    }

    uint64_t* device_inputs = nullptr;
    uint64_t* device_out = nullptr;
    uint64_t* device_input_offsets = nullptr;
    uint64_t* device_output_shape = nullptr;
    size_t input_bytes = static_cast<size_t>(input_len * sizeof(uint64_t));
    size_t out_bytes = static_cast<size_t>(out_len * sizeof(uint64_t));
    size_t input_count_bytes = static_cast<size_t>(input_count * sizeof(uint64_t));
    size_t shape_bytes = static_cast<size_t>(rank * sizeof(uint64_t));
    cudaError_t status = cudaMalloc(&device_inputs, input_bytes);
    if (status != cudaSuccess) {
        return -3;
    }
    status = cudaMalloc(&device_out, out_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        return -3;
    }
    status = cudaMalloc(&device_input_offsets, input_count_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        cudaFree(device_out);
        return -3;
    }
    status = cudaMalloc(&device_output_shape, shape_bytes);
    if (status != cudaSuccess) {
        cudaFree(device_inputs);
        cudaFree(device_out);
        cudaFree(device_input_offsets);
        return -3;
    }

    status = cudaMemcpy(device_inputs, inputs, input_bytes, cudaMemcpyHostToDevice);
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_input_offsets, input_offsets, input_count_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(device_output_shape, output_shape, shape_bytes, cudaMemcpyHostToDevice);
    }
    if (status == cudaSuccess) {
        constexpr uint64_t threads_per_block = 256;
        uint64_t blocks = block_count(out_len);
        field_stack_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_inputs,
            device_out,
            out_len,
            device_input_offsets,
            device_output_shape,
            rank,
            dim);
        status = cudaGetLastError();
    }
    if (status == cudaSuccess) {
        status = cudaDeviceSynchronize();
    }
    if (status == cudaSuccess) {
        status = cudaMemcpy(out, device_out, out_bytes, cudaMemcpyDeviceToHost);
    }

    cudaFree(device_inputs);
    cudaFree(device_out);
    cudaFree(device_input_offsets);
    cudaFree(device_output_shape);
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

extern "C" int tensor_vm_cuda_field_triangular(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t rows,
    uint64_t cols,
    int64_t diagonal,
    uint32_t lower) {
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
        field_triangular_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input,
            device_out,
            rows,
            cols,
            diagonal,
            lower);
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

extern "C" int tensor_vm_cuda_fixed_unary(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t len,
    int64_t scale,
    uint32_t op) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    if (op > 4) {
        return -2;
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
        fixed_unary_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input, device_out, len, scale, op);
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

extern "C" int tensor_vm_cuda_fixed_softmax(
    uint32_t device_index,
    const uint64_t* input,
    uint64_t* out,
    uint64_t outer,
    uint64_t axis,
    uint64_t inner,
    int64_t scale) {
    if (input == nullptr || out == nullptr) {
        return -1;
    }
    if (axis == 0) {
        return -2;
    }
    int device_status = select_device(device_index);
    if (device_status != 0) {
        return device_status;
    }
    uint64_t len = outer * axis * inner;
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
        uint64_t groups = outer * inner;
        uint64_t blocks = block_count(groups);
        fixed_softmax_kernel<<<static_cast<unsigned int>(blocks), threads_per_block>>>(
            device_input, device_out, outer, axis, inner, scale);
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
