use std::str::FromStr;

use cosmwasm_std::{ensure, Addr, Coin, Decimal, Deps};

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    error::ContractResult,
    msg::{CalcOutAmtGivenInResponse, SpotPriceResponse},
    order,
    state::ORDERBOOK,
    sudo::ensure_swap_fee,
    tick_math::tick_to_price,
    types::{MarketOrder, OrderDirection},
    ContractError,
};

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
    let (output, _, _) = order::fulfill_order(deps.storage, &mut mock_order, tick_bound)?;

    Ok(CalcOutAmtGivenInResponse { token_out: output })
}
