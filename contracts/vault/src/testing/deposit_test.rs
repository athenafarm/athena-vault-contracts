#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::testing::mock_querier::mock_dependencies_with_querier;
    use crate::testing::mock_querier::WasmMockQuerier;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        attr, from_binary, BankMsg, Coin, CosmosMsg, Decimal, Env, MessageInfo, OwnedDeps,
        StdError, SubMsg, Uint128,
    };
    use athena::vault::{DepositInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg, State};

    #[test]
    fn withdraw_to_user() {
        let mut deps = dependencies_with_balance();
        let balance = Uint128::from(100000000u128);
        
        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: balance,
            }],
        );

        execute_deposit(&mut deps, info);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: balance,
            force_withdraw: false,
        };

        let info = mock_info("addr0000", &[]);

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: deps
                        .querier
                        .deduct_tax(balance)
                        .unwrap()
                }],
            }))]
        );
    }

    #[test]
    fn withdraw_with_yield_over_claim() {
        let mut deps = dependencies_with_balance();
        let vault_balance = Uint128::from(100000000u128);
        let vault_max_balance = vault_balance * Decimal::percent(190);
        let vault_min_balance = vault_balance * Decimal::percent(90);

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::UpdateConfig {
            controller: None,
            strategy: Some(String::from("strategy")),
            invest_percentage: None,
            force_withdraw: Some(true),
            lock_period: Some(200u64),
        };

        let governance_info = mock_info("governance", &[]);

        execute(deps.as_mut(), mock_env(), governance_info.clone(), msg).unwrap();

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_balance,
            }],
        );

        execute_deposit(&mut deps, info);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_max_balance,
            }],
        )]);

        let claim_yield_msg = ExecuteMsg::ClaimYield {};
        let info = mock_info("addr0000", &[]);

        execute(deps.as_mut(), env.clone(), info, claim_yield_msg).unwrap();

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_min_balance,
            }],
        )]);

        let info = mock_info("addr0000", &[]);
        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: vault_balance,
            force_withdraw: true,
        };

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: deps
                        .querier
                        .deduct_tax(vault_min_balance)
                        .unwrap()
                }],
            }))]
        );
    }

    #[test]
    fn force_withdraw_with_insufficient_funds() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::UpdateConfig {
            controller: None,
            strategy: Some(String::from("strategy")),
            invest_percentage: None,
            force_withdraw: Some(true),
            lock_period: None,
        };

        let governance_info = mock_info("governance", &[]);

        execute(deps.as_mut(), mock_env(), governance_info, msg).unwrap();

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let msg = ExecuteMsg::Deposit {};

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("addr0000", &[]);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(70000000u128),
            }],
        )]);

        deps.querier
            .with_invested_balance(&Uint128::from(80000000u128));

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: Uint128::from(100000000u128),
            force_withdraw: true,
        };

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr0000"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(69306930u128)
                }],
            }))]
        );
    }

    #[test]
    fn withdraw_fails_if_already_withdrawn() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        execute_deposit(&mut deps, info);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        let info = mock_info("addr0000", &[]);

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: Uint128::from(100000000u128),
            force_withdraw: false,
        };

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            withdraw_msg.clone(),
        )
        .unwrap();

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Already withdrawn"));
    }

    #[test]
    fn withdraw_fails_if_loss() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        execute_deposit(&mut deps, info);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(70000000u128),
            }],
        )]);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        let info = mock_info("addr0000", &[]);

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: Uint128::from(100000000u128),
            force_withdraw: false,
        };

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Loss"));
    }

    #[test]
    fn withdraw_fails_when_insufficient_funds() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::UpdateConfig {
            controller: None,
            strategy: Some(String::from("strategy")),
            invest_percentage: None,
            force_withdraw: None,
            lock_period: None,
        };

        let governance_info = mock_info("governance", &[]);

        execute(deps.as_mut(), mock_env(), governance_info, msg).unwrap();

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        execute_deposit(&mut deps, info);

        let info = mock_info("addr0000", &[]);

        let mut env = mock_env();
        add_block_by_seconds(&mut env, 620u64);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(70000000u128),
            }],
        )]);

        deps.querier
            .with_invested_balance(&Uint128::from(80000000u128));

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: Uint128::from(100000000u128),
            force_withdraw: false,
        };

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Insufficient"));
    }

    #[test]
    fn withdraw_fails_if_the_deposit_is_locked() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        execute_deposit(&mut deps, info);

        let info = mock_info("addr0000", &[]);

        let msg = ExecuteMsg::Withdraw {
            withdraw_amount: Uint128::from(100000000u128),
            force_withdraw: false,
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Still locked"));
    }

    #[test]
    fn deposit_updates_the_state_for_the_first_deposit() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};

        let env = mock_env();

        let res = execute(deps.as_mut(), env.clone(), info, deposit_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit"),
                attr("amount", Uint128::from(100000000u128)),
                attr("share", Uint128::from(100000000u128)),
                attr("maturity", (env.block.time.seconds() + 200u64).to_string()),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();

        let state: State = from_binary(&res).unwrap();

        assert_eq!(
            State {
                total_share: Uint128::from(100000000u128),
                total_subsidized: Uint128::from(0u128),
            },
            state
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::DepositInfo {
                addr: String::from("addr"),
            },
        )
        .unwrap();

        let deposit_info: DepositInfoResponse = from_binary(&res).unwrap();

        assert_eq!(
            DepositInfoResponse {
                share: Uint128::from(100000000u128),
                maturity: env.block.time.seconds() + 200u64,
                current_amount: Uint128::from(100000000u128),
                principal: Uint128::from(100000000u128),
                principal_claimed: Uint128::zero(),
                yield_amount: Uint128::zero(),
                yield_claimed: Uint128::zero(),
            },
            deposit_info
        );
    }

    #[test]
    fn deposit_updates_the_state_for_a_second_deposit() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_1_msg = ExecuteMsg::Deposit {};

        execute(deps.as_mut(), mock_env(), info, deposit_1_msg).unwrap();

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000 + 100000000u128),
            }],
        )]);

        let deposit_2_msg = ExecuteMsg::Deposit {};

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(50000000u128),
            }],
        );

        let env = mock_env();

        let res = execute(deps.as_mut(), mock_env(), info, deposit_2_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit"),
                attr("amount", Uint128::from(50000000u128)),
                attr("share", Uint128::from(33333333u128)),
                attr("maturity", (env.block.time.seconds() + 200u64).to_string()),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();

        let state: State = from_binary(&res).unwrap();

        assert_eq!(
            State {
                total_share: Uint128::from(133333333u128),
                total_subsidized: Uint128::zero(),
            },
            state
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::DepositInfo {
                addr: String::from("addr"),
            },
        )
        .unwrap();

        let deposit_info: DepositInfoResponse = from_binary(&res).unwrap();

        assert_eq!(
            DepositInfoResponse {
                principal: Uint128::from(150000000u128),
                current_amount: Uint128::from(150000000u128),
                share: Uint128::from(33333333u128),
                maturity: env.block.time.seconds() + 200u64,
                yield_amount: Uint128::zero(),
                yield_claimed: Uint128::zero(),
                principal_claimed: Uint128::zero(),
            },
            deposit_info
        );
    }

    #[test]
    fn deposit_check_deposit_amount() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::Deposit {};

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::zero(),
            }],
        );

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(
            res,
            StdError::generic_err("Deposit amount must be greater than 0")
        );
    }

    #[test]
    fn deposit_check_several_denoms() {
        let mut deps = dependencies_with_balance();

        instantiate_contract(&mut deps);

        let msg = ExecuteMsg::Deposit {};

        let info = mock_info(
            "addr",
            &[
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(10000000u64),
                },
                Coin {
                    denom: "uaud".to_string(),
                    amount: Uint128::from(10000000u64),
                },
            ],
        );

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Cannot deposit several denoms"));
    }

    fn instantiate_contract(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) {
        let msg = InstantiateMsg {
            controller: String::from("controller"),
            stable_denom: String::from("uusd"),
            invest_percentage: Decimal::percent(95u64),
            lock_period: 200u64,
        };

        let info = info_with_uusd();

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    fn execute_deposit(
        deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
        info: MessageInfo,
    ) {
        let msg = ExecuteMsg::Deposit {};

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    fn add_block_by_seconds(env: &mut Env, seconds: u64) {
        let new_block_time = env.block.time.plus_seconds(seconds);

        env.block.time = new_block_time;
    }

    fn dependencies_with_balance() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(150000000u128),
            }],
        )]);

        return deps;
    }

    fn info_with_uusd() -> MessageInfo {
        return mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );
    }
}
