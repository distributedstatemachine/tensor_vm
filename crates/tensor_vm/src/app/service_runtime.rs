use super::{
    RandomnessBeaconRuntimeConfig, RuntimeRole, ServiceRuntimeConfig, run_role_runtime_loop,
    runtime_node_config,
};

pub fn serve_service(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    run_role_runtime_loop(
        ServiceRuntimeConfig {
            runtime_command: "service_serve",
            role: RuntimeRole::Service,
            role_wallet_address: None,
            role_wallet_secret: None,
            node: runtime_node_config(
                data_dir,
                RuntimeRole::Service,
                listen,
                p2p_listen,
                identity_seed,
                auth_token,
                max_requests,
            )?,
            randomness_beacon: RandomnessBeaconRuntimeConfig::off(),
        }
        .randomness_beacon_from_env()?,
    )
}
