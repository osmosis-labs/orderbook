use cw_storage_plus::Map;

use super::node::TreeNode;

// TODO: REMOVE
#[allow(dead_code)]
pub const TREE: Map<&(u64, i64), TreeNode> = Map::new("tree");
