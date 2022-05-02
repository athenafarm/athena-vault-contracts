#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Coin, Decimal, OwnedDeps, StdError, Uint128};
    use athena::vault::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, State};

    #[test]
    fn instantiate_sets_the_correct_state() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();

        let config: ConfigResponse = from_binary(&res).unwrap();

        assert_eq!(
            ConfigResponse {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                controller: String::from("controller"),
                strategy: None,
                stable_denom: String::from("uusd"),
                invest_percentage: Decimal::percent(95u64),
                lock_period: 100u64,
                force_withdraw: false,
            },
            config
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();

        let state: State = from_binary(&res).unwrap();

        assert_eq!(
            State {
                total_share: Uint128::zero(),
                total_subsidized: Uint128::zero(),
            },
            state
        );
    }

    #[test]
    fn instantiate_fails_if_investment_percentage_is_greater_than_100() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        let info = mock_info("addr", &[]);

        let msg = InstantiateMsg {
            controller: String::from("controller"),
            stable_denom: String::from("uusd"),
            invest_percentage: Decimal::percent(101u64),
            lock_period: 100u64,
        };

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();

        assert_eq!(
            res,
            StdError::generic_err("Invest percentage must be less than 100%")
        );
    }

    #[test]
    fn vault_balance() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::VaultBalance {}).unwrap();

        let vault_balance: Uint128 = from_binary(&res).unwrap();

        assert_eq!(Uint128::zero(), vault_balance);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            }],
        )]);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::VaultBalance {}).unwrap();
        let vault_balance: Uint128 = from_binary(&res).unwrap();

        assert_eq!(Uint128::from(200u128), vault_balance);
    }

    #[test]
    fn underlying_balance_takes_the_invested_balance_into_account() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            }],
        )]);

        update_contract_with_stategy(&mut deps, "strategy".to_string());

        deps.querier.with_invested_balance(&Uint128::from(100u128));

        let res = query(deps.as_ref(), mock_env(), QueryMsg::TotalBalance {}).unwrap();
        let total_balance: Uint128 = from_binary(&res).unwrap();

        assert_eq!(Uint128::from(300u128), total_balance);
    }

    #[test]
    fn available_balance_is_0_when_no_strategy_is_set() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            }],
        )]);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Available {}).unwrap();
        let available_amount: Uint128 = from_binary(&res).unwrap();

        assert_eq!(Uint128::from(0u128), available_amount);
    }

    #[test]
    fn available_balance_takes_the_invest_percentage_into_account() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        update_contract_with_stategy(&mut deps, "strategy".to_string());

        let vault_balance = Uint128::from(200u128);
        let invested_balance = Uint128::from(100u128);
        let invest_percentage = Decimal::percent(95u64);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_balance,
            }],
        )]);

        deps.querier.with_invested_balance(&invested_balance);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Available {}).unwrap();

        let available_amount: Uint128 = from_binary(&res).unwrap();

        assert_eq!(
            ((vault_balance + invested_balance) * invest_percentage
                - Uint128::from(invested_balance)),
            available_amount
        );
    }

    #[test]
    fn available_balance_is_0_when_the_invest_percentage_is_exceeded() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        update_contract_with_stategy(&mut deps, "strategy".to_string());

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(5u128),
            }],
        )]);

        deps.querier.with_invested_balance(&Uint128::from(100u128));

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Available {}).unwrap();
        let available_amount: Uint128 = from_binary(&res).unwrap();

        assert_eq!(Uint128::from(0u128), available_amount);
    }

    fn instantiate_contract(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) {
        let msg = InstantiateMsg {
            controller: String::from("controller"),
            stable_denom: String::from("uusd"),
            invest_percentage: Decimal::percent(95u64),
            lock_period: 100u64,
        };

        let info = mock_info("addr", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    fn update_contract_with_stategy(
        deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
        strategy: String,
    ) {
        let msg = ExecuteMsg::UpdateConfig {
            controller: None,
            strategy: Some(strategy),
            invest_percentage: None,
            force_withdraw: None,
            lock_period: None,
        };

        let info = mock_info("governance", &[]);

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
