use cosmwasm_std::{testing::mock_dependencies, Decimal256, Deps, Storage, Uint256};

use crate::{
    sumtree::{
        node::{generate_node_id, NodeType, TreeNode, NODES},
        tree::{get_prefix_sum, get_root_node, TREE},
    },
    types::OrderDirection,
    ContractError,
};

struct TestNodeInsertCase {
    name: &'static str,
    nodes: Vec<NodeType>,
    // Depth first search ordering of node IDs (Could be improved?)
    expected: Vec<u64>,
    // Whether to print the tree
    print: bool,
}

// Asserts all values of internal nodes are as expected
pub fn assert_internal_values(
    test_name: &str,
    deps: Deps,
    internals: Vec<&TreeNode>,
    should_be_balanced: bool,
) {
    for internal_node in internals {
        let left_node = internal_node.get_left(deps.storage).unwrap();
        let right_node = internal_node.get_right(deps.storage).unwrap();

        let accumulated_value = left_node
            .clone()
            .map_or(Decimal256::zero(), |x| x.get_value())
            .checked_add(
                right_node
                    .clone()
                    .map_or(Decimal256::zero(), |x| x.get_value()),
            )
            .unwrap();
        assert_eq!(
            internal_node.get_value(),
            accumulated_value,
            "{} failed on internal node value, expected {} got {} for {}",
            test_name,
            accumulated_value,
            internal_node.get_value(),
            internal_node.key
        );

        let min = left_node
            .clone()
            .map_or(Decimal256::MAX, |n| n.get_min_range())
            .min(
                right_node
                    .clone()
                    .map_or(Decimal256::MAX, |n| n.get_min_range()),
            );
        let max = left_node
            .clone()
            .map_or(Decimal256::MIN, |n| n.get_max_range())
            .max(
                right_node
                    .clone()
                    .map_or(Decimal256::MIN, |n| n.get_max_range()),
            );
        assert_eq!(internal_node.get_min_range(), min);
        assert_eq!(internal_node.get_max_range(), max);

        let balance_factor = right_node
            .clone()
            .map_or(0, |n| n.get_height(deps.storage).unwrap())
            .abs_diff(
                left_node
                    .clone()
                    .map_or(0, |n| n.get_height(deps.storage).unwrap()),
            );

        assert_eq!(
            internal_node.get_weight(),
            internal_node.get_height(deps.storage).unwrap(),
            "{}: Internal weight incorrect for node {}",
            test_name,
            internal_node.key
        );

        if should_be_balanced {
            assert!(
                balance_factor <= 1,
                "{}: Balance factor greater than 1 for node {}",
                test_name,
                internal_node.key
            );
        }

        if let Some(left) = left_node {
            let parent_string = if let Some(parent) = left.parent {
                parent.to_string()
            } else {
                "None".to_string()
            };
            assert_eq!(
                left.parent,
                Some(internal_node.key),
                "{} - Child {} does not have correct parent: expected {}, received {}",
                test_name,
                left,
                internal_node.key,
                parent_string
            );
        }
        if let Some(right) = right_node {
            let parent_string = if let Some(parent) = right.parent {
                parent.to_string()
            } else {
                "None".to_string()
            };
            assert_eq!(
                right.parent,
                Some(internal_node.key),
                "{} - Child {} does not have correct parent: expected {}, received {}",
                test_name,
                right,
                internal_node.key,
                parent_string
            );
        }
    }
}

#[test]
fn test_node_insert_cases() {
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<TestNodeInsertCase> = vec![
        // Pre
        // ---
        //                                        1: 31 1-38
        //                 ┌────────────────────────────────────────────────┐
        //            5: 13 1-20                                     7: 18 20-38
        //     ┌────────────────────────┐                        ┌────────────────────────┐
        //  2: 1 5                 4: 12 8                    3: 20 10                6: 30 8
        //
        // Post
        // ----
        //                                                        1: 37 1-38
        //                             ┌────────────────────────────────────────────────────────────────┐
        //                        5: 19 1-20                                                     7: 18 20-38
        //             ┌────────────────────────────────┐                                ┌────────────────────────────────┐
        //        9: 11 1-12                       4: 12 8                            3: 20 10                        6: 30 8
        //     ┌────────────────┐
        //  2: 1 5          ->8: 6 6
        TestNodeInsertCase {
            name: "Case 1: New node fits in left internal range, insert left",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 5u32),
                NodeType::leaf_uint256(20u32, 10u32),
                NodeType::leaf_uint256(12u32, 8u32),
                NodeType::leaf_uint256(30u32, 8u32),
                NodeType::leaf_uint256(6u32, 6u32),
            ],
            expected: vec![1, 5, 9, 2, 8, 4, 7, 3, 6],
            print: true,
        },
        // Pre
        // ---
        //                                       1: 32 1-38
        //                 ┌────────────────────────────────────────────────┐
        //            5: 19 1-20                                     7: 13 20-38
        //     ┌────────────────────────┐                        ┌────────────────────────┐
        // 2: 1 11                 4: 12 8                    3: 20 5                 6: 30 8
        //
        // Post
        // ----
        //                                                 1: 37 1-38
        //                     ┌────────────────────────────────────────────────────────────────┐
        //                5: 19 1-20                                                     7: 18 20-38
        //     ┌────────────────────────────────┐                                ┌────────────────────────────────┐
        // 2: 1 11                         4: 12 8                          9: 10 20-30                       6: 30 8
        //                                                             ┌────────────────┐
        //                                                         3: 20 5         ->8: 25 5
        TestNodeInsertCase {
            name: "Case 2: New node fits in right internal range, insert right",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 11u32),
                NodeType::leaf_uint256(20u32, 5u32),
                NodeType::leaf_uint256(12u32, 8u32),
                NodeType::leaf_uint256(30u32, 8u32),
                NodeType::leaf_uint256(25u32, 5u32),
            ],
            expected: vec![1, 5, 2, 4, 7, 9, 3, 8, 6],
            print: true,
        },
        // Pre
        // ---
        //                        1: 30 1-38                                
        //             ┌────────────────────────────────┐                
        //        5: 17 1-18                     7: 13 20-38                
        //     ┌────────────────┐                ┌────────────────┐        
        // 2: 1 11         4: 12 6            3: 20 5         6: 30 8  
        //
        // Post
        // ----
        //                                                1: 32 1-38                                                                
        //                     ┌────────────────────────────────────────────────────────────────┐                                
        //                5: 19 1-20                                                     7: 13 20-38                                
        //     ┌────────────────────────────────┐                                ┌────────────────────────────────┐                
        // 2: 1 11                        9: 8 12-20                            3: 20 5                         6: 30 8                
        //                             ┌────────────────┐                                                                        
        //                         4: 12 6         8: 18 2  
        TestNodeInsertCase {
            name: "Case 3: Both left and right are internal, node does not fit in either, insert left",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 11u32),
                NodeType::leaf_uint256(20u32, 5u32),
                NodeType::leaf_uint256(12u32, 6u32),
                NodeType::leaf_uint256(30u32, 8u32),
                NodeType::leaf_uint256(18u32, 2u32),
            ],
            expected: vec![1, 5, 2, 9, 4, 8, 7, 3, 6],
            print: true,
        },
        // Pre
        // ---
        //          1: 20 1-30
        //     ┌────────────────┐
        // 2: 1 10         3: 20 10
        //
        // Post
        // ----
        //                          1: 28 1-30
        //             ┌────────────────────────────────┐
        //        5: 18 1-20                       3: 20 10
        //     ┌────────────────┐
        // 2: 1 10         ->4: 12 8
        TestNodeInsertCase {
            name: "Case 4: New node does not fit in right range (or is less than right.min if right is a leaf) and left node is a leaf, split left",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(20u32, 10u32),
                NodeType::leaf_uint256(12u32, 8u32),
            ],
            expected: vec![1, 5, 2, 4, 3],
            print: true,
        },
        // Pre
        // ---
        //                            1: 28 1-30
        //             ┌────────────────────────────────┐
        //        5: 18 1-20                       3: 20 10
        //     ┌────────────────┐
        // 2: 1 10         4: 12 8
        //
        // Post
        // ----
        //                          1: 36 1-38
        //             ┌────────────────────────────────┐
        //        5: 18 1-20                     7: 18 20-38
        //     ┌────────────────┐                ┌────────────────┐
        // 2: 1 10         4: 12 8            3: 20 10        -> 6: 30 8
        TestNodeInsertCase {
            name: "Case 5: New node does not fit in left range (or is greater than or equal to left.max when left is a leaf) and right node is a leaf, split right",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(20u32, 10u32),
                NodeType::leaf_uint256(12u32, 8u32),
                NodeType::leaf_uint256(30u32, 8u32),
            ],
            expected: vec![1, 5, 2, 4, 7, 3, 6],
            print: true,
        },
        // Pre
        // ---
        //     1: 10 12-22
        //     ┌────────
        // 2: 12 10
        //
        // Post
        // ----
        //          1: 20 1-22
        //     ┌────────────────┐
        // ->2: 1 10         3: 12 10
        TestNodeInsertCase {
            name: "Case 6: Right node is empty, new node is lower than left node, move left node to right and insert left",
            nodes: vec![
                NodeType::leaf_uint256(12u32, 10u32),
                NodeType::leaf_uint256(1u32, 10u32),
            ],
            expected: vec![1, 3, 2],
            print: true,
        },
        // TODO: Explicitly build trees in this test to allow testing case 7
        // TestNodeInsertCase {
        //     name: "Case 7: Left node is empty, new node is higher than right node, move right node to left and insert right",
        //     nodes: vec![
        //         NodeType::leaf_uint256(12u32, 10u32),
        //         NodeType::leaf_uint256(1u32, 10u32),
        //     ],
        //     expected: vec![1, 3, 2],
        //     print: true,
        // },
        
        // Pre
        // ---
        // No Tree
        //
        // Post
        // ----
        //            1: 10 1-11
        //     ┌────────
        // ->2: 1 10
        TestNodeInsertCase {
            name: "Case 8: Left node is empty, insert left",
            nodes: vec![NodeType::leaf_uint256(1u32, 10u32)],
            expected: vec![1, 2],
            print: true,
        },
        // Pre
        // ---
        //     1: 10 1-11
        //     ┌────────
        // 2: 1 10
        //
        // Post
        // ----
        //          1: 20 1-22
        //     ┌────────────────┐
        // 2: 1 10         ->3: 12 10
        TestNodeInsertCase {
            name: "Case 9: Right is empty, insert right",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(12u32, 10u32),
            ],
            expected: vec![1, 2, 3],
            print: true,
        },
        TestNodeInsertCase {
            name: "Insert sequential nodes",
            nodes: vec![
                NodeType::leaf_uint256(5u128, 10u128),
                NodeType::leaf_uint256(15u128, 20u128),
                NodeType::leaf_uint256(35u128, 30u128),
            ],
            expected: vec![1, 2, 5, 3, 4],
            print: true,
        },
        TestNodeInsertCase {
            name: "Insert adjacent nodes in decreasing order",
            nodes: vec![
                NodeType::leaf_uint256(35u128, 25u128),
                NodeType::leaf_uint256(10u128, 25u128),
            ],
            expected: vec![1, 3, 2],
            print: true,
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();
        // Create tree root
        let mut tree = TreeNode::new(
            tick_id,
            direction,
            generate_node_id(deps.as_mut().storage,  tick_id).unwrap(),
            NodeType::internal_uint256(Uint256::zero(), (u32::MAX, u32::MIN)),
        );

        // Insert nodes into tree
        for (idx, node) in test.nodes.iter().enumerate() {
            let mut tree_node = TreeNode::new(
                tick_id,
                direction,
                generate_node_id(deps.as_mut().storage,  tick_id).unwrap(),
                node.clone(),
            );
            NODES
                .save(
                    deps.as_mut().storage,
                    &( tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();

            //Print tree at second last node to see pre-insert
            if test.nodes.len() >= 2 && idx == test.nodes.len() - 2 && test.print {
                print_tree("Pre-Insert Tree", test.name, &tree, &deps.as_ref());
            }
        }

        if test.print {
            print_tree("Post-Insert Tree", test.name, &tree, &deps.as_ref());
        }

        // Return tree in vector form from Depth First Search
        let result = tree.traverse(deps.as_ref().storage).unwrap();

        assert_eq!(
            result,
            test.expected
                .iter()
                .map(|key| NODES
                    .load(deps.as_ref().storage, &( tick_id, *key))
                    .unwrap())
                .collect::<Vec<TreeNode>>()
        );

        // Ensure all internal nodes are correctly summed and contain correct ranges
        let internals: Vec<&TreeNode> = result.iter().filter(|x| x.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, true);
    }
}

struct NodeDeletionTestCase {
    name: &'static str,
    nodes: Vec<NodeType>,
    delete: Vec<u64>,
    // Depth first search ordering of node IDs (Could be improved?)
    expected: Vec<u64>,
    // Whether to print the tree
    print: bool,
}

#[test]
fn test_node_deletion_valid() {
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<NodeDeletionTestCase> = vec![
        // Pre
        // ---
        //          1: 10 1-11
        //     ┌────────
        // ->2: 1 10
        //
        // Post
        // ----
        // No tree
        NodeDeletionTestCase {
            name: "Remove only node",
            nodes: vec![NodeType::leaf_uint256(1u32, 10u32)],
            delete: vec![2],
            expected: vec![],
            print: true,
        },
        // Pre
        // ---
        //          1: 15 1-16
        //     ┌────────────────┐
        // ->2: 1 10         3: 11 5
        //
        // Post
        // ----
        // 1: 5 11-16
        //      ────────┐
        //          3: 11 5
        NodeDeletionTestCase {
            name: "Remove one of two nodes",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(11u32, 5u32),
            ],
            delete: vec![2],
            expected: vec![1, 3],
            print: true,
        },
        // Pre
        // ---
        //                       1: 25 1-26
        //             ┌────────────────────────────────┐
        //        5: 20 1-21                       3: 21 5
        //     ┌────────────────┐
        // ->2: 1 10      4: 11 10
        //
        // Post
        // ----
        //                   1: 15 11-26
        //         ┌────────────────────────────────┐
        //   5: 10 11-21                       3: 21 5
        //         ────────┐
        //             4: 11 10
        NodeDeletionTestCase {
            name: "Remove nested node",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(21u32, 5u32),
                NodeType::leaf_uint256(11u32, 10u32),
            ],
            delete: vec![2],
            expected: vec![1, 5, 4, 3],
            print: true,
        },
        // Pre
        // ---
        //                      1: 25 1-26
        //         ┌────────────────────────────────┐
        //    5: 20 1-21                       3: 21 5
        // ┌────────────────┐
        // ->2: 1 10     ->4: 11 10
        //
        // Post
        // ----
        // 1: 5 21-26
        //      ────────┐
        //          3: 21 5
        NodeDeletionTestCase {
            name: "Remove both children of internal",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(21u32, 5u32),
                NodeType::leaf_uint256(11u32, 10u32),
            ],
            delete: vec![2, 4],
            expected: vec![1, 3],
            print: true,
        },
        // Pre
        // ---
        //                      1: 25 1-26
        //         ┌────────────────────────────────┐
        //    ->5: 20 1-21                       3: 21 5
        // ┌────────────────┐
        // 2: 1 10       4: 11 10
        //
        // Post
        // ----
        // 1: 5 21-26
        //       ────────┐
        //           3: 21 5
        NodeDeletionTestCase {
            name: "Remove parent node",
            nodes: vec![
                NodeType::leaf_uint256(1u32, 10u32),
                NodeType::leaf_uint256(21u32, 5u32),
                NodeType::leaf_uint256(11u32, 10u32),
            ],
            delete: vec![5],
            expected: vec![1, 3],
            print: true,
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();
        let mut tree = TreeNode::new(
            tick_id,
            direction,
            generate_node_id(deps.as_mut().storage,  tick_id).unwrap(),
            NodeType::internal_uint256(Uint256::zero(), (u32::MAX, u32::MIN)),
        );

        for node in test.nodes {
            let mut tree_node = TreeNode::new(
                tick_id,
                direction,
                generate_node_id(deps.as_mut().storage,  tick_id).unwrap(),
                node,
            );
            NODES
                .save(
                    deps.as_mut().storage,
                    &( tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();
        }

        if test.print {
            print_tree("Pre-Deletion Tree", test.name, &tree, &deps.as_ref());
        }

        for key in test.delete.clone() {
            let node = NODES
                .load(deps.as_ref().storage, &( tick_id, key))
                .unwrap();
            node.delete(deps.as_mut().storage).unwrap();
        }

        if test.expected.is_empty() {
            let maybe_parent = tree.get_parent(deps.as_ref().storage).unwrap();
            assert!(maybe_parent.is_none(), "Parent node should not exist");
            continue;
        }

        let tree = NODES
            .load(deps.as_ref().storage, &( tick_id, tree.key))
            .unwrap();

        if test.print {
            print_tree("Post-Deletion Tree", test.name, &tree, &deps.as_ref());
        }

        let result = tree.traverse(deps.as_ref().storage).unwrap();

        assert_eq!(
            result,
            test.expected
                .iter()
                .map(|key| NODES
                    .load(deps.as_ref().storage, &( tick_id, *key))
                    .unwrap())
                .collect::<Vec<TreeNode>>()
        );

        for key in test.delete {
            let maybe_node = NODES
                .may_load(deps.as_ref().storage, &( tick_id, key))
                .unwrap();
            assert!(maybe_node.is_none(), "Node {key} was not deleted");
        }

        let internals: Vec<&TreeNode> = result.iter().filter(|x| x.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, true);
    }
}

struct RotateRightTestCase {
    name: &'static str,
    nodes: Vec<TreeNode>,
    expected: Vec<u64>,
    expected_error: Option<ContractError>,
    print: bool,
}

#[test]
fn test_rotate_right() {
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<RotateRightTestCase> = vec![
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────
        //             2: 2 1-3
        //         ┌────────────────┐
        //      3: 1 1          4: 2 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 2 1-3
        //                 ┌────────────────────────────────┐
        //              3: 1 1                         1: 1 2-3
        //                                         ┌────────
        //                                      4: 2 1
        RotateRightTestCase {
            name: "Left internal right empty",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), None),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), Some(4))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
            ],
            expected: vec![2, 3, 1, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                         4: 2 1
        //         ┌────────
        //      3: 1 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 2 1-3
        //                 ┌────────────────────────────────┐
        //              3: 1 1                         1: 1 2-3
        //                                                 ────────┐
        //                                                      4: 2 1
        RotateRightTestCase {
            name: "Left internal (no left-right ancestor) right leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(4)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), None)
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 3, 1, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                         4: 2 1
        //                 ────────┐
        //                      3: 1 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 2 1-3
        //                                 ────────────────┐
        //                                             1: 2 1-3
        //                                         ┌────────────────┐
        //                                      3: 1 1          4: 2 1
        RotateRightTestCase {
            name: "Left internal (no left-left ancestor) right leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(4)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(None, Some(3))
                .with_parent(1),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 1, 3, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                         5: 3 1
        //         ┌────────────────┐
        //      3: 1 1          4: 2 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 3 1-4
        //                 ┌────────────────────────────────┐
        //              3: 1 1                         1: 2 2-4
        //                                         ┌────────────────┐
        //                                      4: 2 1          5: 3 1
        RotateRightTestCase {
            name: "Left internal right leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(5)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), Some(4))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 3, 1, 4, 5],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                        5: 2 3-5
        //         ┌────────────────┐                ┌────────────────┐
        //      3: 1 1          4: 2 1             6: 3 1          7: 4 1
        //
        // Post-rotation
        // --------------------------
        //                              2: 4 1-5
        //  ┌────────────────────────────────────────────────────────────────┐
        // 3: 1 1                                                         1: 3 2-5
        //                                                   ┌────────────────────────────────┐
        //                                                 4: 2 1                         5: 2 3-5
        //                                          ┌────────────────┐
        //                                        6: 3 1          7: 4 1
        RotateRightTestCase {
            name: "Left internal right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(5)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), Some(4))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![2, 3, 1, 4, 5, 6, 7],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                        5: 2 3-5
        //                 ────────┐                ┌────────────────┐
        //                      4: 2 1             6: 3 1          7: 4 1
        //
        // Post-rotation
        // --------------------------
        //              2: 3 2-5
        //                  ────────────────┐
        //                              1: 3 2-5
        //                  ┌────────────────────────────────┐
        //               4: 2 1                         5: 2 3-5
        //          ┌────────────────┐
        //        6: 3 1          7: 4 1
        RotateRightTestCase {
            name: "Left internal (no left-left) right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(5)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(None, Some(4))
                .with_parent(1),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![2, 1, 4, 5, 6, 7],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //             2: 2 1-3                        5: 2 3-5
        //         ┌────────                        ┌────────────────┐
        //      4: 2 1                             6: 3 1          7: 4 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 3 2-5
        //   ┌────────────────────────────────────────────────────────────────┐
        // 4: 2 1                                                         1: 2 3-5
        //                                                                   ────────────────┐
        //                                                                                 5: 2 3-5
        //                                                                           ┌────────────────┐
        //                                                                         6: 3 1          7: 4 1
        RotateRightTestCase {
            name: "Left internal (no left-right) right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(5)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), None)
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![2, 4, 1, 5, 6, 7],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                                ─────────────────┐
        //                                             5: 2 3-5
        //                                         ┌────────────────┐
        //                                      6: 3 1          7: 4 1
        RotateRightTestCase {
            name: "Left empty right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(None, Some(5)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![],
            expected_error: Some(ContractError::InvalidNodeType),
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //              2: 2 1                         5: 2 3-5
        //                                         ┌────────────────┐
        //                                      6: 3 1          7: 4 1
        RotateRightTestCase {
            name: "Left leaf right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(5)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![],
            expected_error: Some(ContractError::InvalidNodeType),
            print: true,
        },
    ];

    for mut test in test_cases {
        let mut deps = mock_dependencies();
        // Save nodes in storage
        for (idx, node) in test.nodes.iter_mut().enumerate() {
            // Save root node
            if idx == 0 {
                TREE.save(
                    deps.as_mut().storage,
                    &( tick_id, &direction.to_string()),
                    &node.key,
                )
                .unwrap();
            }
            NODES
                .save(deps.as_mut().storage, &( tick_id, node.key), node)
                .unwrap();
        }

        // Sync weights post node storage
        for mut node in test.nodes {
            if node.is_internal() || node.parent.is_none() {
                continue;
            }
            node.sync(deps.as_ref().storage).unwrap();
            let mut parent = node.get_parent(deps.as_ref().storage).unwrap().unwrap();
            parent
                .sync_range_and_value_up(deps.as_mut().storage)
                .unwrap();
        }

        let mut tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Pre-rotation", test.name, &tree, &deps.as_ref());
        }

        let res = tree.rotate_right(deps.as_mut().storage);

        if let Some(err) = test.expected_error {
            assert_eq!(res, Err(err), "{}", test.name);
            continue;
        }

        // Get new root node, it may have changed due to rotationss
        let tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Post-rotation", test.name, &tree, &deps.as_ref());
        }

        let nodes = tree.traverse(deps.as_ref().storage).unwrap();
        let res: Vec<u64> = nodes.iter().map(|n| n.key).collect();
        assert_eq!(res, test.expected, "{}", test.name);

        let internals = nodes.iter().filter(|n| n.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, false);
    }
}

struct RotateLeftTestCase {
    name: &'static str,
    nodes: Vec<TreeNode>,
    expected: Vec<u64>,
    expected_error: Option<ContractError>,
    print: bool,
}

#[test]
fn test_rotate_left() {
    
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<RotateLeftTestCase> = vec![
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                                 ────────────────┐
        //                                             2: 2 1-3
        //                                         ┌────────────────┐
        //                                      3: 1 1          4: 2 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 2 1-3
        //                 ┌────────────────────────────────┐
        //             1: 1 1-2                         4: 2 1
        //                 ────────┐
        //                      3: 1 1
        RotateLeftTestCase {
            name: "Right internal left empty",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(None, Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), Some(4))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
            ],
            expected: vec![2, 1, 3, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //              4: 2 1                         2: 2 1-3
        //                                         ┌────────
        //                                      3: 1 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 2 1-3
        //                 ┌────────────────
        //             1: 2 1-3
        //         ┌────────────────┐
        //      4: 2 1          3: 1 1
        RotateLeftTestCase {
            name: "Right internal (no right-right ancestor) left leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(4), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), None)
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 1, 4, 3],
            expected_error: None,
            print: true,
        },
        // Pre-rotation:
        // --------------------------
        //                             1: 2 1-3
        //                 ┌────────────────────────────────┐
        //              4: 2 1                         2: 1 1-2
        //                                                 ────────┐
        //                                                      3: 1 1
        //
        // Post-rotation:
        // --------------------------
        //                             2: 2 1-3
        //                 ┌────────────────────────────────┐
        //             1: 1 2-3                         3: 1 1
        //         ┌────────
        //      4: 2 1
        RotateLeftTestCase {
            name: "Right internal (no right-left ancestor) left leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(4), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(None, Some(3))
                .with_parent(1),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 1, 4, 3],
            expected_error: None,
            print: true,
        },
        // Pre-rotation:
        // --------------------------
        //                             1: 3 1-4
        //                 ┌────────────────────────────────┐
        //              5: 3 1                         2: 2 1-3
        //                                         ┌────────────────┐
        //                                      3: 1 1          4: 2 1
        //
        // Post-rotation:
        // --------------------------
        //                             2: 3 1-4
        //                 ┌────────────────────────────────┐
        //             1: 2 1-4                         4: 2 1
        //         ┌────────────────┐
        //      5: 3 1          3: 1 1
        RotateLeftTestCase {
            name: "Right internal left leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(5), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(3), Some(4))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(2),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 1, 5, 3, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation:
        // --------------------------
        //                             1: 3 2-5
        //                 ┌────────────────────────────────┐
        //             5: 2 3-5                        2: 1 2-3
        //         ┌────────────────┐                        ────────┐
        //      6: 3 1          7: 4 1                             4: 2 1
        //
        // Post-rotation:
        // --------------------------
        //                                                             2: 3 2-5
        //                                 ┌────────────────────────────────────────────────────────────────┐
        //                             1: 2 3-5                                                         4: 2 1
        //                 ┌────────────────
        //             5: 2 3-5
        //         ┌────────────────┐
        //      6: 3 1          7: 4 1
        RotateLeftTestCase {
            name: "Right internal (no right-left) left internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(5), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(None, Some(4))
                .with_parent(1),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![2, 1, 5, 6, 7, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                             1: 3 2-5
        //                 ┌────────────────────────────────┐
        //             5: 2 3-5                        2: 1 2-3
        //         ┌────────────────┐                ┌────────
        //      6: 3 1          7: 4 1             4: 2 1
        //
        // Post-rotation
        // --------------------------
        //                                                             2: 3 2-5
        //                                 ┌────────────────────────────────
        //                             1: 3 2-5
        //                 ┌────────────────────────────────┐
        //             5: 2 3-5                         4: 2 1
        //         ┌────────────────┐
        //      6: 3 1          7: 4 1
        RotateLeftTestCase {
            name: "Right internal (no right-right) left internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(5), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), None)
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![2, 1, 5, 6, 7, 4],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                                ─────────────────┐
        //                                             5: 2 3-5
        //                                         ┌────────────────┐
        //                                      6: 3 1          7: 4 1
        RotateLeftTestCase {
            name: "Right empty left internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(5), None),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![],
            expected_error: Some(ContractError::InvalidNodeType),
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                        1: 0 4294967295-0
        //                 ┌────────────────────────────────┐
        //              2: 2 1                         5: 2 3-5
        //                                         ┌────────────────┐
        //                                      6: 3 1          7: 4 1
        RotateLeftTestCase {
            name: "Right leaf left internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(5), Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(2u32, (3u32, 5u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![],
            expected_error: Some(ContractError::InvalidNodeType),
            print: true,
        },
    ];

    for mut test in test_cases {
        let mut deps = mock_dependencies();
        // Save nodes in storage
        for (idx, node) in test.nodes.iter_mut().enumerate() {
            // Save root node
            if idx == 0 {
                TREE.save(
                    deps.as_mut().storage,
                    &( tick_id, &direction.to_string()),
                    &node.key,
                )
                .unwrap();
            }
            NODES
                .save(deps.as_mut().storage, &( tick_id, node.key), node)
                .unwrap();
        }

        // Sync weights post node storage
        for mut node in test.nodes {
            if node.is_internal() || node.parent.is_none() {
                continue;
            }
            node.sync(deps.as_ref().storage).unwrap();
            let mut parent = node.get_parent(deps.as_ref().storage).unwrap().unwrap();
            parent
                .sync_range_and_value_up(deps.as_mut().storage)
                .unwrap();
        }

        let mut tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Pre-rotation", test.name, &tree, &deps.as_ref());
        }

        let res = tree.rotate_left(deps.as_mut().storage);

        if let Some(err) = test.expected_error {
            assert_eq!(res, Err(err), "{}", test.name);
            continue;
        }

        // Get new root node, it may have changed due to rotations
        let tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Post-rotation", test.name, &tree, &deps.as_ref());
        }

        let nodes = tree.traverse(deps.as_ref().storage).unwrap();
        let res: Vec<u64> = nodes.iter().map(|n| n.key).collect();
        assert_eq!(res, test.expected, "{}", test.name);

        let internals = nodes.iter().filter(|n| n.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, false);
    }
}

struct RebalanceTestCase {
    name: &'static str,
    nodes: Vec<TreeNode>,
    expected: Vec<u64>,
    expected_error: Option<ContractError>,
    print: bool,
}

#[test]
fn test_rebalance() {
    let tick_id = 1;
    let direction = OrderDirection::Bid;
    let test_cases: Vec<RebalanceTestCase> = vec![
        // Pre-rotation: Case 1: Right Right
        // --------------------------
        //                                                        1: 0 4294967295-0
        //                                 ┌────────────────────────────────────────────────────────────────┐
        //                              2: 1 1                                                         3: 2 1-3
        //                                                                                 ┌────────────────────────────────┐
        //                                                                              4: 4 1                    5: 0 4294967295-0
        //                                                                                                         ┌────────────────┐
        //                                                                                                      6: 2 1          7: 3 1
        //
        // Post-rotation: Case 1: Right Right
        // --------------------------
        //                             3: 2 1-5
        //                 ┌────────────────────────────────┐
        //             1: 2 1-5                   5: 0 4294967295-0
        //         ┌────────────────┐                ┌────────────────┐
        //      2: 1 1          4: 4 1             6: 2 1          7: 3 1
        RebalanceTestCase {
            name: "Case 1: Right Right",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(1),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(3),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(3),
                // Right-Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(5),
                // Right-Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(5),
            ],
            expected: vec![3, 1, 2, 4, 5, 6, 7],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                                                             1: 4 1-5
        //                                 ┌────────────────────────────────────────────────────────────────┐
        //                             2: 3 2-5                                                         3: 1 1
        //                 ┌────────────────────────────────┐
        //             4: 2 2-4                         5: 4 1
        //         ┌────────────────┐
        //      6: 2 1          7: 3 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 4 1-5
        //                 ┌────────────────────────────────┐
        //             4: 2 2-4                        1: 2 1-5
        //         ┌────────────────┐                ┌────────────────┐
        //      6: 2 1          7: 3 1             5: 4 1          3: 1 1
        RebalanceTestCase {
            name: "Case 2: Left Left",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(2),
                // Left-Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(4),
                // Left-Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(4),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 4, 6, 7, 1, 5, 3],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                                                        1: 0 4294967295-0
        //                                 ┌────────────────────────────────────────────────────────────────┐
        //                              2: 1 1                                                         3: 2 1-3
        //                                                                                 ┌────────────────────────────────┐
        //                                                                        4: 0 4294967295-0                     5: 4 1
        //                                                                         ┌────────────────┐
        //                                                                      6: 2 1          7: 3 1
        //
        // Post-rotation
        // --------------------------
        //                             4: 4 1-5
        //                 ┌────────────────────────────────┐
        //             1: 2 1-3                        3: 2 3-5
        //         ┌────────────────┐                ┌────────────────┐
        //      2: 1 1          6: 2 1             7: 3 1          5: 4 1
        RebalanceTestCase {
            name: "Case 3: Right Left",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(1),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(3),
                // Right-Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(4),
                // Right-Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(4),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(3),
            ],
            expected: vec![4, 1, 2, 6, 3, 7, 5],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                                                             1: 4 1-5
        //                                 ┌────────────────────────────────────────────────────────────────┐
        //                             2: 3 2-5                                                         3: 1 1
        //                 ┌────────────────────────────────┐
        //             4: 2 2-4                         5: 4 1
        //         ┌────────────────┐
        //      6: 2 1          7: 3 1
        //
        // Post-rotation
        // --------------------------
        //                             2: 4 1-5
        //                 ┌────────────────────────────────┐
        //             4: 2 2-4                        1: 2 1-5
        //         ┌────────────────┐                ┌────────────────┐
        //      6: 2 1          7: 3 1             5: 4 1          3: 1 1
        RebalanceTestCase {
            name: "Case 4: Left Right",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (1u32, 3u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(2),
                // Left-Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(4),
                // Left-Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(4),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(1u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![2, 4, 6, 7, 1, 5, 3],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //             1: 1 2-3
        //         ┌────────
        //      2: 2 1
        //
        // Post-rotation
        // --------------------------
        //             1: 1 2-3
        //         ┌────────
        //      2: 2 1
        RebalanceTestCase {
            name: "Pre-balanced: left leaf right empty",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), None),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![1, 2],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //             1: 1 2-3
        //                 ────────┐
        //                      2: 2 1
        //
        // Post-rotation
        // --------------------------
        //             1: 1 2-3
        //                 ────────┐
        //                      2: 2 1
        RebalanceTestCase {
            name: "Pre-balanced: left empty right leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(None, Some(2)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![1, 2],
            expected_error: None,
            print: true,
        },
        // Pre-rotation:
        // --------------------------
        //                             1: 3 2-4
        //                 ┌────────────────────────────────┐
        //             2: 2 2-4                         3: 2 1
        //         ┌────────────────┐
        //      4: 2 1          5: 3 1
        //
        // Post-rotation:
        // --------------------------
        //                             1: 3 2-4
        //                 ┌────────────────────────────────┐
        //             2: 2 2-4                         3: 2 1
        //         ┌────────────────┐
        //      4: 2 1          5: 3 1
        RebalanceTestCase {
            name: "Pre-balanced: left internal right leaf",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (2u32, 4u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Left-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Left-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
            ],
            expected: vec![1, 2, 4, 5, 3],
            expected_error: None,
            print: true,
        },
        // Pre-rotation
        // --------------------------
        //                             1: 3 2-4
        //                 ┌────────────────────────────────┐
        //              2: 2 1                         3: 2 2-4
        //                                         ┌────────────────┐
        //                                      4: 2 1          5: 3 1
        //
        // Post-rotation
        // --------------------------
        //                             1: 3 2-4
        //                 ┌────────────────────────────────┐
        //              2: 2 1                         3: 2 2-4
        //                                         ┌────────────────┐
        //                                      4: 2 1          5: 3 1
        RebalanceTestCase {
            name: "Pre-balanced: left leaf right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(1),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::internal_uint256(2u32, (2u32, 4u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(3),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(3),
            ],
            expected: vec![1, 2, 3, 4, 5],
            expected_error: None,
            print: true,
        },
        // Pre-rotation:
        // --------------------------
        //                             1: 4 2-6
        //                 ┌────────────────────────────────┐
        //             2: 2 2-4                        3: 2 4-6
        //         ┌────────────────┐                ┌────────────────┐
        //      4: 2 1          5: 3 1             6: 4 1          7: 5 1
        //
        // Post-rotation:
        // --------------------------
        //                             1: 4 2-6
        //                 ┌────────────────────────────────┐
        //             2: 2 2-4                        3: 2 4-6
        //         ┌────────────────┐                ┌────────────────┐
        //      4: 2 1          5: 3 1             6: 4 1          7: 5 1
        RebalanceTestCase {
            name: "Pre-balanced: left internal right internal",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
                )
                .with_children(Some(2), Some(3)),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    2,
                    NodeType::internal_uint256(2u32, (2u32, 4u32)),
                )
                .with_children(Some(4), Some(5))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    4,
                    NodeType::leaf_uint256(2u32, 1u32),
                )
                .with_parent(2),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    5,
                    NodeType::leaf_uint256(3u32, 1u32),
                )
                .with_parent(2),
                // Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    3,
                    NodeType::internal_uint256(2u32, (2u32, 4u32)),
                )
                .with_children(Some(6), Some(7))
                .with_parent(1),
                // Right-Left
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    6,
                    NodeType::leaf_uint256(4u32, 1u32),
                )
                .with_parent(3),
                // Right-Right
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    7,
                    NodeType::leaf_uint256(5u32, 1u32),
                )
                .with_parent(3),
            ],
            expected: vec![1, 2, 4, 5, 3, 6, 7],
            expected_error: None,
            print: true,
        },
        RebalanceTestCase {
            name: "invalid node type",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::leaf_uint256(1u32, 1u32),
                ),
            ],
            expected: vec![],
            expected_error: Some(ContractError::InvalidNodeType),
            print: true,
        },
        RebalanceTestCase {
            name: "childless internal node",
            nodes: vec![
                // Root
                TreeNode::new(
                    
                    tick_id,
                    direction,
                    1,
                    NodeType::internal_uint256(1u32, (1u32, 2u32)),
                ),
            ],
            expected: vec![],
            expected_error: Some(ContractError::ChildlessInternalNode),
            print: true,
        },
    ];

    for mut test in test_cases {
        let mut deps = mock_dependencies();
        // Save nodes in storage
        for (idx, node) in test.nodes.iter_mut().enumerate() {
            // Save root node
            if idx == 0 {
                TREE.save(
                    deps.as_mut().storage,
                    &( tick_id, &direction.to_string()),
                    &node.key,
                )
                .unwrap();
            }
            NODES
                .save(deps.as_mut().storage, &( tick_id, node.key), node)
                .unwrap();
        }

        // Sync weights post node storage
        for mut node in test.nodes {
            if node.is_internal() || node.parent.is_none() {
                continue;
            }
            node.sync(deps.as_ref().storage).unwrap();
            let mut parent = node.get_parent(deps.as_ref().storage).unwrap().unwrap();
            parent
                .sync_range_and_value_up(deps.as_mut().storage)
                .unwrap();
        }

        let mut tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Pre-rotation", test.name, &tree, &deps.as_ref());
        }

        let res = tree.rebalance(deps.as_mut().storage);

        if let Some(err) = test.expected_error {
            assert_eq!(res, Err(err), "{}", test.name);
            continue;
        }

        // Get new root node, it may have changed due to rotations
        let tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        if test.print {
            print_tree("Post-rotation", test.name, &tree, &deps.as_ref());
        }

        let nodes = tree.traverse(deps.as_ref().storage).unwrap();
        let res: Vec<u64> = nodes.iter().map(|n| n.key).collect();
        assert_eq!(res, test.expected, "{}", test.name);

        let internals = nodes.iter().filter(|n| n.is_internal()).collect();
        assert_internal_values(test.name, deps.as_ref(), internals, true);
    }
}

fn generate_nodes(
    storage: &mut dyn Storage,
    tick_id: i64,
    direction: OrderDirection,
    quantity: u32,
) -> Vec<TreeNode> {
    use rand::rngs::StdRng;
    use rand::{seq::SliceRandom, SeedableRng};

    let mut range: Vec<u32> = (0..quantity).collect();
    let seed = [0u8; 32]; // A fixed seed for deterministic randomness
    let mut rng = StdRng::from_seed(seed);
    range.shuffle(&mut rng);

    let mut nodes = vec![];
    for val in range {
        let id = generate_node_id(storage,  tick_id).unwrap();
        nodes.push(TreeNode::new(
            tick_id,
            direction,
            id,
            NodeType::leaf_uint256(val, 1u32),
        ));
    }
    nodes
}

#[test]
fn test_node_insert_large_quantity() {
    
    let tick_id = 1;
    let direction = OrderDirection::Bid;

    let mut deps = mock_dependencies();
    // Create tree root
    let mut tree = TreeNode::new(
        
        tick_id,
        direction,
        generate_node_id(deps.as_mut().storage,  tick_id).unwrap(),
        NodeType::internal_uint256(0u32, (u32::MAX, u32::MIN)),
    );

    TREE.save(
        deps.as_mut().storage,
        &( tick_id, &direction.to_string()),
        &tree.key,
    )
    .unwrap();

    let nodes = generate_nodes(deps.as_mut().storage, tick_id, direction, 1000);

    let target_etas = Decimal256::from_ratio(536u128, 1u128);
    let mut expected_prefix_sum = Decimal256::zero();

    // Insert nodes into tree
    for mut node in nodes {
        NODES
            .save(deps.as_mut().storage, &( tick_id, node.key), &node)
            .unwrap();
        tree.insert(deps.as_mut().storage, &mut node).unwrap();
        tree = get_root_node(deps.as_ref().storage,  tick_id, direction).unwrap();
        // Track insertions that fall below our target ETAS
        if node.get_min_range() <= target_etas {
            expected_prefix_sum = expected_prefix_sum.checked_add(Decimal256::one()).unwrap();
        }
    }

    // Return tree in vector form from Depth First Search
    let result = tree.traverse(deps.as_ref().storage).unwrap();

    // Ensure all internal nodes are correctly summed and contain correct ranges
    let internals: Vec<&TreeNode> = result.iter().filter(|x| x.is_internal()).collect();
    assert_internal_values("Large amount of nodes", deps.as_ref(), internals, true);

    // Ensure prefix sum functions correctly
    let root_node = get_root_node(deps.as_mut().storage,  tick_id, direction).unwrap();

    let prefix_sum = get_prefix_sum(deps.as_mut().storage, root_node, target_etas).unwrap();
    assert_eq!(expected_prefix_sum, prefix_sum);
}

const SPACING: u32 = 2u32;
const RIGHT_CORNER: &str = "┐";
const LEFT_CORNER: &str = "┌";
const STRAIGHT: &str = "─";

pub fn spacing(len: u32) -> String {
    let mut s = "".to_string();
    for _ in 0..len {
        s.push(' ');
    }
    s
}

pub fn print_tree(title: &'static str, test_name: &'static str, root: &TreeNode, deps: &Deps) {
    println!("{title}: {test_name}");
    println!("--------------------------");
    let nodes = root.traverse_bfs(deps.storage).unwrap();
    for (idx, row) in nodes.iter().enumerate() {
        print_tree_row(row.clone(), idx == 0, (nodes.len() - idx - 1) as u32);
    }
    println!();
}

pub fn print_tree_row(row: Vec<(Option<TreeNode>, Option<TreeNode>)>, top: bool, height: u32) {
    let blank_spacing_length = 2u32.pow(height + 1) * SPACING;
    let blank_spacing = spacing(blank_spacing_length);

    let mut node_spacing = "".to_string();
    for _ in 0..blank_spacing_length {
        node_spacing.push_str(STRAIGHT);
    }

    if !top {
        let mut line = "".to_string();
        for (left, right) in row.clone() {
            let print_left_top = if left.is_some() {
                format!("{blank_spacing}{LEFT_CORNER}{node_spacing}")
            } else {
                spacing(blank_spacing_length * 2)
            };
            let print_right_top = if right.is_some() {
                format!("{node_spacing}{RIGHT_CORNER}{blank_spacing}")
            } else {
                spacing(blank_spacing_length * 2)
            };
            line.push_str(format!("{print_left_top}{print_right_top}").as_str())
        }
        println!("{line}")
    }

    let mut line = "".to_string();
    for (left, right) in row {
        let left_node_length = if let Some(left) = left.clone() {
            left.to_string().len()
        } else {
            0
        };
        let print_left_top = if let Some(left) = left {
            // Shift spacing to adjust for length of node string
            let left_space =
                spacing(blank_spacing_length - (left_node_length as f32 / 2.0).ceil() as u32);

            format!("{left_space}{left}{blank_spacing}")
        } else {
            spacing(blank_spacing_length * 2)
        };
        let right_node_length = if let Some(right) = right.clone() {
            right.to_string().len()
        } else {
            0
        };
        let print_right_top = if let Some(right) = right {
            // Shift spacing to adjust for length of left and right node string
            let right_space = spacing(
                blank_spacing_length
                    - (right_node_length as f32 / 2.0).ceil() as u32
                    - (left_node_length as f32 / 2.0).floor() as u32,
            );
            format!("{right_space}{right}{blank_spacing}")
        } else if !top {
            spacing(blank_spacing_length * 2)
        } else {
            // Prevents root from going on to new line
            "".to_string()
        };
        line.push_str(format!("{print_left_top}{print_right_top}").as_str())
    }
    println!("{line}")
}
