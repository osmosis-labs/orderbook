use cosmwasm_std::{Decimal, Decimal256};
use lazy_static::lazy_static;
use std::str::FromStr;

pub const MIN_TICK: i64 = -108000000;
pub const MAX_TICK: i64 = 182402823;
pub const EXPONENT_AT_PRICE_ONE: i32 = -6;
pub const GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS: i64 = 9_000_000;
// The swap fee expected by this contract
pub const EXPECTED_SWAP_FEE: Decimal = Decimal::zero();
pub const MAX_BATCH_CLAIM: u32 = 100;
pub const MAX_MAKER_FEE_PERCENTAGE: Decimal256 = Decimal256::percent(5);

lazy_static! {
    pub static ref MAX_SPOT_PRICE: Decimal256 =
        Decimal256::from_str("340282300000000000000").unwrap();
    pub static ref MIN_SPOT_PRICE: Decimal256 = Decimal256::from_str("0.000000000001").unwrap();
}
