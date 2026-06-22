use super::*;

fn hex_bytes(input: &str) -> Vec<u8> {
    assert_eq!(input.len() % 2, 0);
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("hex high nibble");
            let low = (pair[1] as char).to_digit(16).expect("hex low nibble");
            ((high << 4) | low) as u8
        })
        .collect()
}

#[test]
fn validation_seed_is_bound_to_finalized_randomness_and_receipt() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let chain = Chain::new(beacon);
    let receipt_a = hash_bytes(b"test", &[b"receipt-a"]);
    let receipt_b = hash_bytes(b"test", &[b"receipt-b"]);
    let validator = address(b"seed-validator");
    assert_ne!(
        chain.validation_seed(&receipt_a, &validator),
        chain.validation_seed(&receipt_b, &validator)
    );

    let other_chain = Chain::new(hash_bytes(b"test", &[b"other-beacon"]));
    assert_ne!(
        chain.validation_seed(&receipt_a, &validator),
        other_chain.validation_seed(&receipt_a, &validator)
    );
}

#[test]
fn validation_seed_is_bound_to_validator_and_beacon_round() {
    let beacon = hash_bytes(b"test", &[b"round-beacon"]);
    let mut chain = Chain::new(beacon);
    let validator_a = address(b"round-validator-a");
    let validator_b = address(b"round-validator-b");
    chain.register_validator(validator_a, 10_000).unwrap();
    chain.register_validator(validator_b, 10_000).unwrap();
    let receipt = hash_bytes(b"test", &[b"round-receipt"]);

    assert_ne!(
        chain.validation_seed(&receipt, &validator_a),
        chain.validation_seed(&receipt, &validator_b)
    );

    let seed_before = chain.validation_seed(&receipt, &validator_a);
    let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
    chain.produce_block(proposer, 1_000).unwrap();
    assert_ne!(seed_before, chain.validation_seed(&receipt, &validator_a));
    assert_eq!(chain.state().finalized_beacon_round(), 1);
}

#[test]
fn external_randomness_beacon_command_advances_receipt_anchor_source() {
    let genesis_beacon = hash_bytes(b"test", &[b"external-randomness-genesis"]);
    let external_beacon = hash_bytes(b"test", &[b"external-randomness-round-7"]);
    let proof_hash = hash_bytes(b"test", &[b"external-randomness-proof"]);
    let mut chain = Chain::new(genesis_beacon);
    let events = chain
        .apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 7,
            randomness: external_beacon,
            proof_hash,
        })
        .unwrap();
    assert_eq!(
        events,
        vec![ChainEvent::ExternalRandomnessBeaconAccepted {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 7,
            randomness: external_beacon,
        }]
    );
    assert_eq!(chain.state().finalized_beacon_round(), 7);
    assert_eq!(chain.state().finalized_randomness(), external_beacon);
    let record = chain
        .state()
        .external_randomness_beacons()
        .get(&7)
        .expect("external beacon should be recorded");
    assert_eq!(record.source_id, "drand-mainnet-round-v1");
    assert_eq!(record.randomness, external_beacon);
    assert_eq!(record.proof_hash, proof_hash);
    assert_eq!(
        record.proof,
        ExternalRandomnessBeaconProof::LocalDeterministicFixtureV1
    );
    let evidence = chain.state().randomness_binding_evidence();
    assert_eq!(evidence.external_beacon_record_count, 1);
    assert_eq!(evidence.latest_external_beacon_round, 7);

    let miner = address(b"external-randomness-miner");
    chain.register_miner(miner, 100).unwrap();
    let job = MatmulJob::synthetic(7, 0, 4, 4, 4, &external_beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let anchor = chain
        .state()
        .receipt_randomness_anchors()
        .get(&receipt_id)
        .expect("receipt should anchor to current external beacon");
    assert_eq!(anchor.beacon_round, 7);
    assert_eq!(anchor.finalized_randomness, external_beacon);
}

#[test]
fn verified_drand_beacon_command_derives_and_records_randomness() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"verified-drand-genesis"]));
    let public_key = hex_bytes(
        "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11",
    );
    let signature = hex_bytes(
        "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433",
    );
    let expected_randomness: Hash =
        hex_bytes("f3d6adf1daa2c7877f90fb0f1a675ab0a42653a1e2a9b66fee0749d47a47bc57")
            .try_into()
            .unwrap();

    let events = chain
        .apply_command(ChainCommand::SubmitVerifiedDrandBeacon {
            source_id: "drand-testnet-unchained".to_owned(),
            beacon_round: 223_344,
            public_key: public_key.clone(),
            signature: signature.clone(),
        })
        .unwrap();

    assert_eq!(
        events,
        vec![ChainEvent::ExternalRandomnessBeaconAccepted {
            source_id: "drand-testnet-unchained".to_owned(),
            beacon_round: 223_344,
            randomness: expected_randomness,
        }]
    );
    assert_eq!(chain.state().finalized_beacon_round(), 223_344);
    assert_eq!(chain.state().finalized_randomness(), expected_randomness);
    let record = chain
        .state()
        .external_randomness_beacons()
        .get(&223_344)
        .expect("verified drand beacon should be recorded");
    assert_eq!(record.randomness, expected_randomness);
    assert_ne!(record.proof_hash, [0; 32]);
    assert_eq!(
        record.proof,
        ExternalRandomnessBeaconProof::DrandPedersenBlsUnchainedV1 {
            public_key_hash: hash_bytes(
                b"tensor-vm-drand-pedersen-bls-unchained-public-key-v1",
                &[&public_key],
            ),
            signature_hash: hash_bytes(
                b"tensor-vm-drand-pedersen-bls-unchained-signature-v1",
                &[&signature],
            ),
            public_key_len: 48,
            signature_len: 96,
        }
    );
}

#[test]
fn verified_drand_beacon_command_rejects_wrong_round_and_signature() {
    let public_key = hex_bytes(
        "8200fc249deb0148eb918d6e213980c5d01acd7fc251900d9260136da3b54836ce125172399ddc69c4e3e11429b62c11",
    );
    let signature = hex_bytes(
        "94f6b85df7cce7237e8e7df66d794ddad092de5d8bb6a791b97e905aa89852e506ac36a792eba7021e22eebf34891f8914bf9a8dd9233ea0a4c5ca00ef8404999f899073dd2eade61fe54077fee8168f83dcb61a758b6883b38904054e64a433",
    );
    let wrong_signature = hex_bytes(
        "86ecea71376e78abd19aaf0ad52f462a6483626563b1023bd04815a7b953da888c74f5bf6ee672a5688603ab310026230522898f33f23a7de363c66f90ffd49ec77ebf7f6c1478a9ecd6e714b4d532ab43d044da0a16fed13b4791d7fc999e2b",
    );

    let mut wrong_round_chain = Chain::new(hash_bytes(b"test", &[b"wrong-drand-round"]));
    assert_eq!(
        wrong_round_chain.apply_command(ChainCommand::SubmitVerifiedDrandBeacon {
            source_id: "drand-testnet-unchained".to_owned(),
            beacon_round: 223_343,
            public_key: public_key.clone(),
            signature: signature.clone(),
        }),
        Err(TvmError::InvalidReceipt(
            "drand signature verification failed"
        ))
    );

    let mut wrong_signature_chain = Chain::new(hash_bytes(b"test", &[b"wrong-drand-signature"]));
    assert_eq!(
        wrong_signature_chain.apply_command(ChainCommand::SubmitVerifiedDrandBeacon {
            source_id: "drand-testnet-unchained".to_owned(),
            beacon_round: 223_344,
            public_key,
            signature: wrong_signature,
        }),
        Err(TvmError::InvalidReceipt(
            "drand signature verification failed"
        ))
    );
}

#[test]
fn verified_chained_drand_beacon_command_derives_public_default_randomness() {
    let mut chain = Chain::new(hash_bytes(b"test", &[b"verified-chained-drand"]));
    let public_key = hex_bytes(
        "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31",
    );
    let signature = hex_bytes(
        "8d61d9100567de44682506aea1a7a6fa6e5491cd27a0a0ed349ef6910ac5ac20ff7bc3e09d7c046566c9f7f3c6f3b10104990e7cb424998203d8f7de586fb7fa5f60045417a432684f85093b06ca91c769f0e7ca19268375e659c2a2352b4655",
    );
    let previous_signature =
        hex_bytes("176f93498eac9ca337150b46d21dd58673ea4e3581185f869672e59fa4cb390a");
    let source_id = verified_chained_drand_source_id(&public_key);
    let expected = verified_chained_drand_beacon_record(
        source_id.clone(),
        1,
        &public_key,
        &signature,
        &previous_signature,
        0,
    )
    .unwrap();

    let events = chain
        .apply_command(ChainCommand::SubmitVerifiedChainedDrandBeacon {
            source_id: source_id.clone(),
            beacon_round: 1,
            public_key: public_key.clone(),
            signature: signature.clone(),
            previous_signature: previous_signature.clone(),
        })
        .unwrap();

    assert_eq!(
        events,
        vec![ChainEvent::ExternalRandomnessBeaconAccepted {
            source_id: source_id.clone(),
            beacon_round: 1,
            randomness: expected.randomness,
        }]
    );
    let record = chain
        .state()
        .external_randomness_beacons()
        .get(&1)
        .expect("verified chained drand beacon should be recorded");
    assert_eq!(record.randomness, expected.randomness);
    assert_eq!(record.proof_hash, expected.proof_hash);
    assert!(matches!(
        record.proof,
        ExternalRandomnessBeaconProof::DrandPedersenBlsChainedV1 { .. }
    ));
}

#[test]
fn external_randomness_beacon_command_rejects_stale_and_empty_records() {
    let genesis_beacon = hash_bytes(b"test", &[b"external-randomness-reject"]);
    let mut chain = Chain::new(genesis_beacon);
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 0,
            randomness: hash_bytes(b"test", &[b"stale"]),
            proof_hash: hash_bytes(b"test", &[b"stale-proof"]),
        }),
        Err(TvmError::InvalidReceipt(
            "external randomness beacon round is not newer"
        ))
    );
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: "drand-mainnet-round-v1".to_owned(),
            beacon_round: 1,
            randomness: [0; 32],
            proof_hash: hash_bytes(b"test", &[b"empty-proof"]),
        }),
        Err(TvmError::InvalidReceipt(
            "external randomness beacon value is empty"
        ))
    );
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitExternalRandomnessBeacon {
            source_id: String::new(),
            beacon_round: 1,
            randomness: hash_bytes(b"test", &[b"bad-source"]),
            proof_hash: hash_bytes(b"test", &[b"bad-source-proof"]),
        }),
        Err(TvmError::InvalidReceipt(
            "external randomness source id out of bounds"
        ))
    );
}

#[test]
fn validator_vrf_reveal_records_are_chain_verified_and_state_rooted() {
    let beacon = hash_bytes(b"test", &[b"validator-vrf-reveal-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"validator-vrf-reveal-miner");
    let validator = address(b"validator-vrf-reveal-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(17, 0, 4, 4, 4, &beacon, 10);
    let job_id = job.job_id;
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let root_before = chain.state_root();
    let reveal = validation::validator_vrf_reveal_record(&chain, receipt_id, validator, 0).unwrap();
    let expected_output = chain.validation_seed(&receipt_id, &validator);
    assert_eq!(reveal.job_id, job_id);
    assert_eq!(reveal.vrf_output, expected_output);

    let events = chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal.clone()))
        .unwrap();
    assert_eq!(
        events,
        vec![ChainEvent::ValidatorVrfRevealAccepted {
            reveal_id: reveal.reveal_id,
            receipt_id,
            validator,
            beacon_round: reveal.beacon_round,
        }]
    );
    assert_ne!(chain.state_root(), root_before);
    assert_eq!(
        chain.state().validator_vrf_reveals().get(&reveal.reveal_id),
        Some(&ValidatorVrfRevealRecord {
            observed_at_height: 0,
            ..reveal.clone()
        })
    );
    assert_eq!(
        chain
            .state()
            .randomness_binding_evidence()
            .validator_vrf_reveal_count,
        1
    );
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal)),
        Err(TvmError::InvalidReceipt("duplicate validator vrf reveal"))
    );
}

#[test]
fn validator_vrf_reveal_rejects_tampered_binding_fields() {
    let beacon = hash_bytes(b"test", &[b"validator-vrf-reveal-reject"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"validator-vrf-reject-miner");
    let validator = address(b"validator-vrf-reject-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(18, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let reveal = validation::validator_vrf_reveal_record(&chain, receipt_id, validator, 0).unwrap();

    let mut bad_output = reveal.clone();
    bad_output.vrf_output = hash_bytes(b"test", &[b"bad-vrf-output"]);
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(bad_output)),
        Err(TvmError::InvalidReceipt(
            "validator vrf reveal output mismatch"
        ))
    );

    let mut bad_proof = reveal.clone();
    bad_proof.proof_hash = hash_bytes(b"test", &[b"bad-vrf-proof"]);
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(bad_proof)),
        Err(TvmError::InvalidReceipt(
            "validator vrf reveal proof mismatch"
        ))
    );

    let mut bad_signature = reveal;
    bad_signature.signature = [7; 32];
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(bad_signature)),
        Err(TvmError::InvalidReceipt(
            "bad validator vrf reveal signature"
        ))
    );
}

#[test]
fn keyed_validator_vrf_reveal_requires_production_proof() {
    let beacon = hash_bytes(b"test", &[b"validator-vrf-production"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"validator-vrf-production-miner");
    let validator = address(b"validator-vrf-production-validator");
    let secret = "validator-vrf-production-secret";
    let public_key = validation::validator_vrf_ed25519_public_key_from_secret(secret);
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    chain
        .register_validator_vrf_key(validator, public_key)
        .unwrap();

    let job = MatmulJob::synthetic(19, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let helper_reveal = validation::validator_vrf_reveal_record(&chain, receipt_id, validator, 0)
        .expect("legacy helper can still build a local reveal");
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(helper_reveal)),
        Err(TvmError::InvalidReceipt(
            "validator vrf public key mismatch"
        ))
    );

    let reveal = validation::validator_vrf_reveal_record_with_secret(
        &chain, receipt_id, validator, 0, secret,
    )
    .unwrap();
    assert_eq!(reveal.vrf_public_key, public_key);
    assert_eq!(
        reveal.vrf_proof.len(),
        validation::VALIDATOR_VRF_ED25519_PROOF_BYTES
    );
    let events = chain
        .apply_command(ChainCommand::SubmitValidatorVrfReveal(reveal.clone()))
        .unwrap();
    assert_eq!(
        events,
        vec![ChainEvent::ValidatorVrfRevealAccepted {
            reveal_id: reveal.reveal_id,
            receipt_id,
            validator,
            beacon_round: reveal.beacon_round,
        }]
    );

    let mut bad_proof = reveal;
    bad_proof.vrf_proof[0] ^= 1;
    assert_eq!(
        chain.apply_command(ChainCommand::SubmitValidatorVrfReveal(bad_proof)),
        Err(TvmError::InvalidReceipt("bad validator vrf reveal proof"))
    );
}

#[test]
fn admitted_receipt_validation_randomness_is_anchored_at_submission() {
    let beacon = hash_bytes(b"test", &[b"anchored-receipt-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"anchored-receipt-miner");
    let validator_a = address(b"anchored-receipt-validator-a");
    let validator_b = address(b"anchored-receipt-validator-b");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator_a, 10_000).unwrap();
    chain.register_validator(validator_b, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let job_id = job.job_id;
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let anchor = chain
        .state()
        .receipt_randomness_anchors()
        .get(&receipt_id)
        .expect("receipt admission should anchor validation randomness");
    assert_eq!(anchor.receipt_id, receipt_id);
    assert_eq!(anchor.beacon_round, 0);
    assert_eq!(anchor.finalized_randomness, beacon);
    assert_eq!(
        anchor.assignment_seed,
        chain.validator_assignment_seed(&receipt_id)
    );
    assert_eq!(
        anchor.validation_seed_commitment,
        validation::validation_seed_commitment(0, &beacon, &receipt_id)
    );
    let assigned_before = JobScheduler::default()
        .assign_validators(
            &chain,
            receipt_id,
            &chain.validator_assignment_seed(&receipt_id),
        )
        .validators;
    let seed_before = chain.validation_seed(&receipt_id, &validator_a);
    assert_eq!(
        seed_before,
        validation::committed_seed(
            &anchor.validation_seed_commitment,
            &receipt_id,
            &job_id,
            &validator_a,
            0
        )
    );

    chain.produce_block(validator_a, 1_000).unwrap();
    assert_eq!(chain.state().finalized_beacon_round(), 1);
    assert_ne!(chain.state().finalized_randomness(), beacon);

    assert_eq!(
        chain.validation_seed(&receipt_id, &validator_a),
        seed_before
    );
    assert_eq!(
        JobScheduler::default()
            .assign_validators(
                &chain,
                receipt_id,
                &chain.validator_assignment_seed(&receipt_id)
            )
            .validators,
        assigned_before
    );
    let later_job = MatmulJob::synthetic(1, 0, 4, 4, 4, &chain.state().finalized_randomness(), 10);
    let (later_receipt, _a, _b, _c) = TensorOpReceipt::from_job(&later_job, miner, 1, 3).unwrap();
    let later_receipt_id = later_receipt.receipt_id;
    chain.submit_job(JobState::TensorOp(later_job));
    chain.submit_tensor_op_receipt(later_receipt).unwrap();
    assert_eq!(
        chain
            .state()
            .receipt_randomness_anchors()
            .get(&later_receipt_id)
            .unwrap()
            .beacon_round,
        1
    );
    assert_ne!(
        chain.validator_assignment_seed(&receipt_id),
        chain.validator_assignment_seed(&later_receipt_id)
    );
}

#[test]
fn admitted_receipt_attestation_requires_randomness_anchor() {
    let beacon = hash_bytes(b"test", &[b"missing-anchor-beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"missing-anchor-miner");
    let validator = address(b"missing-anchor-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    let receipt_id = receipt.receipt_id;
    let job_id = receipt.job_id;
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();
    let anchored_seed = chain.validation_seed(&receipt_id, &validator);
    chain.remove_receipt_randomness_anchor_for_testing(&receipt_id);

    assert_ne!(
        chain.validation_seed(&receipt_id, &validator),
        anchored_seed
    );
    assert_eq!(
        chain.submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id,
                job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"missing-anchor-checks"]),
                data_availability_passed: true,
            },
        )),
        Err(TvmError::InvalidReceipt(
            "receipt randomness anchor missing"
        ))
    );
}

#[test]
fn randomness_binding_evidence_reports_receipt_bound_finalized_beacon_policy() {
    let beacon = hash_bytes(b"test", &[b"randomness-binding-evidence"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"randomness-binding-miner");
    let validator = address(b"randomness-binding-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 0, 3).unwrap();
    chain.submit_job(JobState::TensorOp(job));
    chain.submit_tensor_op_receipt(receipt).unwrap();

    let evidence = chain.state().randomness_binding_evidence();
    assert_eq!(evidence.beacon_source, validation::RANDOMNESS_BEACON_SOURCE);
    assert_eq!(
        evidence.drand_round_mapping,
        validation::RANDOMNESS_DRAND_ROUND_MAPPING
    );
    assert_ne!(
        evidence.drand_round_mapping,
        "not_configured_local_finalized_beacon"
    );
    assert_eq!(
        evidence.vrf_construction,
        validation::RANDOMNESS_VRF_CONSTRUCTION
    );
    assert_ne!(
        evidence.vrf_construction,
        "not_configured_local_finalized_beacon"
    );
    assert_eq!(
        evidence.assignment_seed_domain,
        validation::ASSIGNMENT_SEED_DOMAIN
    );
    assert_eq!(
        evidence.validation_seed_commitment_domain,
        validation::VALIDATION_SEED_COMMITMENT_DOMAIN
    );
    assert_eq!(
        evidence.validation_seed_reveal_domain,
        validation::VALIDATION_SEED_REVEAL_DOMAIN
    );
    assert!(!evidence.current_block_hash_randomness_allowed);
    assert_eq!(evidence.receipt_anchor_count, 1);
    assert_eq!(evidence.finalized_beacon_anchor_count, 1);
    assert_eq!(evidence.finalized_beacon_round_mapping_count, 1);
    assert_eq!(evidence.validator_vrf_seed_count, 1);
    assert_eq!(evidence.receipt_bound_anchor_count, 1);
    assert_eq!(evidence.consistent_anchor_count, 1);
    assert_eq!(evidence.current_block_hash_anchor_count, 0);
    assert!(evidence.all_receipt_anchors_consistent);

    chain.produce_block(validator, 1_000).unwrap();
    let later_evidence = chain.state().randomness_binding_evidence();
    assert_eq!(later_evidence.receipt_anchor_count, 1);
    assert_eq!(later_evidence.finalized_beacon_round_mapping_count, 1);
    assert_eq!(later_evidence.validator_vrf_seed_count, 1);
    assert_eq!(later_evidence.consistent_anchor_count, 1);
    assert!(later_evidence.all_receipt_anchors_consistent);
}

#[test]
fn proposer_selection_uses_validator_stake() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"validator");
    chain.register_validator(validator, 10_000).unwrap();
    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn fallback_proposer_handles_zero_stake_validator_records() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"zero-stake-validator");
    chain.register_validator(validator, 10_000).unwrap();
    chain.set_validator_stake_for_testing(validator, 0).unwrap();

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn proposer_selection_ignores_tensorwork() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"settled-miner");
    let validator = address(b"validator-proposer");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    chain
        .set_miner_tensor_work_for_testing(miner, 1_000_000, 1_000_000)
        .unwrap();

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    assert_eq!(
        chain.produce_block(miner, 1_000),
        Err(TvmError::UnknownValidator)
    );
}
