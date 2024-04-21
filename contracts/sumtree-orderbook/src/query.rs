use std::str::FromStr;

use cosmwasm_std::{coin, ensure, Addr, Coin, Decimal, Deps, Order, Uint128};
use cw_storage_plus::Bound;

use crate::{
    constants::{ALL_TICKS_DEFAULT_LIMIT, ALL_TICKS_MAX_LIMIT, MAX_TICK, MIN_TICK},
    error::ContractResult,
    msg::{
        AllTicksResponse, CalcOutAmtGivenInResponse, GetTotalPoolLiquidityResponse,
        SpotPriceResponse, TickIdAndState,
    },
    order,
    state::{ORDERBOOK, TICK_STATE},
    sudo::ensure_swap_fee,
    tick_math::tick_to_price,
    types::{MarketOrder, OrderDirection},
    ContractError,
};

/// Calculates the spot price given current orderbook state.
///
/// Spot price is calculated by taking the best available price (next tick) with liquidity for the opposite direction of the order.
/// i.e. if the order direction is bid, the spot price is the price of the next ask tick with ask liquidity, and if the order direction is bid,
/// the spot price is the price of the next bid tick with bid liquidity.
///
/// Errors if:
/// 1. Provided denoms are the same
/// 2. One or more of the provided denoms are not supported by the orderbook
pub(crate) fn spot_price(
    deps: Deps,
    quote_asset_denom: String,
    base_asset_denom: String,
) -> ContractResult<SpotPriceResponse> {
    // Ensure provided denoms do not match
    ensure!(
        quote_asset_denom != base_asset_denom,
        ContractError::InvalidPair {
            token_in_denom: quote_asset_denom,
            token_out_denom: base_asset_denom
        }
    );

    // Fetch orderbook to retrieve tick info
    let orderbook = ORDERBOOK.load(deps.storage)?;
    // Determine the order direction by denom pairing
    let direction = orderbook.direction_from_pair(quote_asset_denom, base_asset_denom)?;

    // Determine next tick based on desired order direction
    let next_tick = match direction {
        OrderDirection::Ask => orderbook.next_bid_tick,
        OrderDirection::Bid => orderbook.next_ask_tick,
    };

    // Generate spot price based on current active tick for desired order direction
    let price = tick_to_price(next_tick)?;

    Ok(SpotPriceResponse {
        spot_price: Decimal::from_str(&price.to_string())?,
    })
}

/// Calculates the output amount given the input amount for the current orderbook state.
///
/// Output is calculated by generating a mock market order, the direction of which is dependent on the order of the input/output denoms versus what the orderbook expects.
/// The mock order is then filled against the current orderbook state, and the output amount is the result of the fill.
///
/// Errors if:
/// 1. The provided swap fee does not match the orderbook's expected swap fee, which is set to zero.
/// 2. The provided denom pair is not supported by the orderbook
pub(crate) fn calc_out_amount_given_in(
    deps: Deps,
    token_in: Coin,
    token_out_denom: String,
    swap_fee: Decimal,
) -> ContractResult<CalcOutAmtGivenInResponse> {
    // Ensure the provided swap fee matches what the orderbook expects
    ensure_swap_fee(swap_fee)?;

    // Fetch orderbook
    let orderbook = ORDERBOOK.load(deps.storage)?;
    // Determine order direction
    let direction = orderbook.direction_from_pair(token_in.denom, token_out_denom)?;

    let tick_bound = match direction {
        OrderDirection::Bid => MAX_TICK,
        OrderDirection::Ask => MIN_TICK,
    };

    // Generate mock order for query
    let mut mock_order = MarketOrder::new(token_in.amount, direction, Addr::unchecked("querier"));

    // Generate output coin given the input order by simulating a fill against current orderbook state
    let order::PostMarketOrderState { output, .. } =
        order::run_market_order_internal(deps.storage, &mut mock_order, tick_bound)?;

    Ok(CalcOutAmtGivenInResponse { token_out: output })
}

/// Calculates the total pool liquidity for the current orderbook state.
///
/// Total pool liquidity is calculated by summing the total amount of liquidity in each active tick.
///
/// Errors if:
/// 1. Summing total liquidity overflows Uint128
pub(crate) fn total_pool_liquidity(deps: Deps) -> ContractResult<GetTotalPoolLiquidityResponse> {
    let orderbook = ORDERBOOK.load(deps.storage)?;

    // Create tracking variables for both denoms
    let mut ask_amount = coin(0u128, orderbook.base_denom);
    let mut bid_amount = coin(0u128, orderbook.quote_denom);
    // Fetch all ticks from state
    let all_ticks = TICK_STATE.keys(deps.storage, None, None, Order::Ascending);

    // Iterate over each tick
    for maybe_tick_id in all_ticks {
        let tick_id = maybe_tick_id?;
        let tick = TICK_STATE.load(deps.storage, tick_id)?;

        // Increment the ask amount by the total ask liquidity in this tick
        let ask_values = tick.get_values(OrderDirection::Ask);
        ask_amount.amount = ask_amount.amount.checked_add(Uint128::try_from(
            ask_values.total_amount_of_liquidity.to_uint_floor(),
        )?)?;

        // Increment the bid amount by the total bid liquidity in this tick
        let bid_values = tick.get_values(OrderDirection::Bid);
        bid_amount.amount = bid_amount.amount.checked_add(Uint128::try_from(
            bid_values.total_amount_of_liquidity.to_uint_floor(),
        )?)?;
    }

    // May return 0 amounts if there is no liquidity in the orderbook
    Ok(GetTotalPoolLiquidityResponse {
        total_pool_liquidity: vec![ask_amount, bid_amount],
    })
}

/// Returns all active ticks in the orderbook.
pub(crate) fn all_ticks(
    deps: Deps,
    start_after: Option<i64>,
    end_at: Option<i64>,
    limit: Option<usize>,
) -> ContractResult<AllTicksResponse> {
    // Fetch all tick IDs
    let all_ticks = TICK_STATE
        .keys(
            deps.storage,
            start_after.map(Bound::inclusive),
            end_at.map(Bound::inclusive),
            Order::Ascending,
        )
        .take(
            limit
                .unwrap_or(ALL_TICKS_DEFAULT_LIMIT)
                .min(ALL_TICKS_MAX_LIMIT),
        );

    // Map tick IDs to tick states
    let all_tick_states: Vec<TickIdAndState> = all_ticks
        .map(|tick_id| {
            let tick_id = tick_id.unwrap();
            let tick_state = TICK_STATE.load(deps.storage, tick_id).unwrap();
            TickIdAndState {
                tick_id,
                tick_state,
            }
        })
        .collect();

    Ok(AllTicksResponse {
        ticks: all_tick_states,
    })
}
