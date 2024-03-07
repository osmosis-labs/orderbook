use cosmwasm_std::{testing::mock_dependencies, Storage, Uint128};

use crate::sumtree::node::{NodeType, TreeNode, NODES};

pub fn print_tree(storage: &dyn Storage, node: &TreeNode, depth: u8, top: bool) {
    let padding = "      ";
    let mut pre_padding = "".to_string();
    let mut vertical_padding = "".to_string();
    if depth >= 1 {
        for _ in 0..depth {
            pre_padding.push_str(padding);
            vertical_padding.push_str(padding);
        }
        pre_padding.push_str("+─────[");
    }
    if let Some(right) = node.get_right(storage).unwrap() {
        print_tree(storage, &right, depth + 1, true);
    } else if !top {
        println!("{vertical_padding}|");
    } else {
        println!();
    }
    println!("{pre_padding}{node}");
    if let Some(left) = node.get_left(storage).unwrap() {
        print_tree(storage, &left, depth + 1, false);
    } else if top {
        println!("{vertical_padding}|");
    } else {
        println!();
    }
}

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
        TestNodeInsertCase {
            name: "Case: Left Empty Right Empty",
            nodes: vec![NodeType::leaf(1u32, 10u32)],
            expected: vec![1, 2],
            print: true,
        },
        TestNodeInsertCase {
            name: "Case: Left Leaf, Right Empty",
            nodes: vec![NodeType::leaf(1u32, 10u32), NodeType::leaf(12u32, 10u32)],
            expected: vec![1, 2, 3],
            print: true,
        },
        TestNodeInsertCase {
            name: "Case: Left Leaf, Right Leaf, Left Insert",
            nodes: vec![
                NodeType::leaf(1u32, 10u32),
                NodeType::leaf(20u32, 10u32),
                NodeType::leaf(12u32, 8u32),
            ],
            expected: vec![1, 5, 2, 4, 3],
            print: true,
        },
        TestNodeInsertCase {
            name: "Case: Left Internal, Right Leaf, Right Insert",
            nodes: vec![
                NodeType::leaf(1u32, 10u32),
                NodeType::leaf(20u32, 10u32),
                NodeType::leaf(12u32, 8u32),
                NodeType::leaf(30u32, 8u32),
            ],
            expected: vec![1, 5, 2, 4, 7, 3, 6],
            print: true,
        },
        TestNodeInsertCase {
            name: "Case: Left Internal, Right Internal, Left Insert",
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
        TestNodeInsertCase {
            name: "Case: Left Internal, Right Internal, Right Insert",
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
    ];

    for test in test_cases {
        let mut deps = mock_dependencies();
        let mut tree = TreeNode::new(
            book_id,
            tick_id,
            generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap(),
            NodeType::internal(Uint128::zero(), (u32::MAX, u32::MIN)),
        );

        for node in test.nodes {
            let mut tree_node = TreeNode::new(
                book_id,
                tick_id,
                generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap(),
                node,
            );
            NODES
                .save(
                    deps.as_mut().storage,
                    &(book_id, tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();
        }

        if test.print {
            println!("Print Tree: {}", test.name);
            println!("--------------------------");
            print_tree(deps.as_ref().storage, &tree, 0, true);
            println!();
        }

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

        let internals: Vec<&TreeNode> = result.iter().filter(|x| x.is_internal()).collect();
        for internal_node in internals {
            let left_node = internal_node.get_left(deps.as_ref().storage).unwrap();
            let right_node = internal_node.get_right(deps.as_ref().storage).unwrap();

            let accumulated_value = left_node
                .clone()
                .map(|x| x.get_value())
                .unwrap_or_default()
                .checked_add(
                    right_node
                        .clone()
                        .map(|x| x.get_value())
                        .unwrap_or_default(),
                )
                .unwrap();
            assert_eq!(internal_node.get_value(), accumulated_value);

            let min = left_node
                .clone()
                .map(|n| n.get_min_range())
                .unwrap_or(Uint128::MAX)
                .min(
                    right_node
                        .clone()
                        .map(|n| n.get_min_range())
                        .unwrap_or(Uint128::MAX),
                );
            let max = left_node
                .map(|n| n.get_max_range())
                .unwrap_or(Uint128::MIN)
                .max(
                    right_node
                        .map(|n| n.get_max_range())
                        .unwrap_or(Uint128::MIN),
                );
            assert_eq!(internal_node.get_min_range(), min);
            assert_eq!(internal_node.get_max_range(), max);
        }
    }
}
