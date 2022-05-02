use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, CanonicalAddr, Decimal, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

use athena::asset::AssetInfo;
use athena::vault_strategy::State;

const KEY_CONFIG: &[u8] = b"config";
const KEY_STRATEGY_STATE: &[u8] = b"state";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub contract_addr: CanonicalAddr,
    pub controller: CanonicalAddr,
    pub vault: CanonicalAddr,
    pub performance_fee: Decimal,
    pub stable_denom: String,
    pub anchor_market: CanonicalAddr,
    pub aterra_contract: CanonicalAddr,
    pub mirror_token: CanonicalAddr,
    pub mirror_staking: CanonicalAddr,
    pub mirror_mint: CanonicalAddr,
    pub mirror_oracle: CanonicalAddr,
    pub terraswap_factory: CanonicalAddr,
}

impl Config {
    pub fn get_underlying(&self) -> AssetInfo {
        AssetInfo::NativeToken {
            denom: self.stable_denom.to_string(),
        }
    }

    pub fn get_aterra_asset_info(&self, api: &dyn Api) -> StdResult<AssetInfo> {
        Ok(AssetInfo::Token {
            contract_addr: api.addr_humanize(&self.aterra_contract)?.to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MirrorAsset {
    pub swap_pair: CanonicalAddr,
    pub liquidity_token: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    singleton(storage, KEY_STRATEGY_STATE).save(state)
}

pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
    singleton_read(storage, KEY_STRATEGY_STATE).load()
}
