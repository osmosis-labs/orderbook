use core::fmt;
use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Uint128};

#[cw_serde]
#[derive(Copy)]
pub enum OrderDirection {
    Bid,
    Ask,
}

impl OrderDirection {
    /// Returns the opposite order direction.
    pub fn opposite(&self) -> Self {
        match self {
            OrderDirection::Bid => OrderDirection::Ask,
            OrderDirection::Ask => OrderDirection::Bid,
        }
    }
}

impl From<OrderDirection> for String {
    fn from(direction: OrderDirection) -> String {
        match direction {
            OrderDirection::Ask => "ask".to_string(),
            OrderDirection::Bid => "bid".to_string(),
        }
    }
}

impl Display for OrderDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from(*self))
    }
}

#[cw_serde]
pub struct LimitOrder {
    pub book_id: u64,
    pub tick_id: i64,
    pub order_id: u64,
    pub order_direction: OrderDirection,
    pub owner: Addr,
    pub quantity: Uint128,
    pub etas: Decimal256,
}

impl LimitOrder {
    pub fn new(
        book_id: u64,
        tick_id: i64,
        order_id: u64,
        order_direction: OrderDirection,
        owner: Addr,
        quantity: Uint128,
        etas: Decimal256,
    ) -> Self {
        LimitOrder {
            book_id,
            tick_id,
            order_id,
            order_direction,
            owner,
            quantity,
            etas,
        }
    }
}

#[cw_serde]
pub struct MarketOrder {
    pub book_id: u64,
    pub quantity: Uint128,
    pub order_direction: OrderDirection,
    pub owner: Addr,
}

impl MarketOrder {
    pub fn new(
        book_id: u64,
        quantity: Uint128,
        order_direction: OrderDirection,
        owner: Addr,
    ) -> Self {
        MarketOrder {
            book_id,
            quantity,
            order_direction,
            owner,
        }
    }
}

impl From<LimitOrder> for MarketOrder {
    fn from(limit_order: LimitOrder) -> Self {
        MarketOrder {
            book_id: limit_order.book_id,
            quantity: limit_order.quantity,
            order_direction: limit_order.order_direction,
            owner: limit_order.owner,
        }
    }
}

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
