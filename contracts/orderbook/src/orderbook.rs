use crate::error::ContractError;
use crate::types::Orderbook;
use crate::state::new_orderbook_id;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn create_orderbook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    quote_denom: String,
    base_denom: String,
) -> Result<Response, ContractError> {
    let book_id = new_orderbook_id(deps.storage).unwrap();
    let _book = Orderbook {
        book_id,
        quote_denom,
        base_denom,
        current_tick: 0,
        next_bid_tick: -1,
        next_ask_tick: 1,
    };

    Ok(Response::new().add_attribute("method", "createOrderbook"))
}
