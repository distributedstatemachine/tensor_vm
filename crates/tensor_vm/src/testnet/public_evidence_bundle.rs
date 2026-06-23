use super::public_evidence_crypto::{
    aggregate_public_evidence_record_roots, public_evidence_record_message,
    public_evidence_supporting_artifact_uri, public_network_runtime_observations_for_run,
    public_run_window_message, sign_public_evidence_record, sign_public_run_window,
};
use super::public_operators::{MatchedPublicOperators, public_operator_attestation_key};
use super::{
    PublicDataAvailabilityStatus, PublicEvidenceAuditorRecord, PublicEvidencePublication,
    PublicEvidenceRecordKind, PublicEvidenceRecordSummaries, PublicEvidenceSupportingArtifact,
    PublicNodeRole, PublicOperatorIdentityAttestation, PublicTestnetCriteria,
    PublicTestnetEvidenceBundle, PublicTestnetEvidenceBundleReport, PublicTestnetRunEvidence,
    PublicValidatorVrfLifecyclePhase, public_testnet_criteria_are_full_spec,
};
use crate::hash::hex;
use crate::types::{Hash, Signature, address, verify_signature};
use std::collections::{BTreeMap, BTreeSet};

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
            (
                PublicEvidenceRecordKind::DetectionMeasurements,
                record_summaries.detection_measurement_root,
                record_summaries.detection_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::ValidatorVrfLifecycle,
                record_summaries.validator_vrf_lifecycle_root,
                record_summaries.validator_vrf_lifecycle_records,
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
            block_history_raw_records: Vec::new(),
            finality_history_records: record_summaries.finality_history_records,
            finality_history_root: record_summaries.finality_history_root,
            finality_history_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::FinalityHistory,
                &record_summaries.finality_history_root,
                record_summaries.finality_history_records,
            ),
            finality_history_raw_records: Vec::new(),
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
            data_availability_raw_records: Vec::new(),
            invalid_work_rejection_records: record_summaries.invalid_work_rejection_records,
            invalid_work_rejection_root: record_summaries.invalid_work_rejection_root,
            invalid_work_rejection_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &record_summaries.invalid_work_rejection_root,
                record_summaries.invalid_work_rejection_records,
            ),
            invalid_work_raw_records: Vec::new(),
            reward_settlement_root: record_summaries.reward_settlement_root,
            reward_settlement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::RewardSettlements,
                &record_summaries.reward_settlement_root,
                reward_settlement_records,
            ),
            reward_settlement_raw_records: Vec::new(),
            detection_measurement_records: record_summaries.detection_measurement_records,
            detection_measurement_root: record_summaries.detection_measurement_root,
            detection_measurement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::DetectionMeasurements,
                &record_summaries.detection_measurement_root,
                record_summaries.detection_measurement_records,
            ),
            detection_measurement_raw_records: Vec::new(),
            validator_vrf_lifecycle_records: record_summaries.validator_vrf_lifecycle_records,
            validator_vrf_lifecycle_root: record_summaries.validator_vrf_lifecycle_root,
            validator_vrf_lifecycle_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::ValidatorVrfLifecycle,
                &record_summaries.validator_vrf_lifecycle_root,
                record_summaries.validator_vrf_lifecycle_records,
            ),
            validator_vrf_lifecycle_raw_records: Vec::new(),
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
        let has_randomness_beacon_evidence = self.run.observed_blocks > 0
            && self.randomness_beacon_records == self.run.observed_blocks
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::RandomnessBeaconEvidence,
                &self.randomness_beacon_root,
                self.randomness_beacon_records,
                &self.randomness_beacon_signature,
            );
        let has_public_randomness_beacon_records = self.has_public_randomness_beacon_records();
        let has_verified_public_randomness_beacon_records = false;
        let has_public_chain_history_records = self.has_public_chain_history_records();
        let has_public_operational_records = self.has_public_operational_records();
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
        let has_deployed_detection_measurement_records = run_evidence
            .has_deployed_detection_measurements
            && self.detection_measurement_records == self.run.detection_measurement_records
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::DetectionMeasurements,
                &self.detection_measurement_root,
                self.detection_measurement_records,
                &self.detection_measurement_signature,
            );
        let has_validator_vrf_lifecycle_record_summary = run_evidence
            .has_validator_vrf_lifecycle_evidence
            && self.validator_vrf_lifecycle_records == self.run.validator_vrf_lifecycle_records
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::ValidatorVrfLifecycle,
                &self.validator_vrf_lifecycle_root,
                self.validator_vrf_lifecycle_records,
                &self.validator_vrf_lifecycle_signature,
            );
        let has_public_validator_vrf_lifecycle_records =
            self.has_public_validator_vrf_lifecycle_records();
        let has_verified_public_validator_vrf_lifecycle_records = false;
        let has_deployed_public_service_evidence = run_evidence.has_deployed_public_services;
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
            (
                PublicEvidenceRecordKind::DetectionMeasurements,
                &self.detection_measurement_root,
                self.detection_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::ValidatorVrfLifecycle,
                &self.validator_vrf_lifecycle_root,
                self.validator_vrf_lifecycle_records,
            ),
        ];
        let has_public_supporting_record_artifacts = self.supporting_artifacts.len()
            == required_supporting_artifacts.len()
            && self.has_distinct_public_supporting_artifact_uris()
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
            && has_deployed_public_service_evidence
            && has_randomness_beacon_evidence
            && has_data_availability_measurements
            && has_invalid_work_rejection_records
            && has_reward_settlement_record_summary
            && has_deployed_detection_measurement_records
            && has_validator_vrf_lifecycle_record_summary
            && has_public_supporting_record_artifacts;
        let full_spec_evidence_met = public_testnet_criteria_are_full_spec(criteria)
            && run_evidence.public_criterion_met
            && run_evidence.has_cuda_verified_miners
            && run_evidence.has_cuda_graph_execution_evidence
            && run_evidence.has_validator_vrf_lifecycle_evidence
            && has_validator_vrf_lifecycle_record_summary
            && run_evidence.has_deployed_detection_measurements
            && independently_checkable
            && has_public_randomness_beacon_records
            && has_verified_public_randomness_beacon_records
            && has_public_chain_history_records
            && has_public_operational_records
            && has_public_validator_vrf_lifecycle_records
            && has_verified_public_validator_vrf_lifecycle_records;
        let has_cuda_verified_miners = run_evidence.has_cuda_verified_miners;
        let has_cuda_graph_execution_evidence = run_evidence.has_cuda_graph_execution_evidence;
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
            has_deployed_public_service_evidence,
            has_deployed_detection_measurement_records,
            has_validator_vrf_lifecycle_record_summary,
            has_public_randomness_beacon_records,
            has_verified_public_randomness_beacon_records,
            has_public_validator_vrf_lifecycle_records,
            has_verified_public_validator_vrf_lifecycle_records,
            has_public_supporting_record_artifacts,
            has_cuda_verified_miners,
            has_cuda_graph_execution_evidence,
            independently_checkable,
            full_spec_evidence_met,
        }
    }

    fn has_distinct_public_supporting_artifact_uris(&self) -> bool {
        let mut artifact_uris = BTreeSet::new();
        self.supporting_artifacts
            .iter()
            .all(|artifact| artifact_uris.insert(artifact.artifact_uri.as_str()))
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
        let mut observed_blocks = BTreeSet::new();
        let mut beacon_rounds = BTreeSet::new();
        if self.randomness_beacon_raw_records.iter().any(|record| {
            record.observed_block >= self.run.observed_blocks
                || !observed_blocks.insert(record.observed_block)
                || !beacon_rounds.insert((record.source_id, record.beacon_round))
        }) {
            return false;
        }
        if observed_blocks.len() as u64 != self.run.observed_blocks {
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

    fn has_public_operational_records(&self) -> bool {
        self.has_public_data_availability_measurement_records()
            && self.has_public_invalid_work_rejection_records()
            && self.has_public_reward_settlement_records()
            && self.has_public_detection_measurement_records()
    }

    fn has_public_data_availability_measurement_records(&self) -> bool {
        let mut receipt_roots = BTreeSet::new();
        if self.data_availability_raw_records.iter().any(|record| {
            !self.observed_block_in_run(record.observed_block)
                || record.receipt_root == [0; 32]
                || !receipt_roots.insert(record.receipt_root)
        }) {
            return false;
        }
        self.raw_operational_records_match(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            self.data_availability_measurement_records,
            &self.data_availability_measurement_root,
            self.data_availability_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn has_public_invalid_work_rejection_records(&self) -> bool {
        let mut receipt_roots = BTreeSet::new();
        if self.invalid_work_raw_records.iter().any(|record| {
            !self.observed_block_in_run(record.observed_block)
                || record.receipt_root == [0; 32]
                || !receipt_roots.insert(record.receipt_root)
        }) {
            return false;
        }
        self.raw_operational_records_match(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            self.invalid_work_rejection_records,
            &self.invalid_work_rejection_root,
            self.invalid_work_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn has_public_reward_settlement_records(&self) -> bool {
        let mut receipt_roots = BTreeSet::new();
        if self.reward_settlement_raw_records.iter().any(|record| {
            !self.observed_block_in_run(record.observed_block)
                || record.receipt_root == [0; 32]
                || record.miner_id == [0; 32]
                || record.validator_id == [0; 32]
                || !receipt_roots.insert(record.receipt_root)
        }) {
            return false;
        }
        self.raw_operational_records_match(
            PublicEvidenceRecordKind::RewardSettlements,
            self.run.reward_settlement_records,
            &self.reward_settlement_root,
            self.reward_settlement_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn has_public_detection_measurement_records(&self) -> bool {
        if !self
            .detection_measurement_raw_records
            .iter()
            .all(|record| self.detection_measurement_record_fields_valid(record))
        {
            return false;
        }
        self.raw_operational_records_match(
            PublicEvidenceRecordKind::DetectionMeasurements,
            self.detection_measurement_records,
            &self.detection_measurement_root,
            self.detection_measurement_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn detection_measurement_record_fields_valid(
        &self,
        record: &super::PublicDetectionMeasurementRecord,
    ) -> bool {
        self.observed_block_in_run(record.observed_block)
            && !record.mechanism.is_empty()
            && record
                .mechanism
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && record.subject_root != [0; 32]
            && record.sample_count > 0
            && record.detected_count <= record.sample_count
    }

    fn has_public_validator_vrf_lifecycle_records(&self) -> bool {
        if self.validator_vrf_lifecycle_records == 0
            || self.validator_vrf_lifecycle_raw_records.len() as u64
                != self.validator_vrf_lifecycle_records
        {
            return false;
        }
        let available_receipt_roots = self.available_receipt_roots();
        if available_receipt_roots.is_empty()
            || self.validator_vrf_lifecycle_records
                != (available_receipt_roots.len() as u64).saturating_mul(2)
        {
            return false;
        }
        let mut lifecycle_by_receipt = BTreeMap::<Hash, ValidatorVrfLifecycleEvidence>::new();
        for record in &self.validator_vrf_lifecycle_raw_records {
            if !self.observed_block_in_run(record.observed_block)
                || record.receipt_root == [0; 32]
                || record.validator_id == [0; 32]
                || record.beacon_round == 0
            {
                return false;
            }
            let lifecycle = lifecycle_by_receipt.entry(record.receipt_root).or_default();
            if !lifecycle.record(record) {
                return false;
            }
        }
        if lifecycle_by_receipt
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != available_receipt_roots
        {
            return false;
        }
        if !lifecycle_by_receipt.values().all(|lifecycle| {
            lifecycle.committed
                && lifecycle.revealed
                && lifecycle.committed_validator_id == lifecycle.revealed_validator_id
                && lifecycle.committed_beacon_round == lifecycle.revealed_beacon_round
                && lifecycle.committed_observed_block <= lifecycle.revealed_observed_block
        }) {
            return false;
        }
        self.raw_operational_records_match(
            PublicEvidenceRecordKind::ValidatorVrfLifecycle,
            self.validator_vrf_lifecycle_records,
            &self.validator_vrf_lifecycle_root,
            self.validator_vrf_lifecycle_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn available_receipt_roots(&self) -> BTreeSet<Hash> {
        self.data_availability_raw_records
            .iter()
            .filter(|record| record.status == PublicDataAvailabilityStatus::Available)
            .map(|record| record.receipt_root)
            .collect()
    }

    fn has_public_chain_history_records(&self) -> bool {
        let mut block_roots_by_height = BTreeMap::new();
        if self.block_history_raw_records.iter().any(|record| {
            !self.observed_block_in_run(record.block)
                || record.block_root == [0; 32]
                || block_roots_by_height
                    .insert(record.block, record.block_root)
                    .is_some()
        }) {
            return false;
        }

        let mut finality_blocks = BTreeSet::new();
        let mut finalized_blocks = 0_u64;
        for record in &self.finality_history_raw_records {
            if !self.observed_block_in_run(record.block)
                || record.block_root == [0; 32]
                || !finality_blocks.insert(record.block)
            {
                return false;
            }
            if block_roots_by_height.get(&record.block) != Some(&record.block_root) {
                return false;
            }
            if record.status == super::PublicFinalityHistoryStatus::Finalized {
                finalized_blocks = finalized_blocks.saturating_add(1);
            }
        }
        if finality_blocks.len() != block_roots_by_height.len()
            || finalized_blocks != self.run.finalized_blocks
        {
            return false;
        }
        if block_roots_by_height
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != self.observed_block_range()
        {
            return false;
        }

        self.raw_operational_records_match(
            PublicEvidenceRecordKind::BlockHistory,
            self.block_history_records,
            &self.block_history_root,
            self.block_history_raw_records
                .iter()
                .map(|record| record.record_root()),
        ) && self.raw_operational_records_match(
            PublicEvidenceRecordKind::FinalityHistory,
            self.finality_history_records,
            &self.finality_history_root,
            self.finality_history_raw_records
                .iter()
                .map(|record| record.record_root()),
        )
    }

    fn observed_block_in_run(&self, observed_block: u64) -> bool {
        observed_block < self.run.observed_blocks
    }

    fn observed_block_range(&self) -> BTreeSet<u64> {
        (0..self.run.observed_blocks).collect()
    }

    fn raw_operational_records_match(
        &self,
        kind: PublicEvidenceRecordKind,
        expected_count: u64,
        expected_root: &Hash,
        record_roots: impl Iterator<Item = Hash>,
    ) -> bool {
        if expected_count == 0 {
            return false;
        }
        let record_roots = record_roots.collect::<Vec<_>>();
        if record_roots.len() as u64 != expected_count {
            return false;
        }
        aggregate_public_evidence_record_roots(kind, &record_roots)
            .is_ok_and(|record_root| record_root == *expected_root)
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
        let mut observed_peer_ids = BTreeSet::new();
        let mut observed_listen_addresses = BTreeSet::new();
        let mut record_roots = Vec::with_capacity(required_count);
        for observation in &self.network_runtime_observations {
            let Ok(listen_address) = observation.listen_address.parse::<libp2p::Multiaddr>() else {
                return false;
            };
            if !expected_operator_ids.contains(&observation.operator_id)
                || !self
                    .run
                    .observation_is_within_run(observation.observed_at_unix_seconds)
                || !observation.has_public_network_observation_proof()
                || !observed_operator_ids.insert(observation.operator_id)
                || !observed_peer_ids.insert(observation.peer_id.clone())
                || !observed_listen_addresses.insert(listen_address.to_string())
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

#[derive(Default)]
struct ValidatorVrfLifecycleEvidence {
    committed: bool,
    revealed: bool,
    committed_validator_id: Hash,
    revealed_validator_id: Hash,
    committed_beacon_round: u64,
    revealed_beacon_round: u64,
    committed_observed_block: u64,
    revealed_observed_block: u64,
}

impl ValidatorVrfLifecycleEvidence {
    fn record(&mut self, record: &super::PublicValidatorVrfLifecycleRecord) -> bool {
        match record.phase {
            PublicValidatorVrfLifecyclePhase::Committed => {
                if self.committed {
                    return false;
                }
                self.committed = true;
                self.committed_validator_id = record.validator_id;
                self.committed_beacon_round = record.beacon_round;
                self.committed_observed_block = record.observed_block;
            }
            PublicValidatorVrfLifecyclePhase::Revealed => {
                if self.revealed {
                    return false;
                }
                self.revealed = true;
                self.revealed_validator_id = record.validator_id;
                self.revealed_beacon_round = record.beacon_round;
                self.revealed_observed_block = record.observed_block;
            }
        }
        true
    }
}
