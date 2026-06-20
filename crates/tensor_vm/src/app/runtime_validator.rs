use crate::{NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService};

use super::{
    ServiceRuntimeConfig, chain_announcement_checkpoint, fetch_validator_role_missing_tensors,
    publish_new_chain_announcements, publish_validator_block_proposal,
    runtime_production::next_block_timestamp, runtime_role_wallet_registration,
    submit_validator_role_attestation, submit_validator_role_audit_report,
    submit_validator_role_block_proposal, submit_validator_role_block_vote,
    validator_role_audit_observation, validator_role_block_proposal_observation,
    validator_role_work_observation,
};

pub fn tick_validator_role_work_once(
    config: &ServiceRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    let Some(validator) = config.role_wallet_address else {
        return Ok(false);
    };
    if runtime_role_wallet_registration(
        config.role,
        config.role_wallet_address,
        &server.gateway().node.chain,
    ) != "validator"
    {
        return Ok(false);
    }
    let observation = validator_role_work_observation(&server.gateway().node, validator);
    let receipt_to_fetch = observation.artifact_missing_receipts.iter().next().copied();
    let mut receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
    let mut status_changed = false;
    if runtime_state.record_validator_work_observation(
        observation.assigned_receipts,
        observation.unattested_receipts,
        observation.artifact_ready_receipts,
        observation.artifact_missing_receipts,
    ) {
        status_changed = true;
    }
    if receipt_to_submit.is_none()
        && let Some(receipt_id) = receipt_to_fetch
    {
        let fetch_report = fetch_validator_role_missing_tensors(
            &mut server.gateway_mut().node,
            p2p_service,
            receipt_id,
        )?;
        if fetch_report.attempts > 0
            || fetch_report.successes > 0
            || fetch_report.failures > 0
            || fetch_report.tensors_inserted > 0
        {
            runtime_state.record_validator_remote_tensor_fetch(
                fetch_report.attempts,
                fetch_report.successes,
                fetch_report.failures,
                fetch_report.bytes,
                fetch_report.tensors_inserted,
            );
            let observation = validator_role_work_observation(&server.gateway().node, validator);
            receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
            runtime_state.record_validator_work_observation(
                observation.assigned_receipts,
                observation.unattested_receipts,
                observation.artifact_ready_receipts,
                observation.artifact_missing_receipts,
            );
            status_changed = true;
        }
    }
    if let Some(receipt_id) = receipt_to_submit {
        let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
        if let Some(submission) = submit_validator_role_attestation(
            &mut server.gateway_mut().node,
            validator,
            receipt_id,
        )? {
            publish_new_chain_announcements(
                p2p_service,
                &announcement_checkpoint,
                &server.gateway().node.chain,
            )?;
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| {
                    format!("failed to persist validator attestation state: {error}")
                })?;
            runtime_state
                .record_validator_attestation_submission(submission.attestations_submitted);
            let observation = validator_role_work_observation(&server.gateway().node, validator);
            runtime_state.record_validator_work_observation(
                observation.assigned_receipts,
                observation.unattested_receipts,
                observation.artifact_ready_receipts,
                observation.artifact_missing_receipts,
            );
            status_changed = true;
        }
    }
    let audit_observation = validator_role_audit_observation(&server.gateway().node, validator);
    let audit_to_submit = audit_observation
        .artifact_ready_audits
        .iter()
        .next()
        .copied();
    if runtime_state.record_validator_audit_observation(
        audit_observation.assigned_audits,
        audit_observation.unreported_audits,
        audit_observation.artifact_ready_audits,
        audit_observation.artifact_missing_audits,
    ) {
        status_changed = true;
    }
    if let Some(audit_id) = audit_to_submit {
        let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
        if let Some(submission) =
            submit_validator_role_audit_report(&mut server.gateway_mut().node, validator, audit_id)?
        {
            publish_new_chain_announcements(
                p2p_service,
                &announcement_checkpoint,
                &server.gateway().node.chain,
            )?;
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| {
                    format!("failed to persist validator audit report state: {error}")
                })?;
            runtime_state
                .record_validator_audit_report_submission(submission.audit_reports_submitted);
            let audit_observation =
                validator_role_audit_observation(&server.gateway().node, validator);
            runtime_state.record_validator_audit_observation(
                audit_observation.assigned_audits,
                audit_observation.unreported_audits,
                audit_observation.artifact_ready_audits,
                audit_observation.artifact_missing_audits,
            );
            status_changed = true;
        }
    }
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    if let Some(submission) =
        submit_validator_role_block_vote(&mut server.gateway_mut().node, validator)?
    {
        publish_new_chain_announcements(
            p2p_service,
            &announcement_checkpoint,
            &server.gateway().node.chain,
        )?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist validator block vote state: {error}"))?;
        runtime_state.record_validator_block_vote_submission(submission.block_votes_submitted);
        status_changed = true;
    }
    if config.node.local_block_proposer() {
        server
            .gateway_mut()
            .node
            .chain
            .prepare_block_parent_state()
            .map_err(|error| {
                format!("validator proposer failed to prepare parent state: {error}")
            })?;
        let observation =
            validator_role_block_proposal_observation(&server.gateway().node, validator);
        let proposer_work_ready = !observation.settled_receipts.is_empty();
        if runtime_state.record_validator_block_proposal_observation(
            observation.settled_receipts,
            observation.artifact_ready_receipts,
            observation.attested_receipts,
        ) {
            status_changed = true;
        }
        if proposer_work_ready {
            let timestamp = next_block_timestamp(server);
            if let Some(proposal) = submit_validator_role_block_proposal(
                &mut server.gateway_mut().node,
                validator,
                timestamp,
            )? {
                let Some(block) = server.gateway().node.chain.blocks().last() else {
                    return Ok(status_changed);
                };
                publish_validator_block_proposal(p2p_service, block)?;
                store
                    .persist_chain(&server.gateway().node.chain)
                    .map_err(|error| {
                        format!("failed to persist validator block proposal: {error}")
                    })?;
                runtime_state.record_produced_block();
                runtime_state.record_validator_block_proposal_submission(
                    proposal.blocks_proposed,
                    proposal.useful_blocks_proposed,
                    proposal.fallback_blocks_proposed,
                    proposal.selected_receipts.len(),
                );
                status_changed = true;
            }
        }
    }
    Ok(status_changed)
}
