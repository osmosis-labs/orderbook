use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum OrderDirection {
    Bid,
    Ask,
}

#[cw_serde]
pub struct LimitOrder {
    pub book_id: u64,
    pub tick_id: i64,
    pub order_id: u64,
    pub order_direction: OrderDirection,
    pub owner: Addr,
    // TODO: Can this be Uint128? To match Coin amount type
    pub quantity: u64,
}

impl LimitOrder {
    pub fn new(book_id: u64, tick_id: i64, order_id: u64, order_direction: OrderDirection, owner: Addr, quantity: u64) -> Self {
        LimitOrder {
            book_id,
            tick_id,
            order_id,
            order_direction,
            owner,
            quantity,
        }
    }
}