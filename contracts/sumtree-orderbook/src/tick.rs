use crate::{
    error::ContractError,
    state::TICK_STATE,
    sumtree::tree::{get_or_init_root_node, get_prefix_sum},
    types::OrderDirection,
};
use cosmwasm_std::{ensure, Decimal256, Storage};

/// Syncs the tick state, ensuring that its ETAS reflects cancellations that have occurred
/// up until the `current_tick_etas`
pub fn sync_tick(
    storage: &mut dyn Storage,
    tick_id: i64,
    current_tick_bid_etas: Decimal256,
    current_tick_ask_etas: Decimal256,
) -> Result<(), ContractError> {
    let mut tick_state = TICK_STATE.load(storage, tick_id)?;

    // Assuming `get_values` returns a type that can be stored directly in a tuple.
    // Adjust the type `(OrderDirection, YourValueType)` as necessary.
    let mut bid_values = tick_state.get_values(OrderDirection::Bid);
    let mut ask_values = tick_state.get_values(OrderDirection::Ask);

    // If both sets of tick values are already up to date, skip the sync altogether.
    if bid_values.last_tick_sync_etas == current_tick_bid_etas
        && ask_values.last_tick_sync_etas == current_tick_ask_etas
    {
        return Ok(());
    }

    // Sync tick for each order direction.
    //
    // We handle this by iterating through order direction
    // and only writing changes to tick values at the end of each iteration, allowing us to
    // cleanly bubble up the changes to write to state after the loop without running duplicate
    // calls for each direction.
    for &direction in [OrderDirection::Bid, OrderDirection::Ask].iter() {
        let (mut tick_value, target_etas) = match direction {
            OrderDirection::Bid => (bid_values.clone(), current_tick_bid_etas),
            OrderDirection::Ask => (ask_values.clone(), current_tick_ask_etas),
        };

        // If tick state for current order direction is already up to date,
        // skip the check. This saves us from walking the tree for both order directions
        // even though in most cases we will likely only need to sync one.
        if tick_value.last_tick_sync_etas >= target_etas {
            continue;
        }

        // Get previous cumulative realized cancels to compare against for ETAS updates.
        let old_cumulative_realized_cancels = tick_value.cumulative_realized_cancels;

        // Fetch sumtree for tick by order direction. If none exists, initialize one.
        let tree = get_or_init_root_node(storage, tick_id, direction)?;

        // Assuming `calculate_prefix_sum` is a function that calculates the prefix sum at the given ETAS.
        // This function needs to be implemented based on your sumtree structure and logic.
        let new_cumulative_realized_cancels = get_prefix_sum(storage, tree, target_etas)?;

        // Calculate the growth in realized cancels since previous sync.
        // This is equivalent to the amount we will need to add to the tick's ETAS.
        let realized_since_last_sync =
            new_cumulative_realized_cancels.checked_sub(old_cumulative_realized_cancels)?;

        // Update the tick state to represent new ETAS and new cumulative realized cancels.
        tick_value.effective_total_amount_swapped = tick_value
            .effective_total_amount_swapped
            .checked_add(realized_since_last_sync)?;
        tick_value.cumulative_realized_cancels = new_cumulative_realized_cancels;
        tick_value.last_tick_sync_etas = target_etas;

        // Defense in depth guardrail: ensure that tick sync does not push tick ETAS past CTT.
        ensure!(
            tick_value.effective_total_amount_swapped <= tick_value.cumulative_total_value,
            ContractError::InvalidTickSync
        );

        // Write changes to appropriate tick values by direction.
        // These will be written to tick state after both have been updated.
        match direction {
            OrderDirection::Bid => bid_values = tick_value,
            OrderDirection::Ask => ask_values = tick_value,
        };
    }

    // Write updated tick values to state
    tick_state.set_values(OrderDirection::Bid, bid_values);
    tick_state.set_values(OrderDirection::Ask, ask_values);
    TICK_STATE.save(storage, tick_id, &tick_state)?;

    Ok(())
}
