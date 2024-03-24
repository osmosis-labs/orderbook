use std::i64::MAX;

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    error::ContractError,
    order::*,
    orderbook::*,
    state::*,
    sumtree::{
        node::{NodeType, TreeNode},
        tree::TREE,
    },
    types::{LimitOrder, MarketOrder, OrderDirection, REPLY_ID_REFUND},
};
use cosmwasm_std::{coin, Addr, BankMsg, Coin, DepsMut, Empty, Env, SubMsg, Uint128, Uint256};
use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    Decimal256,
};
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
            &mut deps.as_mut(),
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

            // Verify liquidity was not updated
            let state = TICK_STATE
                .load(&deps.storage, &(test.book_id, test.tick_id))
                .unwrap_or_default();
            assert!(
                state.total_amount_of_liquidity.is_zero(),
                "{}",
                format_test_name(test.name)
            );
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
        assert_eq!(
            response.attributes[7],
            ("quantity_fulfilled", "0"),
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
        assert_eq!(order.etas, Decimal256::zero());

        // Validate liquidity updated as intended
        let state = TICK_STATE
            .load(&deps.storage, &(test.book_id, test.tick_id))
            .unwrap();
        assert_eq!(
            state.total_amount_of_liquidity,
            Decimal256::from_ratio(test.quantity, Uint256::one()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            state.cumulative_total_value,
            Decimal256::from_ratio(test.quantity, Uint256::one()),
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(
            state.effective_total_amount_swapped,
            Decimal256::zero(),
            "{}",
            format_test_name(test.name)
        );
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
            let place_info = mock_info(
                test.owner,
                &[coin(test.quantity.u128(), base_denom.clone())],
            );
            place_limit(
                &mut deps.as_mut(),
                env.clone(),
                place_info,
                test.book_id,
                test.tick_id,
                test.order_direction,
                test.quantity,
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
            let state = TICK_STATE
                .load(deps.as_ref().storage, &(test.book_id, test.tick_id))
                .unwrap_or_default();
            if test.place_order {
                assert_eq!(
                    state.total_amount_of_liquidity,
                    Decimal256::from_ratio(test.quantity, Uint256::one()),
                    "{}",
                    format_test_name(test.name)
                );
            } else {
                assert!(
                    state.total_amount_of_liquidity.is_zero(),
                    "{}",
                    format_test_name(test.name)
                );
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
        let state = TICK_STATE
            .load(deps.as_ref().storage, &(test.book_id, test.tick_id))
            .unwrap_or_default();

        assert!(
            state.total_amount_of_liquidity.is_zero(),
            "{}",
            format_test_name(test.name)
        );

        // -- Sumtree --

        // Ensure tree is saved correctly
        let tree = TREE
            .load(deps.as_ref().storage, &(valid_book_id, test.tick_id))
            .unwrap();

        // Traverse the tree to check its form
        let res = tree.traverse(deps.as_ref().storage).unwrap();
        let mut root_node = TreeNode::new(
            valid_book_id,
            test.tick_id,
            1,
            NodeType::internal(test.quantity, (0u128, test.quantity)),
        );
        let mut cancelled_node = TreeNode::new(
            valid_book_id,
            test.tick_id,
            2,
            NodeType::leaf(0u128, test.quantity),
        );
        root_node.left = Some(cancelled_node.key);
        cancelled_node.parent = Some(root_node.key);

        // Ensure tree traversal returns expected ordering
        assert_eq!(res, vec![root_node, cancelled_node])
    }
}

struct RunMarketOrderTestCase {
    name: &'static str,
    placed_order: MarketOrder,
    tick_bound: i64,
    orders: Vec<LimitOrder>,
    sent: Uint128,

    expected_output: Uint128,
    expected_error: Option<ContractError>,
}

#[test]
fn test_run_market_order() {
    let valid_book_id = 0;
    let invalid_book_id = valid_book_id + 1;
    // TODO: move these defaults to global scope or helper file
    let default_current_tick = 0;
    let default_orders_per_tick = 1;
    let default_owner = "creator";
    let default_sender = "sender";
    let default_quantity = Uint128::new(1000);
    let test_cases = vec![
        RunMarketOrderTestCase {
            name: "happy path bid at negative tick",
            sent: Uint128::new(1000),
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::new(1000),
                OrderDirection::Bid,
                Addr::unchecked(default_sender),
            ),
            tick_bound: MAX_TICK,

            // Orders to fill against
            orders: generate_limit_orders(
                valid_book_id,
                &[-1500000],
                // Current tick is below the active limit orders
                -2500000,
                // 1000 units of liquidity total
                10,
                default_quantity,
            ),

            // Bidding 1000 units of input into tick -1500000, which corresponds to $0.85,
            // implies 1000*0.85 = 850 units of output.
            expected_output: Uint128::new(850),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "happy path bid at positive tick",
            sent: Uint128::new(1000),
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::new(1000),
                OrderDirection::Bid,
                Addr::unchecked(default_sender),
            ),
            tick_bound: MAX_TICK,

            // Orders to fill against
            orders: generate_limit_orders(
                valid_book_id,
                &[40000000],
                // Current tick is below the active limit orders
                default_current_tick,
                // Two orders with sufficient total liquidity to process the
                // full market order
                2,
                Uint128::new(25_000_000),
            ),

            // Bidding 1000 units of input into tick 40,000,000, which corresponds to a
            // price of $50000 (from tick math test cases).
            //
            // This implies 1000*50000 = 50,000,000 units of output.
            expected_output: Uint128::new(50_000_000),
            expected_error: None,
        },
        RunMarketOrderTestCase {
            name: "happy path bid at positive tick",
            sent: Uint128::new(1000),
            placed_order: MarketOrder::new(
                valid_book_id,
                Uint128::new(1000),
                OrderDirection::Bid,
                Addr::unchecked(default_sender),
            ),
            tick_bound: MAX_TICK,

            // Orders to fill against
            orders: generate_limit_orders(
                valid_book_id,
                &[-17765433],
                // Current tick is below the active limit orders
                -20000000,
                // Four limit orders with sufficient total liquidity to process the
                // full market order
                4,
                Uint128::new(3),
            ),

            // Bidding 1000 units of input into tick -17765433, which corresponds to a
            // price of $0.012345670000000000 (from tick math test cases).
            //
            // This implies 1000*0.012345670000000000 = 12.34567 units of output,
            // truncated to 12 units.
            expected_output: Uint128::new(12),
            expected_error: None,
        },
        // Happy path cases in the ask direction

        // Cases where the full order can't be filled in either direction
        // * requires adding refund, could do in follow up PR

        // Error cases:
        // * Invalid book id
        // * Invalid tick bound (both in bid and ask directions)
        // * Insufficient funds sent

        // Edge cases:
        // * Empty orderbook
        // * Exactly min tick/max tick
    ];

    for test in test_cases {
        // --- Setup ---

        // Create a mock environment and info
        let coin_vec = vec![coin(
            test.sent.u128(),
            if test.placed_order.order_direction == OrderDirection::Ask {
                "base"
            } else {
                "quote"
            },
        )];
        let balances = [(default_sender, coin_vec.as_slice())];
        let mut deps = mock_dependencies_with_balances(&balances);
        let env = mock_env();
        let info = mock_info(default_sender, &coin_vec);

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

        // Place limit orders on orderbook
        place_multiple_limit_orders(
            &mut deps.as_mut(),
            env.clone(),
            default_owner,
            valid_book_id,
            test.orders,
        )
        .unwrap();

        // --- System under test ---

        let mut market_order = test.placed_order.clone();
        let response = run_market_order(deps.as_mut().storage, &mut market_order, test.tick_bound);

        // --- Assertions ---

        // TODO: handle error cases

        // Assert no error
        let response = response.unwrap();

        // We expect the output denom to be the opposite of the input denom,
        // although we derive it directly from the order direction to ensure correctness.
        let quote_denom = "quote".to_string();
        let base_denom = "base".to_string();
        let expected_denom = match test.placed_order.order_direction {
            OrderDirection::Bid => base_denom.clone(),
            OrderDirection::Ask => quote_denom.clone(),
        };
        let expected_msg = BankMsg::Send {
            to_address: default_sender.to_string(),
            amount: vec![coin(test.expected_output.u128(), expected_denom)],
        };

        // Ensure output is as expected
        assert_eq!(
            response.0,
            test.expected_output,
            "{}",
            format_test_name(test.name)
        );
        assert_eq!(response.1, expected_msg, "{}", format_test_name(test.name));
    }
}

/// Generates a set of `LimitOrder` objects for testing purposes.
/// `orders_per_tick` orders are generated for each tick in `tick_ids`,
/// with order direction being determined such that they are all placed
/// around `current_tick`.
fn generate_limit_orders(
    book_id: u64,
    tick_ids: &[i64],
    current_tick: i64,
    orders_per_tick: usize,
    quantity_per_order: Uint128,
) -> Vec<LimitOrder> {
    let mut orders = Vec::new();
    for &tick_id in tick_ids {
        let order_direction = if tick_id < current_tick {
            OrderDirection::Bid
        } else {
            OrderDirection::Ask
        };

        for order_num in 0..orders_per_tick {
            let order = LimitOrder {
                book_id: book_id,
                tick_id,
                order_id: order_num as u64,
                order_direction: order_direction.clone(),
                owner: Addr::unchecked("creator"),
                quantity: quantity_per_order,

                // We set this to zero since it will be unused anyway
                etas: Decimal256::zero(),
            };
            orders.push(order);
        }
    }
    orders
}

/// Places a vector of limit orders on the given book_id for a specified owner.
fn place_multiple_limit_orders(
    deps: &mut DepsMut,
    env: Env,
    owner: &str,
    book_id: u64,
    orders: Vec<LimitOrder>,
) -> Result<(), ContractError> {
    for order in orders {
        let coin_vec = vec![coin(
            order.quantity.u128(),
            match order.order_direction {
                OrderDirection::Ask => "base",
                OrderDirection::Bid => "quote",
            },
        )];
        let info = mock_info(owner, &coin_vec);

        // Place the limit order
        place_limit(
            deps,
            env.clone(),
            info,
            book_id,
            order.tick_id,
            order.order_direction.clone(),
            order.quantity,
        )?;
    }
    Ok(())
}
