use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractError;
use crate::state::{new_orderbook_id, DENOM_PAIR_BOOK_ID, ORDERBOOKS};
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
    let book = Orderbook::new(
        book_id,
        quote_denom.clone(),
        base_denom.clone(),
        0,
        MIN_TICK,
        MAX_TICK,
    );

    ORDERBOOKS.save(deps.storage, &book_id, &book)?;
    DENOM_PAIR_BOOK_ID
        .save(deps.storage, (&quote_denom, &base_denom), &book_id)
        .unwrap();
    DENOM_PAIR_BOOK_ID
        .save(deps.storage, (&base_denom, &quote_denom), &book_id)
        .unwrap();
    Ok(Response::new()
        .add_attribute("method", "createOrderbook")
        .add_attribute("book_id", book_id.to_string()))
}
