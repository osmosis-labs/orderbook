use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use crate::error::ContractError;
use crate::orderbook::types::Orderbook;

pub fn create_orderbook(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    quote_denom: String,
    base_denom: String,
) -> Result<Response, ContractError> {
    let _book = Orderbook {
        book_id: 0,
        quote_denom,
        base_denom,
        current_tick: 0,
        next_bid_tick: -1,
        next_ask_tick: 1,
    };

    Ok(Response::new()
        .add_attribute("method", "createOrderbook"))
}