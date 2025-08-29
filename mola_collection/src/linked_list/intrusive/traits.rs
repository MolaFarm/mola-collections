use core::ptr::NonNull;

use super::iter::LinkedListIter;

/// A trait for a linked list.
pub trait List: Link + LinkWithPrev {
    /// Get the head of the linked list
    fn head(&self) -> Option<NonNull<Self::Target>>;

    /// Set the head of the linked list
    fn set_head(&mut self, head: Option<NonNull<Self::Target>>);

    /// Push a new node to the front of the linked list
    fn push(&mut self, node: NonNull<Self::Target>);

    /// Pop a node from the front of the linked list
    fn pop(&mut self) -> Option<NonNull<Self::Target>>;

    /// Remove a node from the linked list
    fn remove(&mut self, node: NonNull<Self::Target>) -> Option<NonNull<Self::Target>>;

    /// Quick remove a node from the linked list without checking if it exists
    /// 
    /// This method quickly removes a node with an optional parent pointer,
    /// this will directly detach the node from the linked list without checking if it exists.
    /// Usually you will need a custom structure(index) to prove that the node is in the linked list.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the node exists in the linked list.
    /// It will not check if the node is actually in the list.
    unsafe fn quick_remove(&mut self, node: NonNull<Self::Target>, parent: Option<NonNull<Self::Target>>) -> Option<NonNull<Self::Target>>;

    /// Check if the linked list is empty
    fn is_empty(&self) -> bool;

    /// Get the number of nodes in the linked list
    fn count(&self) -> usize;

    /// Get an iterator over the linked list
    /// # Safety
    /// The caller must ensure that the linked list is not modified while iterating.
    unsafe fn iter<'a>(&'a self) -> LinkedListIter<'a, Self::Target, Self> 
    where 
        Self::Target: Node
    {
        unsafe { LinkedListIter::new(self) }
    }
}

/// A trait for a link in a linked list.
pub trait Link: Sized {
    /// The target type of the link.
    type Target;

    /// Get the next pointer in the linked list
    fn next(&self) -> Option<NonNull<Self::Target>>;

    /// Set the next pointer in the linked list
    fn set_next(&mut self, next: Option<NonNull<Self::Target>>);
}

/// A trait for a link with a previous pointer.
pub trait LinkWithPrev: Link {
    /// Get the previous pointer in the linked list
    fn prev(&self) -> Option<NonNull<Self::Target>>;

    /// Set the previous pointer in the linked list
    fn set_prev(&mut self, parent: Option<NonNull<Self::Target>>);
}

/// A trait for a node in a linked list.
pub trait Node: Link {
    /// Append the node to a linked list
    fn append_to<L>(&mut self, list: &mut L)
    where
        L: List<Target = Self>;

    /// Detach the node from the linked list
    /// 
    /// # Safety
    /// 
    /// The parent node must be the one that contains this node or a `LinkedList`
    /// that contains this node. It will update the parent's next pointer to skip this node.
    unsafe fn detach<L>(&mut self, parent: Option<&mut L>)
    where 
        L: Link<Target = Self>;
}

/// A trait for a node that contains data.
pub trait NodeWithData: Node {
    /// The type of data stored in the node.
    type Data;

    /// Get the data associated with the node
    fn data(&self) -> &Self::Data;

    /// Get a mutable reference to the data associated with the node
    fn data_mut(&mut self) -> &mut Self::Data;
}
