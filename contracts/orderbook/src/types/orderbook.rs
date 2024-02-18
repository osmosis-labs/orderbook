use cosmwasm_schema::cw_serde;

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
    pub fn get_expected_denom_for_direction(&self, order_direction: &OrderDirection) -> String {
        match order_direction {
            OrderDirection::Bid => self.quote_denom.clone(),
            OrderDirection::Ask => self.base_denom.clone(),
        }
    }
}
