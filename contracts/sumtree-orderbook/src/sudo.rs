use cosmwasm_std::{
    ensure, entry_point, BankMsg, Coin, Decimal, DepsMut, Env, Response, StdError, SubMsg, Uint128,
};

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    error::ContractResult,
    msg::SudoMsg,
    order::run_market_order,
    state::{DENOM_PAIR_BOOK_ID, ORDERBOOKS},
    types::{MarketOrder, OrderDirection, REPLY_ID_SUDO_SWAP_EX_AMT_IN},
    ContractError,
};

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::SwapExactAmountIn {
            sender,
            token_in,
            token_out_denom,
            token_out_min_amount,
            swap_fee,
        } => dispatch_swap_exact_amount_in(
            deps,
            sender,
            token_in,
            token_out_denom,
            token_out_min_amount,
            swap_fee,
        ),
        SudoMsg::SwapExactAmountOut {
            sender,
            token_in_denom,
            token_in_max_amount,
            token_out,
            swap_fee,
        } => dispatch_swap_exact_amount_out(
            sender,
            token_in_denom,
            token_in_max_amount,
            token_out,
            swap_fee,
        ),
    }
}

/// Swaps the provided token in for the desired token out while restricting the possible minimum output.
/// The swap is performed by first determining the orderbook to be used before generating a market order against that orderbook.
/// Order direction is automatically determined by the token in/token out pairing.
///
/// Errors if the amount provided by the swap does not meet the `token_out_min_amount` or if there is no orderbook for the provided pair.
pub(crate) fn dispatch_swap_exact_amount_in(
    deps: DepsMut,
    sender: String,
    token_in: Coin,
    token_out_denom: String,
    token_out_min_amount: Uint128,
    swap_fee: Decimal,
) -> ContractResult<Response> {
    let token_in_denom = token_in.denom.clone();

    // Load the book ID for the provided pair
    let book_id = DENOM_PAIR_BOOK_ID
        .may_load(deps.storage, (&token_in_denom, &token_out_denom))?
        .ok_or(ContractError::OrderbookNotFound {
            in_denom: token_in_denom.clone(),
            out_denom: token_out_denom.clone(),
        })?;
    // Load the orderbook for the provided pair
    let orderbook = ORDERBOOKS
        .may_load(deps.storage, &book_id)?
        .ok_or(ContractError::InvalidBookId { book_id })?;

    // A tuple representing the expected in and out token denominations
    // Of the form (in token, out token)
    let in_out_tuple = (token_in_denom, token_out_denom.clone());

    // Determine order direction based on token in/out denoms
    let order_direction =
        if (orderbook.base_denom.clone(), orderbook.quote_denom.clone()) == in_out_tuple {
            OrderDirection::Ask
        } else if (orderbook.quote_denom, orderbook.base_denom) == in_out_tuple {
            OrderDirection::Bid
        } else {
            return Err(ContractError::InvalidBookId { book_id });
        };

    // Generate market order to be run
    let mut order = MarketOrder::new(
        book_id,
        token_in.amount,
        order_direction,
        deps.api.addr_validate(&sender)?,
    );

    // Market orders always run until either the input is filled or the orderbook is exhausted.
    let tick_bound = match order_direction {
        OrderDirection::Bid => MAX_TICK,
        OrderDirection::Ask => MIN_TICK,
    };

    // Run market order against orderbook
    let (output, bank_msg) = run_market_order(deps.storage, &mut order, tick_bound)?;

    // Validate the fullfillment message against the order
    if let BankMsg::Send { amount, to_address } = bank_msg.clone() {
        let fullfillment_amt = amount
            .first()
            .ok_or(ContractError::Std(StdError::generic_err(
                "No fullfillment message generated from market order",
            )))?;
        ensure!(to_address == sender, ContractError::InvalidMarketOrder);
        ensure_fullfilment_amt(
            None,
            Some(token_out_min_amount),
            token_out_denom.clone(),
            fullfillment_amt,
        )?;
    } else {
        return Err(ContractError::InvalidMarketOrder);
    }

    // Ensure the provided swap fee matches what is expected
    ensure_swap_fee(swap_fee)?;

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_error(
            bank_msg,
            REPLY_ID_SUDO_SWAP_EX_AMT_IN,
        ))
        .add_attributes(vec![
            ("method", "swapExactAmountIn"),
            ("sender", &sender),
            ("token_in", &token_in.to_string()),
            ("token_out_denom", &token_out_denom),
            ("token_out_min_amount", &token_out_min_amount.to_string()),
            ("output_quantity", &output.to_string()),
        ]))
}

pub(crate) fn dispatch_swap_exact_amount_out(
    _sender: String,
    _token_in_denom: String,
    _token_in_max_amount: Uint128,
    _token_out: Coin,
    _swap_fee: Decimal,
) -> ContractResult<Response> {
    todo!();
}

/// Ensures that the generated fullfillment meets the criteria set by the CW Pool interface. Ensures the following:
/// 1. An optional provided maximum amount (swap exact amount out)
/// 2. An optional provided minimum amount (swap exact amount in)
/// 3. An expected denom
pub(crate) fn ensure_fullfilment_amt(
    max_amount: Option<Uint128>,
    min_amount: Option<Uint128>,
    expected_denom: String,
    fullfilled: &Coin,
) -> ContractResult<()> {
    // Generated amount must be less than or equal to the maximum allowed amount
    if let Some(max_amount) = max_amount {
        ensure!(
            fullfilled.amount <= max_amount,
            ContractError::InvalidMarketOrder
        );
    }
    // Generated amount must be more than or equal to the minimum allowed amount
    if let Some(min_amount) = min_amount {
        ensure!(
            fullfilled.amount >= min_amount,
            ContractError::InvalidMarketOrder
        );
    }

    // The denom of the fullfillment must match the expected denom
    ensure!(
        fullfilled.denom == expected_denom,
        ContractError::InvalidMarketOrder
    );

    Ok(())
}

// The swap fee expected by this contract
pub const EXPECTED_SWAP_FEE: Decimal = Decimal::zero();

/// Ensures that the provided swap fee matches what is expected by this contract
pub(crate) fn ensure_swap_fee(fee: Decimal) -> ContractResult<()> {
    ensure!(fee == EXPECTED_SWAP_FEE, ContractError::InvalidMarketOrder);
    Ok(())
}
