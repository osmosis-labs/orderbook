use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, BankMsg, Coin, Decimal, Decimal256, SubMsg, Uint128,
};

use crate::{
    orderbook::create_orderbook,
    sudo::{dispatch_swap_exact_amount_in, ensure_fullfilment_amount, EXPECTED_SWAP_FEE},
    types::{LimitOrder, OrderDirection, REPLY_ID_SUDO_SWAP_EX_AMT_IN},
    ContractError,
};

use super::test_utils::{format_test_name, OrderOperation};

struct EnsureFulfillmentAmountTestCase {
    name: &'static str,
    max_amount: Option<Uint128>,
    min_amount: Option<Uint128>,
    expected_denom: String,
    fulfilled: Coin,
    expected_error: Option<ContractError>,
}

#[test]
fn test_ensure_fulfillment_amount() {
    let valid_denom = "denoma";
    let test_cases: Vec<EnsureFulfillmentAmountTestCase> = vec![
        EnsureFulfillmentAmountTestCase {
            name: "valid fulfillment",
            max_amount: Some(Uint128::from(100u128)),
            min_amount: Some(Uint128::zero()),
            expected_denom: valid_denom.to_string(),
            fulfilled: coin(50u128, valid_denom),
            expected_error: None,
        },
        EnsureFulfillmentAmountTestCase {
            name: "exceed max",
            max_amount: Some(Uint128::from(100u128)),
            min_amount: Some(Uint128::zero()),
            expected_denom: valid_denom.to_string(),
            fulfilled: coin(101u128, valid_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Exceeded max swap amount: expected {} received {}",
                    Uint128::from(100u128),
                    Uint128::from(101u128)
                ),
            }),
        },
        EnsureFulfillmentAmountTestCase {
            name: "do not meet min",
            max_amount: Some(Uint128::from(100u128)),
            min_amount: Some(Uint128::from(50u128)),
            expected_denom: valid_denom.to_string(),
            fulfilled: coin(41u128, valid_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Did not meet minimum swap amount: expected {} received {}",
                    Uint128::from(50u128),
                    Uint128::from(41u128)
                ),
            }),
        },
        EnsureFulfillmentAmountTestCase {
            name: "invalid denom",
            max_amount: Some(Uint128::from(100u128)),
            min_amount: Some(Uint128::zero()),
            expected_denom: valid_denom.to_string(),
            fulfilled: coin(41u128, "some other denom"),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Incorrect denom: expected {} received {}",
                    valid_denom, "some other denom"
                ),
            }),
        },
    ];

    for test in test_cases {
        // -- System under test --
        let resp = ensure_fullfilment_amount(
            test.max_amount,
            test.min_amount,
            test.expected_denom,
            &test.fulfilled,
        );

        if let Some(expected_err) = test.expected_error {
            assert_eq!(
                resp.unwrap_err(),
                expected_err,
                "{}: did not receive expected error",
                format_test_name(test.name)
            );
            continue;
        }

        // No assertions as error was not produced
    }
}

struct SwapExactAmountInTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    token_in: Coin,
    token_out_denom: &'static str,
    token_out_min_amount: Uint128,
    swap_fee: Decimal,
    expected_output: Coin,
    expected_error: Option<ContractError>,
}

#[test]
fn test_swap_exact_amount_in() {
    let valid_book_id = 0;
    let valid_tick_id = 0;
    let quote_denom = "quote";
    let base_denom = "base";
    let sender = Addr::unchecked("sender");
    let test_cases: Vec<SwapExactAmountInTestCase> = vec![
        SwapExactAmountInTestCase {
            name: "BID: valid basic swap",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_book_id,
                valid_tick_id,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            token_in: coin(100u128, base_denom),
            token_out_denom: quote_denom,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(100u128, quote_denom),
            expected_error: None,
        },
        SwapExactAmountInTestCase {
            name: "min amount not met",
            pre_operations: vec![],
            token_in: coin(100u128, base_denom),
            token_out_denom: quote_denom,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            expected_output: coin(100u128, quote_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Did not meet minimum swap amount: expected {} received {}",
                    Uint128::from(100u128),
                    Uint128::zero()
                ),
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
            env.clone(),
            info.clone(),
            quote_denom.to_string(),
            base_denom.to_string(),
        )
        .unwrap();

        // Run any pre-operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone(), valid_book_id)
                .unwrap();
        }

        // -- System under test --
        let response = dispatch_swap_exact_amount_in(
            deps.as_mut(),
            sender.to_string(),
            test.token_in,
            test.token_out_denom.to_string(),
            test.token_out_min_amount,
            test.swap_fee,
        );

        // -- Post test assertions --

        // Assert expected error
        if let Some(error) = test.expected_error {
            assert_eq!(
                error,
                response.unwrap_err(),
                "{}: did not receive expected error",
                format_test_name(test.name)
            );
            continue;
        }

        // Response must be valid now
        let response = response.unwrap();
        assert_eq!(
            response.messages.len(),
            1,
            "{}: invalid number of messages in response",
            format_test_name(test.name)
        );

        // Ensure that generated fulfillment message matches what is expected
        let bank_msg = response.messages.first().unwrap();
        let expected_msg = SubMsg::reply_on_error(
            BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![test.expected_output],
            },
            REPLY_ID_SUDO_SWAP_EX_AMT_IN,
        );
        assert_eq!(
            bank_msg,
            &expected_msg,
            "{}: did not receive expected fulfillment message",
            format_test_name(test.name)
        );
    }
}
