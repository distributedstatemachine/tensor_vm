use std::path::Path;

use super::runtime_services::merged_bootstrap_addresses;
use super::{KeyValueReportWriter, p2p_identity_report, runtime_bootstrap_addresses};
use crate::{
    Chain, Libp2pControlPlaneConfig, NodeStore, NodeStoreStatus, PeerRecord, hash::hex,
    spawn_libp2p_service, types::hash_bytes,
};

pub fn init_service_store(data_dir: &str) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    if Path::new(data_dir).exists()
        && Path::new(data_dir)
            .read_dir()
            .map_err(|error| format!("failed to inspect data dir {data_dir}: {error}"))?
            .next()
            .is_some()
    {
        match store.load_chain().and_then(|_| store.compact_status()) {
            Ok(status) => {
                return Ok(service_init_report(&status, true, false, None));
            }
            Err(error) => {
                let status = store.recover_from_chain_state().map_err(|recovery_error| {
                    format!(
                        "existing node store is invalid: {error}; chain-state recovery failed: {recovery_error}"
                    )
                })?;
                return Ok(service_init_report(
                    &status,
                    true,
                    true,
                    Some("chain_state"),
                ));
            }
        }
    }

    let chain = Chain::new(hash_bytes(
        b"tensor-vm-service-genesis",
        &[data_dir.as_bytes()],
    ));
    let status = store
        .persist_chain(&chain)
        .map_err(|error| format!("failed to initialize node store {data_dir}: {error}"))?;
    Ok(service_init_report(&status, false, false, None))
}

pub fn add_service_peer(
    data_dir: &str,
    peer_id: &str,
    address: &str,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let record = PeerRecord::from_strings(peer_id, address)
        .map_err(|error| format!("invalid libp2p bootstrap peer: {error}"))?;
    let bootstrap_address = record
        .bootstrap_multiaddr()
        .map_err(|error| format!("invalid libp2p bootstrap peer: {error}"))?
        .to_string();
    let records = store
        .peer_book_store()
        .upsert_record(record)
        .map_err(|error| format!("failed to update libp2p peer book {data_dir}: {error}"))?;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_peer_add");
    report.field("data_dir", data_dir);
    report.field("peer_id", peer_id);
    report.field("address", address);
    report.field("bootstrap_address", bootstrap_address);
    report.field("bootstrap_peers", records.len());
    Ok(report.finish())
}

pub fn check_service_readiness(
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let configured_bootstrap_addresses = runtime_bootstrap_addresses()?;
    let bootstrap_addresses = merged_bootstrap_addresses(&store, &configured_bootstrap_addresses)?;
    let bootstrap_peer_count = bootstrap_addresses.len();
    let p2p_config = Libp2pControlPlaneConfig {
        listen_addresses: vec![p2p_listen.to_owned()],
        bootstrap_addresses,
        identity_seed,
        ..Libp2pControlPlaneConfig::default()
    };
    let max_transmit_bytes = p2p_config.max_gossipsub_transmit_bytes;
    let request_timeout_seconds = p2p_config.request_timeout_seconds;
    let max_concurrent_streams = p2p_config.max_concurrent_request_streams;
    let idle_timeout_seconds = p2p_config.idle_connection_timeout_seconds;
    let p2p_service = spawn_libp2p_service(p2p_config)
        .map_err(|error| format!("failed to start mandatory libp2p readiness check: {error}"))?;
    let identity = p2p_identity_report(identity_seed);
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_readiness");
    report.field("p2p_listen", p2p_listen);
    report.field("p2p_runtime", "libp2p");
    report.field("p2p_peer_id", p2p_service.peer_id());
    report.field(
        "p2p_gossipsub_topics",
        p2p_service.info().subscribed_topics.len(),
    );
    report.field(
        "p2p_request_response_protocols",
        p2p_service.info().request_response_protocols.len(),
    );
    report.field("p2p_bootstrap_peers", bootstrap_peer_count);
    report.append_report(&identity);
    report.field("p2p_max_transmit_bytes", max_transmit_bytes);
    report.field("p2p_request_timeout_seconds", request_timeout_seconds);
    report.field("p2p_max_concurrent_streams", max_concurrent_streams);
    report.field("p2p_idle_timeout_seconds", idle_timeout_seconds);
    report.field("data_dir", data_dir);
    report.field("node_store_ready", true);
    report.field("libp2p_ready", true);
    Ok(report.finish())
}

fn service_init_report(
    status: &NodeStoreStatus,
    existing_store: bool,
    recovered_store: bool,
    recovery_source: Option<&str>,
) -> String {
    let mut report = KeyValueReportWriter::new();
    report.field("command", "service_init");
    report.field("data_dir", status.data_dir.display());
    report.field("existing_store", existing_store);
    report.field("recovered_store", recovered_store);
    if let Some(recovery_source) = recovery_source {
        report.field("recovery_source", recovery_source);
    }
    report.field("block_count", status.block_count);
    report.field("latest_block_hash", hex(&status.latest_block_hash));
    report.finish()
}
