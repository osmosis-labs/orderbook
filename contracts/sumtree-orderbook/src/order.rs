use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractError;
use crate::state::{new_order_id, orders, ORDERBOOKS, TICK_STATE};
use crate::types::{LimitOrder, OrderDirection, TickState};
use cosmwasm_std::{
    ensure, ensure_eq, Decimal256, DepsMut, Env, MessageInfo, Response, Uint128, Uint256,
};
use cw_utils::{must_pay, nonpayable};

#[allow(clippy::manual_range_contains)]
pub fn place_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    book_id: u64,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
) -> Result<Response, ContractError> {
    // Validate book_id exists
    let orderbook = ORDERBOOKS
        .load(deps.storage, &book_id)
        .map_err(|_| ContractError::InvalidBookId { book_id })?;

    // Validate tick_id is within valid range
    ensure!(
        tick_id >= MIN_TICK && tick_id <= MAX_TICK,
        ContractError::InvalidTickId { tick_id }
    );

    // Ensure order_quantity is positive
    ensure!(
        quantity > Uint128::zero(),
        ContractError::InvalidQuantity { quantity }
    );

    // Determine the correct denom based on order direction
    let expected_denom = orderbook.get_expected_denom(&order_direction);

    // Verify the funds sent with the message match the `quantity` for the correct denom
    // We reject any quantity that is not exactly equal to the amount in the limit order being placed
    let received = must_pay(&info, &expected_denom)?;
    ensure_eq!(
        received,
        quantity,
        ContractError::InsufficientFunds {
            sent: received,
            required: quantity,
        }
    );

    // Generate a new order ID
    let order_id = new_order_id(deps.storage)?;

    // Build limit order
    let limit_order = LimitOrder::new(
        book_id,
        tick_id,
        order_id,
        order_direction,
        info.sender.clone(),
        quantity,
    );

    // Determine if the order needs to be filled
    let should_fill = match order_direction {
        OrderDirection::Ask => tick_id <= orderbook.next_bid_tick,
        OrderDirection::Bid => tick_id >= orderbook.next_ask_tick,
    };

    let response = Response::default();
    // Run order fill if criteria met
    if should_fill {
        todo!()
    }

    // Only save the order if not fully filled
    if limit_order.quantity > Uint128::zero() {
        // Save the order to the orderbook
        orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;
    }

    let quantity_fullfilled = quantity.checked_sub(limit_order.quantity)?;

    // Update tick liquidity
    TICK_STATE.update(deps.storage, &(book_id, tick_id), |state| {
        let mut curr_state = state.unwrap_or_default();
        // Increment total available liquidity by remaining quantity in order post fill
        curr_state.total_amount_of_liquidity = curr_state
            .total_amount_of_liquidity
            .checked_add(Decimal256::from_ratio(
                limit_order.quantity.u128(),
                Uint256::one(),
            ))
            .unwrap();
        // Increment amount swapped by amount fulfilled for order
        curr_state.effective_total_amount_swapped = curr_state
            .effective_total_amount_swapped
            .checked_add(Decimal256::from_ratio(quantity_fullfilled, Uint256::one()))?;
        // TODO: Unsure of what this value is for?
        curr_state.cumulative_total_limits = curr_state
            .cumulative_total_limits
            .checked_add(Decimal256::one())?;

        Ok::<TickState, ContractError>(curr_state)
    })?;

    Ok(response
        .add_attribute("method", "placeLimit")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("book_id", book_id.to_string())
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("order_direction", format!("{order_direction:?}"))
        .add_attribute("quantity", quantity.to_string())
        .add_attribute("quantity_fulfilled", quantity_fullfilled))
}

pub fn cancel_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    book_id: u64,
    tick_id: i64,
    order_id: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let key = (book_id, tick_id, order_id);
    // Check for the order, error if not found
    let order = orders()
        .may_load(deps.storage, &key)?
        .ok_or(ContractError::OrderNotFound {
            book_id,
            tick_id,
            order_id,
        })?;

    // Ensure the sender is the order owner
    ensure_eq!(info.sender, order.owner, ContractError::Unauthorized {});

    todo!();
}

pub fn place_market(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: Implement place_market

    Ok(Response::new()
        .add_attribute("method", "placeMarket")
        .add_attribute("owner", info.sender))
}
