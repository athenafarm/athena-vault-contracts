#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use crate::claim::{claim_principal, claim_yield};
use crate::deposit::{deposit_native, withdraw};
use crate::manage::{invest, update_config};
use crate::querier::{
    query_available, query_config, query_deposit_info, query_state,
    query_total_balance, query_vault_balance,
};
use crate::state::{store_config, store_state, Config};
use athena::vault::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, State};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    if msg.invest_percentage > Decimal::one() {
        return Err(StdError::generic_err(
            "Invest percentage must be less than 100%",
        ));
    }

    store_config(
        deps.storage,
        &Config {
            contract_addr: deps.api.addr_canonicalize(&env.contract.address.as_str())?,
            controller: deps.api.addr_canonicalize(&msg.controller)?,
            strategy: None,
            stable_denom: msg.stable_denom,
            invest_percentage: msg.invest_percentage,
            lock_period: msg.lock_period,
            force_withdraw: false,
        },
    )?;

    store_state(
        deps.storage,
        &State {
            total_share: Uint128::zero(),
            total_subsidized: Uint128::zero(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            controller,
            strategy,
            invest_percentage,
            lock_period,
            force_withdraw,
        } => update_config(
            deps,
            info,
            controller,
            strategy,
            invest_percentage,
            lock_period,
            force_withdraw,
        ),
        ExecuteMsg::Deposit {} => deposit_native(deps, env, info),
        ExecuteMsg::Withdraw {
            withdraw_amount,
            force_withdraw,
        } => withdraw(deps, env, info, withdraw_amount, force_withdraw),
        ExecuteMsg::ClaimYield {} => claim_yield(deps, env, info),
        ExecuteMsg::ClaimPrincipal {} => claim_principal(deps, env, info),
        ExecuteMsg::Invest {} => invest(deps.as_ref(), info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::DepositInfo { addr } => to_binary(&query_deposit_info(deps, addr)?),
        QueryMsg::VaultBalance {} => to_binary(&query_vault_balance(deps)?),
        QueryMsg::TotalBalance {} => to_binary(&query_total_balance(deps)?),
        QueryMsg::Available {} => to_binary(&query_available(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
