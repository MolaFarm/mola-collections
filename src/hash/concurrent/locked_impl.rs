use alloc::boxed::Box;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_utils::CachePadded;
use hashbrown::DefaultHashBuilder;
use hashbrown::Equivalent;
use hashbrown::hash_table::{Entry, HashTable};
use spin::RwLock;

use crate::hash::concurrent::wrapper::MaybeArc;

use super::traits::{MutableMap, ReadableMap, RawHashMap, ShardStorage, MutableGuard, MutableInPlaceMap, ReadableInPlaceMap};
use super::wrapper::ConcurrentMap;

/// A dummy guard for locked concurrent map since it doesn't support mutable guards.
/// This is just a placeholder to satisfy the trait requirements.
pub struct LockedGuard<'a, K, V, M> 
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Send + Sync + PartialEq + 'a,
    M: RawHashMap<K, V> + MutableInPlaceMap<K, V>,
{
    map: &'a M,
    key: K,
    original_value: V,
    value: V,
}

impl<'a, K, V, M> Deref for LockedGuard<'a, K, V, M> 
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Send + Sync + PartialEq + 'a,
    M: RawHashMap<K, V> + MutableInPlaceMap<K, V>,
{
    type Target = V;
    
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, K, V, M> DerefMut for LockedGuard<'a, K, V, M> 
where 
    K: Hash + Eq + Send + Sync + 'a,
    V: Send + Sync + PartialEq + 'a,
    M: RawHashMap<K, V> + MutableInPlaceMap<K, V>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, K, V, M> MutableGuard<'a, K, V> for LockedGuard<'a, K, V, M>
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Send + Sync + PartialEq + 'a,
    M: RawHashMap<K, V> + MutableInPlaceMap<K, V>,
{
    fn commit(self) -> Result<(), ()> {
        self.map.alter(&self.key, |v| {
            if v != &self.original_value {
                // Value has changed by another thread, we cannot commit
                return Err(());
            }
            *v = self.value; // Update the value in the map
            Ok(())
        })
        .unwrap_or(Err(()))
    }
}

/// A single shard of the locked hash table.
pub struct LockedShard<K, V> {
    pub(crate) table: RwLock<HashTable<(K, V)>>,
}

impl<K, V> LockedShard<K, V> {
    /// Create a new shard with the specified capacity.
    ///
    /// # Arguments
    /// * `capacity` - The initial capacity of the shard
    ///
    /// # Returns
    /// A new shard instance
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            table: RwLock::new(HashTable::with_capacity(capacity)),
        }
    }
}

impl<K, V> Default for LockedShard<K, V> {
    fn default() -> Self {
        Self {
            table: RwLock::new(HashTable::new()),
        }
    }
}

/// Storage implementation for locked concurrent hash maps.
///
/// This storage uses spin-based read-write locks to protect each shard,
/// providing thread-safe access with good performance characteristics.
pub struct LockedStorage<K, V> {
    shards: Box<[CachePadded<LockedShard<K, V>>]>,
    count: AtomicUsize,
}

impl<K, V> LockedStorage<K, V> {
    /// Create new locked storage with the specified number of shards and capacity.
    ///
    /// # Arguments
    /// * `shards` - The number of shards (must be a power of two)
    /// * `capacity` - The initial capacity per shard
    ///
    /// # Returns
    /// A new locked storage instance
    ///
    /// # Panics
    /// Panics if `shards` is not a power of two
    pub fn with_shards_and_capacity(shards: usize, capacity: usize) -> Self {
        assert!(
            shards.is_power_of_two(),
            "Number of shards must be a power of two"
        );
        let mut shard_vec = Vec::with_capacity(shards);
        for _ in 0..shards {
            shard_vec.push(CachePadded::new(LockedShard::with_capacity(capacity)));
        }
        Self {
            shards: shard_vec.into_boxed_slice(),
            count: AtomicUsize::new(0),
        }
    }
}

// Default number of shards. Must be a power of two.
const DEFAULT_SHARDS: usize = 32;

impl<K, V> Default for LockedStorage<K, V> {
    fn default() -> Self {
        Self::with_shards_and_capacity(DEFAULT_SHARDS, 0)
    }
}

impl<K, V> ShardStorage<K, V> for LockedStorage<K, V>
where
    K: Send + Sync,
    V: Send + Sync,
{
    type Shard = LockedShard<K, V>;

    fn shard_for_hash(&self, hash: u64) -> &CachePadded<Self::Shard> {
        &self.shards[hash as usize & (self.shards.len() - 1)]
    }

    fn shard_count(&self) -> usize {
        self.shards.len()
    }

    fn shard_increment(&self, num: usize) {
        self.count.fetch_add(num, Ordering::AcqRel);
    }

    fn shard_decrement(&self, num: usize) {
        self.count.fetch_sub(num, Ordering::AcqRel);
    }

    fn shard_len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    fn shard_is_empty(&self) -> bool {
        self.shard_len() == 0
    }
}

/// Type alias for a locked concurrent map using the standard configuration.
pub type LockedMap<K, V, S = DefaultHashBuilder> =
    ConcurrentMap<K, V, S, LockedStorage<K, V>>;

impl<K, V, S> LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Default + Send + Sync,
{
    /// Create a new locked concurrent map with default settings.
    ///
    /// # Returns
    /// A new locked concurrent map instance
    pub fn new() -> Self {
        Self::with_shards_and_capacity_and_hasher(DEFAULT_SHARDS, 0, Default::default())
    }
}

impl<K, V, S> LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    /// Create a new locked concurrent map with custom settings.
    ///
    /// # Arguments
    /// * `shards` - The number of shards (must be a power of two)
    /// * `capacity` - The initial capacity per shard
    /// * `hash_builder` - The hash builder to use
    ///
    /// # Returns
    /// A new locked concurrent map instance
    ///
    /// # Panics
    /// Panics if `shards` is not a power of two
    pub fn with_shards_and_capacity_and_hasher(
        shards: usize,
        capacity: usize,
        hash_builder: S,
    ) -> Self {
        let storage = LockedStorage::with_shards_and_capacity(shards, capacity);
        ConcurrentMap::with_storage_and_hasher(storage, hash_builder)
    }
}

impl<K, V, S> Default for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Default + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, S> RawHashMap<K, V> for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    fn insert(&self, key: K, value: V) -> Option<MaybeArc<V>> {
        let hash = self.hash_key(&key);
        let shard = self.storage.shard_for_hash(hash);
        let mut table = shard.table.write();

        let entry = table.entry(hash, |(k_ref, _)| k_ref == &key, |(k, _)| self.hash_key(k));

        match entry {
            Entry::Occupied(mut occ) => Some(MaybeArc::Owned(core::mem::replace(
                &mut occ.get_mut().1,
                value,
            ))),
            Entry::Vacant(vac) => {
                vac.insert((key, value));
                self.storage.shard_increment(1);
                None
            }
        }
    }

    fn remove<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: ?Sized + Eq + Hash,
    {
        let hash = self.hash_key(key);
        let shard = self.storage.shard_for_hash(hash);
        let mut table = shard.table.write();
        if let Ok(entry) = table.find_entry(hash, |(k, _v)| key.equivalent(k)) {
            let ((_, v), _) = entry.remove();
            self.storage.shard_decrement(1);
            Some(MaybeArc::Owned(v))
        } else {
            None
        }
    }

    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        let hash = self.hash_key(key);
        let shard = self.storage.shard_for_hash(hash);
        let table = shard.table.read();
        table.find(hash, |(k, _v)| key.equivalent(k)).is_some()
    }

    fn len(&self) -> usize {
        self.storage.shard_len()
    }

    fn is_empty(&self) -> bool {
        self.storage.shard_is_empty()
    }
}

impl<K, V, S> ReadableMap<K, V> for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync + Clone,
    S: BuildHasher + Send + Sync,
{
    fn get<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.view(key, |_, v| MaybeArc::Owned(v.clone()))
    }
}

impl<K, V, S> ReadableInPlaceMap<K, V> for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    type ReadResult<R> = Option<R>;

    /// Read an entry under a read lock and compute a result using a closure.
    ///
    /// # Arguments
    /// * `key` - The key to look up
    /// * `f` - A closure that takes references to the found key and value and returns a result
    ///
    /// The closure `f` runs under the read lock and should complete quickly without sleeping.
    ///
    /// # Returns
    /// * `Some(R)` - If the key exists, returns the closure's result
    /// * `None` - If the key does not exist
    fn view<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&K, &V) -> R,
    {
        let hash = self.hash_key(key);
        let shard = self.storage.shard_for_hash(hash);
        let table = shard.table.read();

        table.find(hash, |(k, _)| k.borrow() == key).map(|bucket| {
            let (k, v) = bucket;
            f(k, v)
        })
    }
}

impl<K, V, S> MutableMap<K, V> for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync + Clone + PartialEq,
    S: BuildHasher + Send + Sync,
{
    type Guard<'a> = LockedGuard<'a, K, V, Self> where Self: 'a;

    fn get_mut<'a, Q>(&'a self, _: &Q) -> Option<Self::Guard<'a>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        unimplemented!("Use `alter` or `alter_entry` methods instead of `get_mut`.");
    }
}

impl<K, V, S> MutableInPlaceMap<K, V> for LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    type AlterResult<R> = Option<R>;

    fn alter<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&mut V) -> R,
    {
        let hash = self.hash_key(key);
        let shard = self.storage.shard_for_hash(hash);
        let mut table = shard.table.write();

        table
            .find_mut(hash, |(k, _)| k.borrow() == key)
            .map(|bucket| {
                f(&mut bucket.1)
            })
    }

    fn alter_entry<F, D>(&self, key: K, default: D, f: F)
    where
        F: FnOnce(&mut V),
        D: FnOnce() -> V,
    {
        let hash = self.hash_key(&key);
        let shard = self.storage.shard_for_hash(hash);
        let mut table = shard.table.write();

        let entry = table.entry(hash, |(k_ref, _)| k_ref == &key, |(k, _)| self.hash_key(k));

        match entry {
            Entry::Occupied(mut occ) => {
                f(&mut occ.get_mut().1);
            }
            Entry::Vacant(vac) => {
                let mut value = default();
                f(&mut value);
                vac.insert((key, value));
                self.storage.shard_increment(1);
            }
        }
    }
}

impl<K, V, S> Iterator for LockedMap<K, V ,S>
where
    K: Hash + Eq + Send + Sync + Clone,
    V: Clone,
    S: BuildHasher + Send + Sync,
{
    type Item = (K, MaybeArc<V>);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.storage.shards.iter().find_map(|shard| {
            let guard = shard.table.read();
            guard
                .iter()
                .next()
                .map(|(k, v)| (k.clone(), MaybeArc::Owned(v.clone())))
        })
    }
}

// Add view method directly to LockedConcurrentMap for compatibility
impl<K, V, S> LockedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    /// Remove and return the entire entry associated with the key.
    ///
    /// # Arguments
    /// * `key` - The key to remove
    ///
    /// # Returns
    /// The key-value pair that was removed, if the key existed
    pub fn remove_entry<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        let hash = self.hash_key(key);
        let shard = self.storage.shard_for_hash(hash);
        let mut table = shard.table.write();
        if let Ok(entry) = table.find_entry(hash, |(k, _v)| key.equivalent(k)) {
            let ((k, v), _) = entry.remove();
            self.storage.shard_decrement(1);
            Some((k, v))
        } else {
            None
        }
    }

    /// Clear all entries from the map.
    pub fn clear(&self) {
        for shard in self.storage.shards.iter() {
            let mut table = shard.table.write();
            self.storage.shard_decrement(table.len());
            table.clear();
        }
    }
}

// Builder pattern support
pub struct LockedMapBuilder<S = DefaultHashBuilder> {
    shards: usize,
    capacity: usize,
    hash_builder: Option<S>,
}

impl<S> Default for LockedMapBuilder<S>
where
    S: BuildHasher + Default + Send + Sync,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> LockedMapBuilder<S>
where
    S: BuildHasher + Default + Send + Sync,
{
    /// Create a new builder with default settings.
    ///
    /// # Returns
    /// A new builder instance
    pub fn new() -> Self {
        Self {
            shards: DEFAULT_SHARDS,
            capacity: 0,
            hash_builder: None,
        }
    }
    /// Set a custom hasher for the map.
    ///
    /// # Arguments
    /// * `hasher` - The hash builder to use
    ///
    /// # Returns
    /// The builder instance for method chaining
    pub fn with_hasher(mut self, hasher: S) -> Self {
        self.hash_builder = Some(hasher);
        self
    }

    /// Set the number of shards. Must be a power of two.
    ///
    /// # Arguments
    /// * `shards` - The number of shards
    ///
    /// # Returns
    /// The builder instance for method chaining
    ///
    /// # Panics
    /// Panics if `shards` is not a power of two
    pub fn with_shards(mut self, shards: usize) -> Self {
        assert!(
            shards.is_power_of_two(),
            "Number of shards must be a power of two"
        );
        self.shards = shards;
        self
    }

    /// Set the per-shard capacity.
    ///
    /// # Arguments
    /// * `capacity` - The initial capacity per shard
    ///
    /// # Returns
    /// The builder instance for method chaining
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Build the LockedConcurrentMap with the specified parameters.
    ///
    /// # Returns
    /// A new LockedConcurrentMap instance
    pub fn build<K, V>(self) -> LockedMap<K, V, S>
    where
        K: Hash + Eq + Send + Sync,
        V: Send + Sync,
    {
        LockedMap::with_shards_and_capacity_and_hasher(
            self.shards,
            self.capacity,
            self.hash_builder.unwrap_or_default(),
        )
    }
}
