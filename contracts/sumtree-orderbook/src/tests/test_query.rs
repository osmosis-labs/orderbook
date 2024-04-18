use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin, Decimal, Decimal256, Uint128,
};

use crate::{
    orderbook::create_orderbook,
    query,
    sudo::EXPECTED_SWAP_FEE,
    types::{LimitOrder, MarketOrder, OrderDirection},
    ContractError,
};

use super::test_utils::{
    format_test_name, OrderOperation, LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK,
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
    let sender = Addr::unchecked("sender");
    let base_denom = "base";
    let quote_denom = "quote";
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
            base_denom: base_denom.to_string(),
            quote_denom: quote_denom.to_string(),
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
            base_denom: base_denom.to_string(),
            quote_denom: quote_denom.to_string(),
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
            base_denom: base_denom.to_string(),
            quote_denom: quote_denom.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "BID: moving tick",
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
            base_denom: base_denom.to_string(),
            quote_denom: quote_denom.to_string(),
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
            base_denom: quote_denom.to_string(),
            quote_denom: base_denom.to_string(),
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
            base_denom: quote_denom.to_string(),
            quote_denom: base_denom.to_string(),
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
            base_denom: quote_denom.to_string(),
            quote_denom: base_denom.to_string(),
            expected_price: Decimal::one(),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "ASK: moving tick",
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
            base_denom: quote_denom.to_string(),
            quote_denom: base_denom.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: None,
        },
        SpotPriceTestCase {
            name: "invalid: duplicate denom",
            pre_operations: vec![],
            base_denom: base_denom.to_string(),
            quote_denom: base_denom.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: base_denom.to_string(),
                token_out_denom: base_denom.to_string(),
            }),
        },
        SpotPriceTestCase {
            name: "invalid: incorrect base denom",
            pre_operations: vec![],
            base_denom: "notadenom".to_string(),
            quote_denom: quote_denom.to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: quote_denom.to_string(),
                token_out_denom: "notadenom".to_string(),
            }),
        },
        SpotPriceTestCase {
            name: "invalid: incorrect quote denom",
            pre_operations: vec![],
            base_denom: base_denom.to_string(),
            quote_denom: "notadenom".to_string(),
            expected_price: Decimal::percent(50),
            expected_error: Some(ContractError::InvalidPair {
                token_out_denom: base_denom.to_string(),
                token_in_denom: "notadenom".to_string(),
            }),
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            quote_denom.to_string(),
            base_denom.to_string(),
        )
        .unwrap();

        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        let res = query::spot_price(deps.as_ref(), test.quote_denom, test.base_denom);

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
            res.spot_price,
            test.expected_price,
            "{}: price did not match",
            format_test_name(test.name)
        )
    }
}

struct CalcOutAmountGivenInTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    token_in: Coin,
    token_out_denom: String,
    swap_fee: Decimal,
    expected_output: Coin,
    expected_error: Option<ContractError>,
}

#[test]
fn test_calc_out_amount_given_in() {
    let sender = Addr::unchecked("sender");
    let quote_denom = "quote".to_string();
    let base_denom = "base".to_string();

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
            token_in: coin(100, &quote_denom),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(100, &base_denom),
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
            token_in: coin(150, &quote_denom),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
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
            token_in: coin(150, &quote_denom),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            // Output: 100*1 (tick: 0) + 50*2 (tick: LARGE_POSITIVE_TICK) = 200
            expected_output: coin(200, &base_denom),
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
            token_in: coin(100, &base_denom),
            token_out_denom: quote_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(100, &quote_denom),
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
            token_in: coin(150, &base_denom),
            token_out_denom: quote_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &quote_denom),
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
            token_in: coin(150, &base_denom),
            token_out_denom: quote_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            // Output: 25 at 0.5 tick price + 100 at 1 tick price = 125
            expected_output: coin(125, &quote_denom),
            expected_error: None,
        },
        CalcOutAmountGivenInTestCase {
            name: "insufficient liquidity",
            pre_operations: vec![],
            token_in: coin(100, &quote_denom),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
            expected_error: Some(ContractError::InsufficientLiquidity {}),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid duplicate denom",
            pre_operations: vec![],
            token_in: coin(100, &base_denom),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: base_denom.to_string(),
                token_out_denom: base_denom.to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid in denom",
            pre_operations: vec![],
            token_in: coin(100, "notadenom"),
            token_out_denom: base_denom.clone(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: "notadenom".to_string(),
                token_out_denom: base_denom.to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid out denom",
            pre_operations: vec![],
            token_in: coin(100, &base_denom),
            token_out_denom: "notadenom".to_string(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: base_denom.to_string(),
                token_out_denom: "notadenom".to_string(),
            }),
        },
        CalcOutAmountGivenInTestCase {
            name: "invalid zero amount",
            pre_operations: vec![],
            token_in: coin(0, &base_denom),
            token_out_denom: quote_denom.to_string(),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(0, &base_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: "Input amount cannot be zero".to_string(),
            }),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(sender.as_str(), &[]);

        create_orderbook(
            deps.as_mut(),
            quote_denom.to_string(),
            base_denom.to_string(),
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
            test.token_out_denom.clone(),
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
            res.token_out, test.expected_output,
            "{}: output did not match",
            test.name
        );
    }
}
