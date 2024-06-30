use cosmwasm_std::{Decimal256, Uint128};
use osmosis_test_tube::{Module, OsmosisTestApp};

use super::utils::{assert, orders};
use crate::{
    constants::{MAX_TICK, MIN_TICK},
    setup,
    tests::e2e::modules::cosmwasm_pool::CosmwasmPool,
    types::OrderDirection,
};

struct BasicFulfilledOrderTestCase {
    name: &'static str,
    placed_amount: u128,
    filled_amount: u128,
    tick_id: i64,
    claim_bounty: Option<Decimal256>,
    order_direction: OrderDirection,
    claimer: &'static str,
}

#[test]
fn test_basic_order() {
    let cases = vec![
        BasicFulfilledOrderTestCase {
            name: "basic fulfilled ask",
            placed_amount: 10u128,
            filled_amount: 10u128,
            tick_id: 0,
            claim_bounty: None,
            order_direction: OrderDirection::Ask,
            claimer: "user1",
        },
        BasicFulfilledOrderTestCase {
            name: "basic fulfilled bid",
            placed_amount: 10u128,
            filled_amount: 10u128,
            tick_id: 0,
            claim_bounty: None,
            order_direction: OrderDirection::Bid,
            claimer: "user1",
        },
        BasicFulfilledOrderTestCase {
            name: "basic partially filled ask",
            placed_amount: 10u128,
            filled_amount: 5u128,
            tick_id: 0,
            claim_bounty: None,
            order_direction: OrderDirection::Ask,
            claimer: "user1",
        },
        BasicFulfilledOrderTestCase {
            name: "basic partially filled bid",
            placed_amount: 10u128,
            filled_amount: 5u128,
            tick_id: 0,
            claim_bounty: None,
            order_direction: OrderDirection::Bid,
            claimer: "user1",
        },
        BasicFulfilledOrderTestCase {
            name: "basic fulfilled ask with bounty",
            placed_amount: 100u128,
            filled_amount: 100u128,
            tick_id: 0,
            claim_bounty: Some(Decimal256::percent(1)),
            order_direction: OrderDirection::Ask,
            claimer: "user1",
        },
        BasicFulfilledOrderTestCase {
            name: "basic fulfilled ask with bounty with external claimant",
            placed_amount: 100u128,
            filled_amount: 100u128,
            tick_id: 0,
            claim_bounty: Some(Decimal256::percent(1)),
            order_direction: OrderDirection::Ask,
            claimer: "user2",
        },
    ];
    for case in cases {
        let app = OsmosisTestApp::new();
        let cp = CosmwasmPool::new(&app);
        let t = setup!(&app, "quote", "base", 0);
        let (expected_bid_tick, expected_ask_tick) = if case.order_direction == OrderDirection::Ask
        {
            (MIN_TICK, case.tick_id)
        } else {
            (case.tick_id, MAX_TICK)
        };

        // Place limit
        orders::place_limit(
            &t,
            case.tick_id,
            case.order_direction,
            case.placed_amount,
            case.claim_bounty,
            "user1",
        )
        .unwrap();
        match case.order_direction {
            OrderDirection::Ask => {
                assert::pool_liquidity(&t, case.placed_amount, 0u8, case.name);
                assert::pool_balance(&t, case.placed_amount, 0u8, case.name);
            }
            OrderDirection::Bid => {
                assert::pool_liquidity(&t, 0u8, case.placed_amount, case.name);
                assert::pool_balance(&t, 0u8, case.placed_amount, case.name);
            }
        }
        assert::spot_price(&t, expected_bid_tick, expected_ask_tick, case.name);

        // Fill limit order
        orders::place_market_and_assert_balance(
            &cp,
            &t,
            case.order_direction.opposite(),
            case.filled_amount,
            "user2",
        )
        .unwrap();
        match case.order_direction {
            OrderDirection::Ask => {
                assert::pool_liquidity(&t, case.placed_amount - case.filled_amount, 0u8, case.name);
                assert::pool_balance(
                    &t,
                    case.placed_amount - case.filled_amount,
                    case.filled_amount,
                    case.name,
                );
            }
            OrderDirection::Bid => {
                assert::pool_liquidity(&t, 0u8, case.placed_amount - case.filled_amount, case.name);
                assert::pool_balance(
                    &t,
                    case.filled_amount,
                    case.placed_amount - case.filled_amount,
                    case.name,
                );
            }
        }
        assert::spot_price(&t, expected_bid_tick, expected_ask_tick, case.name);

        // Claim limit
        orders::claim_and_assert_balance(&t, case.claimer, "user1", 0, 0).unwrap();
        match case.order_direction {
            OrderDirection::Ask => {
                assert::pool_liquidity(&t, case.placed_amount - case.filled_amount, 0u8, case.name);
                assert::pool_balance(&t, case.placed_amount - case.filled_amount, 0u8, case.name);
            }
            OrderDirection::Bid => {
                assert::pool_liquidity(&t, 0u8, case.placed_amount - case.filled_amount, case.name);
                assert::pool_balance(&t, 0u8, case.placed_amount - case.filled_amount, case.name);
            }
        }
        assert::spot_price(&t, expected_bid_tick, expected_ask_tick, case.name);
    }
}

#[test]
fn test_cancelled_orders() {
    let app = OsmosisTestApp::new();
    let cp = CosmwasmPool::new(&app);
    let t = setup!(&app, "quote", "base", 0);
    let amount_orders = 3;

    for i in 0..amount_orders {
        orders::place_limit(
            &t,
            0,
            OrderDirection::Ask,
            Uint128::from(100u128),
            None,
            "user1",
        )
        .unwrap();
        orders::cancel_limit_and_assert_balance(&t, "user1", 0, i).unwrap();
    }
    assert::pool_liquidity(&t, 0u8, 0u8, "cancelled orders");
    assert::pool_balance(&t, 0u8, 0u8, "cancelled orders");
    assert::tick_invariants(&t);

    orders::place_limit(
        &t,
        0,
        OrderDirection::Ask,
        Uint128::from(100u128),
        None,
        "user1",
    )
    .unwrap();
    assert::pool_liquidity(&t, 100u8, 0u8, "cancelled orders");
    assert::pool_balance(&t, 100u8, 0u8, "cancelled orders");
    assert::tick_invariants(&t);

    orders::place_market_and_assert_balance(
        &cp,
        &t,
        OrderDirection::Bid,
        Uint128::from(100u128),
        "user1",
    )
    .unwrap();
    assert::tick_invariants(&t);
    assert::pool_liquidity(&t, 0u8, 0u8, "cancelled orders");
    assert::pool_balance(&t, 0u8, 100u8, "cancelled orders");
    assert::tick_invariants(&t);

    orders::claim(&t, "user1", 0, amount_orders).unwrap();
    assert::tick_invariants(&t);
}
