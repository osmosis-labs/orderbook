use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractResult;
use crate::state::ORDERBOOK;
use crate::types::Orderbook;
use cosmwasm_std::DepsMut;

pub fn create_orderbook(
    deps: DepsMut,
    quote_denom: String,
    base_denom: String,
) -> ContractResult<()> {
    // TODO: add necessary validation logic
    // https://github.com/osmosis-labs/orderbook/issues/26

    let book = Orderbook::new(quote_denom, base_denom, 0, MIN_TICK, MAX_TICK);

    ORDERBOOK.save(deps.storage, &book)?;
    Ok(())
}
