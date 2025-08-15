use core::ptr::NonNull;

use super::{
    traits::{Link, LinkWithPrev, List, Node, NodeWithData},
    wrapper::{LinkWrapper, ListWrapper},
};

/// A node in an intrusive linked list.
/// This struct should be embedded in the struct that you want to store in the list.
pub struct ListNode<L, T> {
    link: L,
    data: T,
}

impl<L: Link, T> Link for ListNode<L, T> {
    type Target = Self;

    fn next(&self) -> Option<NonNull<Self::Target>> {
        self.link.next().map(|n| n.cast())
    }

    fn set_next(&mut self, next: Option<NonNull<Self::Target>>) {
        self.link.set_next(next.map(|n| n.cast()));
    }
}

impl<L, T> Node for ListNode<L, T>
where
    L: Link + Node,
{
    fn append_to<U>(&mut self, list: &mut U)
    where
        U: List<Target = Self>,
    {
        unsafe {
            let mut wrapper = ListWrapper::new(list);
            self.link.append_to(&mut wrapper);
        }
    }

    unsafe fn detach<U>(&mut self, parent: Option<&mut U>)
    where
        U: Link<Target = Self>,
    {
        unsafe {
            if let Some(parent) = parent {
                let mut wrapper = LinkWrapper::new(parent);
                self.link.detach(Some(&mut wrapper));
            } else {
                self.link.detach::<LinkWrapper<'_, U, Self, L>>(None);
            }
        }
    }
}

impl<L, T> LinkWithPrev for ListNode<L, T>
where
    L: LinkWithPrev,
{
    fn prev(&self) -> Option<NonNull<Self>> {
        self.link.prev().map(|n| n.cast())
    }

    fn set_prev(&mut self, prev: Option<NonNull<Self>>) {
        self.link.set_prev(prev.map(|n| n.cast()));
    }
}

impl<L, T> NodeWithData for ListNode<L, T>
where
    L: Link + Node,
{
    type Data = T;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }
}

impl<L, T> Default for ListNode<L, T>
where
    L: Default,
    T: Default,
{
    fn default() -> Self {
        Self {
            link: L::default(),
            data: T::default(),
        }
    }
}

unsafe impl<L, T> Send for ListNode<L, T>
where
    L: Send,
    T: Send,
{
}

unsafe impl<L, T> Sync for ListNode<L, T>
where
    L: Sync,
    T: Sync,
{
}
