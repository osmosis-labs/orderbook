use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    Uint128,
};
use cw2::set_contract_version;

use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SpotPriceResponse};

use crate::order;
use crate::orderbook::create_orderbook;
use crate::state::ORDERBOOK;
use crate::tick_math::tick_to_price;
use crate::types::OrderDirection;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sumtree-orderbook";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    create_orderbook(deps, msg.quote_denom.clone(), msg.base_denom.clone())?;

    // With `Response` type, it is possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    Ok(Response::new().add_attributes(vec![
        ("method", "instantiate"),
        ("quote_denom", &msg.quote_denom),
        ("base_denom", &msg.base_denom),
    ]))
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
        // Places limit order on given market
        ExecuteMsg::PlaceLimit {
            tick_id,
            order_direction,
            quantity,
            claim_bounty,
        } => dispatch_place_limit(
            deps,
            env,
            info,
            tick_id,
            order_direction,
            quantity,
            claim_bounty,
        ),

        // Cancels limit order with given ID
        ExecuteMsg::CancelLimit { tick_id, order_id } => {
            order::cancel_limit(deps, env, info, tick_id, order_id)
        }

        // Places a market order on the passed in market
        ExecuteMsg::PlaceMarket {
            order_direction,
            quantity,
        } => order::place_market(deps, env, info, order_direction, quantity),

        // Claims a limit order with given ID
        ExecuteMsg::ClaimLimit { tick_id, order_id } => {
            order::claim_limit(deps, env, info, tick_id, order_id)
        }
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let query_resp = match msg {
        // Find matched incoming message variant and query them your custom logic
        // and then construct your query response with the type usually defined
        // `msg.rs` alongside with the query message itself.
        //
        // use `cosmwasm_std::to_binary` to serialize query response to json binary.
        QueryMsg::SpotPrice {
            quote_asset_denom,
            base_asset_denom,
        } => query_spot_price(deps, quote_asset_denom, base_asset_denom)?,
    };

    Ok(to_json_binary(&query_resp)?)
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

#[allow(clippy::too_many_arguments)]
pub fn dispatch_place_limit(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
    claim_bounty: Option<Decimal>,
) -> Result<Response, ContractError> {
    order::place_limit(
        &mut deps,
        env,
        info,
        tick_id,
        order_direction,
        quantity,
        claim_bounty,
    )
}

pub fn query_spot_price(
    deps: Deps,
    quote_asset_denom: String,
    base_asset_denom: String,
) -> ContractResult<SpotPriceResponse> {
    // Ensure provided denoms do not match
    ensure!(
        quote_asset_denom != base_asset_denom,
        ContractError::InvalidPair {
            token_in_denom: quote_asset_denom,
            token_out_denom: base_asset_denom
        }
    );

    // Fetch orderbook to retrieve tick info
    let orderbook = ORDERBOOK.load(deps.storage)?;
    // Determine the order direction by denom pairing
    let direction = orderbook.direction_from_pair(quote_asset_denom, base_asset_denom)?;

    // Determine next tick based on desired order direction
    let next_tick = match direction {
        OrderDirection::Ask => orderbook.next_bid_tick,
        OrderDirection::Bid => orderbook.next_ask_tick,
    };

    // Generate spot price based on current active tick for desired order direction
    let price = tick_to_price(next_tick)?;

    Ok(SpotPriceResponse {
        spot_price: Decimal::from_str(&price.to_string())?,
    })
}
