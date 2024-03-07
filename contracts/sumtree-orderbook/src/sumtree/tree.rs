use cw_storage_plus::Map;

use super::node::TreeNode;

pub const TREE: Map<&(u64, i64), TreeNode> = Map::new("tree");
