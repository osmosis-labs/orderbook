use cosmwasm_std::{Coin, Coins, Decimal, Uint256};
use cosmwasm_std::{Decimal256, Uint128};
use osmosis_test_tube::{Module, OsmosisTestApp};
use rand::seq::SliceRandom;
use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};

use super::utils::{assert, orders};
use crate::constants::MIN_TICK;
use crate::msg::{CalcOutAmtGivenInResponse, QueryMsg, SpotPriceResponse};
use crate::tests::e2e::modules::cosmwasm_pool::CosmwasmPool;
use crate::tick_math::{amount_to_value, tick_to_price, RoundingDirection};
use crate::types::LimitOrder;
use crate::{
    msg::{DenomsResponse, GetTotalPoolLiquidityResponse},
    setup,
    tests::e2e::test_env::TestEnv,
    types::OrderDirection,
};

// Tick Price = 2
pub(crate) const LARGE_POSITIVE_TICK: i64 = 1000000;
// Tick Price = 0.5
pub(crate) const LARGE_NEGATIVE_TICK: i64 = -5000000;

#[test]
fn test_order_fuzz_large_orders_small_range() {
    run_fuzz_linear(2000, (-10, 10), 0.2);
}

#[test]
fn test_order_fuzz_small_orders_large_range() {
    run_fuzz_linear(100, (LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK), 0.2);
}

#[test]
fn test_order_fuzz_small_orders_small_range() {
    run_fuzz_linear(100, (-10, 0), 0.1);
}

#[test]
fn test_order_fuzz_large_cancelled_orders_small_range() {
    run_fuzz_linear(1000, (MIN_TICK, MIN_TICK + 20), 0.8);
}

// This test takes a very long time to run
// #[test]
// fn test_order_fuzz_very_large_orders_no_bounds() {
//     run_fuzz(3000, (-750, 750), 0.2);
// }

/// Runs a linear fuzz test with the following steps
/// 1. Place x amount of random limit orders in given tick range and cancel with provided probability
/// 2. For both directions fill the entire amount of liquidity available using market orders
/// 3. Claim orders in random order
/// 4. Assert that the orders were filled correctly
fn run_fuzz_linear(amount_limit_orders: u64, tick_range: (i64, i64), cancel_probability: f64) {
    // -- Test Setup --
    let seed: u64 = 123456789;
    let mut rng = StdRng::seed_from_u64(seed);

    let app = OsmosisTestApp::new();
    let cp = CosmwasmPool::new(&app);
    let mut t = setup!(app, "quote", "base");
    let mut orders = vec![];

    // -- System Under Test --

    // Places the set amount of orders within the provided tick range
    // Orders will be cancelled with a chance equal to the provided cancel_probability
    // Tick state is verified after every order is placed (and cancelled)
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

    // For both directions fill the entire amount of liquidity available using market orders
    // For certain cases it is not possible to fill the entire liquidity so a remainder of 1 may occur
    for order_direction in [OrderDirection::Bid, OrderDirection::Ask] {
        // Determine the amount of liquidity for the given direction
        let mut liquidity = t.contract.get_directional_liquidity(order_direction);

        let mut zero_amount_returns = 0;
        let mut user_id = 0;

        // While there is some fillable liquidity we want to place randomised market orders
        while liquidity > 1u128 {
            let username = format!("user{}{}", order_direction, user_id);
            let placed_amount =
                place_random_market(&cp, &mut t, &mut rng, &username, order_direction, liquidity);

            // Increment the username of the order placer
            user_id += 1;
            if placed_amount == 0 {
                // In the case that the last order cannot be filled we want an exit condition
                // If there are 100 consecutive zero amount returns we will break
                zero_amount_returns += 1;
                if zero_amount_returns == 100 {
                    break;
                }
                continue;
            }

            // Reset counter as order was placed
            zero_amount_returns = 0;

            // Update the liquidity
            liquidity = t.contract.get_directional_liquidity(order_direction);
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

    // Shuffle the order of recorded orders (as liquidity is fully filled (except the possibility of a 1 remainder))
    // every order should be claimable and the order should not matter
    orders.shuffle(&mut rng);

    let mut remainder_orders = 0;
    for (username, tick_id, order_id) in orders.iter() {
        // If the order has a claim bounty we will use a separate sender to verify that the bounty is claimed correctly
        // Otherwise we will use the original sender to verify that the order is claimed correctly
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

        orders::claim(&t, sender, order.tick_id, order.order_id).unwrap();

        // For the situation that the order has the 1 remainder we record this for assertions
        let maybe_order = t.contract.query::<LimitOrder>(&QueryMsg::Order {
            order_id: *order_id,
            tick_id: *tick_id,
        });
        if let Ok(order) = maybe_order {
            println!("order: {:?}", order);
            remainder_orders += 1;
        }
    }

    // Assert orders were filled correctly
    assert!(
        remainder_orders <= 2,
        "There should be at most 2 orders that have a remainder, received {}",
        remainder_orders
    );
}

/// Places a random limit order in the provided tick range using the provided username
fn place_random_order(
    t: &mut TestEnv,
    rng: &mut StdRng,
    username: &str,
    tick_range: (i64, i64),
) -> i64 {
    // Quantities are in magnitudes of u32
    let quantity = Uint128::from(rng.gen::<u32>());
    // 50% chance to choose either direction
    let order_direction = if rng.gen_bool(0.5) {
        OrderDirection::Bid
    } else {
        OrderDirection::Ask
    };

    // Get the appropriate denom for the chosen direction
    let DenomsResponse {
        base_denom,
        quote_denom,
    } = t.contract.get_denoms();
    let expected_denom = if order_direction == OrderDirection::Bid {
        &quote_denom
    } else {
        &base_denom
    };
    // Select a random tick from the provided range
    let tick_id = rng.gen_range(tick_range.0..=tick_range.1);
    // Convert the tick to a price
    let price = tick_to_price(tick_id).unwrap();
    // Calculate the minimum amount of the denom that can be bought at the given price
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

    // Add the user account with the appropriate amount of the denom
    t.add_account(
        username,
        vec![
            Coin::new(quantity.u128().max(min.u128()), expected_denom),
            Coin::new(1000000000000000u128, "uosmo"),
        ],
    );

    // Give orders an 80% chance of having a randomised bounty (may be 0)
    let has_claim_bounty = rng.gen_bool(0.8);
    let claim_bounty = if has_claim_bounty {
        Some(Decimal256::percent(rng.gen_range(0..=1)))
    } else {
        None
    };

    // Place the generated limit
    orders::place_limit(
        t,
        tick_id,
        order_direction,
        quantity.max(min),
        claim_bounty,
        username,
    )
    .unwrap();

    // Return the tick id to record the order
    tick_id
}

/// Places a random market order in the provided tick range using the provided username with at most max value
fn place_random_market(
    cp: &CosmwasmPool<OsmosisTestApp>,
    t: &mut TestEnv,
    rng: &mut StdRng,
    username: &str,
    order_direction: OrderDirection,
    max: u128,
) -> u128 {
    // Get the appropriate denom for the chosen direction
    let (token_in_denom, token_out_denom) = if order_direction == OrderDirection::Bid {
        ("quote", "base")
    } else {
        ("base", "quote")
    };
    // Get the spot price for the given denoms
    let SpotPriceResponse { spot_price } = t
        .contract
        .query(&QueryMsg::SpotPrice {
            base_asset_denom: token_in_denom.to_string(),
            quote_asset_denom: token_out_denom.to_string(),
        })
        .unwrap();

    // Determine how much liquidity is available for token in at the current spot price
    // This only provides an estimate as the liquidity may be spread across multiple ticks
    // Hence why it can be difficult to fill the ENTIRE liquidity
    let liquidity_at_price_u256 = amount_to_value(
        order_direction.opposite(),
        Uint128::from(max),
        Decimal256::from(spot_price),
        RoundingDirection::Up,
    )
    .unwrap();

    // Select a random amount of the token in to swap
    let liquidity_at_price = Uint128::try_from(liquidity_at_price_u256).unwrap();
    let amount = rng.gen_range(0..=liquidity_at_price.u128());

    // Calculate the expected amount of token out
    let expected_out =
        t.contract
            .query::<CalcOutAmtGivenInResponse>(&QueryMsg::CalcOutAmountGivenIn {
                token_in: Coin::new(amount, token_in_denom.to_string()),
                token_out_denom: token_out_denom.to_string(),
                swap_fee: Decimal::zero(),
            });

    // If the provided error cannot be filled then we return a 0 amount
    if amount == 0 || expected_out.is_err() || expected_out.unwrap().token_out.amount == "0" {
        return 0;
    }

    // Generate the user account
    t.add_account(
        username,
        vec![
            Coin::new(amount, token_in_denom),
            Coin::new(1000000000000000u128, "uosmo"),
        ],
    );

    // Places the market order and ensures that funds are transferred correctly
    orders::place_market_success(cp, t, order_direction, amount, username).unwrap();
    amount
}
