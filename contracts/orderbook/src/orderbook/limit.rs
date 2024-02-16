use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use crate::error::ContractError;

pub fn place_limit(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: Implement place_limit

    Ok(Response::new()
        .add_attribute("method", "placeLimit")
        .add_attribute("owner", info.sender))
}

pub fn cancel_limit(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: Implement cancel_limit

    Ok(Response::new()
        .add_attribute("method", "cancelLimit")
        .add_attribute("owner", info.sender))
}