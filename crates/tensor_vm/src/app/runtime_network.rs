use super::ingest_network_events;
use crate::{NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

pub fn ingest_network_once(
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    local_producer: bool,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    let ingested = ingest_network_events(
        server,
        p2p_service,
        local_producer,
        runtime_state.pending_payloads_mut(),
    )?;
    if !ingested.has_activity() {
        return Ok(false);
    }
    let should_persist = ingested.applied_blocks > 0
        || ingested.job_payloads_applied > 0
        || ingested.receipt_payloads_applied > 0
        || ingested.attestation_payloads_applied > 0
        || ingested.block_votes_applied > 0
        || ingested.block_check_challenges_applied > 0;
    runtime_state.record_network_ingest(ingested);
    if should_persist {
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist network-applied state: {error}"))?;
    }
    Ok(true)
}
