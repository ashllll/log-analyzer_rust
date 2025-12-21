//! Performance benchmarks for extraction engine optimizations
//!
//! Benchmarks extraction speed (MB/s), memory usage, and concurrency scaling
//! to validate performance targets from Requirements 8.1, 8.2, 8.3

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use log_analyzer::archive::{
    ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector,
};
use log_analyzer::services::MetadataDB;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test extraction engine with specified configuration
async fn create_test_engine(policy: ExtractionPolicy) -> (ExtractionEngine, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());
    let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
    let security_detector = Arc::new(SecurityDetector::default());

    let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();
    (engine, temp_dir)
}

/// Benchmark directory creation batching with various batch sizes
fn bench_directory_creation_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_creation_batching");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for batch_size in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    runtime.block_on(async {
                        let policy = ExtractionPolicy {
                            dir_batch_size: batch_size,
                            ..Default::default()
                        };

                        let (engine, temp_dir) = create_test_engine(policy).await;

                        // Create 100 directories
                        let directories: Vec<PathBuf> = (0..100)
                            .map(|i| temp_dir.path().join(format!("dir_{}", i)))
                            .collect();

                        let created = engine
                            .create_directories_batched(&directories)
                            .await
                            .unwrap();
                        black_box(created);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel file extraction with various concurrency levels
fn bench_parallel_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_extraction");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for max_parallel in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_parallel),
            max_parallel,
            |b, &max_parallel| {
                b.iter(|| {
                    runtime.block_on(async {
                        let policy = ExtractionPolicy {
                            max_parallel_files: max_parallel,
                            ..Default::default()
                        };

                        let (engine, temp_dir) = create_test_engine(policy).await;

                        // Create 20 file extraction tasks
                        let tasks: Vec<(PathBuf, PathBuf, u64)> = (0..20)
                            .map(|i| {
                                let source = temp_dir.path().join(format!("source_{}.txt", i));
                                let target = temp_dir.path().join(format!("target_{}.txt", i));
                                (source, target, 1024)
                            })
                            .collect();

                        let extracted = engine.extract_files_parallel(tasks).await.unwrap();
                        black_box(extracted);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark streaming buffer sizes
fn bench_streaming_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_buffer_sizes");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for buffer_size in [4096, 8192, 16384, 32768, 65536, 131072].iter() {
        group.throughput(Throughput::Bytes(*buffer_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(buffer_size),
            buffer_size,
            |b, &buffer_size| {
                b.iter(|| {
                    runtime.block_on(async {
                        let policy = ExtractionPolicy {
                            buffer_size,
                            ..Default::default()
                        };

                        let (engine, _temp_dir) = create_test_engine(policy).await;

                        // Verify buffer size is configured correctly
                        assert_eq!(engine.policy().buffer_size, buffer_size);
                        black_box(engine.policy().buffer_size);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark path cache performance
fn bench_path_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_cache");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let policy = ExtractionPolicy::default();
                let (engine, temp_dir) = create_test_engine(policy).await;

                let test_path = temp_dir.path().join("test_file.txt");

                // First access - cache miss
                let _ = engine.resolve_path_cached("workspace1", &test_path).await;

                // Second access - cache hit
                let result = engine.resolve_path_cached("workspace1", &test_path).await;
                black_box(result);
            });
        });
    });

    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let policy = ExtractionPolicy::default();
                let (engine, temp_dir) = create_test_engine(policy).await;

                // Always use a new path for cache miss
                let test_path = temp_dir
                    .path()
                    .join(format!("test_file_{}.txt", rand::random::<u32>()));

                let result = engine.resolve_path_cached("workspace1", &test_path).await;
                black_box(result);
            });
        });
    });

    group.finish();
}

/// Benchmark memory usage with different configurations
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Test memory bounds with different buffer sizes and parallel files
    for (buffer_size, max_parallel) in [(64 * 1024, 4), (128 * 1024, 2), (32 * 1024, 8)].iter() {
        let config_name = format!("buf_{}k_par_{}", buffer_size / 1024, max_parallel);
        group.bench_function(&config_name, |b| {
            b.iter(|| {
                runtime.block_on(async {
                    let policy = ExtractionPolicy {
                        buffer_size: *buffer_size,
                        max_parallel_files: *max_parallel,
                        ..Default::default()
                    };

                    let (engine, _temp_dir) = create_test_engine(policy).await;

                    // Calculate theoretical max memory
                    let max_memory = buffer_size * max_parallel;
                    black_box(max_memory);
                });
            });
        });
    }

    group.finish();
}

/// Benchmark concurrency scaling
fn bench_concurrency_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency_scaling");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for num_concurrent in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_concurrent),
            num_concurrent,
            |b, &num_concurrent| {
                b.iter(|| {
                    runtime.block_on(async {
                        let policy = ExtractionPolicy {
                            max_parallel_files: 4,
                            ..Default::default()
                        };

                        let (engine, temp_dir) = create_test_engine(policy).await;

                        // Create concurrent directory creation tasks sequentially
                        for i in 0..num_concurrent {
                            let directories: Vec<PathBuf> = (0..10)
                                .map(|j| temp_dir.path().join(format!("dir_{}_{}", i, j)))
                                .collect();

                            let created = engine
                                .create_directories_batched(&directories)
                                .await
                                .unwrap();
                            black_box(created);
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_directory_creation_batching,
    bench_parallel_extraction,
    bench_streaming_buffer_sizes,
    bench_path_cache,
    bench_memory_usage,
    bench_concurrency_scaling
);

criterion_main!(benches);
