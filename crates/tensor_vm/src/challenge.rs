use crate::error::Result;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field;
    use crate::jobs::{MatmulJob, TensorOpReceipt};
    use crate::types::{address, hash_bytes};

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
        let (_honest_receipt, a, b, mut c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        c.set2(0, 0, field::add(c.get2(0, 0).unwrap(), 1)).unwrap();
        let receipt = TensorOpReceipt::from_output(&job, miner, 1, 5, &a, &b, &c).unwrap();
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
}
