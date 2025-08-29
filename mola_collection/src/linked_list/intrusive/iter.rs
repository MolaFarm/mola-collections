use core::ptr::NonNull;

use super::traits::{List, Node};

/// An iterator over a linked list.
pub struct LinkedListIter<'a, T: Node, L: List> {
    _list: &'a L,
    current: Option<NonNull<T>>,
}

impl<'a, T, L> LinkedListIter<'a, T, L>
where
    T: Node,
    L: List<Target = T>,
{
    /// Creates a new iterator over the given list.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the list is not modified while the iterator is alive.
    pub unsafe fn new(list: &'a L) -> Self {
        Self {
            current: list.head().map(|n| n.cast()),
            _list: list,
        }
    }
}

impl<'a, T, L> Iterator for LinkedListIter<'a, T, L>
where
    T: Node,
    L: List<Target = T>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.inspect(|current| {
            self.current = unsafe { current.as_ref().next().map(|n| n.cast()) };
        })
    }
}

unsafe impl<'a, T, L> Send for LinkedListIter<'a, T, L>
where
    T: Node + Send,
    L: List<Target = T>,
{
}

unsafe impl<'a, T, L> Sync for LinkedListIter<'a, T, L>
where
    T: Node + Sync,
    L: List<Target = T>,
{
}
