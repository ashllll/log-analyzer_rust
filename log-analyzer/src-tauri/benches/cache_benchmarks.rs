use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use log_analyzer::models::log_entry::LogEntry;
use moka::future::Cache;
use std::time::Duration;
use tokio::runtime::Runtime;

fn create_test_cache() -> Cache<String, Vec<LogEntry>> {
    Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300))
        .time_to_idle(Duration::from_secs(60))
        .build()
}

fn create_test_log_entries(count: usize) -> Vec<LogEntry> {
    (0..count)
        .map(|i| LogEntry {
            id: i,
            content: format!("Test log entry {}", i),
            file: format!("/test/path/file_{}.log", i),
            real_path: format!("/test/path/file_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect()
}

fn bench_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_operations");

    let cache = create_test_cache();
    let test_data = create_test_log_entries(100);

    // Pre-populate cache for read benchmarks
    rt.block_on(async {
        for i in 0..50 {
            let key = format!("key_{}", i);
            cache.insert(key, test_data.clone()).await;
        }
    });

    group.bench_function("cache_insert", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("insert_key_{}", rand::random::<u32>());
            cache
                .insert(black_box(key), black_box(test_data.clone()))
                .await;
        })
    });

    group.bench_function("cache_get_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("key_{}", rand::random::<u32>() % 50);
            let result = cache.get(&black_box(key)).await;
            black_box(result)
        })
    });

    group.bench_function("cache_get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let key = format!("miss_key_{}", rand::random::<u32>());
            let result = cache.get(&black_box(key)).await;
            black_box(result)
        })
    });

    group.finish();
}

fn bench_cache_with_computation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_computation");

    let cache = create_test_cache();

    for computation_time_ms in &[1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("get_with_computation", computation_time_ms),
            computation_time_ms,
            |b, &computation_time_ms| {
                b.to_async(&rt).iter(|| async {
                    let key = format!("compute_key_{}", rand::random::<u32>() % 10);
                    let result = cache
                        .get_with(black_box(key), async move {
                            // Simulate computation
                            tokio::time::sleep(Duration::from_millis(computation_time_ms)).await;
                            create_test_log_entries(50)
                        })
                        .await;
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_cache_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_concurrent");

    let cache = create_test_cache();
    let test_data = create_test_log_entries(100);

    for thread_count in &[1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_access", thread_count),
            thread_count,
            |b, &thread_count| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();

                    for i in 0..thread_count {
                        let cache = cache.clone();
                        let data = test_data.clone();
                        let handle = tokio::spawn(async move {
                            for j in 0..10 {
                                let key = format!("thread_{}_key_{}", i, j);
                                cache.insert(key.clone(), data.clone()).await;
                                let _result = cache.get(&key).await;
                            }
                        });
                        handles.push(handle);
                    }

                    futures::future::join_all(handles).await;
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_cache_with_computation,
    bench_cache_concurrent_access
);
criterion_main!(benches);
