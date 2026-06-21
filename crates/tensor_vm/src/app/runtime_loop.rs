use std::{thread, time::Duration};

use crate::{NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

use super::{
    LocalProductionContext, LocalProductionSchedule, RuntimeP2pMetadata, RuntimeRole,
    RuntimeServices, RuntimeStatusSnapshot, ServiceRuntimeConfig, format_role_runtime_report,
    ingest_network_once as ingest_runtime_network_once, serve_rpc_once as serve_runtime_rpc_once,
    start_runtime_services, tick_miner_role_work_once, tick_randomness_beacon_once,
    tick_validator_role_work_once as tick_validator_role_worker_once, write_role_runtime_status,
};

pub struct RoleRuntimeLoop {
    config: ServiceRuntimeConfig,
    store: NodeStore,
    server: RpcHttpServer,
    p2p_service: TensorVmLibp2pService,
    local_synthetic_job_producer: bool,
    local_block_proposer: bool,
    local_block_proposer_delay_blocks: u64,
    local_production: LocalProductionSchedule,
    runtime_state: NodeRuntimeState,
    p2p_metadata: RuntimeP2pMetadata,
}

pub fn run_role_runtime_loop(config: ServiceRuntimeConfig) -> std::result::Result<String, String> {
    let mut runtime = RoleRuntimeLoop::start(config)?;
    runtime.run_until_max_requests()?;
    Ok(runtime.report())
}

impl RoleRuntimeLoop {
    pub fn start(config: ServiceRuntimeConfig) -> std::result::Result<Self, String> {
        let RuntimeServices {
            store,
            server,
            p2p_service,
            p2p_metadata,
        } = start_runtime_services(&config)?;
        let local_production = LocalProductionSchedule::new(config.node.synthetic_block_interval());
        let local_synthetic_job_producer = config.node.local_synthetic_producer();
        let local_block_proposer = config.node.local_block_proposer();
        let local_block_proposer_delay_blocks =
            config.node.local_validator_block_proposer_delay_blocks;
        Ok(Self {
            config,
            store,
            server,
            p2p_service,
            local_synthetic_job_producer,
            local_block_proposer,
            local_block_proposer_delay_blocks,
            local_production,
            runtime_state: NodeRuntimeState::default(),
            p2p_metadata,
        })
    }

    pub fn run_until_max_requests(&mut self) -> std::result::Result<(), String> {
        self.write_status()?;
        self.server.set_nonblocking(true).map_err(|error| {
            format!("failed to configure nonblocking service listener: {error}")
        })?;
        loop {
            if self.max_requests_reached() {
                break;
            }
            self.serve_rpc_once()?;
            self.tick_randomness_beacon_once()?;
            self.ingest_network_once()?;
            self.tick_role_work_once()?;
            self.produce_local_round_if_due()?;
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    fn max_requests_reached(&self) -> bool {
        let max_requests = self.config.node.network.max_requests;
        max_requests != 0 && self.runtime_state.served_requests() >= max_requests
    }

    pub fn serve_rpc_once(&mut self) -> std::result::Result<(), String> {
        if serve_runtime_rpc_once(&self.store, &mut self.server, &mut self.runtime_state)? {
            self.write_status()?;
        }
        Ok(())
    }

    fn ingest_network_once(&mut self) -> std::result::Result<(), String> {
        if ingest_runtime_network_once(
            &self.store,
            &mut self.server,
            &self.p2p_service,
            self.local_synthetic_job_producer,
            &mut self.runtime_state,
        )? {
            self.write_status()?;
        }
        Ok(())
    }

    pub fn tick_randomness_beacon_once(&mut self) -> std::result::Result<(), String> {
        if tick_randomness_beacon_once(
            &self.config.randomness_beacon,
            &self.store,
            &mut self.server,
            &mut self.runtime_state,
        )? {
            self.write_status()?;
        }
        Ok(())
    }

    fn tick_role_work_once(&mut self) -> std::result::Result<(), String> {
        match self.config.role {
            RuntimeRole::Miner => {
                if tick_miner_role_work_once(
                    &self.config,
                    &self.store,
                    &mut self.server,
                    &self.p2p_service,
                    &mut self.runtime_state,
                )? {
                    self.write_status()?;
                }
                Ok(())
            }
            RuntimeRole::Validator => self.tick_validator_role_work_once(),
            RuntimeRole::Proposer | RuntimeRole::Service => Ok(()),
        }
    }

    pub fn tick_validator_role_work_once(&mut self) -> std::result::Result<(), String> {
        if tick_validator_role_worker_once(
            &self.config,
            &self.store,
            &mut self.server,
            &self.p2p_service,
            &mut self.runtime_state,
        )? {
            self.write_status()?;
        }
        Ok(())
    }

    fn produce_local_round_if_due(&mut self) -> std::result::Result<(), String> {
        if self
            .local_production
            .produce_if_due(LocalProductionContext {
                profile: &self.config.node.profile,
                local_producer: self.local_synthetic_job_producer,
                validator: self.config.role_wallet_address,
                store: &self.store,
                server: &mut self.server,
                p2p_service: &self.p2p_service,
                runtime_state: &mut self.runtime_state,
            })?
        {
            self.write_status()?;
        }
        Ok(())
    }

    fn write_status(&self) -> std::result::Result<(), String> {
        write_role_runtime_status(&self.config, &self.status_snapshot())
    }

    fn status_snapshot(&self) -> RuntimeStatusSnapshot {
        RuntimeStatusSnapshot::from_runtime_state(
            &self.runtime_state,
            &self.server,
            &self.p2p_service,
            self.local_synthetic_job_producer,
            self.local_block_proposer,
            self.local_block_proposer_delay_blocks,
            self.config.node.local_block_proposer_delay_satisfied(
                self.server.gateway().node.chain.state().height(),
            ),
            self.config.role,
            self.config.role_wallet_address,
        )
    }

    pub fn server(&self) -> &RpcHttpServer {
        &self.server
    }

    pub fn report(&self) -> String {
        let snapshot = self.status_snapshot();
        format_role_runtime_report(&self.config, &snapshot, &self.p2p_metadata.report())
    }
}
