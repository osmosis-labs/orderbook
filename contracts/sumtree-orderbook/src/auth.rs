use crate::{error::ContractResult, ContractError};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, MessageInfo, Response};
use cw_storage_plus::Item;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const ADMIN_OFFER: Item<Addr> = Item::new("admin_offer");

/// Offers admin rights to a new address.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Addr,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    transfer_admin(deps, new_admin.clone())?;

    Ok(Response::default().add_attributes(vec![
        ("method", "transfer_admin"),
        ("new_admin", new_admin.as_str()),
    ]))
}

/// Cancels any ongoing admin transfer offer.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_cancel_admin_transfer(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    remove_admin_transfer(deps)?;

    Ok(Response::default().add_attributes(vec![("method", "cancel_transfer_admin")]))
}

/// Accepts an admin transfer offer, claiming admin rights to the contract
///
/// Only callable by the address offered admin rights.
pub(crate) fn dispatch_claim_admin(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let offer = ADMIN_OFFER.may_load(deps.storage)?;
    ensure!(
        Some(info.sender.clone()) == offer,
        ContractError::Unauthorized {}
    );

    ADMIN.save(deps.storage, &info.sender)?;
    remove_admin_transfer(deps)?;

    Ok(Response::default().add_attributes(vec![("method", "claim_admin")]))
}

/// Rejects an admin transfer offer.
///
/// Only callable by the address offered admin rights.
pub(crate) fn dispatch_reject_admin_transfer(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    let offer = ADMIN_OFFER.may_load(deps.storage)?;
    ensure!(
        Some(info.sender.clone()) == offer,
        ContractError::Unauthorized {}
    );

    remove_admin_transfer(deps)?;

    Ok(Response::default().add_attributes(vec![("method", "reject_admin_transfer")]))
}

/// Renounces adminship of the contract.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_renounce_adminship(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    remove_admin(deps)?;

    Ok(Response::default().add_attributes(vec![("method", "renounce_adminship")]))
}

pub(crate) fn transfer_admin(deps: DepsMut, new_admin: Addr) -> ContractResult<()> {
    // Ensure provided address is valid
    deps.api.addr_validate(new_admin.as_str())?;
    ADMIN_OFFER.save(deps.storage, &new_admin)?;
    Ok(())
}

pub(crate) fn remove_admin_transfer(deps: DepsMut) -> ContractResult<()> {
    ADMIN_OFFER.remove(deps.storage);
    Ok(())
}

pub(crate) fn remove_admin(deps: DepsMut) -> ContractResult<()> {
    ADMIN.remove(deps.storage);
    Ok(())
}

pub(crate) fn get_admin(deps: Deps) -> ContractResult<Addr> {
    Ok(ADMIN.load(deps.storage)?)
}

pub(crate) fn get_admin_offer(deps: Deps) -> ContractResult<Option<Addr>> {
    Ok(ADMIN_OFFER.may_load(deps.storage)?)
}

/// Validates that the provided address is the current contract admin.
///
/// Errors if:
/// - The provided address is not the current contract admin
/// - The contract does not have an admin
pub(crate) fn ensure_is_admin(deps: Deps, sender: &Addr) -> ContractResult<()> {
    let admin = ADMIN.may_load(deps.storage)?;
    ensure!(
        admin == Some(sender.clone()),
        ContractError::Unauthorized {}
    );

    Ok(())
}
