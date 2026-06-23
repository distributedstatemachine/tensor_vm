use crate::chain::{Chain, ChainParams};
use crate::scheduler::{JobScheduler, SyntheticLocalJobSource};
use crate::types::Hash;
use crate::verify::FreivaldsParams;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainNetwork {
    Local,
    Testnet,
    Mainnet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeRole {
    Gateway,
    Miner,
    Proposer,
    Validator,
    Explorer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceExposure {
    LoopbackOnly,
    PublicHttps,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainProfile {
    pub network: ChainNetwork,
    pub chain_params: ChainParams,
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
    pub synthetic_job_scheduler: Option<JobScheduler>,
    pub public_evidence_required: bool,
    pub service_exposure: ServiceExposure,
}

impl ChainProfile {
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "local" | "local_cpu" => Some(Self::local_cpu()),
            "testnet" | "public_testnet" => Some(Self::public_testnet()),
            "mainnet" => Some(Self::mainnet()),
            _ => None,
        }
    }

    pub fn local_cpu() -> Self {
        let chain_params = ChainParams {
            freivalds: FreivaldsParams {
                validators_per_job: 5,
                minimum_validators: 4,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        Self {
            network: ChainNetwork::Local,
            chain_params,
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
            synthetic_job_scheduler: Some(JobScheduler::with_small_shape((8, 8, 8))),
            public_evidence_required: false,
            service_exposure: ServiceExposure::LoopbackOnly,
        }
    }

    pub fn public_testnet() -> Self {
        Self {
            network: ChainNetwork::Testnet,
            chain_params: ChainParams::default(),
            synthetic_job_scheduler: None,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn mainnet() -> Self {
        Self {
            network: ChainNetwork::Mainnet,
            chain_params: ChainParams::default(),
            synthetic_job_scheduler: None,
            public_evidence_required: true,
            service_exposure: ServiceExposure::PublicHttps,
            ..Self::local_cpu()
        }
    }

    pub fn build_chain(&self, finalized_randomness: Hash) -> Chain {
        Chain::with_params(self.chain_params.clone(), finalized_randomness)
    }

    pub fn label(&self) -> &'static str {
        match self.network {
            ChainNetwork::Local => "local_cpu",
            ChainNetwork::Testnet => "public_testnet",
            ChainNetwork::Mainnet => "mainnet",
        }
    }

    pub fn requires_public_services(&self) -> bool {
        self.service_exposure == ServiceExposure::PublicHttps
    }

    pub fn synthetic_jobs_enabled(&self) -> bool {
        self.synthetic_job_scheduler.is_some()
    }

    pub fn synthetic_job_source(&self) -> Option<SyntheticLocalJobSource> {
        self.synthetic_job_scheduler
            .clone()
            .map(SyntheticLocalJobSource::new)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeConfig {
    pub profile: ChainProfile,
    pub role: NodeRole,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub block_interval: Option<Duration>,
    pub local_synthetic_job_producer: bool,
    pub local_validator_block_proposer: bool,
    pub local_validator_block_proposer_delay_blocks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkConfig {
    pub rpc_listen: String,
    pub p2p_listen: String,
    pub identity_seed: Option<[u8; 32]>,
    pub auth_token: String,
    pub max_requests: usize,
}

impl NetworkConfig {
    pub fn new(rpc_listen: impl Into<String>, p2p_listen: impl Into<String>) -> Self {
        Self {
            rpc_listen: rpc_listen.into(),
            p2p_listen: p2p_listen.into(),
            identity_seed: None,
            auth_token: String::new(),
            max_requests: 0,
        }
    }

    pub fn with_identity_seed(mut self, identity_seed: Option<[u8; 32]>) -> Self {
        self.identity_seed = identity_seed;
        self
    }

    pub fn with_auth_token(mut self, auth_token: impl Into<String>) -> Self {
        self.auth_token = auth_token.into();
        self
    }

    pub fn with_max_requests(mut self, max_requests: usize) -> Self {
        self.max_requests = max_requests;
        self
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::new("127.0.0.1:8545", "127.0.0.1:0")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
}

impl StorageConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }
}

impl NodeConfig {
    pub fn new(profile: ChainProfile, role: NodeRole, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            profile,
            role,
            network: NetworkConfig::default(),
            storage: StorageConfig::new(data_dir),
            block_interval: None,
            local_synthetic_job_producer: false,
            local_validator_block_proposer: false,
            local_validator_block_proposer_delay_blocks: 0,
        }
    }

    pub fn build_chain(&self, finalized_randomness: Hash) -> Chain {
        self.profile.build_chain(finalized_randomness)
    }

    pub fn data_dir(&self) -> &Path {
        &self.storage.data_dir
    }

    pub fn with_network(mut self, network: NetworkConfig) -> Self {
        self.network = network;
        self
    }

    pub fn with_storage(mut self, storage: StorageConfig) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_block_interval(mut self, interval: Option<Duration>) -> Self {
        self.block_interval = interval;
        self
    }

    pub fn with_local_synthetic_job_producer(mut self, enabled: bool) -> Self {
        self.local_synthetic_job_producer = enabled;
        self
    }

    pub fn with_local_validator_block_proposer(mut self, enabled: bool) -> Self {
        self.local_validator_block_proposer = enabled;
        self
    }

    pub fn with_local_validator_block_proposer_delay_blocks(mut self, blocks: u64) -> Self {
        self.local_validator_block_proposer_delay_blocks = blocks;
        self
    }

    pub fn synthetic_block_interval(&self) -> Option<Duration> {
        self.local_runtime_enabled()
            .then(|| {
                self.profile
                    .synthetic_jobs_enabled()
                    .then_some(self.block_interval)
                    .flatten()
            })
            .flatten()
    }

    pub fn can_produce_local_blocks(&self) -> bool {
        self.local_runtime_enabled() && matches!(self.role, NodeRole::Validator)
    }

    pub fn local_block_proposer(&self) -> bool {
        self.local_validator_block_proposer && self.can_produce_local_blocks()
    }

    pub fn local_block_proposer_delay_satisfied(&self, height: u64) -> bool {
        height >= self.local_validator_block_proposer_delay_blocks
    }

    pub fn local_synthetic_producer(&self) -> bool {
        self.local_synthetic_job_producer
            && matches!(self.role, NodeRole::Validator)
            && self.synthetic_block_interval().is_some()
    }

    fn local_runtime_enabled(&self) -> bool {
        matches!(self.profile.network, ChainNetwork::Local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainEngine;
    use crate::scheduler::JobSource;
    use crate::types::hash_bytes;

    #[test]
    fn profiles_build_the_same_chain_engine_base() {
        let beacon = hash_bytes(b"test", &[b"profile-engine"]);
        let profiles = [
            ChainProfile::local_cpu(),
            ChainProfile::public_testnet(),
            ChainProfile::mainnet(),
        ];

        for profile in profiles {
            let chain = profile.build_chain(beacon);
            assert_eq!(chain.params(), &profile.chain_params);
            assert_eq!(chain.view().finalized_randomness(), beacon);
        }
    }

    #[test]
    fn local_cpu_profile_uses_non_unanimous_validator_committee_quorum() {
        let local = ChainProfile::local_cpu();
        let public = ChainProfile::public_testnet();

        assert_eq!(local.validator_count, 5);
        assert_eq!(local.chain_params.freivalds.validators_per_job, 5);
        assert_eq!(local.chain_params.freivalds.minimum_validators, 4);
        assert_eq!(
            local.chain_params.freivalds.minimum_stake_numerator,
            FreivaldsParams::default().minimum_stake_numerator
        );
        assert_eq!(
            local.chain_params.freivalds.minimum_stake_denominator,
            FreivaldsParams::default().minimum_stake_denominator
        );
        assert_eq!(
            public.chain_params.freivalds,
            FreivaldsParams::default(),
            "public profiles should keep the default wider committee policy"
        );
    }

    #[test]
    fn node_config_keeps_role_and_profile_separate_from_chain_state() {
        let profile = ChainProfile::local_cpu();
        let config = NodeConfig::new(profile.clone(), NodeRole::Miner, "local/miner-00");
        let chain = config.build_chain(hash_bytes(b"test", &[b"profile-node-config"]));

        assert_eq!(config.profile, profile);
        assert_eq!(config.role, NodeRole::Miner);
        assert_eq!(config.data_dir(), Path::new("local/miner-00"));
        assert_eq!(config.storage.data_dir, PathBuf::from("local/miner-00"));
        assert_eq!(config.network, NetworkConfig::default());
        assert_eq!(config.block_interval, None);
        assert!(!config.local_synthetic_job_producer);
        assert!(!config.local_validator_block_proposer);
        assert_eq!(config.local_validator_block_proposer_delay_blocks, 0);
        assert_eq!(chain.params(), &profile.chain_params);
        assert!(!profile.public_evidence_required);
        assert!(!profile.requires_public_services());
        assert!(ChainProfile::public_testnet().requires_public_services());
    }

    #[test]
    fn node_config_drives_local_runtime_policy_without_changing_chain_base() {
        let interval = Duration::from_millis(1000);
        let local_validator = NodeConfig::new(
            ChainProfile::from_label("local_cpu").unwrap(),
            NodeRole::Validator,
            "local/validator",
        )
        .with_network(
            NetworkConfig::new("127.0.0.1:9000", "/ip4/127.0.0.1/tcp/19000")
                .with_identity_seed(Some([7; 32]))
                .with_auth_token("secret")
                .with_max_requests(25),
        )
        .with_storage(StorageConfig::new("local/proposer-store"))
        .with_block_interval(Some(interval))
        .with_local_synthetic_job_producer(true)
        .with_local_validator_block_proposer(true)
        .with_local_validator_block_proposer_delay_blocks(20);
        let local_gateway = NodeConfig::new(
            ChainProfile::local_cpu(),
            NodeRole::Gateway,
            "local/gateway",
        )
        .with_block_interval(Some(interval))
        .with_local_synthetic_job_producer(true)
        .with_local_validator_block_proposer(true);
        let local_miner =
            NodeConfig::new(ChainProfile::local_cpu(), NodeRole::Miner, "local/miner")
                .with_block_interval(Some(interval))
                .with_local_synthetic_job_producer(true)
                .with_local_validator_block_proposer(true);
        let local_proposer = NodeConfig::new(
            ChainProfile::local_cpu(),
            NodeRole::Proposer,
            "local/proposer",
        )
        .with_block_interval(Some(interval))
        .with_local_synthetic_job_producer(true)
        .with_local_validator_block_proposer(true);
        let public_validator = NodeConfig::new(
            ChainProfile::public_testnet(),
            NodeRole::Validator,
            "testnet/validator",
        )
        .with_block_interval(Some(interval))
        .with_local_synthetic_job_producer(true)
        .with_local_validator_block_proposer(true);

        assert_eq!(local_validator.synthetic_block_interval(), Some(interval));
        assert!(local_validator.local_block_proposer());
        assert_eq!(
            local_validator.local_validator_block_proposer_delay_blocks,
            20
        );
        assert!(!local_validator.local_block_proposer_delay_satisfied(19));
        assert!(local_validator.local_block_proposer_delay_satisfied(20));
        assert!(local_validator.local_synthetic_producer());
        assert_eq!(
            local_validator.data_dir(),
            Path::new("local/proposer-store")
        );
        assert_eq!(local_validator.network.identity_seed, Some([7; 32]));
        assert_eq!(local_validator.network.auth_token, "secret");
        assert_eq!(local_validator.network.max_requests, 25);
        assert!(!local_gateway.can_produce_local_blocks());
        assert!(!local_gateway.local_block_proposer());
        assert!(!local_gateway.local_synthetic_producer());
        assert_eq!(local_miner.synthetic_block_interval(), Some(interval));
        assert!(!local_miner.can_produce_local_blocks());
        assert!(!local_miner.local_block_proposer());
        assert!(!local_miner.local_synthetic_producer());
        assert!(!local_proposer.can_produce_local_blocks());
        assert!(!local_proposer.local_block_proposer());
        assert!(!local_proposer.local_synthetic_producer());
        assert_eq!(public_validator.synthetic_block_interval(), None);
        assert!(!public_validator.can_produce_local_blocks());
        assert!(!public_validator.local_block_proposer());
        assert!(!public_validator.local_synthetic_producer());
        assert!(ChainProfile::from_label("staging").is_none());
    }

    #[test]
    fn validator_block_proposer_can_be_enabled_without_synthetic_job_publication() {
        let interval = Duration::from_millis(1000);
        let validator = NodeConfig::new(
            ChainProfile::local_cpu(),
            NodeRole::Validator,
            "local/validator",
        )
        .with_block_interval(Some(interval))
        .with_local_validator_block_proposer(true);

        assert!(validator.can_produce_local_blocks());
        assert!(validator.local_block_proposer());
        assert!(!validator.local_synthetic_producer());
    }

    #[test]
    fn profiles_configure_local_synthetic_jobs_without_changing_chain_base() {
        let mut local_source = ChainProfile::local_cpu()
            .synthetic_job_source()
            .expect("local profile should enable deterministic synthetic jobs");
        let public_profile = ChainProfile::public_testnet();
        let mainnet_profile = ChainProfile::mainnet();
        let beacon = hash_bytes(b"test", &[b"profile-synthetic-jobs"]);
        let local_chain = ChainProfile::local_cpu().build_chain(beacon);

        let Some(crate::chain::JobState::TensorOp(job)) = local_source.next_job(&local_chain)
        else {
            panic!("local synthetic source should emit a matmul job first");
        };

        assert_eq!((job.m, job.k, job.n), (8, 8, 8));
        assert!(public_profile.synthetic_job_source().is_none());
        assert!(mainnet_profile.synthetic_job_source().is_none());
        assert_eq!(
            public_profile.build_chain(beacon).params(),
            mainnet_profile.build_chain(beacon).params()
        );
        assert_eq!(ChainProfile::local_cpu().label(), "local_cpu");
        assert_eq!(public_profile.label(), "public_testnet");
        assert_eq!(mainnet_profile.label(), "mainnet");
    }
}
