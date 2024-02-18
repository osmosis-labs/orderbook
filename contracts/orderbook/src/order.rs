use crate::error::ContractError;
use crate::state::*;
use crate::state::{MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::{LimitOrder, OrderDirection, REPLY_ID_REFUND};
use cosmwasm_std::{
    coin, ensure, ensure_eq, BankMsg, DepsMut, Env, MessageInfo, Response, SubMsg, Uint128,
};
use cw_utils::{must_pay, nonpayable};

#[allow(clippy::manual_range_contains)]
pub fn place_limit(
    deps: DepsMut,
    env: Env,
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
    let expected_denom = orderbook.get_expected_denom_for_direction(&order_direction);

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

    // Save the order to the orderbook
    orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

    // Update tick liquidity
    TICK_LIQUIDITY.update(deps.storage, &(book_id, tick_id), |liquidity| {
        Ok::<Uint128, ContractError>(liquidity.unwrap_or_default().checked_add(quantity)?)
    })?;

    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: env.contract.address.to_string(),
            amount: vec![coin(quantity.u128(), expected_denom)],
        })
        .add_attribute("method", "placeLimit")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("book_id", book_id.to_string())
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("order_direction", format!("{order_direction:?}"))
        .add_attribute("quantity", quantity.to_string()))
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

    // Remove order
    orders().remove(deps.storage, &key)?;

    // Update tick liquidity
    TICK_LIQUIDITY.update(deps.storage, &(book_id, tick_id), |liquidity| {
        Ok::<Uint128, ContractError>(liquidity.unwrap_or_default().checked_sub(order.quantity)?)
    })?;

    // Get orderbook info for correct denomination
    let orderbook =
        ORDERBOOKS
            .may_load(deps.storage, &order.book_id)?
            .ok_or(ContractError::InvalidBookId {
                book_id: order.book_id,
            })?;

    // Generate refund
    let expected_denom = orderbook.get_expected_denom_for_direction(&order.order_direction);
    let coin_to_send = coin(order.quantity.u128(), expected_denom);
    let refund_msg = SubMsg::reply_on_error(
        BankMsg::Send {
            to_address: order.owner.to_string(),
            amount: vec![coin_to_send],
        },
        REPLY_ID_REFUND,
    );

    Ok(Response::new()
        .add_attribute("method", "cancelLimit")
        .add_attribute("owner", info.sender)
        .add_attribute("book_id", book_id.to_string())
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_submessage(refund_msg))
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
