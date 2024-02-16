use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct LimitOrder {
    pub order_id: u64,
    pub side: bool,
    pub maker: Addr,
    pub amount: Uint128,
}

impl LimitOrder {
    pub fn new(order_id: u64, side: bool, maker: Addr, amount: Uint128) -> Self {
        LimitOrder {
            order_id,
            side,
            maker,
            amount,
        }
    }
}