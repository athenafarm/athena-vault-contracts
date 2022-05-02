use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, WasmMsg,
};

use crate::anchor_helper::{deposit_to_anchor, redeem_from_anchor};
use crate::mirror_helper::{
    query_mirror_position, query_mirror_staking_rewards, query_pair_info, reserve_simulate_swap,
    simulate_swap,
};
use crate::state::{read_config, read_state, store_state};
use cw20::Cw20ExecuteMsg;
use athena::access_control::{assert_access_privilege, assert_sender_privilege};
use athena::asset::{Asset, AssetInfo};
use athena::querier::query_treasury;
use athena::vault_strategy::ExecuteMsg;
use mirror_protocol::mint::{
    Cw20HookMsg as MirrorMintCw20HookMsg, ExecuteMsg as MirrorMintExecuteMsg, ShortParams,
};
use mirror_protocol::staking::ExecuteMsg as MirrorStakingExecuteMsg;
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, ExecuteMsg as TerraPairExecuteMsg};

/// Deposit UST to anchor money market
pub fn deposit_anchor(deps: DepsMut, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    assert_access_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        info.sender,
        true,
        false,
    )?;

    if amount.is_zero() {
        return Err(StdError::generic_err("Amount must be greater than zero"));
    }
    let mut state = read_state(deps.storage)?;
    state.anchor_deposited = state.anchor_deposited + amount;
    store_state(deps.storage, &state)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    deposit_to_anchor(deps.as_ref(), config.clone(), amount, &mut messages)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "deposit_anchor"),
        attr("amount", amount),
    ]))
}

/// Withdraw UST from anchor money market
pub fn withdraw_anchor(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    aterra_amount: Option<Uint128>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if info.sender != env.contract.address {
        assert_access_privilege(
            &deps.querier,
            deps.api.addr_humanize(&config.controller)?,
            info.sender,
            true,
            false,
        )?;
    }

    redeem_from_anchor(deps, env, config.clone(), aterra_amount)
}

/// Swap denom to mirror token and provide liquidity,
/// and execute hook to invest to mirror protocol
pub fn deposit_mirror(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    mirror_asset_addr: String,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    assert_access_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        info.sender,
        true,
        false,
    )?;

    if amount.is_zero() {
        return Err(StdError::generic_err("Amount must be greater than zero"));
    }

    let mirror_asset = AssetInfo::Token {
        contract_addr: mirror_asset_addr.clone(),
    };

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        mirror_asset,
    )?;

    let half_amount: Uint128 = amount * Decimal::percent(50);

    let offer_asset = Asset {
        amount: half_amount,
        info: config.get_underlying(),
    };

    let offer_asset_tax_deducted = Asset {
        amount: offer_asset.deduct_tax(&deps.querier)?.amount,
        info: config.get_underlying(),
    };

    Ok(Response::new()
        .add_messages(vec![
            offer_asset.clone().into_msg_with_data(
                &deps.querier,
                pair_info.contract_addr,
                to_binary(&TerraPairExecuteMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                    offer_asset: offer_asset_tax_deducted.into(),
                })?,
            )?,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::DepositMirrorHook { mirror_asset_addr })?,
            }),
        ])
        .add_attributes(vec![
            attr("action", "deposit_mirror"),
            attr("amount", amount),
        ]))
}

/// Only contract itself can execute
/// Stake available mirror liquidities to mirror staking pool
pub fn deposit_mirror_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mirror_asset_addr: String,
) -> StdResult<Response> {
    assert_sender_privilege(
        info.sender.to_string(),
        env.clone().contract.address.to_string(),
    )?;

    let config = read_config(deps.storage)?;

    let mirror_asset_info = AssetInfo::Token {
        contract_addr: mirror_asset_addr.to_string(),
    };

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        mirror_asset_info.clone(),
    )?;

    let pool_stable_balance = config.get_underlying().query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(pair_info.contract_addr.to_string()),
    )?;
    let pool_mirror_balance = mirror_asset_info.query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(pair_info.contract_addr.to_string()),
    )?;

    let mirror_token_amount =
        mirror_asset_info.query_balance(&deps.querier, deps.api, env.contract.address)?;

    let stable_amount_for_lp =
        mirror_token_amount * Decimal::from_ratio(pool_stable_balance, pool_mirror_balance);

    let underlying_asset = Asset {
        amount: stable_amount_for_lp,
        info: config.get_underlying(),
    };

    let tax_deducted = underlying_asset.deduct_tax(&deps.querier)?.amount;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: mirror_asset_addr.clone(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
                    amount: mirror_token_amount,
                    expires: None,
                })?,
            }),
            underlying_asset.into_msg_with_data(
                &deps.querier,
                deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
                to_binary(&MirrorStakingExecuteMsg::AutoStake {
                    assets: [
                        Asset {
                            info: config.get_underlying(),
                            amount: tax_deducted,
                        }
                        .into(),
                        Asset {
                            info: mirror_asset_info,
                            amount: mirror_token_amount,
                        }
                        .into(),
                    ],
                    slippage_tolerance: None,
                })?,
            )?,
        ])
        .add_attributes(vec![
            attr("denom_amount", tax_deducted),
            attr("mirror_token_amount", mirror_token_amount),
        ]))
}

/// Unbond mirror LP tokens from mirror staking
/// and swap to UST
pub fn withdraw_mirror(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mirror_lp_amount: Uint128,
    mirror_asset_addr: String,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if info.sender != env.contract.address {
        assert_access_privilege(
            &deps.querier,
            deps.api.addr_humanize(&config.controller)?,
            info.sender,
            true,
            false,
        )?;
    }

    let mirror_asset = AssetInfo::Token {
        contract_addr: mirror_asset_addr.to_string(),
    };

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        mirror_asset,
    )?;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
                funds: vec![],
                msg: to_binary(&MirrorStakingExecuteMsg::Unbond {
                    asset_token: mirror_asset_addr.clone(),
                    amount: mirror_lp_amount,
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_info.liquidity_token,
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    amount: mirror_lp_amount,
                    contract: pair_info.contract_addr,
                    msg: to_binary(&TerraswapCw20HookMsg::WithdrawLiquidity {})?,
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::WithdrawMirrorHook { mirror_asset_addr })?,
            }),
        ])
        .add_attributes(vec![
            attr("action", "withdraw_mirror"),
            attr("amount", mirror_lp_amount),
        ]))
}

pub fn open_short_position(
    deps: DepsMut,
    info: MessageInfo,
    aterra_amount: Uint128,
    collateral_ratio: Decimal,
    mirror_asset_addr: String,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    assert_access_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        info.sender,
        true,
        false,
    )?;

    let aterra_asset_info = AssetInfo::Token {
        contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
    };

    let aterra_asset = Asset {
        info: aterra_asset_info,
        amount: aterra_amount,
    };

    let mirror_asset_info = AssetInfo::Token {
        contract_addr: mirror_asset_addr,
    };

    let mut state = read_state(deps.storage)?;
    state.aterra_collateral = state.aterra_collateral + aterra_amount;
    store_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(vec![aterra_asset.into_msg_with_data(
            &deps.querier,
            deps.api.addr_humanize(&config.mirror_mint)?.to_string(),
            to_binary(&MirrorMintCw20HookMsg::OpenPosition {
                asset_info: mirror_asset_info.into(),
                collateral_ratio,
                short_params: Some(ShortParams {
                    belief_price,
                    max_spread,
                }),
            })?,
        )?])
        .add_attributes(vec![
            attr("action", "open_short_position"),
            attr("aterra_amount", aterra_amount),
            attr("collateral_ratio", collateral_ratio.to_string()),
        ]))
}

pub fn close_short_position(
    deps: DepsMut,
    info: MessageInfo,
    position_idx: Uint128,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    assert_access_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        info.sender,
        true,
        false,
    )?;

    let position = query_mirror_position(deps.as_ref(), config.clone(), position_idx)?;

    if position.is_short == false {
        return Err(StdError::generic_err("Not short position"));
    }

    if position.collateral.amount.is_zero() {
        return Err(StdError::generic_err("Already closed"));
    }

    let burn_asset: Asset = position.asset.into();

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        burn_asset.clone().info,
    )?;

    let reserve_simulation = reserve_simulate_swap(
        deps.as_ref(),
        burn_asset.clone(),
        pair_info.clone().contract_addr,
    )?;

    let offer_asset = Asset {
        info: config.get_underlying(),
        amount: reserve_simulation.offer_amount,
    };

    let mut state = read_state(deps.storage)?;
    state.aterra_collateral = state
        .aterra_collateral
        .checked_sub(position.collateral.amount)?;
    store_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(vec![
            offer_asset.into_msg_with_data(
                &deps.querier,
                pair_info.contract_addr,
                to_binary(&TerraswapCw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })?,
            )?,
            burn_asset.into_msg_with_data(
                &deps.querier,
                deps.api.addr_humanize(&config.mirror_mint)?.to_string(),
                to_binary(&MirrorMintCw20HookMsg::Burn { position_idx })?,
            )?,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.mirror_mint)?.to_string(),
                funds: vec![],
                msg: to_binary(&MirrorMintExecuteMsg::Withdraw {
                    position_idx,
                    collateral: None,
                })?,
            }),
        ])
        .add_attributes(vec![
            attr("action", "close_short_position"),
            attr("position_idx", position_idx),
        ]))
}

/// Only contract itself can execute
/// Swap LP to UST
pub fn withdraw_mirror_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mirror_asset_addr: String,
) -> StdResult<Response> {
    assert_sender_privilege(info.sender.to_string(), env.contract.address.to_string())?;

    let config = read_config(deps.storage)?;

    let mirror_asset_info = AssetInfo::Token {
        contract_addr: mirror_asset_addr,
    };

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        mirror_asset_info.clone(),
    )?;

    let mirror_asset_balance = mirror_asset_info.query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(env.contract.address.to_string()),
    )?;

    let mirror_asset = Asset {
        info: mirror_asset_info,
        amount: mirror_asset_balance,
    };

    if mirror_asset_balance.is_zero() {
        Ok(Response::new())
    } else {
        Ok(Response::new().add_message(mirror_asset.into_msg_with_data(
            &deps.querier,
            pair_info.contract_addr.clone(),
            to_binary(&TerraswapCw20HookMsg::Swap {
                belief_price: None,
                max_spread: None,
                to: None,
            })?,
        )?))
    }
}

/// Withdraw invested UST from anchor and mirror protocol
pub fn withdraw_all(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if info.sender != env.contract.address
        && info.sender != deps.api.addr_humanize(&config.clone().vault)?
    {
        assert_access_privilege(
            &deps.querier,
            deps.api.addr_humanize(&config.controller)?,
            info.sender,
            true,
            false,
        )?;
    }

    let reward_infos = query_mirror_staking_rewards(deps.as_ref(), config.clone())?;

    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::WithdrawAnchor {
            aterra_amount: None,
        })?,
    }));

    for reward_info in reward_infos {
        if !reward_info.bond_amount.is_zero() && !reward_info.is_short {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::WithdrawMirror {
                    mirror_lp_amount: reward_info.bond_amount,
                    mirror_asset_addr: reward_info.asset_token,
                })?,
            }));
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::CompoundMirror {})?,
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw_all"))
}

/// Withdraw MIR reward from mirror staking and swap to UST
pub fn compound_mirror(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if info.sender != env.contract.address {
        assert_access_privilege(
            &deps.querier,
            deps.api.addr_humanize(&config.controller)?,
            info.sender,
            true,
            false,
        )?;
    }

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.mirror_staking)?.to_string(),
                funds: vec![],
                msg: to_binary(&MirrorStakingExecuteMsg::Withdraw { asset_token: None })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::CompoundMirrorHook {})?,
            }),
        ])
        .add_attribute("action", "compound_mirror"))
}

/// Only contract itself can execute
/// Swap MIR to UST
pub fn compound_mirror_hook(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    assert_sender_privilege(info.sender.to_string(), env.contract.address.to_string())?;

    let config = read_config(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut logs: Vec<Attribute> = vec![];

    let mirror_token_info = AssetInfo::Token {
        contract_addr: deps.api.addr_humanize(&config.mirror_token)?.to_string(),
    };

    let pair_info = query_pair_info(
        deps.as_ref(),
        deps.api
            .addr_humanize(&config.terraswap_factory)?
            .to_string(),
        config.get_underlying(),
        mirror_token_info.clone(),
    )?;

    let mirror_token_balance = mirror_token_info.query_balance(
        &deps.querier,
        deps.api,
        Addr::unchecked(env.contract.address.to_string()),
    )?;

    let mirror_token_asset = Asset {
        info: mirror_token_info,
        amount: mirror_token_balance,
    };

    if !mirror_token_balance.is_zero() {
        messages.push(mirror_token_asset.clone().into_msg_with_data(
            &deps.querier,
            pair_info.contract_addr.clone(),
            to_binary(&TerraswapCw20HookMsg::Swap {
                belief_price: None,
                max_spread: None,
                to: None,
            })?,
        )?);

        let profit_amount = simulate_swap(
            deps.as_ref(),
            mirror_token_asset.clone(),
            pair_info.contract_addr,
        )?;

        let performance_fee = profit_amount * config.performance_fee;

        if !performance_fee.is_zero() {
            let fee_asset = Asset {
                amount: performance_fee,
                info: config.get_underlying(),
            };

            messages.push(
                fee_asset.into_msg(
                    &deps.querier,
                    Addr::unchecked(
                        query_treasury(&deps.querier, deps.api.addr_humanize(&config.controller)?)?
                            .to_string(),
                    ),
                )?,
            );
            logs.push(attr("performance_fee", performance_fee));
        } else {
            logs.push(attr("performance_fee", '0'));
        }
    }

    Ok(Response::new().add_messages(messages).add_attributes(logs))
}
