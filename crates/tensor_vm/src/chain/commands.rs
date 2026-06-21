use super::{
    BlockAdmission, Chain, ChainCommand, ChainEngine, ChainEvent, ChainParams, ChainState,
    PendingCreditReward, PendingReceiptReward, ReceiptRewardKind, ReceiptState, TensorBlock,
    accounts, challenges, receipts, settlement,
};
use crate::challenge::ChallengeOutcome;
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash, hash_bytes};

impl ChainEngine for Chain {
    fn apply_command(&mut self, command: ChainCommand) -> Result<Vec<ChainEvent>> {
        match command {
            ChainCommand::RegisterMiner { address, stake } => {
                self.register_miner(address, stake)?;
                Ok(vec![ChainEvent::MinerRegistered(address)])
            }
            ChainCommand::RegisterValidator { address, stake } => {
                self.register_validator(address, stake)?;
                Ok(vec![ChainEvent::ValidatorRegistered(address)])
            }
            ChainCommand::SubmitExternalRandomnessBeacon {
                source_id,
                beacon_round,
                randomness,
                proof_hash,
            } => {
                let record = super::validation::submit_external_randomness_beacon(
                    self,
                    source_id,
                    beacon_round,
                    randomness,
                    proof_hash,
                )?;
                Ok(vec![ChainEvent::ExternalRandomnessBeaconAccepted {
                    source_id: record.source_id,
                    beacon_round: record.beacon_round,
                    randomness: record.randomness,
                }])
            }
            ChainCommand::SubmitValidatorVrfReveal(reveal) => {
                let record = super::validation::submit_validator_vrf_reveal(self, reveal)?;
                Ok(vec![ChainEvent::ValidatorVrfRevealAccepted {
                    reveal_id: record.reveal_id,
                    receipt_id: record.receipt_id,
                    validator: record.validator,
                    beacon_round: record.beacon_round,
                }])
            }
            ChainCommand::Transfer { from, to, amount } => {
                self.transfer(from, to, amount)?;
                Ok(vec![ChainEvent::AccountTransferred { from, to, amount }])
            }
            ChainCommand::CreditReward { address, amount } => {
                let claimable_at_height = pending_credit_reward_claimable_height(self);
                let claim_id = pending_credit_reward_claim_id(
                    &address,
                    amount,
                    self.state.height,
                    self.state.pending_credit_rewards.len() as u64,
                );
                self.state.pending_credit_rewards.insert(
                    claim_id,
                    PendingCreditReward {
                        claim_id,
                        beneficiary: address,
                        amount,
                        claimable_at_height,
                    },
                );
                Ok(vec![ChainEvent::CreditRewardPending {
                    claim_id,
                    beneficiary: address,
                    amount,
                    claimable_at_height,
                }])
            }
            ChainCommand::ClaimReward(address) => {
                let mut events = claim_matured_rewards_for_beneficiary(&mut self.state, address);
                let amount = accounts::claim_reward(self, address)?;
                events.push(ChainEvent::RewardClaimed { address, amount });
                Ok(events)
            }
            ChainCommand::RegisterProgramBody { graph_id, bytes } => {
                receipts::register_program_body(self, graph_id, bytes)?;
                Ok(vec![ChainEvent::ProgramBodyRegistered { graph_id }])
            }
            ChainCommand::SubmitJob(job) => {
                let job_id = job.job_id();
                if let super::JobState::GraphExecution(graph_job) = &job {
                    receipts::submit_graph_job(self, graph_job)?;
                }
                self.submit_job(job);
                Ok(vec![ChainEvent::JobAccepted(job_id)])
            }
            ChainCommand::SubmitReceipt(receipt) => {
                let receipt_id = receipt.receipt_id();
                match receipt {
                    ReceiptState::TensorOp(receipt) => self.submit_tensor_op_receipt(receipt)?,
                    ReceiptState::LinearTrainingStep(receipt) => {
                        self.submit_linear_receipt(receipt)?
                    }
                    ReceiptState::GraphExecution(receipt) => self.submit_graph_receipt(receipt)?,
                }
                Ok(vec![ChainEvent::ReceiptAccepted(receipt_id)])
            }
            ChainCommand::SubmitAttestation(attestation) => {
                let receipt_id = attestation.receipt_id;
                let validator = attestation.validator;
                self.submit_attestation(attestation)?;
                Ok(vec![ChainEvent::AttestationAccepted {
                    receipt_id,
                    validator,
                }])
            }
            ChainCommand::SubmitValidatorAuditReport(report) => {
                let (result, slash) =
                    super::validation::submit_validator_audit_report(self, report)?;
                let mut events = vec![ChainEvent::ValidatorAuditAccepted {
                    audit_id: result.audit_id,
                    auditor: result.auditor,
                    validator: result.validator,
                    passed: result.passed,
                }];
                if let Some(slash) = slash {
                    events.push(ChainEvent::ValidatorAuditSlashApplied {
                        audit_id: slash.audit_id,
                        validator: slash.validator,
                        amount: slash.amount,
                        reason: slash.reason,
                    });
                }
                Ok(events)
            }
            ChainCommand::SubmitValidatorAuditAppeal(appeal) => {
                let record = super::validation::submit_validator_audit_appeal(self, appeal)?;
                Ok(vec![ChainEvent::ValidatorAuditAppealAccepted {
                    audit_id: record.audit_id,
                    validator: record.validator,
                    deadline_height: record.deadline_height,
                }])
            }
            ChainCommand::ResolveValidatorAuditAppeal {
                audit_id,
                resolution,
            } => {
                let outcome =
                    super::validation::resolve_validator_audit_appeal(self, audit_id, resolution)?;
                Ok(vec![ChainEvent::ValidatorAuditAppealResolved {
                    audit_id,
                    validator: outcome.validator,
                    resolution,
                    receipt_reward_reinstated: outcome.receipt_reward_reinstated,
                    stake_refunded_amount: outcome.stake_refunded_amount,
                }])
            }
            ChainCommand::SubmitBlock(block) => match self.admit_block(block)? {
                BlockAdmission::Applied { height, hash } => {
                    Ok(vec![ChainEvent::BlockAccepted { height, hash }])
                }
                BlockAdmission::Replaced {
                    height,
                    old_hash,
                    hash,
                } => Ok(vec![ChainEvent::BlockReplaced {
                    height,
                    old_hash,
                    hash,
                }]),
                BlockAdmission::Reorganized {
                    height,
                    old_head,
                    hash,
                } => Ok(vec![ChainEvent::ChainReorganized {
                    height,
                    old_head,
                    hash,
                }]),
                BlockAdmission::SideBranchStored {
                    height,
                    parent_hash,
                    hash,
                } => Ok(vec![ChainEvent::SideBranchBlockStored {
                    height,
                    parent_hash,
                    hash,
                }]),
                BlockAdmission::Duplicate { .. } => Ok(Vec::new()),
                BlockAdmission::PendingParent { .. } => {
                    Err(TvmError::InvalidReceipt("block parent pending"))
                }
                BlockAdmission::Invalid { .. } => {
                    Err(TvmError::InvalidReceipt("invalid block payload"))
                }
            },
            ChainCommand::SubmitBlockVote(vote) => {
                let block_hash = vote.block_hash;
                let validator = vote.validator;
                let was_finalized = self.is_block_finalized(&block_hash);
                self.submit_block_vote(vote)?;
                let mut events = vec![ChainEvent::BlockVoteAccepted {
                    block_hash,
                    validator,
                }];
                if !was_finalized && self.is_block_finalized(&block_hash) {
                    events.push(ChainEvent::BlockFinalized(block_hash));
                }
                Ok(events)
            }
            ChainCommand::SettleEpoch {
                miner_reward_pool,
                validator_reward_pool,
            } => {
                let settled_before = self.state.settled_receipts.clone();
                let pending_rewards_before = self.state.pending_receipt_rewards.clone();
                self.settle_epoch(miner_reward_pool, validator_reward_pool);
                Ok(settlement::events(
                    self,
                    &settled_before,
                    &pending_rewards_before,
                ))
            }
            ChainCommand::ProduceBlock {
                proposer,
                timestamp,
            } => {
                let block = self.produce_block(proposer, timestamp)?;
                Ok(vec![ChainEvent::BlockProduced {
                    height: block.height,
                    hash: block.hash(),
                }])
            }
            ChainCommand::ProduceRewardedBlock {
                proposer,
                timestamp,
                fixed_block_reward,
                fee_share,
            } => {
                let block = self.produce_block_with_rewards(
                    proposer,
                    timestamp,
                    fixed_block_reward,
                    fee_share,
                )?;
                Ok(vec![ChainEvent::BlockProduced {
                    height: block.height,
                    hash: block.hash(),
                }])
            }
            ChainCommand::ReleaseMaturedProposerRewards => {
                Ok(release_matured_proposer_rewards(&mut self.state))
            }
            ChainCommand::ReleaseMaturedReceiptRewards => {
                Ok(release_matured_receipt_rewards(&mut self.state))
            }
            ChainCommand::ReleaseMaturedChallengeRewards => {
                Ok(release_matured_challenge_rewards(&mut self.state))
            }
            ChainCommand::ReleaseMaturedCreditRewards => {
                Ok(release_matured_credit_rewards(&mut self.state))
            }
            ChainCommand::RegisterModel {
                model_id,
                architecture_hash,
                weight_root,
                config_hash,
            } => {
                self.register_model(model_id, architecture_hash, weight_root, config_hash)?;
                Ok(vec![ChainEvent::ModelRegistered(model_id)])
            }
            ChainCommand::ApplyModelTransition {
                model_id,
                step,
                weight_root_before,
                weight_root_after,
            } => {
                self.apply_model_transition(
                    &model_id,
                    step,
                    &weight_root_before,
                    weight_root_after,
                )?;
                Ok(vec![ChainEvent::ModelTransitionApplied {
                    model_id,
                    step,
                    weight_root_after,
                }])
            }
            ChainCommand::ApplyChallengeOutcome(outcome) => {
                let event = match &outcome {
                    ChallengeOutcome::Rejected { reason } => ChainEvent::ChallengeRejected {
                        reason: reason.clone(),
                    },
                    ChallengeOutcome::ProvenInvalid {
                        dishonest_party,
                        slash_amount,
                        reason,
                    } => ChainEvent::ChallengeProvenInvalid {
                        dishonest_party: *dishonest_party,
                        slash_amount: *slash_amount,
                        reason: reason.clone(),
                    },
                    ChallengeOutcome::BlockCheckProvenInvalid {
                        proposer,
                        proposer_reward_clawback,
                        reason,
                        ..
                    } => ChainEvent::ChallengeProvenInvalid {
                        dishonest_party: *proposer,
                        slash_amount: *proposer_reward_clawback,
                        reason: reason.clone(),
                    },
                };
                challenges::apply_outcome(self, outcome)?;
                Ok(vec![event])
            }
            ChainCommand::SubmitBlockCheckChallenge(challenge) => {
                let outcome = challenges::submit_block_check(self, challenge)?;
                let ChallengeOutcome::BlockCheckProvenInvalid {
                    block_hash,
                    receipt_id,
                    proposer,
                    challenger,
                    proposer_reward_clawback,
                    challenger_reward,
                    penalty_until_height,
                    reason,
                } = outcome
                else {
                    unreachable!("block check challenge returns block check outcome")
                };
                let challenge_id = challenges::block_check_challenge_id(&block_hash, &receipt_id);
                let mut events = vec![ChainEvent::BlockCheckChallengeProven {
                    block_hash,
                    receipt_id,
                    proposer,
                    challenger,
                    proposer_reward_clawback,
                    challenger_reward,
                    penalty_until_height,
                    reason,
                }];
                let claim_id = challenges::challenge_reward_claim_id(&challenge_id, &challenger);
                if let Some(reward) = self.state.pending_challenge_rewards.get(&claim_id) {
                    events.push(ChainEvent::ChallengeRewardPending {
                        claim_id,
                        challenge_id,
                        block_hash,
                        receipt_id,
                        challenger,
                        amount: reward.amount,
                        claimable_at_height: reward.claimable_at_height,
                    });
                }
                Ok(events)
            }
        }
    }

    fn view(&self) -> &ChainState {
        &self.state
    }

    fn params(&self) -> &ChainParams {
        &self.params
    }

    fn blocks(&self) -> &[TensorBlock] {
        &self.blocks
    }
}

pub(super) fn release_all_matured_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    prune_matured_voided_rewards(state)
}

fn claim_matured_rewards_for_beneficiary(
    state: &mut ChainState,
    beneficiary: Address,
) -> Vec<ChainEvent> {
    let mut events =
        release_matured_proposer_rewards_for_beneficiary(state, Some(beneficiary), false);
    events.extend(release_matured_receipt_rewards_with_policy(
        state,
        true,
        true,
        Some(beneficiary),
    ));
    events.extend(release_matured_challenge_rewards_for_beneficiary(
        state,
        Some(beneficiary),
        false,
    ));
    events.extend(release_matured_credit_rewards_for_beneficiary(
        state,
        Some(beneficiary),
    ));
    events
}

fn prune_matured_voided_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    events.extend(release_matured_proposer_rewards_for_beneficiary(
        state, None, true,
    ));
    events.extend(release_matured_challenge_rewards_for_beneficiary(
        state, None, true,
    ));
    events
}

fn release_matured_proposer_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    release_matured_proposer_rewards_for_beneficiary(state, None, false)
}

fn release_matured_proposer_rewards_for_beneficiary(
    state: &mut ChainState,
    beneficiary: Option<Address>,
    voided_only: bool,
) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_proposer_rewards
        .iter()
        .filter(|(_, reward)| {
            reward.claimable_at_height <= state.height
                && beneficiary.is_none_or(|address| reward.proposer == address)
                && (!voided_only || reward.voided_by_challenge)
        })
        .map(|(height, reward)| {
            (
                *height,
                reward.proposer,
                reward.amount,
                reward.voided_by_challenge,
            )
        })
        .collect::<Vec<_>>();
    for (block_height, proposer, amount, voided_by_challenge) in matured {
        state.pending_proposer_rewards.remove(&block_height);
        if voided_by_challenge {
            continue;
        }
        state.rewards.credit(proposer, amount);
        events.push(ChainEvent::ProposerRewardReleased {
            block_height,
            proposer,
            amount,
        });
        events.push(ChainEvent::RewardCredited {
            address: proposer,
            amount,
        });
    }
    events
}

fn release_matured_receipt_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    release_matured_receipt_rewards_with_policy(state, true, false, None)
}

fn release_matured_receipt_rewards_with_policy(
    state: &mut ChainState,
    prune_voided: bool,
    hold_unresolved_validator_audits: bool,
    beneficiary_filter: Option<Address>,
) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_receipt_rewards
        .iter()
        .filter(|(_, reward)| {
            let prunable_without_credit =
                receipt_reward_can_be_pruned_without_credit(state, reward);
            (reward.is_mature_at(state.height)
                || (prunable_without_credit && reward.hold_mature_at(state.height)))
                && state.included_receipts.contains(&reward.receipt_id)
                && (prunable_without_credit
                    || validator_receipt_reward_has_vrf_reveal(state, reward))
                && (prune_voided || !reward.voided_by_challenge)
                && !(hold_unresolved_validator_audits
                    && unresolved_validator_audit_blocks_reward_release(state, reward))
                && beneficiary_filter.is_none_or(|beneficiary| reward.beneficiary == beneficiary)
        })
        .map(|(claim_id, reward)| {
            (
                *claim_id,
                reward.receipt_id,
                reward.beneficiary,
                reward.amount,
                reward.kind,
                reward.voided_by_challenge,
            )
        })
        .collect::<Vec<_>>();
    for (claim_id, receipt_id, beneficiary, amount, kind, voided_by_challenge) in matured {
        state.pending_receipt_rewards.remove(&claim_id);
        let unavailable = state.data_unavailable_receipts.contains(&receipt_id);
        if kind == ReceiptRewardKind::Miner {
            release_pending_miner_tensor_work(
                state,
                receipt_id,
                beneficiary,
                !voided_by_challenge && !unavailable,
            );
        }
        if voided_by_challenge || unavailable {
            continue;
        }
        state.rewards.credit(beneficiary, amount);
        events.push(ChainEvent::ReceiptRewardReleased {
            claim_id,
            receipt_id,
            beneficiary,
            amount,
        });
        events.push(ChainEvent::RewardCredited {
            address: beneficiary,
            amount,
        });
    }
    events
}

fn receipt_reward_can_be_pruned_without_credit(
    state: &ChainState,
    reward: &PendingReceiptReward,
) -> bool {
    reward.voided_by_challenge || state.data_unavailable_receipts.contains(&reward.receipt_id)
}

fn validator_receipt_reward_has_vrf_reveal(
    state: &ChainState,
    reward: &PendingReceiptReward,
) -> bool {
    reward.kind != ReceiptRewardKind::Validator
        || state.validator_vrf_reveals.values().any(|reveal| {
            reveal.receipt_id == reward.receipt_id && reveal.validator == reward.beneficiary
        })
}

fn unresolved_validator_audit_blocks_reward_release(
    state: &ChainState,
    reward: &PendingReceiptReward,
) -> bool {
    reward.kind == ReceiptRewardKind::Validator
        && state
            .validator_audit_assignments
            .values()
            .any(|assignment| {
                assignment.receipt_id == reward.receipt_id
                    && assignment.validator == reward.beneficiary
                    && !state
                        .validator_audit_results
                        .contains_key(&assignment.audit_id)
                    && !state
                        .validator_audit_slashes
                        .contains_key(&assignment.audit_id)
            })
}

fn release_pending_miner_tensor_work(
    state: &mut ChainState,
    receipt_id: Hash,
    miner_address: Address,
    activate: bool,
) {
    let Some(receipt) = state.receipts.get(&receipt_id) else {
        return;
    };
    if receipt.miner() != miner_address {
        return;
    }
    let work = receipt.tensor_work_units();
    let Some(miner) = state.miners.get_mut(&miner_address) else {
        return;
    };
    miner.pending_tensor_work = miner.pending_tensor_work.saturating_sub(work);
    if activate {
        miner.settled_tensor_work = miner.settled_tensor_work.saturating_add(work);
    }
}

fn release_matured_challenge_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    release_matured_challenge_rewards_for_beneficiary(state, None, false)
}

fn release_matured_challenge_rewards_for_beneficiary(
    state: &mut ChainState,
    beneficiary: Option<Address>,
    voided_only: bool,
) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_challenge_rewards
        .iter()
        .filter(|(_, reward)| {
            reward.claimable_at_height <= state.height
                && beneficiary.is_none_or(|beneficiary| reward.challenger == beneficiary)
                && (!voided_only || reward.voided_by_challenge)
        })
        .map(|(claim_id, reward)| {
            (
                *claim_id,
                reward.challenge_id,
                reward.challenger,
                reward.amount,
                reward.voided_by_challenge,
            )
        })
        .collect::<Vec<_>>();
    for (claim_id, challenge_id, challenger, amount, voided_by_challenge) in matured {
        state.pending_challenge_rewards.remove(&claim_id);
        if voided_by_challenge {
            continue;
        }
        state.rewards.credit(challenger, amount);
        events.push(ChainEvent::ChallengeRewardReleased {
            claim_id,
            challenge_id,
            challenger,
            amount,
        });
        events.push(ChainEvent::RewardCredited {
            address: challenger,
            amount,
        });
    }
    events
}

fn release_matured_credit_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    release_matured_credit_rewards_for_beneficiary(state, None)
}

fn release_matured_credit_rewards_for_beneficiary(
    state: &mut ChainState,
    beneficiary: Option<Address>,
) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_credit_rewards
        .iter()
        .filter(|(_, reward)| {
            reward.claimable_at_height <= state.height
                && beneficiary.is_none_or(|beneficiary| reward.beneficiary == beneficiary)
        })
        .map(|(claim_id, reward)| (*claim_id, reward.beneficiary, reward.amount))
        .collect::<Vec<_>>();
    for (claim_id, beneficiary, amount) in matured {
        state.pending_credit_rewards.remove(&claim_id);
        state.rewards.credit(beneficiary, amount);
        events.push(ChainEvent::CreditRewardReleased {
            claim_id,
            beneficiary,
            amount,
        });
        events.push(ChainEvent::RewardCredited {
            address: beneficiary,
            amount,
        });
    }
    events
}

fn pending_credit_reward_claimable_height(chain: &Chain) -> u64 {
    chain
        .state
        .height
        .saturating_add(chain.params.reward_maturity_delay_blocks())
}

fn pending_credit_reward_claim_id(
    beneficiary: &Address,
    amount: u64,
    height: u64,
    sequence: u64,
) -> Hash {
    hash_bytes(
        b"tensor-vm-pending-credit-reward-claim-id-v1",
        &[
            beneficiary,
            &amount.to_le_bytes(),
            &height.to_le_bytes(),
            &sequence.to_le_bytes(),
        ],
    )
}
