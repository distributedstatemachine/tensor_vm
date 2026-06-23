use super::local_runtime_args::{DataDirArgs, IdentitySeedArgs, NodeRuntimeArgs, P2pListenArgs};
use clap::{Args, Subcommand, ValueEnum};
use libp2p::{Multiaddr, PeerId};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum NodeCommand {
    #[command(about = "Initialize the service node store.")]
    Init(DataDirArgs),
    #[command(about = "Manage libp2p peers.")]
    #[command(subcommand)]
    Peer(NodePeerCommand),
    #[command(about = "Check libp2p and node-store readiness.")]
    Check(NodeCheckArgs),
    #[command(about = "Serve RPC, explorer, faucet, telemetry, and libp2p.")]
    Serve(NodeServeArgs),
    #[command(about = "Show node-store status.")]
    Status(DataDirArgs),
    #[command(about = "Show one persisted block from the node store.")]
    Block(NodeBlockArgs),
    #[command(about = "Export public raw evidence records from chain-accepted node state.")]
    ExportPublicEvidence(NodePublicEvidenceExportArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum NodePeerCommand {
    #[command(about = "Add a libp2p bootstrap peer to the node store.")]
    Add(NodePeerAddArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodePeerAddArgs {
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[command(flatten)]
    pub(crate) bootstrap_peer: BootstrapPeerArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct BootstrapPeerArgs {
    #[arg(
        long,
        value_name = "PEER_ID",
        help = "Peer ID to persist as a bootstrap peer."
    )]
    pub(crate) peer_id: PeerId,
    #[arg(
        long,
        value_name = "MULTIADDR",
        help = "Reachable multiaddress for the peer."
    )]
    pub(crate) address: Multiaddr,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodeCheckArgs {
    #[command(flatten)]
    pub(crate) p2p_listen: P2pListenArgs,
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[command(flatten)]
    pub(crate) identity_seed: IdentitySeedArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodeServeArgs {
    #[command(flatten)]
    pub(crate) runtime: NodeRuntimeArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodeBlockArgs {
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[arg(
        long,
        value_name = "HEIGHT",
        help = "Block height to load from the node store."
    )]
    pub(crate) height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct NodePublicEvidenceExportArgs {
    #[command(flatten)]
    pub(crate) data_dir: DataDirArgs,
    #[arg(
        long,
        value_enum,
        value_name = "KIND",
        help = "Public evidence record kind to export from chain-accepted node state."
    )]
    pub(crate) kind: NodePublicEvidenceExportKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum NodePublicEvidenceExportKind {
    RandomnessBeacon,
    ValidatorVrfLifecycle,
}
