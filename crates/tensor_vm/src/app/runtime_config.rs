use std::time::Duration;

use crate::{
    Chain, ChainProfile, NetworkConfig, NodeConfig, NodeRole,
    hash::hex,
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

fn runtime_chain_profile() -> std::result::Result<ChainProfile, String> {
    let label = std::env::var("TENSORVM_CHAIN_PROFILE").unwrap_or_else(|_| "local_cpu".to_owned());
    chain_profile_from_label(&label)
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
    Ok(
        NodeConfig::new(runtime_chain_profile()?, role.node_role(), data_dir)
            .with_network(
                NetworkConfig::new(listen, p2p_listen)
                    .with_identity_seed(identity_seed)
                    .with_auth_token(auth_token)
                    .with_max_requests(max_requests),
            )
            .with_block_interval(runtime_block_interval())
            .with_local_producer(runtime_local_block_producer()),
    )
}

#[derive(Debug)]
pub struct ServiceRuntimeConfig {
    pub runtime_command: &'static str,
    pub role: RuntimeRole,
    pub role_wallet_address: Option<Address>,
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

fn runtime_local_block_producer() -> bool {
    match std::env::var("TENSORVM_LOCAL_CPU_ROLE_PRODUCER") {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}
