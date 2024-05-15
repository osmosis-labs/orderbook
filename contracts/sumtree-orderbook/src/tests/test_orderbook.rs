use crate::{
    constants::{MAX_TICK, MIN_TICK},
    orderbook::*,
    state::ORDERBOOK,
    tests::{
        mock_querier::mock_dependencies_custom,
        test_constants::{BASE_DENOM, QUOTE_DENOM},
    },
    ContractError,
};

struct CreateOrderbookTestCase {
    name: &'static str,
    quote_denom: String,
    base_denom: String,
    expected_error: Option<ContractError>,
}

#[test]
fn test_create_orderbook() {
    let test_cases = vec![
        CreateOrderbookTestCase {
            name: "valid_orderbook",
            quote_denom: QUOTE_DENOM.to_string(),
            base_denom: BASE_DENOM.to_string(),
            expected_error: None,
        },
        CreateOrderbookTestCase {
            name: "invalid quote denom",
            quote_denom: "notadenom".to_string(),
            base_denom: BASE_DENOM.to_string(),
            expected_error: Some(ContractError::InvalidDenom {
                denom: "notadenom".to_string(),
            }),
        },
        CreateOrderbookTestCase {
            name: "invalid base denom",
            quote_denom: QUOTE_DENOM.to_string(),
            base_denom: "notadenom".to_string(),
            expected_error: Some(ContractError::InvalidDenom {
                denom: "notadenom".to_string(),
            }),
        },
        CreateOrderbookTestCase {
            name: "empty denom",
            quote_denom: QUOTE_DENOM.to_string(),
            base_denom: "".to_string(),
            expected_error: Some(ContractError::InvalidDenom {
                denom: "".to_string(),
            }),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();

        // -- System under test --
        let res = create_orderbook(
            deps.as_mut(),
            test.quote_denom.clone(),
            test.base_denom.clone(),
        );

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

        // Verify orderbook is saved correctly
        let orderbook = ORDERBOOK.load(deps.as_ref().storage).unwrap();
        assert_eq!(orderbook.quote_denom, test.quote_denom);
        assert_eq!(orderbook.base_denom, test.base_denom);
        assert_eq!(orderbook.current_tick, 0);
        assert_eq!(orderbook.next_bid_tick, MIN_TICK);
        assert_eq!(orderbook.next_ask_tick, MAX_TICK);
    }
}
