use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Decimal, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};

use athena::controller::{
    ConfigResponse as ControllerConfigResponse, QueryMsg as ControllerQueryMsg, UserRole,
};
use athena::vault_strategy::QueryMsg as StrategyQueryMsg;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    governance: String,
    treasury: String,
    invested_balance: Uint128,
    tax: (Decimal, Uint128),
}

pub fn mock_dependencies_with_querier(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = String::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        canonical_length,
    );

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if route == &TerraRoute::Treasury {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse { rate: self.tax.0 };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        TerraQuery::TaxCap { .. } => {
                            let res = TaxCapResponse { cap: self.tax.1 };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(&msg) {
                Ok(ControllerQueryMsg::Config {}) => {
                    SystemResult::Ok(ContractResult::from(to_binary(&ControllerConfigResponse {
                        governance: self.governance.clone(),
                        treasury: self.treasury.clone(),
                    })))
                }
                Ok(ControllerQueryMsg::UserRole { user }) => {
                    if user == String::from("worker") {
                        SystemResult::Ok(ContractResult::from(to_binary(&UserRole {
                            is_worker: true,
                        })))
                    } else if user == String::from("governance") {
                        SystemResult::Ok(ContractResult::from(to_binary(&UserRole {
                            is_worker: true,
                        })))
                    } else {
                        SystemResult::Ok(ContractResult::from(to_binary(&UserRole {
                            is_worker: false,
                        })))
                    }
                }
                _ => match from_binary(&msg) {
                    Ok(StrategyQueryMsg::TotalBalance { .. }) => SystemResult::Ok(
                        ContractResult::from(to_binary(&self.invested_balance.clone())),
                    ),
                    _ => panic!("DO NOT ENTER HERE"),
                },
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>, _canonical_length: usize) -> Self {
        WasmMockQuerier {
            base,
            governance: String::from("governance"),
            treasury: String::from("treasury"),
            invested_balance: Uint128::from(0u128),
            tax: (Decimal::percent(1), Uint128::from(1000000u128)),
        }
    }

    pub fn with_balance(&mut self, balances: &[(&String, &[Coin])]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.to_vec());
        }
    }

    pub fn with_invested_balance(&mut self, invested_balance: &Uint128) {
        self.invested_balance = invested_balance.clone();
    }

    pub fn compute_tax(&self, amount: Uint128) -> SystemResult<Uint128> {
        let tax = amount
            .checked_sub(amount.multiply_ratio(
                DECIMAL_FRACTION,
                DECIMAL_FRACTION * Decimal::percent(1) + DECIMAL_FRACTION,
            ))
            .unwrap();

        SystemResult::Ok(std::cmp::min(tax, self.tax.1))
    }

    pub fn deduct_tax(&self, amount: Uint128) -> SystemResult<Uint128> {
        let tax = self.compute_tax(amount).unwrap();

        SystemResult::Ok(amount - tax)
    }
}
