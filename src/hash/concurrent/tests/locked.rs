extern crate std;
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use std::{thread, vec};

use crate::hash::concurrent::locked::LockedMapBuilder;

use super::super::prelude::*;
use super::super::locked::LockedMap;

#[test]
fn test_alter_entry_updates_count() {
    let map: LockedMap<i32, String> = LockedMap::new();

    assert_eq!(map.len(), 0);

    // alter_entry should increment count when creating new entry
    map.alter_entry(
        1,
        || "one".to_string(),
        |v| {
            v.push_str("_modified");
        },
    );
    assert_eq!(map.len(), 1);

    // alter_entry should not change count when modifying existing entry
    map.alter_entry(
        1,
        || "default".to_string(),
        |v| {
            v.push_str("_again");
        },
    );
    assert_eq!(map.len(), 1);

    // Verify the value was actually modified
    let result = map.view(&1, |_, v| v.clone());
    assert!(result.unwrap().contains("one_modified_again"));
}

#[test]
fn test_locked_multishard_len() {
    // Create a map with many shards and verify counting works correctly
    let map = LockedMap::with_shards_and_capacity_and_hasher(
        32,
        0,
        hashbrown::DefaultHashBuilder::default(),
    );

    // Insert values that should go to different shards
    for i in 0..100 {
        map.insert(i, format!("value_{}", i));
    }

    assert_eq!(map.len(), 100);
    assert!(!map.is_empty());

    // Remove half the values
    for i in 0..50 {
        map.remove(&i);
    }

    assert_eq!(map.len(), 50);
    assert!(!map.is_empty());

    // Clear all
    map.clear();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn test_locked_len_and_is_empty() {
    let map: LockedMap<i32, String> = LockedMap::new();

    // Test empty map
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());

    // Insert some values
    map.insert(1, "one".to_string());
    assert_eq!(map.len(), 1);
    assert!(!map.is_empty());

    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());
    assert_eq!(map.len(), 3);
    assert!(!map.is_empty());

    // Test overwrite (should not change count)
    let old_value = map.insert(2, "TWO".to_string());
    assert!(old_value.is_some());
    assert_eq!(map.len(), 3);

    // Remove values
    let removed = map.remove(&2);
    assert!(removed.is_some());
    assert_eq!(map.len(), 2);

    map.remove(&1);
    map.remove(&3);
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn test_view() {
    let map = LockedMap::<String, i32>::new();
    map.insert("key1".to_string(), 100);

    // View an existing value
    let result = map.view("key1", |k, v| {
        assert_eq!(*k, "key1");
        *v + 1
    });
    assert_eq!(result, Some(101));

    // View a non-existent key
    let result_none = map.view("key2", |_, _| ());
    assert!(result_none.is_none());
}

#[test]
fn test_alter() {
    let map = LockedMap::<String, i32>::new();
    map.insert("key1".to_string(), 50);

    // Modify an existing value
    let success = map.alter("key1", |v| *v *= 2);
    assert!(success.is_some());
    assert_eq!(map.view("key1", |_, v| *v), Some(100));

    // Attempt to modify a non-existent key
    let failure = map.alter("key2", |v: &mut i32| *v = 0);
    assert!(failure.is_none());
}

#[test]
fn test_alter_entry() {
    let map = LockedMap::<String, i32>::new();

    // Apply alter_entry on a new key
    map.alter_entry("new_key".to_string(), || 0, |v| *v += 10);
    assert_eq!(map.view("new_key", |_, v| *v), Some(10));
    assert_eq!(map.len(), 1);

    // Apply alter_entry on an existing key
    map.alter_entry("new_key".to_string(), || 0, |v| *v += 5);
    assert_eq!(map.view("new_key", |_, v| *v), Some(15));
    assert_eq!(map.len(), 1);
}

#[test]
fn test_raw_hash_map_trait() {
    let map = LockedMap::<String, i32>::new();

    // Test insert
    let prev = map.insert("key1".to_string(), 100);
    assert!(prev.is_none());

    let prev = map.insert("key1".to_string(), 200);
    assert_eq!(prev, Some(MaybeArc::Owned(100)));

    // Test contains_key
    assert!(map.contains_key("key1"));
    assert!(!map.contains_key("key2"));

    // Test remove
    let removed = map.remove("key1");
    assert_eq!(removed, Some(MaybeArc::Owned(200)));
    assert!(!map.contains_key("key1"));

    let removed = map.remove("key1");
    assert!(removed.is_none());
}

#[test]
fn test_builder_pattern() {
    let map: LockedMap<String, i32> = LockedMapBuilder::new()
        .with_shards(16)
        .with_capacity(100)
        .build();

    assert_eq!(map.shard_count(), 16);

    map.insert("test".to_string(), 42);
    assert_eq!(map.view("test", |_, v| *v), Some(42));
}

#[test]
fn test_concurrency() {
    let shard_count =
        (std::thread::available_parallelism().map_or(1, usize::from) * 4).next_power_of_two();
    let map: Arc<LockedMap<usize, usize>> = Arc::new(
        LockedMapBuilder::new()
            .with_shards(shard_count)
            .with_capacity(10000)
            .build(),
    );
    let num_threads = 8;
    let items_per_thread = 1000;

    let mut handles = vec![];

    // Insertion phase
    for i in 0..num_threads {
        let map_clone = Arc::clone(&map);
        let handle = thread::spawn(move || {
            for j in 0..items_per_thread {
                let key = i * items_per_thread + j;
                map_clone.insert(key, key);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(map.len(), num_threads * items_per_thread);

    handles = vec![];

    // Atomic modification phase
    for i in 0..num_threads {
        let map_clone = Arc::clone(&map);
        let handle = thread::spawn(move || {
            for j in 0..items_per_thread {
                let key = i * items_per_thread + j;
                // Increment each value by 1
                map_clone.alter(&key, |v| *v += 1);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }

    // Verification and removal phase
    for i in 0..num_threads {
        for j in 0..items_per_thread {
            let key = i * items_per_thread + j;
            let expected_value = key + 1;
            // Verify the value is correct
            assert_eq!(map.view(&key, |_, v| *v), Some(expected_value));
            // Remove the entry
            let (k, v) = map.remove_entry(&key).unwrap();
            assert_eq!(k, key);
            assert_eq!(v, expected_value);
        }
    }

    assert!(map.is_empty());
}
