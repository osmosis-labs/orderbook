// TODO: Remove this
#![allow(dead_code)]
#[cfg(test)]
use core::fmt;
#[cfg(test)]
use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Storage, Uint128};
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
        value: Uint128,
        // Effective total amount sold
        etas: Uint128,
    },
    Internal {
        // Sum of all values below current
        accumulator: Uint128,
        // Range from min ETAS to max ETAS + value of max ETAS
        range: (Uint128, Uint128),
    },
}

impl NodeType {
    pub fn leaf(etas: impl Into<Uint128>, value: impl Into<Uint128>) -> Self {
        Self::Leaf {
            etas: etas.into(),
            value: value.into(),
        }
    }

    pub fn internal(
        accumulator: impl Into<Uint128>,
        range: (impl Into<Uint128>, impl Into<Uint128>),
    ) -> Self {
        Self::Internal {
            range: (range.0.into(), range.1.into()),
            accumulator: accumulator.into(),
        }
    }
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Internal {
            accumulator: Uint128::zero(),
            range: (Uint128::MAX, Uint128::MIN),
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

    pub fn save(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Ok(NODES.save(storage, &(self.book_id, self.tick_id, self.key), self)?)
    }

    /// Returns the maximum range value of a node.
    ///
    /// For `Internal` nodes, this is the maximum value of the associated range.
    /// For `Leaf` nodes, this is the sum of the `value` and `etas` fields.
    pub fn get_max_range(&self) -> Uint128 {
        match self.node_type {
            NodeType::Internal { range, .. } => range.1,
            NodeType::Leaf { value, etas } => value.checked_add(etas).unwrap(),
        }
    }

    pub fn set_max_range(&mut self, new_max: Uint128) -> ContractResult<()> {
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
    pub fn get_min_range(&self) -> Uint128 {
        match self.node_type {
            NodeType::Internal { range, .. } => range.0,
            NodeType::Leaf { etas, .. } => etas,
        }
    }

    pub fn set_min_range(&mut self, new_min: Uint128) -> ContractResult<()> {
        match &mut self.node_type {
            NodeType::Leaf { .. } => Err(ContractError::InvalidNodeType),
            NodeType::Internal { range, .. } => {
                range.0 = new_min;
                Ok(())
            }
        }
    }

    /// Adds a given value to an internal node's accumulator
    ///
    /// Errors if given node is not internal
    pub fn add_value(&mut self, value: Uint128) -> ContractResult<()> {
        match &mut self.node_type {
            NodeType::Internal { accumulator, .. } => {
                *accumulator = accumulator.checked_add(value)?;
                Ok(())
            }
            NodeType::Leaf { .. } => Err(ContractError::InvalidNodeType),
        }
    }

    /// Gets the value for a given node.
    ///
    /// For `Leaf` nodes this is the `value`.
    ///
    /// For `Internal` nodes this is the `accumulator`.
    pub fn get_value(&self) -> Uint128 {
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
    /// 3. Incompatible left, Right is empty, insert right
    /// 4. Left is leaf, split left
    /// 5. Left is incompatible, right is leaf, split right
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

        // Check if left node exists
        if let Some(mut left_node) = maybe_left {
            if left_node.is_internal() && new_node.get_min_range() < left_node.get_max_range() {
                // Case: Left is internal and new node is in range
                left_node.insert(storage, new_node)?;
                self.save(storage)?;
                return Ok(());
            }

            if let Some(mut right_node) = maybe_right {
                if right_node.is_internal()
                    && right_node.get_min_range() <= new_node.get_min_range()
                {
                    // Case: Left is leaf, right is internal and node is in range
                    right_node.insert(storage, new_node)?;
                    self.save(storage)?;
                    return Ok(());
                }

                if !left_node.is_internal() {
                    // Case: Left is leaf, right is not in range
                    // Insert parent left
                    let new_left = left_node.split(storage, new_node)?;
                    self.left = Some(new_left);
                    self.save(storage)?;

                    return Ok(());
                }

                // Case: Left is leaf, right is leaf
                // Insert parent right
                // Is this ever met?
                let new_right = right_node.split(storage, new_node)?;
                self.right = Some(new_right);
                self.save(storage)?;

                Ok(())
            } else {
                // Case: Left exists and new node outside range, right does not exist
                // Insert right
                self.right = Some(new_node.key);
                new_node.parent = Some(self.key);

                new_node.save(storage)?;
                self.save(storage)?;
                Ok(())
            }
        } else {
            // Left does not exist, check if right exists
            if let Some(mut right_node) = maybe_right {
                if right_node.is_internal()
                    && new_node.get_min_range() >= right_node.get_min_range()
                {
                    // Case: Left does not exist, right does, is internal and node fits in to range
                    right_node.insert(storage, new_node)?;

                    self.save(storage)?;
                    return Ok(());
                }
            }

            // Case: Left does not exist, insert on left
            self.left = Some(new_node.key);

            new_node.parent = Some(self.key);
            new_node.save(storage)?;

            self.save(storage)?;
            Ok(())
        }
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
        self.parent = Some(id);
        new_node.parent = Some(id);
        new_parent.left = Some(new_left);
        new_parent.right = Some(new_right);

        new_parent.save(storage)?;
        self.save(storage)?;
        new_node.save(storage)?;

        Ok(id)
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
}

// For printing in test environments
#[cfg(test)]
impl Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::Leaf { value, etas } => write!(f, "{etas} {value}"),
            NodeType::Internal { accumulator, range } => {
                write!(f, "{} ({}, {})", accumulator, range.0, range.1)
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
