use cosmwasm_std::Uint128;

use super::LimitOrder;

// Describes orders to be fulfilllled as part of a market order or a converted limit order
#[derive(Clone, Debug, PartialEq)]
pub struct Fulfillment {
    pub order: LimitOrder,
    pub amount: Uint128,
}

impl Fulfillment {
    pub fn new(order: LimitOrder, amount: Uint128) -> Self {
        Self { order, amount }
    }
}
