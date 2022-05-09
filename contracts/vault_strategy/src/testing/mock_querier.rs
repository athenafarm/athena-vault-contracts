use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, CanonicalAddr, Coin, ContractResult, Decimal,
    OwnedDeps, Querier, QuerierResult, QueryRequest, StdResult, SystemError, SystemResult, Uint128,
    WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use std::collections::HashMap;

use anchor_market::market::{
    ConfigResponse as AnchorMarketConfigResponse,
    EpochStateResponse as AnchorMarketEpochStateResponse, QueryMsg as AnchorMarketQueryMsg,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use mirror_protocol::oracle::{
    PriceResponse as MirrorOraclePriceResponse, QueryMsg as MirrorOracleQueryMsg,
};
use mirror_protocol::staking::{
    QueryMsg as MirrorStakingQueryMsg, RewardInfoResponse as MirrorStakingRewardInfoResponse,
    RewardInfoResponseItem as MirrorStakingRewardInfoResponseItem,
};
use athena::controller::{
    ConfigResponse as ControllerConfigResponse, QueryMsg as ControllerQueryMsg, UserRole,
};
use athena::vault_strategy::QueryMsg;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};
use terraswap::asset::AssetInfo;
use terraswap::asset::PairInfo;
use terraswap::factory::QueryMsg as TerraSwapFactoryQueryMsg;
use terraswap::pair::{QueryMsg as TerraSwapQueryMsg, SimulationResponse};

static DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    token_querier: TokenQuerier,
    pair_info_querier: PairInfoQuerier,
    governance: String,
    treasury: String,
    tax: (Decimal, Uint128),
    exchange_rate: Decimal,
    aterra_supply: Uint128,
    reward_info: Vec<MirrorStakingRewardInfoResponseItem>,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
    supplies: HashMap<String, Uint128>,
    prices: HashMap<String, Decimal>,
}

impl TokenQuerier {
    pub fn new(
        balances: &[(&String, &[(&String, &Uint128)])],
        supplies: &[(&String, &Uint128)],
        prices: &[(&String, &Decimal)],
    ) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
            supplies: supplies_to_map(supplies),
            prices: prices_to_map(prices),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert((*addr).clone(), **balance);
        }

        balances_map.insert((*contract_addr).clone(), contract_balances_map);
    }
    balances_map
}

pub(crate) fn supplies_to_map(supplies: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut supplies_map: HashMap<String, Uint128> = HashMap::new();
    for (contract_addr, supply) in supplies.iter() {
        supplies_map.insert((*contract_addr).clone(), **supply);
    }
    supplies_map
}

pub(crate) fn prices_to_map(prices: &[(&String, &Decimal)]) -> HashMap<String, Decimal> {
    let mut prices_map: HashMap<String, Decimal> = HashMap::new();
    for (contract_addr, price) in prices.iter() {
        prices_map.insert((*contract_addr).clone(), **price);
    }
    prices_map
}

#[derive(Clone, Default)]
pub struct PairInfoQuerier {
    // this lets us iterate over all pairs that match the first string
    pair_info: HashMap<String, [String; 2]>,
}

impl PairInfoQuerier {
    pub fn new(pair_info: &[(&String, &[String; 2])]) -> Self {
        PairInfoQuerier {
            pair_info: pair_info_to_map(pair_info),
        }
    }
}

pub(crate) fn pair_info_to_map(
    pair_info: &[(&String, &[String; 2])],
) -> HashMap<String, [String; 2]> {
    let mut pair_info_map: HashMap<String, [String; 2]> = HashMap::new();
    for (asset, asset_info) in pair_info.iter() {
        pair_info_map.insert((*asset).clone(), (*asset_info).clone());
    }
    pair_info_map
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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg) {
                    Ok(QueryMsg::Config {}) => {
                        if contract_addr.clone() == String::from("anchor_market") {
                            SystemResult::Ok(ContractResult::from(to_binary(
                                &AnchorMarketConfigResponse {
                                    owner_addr: String::from("owner_addr"),
                                    aterra_contract: String::from("aterra_contract"),
                                    interest_model: String::from("interest_model"),
                                    distribution_model: String::from("distribution_model"),
                                    overseer_contract: String::from("overseer_contract"),
                                    collector_contract: String::from("collector_contract"),
                                    distributor_contract: String::from("distributor_contract"),
                                    stable_denom: "uusd".to_string(),
                                    max_borrow_factor: Decimal::percent(70u64).into(),
                                },
                            )))
                        } else if contract_addr.clone() == String::from("controller") {
                            SystemResult::Ok(ContractResult::from(to_binary(
                                &ControllerConfigResponse {
                                    governance: self.governance.clone(),
                                    treasury: self.treasury.clone(),
                                },
                            )))
                        } else {
                            panic!("DO NOT ENTER HERE")
                        }
                    }

                    _ => match from_binary(&msg) {
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
                            Ok(TerraSwapFactoryQueryMsg::Pair { asset_infos }) => {
                                let mut mirror_token: String = String::default();
                                match &asset_infos[0] {
                                    AssetInfo::Token { contract_addr } => {
                                        mirror_token = contract_addr.clone()
                                    }
                                    _ => {}
                                }
                                match &asset_infos[1] {
                                    AssetInfo::Token { contract_addr } => {
                                        mirror_token = contract_addr.clone()
                                    }
                                    _ => {}
                                }
                                let pair_info =
                                    self.pair_info_querier.pair_info.get(&mirror_token).unwrap();
                                SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                                    asset_infos,
                                    contract_addr: String::from(&pair_info[0]),
                                    liquidity_token: String::from(&pair_info[1]),
                                })))
                            }
                            _ => match from_binary(&msg) {
                                Ok(AnchorMarketQueryMsg::EpochState { .. }) => {
                                    SystemResult::Ok(ContractResult::from(to_binary(
                                        &AnchorMarketEpochStateResponse {
                                            exchange_rate: self.exchange_rate.into(),
                                            aterra_supply: self.aterra_supply.into(),
                                        },
                                    )))
                                }
                                _ => match from_binary(&msg) {
                                    Ok(TerraSwapQueryMsg::Simulation { offer_asset }) => {
                                        let mut mirror_token: String = String::default();
                                        match &offer_asset.info {
                                            AssetInfo::Token { contract_addr } => {
                                                mirror_token = contract_addr.clone()
                                            }
                                            _ => {}
                                        }

                                        let rate =
                                            self.token_querier.prices.get(&mirror_token).unwrap();
                                        SystemResult::Ok(ContractResult::from(to_binary(
                                            &SimulationResponse {
                                                return_amount: offer_asset.amount * rate.clone(),
                                                spread_amount: Uint128::zero(),
                                                commission_amount: Uint128::zero(),
                                            },
                                        )))
                                    }
                                    _ => match from_binary(&msg) {
                                        Ok(MirrorStakingQueryMsg::RewardInfo { .. }) => {
                                            SystemResult::Ok(ContractResult::from(to_binary(
                                                &MirrorStakingRewardInfoResponse {
                                                    staker_addr: String::from(MOCK_CONTRACT_ADDR),
                                                    reward_infos: (&self.reward_info).clone(),
                                                },
                                            )))
                                        }
                                        _ => match from_binary(&msg) {
                                            Ok(MirrorOracleQueryMsg::Price {
                                                base_asset, ..
                                            }) => {
                                                let rate = self
                                                    .token_querier
                                                    .prices
                                                    .get(&String::from(base_asset))
                                                    .unwrap();
                                                SystemResult::Ok(ContractResult::from(to_binary(
                                                    &MirrorOraclePriceResponse {
                                                        rate: rate.clone().clone(),
                                                        last_updated_base: 100u64,
                                                        last_updated_quote: 100u64,
                                                    },
                                                )))
                                            }
                                            _ => match from_binary(msg).unwrap() {
                                                Cw20QueryMsg::TokenInfo {} => {
                                                    let supply: &Uint128 =
                                                        match self.token_querier.supplies.get(contract_addr) {
                                                            Some(supply) => supply,
                                                            None => {
                                                                return SystemResult::Err(SystemError::InvalidRequest {
                                                                    error: format!(
                                                                        "No token info exists for the contract {}",
                                                                        contract_addr
                                                                    ),
                                                                    request: msg.as_slice().into(),
                                                                })
                                                            }
                                                        };
                                                    SystemResult::Ok(ContractResult::Ok(
                                                        to_binary(&TokenInfoResponse {
                                                            name: "mAAPL".to_string(),
                                                            symbol: "mAAPL".to_string(),
                                                            decimals: 6,
                                                            total_supply: supply.clone(),
                                                        })
                                                        .unwrap(),
                                                    ))
                                                }
                                                Cw20QueryMsg::Balance { address } => {
                                                    let balances: &HashMap<String, Uint128> =
                                                        match self.token_querier.balances.get(contract_addr) {
                                                            Some(balances) => balances,
                                                            None => {
                                                                return SystemResult::Err(SystemError::InvalidRequest {
                                                                    error: format!(
                                                                        "No balance info exists for the contract {}",
                                                                        contract_addr
                                                                    ),
                                                                    request: msg.as_slice().into(),
                                                                })
                                                            }
                                                        };
                                                    let balance = match balances.get(&address) {
                                                        Some(v) => *v,
                                                        None => {
                                                            return SystemResult::Ok(
                                                                ContractResult::Ok(
                                                                    to_binary(
                                                                        &Cw20BalanceResponse {
                                                                            balance: Uint128::zero(
                                                                            ),
                                                                        },
                                                                    )
                                                                    .unwrap(),
                                                                ),
                                                            );
                                                        }
                                                    };
                                                    SystemResult::Ok(ContractResult::Ok(
                                                        to_binary(&Cw20BalanceResponse { balance })
                                                            .unwrap(),
                                                    ))
                                                }
                                                _ => panic!("DO NOT ENTER HERE"),
                                            },
                                        },
                                    },
                                },
                            },
                        },
                    },
                }
            }
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();
                let prefix_balance = to_length_prefixed(b"balance").to_vec();
                let prefix_token_info = to_length_prefixed(b"token_info").to_vec();

                if key[..prefix_balance.len()].to_vec() == prefix_balance {
                    let balances: &HashMap<String, Uint128> =
                        match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
                            None => {
                                return SystemResult::Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No balance info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: key.into(),
                                })
                            }
                        };

                    let key_address: &[u8] = &key[prefix_balance.len()..];
                    let address_raw: CanonicalAddr = CanonicalAddr::from(key_address);

                    let api: MockApi = MockApi::default();
                    let address: String = match api.addr_humanize(&address_raw) {
                        Ok(v) => v.to_string(),
                        Err(e) => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: format!("Parsing query request: {}", e),
                                request: key.into(),
                            })
                        }
                    };

                    let balance = match balances.get(&address) {
                        Some(v) => v,
                        None => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: "Balance not found".to_string(),
                                request: key.into(),
                            })
                        }
                    };

                    SystemResult::Ok(ContractResult::from(to_binary(
                        &to_binary(&balance).unwrap(),
                    )))
                } else if key[..prefix_token_info.len()].to_vec() == prefix_token_info {
                    let supply: &Uint128 = match self.token_querier.supplies.get(contract_addr) {
                        Some(supply) => supply,
                        None => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: format!(
                                    "No supply info exists for the contract {}",
                                    contract_addr
                                ),
                                request: key.into(),
                            })
                        }
                    };

                    SystemResult::Ok(ContractResult::from(to_binary(
                        &to_binary(&TokenInfoResponse {
                            name: "mock".into(),
                            symbol: "mock".into(),
                            decimals: 6u8,
                            total_supply: supply.clone().clone(),
                        })
                        .unwrap(),
                    )))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>, _canonical_length: usize) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            pair_info_querier: PairInfoQuerier::default(),
            governance: String::from("governance"),
            treasury: String::from("treasury"),
            tax: (Decimal::percent(1), Uint128::from(1000000u128)),
            exchange_rate: Decimal::zero(),
            aterra_supply: Uint128::zero(),
            reward_info: vec![],
        }
    }

    pub fn with_balance(&mut self, balances: &[(&String, &[Coin])]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.to_vec());
        }
    }

    pub fn with_token_info(
        &mut self,
        balances: &[(&String, &[(&String, &Uint128)])],
        supplies: &[(&String, &Uint128)],
        prices: &[(&String, &Decimal)],
    ) {
        self.token_querier = TokenQuerier::new(balances, supplies, prices);
    }

    pub fn with_anchor_epoch_state(&mut self, exchange_rate: Decimal, aterra_supply: Uint128) {
        self.exchange_rate = exchange_rate;
        self.aterra_supply = aterra_supply;
    }

    pub fn with_mirror_asset(&mut self, pair_info: &[(&String, &[String; 2])]) {
        self.pair_info_querier = PairInfoQuerier::new(pair_info);
    }

    pub fn compute_tax(&self, amount: Uint128) -> StdResult<Uint128> {
        let tax = amount.checked_sub(
            amount
                * Decimal::from_ratio(
                    DECIMAL_FRACTIONAL,
                    DECIMAL_FRACTIONAL * (Decimal::one() + self.tax.0),
                ),
        )?;

        Ok(std::cmp::min(tax, self.tax.1))
    }

    pub fn deduct_tax(&self, amount: Uint128) -> StdResult<Uint128> {
        let tax = self.compute_tax(amount).unwrap();

        Ok(amount.checked_sub(tax)?)
    }

    pub fn with_reward_info(&mut self, reward_info: Vec<MirrorStakingRewardInfoResponseItem>) {
        self.reward_info = reward_info;
    }
}
