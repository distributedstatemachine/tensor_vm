use super::{Chain, HardwareClass, MinerState, ValidatorState, accounts};
use crate::error::{Result, TvmError};
use crate::types::{Address, Hash};

pub fn register_miner(chain: &mut Chain, address: Address, stake: u64) -> Result<()> {
    register_miner_with_profile_and_operator(chain, address, stake, address, HardwareClass::Cpu, 0)
}

pub fn register_miner_with_operator(
    chain: &mut Chain,
    address: Address,
    stake: u64,
    operator_id: Hash,
) -> Result<()> {
    register_miner_with_profile_and_operator(
        chain,
        address,
        stake,
        operator_id,
        HardwareClass::Cpu,
        0,
    )
}

pub fn register_miner_with_profile(
    chain: &mut Chain,
    address: Address,
    stake: u64,
    hardware_class: HardwareClass,
    gpu_utilization_bps: u64,
) -> Result<()> {
    register_miner_with_profile_and_operator(
        chain,
        address,
        stake,
        address,
        hardware_class,
        gpu_utilization_bps,
    )
}

pub fn register_miner_with_profile_and_operator(
    chain: &mut Chain,
    address: Address,
    stake: u64,
    operator_id: Hash,
    hardware_class: HardwareClass,
    gpu_utilization_bps: u64,
) -> Result<()> {
    if stake < chain.params.miner_min_stake {
        return Err(TvmError::InsufficientStake);
    }
    if gpu_utilization_bps > 10_000 {
        return Err(TvmError::InvalidReceipt("gpu utilization exceeds 100%"));
    }
    if !hardware_class.is_gpu() && gpu_utilization_bps != 0 {
        return Err(TvmError::InvalidReceipt(
            "non-gpu miner cannot report gpu utilization",
        ));
    }
    if chain.state.miners.contains_key(&address) {
        return Err(TvmError::InvalidReceipt("miner already registered"));
    }
    accounts::ensure(chain, address);
    chain.state.miners.insert(
        address,
        MinerState {
            address,
            operator_id,
            stake,
            reputation: 0,
            settled_tensor_work: 0,
            pending_tensor_work: 0,
            hardware_class,
            gpu_utilization_bps,
        },
    );
    Ok(())
}

pub fn register_validator(chain: &mut Chain, address: Address, stake: u64) -> Result<()> {
    if stake < chain.params.validator_min_stake {
        return Err(TvmError::InsufficientStake);
    }
    if chain.state.validators.contains_key(&address) {
        return Err(TvmError::InvalidReceipt("validator already registered"));
    }
    accounts::ensure(chain, address);
    chain.state.validators.insert(
        address,
        ValidatorState {
            address,
            stake,
            reputation: 0,
            valid_attestations: 0,
            missed_assignments: 0,
            vrf_public_key: None,
        },
    );
    Ok(())
}

pub fn register_validator_vrf_key(
    chain: &mut Chain,
    validator: Address,
    vrf_public_key: Hash,
) -> Result<()> {
    let Some(state) = chain.state.validators.get_mut(&validator) else {
        return Err(TvmError::UnknownValidator);
    };
    match state.vrf_public_key {
        Some(existing) if existing == vrf_public_key => Ok(()),
        Some(_) => Err(TvmError::InvalidReceipt(
            "validator vrf public key already registered",
        )),
        None => {
            state.vrf_public_key = Some(vrf_public_key);
            Ok(())
        }
    }
}
