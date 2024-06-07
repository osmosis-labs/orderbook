use crate::constants::*;
use crate::error::ContractError;
use crate::tick_math::{
    divide_by_price, multiply_by_price, pow_ten, tick_to_price, RoundingDirection,
};
use cosmwasm_std::{Decimal256, OverflowError, OverflowOperation, Uint128, Uint256};
use std::str::FromStr;

struct TickToPriceTestCase {
    tick_index: i64,
    expected_price: Decimal256,
    expected_error: Option<ContractError>,
}

#[test]
fn test_tick_to_price() {
    // This constant is used to test price iterations near max tick.
    // It essentially derives the amount we expect price to increment by,
    // which with an EXPONENT_AT_PRICE_ONE of -6 should be 10^14.
    let min_increment_near_max_price = Decimal256::from_ratio(
        Uint256::from(10u8)
            .checked_pow((20 + EXPONENT_AT_PRICE_ONE) as u32)
            .unwrap(),
        Uint256::one(),
    );
    let tick_price_test_cases = vec![
        TickToPriceTestCase {
            tick_index: MAX_TICK,
            expected_price: *MAX_SPOT_PRICE,
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: MIN_TICK,
            expected_price: Decimal256::from_str("0.000000000001").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 40000000,
            expected_price: Decimal256::from_str("50000").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 4010000,
            expected_price: Decimal256::from_str("5.01").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 40000001,
            expected_price: Decimal256::from_str("50000.01").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -9999900,
            expected_price: Decimal256::from_str("0.090001").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -2000,
            expected_price: Decimal256::from_str("0.9998").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 40303000,
            expected_price: Decimal256::from_str("53030").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: MAX_TICK - 1,
            expected_price: MAX_SPOT_PRICE
                .checked_sub(min_increment_near_max_price)
                .unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: MIN_TICK,
            expected_price: *MIN_SPOT_PRICE,
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: MIN_TICK + 1,
            expected_price: Decimal256::from_str("0.000000000001000001").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -17765433,
            expected_price: Decimal256::from_str("0.012345670000000000").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -17765432,
            expected_price: Decimal256::from_str("0.012345680000000000").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -107765433,
            expected_price: Decimal256::from_str("0.000000000001234567").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: -107765432,
            expected_price: Decimal256::from_str("0.000000000001234568").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 81234567,
            expected_price: Decimal256::from_str("1234567000").unwrap(),
            expected_error: None,
        },
        // This case involves truncation in the previous case, so the expected price is adjusted accordingly
        TickToPriceTestCase {
            tick_index: 81234567, // Same tick index as the previous case due to truncation
            expected_price: Decimal256::from_str("1234567000").unwrap(), // Expected price matches the truncated price
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 81234568,
            expected_price: Decimal256::from_str("1234568000").unwrap(),
            expected_error: None,
        },
        TickToPriceTestCase {
            tick_index: 0,
            expected_price: Decimal256::from_str("1").unwrap(),
            expected_error: None,
        },
    ];

    for test in tick_price_test_cases {
        let result = tick_to_price(test.tick_index);

        match test.expected_error {
            Some(expected_err) => assert_eq!(result.unwrap_err(), expected_err),
            None => assert_eq!(
                test.expected_price,
                result.unwrap(),
                "expected price and result did not match"
            ),
        }
    }
}

#[test]
fn test_tick_to_price_error_cases() {
    let test_cases = vec![
        TickToPriceTestCase {
            tick_index: MAX_TICK + 1,
            expected_price: Decimal256::zero(),
            expected_error: Some(ContractError::TickOutOfBounds {
                tick_id: MAX_TICK + 1,
            }),
        },
        TickToPriceTestCase {
            tick_index: MIN_TICK - 1,
            expected_price: Decimal256::zero(),
            expected_error: Some(ContractError::TickOutOfBounds {
                tick_id: MIN_TICK - 1,
            }),
        },
    ];

    for test in test_cases {
        let result = tick_to_price(test.tick_index);
        assert!(result.is_err());
        if let Some(expected_err) = test.expected_error {
            assert_eq!(result.unwrap_err(), expected_err);
        }
    }
}

#[test]
fn test_pow_ten() {
    struct PowTenTestCase {
        exponent: i32,
        expected_result: Decimal256,
    }

    let test_cases = vec![
        PowTenTestCase {
            exponent: 0,
            expected_result: Decimal256::from_str("1").unwrap(),
        },
        PowTenTestCase {
            exponent: 1,
            expected_result: Decimal256::from_str("10").unwrap(),
        },
        PowTenTestCase {
            exponent: -1,
            expected_result: Decimal256::from_str("0.1").unwrap(),
        },
        PowTenTestCase {
            exponent: 5,
            expected_result: Decimal256::from_str("100000").unwrap(),
        },
        PowTenTestCase {
            exponent: -5,
            expected_result: Decimal256::from_str("0.00001").unwrap(),
        },
        PowTenTestCase {
            exponent: 10,
            expected_result: Decimal256::from_str("10000000000").unwrap(),
        },
        PowTenTestCase {
            exponent: -10,
            expected_result: Decimal256::from_str("0.0000000001").unwrap(),
        },
    ];

    for test in test_cases {
        let result = pow_ten(test.exponent).unwrap();
        assert_eq!(test.expected_result, result);
    }
}

struct OperByPriceTestCase {
    name: &'static str,
    price: Decimal256,
    amount: Uint128,
    expected_result: Uint256,
    expected_error: Option<ContractError>,
    rounding_direction: RoundingDirection,
}

#[test]
fn test_multiply_by_price() {
    let test_cases: Vec<OperByPriceTestCase> = vec![
        OperByPriceTestCase {
            name: "basic price multiplication",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::one()),
            amount: Uint128::from(10u128),
            expected_result: Uint256::from(50u128),
            expected_error: None,
            rounding_direction: RoundingDirection::Down,
        },
        OperByPriceTestCase {
            name: "basic price multiplication w/ rounding (down)",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::from_u128(100)),
            amount: Uint128::from(3u128),

            // 0.05 * 3 = 0.15, which truncates to 0
            expected_result: Uint256::zero(),
            expected_error: None,
            rounding_direction: RoundingDirection::Down,
        },
        OperByPriceTestCase {
            name: "basic price multiplication w/ rounding (up)",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::from_u128(100)),
            amount: Uint128::from(3u128),

            // 0.05 * 3 = 0.15, which truncates to 0
            expected_result: Uint256::one(),
            expected_error: None,
            rounding_direction: RoundingDirection::Up,
        },
        OperByPriceTestCase {
            name: "error overflow",
            price: Decimal256::MAX,
            amount: Uint128::MAX,
            expected_result: Uint256::from(1u128),
            expected_error: Some(ContractError::Overflow(OverflowError {
                operation: OverflowOperation::Mul,
                operand2: Decimal256::MAX.to_string(),
                operand1: Uint128::MAX.to_string(),
            })),
            rounding_direction: RoundingDirection::Down,
        },
    ];

    for test in test_cases {
        let result = multiply_by_price(test.amount, test.price, test.rounding_direction);
        if let Some(expected_error) = test.expected_error {
            assert_eq!(result.unwrap_err(), expected_error, "{}", test.name);
        } else {
            assert_eq!(result.unwrap(), test.expected_result, "{}", test.name);
        }
    }
}

#[test]
fn test_divide_by_price() {
    let test_cases: Vec<OperByPriceTestCase> = vec![
        OperByPriceTestCase {
            name: "basic price division",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::one()),
            amount: Uint128::from(10u128),
            expected_result: Uint256::from(2u128),
            expected_error: None,
            rounding_direction: RoundingDirection::Down,
        },
        OperByPriceTestCase {
            name: "basic price division w/ rounding (down)",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::one()),
            amount: Uint128::from(1u128),
            expected_result: Uint256::zero(),
            expected_error: None,
            rounding_direction: RoundingDirection::Down,
        },
        OperByPriceTestCase {
            name: "basic price division w/ rounding (up)",
            price: Decimal256::from_ratio(Uint256::from_u128(5u128), Uint256::one()),
            amount: Uint128::from(1u128),
            expected_result: Uint256::one(),
            expected_error: None,
            rounding_direction: RoundingDirection::Up,
        },
        OperByPriceTestCase {
            name: "error overflow",
            price: Decimal256::from_ratio(Uint256::one(), Uint256::MAX),
            amount: Uint128::MAX,
            expected_result: Uint256::from(1u128),
            expected_error: Some(ContractError::Overflow(OverflowError {
                operation: OverflowOperation::Mul,
                operand2: Decimal256::from_ratio(Uint256::one(), Uint256::MAX).to_string(),
                operand1: Uint128::MAX.to_string(),
            })),
            rounding_direction: RoundingDirection::Down,
        },
    ];

    for test in test_cases {
        let result = divide_by_price(test.amount, test.price, test.rounding_direction);
        if let Some(expected_error) = test.expected_error {
            assert_eq!(result.unwrap_err(), expected_error, "{}", test.name);
        } else {
            assert_eq!(result.unwrap(), test.expected_result, "{}", test.name);
        }
    }
}
