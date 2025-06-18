use core::ops::Deref;

use crate::hash::concurrent::traits::RawHashMap;

mod locked_impl;
mod rcu_impl;
mod traits;
mod wrapper;

#[cfg(test)]
mod tests;

pub mod locked {
    pub use super::locked_impl::*;
}

pub mod rcu {
    pub use super::rcu_impl::*;
}

pub mod prelude {
    pub use super::traits::*;
    pub use super::wrapper::{MaybeArc, ConcurrentMap};
}

pub type LockedMap<K, V> = DefaultHashMap<K, V, locked_impl::LockedMap<K, V>>;
pub type RcuMap<K, V> = DefaultHashMap<K, V, rcu_impl::HamtMap<K, V>>;

pub struct DefaultHashMap<K, V, M>
where
    M: RawHashMap<K, V>
{
    inner: M,
    _marker: core::marker::PhantomData<(K, V)>,
}

impl<K, V, M> DefaultHashMap<K, V, M>
where
    M: RawHashMap<K, V> + Default,
{
    /// Creates a new `DefaultHashMap` with the given inner map.
    pub fn new() -> Self {
        Self {
            inner: M::default(),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<K, V, M> Deref for DefaultHashMap<K, V, M>
where
    M: RawHashMap<K, V>,
{
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}