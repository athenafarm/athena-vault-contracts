use cosmwasm_std::{attr, Decimal, Deps, DepsMut, MessageInfo, Response, StdError, StdResult};

use crate::querier::get_available;
use crate::state::{read_config, store_config};
use athena::access_control::{assert_access_privilege, assert_governance_privilege};
use athena::asset::{Asset, AssetInfo};

/// Update vault configuration
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    controller: Option<String>,
    strategy: Option<String>,
    invest_percentage: Option<Decimal>,
    lock_period: Option<u64>,
    force_withdraw: Option<bool>,
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

    if let Some(strategy) = strategy {
        config.strategy = Some(deps.api.addr_canonicalize(&strategy)?);
    }

    if let Some(invest_percentage) = invest_percentage {
        if invest_percentage > Decimal::one() {
            return Err(StdError::generic_err(
                "Invest percentage must be less than 100%",
            ));
        }

        config.invest_percentage = invest_percentage;
    }
    
    if let Some(lock_period) = lock_period {
        config.lock_period = lock_period;
    }

    if let Some(force_withdraw) = force_withdraw {
        config.force_withdraw = force_withdraw;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// Send available amount to strategy
pub fn invest(deps: Deps, info: MessageInfo) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    assert_access_privilege(
        &deps.querier,
        deps.api.addr_humanize(&config.controller)?,
        info.sender,
        true,
        false,
    )?;

    if config.strategy.is_none() {
        return Err(StdError::generic_err("Strategy is not defined"));
    }

    let available_balance = get_available(deps, config.clone())?;
    if available_balance.is_zero() {
        return Err(StdError::generic_err("Nothing to invest"));
    }

    let asset = Asset {
        amount: available_balance,
        info: AssetInfo::NativeToken {
            denom: config.stable_denom,
        },
    };

    Ok(Response::new()
        .add_messages(vec![asset.into_msg(
            &deps.querier,
            deps.api.addr_humanize(&config.strategy.unwrap())?,
        )?])
        .add_attributes(vec![
            attr("action", "invest"),
            attr("amount", available_balance),
        ]))
}
