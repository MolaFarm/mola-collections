use core::ptr::NonNull;

use super::{
    node::ListNode,
    traits::{Link, LinkWithPrev, List, Node},
};

/// A node in a doubly linked list.
pub type DoubleNode<T> = ListNode<DoubleLink, T>;

/// A link in a doubly linked list.
#[derive(Default)]
pub struct DoubleLink {
    next: Option<NonNull<Self>>,
    prev: Option<NonNull<Self>>,
}

impl Link for DoubleLink {
    type Target = Self;

    fn next(&self) -> Option<NonNull<Self::Target>> {
        self.next
    }

    fn set_next(&mut self, next: Option<NonNull<Self::Target>>) {
        self.next = next;
    }
}

impl LinkWithPrev for DoubleLink {
    fn prev(&self) -> Option<NonNull<Self::Target>> {
        self.prev
    }

    fn set_prev(&mut self, prev: Option<NonNull<Self::Target>>) {
        self.prev = prev;
    }
}

impl Node for DoubleLink {
    fn append_to<L>(&mut self, list: &mut L)
    where
        L: List<Target = Self>,
    {
        let self_ptr = NonNull::from(&mut *self);
        self.set_next(list.next());
        if let Some(next) = self.next() {
            let next = unsafe { &mut *next.as_ptr() };
            next.set_prev(Some(self_ptr));
        }
        self.set_prev(list.prev());
        list.set_next(Some(self_ptr.cast()));
    }

    unsafe fn detach<L>(&mut self, parent: Option<&mut L>)
    where
        L: Link<Target = Self>,
    {
        if let Some(parent) = parent {
            assert_eq!(
                parent.next(),
                Some(NonNull::from(&mut *self).cast()),
                "Parent must be the one that contains this node"
            );

            parent.set_next(self.next());
            if let Some(next) = self.next() {
                let next = unsafe { &mut *next.as_ptr() };
                next.set_prev(self.prev());
            }
        } else {
            let prev = self
                .prev()
                .map(|n| unsafe { &mut *n.as_ptr() })
                .expect("Trying to detach an orphan node");
            unsafe { self.detach(Some(prev)) };
        }
    }
}

unsafe impl Send for DoubleLink {}
unsafe impl Sync for DoubleLink {}
