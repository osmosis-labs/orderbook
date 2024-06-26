use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info},
    to_json_binary, Addr, BankMsg, Coin, Decimal, Decimal256, Empty, StdError, SubMsg, Uint128,
    Uint256,
};

use crate::{
    auth::ADMIN,
    constants::EXPECTED_SWAP_FEE,
    contract::execute,
    msg::{AuthExecuteMsg, ExecuteMsg, SudoMsg, SwapExactAmountInResponseData},
    orderbook::create_orderbook,
    state::IS_ACTIVE,
    sudo::{
        dispatch_swap_exact_amount_in, ensure_is_active, set_active, sudo, validate_output_amount,
    },
    tests::{mock_querier::mock_dependencies_custom, test_constants::QUOTE_DENOM},
    types::{
        coin_u256, Coin256, LimitOrder, MsgSend256, OrderDirection, REPLY_ID_REFUND,
        REPLY_ID_SUDO_SWAP_EXACT_IN,
    },
    ContractError,
};

use super::{
    test_constants::{BASE_DENOM, DEFAULT_SENDER},
    test_utils::{format_test_name, OrderOperation},
};

struct ValidateOutputAmountTestCase {
    name: &'static str,
    max_in_amount: Uint256,
    min_out_amount: Uint256,
    input: Coin256,
    output: Coin256,
    expected_error: Option<ContractError>,
}

#[test]
fn test_validate_output_amount() {
    let in_denom = "denoma";
    let out_denom = "denomb";
    let test_cases: Vec<ValidateOutputAmountTestCase> = vec![
        ValidateOutputAmountTestCase {
            name: "valid output",
            max_in_amount: Uint256::from(100u128),
            min_out_amount: Uint256::zero(),
            input: coin_u256(50u128, in_denom),
            output: coin_u256(50u128, out_denom),
            expected_error: None,
        },
        ValidateOutputAmountTestCase {
            name: "exceed max",
            max_in_amount: Uint256::from(100u128),
            min_out_amount: Uint256::zero(),
            output: coin_u256(50u128, in_denom),
            input: coin_u256(101u128, out_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Exceeded max swap amount: expected {} received {}",
                    Uint128::from(100u128),
                    Uint128::from(101u128)
                ),
            }),
        },
        ValidateOutputAmountTestCase {
            name: "do not meet min",
            max_in_amount: Uint256::from(100u128),
            min_out_amount: Uint256::from(50u128),
            input: coin_u256(50u128, in_denom),
            output: coin_u256(41u128, out_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Did not meet minimum swap amount: expected {} received {}",
                    Uint128::from(50u128),
                    Uint128::from(41u128)
                ),
            }),
        },
        ValidateOutputAmountTestCase {
            name: "duplicate denom",
            max_in_amount: Uint256::from(100u128),
            min_out_amount: Uint256::zero(),
            input: coin_u256(50u128, in_denom),
            output: coin_u256(41u128, in_denom),
            expected_error: Some(ContractError::InvalidSwap {
                error: "Input and output denoms cannot be the same".to_string(),
            }),
        },
    ];

    for test in test_cases {
        // -- System under test --
        let resp = validate_output_amount(
            test.max_in_amount,
            test.min_out_amount,
            &test.input,
            &test.output,
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
    }
}

struct SwapExactAmountInTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    token_in: Coin,
    token_out_denom: &'static str,
    token_out_min_amount: Uint128,
    swap_fee: Decimal,
    target_tick: Option<i64>,
    expected_output: Coin256,
    expected_num_msgs: usize,
    expected_refund_msg: Option<SubMsg<Empty>>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_swap_exact_amount_in() {
    let valid_tick_id = 0;
    let sender = Addr::unchecked(DEFAULT_SENDER);
    let test_cases: Vec<SwapExactAmountInTestCase> = vec![
        SwapExactAmountInTestCase {
            name: "BID: valid basic swap",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            token_in: coin(100u128, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, BASE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: None,
        },
        SwapExactAmountInTestCase {
            name: "BID: min amount not met",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(10u128),
                Decimal256::zero(),
                None,
            ))],
            token_in: coin(100u128, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, BASE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InsufficientLiquidity),
        },
        SwapExactAmountInTestCase {
            name: "BID: zero liquidity in orderbook",
            pre_operations: vec![],
            token_in: coin(100u128, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, BASE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InsufficientLiquidity),
        },
        SwapExactAmountInTestCase {
            name: "ASK: valid basic swap",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: None,
        },
        SwapExactAmountInTestCase {
            name: "ASK: min amount not met",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Bid,
                sender.clone(),
                Uint128::from(10u128),
                Decimal256::zero(),
                None,
            ))],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InsufficientLiquidity),
        },
        SwapExactAmountInTestCase {
            name: "ASK: zero liquidity in orderbook",
            pre_operations: vec![],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: QUOTE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InsufficientLiquidity),
        },
        SwapExactAmountInTestCase {
            name: "invalid in denom",
            pre_operations: vec![],
            token_in: coin(100u128, "notadenom"),
            token_out_denom: QUOTE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: "notadenom".to_string(),
                token_out_denom: QUOTE_DENOM.to_string(),
            }),
        },
        SwapExactAmountInTestCase {
            name: "invalid out denom",
            pre_operations: vec![],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: "notadenom",
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InvalidPair {
                token_in_denom: BASE_DENOM.to_string(),
                token_out_denom: "notadenom".to_string(),
            }),
        },
        SwapExactAmountInTestCase {
            name: "invalid duplicate denom",
            pre_operations: vec![],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: BASE_DENOM,
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: EXPECTED_SWAP_FEE,
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InvalidSwap {
                error: "Input and output denoms cannot be the same".to_string(),
            }),
        },
        SwapExactAmountInTestCase {
            name: "invalid swap fee",
            pre_operations: vec![],
            token_in: coin(100u128, BASE_DENOM),
            token_out_denom: "notadenom",
            token_out_min_amount: Uint128::from(100u128),
            swap_fee: Decimal::one(),
            target_tick: None,
            expected_output: coin_u256(100u128, QUOTE_DENOM),
            expected_num_msgs: 1,
            expected_refund_msg: None,
            expected_error: Some(ContractError::InvalidSwap {
                error: format!(
                    "Provided swap fee does not match: expected {EXPECTED_SWAP_FEE} received {}",
                    Decimal::one()
                ),
            }),
        },
        SwapExactAmountInTestCase {
            name: "BID: valid basic swap to tick",
            pre_operations: vec![
                OrderOperation::PlaceLimit(LimitOrder::new(
                    valid_tick_id,
                    0,
                    OrderDirection::Ask,
                    sender.clone(),
                    // Insufficient tick liquidity to fill the full swap
                    Uint128::from(90u128),
                    Decimal256::zero(),
                    None,
                )),
                OrderOperation::PlaceLimit(LimitOrder::new(
                    10,
                    1,
                    OrderDirection::Ask,
                    sender.clone(),
                    // We expect this to never be reached due to tick bound
                    Uint128::from(10u128),
                    Decimal256::zero(),
                    None,
                )),
            ],
            token_in: coin(100u128, QUOTE_DENOM),
            token_out_denom: BASE_DENOM,
            token_out_min_amount: Uint128::from(90u128),
            swap_fee: EXPECTED_SWAP_FEE,
            // Past the tick with 90 units of liquidity, but before the tick with the remaining 10
            target_tick: Some(5),
            expected_output: coin_u256(90u128, BASE_DENOM),
            expected_num_msgs: 2,
            expected_refund_msg: Some(SubMsg::reply_on_error(
                BankMsg::Send {
                    to_address: sender.to_string(),
                    // We expect 10 units of the input to be leftover
                    amount: vec![coin(10u128, QUOTE_DENOM)],
                },
                REPLY_ID_REFUND,
            )),
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

        // Run any pre-operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // -- System under test --
        let response = dispatch_swap_exact_amount_in(
            deps.as_mut(),
            env.clone(),
            sender.to_string(),
            test.token_in,
            test.token_out_denom.to_string(),
            test.token_out_min_amount,
            test.swap_fee,
            test.target_tick,
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
            test.expected_num_msgs,
            "{}: invalid number of messages in response",
            format_test_name(test.name)
        );

        // Ensure that generated output message matches what is expected
        let bank_msg = response.messages.first().unwrap();
        let expected_msg = SubMsg::reply_on_error(
            MsgSend256 {
                from_address: env.contract.address.to_string(),
                to_address: sender.to_string(),
                amount: vec![test.expected_output.clone()],
            },
            REPLY_ID_SUDO_SWAP_EXACT_IN,
        );
        assert_eq!(
            bank_msg,
            &expected_msg,
            "{}: did not receive expected output message",
            format_test_name(test.name)
        );

        if test.expected_refund_msg.is_some() {
            let refund_msg = &response.messages[1];
            assert_eq!(
                &test.expected_refund_msg.unwrap(),
                refund_msg,
                "{}: did not receive expected refund message",
                format_test_name(test.name)
            );
        }

        let expected_data = to_json_binary(&SwapExactAmountInResponseData {
            token_out_amount: test.expected_output.amount,
        })
        .unwrap();

        assert_eq!(response.data, Some(expected_data))
    }
}

#[test]
fn test_sudo_transfer_admin() {
    let mut deps = mock_dependencies_custom();
    let env = mock_env();
    let new_admin = "newadmin";

    // Create sudo message for test
    let msg = SudoMsg::TransferAdmin {
        new_admin: Addr::unchecked(new_admin),
    };

    // -- System under test --
    sudo(deps.as_mut(), env.clone(), msg).unwrap();

    // -- Post test assertions --
    assert_eq!(
        ADMIN.load(deps.as_ref().storage).unwrap(),
        Addr::unchecked(new_admin)
    );

    // -- Invalid address check --

    // Create sudo message for invalid address test
    let msg = SudoMsg::TransferAdmin {
        new_admin: Addr::unchecked("ab"),
    };

    // -- System under test --
    let res = sudo(deps.as_mut(), env, msg).unwrap_err();

    // -- Post test assertions --
    assert!(matches!(
        res,
        ContractError::Std(StdError::GenericErr { msg: _ })
    ));
}

#[test]
fn test_sudo_remove_admin() {
    let mut deps = mock_dependencies_custom();
    let env = mock_env();
    let admin = "admin";

    // Store admin in state to be removed
    ADMIN
        .save(deps.as_mut().storage, &Addr::unchecked(admin))
        .unwrap();

    // Create sudo message for test
    let msg = SudoMsg::RemoveAdmin {};

    // -- System under test --
    sudo(deps.as_mut(), env, msg).unwrap();

    // -- Post test assertions --
    assert!(ADMIN.may_load(deps.as_ref().storage).unwrap().is_none());
}

struct SetActiveTestCase {
    name: &'static str,
    active_status: Option<bool>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_active() {
    let test_cases = vec![
        SetActiveTestCase {
            name: "active: true",
            active_status: Some(true),
            expected_error: None,
        },
        SetActiveTestCase {
            name: "active: None",
            active_status: None,
            expected_error: None,
        },
        SetActiveTestCase {
            name: "active: false",
            active_status: Some(false),
            expected_error: Some(ContractError::Inactive),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();

        if let Some(active) = test.active_status {
            set_active(deps.as_mut(), active).unwrap();
        }

        // -- System under test --
        let resp = ensure_is_active(deps.as_ref());

        if let Some(expected_err) = test.expected_error {
            assert_eq!(
                resp.unwrap_err(),
                expected_err,
                "{}: did not receive expected error",
                test.name
            );
            continue;
        }

        // -- Post test assertions --
        let is_active = IS_ACTIVE.may_load(deps.as_ref().storage).unwrap();
        assert_eq!(
            is_active, test.active_status,
            "{}: active status did not match expected",
            test.name
        );
    }
}

struct SetActiveExecuteTestCase {
    name: &'static str,
    msg: ExecuteMsg,
    active_status: Option<bool>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_active_execute() {
    let test_cases = vec![
        SetActiveExecuteTestCase {
            name: "active: true, message type: order",
            msg: ExecuteMsg::PlaceLimit {
                tick_id: 0,
                order_direction: OrderDirection::Ask,
                quantity: Uint128::from(100u128),
                claim_bounty: None,
            },
            active_status: Some(true),
            expected_error: None,
        },
        SetActiveExecuteTestCase {
            name: "active: None, message type: order",
            msg: ExecuteMsg::PlaceLimit {
                tick_id: 0,
                order_direction: OrderDirection::Ask,
                quantity: Uint128::from(100u128),
                claim_bounty: None,
            },
            active_status: None,
            expected_error: None,
        },
        SetActiveExecuteTestCase {
            name: "active: false, message type: order",
            msg: ExecuteMsg::PlaceLimit {
                tick_id: 0,
                order_direction: OrderDirection::Ask,
                quantity: Uint128::from(100u128),
                claim_bounty: None,
            },
            active_status: Some(false),
            expected_error: Some(ContractError::Inactive),
        },
        SetActiveExecuteTestCase {
            name: "active: true, message type: auth",
            msg: ExecuteMsg::Auth(AuthExecuteMsg::TransferAdmin {
                new_admin: Addr::unchecked("new_admin"),
            }),
            active_status: Some(true),
            expected_error: None,
        },
        SetActiveExecuteTestCase {
            name: "active: true, message type: auth",
            msg: ExecuteMsg::Auth(AuthExecuteMsg::TransferAdmin {
                new_admin: Addr::unchecked("new_admin"),
            }),
            active_status: None,
            expected_error: None,
        },
        SetActiveExecuteTestCase {
            name: "active: true, message type: auth",
            msg: ExecuteMsg::Auth(AuthExecuteMsg::TransferAdmin {
                new_admin: Addr::unchecked("new_admin"),
            }),
            active_status: Some(false),
            expected_error: None,
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(DEFAULT_SENDER, &[coin(100u128, BASE_DENOM)]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(DEFAULT_SENDER))
            .unwrap();
        if let Some(active) = test.active_status {
            set_active(deps.as_mut(), active).unwrap();
        }

        // -- System under test --
        let resp = execute(deps.as_mut(), env, info, test.msg);
        if let Some(expected_err) = test.expected_error {
            assert_eq!(
                resp.unwrap_err(),
                expected_err,
                "{}: did not receive expected error",
                test.name
            );
            continue;
        }

        assert!(
            resp.is_ok(),
            "{}: execute message unexpectedly failed; {}",
            test.name,
            resp.unwrap_err()
        );
    }
}

struct SetActiveSudoTestCase {
    name: &'static str,
    pre_operations: Vec<OrderOperation>,
    msg: SudoMsg,
    active_status: Option<bool>,
    expected_error: Option<ContractError>,
}

#[test]
fn test_set_active_sudo() {
    let valid_tick_id = 0;
    let sender = Addr::unchecked(DEFAULT_SENDER);
    let test_cases = vec![
        SetActiveSudoTestCase {
            name: "active book, swap exact in",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            msg: SudoMsg::SwapExactAmountIn {
                sender: sender.to_string(),
                token_in: coin(100u128, QUOTE_DENOM),
                token_out_denom: BASE_DENOM.to_string(),
                token_out_min_amount: Uint128::from(100u128),
                swap_fee: Decimal::zero(),
            },
            active_status: Some(true),
            expected_error: None,
        },
        SetActiveSudoTestCase {
            name: "inactive, swap exact in",
            pre_operations: vec![OrderOperation::PlaceLimit(LimitOrder::new(
                valid_tick_id,
                0,
                OrderDirection::Ask,
                sender.clone(),
                Uint128::from(100u128),
                Decimal256::zero(),
                None,
            ))],
            msg: SudoMsg::SwapExactAmountIn {
                sender: sender.to_string(),
                token_in: coin(100u128, QUOTE_DENOM),
                token_out_denom: BASE_DENOM.to_string(),
                token_out_min_amount: Uint128::from(100u128),
                swap_fee: Decimal::zero(),
            },
            active_status: Some(false),
            expected_error: Some(ContractError::Inactive),
        },
    ];

    for test in test_cases {
        // -- Test Setup --
        let mut deps = mock_dependencies_custom();
        let env = mock_env();
        let info = mock_info(DEFAULT_SENDER, &[coin(100u128, BASE_DENOM)]);

        create_orderbook(
            deps.as_mut(),
            QUOTE_DENOM.to_string(),
            BASE_DENOM.to_string(),
        )
        .unwrap();

        // Run pre-operations
        for op in test.pre_operations {
            op.run(deps.as_mut(), env.clone(), info.clone()).unwrap();
        }

        // Set orderbook active status
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked(DEFAULT_SENDER))
            .unwrap();
        if let Some(active) = test.active_status {
            set_active(deps.as_mut(), active).unwrap();
        }

        // -- System under test --
        let resp = sudo(deps.as_mut(), env, test.msg);
        if let Some(expected_err) = test.expected_error {
            assert_eq!(
                resp.unwrap_err(),
                expected_err,
                "{}: did not receive expected error",
                test.name
            );
            continue;
        }

        assert!(
            resp.is_ok(),
            "{}: execute message unexpectedly failed; {}",
            test.name,
            resp.unwrap_err()
        );
    }
}
