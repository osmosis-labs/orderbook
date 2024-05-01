use std::str::FromStr;

use crate::constants::{MAX_BATCH_CLAIM, MAX_TICK, MIN_TICK};
use crate::error::{ContractError, ContractResult};
use crate::proto::MsgSend;
use crate::state::{
    add_directional_liquidity, new_order_id, orders, subtract_directional_liquidity, ORDERBOOK,
    TICK_STATE,
};
use crate::sumtree::node::{generate_node_id, NodeType, TreeNode};
use crate::sumtree::tree::get_or_init_root_node;
use crate::tick::sync_tick;
use crate::tick_math::{amount_to_value, tick_to_price, RoundingDirection};
use crate::types::{
    coin_u256, LimitOrder, MarketOrder, OrderDirection, Orderbook, TickState, REPLY_ID_CLAIM,
    REPLY_ID_CLAIM_BOUNTY, REPLY_ID_REFUND,
};
use cosmwasm_std::{
    coin, ensure, ensure_eq, Addr, BankMsg, Decimal256, DepsMut, Env, MessageInfo, Order, Response,
    Storage, SubMsg, Uint128, Uint256,
};
use cw_storage_plus::Bound;
use cw_utils::{must_pay, nonpayable};
use osmosis_std::types::cosmos::base::v1beta1::Coin;

#[allow(clippy::manual_range_contains, clippy::too_many_arguments)]
pub fn place_limit(
    deps: &mut DepsMut,
    _env: Env,
    info: MessageInfo,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
    claim_bounty: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut orderbook = ORDERBOOK.load(deps.storage)?;

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

    // If applicable, ensure claim_bounty is between 0 and 0.01.
    // We set a conservative upper bound of 1% for claim bounties as a guardrail.
    if let Some(claim_bounty_value) = claim_bounty {
        ensure!(
            claim_bounty_value >= Decimal256::zero()
                && claim_bounty_value <= Decimal256::percent(1),
            ContractError::InvalidClaimBounty {
                claim_bounty: Some(claim_bounty_value)
            }
        );
    }

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
    ORDERBOOK.save(deps.storage, &orderbook)?;

    // Update ETAS from Tick State
    let mut tick_state = TICK_STATE.load(deps.storage, tick_id).unwrap_or_default();
    let mut tick_values = tick_state.get_values(order_direction);

    // Build limit order
    let limit_order = LimitOrder::new(
        tick_id,
        order_id,
        order_direction,
        info.sender.clone(),
        quantity,
        tick_values.cumulative_total_value,
        claim_bounty,
    );

    let quant_dec256 = Decimal256::from_ratio(limit_order.quantity.u128(), Uint256::one());
    // Only save the order if not fully filled
    if limit_order.quantity > Uint128::zero() {
        // Save the order to the orderbook
        orders().save(deps.storage, &(tick_id, order_id), &limit_order)?;

        tick_values.total_amount_of_liquidity = tick_values
            .total_amount_of_liquidity
            .checked_add(quant_dec256)
            .unwrap();
    }

    tick_values.cumulative_total_value = tick_values
        .cumulative_total_value
        .checked_add(Decimal256::from_ratio(quantity, Uint256::one()))?;

    tick_state.set_values(order_direction, tick_values);
    TICK_STATE.save(deps.storage, tick_id, &tick_state)?;
    add_directional_liquidity(deps.storage, order_direction, quant_dec256)?;

    Ok(Response::default()
        .add_attribute("method", "placeLimit")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("order_direction", order_direction.to_string())
        .add_attribute("quantity", quantity.to_string()))
}

pub fn cancel_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tick_id: i64,
    order_id: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let key = (tick_id, order_id);
    // Check for the order, error if not found
    let order = orders()
        .may_load(deps.storage, &key)?
        .ok_or(ContractError::OrderNotFound { tick_id, order_id })?;

    // Ensure the sender is the order owner
    ensure_eq!(info.sender, order.owner, ContractError::Unauthorized {});

    // Ensure the order has not been filled.
    let tick_state = TICK_STATE.load(deps.storage, tick_id).unwrap_or_default();
    let tick_values = tick_state.get_values(order.order_direction);
    ensure!(
        tick_values.effective_total_amount_swapped <= order.etas,
        ContractError::CancelFilledOrder
    );

    // Fetch the sumtree from storage, or create one if it does not exist
    let mut tree = get_or_init_root_node(deps.storage, tick_id, order.order_direction)?;

    // Generate info for new node to insert to sumtree
    let node_id = generate_node_id(deps.storage, order.tick_id)?;
    let mut curr_tick_state =
        TICK_STATE
            .load(deps.storage, order.tick_id)
            .ok()
            .ok_or(ContractError::InvalidTickId {
                tick_id: order.tick_id,
            })?;
    let mut curr_tick_values = curr_tick_state.get_values(order.order_direction);
    let quant_dec256 =
        Decimal256::from_ratio(Uint256::from_uint128(order.quantity), Uint256::one());
    let mut new_node = TreeNode::new(
        order.tick_id,
        order.order_direction,
        node_id,
        NodeType::leaf(order.etas, quant_dec256),
    );

    // Insert new node
    tree.insert(deps.storage, &mut new_node)?;

    // Get orderbook info for correct denomination
    let orderbook = ORDERBOOK.load(deps.storage)?;

    // Generate refund
    let expected_denom = orderbook.get_expected_denom(&order.order_direction);
    let refund_msg = SubMsg::reply_on_error(
        BankMsg::Send {
            to_address: order.owner.to_string(),
            amount: vec![coin(order.quantity.u128(), expected_denom)],
        },
        REPLY_ID_REFUND,
    );

    orders().remove(deps.storage, &(order.tick_id, order.order_id))?;

    curr_tick_values.total_amount_of_liquidity = curr_tick_values
        .total_amount_of_liquidity
        .checked_sub(Decimal256::from_ratio(order.quantity, Uint256::one()))?;
    curr_tick_state.set_values(order.order_direction, curr_tick_values);
    TICK_STATE.save(deps.storage, order.tick_id, &curr_tick_state)?;
    subtract_directional_liquidity(deps.storage, order.order_direction, quant_dec256)?;

    tree.save(deps.storage)?;

    Ok(Response::new()
        .add_attribute("method", "cancelLimit")
        .add_attribute("owner", info.sender)
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_submessage(refund_msg))
}

pub fn claim_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tick_id: i64,
    order_id: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let (amount_claimed, bank_msgs) = claim_order(
        deps.storage,
        info.sender.clone(),
        env.contract.address,
        tick_id,
        order_id,
    )?;

    Ok(Response::new()
        .add_attribute("method", "claimMarket")
        .add_attribute("sender", info.sender)
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("amount_claimed", amount_claimed.to_string())
        .add_submessages(bank_msgs))
}

// batch_claim_limits allows for multiple limit orders to be claimed in a single transaction.
pub fn batch_claim_limits(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    orders: Vec<(i64, u64)>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    ensure!(
        orders.len() <= MAX_BATCH_CLAIM as usize,
        ContractError::BatchClaimLimitExceeded {
            max_batch_claim: MAX_BATCH_CLAIM
        }
    );

    let mut responses: Vec<SubMsg> = Vec::new();

    for (tick_id, order_id) in orders {
        // Attempt to claim each order
        match claim_order(
            deps.storage,
            env.contract.address.clone(),
            info.sender.clone(),
            tick_id,
            order_id,
        ) {
            Ok((_, mut bank_msgs)) => {
                responses.append(&mut bank_msgs);
            }
            Err(_) => {
                // We fail silently on errors to allow for the valid claims to be processed
                // to be processed.
                continue;
            }
        }
    }

    Ok(Response::new()
        .add_attribute("method", "batchClaim")
        .add_attribute("sender", info.sender)
        .add_submessages(responses))
}

// run_market_order processes a market order from the current active tick on the order's orderbook
/// up to the passed in `tick_bound`. **Partial fills are not allowed.**
///
/// Note that this mutates the `order` object
///
/// Returns:
/// * The output after the order has been processed
/// * Bank send message to process the balance transfer
///
/// Returns error if:
/// * Provided order has zero quantity
/// * Tick to price conversion fails for any tick
/// * Order is not fully filled
///
/// CONTRACT: The caller must ensure that the necessary input funds were actually supplied.
#[allow(clippy::manual_range_contains)]
pub fn run_market_order(
    storage: &mut dyn Storage,
    contract_address: Addr,
    order: &mut MarketOrder,
    tick_bound: i64,
) -> Result<(Uint256, MsgSend), ContractError> {
    let PostMarketOrderState {
        output,
        tick_updates,
        updated_orderbook,
    } = run_market_order_internal(storage, order, tick_bound)?;

    // After the core tick iteration loop, write all tick updates to state.
    // We cannot do this during the loop due to the borrow checker.
    for (tick_id, tick_state) in tick_updates {
        TICK_STATE.save(storage, tick_id, &tick_state)?;
    }

    // Reduce the amount of liquidity in the opposite direction of the order by the output amount
    subtract_directional_liquidity(
        storage,
        order.order_direction.opposite(),
        Decimal256::from_ratio(Uint256::from_str(&output.amount)?, Uint256::one()),
    )?;

    // Update tick pointers in orderbook
    ORDERBOOK.save(storage, &updated_orderbook)?;

    Ok((
        Uint256::from_str(&output.amount)?,
        MsgSend {
            from_address: contract_address.to_string(),
            to_address: order.owner.to_string(),
            amount: vec![output],
        },
    ))
}

/// Defines the state changes resulting from a market order.
pub(crate) struct PostMarketOrderState {
    pub output: Coin,
    pub tick_updates: Vec<(i64, TickState)>,
    pub updated_orderbook: Orderbook,
}

/// Attempts to fill a market order against the orderbook. Due to the sumtree-based orderbook design,
/// this does not require iterating linearly through all the filled orders.
///
/// Note that this mutates the `order` object and **does not perform any state mutations**
///
/// Returns:
/// * The output after the order has been processed
/// * Any required tick state updates
/// * The updated orderbook state
///
/// Returns error if:
/// * Provided order has zero quantity
/// * Tick to price conversion fails for any tick
/// * Order is not fully filled
///
/// CONTRACT: The caller must ensure that the necessary input funds were actually supplied.
#[allow(clippy::manual_range_contains)]
pub(crate) fn run_market_order_internal(
    storage: &dyn Storage,
    order: &mut MarketOrder,
    tick_bound: i64,
) -> ContractResult<PostMarketOrderState> {
    // Ensure order is non-empty
    ensure!(
        !order.quantity.is_zero(),
        ContractError::InvalidSwap {
            error: "Input amount cannot be zero".to_string()
        }
    );

    let mut orderbook = ORDERBOOK.load(storage)?;
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
    let ticks = TICK_STATE.keys(
        storage,
        Some(Bound::inclusive(min_tick)),
        Some(Bound::inclusive(max_tick)),
        ordering,
    );

    // Iterate through ticks and fill the market order as appropriate.
    // Due to our sumtree-based design, this process carries only O(1) overhead per tick.
    let mut total_output: Uint256 = Uint256::zero();
    let mut tick_updates: Vec<(i64, TickState)> = Vec::new();

    // The price of the last tick iterated on, if no ticks are iterated price is constant
    let mut last_tick_price = Decimal256::one();
    for maybe_current_tick in ticks {
        let current_tick_id = maybe_current_tick?;
        let mut current_tick = TICK_STATE.load(storage, current_tick_id)?;
        let mut current_tick_values = current_tick.get_values(order.order_direction.opposite());
        let tick_price = tick_to_price(current_tick_id)?;
        last_tick_price = tick_price;

        // Update current tick pointer as we visit ticks
        match order.order_direction.opposite() {
            OrderDirection::Ask => orderbook.next_ask_tick = current_tick_id,
            OrderDirection::Bid => orderbook.next_bid_tick = current_tick_id,
        }

        let output_quantity = amount_to_value(
            order.order_direction,
            order.quantity,
            tick_price,
            RoundingDirection::Down,
        )?;

        // If the output quantity is zero, the remaining input amount cannot generate any output.
        // When this is the case, we consume the remaining input (which is either zero or rounding error dust)
        // and terminate tick iteration.
        if output_quantity.is_zero() {
            order.quantity = Uint128::zero();
            break;
        }

        let output_quantity_dec = Decimal256::from_ratio(output_quantity, Uint256::one());

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
        order.quantity = order
            .quantity
            .checked_sub(Uint128::from_str(&input_filled.to_string())?)?;

        current_tick.set_values(order.order_direction.opposite(), current_tick_values);
        // Add the updated tick state to the vector
        tick_updates.push((current_tick_id, current_tick));

        total_output = total_output.checked_add(Uint256::from_uint128(fill_amount))?;
    }

    // Determine if filling remaining amount on the last possible tick produced any value
    // This will be 0 if the remaining balance is dust
    let remaining_balance = amount_to_value(
        order.order_direction,
        order.quantity,
        last_tick_price,
        RoundingDirection::Down,
    )?;

    // If, after iterating through all remaining ticks, the order quantity is still not filled (excluding dust),
    // we error out as the orderbook has insufficient liquidity to fill the order.
    ensure!(
        remaining_balance.is_zero(),
        ContractError::InsufficientLiquidity
    );

    Ok(PostMarketOrderState {
        output: coin_u256(total_output, &output_denom),
        tick_updates,
        updated_orderbook: orderbook,
    })
}

// Note: This can be called by anyone
pub(crate) fn claim_order(
    storage: &mut dyn Storage,
    contract_address: Addr,
    sender: Addr,
    tick_id: i64,
    order_id: u64,
) -> ContractResult<(Uint256, Vec<SubMsg>)> {
    let orderbook = ORDERBOOK.load(storage)?;
    // Fetch tick values for current order direction
    let tick_state = TICK_STATE
        .may_load(storage, tick_id)?
        .ok_or(ContractError::InvalidTickId { tick_id })?;

    let key = (tick_id, order_id);
    // Check for the order, error if not found
    let mut order = orders()
        .may_load(storage, &key)?
        .ok_or(ContractError::OrderNotFound { tick_id, order_id })?;

    // Sync the tick the order is on to ensure correct ETAS
    let bid_tick_values = tick_state.get_values(OrderDirection::Bid);
    let ask_tick_values = tick_state.get_values(OrderDirection::Ask);
    sync_tick(
        storage,
        tick_id,
        bid_tick_values.effective_total_amount_swapped,
        ask_tick_values.effective_total_amount_swapped,
    )?;

    // Re-fetch tick post sync call
    let tick_state = TICK_STATE
        .may_load(storage, tick_id)?
        .ok_or(ContractError::InvalidTickId { tick_id })?;
    let tick_values = tick_state.get_values(order.order_direction);

    // Early exit if nothing has been filled
    ensure!(
        tick_values.effective_total_amount_swapped > order.etas,
        ContractError::ZeroClaim
    );

    // Calculate amount of order that is currently filled (may be partial).
    // We take the min between (tick_ETAS - order_ETAS) and the order quantity to ensure
    // we don't claim more than the order has available.
    let amount_filled_dec = tick_values
        .effective_total_amount_swapped
        .checked_sub(order.etas)?
        .min(Decimal256::from_ratio(order.quantity, 1u128));
    let amount_filled = Uint128::try_from(amount_filled_dec.to_uint_floor())?;

    // Update order state to reflect the claimed amount.
    //
    // By subtracting the order quantity and moving up the start ETAS,
    // the order should effectively be left as a fresh order with the remaining quantity.
    order.quantity = order.quantity.checked_sub(amount_filled)?;
    order.etas = order.etas.checked_add(amount_filled_dec)?;

    // If order fully filled then remove
    if order.quantity.is_zero() {
        orders().remove(storage, &key)?;
    // Else update in state
    } else {
        orders().save(storage, &key, &order)?;
    }

    // Calculate amount to be sent to order owner
    let tick_price = tick_to_price(tick_id)?;
    let mut amount = amount_to_value(
        order.order_direction,
        amount_filled,
        tick_price,
        RoundingDirection::Down,
    )?;

    // Cannot send a zero amount, may be zero'd out by rounding
    ensure!(!amount.is_zero(), ContractError::ZeroClaim);

    let denom = orderbook.get_opposite_denom(&order.order_direction);

    // Send claim bounty to sender if applicable
    let mut bounty = Uint256::zero();
    if let Some(claim_bounty) = order.claim_bounty {
        // Multiply by the claim bounty ratio and convert to Uint128.
        // Ensure claimed amount is updated to reflect the bounty.
        let bounty_amount =
            Decimal256::from_ratio(amount, Uint256::one()).checked_mul(claim_bounty)?;
        bounty = bounty_amount.to_uint_floor();
        amount = amount.checked_sub(bounty)?;
    }

    // Claimed amount always goes to the order owner
    let bank_msg = MsgSend {
        from_address: contract_address.to_string(),
        to_address: order.owner.to_string(),
        amount: vec![coin_u256(amount, &denom)],
    };
    let mut bank_msg_vec = vec![SubMsg::reply_on_error(bank_msg, REPLY_ID_CLAIM)];

    if !bounty.is_zero() {
        // Bounty always goes to the sender
        let bounty_msg = MsgSend {
            from_address: contract_address.to_string(),
            to_address: sender.to_string(),
            amount: vec![coin_u256(bounty, &denom)],
        };
        bank_msg_vec.push(SubMsg::reply_on_error(bounty_msg, REPLY_ID_CLAIM_BOUNTY));
    }

    Ok((amount, bank_msg_vec))
}
