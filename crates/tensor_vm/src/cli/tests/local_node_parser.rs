use super::parser_support::{
    data_dir_args, identity_seed_args, multiaddr, node_serve_args, p2p_listen_args,
};
use super::{
    BootstrapPeerArgs, NodeBlockArgs, NodeCheckArgs, NodeCommand, NodePeerAddArgs, NodePeerCommand,
    NodePublicEvidenceExportArgs, NodePublicEvidenceExportKind, TvmdCommand, parse_test_cli,
};
use libp2p::PeerId;

#[test]
fn parses_documented_node_commands() {
    assert_eq!(
        parse_test_cli(&["node", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args("/var/lib/tensorvm")))
    );
    let bootstrap_peer = PeerId::random().to_string();
    let expected_peer_add = NodePeerAddArgs {
        data_dir: data_dir_args("/var/lib/tensorvm"),
        bootstrap_peer: BootstrapPeerArgs {
            peer_id: bootstrap_peer.parse().expect("test peer ID must parse"),
            address: multiaddr("/dns/bootstrap.tensorvm.net/tcp/4001"),
        },
    };
    assert_eq!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            &bootstrap_peer,
            "--address",
            "/dns/bootstrap.tensorvm.net/tcp/4001",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Peer(NodePeerCommand::Add(expected_peer_add)))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: p2p_listen_args("/ip4/0.0.0.0/tcp/4001"),
            data_dir: data_dir_args("/var/lib/tensorvm"),
            identity_seed: identity_seed_args(None),
        }))
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: p2p_listen_args("/ip4/0.0.0.0/tcp/4001"),
            data_dir: data_dir_args("/var/lib/tensorvm"),
            identity_seed: identity_seed_args(Some([0x11; 32])),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(node_serve_args(
            "0.0.0.0:8545",
            "/ip4/0.0.0.0/tcp/4001",
            "/var/lib/tensorvm",
            None,
            "secret",
            0,
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(node_serve_args(
            "0.0.0.0:8545",
            "/ip4/0.0.0.0/tcp/4001",
            "/var/lib/tensorvm",
            Some([0x11; 32]),
            "secret",
            0,
        )))
    );
    assert_eq!(
        parse_test_cli(&["node", "status", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Status(data_dir_args("/var/lib/tensorvm")))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "block",
            "--data-dir",
            "/var/lib/tensorvm",
            "--height",
            "3"
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Block(NodeBlockArgs {
            data_dir: data_dir_args("/var/lib/tensorvm"),
            height: 3,
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "export-public-evidence",
            "--data-dir",
            "/var/lib/tensorvm",
            "--kind",
            "randomness-beacon"
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::ExportPublicEvidence(
            NodePublicEvidenceExportArgs {
                data_dir: data_dir_args("/var/lib/tensorvm"),
                kind: NodePublicEvidenceExportKind::RandomnessBeacon,
            }
        ))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "export-public-evidence",
            "--data-dir",
            "/var/lib/tensorvm",
            "--kind",
            "validator-vrf-lifecycle"
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::ExportPublicEvidence(
            NodePublicEvidenceExportArgs {
                data_dir: data_dir_args("/var/lib/tensorvm"),
                kind: NodePublicEvidenceExportKind::ValidatorVrfLifecycle,
            }
        ))
    );
}

#[test]
fn clap_node_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["node", "serve", "--auth-token", "secret"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(node_serve_args(
            "127.0.0.1:8545",
            "/ip4/127.0.0.1/tcp/4001",
            ".tensorvm",
            None,
            "secret",
            0,
        )))
    );
    assert_eq!(
        parse_test_cli(&["node", "init"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args(".tensorvm")))
    );
}
