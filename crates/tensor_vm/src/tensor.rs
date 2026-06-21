use crate::error::{Result, TvmError};
use crate::field::{self, Elem};
use crate::hash::Sha256;
use crate::merkle::{
    MerkleCommitment, MerkleProof, build_proof, leaf_hash, merkle_root, verify_proof,
};
use crate::oracle::OracleRng;
use crate::types::{Hash, hash_bytes};

pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DType {
    Int32,
    Int64,
    Fixed32,
    FieldElement,
    Int8,
    Uint8,
    Bool,
}

impl DType {
    pub fn tag(self) -> u8 {
        match self {
            Self::Int32 => 1,
            Self::Int64 => 2,
            Self::Fixed32 => 3,
            Self::FieldElement => 4,
            Self::Int8 => 5,
            Self::Uint8 => 6,
            Self::Bool => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    RowMajor,
    ChunkedRowMajor,
}

impl Layout {
    pub fn tag(self) -> u8 {
        match self {
            Self::RowMajor => 1,
            Self::ChunkedRowMajor => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorDescriptor {
    pub tensor_id: Hash,
    pub shape: Vec<usize>,
    pub dtype: DType,
    pub scale: i64,
    pub layout: Layout,
    pub chunk_shape: Vec<usize>,
    pub commitment: MerkleCommitment,
    pub byte_size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TensorOpening {
    pub tensor_id: Hash,
    pub chunk_index: u64,
    pub chunk_bytes: Vec<u8>,
    pub merkle_proof: MerkleProof,
}

impl TensorOpening {
    pub fn verify(&self, descriptor: &TensorDescriptor) -> bool {
        if self.tensor_id != descriptor.tensor_id {
            return false;
        }
        let leaf = leaf_hash(&self.tensor_id, self.chunk_index, &self.chunk_bytes);
        verify_proof(&descriptor.commitment.root, leaf, &self.merkle_proof)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    dtype: DType,
    scale: i64,
    layout: Layout,
    data: Vec<Elem>,
}

impl Tensor {
    pub fn zeros(shape: Vec<usize>, dtype: DType) -> Result<Self> {
        let len = checked_len(&shape)?;
        Ok(Self {
            shape,
            dtype,
            scale: 0,
            layout: Layout::RowMajor,
            data: vec![0; len],
        })
    }

    pub fn from_vec(shape: Vec<usize>, dtype: DType, data: Vec<Elem>) -> Result<Self> {
        Self::from_vec_with_scale(shape, dtype, 0, data)
    }

    pub fn from_vec_with_scale(
        shape: Vec<usize>,
        dtype: DType,
        scale: i64,
        data: Vec<Elem>,
    ) -> Result<Self> {
        let expected = checked_len(&shape)?;
        if expected != data.len() {
            return Err(TvmError::InvalidTensorData {
                expected,
                actual: data.len(),
            });
        }
        validate_dtype_values(dtype, &data)?;
        Ok(Self {
            shape,
            dtype,
            scale,
            layout: Layout::RowMajor,
            data: data.into_iter().map(field::normalize).collect(),
        })
    }

    pub fn from_packed_int8_payload(
        shape: Vec<usize>,
        axis: usize,
        output_scale: i64,
        scales: &[Elem],
        quantized: &[Elem],
    ) -> Result<Self> {
        let payload = encode_packed_int8_payload(&shape, axis, output_scale, scales, quantized)?;
        Self::from_vec(vec![payload.len()], DType::Uint8, payload)
    }

    pub fn packed_int8_payload(&self) -> Result<PackedInt8Payload> {
        if self.dtype != DType::Uint8 || self.scale != 0 || self.shape.len() != 1 {
            return Err(TvmError::InvalidReceipt(
                "packed int8 tensor artifact mismatch",
            ));
        }
        decode_packed_int8_payload(&self.data)
    }

    pub fn random(seed: &Hash, shape: Vec<usize>, dtype: DType) -> Result<Self> {
        let shape_bytes = encode_shape(&shape);
        let dtype_bytes = [dtype.tag()];
        let mut rng = OracleRng::new(
            b"tensor-vm-random-tensor-v1",
            &[seed, &shape_bytes, &dtype_bytes],
        );
        let len = checked_len(&shape)?;
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            data.push(random_elem_for_dtype(&mut rng, dtype));
        }
        Self::from_vec(shape, dtype, data)
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn dtype(&self) -> DType {
        self.dtype
    }

    pub fn scale(&self) -> i64 {
        self.scale
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[Elem] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [Elem] {
        &mut self.data
    }

    pub fn rows(&self) -> Result<usize> {
        self.require_rank(2)?;
        Ok(self.shape[0])
    }

    pub fn cols(&self) -> Result<usize> {
        self.require_rank(2)?;
        Ok(self.shape[1])
    }

    pub fn get2(&self, row: usize, col: usize) -> Result<Elem> {
        let rows = self.rows()?;
        let cols = self.cols()?;
        if row >= rows {
            return Err(TvmError::InvalidIndex {
                index: row,
                len: rows,
            });
        }
        if col >= cols {
            return Err(TvmError::InvalidIndex {
                index: col,
                len: cols,
            });
        }
        Ok(self.data[row * cols + col])
    }

    pub fn set2(&mut self, row: usize, col: usize, value: Elem) -> Result<()> {
        let rows = self.rows()?;
        let cols = self.cols()?;
        if row >= rows {
            return Err(TvmError::InvalidIndex {
                index: row,
                len: rows,
            });
        }
        if col >= cols {
            return Err(TvmError::InvalidIndex {
                index: col,
                len: cols,
            });
        }
        self.data[row * cols + col] = field::normalize(value);
        Ok(())
    }

    pub fn row(&self, row: usize) -> Result<&[Elem]> {
        let rows = self.rows()?;
        let cols = self.cols()?;
        if row >= rows {
            return Err(TvmError::InvalidIndex {
                index: row,
                len: rows,
            });
        }
        Ok(&self.data[row * cols..(row + 1) * cols])
    }

    pub fn add(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        self.check_add_sub_encoding(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs_elem)| {
                add_elem_for_dtype(self.dtype, self.scale, rhs.scale, *lhs, *rhs_elem)
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_vec_with_scale(self.shape.clone(), self.dtype, self.scale, data)
    }

    pub fn sub(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        self.check_add_sub_encoding(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs_elem)| {
                sub_elem_for_dtype(self.dtype, self.scale, rhs.scale, *lhs, *rhs_elem)
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_vec_with_scale(self.shape.clone(), self.dtype, self.scale, data)
    }

    pub fn mul(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        self.check_mul_encoding(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs_elem)| {
                multiply_elem_for_dtype(
                    self.dtype, self.scale, rhs.scale, self.scale, *lhs, *rhs_elem,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_vec_with_scale(self.shape.clone(), self.dtype, self.scale, data)
    }

    pub fn div(&self, rhs: &Self) -> Result<Self> {
        self.check_same_shape(rhs)?;
        self.check_div_encoding(rhs)?;
        let data = self
            .data
            .iter()
            .zip(&rhs.data)
            .map(|(lhs, rhs_elem)| {
                divide_elem_for_dtype(
                    self.dtype, self.scale, rhs.scale, self.scale, *lhs, *rhs_elem,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_vec_with_scale(self.shape.clone(), self.dtype, self.scale, data)
    }

    pub fn scalar_mul(&self, scalar: Elem) -> Result<Self> {
        let scalar = field::normalize(scalar);
        let data = self
            .data
            .iter()
            .map(|value| field::mul(*value, scalar))
            .collect();
        Self::from_vec_with_scale(self.shape.clone(), self.dtype, self.scale, data)
    }

    pub fn transpose(&self) -> Result<Self> {
        self.require_rank(2)?;
        let rows = self.shape[0];
        let cols = self.shape[1];
        let mut out = vec![0; self.data.len()];
        for row in 0..rows {
            for col in 0..cols {
                out[col * rows + row] = self.data[row * cols + col];
            }
        }
        Self::from_vec_with_scale(vec![cols, rows], self.dtype, self.scale, out)
    }

    pub fn matmul(&self, rhs: &Self) -> Result<Self> {
        self.require_rank(2)?;
        rhs.require_rank(2)?;
        let rows = self.shape[0];
        let inner = self.shape[1];
        if inner != rhs.shape[0] {
            return Err(TvmError::DimensionMismatch {
                left: self.shape.clone(),
                right: rhs.shape.clone(),
            });
        }
        let cols = rhs.shape[1];
        self.check_matmul_encoding(rhs)?;
        let rhs_t = rhs.transpose()?;
        let mut data = vec![0; rows * cols];
        for row in 0..rows {
            let lhs_row = &self.data[row * inner..(row + 1) * inner];
            for col in 0..cols {
                let rhs_row = &rhs_t.data[col * inner..(col + 1) * inner];
                data[row * cols + col] = matmul_dot_for_dtype(
                    self.dtype, self.scale, rhs.scale, self.scale, lhs_row, rhs_row,
                )?;
            }
        }
        Self::from_vec_with_scale(vec![rows, cols], self.dtype, self.scale, data)
    }

    pub fn reduce_sum(&self, axis: usize) -> Result<Self> {
        self.require_rank(2)?;
        let rows = self.shape[0];
        let cols = self.shape[1];
        match axis {
            0 => {
                let mut out = vec![0; cols];
                for row in 0..rows {
                    for (col, out_cell) in out.iter_mut().enumerate().take(cols) {
                        *out_cell = field::add(*out_cell, self.data[row * cols + col]);
                    }
                }
                Self::from_vec_with_scale(vec![cols], self.dtype, self.scale, out)
            }
            1 => {
                let mut out = vec![0; rows];
                for (row, out_cell) in out.iter_mut().enumerate() {
                    let mut acc = 0;
                    for value in &self.data[row * cols..(row + 1) * cols] {
                        acc = field::add(acc, *value);
                    }
                    *out_cell = acc;
                }
                Self::from_vec_with_scale(vec![rows], self.dtype, self.scale, out)
            }
            _ => Err(TvmError::InvalidAxis { axis, rank: 2 }),
        }
    }

    pub fn dot_vector(&self, vector: &[Elem]) -> Result<Vec<Elem>> {
        self.require_rank(2)?;
        let rows = self.shape[0];
        let cols = self.shape[1];
        if cols != vector.len() {
            return Err(TvmError::InvalidTensorData {
                expected: cols,
                actual: vector.len(),
            });
        }
        let mut out = vec![0; rows];
        for (row, out_cell) in out.iter_mut().enumerate() {
            let mut acc = 0_u128;
            let row_data = &self.data[row * cols..(row + 1) * cols];
            for col in 0..cols {
                acc += row_data[col] as u128 * vector[col] as u128;
            }
            *out_cell = field::reduce_u128(acc);
        }
        Ok(out)
    }

    pub fn row_dot(&self, row: usize, vector: &[Elem]) -> Result<Elem> {
        let row_data = self.row(row)?;
        if row_data.len() != vector.len() {
            return Err(TvmError::InvalidTensorData {
                expected: row_data.len(),
                actual: vector.len(),
            });
        }
        let mut acc = 0_u128;
        for i in 0..row_data.len() {
            acc += row_data[i] as u128 * vector[i] as u128;
        }
        Ok(field::reduce_u128(acc))
    }

    pub fn linear_combination(&self, weights: &[Elem]) -> Result<Elem> {
        if self.data.len() != weights.len() {
            return Err(TvmError::InvalidTensorData {
                expected: self.data.len(),
                actual: weights.len(),
            });
        }
        let mut acc = 0_u128;
        for (value, weight) in self.data.iter().zip(weights) {
            acc += *value as u128 * *weight as u128;
        }
        Ok(field::reduce_u128(acc))
    }

    pub fn squared_error_sum(&self, rhs: &Self) -> Result<Elem> {
        self.check_same_shape(rhs)?;
        self.check_same_encoding(rhs)?;
        let mut acc = 0_u128;
        for (lhs, rhs) in self.data.iter().zip(&rhs.data) {
            let diff = field::sub(*lhs, *rhs);
            acc += diff as u128 * diff as u128;
        }
        Ok(field::reduce_u128(acc))
    }

    pub fn tensor_id(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(b"tensor-vm-tensor-id-v1");
        self.hash_header_into(&mut hasher);
        for value in &self.data {
            hasher.update_u64(*value);
        }
        hasher.finalize()
    }

    pub fn descriptor(&self) -> TensorDescriptor {
        self.descriptor_with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    pub fn descriptor_with_chunk_size(&self, chunk_size: usize) -> TensorDescriptor {
        let tensor_id = self.tensor_id();
        let chunks = self.byte_chunks(chunk_size);
        let leaves: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| leaf_hash(&tensor_id, index as u64, chunk))
            .collect();
        TensorDescriptor {
            tensor_id,
            shape: self.shape.clone(),
            dtype: self.dtype,
            scale: self.scale,
            layout: self.layout,
            chunk_shape: vec![chunk_size],
            commitment: MerkleCommitment {
                root: merkle_root(&leaves),
                leaf_count: leaves.len() as u64,
                chunk_size,
            },
            byte_size: (self.data.len() * std::mem::size_of::<Elem>()) as u64,
        }
    }

    pub fn commitment_root(&self) -> Hash {
        self.descriptor().commitment.root
    }

    pub fn hash_tensor(&self) -> Hash {
        hash_bytes(
            b"tensor-vm-hash-tensor-v1",
            &[&self.tensor_id(), &self.commitment_root()],
        )
    }

    pub fn opening(&self, chunk_index: u64, chunk_size: usize) -> Result<TensorOpening> {
        let descriptor = self.descriptor_with_chunk_size(chunk_size);
        let chunks = self.byte_chunks(chunk_size);
        let chunk = chunks
            .get(chunk_index as usize)
            .ok_or(TvmError::InvalidChunk { chunk_index })?;
        let leaves: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| leaf_hash(&descriptor.tensor_id, index as u64, chunk))
            .collect();
        Ok(TensorOpening {
            tensor_id: descriptor.tensor_id,
            chunk_index,
            chunk_bytes: chunk.clone(),
            merkle_proof: build_proof(&leaves, chunk_index)?,
        })
    }

    fn byte_chunks(&self, chunk_size: usize) -> Vec<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.data.len() * std::mem::size_of::<Elem>());
        for value in &self.data {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        if bytes.is_empty() {
            return vec![Vec::new()];
        }
        bytes
            .chunks(chunk_size.max(1))
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    fn hash_header_into(&self, hasher: &mut Sha256) {
        hasher.update_u64(self.shape.len() as u64);
        for dim in &self.shape {
            hasher.update_u64(*dim as u64);
        }
        hasher.update(&[self.dtype.tag(), self.layout.tag()]);
        hasher.update(&self.scale.to_le_bytes());
    }

    fn require_rank(&self, rank: usize) -> Result<()> {
        if self.shape.len() != rank {
            return Err(TvmError::UnsupportedRank {
                rank: self.shape.len(),
            });
        }
        Ok(())
    }

    fn check_same_shape(&self, rhs: &Self) -> Result<()> {
        if self.shape != rhs.shape {
            return Err(TvmError::ShapeMismatch {
                left: self.shape.clone(),
                right: rhs.shape.clone(),
            });
        }
        Ok(())
    }

    fn check_same_encoding(&self, rhs: &Self) -> Result<()> {
        if self.dtype != rhs.dtype || self.scale != rhs.scale {
            return Err(TvmError::InvalidTensorData {
                expected: self.dtype.tag() as usize,
                actual: rhs.dtype.tag() as usize,
            });
        }
        Ok(())
    }

    fn check_add_sub_encoding(&self, rhs: &Self) -> Result<()> {
        if self.dtype != rhs.dtype || (self.dtype != DType::Fixed32 && self.scale != rhs.scale) {
            return Err(TvmError::InvalidTensorData {
                expected: self.dtype.tag() as usize,
                actual: rhs.dtype.tag() as usize,
            });
        }
        Ok(())
    }

    fn check_mul_encoding(&self, rhs: &Self) -> Result<()> {
        if self.dtype != rhs.dtype || (self.dtype != DType::Fixed32 && self.scale != rhs.scale) {
            return Err(TvmError::InvalidTensorData {
                expected: self.dtype.tag() as usize,
                actual: rhs.dtype.tag() as usize,
            });
        }
        Ok(())
    }

    fn check_matmul_encoding(&self, rhs: &Self) -> Result<()> {
        let valid = match self.dtype {
            DType::FieldElement => {
                rhs.dtype == DType::FieldElement && self.scale == 0 && rhs.scale == 0
            }
            DType::Fixed32 => rhs.dtype == DType::Fixed32,
            _ => false,
        };
        if !valid {
            return Err(TvmError::InvalidTensorData {
                expected: self.dtype.tag() as usize,
                actual: rhs.dtype.tag() as usize,
            });
        }
        Ok(())
    }

    fn check_div_encoding(&self, rhs: &Self) -> Result<()> {
        let valid = match self.dtype {
            DType::FieldElement => {
                rhs.dtype == DType::FieldElement && self.scale == 0 && rhs.scale == 0
            }
            DType::Fixed32 => rhs.dtype == DType::Fixed32,
            _ => false,
        };
        if !valid {
            return Err(TvmError::InvalidTensorData {
                expected: self.dtype.tag() as usize,
                actual: rhs.dtype.tag() as usize,
            });
        }
        Ok(())
    }
}

pub fn random_field_vector(seed: &Hash, label: &[u8], len: usize) -> Vec<Elem> {
    let len_bytes = (len as u64).to_le_bytes();
    let mut rng = OracleRng::new(label, &[seed, &len_bytes]);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(rng.next_field());
    }
    out
}

pub fn encode_shape(shape: &[usize]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + shape.len() * 8);
    out.extend_from_slice(&(shape.len() as u64).to_le_bytes());
    for dim in shape {
        out.extend_from_slice(&(*dim as u64).to_le_bytes());
    }
    out
}

pub fn signed_elem_to_i128(value: Elem) -> i128 {
    let value = field::normalize(value);
    if value > field::MODULUS / 2 {
        value as i128 - field::MODULUS as i128
    } else {
        value as i128
    }
}

pub fn signed_i128_to_elem(value: i128) -> Elem {
    let modulus = field::MODULUS as i128;
    value.rem_euclid(modulus) as Elem
}

pub const PACKED_INT8_MAGIC: &[u8; 4] = b"TVQ8";
pub const PACKED_INT8_VERSION: u8 = 1;
pub const PACKED_INT8_HEADER_LEN: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackedInt8Payload {
    pub shape: Vec<usize>,
    pub axis: usize,
    pub output_scale: i64,
    pub scales: Vec<Elem>,
    pub quantized: Vec<Elem>,
}

pub fn packed_int8_payload_len(shape: &[usize], axis: usize) -> Result<usize> {
    if axis >= shape.len() {
        return Err(TvmError::InvalidReceipt("packed int8 axis mismatch"));
    }
    let elements = checked_len(shape)?;
    PACKED_INT8_HEADER_LEN
        .checked_add(
            shape
                .len()
                .checked_mul(8)
                .ok_or(TvmError::InvalidReceipt("packed int8 shape overflow"))?,
        )
        .and_then(|len| len.checked_add(shape[axis].checked_mul(8)?))
        .and_then(|len| len.checked_add(elements))
        .ok_or(TvmError::InvalidReceipt("packed int8 shape overflow"))
}

pub fn encode_packed_int8_payload(
    shape: &[usize],
    axis: usize,
    output_scale: i64,
    scales: &[Elem],
    quantized: &[Elem],
) -> Result<Vec<Elem>> {
    validate_packed_int8_parts(shape, axis, scales, quantized)?;
    let rank = u8::try_from(shape.len())
        .map_err(|_| TvmError::InvalidReceipt("packed int8 rank overflow"))?;
    let axis_byte =
        u8::try_from(axis).map_err(|_| TvmError::InvalidReceipt("packed int8 axis overflow"))?;
    let mut out = Vec::with_capacity(packed_int8_payload_len(shape, axis)?);
    out.extend_from_slice(PACKED_INT8_MAGIC);
    out.push(PACKED_INT8_VERSION);
    out.push(rank);
    out.push(axis_byte);
    out.push(0);
    out.extend_from_slice(&output_scale.to_le_bytes());
    for dim in shape {
        let dim = u64::try_from(*dim)
            .map_err(|_| TvmError::InvalidReceipt("packed int8 shape overflow"))?;
        out.extend_from_slice(&dim.to_le_bytes());
    }
    for scale in scales {
        let raw = i64::try_from(signed_elem_to_i128(*scale))
            .map_err(|_| TvmError::InvalidReceipt("packed int8 scale mismatch"))?;
        if raw <= 0 {
            return Err(TvmError::InvalidReceipt("packed int8 scale mismatch"));
        }
        out.extend_from_slice(&raw.to_le_bytes());
    }
    for value in quantized {
        let raw = i8::try_from(signed_elem_to_i128(*value))
            .map_err(|_| TvmError::InvalidReceipt("packed int8 value mismatch"))?;
        out.push(raw as u8);
    }
    Ok(out.into_iter().map(Elem::from).collect())
}

pub fn decode_packed_int8_payload(payload: &[Elem]) -> Result<PackedInt8Payload> {
    let bytes = payload
        .iter()
        .map(|value| {
            u8::try_from(field::normalize(*value))
                .map_err(|_| TvmError::InvalidReceipt("packed int8 byte out of range"))
        })
        .collect::<Result<Vec<_>>>()?;
    decode_packed_int8_bytes(&bytes)
}

fn validate_packed_int8_parts(
    shape: &[usize],
    axis: usize,
    scales: &[Elem],
    quantized: &[Elem],
) -> Result<()> {
    if axis >= shape.len() {
        return Err(TvmError::InvalidReceipt("packed int8 axis mismatch"));
    }
    if scales.len() != shape[axis] || quantized.len() != checked_len(shape)? {
        return Err(TvmError::InvalidReceipt("packed int8 metadata mismatch"));
    }
    for scale in scales {
        let raw = i64::try_from(signed_elem_to_i128(*scale))
            .map_err(|_| TvmError::InvalidReceipt("packed int8 scale mismatch"))?;
        if raw <= 0 {
            return Err(TvmError::InvalidReceipt("packed int8 scale mismatch"));
        }
    }
    for value in quantized {
        i8::try_from(signed_elem_to_i128(*value))
            .map_err(|_| TvmError::InvalidReceipt("packed int8 value mismatch"))?;
    }
    Ok(())
}

fn decode_packed_int8_bytes(bytes: &[u8]) -> Result<PackedInt8Payload> {
    if bytes.len() < PACKED_INT8_HEADER_LEN
        || &bytes[0..4] != PACKED_INT8_MAGIC
        || bytes[4] != PACKED_INT8_VERSION
        || bytes[7] != 0
    {
        return Err(TvmError::InvalidReceipt("packed int8 header mismatch"));
    }
    let rank = bytes[5] as usize;
    let axis = bytes[6] as usize;
    if rank == 0 || axis >= rank {
        return Err(TvmError::InvalidReceipt("packed int8 metadata mismatch"));
    }
    let output_scale = i64::from_le_bytes(read_packed_i64(bytes, 8)?);
    let mut offset = PACKED_INT8_HEADER_LEN;
    let mut shape = Vec::with_capacity(rank);
    for _ in 0..rank {
        let dim = u64::from_le_bytes(read_packed_i64(bytes, offset)?);
        offset += 8;
        shape.push(
            usize::try_from(dim)
                .map_err(|_| TvmError::InvalidReceipt("packed int8 shape overflow"))?,
        );
    }
    if bytes.len() != packed_int8_payload_len(&shape, axis)? {
        return Err(TvmError::InvalidReceipt("packed int8 length mismatch"));
    }
    let mut scales = Vec::with_capacity(shape[axis]);
    for _ in 0..shape[axis] {
        let raw = i64::from_le_bytes(read_packed_i64(bytes, offset)?);
        offset += 8;
        if raw <= 0 {
            return Err(TvmError::InvalidReceipt("packed int8 scale mismatch"));
        }
        scales.push(signed_i128_to_elem(raw as i128));
    }
    let quantized = bytes[offset..]
        .iter()
        .map(|byte| signed_i128_to_elem(i8::from_le_bytes([*byte]) as i128))
        .collect();
    Ok(PackedInt8Payload {
        shape,
        axis,
        output_scale,
        scales,
        quantized,
    })
}

fn read_packed_i64(bytes: &[u8], offset: usize) -> Result<[u8; 8]> {
    bytes
        .get(offset..offset + 8)
        .ok_or(TvmError::InvalidReceipt("packed int8 length mismatch"))?
        .try_into()
        .map_err(|_| TvmError::InvalidReceipt("packed int8 length mismatch"))
}

fn validate_dtype_values(dtype: DType, data: &[Elem]) -> Result<()> {
    match dtype {
        DType::Int8 => {
            for value in data {
                let signed = signed_elem_to_i128(*value);
                if !(-128..=127).contains(&signed) {
                    return Err(TvmError::InvalidReceipt("int8 tensor value out of range"));
                }
            }
        }
        DType::Uint8 => {
            for value in data {
                if field::normalize(*value) > 255 {
                    return Err(TvmError::InvalidReceipt("uint8 tensor value out of range"));
                }
            }
        }
        DType::Bool => {
            for value in data {
                if !matches!(field::normalize(*value), 0 | 1) {
                    return Err(TvmError::InvalidReceipt("bool tensor value out of range"));
                }
            }
        }
        DType::Int32 | DType::Int64 | DType::Fixed32 | DType::FieldElement => {}
    }
    Ok(())
}

fn random_elem_for_dtype(rng: &mut OracleRng, dtype: DType) -> Elem {
    match dtype {
        DType::Int8 => {
            let byte = (rng.next_field() % 256) as i128;
            if byte <= 127 {
                byte as Elem
            } else {
                signed_i128_to_elem(byte - 256)
            }
        }
        DType::Uint8 => rng.next_field() % 256,
        DType::Bool => rng.next_field() % 2,
        DType::Int32 | DType::Int64 | DType::Fixed32 | DType::FieldElement => rng.next_field(),
    }
}

pub fn rescale_signed_elem_half_even(value: Elem, from_scale: i64, to_scale: i64) -> Result<Elem> {
    Ok(signed_i128_to_elem(rescale_signed_i128_half_even(
        signed_elem_to_i128(value),
        from_scale,
        to_scale,
    )?))
}

fn rescale_signed_i128_half_even(signed: i128, from_scale: i64, to_scale: i64) -> Result<i128> {
    let delta = to_scale
        .checked_sub(from_scale)
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    if delta >= 0 {
        let shift = u32::try_from(delta)
            .map_err(|_| TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
        Ok(signed
            .checked_mul(
                1_i128
                    .checked_shl(shift)
                    .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?,
            )
            .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?)
    } else {
        let shift = u32::try_from(
            delta
                .checked_neg()
                .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?,
        )
        .map_err(|_| TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
        round_div_pow2_half_even(signed, shift)
    }
}

pub fn fixed32_mul_same_scale_half_even(lhs: Elem, rhs: Elem, scale: i64) -> Result<Elem> {
    fixed32_mul_rescale_half_even(lhs, rhs, scale, scale, scale)
}

pub fn fixed32_mul_rescale_half_even(
    lhs: Elem,
    rhs: Elem,
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
) -> Result<Elem> {
    let product = signed_elem_to_i128(lhs)
        .checked_mul(signed_elem_to_i128(rhs))
        .ok_or(TvmError::InvalidReceipt("tensor fixed multiply overflow"))?;
    let product_scale = lhs_scale
        .checked_add(rhs_scale)
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let rescaled = rescale_signed_i128_half_even(product, product_scale, output_scale)?;
    Ok(signed_i128_to_elem(rescaled))
}

pub fn add_elem_for_dtype(
    dtype: DType,
    lhs_scale: i64,
    rhs_scale: i64,
    lhs: Elem,
    rhs: Elem,
) -> Result<Elem> {
    let rhs = if dtype == DType::Fixed32 {
        rescale_signed_elem_half_even(rhs, rhs_scale, lhs_scale)?
    } else {
        rhs
    };
    Ok(field::add(lhs, rhs))
}

pub fn sub_elem_for_dtype(
    dtype: DType,
    lhs_scale: i64,
    rhs_scale: i64,
    lhs: Elem,
    rhs: Elem,
) -> Result<Elem> {
    let rhs = if dtype == DType::Fixed32 {
        rescale_signed_elem_half_even(rhs, rhs_scale, lhs_scale)?
    } else {
        rhs
    };
    Ok(field::sub(lhs, rhs))
}

pub fn multiply_elem_for_dtype(
    dtype: DType,
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
    lhs: Elem,
    rhs: Elem,
) -> Result<Elem> {
    if dtype == DType::Fixed32 {
        fixed32_mul_rescale_half_even(lhs, rhs, lhs_scale, rhs_scale, output_scale)
    } else {
        Ok(field::mul(lhs, rhs))
    }
}

pub fn fixed32_div_rescale_half_even(
    lhs: Elem,
    rhs: Elem,
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
) -> Result<Elem> {
    let divisor = signed_elem_to_i128(rhs);
    if divisor == 0 {
        return Err(TvmError::InvalidReceipt("tensor fixed division by zero"));
    }
    let scale_delta = rhs_scale
        .checked_add(output_scale)
        .and_then(|scale| scale.checked_sub(lhs_scale))
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let lhs = signed_elem_to_i128(lhs);
    let (numerator, denominator) = if scale_delta >= 0 {
        let shift = u32::try_from(scale_delta)
            .map_err(|_| TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
        (
            lhs.checked_mul(
                1_i128
                    .checked_shl(shift)
                    .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?,
            )
            .ok_or(TvmError::InvalidReceipt("tensor fixed divide overflow"))?,
            divisor,
        )
    } else {
        let shift = u32::try_from(
            scale_delta
                .checked_neg()
                .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?,
        )
        .map_err(|_| TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
        (
            lhs,
            divisor
                .checked_mul(
                    1_i128
                        .checked_shl(shift)
                        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?,
                )
                .ok_or(TvmError::InvalidReceipt("tensor fixed divide overflow"))?,
        )
    };
    Ok(signed_i128_to_elem(round_div_i128_half_even(
        numerator,
        denominator,
    )?))
}

pub fn divide_elem_for_dtype(
    dtype: DType,
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
    lhs: Elem,
    rhs: Elem,
) -> Result<Elem> {
    match dtype {
        DType::FieldElement => Ok(field::mul(lhs, field_inverse(rhs)?)),
        DType::Fixed32 => {
            fixed32_div_rescale_half_even(lhs, rhs, lhs_scale, rhs_scale, output_scale)
        }
        _ => Err(TvmError::InvalidReceipt("tensor div dtype mismatch")),
    }
}

fn matmul_dot_for_dtype(
    dtype: DType,
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
    lhs: &[Elem],
    rhs: &[Elem],
) -> Result<Elem> {
    match dtype {
        DType::FieldElement => {
            let mut acc = 0;
            for (lhs, rhs) in lhs.iter().zip(rhs) {
                acc = field::add(acc, field::mul(*lhs, *rhs));
            }
            Ok(acc)
        }
        DType::Fixed32 => {
            fixed32_matmul_dot_rescale_half_even(lhs, rhs, lhs_scale, rhs_scale, output_scale)
        }
        _ => Err(TvmError::InvalidReceipt("tensor matmul dtype mismatch")),
    }
}

fn fixed32_matmul_dot_rescale_half_even(
    lhs: &[Elem],
    rhs: &[Elem],
    lhs_scale: i64,
    rhs_scale: i64,
    output_scale: i64,
) -> Result<Elem> {
    let product_scale = lhs_scale
        .checked_add(rhs_scale)
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let mut acc = 0_i128;
    for (lhs, rhs) in lhs.iter().zip(rhs) {
        let product = signed_elem_to_i128(*lhs)
            .checked_mul(signed_elem_to_i128(*rhs))
            .ok_or(TvmError::InvalidReceipt("tensor fixed matmul overflow"))?;
        acc = acc
            .checked_add(product)
            .ok_or(TvmError::InvalidReceipt("tensor fixed matmul overflow"))?;
    }
    Ok(signed_i128_to_elem(rescale_signed_i128_half_even(
        acc,
        product_scale,
        output_scale,
    )?))
}

fn round_div_pow2_half_even(value: i128, shift: u32) -> Result<i128> {
    if shift == 0 {
        return Ok(value);
    }
    let divisor = 1_i128
        .checked_shl(shift)
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let magnitude = value
        .checked_abs()
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let quotient = magnitude / divisor;
    let remainder = magnitude % divisor;
    let half = divisor / 2;
    let round_up = remainder > half || (remainder == half && quotient % 2 == 1);
    let rounded = if round_up {
        quotient
            .checked_add(1)
            .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?
    } else {
        quotient
    };
    if value.is_negative() {
        rounded
            .checked_neg()
            .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))
    } else {
        Ok(rounded)
    }
}

fn round_div_i128_half_even(value: i128, divisor: i128) -> Result<i128> {
    if divisor == 0 {
        return Err(TvmError::InvalidReceipt("tensor fixed division by zero"));
    }
    let negative = value.is_negative() ^ divisor.is_negative();
    let numerator = value
        .checked_abs()
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let denominator = divisor
        .checked_abs()
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .checked_mul(2)
        .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?;
    let round_up = twice > denominator || (twice == denominator && quotient % 2 == 1);
    let rounded = if round_up {
        quotient
            .checked_add(1)
            .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))?
    } else {
        quotient
    };
    if negative {
        rounded
            .checked_neg()
            .ok_or(TvmError::InvalidReceipt("tensor fixed scale overflow"))
    } else {
        Ok(rounded)
    }
}

fn field_inverse(value: Elem) -> Result<Elem> {
    let value = field::normalize(value);
    if value == 0 {
        return Err(TvmError::InvalidReceipt("tensor ir division by zero"));
    }
    Ok(field_pow(value, field::MODULUS - 2))
}

fn field_pow(mut base: Elem, mut exponent: Elem) -> Elem {
    let mut acc = 1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            acc = field::mul(acc, base);
        }
        base = field::mul(base, base);
        exponent >>= 1;
    }
    acc
}

fn checked_len(shape: &[usize]) -> Result<usize> {
    if shape.is_empty() {
        return Err(TvmError::EmptyShape);
    }
    let mut len = 1_usize;
    for dim in shape {
        len = len.checked_mul(*dim).ok_or(TvmError::InvalidTensorData {
            expected: usize::MAX,
            actual: 0,
        })?;
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_tensors_are_deterministic() {
        let seed = hash_bytes(b"test", &[b"seed"]);
        let a = Tensor::random(&seed, vec![3, 4], DType::FieldElement).unwrap();
        let b = Tensor::random(&seed, vec![3, 4], DType::FieldElement).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn random_narrow_integer_tensors_are_canonical() {
        let seed = hash_bytes(b"test", &[b"narrow-random-seed"]);
        let int8 = Tensor::random(&seed, vec![64], DType::Int8).unwrap();
        let uint8 = Tensor::random(&seed, vec![64], DType::Uint8).unwrap();
        let bools = Tensor::random(&seed, vec![64], DType::Bool).unwrap();

        assert!(
            int8.as_slice()
                .iter()
                .all(|value| (-128..=127).contains(&signed_elem_to_i128(*value)))
        );
        assert!(
            uint8
                .as_slice()
                .iter()
                .all(|value| field::normalize(*value) <= 255)
        );
        assert!(
            bools
                .as_slice()
                .iter()
                .all(|value| matches!(field::normalize(*value), 0 | 1))
        );
    }

    #[test]
    fn packed_int8_payload_roundtrips_and_rejects_bad_layout() {
        let p = field::MODULUS;
        let shape = vec![2, 3];
        let scales = vec![1, 2, 2];
        let quantized = vec![0, 32, 64, p - 64, p - 64, 64];
        let encoded = encode_packed_int8_payload(&shape, 1, 0, &scales, &quantized).unwrap();
        let expected_bytes = vec![
            84, 86, 81, 56, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
            32, 64, 192, 192, 64,
        ];
        assert_eq!(
            encoded,
            expected_bytes
                .iter()
                .map(|value| *value as Elem)
                .collect::<Vec<_>>()
        );
        assert_eq!(packed_int8_payload_len(&shape, 1).unwrap(), encoded.len());

        let decoded = decode_packed_int8_payload(&encoded).unwrap();
        assert_eq!(decoded.shape, shape);
        assert_eq!(decoded.axis, 1);
        assert_eq!(decoded.output_scale, 0);
        assert_eq!(decoded.scales, scales);
        assert_eq!(decoded.quantized, quantized);

        let mut bad_magic = encoded.clone();
        bad_magic[0] = 0;
        assert_eq!(
            decode_packed_int8_payload(&bad_magic),
            Err(TvmError::InvalidReceipt("packed int8 header mismatch"))
        );
        let mut bad_len = encoded.clone();
        bad_len.pop();
        assert_eq!(
            decode_packed_int8_payload(&bad_len),
            Err(TvmError::InvalidReceipt("packed int8 length mismatch"))
        );
        let mut bad_byte = encoded.clone();
        bad_byte[0] = 256;
        assert_eq!(
            decode_packed_int8_payload(&bad_byte),
            Err(TvmError::InvalidReceipt("packed int8 byte out of range"))
        );
        assert_eq!(
            encode_packed_int8_payload(&shape, 2, 0, &scales, &quantized),
            Err(TvmError::InvalidReceipt("packed int8 axis mismatch"))
        );
    }

    #[test]
    fn packed_int8_tensor_artifact_exposes_descriptor_chunks_and_openings() {
        let p = field::MODULUS;
        let shape = vec![2, 3];
        let scales = vec![1, 2, 2];
        let quantized = vec![0, 32, 64, p - 64, p - 64, 64];
        let artifact =
            Tensor::from_packed_int8_payload(shape.clone(), 1, 0, &scales, &quantized).unwrap();

        assert_eq!(artifact.dtype(), DType::Uint8);
        assert_eq!(artifact.scale(), 0);
        assert_eq!(artifact.shape(), &[artifact.len()]);

        let decoded = artifact.packed_int8_payload().unwrap();
        assert_eq!(decoded.shape, shape);
        assert_eq!(decoded.axis, 1);
        assert_eq!(decoded.output_scale, 0);
        assert_eq!(decoded.scales, scales);
        assert_eq!(decoded.quantized, quantized);

        let descriptor = artifact.descriptor_with_chunk_size(16);
        assert_eq!(descriptor.dtype, DType::Uint8);
        assert_eq!(descriptor.scale, 0);
        assert_eq!(descriptor.shape, vec![artifact.len()]);
        assert!(descriptor.commitment.leaf_count > 1);
        for chunk_index in 0..descriptor.commitment.leaf_count {
            let opening = artifact.opening(chunk_index, 16).unwrap();
            assert!(opening.verify(&descriptor));
        }

        let wrong_dtype = Tensor::from_vec(
            vec![artifact.len()],
            DType::Int64,
            artifact.as_slice().to_vec(),
        )
        .unwrap();
        assert_eq!(
            wrong_dtype.packed_int8_payload(),
            Err(TvmError::InvalidReceipt(
                "packed int8 tensor artifact mismatch"
            ))
        );
    }

    #[test]
    fn fixed32_multiply_rescales_to_input_scale_half_even() {
        let p = field::MODULUS;
        let lhs =
            Tensor::from_vec_with_scale(vec![6], DType::Fixed32, 2, vec![6, 7, p - 6, p - 7, 3, 5])
                .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![6], DType::Fixed32, 2, vec![6, 6, 6, 6, 6, p - 6])
                .unwrap();

        assert_eq!(
            lhs.mul(&rhs).unwrap(),
            Tensor::from_vec_with_scale(
                vec![6],
                DType::Fixed32,
                2,
                vec![9, 10, p - 9, p - 10, 4, p - 8]
            )
            .unwrap()
        );
        assert_eq!(fixed32_mul_same_scale_half_even(3, 6, 2).unwrap(), 4);
        assert_eq!(
            fixed32_mul_same_scale_half_even(5, p - 6, 2).unwrap(),
            p - 8
        );
    }

    #[test]
    fn fixed32_multiply_rescales_mixed_scales_to_lhs_scale_half_even() {
        let p = field::MODULUS;
        let lhs =
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![6, p - 7, 3, p - 3, 5])
                .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 0, vec![2, p - 2, 1, p - 1, 0])
                .unwrap();

        assert_eq!(
            lhs.mul(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![12, 14, 3, 3, 0]).unwrap()
        );

        let lhs = Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 0, vec![2, 3, p - 3, p - 2])
            .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 1, vec![3, 3, 3, 3]).unwrap();
        assert_eq!(
            lhs.mul(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 0, vec![3, 4, p - 4, p - 3])
                .unwrap()
        );
        assert_eq!(fixed32_mul_rescale_half_even(3, 3, 0, 1, 0).unwrap(), 4);
        assert_eq!(
            fixed32_mul_rescale_half_even(p - 3, 3, 0, 1, 0).unwrap(),
            p - 4
        );
    }

    #[test]
    fn fixed32_division_rescales_to_lhs_scale_half_even() {
        let p = field::MODULUS;
        let lhs = Tensor::from_vec_with_scale(
            vec![6],
            DType::Fixed32,
            2,
            vec![12, p - 12, 7, p - 7, 10, p - 10],
        )
        .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![6], DType::Fixed32, 2, vec![4, 4, 2, 2, 4, p - 4])
                .unwrap();

        assert_eq!(
            lhs.div(&rhs).unwrap(),
            Tensor::from_vec_with_scale(
                vec![6],
                DType::Fixed32,
                2,
                vec![12, p - 12, 14, p - 14, 10, 10]
            )
            .unwrap()
        );

        let lhs = Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 0, vec![9, 7, p - 9, p - 7])
            .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 1, vec![4, 4, 4, 4]).unwrap();
        assert_eq!(
            lhs.div(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![4], DType::Fixed32, 0, vec![4, 4, p - 4, p - 4])
                .unwrap()
        );
        assert_eq!(fixed32_div_rescale_half_even(9, 4, 0, 1, 0).unwrap(), 4);
        assert_eq!(fixed32_div_rescale_half_even(7, 4, 0, 1, 0).unwrap(), 4);
        assert_eq!(
            fixed32_div_rescale_half_even(p - 7, 4, 0, 1, 0).unwrap(),
            p - 4
        );
        assert_eq!(
            fixed32_div_rescale_half_even(1, 0, 0, 0, 0),
            Err(TvmError::InvalidReceipt("tensor fixed division by zero"))
        );
    }

    #[test]
    fn fixed32_add_sub_rescale_rhs_to_lhs_scale_half_even() {
        let p = field::MODULUS;
        let lhs =
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![6, p - 7, 3, p - 3, 5])
                .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 0, vec![2, p - 2, 1, p - 1, 0])
                .unwrap();

        assert_eq!(
            lhs.add(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![14, p - 15, 7, p - 7, 5])
                .unwrap()
        );
        assert_eq!(
            lhs.sub(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![5], DType::Fixed32, 2, vec![p - 2, 1, p - 1, 1, 5])
                .unwrap()
        );
        assert_eq!(add_elem_for_dtype(DType::Fixed32, 0, 1, 2, 3).unwrap(), 4);
        assert_eq!(
            sub_elem_for_dtype(DType::Fixed32, 0, 1, p - 2, p - 3).unwrap(),
            0
        );
    }

    #[test]
    fn tensor_construction_accessors_and_empty_commitment_work() {
        assert_eq!(
            Tensor::zeros(Vec::new(), DType::FieldElement),
            Err(TvmError::EmptyShape)
        );
        assert_eq!(
            Tensor::from_vec(vec![2], DType::FieldElement, vec![1]),
            Err(TvmError::InvalidTensorData {
                expected: 2,
                actual: 1,
            })
        );
        assert!(Tensor::zeros(vec![usize::MAX, 2], DType::FieldElement).is_err());

        let mut tensor = Tensor::zeros(vec![2], DType::Int64).unwrap();
        assert_eq!(tensor.shape(), &[2]);
        assert_eq!(tensor.dtype(), DType::Int64);
        assert_eq!(tensor.layout(), Layout::RowMajor);
        assert_eq!(tensor.len(), 2);
        assert!(!tensor.is_empty());
        assert_eq!(tensor.as_slice(), &[0, 0]);
        tensor.as_mut_slice()[1] = 9;
        assert_eq!(tensor.as_slice(), &[0, 9]);

        let empty = Tensor::zeros(vec![0], DType::FieldElement).unwrap();
        assert!(empty.is_empty());
        let descriptor = empty.descriptor_with_chunk_size(4);
        assert_eq!(descriptor.byte_size, 0);
        assert_eq!(descriptor.commitment.leaf_count, 1);
        assert!(empty.opening(0, 4).unwrap().verify(&descriptor));
    }

    #[test]
    fn tensor_scale_is_metadata_and_commitment_bound() {
        let unscaled =
            Tensor::from_vec(vec![2], DType::Fixed32, vec![1, field::MODULUS - 1]).unwrap();
        let scaled =
            Tensor::from_vec_with_scale(vec![2], DType::Fixed32, 1, vec![1, field::MODULUS - 1])
                .unwrap();
        assert_eq!(unscaled.scale(), 0);
        assert_eq!(scaled.scale(), 1);
        assert_eq!(scaled.descriptor().scale, 1);
        assert_ne!(unscaled.tensor_id(), scaled.tensor_id());
        assert_ne!(unscaled.commitment_root(), scaled.commitment_root());
        assert_eq!(signed_elem_to_i128(field::MODULUS - 1), -1);
        assert_eq!(rescale_signed_elem_half_even(3, 1, 0).unwrap(), 2);
        assert_eq!(
            rescale_signed_elem_half_even(field::MODULUS - 3, 1, 0).unwrap(),
            field::MODULUS - 2
        );
    }

    #[test]
    fn tensor_ops_match_small_examples() {
        let a = Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let b =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![7, 8, 9, 10, 11, 12]).unwrap();
        let c = a.matmul(&b).unwrap();
        assert_eq!(
            c,
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![58, 64, 139, 154]).unwrap()
        );
        assert_eq!(a.transpose().unwrap().shape(), &[3, 2]);
        assert_eq!(
            a.reduce_sum(0).unwrap(),
            Tensor::from_vec(vec![3], DType::FieldElement, vec![5, 7, 9]).unwrap()
        );
        assert_eq!(
            a.reduce_sum(1).unwrap(),
            Tensor::from_vec(vec![2], DType::FieldElement, vec![6, 15]).unwrap()
        );
        assert_eq!(a.add(&a).unwrap(), a.scalar_mul(2).unwrap());
        assert_eq!(
            a.mul(&a).unwrap(),
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 4, 9, 16, 25, 36]).unwrap()
        );
    }

    #[test]
    fn fixed32_matmul_accumulates_then_rescales_to_lhs_scale_half_even() {
        let p = field::MODULUS;
        let lhs = Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 0, vec![1, 1, 3, p - 3])
            .unwrap();
        let rhs =
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 1, vec![1, 2, 0, 4]).unwrap();

        assert_eq!(
            lhs.matmul(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![2, 2], DType::Fixed32, 0, vec![0, 3, 2, p - 3])
                .unwrap()
        );

        let lhs = Tensor::from_vec_with_scale(vec![1, 2], DType::Fixed32, 2, vec![3, 3]).unwrap();
        let rhs = Tensor::from_vec_with_scale(vec![2, 1], DType::Fixed32, 2, vec![2, 4]).unwrap();
        assert_eq!(
            lhs.matmul(&rhs).unwrap(),
            Tensor::from_vec_with_scale(vec![1, 1], DType::Fixed32, 2, vec![4]).unwrap()
        );
    }

    #[test]
    fn tensor_vector_checks_and_rank_errors_are_reported() {
        let matrix =
            Tensor::from_vec(vec![2, 3], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(matrix.dot_vector(&[7, 8, 9]).unwrap(), vec![50, 122]);
        assert_eq!(
            matrix.dot_vector(&[1, 2]),
            Err(TvmError::InvalidTensorData {
                expected: 3,
                actual: 2,
            })
        );
        assert_eq!(matrix.row_dot(1, &[7, 8, 9]).unwrap(), 122);
        assert_eq!(
            matrix.row_dot(1, &[1, 2]),
            Err(TvmError::InvalidTensorData {
                expected: 3,
                actual: 2,
            })
        );
        assert_eq!(matrix.linear_combination(&[1, 1, 1, 1, 1, 1]).unwrap(), 21);
        assert_eq!(
            matrix.linear_combination(&[1]),
            Err(TvmError::InvalidTensorData {
                expected: 6,
                actual: 1,
            })
        );
        assert_eq!(
            matrix.reduce_sum(2),
            Err(TvmError::InvalidAxis { axis: 2, rank: 2 })
        );
        let vector = Tensor::from_vec(vec![3], DType::FieldElement, vec![1, 2, 3]).unwrap();
        assert_eq!(
            vector.transpose(),
            Err(TvmError::UnsupportedRank { rank: 1 })
        );
        assert_eq!(
            matrix.matmul(&matrix),
            Err(TvmError::DimensionMismatch {
                left: vec![2, 3],
                right: vec![2, 3],
            })
        );
    }

    #[test]
    fn tensor_openings_verify_and_reject_tampering() {
        let tensor = Tensor::from_vec(
            vec![2, 4],
            DType::FieldElement,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        )
        .unwrap();
        let descriptor = tensor.descriptor_with_chunk_size(16);
        let mut opening = tensor.opening(1, 16).unwrap();
        assert!(opening.verify(&descriptor));
        opening.chunk_bytes[0] ^= 1;
        assert!(!opening.verify(&descriptor));
    }

    #[test]
    fn tensor_tags_and_openings_reject_wrong_descriptor() {
        assert_eq!(DType::Int32.tag(), 1);
        assert_eq!(DType::Int64.tag(), 2);
        assert_eq!(DType::Fixed32.tag(), 3);
        assert_eq!(DType::FieldElement.tag(), 4);
        assert_eq!(DType::Int8.tag(), 5);
        assert_eq!(DType::Uint8.tag(), 6);
        assert_eq!(DType::Bool.tag(), 7);
        assert_eq!(Layout::RowMajor.tag(), 1);
        assert_eq!(Layout::ChunkedRowMajor.tag(), 2);
        assert_eq!(
            encode_shape(&[2, 3]),
            vec![
                2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
            ]
        );

        let tensor = Tensor::from_vec(vec![1], DType::FieldElement, vec![7]).unwrap();
        let other = Tensor::from_vec(vec![1], DType::FieldElement, vec![8]).unwrap();
        let opening = tensor.opening(0, 8).unwrap();
        assert!(!opening.verify(&other.descriptor_with_chunk_size(8)));

        let seed = hash_bytes(b"test", &[b"vector-seed"]);
        assert_eq!(
            random_field_vector(&seed, b"label", 3),
            random_field_vector(&seed, b"label", 3)
        );
    }

    #[test]
    fn narrow_integer_tensors_enforce_canonical_ranges_and_commit_dtype() {
        let int8 =
            Tensor::from_vec(vec![3], DType::Int8, vec![field::MODULUS - 128, 0, 127]).unwrap();
        let uint8 = Tensor::from_vec(vec![3], DType::Uint8, vec![0, 127, 255]).unwrap();
        let bools = Tensor::from_vec(vec![2], DType::Bool, vec![0, 1]).unwrap();
        assert_ne!(int8.tensor_id(), uint8.tensor_id());
        assert_ne!(uint8.tensor_id(), bools.tensor_id());
        assert_eq!(
            Tensor::from_vec(vec![1], DType::Int8, vec![128]),
            Err(TvmError::InvalidReceipt("int8 tensor value out of range"))
        );
        assert_eq!(
            Tensor::from_vec(vec![1], DType::Uint8, vec![256]),
            Err(TvmError::InvalidReceipt("uint8 tensor value out of range"))
        );
        assert_eq!(
            Tensor::from_vec(vec![1], DType::Bool, vec![2]),
            Err(TvmError::InvalidReceipt("bool tensor value out of range"))
        );
    }

    #[test]
    fn tensor_row_and_cell_access_reject_out_of_bounds() {
        let mut tensor =
            Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(
            tensor.row(2),
            Err(TvmError::InvalidIndex { index: 2, len: 2 })
        );
        assert_eq!(
            tensor.get2(2, 0),
            Err(TvmError::InvalidIndex { index: 2, len: 2 })
        );
        assert_eq!(
            tensor.get2(0, 2),
            Err(TvmError::InvalidIndex { index: 2, len: 2 })
        );
        assert_eq!(
            tensor.set2(2, 0, 9),
            Err(TvmError::InvalidIndex { index: 2, len: 2 })
        );
        assert_eq!(
            tensor.set2(0, 2, 9),
            Err(TvmError::InvalidIndex { index: 2, len: 2 })
        );
    }
}
