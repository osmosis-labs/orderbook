use cosmwasm_std::Storage;
use cw_storage_plus::Map;

use crate::error::ContractResult;

use super::node::{TreeNode, NODES};

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
