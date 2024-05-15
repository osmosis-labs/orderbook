use cosmwasm_std::{
    coin, testing::mock_info, Addr, Decimal256, DepsMut, Env, MessageInfo, Uint128,
};

use crate::{
    constants::{MAX_TICK, MIN_TICK},
    error::ContractResult,
    order::{cancel_limit, claim_order, place_limit, run_market_order},
    state::orders,
    types::{LimitOrder, MarketOrder, OrderDirection},
};

use super::test_constants::{MOCK_BASE_DENOM, MOCK_QUOTE_DENOM};

// Tick Price = 2
pub(crate) const LARGE_POSITIVE_TICK: i64 = 1000000;
// Tick Price = 0.5
pub(crate) const LARGE_NEGATIVE_TICK: i64 = -5000000;

pub(crate) fn decimal256_from_u128(input: impl Into<u128>) -> Decimal256 {
    Decimal256::from_ratio(input.into(), 1u128)
}

#[derive(Clone)]
pub(crate) enum OrderOperation {
    RunMarket(MarketOrder),
    PlaceLimitMulti((Vec<i64>, usize, Uint128, OrderDirection)),
    PlaceLimit(LimitOrder),
    Claim((i64, u64)),
    Cancel((i64, u64)),
}

impl OrderOperation {
    pub(crate) fn run(&self, mut deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<()> {
        match self.clone() {
            OrderOperation::RunMarket(mut order) => {
                let tick_bound = match order.order_direction {
                    OrderDirection::Bid => MAX_TICK,
                    OrderDirection::Ask => MIN_TICK,
                };
                run_market_order(deps.storage, env.contract.address, &mut order, tick_bound)
                    .unwrap();
                Ok(())
            }
            OrderOperation::PlaceLimitMulti((
                tick_ids,
                orders_per_tick,
                quantity_per_order,
                direction,
            )) => {
                let orders = generate_limit_orders(
                    tick_ids.as_slice(),
                    orders_per_tick,
                    quantity_per_order,
                    direction,
                );
                place_multiple_limit_orders(&mut deps, env, info.sender.as_str(), orders).unwrap();
                Ok(())
            }
            OrderOperation::PlaceLimit(limit_order) => {
                let coin_vec = vec![coin(
                    limit_order.quantity.u128(),
                    match limit_order.order_direction {
                        OrderDirection::Ask => MOCK_BASE_DENOM,
                        OrderDirection::Bid => MOCK_QUOTE_DENOM,
                    },
                )];
                let info = mock_info(info.sender.as_str(), &coin_vec);
                place_limit(
                    &mut deps,
                    env,
                    info,
                    limit_order.tick_id,
                    limit_order.order_direction,
                    limit_order.quantity,
                    limit_order.claim_bounty,
                )?;
                Ok(())
            }
            OrderOperation::Claim((tick_id, order_id)) => {
                claim_order(
                    deps.storage,
                    info.sender.clone(),
                    env.contract.address,
                    tick_id,
                    order_id,
                )
                .unwrap();
                Ok(())
            }
            OrderOperation::Cancel((tick_id, order_id)) => {
                let order = orders()
                    .load(deps.as_ref().storage, &(tick_id, order_id))
                    .unwrap();
                let info = mock_info(order.owner.as_str(), &[]);
                cancel_limit(deps, env, info, tick_id, order_id).unwrap();
                Ok(())
            }
        }
    }
}

/// Generates a set of `LimitOrder` objects for testing purposes.
/// `orders_per_tick` orders are generated for each tick in `tick_ids`,
/// with order direction being determined such that they are all placed
/// around `current_tick`.
pub(crate) fn generate_limit_orders(
    tick_ids: &[i64],
    orders_per_tick: usize,
    quantity_per_order: Uint128,
    order_direction: OrderDirection,
) -> Vec<LimitOrder> {
    let mut orders = Vec::new();
    for &tick_id in tick_ids {
        for _ in 0..orders_per_tick {
            let order = LimitOrder {
                tick_id,
                order_direction,
                owner: Addr::unchecked("creator"),
                quantity: quantity_per_order,

                // We set these values to zero since they will be unused anyway
                order_id: 0,
                etas: Decimal256::zero(),
                claim_bounty: None,
            };
            orders.push(order);
        }
    }
    orders
}

/// Places a vector of limit orders on the current orderbook for a specified owner.
pub(crate) fn place_multiple_limit_orders(
    deps: &mut DepsMut,
    env: Env,
    owner: &str,
    orders: Vec<LimitOrder>,
) -> ContractResult<()> {
    for order in orders {
        let coin_vec = vec![coin(
            order.quantity.u128(),
            match order.order_direction {
                OrderDirection::Ask => MOCK_BASE_DENOM,
                OrderDirection::Bid => MOCK_QUOTE_DENOM,
            },
        )];
        let info = mock_info(owner, &coin_vec);

        // Place the limit order
        place_limit(
            deps,
            env.clone(),
            info,
            order.tick_id,
            order.order_direction,
            order.quantity,
            order.claim_bounty,
        )?;
    }
    Ok(())
}

#[allow(clippy::uninlined_format_args)]
pub(crate) fn format_test_name(name: &str) -> String {
    format!("\n\nTest case failed: {}\n", name)
}

pub(crate) fn generate_tick_ids(amount: u64) -> Vec<i64> {
    (0..amount as i64).collect::<Vec<i64>>()
}
