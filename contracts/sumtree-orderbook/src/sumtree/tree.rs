use super::node::{generate_node_id, NodeType, TreeNode, NODES};
use crate::{error::ContractResult, types::OrderDirection};
use cosmwasm_std::{Decimal256, Storage};
use cw_storage_plus::Map;

// Key: (tick_id, direction as str)
pub const TREE: Map<&(i64, &str), u64> = Map::new("tree");

#[allow(dead_code)]
/// Retrieves the root node of a specific book and tick from storage.
pub fn get_root_node(
    storage: &dyn Storage,
    tick_id: i64,
    direction: OrderDirection,
) -> ContractResult<TreeNode> {
    let root_id = TREE.load(storage, &(tick_id, &direction.to_string()))?;
    Ok(NODES.load(storage, &(tick_id, root_id))?)
}

#[allow(dead_code)]
/// Retrieves the root node of a specific book and tick from storage.
/// If it is not available, initializes a sumtree and returns the root.
pub fn get_or_init_root_node(
    storage: &mut dyn Storage,
    tick_id: i64,
    direction: OrderDirection,
) -> ContractResult<TreeNode> {
    let tree = if let Ok(tree) = get_root_node(storage, tick_id, direction) {
        tree
    } else {
        let new_root = TreeNode::new(
            tick_id,
            direction,
            generate_node_id(storage, tick_id)?,
            NodeType::default(),
        );
        TREE.save(storage, &(tick_id, &direction.to_string()), &new_root.key)?;
        new_root
    };
    Ok(tree)
}

#[allow(dead_code)]
/// Calculates the prefix sum of values in the sumtree up to a target ETAS.
pub fn get_prefix_sum(
    storage: &dyn Storage,
    root_node: TreeNode,
    target_etas: Decimal256,
    prev_sum: Decimal256,
) -> ContractResult<Decimal256> {
    // We start from the root node's sum, which includes everything in the tree.
    // The prefix sum algorithm will chip away at this until we have the correct
    // prefux sum in O(log(N)) time.
    let starting_sum = TreeNode::get_value(&root_node);

    prefix_sum_walk(storage, &root_node, starting_sum, target_etas, prev_sum)
}

// prefix_sum_walk is a recursive function that walks the sumtree to calculate the prefix sum below the given
// target ETAS. Once called on the root node of a tree, this function walks down the tree while tracking a
// running prefix sum that starts from the maximum possible value (all nodes in the tree) and chips down as
// appropriate.
//
// Since the longest path this function can walk is from the root to a leaf, it runs in O(log(N)) time. Given
// how it is able to terminate early using our sumtree's range properties, in many cases it will likely run
// in much less.

fn prefix_sum_walk(
    storage: &dyn Storage,
    node: &TreeNode,
    mut current_sum: Decimal256,
    target_etas: Decimal256,
    prev_sum: Decimal256,
) -> ContractResult<Decimal256> {
    // Sanity check: target ETAS should be inside node's range.
    if target_etas < node.get_min_range() {
        // If the target ETAS is below the root node's range, we can return zero early.
        return Ok(Decimal256::zero());
    } else if target_etas >= node.get_max_range() {
        return Ok(current_sum);
    }

    // If node is a leaf, we just return its full ETAS value. This is because by this point we
    // know the target ETAS is in the node's range, and if the target ETAS is in the range of a
    // leaf, we count the full leaf towards the prefix sum.
    //
    // Recall that the prefix sum is the sum of all the values of all leaves that have a _starting_
    // ETAS below the target ETAS.
    if !node.is_internal() {
        return Ok(current_sum);
    }

    // We fetch both children here since we need to access both regardless of
    // whether we walk left or right.
    let left_child = node.get_left(storage)?;
    let right_child = node.get_right(storage)?;

    // -- Resync Condition --

    // To prevent requiring a resync there needs to be a condition that covers the case that
    // when realizing the left node the new ETAS is enough to realize the right node (to some extent)
    // To cover this we can determine how much of the left node has been realized, using this we can then determine
    // if realizing what is unrealized from the left node will result in a new ETAS that is enough to realize the
    // right node (to some extent)
    if left_child.is_some() && right_child.is_some() {
        let left_child = left_child.clone().unwrap();
        let right_child = right_child.clone().unwrap();

        // Calculate what the sum is before realizing the current node
        let sum_at_node = current_sum.checked_sub(node.get_value())?;
        // Calculate how much of the node has been realized in a previous sync
        let diff_at_node = prev_sum.saturating_sub(sum_at_node);
        // Calculate how much of the left node is unrealized
        let unrealized_from_left = left_child.get_value().saturating_sub(diff_at_node);
        // Calculate the new ETAS after realizing what is unrealized from the left node
        let new_etas = target_etas.checked_add(unrealized_from_left)?;

        // if the new ETAS is greater than or equal to the right child's min range, we can walk right
        // as the left node MUST be realizable given the invariants of the sumtree mechanism
        if new_etas >= right_child.get_min_range() {
            return prefix_sum_walk(storage, &right_child, current_sum, new_etas, prev_sum);
        }
    }

    // --- Attempt walk left ---

    // If the left child exists, we run the following logic:
    // * If target ETAS < left child's lower bound, exit early with zero
    // * Else if target ETAS <= upper bound, subtract right child sum from prefix sum and walk left
    //
    // If neither of the above conditions are met, we continue to logic around walking right.
    if let Some(left_child) = left_child {
        if target_etas < left_child.get_min_range() {
            // If the target ETAS is below the left child's range, nothing in the
            // entire tree should be included in the prefix sum, so we return zero.
            //
            // TODO: This should not be possible now that the check above is added.
            // Consider removing or erroring here.
            return Ok(Decimal256::zero());
        }

        if target_etas < left_child.get_max_range() {
            // Since the target ETAS is within the left child's range, we can safely conclude
            // that everything below the right child should not be in our prefix sum.
            let right_sum = right_child.map_or(Decimal256::zero(), |r| r.get_value());

            current_sum = current_sum.checked_sub(right_sum)?;

            // Walk left recursively
            current_sum =
                prefix_sum_walk(storage, &left_child, current_sum, target_etas, prev_sum)?;

            return Ok(current_sum);
        }
    }

    // --- Attempt walk right ---

    // If right child either doesn't exist, the current prefix sum is simply the sum of the left child,
    // which is fully below the target ETAS, so we return the prefix sum as is.
    if right_child.is_none() {
        return Ok(current_sum);
    }

    // In the case where right child exists and the target ETAS is above the left child, we run the following logic:
    // * If target ETAS < right child's lower bound: subtract right child's sum from prefix sum and return
    // * If target ETAS <= right child's upper bound: walk right
    // * If target ETAS > right child's upper bound: return full sum
    let right_child = right_child.unwrap();
    if target_etas < right_child.get_min_range() {
        // If the ETAS is below the right child's range, we know that anything below the right child
        // should not be included in the prefix sum. We subtract the right child's sum from the prefix sum.
        current_sum = current_sum.checked_sub(right_child.get_value())?;

        Ok(current_sum)
    } else if target_etas <= right_child.get_max_range() {
        // If the target ETAS falls in the right child's range, we need to walk right.
        // We do not need to update the prefix sum here because we do not know how much
        // to subtract from it yet. The right walk handles this update.

        // Walk right recursively
        current_sum = prefix_sum_walk(storage, &right_child, current_sum, target_etas, prev_sum)?;

        Ok(current_sum)
    } else {
        // If we reach here, everything in the tree is below the target ETAS, so we simply return the full sum.
        Ok(current_sum)
    }
}
