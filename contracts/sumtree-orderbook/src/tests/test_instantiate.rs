use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
};

use super::{
    mock_querier::mock_dependencies_custom,
    test_constants::{BASE_DENOM, DEFAULT_SENDER, QUOTE_DENOM},
};
use crate::{contract::instantiate, msg::InstantiateMsg, ContractError};

struct InstantiateTestCase {
    name: &'static str,
    msg: InstantiateMsg,
    expected_error: Option<ContractError>,
}

#[test]
fn test_instantiate() {
    let test_cases = vec![
        InstantiateTestCase {
            name: "valid instantiate",
            msg: InstantiateMsg {
                quote_denom: QUOTE_DENOM.to_string(),
                base_denom: BASE_DENOM.to_string(),
            },
            expected_error: None,
        },
        InstantiateTestCase {
            name: "invalid instantiate",
            msg: InstantiateMsg {
                // Same denom for both quote and base
                quote_denom: QUOTE_DENOM.to_string(),
                base_denom: QUOTE_DENOM.to_string(),
            },
            expected_error: Some(ContractError::DuplicateDenoms {}),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(DEFAULT_SENDER, &[coin(100u128, BASE_DENOM)]);

        // -- System under test --
        let res = instantiate(deps.as_mut(), env, info, test.msg);

        // -- Post Test Assertions --
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );
            continue;
        }

        assert!(
            res.is_ok(),
            "{}: instantiate message unexpectedly failed; {}",
            test.name,
            res.unwrap_err()
        );
    }
}
