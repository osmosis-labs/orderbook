use cosmwasm_std::{Coin, Coins, Decimal};
use cosmwasm_std::{Decimal256, Uint128};
use osmosis_test_tube::{Account, Module, OsmosisTestApp};
use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};

use super::utils::orders;
use crate::constants::{MAX_TICK, MIN_TICK};
use crate::msg::{AllTicksResponse, CalcOutAmtGivenInResponse, QueryMsg, SpotPriceResponse};
use crate::tests::e2e::modules::cosmwasm_pool::CosmwasmPool;
use crate::tests::test_utils::decimal256_from_u128;
use crate::types::{LimitOrder, Orderbook};
use crate::{
    msg::{DenomsResponse, GetTotalPoolLiquidityResponse},
    setup,
    tests::e2e::test_env::TestEnv,
    types::OrderDirection,
};

#[test]
fn test_order_fuzz() {
    let seed: u64 = 123456789;
    let amount_orders = 100;
    let mut rng = StdRng::seed_from_u64(seed);

    let app = OsmosisTestApp::new();
    let cp = CosmwasmPool::new(&app);
    let mut t = setup!(app, "quote", "base");
    let mut orders = vec![];
    for i in 0..amount_orders {
        let username = format!("user{}", i);
        let chosen_tick = place_random_order(&mut t, &mut rng, &username);
        let is_cancelled = rng.gen_bool(0.1);
        if is_cancelled {
            orders::cancel_limit_success(&t, &username, chosen_tick, i);
            println!("cancelled order: {}", i);
        } else {
            orders.push((username, chosen_tick, i));
        }
        assert_tick_invariants(&mut t);
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

        let mut user_id = 0;
        while !liquidity.is_zero() && user_id < 1000 {
            let amount_raw = rng.gen_range(0..=liquidity.u128());
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

            let liquidity_at_price_u256 = if order_direction == OrderDirection::Bid {
                decimal256_from_u128(liquidity)
                    .checked_div(Decimal256::from(spot_price))
                    .unwrap()
            } else {
                decimal256_from_u128(liquidity)
                    .checked_mul(Decimal256::from(spot_price))
                    .unwrap()
            }
            .to_uint_floor();
            let liquidity_at_price = Uint128::try_from(liquidity_at_price_u256).unwrap();
            let amount = amount_raw.min(liquidity_at_price.u128());
            let expected_out =
                t.contract
                    .query::<CalcOutAmtGivenInResponse>(&QueryMsg::CalcOutAmountGivenIn {
                        token_in: Coin::new(amount, token_in_denom.to_string()),
                        token_out_denom: token_out_denom.to_string(),
                        swap_fee: Decimal::zero(),
                    });
            if amount == 0 || expected_out.is_err() {
                user_id += 1;
                continue;
            }
            let username = format!("user{}{}", order_direction, user_id);

            t.add_account(
                &username,
                vec![
                    Coin::new(amount, token_in_denom),
                    Coin::new(1000000000000000u128, "uosmo"),
                ],
            );
            orders::place_market_success(&cp, &t, order_direction, amount, &username);
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
            assert_tick_invariants(&mut t);
            user_id += 1;
        }
        println!("Placed {} orders in {} direction", user_id, order_direction);
        let GetTotalPoolLiquidityResponse {
            total_pool_liquidity,
        } = t
            .contract
            .query(&QueryMsg::GetTotalPoolLiquidity {})
            .unwrap();
        println!("Total pool liquidity: {:?}", total_pool_liquidity);
    }
    for (username, tick_id, order_id) in orders.iter() {
        t.add_account(
            "claimant",
            vec![
                Coin::new(1, "base"),
                Coin::new(1, "quote"),
                Coin::new(1000000000u128, "uosmo"),
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
        orders::claim_success(&t, sender, order.tick_id, order.order_id);
        println!("Claimed order: {}", order_id);
    }
}

fn place_random_order(t: &mut TestEnv, rng: &mut StdRng, username: &str) -> i64 {
    let quantity = Uint128::from(rng.gen::<u64>());
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
    t.add_account(
        username,
        vec![
            Coin::new(quantity.u128(), expected_denom),
            Coin::new(1000000000000000u128, "uosmo"),
        ],
    );
    assert!(t.accounts.contains_key(username));
    let tick_id = (rng.gen::<i16>() as i64).min(MAX_TICK).max(MIN_TICK);
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
        quantity,
        claim_bounty,
        username,
    )
    .unwrap();

    println!(
        "username: {}, sender: {}, tick_id: {}, order_direction: {}, quantity: {}, claim_bounty: {}",
        username,
        t.accounts[username].address(),
        tick_id,
        order_direction,
        quantity,
        claim_bounty.unwrap_or_default()
    );
    tick_id
}

fn assert_tick_invariants(t: &mut TestEnv) {
    let AllTicksResponse { ticks } = t
        .contract
        .query(&QueryMsg::AllTicks {
            start_from: None,
            end_at: None,
            limit: None,
        })
        .unwrap();

    let ticks_with_bid_amount = ticks.iter().filter(|tick| {
        !tick
            .tick_state
            .get_values(OrderDirection::Bid)
            .total_amount_of_liquidity
            .is_zero()
    });
    let ticks_with_ask_amount = ticks.iter().filter(|tick| {
        !tick
            .tick_state
            .get_values(OrderDirection::Ask)
            .total_amount_of_liquidity
            .is_zero()
    });
    let max_tick_with_bid = ticks_with_bid_amount.max_by_key(|tick| tick.tick_id);
    let min_tick_with_ask = ticks_with_ask_amount.min_by_key(|tick| tick.tick_id);

    let Orderbook {
        next_ask_tick,
        next_bid_tick,
        ..
    } = t.contract.query(&QueryMsg::OrderbookState {}).unwrap();
    if let Some(min_tick_with_ask) = min_tick_with_ask {
        assert_eq!(next_ask_tick, min_tick_with_ask.tick_id);
    }
    if let Some(max_tick_with_bid) = max_tick_with_bid {
        assert_eq!(next_bid_tick, max_tick_with_bid.tick_id);
    }
}
