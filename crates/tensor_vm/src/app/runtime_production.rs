use std::time::{Duration, Instant};

use super::produce_and_publish_synthetic_job;
use crate::{
    ChainProfile, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService, types::Address,
};

pub struct LocalProductionSchedule {
    block_interval: Option<Duration>,
    next_block_at: Option<Instant>,
}

pub struct LocalProductionContext<'a> {
    pub profile: &'a ChainProfile,
    pub local_producer: bool,
    pub validator: Option<Address>,
    pub store: &'a NodeStore,
    pub server: &'a mut RpcHttpServer,
    pub p2p_service: &'a TensorVmLibp2pService,
    pub runtime_state: &'a mut NodeRuntimeState,
}

impl LocalProductionSchedule {
    pub fn new(block_interval: Option<Duration>) -> Self {
        Self {
            block_interval,
            next_block_at: block_interval.map(|interval| Instant::now() + interval),
        }
    }

    pub fn produce_if_due(
        &mut self,
        context: LocalProductionContext<'_>,
    ) -> std::result::Result<bool, String> {
        let Some(interval) = self.block_interval else {
            return Ok(false);
        };
        if self
            .next_block_at
            .is_none_or(|deadline| Instant::now() < deadline)
        {
            return Ok(false);
        }
        let mut status_changed = false;
        if context.local_producer {
            if context.validator.is_none() {
                self.next_block_at = Some(Instant::now() + interval);
                return Ok(false);
            }
            if produce_and_publish_synthetic_job(
                context.server,
                context.p2p_service,
                context.profile,
            )?
            .is_some()
            {
                context
                    .store
                    .persist_chain(&context.server.gateway().node.chain)
                    .map_err(|error| format!("failed to persist produced job: {error}"))?;
                status_changed = true;
            }
        }
        self.next_block_at = Some(Instant::now() + interval);
        Ok(status_changed)
    }
}

pub(super) fn next_block_timestamp(server: &RpcHttpServer) -> u64 {
    let chain = &server.gateway().node.chain;
    chain
        .blocks()
        .last()
        .map(|block| {
            block
                .timestamp
                .saturating_add(chain.params().block_time_seconds)
        })
        .unwrap_or(0)
}
