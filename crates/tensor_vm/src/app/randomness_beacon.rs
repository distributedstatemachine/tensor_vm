use crate::{
    ChainCommand, ChainEngine, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService,
    api::P2pMessage,
    chain::ExternalRandomnessBeaconRecord,
    hash::hex,
    p2p::encode_external_randomness_beacon_payload,
    types::{Hash, hash_bytes, parse_hash_hex},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomnessBeaconMode {
    Off,
    LocalDeterministic,
}

impl RandomnessBeaconMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::LocalDeterministic => "local_deterministic",
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
}

impl RandomnessBeaconRuntimeConfig {
    pub fn off() -> Self {
        Self {
            mode: RandomnessBeaconMode::Off,
            source_id: String::new(),
            beacon_round: 0,
            randomness: [0; 32],
            proof_hash: [0; 32],
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
        }
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
            other => Err(format!(
                "unsupported TENSORVM_RANDOMNESS_BEACON_MODE {other:?}; expected off or local_deterministic"
            )),
        }
    }

    pub fn enabled(&self) -> bool {
        self.mode != RandomnessBeaconMode::Off
    }
}

fn parse_env_hash(name: &str, value: &str) -> std::result::Result<Hash, String> {
    parse_hash_hex(value).map_err(|error| format!("invalid {name}: {error:?}"))
}

pub fn tick_randomness_beacon_once(
    config: &RandomnessBeaconRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    if !config.enabled() {
        return Ok(false);
    }
    if runtime_state.randomness_latest_source_id() == config.source_id
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
        return Ok(true);
    }
    let command = ChainCommand::SubmitExternalRandomnessBeacon {
        source_id: config.source_id.clone(),
        beacon_round: config.beacon_round,
        randomness: config.randomness,
        proof_hash: config.proof_hash,
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
    record.source_id == config.source_id
        && record.beacon_round == config.beacon_round
        && record.randomness == config.randomness
        && record.proof_hash == config.proof_hash
}

pub fn external_randomness_beacon_message(config: &RandomnessBeaconRuntimeConfig) -> P2pMessage {
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
    use crate::p2p::decode_external_randomness_beacon_payload;

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
    fn stored_external_randomness_beacon_matches_configured_record() {
        let config = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 7);
        let record = ExternalRandomnessBeaconRecord {
            source_id: config.source_id.clone(),
            beacon_round: config.beacon_round,
            randomness: config.randomness,
            proof_hash: config.proof_hash,
            observed_at_height: 3,
        };
        assert!(external_randomness_beacon_matches_config(&record, &config));

        let changed = RandomnessBeaconRuntimeConfig::local_deterministic("fixture", 8);
        assert!(!external_randomness_beacon_matches_config(
            &record, &changed
        ));
    }
}
