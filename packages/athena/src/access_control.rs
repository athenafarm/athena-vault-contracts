use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult};

use crate::querier::{query_governance, query_user_role};

pub fn assert_access_privilege(
    querier: &QuerierWrapper,
    controller: Addr,
    user: Addr,
) -> StdResult<()> {
    let user_role = query_user_role(querier, controller, user)?;

    if !user_role.is_worker {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(())
}

pub fn assert_sender_privilege(sender: String, required_sender: String) -> StdResult<()> {
    if sender != required_sender {
        return Err(StdError::generic_err("unauthorized"));
    }
    Ok(())
}

pub fn assert_governance_privilege(
    querier: &QuerierWrapper,
    controller: Addr,
    user: &String,
) -> StdResult<()> {
    if query_governance(querier, controller)? != String::from(user) {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(())
}
