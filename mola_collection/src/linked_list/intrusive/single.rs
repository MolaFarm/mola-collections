use core::ptr::NonNull;

use mola_collection_derive::Node;

use super::traits::{Link, List, Node};

/// A node in a singly linked list.
#[derive(Node)]
#[node(crate_path = "crate")]
pub struct SingleNode<T> {
    link: SingleLink,
    data: T,
}

impl<T: Default> Default for SingleNode<T> {
    fn default() -> Self {
        Self {
            link: SingleLink::default(),
            data: T::default(),
        }
    }
}

/// A link in a singly linked list.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleLink {
    next: Option<NonNull<Self>>,
}

impl Link for SingleLink {
    type Target = Self;

    #[inline]
    fn next(&self) -> Option<NonNull<Self>> {
        self.next
    }

    #[inline]
    fn set_next(&mut self, next: Option<NonNull<Self>>) {
        self.next = next;
    }
}

impl Node for SingleLink {
    #[inline]
    fn append_to<L>(&mut self, list: &mut L)
    where
        L: List<Target = Self>,
    {
        self.set_next(list.next());
        list.set_next(Some(NonNull::from(self).cast()));
    }
    
    #[inline]
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
