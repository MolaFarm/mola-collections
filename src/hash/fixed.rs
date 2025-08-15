use core::hash::{BuildHasher, Hash};
use hashbrown::DefaultHashBuilder;

enum Bucket<K, V> {
    Empty,
    Deleted,
    Occupied { key: K, value: V },
}

pub struct FixedMap<K, V, const CAP: usize, S = DefaultHashBuilder>
where
    S: BuildHasher,
{
    buckets: [Bucket<K, V>; CAP],
    len: usize,
    hasher_builder: S,
}

impl<K, V, const CAP: usize> Default for FixedMap<K, V, CAP>
where
    K: Eq + Hash,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, const CAP: usize> FixedMap<K, V, CAP>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        assert!(CAP.is_power_of_two(), "CAP must be a power of two");
        Self {
            buckets: [const { Bucket::Empty }; CAP],
            len: 0,
            hasher_builder: DefaultHashBuilder::default(),
        }
    }
}

impl<K, V, const CAP: usize, S> FixedMap<K, V, CAP, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn hash_index(&self, key: &K) -> usize {
        
        
        (self.hasher_builder.hash_one(key) as usize) & (CAP - 1)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        assert!(self.len < CAP, "FixedMap is full");
        let mut idx = self.hash_index(&key);
        let mut first_deleted: Option<usize> = None;

        loop {
            match &mut self.buckets[idx] {
                Bucket::Empty => {
                    let target = first_deleted.unwrap_or(idx);
                    self.buckets[target] = Bucket::Occupied { key, value };
                    self.len += 1;
                    return None;
                }
                Bucket::Deleted => {
                    if first_deleted.is_none() {
                        first_deleted = Some(idx);
                    }
                }
                Bucket::Occupied { key: ek, value: ev } => {
                    if ek == &key {
                        let old = core::mem::replace(ev, value);
                        *ek = key;
                        return Some(old);
                    }
                }
            }
            idx = (idx + 1) & (CAP - 1);
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut idx = self.hash_index(key);
        loop {
            match &self.buckets[idx] {
                Bucket::Empty => return None,
                Bucket::Deleted => {}
                Bucket::Occupied { key: ek, value: ev } if ek == key => {
                    return Some(ev);
                }
                _ => {}
            }
            idx = (idx + 1) & (CAP - 1);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut idx = self.hash_index(key);
        loop {
            match &mut self.buckets[idx] {
                Bucket::Empty => return None,
                Bucket::Deleted => {}
                Bucket::Occupied { key: ek, .. } if ek == key => {
                    if let Bucket::Occupied { key: _, value } =
                        core::mem::replace(&mut self.buckets[idx], Bucket::Deleted)
                    {
                        self.len -= 1;
                        return Some(value);
                    }
                    unreachable!()
                }
                _ => {}
            }
            idx = (idx + 1) & (CAP - 1);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == CAP
    }
}

#[cfg(test)]
mod tests {
    use super::FixedMap;

    #[test]
    fn test_insert_and_get() {
        let mut map: FixedMap<_, _, 8> = FixedMap::new();
        assert!(map.is_empty());
        assert_eq!(map.insert(1, "a"), None);
        assert_eq!(map.insert(2, "b"), None);
        assert_eq!(map.get(&1), Some(&"a"));
        assert_eq!(map.get(&2), Some(&"b"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_overwrite() {
        let mut map: FixedMap<_, _, 4> = FixedMap::new();
        map.insert("key", 10);
        assert_eq!(map.insert("key", 20), Some(10));
        assert_eq!(map.get(&"key"), Some(&20));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut map: FixedMap<_, _, 4> = FixedMap::new();
        map.insert(5, "x");
        assert_eq!(map.remove(&5), Some("x"));
        assert_eq!(map.get(&5), None);
        assert!(map.is_empty());
    }

    #[test]
    #[should_panic(expected = "FixedMap is full")]
    fn test_is_full() {
        let mut map: FixedMap<_, _, 2> = FixedMap::new();
        map.insert(1, 1);
        map.insert(2, 2);
        assert!(map.is_full());
        map.insert(3, 3);
    }

    #[test]
    fn test_probe_after_remove() {
        let mut map: FixedMap<_, _, 4> = FixedMap::new();
        map.insert(1, "one");
        map.insert(5, "five");
        assert_eq!(map.remove(&1), Some("one"));
        assert_eq!(map.get(&5), Some(&"five"));
    }
}
