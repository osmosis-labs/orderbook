use crate::constants::{
    DEFAULT_MAKER_FEE, DEFAULT_MAKER_FEE_RECIPIENT, MAX_MAKER_FEE_PERCENTAGE, MAX_TICK, MIN_TICK,
};
use crate::error::ContractResult;
use crate::state::{MAKER_FEE, MAKER_FEE_RECIPIENT, ORDERBOOK};
use crate::types::Orderbook;
use crate::ContractError;
use cosmwasm_std::{ensure, Decimal256, DepsMut, Storage};

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

    // Instantiate orderbook and write to state
    let book = Orderbook::new(quote_denom, base_denom, 0, MIN_TICK, MAX_TICK);
    ORDERBOOK.save(deps.storage, &book)?;

    // Set maker fee
    set_maker_fee(deps.storage, DEFAULT_MAKER_FEE)?;

    // Set maker fee recipient
    set_maker_fee_recipient(deps, DEFAULT_MAKER_FEE_RECIPIENT)?;

    Ok(())
}

/// Sets the maker fee amount for the orderbook.
pub fn set_maker_fee(
    storage: &mut dyn Storage,
    maker_fee: Decimal256,
) -> ContractResult<Decimal256> {
    ensure!(
        maker_fee <= MAX_MAKER_FEE_PERCENTAGE,
        ContractError::InvalidMakerFee {}
    );
    MAKER_FEE.save(storage, &maker_fee)?;

    Ok(maker_fee)
}

/// Sets the recipient address for the maker fee for the orderbook.
pub fn set_maker_fee_recipient(deps: DepsMut, maker_fee_recipient: &str) -> ContractResult<()> {
    let addr = deps
        .api
        .addr_validate(maker_fee_recipient)
        .map_err(|_| ContractError::InvalidMakerFeeRecipient)?;
    MAKER_FEE_RECIPIENT.save(deps.storage, &addr)?;

    Ok(())
}
