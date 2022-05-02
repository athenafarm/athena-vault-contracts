#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};

    use anchor_market::market::{
        Cw20HookMsg as AnchorMarketCw20HookMsg, ExecuteMsg as AnchorExecuteMsg,
    };
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        attr, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, OwnedDeps, StdError,
        SubMsg, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;
    use athena::asset::{Asset, AssetInfo};
    use athena::vault_strategy::{ExecuteMsg, InstantiateMsg, QueryMsg, State};
    use mirror_protocol::staking::{
        ExecuteMsg as MirrorStakingExecuteMsg,
        RewardInfoResponseItem as MirrorStakingRewardInfoResponseItem,
    };
    use terraswap::pair::Cw20HookMsg as TerraswapCw20HookMsg;
    use terraswap::pair::ExecuteMsg as TerraPairExecuteMsg;

    #[test]
    fn deposit_anchor_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::DepositAnchor {
            amount: Uint128::from(100000000u64),
        };

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn deposit_anchor_fails_if_amount_is_zero() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::DepositAnchor {
            amount: Uint128::zero(),
        };
        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("Amount must be greater than zero")
        );
    }

    #[test]
    fn deposit_anchor_by_worker() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);
        let msg = ExecuteMsg::DepositAnchor {
            amount: deposit_amount,
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("anchor_market"),
                funds: vec![Coin {
                    denom: "uusd".into(),
                    amount: deps.querier.deduct_tax(deposit_amount).unwrap(),
                }],
                msg: to_binary(&AnchorExecuteMsg::DepositStable {}).unwrap(),
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit_anchor"),
                attr("amount", deposit_amount),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: deposit_amount,
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn deposit_anchor_several_times() {
        let mut deps = _instantiate_strategy();

        let amount1 = Uint128::from(100000000u64);
        let amount2 = Uint128::from(50000000u64);

        _deposit_anchor(&mut deps, amount1);

        let msg = ExecuteMsg::DepositAnchor { amount: amount2 };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("anchor_market"),
                funds: vec![Coin {
                    denom: "uusd".into(),
                    amount: deps.querier.deduct_tax(amount2).unwrap(),
                }],
                msg: to_binary(&AnchorExecuteMsg::DepositStable {}).unwrap(),
            }))]
        );
        assert_eq!(
            res.attributes,
            vec![attr("action", "deposit_anchor"), attr("amount", amount2),]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: amount1 + amount2,
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn withdraw_anchor_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let withdraw_amount = Uint128::from(50000000u64);

        let aterra_balance = Uint128::from(80000000u64);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: Some(withdraw_amount),
        };

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn withdraw_anchor_fails_if_withdraw_amount_is_greater_than_balance() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let aterra_balance = Uint128::from(80000000u64);

        let withdraw_amount = Uint128::from(150000000u64);

        let exchange_rate = Decimal::percent(150u64);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: Some(withdraw_amount),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("cannot withdraw more than deposit")
        );
    }

    #[test]
    fn withdraw_anchor_by_worker() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let aterra_balance = Uint128::from(80000000u64);

        let withdraw_amount = Uint128::from(50000000u64);

        let exchange_rate = Decimal::percent(150u64);

        let original_deposited =
            deposit_amount * Decimal::from_ratio(withdraw_amount, aterra_balance);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let performance_fee = (withdraw_amount * exchange_rate)
            .checked_sub(original_deposited)
            .unwrap()
            * Decimal::percent(5u64);

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: Some(withdraw_amount),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("aterra_contract"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        contract: String::from("anchor_market"),
                        amount: withdraw_amount,
                        msg: to_binary(&AnchorMarketCw20HookMsg::RedeemStable {}).unwrap(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: String::from("treasury"),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: deps.querier.deduct_tax(performance_fee).unwrap(),
                    }],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_anchor"),
                attr("aterra_amount", withdraw_amount),
                attr("performance_fee", performance_fee),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: deposit_amount.checked_sub(original_deposited).unwrap(),
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn withdraw_anchor_by_itself() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let aterra_balance = Uint128::from(80000000u64);

        let withdraw_amount = Uint128::from(50000000u64);

        let exchange_rate = Decimal::percent(150u64);

        let original_deposited =
            deposit_amount * Decimal::from_ratio(withdraw_amount, aterra_balance);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let performance_fee = (withdraw_amount * exchange_rate)
            .checked_sub(original_deposited)
            .unwrap()
            * Decimal::percent(5u64);

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: Some(withdraw_amount),
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("aterra_contract"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        contract: String::from("anchor_market"),
                        amount: withdraw_amount,
                        msg: to_binary(&AnchorMarketCw20HookMsg::RedeemStable {}).unwrap(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: String::from("treasury"),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: deps.querier.deduct_tax(performance_fee).unwrap(),
                    }],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_anchor"),
                attr("aterra_amount", withdraw_amount),
                attr("performance_fee", performance_fee),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: deposit_amount.checked_sub(original_deposited).unwrap(),
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn withdraw_anchor_when_no_profit() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let aterra_balance = Uint128::from(80000000u64);

        let withdraw_amount = Uint128::from(50000000u64);

        let exchange_rate = Decimal::from_ratio(deposit_amount, aterra_balance);

        let original_deposited =
            deposit_amount * Decimal::from_ratio(withdraw_amount, aterra_balance);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: Some(withdraw_amount),
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("aterra_contract"),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: String::from("anchor_market"),
                    amount: withdraw_amount,
                    msg: to_binary(&AnchorMarketCw20HookMsg::RedeemStable {}).unwrap(),
                })
                .unwrap(),
            })),]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_anchor"),
                attr("aterra_amount", withdraw_amount),
                attr("performance_fee", Uint128::zero()),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: deposit_amount.checked_sub(original_deposited).unwrap(),
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn withdraw_anchor_all_if_amount_is_none() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(100000000u64);

        _deposit_anchor(&mut deps, deposit_amount);

        let aterra_balance = Uint128::from(80000000u64);

        let exchange_rate = Decimal::percent(150u64);

        deps.querier.with_token_info(
            &[(
                &String::from("aterra_contract"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
            )],
            &vec![],
            &vec![],
        );

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let performance_fee = (aterra_balance * exchange_rate)
            .checked_sub(deposit_amount)
            .unwrap()
            * Decimal::percent(5u64);

        let msg = ExecuteMsg::WithdrawAnchor {
            aterra_amount: None,
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("aterra_contract"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        contract: String::from("anchor_market"),
                        amount: aterra_balance,
                        msg: to_binary(&AnchorMarketCw20HookMsg::RedeemStable {}).unwrap(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: String::from("treasury"),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: deps.querier.deduct_tax(performance_fee).unwrap(),
                    }],
                })),
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_anchor"),
                attr("aterra_amount", aterra_balance),
                attr("performance_fee", performance_fee),
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();
        assert_eq!(
            State {
                anchor_deposited: Uint128::zero(),
                aterra_collateral: Uint128::zero()
            },
            state
        );
    }

    #[test]
    fn deposit_mirror_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::DepositMirror {
            amount: Uint128::from(100000000u64),
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn deposit_mirror_fails_if_amount_is_zero() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::DepositMirror {
            amount: Uint128::zero(),
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("Amount must be greater than zero")
        );
    }

    #[test]
    fn deposit_mirror_by_worker() {
        let mut deps = _instantiate_strategy();

        let deposit_amount = Uint128::from(10000000u64);

        deps.querier.with_mirror_asset(&[(
            &String::from("m_apple"),
            &[
                String::from("m_apple_pair"),
                String::from("m_liquidity_token"),
            ],
        )]);

        let msg = ExecuteMsg::DepositMirror {
            amount: deposit_amount,
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let half_amount = deps
            .querier
            .deduct_tax(deposit_amount * Decimal::percent(50u64))
            .unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("m_apple_pair"),
                    funds: vec![Coin {
                        denom: "uusd".into(),
                        amount: half_amount
                    }],
                    msg: to_binary(&TerraPairExecuteMsg::Swap {
                        belief_price: None,
                        max_spread: None,
                        to: None,
                        offer_asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uusd".into()
                            },
                            amount: half_amount,
                        }
                        .into(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::DepositMirrorHook {
                        mirror_asset_addr: String::from("m_apple")
                    })
                    .unwrap(),
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "deposit_mirror"),
                attr("amount", deposit_amount),
            ]
        );
    }

    #[test]
    fn deposit_mirror_hook_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::DepositMirrorHook {
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn deposit_mirror_hook_by_itself() {
        let mut deps = _instantiate_strategy();

        let mirror_token_balance = Uint128::from(10000000u64);
        let pool_mirror_balance = Uint128::from(10000000000000u64);
        let pool_stable_balance = Uint128::from(100000000000u64);

        deps.querier.with_mirror_asset(&[(
            &String::from("m_apple"),
            &[
                String::from("m_apple_pair"),
                String::from("m_liquidity_token"),
            ],
        )]);
        deps.querier.with_token_info(
            &[(
                &String::from("m_apple"),
                &[
                    (&String::from(MOCK_CONTRACT_ADDR), &mirror_token_balance),
                    (&String::from("m_apple_pair"), &pool_mirror_balance),
                ],
            )],
            &vec![],
            &vec![],
        );
        deps.querier.with_balance(&[(
            &String::from("m_apple_pair"),
            &[Coin {
                denom: "uusd".into(),
                amount: pool_stable_balance,
            }],
        )]);

        let required_stable_amount_for_lp =
            mirror_token_balance * Decimal::from_ratio(pool_stable_balance, pool_mirror_balance);
        let tax_deducted = deps
            .querier
            .deduct_tax(required_stable_amount_for_lp)
            .unwrap();

        let msg = ExecuteMsg::DepositMirrorHook {
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("m_apple"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                        spender: String::from("mirror_staking"),
                        amount: mirror_token_balance,
                        expires: None,
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_staking"),
                    funds: vec![Coin {
                        denom: "uusd".into(),
                        amount: tax_deducted,
                    }],
                    msg: to_binary(&MirrorStakingExecuteMsg::AutoStake {
                        assets: [
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".into(),
                                },
                                amount: tax_deducted,
                            }
                            .into(),
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: String::from("m_apple"),
                                },
                                amount: mirror_token_balance,
                            }
                            .into(),
                        ],
                        slippage_tolerance: None,
                    })
                    .unwrap()
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("denom_amount", tax_deducted),
                attr("mirror_token_amount", mirror_token_balance),
            ]
        );
    }

    #[test]
    fn withdraw_mirror_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::WithdrawMirror {
            mirror_lp_amount: Uint128::from(10000000u64),
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn withdraw_mirror_by_worker() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[(
            &String::from("m_apple"),
            &[
                String::from("m_apple_pair"),
                String::from("m_liquidity_token"),
            ],
        )]);

        let withdraw_lp_amount = Uint128::from(10000000u64);

        let msg = ExecuteMsg::WithdrawMirror {
            mirror_lp_amount: withdraw_lp_amount,
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_staking"),
                    funds: vec![],
                    msg: to_binary(&MirrorStakingExecuteMsg::Unbond {
                        asset_token: String::from("m_apple"),
                        amount: withdraw_lp_amount,
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("m_liquidity_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        amount: withdraw_lp_amount,
                        contract: String::from("m_apple_pair"),
                        msg: to_binary(&TerraswapCw20HookMsg::WithdrawLiquidity {}).unwrap(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawMirrorHook {
                        mirror_asset_addr: String::from("m_apple"),
                    })
                    .unwrap(),
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_mirror"),
                attr("amount", withdraw_lp_amount),
            ]
        );
    }

    #[test]
    fn withdraw_mirror_by_itself() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[(
            &String::from("m_apple"),
            &[
                String::from("m_apple_pair"),
                String::from("m_liquidity_token"),
            ],
        )]);

        let withdraw_lp_amount = Uint128::from(10000000u64);

        let msg = ExecuteMsg::WithdrawMirror {
            mirror_lp_amount: withdraw_lp_amount,
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_staking"),
                    funds: vec![],
                    msg: to_binary(&MirrorStakingExecuteMsg::Unbond {
                        asset_token: String::from("m_apple"),
                        amount: withdraw_lp_amount,
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("m_liquidity_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        amount: withdraw_lp_amount,
                        contract: String::from("m_apple_pair"),
                        msg: to_binary(&TerraswapCw20HookMsg::WithdrawLiquidity {}).unwrap(),
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawMirrorHook {
                        mirror_asset_addr: String::from("m_apple"),
                    })
                    .unwrap(),
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw_mirror"),
                attr("amount", withdraw_lp_amount),
            ]
        );
    }

    #[test]
    fn withdraw_mirror_hook_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::WithdrawMirrorHook {
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn withdraw_mirror_hook_by_itself() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[(
            &String::from("m_apple"),
            &[
                String::from("m_apple_pair"),
                String::from("m_liquidity_token"),
            ],
        )]);

        let mirror_token_balance = Uint128::from(10000000u64);
        deps.querier.with_token_info(
            &[(
                &String::from("m_apple"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &mirror_token_balance)],
            )],
            &vec![],
            &vec![],
        );

        let msg = ExecuteMsg::WithdrawMirrorHook {
            mirror_asset_addr: String::from("m_apple"),
        };

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("m_apple"),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    amount: mirror_token_balance,
                    contract: String::from("m_apple_pair"),
                    msg: to_binary(&TerraswapCw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    })
                    .unwrap()
                })
                .unwrap(),
            }))]
        );
        assert_eq!(res.attributes.len(), 0);
    }

    #[test]
    fn compound_mirror_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::CompoundMirror {};

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn compound_mirror_by_worker() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::CompoundMirror {};

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_staking"),
                    funds: vec![],
                    msg: to_binary(&MirrorStakingExecuteMsg::Withdraw { asset_token: None })
                        .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompoundMirrorHook {}).unwrap(),
                })),
            ]
        );
        assert_eq!(res.attributes, vec![attr("action", "compound_mirror"),]);
    }

    #[test]
    fn compound_mirror_by_itself() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::CompoundMirror {};

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_staking"),
                    funds: vec![],
                    msg: to_binary(&MirrorStakingExecuteMsg::Withdraw { asset_token: None })
                        .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompoundMirrorHook {}).unwrap(),
                })),
            ]
        );
        assert_eq!(res.attributes, vec![attr("action", "compound_mirror"),]);
    }

    #[test]
    fn compound_mirror_hook_fails_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::CompoundMirrorHook {};

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn compound_mirror_hook_by_itself() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[(
            &String::from("mirror_token"),
            &[
                String::from("ust_mir_pair"),
                String::from("ust_mir_liquidity_token"),
            ],
        )]);

        let mirror_balance = Uint128::from(10000000u64);
        let mirror_price =
            Decimal::from_ratio(Uint128::from(100000000u64), Uint128::from(10000000u64));
        deps.querier.with_token_info(
            &[(
                &String::from("mirror_token"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &mirror_balance)],
            )],
            &vec![],
            &[(&String::from("mirror_token"), &mirror_price)],
        );

        let return_amount = mirror_balance * mirror_price;
        let performance_fee = return_amount * Decimal::percent(5u64);

        let msg = ExecuteMsg::CompoundMirrorHook {};

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("mirror_token"),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        amount: mirror_balance,
                        contract: String::from("ust_mir_pair"),
                        msg: to_binary(&TerraswapCw20HookMsg::Swap {
                            belief_price: None,
                            max_spread: None,
                            to: None,
                        })
                        .unwrap()
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: String::from("treasury"),
                    amount: vec![Coin {
                        denom: "uusd".into(),
                        amount: deps.querier.deduct_tax(performance_fee.clone()).unwrap(),
                    },],
                }))
            ]
        );
        assert_eq!(
            res.attributes,
            vec![attr("performance_fee", performance_fee)]
        );
    }

    #[test]
    fn compound_mirror_hook_nothing_if_no_mir_balance() {
        let mut deps = _instantiate_strategy();

        let mirror_balance = Uint128::zero();
        let mirror_price =
            Decimal::from_ratio(Uint128::from(100000000u64), Uint128::from(10000000u64));

        deps.querier.with_mirror_asset(&[(
            &String::from("mirror_token"),
            &[
                String::from("ust_mir_pair"),
                String::from("ust_mir_liquidity_token"),
            ],
        )]);

        deps.querier.with_token_info(
            &[(
                &String::from("mirror_token"),
                &[(&String::from(MOCK_CONTRACT_ADDR), &mirror_balance)],
            )],
            &vec![],
            &[(&String::from("mirror_token"), &mirror_price)],
        );

        let msg = ExecuteMsg::CompoundMirrorHook {};

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.attributes.len(), 0);
    }

    #[test]
    fn withdraw_all_falls_if_unauthorized() {
        let mut deps = _instantiate_strategy();

        let msg = ExecuteMsg::WithdrawAll {};

        let info = mock_info("addr", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        assert_eq!(res, StdError::generic_err("unauthorized"));
    }

    #[test]
    fn withdraw_all_when_only_anchor_invested() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[
            (
                &String::from("m_apple"),
                &[
                    String::from("m_apple_pair"),
                    String::from("m_apple_liquidity_token"),
                ],
            ),
            (
                &String::from("m_google"),
                &[
                    String::from("m_google_pair"),
                    String::from("m_google_liquidity_token"),
                ],
            ),
            (
                &String::from("m_luna"),
                &[
                    String::from("m_luna_pair"),
                    String::from("m_luna_liquidity_token"),
                ],
            ),
        ]);

        let msg = ExecuteMsg::WithdrawAll {};

        let info = mock_info("worker", &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawAnchor {
                        aterra_amount: None
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompoundMirror {}).unwrap(),
                })),
            ]
        );
        assert_eq!(res.attributes, vec![attr("action", "withdraw_all"),]);
    }

    #[test]
    fn withdraw_all_both_anchor_and_mirror() {
        let mut deps = _instantiate_strategy();

        deps.querier.with_mirror_asset(&[
            (
                &String::from("m_apple"),
                &[
                    String::from("m_apple_pair"),
                    String::from("m_apple_liquidity_token"),
                ],
            ),
            (
                &String::from("m_google"),
                &[
                    String::from("m_google_pair"),
                    String::from("m_google_liquidity_token"),
                ],
            ),
            (
                &String::from("m_luna"),
                &[
                    String::from("m_luna_pair"),
                    String::from("m_luna_liquidity_token"),
                ],
            ),
        ]);

        deps.querier.with_reward_info(vec![
            MirrorStakingRewardInfoResponseItem {
                asset_token: String::from("m_apple"),
                bond_amount: Uint128::from(100000000u64),
                pending_reward: Uint128::from(100000u64),
                is_short: false,
            },
            MirrorStakingRewardInfoResponseItem {
                asset_token: String::from("m_google"),
                bond_amount: Uint128::from(110000000u64),
                pending_reward: Uint128::from(110000u64),
                is_short: false,
            },
            MirrorStakingRewardInfoResponseItem {
                asset_token: String::from("m_luna"),
                bond_amount: Uint128::from(120000000u64),
                pending_reward: Uint128::from(120000u64),
                is_short: false,
            },
        ]);

        let msg = ExecuteMsg::WithdrawAll {};

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawAnchor {
                        aterra_amount: None
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawMirror {
                        mirror_lp_amount: Uint128::from(100000000u64),
                        mirror_asset_addr: String::from("m_apple")
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawMirror {
                        mirror_lp_amount: Uint128::from(110000000u64),
                        mirror_asset_addr: String::from("m_google")
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawMirror {
                        mirror_lp_amount: Uint128::from(120000000u64),
                        mirror_asset_addr: String::from("m_luna")
                    })
                    .unwrap(),
                })),
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from(MOCK_CONTRACT_ADDR),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompoundMirror {}).unwrap(),
                })),
            ]
        );
        assert_eq!(res.attributes, vec![attr("action", "withdraw_all"),]);
    }

    // utils
    fn _instantiate_strategy() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        let msg = InstantiateMsg {
            controller: String::from("controller"),
            vault: String::from("vault"),
            performance_fee: Decimal::percent(5u64),
            stable_denom: String::from("uusd"),
            anchor_market: String::from("anchor_market"),
            mirror_token: String::from("mirror_token"),
            mirror_staking: String::from("mirror_staking"),
            mirror_mint: String::from("mirror_mint"),
            mirror_oracle: String::from("mirror_oracle"),
            terraswap_factory: String::from("terraswap_factory"),
        };

        let info = mock_info("addr", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        return deps;
    }

    fn _deposit_anchor(
        deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
        deposit_amount: Uint128,
    ) {
        let msg = ExecuteMsg::DepositAnchor {
            amount: deposit_amount,
        };

        let info = mock_info("worker", &[]);

        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
