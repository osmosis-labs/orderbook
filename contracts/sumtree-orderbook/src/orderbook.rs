use crate::constants::{MAX_TICK, MIN_TICK};
use crate::error::ContractResult;
use crate::state::ORDERBOOK;
use crate::types::Orderbook;
use crate::ContractError;
use cosmwasm_std::{ensure, DepsMut};

pub fn create_orderbook(
    deps: DepsMut,
    quote_denom: String,
    base_denom: String,
) -> ContractResult<()> {
    let denoms = [quote_denom.clone(), base_denom.clone()];

    ensure!(quote_denom != base_denom, ContractError::DuplicateDenoms {});

    for denom in denoms {
        let maybe_supply = deps.querier.query_supply(denom.clone());

        // Ensure denom exists and has at least 1 token
        ensure!(
            maybe_supply.is_ok() && !maybe_supply.unwrap().amount.is_zero(),
            ContractError::InvalidDenom { denom }
        );
    }

    let book = Orderbook::new(quote_denom, base_denom, 0, MIN_TICK, MAX_TICK);

    ORDERBOOK.save(deps.storage, &book)?;
    Ok(())
}
