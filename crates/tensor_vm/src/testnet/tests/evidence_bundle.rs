use super::*;

#[test]
fn public_testnet_evidence_bundle_requires_publication_and_audit_records() {
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let bundle = complete_public_evidence_bundle();

    let complete = bundle.evaluate(&criteria, 6);
    assert!(complete.run_evidence.public_criterion_met);
    assert!(complete.has_published_evidence_bundle);
    assert!(complete.has_independent_auditor_records);
    assert!(complete.has_signed_run_window);
    assert!(complete.has_block_history);
    assert!(complete.has_finality_history);
    assert!(complete.has_operator_identity_attestations);
    assert!(complete.has_network_runtime_observations);
    assert!(complete.has_randomness_beacon_evidence);
    assert!(complete.has_data_availability_measurements);
    assert!(complete.has_invalid_work_rejection_records);
    assert!(complete.has_reward_settlement_record_summary);
    assert!(complete.has_public_supporting_record_artifacts);
    assert!(complete.independently_checkable);
    assert!(!complete.full_spec_evidence_met);

    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(full_spec_report.run_evidence.public_criterion_met);
    assert!(full_spec_report.independently_checkable);
    assert!(full_spec_report.full_spec_evidence_met);

    let mut reused_artifact_uri = full_spec_bundle;
    let block_artifact_uri = reused_artifact_uri.supporting_artifacts[0]
        .artifact_uri
        .clone();
    reused_artifact_uri.supporting_artifacts[1].artifact_uri = block_artifact_uri;
    let finality_artifact = &mut reused_artifact_uri.supporting_artifacts[1];
    finality_artifact.artifact_signature = sign_public_evidence_artifact(
        &reused_artifact_uri.publication.manifest_signer,
        &reused_artifact_uri.publication.bundle_id,
        finality_artifact.kind,
        &finality_artifact.artifact_uri,
        &finality_artifact.record_root,
        finality_artifact.record_count,
    );
    let reused_artifact_report =
        reused_artifact_uri.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(reused_artifact_report.run_evidence.public_criterion_met);
    assert!(!reused_artifact_report.has_public_supporting_record_artifacts);
    assert!(!reused_artifact_report.independently_checkable);
    assert!(!reused_artifact_report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_randomness_records_for_full_run() {
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let complete = complete_public_evidence_bundle();
    assert_eq!(
        complete.randomness_beacon_records,
        complete.run.observed_blocks
    );
    assert!(
        complete
            .evaluate(&criteria, 6)
            .has_randomness_beacon_evidence
    );

    let mut undercounted_randomness = complete.clone();
    let randomness_root = undercounted_randomness.randomness_beacon_root;
    let undercounted_record_count = undercounted_randomness
        .run
        .observed_blocks
        .saturating_sub(1);
    resign_record_summary_and_artifact(
        &mut undercounted_randomness,
        PublicEvidenceRecordKind::RandomnessBeaconEvidence,
        randomness_root,
        undercounted_record_count,
    );
    let undercounted_report = undercounted_randomness.evaluate(&criteria, 6);
    assert!(!undercounted_report.has_randomness_beacon_evidence);
    assert!(undercounted_report.has_public_supporting_record_artifacts);
    assert!(!undercounted_report.independently_checkable);

    let mut overcounted_randomness = complete;
    let randomness_root = overcounted_randomness.randomness_beacon_root;
    let overcounted_record_count = overcounted_randomness.run.observed_blocks + 1;
    resign_record_summary_and_artifact(
        &mut overcounted_randomness,
        PublicEvidenceRecordKind::RandomnessBeaconEvidence,
        randomness_root,
        overcounted_record_count,
    );
    let overcounted_report = overcounted_randomness.evaluate(&criteria, 6);
    assert!(!overcounted_report.has_randomness_beacon_evidence);
    assert!(overcounted_report.has_public_supporting_record_artifacts);
    assert!(!overcounted_report.independently_checkable);
}

#[test]
fn public_testnet_evidence_bundle_requires_raw_randomness_records() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    assert!(
        full_spec_bundle
            .evaluate(&full_spec_criteria, full_spec_block_time)
            .full_spec_evidence_met
    );

    let mut missing_raw_randomness = full_spec_bundle.clone();
    missing_raw_randomness.randomness_beacon_raw_records.clear();
    let missing_raw_randomness_report =
        missing_raw_randomness.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(
        missing_raw_randomness_report
            .run_evidence
            .public_criterion_met
    );
    assert!(missing_raw_randomness_report.independently_checkable);
    assert!(!missing_raw_randomness_report.full_spec_evidence_met);

    let resign_randomness_records = |bundle: &mut PublicTestnetEvidenceBundle| {
        let randomness_roots = bundle
            .randomness_beacon_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>();
        let randomness_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            &randomness_roots,
        )
        .unwrap();
        let record_count = bundle.randomness_beacon_records;
        resign_record_summary_and_artifact(
            bundle,
            PublicEvidenceRecordKind::RandomnessBeaconEvidence,
            randomness_root,
            record_count,
        );
    };

    let mut duplicate_observed_block = full_spec_bundle.clone();
    duplicate_observed_block.randomness_beacon_raw_records[1].observed_block =
        duplicate_observed_block.randomness_beacon_raw_records[0].observed_block;
    resign_randomness_records(&mut duplicate_observed_block);
    let report = duplicate_observed_block.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut out_of_range_observed_block = full_spec_bundle.clone();
    out_of_range_observed_block.randomness_beacon_raw_records[1].observed_block =
        out_of_range_observed_block.run.observed_blocks;
    resign_randomness_records(&mut out_of_range_observed_block);
    let report = out_of_range_observed_block.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut duplicate_beacon_round = full_spec_bundle.clone();
    duplicate_beacon_round.randomness_beacon_raw_records[1].beacon_round =
        duplicate_beacon_round.randomness_beacon_raw_records[0].beacon_round;
    resign_randomness_records(&mut duplicate_beacon_round);
    let report = duplicate_beacon_round.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut local_fixture_randomness = full_spec_bundle;
    local_fixture_randomness.randomness_beacon_raw_records = (0..local_fixture_randomness
        .randomness_beacon_records)
        .map(|index| {
            PublicRandomnessBeaconRecord::local_fixture(
                hash_bytes(b"test", &[b"full-spec-local-randomness-source"]),
                index + 1,
                hash_bytes(
                    b"test",
                    &[format!("full-spec-local-randomness-{index}").as_bytes()],
                ),
                hash_bytes(
                    b"test",
                    &[format!("full-spec-local-randomness-proof-{index}").as_bytes()],
                ),
                index,
            )
        })
        .collect();
    let local_fixture_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::RandomnessBeaconEvidence,
        &local_fixture_randomness
            .randomness_beacon_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("local fixture randomness roots should aggregate");
    let local_fixture_record_count = local_fixture_randomness.randomness_beacon_records;
    resign_record_summary_and_artifact(
        &mut local_fixture_randomness,
        PublicEvidenceRecordKind::RandomnessBeaconEvidence,
        local_fixture_root,
        local_fixture_record_count,
    );
    let local_fixture_randomness_report =
        local_fixture_randomness.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(
        local_fixture_randomness_report
            .run_evidence
            .public_criterion_met
    );
    assert!(local_fixture_randomness_report.independently_checkable);
    assert!(!local_fixture_randomness_report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_raw_operational_records() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    assert!(
        full_spec_bundle
            .evaluate(&full_spec_criteria, full_spec_block_time)
            .full_spec_evidence_met
    );

    let mut missing_data_availability = full_spec_bundle.clone();
    missing_data_availability
        .data_availability_raw_records
        .clear();
    let report = missing_data_availability.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut missing_invalid_work = full_spec_bundle.clone();
    missing_invalid_work.invalid_work_raw_records.clear();
    let report = missing_invalid_work.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut missing_reward_settlement = full_spec_bundle.clone();
    missing_reward_settlement
        .reward_settlement_raw_records
        .clear();
    let report = missing_reward_settlement.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut duplicate_data_receipt = full_spec_bundle.clone();
    duplicate_data_receipt.data_availability_raw_records[1].receipt_root =
        duplicate_data_receipt.data_availability_raw_records[0].receipt_root;
    let duplicate_data_roots = duplicate_data_receipt
        .data_availability_raw_records
        .iter()
        .map(|record| record.record_root())
        .collect::<Vec<_>>();
    let duplicate_data_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::DataAvailabilityMeasurements,
        &duplicate_data_roots,
    )
    .unwrap();
    let duplicate_data_count = duplicate_data_receipt.data_availability_measurement_records;
    resign_record_summary_and_artifact(
        &mut duplicate_data_receipt,
        PublicEvidenceRecordKind::DataAvailabilityMeasurements,
        duplicate_data_root,
        duplicate_data_count,
    );
    let report = duplicate_data_receipt.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut duplicate_invalid_work_receipt = full_spec_bundle.clone();
    let mut second_invalid_work =
        duplicate_invalid_work_receipt.invalid_work_raw_records[0].clone();
    second_invalid_work.observed_block = second_invalid_work.observed_block.saturating_add(1);
    duplicate_invalid_work_receipt
        .invalid_work_raw_records
        .push(second_invalid_work);
    duplicate_invalid_work_receipt
        .run
        .invalid_receipts_submitted = 2;
    duplicate_invalid_work_receipt.run.invalid_receipts_rejected = 2;
    duplicate_invalid_work_receipt.invalid_work_rejection_records = 2;
    let duplicate_invalid_work_roots = duplicate_invalid_work_receipt
        .invalid_work_raw_records
        .iter()
        .map(|record| record.record_root())
        .collect::<Vec<_>>();
    let duplicate_invalid_work_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::InvalidWorkRejections,
        &duplicate_invalid_work_roots,
    )
    .unwrap();
    resign_record_summary_and_artifact(
        &mut duplicate_invalid_work_receipt,
        PublicEvidenceRecordKind::InvalidWorkRejections,
        duplicate_invalid_work_root,
        2,
    );
    let report = duplicate_invalid_work_receipt.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut zero_invalid_work_receipt = full_spec_bundle.clone();
    zero_invalid_work_receipt.invalid_work_raw_records[0].receipt_root = [0; 32];
    let zero_invalid_work_roots = zero_invalid_work_receipt
        .invalid_work_raw_records
        .iter()
        .map(|record| record.record_root())
        .collect::<Vec<_>>();
    let zero_invalid_work_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::InvalidWorkRejections,
        &zero_invalid_work_roots,
    )
    .unwrap();
    resign_record_summary_and_artifact(
        &mut zero_invalid_work_receipt,
        PublicEvidenceRecordKind::InvalidWorkRejections,
        zero_invalid_work_root,
        1,
    );
    let report = zero_invalid_work_receipt.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut duplicate_reward_settlement_receipt = full_spec_bundle.clone();
    let mut second_reward_settlement =
        duplicate_reward_settlement_receipt.reward_settlement_raw_records[0].clone();
    second_reward_settlement.observed_block =
        second_reward_settlement.observed_block.saturating_add(1);
    second_reward_settlement.miner_id = address(b"full-spec-reward-settlement-second-miner");
    second_reward_settlement.validator_id =
        address(b"full-spec-reward-settlement-second-validator");
    duplicate_reward_settlement_receipt
        .reward_settlement_raw_records
        .push(second_reward_settlement);
    duplicate_reward_settlement_receipt
        .run
        .reward_settlement_records = 2;
    let duplicate_reward_settlement_roots = duplicate_reward_settlement_receipt
        .reward_settlement_raw_records
        .iter()
        .map(|record| record.record_root())
        .collect::<Vec<_>>();
    let duplicate_reward_settlement_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::RewardSettlements,
        &duplicate_reward_settlement_roots,
    )
    .unwrap();
    resign_record_summary_and_artifact(
        &mut duplicate_reward_settlement_receipt,
        PublicEvidenceRecordKind::RewardSettlements,
        duplicate_reward_settlement_root,
        2,
    );
    let report =
        duplicate_reward_settlement_receipt.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut zero_reward_settlement_participant = full_spec_bundle.clone();
    zero_reward_settlement_participant.reward_settlement_raw_records[0].miner_id = [0; 32];
    let zero_reward_settlement_roots = zero_reward_settlement_participant
        .reward_settlement_raw_records
        .iter()
        .map(|record| record.record_root())
        .collect::<Vec<_>>();
    let zero_reward_settlement_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::RewardSettlements,
        &zero_reward_settlement_roots,
    )
    .unwrap();
    resign_record_summary_and_artifact(
        &mut zero_reward_settlement_participant,
        PublicEvidenceRecordKind::RewardSettlements,
        zero_reward_settlement_root,
        1,
    );
    let report =
        zero_reward_settlement_participant.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_data_root = full_spec_bundle;
    let record_count = mismatched_data_root.data_availability_measurement_records;
    resign_record_summary_and_artifact(
        &mut mismatched_data_root,
        PublicEvidenceRecordKind::DataAvailabilityMeasurements,
        hash_bytes(
            b"test",
            &[b"summary-root-not-derived-from-raw-data-availability"],
        ),
        record_count,
    );
    let report = mismatched_data_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let mut bundle = complete_public_evidence_bundle();

    let mut role_order_bundle = complete_public_evidence_bundle();
    let shared_node_address = address(b"bundle-role-order-shared-address");
    let shared_miner_operator = hash_bytes(b"test", &[b"bundle-role-order-shared-miner"]);
    let independent_miner_address = address(b"bundle-role-order-independent-miner-address");
    let independent_miner_operator = hash_bytes(b"test", &[b"bundle-role-order-independent-miner"]);
    let validator_operator = hash_bytes(b"test", &[b"bundle-role-order-validator"]);
    role_order_bundle.run.nodes = vec![
        PublicNodeEvidence::miner(shared_node_address, shared_miner_operator, 0, 9, 10),
        PublicNodeEvidence::miner(
            independent_miner_address,
            independent_miner_operator,
            0,
            9,
            10,
        ),
        PublicNodeEvidence::validator(shared_node_address, validator_operator, 0, 9, 10),
    ];
    role_order_bundle.operator_identity_attestation_records = 2;
    role_order_bundle.operator_identity_attestations = vec![
        PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Miner,
            independent_miner_address,
            independent_miner_operator,
            manifest_operator_identity_uri(&independent_miner_operator),
            role_order_bundle.run.run_started_at_unix_seconds,
        ),
        PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Validator,
            shared_node_address,
            validator_operator,
            manifest_operator_identity_uri(&validator_operator),
            role_order_bundle.run.run_started_at_unix_seconds,
        ),
    ];
    role_order_bundle.network_runtime_observations = vec![
        public_network_runtime_observation(
            independent_miner_operator,
            0,
            role_order_bundle.run.run_started_at_unix_seconds,
        ),
        public_network_runtime_observation(
            validator_operator,
            1,
            role_order_bundle.run.run_started_at_unix_seconds,
        ),
    ];
    let role_order_network_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &role_order_bundle
            .network_runtime_observations
            .iter()
            .map(|observation| observation.record_root)
            .collect::<Vec<_>>(),
    )
    .expect("role-order network observation roots should aggregate");
    resign_record_summary_and_artifact(
        &mut role_order_bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        role_order_network_root,
        2,
    );
    let role_order_criteria = PublicTestnetCriteria {
        min_miners: 1,
        min_validators: 1,
        ..criteria.clone()
    };
    let role_order_report = role_order_bundle.evaluate(&role_order_criteria, 6);
    assert_eq!(role_order_report.run_evidence.miner_count, 1);
    assert_eq!(role_order_report.run_evidence.validator_count, 1);
    assert!(role_order_report.run_evidence.public_criterion_met);
    assert!(role_order_report.has_operator_identity_attestations);
    assert!(role_order_report.has_network_runtime_observations);
    assert!(role_order_report.independently_checkable);
    assert!(!role_order_report.full_spec_evidence_met);

    let mut exact_quota_bundle = complete_public_evidence_bundle();
    exact_quota_bundle.run.nodes = vec![
        PublicNodeEvidence::miner([1; 32], [1; 32], 0, 9, 10),
        PublicNodeEvidence::miner([2; 32], [2; 32], 0, 9, 10),
        PublicNodeEvidence::miner([3; 32], [2; 32], 0, 9, 10),
        PublicNodeEvidence::validator([1; 32], [10; 32], 0, 9, 10),
        PublicNodeEvidence::validator([2; 32], [10; 32], 0, 9, 10),
    ];
    exact_quota_bundle.operator_identity_attestation_records = 3;
    exact_quota_bundle.operator_identity_attestations = vec![
        PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Miner,
            [1; 32],
            [1; 32],
            manifest_operator_identity_uri(&[1; 32]),
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
        PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Miner,
            [3; 32],
            [2; 32],
            manifest_operator_identity_uri(&[2; 32]),
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
        PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Validator,
            [2; 32],
            [10; 32],
            manifest_operator_identity_uri(&[10; 32]),
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
    ];
    exact_quota_bundle.network_runtime_observations = vec![
        public_network_runtime_observation(
            [1; 32],
            0,
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
        public_network_runtime_observation(
            [2; 32],
            1,
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
        public_network_runtime_observation(
            [10; 32],
            2,
            exact_quota_bundle.run.run_started_at_unix_seconds,
        ),
    ];
    let exact_quota_network_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &exact_quota_bundle
            .network_runtime_observations
            .iter()
            .map(|observation| observation.record_root)
            .collect::<Vec<_>>(),
    )
    .expect("exact-quota network observation roots should aggregate");
    resign_record_summary_and_artifact(
        &mut exact_quota_bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        exact_quota_network_root,
        3,
    );
    let exact_quota_report = exact_quota_bundle.evaluate(&criteria, 6);
    assert_eq!(exact_quota_report.run_evidence.miner_count, 2);
    assert_eq!(exact_quota_report.run_evidence.validator_count, 1);
    assert!(exact_quota_report.run_evidence.public_criterion_met);
    assert!(exact_quota_report.has_operator_identity_attestations);
    assert!(exact_quota_report.has_network_runtime_observations);
    assert!(exact_quota_report.independently_checkable);

    bundle.publication.manifest_signature = [9; 32];
    let tampered_manifest_signature = bundle.evaluate(&criteria, 6);
    assert!(!tampered_manifest_signature.has_published_evidence_bundle);
    assert!(!tampered_manifest_signature.independently_checkable);
    assert!(!tampered_manifest_signature.full_spec_evidence_met);

    bundle = complete_public_evidence_bundle();
    bundle.publication = PublicEvidencePublication::new(
        bundle.publication.bundle_id,
        bundle.publication.public_uri.clone(),
        bundle.publication.manifest_signer,
        2,
        bundle.publication.independent_auditor_count,
    );
    let overreported_manifest_signature_count = bundle.evaluate(&criteria, 6);
    assert!(!overreported_manifest_signature_count.has_published_evidence_bundle);
    assert!(!overreported_manifest_signature_count.independently_checkable);
    assert!(!overreported_manifest_signature_count.full_spec_evidence_met);

    bundle = complete_public_evidence_bundle();
    bundle.run_window_signature = [7; 32];
    let tampered_run_window = bundle.evaluate(&criteria, 6);
    assert!(!tampered_run_window.has_signed_run_window);
    assert!(!tampered_run_window.independently_checkable);
    assert!(!tampered_run_window.full_spec_evidence_met);

    bundle = complete_public_evidence_bundle();
    bundle.run.run_ended_at_unix_seconds = bundle.run.run_started_at_unix_seconds - 1;
    let invalid_run_window = bundle.evaluate(&criteria, 6);
    assert!(!invalid_run_window.has_signed_run_window);
    assert!(!invalid_run_window.run_evidence.has_required_run_duration);
    assert!(!invalid_run_window.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.manifest_signer = [0; 32];
    let missing_manifest_signer = bundle.evaluate(&criteria, 6);
    assert!(!missing_manifest_signer.has_published_evidence_bundle);
    assert!(!missing_manifest_signer.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("http://localhost:8545/evidence.json");
    let local_uri = bundle.evaluate(&criteria, 6);
    assert!(!local_uri.has_published_evidence_bundle);
    assert!(!local_uri.independently_checkable);
    assert!(!local_uri.full_spec_evidence_met);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("https://localhost/evidence.json");
    let localhost_https_uri = bundle.evaluate(&criteria, 6);
    assert!(!localhost_https_uri.has_published_evidence_bundle);
    assert!(!localhost_https_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("https://192.168.1.2/evidence.json");
    let private_https_uri = bundle.evaluate(&criteria, 6);
    assert!(!private_https_uri.has_published_evidence_bundle);
    assert!(!private_https_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication = PublicEvidencePublication::new(
        bundle.publication.bundle_id,
        " https://evidence.tensorvm.net/public-evidence.json".to_owned(),
        bundle.publication.manifest_signer,
        bundle.publication.manifest_signature_count,
        bundle.publication.independent_auditor_count,
    );
    let leading_space_publication_uri = bundle.evaluate(&criteria, 6);
    assert!(!leading_space_publication_uri.has_published_evidence_bundle);
    assert!(!leading_space_publication_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication = PublicEvidencePublication::new(
        bundle.publication.bundle_id,
        "https://evidence.tensorvm.net/public-evidence.json ".to_owned(),
        bundle.publication.manifest_signer,
        bundle.publication.manifest_signature_count,
        bundle.publication.independent_auditor_count,
    );
    let trailing_space_publication_uri = bundle.evaluate(&criteria, 6);
    assert!(!trailing_space_publication_uri.has_published_evidence_bundle);
    assert!(!trailing_space_publication_uri.independently_checkable);

    assert!(public_evidence_uri_is_external(
        "https://evidence.tensorvm.net:443/public-evidence.json"
    ));
    assert!(public_evidence_uri_is_external(
        "https://[2001:4860:4860::8888]/public-evidence.json"
    ));
    assert!(public_evidence_uri_is_external(
        "https://[2001:4860:4860::8888]:443/public-evidence.json"
    ));
    for uri in [
        "https://evidence.tensorvm.net@localhost/public-evidence.json",
        "https://localhost@evidence.tensorvm.net/public-evidence.json",
        "https://evidence.tensorvm.net /public-evidence.json",
        " https://evidence.tensorvm.net/public-evidence.json",
        "https://evidence.tensorvm.net/public-evidence.json ",
        "https://evidence.tensorvm.net/public evidence.json",
        "https://evidence.tensorvm.net:bad/public-evidence.json",
        "https://evidence.tensorvm.net:0/public-evidence.json",
        "https://evidence.example.test/public-evidence.json",
        "https://evidence.tensorvm.example/public-evidence.json",
        "https://example.com/public-evidence.json",
        "https://sub.example.org/public-evidence.json",
        "https://evidence.invalid/public-evidence.json",
        "https://[2001:db8::1]x/public-evidence.json",
        "https://[2001:4860:4860::8888]:/public-evidence.json",
        "https://evidence.tensorvm.net",
        "https://evidence.tensorvm.net/",
        "https://evidence.tensorvm.net?manifest=1",
        "https://evidence.tensorvm.net#manifest",
        "https://evidence.tensorvm.net/public-evidence.json?download=1",
        "https://evidence.tensorvm.net/public-evidence.json#sha256",
        "https:///public-evidence.json",
    ] {
        assert!(!public_evidence_uri_is_external(uri));
    }

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri =
        String::from("https://evidence.tensorvm.net@localhost/public-evidence.json");
    let userinfo_obfuscated_uri = bundle.evaluate(&criteria, 6);
    assert!(!userinfo_obfuscated_uri.has_published_evidence_bundle);
    assert!(!userinfo_obfuscated_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri =
        String::from("https://evidence.tensorvm.net/public-evidence.json?download=1");
    let query_publication_uri = bundle.evaluate(&criteria, 6);
    assert!(!query_publication_uri.has_published_evidence_bundle);
    assert!(!query_publication_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("https://evidence.tensorvm.net/");
    let root_only_publication_uri = bundle.evaluate(&criteria, 6);
    assert!(!root_only_publication_uri.has_published_evidence_bundle);
    assert!(!root_only_publication_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("ipfs://");
    let empty_ipfs_uri = bundle.evaluate(&criteria, 6);
    assert!(!empty_ipfs_uri.has_published_evidence_bundle);
    assert!(!empty_ipfs_uri.has_independent_auditor_records);
    assert!(!empty_ipfs_uri.independently_checkable);

    assert!(public_evidence_uri_is_external(
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3g3/raw.json"
    ));
    assert!(public_evidence_uri_is_external(
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3g3/raw-records_2026-05.json"
    ));
    assert!(public_evidence_uri_is_external(
        "ar://abc_DEF-123/raw_records.json"
    ));
    assert!(public_evidence_uri_is_external("ar://abc_DEF-123"));
    for uri in [
        "ipfs://?cid",
        "ipfs://#cid",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3?download=1",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3#manifest",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/raw.json?download=1",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/raw.json#manifest",
        "ipfs://../manifest.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/../manifest.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/./manifest.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/",
        "ipfs:///manifest.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3//manifest.json",
        " ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3 ",
        "ipfs://white space",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/bad space.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/bad%20path.json",
        "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3\\raw.json",
        "ar://abc_DEF-123/",
        "ar:///",
    ] {
        assert!(!public_evidence_uri_is_external(uri));
    }

    bundle = complete_public_evidence_bundle();
    bundle.publication.public_uri = String::from("ipfs://?cid");
    let malformed_content_uri = bundle.evaluate(&criteria, 6);
    assert!(!malformed_content_uri.has_published_evidence_bundle);
    assert!(!malformed_content_uri.has_independent_auditor_records);
    assert!(!malformed_content_uri.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.auditor_records.clear();
    let missing_auditor_records = bundle.evaluate(&criteria, 6);
    assert!(missing_auditor_records.has_published_evidence_bundle);
    assert!(!missing_auditor_records.has_independent_auditor_records);
    assert!(!missing_auditor_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.auditor_records[0].auditor_signature = [2; 32];
    let tampered_auditor_record = bundle.evaluate(&criteria, 6);
    assert!(!tampered_auditor_record.has_independent_auditor_records);
    assert!(!tampered_auditor_record.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.auditor_records[0].audit_uri = String::from("https://localhost/audit.json");
    let local_auditor_record = bundle.evaluate(&criteria, 6);
    assert!(!local_auditor_record.has_independent_auditor_records);
    assert!(!local_auditor_record.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.auditor_records[0] = PublicEvidenceAuditorRecord::new(
        &bundle.publication.bundle_id,
        &bundle.publication.public_uri,
        address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        bundle.run.run_started_at_unix_seconds,
    );
    let pre_run_end_auditor_record = bundle.evaluate(&criteria, 6);
    assert!(!pre_run_end_auditor_record.has_independent_auditor_records);
    assert!(!pre_run_end_auditor_record.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.auditor_records[0] = PublicEvidenceAuditorRecord::new(
        &bundle.publication.bundle_id,
        &bundle.publication.public_uri,
        bundle.publication.manifest_signer,
        "https://auditors.tensorvm.net/signer-audit.json",
        1_700_000_000,
    );
    let signer_as_auditor = bundle.evaluate(&criteria, 6);
    assert!(!signer_as_auditor.has_independent_auditor_records);
    assert!(!signer_as_auditor.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle
        .auditor_records
        .push(PublicEvidenceAuditorRecord::new(
            &bundle.publication.bundle_id,
            &bundle.publication.public_uri,
            address(b"public-evidence-auditor-extra"),
            "https://auditors.tensorvm.net/extra-audit.json",
            bundle.run.run_ended_at_unix_seconds,
        ));
    let extra_auditor_record = bundle.evaluate(&criteria, 6);
    assert!(!extra_auditor_record.has_independent_auditor_records);
    assert!(!extra_auditor_record.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.block_history_records = 9;
    let missing_block_history = bundle.evaluate(&criteria, 6);
    assert!(!missing_block_history.has_block_history);
    assert!(!missing_block_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let block_history_root = bundle.block_history_root;
    let overreported_block_history_count = bundle.run.observed_blocks + 1;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::BlockHistory,
        block_history_root,
        overreported_block_history_count,
    );
    let overreported_block_history = bundle.evaluate(&criteria, 6);
    assert!(!overreported_block_history.has_block_history);
    assert!(overreported_block_history.has_public_supporting_record_artifacts);
    assert!(!overreported_block_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.block_history_signature = [6; 32];
    let tampered_block_history = bundle.evaluate(&criteria, 6);
    assert!(!tampered_block_history.has_block_history);
    assert!(!tampered_block_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.block_history_root = [0; 32];
    let missing_block_history_root = bundle.evaluate(&criteria, 6);
    assert!(!missing_block_history_root.has_block_history);
    assert!(!missing_block_history_root.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.finality_history_records = 9;
    let missing_finality_history = bundle.evaluate(&criteria, 6);
    assert!(!missing_finality_history.has_finality_history);
    assert!(!missing_finality_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let finality_history_root = bundle.finality_history_root;
    let overreported_finality_history_count = bundle.run.observed_blocks + 1;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::FinalityHistory,
        finality_history_root,
        overreported_finality_history_count,
    );
    let overreported_finality_history = bundle.evaluate(&criteria, 6);
    assert!(!overreported_finality_history.has_finality_history);
    assert!(overreported_finality_history.has_public_supporting_record_artifacts);
    assert!(!overreported_finality_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.finality_history_signature = [5; 32];
    let tampered_finality_history = bundle.evaluate(&criteria, 6);
    assert!(!tampered_finality_history.has_finality_history);
    assert!(!tampered_finality_history.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.operator_identity_attestation_records = 2;
    let missing_operator_attestations = bundle.evaluate(&criteria, 6);
    assert!(!missing_operator_attestations.has_operator_identity_attestations);
    assert!(
        !missing_operator_attestations
            .run_evidence
            .external_operator_evidence
    );
    assert!(
        !missing_operator_attestations
            .run_evidence
            .public_criterion_met
    );
    assert!(!missing_operator_attestations.independently_checkable);
    bundle.operator_identity_attestations.truncate(2);
    let (miner_operators, validator_operators) = bundle
        .run
        .matched_independent_public_operators_for_criteria(&criteria);
    assert!(
        !bundle.has_operator_identity_attestation_records_for_public_operators(
            2,
            &miner_operators,
            &validator_operators
        )
    );

    bundle = complete_public_evidence_bundle();
    bundle.operator_identity_attestation_records = 4;
    let overreported_operator_attestations = bundle.evaluate(&criteria, 6);
    assert!(!overreported_operator_attestations.has_operator_identity_attestations);
    assert!(
        !overreported_operator_attestations
            .run_evidence
            .external_operator_evidence
    );
    assert!(!overreported_operator_attestations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.operator_identity_attestations[0].operator_signature = [8; 32];
    let tampered_operator_attestation = bundle.evaluate(&criteria, 6);
    assert!(!tampered_operator_attestation.has_operator_identity_attestations);
    assert!(
        !tampered_operator_attestation
            .run_evidence
            .external_operator_evidence
    );
    assert!(!tampered_operator_attestation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.operator_identity_attestations[0].identity_uri =
        String::from("https://localhost/operator.json");
    let local_operator_attestation = bundle.evaluate(&criteria, 6);
    assert!(!local_operator_attestation.has_operator_identity_attestations);
    assert!(!local_operator_attestation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let stale_operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
    bundle.operator_identity_attestations[0] = PublicOperatorIdentityAttestation::new(
        PublicNodeRole::Miner,
        address(b"miner-a"),
        stale_operator_id,
        manifest_operator_identity_uri(&stale_operator_id),
        bundle.run.run_started_at_unix_seconds - 1,
    );
    let stale_operator_attestation = bundle.evaluate(&criteria, 6);
    assert!(!stale_operator_attestation.has_operator_identity_attestations);
    assert!(
        !stale_operator_attestation
            .run_evidence
            .external_operator_evidence
    );
    assert!(!stale_operator_attestation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let uncounted_validator_operator_id = hash_bytes(b"test", &[b"uncounted-validator-operator"]);
    let uncounted_validator_address = bundle.run.nodes[0].address;
    bundle.run.nodes.push(PublicNodeEvidence::validator(
        uncounted_validator_address,
        uncounted_validator_operator_id,
        0,
        9,
        10,
    ));
    bundle.operator_identity_attestations[2] = PublicOperatorIdentityAttestation::new(
        PublicNodeRole::Validator,
        uncounted_validator_address,
        uncounted_validator_operator_id,
        manifest_operator_identity_uri(&uncounted_validator_operator_id),
        bundle.run.run_started_at_unix_seconds,
    );
    let uncounted_operator_attestation = bundle.evaluate(&criteria, 6);
    assert_eq!(uncounted_operator_attestation.run_evidence.miner_count, 2);
    assert_eq!(
        uncounted_operator_attestation.run_evidence.validator_count,
        1
    );
    assert!(!uncounted_operator_attestation.has_operator_identity_attestations);
    assert!(
        !uncounted_operator_attestation
            .run_evidence
            .external_operator_evidence
    );
    assert!(!uncounted_operator_attestation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.operator_identity_attestations.clear();
    let missing_signed_operator_records = bundle.evaluate(&criteria, 6);
    assert!(!missing_signed_operator_records.has_operator_identity_attestations);
    assert!(!missing_signed_operator_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let duplicate_operator_node = bundle.run.nodes[0].clone();
    bundle
        .operator_identity_attestations
        .push(PublicOperatorIdentityAttestation::new(
            duplicate_operator_node.role,
            duplicate_operator_node.address,
            duplicate_operator_node.operator_id,
            format!(
                "https://operators.tensorvm.net/{}/duplicate",
                hex(&duplicate_operator_node.operator_id)
            ),
            bundle.run.run_started_at_unix_seconds,
        ));
    let extra_operator_record = bundle.evaluate(&criteria, 6);
    assert!(!extra_operator_record.has_operator_identity_attestations);
    assert!(
        !extra_operator_record
            .run_evidence
            .external_operator_evidence
    );
    assert!(!extra_operator_record.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observation_records = 2;
    let missing_network_runtime_observations = bundle.evaluate(&criteria, 6);
    assert!(!missing_network_runtime_observations.has_network_runtime_observations);
    assert!(!missing_network_runtime_observations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observations.pop();
    let missing_signed_network_runtime_observation = bundle.evaluate(&criteria, 6);
    assert!(!missing_signed_network_runtime_observation.has_network_runtime_observations);
    assert!(!missing_signed_network_runtime_observation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observations[0].operator_id =
        hash_bytes(b"test", &[b"unmatched-network-operator"]);
    let unmatched_network_operator = bundle.evaluate(&criteria, 6);
    assert!(!unmatched_network_operator.has_network_runtime_observations);
    assert!(!unmatched_network_operator.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observations[0].listen_address = String::from("/ip4/127.0.0.1/tcp/4001");
    let local_network_observation = bundle.evaluate(&criteria, 6);
    assert!(!local_network_observation.has_network_runtime_observations);
    assert!(!local_network_observation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observations[0].observed_at_unix_seconds =
        bundle.run.run_started_at_unix_seconds - 1;
    let stale_network_observation = bundle.evaluate(&criteria, 6);
    assert!(!stale_network_observation.has_network_runtime_observations);
    assert!(!stale_network_observation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let duplicate_listen_address = bundle.network_runtime_observations[0]
        .listen_address
        .clone();
    bundle.network_runtime_observations[1] = bundle.network_runtime_observations[1]
        .with_listen_address_for_testing(duplicate_listen_address);
    let duplicate_listen_roots = bundle
        .network_runtime_observations
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    let duplicate_listen_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &duplicate_listen_roots,
    )
    .expect("duplicate-listen network observation roots should aggregate");
    let duplicate_listen_record_count = bundle.network_runtime_observation_records;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        duplicate_listen_root,
        duplicate_listen_record_count,
    );
    let duplicate_listen_observation = bundle.evaluate(&criteria, 6);
    assert!(!duplicate_listen_observation.has_network_runtime_observations);
    assert!(duplicate_listen_observation.has_public_supporting_record_artifacts);
    assert!(!duplicate_listen_observation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let duplicate_peer_id = bundle.network_runtime_observations[0].peer_id.clone();
    bundle.network_runtime_observations[1] =
        bundle.network_runtime_observations[1].with_peer_id_for_testing(duplicate_peer_id);
    let duplicate_peer_roots = bundle
        .network_runtime_observations
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    let duplicate_peer_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &duplicate_peer_roots,
    )
    .expect("duplicate-peer network observation roots should aggregate");
    let duplicate_peer_record_count = bundle.network_runtime_observation_records;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        duplicate_peer_root,
        duplicate_peer_record_count,
    );
    let duplicate_peer_observation = bundle.evaluate(&criteria, 6);
    assert!(!duplicate_peer_observation.has_network_runtime_observations);
    assert!(duplicate_peer_observation.has_public_supporting_record_artifacts);
    assert!(!duplicate_peer_observation.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let network_runtime_root = bundle.network_runtime_observation_root;
    let underreported_network_runtime_count = bundle
        .operator_identity_attestation_records
        .saturating_sub(1);
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        network_runtime_root,
        underreported_network_runtime_count,
    );
    let underreported_network_runtime_observations = bundle.evaluate(&criteria, 6);
    assert!(!underreported_network_runtime_observations.has_network_runtime_observations);
    assert!(underreported_network_runtime_observations.has_operator_identity_attestations);
    assert!(underreported_network_runtime_observations.has_public_supporting_record_artifacts);
    assert!(!underreported_network_runtime_observations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let network_runtime_root = bundle.network_runtime_observation_root;
    let overreported_network_runtime_count = bundle
        .operator_identity_attestation_records
        .saturating_add(1);
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        network_runtime_root,
        overreported_network_runtime_count,
    );
    let overreported_network_runtime_observations = bundle.evaluate(&criteria, 6);
    assert!(!overreported_network_runtime_observations.has_network_runtime_observations);
    assert!(overreported_network_runtime_observations.has_operator_identity_attestations);
    assert!(overreported_network_runtime_observations.has_public_supporting_record_artifacts);
    assert!(!overreported_network_runtime_observations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.network_runtime_observation_signature = [3; 32];
    let tampered_network_runtime_observations = bundle.evaluate(&criteria, 6);
    assert!(!tampered_network_runtime_observations.has_network_runtime_observations);
    assert!(!tampered_network_runtime_observations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.run.network_runtime.gossip_propagation_observed = false;
    let no_network_runtime_observations = bundle.evaluate(&criteria, 6);
    assert!(!no_network_runtime_observations.has_network_runtime_observations);
    assert!(!no_network_runtime_observations.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.randomness_beacon_signature = [9; 32];
    let tampered_randomness_beacon = bundle.evaluate(&criteria, 6);
    assert!(!tampered_randomness_beacon.has_randomness_beacon_evidence);
    assert!(!tampered_randomness_beacon.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.randomness_beacon_records = 0;
    let missing_randomness_beacon = bundle.evaluate(&criteria, 6);
    assert!(!missing_randomness_beacon.has_randomness_beacon_evidence);
    assert!(!missing_randomness_beacon.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.data_availability_measurement_records = 19;
    let missing_data_availability_measurements = bundle.evaluate(&criteria, 6);
    assert!(!missing_data_availability_measurements.has_data_availability_measurements);
    assert!(!missing_data_availability_measurements.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let data_availability_root = bundle.data_availability_measurement_root;
    let overreported_data_availability_count = bundle.run.checked_receipts + 1;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::DataAvailabilityMeasurements,
        data_availability_root,
        overreported_data_availability_count,
    );
    let overreported_data_availability_measurements = bundle.evaluate(&criteria, 6);
    assert!(!overreported_data_availability_measurements.has_data_availability_measurements);
    assert!(overreported_data_availability_measurements.has_public_supporting_record_artifacts);
    assert!(!overreported_data_availability_measurements.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.data_availability_measurement_signature = [4; 32];
    let tampered_data_availability_measurements = bundle.evaluate(&criteria, 6);
    assert!(!tampered_data_availability_measurements.has_data_availability_measurements);
    assert!(!tampered_data_availability_measurements.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.invalid_work_rejection_signature = [2; 32];
    let tampered_invalid_work_records = bundle.evaluate(&criteria, 6);
    assert!(!tampered_invalid_work_records.has_invalid_work_rejection_records);
    assert!(!tampered_invalid_work_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.invalid_work_rejection_records = 0;
    let missing_invalid_work_records = bundle.evaluate(&criteria, 6);
    assert!(!missing_invalid_work_records.has_invalid_work_rejection_records);
    assert!(!missing_invalid_work_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    let invalid_work_root = bundle.invalid_work_rejection_root;
    let overreported_invalid_work_count = bundle.run.invalid_receipts_submitted + 1;
    resign_record_summary_and_artifact(
        &mut bundle,
        PublicEvidenceRecordKind::InvalidWorkRejections,
        invalid_work_root,
        overreported_invalid_work_count,
    );
    let overreported_invalid_work_records = bundle.evaluate(&criteria, 6);
    assert!(!overreported_invalid_work_records.has_invalid_work_rejection_records);
    assert!(overreported_invalid_work_records.has_public_supporting_record_artifacts);
    assert!(!overreported_invalid_work_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.reward_settlement_signature = [1; 32];
    let tampered_reward_records = bundle.evaluate(&criteria, 6);
    assert!(!tampered_reward_records.has_reward_settlement_record_summary);
    assert!(!tampered_reward_records.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.supporting_artifacts.clear();
    let missing_supporting_artifacts = bundle.evaluate(&criteria, 6);
    assert!(!missing_supporting_artifacts.has_public_supporting_record_artifacts);
    assert!(!missing_supporting_artifacts.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.supporting_artifacts[0].artifact_signature = [1; 32];
    let tampered_supporting_artifact = bundle.evaluate(&criteria, 6);
    assert!(!tampered_supporting_artifact.has_public_supporting_record_artifacts);
    assert!(!tampered_supporting_artifact.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.supporting_artifacts[0].artifact_uri = String::from("https://localhost/raw.json");
    let local_supporting_artifact = bundle.evaluate(&criteria, 6);
    assert!(!local_supporting_artifact.has_public_supporting_record_artifacts);
    assert!(!local_supporting_artifact.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.supporting_artifacts[0].artifact_uri = String::from("https://evidence.tensorvm.net/");
    let root_only_supporting_artifact = bundle.evaluate(&criteria, 6);
    assert!(!root_only_supporting_artifact.has_public_supporting_record_artifacts);
    assert!(!root_only_supporting_artifact.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle
        .supporting_artifacts
        .push(bundle.supporting_artifacts[0].clone());
    let duplicate_supporting_artifact = bundle.evaluate(&criteria, 6);
    assert!(!duplicate_supporting_artifact.has_public_supporting_record_artifacts);
    assert!(!duplicate_supporting_artifact.independently_checkable);

    bundle = complete_public_evidence_bundle();
    bundle.run.services.clear();
    let missing_services = bundle.evaluate(&criteria, 6);
    assert!(missing_services.independently_checkable);
    assert!(!missing_services.run_evidence.public_criterion_met);
    assert!(!missing_services.full_spec_evidence_met);

    bundle = complete_public_evidence_bundle();
    bundle.run.service_content.clear();
    let missing_service_content = bundle.evaluate(&criteria, 6);
    assert!(missing_service_content.independently_checkable);
    assert!(
        !missing_service_content
            .run_evidence
            .has_deployed_public_service_content
    );
    assert!(!missing_service_content.run_evidence.public_criterion_met);
    assert!(!missing_service_content.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_deployed_detection_measurements_for_full_spec() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(full_spec_report.has_deployed_detection_measurement_records);
    assert!(full_spec_report.independently_checkable);
    assert!(full_spec_report.full_spec_evidence_met);

    let mut no_run_measurements = full_spec_bundle.clone();
    no_run_measurements.run.detection_measurement_records = 0;
    let report = no_run_measurements.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(!report.has_deployed_detection_measurement_records);
    assert!(!report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut missing_summary = full_spec_bundle.clone();
    let detection_root = missing_summary.detection_measurement_root;
    resign_record_summary_and_artifact(
        &mut missing_summary,
        PublicEvidenceRecordKind::DetectionMeasurements,
        detection_root,
        0,
    );
    let report = missing_summary.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(!report.has_deployed_detection_measurement_records);
    assert!(!report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut missing_raw_records = full_spec_bundle.clone();
    missing_raw_records
        .detection_measurement_raw_records
        .clear();
    let report = missing_raw_records.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let resign_detection_records = |bundle: &mut PublicTestnetEvidenceBundle| {
        let detection_roots = bundle
            .detection_measurement_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>();
        let detection_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::DetectionMeasurements,
            &detection_roots,
        )
        .unwrap();
        let record_count = bundle.detection_measurement_records;
        resign_record_summary_and_artifact(
            bundle,
            PublicEvidenceRecordKind::DetectionMeasurements,
            detection_root,
            record_count,
        );
    };

    let mut malformed_mechanism = full_spec_bundle.clone();
    malformed_mechanism.detection_measurement_raw_records[0].mechanism =
        String::from("bad_mechanism");
    resign_detection_records(&mut malformed_mechanism);
    let report = malformed_mechanism.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut zero_subject = full_spec_bundle.clone();
    zero_subject.detection_measurement_raw_records[0].subject_root = [0; 32];
    resign_detection_records(&mut zero_subject);
    let report = zero_subject.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut zero_sample = full_spec_bundle.clone();
    zero_sample.detection_measurement_raw_records[0].sample_count = 0;
    zero_sample.detection_measurement_raw_records[0].detected_count = 0;
    resign_detection_records(&mut zero_sample);
    let report = zero_sample.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut overdetected = full_spec_bundle.clone();
    overdetected.detection_measurement_raw_records[0].detected_count = overdetected
        .detection_measurement_raw_records[0]
        .sample_count
        .saturating_add(1);
    resign_detection_records(&mut overdetected);
    let report = overdetected.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_raw_root = full_spec_bundle;
    let record_count = mismatched_raw_root.detection_measurement_records;
    resign_record_summary_and_artifact(
        &mut mismatched_raw_root,
        PublicEvidenceRecordKind::DetectionMeasurements,
        hash_bytes(
            b"test",
            &[b"summary-root-not-derived-from-raw-detection-measurements"],
        ),
        record_count,
    );
    let report = mismatched_raw_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.has_deployed_detection_measurement_records);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_cuda_verified_miners_for_full_spec() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(full_spec_report.run_evidence.public_criterion_met);
    assert!(full_spec_report.independently_checkable);
    assert!(full_spec_report.has_cuda_verified_miners);
    assert!(full_spec_report.full_spec_evidence_met);

    let mut missing_cuda = full_spec_bundle.clone();
    missing_cuda.run.cuda_verified_miner_count = 0;
    let missing_cuda_report = missing_cuda.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(missing_cuda_report.run_evidence.public_criterion_met);
    assert!(missing_cuda_report.independently_checkable);
    assert!(!missing_cuda_report.has_cuda_verified_miners);
    assert!(!missing_cuda_report.has_cuda_graph_execution_evidence);
    assert!(!missing_cuda_report.full_spec_evidence_met);

    let mut undercounted_cuda = full_spec_bundle;
    undercounted_cuda.run.cuda_verified_miner_count =
        (full_spec_criteria.min_miners.saturating_sub(1)) as u64;
    let undercounted_cuda_report =
        undercounted_cuda.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(undercounted_cuda_report.run_evidence.public_criterion_met);
    assert!(undercounted_cuda_report.independently_checkable);
    assert_eq!(
        undercounted_cuda_report
            .run_evidence
            .cuda_verified_miner_count,
        (full_spec_criteria.min_miners - 1) as u64
    );
    assert!(!undercounted_cuda_report.has_cuda_verified_miners);
    assert!(!undercounted_cuda_report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_cuda_graph_execution_for_full_spec() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(full_spec_report.run_evidence.public_criterion_met);
    assert!(full_spec_report.independently_checkable);
    assert!(full_spec_report.has_cuda_graph_execution_evidence);
    assert!(full_spec_report.full_spec_evidence_met);

    let mut missing_graph_execution = full_spec_bundle.clone();
    missing_graph_execution.run.cuda_graph_execution_receipts = 0;
    let missing_graph_report =
        missing_graph_execution.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(missing_graph_report.run_evidence.public_criterion_met);
    assert!(missing_graph_report.independently_checkable);
    assert!(!missing_graph_report.has_cuda_graph_execution_evidence);
    assert!(!missing_graph_report.full_spec_evidence_met);

    let mut overcounted_graph_execution = full_spec_bundle.clone();
    overcounted_graph_execution
        .run
        .cuda_graph_execution_receipts = overcounted_graph_execution
        .run
        .checked_receipts
        .saturating_add(1);
    let overcounted_checked_report =
        overcounted_graph_execution.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(overcounted_checked_report.run_evidence.public_criterion_met);
    assert!(overcounted_checked_report.independently_checkable);
    assert!(!overcounted_checked_report.has_cuda_graph_execution_evidence);
    assert!(!overcounted_checked_report.full_spec_evidence_met);

    let mut unavailable_graph_execution = full_spec_bundle;
    unavailable_graph_execution.run.available_receipts = 0;
    let unavailable_graph_report =
        unavailable_graph_execution.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(!unavailable_graph_report.run_evidence.public_criterion_met);
    assert!(unavailable_graph_report.independently_checkable);
    assert!(!unavailable_graph_report.has_cuda_graph_execution_evidence);
    assert!(!unavailable_graph_report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_validator_vrf_lifecycle_for_full_spec() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(full_spec_report.run_evidence.public_criterion_met);
    assert!(full_spec_report.independently_checkable);
    assert!(
        full_spec_report
            .run_evidence
            .has_validator_vrf_lifecycle_evidence
    );
    assert!(full_spec_report.full_spec_evidence_met);

    let mut missing_lifecycle = full_spec_bundle.clone();
    missing_lifecycle.run.validator_vrf_lifecycle_records = 0;
    let missing_lifecycle_report =
        missing_lifecycle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(missing_lifecycle_report.run_evidence.public_criterion_met);
    assert!(!missing_lifecycle_report.independently_checkable);
    assert!(
        !missing_lifecycle_report
            .run_evidence
            .has_validator_vrf_lifecycle_evidence
    );
    assert!(!missing_lifecycle_report.has_validator_vrf_lifecycle_record_summary);
    assert!(!missing_lifecycle_report.full_spec_evidence_met);

    let mut undercounted_lifecycle = full_spec_bundle.clone();
    undercounted_lifecycle.run.validator_vrf_lifecycle_records = undercounted_lifecycle
        .run
        .checked_receipts
        .saturating_sub(1);
    let undercounted_lifecycle_report =
        undercounted_lifecycle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(
        undercounted_lifecycle_report
            .run_evidence
            .public_criterion_met
    );
    assert!(!undercounted_lifecycle_report.independently_checkable);
    assert!(
        !undercounted_lifecycle_report
            .run_evidence
            .has_validator_vrf_lifecycle_evidence
    );
    assert!(!undercounted_lifecycle_report.has_validator_vrf_lifecycle_record_summary);
    assert!(!undercounted_lifecycle_report.full_spec_evidence_met);

    let mut overcounted_lifecycle = full_spec_bundle;
    overcounted_lifecycle.run.validator_vrf_lifecycle_records =
        overcounted_lifecycle.run.checked_receipts.saturating_add(1);
    let overcounted_lifecycle_report =
        overcounted_lifecycle.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(
        overcounted_lifecycle_report
            .run_evidence
            .public_criterion_met
    );
    assert!(!overcounted_lifecycle_report.independently_checkable);
    assert!(
        !overcounted_lifecycle_report
            .run_evidence
            .has_validator_vrf_lifecycle_evidence
    );
    assert!(!overcounted_lifecycle_report.has_validator_vrf_lifecycle_record_summary);
    assert!(!overcounted_lifecycle_report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_raw_validator_vrf_lifecycle_records_for_full_spec() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    assert!(
        full_spec_bundle
            .evaluate(&full_spec_criteria, full_spec_block_time)
            .full_spec_evidence_met
    );

    let mut missing_raw_records = full_spec_bundle.clone();
    missing_raw_records
        .validator_vrf_lifecycle_raw_records
        .clear();
    let report = missing_raw_records.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(report.has_validator_vrf_lifecycle_record_summary);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_raw_records = full_spec_bundle.clone();
    mismatched_raw_records.validator_vrf_lifecycle_raw_records[0].receipt_root =
        hash_bytes(b"test", &[b"mismatched-vrf-lifecycle-receipt"]);
    let report = mismatched_raw_records.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(report.has_validator_vrf_lifecycle_record_summary);
    assert!(!report.full_spec_evidence_met);

    let mut duplicate_receipt_records = full_spec_bundle.clone();
    duplicate_receipt_records.validator_vrf_lifecycle_raw_records[1].receipt_root =
        duplicate_receipt_records.validator_vrf_lifecycle_raw_records[0].receipt_root;
    let duplicate_lifecycle_root = aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::ValidatorVrfLifecycle,
        &duplicate_receipt_records
            .validator_vrf_lifecycle_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>(),
    )
    .expect("duplicate receipt roots still produce distinct record roots");
    let duplicate_record_count = duplicate_receipt_records.validator_vrf_lifecycle_records;
    resign_record_summary_and_artifact(
        &mut duplicate_receipt_records,
        PublicEvidenceRecordKind::ValidatorVrfLifecycle,
        duplicate_lifecycle_root,
        duplicate_record_count,
    );
    let report = duplicate_receipt_records.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(report.has_validator_vrf_lifecycle_record_summary);
    assert!(!report.full_spec_evidence_met);

    let mut incomplete_lifecycle_records = full_spec_bundle;
    incomplete_lifecycle_records.validator_vrf_lifecycle_raw_records[0].phase =
        PublicValidatorVrfLifecyclePhase::Committed;
    let report = incomplete_lifecycle_records.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(report.has_validator_vrf_lifecycle_record_summary);
    assert!(!report.full_spec_evidence_met);
}

#[test]
fn public_testnet_evidence_bundle_requires_raw_chain_history_records() {
    let full_spec_criteria = PublicTestnetCriteria::default();
    let full_spec_block_time = ChainParams::default().block_time_seconds;
    let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
    assert!(
        full_spec_bundle
            .evaluate(&full_spec_criteria, full_spec_block_time)
            .full_spec_evidence_met
    );

    let mut missing_block_history = full_spec_bundle.clone();
    missing_block_history.block_history_raw_records.clear();
    let report = missing_block_history.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut missing_finality_history = full_spec_bundle.clone();
    missing_finality_history
        .finality_history_raw_records
        .clear();
    let report = missing_finality_history.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.run_evidence.public_criterion_met);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let resign_block_history_records = |bundle: &mut PublicTestnetEvidenceBundle| {
        let block_roots = bundle
            .block_history_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>();
        let block_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::BlockHistory,
            &block_roots,
        )
        .unwrap();
        let record_count = bundle.block_history_records;
        resign_record_summary_and_artifact(
            bundle,
            PublicEvidenceRecordKind::BlockHistory,
            block_root,
            record_count,
        );
    };
    let resign_finality_history_records = |bundle: &mut PublicTestnetEvidenceBundle| {
        let finality_roots = bundle
            .finality_history_raw_records
            .iter()
            .map(|record| record.record_root())
            .collect::<Vec<_>>();
        let finality_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::FinalityHistory,
            &finality_roots,
        )
        .unwrap();
        let record_count = bundle.finality_history_records;
        resign_record_summary_and_artifact(
            bundle,
            PublicEvidenceRecordKind::FinalityHistory,
            finality_root,
            record_count,
        );
    };

    let mut duplicate_block_number = full_spec_bundle.clone();
    duplicate_block_number.block_history_raw_records[1].block =
        duplicate_block_number.block_history_raw_records[0].block;
    resign_block_history_records(&mut duplicate_block_number);
    let report = duplicate_block_number.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut zero_block_root = full_spec_bundle.clone();
    zero_block_root.block_history_raw_records[0].block_root = [0; 32];
    zero_block_root.finality_history_raw_records[0].block_root = [0; 32];
    resign_block_history_records(&mut zero_block_root);
    resign_finality_history_records(&mut zero_block_root);
    let report = zero_block_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_finality_block_root = full_spec_bundle.clone();
    mismatched_finality_block_root.finality_history_raw_records[0].block_root =
        hash_bytes(b"test", &[b"public-finality-root-not-in-block-history"]);
    resign_finality_history_records(&mut mismatched_finality_block_root);
    let report = mismatched_finality_block_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_finalized_count = full_spec_bundle.clone();
    mismatched_finalized_count.finality_history_raw_records[0].status =
        PublicFinalityHistoryStatus::Unfinalized;
    resign_finality_history_records(&mut mismatched_finalized_count);
    let report = mismatched_finalized_count.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_block_root = full_spec_bundle.clone();
    let record_count = mismatched_block_root.block_history_records;
    resign_record_summary_and_artifact(
        &mut mismatched_block_root,
        PublicEvidenceRecordKind::BlockHistory,
        hash_bytes(
            b"test",
            &[b"summary-root-not-derived-from-raw-block-history"],
        ),
        record_count,
    );
    let report = mismatched_block_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);

    let mut mismatched_finality_root = full_spec_bundle;
    let record_count = mismatched_finality_root.finality_history_records;
    resign_record_summary_and_artifact(
        &mut mismatched_finality_root,
        PublicEvidenceRecordKind::FinalityHistory,
        hash_bytes(
            b"test",
            &[b"summary-root-not-derived-from-raw-finality-history"],
        ),
        record_count,
    );
    let report = mismatched_finality_root.evaluate(&full_spec_criteria, full_spec_block_time);
    assert!(report.independently_checkable);
    assert!(!report.full_spec_evidence_met);
}
