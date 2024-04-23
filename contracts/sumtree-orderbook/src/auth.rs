use crate::{error::ContractResult, ContractError};
use cosmwasm_std::{ensure, Addr, Api, Deps, DepsMut, MessageInfo, Response, Storage};
use cw_storage_plus::Item;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const ADMIN_OFFER: Item<Addr> = Item::new("admin_offer");
pub const MODERATOR: Item<Addr> = Item::new("moderator");
pub const MODERATOR_OFFER: Item<Addr> = Item::new("moderator_offer");

// -- Admin Methods --

/// Offers admin rights to a new address.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Addr,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    offer_admin(deps.storage, deps.api, new_admin.clone())?;

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

    remove_admin_transfer(deps.storage)?;

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

    update_admin(deps.storage, deps.api, info.sender)?;
    remove_admin_transfer(deps.storage)?;

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

    remove_admin_transfer(deps.storage)?;

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

    remove_admin(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "renounce_adminship")]))
}

pub(crate) fn offer_admin(
    storage: &mut dyn Storage,
    api: &dyn Api,
    new_admin: Addr,
) -> ContractResult<()> {
    // Ensure provided address is valid
    api.addr_validate(new_admin.as_str())?;
    ADMIN_OFFER.save(storage, &new_admin)?;
    Ok(())
}

pub(crate) fn remove_admin_transfer(storage: &mut dyn Storage) -> ContractResult<()> {
    ADMIN_OFFER.remove(storage);
    Ok(())
}

pub(crate) fn update_admin(
    storage: &mut dyn Storage,
    api: &dyn Api,
    new_admin: Addr,
) -> ContractResult<()> {
    // Ensure provided address is valid
    api.addr_validate(new_admin.as_str())?;
    ADMIN.save(storage, &new_admin)?;
    Ok(())
}

pub(crate) fn remove_admin(storage: &mut dyn Storage) -> ContractResult<()> {
    ADMIN.remove(storage);
    Ok(())
}

pub(crate) fn get_admin(storage: &dyn Storage) -> ContractResult<Addr> {
    Ok(ADMIN.load(storage)?)
}

pub(crate) fn get_admin_offer(storage: &dyn Storage) -> ContractResult<Option<Addr>> {
    Ok(ADMIN_OFFER.may_load(storage)?)
}

// -- Moderator Methods --

/// Offers moderator rights to a new address.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_offer_moderator(
    deps: DepsMut,
    info: MessageInfo,
    new_mod: Addr,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    offer_moderator(deps.storage, deps.api, new_mod.clone())?;

    Ok(Response::default().add_attributes(vec![
        ("method", "offer_moderator"),
        ("new_moderator", new_mod.as_str()),
    ]))
}

/// Cancels any ongoing moderator offer.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_cancel_moderator_offer(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    remove_moderator_offer(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "cancel_moderator_offer")]))
}

/// Accepts a moderator offer, claiming moderator rights to the contract
///
/// Only callable by the address offered moderator rights.
pub(crate) fn dispatch_claim_moderator(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    let offer = MODERATOR_OFFER.may_load(deps.storage)?;
    ensure!(
        Some(info.sender.clone()) == offer,
        ContractError::Unauthorized {}
    );

    update_moderator(deps.storage, deps.api, info.sender)?;
    remove_moderator_offer(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "claim_moderator")]))
}

/// Rejects a moderator offer.
///
/// Only callable by the address offered moderator rights.
pub(crate) fn dispatch_reject_moderator_offer(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    let offer = MODERATOR_OFFER.may_load(deps.storage)?;
    ensure!(
        Some(info.sender.clone()) == offer,
        ContractError::Unauthorized {}
    );

    remove_moderator_offer(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "reject_moderator_offer")]))
}

/// Renounces adminship of the contract.
///
/// Only callable by the current admin.
pub(crate) fn dispatch_renounce_moderator_role(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    remove_moderator(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "renounce_adminship")]))
}

pub(crate) fn offer_moderator(
    storage: &mut dyn Storage,
    api: &dyn Api,
    new_mod: Addr,
) -> ContractResult<()> {
    // Ensure provided address is valid
    api.addr_validate(new_mod.as_str())?;
    MODERATOR_OFFER.save(storage, &new_mod)?;
    Ok(())
}

pub(crate) fn remove_moderator_offer(storage: &mut dyn Storage) -> ContractResult<()> {
    MODERATOR_OFFER.remove(storage);
    Ok(())
}

pub(crate) fn update_moderator(
    storage: &mut dyn Storage,
    api: &dyn Api,
    new_admin: Addr,
) -> ContractResult<()> {
    // Ensure provided address is valid
    api.addr_validate(new_admin.as_str())?;
    MODERATOR.save(storage, &new_admin)?;
    Ok(())
}

pub(crate) fn remove_moderator(storage: &mut dyn Storage) -> ContractResult<()> {
    MODERATOR.remove(storage);
    Ok(())
}

pub(crate) fn get_moderator(storage: &dyn Storage) -> ContractResult<Addr> {
    Ok(MODERATOR.load(storage)?)
}

pub(crate) fn get_moderator_offer(storage: &dyn Storage) -> ContractResult<Option<Addr>> {
    Ok(MODERATOR_OFFER.may_load(storage)?)
}

// -- Ensure Methods --

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

/// Validates that the provided address is the current contract moderator.
///
/// Errors if:
/// - The provided address is not the current contract moderator
/// - The contract does not have a moderator
pub(crate) fn ensure_is_moderator(deps: Deps, sender: &Addr) -> ContractResult<()> {
    let moderator = MODERATOR.may_load(deps.storage)?;
    ensure!(
        moderator == Some(sender.clone()),
        ContractError::Unauthorized {}
    );

    Ok(())
}

/// Validates that the provided address is either the current contract moderator or admin.
///
/// Errors if:
/// - The provided address is not the current contract moderator or admin
/// - The contract does not have a moderator or admin
pub(crate) fn ensure_is_admin_or_moderator(deps: Deps, sender: &Addr) -> ContractResult<()> {
    let moderator = MODERATOR.may_load(deps.storage)?;
    let admin = ADMIN.may_load(deps.storage)?;

    ensure!(
        moderator == Some(sender.clone()) || admin == Some(sender.clone()),
        ContractError::Unauthorized {}
    );

    Ok(())
}
