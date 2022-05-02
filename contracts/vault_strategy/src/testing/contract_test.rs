#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};

    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Coin, Decimal, OwnedDeps, StdError, Uint128};
    use athena::vault_strategy::{ConfigResponse, InstantiateMsg, QueryMsg, State};
    use mirror_protocol::staking::RewardInfoResponseItem as MirrorStakingRewardInfoResponseItem;

    #[test]
    fn instantiate_strategy_fails_if_performace_fee_is_greater_than_100() {
        let mut deps = mock_dependencies_with_querier(20, &[]);

        let msg = InstantiateMsg {
            controller: String::from("controller"),
            vault: String::from("vault"),
            performance_fee: Decimal::percent(101u64),
            stable_denom: String::from("uusd"),
            anchor_market: String::from("anchor_market"),
            mirror_token: String::from("mirror_token"),
            mirror_staking: String::from("mirror_staking"),
            mirror_mint: String::from("mirror_mint"),
            mirror_oracle: String::from("mirror_oracle"),
            terraswap_factory: String::from("terraswap_factory"),
        };

        let info = mock_info("addr", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err("Performance fee percentage must be less than 100%")
        );
    }

    #[test]
    fn instantiate_strategy_with_correct_input() {
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

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();

        assert_eq!(
            ConfigResponse {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                controller: String::from("controller"),
                vault: String::from("vault"),
                performance_fee: Decimal::percent(5u64),
                stable_denom: String::from("uusd"),
                anchor_market: String::from("anchor_market"),
                aterra_contract: String::from("aterra_contract"),
                mirror_token: String::from("mirror_token"),
                mirror_staking: String::from("mirror_staking"),
                mirror_mint: String::from("mirror_mint"),
                mirror_oracle: String::from("mirror_oracle"),
                terraswap_factory: String::from("terraswap_factory"),
            },
            config
        );
    }

    #[test]
    fn query_initial_state() {
        let deps = _instantiate_strategy();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state: State = from_binary(&res).unwrap();

        assert_eq!(
            State {
                anchor_deposited: Uint128::zero(),
                aterra_collateral: Uint128::zero(),
            },
            state
        );
    }

    #[test]
    fn query_total_balance() {
        let mut deps = _instantiate_strategy();

        let m_asset_list = [
            String::from("m_apple"),
            String::from("m_google"),
            String::from("m_luna"),
        ];
        let m_asset_pair_list = [
            String::from("m_apple_pair"),
            String::from("m_google_pair"),
            String::from("m_luna_pair"),
        ];
        let m_liquidity_token_list = [
            String::from("m_apple_liquidity_token"),
            String::from("m_google_liquidity_token"),
            String::from("m_luna_liquidity_token"),
        ];
        let stable_liq_balance = [
            Uint128::from(1000000u64),
            Uint128::from(1200000u64),
            Uint128::from(1400000u64),
        ];
        let m_asset_liq_balance = [
            Uint128::from(10000000u64),
            Uint128::from(12000000u64),
            Uint128::from(14000000u64),
        ];
        let m_asset_liq_supply = [
            Uint128::from(1000000000u64),
            Uint128::from(1200000000u64),
            Uint128::from(1400000000u64),
        ];
        let m_asset_price = [
            Decimal::from_ratio(Uint128::from(500000000u64), Uint128::from(10000000u64)),
            Decimal::from_ratio(Uint128::from(300000000u64), Uint128::from(12000000u64)),
            Decimal::from_ratio(Uint128::from(7000000u64), Uint128::from(14000000u64)),
        ];
        let aterra_balance = Uint128::from(80000000u64);
        let exchange_rate = Decimal::percent(150u64);
        let unused_stable_balance = Uint128::from(100000000u64);

        deps.querier.with_mirror_asset(&[
            (
                &m_asset_list[0],
                &[
                    m_asset_pair_list[0].clone(),
                    m_liquidity_token_list[0].clone(),
                ],
            ),
            (
                &m_asset_list[1],
                &[
                    m_asset_pair_list[1].clone(),
                    m_liquidity_token_list[1].clone(),
                ],
            ),
            (
                &m_asset_list[2],
                &[
                    m_asset_pair_list[2].clone(),
                    m_liquidity_token_list[2].clone(),
                ],
            ),
        ]);

        deps.querier.with_token_info(
            &[
                (
                    &m_asset_list[0],
                    &[(&m_asset_pair_list[0], &m_asset_liq_balance[0])],
                ),
                (
                    &m_asset_list[1],
                    &[(&m_asset_pair_list[1], &m_asset_liq_balance[1])],
                ),
                (
                    &m_asset_list[2],
                    &[(&m_asset_pair_list[2], &m_asset_liq_balance[2])],
                ),
                (
                    &String::from("aterra_contract"),
                    &[(&String::from(MOCK_CONTRACT_ADDR), &aterra_balance)],
                ),
            ],
            &[
                (&m_liquidity_token_list[0], &m_asset_liq_supply[0]),
                (&m_liquidity_token_list[1], &m_asset_liq_supply[1]),
                (&m_liquidity_token_list[2], &m_asset_liq_supply[2]),
            ],
            &[
                (&m_asset_list[0], &m_asset_price[0]),
                (&m_asset_list[1], &m_asset_price[1]),
                (&m_asset_list[2], &m_asset_price[2]),
            ],
        );

        deps.querier.with_balance(&[
            (
                &m_asset_pair_list[0].to_string(),
                &[Coin {
                    denom: "uusd".into(),
                    amount: stable_liq_balance[0],
                }],
            ),
            (
                &m_asset_pair_list[1].to_string(),
                &[Coin {
                    denom: "uusd".into(),
                    amount: stable_liq_balance[1],
                }],
            ),
            (
                &m_asset_pair_list[2].to_string(),
                &[Coin {
                    denom: "uusd".into(),
                    amount: stable_liq_balance[2],
                }],
            ),
            (
                &MOCK_CONTRACT_ADDR.to_string(),
                &[Coin {
                    denom: "uusd".into(),
                    amount: unused_stable_balance,
                }],
            ),
        ]);

        let reward_infos: Vec<MirrorStakingRewardInfoResponseItem> = vec![
            MirrorStakingRewardInfoResponseItem {
                asset_token: m_asset_list[0].clone(),
                bond_amount: Uint128::from(100000000u64),
                pending_reward: Uint128::from(100000u64),
                is_short: false,
            },
            MirrorStakingRewardInfoResponseItem {
                asset_token: m_asset_list[1].clone(),
                bond_amount: Uint128::from(110000000u64),
                pending_reward: Uint128::from(110000u64),
                is_short: false,
            },
            MirrorStakingRewardInfoResponseItem {
                asset_token: m_asset_list[2].clone(),
                bond_amount: Uint128::from(120000000u64),
                pending_reward: Uint128::from(120000u64),
                is_short: false,
            },
        ];

        deps.querier.with_reward_info(reward_infos.clone());

        let mut mirror_invested = Uint128::zero();

        for reward_info in reward_infos {
            let index = m_asset_list
                .iter()
                .position(|r| r.clone() == reward_info.asset_token.clone())
                .unwrap();
            let m_asset_value_in_lp: Uint128 =
                m_asset_price[index].clone() * m_asset_liq_balance[index].clone();
            let lp_value = (m_asset_value_in_lp + stable_liq_balance[index].clone())
                * Decimal::from_ratio(reward_info.bond_amount, m_asset_liq_supply[index].clone());

            mirror_invested += lp_value;
        }

        deps.querier
            .with_anchor_epoch_state(exchange_rate, Uint128::zero());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::TotalBalance {}).unwrap();
        let total_balance: Uint128 = from_binary(&res).unwrap();

        assert_eq!(
            total_balance,
            unused_stable_balance + mirror_invested + aterra_balance * exchange_rate
        );
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
}
