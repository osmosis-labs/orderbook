use crate::state::TICK_STATE;
use crate::sumtree::node::{generate_node_id, NodeType, TreeNode, NODES};
use crate::sumtree::tree::{get_or_init_root_node, TREE};
use crate::tick::sync_tick;
use crate::types::{OrderDirection, TickState};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr, Decimal256, Order, Uint128};

struct SyncTickTestCase {
    name: &'static str,
    initial_etas: Decimal256,
    initial_liquidity: Decimal256,
    initial_cumulative_realized_cancels: Decimal256,
    nodes_to_insert: Vec<(Decimal256, Decimal256)>,
    new_etas: Decimal256,
    expected_cumulative_realized: Decimal256,
    expected_new_etas: Decimal256,
}

#[test]
fn test_sync_tick() {
    let default_book_id = 0;
    let default_tick_id = 0;
    let default_order_direction = OrderDirection::Bid;

    let test_cases = vec![
        SyncTickTestCase {
            name: "Basic Sync",
            initial_etas: Decimal256::zero(),
            initial_liquidity: Decimal256::from_ratio(10u128, 1u128),
            initial_cumulative_realized_cancels: Decimal256::zero(),
            nodes_to_insert: vec![
                (
                    Decimal256::from_ratio(5u128, 1u128),
                    Decimal256::from_ratio(1u128, 1u128),
                ),
                (
                    Decimal256::from_ratio(3u128, 1u128),
                    Decimal256::from_ratio(2u128, 1u128),
                ),
            ],
            new_etas: Decimal256::from_ratio(3u128, 1u128),
            expected_cumulative_realized: Decimal256::from_ratio(2u128, 1u128),
            expected_new_etas: Decimal256::from_ratio(12u128, 1u128), // initial + (new - old realized)
        },
        // Add more test cases as needed
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();

        // --- Setup ---

        // Create and save default tick state
        let mut tick_state = TickState::default();
        let mut tick_values = tick_state.get_values(default_order_direction);
        tick_values.effective_total_amount_swapped = test.initial_etas;
        tick_values.total_amount_of_liquidity = test.initial_liquidity;
        tick_values.cumulative_realized_cancels = test.initial_cumulative_realized_cancels;
        tick_state.set_values(default_order_direction, tick_values);
        TICK_STATE
            .save(
                deps.as_mut().storage,
                &(default_book_id, default_tick_id),
                &tick_state,
            )
            .unwrap();

        // Initialize sumtree
        let mut tree = get_or_init_root_node(
            deps.as_mut().storage,
            default_book_id,
            default_tick_id,
            default_order_direction,
        )
        .unwrap();

        // Insert specified nodes into tree
        for (value, etas) in test.nodes_to_insert.iter() {
            let node_id =
                generate_node_id(deps.as_mut().storage, default_book_id, default_tick_id).unwrap();
            let mut node = TreeNode::new(
                default_book_id,
                default_tick_id,
                default_order_direction,
                node_id,
                NodeType::leaf(*etas, *value),
            );
            tree.insert(deps.as_mut().storage, &mut node).unwrap();
        }

        // --- System under test ---

        sync_tick(
            deps.as_mut().storage,
            default_book_id,
            default_tick_id,
            test.new_etas,
        )
        .unwrap();

        // Fetch updated tick state and assert
        let updated_tick_state = TICK_STATE
            .load(deps.as_ref().storage, &(default_book_id, default_tick_id))
            .unwrap();
        let updated_tick_values = updated_tick_state.get_values(default_order_direction);
        assert_eq!(
            updated_tick_values.cumulative_realized_cancels, test.expected_cumulative_realized,
            "{} - cumulative realized cancels",
            test.name
        );
        assert_eq!(
            updated_tick_values.effective_total_amount_swapped, test.expected_new_etas,
            "{} - new ETAS",
            test.name
        );
    }
}
