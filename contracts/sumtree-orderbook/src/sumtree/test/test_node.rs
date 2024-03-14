use cosmwasm_std::{testing::mock_dependencies, Uint128};

use crate::sumtree::node::{generate_node_id, NodeType, TreeNode, NODES};

struct TestNodeInsertCase {
    name: &'static str,
    nodes: Vec<NodeType>,
    // Depth first search ordering of node IDs (Could be improved?)
    expected: Vec<u64>,
    // Whether to print the tree
    print: bool,
}

#[test]
fn test_node_insert_valid() {
    let book_id = 1;
    let tick_id = 1;
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
            name: "Case 1a: Left Internal, Right Internal, Left Insert",
            nodes: vec![
                NodeType::leaf(1u32, 5u32),
                NodeType::leaf(20u32, 10u32),
                NodeType::leaf(12u32, 8u32),
                NodeType::leaf(30u32, 8u32),
                NodeType::leaf(6u32, 6u32),
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
            name: "Case 1b: Left Internal, Right Internal, Right Insert",
            nodes: vec![
                NodeType::leaf(1u32, 11u32),
                NodeType::leaf(20u32, 5u32),
                NodeType::leaf(12u32, 8u32),
                NodeType::leaf(30u32, 8u32),
                NodeType::leaf(25u32, 5u32),
            ],
            expected: vec![1, 5, 2, 4, 7, 9, 3, 8, 6],
            print: true,
        },
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
            name: "Case 2: First Node Insert",
            nodes: vec![NodeType::leaf(1u32, 10u32)],
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
            name: "Case 3: Left Leaf, Right Empty",
            nodes: vec![NodeType::leaf(1u32, 10u32), NodeType::leaf(12u32, 10u32)],
            expected: vec![1, 2, 3],
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
            name: "Case 3: Left Leaf, Right Empty, Larger Order First",
            nodes: vec![NodeType::leaf(12u32, 10u32), NodeType::leaf(1u32, 10u32)],
            expected: vec![1, 3, 2],
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
            name: "Case 4: Left Leaf, Right Leaf, Left Insert",
            nodes: vec![
                NodeType::leaf(1u32, 10u32),
                NodeType::leaf(20u32, 10u32),
                NodeType::leaf(12u32, 8u32),
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
            name: "Case 5: Left Internal, Right Leaf, Right Insert",
            nodes: vec![
                NodeType::leaf(1u32, 10u32),
                NodeType::leaf(20u32, 10u32),
                NodeType::leaf(12u32, 8u32),
                NodeType::leaf(30u32, 8u32),
            ],
            expected: vec![1, 5, 2, 4, 7, 3, 6],
            print: true,
        },
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();
        // Create tree root
        let mut tree = TreeNode::new(
            book_id,
            tick_id,
            generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap(),
            NodeType::internal(Uint128::zero(), (u32::MAX, u32::MIN)),
        );

        // Insert nodes into tree
        for (idx, node) in test.nodes.iter().enumerate() {
            let mut tree_node = TreeNode::new(
                book_id,
                tick_id,
                generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap(),
                node.clone(),
            );
            NODES
                .save(
                    deps.as_mut().storage,
                    &(book_id, tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();

            // Print tree at second last node to see pre-insert
            if test.nodes.len() >= 2 && idx == test.nodes.len() - 2 && test.print {
                println!("Pre Insert Tree: {}", test.name);
                println!("--------------------------");

                let nodes = tree.traverse_bfs(deps.as_ref().storage).unwrap();
                for (idx, row) in nodes.iter().enumerate() {
                    print_tree_row(row.clone(), idx == 0, (nodes.len() - idx - 1) as u32);
                }
                println!();
            }
        }

        if test.print {
            println!("Post Insert Tree: {}", test.name);
            println!("--------------------------");

            let nodes = tree.traverse_bfs(deps.as_ref().storage).unwrap();
            for (idx, row) in nodes.iter().enumerate() {
                print_tree_row(row.clone(), idx == 0, (nodes.len() - idx - 1) as u32);
            }
            println!();
        }

        // Return tree in vector form from Depth First Search
        let result = tree.traverse(deps.as_ref().storage).unwrap();

        assert_eq!(
            result,
            test.expected
                .iter()
                .map(|key| NODES
                    .load(deps.as_ref().storage, &(book_id, tick_id, *key))
                    .unwrap())
                .collect::<Vec<TreeNode>>()
        );

        // Ensure all internal nodes are correctly summed and contain correct ranges
        let internals: Vec<&TreeNode> = result.iter().filter(|x| x.is_internal()).collect();
        for internal_node in internals {
            let left_node = internal_node.get_left(deps.as_ref().storage).unwrap();
            let right_node = internal_node.get_right(deps.as_ref().storage).unwrap();

            let accumulated_value = left_node
                .clone()
                .map_or(Uint128::zero(), |x| x.get_value())
                .checked_add(
                    right_node
                        .clone()
                        .map_or(Uint128::zero(), |x| x.get_value()),
                )
                .unwrap();
            assert_eq!(internal_node.get_value(), accumulated_value);

            let min = left_node
                .clone()
                .map_or(Uint128::MAX, |n| n.get_min_range())
                .min(
                    right_node
                        .clone()
                        .map_or(Uint128::MAX, |n| n.get_min_range()),
                );
            let max = left_node
                .map_or(Uint128::MIN, |n| n.get_max_range())
                .max(right_node.map_or(Uint128::MIN, |n| n.get_max_range()));
            assert_eq!(internal_node.get_min_range(), min);
            assert_eq!(internal_node.get_max_range(), max);
        }
    }
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