use crate::error::ContractError;
use crate::state::{new_orderbook_id, MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::Orderbook;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn create_orderbook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    quote_denom: String,
    base_denom: String,
) -> Result<Response, ContractError> {
    // TODO: add necessary validation logic
    // https://github.com/osmosis-labs/orderbook/issues/26

    let book_id = new_orderbook_id(deps.storage)?;
    let _book = Orderbook::new(book_id, quote_denom, base_denom, 0, MIN_TICK, MAX_TICK);

    ORDERBOOKS.save(deps.storage, &book_id, &_book)?;

    Ok(Response::new()
        .add_attribute("method", "createOrderbook")
        .add_attribute("book_id", book_id.to_string()))
}
