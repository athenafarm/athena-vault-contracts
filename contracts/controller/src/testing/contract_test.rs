#[cfg(test)]
mod tests {

  use crate::contract::{execute, instantiate, query};
  use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
  use cosmwasm_std::{attr, from_binary, StdError};

  use athena::controller::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UserRole};

  #[test]
  fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
      governance: String::from("governance"),
      treasury: String::from("treasury"),
    };

    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
      ConfigResponse {
        governance: String::from("governance"),
        treasury: String::from("treasury"),
      },
      config
    );

    let res = query(
      deps.as_ref(),
      mock_env(),
      QueryMsg::UserRole {
        user: String::from("governance"),
      },
    )
    .unwrap();
    let user_role: UserRole = from_binary(&res).unwrap();
    assert_eq!(
      user_role,
      UserRole {
        is_worker: true,
        is_claimer: true,
      }
    );
  }

  #[test]
  fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
      governance: String::from("governance"),
      treasury: String::from("treasury"),
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update governance
    let info = mock_info("governance", &[]);
    let msg = ExecuteMsg::UpdateConfig {
      governance: Some("governance2".to_string()),
      treasury: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
      ConfigResponse {
        governance: "governance2".to_string(),
        treasury: "treasury".to_string(),
      },
      config
    );

    // update treasury
    let info = mock_info("governance2", &[]);
    let msg = ExecuteMsg::UpdateConfig {
      governance: None,
      treasury: Some("treasury2".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
      ConfigResponse {
        governance: String::from("governance2"),
        treasury: String::from("treasury2"),
      },
      config
    );

    // update governance and treasury
    let info = mock_info("governance2", &[]);
    let msg = ExecuteMsg::UpdateConfig {
      governance: Some("governance3".to_string()),
      treasury: Some("treasury3".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
      ConfigResponse {
        governance: String::from("governance3"),
        treasury: String::from("treasury3"),
      },
      config
    );

    // unauthorized err
    let info = mock_info("governance", &[]);
    let msg = ExecuteMsg::UpdateConfig {
      governance: None,
      treasury: Some("treasury2".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    match res {
      Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
      _ => panic!("Must return unauthorized error"),
    }
  }

  #[test]
  fn test_update_role() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
      governance: String::from("governance"),
      treasury: String::from("treasury"),
    };

    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let msg = ExecuteMsg::UpdateRole {
      user: String::from("user"),
      is_worker: true,
      is_claimer: false,
    };

    // failed with unauthorized error
    let info = mock_info("addr", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

    match res {
      Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
      _ => panic!("Must return unauthorized error"),
    }

    let info = mock_info("governance", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    assert_eq!(
      res.attributes,
      vec![
        attr("action", "update_user_role"),
        attr("user", String::from("user")),
        attr("is_worker", "true"),
        attr("is_claimer", "false"),
      ]
    );

    let res = query(
      deps.as_ref(),
      mock_env(),
      QueryMsg::UserRole {
        user: String::from("user"),
      },
    )
    .unwrap();
    let user_role: UserRole = from_binary(&res).unwrap();
    assert_eq!(
      user_role,
      UserRole {
        is_worker: true,
        is_claimer: false,
      }
    );

    let msg = ExecuteMsg::UpdateRole {
      user: String::from("user1"),
      is_worker: true,
      is_claimer: true,
    };

    let info = mock_info("governance", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    assert_eq!(
      res.attributes,
      vec![
        attr("action", "update_user_role"),
        attr("user", "user1"),
        attr("is_worker", "true"),
        attr("is_claimer", "true"),
      ]
    );

    let res = query(
      deps.as_ref(),
      mock_env(),
      QueryMsg::UserRole {
        user: String::from("user1"),
      },
    )
    .unwrap();
    let user_role: UserRole = from_binary(&res).unwrap();
    assert_eq!(
      user_role,
      UserRole {
        is_worker: true,
        is_claimer: true,
      }
    );
  }
}
