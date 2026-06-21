use super::{NetworkBlockPayloadApply, NetworkPayloadApply};
use crate::{
    chain::{BlockAdmission, Chain, ChainCommand, ChainEngine, JobState},
    challenge::block_check_challenge_id,
    p2p::{
        decode_attestation_payload, decode_block_check_challenge_payload, decode_block_payload,
        decode_block_vote_payload, decode_job_payload, decode_receipt_payload,
        decode_validator_audit_report_payload,
    },
    types::{Hash, hash_bytes},
    verify::ValidatorAttestation,
};

pub fn apply_network_job_payload(
    chain: &mut Chain,
    job_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if job_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(job) = decode_job_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if job.job_id() != job_id {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().jobs().get(&job_id) {
        if existing == &job {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if let JobState::GraphExecution(graph_job) = &job
        && chain.state().program_body(&graph_job.graph_id).is_none()
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitJob(job))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_block_payload(
    chain: &mut Chain,
    height: u64,
    block_hash: Hash,
    payload: &[u8],
) -> NetworkBlockPayloadApply {
    if height == 0 || block_hash == [0; 32] {
        return NetworkBlockPayloadApply::Invalid;
    }
    let Ok(block) = decode_block_payload(payload) else {
        return NetworkBlockPayloadApply::Invalid;
    };
    if block.height != height || block.hash() != block_hash {
        return NetworkBlockPayloadApply::Invalid;
    }
    if chain
        .blocks
        .iter()
        .any(|existing| existing.hash() == block_hash)
    {
        return NetworkBlockPayloadApply::Applied { appended: 0 };
    }
    let parent_known = block.parent_hash == [0; 32]
        || chain
            .blocks()
            .iter()
            .any(|existing| existing.hash() == block.parent_hash)
        || chain.side_branch_blocks().contains_key(&block.parent_hash);
    if height > chain.state().height() && !parent_known {
        return NetworkBlockPayloadApply::Pending;
    }
    let current_head_competitor = height.saturating_add(1) == chain.state().height();
    if height < chain.state().height() && !current_head_competitor && !parent_known {
        return NetworkBlockPayloadApply::Invalid;
    }
    let expected_parent = chain
        .blocks
        .last()
        .map(crate::chain::TensorBlock::hash)
        .unwrap_or([0; 32]);
    if !current_head_competitor && block.parent_hash != expected_parent && !parent_known {
        return NetworkBlockPayloadApply::Pending;
    }

    let mut candidate = chain.clone();
    if !current_head_competitor
        && block.parent_hash == expected_parent
        && candidate.prepare_block_parent_state().is_err()
    {
        return NetworkBlockPayloadApply::Invalid;
    }
    match candidate.admit_block(block) {
        Ok(BlockAdmission::Applied { .. })
        | Ok(BlockAdmission::Replaced { .. })
        | Ok(BlockAdmission::Reorganized { .. })
        | Ok(BlockAdmission::SideBranchStored { .. }) => {
            *chain = candidate;
            NetworkBlockPayloadApply::Applied { appended: 1 }
        }
        Ok(BlockAdmission::Duplicate { .. }) => NetworkBlockPayloadApply::Applied { appended: 0 },
        Ok(BlockAdmission::PendingParent { .. }) => NetworkBlockPayloadApply::Pending,
        Ok(BlockAdmission::Invalid { .. }) | Err(_) => NetworkBlockPayloadApply::Invalid,
    }
}

pub fn apply_network_block_vote_payload(
    chain: &mut Chain,
    block_hash: Hash,
    validator: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if block_hash == [0; 32] || validator == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(vote) = decode_block_vote_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if vote.block_hash != block_hash || vote.validator != validator {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain
        .state()
        .block_votes()
        .get(&block_hash)
        .and_then(|votes| {
            votes
                .iter()
                .find(|existing| existing.validator == validator)
        })
    {
        return if existing == &vote {
            NetworkPayloadApply::Applied
        } else {
            NetworkPayloadApply::Invalid
        };
    }
    if !chain
        .blocks
        .iter()
        .any(|block| block.height == vote.block_height && block.hash() == block_hash)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_receipt_payload(
    chain: &mut Chain,
    receipt_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if receipt_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(receipt) = decode_receipt_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if receipt.receipt_id() != receipt_id {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().receipts().get(&receipt_id) {
        if existing == &receipt {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state().jobs().contains_key(&receipt.job_id())
        || !chain.state().miners().contains_key(&receipt.miner())
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_attestation_payload(
    chain: &mut Chain,
    attestation_id: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if attestation_id == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(attestation) = decode_attestation_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if attestation_announcement_hash(&attestation) != attestation_id {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain
        .state()
        .attestations()
        .get(&attestation.receipt_id)
        .and_then(|items| {
            items
                .iter()
                .find(|existing| existing.validator == attestation.validator)
        })
    {
        if existing == &attestation {
            return NetworkPayloadApply::Applied;
        }
        return NetworkPayloadApply::Invalid;
    }
    if !chain
        .state()
        .validators()
        .contains_key(&attestation.validator)
        || !chain
            .state()
            .receipts()
            .contains_key(&attestation.receipt_id)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_validator_audit_report_payload(
    chain: &mut Chain,
    audit_id: Hash,
    auditor: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if audit_id == [0; 32] || auditor == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(report) = decode_validator_audit_report_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if report.audit_id != audit_id || report.auditor != auditor {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().validator_audit_results().get(&audit_id) {
        return if existing.auditor == report.auditor
            && existing.canonical_result == report.canonical_result
            && existing.canonical_data_availability_passed
                == report.canonical_data_availability_passed
            && existing.checks_root == report.checks_root
            && existing.signature == report.signature
        {
            NetworkPayloadApply::Applied
        } else {
            NetworkPayloadApply::Invalid
        };
    }
    let Some(assignment) = chain.state().validator_audit_assignments().get(&audit_id) else {
        return NetworkPayloadApply::Pending;
    };
    if assignment.auditor != auditor {
        return NetworkPayloadApply::Invalid;
    }
    if !chain.state().validators().contains_key(&auditor)
        || !chain
            .state()
            .attestations()
            .get(&assignment.receipt_id)
            .is_some_and(|items| {
                items
                    .iter()
                    .any(|attestation| attestation.validator == assignment.validator)
            })
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitValidatorAuditReport(report))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_block_check_challenge_payload(
    chain: &mut Chain,
    challenge_id: Hash,
    block_hash: Hash,
    challenger: Hash,
    payload: &[u8],
) -> NetworkPayloadApply {
    if challenge_id == [0; 32] || block_hash == [0; 32] || challenger == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(challenge) = decode_block_check_challenge_payload(payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if challenge.receipt_id == [0; 32]
        || block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id) != challenge_id
        || challenge.block_hash != block_hash
        || challenge.challenger != challenger
    {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().block_check_challenges().get(&challenge_id) {
        return if existing.block_hash == challenge.block_hash
            && existing.receipt_id == challenge.receipt_id
            && existing.challenger == challenge.challenger
            && existing.expected_check_leaf == challenge.expected_check_leaf
            && existing.observed_check_leaf == challenge.observed_check_leaf
        {
            NetworkPayloadApply::Applied
        } else {
            NetworkPayloadApply::Invalid
        };
    }
    if !chain
        .blocks
        .iter()
        .any(|block| block.hash() == challenge.block_hash)
        && !chain
            .observed_invalid_blocks
            .contains_key(&challenge.block_hash)
    {
        return NetworkPayloadApply::Pending;
    }
    chain
        .apply_command(ChainCommand::SubmitBlockCheckChallenge(challenge))
        .map(|_| NetworkPayloadApply::Applied)
        .unwrap_or(NetworkPayloadApply::Invalid)
}

pub fn apply_network_observed_block_check_challenge_payload(
    chain: &mut Chain,
    challenge_id: Hash,
    block_hash: Hash,
    challenger: Hash,
    observed_block_payload: &[u8],
    challenge_payload: &[u8],
) -> NetworkPayloadApply {
    if challenge_id == [0; 32] || block_hash == [0; 32] || challenger == [0; 32] {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(observed_block) = decode_block_payload(observed_block_payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if observed_block.hash() != block_hash {
        return NetworkPayloadApply::Invalid;
    }
    let Ok(challenge) = decode_block_check_challenge_payload(challenge_payload) else {
        return NetworkPayloadApply::Invalid;
    };
    if challenge.receipt_id == [0; 32]
        || block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id) != challenge_id
        || challenge.block_hash != block_hash
        || challenge.challenger != challenger
    {
        return NetworkPayloadApply::Invalid;
    }
    if let Some(existing) = chain.state().block_check_challenges().get(&challenge_id) {
        return if existing.block_hash == challenge.block_hash
            && existing.receipt_id == challenge.receipt_id
            && existing.challenger == challenge.challenger
            && existing.expected_check_leaf == challenge.expected_check_leaf
            && existing.observed_check_leaf == challenge.observed_check_leaf
        {
            NetworkPayloadApply::Applied
        } else {
            NetworkPayloadApply::Invalid
        };
    }
    if observed_block.height > chain.state().height().saturating_add(1) {
        return NetworkPayloadApply::Pending;
    }
    let parent_known = observed_block.height == 0 && observed_block.parent_hash == [0; 32]
        || chain.blocks.iter().any(|block| {
            block.height.saturating_add(1) == observed_block.height
                && block.hash() == observed_block.parent_hash
        });
    if !parent_known {
        return NetworkPayloadApply::Pending;
    }
    if !chain.observed_invalid_blocks.contains_key(&block_hash)
        && let Err(_error) = chain.cache_observed_invalid_block(observed_block)
    {
        return NetworkPayloadApply::Invalid;
    }
    apply_network_block_check_challenge_payload(
        chain,
        challenge_id,
        block_hash,
        challenger,
        challenge_payload,
    )
}

pub fn attestation_announcement_hash(attestation: &ValidatorAttestation) -> Hash {
    hash_bytes(
        b"tensor-vm-attestation-announcement-v1",
        &[
            &attestation.validator,
            &attestation.receipt_id,
            &attestation.job_id,
            &attestation.checks_root,
            &attestation.signature,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::super::{NetworkBlockPayloadApply, NetworkPayloadApply};
    use super::*;
    use crate::{
        chain::{
            BlockProductionKind, BlockVote, ChainCommand, ChainEngine, ChainParams, JobState,
            ReceiptState, TensorBlock, ValidatorAuditReport,
        },
        challenge::{BlockCheckChallenge, block_check_challenge_id},
        jobs::{MatmulJob, PrimitiveType, TensorOpReceipt},
        p2p::{
            encode_attestation_payload, encode_block_check_challenge_payload, encode_block_payload,
            encode_block_vote_payload, encode_job_payload, encode_receipt_payload,
            encode_validator_audit_report_payload,
        },
        scheduler::{JobScheduler, SyntheticLocalJobSource},
        testnet::{LocalTestnet, TestnetConfig},
        types::{address, sign},
        verify::{
            AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
            verify_tensor_op,
        },
    };

    fn local_matmul_round(seed_label: &[u8]) -> LocalTestnet {
        let mut testnet = LocalTestnet::new(
            TestnetConfig::default(),
            hash_bytes(b"tensor-vm-node-payload-test", &[seed_label]),
        );
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);
        testnet
    }

    #[test]
    fn job_payload_application_validates_submit_duplicates_and_invalid_edges() {
        let testnet = local_matmul_round(b"job");
        let job = testnet
            .chain
            .state()
            .jobs()
            .values()
            .next()
            .expect("local round must produce a job")
            .clone();
        let job_id = job.job_id();
        let payload = encode_job_payload(&job);
        let mut chain = testnet.chain.clone();
        chain.remove_job_for_testing(&job_id);

        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(chain.state().jobs().get(&job_id), Some(&job));
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, [0; 32], &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, hash_bytes(b"test", &[b"wrong-job"]), &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &[1, 2, 3]),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = job.clone();
        match &mut conflicting {
            JobState::TensorOp(job) => job.reward_weight = job.reward_weight.saturating_add(1),
            JobState::LinearTrainingStep(job) => {
                job.reward_weight = job.reward_weight.saturating_add(1)
            }
            JobState::GraphExecution(job) => {
                job.reward_weight = job.reward_weight.saturating_add(1)
            }
        }
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &encode_job_payload(&conflicting)),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn graph_job_payload_waits_for_registered_program_body() {
        let seed = hash_bytes(b"test", &[b"graph-job-pending-program"]);
        let mut chain = Chain::new(seed);
        let mut source = SyntheticLocalJobSource::default();
        let graph = SyntheticLocalJobSource::graph_execution_graph();
        let job = JobState::GraphExecution(source.next_graph_job(&chain));
        let job_id = job.job_id();
        let payload = encode_job_payload(&job);

        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            NetworkPayloadApply::Pending
        );
        assert!(!chain.state().jobs().contains_key(&job_id));

        chain
            .apply_command(ChainCommand::RegisterProgramBody {
                graph_id: graph.graph_id(),
                bytes: graph.canonical_json().into_bytes(),
            })
            .unwrap();
        assert_eq!(
            apply_network_job_payload(&mut chain, job_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(chain.state().jobs().get(&job_id), Some(&job));
    }

    #[test]
    fn block_payload_application_admits_next_head_and_rejects_bad_edges() {
        let seed = hash_bytes(b"test", &[b"network-block-payload"]);
        let validator = hash_bytes(b"test", &[b"network-block-validator"]);
        let mut producer = Chain::new(seed);
        producer.register_validator(validator, 10_000).unwrap();
        producer.produce_block(validator, 1_000).unwrap();
        let mut consumer = producer.clone();
        let parent_chain = consumer.clone();
        let block = producer.produce_block(validator, 1_012).unwrap();
        let block_hash = block.hash();
        let payload = encode_block_payload(&block);

        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, block_hash, &payload),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(consumer.blocks, producer.blocks);
        assert!(!consumer.state().finalized_blocks().contains(&block_hash));
        assert!(!consumer.has_block_finality(&block_hash));
        let vote = BlockVote::new(validator, 10_000, &block);
        assert_eq!(
            apply_network_block_vote_payload(
                &mut parent_chain.clone(),
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Pending
        );
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Applied
        );
        assert!(consumer.state().finalized_blocks().contains(&block_hash));
        assert!(consumer.has_block_finality(&block_hash));
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                vote.validator,
                &encode_block_vote_payload(&vote),
            ),
            NetworkPayloadApply::Applied
        );
        let mut conflicting_vote = vote.clone();
        conflicting_vote.signature = [8; 32];
        assert_eq!(
            apply_network_block_vote_payload(
                &mut consumer,
                block_hash,
                conflicting_vote.validator,
                &encode_block_vote_payload(&conflicting_vote),
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, block_hash, &payload),
            NetworkBlockPayloadApply::Applied { appended: 0 }
        );
        assert_eq!(
            apply_network_block_payload(&mut consumer, block.height, [0; 32], &payload),
            NetworkBlockPayloadApply::Invalid
        );

        let mut bad_signature = block.clone();
        bad_signature.proposer_signature = [9; 32];
        assert_eq!(
            apply_network_block_payload(
                &mut parent_chain.clone(),
                bad_signature.height,
                bad_signature.hash(),
                &encode_block_payload(&bad_signature),
            ),
            NetworkBlockPayloadApply::Invalid
        );

        let mut bad_state_root = block.clone();
        bad_state_root.state_root = hash_bytes(b"test", &[b"wrong-block-state-root"]);
        while !bad_state_root.pow_valid() {
            bad_state_root.nonce = bad_state_root.nonce.saturating_add(1);
        }
        let bad_state_root_hash = bad_state_root.hash();
        bad_state_root.proposer_signature = sign(&bad_state_root.proposer, &bad_state_root_hash);
        bad_state_root.validator_signature_aggregate =
            hash_bytes(b"tensor-vm-validator-aggregate", &[&bad_state_root_hash]);
        assert_eq!(
            apply_network_block_payload(
                &mut parent_chain.clone(),
                bad_state_root.height,
                bad_state_root_hash,
                &encode_block_payload(&bad_state_root),
            ),
            NetworkBlockPayloadApply::Invalid
        );

        let future = producer.produce_block(validator, 1_024).unwrap();
        let future_hash = future.hash();
        assert_eq!(
            apply_network_block_payload(
                &mut Chain::new(seed),
                future.height,
                future_hash,
                &encode_block_payload(&future),
            ),
            NetworkBlockPayloadApply::Pending
        );

        let mut conflicting = block.clone();
        conflicting.timestamp = conflicting.timestamp.saturating_add(1);
        assert_eq!(
            apply_network_block_payload(
                &mut producer.clone(),
                conflicting.height,
                conflicting.hash(),
                &encode_block_payload(&conflicting),
            ),
            NetworkBlockPayloadApply::Invalid
        );
    }

    #[test]
    fn block_payload_application_replaces_current_head_with_better_useful_pow() {
        let seed = hash_bytes(b"test", &[b"network-competing-head"]);
        let mut parent = Chain::new(seed);
        let miner = address(b"network-competing-miner");
        let validator_a = address(b"network-competing-validator-a");
        let validator_b = address(b"network-competing-validator-b");
        parent.register_miner(miner, 100).unwrap();
        parent.register_validator(validator_a, 10_000).unwrap();
        parent.register_validator(validator_b, 10_000).unwrap();
        parent.produce_block(validator_a, 1_000).unwrap();
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &seed, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        parent.insert_receipt_for_testing(ReceiptState::TensorOp(receipt.clone()));
        parent.mark_receipt_settled_for_testing(receipt.receipt_id);

        let mut branch_a = parent.clone();
        let mut branch_b = parent.clone();
        let block_a = branch_a.produce_block(validator_a, 1_012).unwrap();
        let block_b = branch_b.produce_block(validator_b, 1_012).unwrap();
        assert_eq!(
            block_a.production_kind,
            BlockProductionKind::UsefulVerificationPow
        );
        assert_eq!(
            block_b.production_kind,
            BlockProductionKind::UsefulVerificationPow
        );
        let (better, worse) = if block_a
            .pow_hash()
            .cmp(&block_b.pow_hash())
            .then_with(|| block_a.hash().cmp(&block_b.hash()))
            .is_lt()
        {
            (block_a, block_b)
        } else {
            (block_b, block_a)
        };

        let mut consumer = parent;
        assert_eq!(
            apply_network_block_payload(
                &mut consumer,
                worse.height,
                worse.hash(),
                &encode_block_payload(&worse),
            ),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(
            apply_network_block_payload(
                &mut consumer,
                better.height,
                better.hash(),
                &encode_block_payload(&better),
            ),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(
            consumer.blocks().last().map(TensorBlock::hash),
            Some(better.hash())
        );
    }

    #[test]
    fn receipt_payload_application_reports_pending_applied_and_invalid_edges() {
        let testnet = local_matmul_round(b"receipt");
        let receipt = testnet
            .chain
            .state()
            .receipts()
            .values()
            .next()
            .expect("local round must produce a receipt")
            .clone();
        let receipt_id = receipt.receipt_id();
        let payload = encode_receipt_payload(&receipt);

        let mut missing_job_chain = testnet.chain.clone();
        missing_job_chain.remove_job_for_testing(&receipt.job_id());
        missing_job_chain.remove_receipt_for_testing(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut missing_job_chain, receipt_id, &payload),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain.remove_receipt_for_testing(&receipt_id);
        apply_chain.remove_attestations_for_testing(&receipt_id);
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, receipt_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_chain.state().receipts().get(&receipt_id),
            Some(&receipt)
        );
        assert_eq!(
            apply_network_receipt_payload(&mut testnet.chain.clone(), receipt_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, [0; 32], &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_receipt_payload(
                &mut apply_chain,
                hash_bytes(b"test", &[b"wrong-receipt"]),
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_receipt_payload(&mut apply_chain, receipt_id, &[1, 2, 3]),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = receipt.clone();
        match &mut conflicting {
            ReceiptState::TensorOp(receipt) => {
                receipt.execution_time_ms = receipt.execution_time_ms.saturating_add(1)
            }
            ReceiptState::LinearTrainingStep(receipt) => {
                receipt.execution_time_ms = receipt.execution_time_ms.saturating_add(1)
            }
            ReceiptState::GraphExecution(receipt) => {
                receipt.execution_time_ms = receipt.execution_time_ms.saturating_add(1)
            }
        }
        assert_eq!(
            apply_network_receipt_payload(
                &mut testnet.chain.clone(),
                receipt_id,
                &encode_receipt_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn attestation_payload_application_reports_pending_applied_and_invalid_edges() {
        let testnet = local_matmul_round(b"attestation");
        let attestation = testnet
            .chain
            .state()
            .attestations()
            .values()
            .flat_map(|items| items.iter())
            .next()
            .expect("local round must produce an attestation")
            .clone();
        let attestation_id = attestation_announcement_hash(&attestation);
        let payload = encode_attestation_payload(&attestation);

        let mut missing_receipt_chain = testnet.chain.clone();
        missing_receipt_chain.remove_receipt_for_testing(&attestation.receipt_id);
        missing_receipt_chain.remove_attestations_for_testing(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut missing_receipt_chain, attestation_id, &payload,),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = testnet.chain.clone();
        apply_chain.remove_attestations_for_testing(&attestation.receipt_id);
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, attestation_id, &payload),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_chain
                .state()
                .attestations()
                .get(&attestation.receipt_id)
                .and_then(|items| items.first()),
            Some(&attestation)
        );
        assert_eq!(
            apply_network_attestation_payload(&mut testnet.chain.clone(), attestation_id, &payload,),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, [0; 32], &payload),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_attestation_payload(
                &mut apply_chain,
                hash_bytes(b"test", &[b"wrong-attestation"]),
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_attestation_payload(&mut apply_chain, attestation_id, &[1, 2, 3]),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = attestation.clone();
        conflicting.checks_root = hash_bytes(b"test", &[b"conflicting-attestation"]);
        let conflicting_id = attestation_announcement_hash(&conflicting);
        assert_eq!(
            apply_network_attestation_payload(
                &mut testnet.chain.clone(),
                conflicting_id,
                &encode_attestation_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
    }

    fn audit_report_chain() -> (Chain, Hash, Hash) {
        let beacon = hash_bytes(b"test", &[b"network-audit-report-beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            validator_audit_sample_numerator: 1,
            validator_audit_sample_denominator: 1,
            validator_audit_window_blocks: 3,
            freivalds: FreivaldsParams {
                validators_per_job: 1,
                minimum_validators: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"network-audit-miner");
        let candidate_auditor = address(b"network-audit-auditor");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(candidate_auditor, 10_000).unwrap();
        let validators: Vec<_> = (0..4)
            .map(|i| address(format!("network-audit-validator-{i}").as_bytes()))
            .collect();
        for validator in &validators {
            chain.register_validator(*validator, 10_000).unwrap();
        }
        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let assigned = JobScheduler::default()
            .assign_validators(
                &chain,
                receipt.receipt_id,
                &chain.validator_assignment_seed(&receipt.receipt_id),
            )
            .validators[0];
        chain
            .submit_attestation(ValidatorAttestation::new(
                assigned,
                10_000,
                AttestationStatement {
                    receipt_id: receipt.receipt_id,
                    job_id: receipt.job_id,
                    primitive_type: PrimitiveType::TensorOp,
                    result: VerificationResult::Valid,
                    checks_root: hash_bytes(b"test", &[b"network-audit-attestation"]),
                    data_availability_passed: true,
                },
            ))
            .unwrap();
        let proposer = chain.proposer_for_next_epoch(&beacon).unwrap();
        chain.produce_block(proposer, 1_000).unwrap();
        let audit_id = *chain
            .state()
            .validator_audit_assignments()
            .keys()
            .next()
            .expect("audit assignment should exist");
        let auditor = chain.state().validator_audit_assignments()[&audit_id].auditor;
        (chain, audit_id, auditor)
    }

    fn block_check_challenge_chain() -> (Chain, BlockCheckChallenge, Hash, Vec<u8>) {
        let beacon = hash_bytes(b"test", &[b"network-block-check-challenge-beacon"]);
        let params = ChainParams {
            agreement_quorum: 1,
            challenge_window_epochs: 1,
            epoch_length: 4,
            freivalds: FreivaldsParams {
                minimum_validators: 1,
                validators_per_job: 1,
                ..FreivaldsParams::default()
            },
            ..ChainParams::default()
        };
        let mut chain = Chain::with_params(params, beacon);
        let miner = address(b"network-block-check-miner");
        let proposer = address(b"network-block-check-proposer");
        let challenger = address(b"network-block-check-watcher");
        chain.register_miner(miner, 100).unwrap();
        chain.register_validator(proposer, 10_000).unwrap();
        chain.register_validator(challenger, 10_000).unwrap();

        let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
        let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
        let report = verify_tensor_op(
            &job,
            &receipt,
            &a,
            &b,
            &c,
            &hash_bytes(b"test", &[b"network-block-check-validation"]),
            &chain.params().freivalds,
        )
        .unwrap();
        chain.submit_job(JobState::TensorOp(job));
        chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
        let assigned = JobScheduler::default()
            .assign_validators(
                &chain,
                receipt.receipt_id,
                &chain.validator_assignment_seed(&receipt.receipt_id),
            )
            .validators[0];
        chain.insert_attestation_for_testing(ValidatorAttestation::new(
            assigned,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ));
        chain.settle_epoch(1_000, 500);
        let block = chain
            .produce_block_with_rewards(proposer, 1_000, 900, 100)
            .unwrap();
        let diagnostic = chain
            .deterministic_bad_block_check_challenge(&block, challenger)
            .unwrap();
        chain
            .install_diagnostic_observed_block(&diagnostic)
            .unwrap();
        assert!(
            chain
                .blocks()
                .iter()
                .any(|stored| stored.hash() == block.hash())
        );
        assert!(
            !chain
                .blocks()
                .iter()
                .any(|stored| stored.hash() == diagnostic.observed_block.hash())
        );
        let observed_block_payload = encode_block_payload(&diagnostic.observed_block);
        let challenge = diagnostic.challenge;
        let challenge_id = block_check_challenge_id(&challenge.block_hash, &challenge.receipt_id);
        (chain, challenge, challenge_id, observed_block_payload)
    }

    #[test]
    fn block_check_challenge_payload_application_reports_pending_applied_and_invalid_edges() {
        let (chain, challenge, challenge_id, _observed_block_payload) =
            block_check_challenge_chain();
        let payload = encode_block_check_challenge_payload(&challenge);
        let mut missing_block = chain.clone();
        missing_block.pop_block_for_testing();
        missing_block.observed_invalid_blocks.clear();
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut missing_block,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &payload,
            ),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = chain.clone();
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &payload,
            ),
            NetworkPayloadApply::Applied
        );
        assert!(
            apply_chain
                .state()
                .block_check_challenges()
                .contains_key(&challenge_id)
        );
        let pending_reward = apply_chain
            .state()
            .pending_challenge_rewards()
            .values()
            .find(|reward| {
                reward.challenge_id == challenge_id
                    && reward.block_hash == challenge.block_hash
                    && reward.receipt_id == challenge.receipt_id
                    && reward.challenger == challenge.challenger
            })
            .expect("accepted network challenge should delay challenger reward");
        assert_eq!(pending_reward.amount, 500);
        assert_eq!(
            pending_reward.claimable_at_height,
            apply_chain
                .state()
                .height()
                .saturating_add(apply_chain.params().reward_maturity_delay_blocks())
        );
        let challenge_reward_claimable_at_height = pending_reward.claimable_at_height;
        assert_eq!(
            apply_chain.state().rewards().balance(&challenge.challenger),
            0
        );
        assert!(
            apply_chain
                .release_matured_challenge_rewards()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            apply_chain.state().rewards().balance(&challenge.challenger),
            0
        );
        apply_chain.set_position_for_testing(challenge_reward_claimable_at_height, 1);
        assert_eq!(
            apply_chain
                .release_matured_challenge_rewards()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            apply_chain.state().rewards().balance(&challenge.challenger),
            500
        );
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &payload,
            ),
            NetworkPayloadApply::Applied
        );
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                [0; 32],
                challenge.block_hash,
                challenge.challenger,
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                hash_bytes(b"test", &[b"wrong-challenge-block"]),
                challenge.challenger,
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                address(b"wrong-challenge-validator"),
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &[1, 2, 3],
            ),
            NetworkPayloadApply::Invalid
        );

        let mut conflicting = challenge.clone();
        conflicting.observed_check_leaf = hash_bytes(b"test", &[b"conflicting-check-leaf"]);
        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &encode_block_check_challenge_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn observed_block_check_challenge_payload_caches_observation_and_applies() {
        let (chain, challenge, challenge_id, observed_block_payload) =
            block_check_challenge_chain();
        let challenge_payload = encode_block_check_challenge_payload(&challenge);
        let mut apply_chain = chain.clone();
        apply_chain.observed_invalid_blocks.clear();

        assert_eq!(
            apply_network_block_check_challenge_payload(
                &mut apply_chain.clone(),
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &challenge_payload,
            ),
            NetworkPayloadApply::Pending
        );
        assert_eq!(
            apply_network_observed_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &observed_block_payload,
                &challenge_payload,
            ),
            NetworkPayloadApply::Applied
        );
        assert!(
            apply_chain
                .state()
                .block_check_challenges()
                .contains_key(&challenge_id)
        );
        assert_eq!(
            apply_network_observed_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                hash_bytes(b"test", &[b"wrong-observed-challenge-block"]),
                challenge.challenger,
                &observed_block_payload,
                &challenge_payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_observed_block_check_challenge_payload(
                &mut apply_chain,
                challenge_id,
                challenge.block_hash,
                challenge.challenger,
                &[1, 2, 3],
                &challenge_payload,
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn validator_audit_report_payload_application_reports_pending_applied_and_invalid_edges() {
        let (chain, audit_id, auditor) = audit_report_chain();
        let report = ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Valid,
            true,
            hash_bytes(b"test", &[b"network-audit-canonical"]),
        );
        let payload = encode_validator_audit_report_payload(&report);

        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut Chain::new(hash_bytes(b"test", &[b"missing-audit-assignment"])),
                audit_id,
                auditor,
                &payload,
            ),
            NetworkPayloadApply::Pending
        );

        let mut apply_chain = chain.clone();
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut apply_chain,
                audit_id,
                auditor,
                &payload,
            ),
            NetworkPayloadApply::Applied
        );
        assert!(apply_chain.state().validator_audit_results()[&audit_id].passed);
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut apply_chain,
                audit_id,
                auditor,
                &payload,
            ),
            NetworkPayloadApply::Applied
        );

        let conflicting = ValidatorAuditReport::new(
            audit_id,
            auditor,
            VerificationResult::Invalid,
            true,
            hash_bytes(b"test", &[b"network-audit-conflict"]),
        );
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut apply_chain,
                audit_id,
                auditor,
                &encode_validator_audit_report_payload(&conflicting),
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut chain.clone(),
                [0; 32],
                auditor,
                &payload
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut chain.clone(),
                audit_id,
                [0; 32],
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut chain.clone(),
                hash_bytes(b"test", &[b"wrong-audit-id"]),
                auditor,
                &payload,
            ),
            NetworkPayloadApply::Invalid
        );
        assert_eq!(
            apply_network_validator_audit_report_payload(
                &mut chain.clone(),
                audit_id,
                auditor,
                &[1, 2, 3],
            ),
            NetworkPayloadApply::Invalid
        );
    }

    #[test]
    fn block_payload_application_reorganizes_to_longer_side_branch() {
        let seed = hash_bytes(b"test", &[b"network-side-branch-reorg"]);
        let params = ChainParams {
            pow_timeout_blocks: 1,
            ..ChainParams::default()
        };
        let mut parent = Chain::with_params(params, seed);
        let validator = address(b"network-side-branch-validator");
        parent
            .register_validator(validator, parent.params().validator_min_stake)
            .unwrap();
        let base = parent.produce_block(validator, 1_000).unwrap();

        let mut canonical = parent.clone();
        let canonical_one = canonical.produce_block(validator, 1_006).unwrap();
        let canonical_two = canonical.produce_block(validator, 1_012).unwrap();

        let mut branch = parent.clone();
        let side_one = branch.produce_block(validator, 1_007).unwrap();
        let side_two = branch.produce_block(validator, 1_013).unwrap();
        let side_three = branch.produce_block(validator, 1_019).unwrap();
        let branch_state = branch.state().clone();

        let mut consumer = parent;
        assert_eq!(
            apply_network_block_payload(
                &mut consumer,
                canonical_one.height,
                canonical_one.hash(),
                &encode_block_payload(&canonical_one),
            ),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(
            apply_network_block_payload(
                &mut consumer,
                canonical_two.height,
                canonical_two.hash(),
                &encode_block_payload(&canonical_two),
            ),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(
            consumer.blocks().last().map(TensorBlock::hash),
            Some(canonical_two.hash())
        );

        for block in [&side_one, &side_two] {
            assert_eq!(
                apply_network_block_payload(
                    &mut consumer,
                    block.height,
                    block.hash(),
                    &encode_block_payload(block),
                ),
                NetworkBlockPayloadApply::Applied { appended: 1 }
            );
            assert_eq!(
                consumer.blocks().last().map(TensorBlock::hash),
                Some(canonical_two.hash())
            );
        }
        assert_eq!(
            apply_network_block_payload(
                &mut consumer,
                side_three.height,
                side_three.hash(),
                &encode_block_payload(&side_three),
            ),
            NetworkBlockPayloadApply::Applied { appended: 1 }
        );
        assert_eq!(consumer.state(), &branch_state);
        assert_eq!(
            consumer
                .blocks()
                .iter()
                .map(TensorBlock::hash)
                .collect::<Vec<_>>(),
            vec![
                base.hash(),
                side_one.hash(),
                side_two.hash(),
                side_three.hash()
            ]
        );
    }
}
