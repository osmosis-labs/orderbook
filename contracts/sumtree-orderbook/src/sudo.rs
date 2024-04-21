use cosmwasm_std::{
    ensure, entry_point, to_json_binary, BankMsg, Coin, Decimal, DepsMut, Env, Response, SubMsg,
    Uint128,
};

use crate::{
    auth,
    constants::{EXPECTED_SWAP_FEE, MAX_TICK, MIN_TICK},
    error::ContractResult,
    msg::{SudoMsg, SwapExactAmountInResponseData},
    order::run_market_order,
    state::ORDERBOOK,
    types::{MarketOrder, OrderDirection, REPLY_ID_SUDO_SWAP_EXACT_IN},
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
            deps,
            sender,
            token_in_denom,
            token_in_max_amount,
            token_out,
            swap_fee,
        ),
        SudoMsg::TransferAdmin { new_admin } => {
            auth::transfer_admin(deps, new_admin.clone())?;
            Ok(Response::default().add_attributes(vec![
                ("method", "sudo_transfer_admin"),
                ("new_admin", new_admin.as_str()),
            ]))
        }
        SudoMsg::CancelAdminTransfer {} => {
            auth::remove_admin_transfer(deps)?;
            Ok(Response::default().add_attributes(vec![("method", "sudo_cancel_admin_transfer")]))
        }
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
    // Ensure the provided swap fee matches what is expected
    ensure_swap_fee(swap_fee)?;

    let token_in_denom = token_in.denom.clone();

    // Ensure in and out denoms are not equal
    ensure!(
        token_in_denom != token_out_denom,
        ContractError::InvalidSwap {
            error: "Input and output denoms cannot be the same".to_string()
        }
    );

    // Load the orderbook for the provided pair
    let orderbook = ORDERBOOK.load(deps.storage)?;

    // Determine order direction based on token in/out denoms
    let order_direction = orderbook.direction_from_pair(token_in_denom, token_out_denom.clone())?;

    // Generate market order to be run
    let mut order = MarketOrder::new(
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

    // Validate the output message against the order
    if let BankMsg::Send { amount, .. } = bank_msg.clone() {
        let output_amt = amount.first().ok_or(ContractError::InvalidSwap {
            error: "Market order did not generate an output message".to_string(),
        })?;
        validate_output_amount(None, Some(token_out_min_amount), &token_in, output_amt)?;
    }

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_error(
            bank_msg,
            REPLY_ID_SUDO_SWAP_EXACT_IN,
        ))
        .add_attributes(vec![
            ("method", "swapExactAmountIn"),
            ("sender", &sender),
            ("token_in", &token_in.to_string()),
            ("token_out_denom", &token_out_denom),
            ("token_out_min_amount", &token_out_min_amount.to_string()),
            ("output_quantity", &output.to_string()),
        ])
        .set_data(to_json_binary(&SwapExactAmountInResponseData {
            token_out_amount: output,
        })?))
}

/// Temporarily unimplemented
pub(crate) fn dispatch_swap_exact_amount_out(
    _deps: DepsMut,
    _sender: String,
    _token_in_denom: String,
    _token_in_max_amount: Uint128,
    _token_out: Coin,
    _swap_fee: Decimal,
) -> ContractResult<Response> {
    unimplemented!();
}

/// Ensures that the generated output meets the criteria set by the CW Pool interface. Ensures the following:
/// 1. An optional provided maximum amount (swap exact amount out)
/// 2. An optional provided minimum amount (swap exact amount in)
/// 3. An expected denom
pub(crate) fn validate_output_amount(
    max_in_amount: Option<Uint128>,
    min_out_amount: Option<Uint128>,
    input: &Coin,
    output: &Coin,
) -> ContractResult<()> {
    // Generated amount must be less than or equal to the maximum allowed amount
    if let Some(max_amount) = max_in_amount {
        ensure!(
            input.amount <= max_amount,
            ContractError::InvalidSwap {
                error: format!(
                    "Exceeded max swap amount: expected {max_amount} received {}",
                    input.amount
                )
            }
        );
    }
    // Generated amount must be more than or equal to the minimum allowed amount
    if let Some(min_amount) = min_out_amount {
        ensure!(
            output.amount >= min_amount,
            ContractError::InvalidSwap {
                error: format!(
                    "Did not meet minimum swap amount: expected {min_amount} received {}",
                    output.amount
                )
            }
        );
    }

    // Ensure in and out denoms are not equal
    ensure!(
        output.denom != input.denom,
        ContractError::InvalidSwap {
            error: "Input and output denoms cannot be the same".to_string()
        }
    );
    Ok(())
}

/// Ensures that the provided swap fee matches what is expected by this contract
pub(crate) fn ensure_swap_fee(fee: Decimal) -> ContractResult<()> {
    ensure!(
        fee == EXPECTED_SWAP_FEE,
        ContractError::InvalidSwap {
            error: format!(
                "Provided swap fee does not match: expected {EXPECTED_SWAP_FEE} received {fee}"
            )
        }
    );
    Ok(())
}
