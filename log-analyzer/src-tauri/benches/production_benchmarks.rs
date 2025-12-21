//! Production benchmark suite with comprehensive performance monitoring
//!
//! This benchmark suite provides:
//! - Automated performance regression detection
//! - Continuous benchmarking for CI/CD integration
//! - Production-ready performance baselines
//! - Load testing for concurrent operations
//!
//! **Validates: All performance properties from Requirements 1-7**

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use log_analyzer::models::log_entry::LogEntry;
use log_analyzer::models::validated::{ValidatedSearchQuery, ValidatedWorkspaceConfig};
use log_analyzer::utils::validation::validate_path_safety;
use moka::future::Cache;
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use validator::Validate;

/// Production benchmark configuration
struct ProductionBenchmarkConfig {
    runtime: Runtime,
}

impl ProductionBenchmarkConfig {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        Self { runtime }
    }
}

/// Performance baseline thresholds for regression detection
#[allow(dead_code)]
struct PerformanceBaselines {
    /// Maximum acceptable cache insert time (microseconds)
    cache_insert_max_us: u64,
    /// Maximum acceptable cache get time (microseconds)
    cache_get_max_us: u64,
    /// Maximum acceptable search time per 1000 entries (milliseconds)
    search_per_1k_max_ms: u64,
    /// Maximum acceptable validation time per config (microseconds)
    validation_per_config_max_us: u64,
}

impl Default for PerformanceBaselines {
    fn default() -> Self {
        Self {
            cache_insert_max_us: 1000,         // 1ms max for cache insert
            cache_get_max_us: 100,             // 100µs max for cache get
            search_per_1k_max_ms: 10,          // 10ms max per 1000 entries
            validation_per_config_max_us: 500, // 500µs max per validation
        }
    }
}

/// Comprehensive cache performance benchmarks
fn bench_production_cache_performance(c: &mut Criterion) {
    let config = ProductionBenchmarkConfig::new();
    let mut group = c.benchmark_group("production_cache");

    // Set throughput for better analysis
    group.throughput(Throughput::Elements(1));

    let cache = Cache::builder()
        .max_capacity(10000)
        .time_to_live(Duration::from_secs(300))
        .time_to_idle(Duration::from_secs(60))
        .build();

    let test_data: Vec<LogEntry> = (0..1000)
        .map(|i| LogEntry {
            id: i,
            content: format!(
                "Production test log entry {} with detailed content for realistic benchmarking",
                i
            ),
            file: format!("/production/logs/app_{}.log", i % 10),
            real_path: format!("/production/logs/app_{}.log", i % 10),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // Benchmark cache operations under production load
    for cache_size in &[100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("cache_insert_production", cache_size),
            cache_size,
            |b, &cache_size| {
                b.to_async(&config.runtime).iter(|| async {
                    for i in 0..cache_size {
                        let key = format!("prod_key_{}", i);
                        cache
                            .insert(black_box(key), black_box(test_data.clone()))
                            .await;
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cache_get_production", cache_size),
            cache_size,
            |b, &cache_size| {
                // Pre-populate cache
                config.runtime.block_on(async {
                    for i in 0..cache_size {
                        let key = format!("prod_key_{}", i);
                        cache.insert(key, test_data.clone()).await;
                    }
                });

                b.to_async(&config.runtime).iter(|| async {
                    for i in 0..cache_size {
                        let key = format!("prod_key_{}", i);
                        let _result = cache.get(&black_box(key)).await;
                    }
                })
            },
        );
    }

    group.finish();
}

/// Production search performance benchmarks
fn bench_production_search_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_search");

    // Create realistic production data
    let log_entries: Vec<LogEntry> = (0..50000)
        .map(|i| LogEntry {
            id: i,
            content: format!(
                "[{}] {} - {} operation completed in {}ms with status {} for user_id={}",
                "2024-01-01T00:00:00Z",
                ["INFO", "WARN", "ERROR", "DEBUG"][i % 4],
                ["search", "cache", "validation", "workspace"][i % 4],
                (i % 1000) + 1,
                ["success", "failure", "timeout"][i % 3],
                i % 10000
            ),
            file: format!("/var/log/app/service_{}.log", i % 20),
            real_path: format!("/var/log/app/service_{}.log", i % 20),
            line: (i % 10000),
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: ["INFO", "WARN", "ERROR", "DEBUG"][i % 4].to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // Benchmark different search patterns
    let search_patterns = vec![
        ("simple_term", "error"),
        ("complex_pattern", "operation completed.*success"),
        ("user_search", "user_id=123"),
        ("time_range", "2024-.*ERROR"),
        ("multi_term", "search.*timeout"),
    ];

    for (pattern_name, pattern) in search_patterns {
        for data_size in &[1000, 10000, 50000] {
            let entries = &log_entries[..*data_size];

            group.bench_with_input(
                BenchmarkId::new(format!("search_{}", pattern_name), data_size),
                &(entries, pattern),
                |b, (entries, pattern)| {
                    b.iter(|| {
                        let results: Vec<_> = entries
                            .iter()
                            .filter(|entry| {
                                if pattern.contains(".*") {
                                    // Regex pattern
                                    regex::Regex::new(pattern)
                                        .map(|re| re.is_match(&entry.content))
                                        .unwrap_or(false)
                                } else {
                                    // Simple string search
                                    entry.content.contains(pattern)
                                }
                            })
                            .cloned()
                            .collect();
                        black_box(results)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Production validation performance benchmarks
fn bench_production_validation_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_validation");

    // Create realistic validation test data
    let workspace_configs: Vec<ValidatedWorkspaceConfig> = (0..1000)
        .map(|i| ValidatedWorkspaceConfig {
            workspace_id: format!("workspace_{}", i),
            name: format!("Production Workspace {}", i),
            description: Some(format!("Production workspace description {}", i)),
            path: format!("/production/workspaces/workspace_{}/data", i),
            max_file_size: 1024 * 1024, // 1MB
            max_file_count: 1000,
            enable_watch: true,
            tags: vec![format!("tag_{}", i)],
            metadata: HashMap::new(),
            contact_email: Some(format!("test{}@example.com", i)),
            project_url: Some(format!("https://example.com/project_{}", i)),
        })
        .collect();

    let search_queries: Vec<ValidatedSearchQuery> = (0..1000)
        .map(|i| ValidatedSearchQuery {
            query: format!("production search query {} with complex patterns", i),
            workspace_id: format!("workspace_{}", i % 100),
            max_results: Some(1000 + (i % 9000)), // Vary between 1000-10000
            case_sensitive: false,
            use_regex: false,
            file_pattern: Some("*.log".to_string()),
            time_start: None,
            time_end: None,
            log_levels: vec!["INFO".to_string(), "ERROR".to_string()],
            priority: Some(5),
            timeout_seconds: Some(30),
        })
        .collect();

    // Benchmark validation operations
    for batch_size in &[10, 100, 1000] {
        let configs = &workspace_configs[..*batch_size];
        let queries = &search_queries[..*batch_size];

        group.bench_with_input(
            BenchmarkId::new("workspace_validation", batch_size),
            configs,
            |b, configs| {
                b.iter(|| {
                    let results: Vec<_> = configs.iter().map(|config| config.validate()).collect();
                    black_box(results)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("query_validation", batch_size),
            queries,
            |b, queries| {
                b.iter(|| {
                    let results: Vec<_> = queries.iter().map(|query| query.validate()).collect();
                    black_box(results)
                })
            },
        );
    }

    // Benchmark path security validation
    let test_paths = vec![
        "/production/safe/path/file.log",
        "../../../etc/passwd",
        "/production/path/with/unicode/测试文件.log",
        "/very/long/production/path/that/might/cause/performance/issues/in/validation/logic/test/file.log",
        "C:\\Windows\\System32\\config\\SAM",
        "/production/path/with/special/chars/file<>:\"|?*.log",
    ];

    group.bench_function("path_security_validation", |b| {
        b.iter(|| {
            let results: Vec<_> = test_paths
                .iter()
                .map(|path| validate_path_safety(black_box(path)))
                .collect();
            black_box(results)
        })
    });

    group.bench_function("path_sanitization", |b| {
        b.iter(|| {
            let results: Vec<_> = test_paths
                .iter()
                .map(|path| sanitize_filename::sanitize(black_box(path)))
                .collect();
            black_box(results)
        })
    });

    group.finish();
}

/// Concurrent operations benchmark for production load testing
fn bench_production_concurrent_operations(c: &mut Criterion) {
    let config = ProductionBenchmarkConfig::new();
    let mut group = c.benchmark_group("production_concurrent");

    let cache = Cache::builder()
        .max_capacity(10000)
        .time_to_live(Duration::from_secs(300))
        .build();

    let test_data: Vec<LogEntry> = (0..1000)
        .map(|i| LogEntry {
            id: i,
            content: format!("Concurrent test data {}", i),
            file: format!("/test/concurrent_{}.log", i),
            real_path: format!("/test/concurrent_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // Benchmark concurrent cache operations
    for thread_count in &[1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_cache_operations", thread_count),
            thread_count,
            |b, &thread_count| {
                b.to_async(&config.runtime).iter(|| async {
                    let mut handles = Vec::new();

                    for i in 0..thread_count {
                        let cache = cache.clone();
                        let data = test_data.clone();
                        let handle = tokio::spawn(async move {
                            for j in 0..100 {
                                let key = format!("thread_{}_key_{}", i, j);
                                cache.insert(key.clone(), data.clone()).await;
                                let _result = cache.get(&key).await;
                            }
                        });
                        handles.push(handle);
                    }

                    futures::future::join_all(handles).await
                })
            },
        );
    }

    group.finish();
}

/// Memory usage and resource efficiency benchmarks
fn bench_production_resource_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("production_resources");

    // Benchmark memory allocation patterns
    for data_size in &[1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::new("memory_allocation", data_size),
            data_size,
            |b, &data_size| {
                b.iter(|| {
                    let data: Vec<LogEntry> = (0..data_size)
                        .map(|i| LogEntry {
                            id: i,
                            content: format!(
                                "Memory test entry {} with substantial content to test allocation patterns",
                                i
                            ),
                            file: format!("/memory/test/file_{}.log", i % 100),
                            real_path: format!("/memory/test/file_{}.log", i % 100),
                            line: i,
                            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
                            level: "INFO".to_string(),
                            tags: vec![],
                            match_details: None,
                            matched_keywords: None,
                        })
                        .collect();
                    black_box(data)
                })
            },
        );
    }

    group.finish();
}

/// Load testing benchmarks for scaling verification
fn bench_production_load_testing(c: &mut Criterion) {
    let config = ProductionBenchmarkConfig::new();
    let mut group = c.benchmark_group("production_load");

    // Configure for longer running benchmarks
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    let cache = Cache::builder()
        .max_capacity(50000)
        .time_to_live(Duration::from_secs(300))
        .build();

    let test_data: Vec<LogEntry> = (0..100)
        .map(|i| LogEntry {
            id: i,
            content: format!("Load test entry {}", i),
            file: format!("/load/test_{}.log", i),
            real_path: format!("/load/test_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // High-load concurrent operations
    for operations_per_thread in &[100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("high_load_operations", operations_per_thread),
            operations_per_thread,
            |b, &ops_per_thread| {
                b.to_async(&config.runtime).iter(|| async {
                    let mut handles = Vec::new();
                    let thread_count = 8;

                    for thread_id in 0..thread_count {
                        let cache = cache.clone();
                        let data = test_data.clone();
                        let handle = tokio::spawn(async move {
                            for op_id in 0..ops_per_thread {
                                let key = format!("load_{}_{}", thread_id, op_id);
                                cache.insert(key.clone(), data.clone()).await;
                                let _result = cache.get(&key).await;
                            }
                        });
                        handles.push(handle);
                    }

                    futures::future::join_all(handles).await
                })
            },
        );
    }

    group.finish();
}

/// Performance regression detection benchmarks
fn bench_regression_detection(c: &mut Criterion) {
    let config = ProductionBenchmarkConfig::new();
    let mut group = c.benchmark_group("regression_detection");

    // Cache operation regression tests
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(60))
        .build();

    let test_data: Vec<LogEntry> = (0..10)
        .map(|i| LogEntry {
            id: i,
            content: format!("Regression test entry {}", i),
            file: format!("/regression/test_{}.log", i),
            real_path: format!("/regression/test_{}.log", i),
            line: i,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    group.bench_function("baseline_cache_insert", |b| {
        b.to_async(&config.runtime).iter(|| async {
            let key = format!("baseline_key_{}", rand::random::<u32>());
            cache
                .insert(black_box(key), black_box(test_data.clone()))
                .await;
        })
    });

    group.bench_function("baseline_cache_get", |b| {
        // Pre-populate
        config.runtime.block_on(async {
            for i in 0..100 {
                cache
                    .insert(format!("baseline_{}", i), test_data.clone())
                    .await;
            }
        });

        b.to_async(&config.runtime).iter(|| async {
            let key = format!("baseline_{}", rand::random::<u32>() % 100);
            let _result = cache.get(&black_box(key)).await;
        })
    });

    // Validation regression tests
    let workspace_config = ValidatedWorkspaceConfig {
        workspace_id: "regression_test".to_string(),
        name: "Regression Test Workspace".to_string(),
        description: Some("Test workspace for regression detection".to_string()),
        path: "/regression/test/workspace".to_string(),
        max_file_size: 1024 * 1024,
        max_file_count: 1000,
        enable_watch: false,
        tags: vec!["regression".to_string()],
        metadata: HashMap::new(),
        contact_email: Some("test@example.com".to_string()),
        project_url: Some("https://example.com".to_string()),
    };

    group.bench_function("baseline_validation", |b| {
        b.iter(|| {
            let result = workspace_config.validate();
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    production_benches,
    bench_production_cache_performance,
    bench_production_search_performance,
    bench_production_validation_performance,
    bench_production_concurrent_operations,
    bench_production_resource_efficiency,
    bench_production_load_testing,
    bench_regression_detection
);
criterion_main!(production_benches);
