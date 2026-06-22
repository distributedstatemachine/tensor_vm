use super::{
    RandomnessBeaconRuntimeConfig, RoleServiceConfig, RuntimeRole, ServiceRuntimeConfig,
    role_wallet_address, run_role_runtime_loop, runtime_node_config,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoleServiceKind {
    Miner,
    Validator,
    Proposer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleServiceRunner {
    kind: RoleServiceKind,
}

impl RoleServiceRunner {
    pub fn miner() -> Self {
        Self {
            kind: RoleServiceKind::Miner,
        }
    }

    pub fn validator() -> Self {
        Self {
            kind: RoleServiceKind::Validator,
        }
    }

    pub fn proposer() -> Self {
        Self {
            kind: RoleServiceKind::Proposer,
        }
    }

    fn runtime_command(self) -> &'static str {
        match self.kind {
            RoleServiceKind::Miner => "miner_run",
            RoleServiceKind::Validator => "validator_run",
            RoleServiceKind::Proposer => "proposer_run",
        }
    }

    fn runtime_role(self) -> RuntimeRole {
        match self.kind {
            RoleServiceKind::Miner => RuntimeRole::Miner,
            RoleServiceKind::Validator => RuntimeRole::Validator,
            RoleServiceKind::Proposer => RuntimeRole::Proposer,
        }
    }

    pub fn service_runtime_config(
        self,
        config: RoleServiceConfig<'_>,
    ) -> std::result::Result<ServiceRuntimeConfig, String> {
        let role = self.runtime_role();
        ServiceRuntimeConfig {
            runtime_command: self.runtime_command(),
            role,
            role_wallet_address: Some(role_wallet_address(config.wallet)?),
            role_wallet_secret: Some(config.wallet.to_owned()),
            node: runtime_node_config(
                config.data_dir,
                role,
                config.listen,
                config.p2p_listen,
                config.identity_seed,
                config.auth_token,
                config.max_requests,
            )?,
            randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
        }
        .randomness_beacon_from_env()
    }

    pub fn run(self, config: RoleServiceConfig<'_>) -> std::result::Result<String, String> {
        let service_report = run_role_runtime_loop(self.service_runtime_config(config)?)?;
        Ok(self.format_report(config, &service_report))
    }

    pub fn format_report(self, config: RoleServiceConfig<'_>, service_report: &str) -> String {
        match self.kind {
            RoleServiceKind::Miner => format!(
                "command=miner_run\nrole=miner\nwallet={}\ndevice={}\nnode={}\nrole_runtime_ready=true\n{service_report}",
                config.wallet,
                config.device.unwrap_or("unknown"),
                config.node
            ),
            RoleServiceKind::Validator => format!(
                "command=validator_run\nrole=validator\nwallet={}\nnode={}\nreference_verifier_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
            RoleServiceKind::Proposer => format!(
                "command=proposer_run\nrole=proposer\nwallet={}\nnode={}\nproposer_ready=true\nrole_runtime_ready=true\n{service_report}",
                config.wallet, config.node
            ),
        }
    }
}
