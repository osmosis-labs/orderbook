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
    _PlaceLimitMulti((&'static [i64], usize, Uint128, i64)),
    PlaceLimit(LimitOrder),
    Claim((u64, i64, u64)),
    Cancel((u64, i64, u64)),
}

impl OrderOperation {
    pub(crate) fn run(
        &self,
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        book_id: u64,
    ) -> ContractResult<()> {
        match self.clone() {
            OrderOperation::RunMarket(mut order) => {
                let tick_bound = match order.order_direction {
                    OrderDirection::Bid => MAX_TICK,
                    OrderDirection::Ask => MIN_TICK,
                };
                run_market_order(deps.storage, &mut order, tick_bound).unwrap();
                Ok(())
            }
            OrderOperation::_PlaceLimitMulti((
                tick_ids,
                orders_per_tick,
                quantity_per_order,
                current_tick,
            )) => {
                let orders = generate_limit_orders(
                    book_id,
                    tick_ids,
                    current_tick,
                    orders_per_tick,
                    quantity_per_order,
                );
                place_multiple_limit_orders(&mut deps, env, info.sender.as_str(), book_id, orders)
                    .unwrap();
                Ok(())
            }
            OrderOperation::PlaceLimit(limit_order) => {
                let coin_vec = vec![coin(
                    limit_order.quantity.u128(),
                    match limit_order.order_direction {
                        OrderDirection::Ask => "base",
                        OrderDirection::Bid => "quote",
                    },
                )];
                let info = mock_info(info.sender.as_str(), &coin_vec);
                place_limit(
                    &mut deps,
                    env,
                    info,
                    limit_order.book_id,
                    limit_order.tick_id,
                    limit_order.order_direction,
                    limit_order.quantity,
                    limit_order.auto_claim_bounty,
                )?;
                Ok(())
            }
            OrderOperation::Claim((book_id, tick_id, order_id)) => {
                claim_order(deps.storage, book_id, tick_id, order_id).unwrap();
                Ok(())
            }
            OrderOperation::Cancel((book_id, tick_id, order_id)) => {
                let order = orders()
                    .load(deps.as_ref().storage, &(book_id, tick_id, order_id))
                    .unwrap();
                let info = mock_info(order.owner.as_str(), &[]);
                cancel_limit(deps, env, info, book_id, tick_id, order_id).unwrap();
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
    book_id: u64,
    tick_ids: &[i64],
    current_tick: i64,
    orders_per_tick: usize,
    quantity_per_order: Uint128,
) -> Vec<LimitOrder> {
    let mut orders = Vec::new();
    for &tick_id in tick_ids {
        let order_direction = if tick_id < current_tick {
            OrderDirection::Bid
        } else {
            OrderDirection::Ask
        };

        for _ in 0..orders_per_tick {
            let order = LimitOrder {
                book_id,
                tick_id,
                order_direction,
                owner: Addr::unchecked("creator"),
                quantity: quantity_per_order,

                // We set these values to zero since they will be unused anyway
                order_id: 0,
                etas: Decimal256::zero(),
                auto_claim_bounty: None,
            };
            orders.push(order);
        }
    }
    orders
}

/// Places a vector of limit orders on the given book_id for a specified owner.
pub(crate) fn place_multiple_limit_orders(
    deps: &mut DepsMut,
    env: Env,
    owner: &str,
    book_id: u64,
    orders: Vec<LimitOrder>,
) -> ContractResult<()> {
    for order in orders {
        let coin_vec = vec![coin(
            order.quantity.u128(),
            match order.order_direction {
                OrderDirection::Ask => "base",
                OrderDirection::Bid => "quote",
            },
        )];
        let info = mock_info(owner, &coin_vec);

        // Place the limit order
        place_limit(
            deps,
            env.clone(),
            info,
            book_id,
            order.tick_id,
            order.order_direction,
            order.quantity,
            order.auto_claim_bounty,
        )?;
    }
    Ok(())
}

#[allow(clippy::uninlined_format_args)]
pub(crate) fn format_test_name(name: &str) -> String {
    format!("\n\nTest case failed: {}\n", name)
}
