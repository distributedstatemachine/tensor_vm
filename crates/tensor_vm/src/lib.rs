//! Reference implementation of the TensorVM MVP.
//!
//! The crate focuses on deterministic local/testnet semantics: finite-field
//! tensors, Merkle commitments, Freivalds verification, linear training-step
//! checks, receipts, attestations, rewards, and the current reference block
//! path. The reviewed v2 MVP replaces settled TensorWork proposer selection
//! with useful-verification PoW; see the docs for the remaining consensus gap.

pub mod api;
pub mod app;
pub mod chain;
pub mod challenge;
mod cli;
mod codec;
pub mod conformance;
pub mod error;
pub mod faucet;
pub mod field;
pub mod hash;
pub mod ir;
pub mod jobs;
pub mod localnet;
pub mod merkle;
pub mod miner;
pub mod node;
pub mod oracle;
pub mod p2p;
pub mod profile;
mod record_fields;
pub mod roles;
pub mod rpc;
pub mod runtime;
pub mod scheduler;
pub mod storage;
pub mod study;
pub mod telemetry;
pub mod tensor;
pub mod tensor_server;
pub mod testnet;
pub mod txpool;
pub mod types;
pub mod validator;
pub mod verify;
pub mod vm;
pub mod watcher;

pub use chain::{
    AccountState, BlockVote, BlockspaceCaps, BlockspaceSelection, Chain, ChainCommand, ChainEngine,
    ChainEvent, ChainParams, ChainState, DeterministicBlockCheckChallenge, HardwareClass, JobState,
    MinerState, ReceiptState, RewardAllocation, RewardState, ValidatorState,
    verified_chained_drand_beacon_record, verified_chained_drand_source_id,
    verified_drand_beacon_record, verified_drand_source_id,
};
pub use challenge::{
    ChallengeOutcome, FraudChallenge, TensorOpChallengeInput, TraceBisectionConfig,
    TraceBisectionRound, TraceBisectionState, TraceBisectionStep, TraceStep,
};
pub use cli::{
    TvmdCli, validate_public_evidence_manifest, validate_public_testnet_preflight_manifest,
};
pub use conformance::{
    ConformanceProfile, ConformanceVector, conformance_suite_hash, conformance_vectors,
    cpu_reference_conformance_profile, ensure_linear_training_step_conformance,
    ensure_tensor_op_job_conformance,
};
pub use error::{Result, TvmError};
pub use faucet::Faucet;
pub use ir::{
    GraphId, GraphOutput, IrArity, IrLiteral, IrOpTier, IrRef, IrValue, IrVerificationClass,
    OpNode, OpSpec, ParamSpec, TensorGraph, TensorSpec, canonical_linear_training_step_graph,
    canonical_matmul_graph, frozen_op_registry, op_spec,
};
pub use jobs::{
    LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
    PrimitiveType, TensorOpReceipt,
};
#[cfg(test)]
pub use localnet::finalize_local_cpu_block;
pub use localnet::{produce_synthetic_cpu_round, produce_synthetic_cpu_round_with_profile};
pub use miner::MinerNode;
pub use node::{
    ChainNetworkPayloadProcessor, NetworkEventContext, NetworkEventIngest, NetworkPayloadApply,
    NetworkPayloadProcessor, NodeRuntimeState, PendingNetworkPayloads, ingest_network_messages,
    network_ingest_order,
};
pub use p2p::{
    GossipTopic, Libp2pControlPlaneConfig, NetworkStackRecommendation, PeerBookStore, PeerRecord,
    RequestResponseProtocol, TensorVmLibp2pNode, TensorVmLibp2pService, TensorVmLibp2pServiceInfo,
    TensorVmNetworkBehaviour, build_libp2p_node, decode_attestation_payload, decode_block_payload,
    decode_block_payload_with_selected_receipts, decode_block_vote_payload,
    decode_external_randomness_beacon_payload, decode_job_payload, decode_message,
    decode_receipt_payload, decode_tensor_payload, decode_validator_audit_report_payload,
    decode_validator_vrf_reveal_payload, decode_verified_chained_drand_beacon_payload,
    decode_verified_drand_beacon_payload, encode_attestation_payload, encode_block_payload,
    encode_block_payload_with_selected_receipts, encode_block_vote_payload,
    encode_external_randomness_beacon_payload, encode_gossipsub_message, encode_job_payload,
    encode_message, encode_receipt_payload, encode_tensor_payload,
    encode_validator_audit_report_payload, encode_validator_vrf_reveal_payload,
    encode_verified_chained_drand_beacon_payload, encode_verified_drand_beacon_payload,
    gossip_topic_for_message, gossipsub_ident_topic, recommended_network_stack,
    request_response_protocol_for_message, request_response_stream_protocol, spawn_libp2p_service,
};
pub use profile::{
    ChainNetwork, ChainProfile, NetworkConfig, NodeConfig, NodeRole, ServiceExposure, StorageConfig,
};
pub use roles::{
    CpuReferenceMinerRole, ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle,
    primitive_type, validator_stake,
};
pub use rpc::{RpcGateway, RpcHttpServer, RpcNode, RpcPolicy, RpcRequest, RpcResponse};
pub use runtime::{
    BackendKind, CpuReferenceBackend, ExecutionBackend, GpuMinerBackend,
    backend_conformance_profile, cuda_device_count, cuda_kernels_compiled,
};
pub use scheduler::{
    JobScheduler, JobSource, MinerAssignment, SyntheticLocalJobSource, ValidatorAssignment,
};
pub use storage::{
    BlockLogStore, ChainSnapshot, ChainStateStore, ChainStore, NodeStore, NodeStoreStatus,
    PersistedNodeState, SnapshotStore,
};
pub use study::{
    CollusionRiskAssessment, CollusionRiskInput, DataWithholdingStudy, FreivaldsSecurity,
    RandomnessAssessment, RandomnessSource, RowSamplingStudy, TensorWorkConcentrationStudy,
    ThreatActor, ThreatActorKind, ThreatModel, VerificationCostStudy, ZeroWorkLivenessStudy,
    assess_randomness, collusion_risk_assessment, data_withholding_study, freivalds_security,
    matmul_verification_cost_study, row_sampling_study, tensorwork_concentration,
    zero_work_liveness_study,
};
pub use tensor::{DType, Layout, Tensor, TensorDescriptor, TensorOpening};
pub use tensor_server::TensorServer;
pub use tensor_vm_explorer::{
    ExplorerAccount, ExplorerBlock, ExplorerJob, ExplorerMiner, ExplorerOverview,
    ExplorerPendingReward, ExplorerRandomnessBindingEvidence, ExplorerReceipt, ExplorerSummary,
    ExplorerValidator, ExplorerValidatorAuditEconomicCalibration,
};
pub use testnet::{
    LocalTestnet, PublicDeploymentServicePlan, PublicEvidenceAuditorRecord,
    PublicEvidencePublication, PublicEvidenceSupportingArtifact, PublicNetworkRuntimeEvidence,
    PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation,
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind,
    PublicTestnetCriteria, PublicTestnetEvidence, PublicTestnetEvidenceBundle,
    PublicTestnetEvidenceBundleReport, PublicTestnetPreflightPlan, PublicTestnetPreflightReport,
    PublicTestnetRunEvidence, TestnetConfig, parse_public_testnet_evidence_manifest,
    parse_public_testnet_preflight_manifest,
};
pub use txpool::TxPool;
pub use types::{Address, Hash, Signature};
pub use validator::{MatmulVerificationInput, ValidatorNode};
pub use verify::{
    AttestationStatement, FreivaldsParams, LinearVerificationReport, TensorOpVerificationReport,
    ValidatorAttestation, VerificationResult, full_freivalds, row_sample_detection_probability,
    row_sampled_freivalds, verify_linear_training_step, verify_tensor_op,
};
pub use watcher::{ChainWatcher, WatchEvent, WatchEventKind, WatchReport, WatcherConfig};
