use crate::{
    Chain, NodeRuntimeState, NodeStore, RpcHttpServer, TensorVmLibp2pService, types::Address,
};

use super::{
    ServiceRuntimeConfig, chain_announcement_checkpoint, ensure_validator_role_vrf_key,
    fetch_validator_role_missing_tensors, publish_block_payload_announcements,
    publish_block_vote_announcements, publish_chain_payload_announcements,
    publish_new_chain_announcements, publish_observed_block_check_challenge,
    publish_runtime_trace_bisection_expectation, publish_runtime_trace_bisection_open,
    publish_runtime_trace_bisection_referee, publish_validator_block_proposal,
    runtime_production::next_block_timestamp, runtime_role_wallet_registration,
    submit_runtime_trace_bisection_expectation, submit_runtime_trace_bisection_open,
    submit_runtime_trace_bisection_referee, submit_validator_role_attestation,
    submit_validator_role_audit_report, submit_validator_role_block_proposal,
    submit_validator_role_block_vote, validator_role_audit_observation,
    validator_role_block_proposal_observation, validator_role_work_observation,
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
    let mut status_changed = false;
    if let Some(registration) = ensure_validator_role_vrf_key(
        &mut server.gateway_mut().node,
        validator,
        config.role_wallet_secret.as_deref(),
    )? {
        runtime_state.record_validator_vrf_key_observation(
            registration.vrf_public_key,
            registration.registered_new_key,
        );
        if registration.registered_new_key {
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| format!("failed to persist validator vrf key: {error}"))?;
            status_changed = true;
        }
    }
    let observation = validator_role_work_observation(&server.gateway().node, validator);
    let receipt_to_fetch = observation.artifact_missing_receipts.iter().next().copied();
    let mut receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
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
            store,
            &mut server.gateway_mut().node,
            p2p_service,
            receipt_id,
        )?;
        if fetch_report.has_activity() {
            if fetch_report.programs_registered > 0 {
                store
                    .persist_chain(&server.gateway().node.chain)
                    .map_err(|error| format!("failed to persist fetched graph program: {error}"))?;
            }
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
            config.role_wallet_secret.as_deref(),
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
    if publish_block_vote_announcements(p2p_service, &server.gateway().node.chain)? > 0 {
        status_changed = true;
    }
    if publish_block_payload_announcements(p2p_service, &server.gateway().node.chain)? > 0 {
        status_changed = true;
    }
    if publish_chain_payload_announcements(p2p_service, &server.gateway().node.chain)? > 0 {
        status_changed = true;
    }
    if publish_pending_proposer_reward_diagnostic(&server.gateway().node.chain, p2p_service)? {
        status_changed = true;
    }
    if let Some(open) =
        submit_runtime_trace_bisection_open(&mut server.gateway_mut().node, validator)?
    {
        publish_runtime_trace_bisection_open(p2p_service, &open)?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| {
                format!("failed to persist runtime trace-bisection open state: {error}")
            })?;
        runtime_state.record_validator_trace_bisection_open_submission(1);
        status_changed = true;
    }
    if let Some(expectation) =
        submit_runtime_trace_bisection_expectation(&mut server.gateway_mut().node, validator)?
    {
        publish_runtime_trace_bisection_expectation(p2p_service, &expectation)?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| {
                format!("failed to persist runtime trace-bisection expectation state: {error}")
            })?;
        runtime_state.record_validator_trace_bisection_expectation_submission(1);
        status_changed = true;
    }
    if let Some(referee) =
        submit_runtime_trace_bisection_referee(&mut server.gateway_mut().node, validator)?
    {
        publish_runtime_trace_bisection_referee(p2p_service, &referee)?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| {
                format!("failed to persist runtime trace-bisection referee state: {error}")
            })?;
        runtime_state.record_validator_trace_bisection_referee_submission(1);
        status_changed = true;
    }
    let local_block_proposer_delay_satisfied = config
        .node
        .local_block_proposer_delay_satisfied(server.gateway().node.chain.state().height());
    let proposer_cadence_ready = server
        .gateway()
        .node
        .chain
        .proposer_cadence_ready(validator);
    let proposer_challenge_throttle_ready = server
        .gateway()
        .node
        .chain
        .proposer_challenge_throttle_ready(validator);
    if config.node.local_block_proposer()
        && local_block_proposer_delay_satisfied
        && proposer_cadence_ready
        && proposer_challenge_throttle_ready
    {
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
        let selected_useful_proposer =
            useful_proposer_selected(&server.gateway().node.chain, validator);
        let selected_local_proposer =
            fallback_proposer_selected(&server.gateway().node.chain, validator);
        let fallback_work_ready =
            observation.settled_receipts.is_empty() && selected_local_proposer;
        let timestamp = if fallback_work_ready {
            fallback_block_timestamp(&server.gateway().node.chain)
        } else {
            next_block_timestamp(server)
        };
        let proposer_work_ready = (!observation.settled_receipts.is_empty()
            && selected_useful_proposer)
            || fallback_work_ready;
        if runtime_state.record_validator_block_proposal_observation(
            observation.settled_receipts,
            observation.artifact_ready_receipts,
            observation.attested_receipts,
        ) {
            status_changed = true;
        }
        if proposer_work_ready
            && let Some(proposal) = submit_validator_role_block_proposal(
                &mut server.gateway_mut().node,
                validator,
                timestamp,
            )?
        {
            let Some(block) = server.gateway().node.chain.blocks().last() else {
                return Ok(status_changed);
            };
            let diagnostic = {
                let diagnostic_block =
                    diagnostic_block_with_pending_proposer_reward(&server.gateway().node.chain);
                diagnostic_block
                    .as_ref()
                    .and_then(|block| {
                        diagnostic_block_check_challenger(
                            &server.gateway().node.chain,
                            block.proposer,
                        )
                        .map(|challenger| (block, challenger))
                    })
                    .map(|challenger| {
                        let (block, challenger) = challenger;
                        server
                            .gateway()
                            .node
                            .chain
                            .deterministic_bad_block_check_challenge(block, challenger)
                    })
                    .transpose()
                    .map_err(|error| {
                        format!("failed to build live diagnostic block-check challenge: {error}")
                    })?
                    .filter(|diagnostic| {
                        !server
                            .gateway()
                            .node
                            .chain
                            .state()
                            .block_check_challenges()
                            .contains_key(&diagnostic.challenge_id)
                    })
            };
            let block_hash = block.hash();
            let parent_state = server
                .gateway()
                .node
                .chain
                .block_parent_state_for_payload(&block_hash)
                .ok_or_else(|| "validator block missing parent-state payload".to_owned())?;
            publish_validator_block_proposal(
                p2p_service,
                block,
                &proposal.selected_receipts,
                parent_state,
            )?;
            if let Some(diagnostic) = diagnostic {
                publish_observed_block_check_challenge(p2p_service, &diagnostic)?;
            }
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| format!("failed to persist validator block proposal: {error}"))?;
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
    Ok(status_changed)
}

fn diagnostic_block_check_challenger(chain: &Chain, proposer: Address) -> Option<Address> {
    chain
        .state()
        .validators()
        .keys()
        .copied()
        .find(|validator| *validator != proposer)
        .or_else(|| chain.state().validators().keys().copied().next())
}

fn publish_pending_proposer_reward_diagnostic(
    chain: &Chain,
    p2p_service: &TensorVmLibp2pService,
) -> std::result::Result<bool, String> {
    if !chain.state().pending_challenge_rewards().is_empty() {
        return Ok(false);
    }
    let Some(block) = diagnostic_block_with_pending_proposer_reward(chain) else {
        return Ok(false);
    };
    let Some(challenger) = diagnostic_block_check_challenger(chain, block.proposer) else {
        return Ok(false);
    };
    let diagnostic = chain
        .deterministic_bad_block_check_challenge(&block, challenger)
        .map_err(|error| {
            format!("failed to build live diagnostic block-check challenge: {error}")
        })?;
    if chain
        .state()
        .block_check_challenges()
        .contains_key(&diagnostic.challenge_id)
    {
        return Ok(false);
    }
    publish_observed_block_check_challenge(p2p_service, &diagnostic)?;
    Ok(true)
}

fn diagnostic_block_with_pending_proposer_reward(
    chain: &Chain,
) -> Option<crate::chain::TensorBlock> {
    chain
        .state()
        .pending_proposer_rewards()
        .values()
        .find_map(|reward| {
            chain
                .blocks()
                .iter()
                .find(|block| {
                    block.height == reward.block_height
                        && block.proposer == reward.proposer
                        && block.production_kind.requires_pow()
                        && chain.is_block_finalized(&block.hash())
                })
                .cloned()
        })
}

fn fallback_block_timestamp(chain: &Chain) -> u64 {
    let Some(parent) = chain.blocks().last() else {
        return 0;
    };
    let timeout_seconds = chain
        .params()
        .pow_timeout_blocks
        .max(1)
        .saturating_mul(chain.params().block_time_seconds.max(1));
    parent.timestamp.saturating_add(timeout_seconds)
}

fn fallback_proposer_selected(chain: &Chain, validator: Address) -> bool {
    chain.proposer_for_next_epoch(&chain.state().finalized_randomness()) == Some(validator)
}

fn useful_proposer_selected(chain: &Chain, validator: Address) -> bool {
    chain
        .state()
        .validators()
        .keys()
        .copied()
        .find(|candidate| {
            chain.proposer_cadence_ready(*candidate)
                && chain.proposer_challenge_throttle_ready(*candidate)
        })
        == Some(validator)
}
