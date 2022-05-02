#[cfg(test)]
mod tests {

    use crate::contract::{execute, instantiate, query};
    use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, from_binary, Coin, Decimal, OwnedDeps, StdError, Uint128};
    use athena::vault::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

    #[test]
    fn update_config_fails_if_sender_is_unauthorized() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::UpdateConfig {
            controller: Some(String::from("controller2")),
            strategy: Some(String::from("strategy")),
            invest_percentage: Some(Decimal::percent(90u64)),
            lock_period: Some(600u64),
            force_withdraw: Some(true),
        };

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn update_config_fails_if_invest_percentage_is_greather_than_100() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        // return an error if the invest_percentage is greater than 100%
        let msg = ExecuteMsg::UpdateConfig {
            controller: Some(String::from("controller2")),
            strategy: Some(String::from("strategy")),
            invest_percentage: Some(Decimal::percent(101u64)),
            lock_period: Some(600u64),
            force_withdraw: Some(true),
        };

        let governance_info = mock_info("governance", &[]);

        let res = execute(deps.as_mut(), mock_env(), governance_info, msg).unwrap_err();

        assert_eq!(
            res,
            StdError::generic_err("Invest percentage must be less than 100%")
        );
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::UpdateConfig {
            controller: Some(String::from("controller2")),
            strategy: Some(String::from("strategy")),
            invest_percentage: Some(Decimal::percent(90u64)),
            lock_period: Some(600u64),
            force_withdraw: Some(true),
        };

        let governance_info = mock_info("governance", &[]);

        let res = execute(deps.as_mut(), mock_env(), governance_info, msg).unwrap();

        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();

        let config: ConfigResponse = from_binary(&res).unwrap();

        assert_eq!(
            ConfigResponse {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                controller: String::from("controller2"),
                strategy: Some(String::from("strategy")),
                stable_denom: String::from("uusd"),
                invest_percentage: Decimal::percent(90u64),
                lock_period: 600u64,
                force_withdraw: true,
            },
            config
        );
    }

    #[test]
    fn invest_fails_if_sender_is_not_worker() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("addr1", &[]),
            ExecuteMsg::Invest {},
        )
        .unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("claimer", &[]),
            ExecuteMsg::Invest {},
        )
        .unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn invest_fails_if_strategy_is_not_defined() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::Invest {};

        let worker_info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), worker_info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Strategy is not defined"));
    }

    #[test]
    fn invest_fails_if_nothing_to_invest() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        update_contract_with_stategy(&mut deps, "strategy".to_string());

        deps.querier
            .with_invested_balance(&Uint128::from(3800000000u128));

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("worker", &[]),
            ExecuteMsg::Invest {},
        )
        .unwrap_err();

        assert_eq!(res, StdError::generic_err("Nothing to invest"));
    }

    #[test]
    fn invest() {
        let vault_balance = Uint128::from(200000000u128);
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        update_contract_with_stategy(&mut deps, "strategy".to_string());

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_balance,
            }],
        )]);

        let msg = ExecuteMsg::Invest {};
        let worker_info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), worker_info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "invest"),
                attr("amount", vault_balance * Decimal::percent(95u64)),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::TotalBalance {}).unwrap();

        let total_balance: Uint128 = from_binary(&res).unwrap();

        assert_eq!(vault_balance, total_balance);
    }

    fn instantiate_contract(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) {
        let msg = InstantiateMsg {
            controller: String::from("controller"),
            stable_denom: String::from("uusd"),
            invest_percentage: Decimal::percent(95u64),
            lock_period: 600u64
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
