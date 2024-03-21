use std::str::FromStr;

use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractError;
use crate::state::{new_order_id, orders, ORDERBOOKS, TICK_STATE};
use crate::sumtree::node::{generate_node_id, NodeType, TreeNode};
use crate::sumtree::tree::TREE;
use crate::types::{LimitOrder, OrderDirection, REPLY_ID_REFUND};
use cosmwasm_std::{
    coin, ensure, ensure_eq, BankMsg, Decimal256, DepsMut, Env, MessageInfo, Response, SubMsg,
    Uint128, Uint256,
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

    // Update ETAS from Tick State
    let mut tick_state = TICK_STATE
        .load(deps.storage, &(book_id, tick_id))
        .unwrap_or_default();

    // Build limit order
    let mut limit_order = LimitOrder::new(
        book_id,
        tick_id,
        order_id,
        order_direction,
        info.sender.clone(),
        quantity,
        tick_state.cumulative_total_value,
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

    let quantity_fullfilled = quantity.checked_sub(limit_order.quantity)?;

    // Only save the order if not fully filled
    if limit_order.quantity > Uint128::zero() {
        // Save the order to the orderbook
        orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

        tick_state.total_amount_of_liquidity = tick_state
            .total_amount_of_liquidity
            .checked_add(Decimal256::from_ratio(
                limit_order.quantity.u128(),
                Uint256::one(),
            ))
            .unwrap();
    }

    tick_state.cumulative_total_value = tick_state
        .cumulative_total_value
        .checked_add(Decimal256::from_ratio(quantity, Uint256::one()))?;

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

    // Fetch the sumtree from storage, or create one if it does not exist
    let mut tree = TREE
        .load(deps.storage, &(order.book_id, order.tick_id))
        .unwrap_or(TreeNode::new(
            order.book_id,
            order.tick_id,
            generate_node_id(deps.storage, order.book_id, order.tick_id)?,
            NodeType::default(),
        ));

    // Generate info for new node to insert to sumtree
    let node_id = generate_node_id(deps.storage, order.book_id, order.tick_id)?;
    let mut curr_tick_state = TICK_STATE
        .load(deps.storage, &(order.book_id, order.tick_id))
        .ok()
        .ok_or(ContractError::InvalidTickId {
            tick_id: order.tick_id,
        })?;

    let etas = Uint128::from_str(&order.etas.to_string())?;

    let mut new_node = TreeNode::new(
        order.book_id,
        order.tick_id,
        node_id,
        NodeType::leaf(etas, order.quantity),
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

    curr_tick_state.total_amount_of_liquidity = curr_tick_state
        .total_amount_of_liquidity
        .checked_sub(Decimal256::from_ratio(order.quantity, Uint256::one()))?;

    TICK_STATE.save(
        deps.storage,
        &(order.book_id, order.tick_id),
        &curr_tick_state,
    )?;

    tree.save(deps.storage)?;
    TREE.save(deps.storage, &(book_id, tick_id), &tree)?;

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
