use super::commands::{TvmdCli, TvmdCommand};
use super::evidence_fields::{public_evidence_record_kind_tag, public_service_kind_tag};
use super::local_commands::{
    BootstrapPeerArgs, DataDirArgs, IdentitySeedArgs, LocalCpuVerifyArgs, LocalnetCommand,
    MinerCheckArgs, MinerCommand, MinerRunArgs, NodeBlockArgs, NodeCheckArgs, NodeCommand,
    NodePeerAddArgs, NodePeerCommand, NodePublicEvidenceExportArgs, NodePublicEvidenceExportKind,
    NodeRuntimeArgs, NodeServeArgs, P2pListenArgs, ProposerCommand, RoleNodeArgs, RoleRuntimeArgs,
    RoleWalletArgs, StakeArgs, ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs,
};
use super::network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line_from_service_log,
    network_observation_root, service_log_field,
};
use super::node_evidence::node_heartbeat_observation_summary_from_file;
use super::public_evidence_commands::{
    AuditorRecordArgs, BlockHeightWindowArgs, EvidenceBundleIdArgs, EvidenceCommand,
    EvidenceNetworkCommand, EvidenceNodeCommand, EvidenceRecordCommand, EvidenceRunCommand,
    EvidenceServiceCommand, ManifestSignerArgs, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NetworkObservationProtocolCountsArgs,
    NetworkObservationTargetArgs, NetworkObservationTransportLimitsArgs, NodeHeartbeatArgs,
    NodeHeartbeatFromFileArgs, ObservationTimestampArgs, OperatorAttestationArgs, OperatorIdArgs,
    PublicCommand, PublicEvidenceManifestArgs, PublicEvidenceRecordContextArgs,
    PublicEvidenceRecordKindArg, PublicNodeIdentityArgs, PublicNodeRoleArg,
    PublicServiceEndpointArgs, PublicServiceKindArg, PublicTestnetManifestArgs, PublicationArgs,
    PublicationBundleArgs, RecordArtifactArgs, RecordArtifactFromFileArgs,
    RecordArtifactFromRootsArgs, RecordArtifactLocatorArgs, RecordFileArgs, RecordRootArgs,
    RecordRootsArgs, RecordSummaryArgs, RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
    RunWindowArgs, RunWindowContextArgs, RunWindowFromFileArgs, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceContentTargetArgs,
    ServiceHealthArgs, ServiceHealthFromFileArgs, ServiceHealthPathArgs,
};
use super::public_evidence_execution::execute_public_evidence_command;
use super::record_evidence_roots::{
    public_evidence_record_root_from_line, public_evidence_record_roots_from_file,
};
use super::record_supporting_evidence::{
    supporting_record_line_prefix, supporting_record_root_from_line,
    validate_supporting_record_payload,
};
use super::run_window_evidence::run_window_observation_summary_from_file;
use super::service_evidence::{
    public_service_content_root, service_health_observation_summary_from_file,
};
use super::value_types::{AddressArg, HashArg, HexBytesArg, MinerDeviceArg};
use crate::error::TvmError;
use crate::hash::hex;
use crate::testnet::{
    PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind,
    aggregate_public_evidence_record_roots, sign_public_evidence_record,
};
use crate::types::{address, hash_bytes};
use libp2p::PeerId;

mod command_help;
mod command_helpers;
mod local_cuda_validation;
mod local_execution_reports;
mod local_miner_parser;
mod local_node_parser;
mod local_node_validation;
mod local_parser;
mod local_role_parser;
mod local_validation;
mod local_validator_parser;
mod manifest_bundle_fixtures;
mod manifest_fixtures;
mod manifest_network_fixtures;
mod manifest_node_fixtures;
mod manifest_preflight_fixtures;
mod manifest_publication_fixtures;
mod manifest_reports;
mod manifest_service_fixtures;
mod parser_support;
mod public_evidence_network_parser;
mod public_evidence_network_rejections;
mod public_evidence_network_reports;
mod public_evidence_node_parser;
mod public_evidence_node_rejections;
mod public_evidence_node_reports;
mod public_evidence_publication_parser;
mod public_evidence_publication_rejections;
mod public_evidence_publication_reports;
mod public_evidence_record_aggregate_reports;
mod public_evidence_record_execution_rejections;
mod public_evidence_record_file_reports;
mod public_evidence_record_line_rejections;
mod public_evidence_record_parser;
mod public_evidence_record_rejections;
mod public_evidence_record_reports;
mod public_evidence_record_summary_reports;
mod public_evidence_run_window_parser;
mod public_evidence_run_window_rejections;
mod public_evidence_run_window_reports;
mod public_evidence_service_health_rejections;
mod public_evidence_service_parser;
mod public_evidence_service_parser_rejections;
mod public_evidence_service_rejections;
mod public_evidence_service_reports;
mod public_parser;
mod report_fields;

use command_helpers::*;
use manifest_bundle_fixtures::*;
use manifest_fixtures::*;
use manifest_node_fixtures::*;
use manifest_preflight_fixtures::*;
use manifest_publication_fixtures::*;
use manifest_service_fixtures::*;
use report_fields::*;
