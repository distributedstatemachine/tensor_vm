use super::{execute_test_cli_args, parse_test_cli};
use crate::chain::{
    Chain, ChainCommand, ChainEngine, JobState, validator_vrf_ed25519_public_key_from_secret,
    validator_vrf_reveal_record_with_secret,
};
use crate::hash::hex;
use crate::jobs::{MatmulJob, TensorOpReceipt};
use crate::storage::NodeStore;
use crate::types::{address, hash_bytes};
use libp2p::PeerId;
use std::path::PathBuf;

fn unique_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tensor-vm-cli-node-validation-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("test dir must be created");
    dir
}

fn assert_exported_records_summarize(kind: &str, record_text: &str) {
    let record_file = unique_test_dir("public-evidence-record-file").join("records.txt");
    std::fs::write(&record_file, record_text).unwrap();
    let record_file_text = record_file.to_string_lossy().into_owned();
    let bundle_id = hex(&hash_bytes(b"test", &[b"node-export-bundle"]));
    let manifest_signer = hex(&address(b"node-export-manifest-signer"));
    let summary = execute_test_cli_args(&[
        "public",
        "evidence",
        "record",
        "summary-file",
        "--kind",
        kind,
        "--bundle-id",
        &bundle_id,
        "--manifest-signer",
        &manifest_signer,
        "--record-file",
        &record_file_text,
    ])
    .unwrap();
    assert!(summary.contains("_records="));
    assert!(!summary.contains("_records=0"));
}

#[test]
fn local_node_cli_rejects_invalid_args() {
    assert!(execute_test_cli_args(&["node", "init", "--data-dir", " "]).is_err());
    assert!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            "not-a-peer-id",
            "--address",
            "/dns/bootstrap.tensorvm.net/tcp/4001",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            "/var/lib/tensorvm",
            "--peer-id",
            &PeerId::random().to_string(),
            "--address",
            "not-a-multiaddr",
        ])
        .is_err()
    );
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    let peer_data_dir = unique_test_dir("peer-mismatch");
    let peer_data_dir = peer_data_dir.to_string_lossy().into_owned();
    let mismatched_peer_address = format!("/dns/bootstrap.tensorvm.net/tcp/4001/p2p/{peer_b}");
    assert!(
        execute_test_cli_args(&[
            "node",
            "peer",
            "add",
            "--data-dir",
            &peer_data_dir,
            "--peer-id",
            &peer_a.to_string(),
            "--address",
            &mismatched_peer_address,
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "localhost:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "not-a-multiaddr",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            " ",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "not-a-multiaddr",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            " ",
            "--auth-token",
            "secret",
        ])
        .is_err()
    );
    assert!(
        execute_test_cli_args(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            " ",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "node",
            "serve",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "abc",
        ])
        .is_err()
    );
}

#[test]
fn local_node_exports_chain_accepted_randomness_evidence_records() {
    let data_dir = unique_test_dir("public-evidence-randomness");
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let mut chain = Chain::new(hash_bytes(b"test", &[b"node-export-genesis"]));
    let randomness = hash_bytes(b"test", &[b"node-export-randomness"]);
    let proof_hash = hash_bytes(b"test", &[b"node-export-proof"]);
    chain
        .apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 11,
            randomness,
            proof_hash,
        })
        .unwrap();
    NodeStore::open(&data_dir).persist_chain(&chain).unwrap();

    let output = execute_test_cli_args(&[
        "node",
        "export-public-evidence",
        "--data-dir",
        &data_dir_text,
        "--kind",
        "randomness-beacon",
    ])
    .unwrap();

    let source_id = hash_bytes(
        b"tensor-vm-public-randomness-source-id-v1",
        &[b"drand-mainnet-round-v1"],
    );
    assert_eq!(
        output,
        format!(
            "randomness_beacon_record={},11,{},{},local-deterministic-fixture-v1,0,accepted\n",
            hex(&source_id),
            hex(&randomness),
            hex(&proof_hash)
        )
    );
    assert_exported_records_summarize("randomness-beacon", &output);
}

#[test]
fn local_node_exports_chain_accepted_validator_vrf_lifecycle_records() {
    let data_dir = unique_test_dir("public-evidence-validator-vrf");
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let beacon = hash_bytes(b"test", &[b"node-export-validator-vrf-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"node-export-vrf-miner");
    let validator = address(b"node-export-vrf-validator");
    let secret = "node-export-vrf-secret";
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    chain
        .apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 13,
            randomness: hash_bytes(b"test", &[b"node-export-vrf-randomness"]),
            proof_hash: hash_bytes(b"test", &[b"node-export-vrf-proof"]),
        })
        .unwrap();
    chain
        .register_validator_vrf_key(
            validator,
            validator_vrf_ed25519_public_key_from_secret(secret),
        )
        .unwrap();

    let job = MatmulJob::synthetic(23, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let reveal =
        validator_vrf_reveal_record_with_secret(&chain, receipt_id, validator, 0, secret).unwrap();
    chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal.clone()))
        .unwrap();
    NodeStore::open(&data_dir).persist_chain(&chain).unwrap();

    let output = execute_test_cli_args(&[
        "node",
        "export-public-evidence",
        "--data-dir",
        &data_dir_text,
        "--kind",
        "validator-vrf-lifecycle",
    ])
    .unwrap();

    assert_eq!(
        output,
        format!(
            "validator_vrf_lifecycle={},{},{},committed,0\nvalidator_vrf_lifecycle={},{},{},revealed,0\n",
            hex(&receipt_id),
            hex(&validator),
            reveal.beacon_round,
            hex(&receipt_id),
            hex(&validator),
            reveal.beacon_round
        )
    );
    assert_exported_records_summarize("validator-vrf-lifecycle", &output);
}
