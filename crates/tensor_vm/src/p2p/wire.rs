use crate::api::P2pMessage;
use crate::chain::{BlockVote, JobState, ReceiptState, TensorBlock, ValidatorAuditReport};
use crate::codec::{self, CodecError};
use crate::error::{Result as TvmResult, TvmError};
use crate::tensor::{DType, Tensor};
use crate::types::Hash;
use crate::verify::ValidatorAttestation;
use libp2p::StreamProtocol;

use super::{GossipTopic, RequestResponseProtocol};

pub(super) const MAX_JOB_SHAPE_DIMS: usize = 16;
pub(super) const MAX_RECEIPT_HASHES: usize = 16;
pub(super) const MAX_TENSOR_SHAPE_DIMS: usize = 16;
pub(super) const MAX_TENSOR_VALUES: usize = 1_000_000;
const MAX_WIRE_BYTES: usize = 16 * 1024 * 1024;
const BLOCK_PAYLOAD_LEN: usize = codec::TENSOR_BLOCK_PAYLOAD_LEN;
const BLOCK_VOTE_PAYLOAD_LEN: usize = codec::BLOCK_VOTE_PAYLOAD_LEN;
const VALIDATOR_AUDIT_REPORT_PAYLOAD_LEN: usize = codec::VALIDATOR_AUDIT_REPORT_PAYLOAD_LEN;

pub fn gossip_topic_for_message(message: &P2pMessage) -> Option<GossipTopic> {
    match message {
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. } => Some(GossipTopic::Blocks),
        P2pMessage::NewJob(_) | P2pMessage::NewJobPayload { .. } => Some(GossipTopic::Jobs),
        P2pMessage::NewReceipt(_) | P2pMessage::NewReceiptPayload { .. } => {
            Some(GossipTopic::Receipts)
        }
        P2pMessage::NewAttestation(_) | P2pMessage::NewAttestationPayload { .. } => {
            Some(GossipTopic::Attestations)
        }
        P2pMessage::NewValidatorAuditReport(_)
        | P2pMessage::NewValidatorAuditReportPayload { .. } => Some(GossipTopic::Attestations),
        P2pMessage::PeerInfo { .. } => Some(GossipTopic::Peers),
        P2pMessage::RequestTensorChunk { .. }
        | P2pMessage::TensorChunkResponse { .. }
        | P2pMessage::RequestTensorRow { .. }
        | P2pMessage::TensorRowResponse { .. }
        | P2pMessage::RequestTensorByCommitmentRoot { .. }
        | P2pMessage::TensorByCommitmentRootResponse { .. }
        | P2pMessage::RequestProgram(_)
        | P2pMessage::ProgramResponse { .. } => None,
    }
}

pub fn request_response_protocol_for_message(
    message: &P2pMessage,
) -> Option<RequestResponseProtocol> {
    match message {
        P2pMessage::RequestTensorChunk { .. } | P2pMessage::TensorChunkResponse { .. } => {
            Some(RequestResponseProtocol::TensorChunk)
        }
        P2pMessage::RequestTensorRow { .. } | P2pMessage::TensorRowResponse { .. } => {
            Some(RequestResponseProtocol::TensorRow)
        }
        P2pMessage::RequestTensorByCommitmentRoot { .. }
        | P2pMessage::TensorByCommitmentRootResponse { .. } => {
            Some(RequestResponseProtocol::TensorByRoot)
        }
        P2pMessage::RequestProgram(_) | P2pMessage::ProgramResponse { .. } => {
            Some(RequestResponseProtocol::Program)
        }
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. }
        | P2pMessage::NewJob(_)
        | P2pMessage::NewJobPayload { .. }
        | P2pMessage::NewReceipt(_)
        | P2pMessage::NewReceiptPayload { .. }
        | P2pMessage::NewAttestation(_)
        | P2pMessage::NewAttestationPayload { .. }
        | P2pMessage::NewValidatorAuditReport(_)
        | P2pMessage::NewValidatorAuditReportPayload { .. }
        | P2pMessage::PeerInfo { .. } => None,
    }
}

pub(super) fn is_request_response_request(message: &P2pMessage) -> bool {
    matches!(
        message,
        P2pMessage::RequestTensorChunk { .. }
            | P2pMessage::RequestTensorRow { .. }
            | P2pMessage::RequestTensorByCommitmentRoot { .. }
            | P2pMessage::RequestProgram(_)
    )
}

pub fn gossipsub_ident_topic(topic: GossipTopic) -> libp2p::gossipsub::IdentTopic {
    libp2p::gossipsub::IdentTopic::new(topic.as_str())
}

pub fn request_response_stream_protocol(
    protocol: RequestResponseProtocol,
) -> TvmResult<StreamProtocol> {
    StreamProtocol::try_from_owned(protocol.as_str().to_owned())
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p stream protocol"))
}

pub fn encode_gossipsub_message(
    message: &P2pMessage,
) -> TvmResult<(libp2p::gossipsub::IdentTopic, Vec<u8>)> {
    let topic = gossip_topic_for_message(message).ok_or(TvmError::InvalidReceipt(
        "message is not a gossipsub announcement",
    ))?;
    Ok((gossipsub_ident_topic(topic), encode_message(message)))
}

pub fn encode_message(message: &P2pMessage) -> Vec<u8> {
    let mut out = Vec::new();
    match message {
        P2pMessage::NewBlock(hash) => {
            out.push(1);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewBlockHeader { height, block_hash } => {
            out.push(12);
            write_u64(&mut out, *height);
            write_hash(&mut out, block_hash);
        }
        P2pMessage::NewBlockPayload {
            height,
            block_hash,
            payload,
        } => {
            out.push(18);
            write_u64(&mut out, *height);
            write_hash(&mut out, block_hash);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewBlockVotePayload {
            block_hash,
            validator,
            payload,
        } => {
            out.push(19);
            write_hash(&mut out, block_hash);
            write_hash(&mut out, validator);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewJob(hash) => {
            out.push(2);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewJobPayload { job_id, payload } => {
            out.push(13);
            write_hash(&mut out, job_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewReceipt(hash) => {
            out.push(3);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewReceiptPayload {
            receipt_id,
            payload,
        } => {
            out.push(14);
            write_hash(&mut out, receipt_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewAttestation(hash) => {
            out.push(4);
            write_hash(&mut out, hash);
        }
        P2pMessage::NewAttestationPayload {
            attestation_id,
            payload,
        } => {
            out.push(15);
            write_hash(&mut out, attestation_id);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewValidatorAuditReport(audit_id) => {
            out.push(20);
            write_hash(&mut out, audit_id);
        }
        P2pMessage::NewValidatorAuditReportPayload {
            audit_id,
            auditor,
            payload,
        } => {
            out.push(21);
            write_hash(&mut out, audit_id);
            write_hash(&mut out, auditor);
            write_bytes(&mut out, payload);
        }
        P2pMessage::RequestTensorChunk {
            tensor_id,
            chunk_index,
        } => {
            out.push(5);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
        }
        P2pMessage::TensorChunkResponse {
            tensor_id,
            chunk_index,
            bytes,
        } => {
            out.push(6);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *chunk_index);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::RequestTensorRow {
            tensor_id,
            row_index,
        } => {
            out.push(7);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
        }
        P2pMessage::TensorRowResponse {
            tensor_id,
            row_index,
            values,
        } => {
            out.push(8);
            write_hash(&mut out, tensor_id);
            write_u64(&mut out, *row_index);
            write_u64(&mut out, values.len() as u64);
            for value in values {
                write_u64(&mut out, *value);
            }
        }
        P2pMessage::RequestTensorByCommitmentRoot { commitment_root } => {
            out.push(16);
            write_hash(&mut out, commitment_root);
        }
        P2pMessage::TensorByCommitmentRootResponse {
            commitment_root,
            payload,
        } => {
            out.push(17);
            write_hash(&mut out, commitment_root);
            write_optional_bytes(&mut out, payload.as_deref());
        }
        P2pMessage::RequestProgram(hash) => {
            out.push(9);
            write_hash(&mut out, hash);
        }
        P2pMessage::ProgramResponse {
            program_hash,
            bytes,
        } => {
            out.push(10);
            write_hash(&mut out, program_hash);
            write_bytes(&mut out, bytes);
        }
        P2pMessage::PeerInfo { address } => {
            out.push(11);
            write_hash(&mut out, address);
        }
    }
    out
}

pub fn decode_message(input: &[u8]) -> TvmResult<P2pMessage> {
    let mut reader = Reader::new(input);
    let tag = reader.read_u8()?;
    let message = match tag {
        1 => P2pMessage::NewBlock(reader.read_hash()?),
        12 => P2pMessage::NewBlockHeader {
            height: reader.read_u64()?,
            block_hash: reader.read_hash()?,
        },
        18 => {
            let height = reader.read_u64()?;
            let block_hash = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(BLOCK_PAYLOAD_LEN)?;
            let block = decode_block_payload(&payload)?;
            if block.height != height || block.hash() != block_hash {
                return Err(TvmError::InvalidReceipt(
                    "block payload announcement mismatch",
                ));
            }
            P2pMessage::NewBlockPayload {
                height,
                block_hash,
                payload,
            }
        }
        19 => {
            let block_hash = reader.read_hash()?;
            let validator = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(BLOCK_VOTE_PAYLOAD_LEN)?;
            let vote = decode_block_vote_payload(&payload)?;
            if vote.block_hash != block_hash || vote.validator != validator {
                return Err(TvmError::InvalidReceipt(
                    "block vote payload announcement mismatch",
                ));
            }
            P2pMessage::NewBlockVotePayload {
                block_hash,
                validator,
                payload,
            }
        }
        2 => P2pMessage::NewJob(reader.read_hash()?),
        13 => P2pMessage::NewJobPayload {
            job_id: reader.read_hash()?,
            payload: reader.read_bytes()?,
        },
        3 => P2pMessage::NewReceipt(reader.read_hash()?),
        14 => P2pMessage::NewReceiptPayload {
            receipt_id: reader.read_hash()?,
            payload: reader.read_bytes()?,
        },
        4 => P2pMessage::NewAttestation(reader.read_hash()?),
        15 => P2pMessage::NewAttestationPayload {
            attestation_id: reader.read_hash()?,
            payload: reader.read_bytes_with_max(codec::ATTESTATION_PAYLOAD_LEN)?,
        },
        20 => P2pMessage::NewValidatorAuditReport(reader.read_hash()?),
        21 => {
            let audit_id = reader.read_hash()?;
            let auditor = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(VALIDATOR_AUDIT_REPORT_PAYLOAD_LEN)?;
            let report = decode_validator_audit_report_payload(&payload)?;
            if report.audit_id != audit_id || report.auditor != auditor {
                return Err(TvmError::InvalidReceipt(
                    "validator audit report payload announcement mismatch",
                ));
            }
            P2pMessage::NewValidatorAuditReportPayload {
                audit_id,
                auditor,
                payload,
            }
        }
        5 => P2pMessage::RequestTensorChunk {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
        },
        6 => P2pMessage::TensorChunkResponse {
            tensor_id: reader.read_hash()?,
            chunk_index: reader.read_u64()?,
            bytes: reader.read_bytes()?,
        },
        7 => P2pMessage::RequestTensorRow {
            tensor_id: reader.read_hash()?,
            row_index: reader.read_u64()?,
        },
        8 => {
            let tensor_id = reader.read_hash()?;
            let row_index = reader.read_u64()?;
            let len = usize::try_from(reader.read_u64()?)
                .map_err(|_| TvmError::InvalidReceipt("tensor row length overflow"))?;
            if len > MAX_TENSOR_VALUES {
                return Err(TvmError::InvalidReceipt("tensor row response too large"));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(reader.read_u64()?);
            }
            P2pMessage::TensorRowResponse {
                tensor_id,
                row_index,
                values,
            }
        }
        16 => P2pMessage::RequestTensorByCommitmentRoot {
            commitment_root: reader.read_hash()?,
        },
        17 => P2pMessage::TensorByCommitmentRootResponse {
            commitment_root: reader.read_hash()?,
            payload: read_optional_bytes(&mut reader)?,
        },
        9 => P2pMessage::RequestProgram(reader.read_hash()?),
        10 => P2pMessage::ProgramResponse {
            program_hash: reader.read_hash()?,
            bytes: reader.read_bytes()?,
        },
        11 => P2pMessage::PeerInfo {
            address: reader.read_hash()?,
        },
        _ => return Err(TvmError::InvalidReceipt("unknown p2p message tag")),
    };
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing p2p bytes"));
    }
    Ok(message)
}

pub fn encode_block_payload(block: &TensorBlock) -> Vec<u8> {
    codec::encode_tensor_block_payload(block)
}

pub fn decode_block_payload(input: &[u8]) -> TvmResult<TensorBlock> {
    codec::decode_tensor_block_payload(input)
        .ok_or(TvmError::InvalidReceipt("invalid block payload length"))
}

pub fn encode_block_vote_payload(vote: &BlockVote) -> Vec<u8> {
    codec::encode_block_vote_payload(vote)
}

pub fn decode_block_vote_payload(input: &[u8]) -> TvmResult<BlockVote> {
    codec::decode_block_vote_payload(input).ok_or(TvmError::InvalidReceipt(
        "invalid block vote payload length",
    ))
}

pub fn encode_tensor_payload(tensor: &Tensor) -> Vec<u8> {
    let mut out = Vec::new();
    write_usize_vec(&mut out, tensor.shape());
    out.push(tensor.dtype().tag());
    write_u64(&mut out, tensor.as_slice().len() as u64);
    for value in tensor.as_slice() {
        write_u64(&mut out, *value);
    }
    out
}

pub fn decode_tensor_payload(input: &[u8]) -> TvmResult<Tensor> {
    let mut reader = Reader::new(input);
    let shape = read_usize_vec(&mut reader, MAX_TENSOR_SHAPE_DIMS)?;
    let dtype = dtype_from_tag(reader.read_u8()?)?;
    let len = read_usize(&mut reader)?;
    if len > MAX_TENSOR_VALUES {
        return Err(TvmError::InvalidReceipt("tensor payload too large"));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(reader.read_u64()?);
    }
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing tensor payload bytes"));
    }
    Tensor::from_vec(shape, dtype, values)
}

pub fn encode_job_payload(job: &JobState) -> Vec<u8> {
    codec::encode_job_payload(job)
}

pub fn decode_job_payload(input: &[u8]) -> TvmResult<JobState> {
    codec::decode_job_payload(input, Some(MAX_JOB_SHAPE_DIMS))
        .map_err(|error| p2p_codec_error(error, "trailing job payload bytes"))
}

pub fn encode_receipt_payload(receipt: &ReceiptState) -> Vec<u8> {
    codec::encode_receipt_payload(receipt)
}

pub fn decode_receipt_payload(input: &[u8]) -> TvmResult<ReceiptState> {
    codec::decode_receipt_payload(input, Some(MAX_RECEIPT_HASHES))
        .map_err(|error| p2p_codec_error(error, "trailing receipt payload bytes"))
}

pub fn encode_attestation_payload(attestation: &ValidatorAttestation) -> Vec<u8> {
    codec::encode_attestation_payload(attestation)
}

pub fn decode_attestation_payload(input: &[u8]) -> TvmResult<ValidatorAttestation> {
    codec::decode_attestation_payload(input)
        .map_err(|error| p2p_codec_error(error, "trailing attestation payload bytes"))
}

pub fn encode_validator_audit_report_payload(report: &ValidatorAuditReport) -> Vec<u8> {
    codec::encode_validator_audit_report_payload(report)
}

pub fn decode_validator_audit_report_payload(input: &[u8]) -> TvmResult<ValidatorAuditReport> {
    codec::decode_validator_audit_report_payload(input)
        .map_err(|error| p2p_codec_error(error, "trailing validator audit report payload bytes"))
}

fn p2p_codec_error(error: CodecError, trailing_error: &'static str) -> TvmError {
    match error {
        CodecError::Truncated => TvmError::InvalidReceipt("short p2p message"),
        CodecError::TrailingBytes => TvmError::InvalidReceipt(trailing_error),
        CodecError::UnknownJobTag => TvmError::InvalidReceipt("unknown job payload tag"),
        CodecError::UnknownReceiptTag => TvmError::InvalidReceipt("unknown receipt payload tag"),
        CodecError::UnknownDType => TvmError::InvalidReceipt("unknown dtype tag"),
        CodecError::UnknownPrimitiveType => TvmError::InvalidReceipt("unknown primitive type tag"),
        CodecError::UnknownVerificationResult => {
            TvmError::InvalidReceipt("unknown verification result tag")
        }
        CodecError::InvalidOptionalU64 => TvmError::InvalidReceipt("invalid optional u64 tag"),
        CodecError::InvalidBool => TvmError::InvalidReceipt("invalid bool tag"),
        CodecError::UsizeOverflow => TvmError::InvalidReceipt("usize overflow"),
        CodecError::ShapeVectorTooLarge => TvmError::InvalidReceipt("shape vector too large"),
        CodecError::HashVectorTooLarge => TvmError::InvalidReceipt("hash vector too large"),
    }
}

fn write_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_usize_vec(out: &mut Vec<u8>, values: &[usize]) {
    write_u64(out, values.len() as u64);
    for value in values {
        write_u64(out, *value as u64);
    }
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn write_optional_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            write_bytes(out, bytes);
        }
        None => out.push(0),
    }
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_u8(&mut self) -> TvmResult<u8> {
        let Some(byte) = self.input.get(self.offset).copied() else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset += 1;
        Ok(byte)
    }

    fn read_u64(&mut self) -> TvmResult<u64> {
        let bytes = self.read_exact(8)?;
        let mut out = [0_u8; 8];
        out.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(out))
    }

    fn read_hash(&mut self) -> TvmResult<Hash> {
        let bytes = self.read_exact(32)?;
        let mut out = [0_u8; 32];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_bytes(&mut self) -> TvmResult<Vec<u8>> {
        self.read_bytes_with_max(MAX_WIRE_BYTES)
    }

    fn read_bytes_with_max(&mut self, max_len: usize) -> TvmResult<Vec<u8>> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| TvmError::InvalidReceipt("p2p byte length overflow"))?;
        if len > max_len {
            return Err(TvmError::InvalidReceipt("p2p byte payload too large"));
        }
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_exact(&mut self, len: usize) -> TvmResult<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(TvmError::InvalidReceipt("p2p length overflow"))?;
        let Some(bytes) = self.input.get(self.offset..end) else {
            return Err(TvmError::InvalidReceipt("short p2p message"));
        };
        self.offset = end;
        Ok(bytes)
    }

    fn is_done(&self) -> bool {
        self.offset == self.input.len()
    }
}

fn read_optional_bytes(reader: &mut Reader<'_>) -> TvmResult<Option<Vec<u8>>> {
    match reader.read_u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.read_bytes()?)),
        _ => Err(TvmError::InvalidReceipt("invalid optional bytes tag")),
    }
}

fn read_usize(reader: &mut Reader<'_>) -> TvmResult<usize> {
    usize::try_from(reader.read_u64()?).map_err(|_| TvmError::InvalidReceipt("usize overflow"))
}

fn read_usize_vec(reader: &mut Reader<'_>, max_len: usize) -> TvmResult<Vec<usize>> {
    let len = read_usize(reader)?;
    if len > max_len {
        return Err(TvmError::InvalidReceipt("shape vector too large"));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(read_usize(reader)?);
    }
    Ok(values)
}

fn dtype_from_tag(tag: u8) -> TvmResult<DType> {
    codec::dtype_from_tag(tag).ok_or(TvmError::InvalidReceipt("unknown dtype tag"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{BlockVote, JobState, ReceiptState, TensorBlock, ValidatorAuditReport};
    use crate::codec;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        PrimitiveType, TensorOpReceipt,
    };
    use crate::p2p::recommended_network_stack;
    use crate::scheduler::SyntheticLocalJobSource;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{AttestationStatement, ValidatorAttestation, VerificationResult};

    #[test]
    fn p2p_messages_roundtrip() {
        let h = hash_bytes(b"test", &[b"h"]);
        let peer = address(b"peer");
        let block = TensorBlock {
            height: 3,
            parent_hash: hash_bytes(b"test", &[b"parent"]),
            epoch: 1,
            proposer: address(b"block-proposer"),
            settled_receipt_set_root: hash_bytes(b"test", &[b"settled"]),
            checks_root: hash_bytes(b"test", &[b"checks"]),
            attestation_root: hash_bytes(b"test", &[b"attestations"]),
            state_root: hash_bytes(b"test", &[b"state"]),
            reward_root: hash_bytes(b"test", &[b"rewards"]),
            beacon_round: 3,
            beacon: hash_bytes(b"test", &[b"beacon"]),
            production_kind: crate::chain::BlockProductionKind::UsefulVerificationPow,
            proposer_reward: 0,
            difficulty_target: [0xff; 32],
            nonce: 7,
            timestamp: 11,
            proposer_signature: hash_bytes(b"test", &[b"proposer-signature"]),
            validator_signature_aggregate: hash_bytes(b"test", &[b"validator-signature"]),
        };
        let block_hash = block.hash();
        let block_payload = encode_block_payload(&block);
        let block_vote = BlockVote::new(address(b"block-vote-validator"), 10_000, &block);
        let tensor = Tensor::from_vec(vec![1, 3], DType::FieldElement, vec![9, 8, 7]).unwrap();
        let tensor_root = tensor.commitment_root();
        let tensor_payload = encode_tensor_payload(&tensor);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 1, 2, 3, 4, &h, 10));
        let miner = address(b"payload-miner");
        let receipt = ReceiptState::TensorOp(
            TensorOpReceipt::from_job(
                match &job {
                    JobState::TensorOp(job) => job,
                    JobState::LinearTrainingStep(_) => unreachable!(),
                },
                miner,
                3,
                4,
            )
            .unwrap()
            .0,
        );
        let attestation = ValidatorAttestation::new(
            address(b"payload-validator"),
            10,
            AttestationStatement {
                receipt_id: receipt.receipt_id(),
                job_id: receipt.job_id(),
                primitive_type: receipt.primitive_type(),
                result: VerificationResult::Valid,
                checks_root: h,
                data_availability_passed: true,
            },
        );
        let attestation_id = hash_bytes(
            b"test-attestation-announcement",
            &[&attestation.validator, &attestation.receipt_id],
        );
        let audit_report = ValidatorAuditReport::new(
            hash_bytes(b"test", &[b"audit-id"]),
            address(b"audit-reporter"),
            VerificationResult::Valid,
            true,
            hash_bytes(b"test", &[b"audit-checks"]),
        );
        let messages = vec![
            P2pMessage::NewBlock(h),
            P2pMessage::NewBlockHeader {
                height: 3,
                block_hash: h,
            },
            P2pMessage::NewBlockPayload {
                height: block.height,
                block_hash,
                payload: block_payload,
            },
            P2pMessage::NewBlockVotePayload {
                block_hash,
                validator: block_vote.validator,
                payload: encode_block_vote_payload(&block_vote),
            },
            P2pMessage::NewJob(h),
            P2pMessage::NewJobPayload {
                job_id: job.job_id(),
                payload: encode_job_payload(&job),
            },
            P2pMessage::NewReceipt(h),
            P2pMessage::NewReceiptPayload {
                receipt_id: receipt.receipt_id(),
                payload: encode_receipt_payload(&receipt),
            },
            P2pMessage::NewAttestation(h),
            P2pMessage::NewAttestationPayload {
                attestation_id,
                payload: encode_attestation_payload(&attestation),
            },
            P2pMessage::NewValidatorAuditReport(audit_report.audit_id),
            P2pMessage::NewValidatorAuditReportPayload {
                audit_id: audit_report.audit_id,
                auditor: audit_report.auditor,
                payload: encode_validator_audit_report_payload(&audit_report),
            },
            P2pMessage::RequestTensorChunk {
                tensor_id: h,
                chunk_index: 7,
            },
            P2pMessage::TensorChunkResponse {
                tensor_id: h,
                chunk_index: 7,
                bytes: vec![1, 2, 3],
            },
            P2pMessage::RequestTensorRow {
                tensor_id: h,
                row_index: 9,
            },
            P2pMessage::TensorRowResponse {
                tensor_id: h,
                row_index: 9,
                values: vec![4, 5, 6],
            },
            P2pMessage::RequestTensorByCommitmentRoot {
                commitment_root: tensor_root,
            },
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: tensor_root,
                payload: Some(tensor_payload),
            },
            P2pMessage::TensorByCommitmentRootResponse {
                commitment_root: h,
                payload: None,
            },
            P2pMessage::RequestProgram(h),
            P2pMessage::ProgramResponse {
                program_hash: h,
                bytes: vec![7, 8],
            },
            P2pMessage::PeerInfo { address: peer },
        ];

        for message in messages {
            assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);
        }
    }

    #[test]
    fn tensor_payloads_roundtrip_and_reject_malformed_edges() {
        let tensor = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let payload = encode_tensor_payload(&tensor);
        assert_eq!(decode_tensor_payload(&payload).unwrap(), tensor);

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_tensor_payload(&trailing).is_err());

        let mut oversized_shape = Vec::new();
        write_u64(&mut oversized_shape, (MAX_TENSOR_SHAPE_DIMS + 1) as u64);
        assert!(decode_tensor_payload(&oversized_shape).is_err());

        let mut oversized_values = Vec::new();
        write_usize_vec(&mut oversized_values, &[1]);
        oversized_values.push(DType::FieldElement.tag());
        write_u64(&mut oversized_values, (MAX_TENSOR_VALUES + 1) as u64);
        assert!(decode_tensor_payload(&oversized_values).is_err());
    }

    #[test]
    fn block_payloads_roundtrip_and_reject_malformed_edges() {
        let block = wire_test_block(b"block-payload-codec", 9);
        let payload = encode_block_payload(&block);

        assert_eq!(decode_block_payload(&payload).unwrap(), block);
        assert!(decode_block_payload(&payload[..payload.len() - 1]).is_err());

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_block_payload(&trailing).is_err());

        let mut wrong_hash = encode_message(&P2pMessage::NewBlockPayload {
            height: block.height,
            block_hash: hash_bytes(b"test", &[b"wrong-block-payload-hash"]),
            payload,
        });
        assert!(decode_message(&wrong_hash).is_err());
        wrong_hash.pop();
        assert!(decode_message(&wrong_hash).is_err());
    }

    #[test]
    fn block_vote_payloads_roundtrip_and_reject_malformed_edges() {
        let block = wire_test_block(b"block-vote-payload-codec", 10);
        let vote = BlockVote::new(address(b"block-vote-codec-validator"), 10_000, &block);
        let payload = encode_block_vote_payload(&vote);

        assert_eq!(decode_block_vote_payload(&payload).unwrap(), vote);
        assert!(decode_block_vote_payload(&payload[..payload.len() - 1]).is_err());

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_block_vote_payload(&trailing).is_err());

        let mut wrong_hash = encode_message(&P2pMessage::NewBlockVotePayload {
            block_hash: hash_bytes(b"test", &[b"wrong-block-vote-hash"]),
            validator: vote.validator,
            payload: payload.clone(),
        });
        assert!(decode_message(&wrong_hash).is_err());
        wrong_hash.pop();
        assert!(decode_message(&wrong_hash).is_err());

        let wrong_validator = encode_message(&P2pMessage::NewBlockVotePayload {
            block_hash: vote.block_hash,
            validator: address(b"wrong-block-vote-validator"),
            payload,
        });
        assert!(decode_message(&wrong_validator).is_err());
    }

    #[test]
    fn validator_audit_report_payloads_roundtrip_and_reject_malformed_edges() {
        let report = ValidatorAuditReport::new(
            hash_bytes(b"test", &[b"audit-payload-id"]),
            address(b"audit-payload-auditor"),
            VerificationResult::Invalid,
            false,
            hash_bytes(b"test", &[b"audit-payload-checks"]),
        );
        let payload = encode_validator_audit_report_payload(&report);
        assert_eq!(
            decode_validator_audit_report_payload(&payload).unwrap(),
            report
        );
        assert_eq!(
            decode_message(&encode_message(
                &P2pMessage::NewValidatorAuditReportPayload {
                    audit_id: report.audit_id,
                    auditor: report.auditor,
                    payload: payload.clone(),
                }
            ))
            .unwrap(),
            P2pMessage::NewValidatorAuditReportPayload {
                audit_id: report.audit_id,
                auditor: report.auditor,
                payload: payload.clone(),
            }
        );

        let mut wrong_audit = encode_message(&P2pMessage::NewValidatorAuditReportPayload {
            audit_id: report.audit_id,
            auditor: report.auditor,
            payload: payload.clone(),
        });
        wrong_audit[1] ^= 0x55;
        assert!(decode_message(&wrong_audit).is_err());

        let mut wrong_auditor = encode_message(&P2pMessage::NewValidatorAuditReportPayload {
            audit_id: report.audit_id,
            auditor: report.auditor,
            payload: payload.clone(),
        });
        wrong_auditor[33] ^= 0x55;
        assert!(decode_message(&wrong_auditor).is_err());

        assert!(decode_validator_audit_report_payload(&payload[..payload.len() - 1]).is_err());
        let mut trailing = payload;
        trailing.push(0);
        assert!(decode_validator_audit_report_payload(&trailing).is_err());
        let mut unknown_result = encode_validator_audit_report_payload(&report);
        unknown_result[64] = 255;
        assert!(decode_validator_audit_report_payload(&unknown_result).is_err());
    }

    #[test]
    fn tensor_row_response_rejects_oversized_len_before_allocation() {
        let mut payload = Vec::new();
        payload.push(8);
        write_hash(&mut payload, &hash_bytes(b"test", &[b"oversized-row"]));
        write_u64(&mut payload, 0);
        write_u64(&mut payload, (MAX_TENSOR_VALUES + 1) as u64);

        assert!(decode_message(&payload).is_err());
    }

    #[test]
    fn job_payloads_roundtrip_and_reject_bad_shape_payloads() {
        let beacon = hash_bytes(b"test", &[b"job-payload"]);
        let tensor_job = JobState::TensorOp(MatmulJob::synthetic(3, 4, 5, 6, 7, &beacon, 20));
        assert_eq!(
            decode_job_payload(&encode_job_payload(&tensor_job)).unwrap(),
            tensor_job
        );

        let weights =
            Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let linear_job = JobState::LinearTrainingStep(
            crate::jobs::LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
                model_id: hash_bytes(b"test", &[b"model"]),
                step: 2,
                batch_seed: hash_bytes(b"test", &[b"batch"]),
                weight_root_before: weights.commitment_root(),
                input_shape: vec![4, 3],
                weight_shape: vec![3, 2],
                target_shape: vec![4, 2],
                lr: 2,
                deadline_block: 30,
            }),
        );
        assert_eq!(
            decode_job_payload(&encode_job_payload(&linear_job)).unwrap(),
            linear_job
        );

        let mut oversized_shape = Vec::new();
        oversized_shape.push(2);
        write_hash(&mut oversized_shape, &hash_bytes(b"test", &[b"bad-job"]));
        write_hash(&mut oversized_shape, &hash_bytes(b"test", &[b"bad-model"]));
        write_u64(&mut oversized_shape, 0);
        write_hash(&mut oversized_shape, &hash_bytes(b"test", &[b"bad-batch"]));
        write_hash(
            &mut oversized_shape,
            &SyntheticLocalJobSource::linear_training_weights().commitment_root(),
        );
        write_u64(&mut oversized_shape, (MAX_JOB_SHAPE_DIMS + 1) as u64);
        assert!(decode_job_payload(&oversized_shape).is_err());
    }

    #[test]
    fn job_payload_decoder_covers_optional_dtype_and_malformed_edges() {
        let beacon = hash_bytes(b"test", &[b"job-payload-edges"]);
        let base_job = MatmulJob::synthetic(4, 5, 2, 3, 4, &beacon, 40);

        for dtype in [DType::Int32, DType::Int64, DType::Fixed32] {
            let mut job = base_job.clone();
            job.dtype = dtype;
            job.modulus = None;
            let job = JobState::TensorOp(job);
            assert_eq!(decode_job_payload(&encode_job_payload(&job)).unwrap(), job);
        }

        let mut unknown_job_tag = encode_job_payload(&JobState::TensorOp(base_job.clone()));
        unknown_job_tag[0] = 99;
        assert!(decode_job_payload(&unknown_job_tag).is_err());

        let mut trailing_payload = encode_job_payload(&JobState::TensorOp(base_job.clone()));
        trailing_payload.push(0);
        assert!(decode_job_payload(&trailing_payload).is_err());

        let mut bad_optional = encode_job_payload(&JobState::TensorOp(base_job.clone()));
        bad_optional[66] = 9;
        assert!(decode_job_payload(&bad_optional).is_err());

        let mut bad_dtype = encode_job_payload(&JobState::TensorOp(base_job));
        bad_dtype[65] = 99;
        assert!(decode_job_payload(&bad_dtype).is_err());
    }

    #[test]
    fn receipt_payloads_roundtrip_and_reject_malformed_edges() {
        let beacon = hash_bytes(b"test", &[b"receipt-payload"]);
        let tensor_job = MatmulJob::synthetic(3, 4, 2, 3, 4, &beacon, 20);
        let tensor_receipt = ReceiptState::TensorOp(
            TensorOpReceipt::from_job(&tensor_job, address(b"tensor-miner"), 5, 6)
                .unwrap()
                .0,
        );
        assert_eq!(
            decode_receipt_payload(&encode_receipt_payload(&tensor_receipt)).unwrap(),
            tensor_receipt
        );

        let weights = SyntheticLocalJobSource::linear_training_weights();
        let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id: hash_bytes(b"test", &[b"receipt-model"]),
            step: 3,
            batch_seed: hash_bytes(b"test", &[b"receipt-batch"]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: 30,
        });
        let linear_receipt = ReceiptState::LinearTrainingStep(
            LinearTrainingStepReceipt::from_job(
                &linear_job,
                address(b"linear-miner"),
                &weights,
                7,
                8,
            )
            .unwrap()
            .0,
        );
        assert_eq!(
            decode_receipt_payload(&encode_receipt_payload(&linear_receipt)).unwrap(),
            linear_receipt
        );

        let mut unknown_receipt_tag = encode_receipt_payload(&tensor_receipt);
        unknown_receipt_tag[0] = 99;
        assert!(decode_receipt_payload(&unknown_receipt_tag).is_err());

        let mut trailing_payload = encode_receipt_payload(&tensor_receipt);
        trailing_payload.push(0);
        assert!(decode_receipt_payload(&trailing_payload).is_err());

        let mut oversized_hashes = Vec::new();
        oversized_hashes.push(1);
        write_hash(
            &mut oversized_hashes,
            &hash_bytes(b"test", &[b"bad-receipt"]),
        );
        write_hash(&mut oversized_hashes, &tensor_job.job_id);
        write_hash(&mut oversized_hashes, &address(b"bad-miner"));
        write_hash(&mut oversized_hashes, &tensor_job.program_hash());
        write_u64(&mut oversized_hashes, (MAX_RECEIPT_HASHES + 1) as u64);
        assert!(decode_receipt_payload(&oversized_hashes).is_err());
    }

    #[test]
    fn attestation_payloads_roundtrip_and_reject_malformed_edges() {
        let validator = address(b"payload-validator");
        let receipt_id = hash_bytes(b"test", &[b"attested-receipt"]);
        let job_id = hash_bytes(b"test", &[b"attested-job"]);
        for (primitive_type, result) in [
            (PrimitiveType::TensorOp, VerificationResult::Valid),
            (
                PrimitiveType::LinearTrainingStep,
                VerificationResult::Invalid,
            ),
            (PrimitiveType::TensorOp, VerificationResult::Unavailable),
        ] {
            let attestation = ValidatorAttestation::new(
                validator,
                11,
                AttestationStatement {
                    receipt_id,
                    job_id,
                    primitive_type,
                    result,
                    checks_root: hash_bytes(b"test", &[&[codec::verification_result_tag(result)]]),
                    data_availability_passed: result != VerificationResult::Unavailable,
                },
            );
            assert_eq!(
                decode_attestation_payload(&encode_attestation_payload(&attestation)).unwrap(),
                attestation
            );
        }

        let attestation = ValidatorAttestation::new(
            validator,
            11,
            AttestationStatement {
                receipt_id,
                job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        );

        let mut bad_primitive = encode_attestation_payload(&attestation);
        bad_primitive[96] = 99;
        assert!(decode_attestation_payload(&bad_primitive).is_err());

        let mut bad_result = encode_attestation_payload(&attestation);
        bad_result[97] = 99;
        assert!(decode_attestation_payload(&bad_result).is_err());

        let mut bad_bool = encode_attestation_payload(&attestation);
        bad_bool[130] = 99;
        assert!(decode_attestation_payload(&bad_bool).is_err());

        let mut trailing_payload = encode_attestation_payload(&attestation);
        trailing_payload.push(0);
        assert!(decode_attestation_payload(&trailing_payload).is_err());
    }

    #[test]
    fn libp2p_mapping_separates_gossip_and_request_response() {
        let h = hash_bytes(b"test", &[b"h"]);
        let block = TensorBlock {
            height: 3,
            parent_hash: hash_bytes(b"test", &[b"mapping-parent"]),
            epoch: 1,
            proposer: address(b"mapping-proposer"),
            settled_receipt_set_root: hash_bytes(b"test", &[b"mapping-settled"]),
            checks_root: hash_bytes(b"test", &[b"mapping-checks"]),
            attestation_root: hash_bytes(b"test", &[b"mapping-attestations"]),
            state_root: hash_bytes(b"test", &[b"mapping-state"]),
            reward_root: hash_bytes(b"test", &[b"mapping-rewards"]),
            beacon_round: 3,
            beacon: hash_bytes(b"test", &[b"mapping-beacon"]),
            production_kind: crate::chain::BlockProductionKind::UsefulVerificationPow,
            proposer_reward: 0,
            difficulty_target: [0xff; 32],
            nonce: 1,
            timestamp: 2,
            proposer_signature: hash_bytes(b"test", &[b"mapping-proposer-signature"]),
            validator_signature_aggregate: hash_bytes(b"test", &[b"mapping-validator-signature"]),
        };
        let block_payload = P2pMessage::NewBlockPayload {
            height: block.height,
            block_hash: block.hash(),
            payload: encode_block_payload(&block),
        };
        let recommendation = recommended_network_stack();
        assert!(recommendation.libp2p_required);
        assert!(recommendation.consensus_transport.contains("libp2p"));
        assert!(recommendation.tensor_fetch_transport.contains("libp2p"));
        assert!(
            recommendation
                .rationale
                .iter()
                .any(|reason| reason.contains("mandatory"))
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewBlock(h)),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewBlockHeader {
                height: 3,
                block_hash: h
            }),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&block_payload),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewBlockVotePayload {
                block_hash: h,
                validator: address(b"mapping-vote-validator"),
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewJob(h)),
            Some(GossipTopic::Jobs)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewJobPayload {
                job_id: h,
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Jobs)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewReceipt(h)),
            Some(GossipTopic::Receipts)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewReceiptPayload {
                receipt_id: h,
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Receipts)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewAttestation(h)),
            Some(GossipTopic::Attestations)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewAttestationPayload {
                attestation_id: h,
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Attestations)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::PeerInfo { address: h }),
            Some(GossipTopic::Peers)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::RequestProgram(h)),
            None
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::RequestTensorByCommitmentRoot {
                commitment_root: h,
            }),
            None
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestTensorChunk {
                tensor_id: h,
                chunk_index: 0,
            }),
            Some(RequestResponseProtocol::TensorChunk)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestTensorRow {
                tensor_id: h,
                row_index: 0,
            }),
            Some(RequestResponseProtocol::TensorRow)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestTensorByCommitmentRoot {
                commitment_root: h,
            }),
            Some(RequestResponseProtocol::TensorByRoot)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::RequestProgram(h)),
            Some(RequestResponseProtocol::Program)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewBlock(h)),
            None
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewBlockHeader {
                height: 3,
                block_hash: h
            }),
            None
        );
        assert_eq!(request_response_protocol_for_message(&block_payload), None);
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewBlockVotePayload {
                block_hash: h,
                validator: address(b"mapping-vote-validator"),
                payload: vec![1, 2, 3],
            }),
            None
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewReceiptPayload {
                receipt_id: h,
                payload: vec![1, 2, 3],
            }),
            None
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewAttestationPayload {
                attestation_id: h,
                payload: vec![1, 2, 3],
            }),
            None
        );
        assert_eq!(
            gossipsub_ident_topic(GossipTopic::Blocks).to_string(),
            "/tensorchain/1/blocks"
        );
        assert_eq!(
            request_response_stream_protocol(RequestResponseProtocol::TensorRow)
                .unwrap()
                .to_string(),
            "/tensorchain/1/tensor/row"
        );
        assert_eq!(
            request_response_stream_protocol(RequestResponseProtocol::TensorByRoot)
                .unwrap()
                .to_string(),
            "/tensorchain/1/tensor/by-root"
        );
    }

    #[test]
    fn gossipsub_encoding_rejects_request_response_messages() {
        let h = hash_bytes(b"test", &[b"gossipsub-encode"]);
        let (topic, payload) = encode_gossipsub_message(&P2pMessage::NewBlock(h)).unwrap();
        assert_eq!(topic.to_string(), "/tensorchain/1/blocks");
        assert_eq!(decode_message(&payload).unwrap(), P2pMessage::NewBlock(h));
        match encode_gossipsub_message(&P2pMessage::RequestProgram(h)) {
            Err(error) => assert_eq!(
                error,
                TvmError::InvalidReceipt("message is not a gossipsub announcement")
            ),
            Ok(_) => panic!("request-response message encoded as gossipsub"),
        }
    }

    #[test]
    fn rejects_trailing_or_short_messages() {
        let mut encoded = encode_message(&P2pMessage::NewJob(hash_bytes(b"test", &[b"job"])));
        encoded.push(0);
        assert!(decode_message(&encoded).is_err());
        assert!(decode_message(&[1, 2, 3]).is_err());
    }

    #[test]
    fn rejects_malformed_payloads() {
        let h = hash_bytes(b"test", &[b"malformed-p2p"]);
        assert_eq!(
            decode_message(&[]),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );
        assert_eq!(
            decode_message(&[99]),
            Err(TvmError::InvalidReceipt("unknown p2p message tag"))
        );

        let mut short_hash = vec![5];
        short_hash.extend_from_slice(&h[..8]);
        assert_eq!(
            decode_message(&short_hash),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );

        let mut truncated_bytes = vec![6];
        write_hash(&mut truncated_bytes, &h);
        write_u64(&mut truncated_bytes, 1);
        write_u64(&mut truncated_bytes, 4);
        truncated_bytes.extend_from_slice(&[1, 2]);
        assert_eq!(
            decode_message(&truncated_bytes),
            Err(TvmError::InvalidReceipt("short p2p message"))
        );
    }

    fn wire_test_block(label: &[u8], height: u64) -> TensorBlock {
        TensorBlock {
            height,
            parent_hash: hash_bytes(b"test-block", &[label, b"parent"]),
            epoch: height / 4,
            proposer: hash_bytes(b"test-block", &[label, b"proposer"]),
            settled_receipt_set_root: hash_bytes(b"test-block", &[label, b"settled"]),
            checks_root: hash_bytes(b"test-block", &[label, b"checks"]),
            attestation_root: hash_bytes(b"test-block", &[label, b"attestations"]),
            state_root: hash_bytes(b"test-block", &[label, b"state"]),
            reward_root: hash_bytes(b"test-block", &[label, b"rewards"]),
            beacon_round: height,
            beacon: hash_bytes(b"test-block", &[label, b"beacon"]),
            production_kind: crate::chain::BlockProductionKind::UsefulVerificationPow,
            proposer_reward: 0,
            difficulty_target: [0xff; 32],
            nonce: height.saturating_add(1),
            timestamp: height.saturating_mul(6),
            proposer_signature: hash_bytes(b"test-block", &[label, b"proposer-signature"]),
            validator_signature_aggregate: hash_bytes(
                b"test-block",
                &[label, b"validator-signature"],
            ),
        }
    }
}
