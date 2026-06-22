use super::payload_application::{
    apply_network_attestation_payload, apply_network_block_check_challenge_payload,
    apply_network_block_payload, apply_network_block_vote_payload, apply_network_job_payload,
    apply_network_observed_block_check_challenge_payload, apply_network_receipt_payload,
    apply_network_trace_bisection_referee_payload, apply_network_trace_bisection_round_payload,
    apply_network_validator_audit_report_payload, apply_network_validator_vrf_reveal_payload,
};
use crate::{chain::Chain, types::Hash};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPayloadApply {
    Applied,
    Pending,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPayloadError {
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkBlockPayloadApply {
    Applied { appended: usize },
    Pending,
    Invalid,
}

pub trait NetworkPayloadProcessor {
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply;

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_observed_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        observed_block_payload: &[u8],
        challenge_payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_trace_bisection_round(
        &mut self,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        transcript_leaf: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_trace_bisection_referee(
        &mut self,
        challenge_id: Hash,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        op_index: u64,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply;

    fn apply_validator_audit_report(
        &mut self,
        audit_id: Hash,
        auditor: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;

    fn apply_validator_vrf_reveal(
        &mut self,
        reveal_id: Hash,
        receipt_id: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply;
}

pub trait NetworkEventContext {
    fn chain(&mut self) -> &mut Chain;

    fn apply_block_payload(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply;
}

pub struct ChainNetworkPayloadProcessor<'a> {
    chain: &'a mut Chain,
}

impl<'a> ChainNetworkPayloadProcessor<'a> {
    pub fn new(chain: &'a mut Chain) -> Self {
        Self { chain }
    }
}

impl NetworkPayloadProcessor for ChainNetworkPayloadProcessor<'_> {
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_job_payload(self.chain, job_id, payload)
    }

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        apply_network_block_payload(self.chain, height, block_hash, payload)
    }

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_vote_payload(self.chain, block_hash, validator, payload)
    }

    fn apply_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_check_challenge_payload(
            self.chain,
            challenge_id,
            block_hash,
            challenger,
            payload,
        )
    }

    fn apply_observed_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        observed_block_payload: &[u8],
        challenge_payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_observed_block_check_challenge_payload(
            self.chain,
            challenge_id,
            block_hash,
            challenger,
            observed_block_payload,
            challenge_payload,
        )
    }

    fn apply_trace_bisection_round(
        &mut self,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        transcript_leaf: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_trace_bisection_round_payload(
            self.chain,
            receipt_id,
            trace_root,
            challenger,
            responder,
            transcript_leaf,
            payload,
        )
    }

    fn apply_trace_bisection_referee(
        &mut self,
        challenge_id: Hash,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        op_index: u64,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_trace_bisection_referee_payload(
            self.chain,
            challenge_id,
            receipt_id,
            trace_root,
            challenger,
            responder,
            op_index,
            payload,
        )
    }

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.chain, receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.chain, attestation_id, payload)
    }

    fn apply_validator_audit_report(
        &mut self,
        audit_id: Hash,
        auditor: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_validator_audit_report_payload(self.chain, audit_id, auditor, payload)
    }

    fn apply_validator_vrf_reveal(
        &mut self,
        reveal_id: Hash,
        receipt_id: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_validator_vrf_reveal_payload(
            self.chain,
            &reveal_id,
            &receipt_id,
            &validator,
            payload,
        )
    }
}

pub(super) struct ContextNetworkPayloadProcessor<'a, C: NetworkEventContext + ?Sized> {
    pub(super) context: &'a mut C,
}

impl<C: NetworkEventContext + ?Sized> NetworkPayloadProcessor
    for ContextNetworkPayloadProcessor<'_, C>
{
    fn apply_job(&mut self, job_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_job_payload(self.context.chain(), job_id, payload)
    }

    fn apply_block(
        &mut self,
        height: u64,
        block_hash: Hash,
        payload: &[u8],
    ) -> NetworkBlockPayloadApply {
        self.context
            .apply_block_payload(height, block_hash, payload)
    }

    fn apply_block_vote(
        &mut self,
        block_hash: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_vote_payload(self.context.chain(), block_hash, validator, payload)
    }

    fn apply_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_block_check_challenge_payload(
            self.context.chain(),
            challenge_id,
            block_hash,
            challenger,
            payload,
        )
    }

    fn apply_observed_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        observed_block_payload: &[u8],
        challenge_payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_observed_block_check_challenge_payload(
            self.context.chain(),
            challenge_id,
            block_hash,
            challenger,
            observed_block_payload,
            challenge_payload,
        )
    }

    fn apply_trace_bisection_round(
        &mut self,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        transcript_leaf: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_trace_bisection_round_payload(
            self.context.chain(),
            receipt_id,
            trace_root,
            challenger,
            responder,
            transcript_leaf,
            payload,
        )
    }

    fn apply_trace_bisection_referee(
        &mut self,
        challenge_id: Hash,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        op_index: u64,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_trace_bisection_referee_payload(
            self.context.chain(),
            challenge_id,
            receipt_id,
            trace_root,
            challenger,
            responder,
            op_index,
            payload,
        )
    }

    fn apply_receipt(&mut self, receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_receipt_payload(self.context.chain(), receipt_id, payload)
    }

    fn apply_attestation(&mut self, attestation_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
        apply_network_attestation_payload(self.context.chain(), attestation_id, payload)
    }

    fn apply_validator_audit_report(
        &mut self,
        audit_id: Hash,
        auditor: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_validator_audit_report_payload(
            self.context.chain(),
            audit_id,
            auditor,
            payload,
        )
    }

    fn apply_validator_vrf_reveal(
        &mut self,
        reveal_id: Hash,
        receipt_id: Hash,
        validator: Hash,
        payload: &[u8],
    ) -> NetworkPayloadApply {
        apply_network_validator_vrf_reveal_payload(
            self.context.chain(),
            &reveal_id,
            &receipt_id,
            &validator,
            payload,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PendingNetworkPayloads, attestation_announcement_hash};
    use super::*;
    use crate::{
        p2p::{encode_attestation_payload, encode_job_payload, encode_receipt_payload},
        scheduler::JobScheduler,
        testnet::{LocalTestnet, TestnetConfig},
        types::hash_bytes,
    };

    fn local_matmul_round(seed_label: &[u8]) -> LocalTestnet {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"tensor-vm-node-payload-test", &[seed_label]),
        );
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet
    }

    #[test]
    fn chain_payload_processor_retries_against_chain_state() {
        let testnet = local_matmul_round(b"processor");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let attestation = testnet
            .chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);

        let mut chain = testnet.chain.clone();
        chain.remove_job_for_testing(&job_id);
        chain.remove_receipt_for_testing(&receipt_id);
        chain.remove_attestations_for_testing(&receipt_id);
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt(receipt_id, encode_receipt_payload(&receipt));
        pending.queue_attestation(attestation_id, encode_attestation_payload(&attestation));

        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &encode_job_payload(&job)),
            NetworkPayloadApply::Applied
        );
        let mut processor = ChainNetworkPayloadProcessor::new(&mut chain);
        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert!(pending.is_empty());
        assert_eq!(chain.state().receipts().get(&receipt_id), Some(&receipt));
        assert_eq!(
            chain
                .state()
                .attestations()
                .get(&receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
    }
}
