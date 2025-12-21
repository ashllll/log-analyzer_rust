//! Performance validation tests
//!
//! These tests validate that our performance improvements are working correctly
//! without running full benchmarks.

use log_analyzer::models::log_entry::LogEntry;
use log_analyzer::models::validated::ValidatedWorkspaceConfig;
use log_analyzer::utils::validation::validate_path_safety;
use moka::future::Cache;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use validator::Validate;

/// Test cache performance improvements
#[tokio::test]
async fn test_cache_performance() {
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300))
        .time_to_idle(Duration::from_secs(60))
        .build();

    let test_data: Vec<LogEntry> = (0..100)
        .map(|i| LogEntry {
            id: i,
            content: format!("Test log entry {}", i),
            file: format!("/test/file_{}.log", i),
            real_path: format!("/test/file_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: "INFO".to_string(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // Test cache insertion performance
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key_{}", i);
        cache.insert(key, test_data.clone()).await;
    }
    let insert_duration = start.elapsed();

    // Should complete within reasonable time (< 100ms for 100 insertions)
    assert!(
        insert_duration < Duration::from_millis(100),
        "Cache insertion took too long: {:?}",
        insert_duration
    );

    // Test cache retrieval performance
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key_{}", i);
        let _result = cache.get(&key).await;
    }
    let get_duration = start.elapsed();

    // Should complete within reasonable time (< 50ms for 100 gets)
    assert!(
        get_duration < Duration::from_millis(50),
        "Cache retrieval took too long: {:?}",
        get_duration
    );

    println!("✅ Cache performance test passed:");
    println!("   - Insert 100 items: {:?}", insert_duration);
    println!("   - Get 100 items: {:?}", get_duration);
}

/// Test validation performance improvements
#[test]
fn test_validation_performance() {
    let workspace_configs: Vec<ValidatedWorkspaceConfig> = (0..100)
        .map(|i| ValidatedWorkspaceConfig {
            workspace_id: format!("workspace_{}", i),
            name: format!("Test Workspace {}", i),
            description: Some(format!("Test workspace description {}", i)),
            path: format!("/test/workspace/path_{}", i),
            max_file_size: 1024 * 1024, // 1MB
            max_file_count: 1000,
            enable_watch: true,
            tags: vec![format!("tag_{}", i)],
            metadata: HashMap::new(),
            contact_email: Some(format!("test{}@example.com", i)),
            project_url: Some(format!("https://example.com/project_{}", i)),
        })
        .collect();

    // Test validation performance
    let start = Instant::now();
    let results: Vec<_> = workspace_configs
        .iter()
        .map(|config| config.validate())
        .collect();
    let validation_duration = start.elapsed();

    // Should complete within reasonable time (< 50ms for 100 validations)
    assert!(
        validation_duration < Duration::from_millis(50),
        "Validation took too long: {:?}",
        validation_duration
    );

    // All validations should pass
    let valid_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(valid_count, 100, "Not all validations passed");

    println!("✅ Validation performance test passed:");
    println!("   - Validate 100 configs: {:?}", validation_duration);
    println!("   - All validations passed: {}/100", valid_count);
}

/// Test search performance improvements
#[test]
fn test_search_performance() {
    let log_entries: Vec<LogEntry> = (0..1000)
        .map(|i| LogEntry {
            id: i,
            content: format!("Test log entry {} with error message", i),
            file: format!("/test/file_{}.log", i),
            real_path: format!("/test/file_{}.log", i),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: if i % 10 == 0 {
                "ERROR".to_string()
            } else {
                "INFO".to_string()
            },
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    // Test simple search performance
    let start = Instant::now();
    let results: Vec<_> = log_entries
        .iter()
        .filter(|entry| entry.content.contains("error"))
        .collect();
    let search_duration = start.elapsed();

    // Should complete within reasonable time (< 10ms for 1000 entries)
    assert!(
        search_duration < Duration::from_millis(10),
        "Search took too long: {:?}",
        search_duration
    );

    // Should find the expected number of results
    assert_eq!(results.len(), 1000, "Unexpected search result count");

    println!("✅ Search performance test passed:");
    println!("   - Search 1000 entries: {:?}", search_duration);
    println!("   - Found {} results", results.len());
}

/// Test path security validation performance
#[test]
fn test_path_security_performance() {
    let test_paths = vec![
        "/safe/path/file.log",
        "../../../etc/passwd",
        "/path/with/unicode/测试文件.log",
        "/very/long/path/that/might/cause/performance/issues.log",
        "C:\\Windows\\System32\\config\\SAM",
        "/path/with/special/chars/file<>:\"|?*.log",
    ];

    // Repeat paths to create larger test set
    let large_path_set: Vec<_> = (0..100).flat_map(|_| test_paths.iter().cloned()).collect();

    // Test path security validation performance
    let start = Instant::now();
    let results: Vec<_> = large_path_set
        .iter()
        .map(|path| validate_path_safety(path))
        .collect();
    let validation_duration = start.elapsed();

    // Should complete within reasonable time (< 20ms for 600 validations)
    assert!(
        validation_duration < Duration::from_millis(20),
        "Path validation took too long: {:?}",
        validation_duration
    );

    // Count valid and invalid paths
    let valid_count = results.iter().filter(|r| r.is_ok()).count();
    let invalid_count = results.len() - valid_count;

    println!("✅ Path security performance test passed:");
    println!(
        "   - Validate {} paths: {:?}",
        results.len(),
        validation_duration
    );
    println!(
        "   - Valid paths: {}, Invalid paths: {}",
        valid_count, invalid_count
    );
}

/// Test concurrent cache operations
#[tokio::test]
async fn test_concurrent_cache_performance() {
    let cache = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300))
        .build();

    let test_data: Vec<LogEntry> = (0..10)
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

    // Test concurrent cache operations
    let start = Instant::now();
    let mut handles = Vec::new();

    for i in 0..4 {
        let cache = cache.clone();
        let data = test_data.clone();
        let handle = tokio::spawn(async move {
            for j in 0..25 {
                let key = format!("thread_{}_key_{}", i, j);
                cache.insert(key.clone(), data.clone()).await;
                let _result = cache.get(&key).await;
            }
        });
        handles.push(handle);
    }

    futures::future::join_all(handles).await;
    let concurrent_duration = start.elapsed();

    // Should complete within reasonable time (< 100ms for 4 threads * 25 ops each)
    assert!(
        concurrent_duration < Duration::from_millis(100),
        "Concurrent cache operations took too long: {:?}",
        concurrent_duration
    );

    println!("✅ Concurrent cache performance test passed:");
    println!("   - 4 threads × 25 operations: {:?}", concurrent_duration);
}

/// Test memory allocation efficiency
#[test]
fn test_memory_allocation_performance() {
    // Test memory allocation patterns for different data sizes
    for &data_size in &[100, 1000, 10000] {
        let start = Instant::now();

        let data: Vec<LogEntry> = (0..data_size)
            .map(|i| LogEntry {
                id: i,
                content: format!("Memory test entry {} with content", i),
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

        let allocation_duration = start.elapsed();

        // Memory allocation should scale reasonably
        let expected_max_duration = Duration::from_millis(data_size as u64 / 100);
        assert!(
            allocation_duration < expected_max_duration,
            "Memory allocation for {} entries took too long: {:?}",
            data_size,
            allocation_duration
        );

        println!(
            "✅ Memory allocation test passed for {} entries: {:?}",
            data_size, allocation_duration
        );

        // Ensure data is actually allocated
        assert_eq!(data.len(), data_size);
    }
}

/// Integration test for all performance improvements
#[tokio::test]
async fn test_integrated_performance() {
    println!("🚀 Running integrated performance test...");

    // Create cache
    let cache = Cache::builder()
        .max_capacity(500)
        .time_to_live(Duration::from_secs(300))
        .build();

    // Create test data
    let log_entries: Vec<LogEntry> = (0..500)
        .map(|i| LogEntry {
            id: i,
            content: format!("Integrated test entry {} with various content", i),
            file: format!("/integrated/test/file_{}.log", i % 50),
            real_path: format!("/integrated/test/file_{}.log", i % 50),
            line: i,
            timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
            level: ["INFO", "WARN", "ERROR", "DEBUG"][i % 4].to_string(),
            tags: vec![format!("tag_{}", i % 10)],
            match_details: None,
            matched_keywords: None,
        })
        .collect();

    let start = Instant::now();

    // 1. Cache operations
    for i in 0..100 {
        let key = format!("integrated_key_{}", i);
        cache.insert(key, log_entries.clone()).await;
    }

    // 2. Search operations
    let search_results: Vec<_> = log_entries
        .iter()
        .filter(|entry| entry.level == "ERROR")
        .collect();

    // 3. Validation operations
    let workspace_config = ValidatedWorkspaceConfig {
        workspace_id: "integrated_test".to_string(),
        name: "Integrated Test Workspace".to_string(),
        description: Some("Test workspace for integrated performance test".to_string()),
        path: "/integrated/test/workspace".to_string(),
        max_file_size: 1024 * 1024,
        max_file_count: 1000,
        enable_watch: true,
        tags: vec!["integration".to_string(), "test".to_string()],
        metadata: HashMap::new(),
        contact_email: Some("test@example.com".to_string()),
        project_url: Some("https://example.com/integrated-test".to_string()),
    };

    let validation_result = workspace_config.validate();

    // 4. Path security validation
    let test_paths = vec!["safe_path.log", "another_path.log", "测试.log"];

    let path_results: Vec<_> = test_paths
        .iter()
        .map(|path| validate_path_safety(path))
        .collect();

    let total_duration = start.elapsed();

    // Verify results
    assert!(
        !search_results.is_empty(),
        "Search should find ERROR entries"
    );
    assert!(
        validation_result.is_ok(),
        "Workspace validation should pass"
    );
    assert!(
        path_results.iter().all(|r| r.is_ok()),
        "All path validations should pass"
    );

    // Performance should be reasonable (< 200ms for integrated test)
    assert!(
        total_duration < Duration::from_millis(200),
        "Integrated performance test took too long: {:?}",
        total_duration
    );

    println!("✅ Integrated performance test passed:");
    println!("   - Total duration: {:?}", total_duration);
    println!("   - Cache operations: 100 insertions");
    println!(
        "   - Search results: {} ERROR entries found",
        search_results.len()
    );
    println!("   - Validation: workspace config validated");
    println!("   - Path security: {} paths validated", path_results.len());
}
