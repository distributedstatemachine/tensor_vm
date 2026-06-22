use super::public_evidence_crypto::{
    aggregate_public_evidence_record_roots, public_evidence_record_message,
    public_evidence_supporting_artifact_uri, public_network_runtime_observations_for_run,
    public_run_window_message, sign_public_evidence_record, sign_public_run_window,
};
use super::public_operators::{MatchedPublicOperators, public_operator_attestation_key};
use super::{
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceRecordKind,
    PublicEvidenceRecordSummaries, PublicEvidenceSupportingArtifact, PublicNodeRole,
    PublicOperatorIdentityAttestation, PublicTestnetCriteria, PublicTestnetEvidenceBundle,
    PublicTestnetEvidenceBundleReport, PublicTestnetRunEvidence,
    public_testnet_criteria_are_full_spec,
};
use crate::hash::hex;
use crate::types::{Hash, Signature, address, verify_signature};
use std::collections::BTreeSet;

impl PublicTestnetEvidenceBundle {
    pub fn new(
        run: PublicTestnetRunEvidence,
        publication: PublicEvidencePublication,
        record_summaries: PublicEvidenceRecordSummaries,
    ) -> Self {
        let signer = publication.manifest_signer;
        let bundle_id = publication.bundle_id;
        let public_uri = publication.public_uri.clone();
        let auditor_records = (0..publication.independent_auditor_count)
            .map(|index| {
                let auditor_label = format!("public-evidence-auditor-{index}");
                PublicEvidenceAuditorRecord::new(
                    &bundle_id,
                    &public_uri,
                    address(auditor_label.as_bytes()),
                    format!(
                        "https://auditors.tensorvm.net/{}/{}",
                        hex(&bundle_id),
                        index
                    ),
                    run.run_ended_at_unix_seconds,
                )
            })
            .collect();
        let operator_identity_attestations = run
            .nodes
            .iter()
            .map(|node| {
                PublicOperatorIdentityAttestation::new(
                    node.role,
                    node.address,
                    node.operator_id,
                    format!("https://operators.tensorvm.net/{}", hex(&node.operator_id)),
                    run.run_started_at_unix_seconds,
                )
            })
            .collect();
        let network_runtime_observations = public_network_runtime_observations_for_run(&run);
        let run_window_signature = sign_public_run_window(
            &signer,
            &bundle_id,
            run.run_started_at_unix_seconds,
            run.run_ended_at_unix_seconds,
            run.observed_blocks,
        );
        let reward_settlement_records = run.reward_settlement_records;
        let supporting_artifacts = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                record_summaries.block_history_root,
                record_summaries.block_history_records,
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                record_summaries.finality_history_root,
                record_summaries.finality_history_records,
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                record_summaries.network_runtime_observation_root,
                record_summaries.network_runtime_observation_records,
            ),
            (
                PublicEvidenceRecordKind::RandomnessBeaconEvidence,
                record_summaries.randomness_beacon_root,
                record_summaries.randomness_beacon_records,
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                record_summaries.data_availability_measurement_root,
                record_summaries.data_availability_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                record_summaries.invalid_work_rejection_root,
                record_summaries.invalid_work_rejection_records,
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                record_summaries.reward_settlement_root,
                reward_settlement_records,
            ),
        ]
        .into_iter()
        .map(|(kind, record_root, record_count)| {
            PublicEvidenceSupportingArtifact::new(
                &bundle_id,
                &signer,
                kind,
                public_evidence_supporting_artifact_uri(&bundle_id, kind),
                record_root,
                record_count,
            )
        })
        .collect();
        Self {
            run,
            publication,
            auditor_records,
            supporting_artifacts,
            run_window_signature,
            block_history_records: record_summaries.block_history_records,
            block_history_root: record_summaries.block_history_root,
            block_history_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::BlockHistory,
                &record_summaries.block_history_root,
                record_summaries.block_history_records,
            ),
            finality_history_records: record_summaries.finality_history_records,
            finality_history_root: record_summaries.finality_history_root,
            finality_history_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::FinalityHistory,
                &record_summaries.finality_history_root,
                record_summaries.finality_history_records,
            ),
            operator_identity_attestation_records: record_summaries
                .operator_identity_attestation_records,
            operator_identity_attestations,
            network_runtime_observations,
            network_runtime_observation_records: record_summaries
                .network_runtime_observation_records,
            network_runtime_observation_root: record_summaries.network_runtime_observation_root,
            network_runtime_observation_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &record_summaries.network_runtime_observation_root,
                record_summaries.network_runtime_observation_records,
            ),
            randomness_beacon_records: record_summaries.randomness_beacon_records,
            randomness_beacon_root: record_summaries.randomness_beacon_root,
            randomness_beacon_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::RandomnessBeaconEvidence,
                &record_summaries.randomness_beacon_root,
                record_summaries.randomness_beacon_records,
            ),
            randomness_beacon_raw_records: Vec::new(),
            data_availability_measurement_records: record_summaries
                .data_availability_measurement_records,
            data_availability_measurement_root: record_summaries.data_availability_measurement_root,
            data_availability_measurement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &record_summaries.data_availability_measurement_root,
                record_summaries.data_availability_measurement_records,
            ),
            invalid_work_rejection_records: record_summaries.invalid_work_rejection_records,
            invalid_work_rejection_root: record_summaries.invalid_work_rejection_root,
            invalid_work_rejection_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &record_summaries.invalid_work_rejection_root,
                record_summaries.invalid_work_rejection_records,
            ),
            reward_settlement_root: record_summaries.reward_settlement_root,
            reward_settlement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::RewardSettlements,
                &record_summaries.reward_settlement_root,
                reward_settlement_records,
            ),
        }
    }

    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
    ) -> PublicTestnetEvidenceBundleReport {
        let has_published_evidence_bundle =
            self.publication.is_published_and_independently_checkable();
        let valid_auditor_record_count = self.valid_auditor_record_count() as u64;
        let has_independent_auditor_records = self.publication.independent_auditor_count > 0
            && self.auditor_records.len() as u64 == self.publication.independent_auditor_count
            && valid_auditor_record_count == self.publication.independent_auditor_count;
        let has_signed_run_window = self.public_run_window_signature_valid();
        let has_block_history = self.run.observed_blocks > 0
            && self.block_history_records == self.run.observed_blocks
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::BlockHistory,
                &self.block_history_root,
                self.block_history_records,
                &self.block_history_signature,
            );
        let has_finality_history = self.run.observed_blocks > 0
            && self.finality_history_records == self.run.observed_blocks
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::FinalityHistory,
                &self.finality_history_root,
                self.finality_history_records,
                &self.finality_history_signature,
            );
        let (miner_operators, validator_operators) = self
            .run
            .matched_independent_public_operators_for_criteria(criteria);
        let miner_count = miner_operators.operator_ids.len();
        let validator_count = validator_operators.operator_ids.len();
        let required_operator_attestation_count = miner_count + validator_count;
        let required_operator_attestations = required_operator_attestation_count as u64;
        let has_operator_identity_attestations = required_operator_attestations > 0
            && self.operator_identity_attestation_records == required_operator_attestations
            && self.has_operator_identity_attestation_records_for_public_operators(
                required_operator_attestation_count,
                &miner_operators,
                &validator_operators,
            );
        let run_evidence = self.run.evaluate(
            criteria,
            block_time_seconds,
            has_operator_identity_attestations,
        );
        let required_network_runtime_observation_count = miner_count + validator_count;
        let required_network_runtime_observations =
            required_network_runtime_observation_count as u64;
        let has_network_runtime_observations =
            self.run.network_runtime.has_production_libp2p_runtime()
                && required_network_runtime_observations > 0
                && self.network_runtime_observation_records
                    == required_network_runtime_observations
                && self.has_network_runtime_observation_records_for_public_operators(
                    required_network_runtime_observation_count,
                    &miner_operators,
                    &validator_operators,
                )
                && self.public_record_signature_valid(
                    PublicEvidenceRecordKind::NetworkRuntimeObservations,
                    &self.network_runtime_observation_root,
                    self.network_runtime_observation_records,
                    &self.network_runtime_observation_signature,
                );
        let has_randomness_beacon_evidence = self.randomness_beacon_records > 0
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::RandomnessBeaconEvidence,
                &self.randomness_beacon_root,
                self.randomness_beacon_records,
                &self.randomness_beacon_signature,
            );
        let has_public_randomness_beacon_records = self.has_public_randomness_beacon_records();
        let has_data_availability_measurements = self.run.checked_receipts > 0
            && self.data_availability_measurement_records == self.run.checked_receipts
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &self.data_availability_measurement_root,
                self.data_availability_measurement_records,
                &self.data_availability_measurement_signature,
            );
        let has_invalid_work_rejection_records = run_evidence.has_invalid_work_rejection_evidence
            && self.invalid_work_rejection_records == self.run.invalid_receipts_submitted
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &self.invalid_work_rejection_root,
                self.invalid_work_rejection_records,
                &self.invalid_work_rejection_signature,
            );
        let has_reward_settlement_record_summary = run_evidence.has_reward_settlement_records
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::RewardSettlements,
                &self.reward_settlement_root,
                self.run.reward_settlement_records,
                &self.reward_settlement_signature,
            );
        let required_supporting_artifacts = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                &self.block_history_root,
                self.block_history_records,
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                &self.finality_history_root,
                self.finality_history_records,
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &self.network_runtime_observation_root,
                self.network_runtime_observation_records,
            ),
            (
                PublicEvidenceRecordKind::RandomnessBeaconEvidence,
                &self.randomness_beacon_root,
                self.randomness_beacon_records,
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &self.data_availability_measurement_root,
                self.data_availability_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &self.invalid_work_rejection_root,
                self.invalid_work_rejection_records,
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                &self.reward_settlement_root,
                self.run.reward_settlement_records,
            ),
        ];
        let has_public_supporting_record_artifacts = self.supporting_artifacts.len()
            == required_supporting_artifacts.len()
            && required_supporting_artifacts
                .iter()
                .all(|(kind, record_root, record_count)| {
                    self.has_exact_public_supporting_record_artifact(
                        *kind,
                        record_root,
                        *record_count,
                    )
                });
        let independently_checkable = has_published_evidence_bundle
            && has_independent_auditor_records
            && has_signed_run_window
            && has_block_history
            && has_finality_history
            && has_operator_identity_attestations
            && has_network_runtime_observations
            && has_randomness_beacon_evidence
            && has_data_availability_measurements
            && has_invalid_work_rejection_records
            && has_reward_settlement_record_summary
            && has_public_supporting_record_artifacts;
        let full_spec_evidence_met = public_testnet_criteria_are_full_spec(criteria)
            && run_evidence.public_criterion_met
            && independently_checkable
            && has_public_randomness_beacon_records;
        PublicTestnetEvidenceBundleReport {
            run_evidence,
            has_published_evidence_bundle,
            has_independent_auditor_records,
            has_signed_run_window,
            has_block_history,
            has_finality_history,
            has_operator_identity_attestations,
            has_network_runtime_observations,
            has_randomness_beacon_evidence,
            has_data_availability_measurements,
            has_invalid_work_rejection_records,
            has_reward_settlement_record_summary,
            has_public_supporting_record_artifacts,
            independently_checkable,
            full_spec_evidence_met,
        }
    }

    fn has_exact_public_supporting_record_artifact(
        &self,
        kind: PublicEvidenceRecordKind,
        record_root: &Hash,
        record_count: u64,
    ) -> bool {
        self.supporting_artifacts
            .iter()
            .filter(|artifact| {
                artifact.kind == kind
                    && artifact.record_root == *record_root
                    && artifact.record_count == record_count
                    && artifact.is_public_and_signed(
                        &self.publication.bundle_id,
                        &self.publication.manifest_signer,
                    )
            })
            .take(2)
            .count()
            == 1
    }

    fn public_record_signature_valid(
        &self,
        kind: PublicEvidenceRecordKind,
        record_root: &Hash,
        record_count: u64,
        signature: &Signature,
    ) -> bool {
        self.publication.manifest_signer != [0; 32]
            && self.publication.bundle_id != [0; 32]
            && *record_root != [0; 32]
            && verify_signature(
                &self.publication.manifest_signer,
                &public_evidence_record_message(
                    &self.publication.bundle_id,
                    kind,
                    record_root,
                    record_count,
                ),
                signature,
            )
    }

    fn has_public_randomness_beacon_records(&self) -> bool {
        if self.randomness_beacon_records == 0
            || self.randomness_beacon_raw_records.len() as u64 != self.randomness_beacon_records
        {
            return false;
        }
        if !self
            .randomness_beacon_raw_records
            .iter()
            .all(|record| record.is_accepted_public_unbiasable())
        {
            return false;
        }
        let record_roots = self
            .randomness_beacon_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>();
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            &record_roots,
        )
        .is_ok_and(|record_root| record_root == self.randomness_beacon_root)
    }

    fn public_run_window_signature_valid(&self) -> bool {
        self.publication.manifest_signer != [0; 32]
            && self.publication.bundle_id != [0; 32]
            && self.run.run_ended_at_unix_seconds >= self.run.run_started_at_unix_seconds
            && verify_signature(
                &self.publication.manifest_signer,
                &public_run_window_message(
                    &self.publication.bundle_id,
                    self.run.run_started_at_unix_seconds,
                    self.run.run_ended_at_unix_seconds,
                    self.run.observed_blocks,
                ),
                &self.run_window_signature,
            )
    }

    fn valid_auditor_record_count(&self) -> usize {
        let mut valid_auditors = BTreeSet::new();
        for auditor in &self.auditor_records {
            if auditor.auditor_id == self.publication.manifest_signer {
                continue;
            }
            if auditor.observed_at_unix_seconds < self.run.run_ended_at_unix_seconds {
                continue;
            }
            if auditor.has_external_auditor_proof(
                &self.publication.bundle_id,
                &self.publication.public_uri,
            ) {
                valid_auditors.insert(auditor.auditor_id);
            }
        }
        valid_auditors.len()
    }

    pub(super) fn has_operator_identity_attestation_records_for_public_operators(
        &self,
        required_count: usize,
        miner_operators: &MatchedPublicOperators,
        validator_operators: &MatchedPublicOperators,
    ) -> bool {
        if self.operator_identity_attestations.len() != required_count {
            return false;
        }
        let expected_attestation_keys =
            Self::public_operator_attestation_keys(miner_operators, validator_operators);
        if expected_attestation_keys.len() != required_count {
            return false;
        }
        let mut observed_attestation_keys = BTreeSet::new();
        for attestation in &self.operator_identity_attestations {
            let attestation_key = public_operator_attestation_key(
                attestation.role,
                &attestation.address,
                &attestation.operator_id,
            );
            if !expected_attestation_keys.contains(&attestation_key)
                || !attestation.has_external_identity_proof()
                || !self
                    .run
                    .observation_is_within_run(attestation.observed_at_unix_seconds)
                || !observed_attestation_keys.insert(attestation_key)
            {
                return false;
            }
        }
        observed_attestation_keys == expected_attestation_keys
    }

    fn public_operator_attestation_keys(
        miner_operators: &MatchedPublicOperators,
        validator_operators: &MatchedPublicOperators,
    ) -> BTreeSet<Hash> {
        let mut attestation_keys = miner_operators.attestation_keys_for_role(PublicNodeRole::Miner);
        attestation_keys
            .extend(validator_operators.attestation_keys_for_role(PublicNodeRole::Validator));
        attestation_keys
    }

    fn public_operator_ids(
        miner_operators: &MatchedPublicOperators,
        validator_operators: &MatchedPublicOperators,
    ) -> BTreeSet<Hash> {
        let mut operator_ids = miner_operators.operator_ids.clone();
        operator_ids.extend(validator_operators.operator_ids.iter().copied());
        operator_ids
    }

    pub(super) fn has_network_runtime_observation_records_for_public_operators(
        &self,
        required_count: usize,
        miner_operators: &MatchedPublicOperators,
        validator_operators: &MatchedPublicOperators,
    ) -> bool {
        if self.network_runtime_observations.len() != required_count {
            return false;
        }
        let expected_operator_ids = Self::public_operator_ids(miner_operators, validator_operators);
        if expected_operator_ids.len() != required_count {
            return false;
        }
        let mut observed_operator_ids = BTreeSet::new();
        let mut record_roots = Vec::with_capacity(required_count);
        for observation in &self.network_runtime_observations {
            if !expected_operator_ids.contains(&observation.operator_id)
                || !self
                    .run
                    .observation_is_within_run(observation.observed_at_unix_seconds)
                || !observation.has_public_network_observation_proof()
                || !observed_operator_ids.insert(observation.operator_id)
            {
                return false;
            }
            record_roots.push(observation.record_root);
        }
        observed_operator_ids == expected_operator_ids
            && aggregate_public_evidence_record_roots(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &record_roots,
            )
            .is_ok_and(|record_root| record_root == self.network_runtime_observation_root)
    }
}
