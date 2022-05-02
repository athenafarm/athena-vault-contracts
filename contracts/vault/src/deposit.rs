use cosmwasm_std::{
    attr, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Decimal
};
use crate::claim::get_updated_yield;
use crate::querier::{get_total_balance, get_vault_balance};
use crate::state::{
    read_config, read_deposit_info, read_state, store_deposit_info, store_state,
    Config, DepositInfo,
};
use athena::asset::{Asset, AssetInfo};

/// Deposit UST and update total share
pub fn deposit_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    
    if info.funds.len() != 1 {
        return Err(StdError::generic_err("Cannot deposit several denoms"));
    }

    // Check base denom deposit
    let deposit_amount: Uint128 = info
        .funds
        .iter()
        .find(|c| c.denom == config.stable_denom)
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(StdError::generic_err("Deposit amount must be greater than 0"));
    }

    let mut state = read_state(deps.storage)?;

    let mut deposit_info = match read_deposit_info(deps.storage, &info.sender) {
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

    deposit_info.maturity = config.lock_period + env.block.time.seconds();

    let total_balance = get_total_balance(deps.as_ref(), config)?.checked_sub(deposit_amount)?;

    deposit_info.current_amount += deposit_amount;
    deposit_info.principal += deposit_amount;
    deposit_info.share = if state.total_share.is_zero() || total_balance <= state.total_subsidized {
        deposit_amount
    } else {
        state.total_share
            * Decimal::from_ratio(
                deposit_amount,
                total_balance.checked_sub(state.total_subsidized)?,
            )
    };

    state.total_share = state.total_share + deposit_info.share;
    
    store_deposit_info(deps.storage, &info.sender, &deposit_info)?;
    store_state(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "deposit"),
        attr("amount", deposit_amount),
        attr("share", deposit_info.share),
        attr("maturity", deposit_info.maturity.to_string()),
    ]))
}

/// Check withdrawable amount and execute withdraw_to_user
pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: Uint128,
    force_withdraw: bool,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    let mut deposit_info = read_deposit_info(deps.storage, &info.sender)?;

    if deposit_info.maturity > env.block.time.seconds() {
        return Err(StdError::generic_err("Still locked"));
    }
    
    let mut state = read_state(deps.storage)?;
    let is_principal_unclaimed = deposit_info.principal - deposit_info.principal_claimed > Uint128::zero();

    if !is_principal_unclaimed {
        return Err(StdError::generic_err("Already withdrawn"));
    }

    let (amount, yield_amount, loss) = get_updated_yield(
        deps.as_ref(),
        config.clone(),
        state.clone(),
        deposit_info.clone(),
    )?;
    
    deposit_info.current_amount = amount;
    deposit_info.yield_amount = if loss { Uint128::zero() } else { yield_amount };

    if loss && (!force_withdraw || !config.force_withdraw) {
        return Err(StdError::generic_err("Loss"));
    }

    let mut principal_withdraw = deposit_info.principal;
    let mut yield_withdraw = if loss {
        Uint128::zero()
    } else {
        deposit_info.yield_amount
    };

    if deposit_info.yield_claimed > deposit_info.yield_amount {
        let over_claim = deposit_info
            .yield_claimed
            .checked_sub(deposit_info.yield_amount)?;
        if yield_withdraw >= over_claim {
            yield_withdraw = yield_withdraw.checked_sub(over_claim)?;
        } else if principal_withdraw + yield_withdraw >= over_claim {
            yield_withdraw = Uint128::zero();
            principal_withdraw = (principal_withdraw + yield_withdraw).checked_sub(over_claim)?;
        } else {
            principal_withdraw = Uint128::zero();
            yield_withdraw = Uint128::zero();
        }
    }

    let total_withdrawable = principal_withdraw + yield_withdraw;
    
    if total_withdrawable.is_zero() {
        Ok(Response::new().add_attributes(vec![
            attr("action", "withdraw"),
            attr("amount", "0"),
        ]))
    } else {
        let vault_balance = get_vault_balance(deps.as_ref(), config.clone())?;
        let mut available_withdraw = withdraw_amount;
    
        if vault_balance < available_withdraw {
            if config.strategy.is_none() || (force_withdraw && config.force_withdraw) {
                available_withdraw = vault_balance;
            } else {
                return Err(StdError::generic_err("Insufficient"));
            }
        }
        
        if available_withdraw <= principal_withdraw {
            deposit_info.principal_claimed += available_withdraw;
            state.total_share = state.total_share * Decimal::from_ratio(
                principal_withdraw.checked_sub(available_withdraw)?,
                vault_balance,
            )
        }
        
        if available_withdraw > principal_withdraw && available_withdraw <= total_withdrawable {
            deposit_info.principal_claimed += principal_withdraw;
            deposit_info.yield_claimed += total_withdrawable.checked_sub(available_withdraw)?;
            state.total_share = state.total_share.checked_sub(deposit_info.share)?;
        }

        let unclaimed = deposit_info
            .clone()
            .current_amount
            .checked_sub(available_withdraw)?;
        state.total_subsidized += unclaimed;
    
        store_state(deps.storage, &state)?;
        store_deposit_info(deps.storage, &info.sender, &deposit_info)?;
    
        let asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.stable_denom,
            },
            amount: available_withdraw,
        };
    
        Ok(Response::new()
            .add_messages(vec![asset.into_msg(
                &deps.querier,
                info.sender,
            )?])
            .add_attributes(vec![
                attr("action", "withdraw"),
                attr("amount", available_withdraw),
            ]))
    }
}
