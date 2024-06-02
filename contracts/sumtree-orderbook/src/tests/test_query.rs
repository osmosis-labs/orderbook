use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
    Addr, Coin, Decimal, Decimal256, Uint128,
};

use crate::{
    constants::{EXPECTED_SWAP_FEE, MAX_TICK, MIN_TICK},
    orderbook::create_orderbook,
    query,
    state::IS_ACTIVE,
    tests::mock_querier::mock_dependencies_custom,
    types::{coin_u256, Coin256, LimitOrder, MarketOrder, OrderDirection, TickState, TickValues},
    ContractError,
};

use super::{
    test_constants::{
        BASE_DENOM, DEFAULT_SENDER, LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK, QUOTE_DENOM,
    },
    test_utils::{decimal256_from_u128, format_test_name, generate_tick_ids, OrderOperation},
};

struct SpotPriceTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    base_denom: String,
    quote_denom: String,
    expected_price: Decimal,
    expected_error: Option<ContractError>,
}

#[test]
fn test_query_spot_price() {
    let sender = Addr::unchecked(DEFAULT_SENDER);
    let test_cases: Vec<SpotPriceTestCase> = vec![
        SpotPriceTestCase {
            name: "BID: basic price 1 query",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::one(),
                Decimal256::zero(),
                None,
            ))],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: QUOTE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "BID: multi tick lowest price",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    2,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    2,
                    3,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
            ],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: QUOTE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "BID: multi direction lowest tick",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
            ],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: QUOTE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "BID: change in spot price",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_POSITIVE_TICK,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::RunMarket(MarketOrder::new(
                    Uint128::from(2u128),
                    OrderDirection::Bid,
                    sender.clone(),
                )),
            ],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: QUOTE_DENOM.to_string(),
            expected_price: Decimal::percent(200),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "ASK: basic price 1 query",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::one(),
                Decimal256::zero(),
                None,
            ))],
            base_denom: QUOTE_DENOM.to_string(),
            quote_denom: BASE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "ASK: multi tick lowest price",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    1,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    -1,
                    2,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    -2,
                    3,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
            ],
            base_denom: QUOTE_DENOM.to_string(),
            quote_denom: BASE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "ASK: multi direction lowest tick",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
            ],
            base_denom: QUOTE_DENOM.to_string(),
            quote_denom: BASE_DENOM.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "ASK: change in spot price",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_NEGATIVE_TICK,
                    1,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::RunMarket(MarketOrder::new(
                    Uint128::from(2u128),
                    OrderDirection::Ask,
                    sender.clone(),
                )),
            ],
            base_denom: QUOTE_DENOM.to_string(),
            quote_denom: BASE_DENOM.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "invalid: duplicate denom",
            pre_operations: vec![],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: BASE_DENOM.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: BASE_DENOM.to_string(),
                token_out_denom: BASE_DENOM.to_string(),
            }),
        },
        SpotPriceTestCase {
            name: "invalid: incorrect base denom",
            pre_operations: vec![],
            base_denom: "notadenom".to_string(),
            quote_denom: QUOTE_DENOM.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: QUOTE_DENOM.to_string(),
                token_out_denom: "notadenom".to_string(),
            }),
        },
        SpotPriceTestCase {
            name: "invalid: incorrect quote denom",
            pre_operations: vec![],
            base_denom: BASE_DENOM.to_string(),
            quote_denom: "notadenom".to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_out_denom: BASE_DENOM.to_string(),
                token_in_denom: "notadenom".to_string(),
            }),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        // Perform any setup market operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // -- System under test --

        let res = query::spot_price(deps.as_ref(), test.quote_denom, test.base_denom);

        // Assert any expected errors from the test
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                format_test_name(test.name)
            );

            continue;
        }

        let res = res.unwrap();
        assert_eq!(
            res.spot_price, test.expected_price,
            "{}: output did not match",
            test.name
        );
    }
}

struct CalcOutAmountGivenInTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    token_in: Coin,
    token_out_denom: &'static str,
    swap_fee: Decimal,
    expected_output: Coin256,
    expected_error: Option<ContractError>,
}

#[test]
fn test_calc_out_amount_given_in() {
    let sender = Addr::unchecked(DEFAULT_SENDER);

    let test_cases = vec![
        CalcOutAmountGivenInTestCase {
            name: "BID: simple swap",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::percent(0),
                None,
            ))],
            token_in: coin(100, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(100u128, BASE_DENOM),
            expected_error: None,
        },
        CalcOutAmountGivenInTestCase {
            name: "BID: invalid partial fill",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(25u128),
                Decimal256::percent(0),
                None,
            ))],
            token_in: coin(150, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InsufficientLiquidity {}),
        },
        CalcOutAmountGivenInTestCase {
            name: "BID: multi-tick/direction swap",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_POSITIVE_TICK,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_POSITIVE_TICK,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
            ],
            token_in: coin(150, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            // Output: 100*1 (tick: 0) + 50*2 (tick: LARGE_POSITIVE_TICK) = 200
            expected_output: coin_u256(200u128, BASE_DENOM),
            expected_error: None,
        },
        CalcOutAmountGivenInTestCase {
            name: "ASK: simple swap",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::percent(0),
                None,
            ))],
            token_in: coin(100, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_error: None,
        },
        CalcOutAmountGivenInTestCase {
            name: "ASK: invalid partial fill",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::from(25u128),
                Decimal256::percent(0),
                None,
            ))],
            token_in: coin(150, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, QUOTE_DENOM),
            expected_error: Some(ContractError::InsufficientLiquidity {}),
        },
        CalcOutAmountGivenInTestCase {
            name: "ASK: multi-tick/direction swap",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_POSITIVE_TICK,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::from(25u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::percent(0),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    LARGE_POSITIVE_TICK,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::from(25u128),
                    Decimal256::percent(0),
                    None,
                )),
            ],
            token_in: coin(150, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            // Output: 25 at 0.5 tick price + 100 at 1 tick price = 125
            expected_output: coin_u256(125u128, QUOTE_DENOM),
            expected_error: None,
        },
        CalcOutAmountGivenInTestCase {
            name: "insufficient liquidity",
            pre_operations: vec![],
            token_in: coin(100, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InsufficientLiquidity {}),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid duplicate denom",
            pre_operations: vec![],
            token_in: coin(100, BASE_DENOM),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: BASE_DENOM.to_string(),
                token_out_denom: BASE_DENOM.to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid in denom",
            pre_operations: vec![],
            token_in: coin(100, "notadenom"),
            token_out_denom: BASE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: "notadenom".to_string(),
                token_out_denom: BASE_DENOM.to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid out denom",
            pre_operations: vec![],
            token_in: coin(100, BASE_DENOM),
            token_out_denom: "notadenom",
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: BASE_DENOM.to_string(),
                token_out_denom: "notadenom".to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid zero amount",
            pre_operations: vec![],
            token_in: coin(0, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin_u256(0u128, BASE_DENOM),
            expected_error: Some(ContractError::InvalidSwap {
                error: "Input amount cannot be zero".to_string(),
            }),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        // Perform any setup market operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // -- System under test --

        let res = query::calc_out_amount_given_in(
            deps.as_ref(),
            test.token_in.clone(),
            test.token_out_denom.to_string(),
            test.swap_fee,
        );

        // Assert any expected errors from the test
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                format_test_name(test.name)
            );

            continue;
        }

        let res = res.unwrap();
        assert_eq!(
            res.token_out,
            test.expected_output.into(),
            "{}: output did not match",
            test.name
        );
    }
}

struct TotalPoolLiquidityTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    expected_output: Vec<Coin>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_total_pool_liquidity() {
    let sender = Addr::unchecked(DEFAULT_SENDER);

    let test_cases = vec![
        TotalPoolLiquidityTestCase {
            name: "simple test",
            pre_operations: vec![],
            expected_output: vec![coin(0, BASE_DENOM), coin(0, QUOTE_DENOM)],
            expected_error: None,
        },
        TotalPoolLiquidityTestCase {
            name: "basic single tick non-empty query",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::from(100u128),
                    Decimal256::zero(),
                    None,
                )),
            ],
            expected_output: vec![coin(100, BASE_DENOM), coin(100, QUOTE_DENOM)],
            expected_error: None,
        },
        TotalPoolLiquidityTestCase {
            name: "multi-tick test",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    // Increasingly spread ticks
                    vec![
                        -1,
                        -2,
                        -3,
                        -5,
                        -8,
                        -13,
                        -21,
                        -34,
                        -55,
                        LARGE_NEGATIVE_TICK,
                        MIN_TICK,
                    ],
                    100,
                    Uint128::from(50u128),
                    OrderDirection::Bid,
                )),
                OrderOperation::PlaceLimitMulti((
                    // Increasingly spread ticks
                    vec![1, 2, 3, 5, 8, 13, 21, 34, 55, LARGE_POSITIVE_TICK, MAX_TICK],
                    100,
                    Uint128::from(110u128),
                    OrderDirection::Ask,
                )),
            ],
            // Base: 11 ticks at 110*100 = 11000*11 = 121000
            // Quote: 11 ticks at 50*100 = 5000*11 = 55000
            expected_output: vec![coin(121000, BASE_DENOM), coin(55000, QUOTE_DENOM)],
            expected_error: None,
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        // Perform any setup market operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // -- System under test --

        let res = query::total_pool_liquidity(deps.as_ref());

        // Assert any expected errors from the test
        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                format_test_name(test.name)
            );

            continue;
        }

        let res = res.unwrap();
        assert_eq!(
            res.total_pool_liquidity, test.expected_output,
            "{}: output did not match",
            test.name
        );
    }
}

struct AllTicksTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    expected_output: Vec<TickState>,
    start_after: Option<i64>,
    end_at: Option<i64>,
    limit: Option<usize>,
}

#[test]
fn test_all_ticks() {
    let sender = Addr::unchecked(DEFAULT_SENDER);

    let test_cases: Vec<AllTicksTestCase> = vec![
        AllTicksTestCase {
            name: "Test all ticks",
            pre_operations: vec![],
            expected_output: vec![],
            start_after: None,
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "Single order, single tick",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::one(),
                Decimal256::zero(),
                None,
            ))],
            expected_output: vec![TickState {
                ask_values: TickValues {
                    total_amount_of_liquidity: Decimal256::one(),
                    cumulative_total_value: Decimal256::one(),
                    effective_total_amount_swapped: Decimal256::zero(),
                    cumulative_realized_cancels: Decimal256::zero(),
                    last_tick_sync_etas: Decimal256::zero(),
                },
                bid_values: TickValues::default(),
            }],
            start_after: None,
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "Multiple directions, single tick",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Bid,
                    sender.clone(),
                    Uint128::one(),
                    Decimal256::zero(),
                    None,
                )),
            ],
            expected_output: vec![TickState {
                ask_values: TickValues {
                    total_amount_of_liquidity: Decimal256::one(),
                    cumulative_total_value: Decimal256::one(),
                    effective_total_amount_swapped: Decimal256::zero(),
                    cumulative_realized_cancels: Decimal256::zero(),
                    last_tick_sync_etas: Decimal256::zero(),
                },
                bid_values: TickValues {
                    total_amount_of_liquidity: Decimal256::one(),
                    cumulative_total_value: Decimal256::one(),
                    effective_total_amount_swapped: Decimal256::zero(),
                    cumulative_realized_cancels: Decimal256::zero(),
                    last_tick_sync_etas: Decimal256::zero(),
                },
            }],
            start_after: None,
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "Multiple directions, many ticks",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(10u128),
                    OrderDirection::Ask,
                )),
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(100u128),
                    OrderDirection::Bid,
                )),
            ],
            expected_output: generate_tick_ids(100)
                .iter()
                .map(|_| TickState {
                    ask_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(100u128),
                        cumulative_total_value: decimal256_from_u128(100u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: None,
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "Multiple directions, many ticks w/ limit",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(10u128),
                    OrderDirection::Ask,
                )),
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(100u128),
                    OrderDirection::Bid,
                )),
            ],
            expected_output: generate_tick_ids(50)
                .iter()
                .map(|_| TickState {
                    ask_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(100u128),
                        cumulative_total_value: decimal256_from_u128(100u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: None,
            end_at: None,
            limit: Some(50),
        },
        AllTicksTestCase {
            name: "Multiple directions, many ticks w/ start after",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(10u128),
                    OrderDirection::Ask,
                )),
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(100u128),
                    OrderDirection::Bid,
                )),
            ],
            expected_output: generate_tick_ids(100)
                .iter()
                .enumerate()
                .filter(|(id, _)| *id >= 90)
                .map(|_| TickState {
                    ask_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(100u128),
                        cumulative_total_value: decimal256_from_u128(100u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: Some(90i64),
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "Multiple directions, many ticks w/ end at",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(10u128),
                    OrderDirection::Ask,
                )),
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(100u128),
                    OrderDirection::Bid,
                )),
            ],
            expected_output: generate_tick_ids(100)
                .iter()
                .enumerate()
                .filter(|(id, _)| *id <= 44)
                .map(|_| TickState {
                    ask_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(100u128),
                        cumulative_total_value: decimal256_from_u128(100u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: None,
            end_at: Some(44i64),
            limit: None,
        },
        AllTicksTestCase {
            name: "Multiple directions, many ticks w/ start after & end at",
            pre_operations: vec![
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(10u128),
                    OrderDirection::Ask,
                )),
                OrderOperation::PlaceLimitMulti((
                    generate_tick_ids(100),
                    10,
                    Uint128::from(100u128),
                    OrderDirection::Bid,
                )),
            ],
            expected_output: generate_tick_ids(100)
                .iter()
                .enumerate()
                .filter(|(id, _)| *id <= 44 && *id >= 21)
                .map(|_| TickState {
                    ask_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(100u128),
                        cumulative_total_value: decimal256_from_u128(100u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: Some(21i64),
            end_at: Some(44i64),
            limit: None,
        },
        AllTicksTestCase {
            name: "large number of ticks",
            pre_operations: vec![OrderOperation::PlaceLimitMulti((
                generate_tick_ids(1010),
                10,
                Uint128::from(100u128),
                OrderDirection::Bid,
            ))],
            expected_output: generate_tick_ids(1010)
                .iter()
                .map(|_| TickState {
                    ask_values: TickValues::default(),
                    bid_values: TickValues {
                        total_amount_of_liquidity: decimal256_from_u128(1000u128),
                        cumulative_total_value: decimal256_from_u128(1000u128),
                        effective_total_amount_swapped: Decimal256::zero(),
                        cumulative_realized_cancels: Decimal256::zero(),
                        last_tick_sync_etas: Decimal256::zero(),
                    },
                })
                .collect(),
            start_after: None,
            end_at: None,
            limit: None,
        },
        AllTicksTestCase {
            name: "single tick paginated",
            pre_operations: vec![OrderOperation::PlaceLimitMulti((
                generate_tick_ids(200),
                10,
                Uint128::from(100u128),
                OrderDirection::Bid,
            ))],
            expected_output: vec![TickState {
                ask_values: TickValues::default(),
                bid_values: TickValues {
                    total_amount_of_liquidity: decimal256_from_u128(1000u128),
                    cumulative_total_value: decimal256_from_u128(1000u128),
                    effective_total_amount_swapped: Decimal256::zero(),
                    cumulative_realized_cancels: Decimal256::zero(),
                    last_tick_sync_etas: Decimal256::zero(),
                },
            }],
            start_after: Some(11),
            end_at: Some(11),
            limit: None,
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        // Perform any setup market operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // -- System under test --

        let res =
            query::all_ticks(deps.as_ref(), test.start_after, test.end_at, test.limit).unwrap();
        assert_eq!(
            res.ticks.len(),
            test.expected_output.len(),
            "{}: output lengths did not match",
            test.name
        );
        assert_eq!(
            res.ticks
                .iter()
                .map(|t| t.tick_state.clone())
                .collect::<Vec<TickState>>(),
            test.expected_output,
            "{}: output did not match",
            test.name
        );
    }
}

pub struct IsActiveTestCase {
    name: &'static str,
    is_active: Option<bool>,
}

#[test]
fn test_is_active() {
    let test_cases = vec![
        IsActiveTestCase {
            name: "active status",
            is_active: Some(true),
        },
        IsActiveTestCase {
            name: "inactive status",
            is_active: Some(false),
        },
        IsActiveTestCase {
            name: "no active status (active)",
            is_active: None,
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();

        // Setup state variables
        if let Some(active) = test.is_active {
            IS_ACTIVE.save(deps.as_mut().storage, &active).unwrap();
        }

        // -- System under test --
        let res = query::is_active(deps.as_ref()).unwrap();

        // -- Test Assertions --
        assert_eq!(
            res,
            test.is_active.unwrap_or(true),
            "{}: active state did not match",
            test.name
        );
    }
}

struct OrdersByOwnerTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    expected_output: Vec<LimitOrder>,
    owner: Addr,
    start_from: Option<(i64, u64)>,
    end_at: Option<(i64, u64)>,
    limit: Option<u64>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_orders_by_owner() {
    let quote_denom = "quote";
    let base_denom = "base";

    let test_cases = vec![
        OrdersByOwnerTestCase {
            name: "no orders",
            pre_operations: vec![],
            expected_output: vec![],
            owner: Addr::unchecked("sender"),
            start_from: None,
            end_at: None,
            limit: None,
            expected_error: None,
        },
        OrdersByOwnerTestCase {
            name: "single order",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                0,
                0,
                OrderDirection::Bid,
                Addr::unchecked("sender"),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            expected_output: vec![LimitOrder::new(
                0,
                0,
                OrderDirection::Bid,
                Addr::unchecked("sender"),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            )],
            owner: Addr::unchecked("sender"),
            start_from: None,
            end_at: None,
            limit: None,
            expected_error: None,
        },
        OrdersByOwnerTestCase {
            name: "multiple orders, pagination limit",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(50u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    1,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
            ],
            expected_output: vec![LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                Addr::unchecked("sender"),
                Uint128::from(50u128),
                Decimal256::zero(),
                None,
            )],
            owner: Addr::unchecked("sender"),
            start_from: None,
            end_at: None,
            limit: Some(1),
            expected_error: None,
        },
        OrdersByOwnerTestCase {
            name: "multiple orders, start_from",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(50u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    1,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
            ],
            expected_output: vec![LimitOrder::new(
                1,
                1,
                OrderDirection::Bid,
                Addr::unchecked("sender"),
                Uint128::from(150u128),
                Decimal256::zero(),
                None,
            )],
            owner: Addr::unchecked("sender"),
            start_from: Some((0, 0)),
            end_at: None,
            limit: None,
            expected_error: None,
        },
        OrdersByOwnerTestCase {
            name: "multiple orders, end_at",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(50u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    1,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
            ],
            expected_output: vec![LimitOrder::new(
                0,
                0,
                OrderDirection::Ask,
                Addr::unchecked("sender"),
                Uint128::from(50u128),
                Decimal256::zero(),
                None,
            )],
            owner: Addr::unchecked("sender"),
            start_from: None,
            end_at: Some((0, 0)),
            limit: None,
            expected_error: None,
        },
        OrdersByOwnerTestCase {
            name: "multiple orders, ordering by tick maintained",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(50u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    1,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    1,
                    2,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    0,
                    3,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::Cancel((1, 1)),
            ],
            expected_output: vec![
                LimitOrder::new(
                    0,
                    0,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(50u128),
                    Decimal256::zero(),
                    None,
                ),
                LimitOrder::new(
                    0,
                    3,
                    OrderDirection::Ask,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    decimal256_from_u128(50u128),
                    None,
                ),
                LimitOrder::new(
                    1,
                    2,
                    OrderDirection::Bid,
                    Addr::unchecked("sender"),
                    Uint128::from(150u128),
                    decimal256_from_u128(150u128),
                    None,
                ),
            ],
            owner: Addr::unchecked("sender"),
            start_from: None,
            end_at: None,
            limit: None,
            expected_error: None,
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(test.owner.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            quote_denom.to_string(),
            base_denom.to_string(),
        )
        .unwrap();

        for operation in test.pre_operations {
            operation
                .run(deps.as_mut(), env.clone(), info.clone())
                .unwrap();
        }

        // -- System under test --
        let res = query::orders_by_owner(
            deps.as_ref(),
            test.owner,
            test.start_from,
            test.end_at,
            test.limit,
        );

        if let Some(err) = test.expected_error {
            assert_eq!(
                res.unwrap_err(),
                err,
                "{}: did not receive expected error",
                test.name
            );

            continue;
        }

        let res = res.unwrap_or_else(|_| {
            panic!(
                "{}: orders_by_owner returned an unexpected error",
                test.name
            );
        });
        assert_eq!(
            res, test.expected_output,
            "{}: output did not match",
            test.name
        );
    }
}
