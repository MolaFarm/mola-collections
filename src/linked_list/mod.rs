//! An intrusive linked list implementation.
//!
//! In an intrusive linked list, the nodes are stored directly in the data structure
//! that is being linked. This means that the data structure must have a field that
//! is a `ListNode`. This is in contrast to a non-intrusive linked list, where the
//! nodes are stored separately from the data structure.
//!
//! # Examples
//!
//! ```
//! use mola_collections::linked_list::intrusive::{
//!     list::LinkedList,
//!     single::SingleNode,
//!     traits::{Link, List, NodeWithData},
//! };
//! use core::ptr::NonNull;
//!
//! let mut list = LinkedList::<SingleNode<i32>>::new();
//! let mut node1 = SingleNode::<i32>::default();
//! let mut node2 = SingleNode::<i32>::default();
//! let mut node3 = SingleNode::<i32>::default();
//!
//! *node1.data_mut() = 1;
//! *node2.data_mut() = 2;
//! *node3.data_mut() = 3;
//!
//! list.push(NonNull::from(&mut node1));
//! list.push(NonNull::from(&mut node2));
//! list.push(NonNull::from(&mut node3));
//!
//! assert_eq!(list.count(), 3);
//!
//! unsafe {
//!     let mut current = list.head();
//!     let mut values = vec![];
//!     while let Some(node) = current {
//!         let node_ref = node.as_ref();
//!         values.push(*node_ref.data());
//!         current = node_ref.next();
//!     }
//!     assert_eq!(values, vec![3, 2, 1]);
//! }
//! ```
pub mod intrusive;
