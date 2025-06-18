use mola_collections::hash::concurrent::{prelude::*, LockedMap, RcuMap};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::prelude::SliceRandom;
use rand::{Rng, thread_rng};
use std::sync::{Arc, Barrier};
use std::thread;

const SAMPLE_SIZE: usize = 10_000;

// Enum to define the workload mix
enum Workload {
    WriteHeavy, // 80% writes, 20% reads
    ReadHeavy,  // 20% writes, 80% reads
    Mixed,      // 50% writes, 50% reads
}

impl Workload {
    fn get_mix(&self) -> (u32, u32) {
        match self {
            Workload::WriteHeavy => (80, 20),
            Workload::ReadHeavy => (20, 80),
            Workload::Mixed => (50, 50),
        }
    }
}

// --- Benchmark for LockedConcurrentMap ---

fn locked_map_benchmark(c: &mut Criterion, map_name: &str, threads: usize, workload: Workload) {
    let mut group = c.benchmark_group(format!("{}_{}_threads", map_name, threads));
    let (write_ratio, _) = workload.get_mix();
    let workload_name = match workload {
        Workload::WriteHeavy => "write_heavy",
        Workload::ReadHeavy => "read_heavy",
        Workload::Mixed => "mixed",
    };

    let map: Arc<LockedMap<String, String>> =
        Arc::new(LockedMap::new());

    for i in 0..SAMPLE_SIZE {
        map.insert(format!("key{}", i), format!("value{}", i));
    }

    group.throughput(Throughput::Elements(SAMPLE_SIZE as u64));

    group.bench_function(BenchmarkId::new(workload_name, SAMPLE_SIZE), |b| {
        b.iter_with_setup(
            || {
                let map_clone = Arc::clone(&map);
                let barrier = Arc::new(Barrier::new(threads));
                let mut keys: Vec<String> = (0..SAMPLE_SIZE).map(|i| format!("key{}", i)).collect();
                keys.shuffle(&mut thread_rng());
                (map_clone, barrier, Arc::new(keys))
            },
            |(map_clone, barrier, keys)| {
                thread::scope(|s| {
                    for _ in 0..threads {
                        let map_clone = Arc::clone(&map_clone);
                        let barrier = Arc::clone(&barrier);
                        let keys = Arc::clone(&keys);

                        s.spawn(move || {
                            let mut rng = thread_rng();
                            barrier.wait();
                            for i in 0..SAMPLE_SIZE / threads {
                                let key = &keys[i % keys.len()];
                                let random_val = rng.gen_range(0..100);

                                if random_val < write_ratio {
                                    map_clone.insert(key.clone(), format!("new_value{}", i));
                                } else {
                                    black_box(map_clone.view(key, |_, v| v.clone()));
                                }
                            }
                        });
                    }
                });
            },
        );
    });

    group.finish();
}

// --- Benchmark for RcuConcurrentMap (ConcurrentRcuMap) ---

fn rcu_map_benchmark(c: &mut Criterion, map_name: &str, threads: usize, workload: Workload) {
    let mut group = c.benchmark_group(format!("{}_{}_threads", map_name, threads));
    let (write_ratio, _) = workload.get_mix();
    let workload_name = match workload {
        Workload::WriteHeavy => "write_heavy",
        Workload::ReadHeavy => "read_heavy",
        Workload::Mixed => "mixed",
    };

    let map: Arc<RcuMap<String, String>> =
        Arc::new(RcuMap::new());

    for i in 0..SAMPLE_SIZE {
        map.insert(format!("key{}", i), format!("value{}", i));
    }

    group.throughput(Throughput::Elements(SAMPLE_SIZE as u64));

    group.bench_function(BenchmarkId::new(workload_name, SAMPLE_SIZE), |b| {
        b.iter_with_setup(
            || {
                let map_clone = Arc::clone(&map);
                let barrier = Arc::new(Barrier::new(threads));
                let mut keys: Vec<String> = (0..SAMPLE_SIZE).map(|i| format!("key{}", i)).collect();
                keys.shuffle(&mut thread_rng());
                (map_clone, barrier, Arc::new(keys))
            },
            |(map_clone, barrier, keys)| {
                thread::scope(|s| {
                    for _ in 0..threads {
                        let map_clone = Arc::clone(&map_clone);
                        let barrier = Arc::clone(&barrier);
                        let keys = Arc::clone(&keys);

                        s.spawn(move || {
                            let mut rng = thread_rng();
                            barrier.wait();
                            for i in 0..SAMPLE_SIZE / threads {
                                let key = &keys[i % keys.len()];
                                let random_val = rng.gen_range(0..100);

                                if random_val < write_ratio {
                                    map_clone.insert(key.clone(), format!("new_value{}", i));
                                } else {
                                    black_box(map_clone.get(key));
                                }
                            }
                        });
                    }
                });
            },
        );
    });

    group.finish();
}

// --- Benchmark definitions for LockedConcurrentMap ---

fn locked_map_small_pressure(c: &mut Criterion) {
    locked_map_benchmark(c, "LockedConcurrentMap", 2, Workload::Mixed);
    locked_map_benchmark(c, "LockedConcurrentMap", 2, Workload::ReadHeavy);
    locked_map_benchmark(c, "LockedConcurrentMap", 2, Workload::WriteHeavy);
}

fn locked_map_medium_pressure(c: &mut Criterion) {
    locked_map_benchmark(c, "LockedConcurrentMap", 4, Workload::Mixed);
    locked_map_benchmark(c, "LockedConcurrentMap", 4, Workload::ReadHeavy);
    locked_map_benchmark(c, "LockedConcurrentMap", 4, Workload::WriteHeavy);
}

fn locked_map_high_pressure(c: &mut Criterion) {
    locked_map_benchmark(c, "LockedConcurrentMap", 8, Workload::Mixed);
    locked_map_benchmark(c, "LockedConcurrentMap", 8, Workload::ReadHeavy);
    locked_map_benchmark(c, "LockedConcurrentMap", 8, Workload::WriteHeavy);
}

// --- Benchmark definitions for ConcurrentRcuMap ---

fn rcu_map_small_pressure(c: &mut Criterion) {
    rcu_map_benchmark(c, "RcuConcurrentMap", 2, Workload::Mixed);
    rcu_map_benchmark(c, "RcuConcurrentMap", 2, Workload::ReadHeavy);
    rcu_map_benchmark(c, "RcuConcurrentMap", 2, Workload::WriteHeavy);
}

fn rcu_map_medium_pressure(c: &mut Criterion) {
    rcu_map_benchmark(c, "RcuConcurrentMap", 4, Workload::Mixed);
    rcu_map_benchmark(c, "RcuConcurrentMap", 4, Workload::ReadHeavy);
    rcu_map_benchmark(c, "RcuConcurrentMap", 4, Workload::WriteHeavy);
}

fn rcu_map_high_pressure(c: &mut Criterion) {
    rcu_map_benchmark(c, "RcuConcurrentMap", 8, Workload::Mixed);
    rcu_map_benchmark(c, "RcuConcurrentMap", 8, Workload::ReadHeavy);
    rcu_map_benchmark(c, "RcuConcurrentMap", 8, Workload::WriteHeavy);
}

criterion_group!(
    benches,
    locked_map_small_pressure,
    locked_map_medium_pressure,
    locked_map_high_pressure,
    rcu_map_small_pressure,
    rcu_map_medium_pressure,
    rcu_map_high_pressure
);
criterion_main!(benches);
