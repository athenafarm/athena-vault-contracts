use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{CanonicalAddr, Addr, Decimal, StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read};
use athena::vault::State;

const KEY_CONFIG: &[u8] = b"config";
const KEY_VAULT_STATE: &[u8] = b"state";
const PREFIX_KEY_DEPOSIT_INFO: &[u8] = b"deposit_info";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub controller: CanonicalAddr,
    pub stable_denom: String,
    pub strategy: Option<CanonicalAddr>,
    pub invest_percentage: Decimal,
    pub lock_period: u64,
    pub force_withdraw: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositInfo {
    pub principal: Uint128,
    pub current_amount: Uint128,
    pub share: Uint128,
    pub maturity: u64,
    pub yield_amount: Uint128,
    pub yield_claimed: Uint128,
    pub principal_claimed: Uint128,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    singleton(storage, KEY_VAULT_STATE).save(state)
}

pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
    singleton_read(storage, KEY_VAULT_STATE).load()
}

pub fn store_deposit_info(
    storage: &mut dyn Storage,
    addr: &Addr,
    deposit_info: &DepositInfo,
) -> StdResult<()> {
    bucket(storage, PREFIX_KEY_DEPOSIT_INFO).save(addr.as_bytes(), deposit_info)
}

pub fn read_deposit_info(storage: &dyn Storage, addr: &Addr) -> StdResult<DepositInfo> {
    bucket_read(storage, PREFIX_KEY_DEPOSIT_INFO).load(addr.as_bytes())
}
