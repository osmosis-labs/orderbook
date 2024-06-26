extern crate rand;

use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{Decimal256, Deps, DepsMut, Uint128};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::sumtree::node::{generate_node_id, NodeType, TreeNode, NODES};
use crate::sumtree::tree::{get_prefix_sum, TREE};
use crate::types::OrderDirection;

use super::test_node::assert_internal_values;

const DEFAULT_TICK_ID: i64 = 0;
const DEFAULT_DIRECTION: OrderDirection = OrderDirection::Bid;

#[test]
fn test_fuzz_insert() {
    // Set up test parameters
    let seed: u64 = 1234567890;
    let num_iterations = 5;
    let num_nodes: usize = 200;
    let mut rng = StdRng::seed_from_u64(seed);

    // Run multiple fuzz tests with random insertions
    for iteration in 0..num_iterations {
        let test_name = format!("Fuzz run {} with {} nodes", iteration + 1, num_nodes);
        println!("{test_name}");

        // Set up sumtree to insert into
        let mut deps = mock_dependencies();
        prepare_sumtree(&mut deps.as_mut(), DEFAULT_TICK_ID, DEFAULT_DIRECTION);

        // Generate a list of `num_nodes` nodes, each directly adjacent to each other
        // as would be the case with newly created limit orders on a tick
        let mut start_etas = Decimal256::zero();
        let nodes: Vec<TreeNode> = (0..num_nodes)
            .map(|_| {
                let node =
                    random_leaf_uint256_node(&mut deps.as_mut(), DEFAULT_TICK_ID, start_etas);
                start_etas = start_etas.checked_add(node.get_value()).unwrap();
                node
            })
            .collect();

        // Pick a random number of the created nodes to insert in random order
        let num_to_insert = rng.gen_range(1..=nodes.len());
        let nodes_to_insert: Vec<_> = nodes
            .choose_multiple(&mut rng, num_to_insert)
            .cloned()
            .collect();

        // Pick a random target ETAS to calculate prefix sum up to
        let max_etas = Uint128::try_from(start_etas.to_uint_floor()).unwrap();
        let target_etas = Decimal256::from_ratio(rng.gen_range(1..=max_etas.into()), 1u128);
        let mut expected_prefix_sum = Decimal256::zero();

        // Process insertions and assert invariants after each insertion
        for node in nodes_to_insert {
            let tree = insert_node(
                &mut deps.as_mut(),
                DEFAULT_TICK_ID,
                DEFAULT_DIRECTION,
                &mut node.clone(),
            );

            // If the inserted node's start ETAS is <= the target ETAS, we add the node's amount
            // to our expected prefix sum.
            if node.get_min_range() <= target_etas.checked_add(expected_prefix_sum).unwrap() {
                expected_prefix_sum = expected_prefix_sum.checked_add(node.get_value()).unwrap();
            }

            // Assert general sumtree invariants after each insertion
            assert_sumtree_invariants(deps.as_ref(), &tree, &test_name);

            // Assert prefix sum correctness
            let prefix_sum = get_prefix_sum(
                deps.as_ref().storage,
                tree.clone(),
                target_etas,
                tree.get_value(),
            )
            .unwrap();
            assert_eq!(
                expected_prefix_sum, prefix_sum,
                "{test_name}: Expected prefix sum {expected_prefix_sum}, got {prefix_sum}. Target ETAS: {target_etas}",
            );
        }
    }
}

// prepare_sumtree sets up an empty sumtree and returns the root node.
pub fn prepare_sumtree(deps: &mut DepsMut, tick_id: i64, direction: OrderDirection) -> TreeNode {
    let root_id = generate_node_id(deps.storage, tick_id).unwrap();
    let tree = TreeNode::new(tick_id, direction, root_id, NodeType::default());
    TREE.save(deps.storage, &(tick_id, &direction.to_string()), &root_id)
        .unwrap();
    NODES
        .save(deps.storage, &(tick_id, tree.key), &tree)
        .unwrap();
    tree
}

// insert_node is a helper function that inserts a node into the sumtree and returns the resulting root node.
//
// It is intended to be used for lower level sumtree tests when we want to test behavior we might expect
// from higher level functions but do not have access to the setup logic they provide.
pub fn insert_node(
    deps: &mut DepsMut,
    tick_id: i64,
    direction: OrderDirection,
    node: &mut TreeNode,
) -> TreeNode {
    let mut root_id = TREE
        .load(deps.storage, &(tick_id, &direction.to_string()))
        .unwrap();
    let mut tree = NODES.load(deps.storage, &(tick_id, root_id)).unwrap();

    tree.insert(deps.storage, node).unwrap();

    root_id = TREE
        .load(deps.storage, &(tick_id, &direction.to_string()))
        .unwrap();
    NODES.load(deps.storage, &(tick_id, root_id)).unwrap()
}

// assert_sumtree_invariants takes in a sumtree and asserts that it maintains basic sumtree invariants.
// TODO: add invariant check for sibling ranges not overlapping here (https://github.com/osmosis-labs/orderbook/issues/94)
pub fn assert_sumtree_invariants(deps: Deps, tree: &TreeNode, test_name: &str) {
    let tree_nodes = tree.traverse(deps.storage).unwrap();
    let internals: Vec<&TreeNode> = tree_nodes.iter().filter(|x| x.is_internal()).collect();
    assert_internal_values(test_name, deps, internals, true);
}

// Generates a random leaf node with a random amount and the given ETAS.
pub fn random_leaf_uint256_node(
    deps: &mut DepsMut,
    tick_id: i64,
    start_etas: Decimal256,
) -> TreeNode {
    let new_node_id = generate_node_id(deps.storage, tick_id).unwrap();
    let mut rng = StdRng::seed_from_u64(new_node_id);

    let new_node = TreeNode::new(
        tick_id,
        DEFAULT_DIRECTION,
        new_node_id,
        NodeType::leaf(start_etas, Decimal256::from_ratio(rng.gen::<u32>(), 1u128)),
    );
    NODES
        .save(deps.storage, &(tick_id, new_node.key), &new_node)
        .unwrap();
    new_node
}
