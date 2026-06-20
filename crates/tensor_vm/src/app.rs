mod block_status;
mod commands;
mod key_value_report;
mod local_cpu_verify;
mod local_testnet_seed;
mod miner_device_readiness;
mod miner_role;
mod network;
mod operator_checks;
mod operator_validation;
mod role_services;
mod runtime_config;
mod runtime_loop;
mod runtime_network;
mod runtime_production;
mod runtime_rpc;
mod runtime_services;
mod runtime_status;
mod runtime_status_snapshot;
mod runtime_validator;
mod service_runtime;
mod shared;
mod status;
mod tvmd_dispatch;
mod tvmd_node_dispatch;
mod tvmd_path;
mod validator_fetch;
mod validator_role;

pub use block_status::service_block_status;
pub use commands::{add_service_peer, check_service_readiness, init_service_store};
pub(crate) use key_value_report::{KeyValueReport, KeyValueReportError, KeyValueReportWriter};
pub use local_cpu_verify::verify_local_cpu_store;
pub use local_testnet_seed::seed_local_testnet;
pub use miner_role::{
    MinerRoleReceiptSubmission, MinerRoleWorkObservation, miner_role_work_observation,
    submit_miner_role_receipt, tick_miner_role_work_once,
};
pub use network::{
    ChainAnnouncementCheckpoint, chain_announcement_checkpoint, ingest_network_events,
    produce_and_publish_synthetic_job, produce_and_publish_synthetic_round,
    produce_and_publish_synthetic_work, publish_new_chain_announcements,
    publish_observed_block_check_challenge, publish_validator_block_proposal,
};
pub use role_services::RoleServiceRunner;
pub use runtime_config::{
    RoleServiceConfig, RuntimeRole, ServiceRuntimeConfig, chain_profile_from_label,
    role_wallet_address, runtime_node_config, runtime_role_wallet_address_text,
    runtime_role_wallet_registered, runtime_role_wallet_registration,
};
pub use runtime_loop::{RoleRuntimeLoop, run_role_runtime_loop};
pub use runtime_network::ingest_network_once;
pub use runtime_production::{LocalProductionContext, LocalProductionSchedule};
pub use runtime_rpc::serve_rpc_once;
pub use runtime_services::{RuntimeP2pMetadata, RuntimeServices, start_runtime_services};
pub use runtime_status::{format_role_runtime_report, write_role_runtime_status};
pub use runtime_status_snapshot::{RuntimeP2pReport, RuntimeStatusSnapshot};
pub use runtime_validator::tick_validator_role_work_once;
pub use service_runtime::serve_service;
pub use shared::{local_cpu_seed_beacon, p2p_identity_report};
pub use status::{hex_hash_list, service_status};
#[cfg(test)]
pub(crate) use tvmd_dispatch::execute_tvmd_command;
pub use tvmd_dispatch::run;
pub use validator_fetch::{
    ValidatorRemoteTensorFetchReport, ValidatorRemoteTensorResponse,
    fetch_validator_role_missing_tensors, validator_remote_tensor_response,
};
pub use validator_role::{
    ValidatorRoleAttestationSubmission, ValidatorRoleAuditObservation,
    ValidatorRoleAuditReportSubmission, ValidatorRoleBlockProposal,
    ValidatorRoleBlockProposalObservation, ValidatorRoleBlockVoteSubmission,
    ValidatorRoleWorkObservation, submit_validator_role_attestation,
    submit_validator_role_audit_report, submit_validator_role_block_proposal,
    submit_validator_role_block_vote, validator_role_audit_observation,
    validator_role_block_proposal_observation, validator_role_work_observation,
};
