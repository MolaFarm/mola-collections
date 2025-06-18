use core::fmt::{Debug, Display};
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;
use core::ops::Deref;

use alloc::sync::Arc;
use crossbeam_utils::CachePadded;
use hashbrown::DefaultHashBuilder;

use super::traits::ShardStorage;

/// A wrapper type that can hold either an owned value or a shared reference
/// wrapped in an `Arc`. This is useful for cases where you want to
/// conditionally use either an owned value or a shared reference without
/// needing to clone the value unnecessarily.
pub enum MaybeArc<T> {
    Owned(T),
    Shared(Arc<T>),
}

impl<T> MaybeArc<T> {
    /// Create a new `MaybeArc` from an owned value.
    pub fn new_owned(value: T) -> Self {
        MaybeArc::Owned(value)
    }

    /// Create a new `MaybeArc` from a shared reference.
    pub fn new_shared(value: Arc<T>) -> Self {
        MaybeArc::Shared(value)
    }

    /// Check if the value is owned (not shared).
    pub fn is_owned(&self) -> bool {
        matches!(self, MaybeArc::Owned(_))
    }

    /// Check if the value is shared (wrapped in an `Arc`).
    pub fn is_shared(&self) -> bool {
        matches!(self, MaybeArc::Shared(_))
    }

    /// Try get the owned value, if it exists.
    pub fn try_owned(self) -> Option<T> {
        if let MaybeArc::Owned(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Try get the shared value, if it exists.
    pub fn try_shared(self) -> Option<Arc<T>> {
        if let MaybeArc::Shared(arc) = self {
            Some(arc)
        } else {
            None
        }
    }
}

impl<T> Deref for MaybeArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeArc::Owned(value) => value,
            MaybeArc::Shared(arc) => arc.deref(),
        }
    }
}

impl<T> AsRef<T> for MaybeArc<T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: Clone> Clone for MaybeArc<T> {
    fn clone(&self) -> Self {
        match self {
            MaybeArc::Owned(value) => MaybeArc::Owned(value.clone()),
            MaybeArc::Shared(arc) => MaybeArc::Shared(Arc::clone(arc)),
        }
    }
}

impl<T: Display> Display for MaybeArc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MaybeArc::Owned(value) => write!(f, "{}", value),
            MaybeArc::Shared(arc) => write!(f, "{}", arc),
        }
    }
}

impl<T: Debug> Debug for MaybeArc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MaybeArc::Owned(value) => write!(f, "MaybeArc::Owned({:?})", value),
            MaybeArc::Shared(arc) => write!(f, "MaybeArc::Shared({:?})", arc),
        }
    }
}

impl<T: PartialEq> PartialEq for MaybeArc<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MaybeArc::Owned(a), MaybeArc::Owned(b)) => a == b,
            (MaybeArc::Shared(a), MaybeArc::Shared(b)) => a == b,
            (MaybeArc::Owned(a), MaybeArc::Shared(b)) => a == b.as_ref(),
            (MaybeArc::Shared(a), MaybeArc::Owned(b)) => a.as_ref() == b,
        }
    }
}

/// A generic concurrent hash map wrapper that provides a unified interface
/// over different shard storage and implementation strategies.
///
/// # Type Parameters
/// * `K` - The key type
/// * `V` - The value type  
/// * `S` - The hash builder type
/// * `Storage` - The shard storage implementation
pub struct ConcurrentMap<K, V, S = DefaultHashBuilder, Storage = ()> {
    /// The underlying shard storage
    pub(crate) storage: Storage,
    /// The hash builder for computing key hashes
    pub(crate) hash_builder: S,
    /// Phantom data for key and value types
    _marker: PhantomData<(K, V)>,
}

impl<K, V, S, Storage> ConcurrentMap<K, V, S, Storage>
where
    K: Eq + Hash,
    S: BuildHasher,
    Storage: ShardStorage<K, V>,
{
    /// Create a new concurrent map with the given storage and hash builder.
    ///
    /// # Arguments
    /// * `storage` - The shard storage implementation
    /// * `hash_builder` - The hash builder for computing key hashes
    ///
    /// # Returns
    /// A new concurrent map instance
    pub fn with_storage_and_hasher(storage: Storage, hash_builder: S) -> Self {
        Self {
            storage,
            hash_builder,
            _marker: PhantomData,
        }
    }

    /// Compute the hash of a key using the configured hash builder.
    ///
    /// # Arguments
    /// * `key` - The key to hash
    ///
    /// # Returns
    /// The hash value of the key
    #[inline]
    pub fn hash_key<Q: ?Sized + Hash>(&self, key: &Q) -> u64 {
        self.hash_builder.hash_one(key)
    }

    /// Get the shard that should contain the given key.
    ///
    /// # Arguments
    /// * `key` - The key to find the shard for
    ///
    /// # Returns
    /// A reference to the appropriate shard
    #[inline]
    pub fn shard_for_key<Q: ?Sized + Hash>(&self, key: &Q) -> &CachePadded<Storage::Shard> {
        let hash = self.hash_key(key);
        self.storage.shard_for_hash(hash)
    }

    /// Get the total number of shards in the map.
    ///
    /// # Returns
    /// The number of shards
    pub fn shard_count(&self) -> usize {
        self.storage.shard_count()
    }
}

impl<K, V, S, Storage> ConcurrentMap<K, V, S, Storage>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
    Storage: ShardStorage<K, V> + Default,
{
    /// Create a new concurrent map with default storage and hash builder.
    ///
    /// # Returns
    /// A new concurrent map instance with default configuration
    pub fn with_defaults() -> Self {
        Self::with_storage_and_hasher(Storage::default(), S::default())
    }
}

// Safety: ConcurrentMap can be Send if its components are Send
unsafe impl<K, V, S, Storage> Send for ConcurrentMap<K, V, S, Storage>
where
    K: Send + Sync,
    V: Send + Sync,
    S: Send + Sync,
    Storage: Send + Sync,
{
}

// Safety: ConcurrentMap can be Sync if its components are Sync
unsafe impl<K, V, S, Storage> Sync for ConcurrentMap<K, V, S, Storage>
where
    K: Send + Sync,
    V: Send + Sync,
    S: Send + Sync,
    Storage: Send + Sync,
{
}
