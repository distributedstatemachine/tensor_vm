use crate::types::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum P2pMessage {
    NewBlock(Hash),
    NewBlockHeader {
        height: u64,
        block_hash: Hash,
    },
    NewBlockPayload {
        height: u64,
        block_hash: Hash,
        payload: Vec<u8>,
    },
    NewBlockVotePayload {
        block_hash: Hash,
        validator: Address,
        payload: Vec<u8>,
    },
    NewBlockCheckChallenge(Hash),
    NewBlockCheckChallengePayload {
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Address,
        payload: Vec<u8>,
    },
    NewObservedBlockCheckChallengePayload {
        challenge_id: Hash,
        block_hash: Hash,
        challenger: Address,
        observed_block_payload: Vec<u8>,
        challenge_payload: Vec<u8>,
    },
    NewTraceBisectionRoundPayload {
        receipt_id: Hash,
        trace_root: Hash,
        challenger: Address,
        responder: Address,
        transcript_leaf: Hash,
        payload: Vec<u8>,
    },
    NewJob(Hash),
    NewJobPayload {
        job_id: Hash,
        payload: Vec<u8>,
    },
    NewReceipt(Hash),
    NewReceiptPayload {
        receipt_id: Hash,
        payload: Vec<u8>,
    },
    NewAttestation(Hash),
    NewAttestationPayload {
        attestation_id: Hash,
        payload: Vec<u8>,
    },
    NewValidatorAuditReport(Hash),
    NewValidatorAuditReportPayload {
        audit_id: Hash,
        auditor: Address,
        payload: Vec<u8>,
    },
    NewExternalRandomnessBeaconPayload {
        source_id: String,
        beacon_round: u64,
        payload: Vec<u8>,
    },
    NewVerifiedDrandBeaconPayload {
        source_id: String,
        beacon_round: u64,
        payload: Vec<u8>,
    },
    NewVerifiedChainedDrandBeaconPayload {
        source_id: String,
        beacon_round: u64,
        payload: Vec<u8>,
    },
    NewValidatorVrfRevealPayload {
        reveal_id: Hash,
        receipt_id: Hash,
        validator: Address,
        payload: Vec<u8>,
    },
    RequestTensorChunk {
        tensor_id: Hash,
        chunk_index: u64,
    },
    TensorChunkResponse {
        tensor_id: Hash,
        chunk_index: u64,
        bytes: Vec<u8>,
    },
    RequestTensorRow {
        tensor_id: Hash,
        row_index: u64,
    },
    TensorRowResponse {
        tensor_id: Hash,
        row_index: u64,
        values: Vec<u64>,
    },
    RequestTensorByCommitmentRoot {
        commitment_root: Hash,
    },
    TensorByCommitmentRootResponse {
        commitment_root: Hash,
        payload: Option<Vec<u8>>,
    },
    RequestProgram(Hash),
    ProgramResponse {
        program_hash: Hash,
        bytes: Vec<u8>,
    },
    RequestTraceOpening {
        trace_root: Hash,
        op_index: u64,
    },
    TraceOpeningResponse {
        trace_root: Hash,
        op_index: u64,
        payload: Option<Vec<u8>>,
    },
    PeerInfo {
        address: Address,
    },
}

pub const NODE_RPC_ROUTES: &[&str] = &[
    "GET /health",
    "GET /rpc/health",
    "GET /chain/head",
    "GET /chain/block/:height",
    "GET /epoch/current",
    "GET /jobs/current",
    "GET /jobs/:job_id",
    "GET /receipts/:receipt_id",
    "GET /miners/:address",
    "GET /validators/:address",
    "GET /explorer",
    "GET /explorer/health",
    "GET /explorer/summary",
    "GET /explorer/overview",
    "GET /explorer/miners",
    "GET /explorer/validators",
    "GET /explorer/jobs",
    "GET /explorer/account/:address",
    "GET /explorer/blocks/latest/:limit",
    "GET /explorer/receipts/latest/:limit",
    "WS /explorer/ws",
    "GET /telemetry",
    "GET /telemetry/health",
    "GET /telemetry/dashboard",
    "GET /faucet",
    "GET /faucet/health",
    "GET /faucet/page",
    "POST /faucet/claim/:address",
    "POST /tx",
    "POST /receipt",
    "POST /attestation",
];

pub const TENSOR_DATA_RPC_ROUTES: &[&str] = &[
    "GET /tensor/:tensor_id/descriptor",
    "GET /tensor/:tensor_id/chunk/:chunk_index",
    "GET /tensor/:tensor_id/row/:row_index",
    "GET /tensor/:tensor_id/opening/:chunk_index",
];

pub const MINER_CLI_COMMANDS: &[&str] = &[
    "tvmd miner register --stake <tokens>",
    "tvmd miner check --wallet <key> [--device cpu|cuda:N] [--node <libp2p-multiaddr>]",
    "tvmd miner run --wallet <key> --auth-token <token> [--device cpu|cuda:N] [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
    "tvmd miner status",
];

pub const VALIDATOR_CLI_COMMANDS: &[&str] = &[
    "tvmd validator register --stake <tokens>",
    "tvmd validator check --wallet <key> [--node <libp2p-multiaddr>]",
    "tvmd validator run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
    "tvmd validator status",
];

pub const PROPOSER_CLI_COMMANDS: &[&str] = &[
    "tvmd proposer run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
];

pub const SERVICE_CLI_COMMANDS: &[&str] = &[
    "tvmd node init [--data-dir <path>]",
    "tvmd node peer add --peer-id <peer-id> --address <libp2p-multiaddr> [--data-dir <path>]",
    "tvmd node check [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>]",
    "tvmd node serve --auth-token <token> [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
];

pub const PUBLIC_EVIDENCE_CLI_COMMANDS: &[&str] = &[
    "tvmd public evidence validate <path>",
    "tvmd public evidence publish --bundle-id <hex> --public-uri <uri> --manifest-signer <address-hex> --manifest-signature-count <n> --independent-auditor-count <n>",
    "tvmd public evidence audit --bundle-id <hex> --public-uri <uri> --auditor-id <address-hex> --audit-uri <uri> --observed-at <unix-seconds>",
    "tvmd public evidence run window --bundle-id <hex> --manifest-signer <address-hex> --started-at <unix-seconds> --ended-at <unix-seconds> --observed-blocks <n>",
    "tvmd public evidence run window-file --bundle-id <hex> --manifest-signer <address-hex> --block-observation-file <path>",
    "tvmd public evidence node heartbeat --role <miner|validator> --address <address-hex> --operator-id <hex> --first-block <n> --last-block <n> --heartbeat-count <n>",
    "tvmd public evidence node heartbeat-file --role <miner|validator> --address <address-hex> --operator-id <hex> --heartbeat-file <path>",
    "tvmd public evidence node operator-attestation --role <miner|validator> --address <address-hex> --operator-id <hex> --identity-uri <uri> --observed-at <unix-seconds>",
    "tvmd public evidence service health --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --first-block <n> --last-block <n> --reachable-count <n> --signed-health-check-count <n>",
    "tvmd public evidence service health-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --observation-file <path>",
    "tvmd public evidence service content --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --content-root <hex> --observed-at <unix-seconds> --min-content-bytes <n>",
    "tvmd public evidence service content-bytes --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-hex <hex-bytes>",
    "tvmd public evidence service content-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-file <path>",
    "tvmd public evidence network observation --operator-id <hex> --peer-id <peer-id> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --gossip-topics <n> --request-response-protocols <n> --bootstrap-peers <n> --max-transmit-bytes <n> --request-timeout-seconds <n> --max-concurrent-streams <n> --idle-timeout-seconds <n>",
    "tvmd public evidence network from-service-log --operator-id <hex> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --service-log <path>",
    "tvmd public evidence record summary --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-root <hex> --record-count <n>",
    "tvmd public evidence record artifact --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-root <hex> --record-count <n>",
    "tvmd public evidence record artifact-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-roots <comma-separated-roots>",
    "tvmd public evidence record artifact-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-file <path>",
    "tvmd public evidence record summary-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-roots <comma-separated-roots>",
    "tvmd public evidence record summary-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-file <path>",
];

pub const PUBLIC_TESTNET_CLI_COMMANDS: &[&str] = &["tvmd public preflight <path>"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_surface_includes_spec_routes() {
        assert_eq!(
            NODE_RPC_ROUTES,
            &[
                "GET /health",
                "GET /rpc/health",
                "GET /chain/head",
                "GET /chain/block/:height",
                "GET /epoch/current",
                "GET /jobs/current",
                "GET /jobs/:job_id",
                "GET /receipts/:receipt_id",
                "GET /miners/:address",
                "GET /validators/:address",
                "GET /explorer",
                "GET /explorer/health",
                "GET /explorer/summary",
                "GET /explorer/overview",
                "GET /explorer/miners",
                "GET /explorer/validators",
                "GET /explorer/jobs",
                "GET /explorer/account/:address",
                "GET /explorer/blocks/latest/:limit",
                "GET /explorer/receipts/latest/:limit",
                "WS /explorer/ws",
                "GET /telemetry",
                "GET /telemetry/health",
                "GET /telemetry/dashboard",
                "GET /faucet",
                "GET /faucet/health",
                "GET /faucet/page",
                "POST /faucet/claim/:address",
                "POST /tx",
                "POST /receipt",
                "POST /attestation",
            ]
        );
        assert_eq!(
            TENSOR_DATA_RPC_ROUTES,
            &[
                "GET /tensor/:tensor_id/descriptor",
                "GET /tensor/:tensor_id/chunk/:chunk_index",
                "GET /tensor/:tensor_id/row/:row_index",
                "GET /tensor/:tensor_id/opening/:chunk_index",
            ]
        );
        assert_eq!(
            MINER_CLI_COMMANDS,
            &[
                "tvmd miner register --stake <tokens>",
                "tvmd miner check --wallet <key> [--device cpu|cuda:N] [--node <libp2p-multiaddr>]",
                "tvmd miner run --wallet <key> --auth-token <token> [--device cpu|cuda:N] [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
                "tvmd miner status",
            ]
        );
        assert_eq!(
            VALIDATOR_CLI_COMMANDS,
            &[
                "tvmd validator register --stake <tokens>",
                "tvmd validator check --wallet <key> [--node <libp2p-multiaddr>]",
                "tvmd validator run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
                "tvmd validator status",
            ]
        );
        assert_eq!(
            PROPOSER_CLI_COMMANDS,
            &[
                "tvmd proposer run --wallet <key> --auth-token <token> [--node <libp2p-multiaddr>] [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
            ]
        );
        assert_eq!(
            SERVICE_CLI_COMMANDS,
            &[
                "tvmd node init [--data-dir <path>]",
                "tvmd node peer add --peer-id <peer-id> --address <libp2p-multiaddr> [--data-dir <path>]",
                "tvmd node check [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>]",
                "tvmd node serve --auth-token <token> [--listen <addr>] [--p2p-listen <libp2p-multiaddr>] [--data-dir <path>] [--max-requests <n>]",
            ]
        );
        assert_eq!(
            PUBLIC_EVIDENCE_CLI_COMMANDS,
            &[
                "tvmd public evidence validate <path>",
                "tvmd public evidence publish --bundle-id <hex> --public-uri <uri> --manifest-signer <address-hex> --manifest-signature-count <n> --independent-auditor-count <n>",
                "tvmd public evidence audit --bundle-id <hex> --public-uri <uri> --auditor-id <address-hex> --audit-uri <uri> --observed-at <unix-seconds>",
                "tvmd public evidence run window --bundle-id <hex> --manifest-signer <address-hex> --started-at <unix-seconds> --ended-at <unix-seconds> --observed-blocks <n>",
                "tvmd public evidence run window-file --bundle-id <hex> --manifest-signer <address-hex> --block-observation-file <path>",
                "tvmd public evidence node heartbeat --role <miner|validator> --address <address-hex> --operator-id <hex> --first-block <n> --last-block <n> --heartbeat-count <n>",
                "tvmd public evidence node heartbeat-file --role <miner|validator> --address <address-hex> --operator-id <hex> --heartbeat-file <path>",
                "tvmd public evidence node operator-attestation --role <miner|validator> --address <address-hex> --operator-id <hex> --identity-uri <uri> --observed-at <unix-seconds>",
                "tvmd public evidence service health --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --first-block <n> --last-block <n> --reachable-count <n> --signed-health-check-count <n>",
                "tvmd public evidence service health-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --health-path <path> --observation-file <path>",
                "tvmd public evidence service content --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --content-root <hex> --observed-at <unix-seconds> --min-content-bytes <n>",
                "tvmd public evidence service content-bytes --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-hex <hex-bytes>",
                "tvmd public evidence service content-file --kind <rpc|explorer|faucet|telemetry> --endpoint-id <hex> --public-url <url> --content-path <path> --observed-at <unix-seconds> --content-file <path>",
                "tvmd public evidence network observation --operator-id <hex> --peer-id <peer-id> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --gossip-topics <n> --request-response-protocols <n> --bootstrap-peers <n> --max-transmit-bytes <n> --request-timeout-seconds <n> --max-concurrent-streams <n> --idle-timeout-seconds <n>",
                "tvmd public evidence network from-service-log --operator-id <hex> --listen-address <public-libp2p-multiaddr> --observed-at <unix-seconds> --service-log <path>",
                "tvmd public evidence record summary --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-root <hex> --record-count <n>",
                "tvmd public evidence record artifact --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-root <hex> --record-count <n>",
                "tvmd public evidence record artifact-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-roots <comma-separated-roots>",
                "tvmd public evidence record artifact-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --artifact-uri <uri> --record-file <path>",
                "tvmd public evidence record summary-roots --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-roots <comma-separated-roots>",
                "tvmd public evidence record summary-file --kind <block-history|finality-history|network-runtime|data-availability|invalid-work|reward-settlement> --bundle-id <hex> --manifest-signer <address-hex> --record-file <path>",
            ]
        );
        assert_eq!(
            PUBLIC_TESTNET_CLI_COMMANDS,
            &["tvmd public preflight <path>"]
        );
    }
}
