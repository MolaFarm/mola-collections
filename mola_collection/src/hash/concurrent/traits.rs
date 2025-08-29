use core::borrow::Borrow;
use core::hash::Hash;
use core::ops::{Deref, DerefMut};

use alloc::sync::Arc;
use crossbeam_utils::CachePadded;

use crate::hash::concurrent::wrapper::MaybeArc;

/// A trait defining the interface for shard storage in concurrent hash maps.
///
/// This trait provides a unified interface for accessing and managing shards
/// in different concurrent hash map implementations.
pub trait ShardStorage<K, V>: Send + Sync {
    /// The type of the individual shard
    type Shard;

    /// Get the shard that should contain the given hash value
    ///
    /// # Arguments
    /// * `hash` - The hash value to determine the shard for
    ///
    /// # Returns
    /// A reference to the appropriate shard
    fn shard_for_hash(&self, hash: u64) -> &CachePadded<Self::Shard>;

    /// Get the total number of shards
    ///
    /// # Returns
    /// The number of shards in the storage
    fn shard_count(&self) -> usize;

    /// Increment the object counter
    fn shard_increment(&self, num: usize);

    /// Decrement the object counter
    fn shard_decrement(&self, num: usize);

    /// Get the count of items in the storage
    ///
    /// # Returns
    /// The number of items in the shard
    fn shard_len(&self) -> usize;

    /// Check if the storage is empty
    ///
    /// # Returns
    /// True if the shard is empty, false otherwise
    fn shard_is_empty(&self) -> bool;
}

/// A trait defining the core hash map operations.
///
/// This trait provides a unified interface for basic hash map operations
/// across different implementations.
pub trait RawHashMap<K, V>: Send + Sync {
    /// Insert a key-value pair into the hash map.
    ///
    /// # Arguments
    /// * `key` - The key to insert
    /// * `value` - The value to insert
    ///
    /// # Returns
    /// The previous value associated with the key, if any
    fn insert(&self, key: K, value: V) -> Option<MaybeArc<V>>;

    /// Remove a key-value pair from the hash map.
    ///
    /// # Arguments
    /// * `key` - The key to remove
    ///
    /// # Returns
    /// The value that was removed, if the key existed
    fn remove<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash;

    /// Check if a key exists in the hash map.
    ///
    /// # Arguments
    /// * `key` - The key to check for
    ///
    /// # Returns
    /// True if the key exists, false otherwise
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash;

    /// Get the total number of entries in the hash map.
    ///
    /// # Returns
    /// The total number of key-value pairs in the map
    fn len(&self) -> usize;

    /// Check if the hash map is empty.
    ///
    /// # Returns
    /// True if the map contains no entries, false otherwise  
    fn is_empty(&self) -> bool;
}

/// A trait for get immutable reference on concurrent hash maps.
pub trait ReadableMap<K, V>: RawHashMap<K, V> 
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    /// Get a value from the hash map.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// A reference to the value, if the key exists
    fn get<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash;
}

/// A trait for read-only view operations on concurrent hash maps.
pub trait ReadableInPlaceMap<K, V>: RawHashMap<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    /// The type returned by view operations
    type ReadResult<R>;

    /// Perform a read-only view operation on a key-value pair.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    /// * `f` - A closure that receives the key and value references
    ///
    /// # Returns
    /// The result of the closure if the key exists, None otherwise
    fn view<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&K, &V) -> R;
}

/// A trait for mutable guards in concurrent hash maps.
///
/// This trait provides a way to safely modify values in a concurrent
/// hash map while ensuring that changes can be committed or rolled back.
pub trait MutableGuard<'a, K, V>: Deref<Target = V> + DerefMut<Target = V> + 'a
where
    K: Eq + Hash,
{
    /// Commit the changes made to the value.
    ///
    /// This method should be called after modifying the value to ensure
    /// that the changes are persisted.
    ///
    /// # Returns
    /// Ok if the commit was successful, Err if there was an issue
    /// committing the changes.
    fn commit(self) -> Result<(), ()>;
}

/// A trait for get mutable reference on concurrent hash maps.
///
/// This trait is designed for implementations that support in-place
/// modification of values.
pub trait MutableMap<K, V>: RawHashMap<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    type Guard<'a>: MutableGuard<'a, K, V> where Self: 'a;

    /// Get a mutable reference to the value associated with a key.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// A mutable guard for the value, if the key exists
    fn get_mut<'a, Q>(&'a self, key: &Q) -> Option<Self::Guard<'a>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash;
}

pub trait MutableInPlaceMap<K, V>: RawHashMap<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    type AlterResult<R>;

    /// Modify an existing entry in place.
    ///
    /// # Arguments
    /// * `key` - The key to modify
    /// * `f` - A closure that receives a mutable reference to the value
    ///
    /// # Returns
    /// The result of the closure if the key exists, None otherwise
    fn alter<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&mut V) -> R;

    /// Atomically modify an entry if it exists, or insert a default and then modify.
    ///
    /// # Arguments
    /// * `key` - The key to operate on
    /// * `default` - A closure to create a new value if the key is absent
    /// * `f` - A closure to modify the existing or newly created value
    fn alter_entry<F, D>(&self, key: K, default: D, f: F)
    where
        F: FnOnce(&mut V),
        D: FnOnce() -> V;

}

/// A trait for concurrent hash maps that support atomic set operation.
///
/// This trait extends the `RawHashMap` with atomic set operation
/// that can be performed without requiring locks.
pub trait AtomicSet<K, V>: RawHashMap<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    fn compare_and_set(&self, key: &K, current: Arc<V>, new: Arc<V>) -> bool;
}
