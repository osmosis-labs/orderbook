use cosmwasm_schema::cw_serde;

use crate::{error::ContractResult, ContractError};

use super::OrderDirection;

#[cw_serde]
pub struct Orderbook {
    pub book_id: u64,

    pub quote_denom: String,
    pub base_denom: String,

    // Note that ticks can be negative
    pub current_tick: i64,
    pub next_bid_tick: i64,
    pub next_ask_tick: i64,
}

impl Orderbook {
    pub fn new(
        book_id: u64,
        quote_denom: String,
        base_denom: String,
        current_tick: i64,
        next_bid_tick: i64,
        next_ask_tick: i64,
    ) -> Self {
        Orderbook {
            book_id,
            quote_denom,
            base_denom,
            current_tick,
            next_bid_tick,
            next_ask_tick,
        }
    }

    /// Get the expected denomination for a given order direction.
    #[inline]
    pub fn get_expected_denom(&self, order_direction: &OrderDirection) -> String {
        match order_direction {
            OrderDirection::Bid => self.quote_denom.clone(),
            OrderDirection::Ask => self.base_denom.clone(),
        }
    }

    /// Get the opposite denomination for a given order direction.
    #[inline]
    pub fn get_opposite_denom(&self, order_direction: &OrderDirection) -> String {
        match order_direction {
            OrderDirection::Bid => self.base_denom.clone(),
            OrderDirection::Ask => self.quote_denom.clone(),
        }
    }

    /// Determines the order direction given a token denom pair.
    ///
    /// Errors if the given pair does not match the current orderbook.
    #[inline]
    pub fn direction_from_pair(
        &self,
        token_in_denom: String,
        token_out_denom: String,
    ) -> ContractResult<OrderDirection> {
        let in_out_tuple: (String, String) = (token_in_denom.clone(), token_out_denom.clone());

        // Determine order direction based on token in/out denoms
        let order_direction = if (self.base_denom.clone(), self.quote_denom.clone()) == in_out_tuple
        {
            OrderDirection::Ask
        } else if (self.quote_denom.clone(), self.base_denom.clone()) == in_out_tuple {
            OrderDirection::Bid
        } else {
            return Err(ContractError::InvalidPair {
                token_in_denom,
                token_out_denom,
            });
        };

        Ok(order_direction)
    }
}
