use crate::{error::ContractResult, ContractError};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, MessageInfo, Response};
use cw_storage_plus::Item;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const ADMIN_OFFER: Item<Addr> = Item::new("admin_offer");

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

pub(crate) fn dispatch_cancel_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
) -> ContractResult<Response> {
    ensure_is_admin(deps.as_ref(), &info.sender)?;

    cancel_transfer_admin(deps)?;

    Ok(Response::default().add_attributes(vec![("method", "cancel_transfer_admin")]))
}

pub(crate) fn transfer_admin(deps: DepsMut, new_admin: Addr) -> ContractResult<()> {
    deps.api.addr_validate(new_admin.as_str())?;
    ADMIN_OFFER.save(deps.storage, &new_admin)?;
    Ok(())
}

pub(crate) fn cancel_transfer_admin(deps: DepsMut) -> ContractResult<()> {
    ADMIN_OFFER.remove(deps.storage);
    Ok(())
}

pub(crate) fn ensure_is_admin(deps: Deps, sender: &Addr) -> ContractResult<()> {
    let admin = ADMIN
        .load(deps.storage)
        .ok()
        .ok_or(ContractError::Unauthorized {})?;
    ensure!(admin == sender, ContractError::Unauthorized {});

    Ok(())
}
