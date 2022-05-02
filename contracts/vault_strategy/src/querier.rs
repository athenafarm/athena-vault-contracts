use cosmwasm_std::{Deps, Env, StdResult, Uint128};

use crate::anchor_helper::get_anchor_balance;
use crate::mirror_helper::get_mirror_balance;
use crate::state::{read_config, read_state, Config};
use athena::vault_strategy::{ConfigResponse, State};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;

    let resp = ConfigResponse {
        contract_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
        controller: deps.api.addr_humanize(&config.controller)?.to_string(),
        vault: deps.api.addr_humanize(&config.vault)?.to_string(),
        performance_fee: config.performance_fee,
        stable_denom: config.stable_denom,
        anchor_market: deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        aterra_contract: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
        mirror_token: deps.api.addr_humanize(&config.mirror_token)?.to_string(),
        mirror_staking: deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
        mirror_mint: deps.api.addr_humanize(&config.mirror_mint)?.to_string(),
        mirror_oracle: deps.api.addr_humanize(&config.mirror_oracle)?.to_string(),
        terraswap_factory: deps
            .api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
    };

    Ok(resp)
}

pub fn query_state(deps: Deps) -> StdResult<State> {
    let state = read_state(deps.storage)?;

    Ok(state)
}

pub fn query_total_balance(deps: Deps, env: Env) -> StdResult<Uint128> {
    let config = read_config(deps.storage)?;
    Ok(get_total_balance(deps, env, config)?)
}

pub fn get_total_balance(deps: Deps, env: Env, config: Config) -> StdResult<Uint128> {
    Ok(config.get_underlying().query_balance(
        &deps.querier,
        deps.api,
        env.contract.address.clone(),
    )? + get_anchor_balance(
        deps,
        config.clone(),
        env.contract.address.clone(),
        env.block.height,
    )? + get_mirror_balance(deps, config.clone())?)
}
