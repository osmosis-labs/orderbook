use cosmwasm_schema::cw_serde;

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