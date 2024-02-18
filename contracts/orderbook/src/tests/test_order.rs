use crate::error::ContractError;
use crate::order::*;
use crate::orderbook::*;
use crate::state::*;
use crate::types::OrderDirection;
use cosmwasm_std::testing::{mock_dependencies_with_balances, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, Uint128};

struct PlaceLimitTestCase {
    name: &'static str,
    book_id: u64,
    tick_id: i64,
    quantity: Uint128,
    sent: Uint128,
    expected_error: Option<ContractError>,
}

#[test]
fn test_place_limit_variations() {
    let valid_book_id = 0;
    let invalid_book_id = valid_book_id + 1;
    let test_cases = vec![
        PlaceLimitTestCase {
            name: "valid order",
            book_id: valid_book_id,
            tick_id: 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            expected_error: None,
        },
        PlaceLimitTestCase {
            name: "invalid book id",
            book_id: invalid_book_id,
            tick_id: 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(1000),
            expected_error: Some(ContractError::InvalidBookId {
                book_id: invalid_book_id,
            }),
        },
        PlaceLimitTestCase {
            name: "invalid tick id",
            book_id: valid_book_id,
            tick_id: MAX_TICK + 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(1000),
            expected_error: Some(ContractError::InvalidTickId {
                tick_id: MAX_TICK + 1,
            }),
        },
        PlaceLimitTestCase {
            name: "invalid quantity",
            book_id: valid_book_id,
            tick_id: 1,
            quantity: Uint128::zero(),
            sent: Uint128::new(1000),
            expected_error: Some(ContractError::InvalidQuantity {
                quantity: Uint128::zero(),
            }),
        },
        PlaceLimitTestCase {
            name: "insufficient funds",
            book_id: valid_book_id,
            tick_id: 1,
            quantity: Uint128::new(1000),
            sent: Uint128::new(500),
            expected_error: Some(ContractError::InsufficientFunds {
                balance: Uint128::new(500),
                required: Uint128::new(1000),
            }),
        },
    ];

    for test in test_cases {
        let coin_vec = vec![coin(test.sent.u128(), "base")];
        let balances = [("creator", coin_vec.as_slice())];
        let mut deps = mock_dependencies_with_balances(&balances);
        let env = mock_env();
        let info = mock_info("creator", &coin_vec);
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        let _create_response = create_orderbook(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            quote_denom,
            base_denom,
        )
        .unwrap();

        let response = place_limit(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            test.book_id,
            test.tick_id,
            OrderDirection::Ask,
            test.quantity,
        );

        // Assert error if applicable
        if let Some(expected_error) = &test.expected_error {
            assert_eq!(response.unwrap_err(), *expected_error);

            // Verify that the order was not put in state
            let order_result = orders()
                .may_load(&deps.storage, &(test.book_id, test.tick_id, 0))
                .unwrap();
            assert!(
                order_result.is_none(),
                "Order should not exist in state for failed case: {}",
                test.name
            );
            return;
        }

        let response = response.unwrap();
        // Assertions on the response for a valid order
        assert_eq!(response.attributes[0], ("method", "placeLimit"));
        assert_eq!(response.attributes[1], ("owner", "creator"));
        assert_eq!(
            response.attributes[2],
            ("book_id", test.book_id.to_string())
        );
        assert_eq!(
            response.attributes[3],
            ("tick_id", test.tick_id.to_string())
        );
        assert_eq!(
            response.attributes[6],
            ("quantity", test.quantity.to_string())
        );

        // Retrieve the order from storage to verify it was saved correctly
        let expected_order_id = 0;
        let order = orders()
            .load(
                &deps.storage,
                &(test.book_id, test.tick_id, expected_order_id),
            )
            .unwrap();

        // Verify the order's fields
        assert_eq!(order.book_id, test.book_id);
        assert_eq!(order.tick_id, test.tick_id);
        assert_eq!(order.order_id, expected_order_id);
        assert_eq!(order.order_direction, OrderDirection::Ask);
        assert_eq!(order.owner, Addr::unchecked("creator"));
        assert_eq!(order.quantity, test.quantity);
    }
}
