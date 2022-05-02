use cosmwasm_std::{
    attr, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};

use crate::querier::{get_balance_by_share, get_total_balance, get_vault_balance};
use crate::state::{
    read_config, read_deposit_info, read_state, store_deposit_info,
    store_state, Config, DepositInfo,
};
use athena::asset::{Asset, AssetInfo};
use athena::vault::State;

/// Claim yield
pub fn claim_yield(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    let mut deposit_info = read_deposit_info(deps.storage, &info.sender)?;

    if deposit_info.maturity > env.block.time.seconds() {
        return Err(StdError::generic_err("Still locked"));
    }

    let mut state = read_state(deps.storage)?;
    let is_principal_unclaimed = deposit_info.principal - deposit_info.principal_claimed > Uint128::zero();
    
    if is_principal_unclaimed {
        let (amount, yield_amount, loss) = get_updated_yield(
            deps.as_ref(),
            config.clone(),
            state.clone(),
            deposit_info.clone(),
        )?;

        if loss || yield_amount <= deposit_info.yield_claimed {
            return Err(StdError::generic_err("Loss"));
        }

        deposit_info.current_amount = amount;
        deposit_info.yield_amount = yield_amount;
    }

    let claimable = deposit_info.yield_amount;
    
    if claimable <= deposit_info.yield_claimed {
        return Err(StdError::generic_err("Nothing to claim"));
    }

    if claimable > get_vault_balance(deps.as_ref(), config.clone())? {
        return Err(StdError::generic_err("Insufficient"));
    }

    deposit_info.yield_claimed += claimable;
    
    if is_principal_unclaimed {
        deposit_info.current_amount = deposit_info.current_amount.checked_sub(claimable)?;

        let total_balance = get_total_balance(deps.as_ref(), config.clone())?.checked_sub(state.total_subsidized)?;
        let claimable_share = state.total_share * Decimal::from_ratio(claimable, total_balance);
        
        deposit_info.share = deposit_info.share.checked_sub(claimable_share)?;
        state.total_share = state.total_share.checked_sub(claimable_share)?;
    } else {
        state.total_subsidized = state.total_subsidized.checked_sub(claimable)?;
    }

    store_state(deps.storage, &state)?;
    store_deposit_info(deps.storage, &info.sender, &deposit_info)?;

    let asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.stable_denom,
        },
        amount: claimable,
    };

    Ok(Response::new()
        .add_messages(vec![asset.into_msg(
            &deps.querier,
            info.sender,
        )?])
        .add_attributes(vec![
            attr("action", "claim_yield"),
            attr("amount", claimable),
        ]))
}

/// Claim principal
pub fn claim_principal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    let mut deposit_info = read_deposit_info(deps.storage, &info.sender)?;

    if deposit_info.maturity > env.block.time.seconds() {
        return Err(StdError::generic_err("Still locked"));
    }
    
    let mut state = read_state(deps.storage)?;
    let is_principal_unclaimed = deposit_info.principal - deposit_info.principal_claimed > Uint128::zero();
    
    if is_principal_unclaimed {
        let (amount, yield_amount, loss) = get_updated_yield(
            deps.as_ref(),
            config.clone(),
            state.clone(),
            deposit_info.clone(),
        )?;
        
        if loss || yield_amount <= deposit_info.yield_claimed {
            return Err(StdError::generic_err("Loss"));
        }

        deposit_info.current_amount = amount;
        deposit_info.yield_amount = yield_amount;
    } else {
        return Err(StdError::generic_err("Principal already claimed"));
    }

    let claimable = deposit_info.principal;

    if claimable > get_vault_balance(deps.as_ref(), config.clone())? {
        return Err(StdError::generic_err("Insufficient"));
    }

    deposit_info.principal_claimed += claimable;
    
    if is_principal_unclaimed {
        deposit_info.current_amount = deposit_info.current_amount.checked_sub(claimable)?;

        let total_balance = get_total_balance(deps.as_ref(), config.clone())?.checked_sub(state.total_subsidized)?;
        let claimable_share = state.total_share * Decimal::from_ratio(claimable, total_balance);

        deposit_info.share = deposit_info.share.checked_sub(claimable_share)?;
        state.total_share = state.total_share.checked_sub(claimable_share)?;
    } else {
        state.total_subsidized = state.total_subsidized.checked_sub(claimable)?;
    }

    store_state(deps.storage, &state)?;
    store_deposit_info(deps.storage, &info.sender, &deposit_info)?;

    let asset = Asset {
        info: AssetInfo::NativeToken {
            denom: config.stable_denom,
        },
        amount: claimable,
    };

    Ok(Response::new()
        .add_messages(vec![asset.into_msg(
            &deps.querier,
            info.sender,
        )?])
        .add_attributes(vec![
            attr("action", "claim_principal"),
            attr("amount", claimable),
        ]))
}

pub fn get_updated_yield(
    deps: Deps,
    config: Config,
    state: State,
    deposit_info: DepositInfo,
) -> StdResult<(Uint128, Uint128, bool)> {
    let amount = get_balance_by_share(deps, config, state, deposit_info.share)?;
    let mut loss = false;

    // if lose == true, yield_amount indicates lost amount
    let yield_amount = if amount >= deposit_info.principal {
        amount.checked_sub(deposit_info.principal)?
    } else {
        loss = true;
        deposit_info.principal.checked_sub(amount)?
    };

    Ok((amount, yield_amount, loss))
}
