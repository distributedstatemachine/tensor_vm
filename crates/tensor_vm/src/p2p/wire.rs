use crate::api::P2pMessage;
use crate::chain::{
    BlockVote, ChainState, JobState, ReceiptState, TensorBlock, ValidatorAuditReport,
    ValidatorVrfRevealRecord,
};
use crate::challenge::{BlockCheckChallenge, TraceBisectionRound, block_check_challenge_id};
use crate::codec::{self, CodecError};
use crate::error::{Result as TvmResult, TvmError};
use crate::ir::{IrOpTrace, IrTraceOpening};
use crate::merkle::MerkleProof;
use crate::storage::{decode_chain_state_snapshot, encode_chain_state_snapshot};
use crate::tensor::{DType, Tensor};
use crate::types::{Hash, verify_signature};
use crate::verify::ValidatorAttestation;
use libp2p::StreamProtocol;

use super::{GossipTopic, RequestResponseProtocol};

pub(super) const MAX_JOB_SHAPE_DIMS: usize = 16;
pub(super) const MAX_RECEIPT_HASHES: usize = 16;
pub(super) const MAX_TENSOR_SHAPE_DIMS: usize = 16;
pub(super) const MAX_TENSOR_VALUES: usize = 1_000_000;
const MAX_TRACE_OUTPUT_ROOTS: usize = 16;
const MAX_TRACE_PROOF_SIBLINGS: usize = 64;
const TRACE_OPENING_PAYLOAD_LEN: usize =
    8 + 8 + 8 + 8 + MAX_TRACE_OUTPUT_ROOTS * 32 + 8 + 8 + MAX_TRACE_PROOF_SIBLINGS * 32;
const MAX_TRACE_BISECTION_EXPECTED_ROOTS: usize = MAX_TRACE_OUTPUT_ROOTS;
const TRACE_BISECTION_ROUND_PAYLOAD_MAX_LEN: usize = 32
    + 32
    + 32
    + 32
    + 8
    + 8
    + 8
    + 8
    + MAX_TRACE_BISECTION_EXPECTED_ROOTS * 32
    + 8
    + TRACE_OPENING_PAYLOAD_LEN
    + 8
    + 8
    + 8
    + 32;
const MAX_WIRE_BYTES: usize = 16 * 1024 * 1024;
const BLOCK_PAYLOAD_LEN: usize = codec::TENSOR_BLOCK_PAYLOAD_LEN;
const BLOCK_PAYLOAD_SELECTION_MAGIC: &[u8; 8] = b"TVMBSL1\0";
const MAX_BLOCK_PAYLOAD_SELECTED_RECEIPTS: usize = 64;
const MAX_BLOCK_PARENT_STATE_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const BLOCK_PAYLOAD_MAX_LEN: usize = BLOCK_PAYLOAD_LEN
    + 8
    + 8
    + MAX_BLOCK_PAYLOAD_SELECTED_RECEIPTS * 32
    + 8
    + MAX_BLOCK_PARENT_STATE_PAYLOAD_BYTES;
const BLOCK_VOTE_PAYLOAD_LEN: usize = codec::BLOCK_VOTE_PAYLOAD_LEN;
const VALIDATOR_AUDIT_REPORT_PAYLOAD_LEN: usize = codec::VALIDATOR_AUDIT_REPORT_PAYLOAD_LEN;
const BLOCK_CHECK_CHALLENGE_PAYLOAD_LEN: usize = codec::BLOCK_CHECK_CHALLENGE_PAYLOAD_MAX_LEN;
const EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES: usize = 96;
const EXTERNAL_RANDOMNESS_BEACON_PAYLOAD_MAX_LEN: usize =
    8 + EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES + 8 + 32 + 32;
const DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES: usize = 48;
const DRAND_PEDERSEN_BLS_SIGNATURE_BYTES: usize = 96;
const VERIFIED_DRAND_BEACON_PAYLOAD_MAX_LEN: usize = 8
    + EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES
    + 8
    + 8
    + DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES
    + 8
    + DRAND_PEDERSEN_BLS_SIGNATURE_BYTES;
const DRAND_PEDERSEN_BLS_PREVIOUS_SIGNATURE_BYTES: usize = 96;
const VERIFIED_CHAINED_DRAND_BEACON_PAYLOAD_MAX_LEN: usize =
    VERIFIED_DRAND_BEACON_PAYLOAD_MAX_LEN + 8 + DRAND_PEDERSEN_BLS_PREVIOUS_SIGNATURE_BYTES;
const VALIDATOR_VRF_PROOF_MAX_BYTES: usize = 64;
const VALIDATOR_VRF_REVEAL_PAYLOAD_MAX_LEN: usize =
    32 + 32 + 32 + 32 + 8 + 8 + 32 + 32 + 32 + 8 + VALIDATOR_VRF_PROOF_MAX_BYTES + 32 + 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalRandomnessBeaconPayload {
    pub source_id: String,
    pub beacon_round: u64,
    pub randomness: Hash,
    pub proof_hash: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDrandBeaconPayload {
    pub source_id: String,
    pub beacon_round: u64,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedChainedDrandBeaconPayload {
    pub source_id: String,
    pub beacon_round: u64,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub previous_signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatorVrfRevealPayload {
    pub reveal: ValidatorVrfRevealRecord,
}

pub fn gossip_topic_for_message(message: &P2pMessage) -> Option<GossipTopic> {
    match message {
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. }
        | P2pMessage::NewBlockCheckChallenge(_)
        | P2pMessage::NewBlockCheckChallengePayload { .. }
        | P2pMessage::NewObservedBlockCheckChallengePayload { .. }
        | P2pMessage::NewTraceBisectionRoundPayload { .. } => Some(GossipTopic::Blocks),
        P2pMessage::NewJob(_) | P2pMessage::NewJobPayload { .. } => Some(GossipTopic::Jobs),
        P2pMessage::NewReceipt(_) | P2pMessage::NewReceiptPayload { .. } => {
            Some(GossipTopic::Receipts)
        }
        P2pMessage::NewAttestation(_) | P2pMessage::NewAttestationPayload { .. } => {
            Some(GossipTopic::Attestations)
        }
        P2pMessage::NewValidatorAuditReport(_)
        | P2pMessage::NewValidatorAuditReportPayload { .. } => Some(GossipTopic::Attestations),
        P2pMessage::NewExternalRandomnessBeaconPayload { .. }
        | P2pMessage::NewVerifiedDrandBeaconPayload { .. }
        | P2pMessage::NewVerifiedChainedDrandBeaconPayload { .. } => Some(GossipTopic::Blocks),
        P2pMessage::NewValidatorVrfRevealPayload { .. } => Some(GossipTopic::Attestations),
        P2pMessage::PeerInfo { .. } => Some(GossipTopic::Peers),
        P2pMessage::RequestTensorChunk { .. }
        | P2pMessage::TensorChunkResponse { .. }
        | P2pMessage::RequestTensorRow { .. }
        | P2pMessage::TensorRowResponse { .. }
        | P2pMessage::RequestTensorByCommitmentRoot { .. }
        | P2pMessage::TensorByCommitmentRootResponse { .. }
        | P2pMessage::RequestProgram(_)
        | P2pMessage::ProgramResponse { .. }
        | P2pMessage::RequestTraceOpening { .. }
        | P2pMessage::TraceOpeningResponse { .. } => None,
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
        P2pMessage::RequestTraceOpening { .. } | P2pMessage::TraceOpeningResponse { .. } => {
            Some(RequestResponseProtocol::TraceOpening)
        }
        P2pMessage::NewBlock(_)
        | P2pMessage::NewBlockHeader { .. }
        | P2pMessage::NewBlockPayload { .. }
        | P2pMessage::NewBlockVotePayload { .. }
        | P2pMessage::NewBlockCheckChallenge(_)
        | P2pMessage::NewBlockCheckChallengePayload { .. }
        | P2pMessage::NewObservedBlockCheckChallengePayload { .. }
        | P2pMessage::NewTraceBisectionRoundPayload { .. }
        | P2pMessage::NewJob(_)
        | P2pMessage::NewJobPayload { .. }
        | P2pMessage::NewReceipt(_)
        | P2pMessage::NewReceiptPayload { .. }
        | P2pMessage::NewAttestation(_)
        | P2pMessage::NewAttestationPayload { .. }
        | P2pMessage::NewValidatorAuditReport(_)
        | P2pMessage::NewValidatorAuditReportPayload { .. }
        | P2pMessage::NewExternalRandomnessBeaconPayload { .. }
        | P2pMessage::NewVerifiedDrandBeaconPayload { .. }
        | P2pMessage::NewVerifiedChainedDrandBeaconPayload { .. }
        | P2pMessage::NewValidatorVrfRevealPayload { .. }
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
            | P2pMessage::RequestTraceOpening { .. }
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
        P2pMessage::NewBlockCheckChallenge(challenge_id) => {
            out.push(22);
            write_hash(&mut out, challenge_id);
        }
        P2pMessage::NewBlockCheckChallengePayload {
            challenge_id,
            block_hash,
            challenger,
            payload,
        } => {
            out.push(23);
            write_hash(&mut out, challenge_id);
            write_hash(&mut out, block_hash);
            write_hash(&mut out, challenger);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewObservedBlockCheckChallengePayload {
            challenge_id,
            block_hash,
            challenger,
            observed_block_payload,
            challenge_payload,
        } => {
            out.push(24);
            write_hash(&mut out, challenge_id);
            write_hash(&mut out, block_hash);
            write_hash(&mut out, challenger);
            write_bytes(&mut out, observed_block_payload);
            write_bytes(&mut out, challenge_payload);
        }
        P2pMessage::NewTraceBisectionRoundPayload {
            receipt_id,
            trace_root,
            challenger,
            responder,
            transcript_leaf,
            payload,
        } => {
            out.push(31);
            write_hash(&mut out, receipt_id);
            write_hash(&mut out, trace_root);
            write_hash(&mut out, challenger);
            write_hash(&mut out, responder);
            write_hash(&mut out, transcript_leaf);
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
        P2pMessage::NewExternalRandomnessBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } => {
            out.push(27);
            write_string(&mut out, source_id);
            write_u64(&mut out, *beacon_round);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } => {
            out.push(29);
            write_string(&mut out, source_id);
            write_u64(&mut out, *beacon_round);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewVerifiedChainedDrandBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } => {
            out.push(30);
            write_string(&mut out, source_id);
            write_u64(&mut out, *beacon_round);
            write_bytes(&mut out, payload);
        }
        P2pMessage::NewValidatorVrfRevealPayload {
            reveal_id,
            receipt_id,
            validator,
            payload,
        } => {
            out.push(28);
            write_hash(&mut out, reveal_id);
            write_hash(&mut out, receipt_id);
            write_hash(&mut out, validator);
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
        P2pMessage::RequestTraceOpening {
            trace_root,
            op_index,
        } => {
            out.push(25);
            write_hash(&mut out, trace_root);
            write_u64(&mut out, *op_index);
        }
        P2pMessage::TraceOpeningResponse {
            trace_root,
            op_index,
            payload,
        } => {
            out.push(26);
            write_hash(&mut out, trace_root);
            write_u64(&mut out, *op_index);
            write_optional_bytes(&mut out, payload.as_deref());
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
            let payload = reader.read_bytes_with_max(BLOCK_PAYLOAD_MAX_LEN)?;
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
        22 => P2pMessage::NewBlockCheckChallenge(reader.read_hash()?),
        23 => {
            let challenge_id = reader.read_hash()?;
            let block_hash = reader.read_hash()?;
            let challenger = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(BLOCK_CHECK_CHALLENGE_PAYLOAD_LEN)?;
            let challenge = decode_block_check_challenge_payload(&payload)?;
            if block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id)
                != challenge_id
                || challenge.block_hash != block_hash
                || challenge.challenger != challenger
            {
                return Err(TvmError::InvalidReceipt(
                    "block check challenge payload announcement mismatch",
                ));
            }
            P2pMessage::NewBlockCheckChallengePayload {
                challenge_id,
                block_hash,
                challenger,
                payload,
            }
        }
        24 => {
            let challenge_id = reader.read_hash()?;
            let block_hash = reader.read_hash()?;
            let challenger = reader.read_hash()?;
            let observed_block_payload = reader.read_bytes_with_max(BLOCK_PAYLOAD_MAX_LEN)?;
            let challenge_payload =
                reader.read_bytes_with_max(BLOCK_CHECK_CHALLENGE_PAYLOAD_LEN)?;
            let observed_block = decode_block_payload(&observed_block_payload)?;
            let challenge = decode_block_check_challenge_payload(&challenge_payload)?;
            if observed_block.hash() != block_hash
                || block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id)
                    != challenge_id
                || challenge.block_hash != block_hash
                || challenge.challenger != challenger
            {
                return Err(TvmError::InvalidReceipt(
                    "observed block check challenge payload announcement mismatch",
                ));
            }
            P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id,
                block_hash,
                challenger,
                observed_block_payload,
                challenge_payload,
            }
        }
        31 => {
            let receipt_id = reader.read_hash()?;
            let trace_root = reader.read_hash()?;
            let challenger = reader.read_hash()?;
            let responder = reader.read_hash()?;
            let transcript_leaf = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(TRACE_BISECTION_ROUND_PAYLOAD_MAX_LEN)?;
            let round = decode_trace_bisection_round_payload(&payload)?;
            if round.receipt_id != receipt_id
                || round.trace_root != trace_root
                || round.challenger != challenger
                || round.responder != responder
                || round.transcript_leaf() != transcript_leaf
            {
                return Err(TvmError::InvalidReceipt(
                    "trace bisection round payload announcement mismatch",
                ));
            }
            P2pMessage::NewTraceBisectionRoundPayload {
                receipt_id,
                trace_root,
                challenger,
                responder,
                transcript_leaf,
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
        27 => {
            let source_id =
                reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
            let beacon_round = reader.read_u64()?;
            let payload = reader.read_bytes_with_max(EXTERNAL_RANDOMNESS_BEACON_PAYLOAD_MAX_LEN)?;
            let decoded = decode_external_randomness_beacon_payload(&payload)?;
            if decoded.source_id != source_id || decoded.beacon_round != beacon_round {
                return Err(TvmError::InvalidReceipt(
                    "external randomness beacon payload announcement mismatch",
                ));
            }
            P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id,
                beacon_round,
                payload,
            }
        }
        29 => {
            let source_id =
                reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
            let beacon_round = reader.read_u64()?;
            let payload = reader.read_bytes_with_max(VERIFIED_DRAND_BEACON_PAYLOAD_MAX_LEN)?;
            let decoded = decode_verified_drand_beacon_payload(&payload)?;
            if decoded.source_id != source_id || decoded.beacon_round != beacon_round {
                return Err(TvmError::InvalidReceipt(
                    "verified drand beacon payload announcement mismatch",
                ));
            }
            P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id,
                beacon_round,
                payload,
            }
        }
        30 => {
            let source_id =
                reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
            let beacon_round = reader.read_u64()?;
            let payload =
                reader.read_bytes_with_max(VERIFIED_CHAINED_DRAND_BEACON_PAYLOAD_MAX_LEN)?;
            let decoded = decode_verified_chained_drand_beacon_payload(&payload)?;
            if decoded.source_id != source_id || decoded.beacon_round != beacon_round {
                return Err(TvmError::InvalidReceipt(
                    "verified chained drand beacon payload announcement mismatch",
                ));
            }
            P2pMessage::NewVerifiedChainedDrandBeaconPayload {
                source_id,
                beacon_round,
                payload,
            }
        }
        28 => {
            let reveal_id = reader.read_hash()?;
            let receipt_id = reader.read_hash()?;
            let validator = reader.read_hash()?;
            let payload = reader.read_bytes_with_max(VALIDATOR_VRF_REVEAL_PAYLOAD_MAX_LEN)?;
            let decoded = decode_validator_vrf_reveal_payload(&payload)?;
            if decoded.reveal.reveal_id != reveal_id
                || decoded.reveal.receipt_id != receipt_id
                || decoded.reveal.validator != validator
            {
                return Err(TvmError::InvalidReceipt(
                    "validator vrf reveal payload announcement mismatch",
                ));
            }
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id,
                receipt_id,
                validator,
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
        25 => P2pMessage::RequestTraceOpening {
            trace_root: reader.read_hash()?,
            op_index: reader.read_u64()?,
        },
        26 => {
            let trace_root = reader.read_hash()?;
            let op_index = reader.read_u64()?;
            let payload = read_optional_bytes_with_max(&mut reader, TRACE_OPENING_PAYLOAD_LEN)?;
            if let Some(payload) = &payload {
                let opening = decode_trace_opening_payload(payload)?;
                if opening.trace_root != trace_root || opening.op_index != op_index {
                    return Err(TvmError::InvalidReceipt(
                        "trace opening response payload mismatch",
                    ));
                }
            }
            P2pMessage::TraceOpeningResponse {
                trace_root,
                op_index,
                payload,
            }
        }
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

pub fn encode_block_payload_with_selected_receipts(
    block: &TensorBlock,
    selected_receipts: &[Hash],
    parent_state: &ChainState,
) -> Vec<u8> {
    let mut out = encode_block_payload(block);
    out.extend_from_slice(BLOCK_PAYLOAD_SELECTION_MAGIC);
    write_u64(&mut out, selected_receipts.len() as u64);
    for receipt_id in selected_receipts {
        write_hash(&mut out, receipt_id);
    }
    write_bytes(&mut out, &encode_chain_state_snapshot(parent_state));
    out
}

pub fn decode_block_payload(input: &[u8]) -> TvmResult<TensorBlock> {
    decode_block_payload_with_selected_receipts(input).map(|decoded| decoded.0)
}

pub fn decode_block_payload_with_selected_receipts(
    input: &[u8],
) -> TvmResult<(TensorBlock, Option<Vec<Hash>>, Option<ChainState>)> {
    if input.len() == BLOCK_PAYLOAD_LEN {
        let block = codec::decode_tensor_block_payload(input)
            .ok_or(TvmError::InvalidReceipt("invalid block payload length"))?;
        return Ok((block, None, None));
    }
    if input.len() < BLOCK_PAYLOAD_LEN + BLOCK_PAYLOAD_SELECTION_MAGIC.len() + 8 {
        return Err(TvmError::InvalidReceipt("invalid block payload length"));
    }
    let block = codec::decode_tensor_block_payload(&input[..BLOCK_PAYLOAD_LEN])
        .ok_or(TvmError::InvalidReceipt("invalid block payload length"))?;
    let mut reader = Reader::new(&input[BLOCK_PAYLOAD_LEN..]);
    if reader.read_exact(BLOCK_PAYLOAD_SELECTION_MAGIC.len())? != BLOCK_PAYLOAD_SELECTION_MAGIC {
        return Err(TvmError::InvalidReceipt("unknown block payload envelope"));
    }
    let count = usize::try_from(reader.read_u64()?)
        .map_err(|_| TvmError::InvalidReceipt("block selection length overflow"))?;
    if count > MAX_BLOCK_PAYLOAD_SELECTED_RECEIPTS {
        return Err(TvmError::InvalidReceipt("block selection too large"));
    }
    let mut selected_receipts = Vec::with_capacity(count);
    for _ in 0..count {
        selected_receipts.push(reader.read_hash()?);
    }
    let parent_state_payload = reader.read_bytes_with_max(MAX_BLOCK_PARENT_STATE_PAYLOAD_BYTES)?;
    let parent_state = decode_chain_state_snapshot(&parent_state_payload)?;
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt("trailing block payload bytes"));
    }
    Ok((block, Some(selected_receipts), Some(parent_state)))
}

pub fn encode_block_vote_payload(vote: &BlockVote) -> Vec<u8> {
    codec::encode_block_vote_payload(vote)
}

pub fn decode_block_vote_payload(input: &[u8]) -> TvmResult<BlockVote> {
    codec::decode_block_vote_payload(input).ok_or(TvmError::InvalidReceipt(
        "invalid block vote payload length",
    ))
}

pub fn encode_block_check_challenge_payload(challenge: &BlockCheckChallenge) -> Vec<u8> {
    codec::encode_block_check_challenge_payload(challenge)
}

pub fn decode_block_check_challenge_payload(input: &[u8]) -> TvmResult<BlockCheckChallenge> {
    codec::decode_block_check_challenge_payload(input)
        .map_err(|error| p2p_codec_error(error, "trailing block check challenge payload bytes"))
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

pub fn encode_trace_opening_payload(opening: &IrTraceOpening) -> Vec<u8> {
    let mut out = Vec::new();
    write_hash(&mut out, &opening.trace_root);
    write_u64(&mut out, opening.op_index);
    write_u64(&mut out, opening.op_trace.op_id as u64);
    write_u64(&mut out, opening.op_trace.output_roots.len() as u64);
    for root in &opening.op_trace.output_roots {
        write_hash(&mut out, root);
    }
    write_u64(&mut out, opening.proof.leaf_index);
    write_u64(&mut out, opening.proof.siblings.len() as u64);
    for sibling in &opening.proof.siblings {
        write_hash(&mut out, sibling);
    }
    out
}

pub fn decode_trace_opening_payload(input: &[u8]) -> TvmResult<IrTraceOpening> {
    let mut reader = Reader::new(input);
    let trace_root = reader.read_hash()?;
    let op_index = reader.read_u64()?;
    let op_id = read_usize(&mut reader)?;
    let output_root_len = read_usize(&mut reader)?;
    if output_root_len > MAX_TRACE_OUTPUT_ROOTS {
        return Err(TvmError::InvalidReceipt(
            "trace opening output roots too large",
        ));
    }
    let mut output_roots = Vec::with_capacity(output_root_len);
    for _ in 0..output_root_len {
        output_roots.push(reader.read_hash()?);
    }
    let proof_leaf_index = reader.read_u64()?;
    let sibling_len = read_usize(&mut reader)?;
    if sibling_len > MAX_TRACE_PROOF_SIBLINGS {
        return Err(TvmError::InvalidReceipt("trace opening proof too large"));
    }
    let mut siblings = Vec::with_capacity(sibling_len);
    for _ in 0..sibling_len {
        siblings.push(reader.read_hash()?);
    }
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing trace opening payload bytes",
        ));
    }
    let opening = IrTraceOpening {
        trace_root,
        op_index,
        op_trace: IrOpTrace {
            op_id,
            output_roots,
        },
        proof: MerkleProof {
            leaf_index: proof_leaf_index,
            siblings,
        },
    };
    if !opening.verify() {
        return Err(TvmError::InvalidReceipt("invalid trace opening payload"));
    }
    Ok(opening)
}

pub fn encode_trace_bisection_round_payload(round: &TraceBisectionRound) -> Vec<u8> {
    let mut out = Vec::new();
    write_hash(&mut out, &round.receipt_id);
    write_hash(&mut out, &round.trace_root);
    write_hash(&mut out, &round.challenger);
    write_hash(&mut out, &round.responder);
    write_u64(&mut out, round.low_op);
    write_u64(&mut out, round.high_op);
    write_u64(&mut out, round.midpoint_op);
    write_u64(&mut out, round.expected_output_roots.len() as u64);
    for root in &round.expected_output_roots {
        write_hash(&mut out, root);
    }
    write_bytes(&mut out, &encode_trace_opening_payload(&round.opening));
    write_u64(&mut out, round.response_deadline_height);
    write_u64(&mut out, round.challenger_bond);
    write_u64(&mut out, round.responder_bond);
    write_hash(&mut out, &round.responder_signature);
    out
}

pub fn decode_trace_bisection_round_payload(input: &[u8]) -> TvmResult<TraceBisectionRound> {
    let mut reader = Reader::new(input);
    let receipt_id = reader.read_hash()?;
    let trace_root = reader.read_hash()?;
    let challenger = reader.read_hash()?;
    let responder = reader.read_hash()?;
    let low_op = reader.read_u64()?;
    let high_op = reader.read_u64()?;
    let midpoint_op = reader.read_u64()?;
    let expected_len = read_usize(&mut reader)?;
    if expected_len > MAX_TRACE_BISECTION_EXPECTED_ROOTS {
        return Err(TvmError::InvalidReceipt(
            "trace bisection expected roots too large",
        ));
    }
    let mut expected_output_roots = Vec::with_capacity(expected_len);
    for _ in 0..expected_len {
        expected_output_roots.push(reader.read_hash()?);
    }
    let opening_payload = reader.read_bytes_with_max(TRACE_OPENING_PAYLOAD_LEN)?;
    let opening = decode_trace_opening_payload(&opening_payload)?;
    let response_deadline_height = reader.read_u64()?;
    let challenger_bond = reader.read_u64()?;
    let responder_bond = reader.read_u64()?;
    let responder_signature = reader.read_hash()?;
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing trace bisection round payload bytes",
        ));
    }
    if low_op >= high_op || midpoint_op != low_op + (high_op - low_op) / 2 {
        return Err(TvmError::InvalidReceipt(
            "trace bisection midpoint mismatch",
        ));
    }
    if opening.trace_root != trace_root || opening.op_index != midpoint_op {
        return Err(TvmError::InvalidReceipt("invalid trace bisection opening"));
    }
    let round = TraceBisectionRound {
        receipt_id,
        trace_root,
        challenger,
        responder,
        low_op,
        high_op,
        midpoint_op,
        expected_output_roots,
        opening,
        response_deadline_height,
        challenger_bond,
        responder_bond,
        responder_signature,
    };
    if !verify_signature(
        &round.responder,
        &round.message_hash(),
        &round.responder_signature,
    ) {
        return Err(TvmError::InvalidReceipt(
            "trace bisection round signature mismatch",
        ));
    }
    Ok(round)
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

pub fn encode_external_randomness_beacon_payload(
    source_id: &str,
    beacon_round: u64,
    randomness: &Hash,
    proof_hash: &Hash,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, source_id);
    write_u64(&mut out, beacon_round);
    write_hash(&mut out, randomness);
    write_hash(&mut out, proof_hash);
    out
}

pub fn decode_external_randomness_beacon_payload(
    input: &[u8],
) -> TvmResult<ExternalRandomnessBeaconPayload> {
    let mut reader = Reader::new(input);
    let source_id = reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
    let beacon_round = reader.read_u64()?;
    let randomness = reader.read_hash()?;
    let proof_hash = reader.read_hash()?;
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing external randomness beacon payload bytes",
        ));
    }
    Ok(ExternalRandomnessBeaconPayload {
        source_id,
        beacon_round,
        randomness,
        proof_hash,
    })
}

pub fn encode_verified_drand_beacon_payload(
    source_id: &str,
    beacon_round: u64,
    public_key: &[u8],
    signature: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, source_id);
    write_u64(&mut out, beacon_round);
    write_bytes(&mut out, public_key);
    write_bytes(&mut out, signature);
    out
}

pub fn decode_verified_drand_beacon_payload(input: &[u8]) -> TvmResult<VerifiedDrandBeaconPayload> {
    let mut reader = Reader::new(input);
    let source_id = reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
    let beacon_round = reader.read_u64()?;
    let public_key = reader.read_bytes_with_max(DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES)?;
    if public_key.len() != DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES {
        return Err(TvmError::InvalidReceipt(
            "verified drand public key length mismatch",
        ));
    }
    let signature = reader.read_bytes_with_max(DRAND_PEDERSEN_BLS_SIGNATURE_BYTES)?;
    if signature.len() != DRAND_PEDERSEN_BLS_SIGNATURE_BYTES {
        return Err(TvmError::InvalidReceipt(
            "verified drand signature length mismatch",
        ));
    }
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing verified drand beacon payload bytes",
        ));
    }
    Ok(VerifiedDrandBeaconPayload {
        source_id,
        beacon_round,
        public_key,
        signature,
    })
}

pub fn encode_verified_chained_drand_beacon_payload(
    source_id: &str,
    beacon_round: u64,
    public_key: &[u8],
    signature: &[u8],
    previous_signature: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    write_string(&mut out, source_id);
    write_u64(&mut out, beacon_round);
    write_bytes(&mut out, public_key);
    write_bytes(&mut out, signature);
    write_bytes(&mut out, previous_signature);
    out
}

pub fn decode_verified_chained_drand_beacon_payload(
    input: &[u8],
) -> TvmResult<VerifiedChainedDrandBeaconPayload> {
    let mut reader = Reader::new(input);
    let source_id = reader.read_string_with_max(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES)?;
    let beacon_round = reader.read_u64()?;
    let public_key = reader.read_bytes_with_max(DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES)?;
    if public_key.len() != DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES {
        return Err(TvmError::InvalidReceipt(
            "verified chained drand public key length mismatch",
        ));
    }
    let signature = reader.read_bytes_with_max(DRAND_PEDERSEN_BLS_SIGNATURE_BYTES)?;
    if signature.len() != DRAND_PEDERSEN_BLS_SIGNATURE_BYTES {
        return Err(TvmError::InvalidReceipt(
            "verified chained drand signature length mismatch",
        ));
    }
    let previous_signature =
        reader.read_bytes_with_max(DRAND_PEDERSEN_BLS_PREVIOUS_SIGNATURE_BYTES)?;
    if previous_signature.is_empty() {
        return Err(TvmError::InvalidReceipt(
            "verified chained drand previous signature length mismatch",
        ));
    }
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing verified chained drand beacon payload bytes",
        ));
    }
    Ok(VerifiedChainedDrandBeaconPayload {
        source_id,
        beacon_round,
        public_key,
        signature,
        previous_signature,
    })
}

pub fn encode_validator_vrf_reveal_payload(reveal: &ValidatorVrfRevealRecord) -> Vec<u8> {
    let mut out = Vec::new();
    write_hash(&mut out, &reveal.reveal_id);
    write_hash(&mut out, &reveal.receipt_id);
    write_hash(&mut out, &reveal.job_id);
    write_hash(&mut out, &reveal.validator);
    write_u64(&mut out, reveal.beacon_round);
    write_u64(&mut out, reveal.validation_round);
    write_hash(&mut out, &reveal.vrf_output);
    write_hash(&mut out, &reveal.proof_hash);
    write_hash(&mut out, &reveal.vrf_public_key);
    write_bytes(&mut out, &reveal.vrf_proof);
    write_hash(&mut out, &reveal.signature);
    write_u64(&mut out, reveal.observed_at_height);
    out
}

pub fn decode_validator_vrf_reveal_payload(input: &[u8]) -> TvmResult<ValidatorVrfRevealPayload> {
    let mut reader = Reader::new(input);
    let reveal = ValidatorVrfRevealRecord {
        reveal_id: reader.read_hash()?,
        receipt_id: reader.read_hash()?,
        job_id: reader.read_hash()?,
        validator: reader.read_hash()?,
        beacon_round: reader.read_u64()?,
        validation_round: reader.read_u64()?,
        vrf_output: reader.read_hash()?,
        proof_hash: reader.read_hash()?,
        vrf_public_key: reader.read_hash()?,
        vrf_proof: reader.read_bytes_with_max(VALIDATOR_VRF_PROOF_MAX_BYTES)?,
        signature: reader.read_hash()?,
        observed_at_height: reader.read_u64()?,
    };
    if !reader.is_done() {
        return Err(TvmError::InvalidReceipt(
            "trailing validator vrf reveal payload bytes",
        ));
    }
    Ok(ValidatorVrfRevealPayload { reveal })
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
        CodecError::InvalidString => TvmError::InvalidReceipt("invalid string"),
        CodecError::UsizeOverflow => TvmError::InvalidReceipt("usize overflow"),
        CodecError::ShapeVectorTooLarge => TvmError::InvalidReceipt("shape vector too large"),
        CodecError::HashVectorTooLarge => TvmError::InvalidReceipt("hash vector too large"),
        CodecError::StringTooLarge => TvmError::InvalidReceipt("string too large"),
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

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_bytes(out, value.as_bytes());
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

    fn read_string_with_max(&mut self, max_len: usize) -> TvmResult<String> {
        let bytes = self.read_bytes_with_max(max_len)?;
        String::from_utf8(bytes).map_err(|_| TvmError::InvalidReceipt("invalid string"))
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
    read_optional_bytes_with_max(reader, MAX_WIRE_BYTES)
}

fn read_optional_bytes_with_max(
    reader: &mut Reader<'_>,
    max_len: usize,
) -> TvmResult<Option<Vec<u8>>> {
    match reader.read_u8()? {
        0 => Ok(None),
        1 => Ok(Some(reader.read_bytes_with_max(max_len)?)),
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
    use crate::chain::{
        BlockVote, Chain, JobState, ReceiptState, TensorBlock, ValidatorAuditReport,
    };
    use crate::challenge::{
        BlockCheckChallenge, BlockCheckChallengeInput, TraceBisectionConfig, TraceBisectionRound,
        TraceBisectionState, block_check_challenge_id,
    };
    use crate::codec;
    use crate::jobs::{
        LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
        PrimitiveType, TensorOpReceipt,
    };
    use crate::merkle::{MerkleProof, build_proof};
    use crate::p2p::recommended_network_stack;
    use crate::scheduler::SyntheticLocalJobSource;
    use crate::tensor::{DType, Tensor};
    use crate::types::{address, hash_bytes};
    use crate::verify::{AttestationStatement, ValidatorAttestation, VerificationResult};

    fn verified_drand_payload_fixture() -> (String, u64, Vec<u8>, Vec<u8>, Vec<u8>) {
        let source_id = "drand-pedersen-bls-unchained-v1:fixture".to_owned();
        let beacon_round = 223_344;
        let public_key = vec![3; DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES];
        let signature = vec![4; DRAND_PEDERSEN_BLS_SIGNATURE_BYTES];
        let payload =
            encode_verified_drand_beacon_payload(&source_id, beacon_round, &public_key, &signature);
        (source_id, beacon_round, public_key, signature, payload)
    }

    fn verified_chained_drand_payload_fixture() -> (String, u64, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)
    {
        let source_id = "drand-pedersen-bls-chained-v1:fixture".to_owned();
        let beacon_round = 1;
        let public_key = vec![5; DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES];
        let signature = vec![6; DRAND_PEDERSEN_BLS_SIGNATURE_BYTES];
        let previous_signature = vec![7; 32];
        let payload = encode_verified_chained_drand_beacon_payload(
            &source_id,
            beacon_round,
            &public_key,
            &signature,
            &previous_signature,
        );
        (
            source_id,
            beacon_round,
            public_key,
            signature,
            previous_signature,
            payload,
        )
    }

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
        let trace_opening = trace_opening_fixture();
        let trace_opening_payload = encode_trace_opening_payload(&trace_opening);
        let trace_bisection_round = trace_bisection_round_fixture();
        let trace_bisection_round_payload =
            encode_trace_bisection_round_payload(&trace_bisection_round);
        let job = JobState::TensorOp(MatmulJob::synthetic(0, 1, 2, 3, 4, &h, 10));
        let miner = address(b"payload-miner");
        let receipt = ReceiptState::TensorOp(
            TensorOpReceipt::from_job(
                match &job {
                    JobState::TensorOp(job) => job,
                    JobState::LinearTrainingStep(_) | JobState::GraphExecution(_) => unreachable!(),
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
        let challenge = wire_test_challenge(b"p2p-roundtrip-challenge");
        let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
        let observed_block = wire_test_block(b"p2p-roundtrip-observed-block", 4);
        let observed_block_payload = encode_block_payload(&observed_block);
        let observed_challenge = wire_test_challenge_for_block(
            b"p2p-roundtrip-observed-challenge",
            observed_block.hash(),
        );
        let observed_challenge_id = block_check_challenge_id(
            &observed_challenge.block_hash,
            &observed_challenge.receipt_id,
        );
        let beacon_source = "local_drand_fixture_v1".to_owned();
        let beacon_round = 42;
        let beacon_randomness = hash_bytes(b"test", &[b"external-beacon-randomness"]);
        let beacon_proof_hash = hash_bytes(b"test", &[b"external-beacon-proof"]);
        let beacon_payload = encode_external_randomness_beacon_payload(
            &beacon_source,
            beacon_round,
            &beacon_randomness,
            &beacon_proof_hash,
        );
        let (
            verified_drand_source,
            verified_drand_round,
            _verified_drand_public_key,
            _verified_drand_signature,
            verified_drand_payload,
        ) = verified_drand_payload_fixture();
        let (
            verified_chained_drand_source,
            verified_chained_drand_round,
            _verified_chained_drand_public_key,
            _verified_chained_drand_signature,
            _verified_chained_drand_previous_signature,
            verified_chained_drand_payload,
        ) = verified_chained_drand_payload_fixture();
        let vrf_reveal = ValidatorVrfRevealRecord {
            reveal_id: hash_bytes(b"test", &[b"roundtrip-reveal-id"]),
            receipt_id: receipt.receipt_id(),
            job_id: receipt.job_id(),
            validator: address(b"roundtrip-vrf-validator"),
            beacon_round,
            validation_round: 0,
            vrf_output: hash_bytes(b"test", &[b"roundtrip-vrf-output"]),
            proof_hash: hash_bytes(b"test", &[b"roundtrip-vrf-proof"]),
            vrf_public_key: [0; 32],
            vrf_proof: Vec::new(),
            signature: [9; 32],
            observed_at_height: 3,
        };
        let vrf_reveal_payload = encode_validator_vrf_reveal_payload(&vrf_reveal);
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
            P2pMessage::NewBlockCheckChallenge(challenge_id),
            P2pMessage::NewBlockCheckChallengePayload {
                challenge_id,
                block_hash: challenge.block_hash,
                challenger: challenge.challenger,
                payload: encode_block_check_challenge_payload(&challenge),
            },
            P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: observed_challenge_id,
                block_hash: observed_challenge.block_hash,
                challenger: observed_challenge.challenger,
                observed_block_payload,
                challenge_payload: encode_block_check_challenge_payload(&observed_challenge),
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
            P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: beacon_source,
                beacon_round,
                payload: beacon_payload,
            },
            P2pMessage::NewVerifiedDrandBeaconPayload {
                source_id: verified_drand_source,
                beacon_round: verified_drand_round,
                payload: verified_drand_payload,
            },
            P2pMessage::NewVerifiedChainedDrandBeaconPayload {
                source_id: verified_chained_drand_source,
                beacon_round: verified_chained_drand_round,
                payload: verified_chained_drand_payload,
            },
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: vrf_reveal.reveal_id,
                receipt_id: vrf_reveal.receipt_id,
                validator: vrf_reveal.validator,
                payload: vrf_reveal_payload,
            },
            P2pMessage::NewTraceBisectionRoundPayload {
                receipt_id: trace_bisection_round.receipt_id,
                trace_root: trace_bisection_round.trace_root,
                challenger: trace_bisection_round.challenger,
                responder: trace_bisection_round.responder,
                transcript_leaf: trace_bisection_round.transcript_leaf(),
                payload: trace_bisection_round_payload,
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
            P2pMessage::RequestTraceOpening {
                trace_root: trace_opening.trace_root,
                op_index: trace_opening.op_index,
            },
            P2pMessage::TraceOpeningResponse {
                trace_root: trace_opening.trace_root,
                op_index: trace_opening.op_index,
                payload: Some(trace_opening_payload),
            },
            P2pMessage::TraceOpeningResponse {
                trace_root: h,
                op_index: 9,
                payload: None,
            },
            P2pMessage::PeerInfo { address: peer },
        ];

        for message in messages {
            assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);
        }
    }

    #[test]
    fn external_randomness_beacon_payloads_roundtrip_and_reject_malformed_edges() {
        let source_id = "local_drand_fixture_v1";
        let beacon_round = 77;
        let randomness = hash_bytes(b"test", &[b"beacon-randomness"]);
        let proof_hash = hash_bytes(b"test", &[b"beacon-proof"]);
        let payload = encode_external_randomness_beacon_payload(
            source_id,
            beacon_round,
            &randomness,
            &proof_hash,
        );
        assert_eq!(
            decode_external_randomness_beacon_payload(&payload).unwrap(),
            ExternalRandomnessBeaconPayload {
                source_id: source_id.to_owned(),
                beacon_round,
                randomness,
                proof_hash,
            }
        );

        let mut trailing = payload.clone();
        trailing.push(0);
        assert_eq!(
            decode_external_randomness_beacon_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing external randomness beacon payload bytes"
            ))
        );

        let oversized_source = "x".repeat(EXTERNAL_RANDOMNESS_BEACON_SOURCE_ID_MAX_BYTES + 1);
        let oversized_payload = encode_external_randomness_beacon_payload(
            &oversized_source,
            beacon_round,
            &randomness,
            &proof_hash,
        );
        assert!(decode_external_randomness_beacon_payload(&oversized_payload).is_err());

        let mismatched = P2pMessage::NewExternalRandomnessBeaconPayload {
            source_id: source_id.to_owned(),
            beacon_round: beacon_round + 1,
            payload,
        };
        assert!(decode_message(&encode_message(&mismatched)).is_err());
    }

    #[test]
    fn verified_drand_beacon_payloads_roundtrip_and_reject_malformed_edges() {
        let (source_id, beacon_round, public_key, signature, payload) =
            verified_drand_payload_fixture();
        assert_eq!(
            decode_verified_drand_beacon_payload(&payload).unwrap(),
            VerifiedDrandBeaconPayload {
                source_id: source_id.clone(),
                beacon_round,
                public_key: public_key.clone(),
                signature: signature.clone(),
            }
        );
        let message = P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id: source_id.clone(),
            beacon_round,
            payload: payload.clone(),
        };
        assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);

        let mut trailing = payload.clone();
        trailing.push(0);
        assert_eq!(
            decode_verified_drand_beacon_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing verified drand beacon payload bytes"
            ))
        );

        let short_public_key = encode_verified_drand_beacon_payload(
            &source_id,
            beacon_round,
            &public_key[..DRAND_PEDERSEN_BLS_PUBLIC_KEY_BYTES - 1],
            &signature,
        );
        assert_eq!(
            decode_verified_drand_beacon_payload(&short_public_key),
            Err(TvmError::InvalidReceipt(
                "verified drand public key length mismatch"
            ))
        );

        let oversized_signature = encode_verified_drand_beacon_payload(
            &source_id,
            beacon_round,
            &public_key,
            &[0; DRAND_PEDERSEN_BLS_SIGNATURE_BYTES + 1],
        );
        assert!(decode_verified_drand_beacon_payload(&oversized_signature).is_err());

        let mismatched = P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id,
            beacon_round: beacon_round + 1,
            payload,
        };
        assert!(decode_message(&encode_message(&mismatched)).is_err());
    }

    #[test]
    fn verified_chained_drand_beacon_payloads_roundtrip_and_reject_malformed_edges() {
        let (source_id, beacon_round, public_key, signature, previous_signature, payload) =
            verified_chained_drand_payload_fixture();
        assert_eq!(
            decode_verified_chained_drand_beacon_payload(&payload).unwrap(),
            VerifiedChainedDrandBeaconPayload {
                source_id: source_id.clone(),
                beacon_round,
                public_key: public_key.clone(),
                signature: signature.clone(),
                previous_signature: previous_signature.clone(),
            }
        );
        let message = P2pMessage::NewVerifiedChainedDrandBeaconPayload {
            source_id: source_id.clone(),
            beacon_round,
            payload: payload.clone(),
        };
        assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);

        let mut trailing = payload.clone();
        trailing.push(0);
        assert_eq!(
            decode_verified_chained_drand_beacon_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing verified chained drand beacon payload bytes"
            ))
        );

        let missing_previous_signature = encode_verified_chained_drand_beacon_payload(
            &source_id,
            beacon_round,
            &public_key,
            &signature,
            &[],
        );
        assert_eq!(
            decode_verified_chained_drand_beacon_payload(&missing_previous_signature),
            Err(TvmError::InvalidReceipt(
                "verified chained drand previous signature length mismatch"
            ))
        );

        let oversized_previous_signature = encode_verified_chained_drand_beacon_payload(
            &source_id,
            beacon_round,
            &public_key,
            &signature,
            &[0; DRAND_PEDERSEN_BLS_PREVIOUS_SIGNATURE_BYTES + 1],
        );
        assert!(
            decode_verified_chained_drand_beacon_payload(&oversized_previous_signature).is_err()
        );

        let mismatched = P2pMessage::NewVerifiedChainedDrandBeaconPayload {
            source_id,
            beacon_round: beacon_round + 1,
            payload,
        };
        assert!(decode_message(&encode_message(&mismatched)).is_err());
    }

    #[test]
    fn validator_vrf_reveal_payloads_roundtrip_and_reject_malformed_edges() {
        let reveal = ValidatorVrfRevealRecord {
            reveal_id: hash_bytes(b"test", &[b"vrf-reveal-id"]),
            receipt_id: hash_bytes(b"test", &[b"vrf-reveal-receipt"]),
            job_id: hash_bytes(b"test", &[b"vrf-reveal-job"]),
            validator: address(b"vrf-reveal-validator"),
            beacon_round: 77,
            validation_round: 2,
            vrf_output: hash_bytes(b"test", &[b"vrf-output"]),
            proof_hash: hash_bytes(b"test", &[b"vrf-proof"]),
            vrf_public_key: [0; 32],
            vrf_proof: Vec::new(),
            signature: [7; 32],
            observed_at_height: 5,
        };
        let payload = encode_validator_vrf_reveal_payload(&reveal);
        assert_eq!(
            decode_validator_vrf_reveal_payload(&payload).unwrap(),
            ValidatorVrfRevealPayload {
                reveal: reveal.clone()
            }
        );

        let mut trailing = payload.clone();
        trailing.push(0);
        assert_eq!(
            decode_validator_vrf_reveal_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing validator vrf reveal payload bytes"
            ))
        );
        assert!(decode_validator_vrf_reveal_payload(&payload[..payload.len() - 1]).is_err());
        let oversized_proof = ValidatorVrfRevealRecord {
            vrf_proof: vec![0; VALIDATOR_VRF_PROOF_MAX_BYTES + 1],
            ..reveal.clone()
        };
        assert!(
            decode_validator_vrf_reveal_payload(&encode_validator_vrf_reveal_payload(
                &oversized_proof
            ))
            .is_err()
        );

        for message in [
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: hash_bytes(b"test", &[b"wrong-reveal"]),
                receipt_id: reveal.receipt_id,
                validator: reveal.validator,
                payload: payload.clone(),
            },
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: reveal.reveal_id,
                receipt_id: hash_bytes(b"test", &[b"wrong-receipt"]),
                validator: reveal.validator,
                payload: payload.clone(),
            },
            P2pMessage::NewValidatorVrfRevealPayload {
                reveal_id: reveal.reveal_id,
                receipt_id: reveal.receipt_id,
                validator: address(b"wrong-validator"),
                payload: payload.clone(),
            },
        ] {
            assert!(decode_message(&encode_message(&message)).is_err());
        }
    }

    #[test]
    fn tensor_payloads_roundtrip_and_reject_malformed_edges() {
        let tensor = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let payload = encode_tensor_payload(&tensor);
        assert_eq!(decode_tensor_payload(&payload).unwrap(), tensor);

        for tensor in [
            Tensor::from_vec(vec![2], DType::Int8, vec![crate::field::MODULUS - 1, 127]).unwrap(),
            Tensor::from_vec(vec![2], DType::Uint8, vec![0, 255]).unwrap(),
            Tensor::from_vec(vec![2], DType::Bool, vec![0, 1]).unwrap(),
        ] {
            assert_eq!(
                decode_tensor_payload(&encode_tensor_payload(&tensor)).unwrap(),
                tensor
            );
        }

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

        let mut bad_bool = Vec::new();
        write_usize_vec(&mut bad_bool, &[1]);
        bad_bool.push(DType::Bool.tag());
        write_u64(&mut bad_bool, 1);
        write_u64(&mut bad_bool, 2);
        assert!(decode_tensor_payload(&bad_bool).is_err());
    }

    #[test]
    fn trace_opening_payloads_roundtrip_and_reject_malformed_edges() {
        let opening = trace_opening_fixture();
        let payload = encode_trace_opening_payload(&opening);
        assert_eq!(decode_trace_opening_payload(&payload).unwrap(), opening);

        let mut tampered = payload.clone();
        tampered[56] = tampered[56].wrapping_add(1);
        assert_eq!(
            decode_trace_opening_payload(&tampered),
            Err(TvmError::InvalidReceipt("invalid trace opening payload"))
        );

        let mut trailing = payload.clone();
        trailing.push(1);
        assert_eq!(
            decode_trace_opening_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing trace opening payload bytes"
            ))
        );

        assert_eq!(
            decode_message(&encode_message(&P2pMessage::TraceOpeningResponse {
                trace_root: opening.trace_root,
                op_index: opening.op_index.saturating_add(1),
                payload: Some(payload),
            })),
            Err(TvmError::InvalidReceipt(
                "trace opening response payload mismatch"
            ))
        );
    }

    #[test]
    fn trace_bisection_round_payloads_roundtrip_and_reject_malformed_edges() {
        let round = trace_bisection_round_fixture();
        let payload = encode_trace_bisection_round_payload(&round);
        assert_eq!(
            decode_trace_bisection_round_payload(&payload).unwrap(),
            round
        );

        let message = P2pMessage::NewTraceBisectionRoundPayload {
            receipt_id: round.receipt_id,
            trace_root: round.trace_root,
            challenger: round.challenger,
            responder: round.responder,
            transcript_leaf: round.transcript_leaf(),
            payload: payload.clone(),
        };
        assert_eq!(decode_message(&encode_message(&message)).unwrap(), message);

        let wrong_leaf = encode_message(&P2pMessage::NewTraceBisectionRoundPayload {
            receipt_id: round.receipt_id,
            trace_root: round.trace_root,
            challenger: round.challenger,
            responder: round.responder,
            transcript_leaf: hash_bytes(b"test", &[b"wrong-trace-bisection-leaf"]),
            payload: payload.clone(),
        });
        assert_eq!(
            decode_message(&wrong_leaf),
            Err(TvmError::InvalidReceipt(
                "trace bisection round payload announcement mismatch"
            ))
        );

        let wrong_trace = encode_message(&P2pMessage::NewTraceBisectionRoundPayload {
            receipt_id: round.receipt_id,
            trace_root: hash_bytes(b"test", &[b"wrong-trace-bisection-root"]),
            challenger: round.challenger,
            responder: round.responder,
            transcript_leaf: round.transcript_leaf(),
            payload: payload.clone(),
        });
        assert_eq!(
            decode_message(&wrong_trace),
            Err(TvmError::InvalidReceipt(
                "trace bisection round payload announcement mismatch"
            ))
        );

        let mut tampered_signature = payload.clone();
        let last = tampered_signature.len() - 1;
        tampered_signature[last] = tampered_signature[last].wrapping_add(1);
        assert_eq!(
            decode_trace_bisection_round_payload(&tampered_signature),
            Err(TvmError::InvalidReceipt(
                "trace bisection round signature mismatch"
            ))
        );

        let mut trailing = payload.clone();
        trailing.push(1);
        assert_eq!(
            decode_trace_bisection_round_payload(&trailing),
            Err(TvmError::InvalidReceipt(
                "trailing trace bisection round payload bytes"
            ))
        );

        let mut oversized_expected_roots = Vec::new();
        write_hash(&mut oversized_expected_roots, &round.receipt_id);
        write_hash(&mut oversized_expected_roots, &round.trace_root);
        write_hash(&mut oversized_expected_roots, &round.challenger);
        write_hash(&mut oversized_expected_roots, &round.responder);
        write_u64(&mut oversized_expected_roots, round.low_op);
        write_u64(&mut oversized_expected_roots, round.high_op);
        write_u64(&mut oversized_expected_roots, round.midpoint_op);
        write_u64(
            &mut oversized_expected_roots,
            (MAX_TRACE_BISECTION_EXPECTED_ROOTS + 1) as u64,
        );
        assert_eq!(
            decode_trace_bisection_round_payload(&oversized_expected_roots),
            Err(TvmError::InvalidReceipt(
                "trace bisection expected roots too large"
            ))
        );
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

        let selected_receipts = [
            hash_bytes(b"test", &[b"selected-receipt-a"]),
            hash_bytes(b"test", &[b"selected-receipt-b"]),
        ];
        let parent_state = Chain::new(hash_bytes(b"test", &[b"block-parent-state"]))
            .state()
            .clone();
        let selected_payload =
            encode_block_payload_with_selected_receipts(&block, &selected_receipts, &parent_state);
        assert_eq!(decode_block_payload(&selected_payload).unwrap(), block);
        assert_eq!(
            decode_block_payload_with_selected_receipts(&selected_payload).unwrap(),
            (
                block.clone(),
                Some(selected_receipts.to_vec()),
                Some(parent_state)
            )
        );
        let mut malformed_selected_payload = selected_payload;
        malformed_selected_payload.push(0);
        assert!(decode_block_payload(&malformed_selected_payload).is_err());

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
    fn block_check_challenge_payloads_roundtrip_and_reject_malformed_edges() {
        let challenge = wire_test_challenge(b"challenge-payload");
        let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
        let payload = encode_block_check_challenge_payload(&challenge);
        let observed_block = wire_test_block(b"observed-challenge-payload", 11);
        let observed_block_payload = encode_block_payload(&observed_block);
        let observed_challenge =
            wire_test_challenge_for_block(b"observed-challenge-payload", observed_block.hash());
        let observed_challenge_id = block_check_challenge_id(
            &observed_challenge.block_hash,
            &observed_challenge.receipt_id,
        );
        let observed_challenge_payload = encode_block_check_challenge_payload(&observed_challenge);

        assert_eq!(
            decode_block_check_challenge_payload(&payload).unwrap(),
            challenge
        );
        assert_eq!(
            decode_message(&encode_message(
                &P2pMessage::NewBlockCheckChallengePayload {
                    challenge_id,
                    block_hash: challenge.block_hash,
                    challenger: challenge.challenger,
                    payload: payload.clone(),
                }
            ))
            .unwrap(),
            P2pMessage::NewBlockCheckChallengePayload {
                challenge_id,
                block_hash: challenge.block_hash,
                challenger: challenge.challenger,
                payload: payload.clone(),
            }
        );

        let mut wrong_id = encode_message(&P2pMessage::NewBlockCheckChallengePayload {
            challenge_id,
            block_hash: challenge.block_hash,
            challenger: challenge.challenger,
            payload: payload.clone(),
        });
        wrong_id[1] ^= 0x55;
        assert!(decode_message(&wrong_id).is_err());

        let wrong_block = encode_message(&P2pMessage::NewBlockCheckChallengePayload {
            challenge_id,
            block_hash: hash_bytes(b"test", &[b"wrong-challenge-block"]),
            challenger: challenge.challenger,
            payload: payload.clone(),
        });
        assert!(decode_message(&wrong_block).is_err());

        let wrong_challenger = encode_message(&P2pMessage::NewBlockCheckChallengePayload {
            challenge_id,
            block_hash: challenge.block_hash,
            challenger: address(b"wrong-challenge-challenger"),
            payload: payload.clone(),
        });
        assert!(decode_message(&wrong_challenger).is_err());

        assert!(decode_block_check_challenge_payload(&payload[..payload.len() - 1]).is_err());
        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_block_check_challenge_payload(&trailing).is_err());

        let mut oversized = challenge.clone();
        oversized.check_leaf_proof.siblings = vec![hash_bytes(b"test", &[b"oversized-proof"]); 65];
        let oversized_payload = encode_block_check_challenge_payload(&oversized);
        assert!(decode_block_check_challenge_payload(&oversized_payload).is_err());
        assert!(
            decode_message(&encode_message(
                &P2pMessage::NewBlockCheckChallengePayload {
                    challenge_id: block_check_challenge_id(
                        &oversized.block_hash,
                        &oversized.receipt_id,
                    ),
                    block_hash: oversized.block_hash,
                    challenger: oversized.challenger,
                    payload: oversized_payload,
                }
            ))
            .is_err()
        );

        assert_eq!(
            decode_message(&encode_message(
                &P2pMessage::NewObservedBlockCheckChallengePayload {
                    challenge_id: observed_challenge_id,
                    block_hash: observed_challenge.block_hash,
                    challenger: observed_challenge.challenger,
                    observed_block_payload: observed_block_payload.clone(),
                    challenge_payload: observed_challenge_payload.clone(),
                }
            ))
            .unwrap(),
            P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: observed_challenge_id,
                block_hash: observed_challenge.block_hash,
                challenger: observed_challenge.challenger,
                observed_block_payload: observed_block_payload.clone(),
                challenge_payload: observed_challenge_payload.clone(),
            }
        );
        let wrong_observed_block =
            encode_message(&P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: observed_challenge_id,
                block_hash: hash_bytes(b"test", &[b"wrong-observed-block"]),
                challenger: observed_challenge.challenger,
                observed_block_payload: observed_block_payload.clone(),
                challenge_payload: observed_challenge_payload.clone(),
            });
        assert!(decode_message(&wrong_observed_block).is_err());
        let wrong_observed_challenge =
            encode_message(&P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id,
                block_hash: observed_challenge.block_hash,
                challenger: observed_challenge.challenger,
                observed_block_payload,
                challenge_payload: payload,
            });
        assert!(decode_message(&wrong_observed_challenge).is_err());
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
            gossip_topic_for_message(&P2pMessage::NewBlockCheckChallenge(h)),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewBlockCheckChallengePayload {
                challenge_id: h,
                block_hash: h,
                challenger: address(b"mapping-challenge-validator"),
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewBlockCheckChallengePayload {
                challenge_id: h,
                block_hash: h,
                challenger: address(b"mapping-challenge-validator"),
                payload: vec![1, 2, 3],
            }),
            None
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewObservedBlockCheckChallengePayload {
                challenge_id: h,
                block_hash: h,
                challenger: address(b"mapping-observed-challenge-validator"),
                observed_block_payload: vec![1, 2, 3],
                challenge_payload: vec![4, 5, 6],
            }),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            request_response_protocol_for_message(
                &P2pMessage::NewObservedBlockCheckChallengePayload {
                    challenge_id: h,
                    block_hash: h,
                    challenger: address(b"mapping-observed-challenge-validator"),
                    observed_block_payload: vec![1, 2, 3],
                    challenge_payload: vec![4, 5, 6],
                }
            ),
            None
        );
        assert_eq!(
            gossip_topic_for_message(&P2pMessage::NewTraceBisectionRoundPayload {
                receipt_id: h,
                trace_root: h,
                challenger: address(b"mapping-trace-challenger"),
                responder: address(b"mapping-trace-responder"),
                transcript_leaf: h,
                payload: vec![1, 2, 3],
            }),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::NewTraceBisectionRoundPayload {
                receipt_id: h,
                trace_root: h,
                challenger: address(b"mapping-trace-challenger"),
                responder: address(b"mapping-trace-responder"),
                transcript_leaf: h,
                payload: vec![1, 2, 3],
            }),
            None
        );
        let beacon_message = P2pMessage::NewExternalRandomnessBeaconPayload {
            source_id: "local_drand_fixture_v1".to_owned(),
            beacon_round: 3,
            payload: encode_external_randomness_beacon_payload(
                "local_drand_fixture_v1",
                3,
                &hash_bytes(b"test", &[b"mapping-randomness"]),
                &hash_bytes(b"test", &[b"mapping-proof"]),
            ),
        };
        assert_eq!(
            gossip_topic_for_message(&beacon_message),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(request_response_protocol_for_message(&beacon_message), None);
        let (source_id, beacon_round, _public_key, _signature, payload) =
            verified_drand_payload_fixture();
        let verified_drand_message = P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id,
            beacon_round,
            payload,
        };
        assert_eq!(
            gossip_topic_for_message(&verified_drand_message),
            Some(GossipTopic::Blocks)
        );
        assert_eq!(
            request_response_protocol_for_message(&verified_drand_message),
            None
        );
        let reveal_message = P2pMessage::NewValidatorVrfRevealPayload {
            reveal_id: h,
            receipt_id: hash_bytes(b"test", &[b"mapping-receipt"]),
            validator: address(b"mapping-vrf-validator"),
            payload: vec![1, 2, 3],
        };
        assert_eq!(
            gossip_topic_for_message(&reveal_message),
            Some(GossipTopic::Attestations)
        );
        assert_eq!(request_response_protocol_for_message(&reveal_message), None);
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
            gossip_topic_for_message(&P2pMessage::RequestTraceOpening {
                trace_root: h,
                op_index: 0,
            }),
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
            request_response_protocol_for_message(&P2pMessage::RequestTraceOpening {
                trace_root: h,
                op_index: 0,
            }),
            Some(RequestResponseProtocol::TraceOpening)
        );
        assert_eq!(
            request_response_protocol_for_message(&P2pMessage::TraceOpeningResponse {
                trace_root: h,
                op_index: 0,
                payload: None,
            }),
            Some(RequestResponseProtocol::TraceOpening)
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
        assert_eq!(
            request_response_stream_protocol(RequestResponseProtocol::TraceOpening)
                .unwrap()
                .to_string(),
            "/tensorchain/1/trace/opening"
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

    fn wire_test_challenge(label: &[u8]) -> BlockCheckChallenge {
        wire_test_challenge_for_block(label, hash_bytes(b"test", &[label, b"block"]))
    }

    fn trace_opening_fixture() -> IrTraceOpening {
        let op_trace = IrOpTrace {
            op_id: 0,
            output_roots: vec![hash_bytes(b"test", &[b"trace-opening-output"])],
        };
        IrTraceOpening {
            trace_root: op_trace.leaf_hash(),
            op_index: 0,
            op_trace,
            proof: MerkleProof {
                leaf_index: 0,
                siblings: Vec::new(),
            },
        }
    }

    fn trace_bisection_round_fixture() -> TraceBisectionRound {
        let challenger = address(b"wire-trace-bisection-challenger");
        let responder = address(b"wire-trace-bisection-responder");
        let traces = [
            IrOpTrace {
                op_id: 0,
                output_roots: vec![hash_bytes(b"test", &[b"trace-bisection-op-0"])],
            },
            IrOpTrace {
                op_id: 1,
                output_roots: vec![hash_bytes(b"test", &[b"trace-bisection-op-1"])],
            },
            IrOpTrace {
                op_id: 2,
                output_roots: vec![hash_bytes(b"test", &[b"trace-bisection-op-2"])],
            },
        ];
        let leaves = traces.iter().map(IrOpTrace::leaf_hash).collect::<Vec<_>>();
        let trace_root = crate::merkle::merkle_root(&leaves);
        let state = TraceBisectionState::new(TraceBisectionConfig {
            receipt_id: hash_bytes(b"test", &[b"trace-bisection-receipt"]),
            trace_root,
            challenger,
            responder,
            op_count: traces.len() as u64,
            response_deadline_height: 42,
            challenger_bond: 700,
            responder_bond: 900,
        })
        .unwrap();
        let midpoint = state.midpoint();
        let opening = IrTraceOpening {
            trace_root,
            op_index: midpoint,
            op_trace: traces[midpoint as usize].clone(),
            proof: build_proof(&leaves, midpoint).unwrap(),
        };
        TraceBisectionRound::new(
            &state,
            vec![hash_bytes(b"test", &[b"trace-bisection-expected"])],
            opening,
        )
        .unwrap()
    }

    fn wire_test_challenge_for_block(label: &[u8], block_hash: Hash) -> BlockCheckChallenge {
        BlockCheckChallenge::new(BlockCheckChallengeInput {
            challenger: address(b"wire-challenge-validator"),
            block_hash,
            receipt_id: hash_bytes(b"test", &[label, b"receipt"]),
            expected_check_leaf: hash_bytes(b"test", &[label, b"expected"]),
            observed_check_leaf: hash_bytes(b"test", &[label, b"observed"]),
            check_leaf_index: 0,
            check_leaf_proof: MerkleProof {
                leaf_index: 0,
                siblings: vec![hash_bytes(b"test", &[label, b"sibling"])],
            },
            recomputed_checks_root: hash_bytes(b"test", &[label, b"recomputed"]),
        })
    }
}
