use cosmwasm_std::{
    attr, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};

use crate::state::{read_config, store_config};
use athena::access_control::{assert_access_privilege, assert_governance_privilege};
use athena::asset::Asset;

/// Update strategy configuration
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    controller: Option<String>,
    performance_fee: Option<Decimal>,
) -> StdResult<Response> {
    let mut config = read_config(deps.storage)?;

    assert_governance_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        &info.sender.to_string(),
    )?;

    if let Some(controller) = controller {
        config.controller = deps.api.addr_canonicalize(&controller)?;
    }

    if let Some(performance_fee) = performance_fee {
        if performance_fee > Decimal::one() {
            return Err(StdError::generic_err(
                "Performance fee percentage must be less than 100%",
            ));
        }

        config.performance_fee = performance_fee;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Withdraw underlying asset to vault
/// If amount is None, then withdraw all underlying assets
pub fn withdraw_to_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    if info.sender != deps.api.addr_humanize(&config.vault)? {
        assert_access_privilege(
            &deps.querier,
            deps.api.addr_humanize(&config.controller)?,
            info.sender,
            true,
            false,
        )?;
    }

    let withdraw_amount = if amount.is_none() {
        config
            .get_underlying()
            .query_balance(&deps.querier, deps.api, env.contract.address)?
    } else {
        amount.unwrap()
    };

    if withdraw_amount.is_zero() {
        return Err(StdError::generic_err("cannot withdraw zero"));
    }
    let withdraw_asset = Asset {
        amount: withdraw_amount,
        info: config.get_underlying(),
    };

    Ok(Response::new()
        .add_message(
            withdraw_asset.into_msg(&deps.querier, deps.api.addr_humanize(&config.vault)?)?,
        )
        .add_attributes(vec![
            attr("action", "withdraw_to_vault"),
            attr("amount", withdraw_amount),
        ]))
}
