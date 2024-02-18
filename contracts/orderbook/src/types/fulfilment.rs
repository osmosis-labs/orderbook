use cosmwasm_std::Uint128;

use super::LimitOrder;

pub struct Fulfilment {
    pub order: LimitOrder,
    pub amount: Uint128,
}

impl Fulfilment {
    pub fn new(order: LimitOrder, amount: Uint128) -> Self {
        Self { order, amount }
    }
}
