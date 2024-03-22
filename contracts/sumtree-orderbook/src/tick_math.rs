use crate::constants::{
    EXPONENT_AT_PRICE_ONE, GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS, MAX_TICK, MIN_TICK,
};
use crate::error::*;
use crate::types::OrderDirection;
use cosmwasm_std::{ensure, Decimal256, Uint128, Uint256};

// tick_to_price converts a tick index to a price.
// If tick_index is zero, the function returns Decimal256::one().
// Errors if the given tick is outside of the bounds allowed by MIN_TICK and MAX_TICK.
#[allow(clippy::manual_range_contains)]
pub fn tick_to_price(tick_index: i64) -> ContractResult<Decimal256> {
    if tick_index == 0 {
        return Ok(Decimal256::one());
    }

    ensure!(
        tick_index >= MIN_TICK && tick_index <= MAX_TICK,
        ContractError::TickOutOfBounds {
            tick_id: tick_index
        }
    );

    // geometric_exponent_delta is the number of times we have incremented the exponent by
    // GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS to reach the current tick index.
    let geometric_exponent_delta: i64 = tick_index / GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS;

    // The exponent at the current tick is the exponent at price one plus the number of times we have incremented the exponent by
    let mut exponent_at_current_tick = (EXPONENT_AT_PRICE_ONE as i64) + geometric_exponent_delta;

    // We must decrement the exponentAtCurrentTick when entering the negative tick range in order to constantly step up in precision when going further down in ticks
    // Otherwise, from tick 0 to tick -(geometricExponentIncrementDistanceInTicks), we would use the same exponent as the exponentAtPriceOne
    if tick_index < 0 {
        exponent_at_current_tick -= 1;
    }

    // We can derive the contribution of each additive tick with 10^(exponent_at_current_tick))
    let current_additive_increment_in_ticks = pow_ten(exponent_at_current_tick as i32)?;

    // The current number of additive ticks are equivalent to the portion of the tick index that is not covered by the geometric component.
    let num_additive_ticks =
        tick_index - (geometric_exponent_delta * GEOMETRIC_EXPONENT_INCREMENT_DISTANCE_IN_TICKS);

    // Price is equal to the sum of the geometric and additive components.
    // Since we derive `geometric_exponent_delta` by division with truncation, we can get the geometric component
    // by simply taking 10^(geometric_exponent_delta).
    //
    // The additive component is simply the number of additive ticks by the current additive increment per tick.
    let geometric_component = pow_ten(geometric_exponent_delta as i32)?;
    let additive_component = Decimal256::from_ratio(
        Uint256::from(num_additive_ticks.unsigned_abs()),
        Uint256::one(),
    )
    .checked_mul(current_additive_increment_in_ticks)?;

    // We manually handle sign here to avoid expensive conversions between Decimal256 and SignedDecimal256.
    let price = if num_additive_ticks < 0 {
        geometric_component.checked_sub(additive_component)
    } else {
        geometric_component.checked_add(additive_component)
    }?;

    Ok(price)
}

// Takes an exponent and returns 10^exponent. Supports negative exponents.
pub fn pow_ten(expo: i32) -> ContractResult<Decimal256> {
    let target_expo = Uint256::from(10u8).checked_pow(expo.unsigned_abs())?;
    if expo < 0 {
        Ok(Decimal256::checked_from_ratio(Uint256::one(), target_expo)?)
    } else {
        let res = Uint256::one().checked_mul(target_expo)?;
        Ok(Decimal256::from_ratio(res, Uint256::one()))
    }
}

// Multiplies a given tick amount by the price for that tick
pub fn multiply_by_price(
    amount: Decimal256,
    price: Decimal256,
    round_up: bool,
) -> ContractResult<Uint128> {
    // Multiply amount by the price
    // TODO: need to handle rounding here (currently does either bankers or floor)
    let amount_to_send_d256 = price.checked_mul(amount)?;

    // Convert to Uint256 with proper rounding
    let amount_to_send_u256 = if round_up {
        amount_to_send_d256.to_uint_ceil()
    } else {
        amount_to_send_d256.to_uint_floor()
    };

    // Run checked conversion to Uint128
    let amount_to_send = Uint128::try_from(amount_to_send_u256).unwrap();

    Ok(amount_to_send)
}

// Divides a given tick amount by the price for that tick
pub fn divide_by_price(
    amount: Decimal256,
    price: Decimal256,
    round_up: bool,
) -> ContractResult<Uint128> {
    // Divide amount by the price
    // TODO: need to handle rounding here (currently does either bankers or floor)
    let amount_to_send_d256 = amount.checked_div(price)?;

    // Convert to Uint256 with proper rounding
    let amount_to_send_u256 = if round_up {
        amount_to_send_d256.to_uint_ceil()
    } else {
        amount_to_send_d256.to_uint_floor()
    };

    // Run checked conversion to Uint128
    let amount_to_send = Uint128::try_from(amount_to_send_u256).unwrap();

    Ok(amount_to_send)
}

/// Converts a tick amount to it's value given a price and order direction
pub fn amount_to_value(
    order: OrderDirection,
    amount: Decimal256,
    price: Decimal256,
) -> ContractResult<Uint128> {
    // TODO: vet rounding direction and review internal math rounding
    let round_up = order == OrderDirection::Ask;
    match order {
        OrderDirection::Bid => multiply_by_price(amount, price, round_up),
        OrderDirection::Ask => divide_by_price(amount, price, round_up),
    }
}
