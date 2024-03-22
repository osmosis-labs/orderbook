use super::node::{TreeNode, NODES};
use crate::error::ContractResult;
use cosmwasm_std::{Storage, Uint128};
use cw_storage_plus::Map;

pub const TREE: Map<&(u64, i64), u64> = Map::new("tree");

#[allow(dead_code)]
/// Retrieves the root node of a specific book and tick from storage.
pub fn get_root_node(
    storage: &dyn Storage,
    book_id: u64,
    tick_id: i64,
) -> ContractResult<TreeNode> {
    let root_id = TREE.load(storage, &(book_id, tick_id))?;
    Ok(NODES.load(storage, &(book_id, tick_id, root_id))?)
}

/// Calculates the prefix sum of values in the sumtree up to a target ETAS.
pub fn get_prefix_sum(
    storage: &dyn Storage,
    book_id: u64,
    tick_id: i64,
    target_etas: Uint128,
) -> ContractResult<Uint128> {
    let root_node = get_root_node(storage, book_id, tick_id)?;

    // 1. Start from the root node and store its sum. This is the sum of the values of all the leaves.
    let mut prefix_sum = TreeNode::get_value(&root_node);

    // The logic below could be abstracted into a recursive helper that takes in a starting sum.

    // 2. Check the upper end of the left child's boundary. If this is <= the input ETAS, walk left. Otherwise, walk right.
    let left_child = root_node.get_left(storage)?;
    let right_child = root_node.get_right(storage)?;

    // Determine which child to walk to next.
    // Attempt to walk left. If it's none, check if right node exists. If not, exit early. If so, check its lower bound: if <= input ETAS, flag to walk right.
    // If it exists, check its upper bound. If it's <= the input ETAS, flag to walk left. Otherwise, flag to walk right.

    // 3. Whenever you are about to walk left, subtract the stored sum of your right child from your current sum. This is because at this stage, everything below the right child is at an ETAS > your input, so it should not be part of the prefix sum.
    // 4. Repeat above until you either reach a node where the highest value is at an ETAS <= your input ETAS

    Ok(prefix_sum)
}
