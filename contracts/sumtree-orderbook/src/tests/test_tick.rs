use crate::state::TICK_STATE;
use crate::sumtree::node::NodeType;
use crate::sumtree::test::test_tree::insert_and_refetch;
use crate::tick::sync_tick;
use crate::types::{OrderDirection, TickState, TickValues};
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::Decimal256;

struct SyncTickTestCase {
    name: &'static str,
    initial_tick_values: TickValues,
    unrealized_cancels: Vec<NodeType>,
    new_etas_per_sync: Decimal256,
    num_syncs: u32,
    expected_cumulative_realized: Decimal256,
    expected_new_etas_post_sync: Decimal256,
}

#[test]
fn test_sync_tick() {
    let default_book_id = 0;
    let default_tick_id = 0;
    let default_order_direction = OrderDirection::Bid;

    let test_cases = vec![
        SyncTickTestCase {
            name: "Basic Sync",
            // Tick with:
            // * 5 units of available liquidity
            // * 5 units of unrealized cancellations
            initial_tick_values: build_tick_values(5, 5),

            // A single unrealized cancel of 5 units at ETAS 3
            unrealized_cancels: vec![NodeType::leaf_uint256(3u32, 5u32)],

            // Update tick such that new ETAS is within scope of cancelled node
            new_etas_per_sync: Decimal256::from_ratio(3u128, 1u128),
            num_syncs: 1,

            // We expect the 5 cancelled units to be realized post-sync
            expected_cumulative_realized: Decimal256::from_ratio(5u128, 1u128),

            // The ETAS should be updated to reflect the 5 units of realized cancellations
            expected_new_etas_post_sync: Decimal256::from_ratio(3u128 + 5u128, 1u128),
        },
        SyncTickTestCase {
            name: "No unrealized cancels",
            // Tick with:
            // * 10 units of available liquidity
            // * 0 units of unrealized cancellations
            initial_tick_values: build_tick_values(10, 0),

            // No unrealized cancels
            unrealized_cancels: vec![],

            // Update tick without any unrealized cancels
            new_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            num_syncs: 1,

            // No change in realized cancellations expected
            expected_cumulative_realized: Decimal256::zero(),

            // The ETAS should remain unchanged as there are no unrealized cancellations
            expected_new_etas_post_sync: Decimal256::from_ratio(5u128, 1u128),
        },
        SyncTickTestCase {
            name: "Multiple unrealized cancels",
            // Tick with:
            // * 20 units of available liquidity
            // * 15 units of unrealized cancellations
            initial_tick_values: build_tick_values(20, 15),

            // Multiple unrealized cancels
            unrealized_cancels: vec![
                NodeType::leaf_uint256(2u32, 3u32),
                NodeType::leaf_uint256(5u32, 12u32),
            ],

            // Update tick to a new ETAS that encompasses all unrealized cancels
            new_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            num_syncs: 1,

            // All unrealized cancels become realized
            expected_cumulative_realized: Decimal256::from_ratio(15u128, 1u128),

            // The ETAS should be updated to reflect all units of realized cancellations
            // and the original ETAS
            expected_new_etas_post_sync: Decimal256::from_ratio(5 + 15u128, 1u128),
        },
        SyncTickTestCase {
            name: "Multiple syncs",
            // Tick with:
            // * 200 units of available liquidity
            // * 150 units of unrealized cancellations
            initial_tick_values: build_tick_values(200, 150),

            // Multiple unrealized cancels (total 150 across many ETASs)
            unrealized_cancels: vec![
                NodeType::leaf_uint256(2u32, 30u32),
                NodeType::leaf_uint256(50u32, 12u32),
                NodeType::leaf_uint256(62u32, 10u32),
                NodeType::leaf_uint256(80u32, 28u32),
                NodeType::leaf_uint256(128u32, 70u32),
            ],

            // Increment tick ETAS by 30 per iteration for 3 iterations.
            // Iteration 1: only node 1 is included
            // Iteration 2: first two nodes included
            // Iteration 3: first four nodes are incldued
            new_etas_per_sync: Decimal256::from_ratio(30u128, 1u128),
            num_syncs: 4,

            // By end of iteration 3, the amounts of the first four nodes should be included.
            // This is equal to 30 + 12 + 10 + 28 = 80
            expected_cumulative_realized: Decimal256::from_ratio(80u128, 1u128),

            // The new ETAS includes all the incremented amounts (3 * 30 each) which represent fills,
            // plus the amount of realized cancellations
            expected_new_etas_post_sync: Decimal256::from_ratio((4u128 * 30u128) + 80u128, 1u128),
        },
    ];

    for test in test_cases {
        // --- Setup ---

        let mut deps = mock_dependencies();

        // Create and save default tick state
        let mut tick_state = TickState::default();
        tick_state.set_values(default_order_direction, test.initial_tick_values);
        TICK_STATE
            .save(
                deps.as_mut().storage,
                &(default_book_id, default_tick_id),
                &tick_state,
            )
            .unwrap();

        // Insert specified nodes into tree
        for node in test.unrealized_cancels.iter() {
            insert_and_refetch(
                deps.as_mut().storage,
                default_book_id,
                default_tick_id,
                default_order_direction,
                node,
            );
        }

        // --- System under test ---

        for _ in 0..test.num_syncs {
            // Increment to prepare for sync
            let mut tick_values = tick_state.get_values(default_order_direction);
            let updated_etas = tick_values.effective_total_amount_swapped + test.new_etas_per_sync;
            tick_values.effective_total_amount_swapped = updated_etas;
            tick_state.set_values(default_order_direction, tick_values);
            TICK_STATE
                .save(
                    deps.as_mut().storage,
                    &(default_book_id, default_tick_id),
                    &tick_state,
                )
                .unwrap();

            // Run sync
            sync_tick(
                deps.as_mut().storage,
                default_book_id,
                default_tick_id,
                updated_etas,
            )
            .unwrap();
        }

        // --- Assertions ---

        // Fetch updated tick state and assert
        let updated_tick_state = TICK_STATE
            .load(deps.as_ref().storage, &(default_book_id, default_tick_id))
            .unwrap();
        let updated_tick_values = updated_tick_state.get_values(default_order_direction);
        assert_eq!(
            test.expected_cumulative_realized, updated_tick_values.cumulative_realized_cancels,
            "Assertion failed on case: {}",
            test.name
        );
        assert_eq!(
            test.expected_new_etas_post_sync, updated_tick_values.effective_total_amount_swapped,
            "Assertion failed on case: {}",
            test.name
        );
    }
}

// build_tick_values builds a `TickValues` that simulates the given total liquidity and unrealized cancels.
// This helper allows us to test tick level functionality without leaning on higher level abstractions like
// place_limit and cancel_limit.
fn build_tick_values(total_liquidity: u128, unrealized_cancels: u128) -> TickValues {
    // We set initial cumulative tick value to zero
    let cumulative_realized_cancels = Decimal256::zero();

    // Cumulative value is the sum of all liquidity ever on the tick, so we add all inputs up for it.
    let cumulative_total_value =
        Decimal256::from_ratio(total_liquidity + unrealized_cancels, 1u128);

    // Total liquidity is the amount of liquidity remaining on the tick, so it excludes any cancelled orders
    let total_amount_of_liquidity = Decimal256::from_ratio(total_liquidity, 1u128);

    // We set default tick ETAS value to zero, and assume the tick is synced already.
    let effective_total_amount_swapped = Decimal256::zero();
    let last_tick_sync_etas = Decimal256::zero();

    TickValues {
        effective_total_amount_swapped,
        cumulative_total_value,
        total_amount_of_liquidity,
        cumulative_realized_cancels,
        last_tick_sync_etas,
    }
}
