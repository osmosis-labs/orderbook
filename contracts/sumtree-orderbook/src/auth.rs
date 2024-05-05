use crate::{
    error::ContractResult,
    msg::{AuthExecuteMsg, AuthQueryMsg},
    state::{MAKER_FEE, MAKER_FEE_RECIPIENT},
    sudo, ContractError,
};
use cosmwasm_std::{ensure, Addr, Api, Decimal, Deps, DepsMut, MessageInfo, Response, Storage};
use cw_storage_plus::Item;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const ADMIN_OFFER: Item<Addr> = Item::new("admin_offer");
pub const MODERATOR: Item<Addr> = Item::new("moderator");
pub const MODERATOR_OFFER: Item<Addr> = Item::new("moderator_offer");

/// Handles incoming Auth messages
pub(crate) fn dispatch(
    deps: DepsMut,
    info: MessageInfo,
    msg: AuthExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        // -- Admin Messages --

        // Offer admin permissions to a new address
        AuthExecuteMsg::TransferAdmin { new_admin } => {
            dispatch_transfer_admin(deps, info, new_admin)
        }

        // Cancel an ongoing admin transfer offer
        AuthExecuteMsg::CancelAdminTransfer {} => dispatch_cancel_admin_transfer(deps, info),

        // Reject an ongoing admin transfer offer
        AuthExecuteMsg::RejectAdminTransfer {} => dispatch_reject_admin_transfer(deps, info),

        // Accept an ongoing admin transfer offer
        AuthExecuteMsg::ClaimAdmin {} => dispatch_claim_admin(deps, info),

        // Renounces adminship of the contract
        AuthExecuteMsg::RenounceAdminship {} => dispatch_renounce_adminship(deps, info),

        // -- Moderator Messages --

        // Offer moderator permissions to a new address
        AuthExecuteMsg::OfferModerator { new_moderator } => {
            dispatch_offer_moderator(deps, info, new_moderator)
        }

        // Reject an ongoing moderator offer
        AuthExecuteMsg::RejectModeratorOffer {} => dispatch_reject_moderator_offer(deps, info),

        // Accept an ongoing moderator offer
        AuthExecuteMsg::ClaimModerator {} => dispatch_claim_moderator(deps, info),

        // -- Shared Messages --

        // Set the active state of the contract
        AuthExecuteMsg::SetActive { active } => dispatch_set_active(deps, info, active),

        // Set the maker fee amount for the contract
        AuthExecuteMsg::SetMakerFee { fee } => dispatch_set_maker_fee(deps, info, fee),

        // Set the recipient address for the maker fee for the contract
        AuthExecuteMsg::SetMakerFeeRecipient { recipient } => {
            dispatch_set_maker_fee_recipient(deps, info, recipient)
        }
    }
}

/// Handles incoming Auth queries
pub(crate) fn query(deps: Deps, msg: AuthQueryMsg) -> ContractResult<Option<Addr>> {
    match msg {
        // Current Admin Query
        AuthQueryMsg::Admin {} => get_admin(deps.storage),
        // Current Admin Offer Query
        AuthQueryMsg::AdminOffer {} => get_admin_offer(deps.storage),
        // Current Moderator Query
        AuthQueryMsg::Moderator {} => get_moderator(deps.storage),
        // Current Moderator Offer Query
        AuthQueryMsg::ModeratorOffer {} => get_moderator_offer(deps.storage),
    }
}

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
    ensure!(Some(info.sender) == offer, ContractError::Unauthorized {});

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

pub(crate) fn get_admin(storage: &dyn Storage) -> ContractResult<Option<Addr>> {
    Ok(ADMIN.may_load(storage)?)
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
    ensure!(Some(info.sender) == offer, ContractError::Unauthorized {});

    remove_moderator_offer(deps.storage)?;

    Ok(Response::default().add_attributes(vec![("method", "reject_moderator_offer")]))
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

pub(crate) fn get_moderator(storage: &dyn Storage) -> ContractResult<Option<Addr>> {
    Ok(MODERATOR.may_load(storage)?)
}

pub(crate) fn get_moderator_offer(storage: &dyn Storage) -> ContractResult<Option<Addr>> {
    Ok(MODERATOR_OFFER.may_load(storage)?)
}

// -- Shared Methods --

/// Sets the active state of the orderbook.
///
/// Only callable by either moderator or admin.
pub(crate) fn dispatch_set_active(
    deps: DepsMut,
    info: MessageInfo,
    active: bool,
) -> ContractResult<Response> {
    ensure_is_admin_or_moderator(deps.as_ref(), &info.sender)?;

    sudo::set_active(deps, active)
}

/// Sets the maker fee amount for the orderbook.
///
/// Only callable by either moderator or admin.
pub(crate) fn dispatch_set_maker_fee(
    deps: DepsMut,
    info: MessageInfo,
    maker_fee: Decimal,
) -> ContractResult<Response> {
    ensure_is_admin_or_moderator(deps.as_ref(), &info.sender)?;

    MAKER_FEE.save(deps.storage, &maker_fee)?;

    Ok(Response::default().add_attributes(vec![
        ("method", "set_maker_fee"),
        ("maker_fee", &maker_fee.to_string()),
    ]))
}

/// Sets the recipient address for the maker fee for the orderbook.
///
/// Only callable by either moderator or admin.
pub(crate) fn dispatch_set_maker_fee_recipient(
    deps: DepsMut,
    info: MessageInfo,
    maker_fee_recipient: Addr,
) -> ContractResult<Response> {
    ensure_is_admin_or_moderator(deps.as_ref(), &info.sender)?;

    let addr = deps.api.addr_validate(maker_fee_recipient.as_str())?;
    MAKER_FEE_RECIPIENT.save(deps.storage, &addr)?;

    Ok(Response::default().add_attributes(vec![
        ("method", "set_maker_fee_recipient"),
        ("maker_fee_recipient", addr.as_str()),
    ]))
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
