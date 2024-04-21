#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    Uint128,
};
use cw2::set_contract_version;

use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::orderbook::create_orderbook;
use crate::query;
use crate::types::OrderDirection;
use crate::{auth, order};

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

        ExecuteMsg::BatchClaim { orders } => order::batch_claim_limits(deps, info, orders),

        // -- Admin Messages --

        // Offer admin permissions to a new address
        ExecuteMsg::TransferAdmin { new_admin } => {
            auth::dispatch_transfer_admin(deps, info, new_admin)
        }

        // Cancel an ongoing admin transfer offer
        ExecuteMsg::CancelAdminTransfer {} => auth::dispatch_cancel_admin_transfer(deps, info),

        // Reject an ongoing admin transfer offer
        ExecuteMsg::RejectAdminTransfer {} => auth::dispatch_reject_admin_transfer(deps, info),

        // Accept an ongoing admin transfer offer
        ExecuteMsg::ClaimAdmin {} => auth::dispatch_claim_admin(deps, info),

        // Renounces adminship of the contract
        ExecuteMsg::RenounceAdminship {} => auth::dispatch_renounce_adminship(deps, info),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::SpotPrice {
            quote_asset_denom,
            base_asset_denom,
        } => Ok(to_json_binary(&query::spot_price(
            deps,
            quote_asset_denom,
            base_asset_denom,
        )?)?),
        QueryMsg::CalcOutAmountGivenIn {
            token_in,
            token_out_denom,
            swap_fee,
        } => Ok(to_json_binary(&query::calc_out_amount_given_in(
            deps,
            token_in,
            token_out_denom,
            swap_fee,
        )?)?),
        QueryMsg::GetTotalPoolLiquidity {} => {
            Ok(to_json_binary(&query::total_pool_liquidity(deps)?)?)
        }
        QueryMsg::CalcInAmtGivenOut {} => unimplemented!(),
        QueryMsg::Admin {} => Ok(to_json_binary(&auth::get_admin(deps)?)?),
        QueryMsg::AdminOffer {} => Ok(to_json_binary(&auth::get_admin_offer(deps)?)?),
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
