use core::ptr::NonNull;

use super::traits::{Link, LinkWithPrev, List, Node};

/// A generic intrusive linked list.
#[derive(Debug)]
pub struct LinkedList<T: Node> {
    head: Option<NonNull<T>>,
    count: usize,
}

impl<T> LinkedList<T>
where
    T: Node,
{
    /// Creates a new, empty linked list.
    pub const fn new() -> Self {
        LinkedList {
            head: None,
            count: 0,
        }
    }
}

impl<T> Link for LinkedList<T>
where
    T: Node,
{
    type Target = T;

    fn next(&self) -> Option<NonNull<T>> {
        self.head
    }

    fn set_next(&mut self, next: Option<NonNull<T>>) {
        self.head = next;
    }
}

impl<T> LinkWithPrev for LinkedList<T>
where
    T: Node,
{
    /// Get the previous pointer in the linked list.
    /// This implementation is for treating LinkedList as a `Link` to
    /// simplify the link operations. So it will always return `None`
    /// since it is not a real link.
    fn prev(&self) -> Option<NonNull<T>> {
        None
    }

    /// Set the previous pointer in the linked list.
    /// This implementation is for treating LinkedList as a `Link` to
    /// simplify the link operations. So it will not do anything.
    fn set_prev(&mut self, _parent: Option<NonNull<T>>) {}
}

impl<T> List for LinkedList<T>
where
    T: Node<Target = T>,
{
    fn head(&self) -> Option<NonNull<T>> {
        self.next()
    }

    fn set_head(&mut self, head: Option<NonNull<T>>) {
        self.set_next(head);
    }

    fn push(&mut self, node: NonNull<T>) {
        unsafe {
            let node_ref = &mut *node.as_ptr();
            node_ref.append_to(self);
            self.count += 1;
        }
    }

    fn pop(&mut self) -> Option<NonNull<T>> {
        self.head.inspect(|head| {
            unsafe {
                let head_ref = &mut *head.as_ptr();
                head_ref.detach(Some(self));
                self.count -= 1;
            }
        })
    }

    fn remove(&mut self, node: NonNull<T>) -> Option<NonNull<T>> {
        unsafe {
            let mut prev: Option<NonNull<T>> = None;
            for current in self.iter() {
                if current == node {
                    let node_ptr = &mut *current.as_ptr();
                    if let Some(prev) = prev {
                        node_ptr.detach(Some(&mut *prev.as_ptr()));
                    } else {
                        node_ptr.detach(Some(self));
                    }
                    self.count -= 1;
                    return Some(current);
                }
                prev = Some(current);
            }
            None
        }
    }

    unsafe fn quick_remove(
        &mut self,
        node: NonNull<T>,
        parent: Option<NonNull<T>>,
    ) -> Option<NonNull<T>> {
        unsafe {
            let node_ref = &mut *node.as_ptr();
            if let Some(parent) = parent {
                node_ref.detach(Some(&mut *parent.as_ptr()));
            } else if self.head == Some(node) {
                node_ref.detach(Some(self));
            } else {
                node_ref.detach::<T>(None);
            }
            self.count -= 1;
            Some(node)
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn count(&self) -> usize {
        self.count
    }
}

impl<T> Default for LinkedList<T>
where
    T: Node,
{
    fn default() -> Self {
        Self {
            head: None,
            count: 0,
        }
    }
}

unsafe impl<T: Node + Send> Send for LinkedList<T> {}
unsafe impl<T: Node + Sync> Sync for LinkedList<T> {}
