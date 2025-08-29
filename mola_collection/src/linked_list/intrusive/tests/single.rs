extern crate std;

use std::vec;

use core::ptr::NonNull;

use crate::linked_list::intrusive::{
    list::LinkedList,
    single::SingleNode,
    traits::{List, NodeWithData},
};

#[test]
fn test_single_list_push_pop() {
    let mut list = LinkedList::<SingleNode<i32>>::new();
    assert!(list.is_empty());

    let mut node1 = SingleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = SingleNode::<i32>::default();
    *node2.data_mut() = 2;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));

    assert_eq!(list.count(), 2);
    assert!(!list.is_empty());

    let popped = list.pop().unwrap();
    assert_eq!(unsafe { *popped.as_ref().data() }, 2);
    assert_eq!(list.count(), 1);

    let popped = list.pop().unwrap();
    assert_eq!(unsafe { *popped.as_ref().data() }, 1);
    assert_eq!(list.count(), 0);

    assert!(list.is_empty());
    assert!(list.pop().is_none());
}

#[test]
fn test_single_list_iter() {
    let mut list = LinkedList::<SingleNode<i32>>::new();
    let mut node1 = SingleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = SingleNode::<i32>::default();
    *node2.data_mut() = 2;
    let mut node3 = SingleNode::<i32>::default();
    *node3.data_mut() = 3;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));
    list.push(NonNull::from(&mut node3));

    let mut values = vec![];
    unsafe {
        for node in list.iter() {
            values.push(*node.as_ref().data());
        }
    }
    assert_eq!(values, vec![3, 2, 1]);
}

#[test]
fn test_single_list_remove() {
    let mut list = LinkedList::<SingleNode<i32>>::new();
    let mut node1 = SingleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = SingleNode::<i32>::default();
    *node2.data_mut() = 2;
    let mut node3 = SingleNode::<i32>::default();
    *node3.data_mut() = 3;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));
    list.push(NonNull::from(&mut node3));

    // Remove middle
    let removed = list.remove(NonNull::from(&mut node2));
    assert!(removed.is_some());
    assert_eq!(unsafe { *removed.unwrap().as_ref().data() }, 2);
    assert_eq!(list.count(), 2);

    let mut values = vec![];
    unsafe {
        for node in list.iter() {
            values.push(*node.as_ref().data());
        }
    }
    assert_eq!(values, vec![3, 1]);

    // Remove head
    let removed = list.remove(NonNull::from(&mut node3));
    assert!(removed.is_some());
    assert_eq!(list.count(), 1);
    assert_eq!(unsafe { *list.head().unwrap().as_ref().data() }, 1);

    // Remove tail
    let removed = list.remove(NonNull::from(&mut node1));
    assert!(removed.is_some());
    assert!(list.is_empty());
}
