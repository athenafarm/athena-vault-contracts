use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult};

use crate::querier::{query_governance, query_user_role};

pub fn assert_access_privilege(
    querier: &QuerierWrapper,
    controller: Addr,
    user: Addr,
    is_worker: bool,
    is_claimer: bool,
) -> StdResult<()> {
    if !is_worker && !is_claimer {
        Ok(())
    } else {
        let user_role = query_user_role(querier, controller, user)?;
        if (is_worker && !user_role.is_worker) || (is_claimer && !user_role.is_claimer) {
            return Err(StdError::generic_err("unauthorized"));
        }

        Ok(())
    }
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
