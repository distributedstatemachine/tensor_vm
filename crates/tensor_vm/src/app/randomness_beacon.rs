use crate::{
    ChainCommand, ChainEngine, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService,
    api::P2pMessage,
    chain::{
        ExternalRandomnessBeaconProof, ExternalRandomnessBeaconRecord,
        verified_drand_beacon_record, verified_drand_source_id,
    },
    hash::hex,
    p2p::{encode_external_randomness_beacon_payload, encode_verified_drand_beacon_payload},
    types::{Hash, hash_bytes, parse_hash_hex},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomnessBeaconMode {
    Off,
    LocalDeterministic,
    VerifiedDrand,
}

impl RandomnessBeaconMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::LocalDeterministic => "local_deterministic",
            Self::VerifiedDrand => "verified_drand",
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
            other => Err(format!(
                "unsupported TENSORVM_RANDOMNESS_BEACON_MODE {other:?}; expected off, local_deterministic, or verified_drand"
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
        decode_external_randomness_beacon_payload, decode_verified_drand_beacon_payload,
    };
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const VERIFIED_DRAND_PUBLIC_KEY_HEX: &str = "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11";
    const VERIFIED_DRAND_SIGNATURE_HEX: &str = "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433";

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

    fn clear_randomness_beacon_env() {
        unsafe {
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_MODE");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_SOURCE_ID");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_ROUND");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_RANDOMNESS");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_PROOF_HASH");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_PUBLIC_KEY_HEX");
            std::env::remove_var("TENSORVM_RANDOMNESS_BEACON_DRAND_SIGNATURE_HEX");
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
