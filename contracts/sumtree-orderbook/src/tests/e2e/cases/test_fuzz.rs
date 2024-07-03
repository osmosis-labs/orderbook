use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use cosmwasm_std::{Coin, Decimal};
use cosmwasm_std::{Decimal256, Uint128};
use osmosis_test_tube::{Account, Module, OsmosisTestApp};
use rand::seq::SliceRandom;
use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};

use super::utils::{assert, orders};
use crate::constants::MIN_TICK;
use crate::msg::{CalcOutAmtGivenInResponse, QueryMsg};
use crate::tests::e2e::modules::cosmwasm_pool::CosmwasmPool;
use crate::tick_math::{amount_to_value, tick_to_price, RoundingDirection};
use crate::{
    msg::{DenomsResponse, GetTotalPoolLiquidityResponse},
    setup,
    tests::e2e::test_env::TestEnv,
    types::OrderDirection,
};

// Tick Price = 100000
pub(crate) const LARGE_POSITIVE_TICK: i64 = 4500000;
// Tick Price = 0.00001
pub(crate) const LARGE_NEGATIVE_TICK: i64 = -4500000;
// pub(crate) const LARGE_NEGATIVE_TICK: i64 = -5000000;

// Loops over a provided action for the provided duration
// Tracks the number of operations and iterations
// Duration is in seconds
fn run_for_duration(
    duration: u64,
    count_per_iteration: u64,
    action: impl FnOnce(u64) + std::marker::Copy,
) {
    let duration = Duration::from_secs(duration);
    let now = SystemTime::now();
    let end = now.checked_add(duration).unwrap();

    let mut oper_count = 0;
    let mut iterations = 0;
    while SystemTime::now().le(&end) {
        action(count_per_iteration);

        oper_count += count_per_iteration;
        iterations += 1;
    }
    println!("operations: {}", oper_count);
    println!("iterations: {}", iterations);
}

#[test]
fn test_order_fuzz_linear_large_orders_small_range() {
    let oper_per_iteration = 1000;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (-10, 10), 0.2);
    });
}

#[test]
fn test_order_fuzz_linear_small_orders_large_range() {
    let oper_per_iteration = 2000;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK), 0.2);
    });
}

// This test takes a VERY long time to run
// #[test]
// fn test_order_fuzz_linear_very_large_orders_large_range() {
//     run_fuzz_linear(5000, (LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK), 0.2);
// }

#[test]
fn test_order_fuzz_linear_small_orders_small_range() {
    let oper_per_iteration = 100;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (-10, 0), 0.1);
    });
}

#[test]
fn test_order_fuzz_linear_large_cancelled_orders_small_range() {
    let oper_per_iteration = 2000;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (MIN_TICK, MIN_TICK + 20), 0.8);
    });
}

#[test]
fn test_order_fuzz_linear_small_cancelled_orders_large_range() {
    let oper_per_iteration = 100;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (LARGE_NEGATIVE_TICK, LARGE_POSITIVE_TICK), 0.8);
    });
}

#[test]
fn test_order_fuzz_linear_large_all_cancelled_orders_small_range() {
    let oper_per_iteration = 2000;
    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_linear(count, (-10, 10), 1.0);
    });
}

#[test]
fn test_order_fuzz_linear_single_tick() {
    let oper_per_iteration = 2000;
    run_for_duration(10, oper_per_iteration, |count| {
        run_fuzz_linear(count, (0, 0), 0.2);
    });
}

#[test]
fn test_order_fuzz_mixed() {
    let oper_per_iteration = 2000;
    run_for_duration(10, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (-20, 20));
    });
}

#[test]
fn test_order_fuzz_mixed_single_tick() {
    let oper_per_iteration = 2000;

    run_for_duration(10, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (0, 0));
    });
}

#[test]
fn test_order_fuzz_mixed_large_negative_tick_range() {
    let oper_per_iteration = 2000;

    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (LARGE_NEGATIVE_TICK, LARGE_NEGATIVE_TICK + 10));
    });
}

#[test]
fn test_order_fuzz_mixed_large_positive_tick_range() {
    let oper_per_iteration = 2000;

    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (LARGE_POSITIVE_TICK - 10, LARGE_POSITIVE_TICK));
    });
}

#[test]
fn test_order_fuzz_mixed_min_tick() {
    let oper_per_iteration = 2000;

    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (MIN_TICK, MIN_TICK + 10));
    });
}

#[test]
fn test_order_fuzz_large_tick_range() {
    let oper_per_iteration = 2000;

    run_for_duration(60, oper_per_iteration, |count| {
        run_fuzz_mixed(count, (MIN_TICK, LARGE_POSITIVE_TICK));
    });
}

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
    let mut t = setup!(&app, "quote", "base", 1);

    let mut orders = vec![];

    // -- System Under Test --

    // -- Step 1: Place Limits --

    // Places the set amount of orders within the provided tick range
    // Orders will be cancelled with a chance equal to the provided cancel_probability
    // Tick state is verified after every order is placed (and cancelled)
    for i in 0..amount_limit_orders {
        let username = format!("user{}", i);
        let chosen_tick = place_random_limit(&mut t, &mut rng, &username, tick_range);
        let is_cancelled = rng.gen_bool(cancel_probability);

        if is_cancelled {
            orders::cancel_limit_and_assert_balance(&t, &username, chosen_tick, i).unwrap();
        } else {
            orders.push((username, chosen_tick, i));
        }

        assert::tick_invariants(&t);
    }

    // -- Step 2: Place Market Orders --

    // For both directions fill the entire amount of liquidity available using market orders
    // For certain cases it is not possible to fill the entire liquidity so a remainder of 1 may occur
    for order_direction in [OrderDirection::Bid, OrderDirection::Ask] {
        // Determine the amount of liquidity for the given direction
        let mut liquidity = t.contract.get_directional_liquidity(order_direction);

        // A counter to track the number of zero amount returns
        let mut zero_amount_returns = 0;
        // A counter to track the current user ID
        let mut user_id = 0;

        let mut previous_expected_out =
            assert::decrementing_market_order_output(&t, u128::MAX, 10000000u128, order_direction);

        // While there is some fillable liquidity we want to place randomised market orders
        while liquidity > 1u128 {
            let username = format!("user{}{}", order_direction, user_id);
            let placed_amount =
                place_random_market(&cp, &mut t, &mut rng, &username, order_direction);

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
            assert::has_liquidity(&t);

            previous_expected_out = assert::decrementing_market_order_output(
                &t,
                previous_expected_out.u128(),
                100u128,
                order_direction,
            );
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

    // -- Step 3: Claim Orders --

    // Shuffle the order of recorded orders (as liquidity is fully filled (except the possibility of a 1 remainder))
    // every order should be claimable and the order should not matter
    orders.shuffle(&mut rng);

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
        let order = t
            .contract
            .get_order(t.accounts[username].address(), *tick_id, *order_id)
            .unwrap();
        let sender = if order.claim_bounty.is_some() {
            "claimant"
        } else {
            username
        };

        orders::claim_and_assert_balance(&t, sender, username, order.tick_id, order.order_id)
            .unwrap();

        // For the situation that the order has the 1 remainder we record this for assertions
        let maybe_order = t.contract.get_order(
            t.accounts[username].address(),
            order.tick_id,
            order.order_id,
        );
        if let Some(order) = maybe_order {
            orders::cancel_limit_and_assert_balance(&t, username, order.tick_id, order.order_id)
                .unwrap();
        }
    }

    // -- Post Test Assertions --
    assert::clean_ticks(&t);
    assert::no_remaining_orders(&t);
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum MixedFuzzOperation {
    PlaceLimit,
    PlaceMarket,
    CancelLimit,
    Claim,
}

impl MixedFuzzOperation {
    /// Chooses a random fuzz operation
    fn random(rng: &mut StdRng) -> Self {
        let index: u32 = rng.gen_range(0..=100);

        if index < 25 {
            Self::PlaceLimit
        } else if index < 50 {
            Self::PlaceMarket
        } else if index < 75 {
            Self::CancelLimit
        } else {
            Self::Claim
        }
    }

    /// Attempts to run the chosen fuzz operation
    ///
    /// Returns true if the operation was successful, false otherwise
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        t: &mut TestEnv,
        cp: &CosmwasmPool<OsmosisTestApp>,
        rng: &mut StdRng,
        iteration: u64,
        orders: &mut HashMap<u64, (String, i64)>,
        order_count: &mut u64,
        tick_bounds: (i64, i64),
    ) -> Result<bool, &'static str> {
        let username = format!("user{}", iteration);
        match self {
            MixedFuzzOperation::PlaceLimit => {
                // Place the limit order
                let tick_id = place_random_limit(t, rng, &username, tick_bounds);
                // Record the order for claims/cancels
                orders.insert(*order_count, (username, tick_id));
                *order_count += 1;
                Ok(true)
            }
            MixedFuzzOperation::PlaceMarket => {
                // Determine the market direction
                let maybe_market_direction = get_random_market_direction(t, rng);
                // May error if the orderbook has 0 liquidity for both directions
                if maybe_market_direction.is_err() {
                    return Ok(false);
                }
                let market_direction = maybe_market_direction.unwrap();
                // Determine the maximum amount of the opposite direction that can be bought
                let max_amount = t
                    .contract
                    .get_directional_liquidity(market_direction.opposite());
                // If nothing can be bought then we skip this operation
                if max_amount == 0 {
                    return Ok(false);
                }

                // Place the order
                let amount = place_random_market(cp, t, rng, &username, market_direction);

                Ok(amount != 0)
            }
            MixedFuzzOperation::CancelLimit => {
                // If there are no active orders skip the operation
                if orders.is_empty() {
                    return Ok(false);
                }

                // Determine the order to be cancelled
                let order_ids = orders.keys().collect::<Vec<&u64>>();
                let order_idx = rng.gen_range(0..order_ids.len());
                let order_id = *order_ids[order_idx];
                let (username, tick_id) = orders.get(&order_id).unwrap().clone();

                // We cannot cancel an order if it is partially filled
                let order = t
                    .contract
                    .get_order(t.accounts[&username].address(), tick_id, order_id)
                    .unwrap();
                let amount_claimable = t.contract.get_order_claimable_amount(order.clone());

                // Determine if the order can be cancelled
                if amount_claimable > 0 {
                    return Ok(false);
                }

                // Cancel the order
                orders::cancel_limit_and_assert_balance(t, &username, tick_id, order_id).unwrap();
                // Remove the order once we know it is cancellable
                orders.remove(&order_id).unwrap();
                Ok(true)
            }
            MixedFuzzOperation::Claim => {
                // If there are no active orders skip the operation
                if orders.is_empty() {
                    return Ok(false);
                }

                // Determine the order to be claimed
                let order_ids = orders.keys().collect::<Vec<&u64>>();
                let order_idx = rng.gen_range(0..order_ids.len());
                let order_id = *order_ids[order_idx];
                let (username, tick_id) = orders.get(&order_id).unwrap().clone();

                // We cannot claim an order if it has nothing to be claimed
                let order = t
                    .contract
                    .get_order(t.accounts[&username].address(), tick_id, order_id)
                    .unwrap();
                let amount_claimable = t.contract.get_order_claimable_amount(order.clone());

                // Determine if the order can be claimed
                if amount_claimable == 0 {
                    return Ok(false);
                }

                let price = tick_to_price(order.tick_id).unwrap();
                let expected_received_u256 = amount_to_value(
                    order.order_direction,
                    Uint128::from(amount_claimable),
                    price,
                    RoundingDirection::Down,
                )
                .unwrap();

                if expected_received_u256.is_zero() {
                    return Ok(false);
                }

                let claimant = if order.claim_bounty.is_some() {
                    t.add_account("claimant", vec![Coin::new(1000000000000u128, "uosmo")]);
                    "claimant"
                } else {
                    username.as_str()
                };

                // Claim the order
                match orders::claim_and_assert_balance(t, claimant, &username, tick_id, order_id) {
                    Ok(_) => {
                        let order = t.contract.get_order(
                            t.accounts[&username].address(),
                            tick_id,
                            order_id,
                        );
                        if order.is_none() {
                            // Remove the order once we know its claimable
                            orders.remove(&order_id).unwrap();
                        }
                        Ok(true)
                    }
                    Err(e) => {
                        panic!("{e}")
                    }
                }
            }
        }
    }
}

/// Runs a fuzz test that randomly chooses between 4 operations:
/// 1. Place a Limit
/// 2. Place a Market
/// 3. Cancel a Limit
/// 4. Claim a Limit
///
/// These operations are chosen at random and if they are an invalid operation they are skipped and a new operation is chosen.
/// Orders are placing in a tick range determined by the current tick bounds with the intent that ticks spread over time randomly to the desired tick bounds.
/// Expected errors are handled by skipping the operation and randomly choosing a new operation. Any errors returned are expected to be because of an issue in the orderbook.
fn run_fuzz_mixed(amount_of_orders: u64, tick_bounds: (i64, i64)) {
    // -- Test Setup --
    let seed: u64 = 123456789;
    let mut rng = StdRng::seed_from_u64(seed);

    let app = OsmosisTestApp::new();
    let cp = CosmwasmPool::new(&app);
    let mut t = setup!(&app, "quote", "base", 1);

    // A record of the orders placed to allow for simpler management of cancellations and claims
    let mut orders: HashMap<u64, (String, i64)> = HashMap::new();
    // A count of the orders placed to track the current order ID
    let mut order_count = 0;

    // Record how many times each operation is chosen for assertion post test
    let mut oper_count: HashMap<MixedFuzzOperation, u64> = HashMap::new();
    oper_count.insert(MixedFuzzOperation::PlaceLimit, 0);
    oper_count.insert(MixedFuzzOperation::PlaceMarket, 0);
    oper_count.insert(MixedFuzzOperation::CancelLimit, 0);
    oper_count.insert(MixedFuzzOperation::Claim, 0);

    // -- System Under Test --
    for i in 0..amount_of_orders {
        // Chooses an operation at random
        let mut operation = MixedFuzzOperation::random(&mut rng);

        // We add an escape clause in the case that the test ever gets caught in an infinite loop
        let mut repeated_failures = 0;

        // Repeat randomising operations until a successful one is chosen
        while !operation
            .run(
                &mut t,
                &cp,
                &mut rng,
                i,
                &mut orders,
                &mut order_count,
                tick_bounds,
            )
            .unwrap()
        {
            operation = MixedFuzzOperation::random(&mut rng);
            repeated_failures += 1;
            if repeated_failures > 100 {
                panic!("Caught in loop");
            }
        }
        oper_count.entry(operation).and_modify(|c| *c += 1);

        // -- Post operation assertions --
        assert::tick_invariants(&t);
        assert::has_liquidity(&t);
    }

    for (order_id, (username, tick_id)) in orders.clone().iter() {
        let _ = orders::claim_and_assert_balance(&t, username, username, *tick_id, *order_id);

        // Order may be cleared by fully claiming, in which case we want to continue to the next order
        if t.contract
            .get_order(t.accounts[username.as_str()].address(), *tick_id, *order_id)
            .is_none()
        {
            continue;
        }

        // If cancelling is a success we can continue to the next order
        if orders::cancel_limit_and_assert_balance(&t, username, *tick_id, *order_id).is_ok() {
            continue;
        }

        // If an order cannot be claimed or cancelled something has gone wrong
        let order = t.contract.get_order(username.clone(), *tick_id, *order_id);
        assert!(
            order.is_none(),
            "order was not cleaned from state: {order:?}"
        );
    }

    // -- Post test assertions --

    // Assert every operation ran at least once successfully
    assert!(
        oper_count.values().all(|c| *c > 0),
        "not all operations were used"
    );
    assert::no_remaining_orders(&t);
    assert::clean_ticks(&t);
}

/// Places a random limit order in the provided tick range using the provided username
fn place_random_limit(
    t: &mut TestEnv,
    rng: &mut StdRng,
    username: &str,
    tick_range: (i64, i64),
) -> i64 {
    // Quantities are in magnitudes of u32
    let quantity = Uint128::from(rng.gen::<u64>());
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

    // Add the user account with the appropriate amount of the denom
    t.add_account(
        username,
        vec![
            Coin::new(quantity.u128(), expected_denom),
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
        quantity,
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
) -> u128 {
    // Get the appropriate denom for the chosen direction
    let (token_in_denom, token_out_denom) = if order_direction == OrderDirection::Bid {
        ("quote", "base")
    } else {
        ("base", "quote")
    };

    // Select a random amount of the token in to swap
    // let liquidity_at_price = Uint128::try_from(liquidity_at_price_u256).unwrap();
    let max_amount = t.contract.get_max_market_amount(order_direction);
    let amount = rng.gen_range(0..=max_amount);

    if amount == 0 {
        return 0;
    }

    // Calculate the expected amount of token out
    let expected_out =
        t.contract
            .query::<CalcOutAmtGivenInResponse>(&QueryMsg::CalcOutAmountGivenIn {
                token_in: Coin::new(amount, token_in_denom.to_string()),
                token_out_denom: token_out_denom.to_string(),
                swap_fee: Decimal::zero(),
            });

    if let Ok(expected_out) = expected_out {
        if expected_out.token_out.amount == "0" {
            return 0;
        }
    } else if expected_out.is_err() {
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
    orders::place_market_and_assert_balance(cp, t, order_direction, amount, username).unwrap();

    // We return the amount placed for recording
    amount
}

/// Determines a random market direction based on the available liquidity for bids and asks.
/// Errors if both directions have no liquidity
///
/// Chooses a direction if that direction is the only one with liquidity.
fn get_random_market_direction<'a>(
    t: &TestEnv,
    rng: &mut StdRng,
) -> Result<OrderDirection, &'a str> {
    let bid_liquidity = t.contract.get_directional_liquidity(OrderDirection::Bid);
    let ask_liquidity = t.contract.get_directional_liquidity(OrderDirection::Ask);
    if bid_liquidity == 0 && ask_liquidity == 0 {
        return Err("No liquidity available to place market order");
    }

    let bid_probability = if bid_liquidity != 0 && ask_liquidity == 0 {
        1.0
    } else if bid_liquidity != 0 {
        0.5
    } else {
        0.0
    };

    if rng.gen_bool(bid_probability) {
        Ok(OrderDirection::Bid)
    } else {
        Ok(OrderDirection::Ask)
    }
}
