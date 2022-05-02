use crate::controller::{
    ConfigResponse as ControllerConfigResponse, QueryMsg as ControllerQueryMsg, UserRole,
};
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankQuery, Coin, Decimal, QuerierWrapper, QueryRequest,
    StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use terra_cosmwasm::TerraQuerier;

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000;

pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn query_supply(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<Uint128> {
    // load price form the oracle
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info.total_supply)
}

pub fn query_tax_rate(querier: &QuerierWrapper) -> StdResult<Decimal> {
    let terra_querier = TerraQuerier::new(querier);
    Ok(terra_querier.query_tax_rate()?.rate)
}

pub fn query_tax_cap(querier: &QuerierWrapper, denom: String) -> StdResult<Uint128> {
    let terra_querier = TerraQuerier::new(querier);
    Ok(terra_querier.query_tax_cap(denom)?.cap)
}

pub fn compute_tax(querier: &QuerierWrapper, coin: &Coin) -> StdResult<Uint128> {
    let terra_querier = TerraQuerier::new(querier);
    let tax_rate = (terra_querier.query_tax_rate()?).rate;
    let tax_cap = (terra_querier.query_tax_cap(coin.denom.to_string())?).cap;
    let tax = coin
        .amount
        .checked_sub(
            coin.amount
                * Decimal::from_ratio(
                    DECIMAL_FRACTIONAL,
                    Uint128::from(DECIMAL_FRACTIONAL) * (Decimal::one() + tax_rate),
                ),
        )
        .unwrap();
    Ok(std::cmp::min(tax, tax_cap))
}

pub fn deduct_tax(querier: &QuerierWrapper, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(querier, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: coin.amount.checked_sub(tax_amount).unwrap(),
    })
}

pub fn query_governance(querier: &QuerierWrapper, controller: Addr) -> StdResult<String> {
    let controller_config: ControllerConfigResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: controller.to_string(),
            msg: to_binary(&ControllerQueryMsg::Config {})?,
        }))?;

    Ok(controller_config.governance)
}

pub fn query_treasury(querier: &QuerierWrapper, controller: Addr) -> StdResult<String> {
    let controller_config: ControllerConfigResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: controller.to_string(),
            msg: to_binary(&ControllerQueryMsg::Config {})?,
        }))?;

    Ok(controller_config.treasury)
}

pub fn query_user_role(
    querier: &QuerierWrapper,
    controller: Addr,
    user: Addr,
) -> StdResult<UserRole> {
    let user_role: UserRole = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: controller.to_string(),
        msg: to_binary(&ControllerQueryMsg::UserRole {
            user: user.to_string(),
        })?,
    }))?;

    Ok(user_role)
}
