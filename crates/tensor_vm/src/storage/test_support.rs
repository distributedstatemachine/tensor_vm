use crate::chain::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobState, ReceiptState, TensorBlock,
};
use crate::types::{Address, Hash};
use crate::verify::ValidatorAttestation;

pub(super) fn register_block_producer(chain: &mut Chain, producer: Address) {
    chain
        .apply_command(ChainCommand::RegisterMiner {
            address: producer,
            stake: chain.params().miner_min_stake,
        })
        .unwrap();
    chain
        .apply_command(ChainCommand::RegisterValidator {
            address: producer,
            stake: chain.params().validator_min_stake,
        })
        .unwrap();
}

pub(super) fn register_validator(chain: &mut Chain, validator: Address) {
    chain
        .apply_command(ChainCommand::RegisterValidator {
            address: validator,
            stake: chain.params().validator_min_stake,
        })
        .unwrap();
}

pub(super) fn transfer(chain: &mut Chain, from: Address, to: Address, amount: u64) {
    chain
        .apply_command(ChainCommand::Transfer { from, to, amount })
        .unwrap();
}

pub(super) fn submit_job(chain: &mut Chain, job: JobState) {
    chain.apply_command(ChainCommand::SubmitJob(job)).unwrap();
}

pub(super) fn submit_receipt(chain: &mut Chain, receipt: ReceiptState) {
    chain
        .apply_command(ChainCommand::SubmitReceipt(receipt))
        .unwrap();
}

pub(super) fn submit_attestation(chain: &mut Chain, attestation: ValidatorAttestation) {
    chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .unwrap();
}

pub(super) fn register_model(
    chain: &mut Chain,
    model_id: Hash,
    architecture_hash: Hash,
    weight_root: Hash,
    config_hash: Hash,
) {
    chain
        .apply_command(ChainCommand::RegisterModel {
            model_id,
            architecture_hash,
            weight_root,
            config_hash,
        })
        .unwrap();
}

pub(super) fn credit_reward(chain: &mut Chain, address: Address, amount: u64) {
    chain
        .apply_command(ChainCommand::CreditReward { address, amount })
        .unwrap();
}

pub(super) fn produce_block(chain: &mut Chain, proposer: Address, timestamp: u64) -> TensorBlock {
    let block_count = chain.blocks().len();
    let proposer = chain
        .proposer_for_next_epoch(&chain.state().finalized_randomness())
        .unwrap_or(proposer);
    let timestamp = chain
        .blocks()
        .last()
        .map(|parent| {
            timestamp.max(
                parent.timestamp.saturating_add(
                    chain
                        .params()
                        .pow_timeout_blocks
                        .max(1)
                        .saturating_mul(chain.params().block_time_seconds.max(1)),
                ),
            )
        })
        .unwrap_or(timestamp);
    chain
        .apply_command(ChainCommand::ProduceBlock {
            proposer,
            timestamp,
        })
        .unwrap();
    assert_eq!(chain.blocks().len(), block_count + 1);
    chain.blocks().last().unwrap().clone()
}

pub(super) fn submit_block_vote(chain: &mut Chain, vote: BlockVote) {
    chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .unwrap();
}
