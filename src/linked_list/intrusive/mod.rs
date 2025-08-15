//! # Intrusive Linked List
//!
//! This module provides an implementation of an intrusive linked list.
//!
//! ## Core Components
//!
//! - [`traits`]: Defines the core traits for the linked list, such as `List`, `Link`, and `Node`.
//! - [`list::LinkedList`]: A generic implementation of a linked list.
//! - [`single::SingleLink`] and [`double::DoubleLink`]: Link types for creating singly and doubly linked lists.
//! - [`node::ListNode`]: A node that can be embedded in a struct to make it part of a linked list.
//!
//! ## Safety
//!
//! This implementation uses `unsafe` code extensively for performance and to manage raw pointers.
//! The user of this module is responsible for upholding several invariants:
//!
//! - Nodes must outlive the list they are in.
//! - A node must not be in two lists at the same time.
//! - When iterating, the list must not be modified.
//! - When removing a node, the provided parent (if any) must be the correct parent of the node.

pub mod traits;
pub mod wrapper;
pub mod single;
pub mod double;
pub mod node;
pub mod list;
pub mod iter;

#[cfg(test)]
mod tests;
