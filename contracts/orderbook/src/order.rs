use crate::error::ContractError;
use crate::state::*;
use crate::state::{MAX_TICK, MIN_TICK, ORDERBOOKS};
use crate::types::{LimitOrder, OrderDirection};
use cosmwasm_std::{
    coin, Addr, BalanceResponse, BankMsg, BankQuery, Coin, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdResult, Uint128,
};

// TODO: move this into a balance helper file
pub fn query_balance(querier: &QuerierWrapper, addr: &Addr, denom: &str) -> StdResult<Coin> {
    let res: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: addr.to_string(),
        denom: denom.to_string(),
    }))?;
    Ok(Coin {
        denom: denom.to_string(),
        amount: res.amount.amount,
    })
}

pub fn place_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    book_id: u64,
    tick_id: i64,
    order_direction: OrderDirection,
    quantity: Uint128,
) -> Result<Response, ContractError> {
    // Validate book_id exists
    let orderbook = ORDERBOOKS
        .load(deps.storage, &book_id)
        .map_err(|_| ContractError::InvalidBookId { book_id })?;

    // Validate tick_id is within valid range
    if tick_id < MIN_TICK || tick_id > MAX_TICK {
        return Err(ContractError::InvalidTickId { tick_id });
    }

    // Validate order_quantity is > 0
    if quantity.is_zero() {
        return Err(ContractError::InvalidQuantity { quantity });
    }

    // Verify the sender has `quantity` balance of the correct denom
    let denom = match order_direction {
        OrderDirection::Bid => orderbook.quote_denom,
        OrderDirection::Ask => orderbook.base_denom,
    };
    let balance = query_balance(&deps.querier, &info.sender, &denom)?.amount;
    if balance < quantity {
        return Err(ContractError::InsufficientFunds {
            balance,
            required: quantity,
        });
    }

    // Generate a new order ID
    let order_id = new_order_id(deps.storage)?;

    let limit_order = LimitOrder::new(
        book_id,
        tick_id,
        order_id,
        order_direction.clone(),
        info.sender.clone(),
        quantity,
    );

    // Save the order to the orderbook
    orders().save(deps.storage, &(book_id, tick_id, order_id), &limit_order)?;

    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: env.contract.address.to_string(),
            amount: vec![coin(quantity.u128(), denom)],
        })
        .add_attribute("method", "placeLimit")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("book_id", book_id.to_string())
        .add_attribute("tick_id", tick_id.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("order_direction", format!("{:?}", order_direction))
        .add_attribute("quantity", quantity.to_string()))
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

pub fn place_market(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO: Implement place_market

    Ok(Response::new()
        .add_attribute("method", "placeMarket")
        .add_attribute("owner", info.sender))
}
