use cosmwasm_std::Decimal256;
use std::str::FromStr;

pub const MIN_TICK: i64 = -108000000;
pub const MAX_TICK: i64 = 342000000;
pub const EXPONENT_AT_PRICE_ONE: i32 = -6;
pub const GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS: i64 = 9_000_000;

// TODO: optimize this using lazy_static
pub fn max_spot_price() -> Decimal256 {
    Decimal256::from_str("100000000000000000000000000000000000000").unwrap()
}

pub fn min_spot_price() -> Decimal256 {
    Decimal256::from_str("0.000000000001").unwrap()
}