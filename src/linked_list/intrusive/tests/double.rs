extern crate std;

use std::vec;

use core::ptr::NonNull;

use crate::linked_list::intrusive::{
    double::DoubleNode,
    list::LinkedList,
    traits::{Link, LinkWithPrev, List, NodeWithData},
};

#[test]
fn test_double_list_push_pop() {
    let mut list = LinkedList::<DoubleNode<i32>>::new();
    assert!(list.is_empty());

    let mut node1 = DoubleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = DoubleNode::<i32>::default();
    *node2.data_mut() = 2;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));

    assert_eq!(list.count(), 2);
    assert!(!list.is_empty());

    unsafe {
        let popped = list.pop().unwrap();
        assert_eq!(*popped.as_ref().data(), 2);
        assert!(popped.as_ref().prev().is_none());
        assert_eq!(list.count(), 1);

        let head = list.head().unwrap();
        assert!(head.as_ref().prev().is_none());

        let popped = list.pop().unwrap();
        assert_eq!(*popped.as_ref().data(), 1);
        assert_eq!(list.count(), 0);
    }

    assert!(list.is_empty());
    assert!(list.pop().is_none());
}

#[test]
fn test_double_list_iter() {
    let mut list = LinkedList::<DoubleNode<i32>>::new();
    let mut node1 = DoubleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = DoubleNode::<i32>::default();
    *node2.data_mut() = 2;
    let mut node3 = DoubleNode::<i32>::default();
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
fn test_double_list_remove() {
    let mut list = LinkedList::<DoubleNode<i32>>::new();
    let mut node1 = DoubleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = DoubleNode::<i32>::default();
    *node2.data_mut() = 2;
    let mut node3 = DoubleNode::<i32>::default();
    *node3.data_mut() = 3;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));
    list.push(NonNull::from(&mut node3)); // list is 3 -> 2 -> 1

    // Remove middle
    unsafe {
        let removed = list.remove(NonNull::from(&mut node2));
        assert!(removed.is_some());
        assert_eq!(*removed.unwrap().as_ref().data(), 2);
        
        // Check links
        let head = list.head().unwrap().as_ref();
        let tail = head.next().unwrap().as_ref();
        assert_eq!(*head.data(), 3);
        assert_eq!(*tail.data(), 1);
        assert_eq!(head.next().unwrap().as_ptr(), tail as *const _ as *mut _);
        assert_eq!(tail.prev().unwrap().as_ptr(), head as *const _ as *mut _);
    }
    assert_eq!(list.count(), 2);

    // Remove head
    unsafe {
        let removed = list.remove(NonNull::from(&mut node3));
        assert!(removed.is_some());
        let new_head = list.head().unwrap().as_ref();
        assert_eq!(*new_head.data(), 1);
        assert!(new_head.prev().is_none());
    }
    assert_eq!(list.count(), 1);

    // Remove tail
    let removed = list.remove(NonNull::from(&mut node1));
    assert!(removed.is_some());
    assert!(list.is_empty());
}

#[test]
fn test_double_list_quick_remove() {
    let mut list = LinkedList::<DoubleNode<i32>>::new();
    let mut node1 = DoubleNode::<i32>::default();
    *node1.data_mut() = 1;
    let mut node2 = DoubleNode::<i32>::default();
    *node2.data_mut() = 2;
    let mut node3 = DoubleNode::<i32>::default();
    *node3.data_mut() = 3;

    list.push(NonNull::from(&mut node1));
    list.push(NonNull::from(&mut node2));
    list.push(NonNull::from(&mut node3)); // list is 3 -> 2 -> 1

    // Quick remove middle
    unsafe {
        let removed = list.quick_remove(NonNull::from(&mut node2), Some(NonNull::from(&mut node3)));
        assert!(removed.is_some());
        
        // Check links
        let head = list.head().unwrap().as_ref();
        let tail = head.next().unwrap().as_ref();
        assert_eq!(head.next().unwrap().as_ptr(), tail as *const _ as *mut _);
        assert_eq!(tail.prev().unwrap().as_ptr(), head as *const _ as *mut _);
    }
    assert_eq!(list.count(), 2);
}

