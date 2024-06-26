use core::fmt;
use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Timestamp, Uint128};

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
    pub tick_id: i64,
    pub order_id: u64,
    pub order_direction: OrderDirection,
    pub owner: Addr,
    pub quantity: Uint128,
    pub etas: Decimal256,
    pub claim_bounty: Option<Decimal256>,
    // Immutable quantity of the order when placed
    pub placed_quantity: Uint128,
    #[serde(default)]
    pub placed_at: Timestamp,
}

impl LimitOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tick_id: i64,
        order_id: u64,
        order_direction: OrderDirection,
        owner: Addr,
        quantity: Uint128,
        etas: Decimal256,
        claim_bounty: Option<Decimal256>,
    ) -> Self {
        LimitOrder {
            tick_id,
            order_id,
            order_direction,
            owner,
            quantity,
            etas,
            claim_bounty,
            placed_quantity: quantity,
            placed_at: Timestamp::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_placed_quantity(mut self, quantity: impl Into<Uint128>) -> Self {
        self.placed_quantity = quantity.into();
        self
    }

    pub(crate) fn with_placed_at(mut self, placed_at: Timestamp) -> Self {
        self.placed_at = placed_at;
        self
    }
}

#[cw_serde]
pub struct MarketOrder {
    pub quantity: Uint128,
    pub order_direction: OrderDirection,
    pub owner: Addr,
}

impl MarketOrder {
    pub fn new(quantity: Uint128, order_direction: OrderDirection, owner: Addr) -> Self {
        MarketOrder {
            quantity,
            order_direction,
            owner,
        }
    }
}

impl From<LimitOrder> for MarketOrder {
    fn from(limit_order: LimitOrder) -> Self {
        MarketOrder {
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
    ByTick(i64, Addr),
}

impl FilterOwnerOrders {
    pub fn all(owner: Addr) -> Self {
        FilterOwnerOrders::All(owner)
    }

    pub fn by_tick(tick_id: i64, owner: Addr) -> Self {
        FilterOwnerOrders::ByTick(tick_id, owner)
    }
}
