use super::public_urls::{
    public_host_is_external, public_https_authorities_match, public_https_host, public_https_path,
};
use super::{
    PublicDeploymentServicePlan, PublicServiceKind, PublicTestnetPreflightPlan,
    PublicTestnetPreflightReport, public_service_kinds, required_blocks_for_days,
};
use std::collections::BTreeSet;

impl PublicDeploymentServicePlan {
    pub fn is_public_https_endpoint(&self) -> bool {
        let Some(host) = public_https_host(&self.public_url) else {
            return false;
        };
        public_host_is_external(host)
    }

    pub fn has_public_content_surface(&self) -> bool {
        let Some(host) = public_https_host(&self.content_url) else {
            return false;
        };
        public_host_is_external(host)
            && public_https_authorities_match(&self.public_url, &self.content_url)
            && self.content_path == self.kind.content_path()
            && public_https_path(&self.content_url) == Some(self.kind.content_path())
    }

    pub fn is_ready_for_public_run(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.is_public_https_endpoint()
            && self.health_path.starts_with('/')
            && self.health_path.len() > 1
            && public_https_path(&self.public_url) == Some(self.health_path.as_str())
            && self.has_public_content_surface()
            && self.auth_enabled
            && self.rate_limit_enabled
    }
}

impl PublicTestnetPreflightPlan {
    pub fn evaluate(&self, block_time_seconds: u64) -> PublicTestnetPreflightReport {
        let required_blocks =
            required_blocks_for_days(self.criteria.duration_days, block_time_seconds.max(1));
        let has_required_miners = self.config.miner_count >= self.criteria.min_miners;
        let has_required_validators = self.config.validator_count >= self.criteria.min_validators;
        let has_positive_stakes = self.config.miner_stake > 0 && self.config.validator_stake > 0;
        let has_funded_faucet =
            self.config.faucet_drip > 0 && self.config.faucet_balance >= self.config.faucet_drip;
        let has_cuda_ready_miners = self.cuda_kernels_available
            && self.config.miner_count > 0
            && self.cuda_ready_miner_count == self.config.miner_count;
        let planned_node_count = self
            .config
            .miner_count
            .saturating_add(self.config.validator_count);
        let has_libp2p_ready_nodes =
            planned_node_count > 0 && self.libp2p_ready_node_count == planned_node_count;
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_rpc_service_plan = self.has_ready_service_plan(PublicServiceKind::Rpc);
        let has_explorer_service_plan = self.has_ready_service_plan(PublicServiceKind::Explorer);
        let has_faucet_service_plan = self.has_ready_service_plan(PublicServiceKind::Faucet);
        let has_telemetry_service_plan = self.has_ready_service_plan(PublicServiceKind::Telemetry);
        let has_public_service_content_plan = self
            .has_ready_service_content_plan(PublicServiceKind::Rpc)
            && self.has_ready_service_content_plan(PublicServiceKind::Explorer)
            && self.has_ready_service_content_plan(PublicServiceKind::Faucet)
            && self.has_ready_service_content_plan(PublicServiceKind::Telemetry);
        let has_public_service_plan = has_rpc_service_plan
            && has_explorer_service_plan
            && has_faucet_service_plan
            && has_telemetry_service_plan
            && has_public_service_content_plan
            && self.has_exact_ready_service_plans()
            && self.has_distinct_ready_service_endpoint_ids()
            && self.has_distinct_ready_service_urls();
        let local_shape_ready = has_required_miners
            && has_required_validators
            && has_positive_stakes
            && has_funded_faucet
            && required_blocks > 0;
        let deployment_plan_ready = has_cuda_ready_miners
            && has_libp2p_ready_nodes
            && has_production_libp2p_runtime
            && has_public_service_plan;
        PublicTestnetPreflightReport {
            miner_count: self.config.miner_count,
            validator_count: self.config.validator_count,
            required_blocks,
            has_required_miners,
            has_required_validators,
            has_positive_stakes,
            has_funded_faucet,
            has_cuda_kernels_available: self.cuda_kernels_available,
            cuda_ready_miner_count: self.cuda_ready_miner_count,
            has_cuda_ready_miners,
            libp2p_ready_node_count: self.libp2p_ready_node_count,
            has_libp2p_ready_nodes,
            has_production_libp2p_runtime,
            has_rpc_service_plan,
            has_explorer_service_plan,
            has_faucet_service_plan,
            has_telemetry_service_plan,
            has_public_service_content_plan,
            has_public_service_plan,
            local_shape_ready,
            deployment_plan_ready,
            can_start_public_run: local_shape_ready && deployment_plan_ready,
        }
    }

    fn has_ready_service_plan(&self, kind: PublicServiceKind) -> bool {
        self.services
            .iter()
            .any(|service| service.kind == kind && service.is_ready_for_public_run())
    }

    fn has_ready_service_content_plan(&self, kind: PublicServiceKind) -> bool {
        self.services
            .iter()
            .any(|service| service.kind == kind && service.has_public_content_surface())
    }

    pub(super) fn has_distinct_ready_service_endpoint_ids(&self) -> bool {
        let mut endpoint_ids = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(service) = self
                .services
                .iter()
                .find(|service| service.kind == kind && service.is_ready_for_public_run())
            else {
                return false;
            };
            if !endpoint_ids.insert(service.endpoint_id) {
                return false;
            }
        }
        true
    }

    fn has_distinct_ready_service_urls(&self) -> bool {
        let mut health_urls = BTreeSet::new();
        let mut content_urls = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(service) = self
                .services
                .iter()
                .find(|service| service.kind == kind && service.is_ready_for_public_run())
            else {
                return false;
            };
            if !health_urls.insert(service.public_url.as_str())
                || !content_urls.insert(service.content_url.as_str())
            {
                return false;
            }
        }
        true
    }

    fn has_exact_ready_service_plans(&self) -> bool {
        self.services.len() == public_service_kinds().len()
            && public_service_kinds().iter().all(|kind| {
                self.services
                    .iter()
                    .filter(|service| service.kind == *kind && service.is_ready_for_public_run())
                    .count()
                    == 1
            })
    }
}
