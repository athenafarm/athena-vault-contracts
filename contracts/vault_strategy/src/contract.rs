#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use crate::anchor_helper::query_anchor_market_config;
use crate::invest::{
    close_short_position, compound_mirror, compound_mirror_hook, deposit_anchor, deposit_mirror,
    deposit_mirror_hook, open_short_position, withdraw_all, withdraw_anchor, withdraw_mirror,
    withdraw_mirror_hook,
};
use crate::manage::{update_config, withdraw_to_vault};
use crate::querier::{query_config, query_state, query_total_balance};
use crate::state::{store_config, store_state, Config};
use athena::vault_strategy::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, State};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    if msg.performance_fee > Decimal::one() {
        return Err(StdError::generic_err(
            "Performance fee percentage must be less than 100%",
        ));
    }

    let anchor_market_config = query_anchor_market_config(deps.as_ref(), &msg.anchor_market);

    store_config(
        deps.storage,
        &Config {
            contract_addr: deps.api.addr_canonicalize(&env.contract.address.as_str())?,
            controller: deps.api.addr_canonicalize(&msg.controller)?,
            vault: deps.api.addr_canonicalize(&msg.vault)?,
            performance_fee: msg.performance_fee,
            stable_denom: msg.stable_denom,
            anchor_market: deps.api.addr_canonicalize(&msg.anchor_market)?,
            aterra_contract: deps
                .api
                .addr_canonicalize(&anchor_market_config?.aterra_contract)?,
            mirror_token: deps.api.addr_canonicalize(&msg.mirror_token)?,
            mirror_staking: deps.api.addr_canonicalize(&msg.mirror_staking)?,
            mirror_mint: deps.api.addr_canonicalize(&msg.mirror_mint)?,
            mirror_oracle: deps.api.addr_canonicalize(&msg.mirror_oracle)?,
            terraswap_factory: deps.api.addr_canonicalize(&msg.terraswap_factory)?,
        },
    )?;

    store_state(
        deps.storage,
        &State {
            anchor_deposited: Uint128::zero(),
            aterra_collateral: Uint128::zero(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            controller,
            performance_fee,
        } => update_config(deps, info, controller, performance_fee),
        ExecuteMsg::DepositAnchor { amount } => deposit_anchor(deps, info, amount),
        ExecuteMsg::WithdrawAnchor { aterra_amount } => {
            withdraw_anchor(deps, env, info, aterra_amount)
        }
        ExecuteMsg::DepositMirror {
            amount,
            mirror_asset_addr,
        } => deposit_mirror(deps, env, info, amount, mirror_asset_addr),
        ExecuteMsg::DepositMirrorHook { mirror_asset_addr } => {
            deposit_mirror_hook(deps, env, info, mirror_asset_addr)
        }
        ExecuteMsg::WithdrawMirror {
            mirror_lp_amount,
            mirror_asset_addr,
        } => withdraw_mirror(deps, env, info, mirror_lp_amount, mirror_asset_addr),
        ExecuteMsg::WithdrawMirrorHook { mirror_asset_addr } => {
            withdraw_mirror_hook(deps, env, info, mirror_asset_addr)
        }
        ExecuteMsg::CompoundMirror {} => compound_mirror(deps, env, info),
        ExecuteMsg::CompoundMirrorHook {} => compound_mirror_hook(deps, env, info),
        ExecuteMsg::OpenShortPosition {
            aterra_amount,
            collateral_ratio,
            mirror_asset_addr,
            belief_price,
            max_spread,
        } => open_short_position(
            deps,
            info,
            aterra_amount,
            collateral_ratio,
            mirror_asset_addr,
            belief_price,
            max_spread,
        ),
        ExecuteMsg::CloseShortPosition { position_idx } => {
            close_short_position(deps, info, position_idx)
        }
        ExecuteMsg::WithdrawAll {} => withdraw_all(deps, env, info),
        ExecuteMsg::WithdrawToVault { amount } => withdraw_to_vault(deps, env, info, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::TotalBalance {} => to_binary(&query_total_balance(deps, env)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
