use core::ptr::NonNull;

use super::{iter::LinkedListIter, traits::{Link, LinkWithPrev, List, Node}};

/// A wrapper for a link that map a `Link` to a different type.
/// This is useful for creating a link contains extra metadata.
pub struct LinkWrapper<'a, L, T, M>
where 
    L: Link<Target = T>,
{
    inner: &'a mut L,
    _marker: core::marker::PhantomData<M>,
}

impl<'a, L, T, M> LinkWrapper<'a, L, T, M>
where
    L: Link<Target = T>,
{
    /// Create a new `LinkWrapper` with the given inner link.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the inner link and is a subset of the target type.
    pub unsafe fn new(inner: &'a mut L) -> Self {
        LinkWrapper {
            inner,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a, L, T, M> Link for LinkWrapper<'a, L, T, M>
where
    L: Link<Target = T>,
{
    type Target = M;

    fn next(&self) -> Option<NonNull<M>> {
        self.inner.next().map(|n| n.cast())
    }

    fn set_next(&mut self, next: Option<NonNull<M>>) {
        self.inner.set_next(next.map(|n| n.cast()));
    }
}

impl<'a, L, T, M> LinkWithPrev for LinkWrapper<'a, L, T, M>
where
    L: LinkWithPrev<Target = T>,
{
    fn prev(&self) -> Option<NonNull<M>> {
        self.inner.prev().map(|n| n.cast())
    }

    fn set_prev(&mut self, prev: Option<NonNull<M>>) {
        self.inner.set_prev(prev.map(|n| n.cast()));
    }
}

/// A wrapper for a linked list that maps a `List` to a different type.
/// This is useful for creating a link contains extra metadata.
pub struct ListWrapper<'a, L, T, M>
where
    L: List<Target = T>,
{
    inner: &'a mut L,
    _marker: core::marker::PhantomData<M>,
}

impl<'a, L, T, M> ListWrapper<'a, L, T, M>
where
    L: List<Target = T>,
{
    /// Create a new `ListWrapper` with the given inner list.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the inner list and is a subset of the target type.
    pub unsafe fn new(inner: &'a mut L) -> Self {
        ListWrapper {
            inner,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a, L, T, M> Link for ListWrapper<'a, L, T, M>
where
    L: List<Target = T>,
{
    type Target = M;

    fn next(&self) -> Option<NonNull<M>> {
        self.inner.next().map(|n| n.cast())
    }

    fn set_next(&mut self, next: Option<NonNull<M>>) {
        self.inner.set_next(next.map(|n| n.cast()));
    }
}

impl<'a, L, T, M> LinkWithPrev for ListWrapper<'a, L, T, M>
where
    L: List<Target = T>,
{
    fn prev(&self) -> Option<NonNull<M>> {
        self.inner.prev().map(|n| n.cast())
    }

    fn set_prev(&mut self, prev: Option<NonNull<M>>) {
        self.inner.set_prev(prev.map(|n| n.cast()));
    }
}

impl<'a, L, T, M> List for ListWrapper<'a, L, T, M>
where
    L: List<Target = T>,
{
    fn head(&self) -> Option<NonNull<M>> {
        self.inner.head().map(|n| n.cast())
    }

    fn set_head(&mut self, head: Option<NonNull<M>>) {
        self.inner.set_head(head.map(|n| n.cast()));
    }

    fn push(&mut self, node: NonNull<M>) {
        self.inner.push(node.cast());
    }

    fn pop(&mut self) -> Option<NonNull<M>> {
        self.inner.pop().map(|n| n.cast())
    }

    fn remove(&mut self, node: NonNull<M>) -> Option<NonNull<M>> {
        self.inner.remove(node.cast()).map(|n| n.cast())
    }

    unsafe fn quick_remove(&mut self, node: NonNull<M>, parent: Option<NonNull<M>>) -> Option<NonNull<M>> {
        unsafe {
            self.inner.quick_remove(node.cast(), parent.map(|n| n.cast())).map(|n| n.cast())
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }

    unsafe fn iter<'b>(&'b self) -> LinkedListIter<'b, M, Self> 
    where 
        M: Node,
    {
        unsafe { LinkedListIter::new(self) }
    }
}