use crate::app::PublicEvidenceExportKind;
use crate::cli::{LocalnetCommand, NodeCommand, NodePeerCommand, NodePublicEvidenceExportKind};
use crate::types::Hash;
use libp2p::{Multiaddr, PeerId};
use std::net::SocketAddr;
use std::path::Path;

use super::operator_validation::{validate_data_dir, validate_service_runtime};
use super::tvmd_path::path_arg;
use super::{
    add_service_peer, check_service_readiness, export_public_evidence_records, init_service_store,
    seed_local_testnet, serve_service, service_block_status, service_status,
    verify_local_cpu_store,
};

pub(super) fn execute_node_command(command: &NodeCommand) -> std::result::Result<String, String> {
    match command {
        NodeCommand::Init(args) => execute_node_init(&args.data_dir),
        NodeCommand::Peer(command) => execute_node_peer_command(command),
        NodeCommand::Check(args) => execute_node_check(
            &args.p2p_listen.p2p_listen,
            &args.data_dir.data_dir,
            args.identity_seed
                .identity_seed
                .map(|seed| seed.into_hash()),
        ),
        NodeCommand::Serve(args) => {
            let runtime = &args.runtime;
            execute_node_serve(
                &runtime.listen,
                &runtime.p2p_listen.p2p_listen,
                &runtime.data_dir.data_dir,
                runtime
                    .identity_seed
                    .identity_seed
                    .map(|seed| seed.into_hash()),
                &runtime.auth_token,
                runtime.max_requests,
            )
        }
        NodeCommand::Status(args) => execute_node_status(&args.data_dir),
        NodeCommand::Block(args) => execute_node_block(&args.data_dir.data_dir, args.height),
        NodeCommand::ExportPublicEvidence(args) => {
            execute_node_export_public_evidence(&args.data_dir.data_dir, args.kind)
        }
    }
}

fn execute_node_init(data_dir: &Path) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    init_service_store(&data_dir)
}

fn execute_node_peer_command(command: &NodePeerCommand) -> std::result::Result<String, String> {
    match command {
        NodePeerCommand::Add(args) => execute_node_peer_add(
            &args.data_dir.data_dir,
            &args.bootstrap_peer.peer_id,
            &args.bootstrap_peer.address,
        ),
    }
}

fn execute_node_peer_add(
    data_dir: &Path,
    peer_id: &PeerId,
    address: &Multiaddr,
) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    add_service_peer(&data_dir, &peer_id.to_string(), &address.to_string())
}

fn execute_node_check(
    p2p_listen: &Multiaddr,
    data_dir: &Path,
    identity_seed: Option<Hash>,
) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    check_service_readiness(&p2p_listen.to_string(), &data_dir, identity_seed)
}

fn execute_node_serve(
    listen: &SocketAddr,
    p2p_listen: &Multiaddr,
    data_dir: &Path,
    identity_seed: Option<Hash>,
    auth_token: &str,
    max_requests: usize,
) -> std::result::Result<String, String> {
    let listen = listen.to_string();
    let p2p_listen = p2p_listen.to_string();
    let data_dir = path_arg(data_dir);
    validate_service_runtime(&data_dir, auth_token)?;
    serve_service(
        &listen,
        &p2p_listen,
        &data_dir,
        identity_seed,
        auth_token,
        max_requests,
    )
}

fn execute_node_status(data_dir: &Path) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    service_status(&data_dir)
}

fn execute_node_block(data_dir: &Path, height: u64) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    service_block_status(&data_dir, height)
}

fn execute_node_export_public_evidence(
    data_dir: &Path,
    kind: NodePublicEvidenceExportKind,
) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    let kind = match kind {
        NodePublicEvidenceExportKind::RandomnessBeacon => {
            PublicEvidenceExportKind::RandomnessBeacon
        }
        NodePublicEvidenceExportKind::ValidatorVrfLifecycle => {
            PublicEvidenceExportKind::ValidatorVrfLifecycle
        }
    };
    export_public_evidence_records(&data_dir, kind).map_err(|error| error.to_string())
}

pub(super) fn execute_localnet_command(
    command: &LocalnetCommand,
) -> std::result::Result<String, String> {
    match command {
        LocalnetCommand::Seed(args) => execute_localnet_seed(&args.data_dir),
        LocalnetCommand::Verify(args) => {
            execute_localnet_verify(&args.data_dir.data_dir, args.json)
        }
    }
}

fn execute_localnet_seed(data_dir: &Path) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    seed_local_testnet(&data_dir)
}

fn execute_localnet_verify(data_dir: &Path, json: bool) -> std::result::Result<String, String> {
    let data_dir = validated_data_dir(data_dir)?;
    verify_local_cpu_store(&data_dir, json)
}

fn validated_data_dir(data_dir: &Path) -> std::result::Result<String, String> {
    let data_dir = path_arg(data_dir);
    validate_data_dir(&data_dir)?;
    Ok(data_dir)
}
