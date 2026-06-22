use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
use super::public_evidence_record_artifact_commands::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
};
use super::public_evidence_signing_commands::ManifestSignerArgs;
use super::value_types::HashArg;
use crate::testnet::PublicEvidenceRecordKind;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceRecordCommand {
    #[command(about = "Generate a supporting-record summary.")]
    Summary(RecordSummaryArgs),
    #[command(about = "Generate a supporting-record artifact locator.")]
    Artifact(RecordArtifactArgs),
    #[command(about = "Generate a supporting-record artifact locator from roots.")]
    ArtifactRoots(RecordArtifactFromRootsArgs),
    #[command(about = "Generate a supporting-record artifact locator from a file.")]
    ArtifactFile(RecordArtifactFromFileArgs),
    #[command(about = "Generate a supporting-record summary from roots.")]
    SummaryRoots(RecordSummaryFromRootsArgs),
    #[command(about = "Generate a supporting-record summary from a file.")]
    SummaryFile(RecordSummaryFromFileArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordSummaryArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) root: RecordRootArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordSummaryFromRootsArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) roots: RecordRootsArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordSummaryFromFileArgs {
    #[command(flatten)]
    pub(crate) context: PublicEvidenceRecordContextArgs,
    #[command(flatten)]
    pub(crate) file: RecordFileArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicEvidenceRecordContextArgs {
    #[arg(long, help = "Supporting-record class.")]
    pub(crate) kind: PublicEvidenceRecordKindArg,
    #[command(flatten)]
    pub(crate) bundle: EvidenceBundleIdArgs,
    #[command(flatten)]
    pub(crate) signer: ManifestSignerArgs,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordRootArgs {
    #[arg(
        long,
        value_name = "HEX",
        help = "Root hash of the supporting-record set."
    )]
    pub(crate) record_root: HashArg,
    #[arg(
        long,
        value_name = "N",
        help = "Number of records covered by the root."
    )]
    pub(crate) record_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordRootsArgs {
    #[arg(
        long,
        value_name = "HEX[,HEX...]",
        value_delimiter = ',',
        num_args = 1..,
        help = "Comma-delimited record roots to aggregate."
    )]
    pub(crate) record_roots: Vec<HashArg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct RecordFileArgs {
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        help = "File containing supporting records to summarize."
    )]
    pub(crate) record_file: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum PublicEvidenceRecordKindArg {
    BlockHistory,
    FinalityHistory,
    NetworkRuntime,
    RandomnessBeacon,
    DataAvailability,
    InvalidWork,
    RewardSettlement,
    DetectionMeasurement,
    ValidatorVrfLifecycle,
}

impl From<PublicEvidenceRecordKindArg> for PublicEvidenceRecordKind {
    fn from(kind: PublicEvidenceRecordKindArg) -> Self {
        match kind {
            PublicEvidenceRecordKindArg::BlockHistory => Self::BlockHistory,
            PublicEvidenceRecordKindArg::FinalityHistory => Self::FinalityHistory,
            PublicEvidenceRecordKindArg::NetworkRuntime => Self::NetworkRuntimeObservations,
            PublicEvidenceRecordKindArg::RandomnessBeacon => Self::RandomnessBeaconEvidence,
            PublicEvidenceRecordKindArg::DataAvailability => Self::DataAvailabilityMeasurements,
            PublicEvidenceRecordKindArg::InvalidWork => Self::InvalidWorkRejections,
            PublicEvidenceRecordKindArg::RewardSettlement => Self::RewardSettlements,
            PublicEvidenceRecordKindArg::DetectionMeasurement => Self::DetectionMeasurements,
            PublicEvidenceRecordKindArg::ValidatorVrfLifecycle => Self::ValidatorVrfLifecycle,
        }
    }
}
