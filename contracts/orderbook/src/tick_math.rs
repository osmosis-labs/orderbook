use crate::state::{
    EXPONENT_AT_PRICE_ONE, GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS, MAX_TICK, MIN_TICK,
};
use cosmwasm_std::{Decimal, Uint128};

#[derive(Debug)]
pub enum TickPriceError {
    BelowMin,
    AboveMax,
    OutOfBounds,
}

pub fn tick_to_price(tick_index: i64) -> Result<Decimal, TickPriceError> {
    if tick_index == 0 {
        return Ok(Decimal::one());
    }

    if tick_index < MIN_TICK {
        return Err(TickPriceError::BelowMin);
    } else if tick_index > MAX_TICK {
        return Err(TickPriceError::AboveMax);
    }

    let geometric_exponent_delta: i64 = tick_index / GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS;
    let mut exponent_at_current_tick = (EXPONENT_AT_PRICE_ONE as i64) + geometric_exponent_delta;

    // We must decrement the exponentAtCurrentTick when entering the negative tick range in order to constantly step up in precision when going further down in ticks
    // Otherwise, from tick 0 to tick -(geometricExponentIncrementDistanceInTicks), we would use the same exponent as the exponentAtPriceOne
    if tick_index < 0 {
        exponent_at_current_tick -= 1;
    }

    let current_additive_increment_in_ticks =
        Decimal::from_ratio(10u128.pow(exponent_at_current_tick as u32), Uint128::one());
    let num_additive_ticks =
        tick_index - (geometric_exponent_delta * GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS);
    let price = Decimal::from_ratio(10u128.pow(geometric_exponent_delta as u32), Uint128::one())
        + Decimal::from_ratio(num_additive_ticks as u128, Uint128::one())
            * current_additive_increment_in_ticks;

    Ok(price)
}
