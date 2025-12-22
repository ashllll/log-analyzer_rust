//! 生产性能验证基准测试
//!
//! 测试所有主要操作的性能，包括：
//! - 缓存命中率和内存使用
//! - parking_lot vs std::sync 并发性能
//! - eyre vs 自定义错误类型开销
//! - 服务创建和依赖注入开销

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use log_analyzer::services::{AppServices, ServiceConfiguration};
use log_analyzer::utils::validation::validate_safe_path;
use moka::sync::Cache;
use parking_lot::Mutex as ParkingLotMutex;
use std::sync::Mutex as StdMutex;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 基准测试：缓存性能（moka vs lru）
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    // Moka 缓存性能
    group.bench_function("moka_insert_and_get", |b| {
        let cache: Cache<String, String> = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(60))
            .build();

        b.iter(|| {
            for i in 0..100 {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);
                cache.insert(key.clone(), value.clone());
                black_box(cache.get(&key));
            }
        });
    });

    // Moka 缓存命中率测试
    group.bench_function("moka_hit_rate", |b| {
        let cache: Cache<String, String> = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(60))
            .build();

        // 预填充缓存
        for i in 0..50 {
            cache.insert(format!("key_{}", i), format!("value_{}", i));
        }

        b.iter(|| {
            // 50% 命中率
            for i in 0..100 {
                let key = format!("key_{}", i % 100);
                black_box(cache.get(&key));
            }
        });
    });

    group.finish();
}

/// 基准测试：并发性能（parking_lot vs std::sync）
fn bench_concurrency_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency_performance");

    // parking_lot Mutex 性能
    group.bench_function("parking_lot_mutex", |b| {
        let data = Arc::new(ParkingLotMutex::new(0));

        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..10 {
                let data = Arc::clone(&data);
                let handle = thread::spawn(move || {
                    for _ in 0..100 {
                        let mut guard = data.lock();
                        *guard += 1;
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    // std::sync Mutex 性能（用于对比）
    group.bench_function("std_sync_mutex", |b| {
        let data = Arc::new(StdMutex::new(0));

        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..10 {
                let data = Arc::clone(&data);
                let handle = thread::spawn(move || {
                    for _ in 0..100 {
                        let mut guard = data.lock().unwrap();
                        *guard += 1;
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    // parking_lot 超时锁性能
    group.bench_function("parking_lot_try_lock_for", |b| {
        let data = Arc::new(ParkingLotMutex::new(0));

        b.iter(|| {
            let data_clone = Arc::clone(&data);
            match data_clone.try_lock_for(Duration::from_millis(10)) {
                Some(mut guard) => {
                    *guard += 1;
                }
                None => {}
            }
        });
    });

    group.finish();
}

/// 基准测试：错误处理开销（eyre vs Result<T, String>）
fn bench_error_handling_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling_overhead");

    // eyre 错误处理
    group.bench_function("eyre_error_handling", |b| {
        b.iter(|| {
            let result: eyre::Result<()> = (|| {
                validate_safe_path("valid/path/to/file.txt")?;
                Ok(())
            })();
            black_box(result);
        });
    });

    // 简单 Result 错误处理（用于对比）
    group.bench_function("simple_result_error_handling", |b| {
        b.iter(|| {
            let result: Result<(), String> = (|| {
                if "valid/path".contains("..") {
                    return Err("Invalid path".to_string());
                }
                Ok(())
            })();
            black_box(result);
        });
    });

    // eyre 错误上下文添加
    group.bench_function("eyre_with_context", |b| {
        use eyre::Context;

        b.iter(|| {
            let result: eyre::Result<()> = (|| {
                validate_safe_path("valid/path/to/file.txt")
                    .context("Validating file path")?;
                Ok(())
            })();
            black_box(result);
        });
    });

    group.finish();
}

/// 基准测试：服务创建和依赖注入开销
fn bench_service_creation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("service_creation_overhead");

    // 默认服务创建
    group.bench_function("default_service_creation", |b| {
        b.iter(|| {
            let services = AppServices::new().unwrap();
            black_box(services);
        });
    });

    // 使用 Builder 模式创建服务
    group.bench_function("builder_service_creation", |b| {
        b.iter(|| {
            let services = AppServices::builder().build().unwrap();
            black_box(services);
        });
    });

    // 使用配置创建服务
    group.bench_function("config_service_creation", |b| {
        let config = ServiceConfiguration::development();
        b.iter(|| {
            let services = AppServices::builder()
                .with_config(config.clone())
                .build()
                .unwrap();
            black_box(services);
        });
    });

    // 服务健康检查开销
    group.bench_function("service_health_check", |b| {
        let services = AppServices::new().unwrap();
        b.iter(|| {
            let health = services.overall_health();
            black_box(health);
        });
    });

    group.finish();
}

/// 基准测试：验证性能
fn bench_validation_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_performance");

    // 路径验证性能
    group.bench_function("path_validation", |b| {
        b.iter(|| {
            let result = validate_safe_path("valid/path/to/file.txt");
            black_box(result);
        });
    });

    // 无效路径验证性能
    group.bench_function("invalid_path_validation", |b| {
        b.iter(|| {
            let result = validate_safe_path("../../../etc/passwd");
            black_box(result);
        });
    });

    // 批量路径验证
    group.bench_function("batch_path_validation", |b| {
        let paths = vec![
            "path1/file.txt",
            "path2/file.txt",
            "path3/file.txt",
            "path4/file.txt",
            "path5/file.txt",
        ];

        b.iter(|| {
            for path in &paths {
                let result = validate_safe_path(path);
                black_box(result);
            }
        });
    });

    group.finish();
}

/// 基准测试：内存使用和缓存效率
fn bench_memory_and_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_and_cache_efficiency");

    // 不同缓存大小的性能
    for size in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("cache_size", size),
            size,
            |b, &size| {
                let cache: Cache<String, Vec<u8>> = Cache::builder()
                    .max_capacity(size)
                    .time_to_live(Duration::from_secs(60))
                    .build();

                b.iter(|| {
                    for i in 0..size {
                        let key = format!("key_{}", i);
                        let value = vec![0u8; 1024]; // 1KB 数据
                        cache.insert(key.clone(), value);
                        black_box(cache.get(&key));
                    }
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：并发缓存访问
fn bench_concurrent_cache_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_cache_access");

    // 并发读取
    group.bench_function("concurrent_cache_reads", |b| {
        let cache: Arc<Cache<String, String>> = Arc::new(
            Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60))
                .build(),
        );

        // 预填充缓存
        for i in 0..100 {
            cache.insert(format!("key_{}", i), format!("value_{}", i));
        }

        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..10 {
                let cache = Arc::clone(&cache);
                let handle = thread::spawn(move || {
                    for i in 0..100 {
                        let key = format!("key_{}", i % 100);
                        black_box(cache.get(&key));
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    // 并发写入
    group.bench_function("concurrent_cache_writes", |b| {
        let cache: Arc<Cache<String, String>> = Arc::new(
            Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60))
                .build(),
        );

        b.iter(|| {
            let mut handles = vec![];
            for thread_id in 0..10 {
                let cache = Arc::clone(&cache);
                let handle = thread::spawn(move || {
                    for i in 0..100 {
                        let key = format!("key_{}_{}", thread_id, i);
                        let value = format!("value_{}_{}", thread_id, i);
                        cache.insert(key, value);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_performance,
    bench_concurrency_performance,
    bench_error_handling_overhead,
    bench_service_creation_overhead,
    bench_validation_performance,
    bench_memory_and_cache_efficiency,
    bench_concurrent_cache_access,
);

criterion_main!(benches);
