use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal256;

use super::OrderDirection;

#[cw_serde]
pub struct TickValues {
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

    /// Cumulative Realized Cancellations at tick
    /// - Increases as cancellations are checkpointed in batches on the sumtree
    /// - Equivalent to the prefix sum at the tick's current ETAS after being synced
    pub cumulative_realized_cancels: Decimal256,

    /// last_tick_sync_etas is the ETAS value after the most recent tick sync.
    /// It is used to skip tick syncs if ETAS has not changed since the previous
    /// sync.
    pub last_tick_sync_etas: Decimal256,
}

impl Default for TickValues {
    fn default() -> Self {
        TickValues {
            total_amount_of_liquidity: Decimal256::zero(),
            cumulative_total_value: Decimal256::zero(),
            effective_total_amount_swapped: Decimal256::zero(),
            cumulative_realized_cancels: Decimal256::zero(),
            last_tick_sync_etas: Decimal256::zero(),
        }
    }
}

/// Represents the state of a specific price tick in a liquidity pool.
///
/// The state is split into two parts for the ask and bid directions.
#[cw_serde]
#[derive(Default)]
pub struct TickState {
    /// Values for the ask direction of the tick
    pub ask_values: TickValues,
    /// Values for the bid direction of the tick
    pub bid_values: TickValues,
}

impl TickState {
    pub fn get_values(&self, direction: OrderDirection) -> TickValues {
        if direction == OrderDirection::Ask {
            self.ask_values.clone()
        } else {
            self.bid_values.clone()
        }
    }

    pub fn set_values(&mut self, direction: OrderDirection, values: TickValues) {
        if direction == OrderDirection::Ask {
            self.ask_values = values;
        } else {
            self.bid_values = values;
        }
    }
}

/// Represents the state of a specific price tick in a liquidity pool.
///
/// The state is split into two parts for the ask and bid directions.
#[cw_serde]
#[derive(Default)]
pub struct TickState {
    /// Values for the ask direction of the tick
    pub ask_values: TickValues,
    /// Values for the bid direction of the tick
    pub bid_values: TickValues,
}

impl TickState {
    pub fn get_values(&self, direction: OrderDirection) -> TickValues {
        if direction == OrderDirection::Ask {
            self.ask_values.clone()
        } else {
            self.bid_values.clone()
        }
    }

    pub fn set_values(&mut self, direction: OrderDirection, values: TickValues) {
        if direction == OrderDirection::Ask {
            self.ask_values = values;
        } else {
            self.bid_values = values;
        }
    }
}
