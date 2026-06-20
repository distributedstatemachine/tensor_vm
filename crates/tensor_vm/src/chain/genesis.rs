use super::state::{Chain, ChainParams, ChainState, RewardState};
use crate::types::Hash;
use std::collections::{BTreeMap, BTreeSet};

pub fn with_params(params: ChainParams, finalized_randomness: Hash) -> Chain {
    Chain {
        params,
        state: ChainState {
            height: 0,
            epoch: 0,
            finalized_beacon_round: 0,
            finalized_randomness,
            genesis_beacon_round: 0,
            genesis_randomness: finalized_randomness,
            accounts: BTreeMap::new(),
            miners: BTreeMap::new(),
            validators: BTreeMap::new(),
            jobs: BTreeMap::new(),
            program_bodies: BTreeMap::new(),
            receipts: BTreeMap::new(),
            receipt_randomness_anchors: BTreeMap::new(),
            attestations: BTreeMap::new(),
            block_votes: BTreeMap::new(),
            finalized_blocks: BTreeSet::new(),
            data_unavailable_receipts: BTreeSet::new(),
            data_unavailability_slashes: BTreeMap::new(),
            validator_audit_assignments: BTreeMap::new(),
            validator_audit_results: BTreeMap::new(),
            validator_audit_slashes: BTreeMap::new(),
            validator_audit_appeals: BTreeMap::new(),
            settled_receipts: BTreeSet::new(),
            included_receipts: BTreeSet::new(),
            block_selected_receipts: BTreeMap::new(),
            block_check_challenges: BTreeMap::new(),
            challenged_receipts: BTreeSet::new(),
            proposer_penalty_until: BTreeMap::new(),
            pending_proposer_rewards: BTreeMap::new(),
            pending_receipt_rewards: BTreeMap::new(),
            pending_challenge_rewards: BTreeMap::new(),
            pending_credit_rewards: BTreeMap::new(),
            model_states: BTreeMap::new(),
            rewards: RewardState::default(),
        },
        blocks: Vec::new(),
    }
}
