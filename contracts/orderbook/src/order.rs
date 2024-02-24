use crate::error::ContractError;
use crate::state::*;
use crate::state::{MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::{Fulfillment, LimitOrder, MarketOrder, OrderDirection, REPLY_ID_REFUND};
use cosmwasm_std::{
    coin, ensure, ensure_eq, ensure_ne, BankMsg, Decimal, DepsMut, Env, MessageInfo, Order,
    Response, Storage, SubMsg, Uint128,
};
use cw_storage_plus::Bound;
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

    // Save the order to the orderbook
    orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

    // Update tick liquidity
    TICK_LIQUIDITY.update(deps.storage, &(book_id, tick_id), |liquidity| {
        Ok::<Uint128, ContractError>(liquidity.unwrap_or_default().checked_add(quantity)?)
    })?;

    Ok(Response::new()
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
    reduce_tick_liquidity(deps.storage, book_id, tick_id, order.quantity)?;

    // Get orderbook info for correct denomination
    let orderbook =
        ORDERBOOKS
            .may_load(deps.storage, &order.book_id)?
            .ok_or(ContractError::InvalidBookId {
                book_id: order.book_id,
            })?;

    // Generate refund
    let expected_denom = orderbook.get_expected_denom(&order.order_direction);
    let refund_msg = SubMsg::reply_on_error(
        BankMsg::Send {
            to_address: order.owner.to_string(),
            amount: vec![coin(order.quantity.u128(), expected_denom)],
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

#[allow(clippy::manual_range_contains)]
pub fn run_market_order(
    storage: &mut dyn Storage,
    order: &mut MarketOrder,
    tick_bound: Option<i64>,
) -> Result<(Vec<Fulfillment>, BankMsg), ContractError> {
    let mut fulfillments: Vec<Fulfillment> = vec![];
    let mut amount_fulfilled: Uint128 = Uint128::zero();
    let orderbook = ORDERBOOKS.load(storage, &order.book_id)?;
    let placed_order_denom = orderbook.get_expected_denom(&order.order_direction);

    let (min_tick, max_tick, ordering) = match order.order_direction {
        OrderDirection::Ask => {
            if let Some(tick_bound) = tick_bound {
                ensure!(
                    tick_bound <= orderbook.next_bid_tick
                        && tick_bound <= MAX_TICK
                        && tick_bound >= MIN_TICK,
                    ContractError::InvalidTickId {
                        tick_id: tick_bound
                    }
                );
            }
            (tick_bound, Some(orderbook.next_bid_tick), Order::Descending)
        }
        OrderDirection::Bid => {
            if let Some(tick_bound) = tick_bound {
                ensure!(
                    tick_bound >= orderbook.next_ask_tick
                        && tick_bound <= MAX_TICK
                        && tick_bound >= MIN_TICK,
                    ContractError::InvalidTickId {
                        tick_id: tick_bound
                    }
                );
            }
            (Some(orderbook.next_ask_tick), tick_bound, Order::Ascending)
        }
    };

    // Create ticks iterator between first tick and requested tick
    let ticks = TICK_LIQUIDITY.prefix(order.book_id).range(
        storage,
        min_tick.map(Bound::inclusive),
        max_tick.map(Bound::inclusive),
        ordering,
    );

    for maybe_current_tick in ticks {
        let current_tick = maybe_current_tick?.0;

        // Create orders iterator for all orders on current tick
        let tick_orders = orders().prefix((order.book_id, current_tick)).range(
            storage,
            None,
            None,
            Order::Ascending,
        );

        for maybe_current_order in tick_orders {
            let current_order = maybe_current_order?.1;
            ensure_ne!(
                current_order.order_direction,
                order.order_direction,
                ContractError::MismatchedOrderDirection {}
            );
            let fill_quantity = order.quantity.min(current_order.quantity);
            // Add to total amount fulfilled from placed order
            amount_fulfilled = amount_fulfilled.checked_add(fill_quantity)?;
            // Generate fulfillment for current order
            let fulfillment = Fulfillment::new(current_order, fill_quantity);
            fulfillments.push(fulfillment);

            // Update remaining order quantity
            order.quantity = order.quantity.checked_sub(fill_quantity)?;
            // TODO: Price detection
            if order.quantity.is_zero() {
                return Ok((
                    fulfillments,
                    BankMsg::Send {
                        to_address: order.owner.to_string(),
                        amount: vec![coin(amount_fulfilled.u128(), placed_order_denom)],
                    },
                ));
            }
        }

        // TODO: Price detection
        if order.quantity.is_zero() {
            return Ok((
                fulfillments,
                BankMsg::Send {
                    to_address: order.owner.to_string(),
                    amount: vec![coin(amount_fulfilled.u128(), placed_order_denom)],
                },
            ));
        }
    }

    // TODO: Price detection
    Ok((
        fulfillments,
        BankMsg::Send {
            to_address: order.owner.to_string(),
            amount: vec![coin(amount_fulfilled.u128(), placed_order_denom)],
        },
    ))
}

pub fn resolve_fulfillments(
    storage: &mut dyn Storage,
    fulfillments: Vec<Fulfillment>,
) -> Result<Vec<BankMsg>, ContractError> {
    let mut msgs: Vec<BankMsg> = vec![];
    let orderbook = ORDERBOOKS.load(storage, &fulfillments[0].order.book_id)?;
    for mut fulfillment in fulfillments {
        ensure_eq!(
            fulfillment.order.book_id,
            orderbook.book_id,
            // TODO: Error not expressive
            ContractError::InvalidFulfillment {
                order_id: fulfillment.order.order_id,
                book_id: fulfillment.order.book_id,
                amount_required: fulfillment.amount,
                amount_remaining: fulfillment.order.quantity,
                reason: Some("Fulfillment is part of another order book".to_string()),
            }
        );
        let denom = orderbook.get_expected_denom(&fulfillment.order.order_direction);
        // TODO: Add price detection for tick
        let msg = fulfillment
            .order
            .fill(&denom, fulfillment.amount, Decimal::one())?;
        msgs.push(msg);
        if fulfillment.order.quantity.is_zero() {
            orders().remove(
                storage,
                &(
                    fulfillment.order.book_id,
                    fulfillment.order.tick_id,
                    fulfillment.order.order_id,
                ),
            )?;
        } else {
            orders().save(
                storage,
                &(
                    fulfillment.order.book_id,
                    fulfillment.order.tick_id,
                    fulfillment.order.order_id,
                ),
                &fulfillment.order,
            )?;
        }
        // TODO: possible optimization by grouping tick/liquidity and calling this once per tick?
        reduce_tick_liquidity(
            storage,
            fulfillment.order.book_id,
            fulfillment.order.tick_id,
            fulfillment.amount,
        )?;
    }
    Ok(msgs)
}
