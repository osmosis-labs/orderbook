use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal256;

/// Represents the state of a specific price tick in a liquidity pool.
#[cw_serde]
pub struct TickState {
    /// Total Amount of Liquidity at tick (TAL)
    /// - Every limit order placement increments this value.
    /// - Every swap at this tick decrements this value.
    /// - Every cancellation decrements this value.
    pub total_amount_of_liquidity: Decimal256,

    /// Cumulative Total Limits at tick (CTT)
    /// - Every limit order placement increments this value.
    /// - There might be an edge-case optimization to lower this value.
    pub cumulative_total_value: Decimal256,

    /// Effective Total Amount Swapped at tick (ETAS)
    /// - Every swap increments ETAS by the swap amount.
    /// - There will be other ways to update ETAS as described below.
    pub effective_total_amount_swapped: Decimal256,
}

impl Default for TickState {
    fn default() -> Self {
        TickState {
            total_amount_of_liquidity: Decimal256::zero(),
            cumulative_total_value: Decimal256::zero(),
            effective_total_amount_swapped: Decimal256::zero(),
        }
    }
}
