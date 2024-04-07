use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractError;
use crate::state::{new_order_id, orders, ORDERBOOKS, TICK_STATE};
use crate::sumtree::node::{generate_node_id, NodeType, TreeNode};
use crate::sumtree::tree::get_or_init_root_node;
use crate::tick_math::{amount_to_value, tick_to_price, RoundingDirection};
use crate::types::{LimitOrder, MarketOrder, OrderDirection, TickState, REPLY_ID_REFUND};
use cosmwasm_std::{
    coin, ensure, ensure_eq, BankMsg, Decimal256, DepsMut, Env, MessageInfo, Order, Response,
    Storage, SubMsg, Uint128, Uint256,
};
use cw_storage_plus::Bound;
use cw_utils::{must_pay, nonpayable};

#[allow(clippy::manual_range_contains)]
pub fn place_limit(
    deps: &mut DepsMut,
    _env: Env,
    info: MessageInfo,
    book_id: u64,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
) -> Result<Response, ContractError> {
    // Validate book_id exists
    let mut orderbook = ORDERBOOKS
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

    // If bid and tick_id is higher than next bid tick, update next bid tick
    // If ask and tick_id is lower than next ask tick, update next ask tick
    match order_direction {
        OrderDirection::Bid => {
            if tick_id > orderbook.next_bid_tick {
                orderbook.next_bid_tick = tick_id;
            }
        }
        OrderDirection::Ask => {
            if tick_id < orderbook.next_ask_tick {
                orderbook.next_ask_tick = tick_id;
            }
        }
    }
    ORDERBOOKS.save(deps.storage, &book_id, &orderbook)?;

    // Update ETAS from Tick State
    let mut tick_state = TICK_STATE
        .load(deps.storage, &(book_id, tick_id))
        .unwrap_or_default();
    let mut tick_values = tick_state.get_values(order_direction);

    // Build limit order
    let mut limit_order = LimitOrder::new(
        book_id,
        tick_id,
        order_id,
        order_direction,
        info.sender.clone(),
        quantity,
        tick_values.cumulative_total_value,
    );

    // Determine if the order needs to be filled
    let should_fill = match order_direction {
        OrderDirection::Ask => tick_id <= orderbook.next_bid_tick,
        OrderDirection::Bid => tick_id >= orderbook.next_ask_tick,
    };

    let mut response = Response::default();
    // Run order fill if criteria met
    if should_fill {
        let mut market_order = MarketOrder::from(limit_order.clone());
        let tick_bound = match market_order.order_direction {
            OrderDirection::Bid => MAX_TICK,
            OrderDirection::Ask => MIN_TICK,
        };
        let (_, fill_msg) = run_market_order(deps.storage, &mut market_order, tick_bound)?;
        response = response.add_submessage(SubMsg::reply_on_error(fill_msg, 1));

        limit_order.quantity = market_order.quantity;
    }

    let quantity_fullfilled = quantity.checked_sub(limit_order.quantity)?;

    // Only save the order if not fully filled
    if limit_order.quantity > Uint128::zero() {
        // Save the order to the orderbook
        orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

        tick_values.total_amount_of_liquidity = tick_values
            .total_amount_of_liquidity
            .checked_add(Decimal256::from_ratio(
                limit_order.quantity.u128(),
                Uint256::one(),
            ))
            .unwrap();
    }

    tick_values.cumulative_total_value = tick_values
        .cumulative_total_value
        .checked_add(Decimal256::from_ratio(quantity, Uint256::one()))?;

    tick_state.set_values(order_direction, tick_values);
    TICK_STATE.save(deps.storage, &(book_id, tick_id), &tick_state)?;

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

    // Ensure the order has not been filled.
    // TODO: support cancelling partially filled orders by claiming above
    // if a partial fill is detected. Tracked in issue https://github.com/osmosis-labs/orderbook/issues/75
    let tick_state = TICK_STATE
        .load(deps.storage, &(book_id, tick_id))
        .unwrap_or_default();
    let tick_values = tick_state.get_values(order.order_direction);
    ensure!(
        tick_values.effective_total_amount_swapped <= order.etas,
        ContractError::CancelFilledOrder
    );

    // Fetch the sumtree from storage, or create one if it does not exist
    let mut tree = get_or_init_root_node(deps.storage, book_id, tick_id, order.order_direction)?;

    // Generate info for new node to insert to sumtree
    let node_id = generate_node_id(deps.storage, order.book_id, order.tick_id)?;
    let mut curr_tick_state = TICK_STATE
        .load(deps.storage, &(order.book_id, order.tick_id))
        .ok()
        .ok_or(ContractError::InvalidTickId {
            tick_id: order.tick_id,
        })?;
    let mut curr_tick_values = curr_tick_state.get_values(order.order_direction);

    let mut new_node = TreeNode::new(
        order.book_id,
        order.tick_id,
        order.order_direction,
        node_id,
        NodeType::leaf(
            order.etas,
            Decimal256::from_ratio(Uint256::from_uint128(order.quantity), Uint256::one()),
        ),
    );

    // Insert new node
    tree.insert(deps.storage, &mut new_node)?;

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

    orders().remove(
        deps.storage,
        &(order.book_id, order.tick_id, order.order_id),
    )?;

    curr_tick_values.total_amount_of_liquidity = curr_tick_values
        .total_amount_of_liquidity
        .checked_sub(Decimal256::from_ratio(order.quantity, Uint256::one()))?;
    curr_tick_state.set_values(order.order_direction, curr_tick_values);
    TICK_STATE.save(
        deps.storage,
        &(order.book_id, order.tick_id),
        &curr_tick_state,
    )?;

    tree.save(deps.storage)?;

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

// run_market_order processes a market order from the current active tick on the order's orderbook
// up to the passed in `tick_bound`. This allows for this function to be useful both for regular
// market orders and as a helper to partially fill any limit orders that are placed past the best
// current price on an orderbook.
//
// Note that this mutates the `order` object, so in the case where this function is used to partially
// fill a limit order, it should leave the order in a valid and up-to-date state to be placed on the
// orderbook.
//
// Returns:
// * The output after the order has been processed
// * Bank send message to process the balance transfer
//
// Returns error if:
// * Orderbook with given ID doesn't exist (order.book_id)
// * Tick to price conversion fails for any tick
//
// CONTRACT: The caller must ensure that the necessary input funds were actually supplied.
#[allow(clippy::manual_range_contains)]
pub fn run_market_order(
    storage: &mut dyn Storage,
    order: &mut MarketOrder,
    tick_bound: i64,
) -> Result<(Uint128, BankMsg), ContractError> {
    let mut orderbook =
        ORDERBOOKS
            .load(storage, &order.book_id)
            .map_err(|_| ContractError::InvalidBookId {
                book_id: order.book_id,
            })?;
    let output_denom = orderbook.get_opposite_denom(&order.order_direction);

    // Ensure the given tick bound is within global limits
    ensure!(
        tick_bound <= MAX_TICK && tick_bound >= MIN_TICK,
        ContractError::InvalidTickId {
            tick_id: tick_bound
        }
    );

    // Derive appropriate bounds for tick iterator based on order direction:
    // * If the order is an Ask, we iterate from [next_bid_tick, tick_bound] in descending order.
    // * If the order is a Bid, we iterate from [tick_bound, next_ask_tick] in ascending order.
    let (min_tick, max_tick, ordering) = match order.order_direction {
        OrderDirection::Ask => {
            ensure!(
                tick_bound <= orderbook.next_bid_tick,
                ContractError::InvalidTickId {
                    tick_id: tick_bound
                }
            );
            (tick_bound, orderbook.next_bid_tick, Order::Descending)
        }
        OrderDirection::Bid => {
            ensure!(
                tick_bound >= orderbook.next_ask_tick,
                ContractError::InvalidTickId {
                    tick_id: tick_bound
                }
            );
            (orderbook.next_ask_tick, tick_bound, Order::Ascending)
        }
    };

    // Create tick iterator between first tick and requested tick
    let ticks = TICK_STATE.prefix(order.book_id).range(
        storage,
        Some(min_tick).map(Bound::inclusive),
        Some(max_tick).map(Bound::inclusive),
        ordering,
    );

    // Iterate through ticks and fill the market order as appropriate.
    // Due to our sumtree-based design, this process carries only O(1) overhead per tick.
    let mut total_output: Uint128 = Uint128::zero();
    let mut tick_updates: Vec<(i64, TickState)> = Vec::new();
    for maybe_current_tick in ticks {
        let (current_tick_id, mut current_tick) = maybe_current_tick?;
        let mut current_tick_values = current_tick.get_values(order.order_direction.opposite());
        let tick_price = tick_to_price(current_tick_id)?;

        // Update current tick pointer as we visit ticks
        match order.order_direction.opposite() {
            OrderDirection::Ask => orderbook.next_ask_tick = current_tick_id,
            OrderDirection::Bid => orderbook.next_bid_tick = current_tick_id,
        }

        // Early exit if order filled
        if order.quantity.is_zero() {
            break;
        }

        let output_quantity = amount_to_value(
            order.order_direction,
            order.quantity,
            tick_price,
            RoundingDirection::Down,
        )?;

        let output_quantity_dec =
            Decimal256::from_ratio(Uint256::from_uint128(output_quantity), Uint256::one());

        // If order quantity is less than the current tick's liquidity, fill the whole order.
        // Otherwise, fill the whole tick.
        let fill_amount_dec = if output_quantity_dec < current_tick_values.total_amount_of_liquidity
        {
            output_quantity_dec
        } else {
            current_tick_values.total_amount_of_liquidity
        };

        // Update tick and order state to process the fill
        current_tick_values.total_amount_of_liquidity = current_tick_values
            .total_amount_of_liquidity
            .checked_sub(fill_amount_dec)?;

        current_tick_values.effective_total_amount_swapped = current_tick_values
            .effective_total_amount_swapped
            .checked_add(fill_amount_dec)?;

        // Note: this conversion errors if fill_amount_dec does not fit into Uint128
        // By the time we get here, this should not be possible.
        let fill_amount = Uint128::try_from(fill_amount_dec.to_uint_floor())?;

        let input_filled = amount_to_value(
            order.order_direction.opposite(),
            fill_amount,
            tick_price,
            RoundingDirection::Up,
        )?;
        order.quantity = order.quantity.checked_sub(input_filled)?;

        current_tick.set_values(order.order_direction.opposite(), current_tick_values);
        // Add the updated tick state to the vector
        tick_updates.push((current_tick_id, current_tick));

        total_output = total_output.checked_add(fill_amount)?;
    }

    // After the core tick iteration loop, write all tick updates to state.
    // We cannot do this during the loop due to the borrow checker.
    for (tick_id, tick_state) in tick_updates {
        TICK_STATE.save(storage, &(order.book_id, tick_id), &tick_state)?;
    }

    // Update tick pointers in orderbook
    ORDERBOOKS.save(storage, &order.book_id, &orderbook)?;

    // TODO: If we intend to support refunds for partial fills, we will need to return
    // the consumed input here as well. If we choose not to, we should error in this case.
    //
    // Tracked in issue https://github.com/osmosis-labs/orderbook/issues/86
    Ok((
        total_output,
        BankMsg::Send {
            to_address: order.owner.to_string(),
            amount: vec![coin(total_output.u128(), output_denom)],
        },
    ))
}
