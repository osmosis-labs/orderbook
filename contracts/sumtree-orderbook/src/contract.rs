#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::order;
use crate::orderbook;
use crate::types::OrderDirection;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sumtree-orderbook";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // With `Response` type, it is possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

/// Handling contract migration
/// To make a contract migratable, you need
/// - this entry_point implemented
/// - only contract admin can migrate, so admin has to be set at contract initiation time
/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        // Find matched incoming message variant and execute them with your custom logic.
        //
        // With `Response` type, it is possible to dispatch message to invoke external logic.
        // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    }
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Creates a new orderbook
        ExecuteMsg::CreateOrderbook {
            quote_denom,
            base_denom,
        } => orderbook::create_orderbook(deps, env, info, quote_denom, base_denom),

        // Places limit order on given market
        ExecuteMsg::PlaceLimit {
            book_id,
            tick_id,
            order_direction,
            quantity,
        } => dispatch_place_limit(deps, env, info, book_id, tick_id, order_direction, quantity),

        // Cancels limit order with given ID
        ExecuteMsg::CancelLimit {
            book_id,
            tick_id,
            order_id,
        } => order::cancel_limit(deps, env, info, book_id, tick_id, order_id),

        // Places a market order on the passed in market
        ExecuteMsg::PlaceMarket {
            book_id,
            order_direction,
            quantity,
        } => order::place_market(deps, env, info, book_id, order_direction, quantity),

        // Claims a limit order with given ID
        ExecuteMsg::ClaimLimit {
            book_id,
            tick_id,
            order_id,
        } => order::claim_limit(deps, env, info, book_id, tick_id, order_id),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Find matched incoming message variant and query them your custom logic
        // and then construct your query response with the type usually defined
        // `msg.rs` alongside with the query message itself.
        //
        // use `cosmwasm_std::to_binary` to serialize query response to json binary.
    }
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // With `Response` type, it is still possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    ensure!(
        msg.result.is_ok(),
        ContractError::ReplyError {
            id: msg.id,
            error: msg.result.unwrap_err(),
        }
    );
    todo!()
}

pub fn dispatch_place_limit(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    book_id: u64,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
) -> Result<Response, ContractError> {
    order::place_limit(
        &mut deps,
        env,
        info,
        book_id,
        tick_id,
        order_direction,
        quantity,
    )
}
