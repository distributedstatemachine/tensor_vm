use super::{
    NetworkBlockPayloadApply, NetworkEventIngest, NetworkPayloadApply, NetworkPayloadProcessor,
};
use crate::types::Hash;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct PendingNetworkPayloads {
    jobs: BTreeMap<Hash, Vec<u8>>,
    blocks: BTreeMap<(u64, Hash), Vec<u8>>,
    block_votes: BTreeMap<(Hash, Hash), Vec<u8>>,
    receipts: BTreeMap<Hash, Vec<u8>>,
    attestations: BTreeMap<Hash, Vec<u8>>,
    validator_audit_reports: BTreeMap<(Hash, Hash), Vec<u8>>,
}

impl PendingNetworkPayloads {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
            && self.blocks.is_empty()
            && self.block_votes.is_empty()
            && self.receipts.is_empty()
            && self.attestations.is_empty()
            && self.validator_audit_reports.is_empty()
    }

    pub fn pending_job_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn pending_block_count(&self) -> usize {
        self.blocks.len()
    }
    pub fn queue_job(&mut self, job_id: Hash, payload: Vec<u8>) {
        self.jobs.entry(job_id).or_insert(payload);
    }

    pub fn pending_block_vote_count(&self) -> usize {
        self.block_votes.len()
    }

    pub fn pending_receipt_count(&self) -> usize {
        self.receipts.len()
    }

    pub fn pending_attestation_count(&self) -> usize {
        self.attestations.len()
    }

    pub fn pending_validator_audit_report_count(&self) -> usize {
        self.validator_audit_reports.len()
    }

    pub fn queue_receipt(&mut self, receipt_id: Hash, payload: Vec<u8>) {
        self.receipts.entry(receipt_id).or_insert(payload);
    }

    pub fn queue_block(&mut self, height: u64, block_hash: Hash, payload: Vec<u8>) {
        self.blocks.entry((height, block_hash)).or_insert(payload);
    }

    pub fn queue_block_vote(&mut self, block_hash: Hash, validator: Hash, payload: Vec<u8>) {
        self.block_votes
            .entry((block_hash, validator))
            .or_insert(payload);
    }

    pub fn queue_attestation(&mut self, attestation_id: Hash, payload: Vec<u8>) {
        self.attestations.entry(attestation_id).or_insert(payload);
    }

    pub fn queue_validator_audit_report(
        &mut self,
        audit_id: Hash,
        auditor: Hash,
        payload: Vec<u8>,
    ) {
        self.validator_audit_reports
            .entry((audit_id, auditor))
            .or_insert(payload);
    }

    pub fn retry_with<P: NetworkPayloadProcessor + ?Sized>(
        &mut self,
        processor: &mut P,
    ) -> NetworkEventIngest {
        let mut ingested = NetworkEventIngest::default();
        loop {
            let mut progressed = false;
            for job_id in self.jobs.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .jobs
                    .get(&job_id)
                    .expect("queued job payload must exist")
                    .clone();
                match processor.apply_job(job_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.jobs.remove(&job_id);
                        ingested.job_payloads_applied =
                            ingested.job_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.jobs.remove(&job_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for receipt_id in self.receipts.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .receipts
                    .get(&receipt_id)
                    .expect("queued receipt payload must exist")
                    .clone();
                match processor.apply_receipt(receipt_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.receipts.remove(&receipt_id);
                        ingested.receipt_payloads_applied =
                            ingested.receipt_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.receipts.remove(&receipt_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for attestation_id in self.attestations.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .attestations
                    .get(&attestation_id)
                    .expect("queued attestation payload must exist")
                    .clone();
                match processor.apply_attestation(attestation_id, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.attestations.remove(&attestation_id);
                        ingested.attestation_payloads_applied =
                            ingested.attestation_payloads_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.attestations.remove(&attestation_id);
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (height, block_hash) in self.blocks.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .blocks
                    .get(&(height, block_hash))
                    .expect("queued block payload must exist")
                    .clone();
                match processor.apply_block(height, block_hash, &payload) {
                    NetworkBlockPayloadApply::Applied { appended } => {
                        self.blocks.remove(&(height, block_hash));
                        ingested.block_payloads_applied =
                            ingested.block_payloads_applied.saturating_add(1);
                        ingested.applied_blocks = ingested.applied_blocks.saturating_add(appended);
                        progressed = true;
                    }
                    NetworkBlockPayloadApply::Pending => {}
                    NetworkBlockPayloadApply::Invalid => {
                        self.blocks.remove(&(height, block_hash));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (block_hash, validator) in self.block_votes.keys().copied().collect::<Vec<_>>() {
                let payload = self
                    .block_votes
                    .get(&(block_hash, validator))
                    .expect("queued block vote payload must exist")
                    .clone();
                match processor.apply_block_vote(block_hash, validator, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.block_votes.remove(&(block_hash, validator));
                        ingested.block_votes_applied =
                            ingested.block_votes_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.block_votes.remove(&(block_hash, validator));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (audit_id, auditor) in self
                .validator_audit_reports
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let payload = self
                    .validator_audit_reports
                    .get(&(audit_id, auditor))
                    .expect("queued validator audit report payload must exist")
                    .clone();
                match processor.apply_validator_audit_report(audit_id, auditor, &payload) {
                    NetworkPayloadApply::Applied => {
                        self.validator_audit_reports.remove(&(audit_id, auditor));
                        ingested.validator_audit_reports_applied =
                            ingested.validator_audit_reports_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.validator_audit_reports.remove(&(audit_id, auditor));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            if !progressed {
                break;
            }
        }
        ingested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RetryProcessor {
        block_result: NetworkBlockPayloadApply,
        receipt_result: NetworkPayloadApply,
        attestation_result: NetworkPayloadApply,
        validator_audit_report_result: NetworkPayloadApply,
        block_attempts: usize,
        receipt_attempts: usize,
        attestation_attempts: usize,
        validator_audit_report_attempts: usize,
    }

    impl NetworkPayloadProcessor for RetryProcessor {
        fn apply_job(&mut self, _job_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
            NetworkPayloadApply::Applied
        }

        fn apply_block(
            &mut self,
            _height: u64,
            _block_hash: Hash,
            _payload: &[u8],
        ) -> NetworkBlockPayloadApply {
            self.block_attempts = self.block_attempts.saturating_add(1);
            self.block_result
        }

        fn apply_block_vote(
            &mut self,
            _block_hash: Hash,
            _validator: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            NetworkPayloadApply::Pending
        }

        fn apply_receipt(&mut self, _receipt_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
            self.receipt_attempts = self.receipt_attempts.saturating_add(1);
            self.receipt_result
        }

        fn apply_attestation(
            &mut self,
            _attestation_id: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.attestation_attempts = self.attestation_attempts.saturating_add(1);
            self.attestation_result
        }

        fn apply_validator_audit_report(
            &mut self,
            _audit_id: Hash,
            _auditor: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.validator_audit_report_attempts =
                self.validator_audit_report_attempts.saturating_add(1);
            self.validator_audit_report_result
        }
    }

    impl RetryProcessor {
        fn new(
            receipt_result: NetworkPayloadApply,
            attestation_result: NetworkPayloadApply,
        ) -> Self {
            Self {
                block_result: NetworkBlockPayloadApply::Pending,
                receipt_result,
                attestation_result,
                validator_audit_report_result: NetworkPayloadApply::Pending,
                block_attempts: 0,
                receipt_attempts: 0,
                attestation_attempts: 0,
                validator_audit_report_attempts: 0,
            }
        }
    }

    #[test]
    fn pending_payloads_retry_applies_and_invalidates_until_quiescent() {
        let receipt_id = [1; 32];
        let attestation_id = [2; 32];
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt(receipt_id, vec![10]);
        pending.queue_attestation(attestation_id, vec![20]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Applied, NetworkPayloadApply::Invalid);

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 0);
        assert_eq!(ingested.invalid_events, 1);
        assert!(pending.is_empty());
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
    }

    #[test]
    fn pending_payloads_retry_handles_invalid_receipts_and_applied_attestations() {
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([3; 32], vec![30]);
        pending.queue_attestation([4; 32], vec![40]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Invalid, NetworkPayloadApply::Applied);

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 0);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.invalid_events, 1);
        assert!(pending.is_empty());
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
    }

    #[test]
    fn pending_payloads_retry_keeps_pending_payloads() {
        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([5; 32], vec![50]);
        pending.queue_attestation([6; 32], vec![60]);
        pending.queue_validator_audit_report([7; 32], [8; 32], vec![70]);
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Pending, NetworkPayloadApply::Pending);

        let ingested = pending.retry_with(&mut processor);

        assert!(!ingested.has_activity());
        assert_eq!(pending.pending_receipt_count(), 1);
        assert_eq!(pending.pending_attestation_count(), 1);
        assert_eq!(pending.pending_validator_audit_report_count(), 1);
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
        assert_eq!(processor.validator_audit_report_attempts, 1);
    }

    #[test]
    fn pending_payloads_keep_first_payload_for_duplicate_ids() {
        struct PayloadCapturingProcessor {
            block_payloads: Vec<Vec<u8>>,
            receipt_payloads: Vec<Vec<u8>>,
            attestation_payloads: Vec<Vec<u8>>,
            validator_audit_report_payloads: Vec<Vec<u8>>,
        }

        impl NetworkPayloadProcessor for PayloadCapturingProcessor {
            fn apply_job(&mut self, _job_id: Hash, _payload: &[u8]) -> NetworkPayloadApply {
                NetworkPayloadApply::Applied
            }

            fn apply_block(
                &mut self,
                _height: u64,
                _block_hash: Hash,
                payload: &[u8],
            ) -> NetworkBlockPayloadApply {
                self.block_payloads.push(payload.to_vec());
                NetworkBlockPayloadApply::Applied { appended: 1 }
            }

            fn apply_block_vote(
                &mut self,
                _block_hash: Hash,
                _validator: Hash,
                _payload: &[u8],
            ) -> NetworkPayloadApply {
                NetworkPayloadApply::Applied
            }

            fn apply_receipt(&mut self, _receipt_id: Hash, payload: &[u8]) -> NetworkPayloadApply {
                self.receipt_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }

            fn apply_attestation(
                &mut self,
                _attestation_id: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.attestation_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }

            fn apply_validator_audit_report(
                &mut self,
                _audit_id: Hash,
                _auditor: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.validator_audit_report_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }
        }

        let mut pending = PendingNetworkPayloads::default();
        pending.queue_receipt([7; 32], vec![70]);
        pending.queue_receipt([7; 32], vec![71]);
        pending.queue_attestation([8; 32], vec![80]);
        pending.queue_attestation([8; 32], vec![81]);
        pending.queue_validator_audit_report([9; 32], [10; 32], vec![90]);
        pending.queue_validator_audit_report([9; 32], [10; 32], vec![91]);
        let mut processor = PayloadCapturingProcessor {
            block_payloads: Vec::new(),
            receipt_payloads: Vec::new(),
            attestation_payloads: Vec::new(),
            validator_audit_report_payloads: Vec::new(),
        };

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.validator_audit_reports_applied, 1);
        assert_eq!(processor.receipt_payloads, vec![vec![70]]);
        assert_eq!(processor.attestation_payloads, vec![vec![80]]);
        assert_eq!(processor.validator_audit_report_payloads, vec![vec![90]]);
        assert!(pending.is_empty());
    }
}
