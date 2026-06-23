use super::public_urls::{public_https_authorities_match, public_https_url_key};
use super::{
    Hash, PublicServiceKind, PublicTestnetCriteria, PublicTestnetEvidence,
    PublicTestnetRunEvidence, public_service_kinds, ratio_parts_to_bps, required_blocks_for_days,
    required_duration_seconds_for_days,
};
use std::collections::BTreeSet;

impl PublicTestnetRunEvidence {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let (miner_operators, validator_operators) =
            self.matched_independent_public_operators_for_criteria(criteria);
        let miner_count = miner_operators.operator_ids.len();
        let validator_count = validator_operators.operator_ids.len();
        let required_blocks =
            required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
        let required_duration_seconds = required_duration_seconds_for_days(criteria.duration_days);
        let has_valid_run_window =
            self.run_ended_at_unix_seconds >= self.run_started_at_unix_seconds;
        let observed_duration_seconds = if has_valid_run_window {
            self.run_ended_at_unix_seconds
                .saturating_sub(self.run_started_at_unix_seconds)
        } else {
            0
        };
        let finality_rate_bps = ratio_parts_to_bps(self.finalized_blocks, self.observed_blocks);
        let data_availability_bps =
            ratio_parts_to_bps(self.available_receipts, self.checked_receipts);
        let has_consistent_finality_counts = self.finalized_blocks <= self.observed_blocks;
        let has_consistent_data_availability_counts =
            self.available_receipts <= self.checked_receipts;
        let invalid_work_rejection_rate_bps = ratio_parts_to_bps(
            self.invalid_receipts_rejected,
            self.invalid_receipts_submitted,
        );
        let has_required_miners = miner_count >= criteria.min_miners;
        let has_required_validators = validator_count >= criteria.min_validators;
        let has_required_run_duration =
            has_valid_run_window && observed_duration_seconds >= required_duration_seconds;
        let has_required_block_count = self.observed_blocks >= required_blocks;
        let has_required_finality =
            has_consistent_finality_counts && finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability = has_consistent_data_availability_counts
            && data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = self.invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected <= self.invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            self.reward_settlement_records >= criteria.min_reward_settlement_records;
        let has_cuda_verified_miners =
            miner_count > 0 && self.cuda_verified_miner_count >= miner_count as u64;
        let has_cuda_graph_execution_evidence = has_cuda_verified_miners
            && self.cuda_graph_execution_receipts > 0
            && self.cuda_graph_execution_receipts <= self.checked_receipts
            && self.cuda_graph_execution_receipts <= self.available_receipts;
        let has_validator_vrf_lifecycle_evidence = self.checked_receipts > 0
            && self.available_receipts == self.checked_receipts
            && self.validator_vrf_lifecycle_records == self.checked_receipts;
        let has_deployed_detection_measurements = self.detection_measurement_records > 0;
        let external_operator_evidence =
            external_operator_evidence && miner_count > 0 && validator_count > 0;
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_exact_deployed_service_records = self.has_exact_deployed_service_records();
        let has_rpc_content = has_exact_deployed_service_records
            && self.has_service_content_for_reachable_endpoint(PublicServiceKind::Rpc);
        let has_explorer_content = has_exact_deployed_service_records
            && self.has_service_content_for_reachable_endpoint(PublicServiceKind::Explorer);
        let has_faucet_content = has_exact_deployed_service_records
            && self.has_service_content_for_reachable_endpoint(PublicServiceKind::Faucet);
        let has_telemetry_content = has_exact_deployed_service_records
            && self.has_service_content_for_reachable_endpoint(PublicServiceKind::Telemetry);
        let has_distinct_deployed_service_endpoint_ids =
            self.has_distinct_deployed_service_endpoint_ids();
        let has_distinct_deployed_service_content_roots =
            self.has_distinct_deployed_service_content_roots();
        let has_distinct_deployed_service_urls = self.has_distinct_deployed_service_urls();
        let has_deployed_public_service_content = has_rpc_content
            && has_explorer_content
            && has_faucet_content
            && has_telemetry_content
            && has_distinct_deployed_service_content_roots;
        let has_deployed_rpc_service = has_rpc_content;
        let has_deployed_explorer_service = has_explorer_content;
        let has_deployed_faucet_service = has_faucet_content;
        let has_deployed_telemetry_service = has_telemetry_content;
        let has_deployed_public_services = has_deployed_rpc_service
            && has_deployed_explorer_service
            && has_deployed_faucet_service
            && has_deployed_telemetry_service
            && has_deployed_public_service_content
            && has_distinct_deployed_service_endpoint_ids
            && has_distinct_deployed_service_urls;
        let public_criterion_met = has_required_miners
            && has_required_validators
            && has_required_run_duration
            && has_required_block_count
            && has_required_finality
            && has_required_data_availability
            && has_invalid_work_rejection_evidence
            && has_reward_settlement_records
            && has_production_libp2p_runtime
            && has_deployed_public_services
            && external_operator_evidence;
        PublicTestnetEvidence {
            miner_count,
            validator_count,
            run_started_at_unix_seconds: self.run_started_at_unix_seconds,
            run_ended_at_unix_seconds: self.run_ended_at_unix_seconds,
            observed_duration_seconds,
            required_duration_seconds,
            observed_blocks: self.observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted: self.invalid_receipts_submitted,
            invalid_receipts_rejected: self.invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records: self.reward_settlement_records,
            external_operator_evidence,
            has_production_libp2p_runtime,
            has_deployed_rpc_service,
            has_deployed_explorer_service,
            has_deployed_faucet_service,
            has_deployed_telemetry_service,
            has_deployed_public_service_content,
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_run_duration,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            cuda_verified_miner_count: self.cuda_verified_miner_count,
            has_cuda_verified_miners,
            cuda_graph_execution_receipts: self.cuda_graph_execution_receipts,
            has_cuda_graph_execution_evidence,
            validator_vrf_lifecycle_records: self.validator_vrf_lifecycle_records,
            has_validator_vrf_lifecycle_evidence,
            detection_measurement_records: self.detection_measurement_records,
            has_deployed_detection_measurements,
            public_criterion_met,
        }
    }

    fn has_service_content_for_reachable_endpoint(&self, kind: PublicServiceKind) -> bool {
        self.deployed_service_content_root(kind).is_some()
    }

    fn has_exact_deployed_service_records(&self) -> bool {
        self.services.len() == public_service_kinds().len()
            && self.service_content.len() == public_service_kinds().len()
            && public_service_kinds().iter().all(|kind| {
                self.services
                    .iter()
                    .filter(|service| service.kind == *kind)
                    .count()
                    == 1
                    && self
                        .service_content
                        .iter()
                        .filter(|content| content.kind == *kind)
                        .count()
                        == 1
            })
    }

    fn deployed_service_content_root(&self, kind: PublicServiceKind) -> Option<Hash> {
        self.services
            .iter()
            .filter(|service| {
                service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
            })
            .find_map(|service| {
                self.service_content.iter().find_map(|content| {
                    let matches_content = content.kind == kind
                        && content.endpoint_id == service.endpoint_id
                        && content.has_external_content_proof()
                        && public_https_authorities_match(&service.public_url, &content.public_url)
                        && self.observation_is_within_run(content.observed_at_unix_seconds);
                    matches_content.then_some(content.content_root)
                })
            })
    }

    fn deployed_service_endpoint_id(&self, kind: PublicServiceKind) -> Option<Hash> {
        self.services
            .iter()
            .filter(|service| {
                service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
            })
            .find_map(|service| {
                self.service_content
                    .iter()
                    .any(|content| {
                        content.kind == kind
                            && content.endpoint_id == service.endpoint_id
                            && content.has_external_content_proof()
                            && public_https_authorities_match(
                                &service.public_url,
                                &content.public_url,
                            )
                            && self.observation_is_within_run(content.observed_at_unix_seconds)
                    })
                    .then_some(service.endpoint_id)
            })
    }

    fn deployed_service_urls(&self, kind: PublicServiceKind) -> Option<(String, String)> {
        self.services
            .iter()
            .filter(|service| {
                service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
            })
            .find_map(|service| {
                self.service_content.iter().find_map(|content| {
                    let matches_content = content.kind == kind
                        && content.endpoint_id == service.endpoint_id
                        && content.has_external_content_proof()
                        && public_https_authorities_match(&service.public_url, &content.public_url)
                        && self.observation_is_within_run(content.observed_at_unix_seconds);
                    if !matches_content {
                        return None;
                    }
                    Some((
                        public_https_url_key(&service.public_url)?,
                        public_https_url_key(&content.public_url)?,
                    ))
                })
            })
    }

    fn has_distinct_deployed_service_endpoint_ids(&self) -> bool {
        let mut endpoint_ids = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(endpoint_id) = self.deployed_service_endpoint_id(kind) else {
                return false;
            };
            if !endpoint_ids.insert(endpoint_id) {
                return false;
            }
        }
        true
    }

    fn has_distinct_deployed_service_urls(&self) -> bool {
        let mut service_urls = BTreeSet::new();
        let mut content_urls = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some((service_url, content_url)) = self.deployed_service_urls(kind) else {
                return false;
            };
            if !service_urls.insert(service_url) || !content_urls.insert(content_url) {
                return false;
            }
        }
        true
    }

    fn has_distinct_deployed_service_content_roots(&self) -> bool {
        let mut content_roots = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(content_root) = self.deployed_service_content_root(kind) else {
                return false;
            };
            if !content_roots.insert(content_root) {
                return false;
            }
        }
        true
    }

    pub(super) fn observation_is_within_run(&self, observed_at_unix_seconds: u64) -> bool {
        self.run_started_at_unix_seconds <= observed_at_unix_seconds
            && observed_at_unix_seconds <= self.run_ended_at_unix_seconds
    }
}
