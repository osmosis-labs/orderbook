use crate::error::ContractError;
use crate::state::*;
use crate::state::{MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::{LimitOrder, OrderDirection};
use cosmwasm_std::{coin, BankMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use cw_utils::must_pay;

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
    if tick_id < MIN_TICK || tick_id > MAX_TICK {
        return Err(ContractError::InvalidTickId { tick_id });
    }

    // Validate order_quantity is positive
    if quantity <= Uint128::zero() {
        return Err(ContractError::InvalidQuantity { quantity });
    }

    // Determine the correct denom based on order direction
    let expected_denom = match order_direction {
        OrderDirection::Bid => orderbook.quote_denom,
        OrderDirection::Ask => orderbook.base_denom,
    };

    // Verify the funds sent with the message match the `quantity` for the correct denom
    // We reject any quantity that is not exactly equal to the amount in the limit order being placed
    let received = must_pay(&info, &expected_denom)?;
    if received != quantity {
        return Err(ContractError::InsufficientFunds {
            balance: received,
            required: quantity,
        });
    }

    // Generate a new order ID
    let order_id = new_order_id(deps.storage)?;

    let limit_order = LimitOrder::new(
        book_id,
        tick_id,
        order_id,
        order_direction.clone(),
        info.sender.clone(),
        quantity,
    );

    // Save the order to the orderbook
    orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

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
        .add_attribute("order_direction", format!("{:?}", order_direction))
        .add_attribute("quantity", quantity.to_string()))
}

pub fn cancel_limit(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: Implement cancel_limit

    Ok(Response::new()
        .add_attribute("method", "cancelLimit")
        .add_attribute("owner", info.sender))
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
