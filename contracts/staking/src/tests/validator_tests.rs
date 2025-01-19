use crate::contract::execute;
use crate::msg::ExecuteMsg;
use crate::tests::test_helper::{init, CELESTIAVAL1, CELESTIAVAL3, OSMO3};
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{coins, Addr};

#[test]
fn proper_add_validator() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::AddValidator {
        new_validator: CELESTIAVAL3.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_ok());

    let attrs = res.unwrap().attributes;
    assert_eq!(attrs[0].value, "add_validator");
    assert_eq!(attrs[1].value, CELESTIAVAL3);
}

#[test]
fn duplicate_add_validator() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::AddValidator {
        new_validator: CELESTIAVAL1.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}
#[test]
fn proper_remove_validator() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::RemoveValidator {
        validator: CELESTIAVAL1.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_ok());

    let attrs = res.unwrap().attributes;
    assert_eq!(attrs[0].value, "remove_validator");
    assert_eq!(attrs[1].value, CELESTIAVAL1);
}

#[test]
fn invalid_remove_validator() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::RemoveValidator {
        validator: CELESTIAVAL3.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}
#[test]
fn non_admin_remove_validator() {
    let mut deps = init();
    let info = message_info(&deps.api.addr_make("bob"), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::RemoveValidator {
        validator: CELESTIAVAL1.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}
#[test]
fn non_admin_add_validator() {
    let mut deps = init();
    let info = message_info(&deps.api.addr_make("bob"), &coins(1000, "uosmo"));
    let msg = ExecuteMsg::AddValidator {
        new_validator: CELESTIAVAL3.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}
