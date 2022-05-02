#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use crate::state::{read_config, read_user_role, store_config, store_user_role, Config};
use athena::controller::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UserRole,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            governance: deps.api.addr_canonicalize(&msg.governance)?,
            treasury: deps.api.addr_canonicalize(&msg.treasury)?,
        },
    )?;

    store_user_role(
        deps.storage,
        &deps.api.addr_canonicalize(&msg.governance)?,
        &UserRole {
            is_worker: true,
            is_claimer: true,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        _ => {
            assert_governance_privilege(deps.as_ref(), info)?;
            match msg {
                ExecuteMsg::UpdateConfig {
                    governance,
                    treasury,
                } => update_config(deps, governance, treasury),
                ExecuteMsg::UpdateRole {
                    user,
                    is_worker,
                    is_claimer,
                } => update_user_role(deps, user, is_worker, is_claimer),
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::UserRole { user } => to_binary(&query_user_role(deps, user)?),
    }
}

fn assert_governance_privilege(deps: Deps, info: MessageInfo) -> StdResult<()> {
    if read_config(deps.storage)?.governance != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(())
}

pub fn update_config(
    deps: DepsMut,
    governance: Option<String>,
    treasury: Option<String>,
) -> StdResult<Response> {
    let mut config = read_config(deps.storage)?;

    if let Some(governance) = governance {
        config.governance = deps.api.addr_canonicalize(&governance)?;
    }

    if let Some(treasury) = treasury {
        config.treasury = deps.api.addr_canonicalize(&treasury)?;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn update_user_role(
    deps: DepsMut,
    user: String,
    is_worker: bool,
    is_claimer: bool,
) -> StdResult<Response> {
    store_user_role(
        deps.storage,
        &deps.api.addr_canonicalize(&user)?,
        &UserRole {
            is_worker,
            is_claimer,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update_user_role"),
        attr("user", user),
        attr("is_worker", is_worker.to_string()),
        attr("is_claimer", is_claimer.to_string()),
    ]))
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = read_config(deps.storage)?;

    let resp = ConfigResponse {
        governance: deps.api.addr_humanize(&config.governance)?.to_string(),
        treasury: deps.api.addr_humanize(&config.treasury)?.to_string(),
    };

    Ok(resp)
}

pub fn query_user_role(deps: Deps, user: String) -> StdResult<UserRole> {
    let user_role = read_user_role(deps.storage, &deps.api.addr_canonicalize(&user)?)?;

    Ok(user_role)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
