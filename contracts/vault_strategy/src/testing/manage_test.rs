#[cfg(test)]
mod tests {
  use crate::contract::{execute, instantiate, query};
  use crate::testing::mock_querier::{mock_dependencies_with_querier, WasmMockQuerier};

  use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
  use cosmwasm_std::{
    attr, from_binary, BankMsg, Coin, CosmosMsg, Decimal, OwnedDeps, StdError, SubMsg, Uint128,
  };
  use athena::vault_strategy::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

  #[test]
  fn update_config_fails_if_unauthorized() {
    let mut deps = _instantiate_strategy();

    let msg = ExecuteMsg::UpdateConfig {
      controller: Some(String::from("controller2")),
      performance_fee: Some(Decimal::percent(10u64)),
    };

    let info = mock_info("addr", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized"));
  }

  #[test]
  fn update_config_fails_if_performance_fee_is_greater_than_100() {
    let mut deps = _instantiate_strategy();

    let msg = ExecuteMsg::UpdateConfig {
      controller: Some(String::from("controller2")),
      performance_fee: Some(Decimal::percent(101u64)),
    };
    let info = mock_info("governance", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
      res,
      StdError::generic_err("Performance fee percentage must be less than 100%")
    );
  }

  #[test]
  fn update_config() {
    let mut deps = _instantiate_strategy();
    // update governance
    let msg = ExecuteMsg::UpdateConfig {
      controller: Some(String::from("controller2")),
      performance_fee: Some(Decimal::percent(10u64)),
    };

    let info = mock_info("governance", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
      ConfigResponse {
        contract_addr: String::from(MOCK_CONTRACT_ADDR),
        controller: String::from("controller2"),
        vault: String::from("vault"),
        performance_fee: Decimal::percent(10u64),
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
  fn withdraw_to_vault_fails_if_unauthorized() {
    let mut deps = _instantiate_strategy();

    let msg = ExecuteMsg::WithdrawToVault {
      amount: Some(Uint128::from(100000000u64)),
    };

    deps.querier.with_balance(&[(
      &String::from(MOCK_CONTRACT_ADDR),
      &[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(150000000u64),
      }],
    )]);

    let info = mock_info("addr", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
      Err(StdError::GenericErr { msg }) => {
        if msg != "unauthorized" {
          panic!("Must return unauthorized error");
        }
      }
      _ => panic!("Must return unauthorized error"),
    }
  }

  #[test]
  fn withdraw_to_vault_fails_if_amount_is_zero() {
    let mut deps = _instantiate_strategy();

    let msg = ExecuteMsg::WithdrawToVault {
      amount: Some(Uint128::zero()),
    };

    deps.querier.with_balance(&[(
      &String::from(MOCK_CONTRACT_ADDR),
      &[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(150000000u64),
      }],
    )]);

    let info = mock_info("worker", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
      Err(StdError::GenericErr { msg }) => {
        if msg != "cannot withdraw zero" {
          panic!("Must return 'cannot withdraw zero' error");
        }
      }
      _ => panic!("Must return 'cannot withdraw zero' error"),
    }
  }

  #[test]
  fn withdraw_to_vault_fails_if_no_balance() {
    let mut deps = _instantiate_strategy();

    let msg = ExecuteMsg::WithdrawToVault { amount: None };

    deps.querier.with_balance(&[(
      &String::from(MOCK_CONTRACT_ADDR),
      &[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::zero(),
      }],
    )]);

    let info = mock_info("worker", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
      Err(StdError::GenericErr { msg }) => {
        if msg != "cannot withdraw zero" {
          panic!("Must return 'cannot withdraw zero' error");
        }
      }
      _ => panic!("Must return 'cannot withdraw zero' error"),
    }
  }

  #[test]
  fn withdraw_to_vault_by_worker() {
    let mut deps = _instantiate_strategy();

    let withdraw_amount = Uint128::from(100000000u64);
    let msg = ExecuteMsg::WithdrawToVault {
      amount: Some(withdraw_amount),
    };

    let info = mock_info("worker", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let amount_without_tax = deps.querier.deduct_tax(withdraw_amount).unwrap();
    assert_eq!(
      res.messages,
      vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: String::from("vault"),
        amount: vec![Coin {
          denom: "uusd".to_string(),
          amount: amount_without_tax,
        }],
      }),)]
    );
    assert_eq!(
      res.attributes,
      vec![
        attr("action", "withdraw_to_vault"),
        attr("amount", withdraw_amount),
      ]
    );
  }

  #[test]
  fn withdraw_to_vault_by_vault() {
    let mut deps = _instantiate_strategy();

    let withdraw_amount = Uint128::from(100000000u64);
    let msg = ExecuteMsg::WithdrawToVault {
      amount: Some(withdraw_amount),
    };

    let info = mock_info("vault", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let amount_without_tax = deps.querier.deduct_tax(withdraw_amount).unwrap();
    assert_eq!(
      res.messages,
      vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: String::from("vault"),
        amount: vec![Coin {
          denom: "uusd".to_string(),
          amount: amount_without_tax,
        }],
      }),)]
    );
    assert_eq!(
      res.attributes,
      vec![
        attr("action", "withdraw_to_vault"),
        attr("amount", withdraw_amount),
      ]
    );
  }

  #[test]
  fn withdraw_all_to_vault_if_amount_is_none() {
    let mut deps = _instantiate_strategy();

    let info = mock_info("worker", &[]);
    let msg = ExecuteMsg::WithdrawToVault { amount: None };

    let strategy_balance = Uint128::from(120000000u64);
    deps.querier.with_balance(&[(
      &String::from(MOCK_CONTRACT_ADDR),
      &[Coin {
        denom: "uusd".to_string(),
        amount: strategy_balance,
      }],
    )]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let amount_without_tax = deps.querier.deduct_tax(strategy_balance).unwrap();
    assert_eq!(
      res.messages,
      vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: String::from("vault"),
        amount: vec![Coin {
          denom: "uusd".to_string(),
          amount: amount_without_tax,
        }],
      }),)]
    );
    assert_eq!(
      res.attributes,
      vec![
        attr("action", "withdraw_to_vault"),
        attr("amount", strategy_balance),
      ]
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
