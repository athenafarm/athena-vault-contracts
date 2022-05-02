use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, CosmosMsg, Decimal, Deps, DepsMut, Env, QueryRequest,
    Response, StdError, StdResult, Uint128, WasmQuery,
};

use crate::state::{read_state, store_state, Config};
use anchor_market::market::{
    ConfigResponse as AnchorMarketConfigResponse,
    EpochStateResponse as AnchorMarketEpochStateResponse, QueryMsg as AnchorMarketQueryMsg,
};
use anchor_market::market::{
    Cw20HookMsg as AnchorMarketCw20HookMsg, ExecuteMsg as AnchorExecuteMsg,
};
use athena::asset::{Asset, AssetInfo};
use athena::querier::{query_token_balance, query_treasury};

pub fn query_anchor_market_config(
    deps: Deps,
    anchor_market_address: &String,
) -> StdResult<AnchorMarketConfigResponse> {
    let anchor_market_config_response: AnchorMarketConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: String::from(anchor_market_address),
            msg: to_binary(&AnchorMarketQueryMsg::Config {})?,
        }))?;

    Ok(anchor_market_config_response)
}

fn query_anchor_exchange_rate(
    deps: Deps,
    block_height: Option<u64>,
    anchor_market: &String,
) -> StdResult<Decimal> {
    let anchor_market_epoch_state_response: AnchorMarketEpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: String::from(anchor_market),
            msg: to_binary(&AnchorMarketQueryMsg::EpochState {
                block_height: block_height,
                distributed_interest: None,
            })?,
        }))?;

    Ok(anchor_market_epoch_state_response.exchange_rate.into())
}

pub fn get_anchor_balance(
    deps: Deps,
    config: Config,
    addr: Addr,
    block_height: u64,
) -> StdResult<Uint128> {
    let aterra_balance = query_token_balance(
        &deps.querier,
        deps.api.addr_humanize(&config.aterra_contract)?,
        addr,
    )?;

    if aterra_balance.is_zero() {
        Ok(Uint128::zero())
    } else {
        let anchor_exchange_rate = query_anchor_exchange_rate(
            deps,
            Some(block_height),
            &deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        );
        Ok(aterra_balance * anchor_exchange_rate?)
    }
}

pub fn deposit_to_anchor(
    deps: Deps,
    config: Config,
    amount: Uint128,
    messages: &mut Vec<CosmosMsg>,
) -> StdResult<()> {
    let asset = Asset {
        amount: amount,
        info: config.get_underlying(),
    };

    messages.push(asset.into_msg_with_data(
        &deps.querier,
        deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        to_binary(&AnchorExecuteMsg::DepositStable {})?,
    )?);

    Ok(())
}

pub fn redeem_from_anchor(
    deps: DepsMut,
    env: Env,
    config: Config,
    aterra_amount: Option<Uint128>,
) -> StdResult<Response> {
    let aterra_balance = query_token_balance(
        &deps.querier,
        deps.api.addr_humanize(&config.aterra_contract)?,
        env.contract.address,
    )?;

    let aterra_to_redeem = if aterra_amount.is_none() {
        aterra_balance
    } else {
        aterra_amount.unwrap()
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut logs: Vec<Attribute> = vec![];

    if aterra_to_redeem > aterra_balance {
        return Err(StdError::generic_err("cannot withdraw more than deposit"));
    }
    if !aterra_to_redeem.is_zero() {
        let anchor_exchange_rate = query_anchor_exchange_rate(
            deps.as_ref(),
            Some(env.block.height),
            &deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        );
        let anchor_balance = aterra_to_redeem * anchor_exchange_rate?;

        let withdraw_asset = Asset {
            amount: aterra_to_redeem,
            info: AssetInfo::Token {
                contract_addr: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
            },
        };

        messages.push(withdraw_asset.into_msg_with_data(
            &deps.querier,
            deps.api.addr_humanize(&config.anchor_market)?.to_string(),
            to_binary(&AnchorMarketCw20HookMsg::RedeemStable {})?,
        )?);
        logs.push(attr("action", "withdraw_anchor"));
        logs.push(attr("aterra_amount", aterra_to_redeem));

        let mut state = read_state(deps.storage)?;

        let original_deposited =
            state.anchor_deposited * Decimal::from_ratio(aterra_to_redeem, aterra_balance);
        if original_deposited < anchor_balance {
            let performance_fee =
                anchor_balance.checked_sub(original_deposited)? * config.performance_fee;

            if !performance_fee.is_zero() {
                let fee_asset = Asset {
                    amount: performance_fee,
                    info: config.get_underlying(),
                };

                messages.push(
                    fee_asset.into_msg(
                        &deps.querier,
                        Addr::unchecked(
                            query_treasury(
                                &deps.querier,
                                deps.api.addr_humanize(&config.controller)?,
                            )?
                            .to_string(),
                        ),
                    )?,
                );
                logs.push(attr("performance_fee", performance_fee));
            } else {
                logs.push(attr("performance_fee", '0'));
            }
        } else {
            logs.push(attr("performance_fee", '0'));
        }
        state.anchor_deposited = state.anchor_deposited.checked_sub(original_deposited)?;
        store_state(deps.storage, &state)?;
    }
    Ok(Response::new().add_messages(messages).add_attributes(logs))
}

pub fn get_anchor_balance_without_fee(
    deps: DepsMut,
    env: Env,
    config: Config,
) -> StdResult<Uint128> {
    let aterra_amount = query_token_balance(
        &deps.querier,
        deps.api.addr_humanize(&config.aterra_contract)?,
        env.contract.address,
    )?;
    if !aterra_amount.is_zero() {
        let anchor_exchange_rate = query_anchor_exchange_rate(
            deps.as_ref(),
            Some(env.block.height),
            &deps.api.addr_humanize(&config.anchor_market)?.to_string(),
        );
        let anchor_balance = aterra_amount * anchor_exchange_rate?;

        let state = read_state(deps.storage)?;
        let performance_fee = if state.anchor_deposited < anchor_balance {
            anchor_balance.checked_sub(state.anchor_deposited)? * config.performance_fee
        } else {
            Uint128::zero()
        };
        Ok(anchor_balance.checked_sub(performance_fee)?)
    } else {
        Ok(Uint128::zero())
    }
}
