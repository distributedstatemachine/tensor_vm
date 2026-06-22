use crate::{
    ChainCommand, ChainEngine, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService,
    api::P2pMessage,
    chain::{
        ExternalRandomnessBeaconProof, ExternalRandomnessBeaconRecord,
        verified_chained_drand_beacon_record, verified_chained_drand_source_id,
        verified_drand_beacon_record, verified_drand_source_id,
    },
    hash::hex,
    p2p::{
        encode_external_randomness_beacon_payload, encode_verified_chained_drand_beacon_payload,
        encode_verified_drand_beacon_payload,
    },
    types::{Hash, hash_bytes, parse_hash_hex},
};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL: &str = "https://api.drand.sh/v2";
pub const PUBLIC_DRAND_DEFAULT_CHAIN_HASH: &str =
    "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce";
const PUBLIC_DRAND_DEFAULT_TIMEOUT_MS: u64 = 5_000;
const PUBLIC_DRAND_DEFAULT_POLL_INTERVAL_TICKS: u64 = 1_200;
const PUBLIC_DRAND_DEFAULT_FAILURE_BACKOFF_MAX_TICKS: u64 = 9_600;
const PUBLIC_DRAND_DEFAULT_MAX_ROUND_LAG: u64 = 2;
const PUBLIC_DRAND_CHAINED_SCHEME: &str = "pedersen-bls-chained";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomnessBeaconMode {
    Off,
    LocalDeterministic,
    VerifiedDrand,
    PublicDrand,
}

impl RandomnessBeaconMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::LocalDeterministic => "local_deterministic",
            Self::VerifiedDrand => "verified_drand",
            Self::PublicDrand => "public_drand",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomnessBeaconRuntimeConfig {
    pub mode: RandomnessBeaconMode,
    pub source_id: String,
    pub beacon_round: u64,
    pub randomness: Hash,
    pub proof_hash: Hash,
    pub drand_public_key: Vec<u8>,
    pub drand_signature: Vec<u8>,
    pub drand_previous_signature: Vec<u8>,
    pub drand_http_base_url: String,
    pub drand_chain_hash: String,
    pub drand_fetch_timeout_ms: u64,
    pub drand_poll_interval_ticks: u64,
    pub drand_failure_backoff_max_ticks: u64,
    pub drand_genesis_time: u64,
    pub drand_period: u64,
    pub drand_expected_latest_round: u64,
    pub drand_round_lag: u64,
    pub drand_max_round_lag: u64,
}

impl RandomnessBeaconRuntimeConfig {
    pub fn off() -> Self {
        Self {
            mode: RandomnessBeaconMode::Off,
            source_id: String::new(),
            beacon_round: 0,
            randomness: [0; 32],
            proof_hash: [0; 32],
            drand_public_key: Vec::new(),
            drand_signature: Vec::new(),
            drand_previous_signature: Vec::new(),
            drand_http_base_url: String::new(),
            drand_chain_hash: String::new(),
            drand_fetch_timeout_ms: 0,
            drand_poll_interval_ticks: 0,
            drand_failure_backoff_max_ticks: 0,
            drand_genesis_time: 0,
            drand_period: 0,
            drand_expected_latest_round: 0,
            drand_round_lag: 0,
            drand_max_round_lag: 0,
        }
    }

    pub fn local_deterministic(source_id: impl Into<String>, beacon_round: u64) -> Self {
        let source_id = source_id.into();
        let round = beacon_round.to_le_bytes();
        let randomness = hash_bytes(
            b"tensor-vm-local-drand-fixture-randomness-v1",
            &[source_id.as_bytes(), &round],
        );
        let proof_hash = hash_bytes(
            b"tensor-vm-local-drand-fixture-proof-v1",
            &[source_id.as_bytes(), &round, &randomness],
        );
        Self {
            mode: RandomnessBeaconMode::LocalDeterministic,
            source_id,
            beacon_round,
            randomness,
            proof_hash,
            drand_public_key: Vec::new(),
            drand_signature: Vec::new(),
            drand_previous_signature: Vec::new(),
            drand_http_base_url: String::new(),
            drand_chain_hash: String::new(),
            drand_fetch_timeout_ms: 0,
            drand_poll_interval_ticks: 0,
            drand_failure_backoff_max_ticks: 0,
            drand_genesis_time: 0,
            drand_period: 0,
            drand_expected_latest_round: 0,
            drand_round_lag: 0,
            drand_max_round_lag: 0,
        }
    }

    pub fn verified_drand(
        beacon_round: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    ) -> std::result::Result<Self, String> {
        if beacon_round == 0 {
            return Err("verified drand beacon round must be greater than zero".to_owned());
        }
        let source_id = verified_drand_source_id(&public_key);
        Self::verified_drand_with_source(source_id, beacon_round, public_key, signature)
    }

    pub fn verified_drand_with_source(
        source_id: String,
        beacon_round: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    ) -> std::result::Result<Self, String> {
        if beacon_round == 0 {
            return Err("verified drand beacon round must be greater than zero".to_owned());
        }
        let expected_source_id = verified_drand_source_id(&public_key);
        if source_id != expected_source_id {
            return Err(format!(
                "verified drand source id must equal public key hash source {expected_source_id}"
            ));
        }
        let record = verified_drand_beacon_record(
            source_id.clone(),
            beacon_round,
            &public_key,
            &signature,
            0,
        )
        .map_err(|error| format!("invalid verified drand beacon config: {error}"))?;
        Ok(Self {
            mode: RandomnessBeaconMode::VerifiedDrand,
            source_id,
            beacon_round,
            randomness: record.randomness,
            proof_hash: record.proof_hash,
            drand_public_key: public_key,
            drand_signature: signature,
            drand_previous_signature: Vec::new(),
            drand_http_base_url: String::new(),
            drand_chain_hash: String::new(),
            drand_fetch_timeout_ms: 0,
            drand_poll_interval_ticks: 0,
            drand_failure_backoff_max_ticks: 0,
            drand_genesis_time: 0,
            drand_period: 0,
            drand_expected_latest_round: 0,
            drand_round_lag: 0,
            drand_max_round_lag: 0,
        })
    }

    pub fn verified_chained_drand(
        beacon_round: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
        previous_signature: Vec<u8>,
    ) -> std::result::Result<Self, String> {
        if beacon_round == 0 {
            return Err("verified chained drand beacon round must be greater than zero".to_owned());
        }
        let source_id = verified_chained_drand_source_id(&public_key);
        Self::verified_chained_drand_with_source(
            source_id,
            beacon_round,
            public_key,
            signature,
            previous_signature,
        )
    }

    pub fn verified_chained_drand_with_source(
        source_id: String,
        beacon_round: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
        previous_signature: Vec<u8>,
    ) -> std::result::Result<Self, String> {
        if beacon_round == 0 {
            return Err("verified chained drand beacon round must be greater than zero".to_owned());
        }
        let expected_source_id = verified_chained_drand_source_id(&public_key);
        if source_id != expected_source_id {
            return Err(format!(
                "verified chained drand source id must equal public key hash source {expected_source_id}"
            ));
        }
        let record = verified_chained_drand_beacon_record(
            source_id.clone(),
            beacon_round,
            &public_key,
            &signature,
            &previous_signature,
            0,
        )
        .map_err(|error| format!("invalid verified chained drand beacon config: {error}"))?;
        Ok(Self {
            mode: RandomnessBeaconMode::PublicDrand,
            source_id,
            beacon_round,
            randomness: record.randomness,
            proof_hash: record.proof_hash,
            drand_public_key: public_key,
            drand_signature: signature,
            drand_previous_signature: previous_signature,
            drand_http_base_url: String::new(),
            drand_chain_hash: String::new(),
            drand_fetch_timeout_ms: 0,
            drand_poll_interval_ticks: 0,
            drand_failure_backoff_max_ticks: 0,
            drand_genesis_time: 0,
            drand_period: 0,
            drand_expected_latest_round: 0,
            drand_round_lag: 0,
            drand_max_round_lag: 0,
        })
    }

    pub fn public_drand(
        http_base_url: impl Into<String>,
        chain_hash: impl Into<String>,
        fetch_timeout_ms: u64,
    ) -> std::result::Result<Self, String> {
        let http_base_url = normalize_drand_http_base_url(&http_base_url.into())?;
        let chain_hash = chain_hash.into();
        if parse_env_hex_bytes("TENSORVM_RANDOMNESS_BEACON_DRAND_CHAIN_HASH", &chain_hash)?.len()
            != 32
        {
            return Err("TENSORVM_RANDOMNESS_BEACON_DRAND_CHAIN_HASH must be 32 bytes".to_owned());
        }
        if fetch_timeout_ms == 0 {
            return Err(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_FETCH_TIMEOUT_MS must be greater than zero"
                    .to_owned(),
            );
        }
        Ok(Self {
            mode: RandomnessBeaconMode::PublicDrand,
            source_id: format!("public-drand:{}", &chain_hash[..16]),
            beacon_round: 0,
            randomness: [0; 32],
            proof_hash: [0; 32],
            drand_public_key: Vec::new(),
            drand_signature: Vec::new(),
            drand_previous_signature: Vec::new(),
            drand_http_base_url: http_base_url,
            drand_chain_hash: chain_hash,
            drand_fetch_timeout_ms: fetch_timeout_ms,
            drand_poll_interval_ticks: PUBLIC_DRAND_DEFAULT_POLL_INTERVAL_TICKS,
            drand_failure_backoff_max_ticks: PUBLIC_DRAND_DEFAULT_FAILURE_BACKOFF_MAX_TICKS,
            drand_genesis_time: 0,
            drand_period: 0,
            drand_expected_latest_round: 0,
            drand_round_lag: 0,
            drand_max_round_lag: PUBLIC_DRAND_DEFAULT_MAX_ROUND_LAG,
        })
    }

    pub fn from_env() -> std::result::Result<Self, String> {
        let mode =
            std::env::var("TENSORVM_RANDOMNESS_BEACON_MODE").unwrap_or_else(|_| "off".to_owned());
        match mode.as_str() {
            "" | "off" | "OFF" | "disabled" | "DISABLED" => Ok(Self::off()),
            "local_deterministic" => {
                let source_id = std::env::var("TENSORVM_RANDOMNESS_BEACON_SOURCE_ID")
                    .unwrap_or_else(|_| "local_drand_fixture_v1".to_owned());
                if source_id.trim().is_empty() {
                    return Err(
                        "TENSORVM_RANDOMNESS_BEACON_SOURCE_ID must not be empty in local_deterministic mode"
                            .to_owned(),
                    );
                }
                let beacon_round = std::env::var("TENSORVM_RANDOMNESS_BEACON_ROUND")
                    .map_err(|_| {
                        "TENSORVM_RANDOMNESS_BEACON_ROUND is required for local_deterministic mode"
                            .to_owned()
                    })?
                    .parse::<u64>()
                    .map_err(|error| {
                        format!("invalid TENSORVM_RANDOMNESS_BEACON_ROUND: {error}")
                    })?;
                if beacon_round == 0 {
                    return Err(
                        "TENSORVM_RANDOMNESS_BEACON_ROUND must be greater than zero in local_deterministic mode"
                            .to_owned(),
                    );
                }
                let mut config = Self::local_deterministic(source_id, beacon_round);
                if let Ok(randomness) = std::env::var("TENSORVM_RANDOMNESS_BEACON_RANDOMNESS") {
                    config.randomness =
                        parse_env_hash("TENSORVM_RANDOMNESS_BEACON_RANDOMNESS", &randomness)?;
                }
                if let Ok(proof_hash) = std::env::var("TENSORVM_RANDOMNESS_BEACON_PROOF_HASH") {
                    config.proof_hash =
                        parse_env_hash("TENSORVM_RANDOMNESS_BEACON_PROOF_HASH", &proof_hash)?;
                }
                Ok(config)
            }
            "verified_drand" => {
                let beacon_round = std::env::var("TENSORVM_RANDOMNESS_BEACON_ROUND")
                    .map_err(|_| {
                        "TENSORVM_RANDOMNESS_BEACON_ROUND is required for verified_drand mode"
                            .to_owned()
                    })?
                    .parse::<u64>()
                    .map_err(|error| {
                        format!("invalid TENSORVM_RANDOMNESS_BEACON_ROUND: {error}")
                    })?;
                let public_key = parse_env_hex_bytes(
                    "TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX",
                    &std::env::var("TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX").map_err(
                        |_| {
                            "TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX is required for verified_drand mode"
                                .to_owned()
                        },
                    )?,
                )?;
                let signature = parse_env_hex_bytes(
                    "TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX",
                    &std::env::var("TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX").map_err(
                        |_| {
                            "TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX is required for verified_drand mode"
                                .to_owned()
                        },
                    )?,
                )?;
                let expected_source_id = verified_drand_source_id(&public_key);
                let source_id = std::env::var("TENSORVM_RANDOMNESS_BEACON_SOURCE_ID")
                    .unwrap_or(expected_source_id.clone());
                if source_id != expected_source_id {
                    return Err(format!(
                        "TENSORVM_RANDOMNESS_BEACON_SOURCE_ID must equal {expected_source_id} in verified_drand mode"
                    ));
                }
                Self::verified_drand_with_source(source_id, beacon_round, public_key, signature)
            }
            "public_drand" => {
                let http_base_url = std::env::var("TENSORVM_RANDOMNESS_BEACON_DRAND_HTTP_BASE_URL")
                    .unwrap_or_else(|_| PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL.to_owned());
                let chain_hash = std::env::var("TENSORVM_RANDOMNESS_BEACON_DRAND_CHAIN_HASH")
                    .unwrap_or_else(|_| PUBLIC_DRAND_DEFAULT_CHAIN_HASH.to_owned());
                let fetch_timeout_ms =
                    match std::env::var("TENSORVM_RANDOMNESS_BEACON_DRAND_FETCH_TIMEOUT_MS") {
                        Ok(value) => value.parse::<u64>().map_err(|error| {
                            format!(
                                "invalid TENSORVM_RANDOMNESS_BEACON_DRAND_FETCH_TIMEOUT_MS: {error}"
                            )
                        })?,
                        Err(_) => PUBLIC_DRAND_DEFAULT_TIMEOUT_MS,
                    };
                let mut config = Self::public_drand(http_base_url, chain_hash, fetch_timeout_ms)?;
                config.drand_poll_interval_ticks = parse_optional_positive_env_u64(
                    "TENSORVM_RANDOMNESS_BEACON_DRAND_POLL_INTERVAL_TICKS",
                    PUBLIC_DRAND_DEFAULT_POLL_INTERVAL_TICKS,
                )?;
                config.drand_failure_backoff_max_ticks = parse_optional_positive_env_u64(
                    "TENSORVM_RANDOMNESS_BEACON_DRAND_FAILURE_BACKOFF_MAX_TICKS",
                    PUBLIC_DRAND_DEFAULT_FAILURE_BACKOFF_MAX_TICKS,
                )?;
                if config.drand_failure_backoff_max_ticks < config.drand_poll_interval_ticks {
                    return Err(
                        "TENSORVM_RANDOMNESS_BEACON_DRAND_FAILURE_BACKOFF_MAX_TICKS must be greater than or equal to TENSORVM_RANDOMNESS_BEACON_DRAND_POLL_INTERVAL_TICKS"
                            .to_owned(),
                    );
                }
                config.drand_max_round_lag = parse_optional_env_u64(
                    "TENSORVM_RANDOMNESS_BEACON_DRAND_MAX_ROUND_LAG",
                    PUBLIC_DRAND_DEFAULT_MAX_ROUND_LAG,
                )?;
                Ok(config)
            }
            other => Err(format!(
                "unsupported TENSORVM_RANDOMNESS_BEACON_MODE {other:?}; expected off, local_deterministic, verified_drand, or public_drand"
            )),
        }
    }

    pub fn enabled(&self) -> bool {
        self.mode != RandomnessBeaconMode::Off
    }
}

fn parse_optional_positive_env_u64(name: &str, default: u64) -> std::result::Result<u64, String> {
    match std::env::var(name) {
        Ok(value) => {
            let parsed = value
                .parse::<u64>()
                .map_err(|error| format!("invalid {name}: {error}"))?;
            if parsed == 0 {
                return Err(format!("{name} must be greater than zero"));
            }
            Ok(parsed)
        }
        Err(_) => Ok(default),
    }
}

fn parse_optional_env_u64(name: &str, default: u64) -> std::result::Result<u64, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| format!("invalid {name}: {error}")),
        Err(_) => Ok(default),
    }
}

fn parse_env_hash(name: &str, value: &str) -> std::result::Result<Hash, String> {
    parse_hash_hex(value).map_err(|error| format!("invalid {name}: {error:?}"))
}

fn parse_env_hex_bytes(name: &str, value: &str) -> std::result::Result<Vec<u8>, String> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return Err(format!("invalid {name}: odd-length hex"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .ok_or_else(|| format!("invalid {name}: non-hex byte"))?;
            let low = (pair[1] as char)
                .to_digit(16)
                .ok_or_else(|| format!("invalid {name}: non-hex byte"))?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

fn normalize_drand_http_base_url(value: &str) -> std::result::Result<String, String> {
    let value = value.trim_end_matches('/');
    if value.trim() != value || value.is_empty() {
        return Err("drand HTTP base URL must not be empty or whitespace padded".to_owned());
    }
    if !value.starts_with("https://") && !value.starts_with("http://127.0.0.1") {
        return Err("drand HTTP base URL must be https or local test loopback".to_owned());
    }
    Ok(value.to_owned())
}

#[derive(Debug, Deserialize)]
struct DrandChainInfoResponse {
    public_key: String,
    period: u64,
    genesis_time: u64,
    chain_hash: String,
    scheme: String,
}

#[derive(Debug, Deserialize)]
struct DrandRoundResponse {
    round: u64,
    signature: String,
    previous_signature: Option<String>,
}

pub fn drand_round_for_unix_time(
    genesis_time: u64,
    period: u64,
    unix_time: u64,
) -> std::result::Result<u64, String> {
    if period == 0 {
        return Err("drand period must be greater than zero".to_owned());
    }
    if unix_time < genesis_time {
        return Err("unix time is before drand genesis time".to_owned());
    }
    Ok(((unix_time - genesis_time) / period).saturating_add(1))
}

pub fn drand_rounds_per_chain_epoch(
    drand_period: u64,
    block_time_seconds: u64,
    epoch_length: u64,
) -> std::result::Result<u64, String> {
    if drand_period == 0 {
        return Err("drand period must be greater than zero".to_owned());
    }
    let epoch_seconds = block_time_seconds
        .max(1)
        .saturating_mul(epoch_length.max(1));
    Ok(epoch_seconds.saturating_add(drand_period.saturating_sub(1)) / drand_period)
}

#[cfg(test)]
fn public_drand_config_from_json(
    expected_chain_hash: &str,
    info_json: &str,
    round_json: &str,
) -> std::result::Result<RandomnessBeaconRuntimeConfig, String> {
    public_drand_config_from_json_at_time(expected_chain_hash, info_json, round_json, None)
}

fn public_drand_config_from_json_at_time(
    expected_chain_hash: &str,
    info_json: &str,
    round_json: &str,
    observed_unix_time: Option<u64>,
) -> std::result::Result<RandomnessBeaconRuntimeConfig, String> {
    let info: DrandChainInfoResponse = serde_json::from_str(info_json)
        .map_err(|error| format!("invalid drand info response JSON: {error}"))?;
    if info.chain_hash != expected_chain_hash {
        return Err(format!(
            "drand chain hash mismatch: expected {expected_chain_hash}, got {}",
            info.chain_hash
        ));
    }
    if info.scheme != PUBLIC_DRAND_CHAINED_SCHEME {
        return Err(format!(
            "unsupported drand scheme {:?}; expected {PUBLIC_DRAND_CHAINED_SCHEME}",
            info.scheme
        ));
    }
    drand_round_for_unix_time(info.genesis_time, info.period, info.genesis_time)?;
    let round: DrandRoundResponse = serde_json::from_str(round_json)
        .map_err(|error| format!("invalid drand round response JSON: {error}"))?;
    let public_key = parse_env_hex_bytes("drand public_key", &info.public_key)?;
    let signature = parse_env_hex_bytes("drand signature", &round.signature)?;
    let previous_signature = parse_env_hex_bytes(
        "drand previous_signature",
        round
            .previous_signature
            .as_deref()
            .ok_or_else(|| "drand chained response missing previous_signature".to_owned())?,
    )?;
    let mut config = RandomnessBeaconRuntimeConfig::verified_chained_drand(
        round.round,
        public_key,
        signature,
        previous_signature,
    )?;
    config.drand_genesis_time = info.genesis_time;
    config.drand_period = info.period;
    if let Some(unix_time) = observed_unix_time {
        config.drand_expected_latest_round =
            drand_round_for_unix_time(info.genesis_time, info.period, unix_time)?;
        config.drand_round_lag = config
            .drand_expected_latest_round
            .saturating_sub(config.beacon_round);
    }
    Ok(config)
}

pub trait DrandBeaconClient {
    fn fetch_latest_chained(
        &self,
        config: &RandomnessBeaconRuntimeConfig,
    ) -> std::result::Result<RandomnessBeaconRuntimeConfig, String>;
}

pub struct HttpDrandBeaconClient;

impl DrandBeaconClient for HttpDrandBeaconClient {
    fn fetch_latest_chained(
        &self,
        config: &RandomnessBeaconRuntimeConfig,
    ) -> std::result::Result<RandomnessBeaconRuntimeConfig, String> {
        let timeout = Duration::from_millis(config.drand_fetch_timeout_ms);
        let info_url = format!(
            "{}/chains/{}/info",
            config.drand_http_base_url, config.drand_chain_hash
        );
        let round_url = format!(
            "{}/chains/{}/rounds/latest",
            config.drand_http_base_url, config.drand_chain_hash
        );
        let info_json = ureq::get(&info_url)
            .timeout(timeout)
            .call()
            .map_err(|error| format!("failed to fetch drand chain info: {error}"))?
            .into_string()
            .map_err(|error| format!("failed to read drand chain info response: {error}"))?;
        let round_json = ureq::get(&round_url)
            .timeout(timeout)
            .call()
            .map_err(|error| format!("failed to fetch drand latest round: {error}"))?
            .into_string()
            .map_err(|error| format!("failed to read drand latest round response: {error}"))?;
        let observed_unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before UNIX epoch: {error}"))?
            .as_secs();
        let mut fetched = public_drand_config_from_json_at_time(
            &config.drand_chain_hash,
            &info_json,
            &round_json,
            Some(observed_unix_time),
        )?;
        fetched.drand_http_base_url = config.drand_http_base_url.clone();
        fetched.drand_chain_hash = config.drand_chain_hash.clone();
        fetched.drand_fetch_timeout_ms = config.drand_fetch_timeout_ms;
        fetched.drand_poll_interval_ticks = config.drand_poll_interval_ticks;
        fetched.drand_failure_backoff_max_ticks = config.drand_failure_backoff_max_ticks;
        fetched.drand_max_round_lag = config.drand_max_round_lag;
        Ok(fetched)
    }
}

pub fn tick_randomness_beacon_once(
    config: &RandomnessBeaconRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    tick_randomness_beacon_once_with_client(
        config,
        store,
        server,
        p2p_service,
        runtime_state,
        &HttpDrandBeaconClient,
    )
}

pub fn tick_randomness_beacon_once_with_client(
    config: &RandomnessBeaconRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
    drand_client: &dyn DrandBeaconClient,
) -> std::result::Result<bool, String> {
    if !config.enabled() {
        return Ok(false);
    }
    let fetched_config;
    let public_drand_poll =
        config.mode == RandomnessBeaconMode::PublicDrand && config.beacon_round == 0;
    let config = if config.mode == RandomnessBeaconMode::PublicDrand && config.beacon_round == 0 {
        if !runtime_state.randomness_public_drand_poll_due() {
            return Ok(false);
        }
        runtime_state.record_randomness_public_drand_fetch_attempt();
        fetched_config = match drand_client.fetch_latest_chained(config) {
            Ok(config) => config,
            Err(error) => {
                runtime_state.record_randomness_beacon_failure(
                    &config.source_id,
                    config.beacon_round,
                    &error,
                );
                runtime_state.record_randomness_public_drand_fetch_failure(
                    config.drand_poll_interval_ticks,
                    config.drand_failure_backoff_max_ticks,
                );
                return Ok(true);
            }
        };
        &fetched_config
    } else {
        config
    };
    if !public_drand_poll
        && runtime_state.randomness_latest_source_id() == config.source_id
        && runtime_state.randomness_latest_round() == config.beacon_round
        && runtime_state.randomness_beacons_observed() > 0
    {
        if !runtime_state.randomness_beacon_published(&config.source_id, config.beacon_round)
            && p2p_service.connected_peer_count() > 0
            && server
                .gateway()
                .node
                .chain
                .state()
                .external_randomness_beacons()
                .contains_key(&config.beacon_round)
        {
            publish_external_randomness_beacon(p2p_service, config).map_err(|error| {
                runtime_state.record_randomness_beacon_failure(
                    &config.source_id,
                    config.beacon_round,
                    &error,
                );
                error
            })?;
            runtime_state
                .record_randomness_beacon_published(&config.source_id, config.beacon_round);
            return Ok(true);
        }
        return Ok(false);
    }
    runtime_state.record_randomness_beacon_observed(&config.source_id, config.beacon_round);
    let chain = &mut server.gateway_mut().node.chain;
    if public_drand_poll {
        let rounds_per_chain_epoch = drand_rounds_per_chain_epoch(
            config.drand_period,
            chain.params().block_time_seconds,
            chain.params().epoch_length,
        )
        .unwrap_or_default();
        let chain_epoch = chain
            .state()
            .height()
            .checked_div(chain.params().epoch_length.max(1))
            .unwrap_or_default();
        runtime_state.record_randomness_public_drand_mapping_observation(
            config.drand_expected_latest_round,
            config.drand_round_lag,
            config.drand_max_round_lag,
            rounds_per_chain_epoch,
            chain_epoch,
        );
        if config.drand_expected_latest_round > 0
            && config.drand_round_lag > config.drand_max_round_lag
        {
            runtime_state.record_randomness_beacon_skipped(&config.source_id, config.beacon_round);
            runtime_state
                .record_randomness_public_drand_fetch_stale(config.drand_poll_interval_ticks);
            return Ok(true);
        }
    }
    if public_drand_poll && config.beacon_round <= chain.state().finalized_beacon_round() {
        runtime_state.record_randomness_beacon_skipped(&config.source_id, config.beacon_round);
        runtime_state.record_randomness_public_drand_fetch_stale(config.drand_poll_interval_ticks);
        return Ok(true);
    }
    if let Some(record) = chain
        .state()
        .external_randomness_beacons()
        .get(&config.beacon_round)
    {
        if external_randomness_beacon_matches_config(record, config) {
            runtime_state.record_randomness_beacon_applied(&config.source_id, config.beacon_round);
        } else {
            runtime_state.record_randomness_beacon_failure(
                &config.source_id,
                config.beacon_round,
                "configured external randomness beacon conflicts with stored chain record",
            );
        }
        return Ok(true);
    }
    if config.beacon_round <= chain.state().finalized_beacon_round() {
        runtime_state.record_randomness_beacon_skipped(&config.source_id, config.beacon_round);
        if public_drand_poll {
            runtime_state
                .record_randomness_public_drand_fetch_stale(config.drand_poll_interval_ticks);
        }
        return Ok(true);
    }
    let command = match config.mode {
        RandomnessBeaconMode::Off => return Ok(false),
        RandomnessBeaconMode::LocalDeterministic => ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            randomness: config.randomness,
            proof_hash: config.proof_hash,
        },
        RandomnessBeaconMode::VerifiedDrand => ChainCommand::SubmitVerifiedDrandBeacon {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            public_key: config.drand_public_key.clone(),
            signature: config.drand_signature.clone(),
        },
        RandomnessBeaconMode::PublicDrand => ChainCommand::SubmitVerifiedChainedDrandBeacon {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            public_key: config.drand_public_key.clone(),
            signature: config.drand_signature.clone(),
            previous_signature: config.drand_previous_signature.clone(),
        },
    };
    match chain.apply_command(command) {
        Ok(_) => {
            store.persist_chain(chain).map_err(|error| {
                format!("failed to persist external randomness beacon: {error}")
            })?;
            publish_external_randomness_beacon(p2p_service, config).map_err(|error| {
                runtime_state.record_randomness_beacon_failure(
                    &config.source_id,
                    config.beacon_round,
                    &error,
                );
                error
            })?;
            runtime_state.record_randomness_beacon_applied(&config.source_id, config.beacon_round);
            if public_drand_poll {
                runtime_state
                    .record_randomness_public_drand_fetch_success(config.drand_poll_interval_ticks);
            }
            Ok(true)
        }
        Err(error) => {
            runtime_state.record_randomness_beacon_failure(
                &config.source_id,
                config.beacon_round,
                &error.to_string(),
            );
            Ok(true)
        }
    }
}

fn external_randomness_beacon_matches_config(
    record: &ExternalRandomnessBeaconRecord,
    config: &RandomnessBeaconRuntimeConfig,
) -> bool {
    if record.source_id != config.source_id
        || record.beacon_round != config.beacon_round
        || record.randomness != config.randomness
        || record.proof_hash != config.proof_hash
    {
        return false;
    }
    match config.mode {
        RandomnessBeaconMode::Off => false,
        RandomnessBeaconMode::LocalDeterministic => {
            matches!(
                record.proof,
                ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1
            )
        }
        RandomnessBeaconMode::VerifiedDrand => verified_drand_beacon_record(
            config.source_id.clone(),
            config.beacon_round,
            &config.drand_public_key,
            &config.drand_signature,
            record.observed_at_height,
        )
        .is_ok_and(|expected| expected.proof == record.proof),
        RandomnessBeaconMode::PublicDrand => verified_chained_drand_beacon_record(
            config.source_id.clone(),
            config.beacon_round,
            &config.drand_public_key,
            &config.drand_signature,
            &config.drand_previous_signature,
            record.observed_at_height,
        )
        .is_ok_and(|expected| expected.proof == record.proof),
    }
}

pub fn external_randomness_beacon_message(config: &RandomnessBeaconRuntimeConfig) -> P2pMessage {
    match config.mode {
        RandomnessBeaconMode::Off | RandomnessBeaconMode::LocalDeterministic => {
            P2pMessage::NewExternalRandomnessBeaconPayload {
                source_id: config.source_id.clone(),
                beacon_round: config.beacon_round,
                payload: encode_external_randomness_beacon_payload(
                    &config.source_id,
                    config.beacon_round,
                    &config.randomness,
                    &config.proof_hash,
                ),
            }
        }
        RandomnessBeaconMode::VerifiedDrand => P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            payload: encode_verified_drand_beacon_payload(
                &config.source_id,
                config.beacon_round,
                &config.drand_public_key,
                &config.drand_signature,
            ),
        },
        RandomnessBeaconMode::PublicDrand => P2pMessage::NewVerifiedChainedDrandBeaconPayload {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            payload: encode_verified_chained_drand_beacon_payload(
                &config.source_id,
                config.beacon_round,
                &config.drand_public_key,
                &config.drand_signature,
                &config.drand_previous_signature,
            ),
        },
    }
}

fn publish_external_randomness_beacon(
    p2p_service: &TensorVmLibp2pService,
    config: &RandomnessBeaconRuntimeConfig,
) -> std::result::Result<(), String> {
    p2p_service
        .publish_gossip(external_randomness_beacon_message(config))
        .map_err(|error| format!("failed to publish external randomness beacon gossip: {error}"))
}

pub fn randomness_beacon_source_label(config: &RandomnessBeaconRuntimeConfig) -> String {
    if !config.enabled() {
        return "none".to_owned();
    }
    format!("{}:{}", config.mode.label(), config.source_id)
}

pub fn randomness_beacon_hash_label(hash: &Hash) -> String {
    hex(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::{
        decode_external_randomness_beacon_payload, decode_verified_chained_drand_beacon_payload,
        decode_verified_drand_beacon_payload,
    };
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const VERIFIED_DRAND_PUBLIC_KEY_HEX: &str = "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11";
    const VERIFIED_DRAND_SIGNATURE_HEX: &str = "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433";
    const PUBLIC_DRAND_DEFAULT_PUBLIC_KEY_HEX: &str = "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31";
    const PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX: &str = "8d61d9100567de44682506aea1a7a6fa6e5491cd27a0a0ed349ef6910ac5ac20ff7bc3e09d7c046566c9f7f3c6f3b10104990e7cb424998203d8f7de586fb7fa5f60045417a432684f85093b06ca91c769f0e7ca19268375e659c2a2352b4655";
    const PUBLIC_DRAND_DEFAULT_ROUND_1_PREVIOUS_SIGNATURE_HEX: &str =
        "176f93498eac9ca337150b46d21dd58673ea4e3581185f869672e59fa4cb390a";
    const PUBLIC_DRAND_DEFAULT_INFO_JSON: &str = r#"{"public_key":"868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31","period":30,"genesis_time":1595431050,"chain_hash":"8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce","scheme":"pedersen-bls-chained"}"#;
    const PUBLIC_DRAND_DEFAULT_ROUND_1_JSON: &str = r#"{"round":1,"signature":"8d61d9100567de44682506aea1a7a6fa6e5491cd27a0a0ed349ef6910ac5ac20ff7bc3e09d7c046566c9f7f3c6f3b10104990e7cb424998203d8f7de586fb7fa5f60045417a432684f85093b06ca91c769f0e7ca19268375e659c2a2352b4655","previous_signature":"176f93498eac9ca337150b46d21dd58673ea4e3581185f869672e59fa4cb390a"}"#;

    fn verified_drand_vector() -> (u64, Vec<u8>, Vec<u8>, Hash) {
        (
            223_344,
            parse_env_hex_bytes(
                "VERIFIED_DRAND_PUBLIC_KEY_HEX",
                VERIFIED_DRAND_PUBLIC_KEY_HEX,
            )
            .unwrap(),
            parse_env_hex_bytes("VERIFIED_DRAND_SIGNATURE_HEX", VERIFIED_DRAND_SIGNATURE_HEX)
                .unwrap(),
            parse_hash_hex("f3d6adf1daa2c7877f90fb0f1a675ab0a42653a1e2a9b66fee0749d47a47bc57")
                .unwrap(),
        )
    }

    fn public_drand_default_round_1_vector() -> (u64, Vec<u8>, Vec<u8>, Vec<u8>, Hash) {
        let public_key = parse_env_hex_bytes(
            "PUBLIC_DRAND_DEFAULT_PUBLIC_KEY_HEX",
            PUBLIC_DRAND_DEFAULT_PUBLIC_KEY_HEX,
        )
        .unwrap();
        let signature = parse_env_hex_bytes(
            "PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX",
            PUBLIC_DRAND_DEFAULT_ROUND_1_SIGNATURE_HEX,
        )
        .unwrap();
        let previous_signature = parse_env_hex_bytes(
            "PUBLIC_DRAND_DEFAULT_ROUND_1_PREVIOUS_SIGNATURE_HEX",
            PUBLIC_DRAND_DEFAULT_ROUND_1_PREVIOUS_SIGNATURE_HEX,
        )
        .unwrap();
        let record = verified_chained_drand_beacon_record(
            verified_chained_drand_source_id(&public_key),
            1,
            &public_key,
            &signature,
            &previous_signature,
            0,
        )
        .unwrap();
        (
            1,
            public_key,
            signature,
            previous_signature,
            record.randomness,
        )
    }

    fn clear_randomness_beacon_env() {
        unsafe {
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_MODE");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_SOURCE_ID");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_ROUND");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_RANDOMNESS");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_PROOF_HASH");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_HTTP_BASE_URL");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_CHAIN_HASH");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_FETCH_TIMEOUT_MS");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_POLL_INTERVAL_TICKS");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_FAILURE_BACKOFF_MAX_TICKS");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_MAX_ROUND_LAG");
        }
    }

    #[test]
    fn local_deterministic_beacon_config_is_stable() {
        let left = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 7);
        let right = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 7);
        let changed = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 8);
        assert_eq!(left, right);
        assert_ne!(left.randomness, [0; 32]);
        assert_ne!(left.proof_hash, [0; 32]);
        assert_ne!(left.randomness, changed.randomness);
        assert_eq!(left.mode.label(), "local_deterministic");
    }

    #[test]
    fn external_randomness_beacon_message_carries_configured_record() {
        let config = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 7);
        let P2pMessage::NewExternalRandomnessBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } = external_randomness_beacon_message(&config)
        else {
            panic!("configured beacon must produce external beacon payload");
        };

        assert_eq!(source_id, config.source_id.as_str());
        assert_eq!(beacon_round, config.beacon_round);
        let decoded = decode_external_randomness_beacon_payload(&payload).unwrap();
        assert_eq!(decoded.source_id, source_id);
        assert_eq!(decoded.beacon_round, beacon_round);
        assert_eq!(decoded.randomness, config.randomness);
        assert_eq!(decoded.proof_hash, config.proof_hash);
    }

    #[test]
    fn verified_drand_beacon_config_from_env_builds_payload() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_randomness_beacon_env();
        unsafe {
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_MODE", "verified_drand");
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_ROUND", "223344");
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX",
                VERIFIED_DRAND_PUBLIC_KEY_HEX,
            );
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX",
                VERIFIED_DRAND_SIGNATURE_HEX,
            );
        }

        let (round, public_key, signature, expected_randomness) = verified_drand_vector();
        let config = RandomnessBeaconRuntimeConfig::from_env().unwrap();
        assert_eq!(config.mode, RandomnessBeaconMode::VerifiedDrand);
        assert_eq!(config.mode.label(), "verified_drand");
        assert_eq!(config.source_id, verified_drand_source_id(&public_key));
        assert_eq!(config.beacon_round, round);
        assert_eq!(config.randomness, expected_randomness);
        assert_ne!(config.proof_hash, [0; 32]);

        let P2pMessage::NewVerifiedDrandBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } = external_randomness_beacon_message(&config)
        else {
            panic!("verified drand config must produce verified drand payload");
        };

        assert_eq!(source_id, config.source_id.as_str());
        assert_eq!(beacon_round, round);
        let decoded = decode_verified_drand_beacon_payload(&payload).unwrap();
        assert_eq!(decoded.source_id, source_id);
        assert_eq!(decoded.beacon_round, round);
        assert_eq!(decoded.public_key, public_key);
        assert_eq!(decoded.signature, signature);

        clear_randomness_beacon_env();
    }

    #[test]
    fn verified_drand_beacon_config_rejects_source_public_key_mismatch() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_randomness_beacon_env();
        unsafe {
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_MODE", "verified_drand");
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_ROUND", "223344");
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_SOURCE_ID",
                "drand-pedersen-bls-unchained-v1:wrong",
            );
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX",
                VERIFIED_DRAND_PUBLIC_KEY_HEX,
            );
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX",
                VERIFIED_DRAND_SIGNATURE_HEX,
            );
        }

        let error = RandomnessBeaconRuntimeConfig::from_env().unwrap_err();
        assert!(error.contains("TENSORVM_RANDOMNESS_BEACON_SOURCE_ID must equal"));

        clear_randomness_beacon_env();
    }

    #[test]
    fn public_drand_round_mapping_handles_boundaries() {
        assert_eq!(drand_round_for_unix_time(100, 30, 100).unwrap(), 1);
        assert_eq!(drand_round_for_unix_time(100, 30, 129).unwrap(), 1);
        assert_eq!(drand_round_for_unix_time(100, 30, 130).unwrap(), 2);
        assert!(drand_round_for_unix_time(100, 0, 130).is_err());
        assert!(drand_round_for_unix_time(100, 30, 99).is_err());
    }

    #[test]
    fn public_drand_config_from_env_uses_default_chain() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_randomness_beacon_env();
        unsafe {
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_MODE", "public_drand");
        }

        let config = RandomnessBeaconRuntimeConfig::from_env().unwrap();
        assert_eq!(config.mode, RandomnessBeaconMode::PublicDrand);
        assert_eq!(config.mode.label(), "public_drand");
        assert_eq!(config.source_id, "public-drand:8990e7a9aaed2ffe");
        assert_eq!(config.beacon_round, 0);
        assert_eq!(
            config.drand_http_base_url,
            PUBLIC_DRAND_DEFAULT_HTTP_BASE_URL
        );
        assert_eq!(config.drand_chain_hash, PUBLIC_DRAND_DEFAULT_CHAIN_HASH);
        assert_eq!(
            config.drand_fetch_timeout_ms,
            PUBLIC_DRAND_DEFAULT_TIMEOUT_MS
        );
        assert_eq!(
            config.drand_poll_interval_ticks,
            PUBLIC_DRAND_DEFAULT_POLL_INTERVAL_TICKS
        );
        assert_eq!(
            config.drand_failure_backoff_max_ticks,
            PUBLIC_DRAND_DEFAULT_FAILURE_BACKOFF_MAX_TICKS
        );
        assert_eq!(
            config.drand_max_round_lag,
            PUBLIC_DRAND_DEFAULT_MAX_ROUND_LAG
        );

        clear_randomness_beacon_env();
    }

    #[test]
    fn public_drand_config_from_env_accepts_poll_and_backoff_knobs() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_randomness_beacon_env();
        unsafe {
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_MODE", "public_drand");
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_DRAND_POLL_INTERVAL_TICKS", "7");
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_FAILURE_BACKOFF_MAX_TICKS",
                "28",
            );
            std::env::set_var("TENSORVM_RANDOMNESS_BEACON_DRAND_MAX_ROUND_LAG", "3");
        }

        let config = RandomnessBeaconRuntimeConfig::from_env().unwrap();
        assert_eq!(config.drand_poll_interval_ticks, 7);
        assert_eq!(config.drand_failure_backoff_max_ticks, 28);
        assert_eq!(config.drand_max_round_lag, 3);

        unsafe {
            std::env::set_var(
                "TENSORVM_RANDOMNESS_BEACON_DRAND_FAILURE_BACKOFF_MAX_TICKS",
                "6",
            );
        }
        assert!(
            RandomnessBeaconRuntimeConfig::from_env()
                .unwrap_err()
                .contains("FAILURE_BACKOFF_MAX_TICKS")
        );

        clear_randomness_beacon_env();
    }

    #[test]
    fn public_drand_fetch_json_builds_chained_verified_payload() {
        let (round, public_key, signature, previous_signature, expected_randomness) =
            public_drand_default_round_1_vector();
        let config = public_drand_config_from_json(
            PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
            PUBLIC_DRAND_DEFAULT_INFO_JSON,
            PUBLIC_DRAND_DEFAULT_ROUND_1_JSON,
        )
        .unwrap();

        assert_eq!(config.mode, RandomnessBeaconMode::PublicDrand);
        assert_eq!(
            config.source_id,
            verified_chained_drand_source_id(&public_key)
        );
        assert_eq!(config.beacon_round, round);
        assert_eq!(config.randomness, expected_randomness);
        assert_eq!(config.drand_public_key, public_key);
        assert_eq!(config.drand_signature, signature);
        assert_eq!(config.drand_previous_signature, previous_signature);

        let P2pMessage::NewVerifiedChainedDrandBeaconPayload {
            source_id,
            beacon_round,
            payload,
        } = external_randomness_beacon_message(&config)
        else {
            panic!("public drand config must produce verified chained drand payload");
        };
        assert_eq!(source_id, config.source_id);
        assert_eq!(beacon_round, round);
        let decoded = decode_verified_chained_drand_beacon_payload(&payload).unwrap();
        assert_eq!(decoded.source_id, config.source_id);
        assert_eq!(decoded.beacon_round, round);
        assert_eq!(decoded.public_key, config.drand_public_key);
        assert_eq!(decoded.signature, config.drand_signature);
        assert_eq!(decoded.previous_signature, config.drand_previous_signature);
    }

    #[test]
    fn public_drand_fetch_json_records_round_freshness_at_observed_time() {
        let config = public_drand_config_from_json_at_time(
            PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
            PUBLIC_DRAND_DEFAULT_INFO_JSON,
            PUBLIC_DRAND_DEFAULT_ROUND_1_JSON,
            Some(1_595_431_080),
        )
        .unwrap();

        assert_eq!(config.beacon_round, 1);
        assert_eq!(config.drand_genesis_time, 1_595_431_050);
        assert_eq!(config.drand_period, 30);
        assert_eq!(config.drand_expected_latest_round, 2);
        assert_eq!(config.drand_round_lag, 1);
        assert_eq!(drand_rounds_per_chain_epoch(30, 6, 100).unwrap(), 20);
        assert_eq!(drand_rounds_per_chain_epoch(31, 6, 100).unwrap(), 20);
    }

    #[test]
    fn public_drand_fetch_rejects_wrong_chain_or_scheme() {
        let wrong_chain = PUBLIC_DRAND_DEFAULT_INFO_JSON.replace(
            PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert!(
            public_drand_config_from_json(
                PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
                &wrong_chain,
                PUBLIC_DRAND_DEFAULT_ROUND_1_JSON
            )
            .unwrap_err()
            .contains("drand chain hash mismatch")
        );

        let wrong_scheme = PUBLIC_DRAND_DEFAULT_INFO_JSON
            .replace("pedersen-bls-chained", "bls-unchained-g1-rfc9380");
        assert!(
            public_drand_config_from_json(
                PUBLIC_DRAND_DEFAULT_CHAIN_HASH,
                &wrong_scheme,
                PUBLIC_DRAND_DEFAULT_ROUND_1_JSON
            )
            .unwrap_err()
            .contains("unsupported drand scheme")
        );
    }

    #[test]
    fn stored_external_randomness_beacon_matches_configured_record() {
        let config = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 7);
        let record = ExternalRandomnessBeaconRecord {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            randomness: config.randomness,
            proof_hash: config.proof_hash,
            proof: crate::chain::ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1,
            observed_at_height: 3,
        };
        assert!(external_randomness_beacon_matches_config(&record, &config));

        let changed = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 8);
        assert!(!external_randomness_beacon_matches_config(
            &record, &changed
        ));

        let (round, public_key, signature, _) = verified_drand_vector();
        let config =
            RandomnessBeaconRuntimeConfig::verified_drand(round, public_key, signature).unwrap();
        let record = verified_drand_beacon_record(
            config.source_id.clone(),
            config.beacon_round,
            &config.drand_public_key,
            &config.drand_signature,
            9,
        )
        .unwrap();
        assert!(external_randomness_beacon_matches_config(&record, &config));

        let mut changed = record.clone();
        changed.proof = ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1;
        assert!(!external_randomness_beacon_matches_config(
            &changed, &config
        ));
    }
}
