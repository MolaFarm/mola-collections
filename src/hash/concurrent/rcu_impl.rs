use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

use arc_swap::ArcSwap;
use crossbeam_utils::CachePadded;
use hashbrown::DefaultHashBuilder;
use rpds::{HashTrieMap, HashTrieMapSync};

use crate::hash::concurrent::wrapper::MaybeArc;

use super::traits::{RawHashMap, ReadableMap, ShardStorage, MutableMap, AtomicSet, MutableGuard, MutableInPlaceMap, ReadableInPlaceMap};
use super::wrapper::ConcurrentMap;

/// A simple backoff strategy for spin-then-yield.
/// This helps reduce contention during high-frequency CAS loops.
#[inline]
fn backoff(step: &mut usize) {
    if *step < 10 {
        // Spin for a few iterations, doubling each time.
        (0..1 << *step).for_each(|_| core::hint::spin_loop());
        *step += 1;
    } else {
        (0..1 << 10).for_each(|_| {
            core::hint::spin_loop();
        });
    }
}

pub struct Mutable<'a, K, V, M>
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Clone + Send + Sync + 'a,
    M: RawHashMap<K, V> + AtomicSet<K, V>,
{
    map: &'a M,
    key: K,
    value_arc: Arc<V>,
    value: V,
}

impl<'a, K, V, M> Deref for Mutable<'a, K, V, M>
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Clone + Send + Sync + 'a,
    M: RawHashMap<K, V> + AtomicSet<K, V>,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, K, V, M> DerefMut for Mutable<'a, K, V, M>
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Clone + Send + Sync + 'a,
    M: RawHashMap<K, V> + AtomicSet<K, V>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, K, V, M> MutableGuard<'a, K, V> for Mutable<'a, K, V, M>
where
    K: Hash + Eq + Send + Sync + 'a,
    V: Clone + Send + Sync + 'a,
    M: RawHashMap<K, V> + AtomicSet<K, V>,
{
    fn commit(self) -> Result<(), ()> {
        if self
            .map
            .compare_and_set(&self.key, self.value_arc, Arc::new(self.value))
        {
            // Successfully updated the map with the new value.
            Ok(())
        } else {
            // The CAS failed, meaning another thread modified the value.
            Err(())
        }
    }
}

/// A single shard of the RCU hash table.
/// It now holds a swappable Arc pointer, managed safely by ArcSwap.
pub struct RcuShard<K, V> {
    pub(crate) table: ArcSwap<HashTrieMapSync<K, Arc<V>>>,
}

impl<K, V> Default for RcuShard<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self {
            // Initialize with an empty map. ArcSwap handles wrapping it in an Arc.
            table: ArcSwap::from_pointee(HashTrieMap::new_sync()),
        }
    }
}

/// Storage implementation for RCU-based concurrent hash maps.
///
/// This storage uses ArcSwap to provide lock-free reads
/// and efficient copy-on-write updates, without a separate GC mechanism.
pub struct RcuStorage<K, V> {
    shards: Box<[CachePadded<RcuShard<K, V>>]>,
    /// Atomic counter for the number of objects in the storage
    count: AtomicUsize,
}

// RcuStorage no longer needs a custom Drop impl, as ArcSwap handles everything.

impl<K, V> RcuStorage<K, V>
where
    K: Eq + Hash,
{
    /// Create new RCU storage with the specified number of shards and pinner function.
    ///
    /// # Arguments
    /// * `shards` - The number of shards (must be a power of two)
    ///
    /// # Returns
    /// A new RCU storage instance
    ///
    /// # Panics
    /// Panics if `shards` is not a power of two
    pub fn with_shards(shards: usize) -> Self {
        assert!(
            shards.is_power_of_two(),
            "Number of shards must be a power of two"
        );
        let mut shard_vec = Vec::with_capacity(shards);
        for _ in 0..shards {
            shard_vec.push(CachePadded::new(RcuShard::default()));
        }
        Self {
            shards: shard_vec.into_boxed_slice(),
            count: AtomicUsize::new(0),
        }
    }
}

// Default number of shards. Must be a power of two.
const DEFAULT_SHARDS: usize = 32;

impl<K, V> ShardStorage<K, V> for RcuStorage<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
{
    type Shard = RcuShard<K, V>;

    fn shard_for_hash(&self, hash: u64) -> &CachePadded<Self::Shard> {
        &self.shards[hash as usize & (self.shards.len() - 1)]
    }

    fn shard_count(&self) -> usize {
        self.shards.len()
    }

    fn shard_increment(&self, num: usize) {
        self.count.fetch_add(num, Ordering::Relaxed);
    }

    fn shard_decrement(&self, num: usize) {
        self.count.fetch_sub(num, Ordering::Relaxed);
    }

    fn shard_len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    fn shard_is_empty(&self) -> bool {
        self.shard_len() == 0
    }
}

/// Type alias for a RCU-based concurrent hash map using the standard configuration.
pub type HamtMap<K, V, S = DefaultHashBuilder> = ConcurrentMap<K, V, S, RcuStorage<K, V>>;

impl<K, V, S> HamtMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Default + Send + Sync,
{
    /// Create a new RCU concurrent map.
    ///
    /// # Returns
    /// A new RCU concurrent map instance
    pub fn new() -> Self {
        Self::with_shards_and_hasher(DEFAULT_SHARDS, Default::default())
    }
}

impl<K, V, S> HamtMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    /// Create a new RCU concurrent map with custom settings.
    ///
    /// # Arguments
    /// * `shards` - The number of shards (must be a power of two)
    /// * `hash_builder` - The hash builder to use
    ///
    /// # Returns
    /// A new RCU concurrent map instance
    ///
    /// # Panics
    /// Panics if `shards` is not a power of two
    pub fn with_shards_and_hasher(shards: usize, hash_builder: S) -> Self {
        let storage = RcuStorage::with_shards(shards);
        ConcurrentMap::with_storage_and_hasher(storage, hash_builder)
    }
}

impl<K, V, S> Default for HamtMap<K, V, S>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Default + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, S> RawHashMap<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    fn insert(&self, key: K, value: V) -> Option<MaybeArc<V>> {
        let shard = self.shard_for_key(&key);
        let value = Arc::new(value);

        let mut backoff_step = 0;
        loop {
            // Load the current Arc pointer to the map. This is cheap and safe.
            let old_arc = shard.table.load();
            let new_table = old_arc.insert(key.clone(), value.clone());
            let new_arc = Arc::new(new_table);

            // `compare_and_swap` atomically swaps the pointer if the content matches.
            // It returns the Arc that was in the map before the swap.
            // We compare its pointer to the old_arc's pointer to see if we succeeded.
            if Arc::ptr_eq(&old_arc, &shard.table.compare_and_swap(&old_arc, new_arc)) {
                // Success! ArcSwap handles the safe reclamation of the old Arc.
                let old_val = old_arc.get(&key).cloned();
                if old_val.is_none() {
                    // If it was a new key, increment the count.
                    self.storage.shard_increment(1);
                    return None;
                } else {
                    // If we replaced a key, return the old value.
                    return Some(MaybeArc::Shared(old_val.unwrap()));
                }
            } else {
                // CAS failed, another thread won the race. Backoff and retry.
                backoff(&mut backoff_step);
            }
        }
    }

    fn remove<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: ?Sized + Eq + Hash,
    {
        let shard = self.shard_for_key(key);

        let mut backoff_step = 0;
        loop {
            let old_arc = shard.table.load();

            // First, check if the key exists. If not, we can exit early.
            if !old_arc.contains_key(key) {
                return None;
            }

            let old_val = old_arc.get(key).cloned();
            let new_table = old_arc.remove(key);
            let new_arc = Arc::new(new_table);

            if Arc::ptr_eq(&old_arc, &shard.table.compare_and_swap(&old_arc, new_arc)) {
                // Successfully removed. Decrement count and return the old value.
                self.storage.shard_decrement(1);
                return old_val.map(MaybeArc::Shared);
            } else {
                backoff(&mut backoff_step);
            }
        }
    }

    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        let shard = self.shard_for_key(key);
        shard.table.load_full().contains_key(key)
    }

    fn len(&self) -> usize {
        self.storage.shard_len()
    }

    fn is_empty(&self) -> bool {
        self.storage.shard_is_empty()
    }
}

impl<K, V, S> Iterator for HamtMap<K, V, S>
where
    K: Eq + Hash + Clone,
    S: BuildHasher,
{
    type Item = (K, MaybeArc<V>);

    fn next(&mut self) -> Option<Self::Item> {
        // Use the iterator from the underlying storage.
        self.storage.shards.iter().find_map(|shard| {
            let table_arc = shard.table.load_full();
            table_arc
                .iter()
                .next()
                .map(|(k, v)| (k.clone(), MaybeArc::Shared(v.clone())))
        })
    }
}

impl<K, V, S> ReadableMap<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    fn get<Q>(&self, key: &Q) -> Option<MaybeArc<V>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        // Read path is extremely simple and safe. `load_full` returns a full Arc.
        let shard = self.shard_for_key(key);
        let table_arc = shard.table.load_full();
        table_arc.get(key).map(Arc::clone).map(MaybeArc::Shared)
    }
}

impl<K, V, S> ReadableInPlaceMap<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    type ReadResult<R> = Option<R>;

    fn view<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&K, &V) -> R,
    {
        let shard = self.shard_for_key(key);
        let table_arc = shard.table.load_full();
        table_arc
            .get_key_value(key)
            .map(|(k, arc_v)| f(k, arc_v.as_ref()))
    }
}

impl<K, V, S> AtomicSet<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Send + Sync,
    S: BuildHasher + Send + Sync,
{
    fn compare_and_set(&self, key: &K, old_value: Arc<V>, new_value: Arc<V>) -> bool {
        let shard = self.shard_for_key(key);
        let mut backoff_step = 0;

        loop {
            let old_arc = shard.table.load();
            if let Some(current_value) = old_arc.get(key) {
                if Arc::ptr_eq(current_value, &old_value) {
                    // Perform the CAS operation
                    let new_table = old_arc.insert(key.clone(), new_value.clone());
                    let new_arc = Arc::new(new_table);

                    if Arc::ptr_eq(&old_arc, &shard.table.compare_and_swap(&old_arc, new_arc)) {
                        return true; // CAS succeeded
                    }
                } else {
                    // Current value does not match old_value, cannot update
                    return false;
                }
            } else {
                // Key does not exist, cannot update
                return false;
            }

            // CAS failed, backoff and retry
            backoff(&mut backoff_step);
        }
    }
}

impl<K, V, S> MutableMap<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
    S: BuildHasher + Send + Sync,
{
    type Guard<'a> = Mutable<'a, K, V, Self> where Self: 'a;

    fn get_mut<'a, Q>(&'a self, key: &Q) -> Option<Self::Guard<'a>>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        let shard = self.shard_for_key(key);
        let table_arc = shard.table.load_full();
        table_arc
            .get_key_value(key)
            .map(|(k, v)| {
                let value_arc = Arc::clone(v);
                let value = v.as_ref().clone();
                Mutable {
                    map: self,
                    key: k.clone(),
                    value_arc,
                    value,
                }
            })
    }
}

impl<K, V, S> MutableInPlaceMap<K, V> for HamtMap<K, V, S>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
    S: BuildHasher + Send + Sync,
{
    type AlterResult<R> = Option<R>;

    fn alter<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
        F: FnOnce(&mut V) -> R,
    {
        self.get_mut(key)
            .map(|mut guard| {
                let ret = f(&mut guard);
                if guard.commit().is_ok() {
                    Some(ret)
                } else {
                    None
                }
            })
            .unwrap_or(None)
    }

    fn alter_entry<F, D>(&self, key: K, default: D, f: F)
    where
        F: FnOnce(&mut V),
        D: FnOnce() -> V,
    {
        self.get_mut(&key)
            .map(|mut guard| {
                f(&mut guard);
                guard.commit().is_ok()
            })
            .unwrap_or_else(|| {
                // If the key was not found, insert a new entry with the default value
                let value = default();
                self.insert(key, value).is_none()
            });
    }
}

// Safety: RcuStorage can be safely sent and shared across threads
// when its components are Send + Sync. This is true because ArcSwap is
// Send + Sync if the underlying T is Send + Sync.
unsafe impl<K, V> Send for RcuStorage<K, V>
where
    K: Send + Sync,
    V: Send + Sync,
{
}

unsafe impl<K, V> Sync for RcuStorage<K, V>
where
    K: Send + Sync,
    V: Send + Sync,
{
}
