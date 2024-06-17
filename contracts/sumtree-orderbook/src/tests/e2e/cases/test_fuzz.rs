use cosmwasm_std::{Coin, Coins, Decimal, Uint256};
use cosmwasm_std::{Decimal256, Uint128};
use osmosis_test_tube::{Module, OsmosisTestApp};
use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};

use super::utils::{assert, orders};
use crate::constants::{MAX_TICK, MIN_TICK};
use crate::msg::{AllTicksResponse, CalcOutAmtGivenInResponse, QueryMsg, SpotPriceResponse};
use crate::tests::e2e::modules::cosmwasm_pool::CosmwasmPool;
use crate::tick_math::{amount_to_value, tick_to_price, RoundingDirection};
use crate::types::LimitOrder;
use crate::{
    msg::{DenomsResponse, GetTotalPoolLiquidityResponse},
    setup,
    tests::e2e::test_env::TestEnv,
    types::OrderDirection,
};

#[test]
fn test_order_fuzz_large_orders_small_range() {
    run_fuzz_linear(2000, (-10, 10), 0.2);
}

#[test]
fn test_order_fuzz_small_orders_large_range() {
    run_fuzz_linear(100, (MIN_TICK, MAX_TICK), 0.2);
}

#[test]
fn test_order_fuzz_small_orders_small_range() {
    run_fuzz_linear(100, (-10, 0), 0.1);
}

#[test]
fn test_order_fuzz_large_cancelled_orders_small_range() {
    run_fuzz_linear(1000, (MIN_TICK, MIN_TICK + 20), 0.8);
}

// #[test]
// fn test_order_fuzz_very_large_orders_no_bounds() {
//     run_fuzz(3000, (-750, 750), 0.2);
// }

fn run_fuzz_linear(amount_limit_orders: u64, tick_range: (i64, i64), cancel_probability: f64) {
    let seed: u64 = 123456789;
    let mut rng = StdRng::seed_from_u64(seed);

    let app = OsmosisTestApp::new();
    let cp = CosmwasmPool::new(&app);
    let mut t = setup!(app, "quote", "base");
    let mut orders = vec![];
    for i in 0..amount_limit_orders {
        let username = format!("user{}", i);
        let chosen_tick = place_random_order(&mut t, &mut rng, &username, tick_range);
        let is_cancelled = rng.gen_bool(cancel_probability);
        if is_cancelled {
            orders::cancel_limit_success(&t, &username, chosen_tick, i);
        } else {
            orders.push((username, chosen_tick, i));
        }
        assert::tick_invariants(&t);
    }

    for order_direction in [OrderDirection::Bid, OrderDirection::Ask] {
        let GetTotalPoolLiquidityResponse {
            total_pool_liquidity,
        } = t
            .contract
            .query(&QueryMsg::GetTotalPoolLiquidity {})
            .unwrap();
        let mut liquidity = if order_direction == OrderDirection::Bid {
            Coins::try_from(total_pool_liquidity.clone())
                .unwrap()
                .amount_of("base")
        } else {
            Coins::try_from(total_pool_liquidity.clone())
                .unwrap()
                .amount_of("quote")
        };

        let mut zero_amount_returns = 0;
        let mut user_id = 0;
        while liquidity.gt(&Uint128::one()) {
            let username = format!("user{}{}", order_direction, user_id);
            let placed_amount = place_random_market(
                &cp,
                &mut t,
                &mut rng,
                &username,
                order_direction,
                liquidity.u128(),
            );

            user_id += 1;
            if placed_amount == 0 {
                zero_amount_returns += 1;
                if zero_amount_returns == 100 {
                    break;
                }
                continue;
            }

            let GetTotalPoolLiquidityResponse {
                total_pool_liquidity,
            } = t
                .contract
                .query(&QueryMsg::GetTotalPoolLiquidity {})
                .unwrap();
            liquidity = if order_direction == OrderDirection::Bid {
                Coins::try_from(total_pool_liquidity.clone())
                    .unwrap()
                    .amount_of("base")
            } else {
                Coins::try_from(total_pool_liquidity.clone())
                    .unwrap()
                    .amount_of("quote")
            };
            assert::tick_invariants(&t);
        }
        println!(
            "Placed {} market orders in {} direction",
            user_id, order_direction
        );
    }

    let GetTotalPoolLiquidityResponse {
        total_pool_liquidity,
    } = t
        .contract
        .query(&QueryMsg::GetTotalPoolLiquidity {})
        .unwrap();
    println!("Total remaining pool liquidity: {:?}", total_pool_liquidity);
    orders.reverse();
    for (username, tick_id, order_id) in orders.iter() {
        t.add_account(
            "claimant",
            vec![
                Coin::new(1, "base"),
                Coin::new(1, "quote"),
                Coin::new(1000000000000u128, "uosmo"),
            ],
        );
        let order: LimitOrder = t
            .contract
            .query(&QueryMsg::Order {
                order_id: *order_id,
                tick_id: *tick_id,
            })
            .unwrap();
        let sender = if order.claim_bounty.is_some() {
            "claimant"
        } else {
            username
        };
        let AllTicksResponse { ticks } = t
            .contract
            .query(&QueryMsg::AllTicks {
                start_from: Some(order.tick_id),
                end_at: None,
                limit: Some(1),
            })
            .unwrap();
        let tick = ticks.first().unwrap();
        let price = tick_to_price(tick.tick_id).unwrap();
        let value = amount_to_value(
            order.order_direction,
            order.quantity,
            price,
            RoundingDirection::Down,
        )
        .unwrap();
        let contract_balance = Coins::try_from(t.get_balance(&t.contract.contract_addr)).unwrap();

        // We cannot verify how much to expect as tick is synced as part of the claim process
        // Hence orders::claim is used instead of orders::claim_success
        match orders::claim_success(&t, sender, order.tick_id, order.order_id) {
            Ok(res) => {
                let gas_used = res.gas_info.gas_used;
                if gas_used >= 200000 {
                    println!("gas_used: {}", res.gas_info.gas_used);
                }
            }
            Err(e) => {
                println!("Failed to claim order {}: {:?}", order.order_id, e);
                println!("contract_balance: {:?}", contract_balance);
                println!(
                    "order etas: {}, price: {}, value: {}, tick etas: {}",
                    order.etas,
                    price,
                    value,
                    tick.tick_state
                        .get_values(order.order_direction)
                        .effective_total_amount_swapped
                );
            }
        }

        let maybe_order = t.contract.query::<LimitOrder>(&QueryMsg::Order {
            order_id: *order_id,
            tick_id: *tick_id,
        });
        if let Ok(order) = maybe_order {
            println!("order: {:?}", order);
        }
    }
}

fn place_random_order(
    t: &mut TestEnv,
    rng: &mut StdRng,
    username: &str,
    tick_range: (i64, i64),
) -> i64 {
    let quantity = Uint128::from(rng.gen::<u32>());
    let order_direction = if rng.gen_bool(0.5) {
        OrderDirection::Bid
    } else {
        OrderDirection::Ask
    };
    let DenomsResponse {
        base_denom,
        quote_denom,
    } = t.contract.get_denoms();
    let expected_denom = if order_direction == OrderDirection::Bid {
        &quote_denom
    } else {
        &base_denom
    };
    let tick_id = rng.gen_range(tick_range.0..=tick_range.1);
    let price = tick_to_price(tick_id).unwrap();
    let min = Uint128::try_from(
        amount_to_value(
            order_direction.opposite(),
            Uint128::one(),
            price,
            RoundingDirection::Up,
        )
        .unwrap()
        .min(Uint256::from(Uint128::MAX)),
    )
    .unwrap();

    t.add_account(
        username,
        vec![
            Coin::new(quantity.u128().max(min.u128()), expected_denom),
            Coin::new(1000000000000000u128, "uosmo"),
        ],
    );

    let has_claim_bounty = rng.gen_bool(0.8);
    let claim_bounty = if has_claim_bounty {
        Some(Decimal256::percent(rng.gen_range(0..=1)))
    } else {
        None
    };

    orders::place_limit(
        t,
        tick_id,
        order_direction,
        quantity.max(min),
        claim_bounty,
        username,
    )
    .unwrap();

    // println!(
    //     "username: {}, sender: {}, tick_id: {}, order_direction: {}, quantity: {}, claim_bounty: {}",
    //     username,
    //     t.accounts[username].address(),
    //     tick_id,
    //     order_direction,
    //     quantity,
    //     claim_bounty.unwrap_or_default()
    // );
    tick_id
}

fn place_random_market(
    cp: &CosmwasmPool<OsmosisTestApp>,
    t: &mut TestEnv,
    rng: &mut StdRng,
    username: &str,
    order_direction: OrderDirection,
    max: u128,
) -> u128 {
    let (token_in_denom, token_out_denom) = if order_direction == OrderDirection::Bid {
        ("quote", "base")
    } else {
        ("base", "quote")
    };
    let SpotPriceResponse { spot_price } = t
        .contract
        .query(&QueryMsg::SpotPrice {
            base_asset_denom: token_in_denom.to_string(),
            quote_asset_denom: token_out_denom.to_string(),
        })
        .unwrap();
    let liquidity_at_price_u256 = amount_to_value(
        order_direction.opposite(),
        Uint128::from(max),
        Decimal256::from(spot_price),
        RoundingDirection::Up,
    )
    .unwrap();

    let liquidity_at_price = Uint128::try_from(liquidity_at_price_u256).unwrap();
    let amount = rng.gen_range(0..=liquidity_at_price.u128());
    let expected_out =
        t.contract
            .query::<CalcOutAmtGivenInResponse>(&QueryMsg::CalcOutAmountGivenIn {
                token_in: Coin::new(amount, token_in_denom.to_string()),
                token_out_denom: token_out_denom.to_string(),
                swap_fee: Decimal::zero(),
            });
    if amount == 0 || expected_out.is_err() || expected_out.unwrap().token_out.amount == "0" {
        return 0;
    }

    t.add_account(
        username,
        vec![
            Coin::new(amount, token_in_denom),
            Coin::new(1000000000000000u128, "uosmo"),
        ],
    );
    orders::place_market_success(cp, t, order_direction, amount, username).unwrap();
    amount
}
