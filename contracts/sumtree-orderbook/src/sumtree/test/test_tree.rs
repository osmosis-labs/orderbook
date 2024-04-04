use crate::sumtree::node::{generate_node_id, NodeType, TreeNode, NODES};
use crate::sumtree::test::test_node::assert_internal_values;
use crate::sumtree::tree::{get_prefix_sum, TREE};
use crate::types::OrderDirection;
use cosmwasm_std::{testing::mock_dependencies, Decimal256};

struct TestPrefixSumCase {
    name: &'static str,
    nodes: Vec<NodeType>,
    target_etas: Decimal256,
    expected_sum: Decimal256,
}

#[test]
fn test_get_prefix_sum_valid() {
    let book_id = 1;
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<TestPrefixSumCase> = vec![
        TestPrefixSumCase {
            name: "Single node, target ETAS equal to node ETAS",
            nodes: vec![NodeType::leaf_uint256(10u128, 5u128)],
            target_etas: Decimal256::from_ratio(10u128, 1u128),

            // We expect the full value of the node because the prefix
            // sum is intended to return "all nodes that overlap with
            // the target ETAS".
            //
            // Since node ranges are inclusive of the
            // lower bound, the node here should be included in the sum.
            expected_sum: Decimal256::from_ratio(5u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Single node, target ETAS below node range",
            nodes: vec![NodeType::leaf_uint256(50u128, 20u128)],
            target_etas: Decimal256::from_ratio(25u128, 1u128),

            expected_sum: Decimal256::zero(),
        },
        TestPrefixSumCase {
            name: "Single node, target ETAS above node range",
            nodes: vec![NodeType::leaf_uint256(10u128, 10u128)],
            target_etas: Decimal256::from_ratio(30u128, 1u128),

            expected_sum: Decimal256::from_ratio(10u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Multiple nodes, target ETAS in the middle",
            nodes: vec![
                NodeType::leaf_uint256(5u128, 10u128),
                NodeType::leaf_uint256(15u128, 20u128),
                NodeType::leaf_uint256(35u128, 30u128),
            ],
            target_etas: Decimal256::from_ratio(20u128, 1u128),

            expected_sum: Decimal256::from_ratio(30u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Target ETAS below all nodes",
            nodes: vec![
                NodeType::leaf_uint256(10u128, 10u128),
                NodeType::leaf_uint256(20u128, 20u128),
                NodeType::leaf_uint256(40u128, 30u128),
            ],
            target_etas: Decimal256::from_ratio(5u128, 1u128),

            expected_sum: Decimal256::zero(),
        },
        TestPrefixSumCase {
            name: "Target ETAS above all nodes",
            nodes: vec![
                NodeType::leaf_uint256(10u128, 10u128),
                NodeType::leaf_uint256(20u128, 20u128),
                NodeType::leaf_uint256(40u128, 30u128),
            ],
            target_etas: Decimal256::from_ratio(45u128, 1u128),

            expected_sum: Decimal256::from_ratio(60u128, 1u128), // Sum of all nodes
        },
        TestPrefixSumCase {
            name: "Nodes inserted in reverse order",
            nodes: vec![
                NodeType::leaf_uint256(30u128, 10u128),
                NodeType::leaf_uint256(20u128, 5u128),
                NodeType::leaf_uint256(10u128, 5u128),
            ],
            target_etas: Decimal256::from_ratio(25u128, 1u128),

            expected_sum: Decimal256::from_ratio(10u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Nodes inserted in shuffled order",
            nodes: vec![
                NodeType::leaf_uint256(30u128, 10u128),
                NodeType::leaf_uint256(10u128, 5u128),
                NodeType::leaf_uint256(20u128, 5u128),
            ],
            target_etas: Decimal256::from_ratio(25u128, 1u128),

            expected_sum: Decimal256::from_ratio(10u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Nodes inserted in shuffled order, target ETAS at node lower bound",
            nodes: vec![
                NodeType::leaf_uint256(30u128, 11u128),
                NodeType::leaf_uint256(10u128, 7u128),
                NodeType::leaf_uint256(20u128, 5u128),
            ],
            target_etas: Decimal256::from_ratio(20u128, 1u128),

            // We expect the sum of the 2nd and 3rd nodes, so 7 + 5
            expected_sum: Decimal256::from_ratio(12u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Nodes with large gaps between ranges",
            nodes: vec![
                NodeType::leaf_uint256(10u128, 10u128),
                NodeType::leaf_uint256(50u128, 20u128),
                NodeType::leaf_uint256(100u128, 30u128),
            ],
            target_etas: Decimal256::from_ratio(75u128, 1u128),

            expected_sum: Decimal256::from_ratio(30u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Nodes with adjacent ranges",
            nodes: vec![
                NodeType::leaf_uint256(10u128, 10u128),
                NodeType::leaf_uint256(20u128, 10u128),
                NodeType::leaf_uint256(30u128, 10u128),
            ],
            target_etas: Decimal256::from_ratio(25u128, 1u128),

            expected_sum: Decimal256::from_ratio(20u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Complex case with many nodes (shuffled, adjacent, spaced out)",
            nodes: vec![
                NodeType::leaf_uint256(10u128, 5u128),    // 10-15
                NodeType::leaf_uint256(121u128, 19u128),  // 121-140
                NodeType::leaf_uint256(15u128, 4u128),    // 15-19 adjacent to the first
                NodeType::leaf_uint256(50u128, 10u128),   // 50-60
                NodeType::leaf_uint256(61u128, 9u128),    // 61-70
                NodeType::leaf_uint256(100u128, 20u128),  // 100-120
                NodeType::leaf_uint256(200u128, 50u128),  // 200-250
                NodeType::leaf_uint256(260u128, 40u128),  // 260-300
                NodeType::leaf_uint256(301u128, 29u128),  // 301-330
                NodeType::leaf_uint256(400u128, 100u128), // 400-500
                NodeType::leaf_uint256(600u128, 150u128), // 600-750
            ],
            // Target includes everything except the last two nodes
            target_etas: Decimal256::from_ratio(305u128, 1u128),

            // Sum of all nodes except the last two:
            // 5 + 19 + 4 + 10 + 9 + 20 + 50 + 40 + 29 = 186
            expected_sum: Decimal256::from_ratio(186u128, 1u128),
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();

        let mut root_id = generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap();
        let mut tree = TreeNode::new(book_id, tick_id, direction, root_id, NodeType::default());
        TREE.save(
            deps.as_mut().storage,
            &(book_id, tick_id, &direction.to_string()),
            &root_id,
        )
        .unwrap();
        NODES
            .save(deps.as_mut().storage, &(book_id, tick_id, tree.key), &tree)
            .unwrap();

        // Insert nodes into tree
        for node in test.nodes.iter() {
            let new_node_id = generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap();
            let mut tree_node =
                TreeNode::new(book_id, tick_id, direction, new_node_id, node.clone());
            NODES
                .save(
                    deps.as_mut().storage,
                    &(book_id, tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();

            // Process insertion
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();

            // Refetch tree. We do this manually to avoid using higher level orderbook
            // functions in low level sumtree tests.
            root_id = TREE
                .load(
                    deps.as_mut().storage,
                    &(book_id, tick_id, &direction.to_string()),
                )
                .unwrap();
            tree = NODES
                .load(deps.as_mut().storage, &(book_id, tick_id, root_id))
                .unwrap();
        }

        // Assert that the resulting tree maintains basic sumtree invariants
        let tree_nodes = tree.traverse(deps.as_ref().storage).unwrap();
        let internals: Vec<&TreeNode> = tree_nodes.iter().filter(|x| x.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, true);

        // System under test: get prefix sum
        let prefix_sum = get_prefix_sum(deps.as_ref().storage, tree, test.target_etas).unwrap();

        // Assert that the correct value was returned
        assert_eq!(
            test.expected_sum, prefix_sum,
            "{}: Expected prefix sum {}, got {}",
            test.name, test.expected_sum, prefix_sum
        );

        // Refetch tree and assert that its nodes were unchanged
        let tree = NODES
            .load(deps.as_mut().storage, &(book_id, tick_id, root_id))
            .unwrap();
        let tree_nodes_post = tree.traverse(deps.as_ref().storage).unwrap();
        assert_eq!(
            tree_nodes, tree_nodes_post,
            "Prefix sum mutated tree. Test case: {}",
            test.name
        );
    }
}
