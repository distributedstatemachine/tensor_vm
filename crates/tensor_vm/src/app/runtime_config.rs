use std::time::Duration;

use crate::{
    Chain, ChainNetwork, ChainProfile, NetworkConfig, NodeConfig, NodeRole,
    hash::hex,
    p2p::normalize_bootstrap_multiaddr,
    types::{Address, address},
};

use super::RandomnessBeaconRuntimeConfig;

#[derive(Clone, Copy, Debug)]
pub struct RoleServiceConfig<'a> {
    pub wallet: &'a str,
    pub device: Option<&'a str>,
    pub node: &'a str,
    pub listen: &'a str,
    pub p2p_listen: &'a str,
    pub data_dir: &'a str,
    pub identity_seed: Option<[u8; 32]>,
    pub auth_token: &'a str,
    pub max_requests: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRole {
    Service,
    Miner,
    Validator,
    Proposer,
}

impl RuntimeRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Miner => "miner",
            Self::Validator => "validator",
            Self::Proposer => "proposer",
        }
    }

    pub fn node_role(self) -> NodeRole {
        match self {
            Self::Service => NodeRole::Gateway,
            Self::Miner => NodeRole::Miner,
            Self::Validator => NodeRole::Validator,
            Self::Proposer => NodeRole::Proposer,
        }
    }
}

pub(super) fn runtime_chain_profile() -> std::result::Result<ChainProfile, String> {
    let label = std::env::var("TENSORVM_CHAIN_PROFILE").unwrap_or_else(|_| "local_cpu".to_owned());
    let mut profile = chain_profile_from_label(&label)?;
    if matches!(profile.network, crate::profile::ChainNetwork::Local) {
        profile.chain_params.proposer_cooldown_blocks = runtime_local_proposer_cooldown_blocks();
    }
    Ok(profile)
}

pub fn chain_profile_from_label(label: &str) -> std::result::Result<ChainProfile, String> {
    ChainProfile::from_label(label).ok_or_else(|| {
        format!(
            "unsupported TENSORVM_CHAIN_PROFILE {label:?}; expected local_cpu, public_testnet, or mainnet"
        )
    })
}

pub fn runtime_node_config(
    data_dir: &str,
    role: RuntimeRole,
    listen: &str,
    p2p_listen: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<NodeConfig, String> {
    let profile = runtime_chain_profile()?;
    let local_runtime = matches!(profile.network, ChainNetwork::Local);
    let mut config = NodeConfig::new(profile, role.node_role(), data_dir).with_network(
        NetworkConfig::new(listen, p2p_listen)
            .with_bootstrap_addresses(runtime_bootstrap_addresses()?)
            .with_identity_seed(identity_seed)
            .with_auth_token(auth_token)
            .with_max_requests(max_requests),
    );
    if local_runtime {
        config = config
            .with_block_interval(runtime_block_interval())
            .with_local_synthetic_job_producer(runtime_local_synthetic_job_producer())
            .with_local_validator_block_proposer(runtime_local_validator_block_proposer())
            .with_local_validator_block_proposer_delay_blocks(
                runtime_local_validator_block_proposer_delay_blocks(),
            );
    }
    Ok(config)
}

#[derive(Debug)]
pub struct ServiceRuntimeConfig {
    pub runtime_command: &'static str,
    pub role: RuntimeRole,
    pub role_wallet_address: Option<Address>,
    pub role_wallet_secret: Option<String>,
    pub miner_device: Option<String>,
    pub node: NodeConfig,
    pub randomness_beacon: RandomnessBeaconRuntimeConfig,
}

impl ServiceRuntimeConfig {
    pub fn randomness_beacon_from_env(mut self) -> std::result::Result<Self, String> {
        self.randomness_beacon = RandomnessBeaconRuntimeConfig::from_env()?;
        Ok(self)
    }
}

pub fn role_wallet_address(wallet: &str) -> std::result::Result<Address, String> {
    let wallet = wallet.trim();
    if wallet.is_empty() {
        return Err("wallet argument is empty".to_owned());
    }
    Ok(address(wallet.as_bytes()))
}

pub fn runtime_role_wallet_address_text(address: Option<Address>) -> String {
    address
        .map(|address| hex(&address))
        .unwrap_or_else(|| "none".to_owned())
}

pub fn runtime_role_wallet_registration(
    role: RuntimeRole,
    address: Option<Address>,
    chain: &Chain,
) -> &'static str {
    let Some(address) = address else {
        return "none";
    };
    match role {
        RuntimeRole::Miner => {
            if chain.state().miners().contains_key(&address) {
                "miner"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Validator => {
            if chain.state().validators().contains_key(&address) {
                "validator"
            } else {
                "unregistered"
            }
        }
        RuntimeRole::Proposer if chain.state().validators().contains_key(&address) => "validator",
        RuntimeRole::Proposer => "unregistered",
        RuntimeRole::Service => "none",
    }
}

pub fn runtime_role_wallet_registered(
    role: RuntimeRole,
    address: Option<Address>,
    chain: &Chain,
) -> bool {
    !matches!(
        runtime_role_wallet_registration(role, address, chain),
        "none" | "unregistered"
    )
}

fn runtime_block_interval() -> Option<Duration> {
    std::env::var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
}

fn runtime_bool_env(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

pub(crate) fn runtime_bootstrap_addresses() -> std::result::Result<Vec<String>, String> {
    let Some(value) = std::env::var("TENSORVM_BOOTSTRAP_PEERS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|address| !address.is_empty())
        .map(|address| {
            normalize_bootstrap_multiaddr(address)
                .map_err(|error| format!("invalid TENSORVM_BOOTSTRAP_PEERS address: {error}"))
        })
        .collect()
}

fn runtime_local_synthetic_job_producer() -> bool {
    runtime_bool_env("TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER")
}

fn runtime_local_validator_block_proposer() -> bool {
    runtime_bool_env("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER")
}

fn runtime_local_validator_block_proposer_delay_blocks() -> u64 {
    std::env::var("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn runtime_local_proposer_cooldown_blocks() -> u64 {
    std::env::var("TENSORVM_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const RUNTIME_ENV: &[&str] = &[
        "TENSORVM_CHAIN_PROFILE",
        "TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS",
        "TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER",
        "TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER",
        "TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS",
        "TENSORVM_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS",
        "TENSORVM_BOOTSTRAP_PEERS",
    ];

    struct RuntimeEnvGuard {
        _lock: MutexGuard<'static, ()>,
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl RuntimeEnvGuard {
        fn new() -> Self {
            let lock = ENV_LOCK.lock().expect("runtime env test lock poisoned");
            let saved = RUNTIME_ENV
                .iter()
                .map(|name| (*name, std::env::var(name).ok()))
                .collect();
            unsafe {
                for name in RUNTIME_ENV {
                    std::env::remove_var(name);
                }
            }
            Self { _lock: lock, saved }
        }
    }

    impl Drop for RuntimeEnvGuard {
        fn drop(&mut self) {
            unsafe {
                for name in RUNTIME_ENV {
                    std::env::remove_var(name);
                }
                for (name, value) in &self.saved {
                    if let Some(value) = value {
                        std::env::set_var(name, value);
                    }
                }
            }
        }
    }

    fn runtime_validator_config() -> NodeConfig {
        runtime_node_config(
            "runtime-profile-test",
            RuntimeRole::Validator,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            None,
            "token",
            3,
        )
        .expect("runtime node config should build")
    }

    fn bootstrap_peer_address() -> String {
        format!("/ip4/127.0.0.1/tcp/4001/p2p/{}", libp2p::PeerId::random())
    }

    #[test]
    fn public_runtime_profiles_ignore_local_cpu_production_env_knobs() {
        let _env = RuntimeEnvGuard::new();
        unsafe {
            std::env::set_var("TENSORVM_CHAIN_PROFILE", "public_testnet");
            std::env::set_var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25");
            std::env::set_var("TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER", "true");
            std::env::set_var("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER", "true");
            std::env::set_var(
                "TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS",
                "7",
            );
            std::env::set_var("TENSORVM_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS", "9");
        }

        let config = runtime_validator_config();

        assert_eq!(config.profile.label(), "public_testnet");
        assert!(config.profile.requires_public_services());
        assert_eq!(config.profile.chain_params.proposer_cooldown_blocks, 0);
        assert_eq!(config.block_interval, None);
        assert!(!config.local_synthetic_job_producer);
        assert!(!config.local_validator_block_proposer);
        assert_eq!(config.local_validator_block_proposer_delay_blocks, 0);
        assert!(!config.can_produce_local_blocks());
        assert!(!config.local_synthetic_producer());
        assert!(!config.local_block_proposer());
    }

    #[test]
    fn local_runtime_profile_honors_local_cpu_production_env_knobs() {
        let _env = RuntimeEnvGuard::new();
        unsafe {
            std::env::set_var("TENSORVM_CHAIN_PROFILE", "local_cpu");
            std::env::set_var("TENSORVM_LOCAL_CPU_BLOCK_INTERVAL_MS", "25");
            std::env::set_var("TENSORVM_LOCAL_CPU_SYNTHETIC_JOB_PRODUCER", "true");
            std::env::set_var("TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER", "true");
            std::env::set_var(
                "TENSORVM_LOCAL_CPU_VALIDATOR_BLOCK_PROPOSER_DELAY_BLOCKS",
                "7",
            );
            std::env::set_var("TENSORVM_LOCAL_CPU_PROPOSER_COOLDOWN_BLOCKS", "9");
        }

        let config = runtime_validator_config();

        assert_eq!(config.profile.label(), "local_cpu");
        assert_eq!(config.profile.chain_params.proposer_cooldown_blocks, 9);
        assert_eq!(config.block_interval, Some(Duration::from_millis(25)));
        assert!(config.local_synthetic_job_producer);
        assert!(config.local_validator_block_proposer);
        assert_eq!(config.local_validator_block_proposer_delay_blocks, 7);
        assert!(config.can_produce_local_blocks());
        assert!(config.local_synthetic_producer());
        assert!(config.local_block_proposer());
    }

    #[test]
    fn runtime_node_config_loads_valid_bootstrap_peer_env() {
        let _env = RuntimeEnvGuard::new();
        let bootstrap = bootstrap_peer_address();
        unsafe {
            std::env::set_var("TENSORVM_BOOTSTRAP_PEERS", &bootstrap);
        }

        let config = runtime_validator_config();

        assert_eq!(config.network.bootstrap_addresses, vec![bootstrap]);
    }

    #[test]
    fn runtime_node_config_rejects_malformed_bootstrap_peer_env() {
        let _env = RuntimeEnvGuard::new();
        unsafe {
            std::env::set_var("TENSORVM_BOOTSTRAP_PEERS", "/ip4/127.0.0.1/tcp/4001");
        }

        let error = runtime_node_config(
            "runtime-profile-test",
            RuntimeRole::Validator,
            "127.0.0.1:0",
            "/ip4/127.0.0.1/tcp/0",
            None,
            "token",
            3,
        )
        .expect_err("missing peer id must reject bootstrap env");

        assert!(error.contains("invalid TENSORVM_BOOTSTRAP_PEERS address"));
        assert!(error.contains("bootstrap address missing peer id"));
    }
}
