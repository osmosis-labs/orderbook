// TODO: Remove this
#![allow(dead_code)]
#[cfg(test)]
use core::fmt;
#[cfg(test)]
use std::fmt::Display;

use cosmwasm_schema::cw_serde;
#[cfg(test)]
use cosmwasm_std::Uint256;
use cosmwasm_std::{ensure, Decimal256, Storage};
use cw_storage_plus::Map;

use crate::{error::ContractResult, ContractError};

pub const NODES: Map<&(u64, i64, u64), TreeNode> = Map::new("nodes");
pub const NODE_ID_COUNTER: Map<&(u64, i64), u64> = Map::new("node_id");

pub fn generate_node_id(
    storage: &mut dyn Storage,
    book_id: u64,
    tick_id: i64,
) -> ContractResult<u64> {
    let mut counter = NODE_ID_COUNTER
        .may_load(storage, &(book_id, tick_id))?
        .unwrap_or_default();
    counter += 1;
    NODE_ID_COUNTER.save(storage, &(book_id, tick_id), &counter)?;
    Ok(counter)
}

#[cw_serde]
pub enum NodeType {
    Leaf {
        // Amount cancelled from order
        value: Decimal256,
        // Effective total amount sold
        etas: Decimal256,
    },
    Internal {
        // Sum of all values below current
        accumulator: Decimal256,
        // Range from min ETAS to max ETAS + value of max ETAS
        range: (Decimal256, Decimal256),
    },
}

impl NodeType {
    pub fn leaf(etas: Decimal256, value: Decimal256) -> Self {
        Self::Leaf { etas, value }
    }

    /// Utility function to help with testing
    ///
    /// Decimal256 does not allow conversion from primitives but Uint256 does, this makes writing tests simpler
    #[cfg(test)]
    pub fn leaf_uint256(etas: impl Into<Uint256>, value: impl Into<Uint256>) -> Self {
        Self::Leaf {
            etas: Decimal256::from_ratio(etas, Uint256::one()),
            value: Decimal256::from_ratio(value, Uint256::one()),
        }
    }

    pub fn internal(
        accumulator: impl Into<Decimal256>,
        range: (impl Into<Decimal256>, impl Into<Decimal256>),
    ) -> Self {
        Self::Internal {
            range: (range.0.into(), range.1.into()),
            accumulator: accumulator.into(),
        }
    }

    /// Utility function to help with testing
    ///
    /// Decimal256 does not allow conversion from primitives but Uint256 does, this makes writing tests simpler
    #[cfg(test)]
    pub fn internal_uint256(
        accumulator: impl Into<Uint256>,
        range: (impl Into<Uint256>, impl Into<Uint256>),
    ) -> Self {
        Self::Internal {
            range: (
                Decimal256::from_ratio(range.0.into(), Uint256::one()),
                Decimal256::from_ratio(range.1.into(), Uint256::one()),
            ),
            accumulator: Decimal256::from_ratio(accumulator, Uint256::one()),
        }
    }
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Internal {
            accumulator: Decimal256::zero(),
            range: (Decimal256::MAX, Decimal256::MIN),
        }
    }
}

#[cw_serde]
pub struct TreeNode {
    pub key: u64,
    pub book_id: u64,
    pub tick_id: i64,
    pub left: Option<u64>,
    pub right: Option<u64>,
    pub parent: Option<u64>,
    pub node_type: NodeType,
}

#[cfg(test)]
pub type BFSVec = Vec<Vec<(Option<TreeNode>, Option<TreeNode>)>>;

impl TreeNode {
    pub fn new(book_id: u64, tick_id: i64, key: u64, node_type: NodeType) -> Self {
        Self {
            key,
            book_id,
            tick_id,
            left: None,
            right: None,
            parent: None,
            node_type,
        }
    }

    pub fn is_internal(&self) -> bool {
        matches!(self.node_type, NodeType::Internal { .. })
    }

    pub fn get_right(&self, storage: &dyn Storage) -> ContractResult<Option<TreeNode>> {
        if let Some(right) = self.right {
            Ok(NODES.may_load(storage, &(self.book_id, self.tick_id, right))?)
        } else {
            Ok(None)
        }
    }

    pub fn get_left(&self, storage: &dyn Storage) -> ContractResult<Option<TreeNode>> {
        if let Some(left) = self.left {
            Ok(NODES.may_load(storage, &(self.book_id, self.tick_id, left))?)
        } else {
            Ok(None)
        }
    }

    pub fn get_parent(&self, storage: &dyn Storage) -> ContractResult<Option<TreeNode>> {
        if let Some(parent) = self.parent {
            Ok(NODES.may_load(storage, &(self.book_id, self.tick_id, parent))?)
        } else {
            Ok(None)
        }
    }

    pub fn has_child(&self) -> bool {
        self.left.is_some() || self.right.is_some()
    }

    pub fn save(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Ok(NODES.save(storage, &(self.book_id, self.tick_id, self.key), self)?)
    }

    /// Returns the maximum range value of a node.
    ///
    /// For `Internal` nodes, this is the maximum value of the associated range.
    /// For `Leaf` nodes, this is the sum of the `value` and `etas` fields.
    pub fn get_max_range(&self) -> Decimal256 {
        match self.node_type {
            NodeType::Internal { range, .. } => range.1,
            NodeType::Leaf { value, etas } => value.checked_add(etas).unwrap(),
        }
    }

    pub fn set_max_range(&mut self, new_max: Decimal256) -> ContractResult<()> {
        match &mut self.node_type {
            NodeType::Leaf { .. } => Err(ContractError::InvalidNodeType),
            NodeType::Internal { range, .. } => {
                range.1 = new_max;
                Ok(())
            }
        }
    }

    /// Returns the minimum value of a node.
    ///
    /// For internal nodes, this is the minimum value of the associated range.
    /// For leaf nodes, this is the value.
    pub fn get_min_range(&self) -> Decimal256 {
        match self.node_type {
            NodeType::Internal { range, .. } => range.0,
            NodeType::Leaf { etas, .. } => etas,
        }
    }

    pub fn set_min_range(&mut self, new_min: Decimal256) -> ContractResult<()> {
        match &mut self.node_type {
            NodeType::Leaf { .. } => Err(ContractError::InvalidNodeType),
            NodeType::Internal { range, .. } => {
                range.0 = new_min;
                Ok(())
            }
        }
    }

    pub fn set_value(&mut self, value: Decimal256) -> ContractResult<()> {
        match &mut self.node_type {
            NodeType::Internal { accumulator, .. } => {
                *accumulator = value;
                Ok(())
            }
            NodeType::Leaf { .. } => Err(ContractError::InvalidNodeType),
        }
    }

    /// Adds a given value to an internal node's accumulator
    ///
    /// Errors if given node is not internal
    pub fn add_value(&mut self, value: Decimal256) -> ContractResult<()> {
        self.set_value(self.get_value().checked_add(value)?)
    }

    /// Recalculates the range and accumulated value for a node and propagates it up the tree
    ///
    /// Must be an internal node
    pub fn sync_range_and_value(&mut self, storage: &mut dyn Storage) -> ContractResult<()> {
        ensure!(self.is_internal(), ContractError::InvalidNodeType);
        let maybe_left = self.get_left(storage)?;
        let maybe_right = self.get_right(storage)?;

        let left_exists = maybe_left.is_some();
        let right_exists = maybe_right.is_some();

        if !self.has_child() {
            return Err(ContractError::ChildlessInternalNode);
        }

        let (min, max) = if left_exists && !right_exists {
            let left = maybe_left.clone().unwrap();
            (left.get_min_range(), left.get_max_range())
        } else if right_exists && !left_exists {
            let right = maybe_right.clone().unwrap();
            (right.get_min_range(), right.get_max_range())
        } else {
            let left = maybe_left.clone().unwrap();
            let right = maybe_right.clone().unwrap();

            (
                left.get_min_range().min(right.get_min_range()),
                left.get_max_range().max(right.get_max_range()),
            )
        };

        self.set_min_range(min)?;
        self.set_max_range(max)?;

        let value = maybe_left
            .map(|n| n.get_value())
            .unwrap_or_default()
            .checked_add(maybe_right.map(|n| n.get_value()).unwrap_or_default())?;
        self.set_value(value)?;

        // Must save before propagating as parent will read this node
        self.save(storage)?;

        if let Some(mut parent) = self.get_parent(storage)? {
            parent.sync_range_and_value(storage)?;
        }

        Ok(())
    }

    /// Gets the value for a given node.
    ///
    /// For `Leaf` nodes this is the `value`.
    ///
    /// For `Internal` nodes this is the `accumulator`.
    pub fn get_value(&self) -> Decimal256 {
        match self.node_type {
            NodeType::Leaf { value, .. } => value,
            NodeType::Internal { accumulator, .. } => accumulator,
        }
    }

    /// Inserts a given node in to the tree
    ///
    /// If the node is internal an error is returned.
    ///
    /// If the node is a leaf it will be inserted by the following priority:
    /// 1. New node fits in either left or right range, insert accordingly
    /// 2. Left is empty, insert left
    /// 3. Out of range for left, Right is empty, insert right
    /// 4. Left is leaf, split left
    /// 5. Left is out of range, right is leaf, split right
    pub fn insert(
        &mut self,
        storage: &mut dyn Storage,
        new_node: &mut TreeNode,
    ) -> ContractResult<()> {
        ensure!(self.is_internal(), ContractError::InvalidNodeType);
        ensure!(!new_node.is_internal(), ContractError::InvalidNodeType);

        // New node is being placed below current, update ranges and accumulator as tree is traversed
        self.add_value(new_node.get_value())?;
        if self.get_min_range() > new_node.get_min_range() {
            self.set_min_range(new_node.get_min_range())?;
        }

        if self.get_max_range() < new_node.get_max_range() {
            self.set_max_range(new_node.get_max_range())?;
        }

        let maybe_left = self.get_left(storage)?;
        let maybe_right = self.get_right(storage)?;

        let is_in_left_range = maybe_left.clone().map_or(false, |left| {
            left.is_internal() && new_node.get_min_range() < left.get_max_range()
        });
        let is_in_right_range = maybe_right.clone().map_or(false, |right| {
            right.is_internal() && new_node.get_min_range() > right.get_min_range()
        });

        // Case 1 Left
        if is_in_left_range {
            // Can unwrap as node must exist
            let mut left = maybe_left.unwrap();

            left.insert(storage, new_node)?;
            self.save(storage)?;
            return Ok(());
        }
        // Case 1 Right
        if is_in_right_range {
            // Can unwrap as node must exist
            let mut right = maybe_right.unwrap();
            right.insert(storage, new_node)?;
            self.save(storage)?;
            return Ok(());
        }

        // Case 2
        if maybe_left.is_none() {
            self.left = Some(new_node.key);
            new_node.parent = Some(self.key);
            new_node.save(storage)?;
            self.save(storage)?;
            return Ok(());
        }

        // Case 3: reordering
        let is_lower_than_left_leaf = maybe_left.clone().map_or(false, |l| {
            !l.is_internal() && new_node.get_max_range() <= l.get_min_range()
        });
        if is_lower_than_left_leaf && maybe_right.is_none() {
            self.right = self.left;
            self.left = Some(new_node.key);
            new_node.parent = Some(self.key);
            new_node.save(storage)?;
            self.save(storage)?;
            return Ok(());
        }

        // Case 3
        if !is_in_left_range && maybe_right.is_none() {
            self.right = Some(new_node.key);
            new_node.parent = Some(self.key);
            new_node.save(storage)?;
            self.save(storage)?;
            return Ok(());
        }

        let left_is_leaf = maybe_left.clone().map_or(false, |left| !left.is_internal());
        let right_is_leaf = maybe_right
            .clone()
            .map_or(false, |right| !right.is_internal());

        // Case 4
        if left_is_leaf {
            let mut left = maybe_left.unwrap();
            let new_left = left.split(storage, new_node)?;
            self.left = Some(new_left);
            self.save(storage)?;
            return Ok(());
        }

        // Case 5: Reordering
        // TODO: Add edge case test for this
        let is_higher_than_right_leaf = maybe_right.clone().map_or(false, |r| {
            !r.is_internal() && new_node.get_min_range() >= r.get_max_range()
        });
        if is_higher_than_right_leaf && maybe_left.is_none() {
            self.left = self.right;
            self.right = Some(new_node.key);
            new_node.parent = Some(self.key);
            new_node.save(storage)?;
            self.save(storage)?;
            return Ok(());
        }

        // Case 5
        if !is_in_left_range && right_is_leaf {
            let mut right = maybe_right.unwrap();
            let new_right = right.split(storage, new_node)?;
            self.right = Some(new_right);
            self.save(storage)?;
            return Ok(());
        }

        Ok(())
    }

    /// Splits a given node by generating a new parent internal node and assigning the current and new node as ordered children.
    /// Split nodes are ordered by ETAS in ascending order left to right.
    ///
    /// Returns an ID for the new parent node
    pub fn split(
        &mut self,
        storage: &mut dyn Storage,
        new_node: &mut TreeNode,
    ) -> ContractResult<u64> {
        ensure!(!self.is_internal(), ContractError::InvalidNodeType);
        let id = generate_node_id(storage, self.book_id, self.tick_id)?;
        let accumulator = self.get_value().checked_add(new_node.get_value())?;

        // Determine which node goes to which side, maintaining order by ETAS
        let (new_left, new_right) = if self.get_min_range() < new_node.get_min_range() {
            (self.key, new_node.key)
        } else {
            (new_node.key, self.key)
        };
        // Determine the new range for the generated parent
        let (new_min, new_max) = (
            new_node.get_min_range().min(self.get_min_range()),
            new_node.get_max_range().max(self.get_max_range()),
        );

        let mut new_parent = TreeNode::new(
            self.book_id,
            self.tick_id,
            id,
            NodeType::internal(accumulator, (new_min, new_max)),
        );

        // Save new key references
        new_parent.parent = self.parent;
        new_parent.left = Some(new_left);
        new_parent.right = Some(new_right);
        self.parent = Some(id);
        new_node.parent = Some(id);

        new_parent.save(storage)?;
        self.save(storage)?;
        new_node.save(storage)?;

        Ok(id)
    }

    /// Deletes a given node from the tree and propagates value changes up through its parent nodes.
    ///
    /// If the parent node has no children after removal it is also deleted recursively, to prune empty branches.
    pub fn delete(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        let maybe_parent = self.get_parent(storage)?;
        if let Some(mut parent) = maybe_parent {
            // Remove node reference from parent
            if parent.left == Some(self.key) {
                parent.left = None;
            } else if parent.right == Some(self.key) {
                parent.right = None;
            }

            if !parent.has_child() {
                // Remove no-children parents
                parent.delete(storage)?;
            } else {
                // Update parents values after removing node
                parent.sync_range_and_value(storage)?;
            }
        }

        NODES.remove(storage, &(self.book_id, self.tick_id, self.key));

        Ok(())
    }

    #[cfg(test)]
    /// Depth first search traversal of tree
    pub fn traverse(&self, storage: &dyn Storage) -> ContractResult<Vec<TreeNode>> {
        let mut nodes = vec![];
        nodes.push(self.clone());
        if !self.is_internal() {
            return Ok(nodes);
        }
        if let Some(left) = self.get_left(storage)? {
            nodes.append(&mut left.traverse(storage)?);
        }
        if let Some(right) = self.get_right(storage)? {
            nodes.append(&mut right.traverse(storage)?);
        }
        Ok(nodes)
    }

    #[cfg(test)]
    pub fn get_height(&self, storage: &dyn Storage) -> ContractResult<u8> {
        let mut height = 0;
        if let Some(left) = self.get_left(storage)? {
            height = height.max(left.get_height(storage)?);
        }
        if let Some(right) = self.get_right(storage)? {
            height = height.max(right.get_height(storage)?);
        }
        Ok(height + 1)
    }

    #[cfg(test)]
    pub fn traverse_bfs(&self, storage: &dyn Storage) -> ContractResult<BFSVec> {
        let mut result = vec![vec![(Some(self.clone()), None)]];
        let mut queue: Vec<Option<TreeNode>> = vec![Some(self.clone())];
        while queue.iter().any(|n| n.is_some()) {
            let mut level = vec![];
            let mut next_queue: Vec<Option<TreeNode>> = vec![];
            for node in queue {
                if let Some(node) = node {
                    level.push((node.get_left(storage)?, node.get_right(storage)?));
                    next_queue.push(node.get_left(storage)?);
                    next_queue.push(node.get_right(storage)?);
                } else {
                    level.push((None, None));
                    next_queue.push(None);
                    next_queue.push(None);
                }
            }
            queue = next_queue;
            result.push(level);
        }
        Ok(result)
    }
}

// For printing in test environments
#[cfg(test)]
impl Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::Leaf { value, etas } => write!(f, "{etas} {value}"),
            NodeType::Internal { accumulator, range } => {
                write!(f, "{} {}-{}", accumulator, range.0, range.1)
            }
        }
    }
}

// For printing in test environments
#[cfg(test)]
impl Display for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.node_type)
    }
}
