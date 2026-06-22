use crate::error::{Result, TvmError};
use crate::ir::IrTraceOpening;
use crate::jobs::{MatmulJob, TensorOpReceipt};
use crate::merkle::MerkleProof;
use crate::tensor::Tensor;
use crate::types::{Address, Hash, Signature, hash_bytes, sign, verify_signature};
use crate::verify::{
    FreivaldsParams, TensorOpVerificationReport, VerificationResult, verify_tensor_op,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceStep {
    pub op_index: u64,
    pub op_name: String,
    pub input_roots: Vec<Hash>,
    pub output_root: Hash,
}

impl TraceStep {
    pub fn hash(&self) -> Hash {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&self.op_index.to_le_bytes());
        encoded.extend_from_slice(&(self.op_name.len() as u64).to_le_bytes());
        encoded.extend_from_slice(self.op_name.as_bytes());
        encoded.extend_from_slice(&(self.input_roots.len() as u64).to_le_bytes());
        for root in &self.input_roots {
            encoded.extend_from_slice(root);
        }
        encoded.extend_from_slice(&self.output_root);
        hash_bytes(b"tensor-vm-trace-step-v1", &[&encoded])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FraudChallenge {
    pub challenger: Address,
    pub receipt_id: Hash,
    pub disputed_step: u64,
    pub reason: String,
    pub evidence_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChallengeOutcome {
    ProvenInvalid {
        dishonest_party: Address,
        slash_amount: u64,
        reason: String,
    },
    BlockCheckProvenInvalid {
        block_hash: Hash,
        receipt_id: Hash,
        proposer: Address,
        challenger: Address,
        proposer_reward_clawback: u64,
        challenger_reward: u64,
        penalty_until_height: u64,
        reason: String,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBisectionState {
    pub receipt_id: Hash,
    pub trace_root: Hash,
    pub challenger: Address,
    pub responder: Address,
    pub low_op: u64,
    pub high_op: u64,
    pub response_deadline_height: u64,
    pub challenger_bond: u64,
    pub responder_bond: u64,
    pub transcript_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBisectionConfig {
    pub receipt_id: Hash,
    pub trace_root: Hash,
    pub challenger: Address,
    pub responder: Address,
    pub op_count: u64,
    pub response_deadline_height: u64,
    pub challenger_bond: u64,
    pub responder_bond: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBisectionRound {
    pub receipt_id: Hash,
    pub trace_root: Hash,
    pub challenger: Address,
    pub responder: Address,
    pub low_op: u64,
    pub high_op: u64,
    pub midpoint_op: u64,
    pub expected_output_roots: Vec<Hash>,
    pub opening: IrTraceOpening,
    pub response_deadline_height: u64,
    pub challenger_bond: u64,
    pub responder_bond: u64,
    pub responder_signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceBisectionStep {
    Narrowed {
        next_state: TraceBisectionState,
        matched_midpoint: bool,
    },
    Isolated {
        op_index: u64,
    },
    TimedOut {
        forfeiting_party: Address,
        challenger_bond: u64,
        responder_bond: u64,
    },
}

impl TraceBisectionState {
    pub fn new(config: TraceBisectionConfig) -> Result<Self> {
        if config.op_count == 0 {
            return Err(TvmError::InvalidReceipt("empty trace dispute"));
        }
        let transcript_root = hash_bytes(
            b"tensor-vm-trace-bisection-session-v1",
            &[
                &config.receipt_id,
                &config.trace_root,
                &config.challenger,
                &config.responder,
                &config.op_count.to_le_bytes(),
                &config.response_deadline_height.to_le_bytes(),
                &config.challenger_bond.to_le_bytes(),
                &config.responder_bond.to_le_bytes(),
            ],
        );
        Ok(Self {
            receipt_id: config.receipt_id,
            trace_root: config.trace_root,
            challenger: config.challenger,
            responder: config.responder,
            low_op: 0,
            high_op: config.op_count - 1,
            response_deadline_height: config.response_deadline_height,
            challenger_bond: config.challenger_bond,
            responder_bond: config.responder_bond,
            transcript_root,
        })
    }

    pub fn is_isolated(&self) -> bool {
        self.low_op == self.high_op
    }

    pub fn midpoint(&self) -> u64 {
        FraudChallenge::midpoint(self.low_op, self.high_op)
    }

    pub fn timed_out(&self, current_height: u64) -> bool {
        current_height > self.response_deadline_height
    }

    pub fn timeout_step(&self, current_height: u64) -> Option<TraceBisectionStep> {
        self.timed_out(current_height)
            .then_some(TraceBisectionStep::TimedOut {
                forfeiting_party: self.responder,
                challenger_bond: self.challenger_bond,
                responder_bond: self.responder_bond,
            })
    }

    pub fn apply_round(&self, round: &TraceBisectionRound) -> Result<TraceBisectionStep> {
        if self.is_isolated() {
            return Ok(TraceBisectionStep::Isolated {
                op_index: self.low_op,
            });
        }
        round.verify_for_state(self)?;
        let matched_midpoint = round.expected_output_roots == round.opening.op_trace.output_roots;
        let (low_op, high_op) = if matched_midpoint {
            (round.midpoint_op + 1, self.high_op)
        } else {
            (self.low_op, round.midpoint_op)
        };
        let next_state = TraceBisectionState {
            receipt_id: self.receipt_id,
            trace_root: self.trace_root,
            challenger: self.challenger,
            responder: self.responder,
            low_op,
            high_op,
            response_deadline_height: self.response_deadline_height,
            challenger_bond: self.challenger_bond,
            responder_bond: self.responder_bond,
            transcript_root: next_trace_bisection_root(
                &self.transcript_root,
                &round.transcript_leaf(),
                low_op,
                high_op,
            ),
        };
        Ok(if next_state.is_isolated() {
            TraceBisectionStep::Isolated { op_index: low_op }
        } else {
            TraceBisectionStep::Narrowed {
                next_state,
                matched_midpoint,
            }
        })
    }
}

impl TraceBisectionRound {
    pub fn new(
        state: &TraceBisectionState,
        expected_output_roots: Vec<Hash>,
        opening: IrTraceOpening,
    ) -> Result<Self> {
        let unsigned = Self {
            receipt_id: state.receipt_id,
            trace_root: state.trace_root,
            challenger: state.challenger,
            responder: state.responder,
            low_op: state.low_op,
            high_op: state.high_op,
            midpoint_op: state.midpoint(),
            expected_output_roots,
            opening,
            response_deadline_height: state.response_deadline_height,
            challenger_bond: state.challenger_bond,
            responder_bond: state.responder_bond,
            responder_signature: [0; 32],
        };
        unsigned.verify_unsigned_for_state(state)?;
        let message = unsigned.message_hash();
        Ok(Self {
            responder_signature: sign(&state.responder, &message),
            ..unsigned
        })
    }

    pub fn message_hash(&self) -> Hash {
        trace_bisection_round_hash(self, false)
    }

    pub fn transcript_leaf(&self) -> Hash {
        trace_bisection_round_hash(self, true)
    }

    pub fn verify_for_state(&self, state: &TraceBisectionState) -> Result<()> {
        self.verify_unsigned_for_state(state)?;
        if !verify_signature(
            &self.responder,
            &self.message_hash(),
            &self.responder_signature,
        ) {
            return Err(TvmError::InvalidReceipt(
                "trace bisection round signature mismatch",
            ));
        }
        Ok(())
    }

    fn verify_unsigned_for_state(&self, state: &TraceBisectionState) -> Result<()> {
        if self.receipt_id != state.receipt_id
            || self.trace_root != state.trace_root
            || self.challenger != state.challenger
            || self.responder != state.responder
            || self.low_op != state.low_op
            || self.high_op != state.high_op
            || self.response_deadline_height != state.response_deadline_height
            || self.challenger_bond != state.challenger_bond
            || self.responder_bond != state.responder_bond
        {
            return Err(TvmError::InvalidReceipt(
                "trace bisection round state mismatch",
            ));
        }
        if state.is_isolated() || self.midpoint_op != state.midpoint() {
            return Err(TvmError::InvalidReceipt(
                "trace bisection midpoint mismatch",
            ));
        }
        if self.opening.trace_root != state.trace_root
            || self.opening.op_index != self.midpoint_op
            || !self.opening.verify()
        {
            return Err(TvmError::InvalidReceipt("invalid trace bisection opening"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockCheckChallenge {
    pub challenger: Address,
    pub block_hash: Hash,
    pub receipt_id: Hash,
    pub expected_check_leaf: Hash,
    pub observed_check_leaf: Hash,
    pub check_leaf_index: u64,
    pub check_leaf_proof: MerkleProof,
    pub recomputed_checks_root: Hash,
    pub challenger_signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockCheckChallengeInput {
    pub challenger: Address,
    pub block_hash: Hash,
    pub receipt_id: Hash,
    pub expected_check_leaf: Hash,
    pub observed_check_leaf: Hash,
    pub check_leaf_index: u64,
    pub check_leaf_proof: MerkleProof,
    pub recomputed_checks_root: Hash,
}

impl BlockCheckChallenge {
    pub fn new(input: BlockCheckChallengeInput) -> Self {
        let message = Self::message_hash(
            &input.block_hash,
            &input.receipt_id,
            &input.expected_check_leaf,
            &input.observed_check_leaf,
            input.check_leaf_index,
            &input.recomputed_checks_root,
        );
        Self {
            challenger: input.challenger,
            block_hash: input.block_hash,
            receipt_id: input.receipt_id,
            expected_check_leaf: input.expected_check_leaf,
            observed_check_leaf: input.observed_check_leaf,
            check_leaf_index: input.check_leaf_index,
            check_leaf_proof: input.check_leaf_proof,
            recomputed_checks_root: input.recomputed_checks_root,
            challenger_signature: sign(&input.challenger, &message),
        }
    }

    pub fn verify_signature(&self) -> bool {
        verify_signature(
            &self.challenger,
            &Self::message_hash(
                &self.block_hash,
                &self.receipt_id,
                &self.expected_check_leaf,
                &self.observed_check_leaf,
                self.check_leaf_index,
                &self.recomputed_checks_root,
            ),
            &self.challenger_signature,
        )
    }

    fn message_hash(
        block_hash: &Hash,
        receipt_id: &Hash,
        expected_check_leaf: &Hash,
        observed_check_leaf: &Hash,
        check_leaf_index: u64,
        recomputed_checks_root: &Hash,
    ) -> Hash {
        hash_bytes(
            b"tensor-vm-block-check-challenge-v1",
            &[
                block_hash,
                receipt_id,
                expected_check_leaf,
                observed_check_leaf,
                &check_leaf_index.to_le_bytes(),
                recomputed_checks_root,
            ],
        )
    }
}

pub fn block_check_challenge_id(block_hash: &Hash, receipt_id: &Hash) -> Hash {
    hash_bytes(
        b"tensor-vm-block-check-challenge-id-v1",
        &[block_hash, receipt_id],
    )
}

#[derive(Clone, Debug)]
pub struct TensorOpChallengeInput<'a> {
    pub challenger: Address,
    pub job: &'a MatmulJob,
    pub receipt: &'a TensorOpReceipt,
    pub a: &'a Tensor,
    pub b: &'a Tensor,
    pub c: &'a Tensor,
    pub validation_seed: &'a Hash,
    pub params: &'a FreivaldsParams,
}

impl FraudChallenge {
    pub fn midpoint(low: u64, high: u64) -> u64 {
        low + (high - low) / 2
    }

    pub fn tensor_op(input: TensorOpChallengeInput<'_>) -> Result<Self> {
        let report = verify_tensor_op(
            input.job,
            input.receipt,
            input.a,
            input.b,
            input.c,
            input.validation_seed,
            input.params,
        )?;
        let reason = tensor_op_challenge_reason(&report).to_owned();
        let evidence_root = hash_bytes(
            b"tensor-vm-fraud-evidence-v1",
            &[
                &input.receipt.receipt_id,
                &report.checks_root,
                input.validation_seed,
            ],
        );
        Ok(Self {
            challenger: input.challenger,
            receipt_id: input.receipt.receipt_id,
            disputed_step: 0,
            reason,
            evidence_root,
        })
    }

    pub fn resolve_against_miner(
        &self,
        miner: Address,
        verification_result: VerificationResult,
        slash_amount: u64,
    ) -> ChallengeOutcome {
        if verification_result == VerificationResult::Valid {
            ChallengeOutcome::Rejected {
                reason: "receipt is valid".to_owned(),
            }
        } else {
            ChallengeOutcome::ProvenInvalid {
                dishonest_party: miner,
                slash_amount,
                reason: self.reason.clone(),
            }
        }
    }
}

fn tensor_op_challenge_reason(report: &TensorOpVerificationReport) -> &'static str {
    if report.result == VerificationResult::Valid {
        "receipt verified"
    } else if !report.full_freivalds_passed {
        "full Freivalds check failed"
    } else if !report.data_availability_passed {
        "data unavailable"
    } else {
        "receipt invalid"
    }
}

fn trace_bisection_round_hash(round: &TraceBisectionRound, include_signature: bool) -> Hash {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&round.receipt_id);
    encoded.extend_from_slice(&round.trace_root);
    encoded.extend_from_slice(&round.challenger);
    encoded.extend_from_slice(&round.responder);
    encoded.extend_from_slice(&round.low_op.to_le_bytes());
    encoded.extend_from_slice(&round.high_op.to_le_bytes());
    encoded.extend_from_slice(&round.midpoint_op.to_le_bytes());
    encoded.extend_from_slice(&(round.expected_output_roots.len() as u64).to_le_bytes());
    for root in &round.expected_output_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(&encode_trace_opening_for_hash(&round.opening));
    encoded.extend_from_slice(&round.response_deadline_height.to_le_bytes());
    encoded.extend_from_slice(&round.challenger_bond.to_le_bytes());
    encoded.extend_from_slice(&round.responder_bond.to_le_bytes());
    if include_signature {
        encoded.extend_from_slice(&round.responder_signature);
    }
    hash_bytes(b"tensor-vm-trace-bisection-round-v1", &[&encoded])
}

fn encode_trace_opening_for_hash(opening: &IrTraceOpening) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&opening.trace_root);
    encoded.extend_from_slice(&opening.op_index.to_le_bytes());
    encoded.extend_from_slice(&opening.op_trace.op_id.to_le_bytes());
    encoded.extend_from_slice(&(opening.op_trace.output_roots.len() as u64).to_le_bytes());
    for root in &opening.op_trace.output_roots {
        encoded.extend_from_slice(root);
    }
    encoded.extend_from_slice(&opening.proof.leaf_index.to_le_bytes());
    encoded.extend_from_slice(&(opening.proof.siblings.len() as u64).to_le_bytes());
    for sibling in &opening.proof.siblings {
        encoded.extend_from_slice(sibling);
    }
    encoded
}

fn next_trace_bisection_root(
    previous_root: &Hash,
    round_leaf: &Hash,
    low_op: u64,
    high_op: u64,
) -> Hash {
    hash_bytes(
        b"tensor-vm-trace-bisection-transcript-v1",
        &[
            previous_root,
            round_leaf,
            &low_op.to_le_bytes(),
            &high_op.to_le_bytes(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field;
    use crate::ir::{
        GraphOutput, IrExecution, IrExecutionInputs, IrLiteral, IrRef, IrValue, OpNode,
        TensorGraph, TensorSpec,
    };
    use crate::jobs::{GraphJob, GraphReceipt, MatmulJob, TensorOpReceipt};
    use crate::tensor::DType;
    use crate::types::{address, hash_bytes};
    use std::collections::BTreeMap;

    #[test]
    fn trace_step_hash_changes_with_output() {
        let root = hash_bytes(b"test", &[b"root"]);
        let mut step = TraceStep {
            op_index: 1,
            op_name: "matmul".to_owned(),
            input_roots: vec![root],
            output_root: root,
        };
        let before = step.hash();
        step.output_root = hash_bytes(b"test", &[b"other"]);
        assert_ne!(before, step.hash());
        assert_eq!(FraudChallenge::midpoint(10, 20), 15);
    }

    #[test]
    fn fraud_challenge_proves_invalid_tensorop_and_resolves_slash() {
        let beacon = hash_bytes(b"test", &[b"beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let miner = address(b"miner");
        let challenger = address(b"challenger");
        let (mut receipt, a, b, mut c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        c.set2(0, 0, field::add(c.get2(0, 0).unwrap(), 1)).unwrap();
        receipt.output_roots = vec![c.commitment_root()];
        receipt.trace_root = hash_bytes(b"test", &[b"fraud-challenge-invalid-trace"]);
        receipt.receipt_id = receipt.recompute_receipt_id();
        receipt.signature = sign(&receipt.miner, &receipt.receipt_id);
        let seed = hash_bytes(b"test", &[b"validation"]);
        let challenge = FraudChallenge::tensor_op(TensorOpChallengeInput {
            challenger,
            job: &job,
            receipt: &receipt,
            a: &a,
            b: &b,
            c: &c,
            validation_seed: &seed,
            params: &FreivaldsParams::default(),
        })
        .unwrap();
        assert_eq!(challenge.reason, "full Freivalds check failed");
        assert_eq!(
            challenge.resolve_against_miner(miner, VerificationResult::Invalid, 25),
            ChallengeOutcome::ProvenInvalid {
                dishonest_party: miner,
                slash_amount: 25,
                reason: "full Freivalds check failed".to_owned(),
            }
        );
    }

    #[test]
    fn fraud_challenge_rejects_valid_tensorop_receipt() {
        let beacon = hash_bytes(b"test", &[b"valid-challenge-beacon"]);
        let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
        let miner = address(b"valid-challenge-miner");
        let challenger = address(b"valid-challenge-challenger");
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let seed = hash_bytes(b"test", &[b"valid-challenge-validation"]);

        let challenge = FraudChallenge::tensor_op(TensorOpChallengeInput {
            challenger,
            job: &job,
            receipt: &receipt,
            a: &a,
            b: &b,
            c: &c,
            validation_seed: &seed,
            params: &FreivaldsParams::default(),
        })
        .unwrap();

        assert_eq!(challenge.reason, "receipt verified");
        assert_eq!(
            challenge.resolve_against_miner(miner, VerificationResult::Valid, 25),
            ChallengeOutcome::Rejected {
                reason: "receipt is valid".to_owned(),
            }
        );
    }

    #[test]
    fn tensor_op_challenge_reason_covers_availability_and_generic_invalid_cases() {
        let checks_root = hash_bytes(b"test", &[b"challenge-reason-checks"]);
        assert_eq!(
            tensor_op_challenge_reason(&TensorOpVerificationReport {
                result: VerificationResult::Invalid,
                full_freivalds_passed: true,
                sampled_rows_checked: 1,
                data_availability_passed: false,
                conformance_suite_hash: crate::conformance::conformance_suite_hash(),
                checks_root,
            }),
            "data unavailable"
        );
        assert_eq!(
            tensor_op_challenge_reason(&TensorOpVerificationReport {
                result: VerificationResult::Invalid,
                full_freivalds_passed: true,
                sampled_rows_checked: 1,
                data_availability_passed: true,
                conformance_suite_hash: crate::conformance::conformance_suite_hash(),
                checks_root,
            }),
            "receipt invalid"
        );
    }

    #[test]
    fn trace_bisection_rounds_narrow_to_disputed_op() {
        let (state, execution, receipt) = trace_bisection_fixture();
        assert_eq!(state.low_op, 0);
        assert_eq!(state.high_op, 3);
        assert_eq!(state.midpoint(), 1);

        let first_opening = receipt.trace_opening(&execution, state.midpoint()).unwrap();
        let first_round = TraceBisectionRound::new(
            &state,
            first_opening.op_trace.output_roots.clone(),
            first_opening,
        )
        .unwrap();
        let TraceBisectionStep::Narrowed {
            next_state,
            matched_midpoint,
        } = state.apply_round(&first_round).unwrap()
        else {
            panic!("first round should narrow the dispute");
        };
        assert!(matched_midpoint);
        assert_eq!((next_state.low_op, next_state.high_op), (2, 3));
        assert_ne!(next_state.transcript_root, state.transcript_root);
        assert_eq!(next_state.midpoint(), 2);

        let second_opening = receipt
            .trace_opening(&execution, next_state.midpoint())
            .unwrap();
        let second_round = TraceBisectionRound::new(
            &next_state,
            vec![hash_bytes(b"test", &[b"wrong-midpoint-output"])],
            second_opening,
        )
        .unwrap();
        assert_eq!(
            next_state.apply_round(&second_round).unwrap(),
            TraceBisectionStep::Isolated { op_index: 2 }
        );
    }

    #[test]
    fn trace_bisection_rejects_tampered_signature_and_opening() {
        let (state, execution, receipt) = trace_bisection_fixture();
        let opening = receipt.trace_opening(&execution, state.midpoint()).unwrap();
        let mut round =
            TraceBisectionRound::new(&state, opening.op_trace.output_roots.clone(), opening)
                .unwrap();

        round.responder_signature[0] ^= 1;
        assert_eq!(
            round.verify_for_state(&state),
            Err(TvmError::InvalidReceipt(
                "trace bisection round signature mismatch"
            ))
        );

        let opening = receipt.trace_opening(&execution, state.midpoint()).unwrap();
        let mut round =
            TraceBisectionRound::new(&state, opening.op_trace.output_roots.clone(), opening)
                .unwrap();
        round.opening.op_trace.output_roots[0] = hash_bytes(b"test", &[b"tampered-opening-root"]);
        assert_eq!(
            round.verify_for_state(&state),
            Err(TvmError::InvalidReceipt("invalid trace bisection opening"))
        );

        let opening = receipt.trace_opening(&execution, state.midpoint()).unwrap();
        let mut round =
            TraceBisectionRound::new(&state, opening.op_trace.output_roots.clone(), opening)
                .unwrap();
        round.midpoint_op += 1;
        assert_eq!(
            round.verify_for_state(&state),
            Err(TvmError::InvalidReceipt(
                "trace bisection midpoint mismatch"
            ))
        );
    }

    #[test]
    fn trace_bisection_reports_timeout_and_bond_envelope() {
        let (state, _execution, _receipt) = trace_bisection_fixture();
        assert!(!state.timed_out(20));
        assert_eq!(state.timeout_step(20), None);
        assert_eq!(
            state.timeout_step(21),
            Some(TraceBisectionStep::TimedOut {
                forfeiting_party: state.responder,
                challenger_bond: 7,
                responder_bond: 11,
            })
        );
    }

    #[test]
    fn trace_bisection_rejects_empty_trace_session() {
        assert_eq!(
            TraceBisectionState::new(TraceBisectionConfig {
                receipt_id: hash_bytes(b"test", &[b"empty-receipt"]),
                trace_root: hash_bytes(b"test", &[b"empty-trace"]),
                challenger: address(b"empty-challenger"),
                responder: address(b"empty-responder"),
                op_count: 0,
                response_deadline_height: 20,
                challenger_bond: 7,
                responder_bond: 11,
            }),
            Err(TvmError::InvalidReceipt("empty trace dispute"))
        );
    }

    fn trace_bisection_fixture() -> (TraceBisectionState, IrExecution, GraphReceipt) {
        let graph = TensorGraph {
            ir_version: 1,
            inputs: vec![TensorSpec {
                name: "x".to_owned(),
                shape: vec![4],
                dtype: DType::FieldElement,
                scale: 0,
            }],
            params: Vec::new(),
            ops: vec![
                OpNode {
                    id: 0,
                    op: "relu".to_owned(),
                    args: vec![IrRef::Input {
                        name: "x".to_owned(),
                    }],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec {
                        name: "nonnegative".to_owned(),
                        shape: vec![4],
                        dtype: DType::FieldElement,
                        scale: 0,
                    }],
                },
                OpNode {
                    id: 1,
                    op: "reshape".to_owned(),
                    args: vec![IrRef::Op { id: 0, idx: 0 }],
                    kwargs: BTreeMap::from([(
                        "shape".to_owned(),
                        IrValue::Literal(IrLiteral::List(vec![
                            IrLiteral::Uint(2),
                            IrLiteral::Uint(2),
                        ])),
                    )]),
                    out: vec![TensorSpec {
                        name: "matrix".to_owned(),
                        shape: vec![2, 2],
                        dtype: DType::FieldElement,
                        scale: 0,
                    }],
                },
                OpNode {
                    id: 2,
                    op: "add".to_owned(),
                    args: vec![IrRef::Op { id: 1, idx: 0 }, IrRef::Op { id: 1, idx: 0 }],
                    kwargs: BTreeMap::new(),
                    out: vec![TensorSpec {
                        name: "doubled".to_owned(),
                        shape: vec![2, 2],
                        dtype: DType::FieldElement,
                        scale: 0,
                    }],
                },
                OpNode {
                    id: 3,
                    op: "sum".to_owned(),
                    args: vec![IrRef::Op { id: 2, idx: 0 }],
                    kwargs: BTreeMap::from([
                        ("dim".to_owned(), IrValue::Literal(IrLiteral::Uint(1))),
                        (
                            "keepdim".to_owned(),
                            IrValue::Literal(IrLiteral::Bool(false)),
                        ),
                    ]),
                    out: vec![TensorSpec {
                        name: "rows".to_owned(),
                        shape: vec![2],
                        dtype: DType::FieldElement,
                        scale: 0,
                    }],
                },
            ],
            outputs: vec![GraphOutput {
                name: "rows".to_owned(),
                value: IrRef::Op { id: 3, idx: 0 },
            }],
        };
        let input = Tensor::from_vec(vec![4], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
        let inputs = BTreeMap::from([("x".to_owned(), input.clone())]);
        let input_roots = BTreeMap::from([("x".to_owned(), input.commitment_root())]);
        let graph_id = graph.validate_for_consensus().unwrap();
        let job = GraphJob::new(0, graph_id, input_roots, BTreeMap::new(), 20, 1, 4);
        let execution = graph
            .execute_exact(&IrExecutionInputs {
                tensors: inputs.clone(),
                field_params: BTreeMap::new(),
            })
            .unwrap();
        let (receipt, _outputs) = GraphReceipt::from_execution(
            &job,
            &graph,
            address(b"trace-bisection-responder"),
            &inputs,
            3,
            4,
        )
        .unwrap();
        assert_eq!(receipt.trace_root, execution.trace_root);
        assert_eq!(execution.op_traces.len(), 4);
        let state = TraceBisectionState::new(TraceBisectionConfig {
            receipt_id: receipt.receipt_id,
            trace_root: receipt.trace_root,
            challenger: address(b"trace-bisection-challenger"),
            responder: receipt.miner,
            op_count: execution.op_traces.len() as u64,
            response_deadline_height: 20,
            challenger_bond: 7,
            responder_bond: 11,
        })
        .unwrap();
        (state, execution, receipt)
    }
}
