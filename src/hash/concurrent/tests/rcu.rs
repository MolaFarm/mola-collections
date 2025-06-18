extern crate std;
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, Ordering};
use crossbeam_utils::thread as scoped_thread;
use rand::{Rng, seq::SliceRandom, thread_rng};
use std::sync::Barrier;
use std::thread;
use std::vec;

use super::super::prelude::*;
use super::super::rcu::HamtMap;

#[test]
fn test_rcu_len_and_is_empty() {
    let map: HamtMap<i32, String> = HamtMap::new();

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
fn test_rcu_multishard_len() {
    // Create a map with many shards and verify counting works correctly
    let map =
        HamtMap::with_shards_and_hasher(32, hashbrown::DefaultHashBuilder::default());

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
}

#[test]
fn test_rcu_map_two_threads_mixed() {
    const SAMPLE_SIZE: usize = 10_000;

    // Initialize the map and pre-insert SAMPLE_SIZE elements
    let map: Arc<HamtMap<usize, String>> = Arc::new(HamtMap::new());
    for i in 0..SAMPLE_SIZE {
        map.insert(i, format!("value{}", i));
    }

    // Prepare the synchronization barrier and a shuffled list of keys
    let barrier = Arc::new(Barrier::new(2));
    let mut keys: Vec<usize> = (0..SAMPLE_SIZE).collect();
    keys.shuffle(&mut thread_rng());
    let keys = Arc::new(keys);

    // Mixed workload: 50/50 read/write ratio
    let write_ratio = 0.5;

    // Spawn two threads to perform concurrent reads and writes
    scoped_thread::scope(|s| {
        for _ in 0..2 {
            let map = Arc::clone(&map);
            let barrier = Arc::clone(&barrier);
            let keys: Arc<Vec<usize>> = Arc::clone(&keys);

            s.spawn(move |_| {
                // Wait until both threads are ready
                barrier.wait();

                let mut rng = thread_rng();
                // Each thread performs SAMPLE_SIZE / 2 operations
                for i in 0..(SAMPLE_SIZE / 2) {
                    let key = &keys[i % keys.len()];
                    if rng.gen_bool(write_ratio) {
                        // Write operation: update the value
                        map.insert(*key, format!("new_value_{}", i));
                    } else {
                        // Read operation: ensure the key exists
                        let _ = map.get(key).expect("key should exist");
                    }
                }
            });
        }
    })
    .expect("failed to run threads");

    // After all concurrent operations, verify all original keys are still readable
    for i in 0..SAMPLE_SIZE {
        let val = map
            .get(&i)
            .unwrap_or_else(|| panic!("Key {} does not exist", i));
        // Ensure the value is not empty; it may be the original or updated value
        assert!(!val.is_empty(), "Value for key {} should not be empty", i);
    }
}

#[test]
fn test_simple_api() {
    // Use default pinner (epoch::pin)
    let map = HamtMap::<String, i32>::new();

    // API calls no longer need guard
    map.insert("key1".to_string(), 123);
    let val = map.get("key1");
    assert_eq!(*val.unwrap().as_ref(), 123);

    let removed = map.remove("key1");
    assert_eq!(*removed.unwrap().as_ref(), 123);
    assert!(map.get("key1").is_none());
}

#[test]
fn test_view_trait() {
    let map = HamtMap::<String, i32>::new();

    map.insert("key1".to_string(), 42);

    // Test the ReadOnlyView trait
    let result = map.view("key1", |k, v| {
        assert_eq!(*k, "key1");
        *v + 1
    });
    assert_eq!(result, Some(43));

    // Test view on non-existent key
    let result = map.view("key2", |_, _| 0);
    assert!(result.is_none());
}

#[test]
fn test_raw_hash_map_trait() {
    let map = HamtMap::<String, i32>::new();

    // Test insert
    let prev = map.insert("key1".to_string(), 100);
    assert!(prev.is_none());

    // Test contains_key
    assert!(map.contains_key("key1"));
    assert!(!map.contains_key("key2"));

    // Test remove
    let removed = map.remove("key1");
    assert_eq!(*removed.unwrap().as_ref(), 100);

    // But the key should be gone
    assert!(!map.contains_key("key1"));
}
#[test]
fn test_concurrency() {
    // In concurrent tests, each thread will use the global `epoch::pin`
    let map = Arc::new(HamtMap::<usize, usize>::new());
    let writer_done = Arc::new(AtomicBool::new(false));

    let map_clone_writer = Arc::clone(&map);
    let writer_done_clone = Arc::clone(&writer_done);

    // Writer thread
    let writer_handle = thread::spawn(move || {
        for i in 0..100 {
            // Use insert for the new API
            map_clone_writer.insert(i, i);
        }
        writer_done_clone.store(true, Ordering::SeqCst);
    });

    let mut reader_handles = vec![];
    // Multiple reader threads
    for _ in 0..4 {
        let map_clone_reader = Arc::clone(&map);
        let writer_done_clone_reader = Arc::clone(&writer_done);

        let reader_handle = thread::spawn(move || {
            while !writer_done_clone_reader.load(Ordering::SeqCst) {
                for i in 0..100 {
                    // Use get for the new API
                    let _ = map_clone_reader.get(&i);
                }
            }
        });
        reader_handles.push(reader_handle);
    }

    writer_handle.join().unwrap();
    for handle in reader_handles {
        handle.join().unwrap();
    }

    // Verify all keys were inserted correctly
    for i in 0..100 {
        let value = map.get(&i);
        assert!(value.is_some(), "Key {} should exist", i);
        assert_eq!(*value.unwrap(), i, "Value for key {} should be {}", i, i);
    }
}

#[test]
fn test_mixed_operations() {
    let map = HamtMap::<String, i32>::new();

    // Insert some values
    for i in 0..10 {
        map.insert(format!("key{}", i), i);
    }

    // Check they all exist
    for i in 0..10 {
        assert!(map.contains_key(&format!("key{}", i)));
        let val = map.get(&format!("key{}", i));
        assert_eq!(*val.unwrap().as_ref(), i);
    }

    // Remove half of them
    for i in 0..5 {
        assert_eq!(*map.remove(&format!("key{}", i)).unwrap().as_ref(), i);
    }

    // Check the state
    for i in 0..5 {
        assert!(!map.contains_key(&format!("key{}", i)));
    }
    for i in 5..10 {
        assert!(map.contains_key(&format!("key{}", i)));
    }
}

#[test]
fn test_view_operations() {
    let map = HamtMap::<String, String>::new();

    map.insert("hello".to_string(), "world".to_string());

    // Use view to read and transform
    let result = map.view("hello", |k, v| format!("{}:{}", k, v));
    assert_eq!(result, Some("hello:world".to_string()));

    // Use view to check length
    let len = map.view("hello", |_, v| v.len());
    assert_eq!(len, Some(5));
}

#[test]
fn test_get_mut() {
    use crate::hash::concurrent::traits::{MutableGuard, MutableMap};

    let map = HamtMap::<String, i32>::new();

    // Insert some values
    map.insert("key1".to_string(), 42);
    map.insert("key2".to_string(), 100);

    // Test get_mut on existing key
    if let Some(mut guard) = map.get_mut("key1") {
        assert_eq!(*guard, 42);
        *guard = 99; // Modify the value
        assert!(guard.commit().is_ok()); // Commit the change
    } else {
        panic!("Expected to find key1");
    }

    // Verify the change was persisted
    let result = map.get("key1");
    assert_eq!(*result.unwrap().as_ref(), 99);

    // Test get_mut on non-existent key
    let result = map.get_mut("nonexistent");
    assert!(result.is_none());

    // Test alter method (which uses get_mut internally)
    let success = map.alter("key2", |v| *v *= 2);
    assert!(success.is_some());

    // Verify alter worked
    let result = map.get("key2");
    assert_eq!(*result.unwrap().as_ref(), 200);

    // Test alter on non-existent key
    let success = map.alter("nonexistent", |v| *v = 0);
    assert!(success.is_none());
}
