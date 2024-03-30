use crate::sumtree::node::{generate_node_id, NodeType, TreeNode, NODES};
use crate::sumtree::test::test_node::print_tree;
use crate::sumtree::tree::get_prefix_sum;
use cosmwasm_std::{testing::mock_dependencies, Decimal256};

struct TestPrefixSumCase {
    name: &'static str,
    nodes: Vec<NodeType>,
    target_etas: Decimal256,
    print: bool,
    expected_sum: Decimal256,
}

#[test]
fn test_get_prefix_sum_valid() {
    let book_id = 1;
    let tick_id = 1;
    let test_cases: Vec<TestPrefixSumCase> = vec![
        TestPrefixSumCase {
            name: "Single node, target ETAS equal to node ETAS",
            nodes: vec![NodeType::leaf_uint256(10u128, 5u128)],
            target_etas: Decimal256::from_ratio(10u128, 1u128),
            print: true,
            expected_sum: Decimal256::from_ratio(5u128, 1u128),
        },
        TestPrefixSumCase {
            name: "Multiple nodes, target ETAS in the middle",
            nodes: vec![
                NodeType::leaf_uint256(5u128, 10u128),
                NodeType::leaf_uint256(15u128, 20u128),
                NodeType::leaf_uint256(35u128, 30u128),
            ],
            target_etas: Decimal256::from_ratio(20u128, 1u128),
            print: true,
            expected_sum: Decimal256::from_ratio(30u128, 1u128),
        },
        // TestPrefixSumCase {
        //     name: "Target ETAS below all nodes",
        //     nodes: vec![
        //         TestNode {
        //             etas: Decimal256::from_ratio(10u128, 1u128),
        //             amount: Decimal256::from_ratio(10u128, 1u128),
        //         },
        //         TestNode {
        //             etas: Decimal256::from_ratio(20u128, 1u128),
        //             amount: Decimal256::from_ratio(20u128, 1u128),
        //         },
        //         TestNode {
        //             etas: Decimal256::from_ratio(30u128, 1u128),
        //             amount: Decimal256::from_ratio(30u128, 1u128),
        //         },
        //     ],
        //     target_etas: Decimal256::from_ratio(5u128, 1u128),
        //     expected_sum: Decimal256::zero(),
        // },
        // TestPrefixSumCase {
        //     name: "Target ETAS above all nodes",
        //     nodes: vec![
        //         TestNode {
        //             etas: Decimal256::from_ratio(10u128, 1u128),
        //             amount: Decimal256::from_ratio(10u128, 1u128),
        //         },
        //         TestNode {
        //             etas: Decimal256::from_ratio(20u128, 1u128),
        //             amount: Decimal256::from_ratio(20u128, 1u128),
        //         },
        //         TestNode {
        //             etas: Decimal256::from_ratio(30u128, 1u128),
        //             amount: Decimal256::from_ratio(30u128, 1u128),
        //         },
        //     ],
        //     target_etas: Decimal256::from_ratio(35u128, 1u128),
        //     expected_sum: Decimal256::from_ratio(60u128, 1u128), // Sum of all nodes
        // },
    ];

    for test in test_cases {
        println!("\n--------------------------------");
        println!("Running test: {}", test.name);
        println!("\n--------------------------------");
        let mut deps = mock_dependencies();

        let root_id = generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap();
        let mut tree = TreeNode::new(book_id, tick_id, root_id, NodeType::default());
        NODES
            .save(deps.as_mut().storage, &(book_id, tick_id, tree.key), &tree)
            .unwrap();

        // Insert nodes into tree
        for (idx, node) in test.nodes.iter().enumerate() {
            let new_node_id = generate_node_id(deps.as_mut().storage, book_id, tick_id).unwrap();
            println!("New node ID: {}", new_node_id);
            let mut tree_node = TreeNode::new(book_id, tick_id, new_node_id, node.clone());
            NODES
                .save(
                    deps.as_mut().storage,
                    &(book_id, tick_id, tree_node.key),
                    &tree_node,
                )
                .unwrap();

            // Why does it seem like insertions are overwriting each other?
            println!("Inserting node");
            tree.insert(deps.as_mut().storage, &mut tree_node).unwrap();

            if idx >= 2 {
                print_tree("Inserted node:", test.name, &tree, &deps.as_ref());
            }
        }

        let root_node = NODES
            .load(deps.as_mut().storage, &(book_id, tick_id, root_id))
            .unwrap();

        if test.print {
            print_tree("Final tree:", test.name, &root_node, &deps.as_ref());
        }

        let prefix_sum =
            get_prefix_sum(deps.as_ref().storage, root_node, test.target_etas).unwrap();

        assert_eq!(
            test.expected_sum, prefix_sum,
            "{}: Expected prefix sum {}, got {}",
            test.name, test.expected_sum, prefix_sum
        );
    }
}
