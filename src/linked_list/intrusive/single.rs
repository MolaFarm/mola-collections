use core::ptr::NonNull;

use super::{node::ListNode, traits::{Link, List, Node}};

/// A node in a singly linked list.
pub type SingleNode<T> = ListNode<SingleLink, T>;

/// A link in a singly linked list.
#[derive(Debug, Default)]
pub struct SingleLink {
    next: Option<NonNull<Self>>,
}

impl Link for SingleLink {
    type Target = Self;

    fn next(&self) -> Option<NonNull<Self>> {
        self.next
    }

    fn set_next(&mut self, next: Option<NonNull<Self>>) {
        self.next = next;
    }
}

impl Node for SingleLink {
    fn append_to<L>(&mut self, list: &mut L)
    where
        L: List<Target = Self>,
    {
        self.set_next(list.next());
        list.set_next(Some(NonNull::from(self).cast()));
    }
    
    unsafe fn detach<L>(&mut self, parent: Option<&mut L>)
    where 
        L: Link<Target = Self>,
    {
        let parent = parent.expect("Parent must be provided for detaching");
        parent.set_next(self.next());
    }
}

unsafe impl Send for SingleLink {}
unsafe impl Sync for SingleLink {}
