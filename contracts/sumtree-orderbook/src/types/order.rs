use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, ensure, Addr, BankMsg, Uint128};

use crate::{
    tick_math::{amount_to_value, tick_to_price},
    ContractError,
};

#[cw_serde]
#[derive(Copy)]
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

    // Transfers the specified quantity of the order's asset to the owner
    pub fn fill(
        &mut self,
        denom: impl Into<String>,
        quantity: Uint128,
    ) -> Result<BankMsg, ContractError> {
        ensure!(
            self.quantity >= quantity,
            ContractError::InvalidFulfillment {
                order_id: self.order_id,
                book_id: self.book_id,
                amount_required: quantity,
                amount_remaining: self.quantity,
                reason: Some("Order does not have enough funds".to_string())
            }
        );
        self.quantity = self.quantity.checked_sub(quantity)?;
        // Determine price
        let price = tick_to_price(self.tick_id)?;
        // Multiply quantity by price
        let amount_to_send = amount_to_value(self.order_direction, quantity, price)?;
        Ok(BankMsg::Send {
            to_address: self.owner.to_string(),
            amount: vec![coin(amount_to_send.u128(), denom.into())],
        })
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
