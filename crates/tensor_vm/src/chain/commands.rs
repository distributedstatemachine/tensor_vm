use super::{
    BlockAdmission, Chain, ChainCommand, ChainEngine, ChainEvent, ChainParams, ChainState,
    PendingCreditReward, ReceiptState, TensorBlock, accounts, challenges, receipts, settlement,
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
                let amount = self.state.rewards.balance(&address);
                accounts::claim_reward(self, address)?;
                Ok(vec![ChainEvent::RewardClaimed { address, amount }])
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
            ChainCommand::SubmitBlock(block) => match self.admit_block(block)? {
                BlockAdmission::Applied { height, hash } => {
                    Ok(vec![ChainEvent::BlockAccepted { height, hash }])
                }
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
    let mut events = release_matured_proposer_rewards(state);
    events.extend(release_matured_receipt_rewards(state));
    events.extend(release_matured_challenge_rewards(state));
    events.extend(release_matured_credit_rewards(state));
    events
}

fn release_matured_proposer_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_proposer_rewards
        .iter()
        .filter(|(_, reward)| reward.claimable_at_height <= state.height)
        .filter(|(_, reward)| reward.voided_by_challenge || !reward.requires_useful_successor)
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
    let mut events = Vec::new();
    let matured = state
        .pending_receipt_rewards
        .iter()
        .filter(|(_, reward)| reward.claimable_at_height <= state.height)
        .map(|(claim_id, reward)| {
            (
                *claim_id,
                reward.receipt_id,
                reward.beneficiary,
                reward.amount,
                reward.voided_by_challenge,
            )
        })
        .collect::<Vec<_>>();
    for (claim_id, receipt_id, beneficiary, amount, voided_by_challenge) in matured {
        state.pending_receipt_rewards.remove(&claim_id);
        if voided_by_challenge || state.data_unavailable_receipts.contains(&receipt_id) {
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

fn release_matured_challenge_rewards(state: &mut ChainState) -> Vec<ChainEvent> {
    let mut events = Vec::new();
    let matured = state
        .pending_challenge_rewards
        .iter()
        .filter(|(_, reward)| reward.claimable_at_height <= state.height)
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
    let mut events = Vec::new();
    let matured = state
        .pending_credit_rewards
        .iter()
        .filter(|(_, reward)| reward.claimable_at_height <= state.height)
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
