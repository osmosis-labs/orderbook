use crate::{
    constants::{MAX_TICK, MIN_TICK},
    error::ContractError,
    order::*,
    orderbook::*,
    state::*,
    types::{Fulfillment, LimitOrder, MarketOrder, OrderDirection, REPLY_ID_REFUND},
};
use cosmwasm_std::testing::{mock_dependencies_with_balances, mock_env, mock_info};
use cosmwasm_std::{coin, Addr, BankMsg, Coin, Empty, SubMsg, Uint128};
use cw_utils::PaymentError;

#[allow(clippy::uninlined_format_args)]
fn format_test_name(name: &str) -> String {
    format!("\n\nTest case failed: {}\n", name)
}

struct PlaceLimitTestCase {
    name: &'static str,
    book_id: u64,
    tick_id: i64,
    quantity: Uint128,
    sent: Uint128,
    order_direction: OrderDirection,
    expected_error: Option<ContractError>,
}

#[test]
fn test_place_limit() {
    let valid_book_id = 0;
    let invalid_book_id = valid_book_id + 1;
    let test_cases = vec![
        PlaceLimitTestCase {
            name: "valid order with positive tick id",
            book_id: valid_book_id,
            tick_id: 10,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            order_direction: OrderDirection::Ask,
            expected_error: None,
        },
        PlaceLimitTestCase {
            name: "valid order with zero tick id",
            book_id: valid_book_id,
            tick_id: 0,
            quantity: Uint128::new(34321),
            sent: Uint128::new(34321),
            order_direction: OrderDirection::Bid,
            expected_error: None,
        },
        PlaceLimitTestCase {
            name: "valid order with negative tick id",
            book_id: valid_book_id,
            tick_id: -5,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            order_direction: OrderDirection::Bid,
            expected_error: None,
        },
        PlaceLimitTestCase {
            name: "valid order with large quantity",
            book_id: valid_book_id,
            tick_id: 3,
            quantity: Uint128::new(34321),
            sent: Uint128::new(34321),
            order_direction: OrderDirection::Ask,
            expected_error: None,
        },
        PlaceLimitTestCase {
            name: "invalid book id",
            book_id: invalid_book_id,
            tick_id: 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            order_direction: OrderDirection::Ask,
            expected_error: Some(ContractError::InvalidBookId {
                book_id: invalid_book_id,
            }),
        },
        PlaceLimitTestCase {
            name: "invalid tick id (max)",
            book_id: valid_book_id,
            tick_id: MAX_TICK + 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            order_direction: OrderDirection::Ask,
            expected_error: Some(ContractError::InvalidTickId {
                tick_id: MAX_TICK + 1,
            }),
        },
        PlaceLimitTestCase {
            name: "invalid tick id (min)",
            book_id: valid_book_id,
            tick_id: MIN_TICK - 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(100),
            order_direction: OrderDirection::Ask,
            expected_error: Some(ContractError::InvalidTickId {
                tick_id: MIN_TICK - 1,
            }),
        },
        PlaceLimitTestCase {
            name: "invalid quantity",
            book_id: valid_book_id,
            tick_id: 1,
            quantity: Uint128::zero(),
            sent: Uint128::new(1000),
            order_direction: OrderDirection::Ask,
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
            order_direction: OrderDirection::Ask,
            expected_error: Some(ContractError::InsufficientFunds {
                sent: Uint128::new(500),
                required: Uint128::new(1000),
            }),
        },
        PlaceLimitTestCase {
            name: "excessive funds",
            book_id: valid_book_id,
            tick_id: 1,
            quantity: Uint128::new(100),
            sent: Uint128::new(500),
            order_direction: OrderDirection::Ask,
            expected_error: Some(ContractError::InsufficientFunds {
                sent: Uint128::new(500),
                required: Uint128::new(100),
            }),
        },
    ];

    for test in test_cases {
        // --- Setup ---

        // Create a mock environment and info
        let coin_vec = vec![coin(
            test.sent.u128(),
            if test.order_direction == OrderDirection::Ask {
                "base"
            } else {
                "quote"
            },
        )];
        let balances = [("creator", coin_vec.as_slice())];
        let mut deps = mock_dependencies_with_balances(&balances);
        let env = mock_env();
        let info = mock_info("creator", &coin_vec);

        // Create an orderbook to operate on
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

        // --- System under test ---

        let response = place_limit(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            test.book_id,
            test.tick_id,
            test.order_direction,
            test.quantity,
        );

        // --- Assertions ---

        // Error case assertions if applicable
        if let Some(expected_error) = &test.expected_error {
            assert_eq!(
                response.unwrap_err(),
                *expected_error,
                "{}",
                format_test_name(test.name)
            );

            // Verify that the order was not put in state
            let order_result = orders()
                .may_load(&deps.storage, &(test.book_id, test.tick_id, 0))
                .unwrap();
            assert!(order_result.is_none(), "{}", format_test_name(test.name));

            // Verifiy liquidity was not updated
            let liquidity = TICK_LIQUIDITY
                .load(&deps.storage, &(test.book_id, test.tick_id))
                .unwrap_or_default();
            assert!(liquidity.is_zero(), "{}", format_test_name(test.name));
            continue;
        }

        // Assert no error and retrieve response contents
        let response = response.unwrap();

        // Assertions on the response for a valid order
        assert_eq!(
            response.attributes[0],
            ("method", "placeLimit"),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[1],
            ("owner", "creator"),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[2],
            ("book_id", test.book_id.to_string()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[3],
            ("tick_id", test.tick_id.to_string()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[6],
            ("quantity", test.quantity.to_string()),
            "{}",
            format_test_name(test.name)
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
        assert_eq!(
            order.book_id,
            test.book_id,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            order.tick_id,
            test.tick_id,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            order.order_id,
            expected_order_id,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            order.order_direction,
            test.order_direction,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            order.owner,
            Addr::unchecked("creator"),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            order.quantity,
            test.quantity,
            "{}",
            format_test_name(test.name)
        );

        // Validate liquidity updated as intended
        let liquidity = TICK_LIQUIDITY
            .load(&deps.storage, &(test.book_id, test.tick_id))
            .unwrap();
        assert_eq!(liquidity, test.quantity, "{}", format_test_name(test.name));
    }
}

struct CancelLimitTestCase {
    name: &'static str,
    book_id: u64,
    tick_id: i64,
    order_id: u64,
    order_direction: OrderDirection,
    quantity: Uint128,
    place_order: bool,
    expected_error: Option<ContractError>,
    owner: &'static str,
    sender: Option<&'static str>,
    sent: Vec<Coin>,
}

#[test]
fn test_cancel_limit() {
    let valid_book_id = 0;
    let test_cases = vec![
        CancelLimitTestCase {
            name: "valid order cancel",
            book_id: valid_book_id,
            tick_id: 1,
            order_id: 0,
            order_direction: OrderDirection::Ask,
            quantity: Uint128::from(100u128),
            place_order: true,
            expected_error: None,
            owner: "creator",
            sender: None,
            sent: vec![],
        },
        CancelLimitTestCase {
            name: "sent funds accidentally",
            book_id: valid_book_id,
            tick_id: 1,
            order_id: 0,
            order_direction: OrderDirection::Ask,
            quantity: Uint128::from(100u128),
            place_order: true,
            expected_error: Some(ContractError::PaymentError(PaymentError::NonPayable {})),
            owner: "creator",
            sender: None,
            sent: vec![coin(100, "quote")],
        },
        CancelLimitTestCase {
            name: "unauthorized cancel (not owner)",
            book_id: valid_book_id,
            tick_id: 1,
            order_id: 0,
            order_direction: OrderDirection::Ask,
            quantity: Uint128::from(100u128),
            place_order: true,
            expected_error: Some(ContractError::Unauthorized {}),
            owner: "creator",
            sender: Some("malicious_user"),
            sent: vec![],
        },
        CancelLimitTestCase {
            name: "order not found",
            book_id: valid_book_id,
            tick_id: 1,
            order_id: 0,
            order_direction: OrderDirection::Ask,
            quantity: Uint128::from(100u128),
            place_order: false,
            expected_error: Some(ContractError::OrderNotFound {
                book_id: valid_book_id,
                tick_id: 1,
                order_id: 0,
            }),
            owner: "creator",
            sender: None,
            sent: vec![],
        },
    ];

    for test in test_cases {
        // --- Setup ---

        // Create a mock environment and info
        let balances = [(test.owner, test.sent.as_slice())];
        let mut deps = mock_dependencies_with_balances(&balances);
        let env = mock_env();
        let info = mock_info(test.sender.unwrap_or(test.owner), test.sent.as_slice());

        // Create an orderbook to operate on
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        create_orderbook(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            quote_denom.clone(),
            base_denom.clone(),
        )
        .unwrap();

        if test.place_order {
            orders()
                .save(
                    deps.as_mut().storage,
                    &(test.book_id, test.tick_id, test.order_id),
                    &LimitOrder::new(
                        test.book_id,
                        test.tick_id,
                        test.order_id,
                        test.order_direction,
                        Addr::unchecked(test.owner),
                        test.quantity,
                    ),
                )
                .unwrap();
            // Update tick liquidity
            TICK_LIQUIDITY
                .update(
                    deps.as_mut().storage,
                    &(test.book_id, test.tick_id),
                    |liquidity| {
                        Ok::<Uint128, ContractError>(
                            liquidity.unwrap_or_default().checked_add(test.quantity)?,
                        )
                    },
                )
                .unwrap();
        }

        // --- System under test ---

        let response = cancel_limit(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            test.book_id,
            test.tick_id,
            test.order_id,
        );

        // --- Assertions ---

        // Error case assertions if applicable
        if let Some(expected_error) = &test.expected_error {
            assert_eq!(
                response.unwrap_err(),
                *expected_error,
                "{}",
                format_test_name(test.name)
            );

            // Verify that the order was not put in state
            let order_result = orders()
                .may_load(&deps.storage, &(test.book_id, test.tick_id, test.order_id))
                .unwrap();
            assert!(
                order_result.is_some() == test.place_order,
                "{}",
                format_test_name(test.name)
            );

            // Verify Liqudity was updated as intended
            let liquidity = TICK_LIQUIDITY
                .load(deps.as_ref().storage, &(test.book_id, test.tick_id))
                .unwrap_or_default();
            if test.place_order {
                assert_eq!(liquidity, test.quantity, "{}", format_test_name(test.name));
            } else {
                assert!(liquidity.is_zero(), "{}", format_test_name(test.name));
            }
            continue;
        }

        // Assert no error and retrieve response contents
        let response = response.unwrap();
        let refund_denom = match test.order_direction {
            OrderDirection::Bid => quote_denom.clone(),
            OrderDirection::Ask => base_denom.clone(),
        };
        let expected_refund_msg: SubMsg<Empty> = SubMsg::reply_on_error(
            BankMsg::Send {
                to_address: test.owner.to_string(),
                amount: vec![coin(test.quantity.u128(), refund_denom)],
            },
            REPLY_ID_REFUND,
        );

        // Assertions on the response for a valid order
        assert_eq!(
            response.attributes[0],
            ("method", "cancelLimit"),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[1],
            ("owner", test.owner),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[2],
            ("book_id", test.book_id.to_string()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[3],
            ("tick_id", test.tick_id.to_string()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.attributes[4],
            ("order_id", test.order_id.to_string()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.messages.len(),
            1,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            response.messages[0],
            expected_refund_msg,
            "{}",
            format_test_name(test.name)
        );

        // Retrieve the order from storage to verify it was saved correctly
        let expected_order_id = 0;
        let order = orders()
            .may_load(
                &deps.storage,
                &(test.book_id, test.tick_id, expected_order_id),
            )
            .unwrap();

        // Verify the order's fields
        assert!(order.is_none(), "{}", format_test_name(test.name));

        // Validate liquidity updated as intended
        let liquidity = TICK_LIQUIDITY
            .load(deps.as_ref().storage, &(test.book_id, test.tick_id))
            .unwrap_or_default();

        assert!(liquidity.is_zero(), "{}", format_test_name(test.name));
    }
}

struct ResolveFulfillmentsTestCase {
    pub name: &'static str,
    pub book_id: u64,
    /// bool represents if order is removed
    pub fulfillments: Vec<(Fulfillment, bool)>,
    // (tick_id, liquidity)
    pub expected_liquidity: Vec<(i64, Uint128)>,
    pub expected_error: Option<ContractError>,
}

#[test]
fn test_resolve_fulfillments() {
    let valid_book_id = 0;
    let test_cases: Vec<ResolveFulfillmentsTestCase> = vec![
        ResolveFulfillmentsTestCase {
            name: "standard fulfillments (single tick) ",
            book_id: valid_book_id,
            fulfillments: vec![
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            1,
                            0,
                            OrderDirection::Ask,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            1,
                            1,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(50u128),
                    ),
                    false,
                ),
            ],
            expected_liquidity: vec![(1, Uint128::from(50u128))],
            expected_error: None,
        },
        ResolveFulfillmentsTestCase {
            name: "standard fulfillments (multi tick)",
            book_id: valid_book_id,
            fulfillments: vec![
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            1,
                            0,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            1,
                            1,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            2,
                            3,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            2,
                            4,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(50u128),
                    ),
                    false,
                ),
            ],
            expected_liquidity: vec![(1, Uint128::zero()), (2, Uint128::from(50u128))],
            expected_error: None,
        },
        ResolveFulfillmentsTestCase {
            name: "Wrong order book",
            book_id: valid_book_id,
            fulfillments: vec![
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            0,
                            1,
                            0,
                            OrderDirection::Ask,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
                (
                    Fulfillment::new(
                        LimitOrder::new(
                            1,
                            1,
                            1,
                            OrderDirection::Bid,
                            Addr::unchecked("creator"),
                            Uint128::from(100u128),
                        ),
                        Uint128::from(100u128),
                    ),
                    true,
                ),
            ],
            expected_liquidity: vec![(1, Uint128::zero())],
            expected_error: Some(ContractError::InvalidFulfillment {
                order_id: 1,
                book_id: 1,
                amount_required: Uint128::from(100u128),
                amount_remaining: Uint128::from(100u128),
                reason: Some("Fulfillment is part of another order book".to_string()),
            }),
        },
        ResolveFulfillmentsTestCase {
            name: "Invalid fulfillment (insufficient funds)",
            book_id: valid_book_id,
            fulfillments: vec![(
                Fulfillment::new(
                    LimitOrder::new(
                        0,
                        0,
                        0,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(100u128),
                    ),
                    Uint128::from(200u128),
                ),
                true,
            )],
            expected_liquidity: vec![(1, Uint128::zero())],
            expected_error: Some(ContractError::InvalidFulfillment {
                order_id: 0,
                book_id: 0,
                amount_required: Uint128::from(200u128),
                amount_remaining: Uint128::from(100u128),
                reason: Some("Order does not have enough funds".to_string()),
            }),
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies_with_balances(&[]);
        let env = mock_env();
        let info = mock_info("maker", &[]);

        // Create an orderbook to operate on
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        create_orderbook(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            quote_denom.clone(),
            base_denom.clone(),
        )
        .unwrap();

        let fulfillments = test
            .fulfillments
            .iter()
            .map(|f| f.clone().0)
            .collect::<Vec<Fulfillment>>();

        // Add orders to state
        for Fulfillment { order, .. } in fulfillments.clone() {
            orders()
                .save(
                    deps.as_mut().storage,
                    &(order.book_id, order.tick_id, order.order_id),
                    &order,
                )
                .unwrap();
            TICK_LIQUIDITY
                .update(
                    deps.as_mut().storage,
                    &(order.book_id, order.tick_id),
                    |l| {
                        Ok::<Uint128, ContractError>(
                            l.unwrap_or_default().checked_add(order.quantity).unwrap(),
                        )
                    },
                )
                .unwrap();
        }

        let response = resolve_fulfillments(deps.as_mut().storage, fulfillments);

        // -- POST STATE --

        if let Some(expected_error) = &test.expected_error {
            let err = response.unwrap_err();
            assert_eq!(err, *expected_error, "{}", format_test_name(test.name));
            // NOTE: We cannot check if orders/tick liquidity were unaltered as changes are made in a for loop that is not rolled back upon error

            continue;
        }

        // Check tick liquidity updated as expected
        for (tick_id, expected_liquidity) in test.expected_liquidity {
            let liquidity = TICK_LIQUIDITY
                .may_load(deps.as_ref().storage, &(test.book_id, tick_id))
                .unwrap();
            assert_eq!(
                liquidity.is_none(),
                expected_liquidity.is_zero(),
                "{}",
                format_test_name(test.name)
            );
            if let Some(post_liquidity) = liquidity {
                assert_eq!(
                    post_liquidity,
                    expected_liquidity,
                    "{}",
                    format_test_name(test.name)
                );
            }
        }

        let orderbook = ORDERBOOKS
            .load(deps.as_ref().storage, &valid_book_id)
            .unwrap();

        let response = response.unwrap();

        for (idx, (Fulfillment { order, amount }, removed)) in test.fulfillments.iter().enumerate()
        {
            let saved_order = orders()
                .may_load(
                    deps.as_ref().storage,
                    &(order.book_id, order.tick_id, order.order_id),
                )
                .unwrap();
            // Check order is updated as expected
            assert_eq!(
                saved_order.is_none(),
                *removed,
                "{}",
                format_test_name(test.name)
            );
            // If not removed check quantity updated
            if !removed {
                assert_eq!(
                    saved_order.unwrap().quantity,
                    order.quantity.checked_sub(*amount).unwrap(),
                    "{}",
                    format_test_name(test.name)
                );
            }

            // Check message is generated as expected
            let mut order = order.clone();
            let denom = orderbook.get_expected_denom(&order.order_direction);
            let msg = order.fill(denom, *amount).unwrap();

            assert_eq!(response[idx], msg, "{}", format_test_name(test.name));
        }
    }
}

struct RunMarketOrderTestCase {
    pub name: &'static str,
    pub placed_order: MarketOrder,
    pub tick_bound: Option<i64>,
    pub extra_orders: Vec<LimitOrder>,
    pub expected_fulfillments: Vec<Fulfillment>,
    pub expected_remainder: Uint128,
    pub expected_error: Option<ContractError>,
}

#[test]
fn test_run_market_order() {
    let valid_book_id = 0;
    let test_cases: Vec<RunMarketOrderTestCase> = vec![
        RunMarketOrderTestCase {
            name: "standard market order (single tick) ASK",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -1,
                        0,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -1,
                        1,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(50u128),
                ),
            ],
            expected_remainder: Uint128::zero(),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (multi tick) ASK",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -1,
                        0,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -2,
                        1,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(50u128),
                ),
            ],
            expected_remainder: Uint128::zero(),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "excessive market order (single tick) ASK",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(1000u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -1,
                        0,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        -2,
                        1,
                        OrderDirection::Bid,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(150u128),
                ),
            ],
            expected_remainder: Uint128::from(800u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (no tick) ASK",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(1000u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![],
            expected_remainder: Uint128::from(1000u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (multi tick - bound) ASK",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: Some(-1),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                -2,
                1,
                OrderDirection::Bid,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    -1,
                    0,
                    OrderDirection::Bid,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "invalid ASK tick bound",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Ask,
                Addr::unchecked("creator"),
            ),
            tick_bound: Some(1),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                -2,
                1,
                OrderDirection::Bid,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    -1,
                    0,
                    OrderDirection::Bid,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: Some(ContractError::InvalidTickId { tick_id: 1 }),
        },
        RunMarketOrderTestCase {
            name: "standard market order (single tick) BID",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        1,
                        0,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        1,
                        1,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(50u128),
                ),
            ],
            expected_remainder: Uint128::zero(),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (multi tick) BID",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        1,
                        0,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        2,
                        1,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(50u128),
                ),
            ],
            expected_remainder: Uint128::zero(),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "excessive market order (single tick) BID",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(1000u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        1,
                        0,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(50u128),
                    ),
                    Uint128::from(50u128),
                ),
                Fulfillment::new(
                    LimitOrder::new(
                        valid_book_id,
                        2,
                        1,
                        OrderDirection::Ask,
                        Addr::unchecked("creator"),
                        Uint128::from(150u128),
                    ),
                    Uint128::from(150u128),
                ),
            ],
            expected_remainder: Uint128::from(800u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (no tick) BID",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(1000u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            tick_bound: None,
            extra_orders: vec![],
            expected_fulfillments: vec![],
            expected_remainder: Uint128::from(1000u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "standard market order (multi tick - bound) BID",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                2,
                1,
                OrderDirection::Ask,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            tick_bound: Some(1),
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    1,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "invalid BID tick bound",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                2,
                1,
                OrderDirection::Ask,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            tick_bound: Some(0),
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    1,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: Some(ContractError::InvalidTickId { tick_id: 0 }),
        },
        RunMarketOrderTestCase {
            name: "tick too large",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                2,
                1,
                OrderDirection::Ask,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            tick_bound: Some(MAX_TICK + 1),
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    1,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: Some(ContractError::InvalidTickId {
                tick_id: MAX_TICK + 1,
            }),
        },
        RunMarketOrderTestCase {
            name: "tick too small",
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::from(100u128),
                OrderDirection::Bid,
                Addr::unchecked("creator"),
            ),
            extra_orders: vec![LimitOrder::new(
                valid_book_id,
                2,
                1,
                OrderDirection::Ask,
                Addr::unchecked("creator"),
                Uint128::from(150u128),
            )],
            tick_bound: Some(MIN_TICK - 1),
            expected_fulfillments: vec![Fulfillment::new(
                LimitOrder::new(
                    valid_book_id,
                    1,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("creator"),
                    Uint128::from(50u128),
                ),
                Uint128::from(50u128),
            )],
            expected_remainder: Uint128::from(50u128),
            expected_error: Some(ContractError::InvalidTickId {
                tick_id: MIN_TICK - 1,
            }),
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies_with_balances(&[]);
        let env = mock_env();
        let info = mock_info("maker", &[]);

        // Create an orderbook to operate on
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        create_orderbook(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            quote_denom.clone(),
            base_denom.clone(),
        )
        .unwrap();

        let fulfillments = test.expected_fulfillments.to_vec();
        let mut all_orders: Vec<LimitOrder> = fulfillments
            .iter()
            .map(|Fulfillment { order, .. }| order.clone())
            .collect();
        all_orders.extend(test.extra_orders);

        // Add orders to state
        for order in all_orders.clone() {
            orders()
                .save(
                    deps.as_mut().storage,
                    &(order.book_id, order.tick_id, order.order_id),
                    &order,
                )
                .unwrap();
            TICK_LIQUIDITY
                .update(
                    deps.as_mut().storage,
                    &(order.book_id, order.tick_id),
                    |l| {
                        Ok::<Uint128, ContractError>(
                            l.unwrap_or_default().checked_add(order.quantity).unwrap(),
                        )
                    },
                )
                .unwrap();

            let mut orderbook = ORDERBOOKS
                .load(deps.as_ref().storage, &valid_book_id)
                .unwrap();
            match order.order_direction {
                OrderDirection::Ask => {
                    if order.tick_id < orderbook.next_ask_tick {
                        orderbook.next_ask_tick = order.tick_id;
                    }
                    ORDERBOOKS
                        .save(deps.as_mut().storage, &valid_book_id, &orderbook)
                        .unwrap();
                }
                OrderDirection::Bid => {
                    if order.tick_id > orderbook.next_bid_tick {
                        orderbook.next_bid_tick = order.tick_id;
                    }
                    ORDERBOOKS
                        .save(deps.as_mut().storage, &valid_book_id, &orderbook)
                        .unwrap();
                }
            }
        }

        let mut market_order = test.placed_order.clone();
        let response = run_market_order(deps.as_mut().storage, &mut market_order, test.tick_bound);

        // -- POST STATE --

        if let Some(expected_error) = &test.expected_error {
            let err = response.unwrap_err();
            assert_eq!(err, *expected_error, "{}", format_test_name(test.name));

            continue;
        }

        let response = response.unwrap();

        for (idx, fulfillment) in test.expected_fulfillments.iter().enumerate() {
            // Check fulfillment is generated as expected
            assert_eq!(
                response.0[idx],
                *fulfillment,
                "{}",
                format_test_name(test.name)
            );
        }

        assert_eq!(
            market_order.quantity,
            test.expected_remainder,
            "{}",
            format_test_name(test.name)
        );
    }
}

// TODO: Merge in to place limit test cases and remove
// struct RunLimitOrderTestCase {
//     pub name: &'static str,
//     pub order: LimitOrder,
//     pub expected_fulfillments: Vec<Fulfillment>,
//     pub expected_bank_msgs: Vec<BankMsg>,
//     pub expected_liquidity: Vec<(i64, Uint128)>,
//     pub expected_remainder: Uint128,
//     pub expected_error: Option<ContractError>,
// }

// #[test]
// fn test_run_limit_order() {
//     let valid_book_id = 0;
//     let test_cases: Vec<RunLimitOrderTestCase> = vec![
//         RunLimitOrderTestCase {
//             name: "run limit order with single fulfillment ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -1,
//                 0,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(50u128),
//             ),
//             expected_fulfillments: vec![Fulfillment::new(
//                 LimitOrder::new(
//                     valid_book_id,
//                     -1,
//                     0,
//                     OrderDirection::Bid,
//                     Addr::unchecked("maker"),
//                     Uint128::from(50u128),
//                 ),
//                 Uint128::from(50u128),
//             )],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker".to_string(),
//                     amount: vec![coin(50, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(50, "base")],
//                 },
//             ],
//             expected_liquidity: vec![(-1, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -1,
//                 0,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         0,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(75u128),
//                     ),
//                     Uint128::from(75u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "base")],
//                 },
//             ],
//             expected_liquidity: vec![(-1, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments across multiple ticks ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -3,
//                 2,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         0,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -2,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(75u128),
//                     ),
//                     Uint128::from(75u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "base")],
//                 },
//             ],
//             expected_liquidity: vec![(-1, Uint128::zero()), (-2, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments w/ partial ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -1,
//                 0,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         0,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "base")],
//                 },
//             ],
//             expected_liquidity: vec![(-1, Uint128::from(75u128))],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments w/ remainder ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -1,
//                 0,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(1000u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         0,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(150u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(150, "quote")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(175, "base")],
//                 },
//             ],
//             expected_liquidity: vec![(-1, Uint128::zero())],
//             expected_remainder: Uint128::from(825u128),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "invalid tick ASK",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Ask,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         0,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         -1,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![],
//             expected_liquidity: vec![],
//             expected_remainder: Uint128::zero(),
//             expected_error: Some(ContractError::InvalidTickId { tick_id: 1 }),
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with single fulfillment BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(50u128),
//             ),
//             expected_fulfillments: vec![Fulfillment::new(
//                 LimitOrder::new(
//                     valid_book_id,
//                     1,
//                     0,
//                     OrderDirection::Ask,
//                     Addr::unchecked("maker"),
//                     Uint128::from(50u128),
//                 ),
//                 Uint128::from(50u128),
//             )],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker".to_string(),
//                     amount: vec![coin(50, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(50, "quote")],
//                 },
//             ],
//             expected_liquidity: vec![(1, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(75u128),
//                     ),
//                     Uint128::from(75u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "quote")],
//                 },
//             ],
//             expected_liquidity: vec![(1, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments across multiple ticks BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 3,
//                 2,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         2,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(75u128),
//                     ),
//                     Uint128::from(75u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "quote")],
//                 },
//             ],
//             expected_liquidity: vec![(1, Uint128::zero()), (2, Uint128::zero())],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments w/ partial BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(75, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(100, "quote")],
//                 },
//             ],
//             expected_liquidity: vec![(1, Uint128::from(75u128))],
//             expected_remainder: Uint128::zero(),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "run limit order with multiple fulfillments w/ remainder BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(1000u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(150u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![
//                 BankMsg::Send {
//                     to_address: "maker1".to_string(),
//                     amount: vec![coin(25, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "maker2".to_string(),
//                     amount: vec![coin(150, "base")],
//                 },
//                 BankMsg::Send {
//                     to_address: "creator".to_string(),
//                     amount: vec![coin(175, "quote")],
//                 },
//             ],
//             expected_liquidity: vec![(1, Uint128::zero())],
//             expected_remainder: Uint128::from(825u128),
//             expected_error: None,
//         },
//         RunLimitOrderTestCase {
//             name: "invalid tick BID",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 -1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![],
//             expected_liquidity: vec![],
//             expected_remainder: Uint128::zero(),
//             expected_error: Some(ContractError::InvalidTickId { tick_id: -1 }),
//         },
//         RunLimitOrderTestCase {
//             name: "mismatched order direction",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Bid,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![],
//             expected_liquidity: vec![],
//             expected_remainder: Uint128::zero(),
//             expected_error: Some(ContractError::MismatchedOrderDirection {}),
//         },
//         RunLimitOrderTestCase {
//             name: "tick too large",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 MAX_TICK + 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![],
//             expected_liquidity: vec![],
//             expected_remainder: Uint128::zero(),
//             expected_error: Some(ContractError::InvalidTickId {
//                 tick_id: MAX_TICK + 1,
//             }),
//         },
//         RunLimitOrderTestCase {
//             name: "tick too small",
//             order: LimitOrder::new(
//                 valid_book_id,
//                 MIN_TICK - 1,
//                 0,
//                 OrderDirection::Bid,
//                 Addr::unchecked("creator"),
//                 Uint128::from(100u128),
//             ),
//             expected_fulfillments: vec![
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         0,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker1"),
//                         Uint128::from(25u128),
//                     ),
//                     Uint128::from(25u128),
//                 ),
//                 Fulfillment::new(
//                     LimitOrder::new(
//                         valid_book_id,
//                         1,
//                         1,
//                         OrderDirection::Ask,
//                         Addr::unchecked("maker2"),
//                         Uint128::from(150u128),
//                     ),
//                     Uint128::from(50u128),
//                 ),
//             ],
//             expected_bank_msgs: vec![],
//             expected_liquidity: vec![],
//             expected_remainder: Uint128::zero(),
//             expected_error: Some(ContractError::InvalidTickId {
//                 tick_id: MIN_TICK - 1,
//             }),
//         },
//     ];

//     for test in test_cases {
//         let mut deps = mock_dependencies_with_balances(&[]);
//         let env = mock_env();
//         let info = mock_info("maker", &[]);

//         // Create an orderbook to operate on
//         let quote_denom = "quote".to_string();
//         let base_denom = "base".to_string();
//         create_orderbook(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             quote_denom.clone(),
//             base_denom.clone(),
//         )
//         .unwrap();

//         let fulfillments = test.expected_fulfillments.to_vec();
//         let all_orders: Vec<LimitOrder> = fulfillments
//             .iter()
//             .map(|Fulfillment { order, .. }| order.clone())
//             .collect();

//         // Add orders to state
//         for order in all_orders.clone() {
//             orders()
//                 .save(
//                     deps.as_mut().storage,
//                     &(order.book_id, order.tick_id, order.order_id),
//                     &order,
//                 )
//                 .unwrap();
//             TICK_LIQUIDITY
//                 .update(
//                     deps.as_mut().storage,
//                     &(order.book_id, order.tick_id),
//                     |l| {
//                         Ok::<Uint128, ContractError>(
//                             l.unwrap_or_default().checked_add(order.quantity).unwrap(),
//                         )
//                     },
//                 )
//                 .unwrap();

//             let mut orderbook = ORDERBOOKS
//                 .load(deps.as_ref().storage, &valid_book_id)
//                 .unwrap();
//             match order.order_direction {
//                 OrderDirection::Ask => {
//                     if order.tick_id < orderbook.next_ask_tick {
//                         orderbook.next_ask_tick = order.tick_id;
//                     }
//                     ORDERBOOKS
//                         .save(deps.as_mut().storage, &valid_book_id, &orderbook)
//                         .unwrap();
//                 }
//                 OrderDirection::Bid => {
//                     if order.tick_id > orderbook.next_bid_tick {
//                         orderbook.next_bid_tick = order.tick_id;
//                     }
//                     ORDERBOOKS
//                         .save(deps.as_mut().storage, &valid_book_id, &orderbook)
//                         .unwrap();
//                 }
//             }
//         }

//         let mut order = test.order.clone();
//         let response = run_limit_order(deps.as_mut().storage, &mut order);
//         if let Some(expected_error) = &test.expected_error {
//             let err = response.unwrap_err();
//             assert_eq!(err, *expected_error, "{}", format_test_name(test.name));

//             continue;
//         }

//         let bank_msgs = response.unwrap();

//         for (tick_id, expected_liquidity) in test.expected_liquidity {
//             let maybe_current_liquidity = TICK_LIQUIDITY
//                 .may_load(deps.as_ref().storage, &(valid_book_id, tick_id))
//                 .unwrap();

//             if expected_liquidity.is_zero() {
//                 assert!(
//                     maybe_current_liquidity.is_none(),
//                     "{}",
//                     format_test_name(test.name)
//                 );
//             } else {
//                 assert_eq!(
//                     maybe_current_liquidity.unwrap(),
//                     expected_liquidity,
//                     "{}",
//                     format_test_name(test.name)
//                 );
//             }
//         }

//         for fulfillment in test.expected_fulfillments {
//             if fulfillment.amount == fulfillment.order.quantity {
//                 let maybe_order = orders()
//                     .may_load(
//                         deps.as_ref().storage,
//                         &(
//                             fulfillment.order.book_id,
//                             fulfillment.order.tick_id,
//                             fulfillment.order.order_id,
//                         ),
//                     )
//                     .unwrap();
//                 assert!(maybe_order.is_none(), "{}", format_test_name(test.name));
//             }
//         }

//         assert_eq!(
//             test.expected_bank_msgs,
//             bank_msgs,
//             "{}",
//             format_test_name(test.name)
//         );

//         assert_eq!(
//             order.quantity,
//             test.expected_remainder,
//             "{}",
//             format_test_name(test.name)
//         );
//     }
// }
