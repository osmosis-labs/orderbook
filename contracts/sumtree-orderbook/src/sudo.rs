use cosmwasm_std::{entry_point, Coin, Decimal, DepsMut, Env, Response, Uint128};

use crate::{error::ContractResult, msg::SudoMsg, state::DENOM_PAIR_BOOK_ID, ContractError};

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

pub(crate) fn dispatch_swap_exact_amount_in(
    deps: DepsMut,
    sender: String,
    token_in: Coin,
    token_out_denom: String,
    token_out_min_amount: Uint128,
    swap_fee: Decimal,
) -> ContractResult<Response> {
    let token_in_denom = token_in.denom;
    let _book_id = DENOM_PAIR_BOOK_ID
        .may_load(deps.storage, (&token_in_denom, &token_out_denom))?
        .ok_or(ContractError::OrderbookNotFound {
            in_denom: token_in_denom,
            out_denom: token_out_denom,
        });
    Ok(Response::default())
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
