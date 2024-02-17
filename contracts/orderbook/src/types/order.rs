use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
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
    pub quantity: Uint128,
}

impl LimitOrder {
    pub fn new(
        book_id: u64,
        tick_id: i64,
        order_id: u64,
        order_direction: OrderDirection,
        owner: Addr,
        quantity: Uint128,
    ) -> Self {
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

// TODO: Unnecessary if finite queries not required
/// Defines the different way an owners orders can be filtered, all enums filter by owner with each getting more finite
#[derive(Clone)]
pub enum FilterOwnerOrders {
    All(Addr),
    ByBook(u64, Addr),
    ByTick(u64, i64, Addr),
}

impl FilterOwnerOrders {
    pub fn all(owner: Addr) -> Self {
        FilterOwnerOrders::All(owner)
    }

    pub fn by_book(book_id: u64, owner: Addr) -> Self {
        FilterOwnerOrders::ByBook(book_id, owner)
    }

    pub fn by_tick(book_id: u64, tick_id: i64, owner: Addr) -> Self {
        FilterOwnerOrders::ByTick(book_id, tick_id, owner)
    }
}
