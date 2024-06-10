use cosmwasm_std::{Decimal, Decimal256};
use std::str::FromStr;

pub const MIN_TICK: i64 = -108000000;
pub const MAX_TICK: i64 = 182402823;
pub const EXPONENT_AT_PRICE_ONE: i32 = -6;
pub const GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS: i64 = 9_000_000;
// The swap fee expected by this contract
pub const EXPECTED_SWAP_FEE: Decimal = Decimal::zero();
pub const MAX_BATCH_CLAIM: u32 = 100;
pub const MAX_MAKER_FEE_PERCENTAGE: Decimal256 = Decimal256::percent(5);

// Address controlled by Osmosis governance
pub const OSMOSIS_GOV_ADDR: &str = "osmo10d07y265gmmuvt4z0w9aw880jnsr700jjeq4qp";

// Circuit breaker is set up as a DAODAO subDAO that can be found here:
// https://daodao.zone/dao/osmo1peuxfjj66n2qt2v5jmqlvzz8neakjgduez7vttvemw58uug6546sr60ngl/home
pub const CIRCUIT_BREAKER_SUBDAO_ADDR: &str =
    "osmo1peuxfjj66n2qt2v5jmqlvzz8neakjgduez7vttvemw58uug6546sr60ngl";

// By default, the maker fee recipient is set to be the same module address that
// taker fees are sent to. This can be changed by governance through the governance-controlled
// wallet set as admin.
pub const DEFAULT_MAKER_FEE_RECIPIENT: &str = "osmo1r9jc2234fljy93z80cevqjt3nmjycec8aj4cc6";

// By default, the maker fee is set to zero. This can be updated by governance.
pub const DEFAULT_MAKER_FEE: Decimal256 = Decimal256::zero();

// TODO: optimize this using lazy_static
pub fn max_spot_price() -> Decimal256 {
    Decimal256::from_str("340282300000000000000").unwrap()
}

pub fn min_spot_price() -> Decimal256 {
    Decimal256::from_str("0.000000000001").unwrap()
}
