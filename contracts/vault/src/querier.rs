use cosmwasm_std::{to_binary, Decimal, Deps, QueryRequest, StdResult, Uint128, WasmQuery};

use crate::state::{read_config, read_deposit_info, read_state, Config, DepositInfo};
use athena::vault_strategy::QueryMsg as StrategyQueryMsg;
use athena::vault::{ConfigResponse, DepositInfoResponse, State};
use athena::asset::AssetInfo;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;
    let strategy = if config.strategy.is_none() {
        None
    } else {
        Some(
            deps.api
                .addr_humanize(&config.strategy.unwrap())?
                .to_string(),
        )
    };
    let resp = ConfigResponse {
        contract_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
        controller: deps.api.addr_humanize(&config.controller)?.to_string(),
        stable_denom: config.stable_denom,
        strategy,
        invest_percentage: config.invest_percentage,
        lock_period: config.lock_period,
        force_withdraw: config.force_withdraw,
    };

    Ok(resp)
}

pub fn query_vault_balance(deps: Deps) -> StdResult<Uint128> {
    let config = read_config(deps.storage)?;
    Ok(get_vault_balance(deps, config)?)
}

pub fn get_vault_balance(deps: Deps, config: Config) -> StdResult<Uint128> {
    let asset = AssetInfo::NativeToken {
        denom: config.stable_denom,
    };
    
    asset.query_balance(
        &deps.querier,
        deps.api,
        deps.api.addr_humanize(&config.contract_addr)?,
    )
}

pub fn query_total_balance(deps: Deps) -> StdResult<Uint128> {
    let config = read_config(deps.storage)?;
    Ok(get_total_balance(deps, config)?)
}

pub fn get_total_balance(deps: Deps, config: Config) -> StdResult<Uint128> {
    if config.strategy.is_none() {
        Ok(get_vault_balance(deps, config.clone())?)
    } else {
        let invested_balance = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps
                .api
                .addr_humanize(&config.clone().strategy.unwrap())?
                .to_string(),
            msg: to_binary(&StrategyQueryMsg::TotalBalance {})?,
        }))?;
        Ok(get_vault_balance(deps, config.clone())?
            .checked_add(invested_balance)
            .unwrap())
    }
}

pub fn query_available(deps: Deps) -> StdResult<Uint128> {
    let config = read_config(deps.storage)?;
    get_available(deps, config)
}

pub fn get_available(deps: Deps, config: Config) -> StdResult<Uint128> {
    if config.clone().strategy.is_none() {
        Ok(Uint128::zero())
    } else {
        let vault_balance = get_vault_balance(deps, config.clone())?;
        let invested_balance = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps
                .api
                .addr_humanize(&config.clone().strategy.unwrap())?
                .to_string(),
            msg: to_binary(&StrategyQueryMsg::TotalBalance {})?,
        }))?;
        let max_available_balance =
            vault_balance.checked_add(invested_balance)? * config.invest_percentage;
        if max_available_balance > invested_balance {
            Ok(max_available_balance.checked_sub(invested_balance).unwrap())
        } else {
            Ok(Uint128::zero())
        }
    }
}

pub fn get_balance_by_share(
    deps: Deps,
    config: Config,
    state: State,
    share: Uint128,
) -> StdResult<Uint128> {
    let total_balance = get_total_balance(deps, config)?;

    if state.total_share.is_zero() || total_balance < state.total_subsidized {
        Ok(Uint128::zero())
    } else if share >= state.total_share {
        Ok(total_balance.checked_sub(state.total_subsidized)?)
    } else {
        Ok(total_balance.checked_sub(state.total_subsidized)?
            * Decimal::from_ratio(share, state.total_share))
    }
}

pub fn query_deposit_info(deps: Deps, addr: String) -> StdResult<DepositInfoResponse> {
    let deposit_info = match read_deposit_info(deps.storage, &deps.api.addr_validate(&addr)?) {
        Ok(info) => info,
        Err(_) => DepositInfo{
            principal: Uint128::zero(),
            current_amount: Uint128::zero(),
            share: Uint128::zero(),
            maturity: u64::MIN,
            yield_amount: Uint128::zero(),
            yield_claimed: Uint128::zero(),
            principal_claimed: Uint128::zero(),
        }
    };
    Ok(DepositInfoResponse {
        principal: deposit_info.principal,
        current_amount: deposit_info.current_amount,
        share: deposit_info.share,
        maturity: deposit_info.maturity,
        yield_amount: deposit_info.yield_amount,
        yield_claimed: deposit_info.yield_claimed,
        principal_claimed: deposit_info.principal_claimed,
    })
}

pub fn query_state(deps: Deps) -> StdResult<State> {
    let state = read_state(deps.storage)?;
    Ok(state)
}
