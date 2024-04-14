use crate::state::TICK_STATE;
use crate::sumtree::node::NodeType;
use crate::sumtree::test::test_tree::insert_and_refetch;
use crate::tick::sync_tick;
use crate::types::{OrderDirection, TickState, TickValues};
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{Decimal256, Storage};

struct SyncTickTestCase {
    name: &'static str,
    initial_tick_bid_values: TickValues,
    initial_tick_ask_values: TickValues,

    unrealized_cancels_bid: Vec<NodeType>,
    unrealized_cancels_ask: Vec<NodeType>,

    new_bid_etas_per_sync: Decimal256,
    new_ask_etas_per_sync: Decimal256,

    num_syncs: u32,

    expected_cumulative_realized_bid: Decimal256,
    expected_cumulative_realized_ask: Decimal256,

    expected_new_bid_etas_post_sync: Decimal256,
    expected_new_ask_etas_post_sync: Decimal256,
}

#[test]
fn test_sync_tick() {
    let default_book_id = 0;
    let default_tick_id = 0;

    let test_cases = vec![
        // --- Bid-only cases ---
        SyncTickTestCase {
            name: "Basic Sync",
            // Tick with:
            // * 5 units of available liquidity
            // * 5 units of unrealized cancellations
            initial_tick_bid_values: build_tick_values(5, 5),
            initial_tick_ask_values: build_tick_values(0, 0),

            // A single unrealized cancel of 5 units at ETAS 3
            unrealized_cancels_bid: vec![NodeType::leaf_uint256(3u32, 5u32)],
            unrealized_cancels_ask: vec![],

            // Update tick such that new ETAS is within scope of cancelled node
            new_bid_etas_per_sync: Decimal256::from_ratio(3u128, 1u128),
            new_ask_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // We expect the 5 cancelled units to be realized post-sync
            expected_cumulative_realized_bid: Decimal256::from_ratio(5u128, 1u128),
            expected_cumulative_realized_ask: Decimal256::zero(),

            // The ETAS should be updated to reflect the 5 units of realized cancellations
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(3u128 + 5u128, 1u128),
            expected_new_ask_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "No unrealized cancels",
            // Tick with:
            // * 10 units of available liquidity
            // * 0 units of unrealized cancellations
            initial_tick_bid_values: build_tick_values(10, 0),
            initial_tick_ask_values: build_tick_values(0, 0),

            // No unrealized cancels
            unrealized_cancels_bid: vec![],
            unrealized_cancels_ask: vec![],

            // Update tick without any unrealized cancels
            new_bid_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            new_ask_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // No change in realized cancellations expected
            expected_cumulative_realized_bid: Decimal256::zero(),
            expected_cumulative_realized_ask: Decimal256::zero(),

            // The ETAS should remain unchanged as there are no unrealized cancellations
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(5u128, 1u128),
            expected_new_ask_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "Multiple unrealized cancels",
            // Tick with:
            // * 20 units of available liquidity
            // * 15 units of unrealized cancellations
            initial_tick_bid_values: build_tick_values(20, 15),
            initial_tick_ask_values: build_tick_values(0, 0),

            // Multiple unrealized cancels
            unrealized_cancels_bid: vec![
                NodeType::leaf_uint256(2u32, 3u32),
                NodeType::leaf_uint256(5u32, 12u32),
            ],
            unrealized_cancels_ask: vec![],

            // Update tick to a new ETAS that encompasses all unrealized cancels
            new_bid_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            new_ask_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // All unrealized cancels become realized
            expected_cumulative_realized_bid: Decimal256::from_ratio(15u128, 1u128),
            expected_cumulative_realized_ask: Decimal256::zero(),

            // The ETAS should be updated to reflect all units of realized cancellations
            // and the original ETAS
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(5 + 15u128, 1u128),
            expected_new_ask_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "Multiple syncs",
            // Tick with:
            // * 200 units of available liquidity
            // * 150 units of unrealized cancellations
            initial_tick_bid_values: build_tick_values(200, 150),
            initial_tick_ask_values: build_tick_values(0, 0),

            // Multiple unrealized cancels (total 150 across many ETASs)
            unrealized_cancels_bid: vec![
                NodeType::leaf_uint256(2u32, 30u32),
                NodeType::leaf_uint256(50u32, 12u32),
                NodeType::leaf_uint256(62u32, 10u32),
                NodeType::leaf_uint256(80u32, 28u32),
                NodeType::leaf_uint256(128u32, 70u32),
            ],
            unrealized_cancels_ask: vec![],

            // Increment tick ETAS by 30 per iteration for 3 iterations.
            // Iteration 1: only node 1 is included
            // Iteration 2: first two nodes included
            // Iteration 3: first four nodes are incldued
            new_bid_etas_per_sync: Decimal256::from_ratio(30u128, 1u128),
            new_ask_etas_per_sync: Decimal256::zero(),
            num_syncs: 4,

            // By end of iteration 3, the amounts of the first four nodes should be included.
            // This is equal to 30 + 12 + 10 + 28 = 80
            expected_cumulative_realized_bid: Decimal256::from_ratio(80u128, 1u128),
            expected_cumulative_realized_ask: Decimal256::zero(),

            // The new ETAS includes all the incremented amounts (3 * 30 each) which represent fills,
            // plus the amount of realized cancellations
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(
                (4u128 * 30u128) + 80u128,
                1u128,
            ),
            expected_new_ask_etas_post_sync: Decimal256::zero(),
        },
        // --- Ask-only cases ---
        SyncTickTestCase {
            name: "Basic Sync - Ask",
            // Tick with:
            // * 5 units of available liquidity
            // * 5 units of unrealized cancellations
            initial_tick_ask_values: build_tick_values(5, 5),
            initial_tick_bid_values: build_tick_values(0, 0),

            // A single unrealized cancel of 5 units at ETAS 3
            unrealized_cancels_ask: vec![NodeType::leaf_uint256(3u32, 5u32)],
            unrealized_cancels_bid: vec![],

            // Update tick such that new ETAS is within scope of cancelled node
            new_ask_etas_per_sync: Decimal256::from_ratio(3u128, 1u128),
            new_bid_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // We expect the 5 cancelled units to be realized post-sync
            expected_cumulative_realized_ask: Decimal256::from_ratio(5u128, 1u128),
            expected_cumulative_realized_bid: Decimal256::zero(),

            // The ETAS should be updated to reflect the 5 units of realized cancellations
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(3u128 + 5u128, 1u128),
            expected_new_bid_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "No unrealized cancels - Ask",
            // Tick with:
            // * 10 units of available liquidity
            // * 0 units of unrealized cancellations
            initial_tick_ask_values: build_tick_values(10, 0),
            initial_tick_bid_values: build_tick_values(0, 0),

            // No unrealized cancels
            unrealized_cancels_ask: vec![],
            unrealized_cancels_bid: vec![],

            // Update tick without any unrealized cancels
            new_ask_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            new_bid_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // No change in realized cancellations expected
            expected_cumulative_realized_ask: Decimal256::zero(),
            expected_cumulative_realized_bid: Decimal256::zero(),

            // The ETAS should remain unchanged as there are no unrealized cancellations
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(5u128, 1u128),
            expected_new_bid_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "Multiple unrealized cancels - Ask",
            // Tick with:
            // * 20 units of available liquidity
            // * 15 units of unrealized cancellations
            initial_tick_ask_values: build_tick_values(20, 15),
            initial_tick_bid_values: build_tick_values(0, 0),

            // Multiple unrealized cancels
            unrealized_cancels_ask: vec![
                NodeType::leaf_uint256(2u32, 3u32),
                NodeType::leaf_uint256(5u32, 12u32),
            ],
            unrealized_cancels_bid: vec![],

            // Update tick to a new ETAS that encompasses all unrealized cancels
            new_ask_etas_per_sync: Decimal256::from_ratio(5u128, 1u128),
            new_bid_etas_per_sync: Decimal256::zero(),
            num_syncs: 1,

            // All unrealized cancels become realized
            expected_cumulative_realized_ask: Decimal256::from_ratio(15u128, 1u128),
            expected_cumulative_realized_bid: Decimal256::zero(),

            // The ETAS should be updated to reflect all units of realized cancellations
            // and the original ETAS
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(5 + 15u128, 1u128),
            expected_new_bid_etas_post_sync: Decimal256::zero(),
        },
        SyncTickTestCase {
            name: "Multiple syncs - Ask",
            // Tick with:
            // * 200 units of available liquidity
            // * 150 units of unrealized cancellations
            initial_tick_ask_values: build_tick_values(200, 150),
            initial_tick_bid_values: build_tick_values(0, 0),

            // Multiple unrealized cancels (total 150 across many ETASs)
            unrealized_cancels_ask: vec![
                NodeType::leaf_uint256(2u32, 30u32),
                NodeType::leaf_uint256(50u32, 12u32),
                NodeType::leaf_uint256(62u32, 10u32),
                NodeType::leaf_uint256(80u32, 28u32),
                NodeType::leaf_uint256(128u32, 70u32),
            ],
            unrealized_cancels_bid: vec![],

            // Increment tick ETAS by 30 per iteration for 3 iterations.
            // Iteration 1: only node 1 is included
            // Iteration 2: first two nodes included
            // Iteration 3: first four nodes are included
            new_ask_etas_per_sync: Decimal256::from_ratio(30u128, 1u128),
            new_bid_etas_per_sync: Decimal256::zero(),
            num_syncs: 4,

            // By end of iteration 3, the amounts of the first four nodes should be included.
            // This is equal to 30 + 12 + 10 + 28 = 80
            expected_cumulative_realized_ask: Decimal256::from_ratio(80u128, 1u128),
            expected_cumulative_realized_bid: Decimal256::zero(),

            // The new ETAS includes all the incremented amounts (3 * 30 each) which represent fills,
            // plus the amount of realized cancellations
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(
                (4u128 * 30u128) + 80u128,
                1u128,
            ),
            expected_new_bid_etas_post_sync: Decimal256::zero(),
        },
        // --- Bid and Ask cases ---
        SyncTickTestCase {
            name: "Bid and Ask - Basic Sync",
            // Tick with:
            // * 5 units of available liquidity for both bid and ask
            // * 5 units of unrealized cancellations for both bid and ask
            initial_tick_bid_values: build_tick_values(5, 5),
            initial_tick_ask_values: build_tick_values(5, 5),

            // A single unrealized cancel of 5 units at ETAS 3 for both bid and ask
            unrealized_cancels_bid: vec![NodeType::leaf_uint256(3u32, 5u32)],
            unrealized_cancels_ask: vec![NodeType::leaf_uint256(3u32, 5u32)],

            // Update tick such that new ETAS is within scope of cancelled node for both bid and ask
            new_bid_etas_per_sync: Decimal256::from_ratio(3u128, 1u128),
            new_ask_etas_per_sync: Decimal256::from_ratio(3u128, 1u128),
            num_syncs: 1,

            // We expect the 5 cancelled units to be realized post-sync for both bid and ask
            expected_cumulative_realized_bid: Decimal256::from_ratio(5u128, 1u128),
            expected_cumulative_realized_ask: Decimal256::from_ratio(5u128, 1u128),

            // The ETAS should be updated to reflect the 5 units of realized cancellations for both bid and ask
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(3u128 + 5u128, 1u128),
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(3u128 + 5u128, 1u128),
        },
        SyncTickTestCase {
            name: "Bid and Ask - Multiple Syncs",
            // Tick with:
            // * 100 units of available liquidity for bid, 200 for ask
            // * 50 units of unrealized cancellations for bid, 150 for ask
            initial_tick_bid_values: build_tick_values(100, 50),
            initial_tick_ask_values: build_tick_values(200, 150),

            // Multiple unrealized cancels for both bid and ask
            unrealized_cancels_bid: vec![
                NodeType::leaf_uint256(35u32, 25u32),
                NodeType::leaf_uint256(10u32, 25u32),
            ],
            unrealized_cancels_ask: vec![
                NodeType::leaf_uint256(10u32, 50u32),
                NodeType::leaf_uint256(60u32, 100u32),
            ],

            // Increment tick ETAS by 10 for bid and 40 for ask per iteration for 2 iterations
            new_bid_etas_per_sync: Decimal256::from_ratio(10u128, 1u128),
            new_ask_etas_per_sync: Decimal256::from_ratio(40u128, 1u128),
            num_syncs: 2,

            // By end of iteration 2, only the first bid node and both ask nodes should be included.
            expected_cumulative_realized_bid: Decimal256::from_ratio(25u128, 1u128),
            expected_cumulative_realized_ask: Decimal256::from_ratio(150u128, 1u128),

            // The new ETAS includes all the incremented amounts which represent fills,
            // plus the amount of realized cancellations for both bid and ask
            expected_new_bid_etas_post_sync: Decimal256::from_ratio(
                (2u128 * 10u128) + 25u128,
                1u128,
            ),
            expected_new_ask_etas_post_sync: Decimal256::from_ratio(
                (2u128 * 40u128) + 150u128,
                1u128,
            ),
        },
    ];

    for test in test_cases {
        // --- Setup ---

        let mut deps = mock_dependencies();

        // Create and save default tick state
        let mut tick_state = TickState::default();
        tick_state.set_values(OrderDirection::Bid, test.initial_tick_bid_values);
        tick_state.set_values(OrderDirection::Ask, test.initial_tick_ask_values);
        TICK_STATE
            .save(
                deps.as_mut().storage,
                &(default_book_id, default_tick_id),
                &tick_state,
            )
            .unwrap();

        // Insert specified nodes into tree
        for (unrealized_cancels, direction) in [
            (&test.unrealized_cancels_bid, OrderDirection::Bid),
            (&test.unrealized_cancels_ask, OrderDirection::Ask),
        ] {
            for node in unrealized_cancels.iter() {
                insert_and_refetch(
                    deps.as_mut().storage,
                    default_book_id,
                    default_tick_id,
                    direction,
                    node,
                );
            }
        }

        // --- System under test ---

        for loop_number in 0..test.num_syncs {
            // Increment tick ETAS for each step
            let (updated_bid_etas, updated_ask_etas) = increment_tick_etas(
                deps.as_mut().storage,
                default_book_id,
                default_tick_id,
                &mut tick_state,
                test.new_bid_etas_per_sync,
                test.new_ask_etas_per_sync,
            );

            // Run sync
            sync_tick(
                deps.as_mut().storage,
                default_book_id,
                default_tick_id,
                updated_bid_etas,
                updated_ask_etas,
            )
            .unwrap();
        }

        // --- Assertions ---

        // Fetch updated tick state and assert
        let updated_tick_state = TICK_STATE
            .load(deps.as_ref().storage, &(default_book_id, default_tick_id))
            .unwrap();
        let (updated_bid_tick_values, updated_ask_tick_values) = (
            updated_tick_state.get_values(OrderDirection::Bid),
            updated_tick_state.get_values(OrderDirection::Ask),
        );

        // Assert post-sync tick cumulative realized cancels
        assert_eq!(
            test.expected_cumulative_realized_bid,
            updated_bid_tick_values.cumulative_realized_cancels,
            "Assertion failed on case: {}",
            test.name
        );
        assert_eq!(
            test.expected_cumulative_realized_ask,
            updated_ask_tick_values.cumulative_realized_cancels,
            "Assertion failed on case: {}",
            test.name
        );

        // Assert post-sync tick ETAS values
        assert_eq!(
            test.expected_new_bid_etas_post_sync,
            updated_bid_tick_values.effective_total_amount_swapped,
            "Assertion failed on case: {}",
            test.name
        );
        assert_eq!(
            test.expected_new_ask_etas_post_sync,
            updated_ask_tick_values.effective_total_amount_swapped,
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

// increment_tick_etas increments the ETAS of a tick by the given amounts for both bid and ask orders.
fn increment_tick_etas(
    storage: &mut dyn Storage,
    book_id: u64,
    tick_id: i64,
    tick_state: &mut TickState,
    new_bid_etas_per_sync: Decimal256,
    new_ask_etas_per_sync: Decimal256,
) -> (Decimal256, Decimal256) {
    let (mut bid_tick_values, mut ask_tick_values) = (
        tick_state.get_values(OrderDirection::Bid),
        tick_state.get_values(OrderDirection::Ask),
    );

    let (updated_bid_etas, updated_ask_etas) = (
        bid_tick_values.effective_total_amount_swapped + new_bid_etas_per_sync,
        ask_tick_values.effective_total_amount_swapped + new_ask_etas_per_sync,
    );
    bid_tick_values.effective_total_amount_swapped = updated_bid_etas;
    ask_tick_values.effective_total_amount_swapped = updated_ask_etas;
    tick_state.set_values(OrderDirection::Bid, bid_tick_values);
    tick_state.set_values(OrderDirection::Ask, ask_tick_values);

    TICK_STATE
        .save(storage, &(book_id, tick_id), tick_state)
        .unwrap();

    (updated_bid_etas, updated_ask_etas)
}
