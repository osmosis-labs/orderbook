use crate::types::OrderDirection;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    CreateOrderbook {
        quote_denom: String,
        base_denom: String,
    },
    PlaceLimit {
        book_id: u64,
        tick_id: i64,
        order_direction: OrderDirection,
        quantity: Uint128,
        claim_bounty: Option<Decimal>,
    },
    CancelLimit {
        book_id: u64,
        tick_id: i64,
        order_id: u64,
    },
    PlaceMarket {
        book_id: u64,
        order_direction: OrderDirection,
        quantity: Uint128,
    },
    ClaimLimit {
        book_id: u64,
        tick_id: i64,
        order_id: u64,
    },
    BatchClaim {
        book_id: u64,
        orders: Vec<(i64, u64)>,
    },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // This example query variant indicates that any client can query the contract
    // using `YourQuery` and it will return `YourQueryResponse`
    // This `returns` information will be included in contract's schema
    // which is used for client code generation.
    //
    // #[returns(YourQueryResponse)]
    // YourQuery {},
}

// We define a custom struct for each query response
// #[cw_serde]
// pub struct YourQueryResponse {}
