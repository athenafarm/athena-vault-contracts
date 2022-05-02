#[cfg(test)]
mod tests {

    use crate::contract::{execute, instantiate, query};
    use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        from_binary, BankMsg, Coin, CosmosMsg, Decimal, Env, OwnedDeps, StdError,
        SubMsg, Uint128,
    };
    use athena::vault::{ExecuteMsg, InstantiateMsg, QueryMsg, State};

    #[test]
    fn claim_principal() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000020u128),
            }],
        )]);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};
        
        execute(deps.as_mut(), mock_env(), info, deposit_msg).unwrap();

        let mut env = mock_env();
        let claimer = mock_info("addr", &[]);

        add_block_by_seconds(&mut env, 620u64);

        let claim_principal_msg = ExecuteMsg::ClaimPrincipal {};

        let res = execute(deps.as_mut(), env, claimer, claim_principal_msg).unwrap();
        
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: deps
                        .querier
                        .deduct_tax(Uint128::from(100000000u128))
                        .unwrap()
                }],
            }))]
        );
    }

    #[test]
    fn claim_principal_fails_if_still_locked() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(150000000u128),
            }],
        )]);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};
        
        execute(deps.as_mut(), mock_env(), info, deposit_msg).unwrap();

        let claimer = mock_info("addr", &[]);

        let claim_principal_msg = ExecuteMsg::ClaimPrincipal {};

        let res = execute(deps.as_mut(), mock_env(), claimer, claim_principal_msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Still locked"));
    }

    #[test]
    fn claim_yield() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000020u128),
            }],
        )]);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};
        
        execute(deps.as_mut(), mock_env(), info, deposit_msg).unwrap();

        let mut env = mock_env();
        let claimer = mock_info("addr", &[]);

        add_block_by_seconds(&mut env, 620u64);

        let claim_yield_msg = ExecuteMsg::ClaimYield {};

        let res = execute(deps.as_mut(), env, claimer, claim_yield_msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(20u128) - Uint128::from(1u128) // yield - fee
                }],
            }))]
        );
    }

    #[test]
    fn claim_yield_fails_if_still_locked() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(150000000u128),
            }],
        )]);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(100000000u128),
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};
        
        execute(deps.as_mut(), mock_env(), info, deposit_msg).unwrap();

        let claimer = mock_info("addr", &[]);

        let claim_yield_msg = ExecuteMsg::ClaimYield {};

        let res = execute(deps.as_mut(), mock_env(), claimer, claim_yield_msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("Still locked"));
    }

    #[test]
    fn claim_yield_updates_the_state_after_a_withdraw() {
        let principal = Uint128::from(100000000u128);
        let principal_yield = Uint128::from(20u128);
        let vault_balance = principal + principal_yield;

        let mut deps = mock_dependencies_with_querier(20, &[]);

        instantiate_contract(&mut deps);

        deps.querier.with_balance(&[(
            &String::from(MOCK_CONTRACT_ADDR),
            &[Coin {
                denom: "uusd".to_string(),
                amount: vault_balance,
            }],
        )]);

        let info = mock_info(
            "addr",
            &[Coin {
                denom: "uusd".to_string(),
                amount: principal,
            }],
        );

        let deposit_msg = ExecuteMsg::Deposit {};
        
        execute(deps.as_mut(), mock_env(), info, deposit_msg).unwrap();

        let mut env = mock_env();
        let claimer = mock_info("addr", &[]);

        add_block_by_seconds(&mut env, 620u64);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();

        assert_eq!(
            State {
                total_share: principal,
                total_subsidized: Uint128::zero(),
            },
            state
        );

        let withdraw_msg = ExecuteMsg::Withdraw {
            withdraw_amount: principal,
            force_withdraw: false,
        };

        let withdraw_response = execute(deps.as_mut(), env.clone(), mock_info("addr", &[]), withdraw_msg).unwrap();
        
        assert_eq!(
            withdraw_response.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: deps
                        .querier
                        .deduct_tax(principal)
                        .unwrap()
                }],
            }))]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        
        assert_eq!(
            State {
                total_share: Uint128::zero(),
                total_subsidized: principal_yield,
            },
            state
        );

        let claim_yield_msg = ExecuteMsg::ClaimYield {};

        let claim_yield_res = execute(deps.as_mut(), env, claimer, claim_yield_msg).unwrap();

        assert_eq!(
            claim_yield_res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("addr"),
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: principal_yield - Uint128::from(1u128) // yield - fee
                }],
            }))]
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

    fn instantiate_contract(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) {
        let msg = InstantiateMsg {
            controller: String::from("controller"),
            stable_denom: String::from("uusd"),
            invest_percentage: Decimal::percent(95u64),
            lock_period: 400u64,
        };

        let info = mock_info("addr", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    fn add_block_by_seconds(env: &mut Env, seconds: u64) {
        let new_block_time = env.block.time.plus_seconds(seconds);

        env.block.time = new_block_time;
    }
}
