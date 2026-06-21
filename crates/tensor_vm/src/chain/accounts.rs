use super::{AccountState, Chain};
use crate::error::{Result, TvmError};
use crate::types::Address;

pub fn ensure(chain: &mut Chain, address: Address) -> &mut AccountState {
    chain.state.accounts.entry(address).or_insert(AccountState {
        address,
        balance: 0,
        nonce: 0,
    })
}

pub fn credit(chain: &mut Chain, address: Address, amount: u64) {
    let account = ensure(chain, address);
    account.balance = account.balance.saturating_add(amount);
}

pub fn transfer(chain: &mut Chain, from: Address, to: Address, amount: u64) -> Result<()> {
    let from_account = ensure(chain, from);
    if from_account.balance < amount {
        return Err(TvmError::InvalidReceipt("insufficient account balance"));
    }
    from_account.balance -= amount;
    from_account.nonce += 1;
    let to_account = ensure(chain, to);
    to_account.balance = to_account.balance.saturating_add(amount);
    Ok(())
}

pub fn claim_reward(chain: &mut Chain, address: Address) -> Result<u64> {
    let reward = chain.state.rewards.balance(&address);
    if reward == 0 {
        return Err(TvmError::InvalidReceipt("no reward to claim"));
    }
    credit(chain, address, reward);
    chain.state.rewards.clear_balance(address);
    Ok(reward)
}
