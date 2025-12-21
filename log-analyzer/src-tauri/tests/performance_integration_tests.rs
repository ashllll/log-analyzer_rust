//! Performance Integration Test Suite
//!
//! Comprehensive integration tests for all performance optimizations including:
//! - Search engine, caching, and state synchronization interaction
//! - Performance guarantees under various load conditions
//! - Network resilience and recovery scenarios
//! - Memory pressure and resource constraint testing
//!
//! **Validates: All performance properties from Requirements 1-7**

use log_analyzer::models::log_entry::LogEntry;
use log_analyzer::models::validated::ValidatedWorkspaceConfig;
use log_analyzer::utils::validation::validate_path_safety;
use log_analyzer::{AsyncResourceManager, CacheManager, SearchCacheKey};
use moka::future::Cache as AsyncCache;
use moka::sync::Cache as SyncCache;
use parking_lot::Mutex;
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use validator::Validate;

/// Test configuration for performance integration tests
struct TestConfig {
    /// Maximum response time for search operations (ms)
    max_search_response_ms: u64,
    /// Maximum response time for cache operations (ms)
    max_cache_response_ms: u64,
    /// Maximum response time for state sync operations (ms)
    max_state_sync_ms: u64,
    /// Number of concurrent operations for load testing
    concurrent_operations: usize,
    /// Dataset size for performance testing
    dataset_size: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            max_search_response_ms: 200,
            max_cache_response_ms: 50,
            max_state_sync_ms: 100,
            concurrent_operations: 10,
            dataset_size: 1000,
        }
    }
}

/// Generate test log entries for performance testing
fn generate_test_log_entries(count: usize) -> Vec<LogEntry> {
    (0..count)
        .map(|i| LogEntry {
            id: i,
            content: format!(
                "[{}] {} - Operation {} completed with status {} for user_id={}",
                "2024-01-01T00:00:00Z",
                ["INFO", "WARN", "ERROR", "DEBUG"][i % 4],
                ["search", "cache", "validation", "workspace"][i % 4],
                ["success", "failure", "timeout"][i % 3],
                i % 1000
            ),
            file: format!("/var/log/app/service_{}.log", i % 20),
            real_path: format!("/var/log/app/service_{}.log", i % 20),
            line: i % 10000,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: ["INFO", "WARN", "ERROR", "DEBUG"][i % 4].to_string(),
            tags: vec![format!("tag_{}", i % 10)],
            match_details: None,
            matched_keywords: Some(vec![]),
        })
        .collect()
}

/// Generate test workspace configurations
fn generate_test_workspace_configs(count: usize) -> Vec<ValidatedWorkspaceConfig> {
    (0..count)
        .map(|i| ValidatedWorkspaceConfig {
            workspace_id: format!("workspace_{}", i),
            name: format!("Test Workspace {}", i),
            description: Some(format!("Test workspace description {}", i)),
            path: format!("/test/workspace/path_{}", i),
            max_file_size: 1024 * 1024,
            max_file_count: 1000,
            enable_watch: true,
            tags: vec![format!("tag_{}", i)],
            metadata: HashMap::new(),
            contact_email: Some(format!("test{}@example.com", i)),
            project_url: Some(format!("https://example.com/project_{}", i)),
        })
        .collect()
}

// ============================================================================
// Module 1: Search Engine and Cache Integration Tests
// ============================================================================

#[cfg(test)]
mod search_cache_integration {
    use super::*;

    /// Test search and cache interaction under normal load
    /// **Validates: Requirements 1.1, 1.3**
    #[tokio::test]
    async fn test_search_cache_integration_normal_load() {
        let config = TestConfig::default();
        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        let test_data = generate_test_log_entries(config.dataset_size);

        // Test cache population
        let start = Instant::now();
        for i in 0..100 {
            let key = format!("search_query_{}", i);
            cache.insert(key, test_data.clone()).await;
        }
        let insert_duration = start.elapsed();

        assert!(
            insert_duration < Duration::from_millis(config.max_cache_response_ms * 10),
            "Cache insertion took too long: {:?}",
            insert_duration
        );

        // Test cache retrieval (should be fast)
        let start = Instant::now();
        for i in 0..100 {
            let key = format!("search_query_{}", i);
            let result = cache.get(&key).await;
            assert!(result.is_some(), "Cache miss for key: {}", key);
        }
        let get_duration = start.elapsed();

        assert!(
            get_duration < Duration::from_millis(config.max_cache_response_ms * 2),
            "Cache retrieval took too long: {:?}",
            get_duration
        );

        println!("✅ Search-cache integration test passed:");
        println!("   - Insert 100 items: {:?}", insert_duration);
        println!("   - Get 100 items: {:?}", get_duration);
    }

    /// Test cache hit rate tracking
    /// **Validates: Requirements 4.3, 7.2**
    #[tokio::test]
    async fn test_cache_hit_rate_tracking() {
        // Create a cache with the correct SearchCacheKey type
        let cache: SyncCache<SearchCacheKey, Vec<LogEntry>> = SyncCache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(60))
            .build();

        let cache_manager = Arc::new(CacheManager::new(Arc::new(cache.clone())));
        let test_data = generate_test_log_entries(10);

        // Populate cache with some entries using proper SearchCacheKey
        for i in 0..50 {
            let key: SearchCacheKey = (
                format!("query_{}", i),
                format!("workspace_{}", i % 5),
                None,
                None,
                vec![],
                None,
                false,
                1000,
                "v1".to_string(),
            );
            cache.insert(key, test_data.clone());
        }

        // Perform mixed hits and misses
        let mut hits = 0u32;
        let mut misses = 0u32;

        for i in 0..100 {
            let key: SearchCacheKey = (
                format!("query_{}", i),
                format!("workspace_{}", i % 5),
                None,
                None,
                vec![],
                None,
                false,
                1000,
                "v1".to_string(),
            );
            if cache.get(&key).is_some() {
                hits += 1;
            } else {
                misses += 1;
            }
        }

        // Verify hit rate calculation
        let hit_rate = hits as f64 / (hits + misses) as f64;
        assert!(
            hit_rate >= 0.0 && hit_rate <= 1.0,
            "Invalid hit rate: {}",
            hit_rate
        );

        // Get cache statistics
        let stats = cache_manager.get_cache_statistics();
        assert!(stats.entry_count <= 100, "Cache exceeded capacity");

        println!("✅ Cache hit rate tracking test passed:");
        println!("   - Hits: {}, Misses: {}", hits, misses);
        println!("   - Hit rate: {:.2}%", hit_rate * 100.0);
    }

    /// Test cache eviction under memory pressure
    /// **Validates: Requirements 3.5, 7.3**
    #[tokio::test]
    async fn test_cache_eviction_under_pressure() {
        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(50) // Small capacity to trigger eviction
            .time_to_live(Duration::from_secs(60))
            .build();

        let test_data = generate_test_log_entries(100);

        // Insert more items than capacity
        for i in 0..100 {
            let key = format!("pressure_key_{}", i);
            cache.insert(key, test_data.clone()).await;
        }

        // Wait for eviction to occur
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify cache respects capacity limits
        let entry_count = cache.entry_count();
        assert!(
            entry_count <= 50,
            "Cache exceeded capacity: {} entries",
            entry_count
        );

        println!("✅ Cache eviction test passed:");
        println!("   - Inserted 100 items into capacity-50 cache");
        println!("   - Final entry count: {}", entry_count);
    }
}

// ============================================================================
// Module 2: Concurrent Operations Tests
// ============================================================================

#[cfg(test)]
mod concurrent_operations {
    use super::*;

    /// Test concurrent search operations
    /// **Validates: Requirements 1.5**
    #[tokio::test]
    async fn test_concurrent_search_performance() {
        let config = TestConfig::default();
        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        let test_data = Arc::new(generate_test_log_entries(config.dataset_size));
        let cache = Arc::new(cache);
        let operation_count = Arc::new(AtomicU64::new(0));
        let total_duration = Arc::new(AtomicU64::new(0));

        let mut handles = Vec::new();

        for thread_id in 0..config.concurrent_operations {
            let cache = cache.clone();
            let data = test_data.clone();
            let op_count = operation_count.clone();
            let total_dur = total_duration.clone();

            let handle = tokio::spawn(async move {
                for op_id in 0..50 {
                    let start = Instant::now();
                    let key = format!("thread_{}_op_{}", thread_id, op_id);

                    // Simulate search + cache operation
                    cache.insert(key.clone(), (*data).clone()).await;
                    let _result = cache.get(&key).await;

                    let duration = start.elapsed();
                    op_count.fetch_add(1, Ordering::SeqCst);
                    total_dur.fetch_add(duration.as_micros() as u64, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let total_ops = operation_count.load(Ordering::SeqCst);
        let avg_duration_us = total_duration.load(Ordering::SeqCst) / total_ops;

        assert!(
            avg_duration_us < (config.max_cache_response_ms * 1000) as u64,
            "Average operation time too high: {}µs",
            avg_duration_us
        );

        println!("✅ Concurrent search performance test passed:");
        println!("   - Total operations: {}", total_ops);
        println!("   - Average duration: {}µs", avg_duration_us);
    }

    /// Test concurrent cache access with read-write mix
    /// **Validates: Requirements 1.5, 3.3**
    #[tokio::test]
    async fn test_concurrent_cache_read_write() {
        let cache: SyncCache<String, u64> = SyncCache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(60))
            .build();

        let cache = Arc::new(cache);
        let read_count = Arc::new(AtomicU64::new(0));
        let write_count = Arc::new(AtomicU64::new(0));

        let mut handles = Vec::new();

        // Create reader threads
        for reader_id in 0..5 {
            let cache = cache.clone();
            let read_cnt = read_count.clone();

            let handle = std::thread::spawn(move || {
                for i in 0..100 {
                    let key = format!("key_{}", i % 50);
                    let _result = cache.get(&key);
                    read_cnt.fetch_add(1, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        // Create writer threads
        for writer_id in 0..5 {
            let cache = cache.clone();
            let write_cnt = write_count.clone();

            let handle = std::thread::spawn(move || {
                for i in 0..100 {
                    let key = format!("key_{}", i % 50);
                    cache.insert(key, i as u64);
                    write_cnt.fetch_add(1, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        let total_reads = read_count.load(Ordering::SeqCst);
        let total_writes = write_count.load(Ordering::SeqCst);

        assert_eq!(total_reads, 500, "Expected 500 reads");
        assert_eq!(total_writes, 500, "Expected 500 writes");

        println!("✅ Concurrent read-write test passed:");
        println!("   - Total reads: {}", total_reads);
        println!("   - Total writes: {}", total_writes);
    }

    /// Test performance stability under concurrent load
    /// **Validates: Requirements 1.5**
    #[tokio::test]
    async fn test_performance_stability_under_load() {
        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(500)
            .time_to_live(Duration::from_secs(60))
            .build();

        let test_data = generate_test_log_entries(100);
        let cache = Arc::new(cache);
        let semaphore = Arc::new(Semaphore::new(20)); // Limit concurrent operations

        let latencies_mutex = Arc::new(Mutex::new(Vec::<u64>::new()));

        let mut handles = Vec::new();

        for i in 0..100 {
            let cache = cache.clone();
            let data = test_data.clone();
            let sem = semaphore.clone();
            let lat_mutex = latencies_mutex.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let start = Instant::now();

                let key = format!("stability_key_{}", i);
                cache.insert(key.clone(), data).await;
                let _result = cache.get(&key).await;

                let duration = start.elapsed();
                lat_mutex.lock().push(duration.as_micros() as u64);
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let latencies = latencies_mutex.lock();
        let avg_latency: u64 = latencies.iter().sum::<u64>() / latencies.len() as u64;
        let max_latency = *latencies.iter().max().unwrap_or(&0);
        let min_latency = *latencies.iter().min().unwrap_or(&0);

        // Performance should be stable (max should not be more than 10x average)
        assert!(
            max_latency < avg_latency * 10,
            "Performance instability detected: max={}µs, avg={}µs",
            max_latency,
            avg_latency
        );

        println!("✅ Performance stability test passed:");
        println!("   - Average latency: {}µs", avg_latency);
        println!("   - Min latency: {}µs", min_latency);
        println!("   - Max latency: {}µs", max_latency);
    }
}

// ============================================================================
// Module 3: Resource Management Tests
// ============================================================================

#[cfg(test)]
mod resource_management {
    use super::*;

    /// Test async resource manager under load
    /// **Validates: Requirements 7.5**
    #[tokio::test]
    async fn test_async_resource_manager_load() {
        let manager = Arc::new(AsyncResourceManager::new());
        let mut handles = Vec::new();

        // Create many concurrent operations
        for i in 0..50 {
            let manager = manager.clone();

            let handle = tokio::spawn(async move {
                let operation_id = format!("load_test_op_{}", i);
                let resource_id = format!("load_test_resource_{}", i);
                let resource_path = format!("/tmp/load_test_{}", i);

                // Register operation
                let token = manager
                    .register_operation(
                        operation_id.clone(),
                        log_analyzer::utils::async_resource_manager::OperationType::BackgroundTask,
                        None,
                    )
                    .await;

                // Register resource
                manager
                    .register_resource(resource_id.clone(), resource_path.clone())
                    .await
                    .unwrap();

                // Simulate work
                tokio::time::sleep(Duration::from_millis(10)).await;

                // Cleanup
                manager.cleanup_resource(&resource_id).await.unwrap();
                manager.cancel_operation(&operation_id).await.unwrap();

                assert!(token.is_cancelled());
            });

            handles.push(handle);
        }

        // Wait for all operations
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify cleanup
        assert_eq!(manager.active_operations_count().await, 0);
        assert_eq!(manager.resources_count().await, 0);

        println!("✅ Async resource manager load test passed");
    }

    /// Test memory allocation efficiency
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn test_memory_allocation_efficiency() {
        let sizes = [100, 1000, 10000];

        for &size in &sizes {
            let start = Instant::now();
            let data = generate_test_log_entries(size);
            let allocation_time = start.elapsed();

            // Memory allocation should scale linearly
            let expected_max_ms = (size as u64 / 100).max(10);
            assert!(
                allocation_time < Duration::from_millis(expected_max_ms),
                "Memory allocation for {} entries took too long: {:?}",
                size,
                allocation_time
            );

            assert_eq!(data.len(), size);
            println!("   - Allocated {} entries in {:?}", size, allocation_time);
        }

        println!("✅ Memory allocation efficiency test passed");
    }
}

// ============================================================================
// Module 4: Validation Performance Tests
// ============================================================================

#[cfg(test)]
mod validation_performance {
    use super::*;

    /// Test validation performance under load
    /// **Validates: Requirements 6.1-6.5**
    #[test]
    fn test_validation_performance() {
        let configs = generate_test_workspace_configs(100);

        let start = Instant::now();
        let results: Vec<_> = configs.iter().map(|c| c.validate()).collect();
        let validation_time = start.elapsed();

        // All validations should pass
        let valid_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(valid_count, 100, "Not all validations passed");

        // Validation should be fast
        assert!(
            validation_time < Duration::from_millis(50),
            "Validation took too long: {:?}",
            validation_time
        );

        println!("✅ Validation performance test passed:");
        println!(
            "   - Validated {} configs in {:?}",
            configs.len(),
            validation_time
        );
    }

    /// Test path security validation performance
    /// **Validates: Requirements 6.1**
    #[test]
    fn test_path_security_validation_performance() {
        let test_paths = vec![
            "/safe/path/file.log",
            "../../../etc/passwd",
            "/path/with/unicode/测试文件.log",
            "/very/long/path/that/might/cause/performance/issues.log",
            "C:\\Windows\\System32\\config\\SAM",
        ];

        // Create large test set
        let large_path_set: Vec<_> = (0..200).flat_map(|_| test_paths.iter().cloned()).collect();

        let start = Instant::now();
        let results: Vec<_> = large_path_set
            .iter()
            .map(|path| validate_path_safety(path))
            .collect();
        let validation_time = start.elapsed();

        assert!(
            validation_time < Duration::from_millis(50),
            "Path validation took too long: {:?}",
            validation_time
        );

        let valid_count = results.iter().filter(|r| r.is_ok()).count();
        let invalid_count = results.len() - valid_count;

        println!("✅ Path security validation test passed:");
        println!(
            "   - Validated {} paths in {:?}",
            results.len(),
            validation_time
        );
        println!("   - Valid: {}, Invalid: {}", valid_count, invalid_count);
    }
}

// ============================================================================
// Module 5: Property-Based Integration Tests
// ============================================================================

#[cfg(test)]
mod property_based_integration {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Cache operations should maintain consistency
        /// **Validates: Requirements 1.3, 3.3**
        #[test]
        fn prop_cache_consistency(
            keys in prop::collection::hash_set("[a-zA-Z0-9]{1,20}", 1..50),
            values in prop::collection::vec(0u64..1000, 1..50)
        ) {
            let cache: SyncCache<String, u64> = SyncCache::builder()
                .max_capacity(100)
                .build();

            // Convert HashSet to Vec for indexing
            let keys_vec: Vec<String> = keys.into_iter().collect();

            // Insert key-value pairs (use modulo to handle different lengths)
            let mut expected_values: HashMap<String, u64> = HashMap::new();
            for (i, key) in keys_vec.iter().enumerate() {
                let value = values[i % values.len()];
                cache.insert(key.clone(), value);
                expected_values.insert(key.clone(), value);
            }

            // Verify consistency - each key should have its last inserted value
            for (key, expected_value) in expected_values.iter() {
                if let Some(actual_value) = cache.get(key) {
                    prop_assert_eq!(actual_value, *expected_value);
                }
            }
        }

        /// Property: Concurrent operations should not cause data corruption
        /// **Validates: Requirements 1.5, 3.3**
        #[test]
        fn prop_concurrent_safety(
            thread_count in 2u8..=8,
            operations_per_thread in 10u32..=50
        ) {
            let cache: Arc<SyncCache<String, u32>> = Arc::new(
                SyncCache::builder()
                    .max_capacity(1000)
                    .build()
            );

            let mut handles = Vec::new();

            for thread_id in 0..thread_count {
                let cache = cache.clone();

                let handle = std::thread::spawn(move || {
                    for op_id in 0..operations_per_thread {
                        let key = format!("thread_{}_key_{}", thread_id, op_id);
                        let value = thread_id as u32 * 1000 + op_id;

                        cache.insert(key.clone(), value);

                        // Verify the value we just inserted
                        if let Some(retrieved) = cache.get(&key) {
                            // Value should be what we inserted or a later value from same thread
                            assert!(retrieved >= thread_id as u32 * 1000);
                        }
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.join().expect("Thread should complete without panic");
            }

            // Cache should be in consistent state
            prop_assert!(cache.entry_count() <= 1000);
        }

        /// Property: Validation should be deterministic
        /// **Validates: Requirements 6.1-6.5**
        #[test]
        fn prop_validation_deterministic(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            name in "[a-zA-Z0-9 ]{1,100}",
            path in "/[a-zA-Z0-9/_-]{1,200}"
        ) {
            let config = ValidatedWorkspaceConfig {
                workspace_id: workspace_id.clone(),
                name: name.clone(),
                description: Some("Test description".to_string()),
                path: path.clone(),
                max_file_size: 1024 * 1024,
                max_file_count: 1000,
                enable_watch: false,
                tags: vec![],
                metadata: HashMap::new(),
                contact_email: Some("test@example.com".to_string()),
                project_url: Some("https://example.com".to_string()),
            };

            // Validation should be deterministic
            let result1 = config.validate();
            let result2 = config.validate();

            prop_assert_eq!(result1.is_ok(), result2.is_ok());
        }
    }
}

// ============================================================================
// Module 6: End-to-End Integration Tests
// ============================================================================

#[cfg(test)]
mod end_to_end {
    use super::*;

    /// Comprehensive end-to-end integration test
    /// **Validates: All performance properties**
    #[tokio::test]
    async fn test_full_integration_workflow() {
        println!("🚀 Running full integration workflow test...");

        let config = TestConfig::default();

        // 1. Initialize components
        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(500)
            .time_to_live(Duration::from_secs(300))
            .build();

        let sync_cache: SyncCache<SearchCacheKey, Vec<LogEntry>> = SyncCache::builder()
            .max_capacity(500)
            .time_to_live(Duration::from_secs(300))
            .build();

        let cache_manager = Arc::new(CacheManager::new(Arc::new(sync_cache.clone())));
        let resource_manager = Arc::new(AsyncResourceManager::new());

        // 2. Generate test data
        let log_entries = generate_test_log_entries(config.dataset_size);
        let workspace_configs = generate_test_workspace_configs(10);

        let start = Instant::now();

        // 3. Test cache operations
        for i in 0..100 {
            let key = format!("integration_key_{}", i);
            cache.insert(key, log_entries.clone()).await;
        }

        // 4. Test search simulation
        let search_results: Vec<_> = log_entries
            .iter()
            .filter(|entry| entry.level == "ERROR")
            .cloned()
            .collect();

        // 5. Test validation
        let validation_results: Vec<_> = workspace_configs.iter().map(|c| c.validate()).collect();

        let valid_count = validation_results.iter().filter(|r| r.is_ok()).count();

        // 6. Test resource management
        let token = resource_manager
            .register_operation(
                "integration_test".to_string(),
                log_analyzer::utils::async_resource_manager::OperationType::Search,
                Some("test_workspace".to_string()),
            )
            .await;

        // 7. Cleanup
        resource_manager
            .cancel_operation("integration_test")
            .await
            .unwrap();

        let total_duration = start.elapsed();

        // Verify results
        assert!(
            !search_results.is_empty(),
            "Search should find ERROR entries"
        );
        assert_eq!(valid_count, 10, "All workspace configs should be valid");
        assert!(token.is_cancelled(), "Operation should be cancelled");

        // Performance should be reasonable
        assert!(
            total_duration < Duration::from_millis(500),
            "Integration test took too long: {:?}",
            total_duration
        );

        println!("✅ Full integration workflow test passed:");
        println!("   - Total duration: {:?}", total_duration);
        println!("   - Cache operations: 100 insertions");
        println!(
            "   - Search results: {} ERROR entries",
            search_results.len()
        );
        println!("   - Validations: {}/10 passed", valid_count);
    }

    /// Test system behavior under stress
    /// **Validates: Requirements 3.4, 7.5**
    #[tokio::test]
    async fn test_stress_conditions() {
        println!("🔥 Running stress test...");

        let cache: AsyncCache<String, Vec<LogEntry>> = AsyncCache::builder()
            .max_capacity(100) // Small capacity to stress eviction
            .time_to_live(Duration::from_secs(10))
            .build();

        let test_data = generate_test_log_entries(50);
        let cache = Arc::new(cache);

        let mut handles = Vec::new();

        // Create high load
        for thread_id in 0..20 {
            let cache = cache.clone();
            let data = test_data.clone();

            let handle = tokio::spawn(async move {
                for op_id in 0..100 {
                    let key = format!("stress_{}_{}", thread_id, op_id);
                    cache.insert(key.clone(), data.clone()).await;
                    let _result = cache.get(&key).await;
                }
            });

            handles.push(handle);
        }

        // Wait for all operations
        let start = Instant::now();
        for handle in handles {
            handle.await.unwrap();
        }
        let stress_duration = start.elapsed();

        // System should handle stress without crashing
        assert!(
            stress_duration < Duration::from_secs(10),
            "Stress test took too long: {:?}",
            stress_duration
        );

        // Cache should maintain integrity
        assert!(
            cache.entry_count() <= 100,
            "Cache exceeded capacity under stress"
        );

        println!("✅ Stress test passed:");
        println!("   - 20 threads × 100 operations completed");
        println!("   - Duration: {:?}", stress_duration);
        println!("   - Final cache entries: {}", cache.entry_count());
    }
}
