use super::{
    NetworkBlockPayloadApply, NetworkEventIngest, NetworkPayloadApply, NetworkPayloadProcessor,
};
use crate::types::Hash;
use std::collections::BTreeMap;

type BlockCheckChallengeKey = (Hash, Hash, Hash);
type ObservedBlockCheckChallengePayload = (Vec<u8>, Vec<u8>);
type TraceBisectionRoundKey = (Hash, Hash, Hash, Hash, Hash);
type TraceBisectionRefereeKey = (Hash, Hash, Hash, Hash, Hash, u64);
type ValidatorVrfRevealKey = (Hash, Hash, Hash);

#[derive(Debug, Default)]
pub struct PendingNetworkPayloads {
    jobs: BTreeMap<Hash, Vec<u8>>,
    blocks: BTreeMap<(u64, Hash), Vec<u8>>,
    block_votes: BTreeMap<(Hash, Hash), Vec<u8>>,
    block_check_challenges: BTreeMap<BlockCheckChallengeKey, Vec<u8>>,
    observed_block_check_challenges:
        BTreeMap<BlockCheckChallengeKey, ObservedBlockCheckChallengePayload>,
    trace_bisection_rounds: BTreeMap<TraceBisectionRoundKey, Vec<u8>>,
    trace_bisection_referees: BTreeMap<TraceBisectionRefereeKey, Vec<u8>>,
    receipts: BTreeMap<Hash, Vec<u8>>,
    attestations: BTreeMap<Hash, Vec<u8>>,
    validator_audit_reports: BTreeMap<(Hash, Hash), Vec<u8>>,
    validator_vrf_reveals: BTreeMap<ValidatorVrfRevealKey, Vec<u8>>,
}

impl PendingNetworkPayloads {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
            && self.blocks.is_empty()
            && self.block_votes.is_empty()
            && self.block_check_challenges.is_empty()
            && self.observed_block_check_challenges.is_empty()
            && self.trace_bisection_rounds.is_empty()
            && self.trace_bisection_referees.is_empty()
            && self.receipts.is_empty()
            && self.attestations.is_empty()
            && self.validator_audit_reports.is_empty()
            && self.validator_vrf_reveals.is_empty()
    }

    pub fn pending_job_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn pending_job_payloads(&self) -> Vec<(Hash, Vec<u8>)> {
        self.jobs
            .iter()
            .map(|(job_id, payload)| (*job_id, payload.clone()))
            .collect()
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

    pub fn pending_block_check_challenge_count(&self) -> usize {
        self.block_check_challenges
            .len()
            .saturating_add(self.observed_block_check_challenges.len())
    }

    pub fn pending_trace_bisection_round_count(&self) -> usize {
        self.trace_bisection_rounds.len()
    }

    pub fn pending_trace_bisection_referee_count(&self) -> usize {
        self.trace_bisection_referees.len()
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

    pub fn pending_validator_vrf_reveal_count(&self) -> usize {
        self.validator_vrf_reveals.len()
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

    pub fn queue_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        payload: Vec<u8>,
    ) {
        self.block_check_challenges
            .entry((challenge_id, block_hash, challenger))
            .or_insert(payload);
    }

    pub fn queue_observed_block_check_challenge(
        &mut self,
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Hash,
        observed_block_payload: Vec<u8>,
        challenge_payload: Vec<u8>,
    ) {
        self.observed_block_check_challenges
            .entry((challenge_id, block_hash, challenger))
            .or_insert((observed_block_payload, challenge_payload));
    }

    pub fn queue_trace_bisection_round(
        &mut self,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        transcript_leaf: Hash,
        payload: Vec<u8>,
    ) {
        self.trace_bisection_rounds
            .entry((
                receipt_id,
                trace_root,
                challenger,
                responder,
                transcript_leaf,
            ))
            .or_insert(payload);
    }

    pub fn queue_trace_bisection_referee(
        &mut self,
        challenge_id: Hash,
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Hash,
        responder: Hash,
        op_index: u64,
        payload: Vec<u8>,
    ) {
        self.trace_bisection_referees
            .entry((
                challenge_id,
                receipt_id,
                trace_root,
                challenger,
                responder,
                op_index,
            ))
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

    pub fn queue_validator_vrf_reveal(
        &mut self,
        reveal_id: Hash,
        receipt_id: Hash,
        validator: Hash,
        payload: Vec<u8>,
    ) {
        self.validator_vrf_reveals
            .entry((reveal_id, receipt_id, validator))
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
            for (challenge_id, block_hash, challenger) in self
                .block_check_challenges
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let payload = self
                    .block_check_challenges
                    .get(&(challenge_id, block_hash, challenger))
                    .expect("queued block check challenge payload must exist")
                    .clone();
                match processor.apply_block_check_challenge(
                    challenge_id,
                    block_hash,
                    challenger,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        self.block_check_challenges
                            .remove(&(challenge_id, block_hash, challenger));
                        ingested.block_check_challenges_applied =
                            ingested.block_check_challenges_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.block_check_challenges
                            .remove(&(challenge_id, block_hash, challenger));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (challenge_id, block_hash, challenger) in self
                .observed_block_check_challenges
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let (observed_block_payload, challenge_payload) = self
                    .observed_block_check_challenges
                    .get(&(challenge_id, block_hash, challenger))
                    .expect("queued observed block check challenge payload must exist")
                    .clone();
                match processor.apply_observed_block_check_challenge(
                    challenge_id,
                    block_hash,
                    challenger,
                    &observed_block_payload,
                    &challenge_payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        self.observed_block_check_challenges.remove(&(
                            challenge_id,
                            block_hash,
                            challenger,
                        ));
                        ingested.block_check_challenges_applied =
                            ingested.block_check_challenges_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.observed_block_check_challenges.remove(&(
                            challenge_id,
                            block_hash,
                            challenger,
                        ));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (receipt_id, trace_root, challenger, responder, transcript_leaf) in self
                .trace_bisection_rounds
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let payload = self
                    .trace_bisection_rounds
                    .get(&(
                        receipt_id,
                        trace_root,
                        challenger,
                        responder,
                        transcript_leaf,
                    ))
                    .expect("queued trace bisection round payload must exist")
                    .clone();
                match processor.apply_trace_bisection_round(
                    receipt_id,
                    trace_root,
                    challenger,
                    responder,
                    transcript_leaf,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        self.trace_bisection_rounds.remove(&(
                            receipt_id,
                            trace_root,
                            challenger,
                            responder,
                            transcript_leaf,
                        ));
                        ingested.trace_bisection_rounds_applied =
                            ingested.trace_bisection_rounds_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.trace_bisection_rounds.remove(&(
                            receipt_id,
                            trace_root,
                            challenger,
                            responder,
                            transcript_leaf,
                        ));
                        ingested.invalid_events = ingested.invalid_events.saturating_add(1);
                        progressed = true;
                    }
                }
            }
            for (challenge_id, receipt_id, trace_root, challenger, responder, op_index) in self
                .trace_bisection_referees
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let payload = self
                    .trace_bisection_referees
                    .get(&(
                        challenge_id,
                        receipt_id,
                        trace_root,
                        challenger,
                        responder,
                        op_index,
                    ))
                    .expect("queued trace bisection referee payload must exist")
                    .clone();
                match processor.apply_trace_bisection_referee(
                    challenge_id,
                    receipt_id,
                    trace_root,
                    challenger,
                    responder,
                    op_index,
                    &payload,
                ) {
                    NetworkPayloadApply::Applied => {
                        self.trace_bisection_referees.remove(&(
                            challenge_id,
                            receipt_id,
                            trace_root,
                            challenger,
                            responder,
                            op_index,
                        ));
                        ingested.trace_bisection_referees_applied =
                            ingested.trace_bisection_referees_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.trace_bisection_referees.remove(&(
                            challenge_id,
                            receipt_id,
                            trace_root,
                            challenger,
                            responder,
                            op_index,
                        ));
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
            for (reveal_id, receipt_id, validator) in self
                .validator_vrf_reveals
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let payload = self
                    .validator_vrf_reveals
                    .get(&(reveal_id, receipt_id, validator))
                    .expect("queued validator vrf reveal payload must exist")
                    .clone();
                match processor
                    .apply_validator_vrf_reveal(reveal_id, receipt_id, validator, &payload)
                {
                    NetworkPayloadApply::Applied => {
                        self.validator_vrf_reveals
                            .remove(&(reveal_id, receipt_id, validator));
                        ingested.validator_vrf_reveals_applied =
                            ingested.validator_vrf_reveals_applied.saturating_add(1);
                        progressed = true;
                    }
                    NetworkPayloadApply::Pending => {}
                    NetworkPayloadApply::Invalid => {
                        self.validator_vrf_reveals
                            .remove(&(reveal_id, receipt_id, validator));
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
        validator_vrf_reveal_result: NetworkPayloadApply,
        block_check_challenge_result: NetworkPayloadApply,
        observed_block_check_challenge_result: NetworkPayloadApply,
        trace_bisection_round_result: NetworkPayloadApply,
        trace_bisection_referee_result: NetworkPayloadApply,
        block_attempts: usize,
        receipt_attempts: usize,
        attestation_attempts: usize,
        validator_audit_report_attempts: usize,
        validator_vrf_reveal_attempts: usize,
        block_check_challenge_attempts: usize,
        observed_block_check_challenge_attempts: usize,
        trace_bisection_round_attempts: usize,
        trace_bisection_referee_attempts: usize,
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

        fn apply_block_check_challenge(
            &mut self,
            _challenge_id: Hash,
            _block_hash: Hash,
            _challenger: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.block_check_challenge_attempts =
                self.block_check_challenge_attempts.saturating_add(1);
            self.block_check_challenge_result
        }

        fn apply_observed_block_check_challenge(
            &mut self,
            _challenge_id: Hash,
            _block_hash: Hash,
            _challenger: Hash,
            _observed_block_payload: &[u8],
            _challenge_payload: &[u8],
        ) -> NetworkPayloadApply {
            self.observed_block_check_challenge_attempts = self
                .observed_block_check_challenge_attempts
                .saturating_add(1);
            self.observed_block_check_challenge_result
        }

        fn apply_trace_bisection_round(
            &mut self,
            _receipt_id: Hash,
            _trace_root: Hash,
            _challenger: Hash,
            _responder: Hash,
            _transcript_leaf: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.trace_bisection_round_attempts =
                self.trace_bisection_round_attempts.saturating_add(1);
            self.trace_bisection_round_result
        }

        fn apply_trace_bisection_referee(
            &mut self,
            _challenge_id: Hash,
            _receipt_id: Hash,
            _trace_root: Hash,
            _challenger: Hash,
            _responder: Hash,
            _op_index: u64,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.trace_bisection_referee_attempts =
                self.trace_bisection_referee_attempts.saturating_add(1);
            self.trace_bisection_referee_result
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

        fn apply_validator_vrf_reveal(
            &mut self,
            _reveal_id: Hash,
            _receipt_id: Hash,
            _validator: Hash,
            _payload: &[u8],
        ) -> NetworkPayloadApply {
            self.validator_vrf_reveal_attempts =
                self.validator_vrf_reveal_attempts.saturating_add(1);
            self.validator_vrf_reveal_result
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
                validator_vrf_reveal_result: NetworkPayloadApply::Pending,
                block_check_challenge_result: NetworkPayloadApply::Pending,
                observed_block_check_challenge_result: NetworkPayloadApply::Pending,
                trace_bisection_round_result: NetworkPayloadApply::Pending,
                trace_bisection_referee_result: NetworkPayloadApply::Pending,
                block_attempts: 0,
                receipt_attempts: 0,
                attestation_attempts: 0,
                validator_audit_report_attempts: 0,
                validator_vrf_reveal_attempts: 0,
                block_check_challenge_attempts: 0,
                observed_block_check_challenge_attempts: 0,
                trace_bisection_round_attempts: 0,
                trace_bisection_referee_attempts: 0,
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
        pending.queue_validator_vrf_reveal([15; 32], [16; 32], [17; 32], vec![75]);
        pending.queue_block_check_challenge([9; 32], [10; 32], [11; 32], vec![80]);
        pending.queue_observed_block_check_challenge(
            [12; 32],
            [13; 32],
            [14; 32],
            vec![90],
            vec![91],
        );
        pending.queue_trace_bisection_round(
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            [24; 32],
            vec![92],
        );
        pending.queue_trace_bisection_referee(
            [25; 32],
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            0,
            vec![93],
        );
        let mut processor =
            RetryProcessor::new(NetworkPayloadApply::Pending, NetworkPayloadApply::Pending);

        let ingested = pending.retry_with(&mut processor);

        assert!(!ingested.has_activity());
        assert_eq!(pending.pending_receipt_count(), 1);
        assert_eq!(pending.pending_attestation_count(), 1);
        assert_eq!(pending.pending_validator_audit_report_count(), 1);
        assert_eq!(pending.pending_validator_vrf_reveal_count(), 1);
        assert_eq!(pending.pending_block_check_challenge_count(), 2);
        assert_eq!(pending.pending_trace_bisection_round_count(), 1);
        assert_eq!(pending.pending_trace_bisection_referee_count(), 1);
        assert_eq!(processor.receipt_attempts, 1);
        assert_eq!(processor.attestation_attempts, 1);
        assert_eq!(processor.validator_audit_report_attempts, 1);
        assert_eq!(processor.validator_vrf_reveal_attempts, 1);
        assert_eq!(processor.block_check_challenge_attempts, 1);
        assert_eq!(processor.observed_block_check_challenge_attempts, 1);
        assert_eq!(processor.trace_bisection_round_attempts, 1);
        assert_eq!(processor.trace_bisection_referee_attempts, 1);
    }

    #[test]
    fn pending_payloads_keep_first_payload_for_duplicate_ids() {
        struct PayloadCapturingProcessor {
            block_payloads: Vec<Vec<u8>>,
            receipt_payloads: Vec<Vec<u8>>,
            attestation_payloads: Vec<Vec<u8>>,
            validator_audit_report_payloads: Vec<Vec<u8>>,
            validator_vrf_reveal_payloads: Vec<Vec<u8>>,
            block_check_challenge_payloads: Vec<Vec<u8>>,
            observed_block_check_challenge_payloads: Vec<(Vec<u8>, Vec<u8>)>,
            trace_bisection_round_payloads: Vec<Vec<u8>>,
            trace_bisection_referee_payloads: Vec<Vec<u8>>,
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

            fn apply_block_check_challenge(
                &mut self,
                _challenge_id: Hash,
                _block_hash: Hash,
                _challenger: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.block_check_challenge_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }

            fn apply_observed_block_check_challenge(
                &mut self,
                _challenge_id: Hash,
                _block_hash: Hash,
                _challenger: Hash,
                observed_block_payload: &[u8],
                challenge_payload: &[u8],
            ) -> NetworkPayloadApply {
                self.observed_block_check_challenge_payloads
                    .push((observed_block_payload.to_vec(), challenge_payload.to_vec()));
                NetworkPayloadApply::Applied
            }

            fn apply_trace_bisection_round(
                &mut self,
                _receipt_id: Hash,
                _trace_root: Hash,
                _challenger: Hash,
                _responder: Hash,
                _transcript_leaf: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.trace_bisection_round_payloads.push(payload.to_vec());
                NetworkPayloadApply::Applied
            }

            fn apply_trace_bisection_referee(
                &mut self,
                _challenge_id: Hash,
                _receipt_id: Hash,
                _trace_root: Hash,
                _challenger: Hash,
                _responder: Hash,
                _op_index: u64,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.trace_bisection_referee_payloads.push(payload.to_vec());
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

            fn apply_validator_vrf_reveal(
                &mut self,
                _reveal_id: Hash,
                _receipt_id: Hash,
                _validator: Hash,
                payload: &[u8],
            ) -> NetworkPayloadApply {
                self.validator_vrf_reveal_payloads.push(payload.to_vec());
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
        pending.queue_validator_vrf_reveal([17; 32], [18; 32], [19; 32], vec![95]);
        pending.queue_validator_vrf_reveal([17; 32], [18; 32], [19; 32], vec![96]);
        pending.queue_block_check_challenge([11; 32], [12; 32], [13; 32], vec![100]);
        pending.queue_block_check_challenge([11; 32], [12; 32], [13; 32], vec![101]);
        pending.queue_observed_block_check_challenge(
            [14; 32],
            [15; 32],
            [16; 32],
            vec![110],
            vec![111],
        );
        pending.queue_observed_block_check_challenge(
            [14; 32],
            [15; 32],
            [16; 32],
            vec![112],
            vec![113],
        );
        pending.queue_trace_bisection_round(
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            [24; 32],
            vec![120],
        );
        pending.queue_trace_bisection_round(
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            [24; 32],
            vec![121],
        );
        pending.queue_trace_bisection_referee(
            [25; 32],
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            0,
            vec![122],
        );
        pending.queue_trace_bisection_referee(
            [25; 32],
            [20; 32],
            [21; 32],
            [22; 32],
            [23; 32],
            0,
            vec![123],
        );
        let mut processor = PayloadCapturingProcessor {
            block_payloads: Vec::new(),
            receipt_payloads: Vec::new(),
            attestation_payloads: Vec::new(),
            validator_audit_report_payloads: Vec::new(),
            validator_vrf_reveal_payloads: Vec::new(),
            block_check_challenge_payloads: Vec::new(),
            observed_block_check_challenge_payloads: Vec::new(),
            trace_bisection_round_payloads: Vec::new(),
            trace_bisection_referee_payloads: Vec::new(),
        };

        let ingested = pending.retry_with(&mut processor);

        assert_eq!(ingested.receipt_payloads_applied, 1);
        assert_eq!(ingested.attestation_payloads_applied, 1);
        assert_eq!(ingested.validator_audit_reports_applied, 1);
        assert_eq!(ingested.validator_vrf_reveals_applied, 1);
        assert_eq!(ingested.block_check_challenges_applied, 2);
        assert_eq!(ingested.trace_bisection_rounds_applied, 1);
        assert_eq!(ingested.trace_bisection_referees_applied, 1);
        assert_eq!(processor.receipt_payloads, vec![vec![70]]);
        assert_eq!(processor.attestation_payloads, vec![vec![80]]);
        assert_eq!(processor.validator_audit_report_payloads, vec![vec![90]]);
        assert_eq!(processor.validator_vrf_reveal_payloads, vec![vec![95]]);
        assert_eq!(processor.block_check_challenge_payloads, vec![vec![100]]);
        assert_eq!(
            processor.observed_block_check_challenge_payloads,
            vec![(vec![110], vec![111])]
        );
        assert_eq!(processor.trace_bisection_round_payloads, vec![vec![120]]);
        assert_eq!(processor.trace_bisection_referee_payloads, vec![vec![122]]);
        assert!(pending.is_empty());
    }
}
