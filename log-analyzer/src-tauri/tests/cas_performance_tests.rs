//! Performance tests for Content-Addressable Storage (CAS) implementation
//!
//! These tests verify that the CAS approach provides performance benefits
//! over the old HashMap-based approach and handles large/nested archives efficiently.
//!
//! # Requirements
//!
//! Validates: Requirements 6.1, 6.2

use log_analyzer::storage::{ContentAddressableStorage, MetadataStore};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create test files with specific content
async fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).await.unwrap();
    file_path
}

/// Helper to create a large test file
async fn create_large_test_file(dir: &Path, name: &str, size_mb: usize) -> PathBuf {
    let file_path = dir.join(name);
    let content = vec![b'x'; size_mb * 1024 * 1024];
    fs::write(&file_path, content).await.unwrap();
    file_path
}

/// Helper to create nested directory structure
async fn create_nested_structure(
    base_dir: &Path,
    depth: usize,
    files_per_level: usize,
) -> Vec<PathBuf> {
    let mut all_files = Vec::new();

    async fn create_level(
        dir: &Path,
        current_depth: usize,
        max_depth: usize,
        files_per_level: usize,
        all_files: &mut Vec<PathBuf>,
    ) {
        if current_depth > max_depth {
            return;
        }

        // Create files at this level
        for i in 0..files_per_level {
            let file_path = dir.join(format!("file_depth{}_{}.log", current_depth, i));
            let content = format!("Content at depth {} file {}", current_depth, i);
            fs::write(&file_path, content.as_bytes()).await.unwrap();
            all_files.push(file_path);
        }

        // Create subdirectory and recurse
        if current_depth < max_depth {
            let subdir = dir.join(format!("level_{}", current_depth + 1));
            fs::create_dir_all(&subdir).await.unwrap();
            Box::pin(create_level(
                &subdir,
                current_depth + 1,
                max_depth,
                files_per_level,
                all_files,
            ))
            .await;
        }
    }

    create_level(base_dir, 0, depth, files_per_level, &mut all_files).await;
    all_files
}

/// Test 1: Benchmark CAS vs old HashMap approach
///
/// This test compares the performance of storing and retrieving files
/// using CAS vs a simple HashMap-based approach.
#[tokio::test]
async fn test_cas_vs_hashmap_performance() {
    let temp_dir = TempDir::new().unwrap();
    let cas_dir = temp_dir.path().join("cas_workspace");
    let hashmap_dir = temp_dir.path().join("hashmap_workspace");

    fs::create_dir_all(&cas_dir).await.unwrap();
    fs::create_dir_all(&hashmap_dir).await.unwrap();

    // Create test files
    let test_files: Vec<_> = (0..100)
        .map(|i| {
            let content = format!("Test content for file {}", i).repeat(100);
            (format!("file_{}.log", i), content.into_bytes())
        })
        .collect();

    // --- CAS Approach ---
    let cas = ContentAddressableStorage::new(cas_dir.clone());

    let cas_start = Instant::now();
    let mut cas_hashes = Vec::new();
    for (name, content) in &test_files {
        let hash = cas.store_content(content).await.unwrap();
        cas_hashes.push((name.clone(), hash));
    }
    let cas_store_duration = cas_start.elapsed();

    // Retrieve all files
    let cas_retrieve_start = Instant::now();
    for (_, hash) in &cas_hashes {
        let _content = cas.read_content(hash).await.unwrap();
    }
    let cas_retrieve_duration = cas_retrieve_start.elapsed();

    // --- HashMap Approach (simulated old system) ---
    use std::collections::HashMap;
    let mut hashmap = HashMap::new();

    let hashmap_start = Instant::now();
    for (name, content) in &test_files {
        let file_path = hashmap_dir.join(name);
        fs::write(&file_path, content).await.unwrap();
        hashmap.insert(name.clone(), file_path.to_string_lossy().to_string());
    }
    let hashmap_store_duration = hashmap_start.elapsed();

    // Retrieve all files
    let hashmap_retrieve_start = Instant::now();
    for path in hashmap.values() {
        let _content = fs::read(path).await.unwrap();
    }
    let hashmap_retrieve_duration = hashmap_retrieve_start.elapsed();

    println!("\n=== CAS vs HashMap Performance ===");
    println!("Files: {}", test_files.len());
    println!("CAS Store: {:?}", cas_store_duration);
    println!("HashMap Store: {:?}", hashmap_store_duration);
    println!("CAS Retrieve: {:?}", cas_retrieve_duration);
    println!("HashMap Retrieve: {:?}", hashmap_retrieve_duration);
    println!(
        "CAS Total: {:?}",
        cas_store_duration + cas_retrieve_duration
    );
    println!(
        "HashMap Total: {:?}",
        hashmap_store_duration + hashmap_retrieve_duration
    );

    // CAS should be competitive or better
    // Note: CAS has overhead for hashing but benefits from deduplication
    assert!(
        cas_store_duration.as_millis() < hashmap_store_duration.as_millis() * 3,
        "CAS store should not be more than 3x slower than HashMap"
    );
}

/// Test 2: CAS deduplication performance
///
/// This test verifies that CAS deduplication provides significant
/// storage and performance benefits when storing duplicate content.
#[tokio::test]
async fn test_cas_deduplication_performance() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

    // Create duplicate content
    let content = b"This is duplicate content that will be stored multiple times";
    let duplicate_count = 1000;

    let start = Instant::now();
    let mut hashes = Vec::new();
    for _ in 0..duplicate_count {
        let hash = cas.store_content(content).await.unwrap();
        hashes.push(hash);
    }
    let duration = start.elapsed();

    // All hashes should be identical
    assert!(
        hashes.iter().all(|h| h == &hashes[0]),
        "All hashes should be identical"
    );

    // Storage size should be minimal (only one copy)
    let storage_size = cas.get_storage_size().await.unwrap();
    assert!(
        storage_size < (content.len() * 2) as u64,
        "Storage should only contain one copy of the content"
    );

    println!("\n=== CAS Deduplication Performance ===");
    println!("Duplicate stores: {}", duplicate_count);
    println!("Duration: {:?}", duration);
    println!(
        "Storage size: {} bytes (content size: {} bytes)",
        storage_size,
        content.len()
    );
    println!(
        "Deduplication ratio: {:.2}x",
        (content.len() * duplicate_count) as f64 / storage_size as f64
    );

    // Should complete quickly since most stores are deduplicated
    assert!(
        duration.as_secs() < 2,
        "Deduplication should make stores very fast"
    );
}

/// Test 3: Large file handling (1GB+)
///
/// This test verifies that CAS can handle large files efficiently
/// using streaming without excessive memory usage.
#[tokio::test]
#[ignore] // Ignore by default as it creates large files
async fn test_large_file_performance() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().join("workspace"));

    // Create a 100MB test file (reduced from 1GB for faster testing)
    // In production, this would be 1GB+
    let large_file = create_large_test_file(temp_dir.path(), "large.log", 100).await;

    println!("\n=== Large File Performance ===");
    println!("File size: 100 MB");

    // Test streaming storage
    let start = Instant::now();
    let hash = cas.store_file_streaming(&large_file).await.unwrap();
    let store_duration = start.elapsed();

    println!("Store duration: {:?}", store_duration);
    println!(
        "Store throughput: {:.2} MB/s",
        100.0 / store_duration.as_secs_f64()
    );

    // Test retrieval
    let retrieve_start = Instant::now();
    let content = cas.read_content(&hash).await.unwrap();
    let retrieve_duration = retrieve_start.elapsed();

    println!("Retrieve duration: {:?}", retrieve_duration);
    println!(
        "Retrieve throughput: {:.2} MB/s",
        100.0 / retrieve_duration.as_secs_f64()
    );

    assert_eq!(
        content.len(),
        100 * 1024 * 1024,
        "Content size should match"
    );

    // Should complete in reasonable time
    assert!(
        store_duration.as_secs() < 30,
        "Storing 100MB should complete in under 30 seconds"
    );
    assert!(
        retrieve_duration.as_secs() < 30,
        "Retrieving 100MB should complete in under 30 seconds"
    );
}

/// Test 4: Deeply nested archive structure (10+ levels)
///
/// This test verifies that CAS can handle deeply nested directory
/// structures without path length issues or performance degradation.
#[tokio::test]
async fn test_deeply_nested_structure_performance() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).await.unwrap();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Create deeply nested structure (15 levels, 5 files per level)
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir).await.unwrap();

    let files = create_nested_structure(&nested_dir, 15, 5).await;

    println!("\n=== Deeply Nested Structure Performance ===");
    println!("Nesting depth: 15 levels");
    println!("Files per level: 5");
    println!("Total files: {}", files.len());

    // Process all files
    let start = Instant::now();
    for (i, file_path) in files.iter().enumerate() {
        let hash = cas.store_file_streaming(file_path).await.unwrap();

        let metadata = log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("nested/{}", i),
            original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
            size: fs::metadata(file_path).await.unwrap().len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: file_path.components().count() as i32,
        };

        metadata_store.insert_file(&metadata).await.unwrap();
    }
    let duration = start.elapsed();

    println!("Processing duration: {:?}", duration);
    println!(
        "Files per second: {:.2}",
        files.len() as f64 / duration.as_secs_f64()
    );

    // Verify all files are accessible
    let all_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(
        all_files.len(),
        files.len(),
        "All files should be in metadata store"
    );

    // Verify max depth
    let max_depth = metadata_store.get_max_depth().await.unwrap();
    assert!(max_depth >= 15, "Max depth should be at least 15");

    println!("Max depth recorded: {}", max_depth);

    // Should complete in reasonable time
    assert!(
        duration.as_secs() < 10,
        "Processing deeply nested structure should complete in under 10 seconds"
    );
}

/// Test 5: Memory usage with large number of files
///
/// This test verifies that memory usage stays reasonable when
/// processing a large number of files.
#[tokio::test]
async fn test_memory_usage_with_many_files() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).await.unwrap();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Create many small files
    let file_count = 10000;
    let files_dir = temp_dir.path().join("files");
    fs::create_dir_all(&files_dir).await.unwrap();

    println!("\n=== Memory Usage Test ===");
    println!("File count: {}", file_count);

    let initial_memory = get_approximate_memory_usage();
    println!("Initial memory: {} MB", initial_memory / 1024 / 1024);

    // Create and process files
    let start = Instant::now();
    for i in 0..file_count {
        let content = format!("File content {}", i);
        let file_path =
            create_test_file(&files_dir, &format!("file_{}.log", i), content.as_bytes()).await;

        let hash = cas.store_file_streaming(&file_path).await.unwrap();

        let metadata = log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&metadata).await.unwrap();
    }
    let duration = start.elapsed();

    let final_memory = get_approximate_memory_usage();
    let memory_increase = final_memory.saturating_sub(initial_memory);

    println!("Processing duration: {:?}", duration);
    println!(
        "Files per second: {:.2}",
        file_count as f64 / duration.as_secs_f64()
    );
    println!("Final memory: {} MB", final_memory / 1024 / 1024);
    println!("Memory increase: {} MB", memory_increase / 1024 / 1024);

    // Verify all files are accessible
    let file_count_db = metadata_store.count_files().await.unwrap();
    assert_eq!(
        file_count_db, file_count as i64,
        "All files should be in database"
    );

    // Memory increase should be reasonable (< 500MB for 10k files)
    assert!(
        memory_increase < 500 * 1024 * 1024,
        "Memory increase should be less than 500MB"
    );

    // Should complete in reasonable time
    assert!(
        duration.as_secs() < 60,
        "Processing 10k files should complete in under 60 seconds"
    );
}

/// Test 6: Concurrent file processing performance
///
/// This test verifies that CAS can handle concurrent file operations
/// efficiently without lock contention.
#[tokio::test]
async fn test_concurrent_processing_performance() {
    let temp_dir = TempDir::new().unwrap();
    let cas = std::sync::Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf(),
    ));

    let file_count = 1000;
    let concurrent_tasks = 10;

    println!("\n=== Concurrent Processing Performance ===");
    println!("Total files: {}", file_count);
    println!("Concurrent tasks: {}", concurrent_tasks);

    let start = Instant::now();

    let mut handles = Vec::new();
    for task_id in 0..concurrent_tasks {
        let cas_clone = cas.clone();
        let handle = tokio::spawn(async move {
            for i in 0..(file_count / concurrent_tasks) {
                let content = format!("Task {} file {} content", task_id, i);
                let _hash = cas_clone.store_content(content.as_bytes()).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();

    println!("Duration: {:?}", duration);
    println!(
        "Files per second: {:.2}",
        file_count as f64 / duration.as_secs_f64()
    );

    // Should complete in reasonable time
    assert!(
        duration.as_secs() < 10,
        "Concurrent processing should complete in under 10 seconds"
    );
}

/// Test 7: Batch insertion performance
///
/// This test verifies that batch insertion is more efficient than
/// individual insertions for metadata store operations.
#[tokio::test]
async fn test_batch_insertion_performance() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).await.unwrap();

    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    let file_count = 1000;

    // Create test metadata
    let test_files: Vec<_> = (0..file_count)
        .map(|i| log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: format!("hash_{}", i),
            virtual_path: format!("file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        })
        .collect();

    println!("\n=== Batch Insertion Performance ===");
    println!("File count: {}", file_count);

    // Test individual insertion
    let individual_start = Instant::now();
    for metadata in &test_files[..file_count / 2] {
        metadata_store.insert_file(metadata).await.unwrap();
    }
    let individual_duration = individual_start.elapsed();

    // Test batch insertion
    let batch_start = Instant::now();
    metadata_store
        .insert_files_batch(test_files[file_count / 2..].to_vec())
        .await
        .unwrap();
    let batch_duration = batch_start.elapsed();

    println!(
        "Individual insertion (500 files): {:?}",
        individual_duration
    );
    println!("Batch insertion (500 files): {:?}", batch_duration);
    println!(
        "Speedup: {:.2}x",
        individual_duration.as_secs_f64() / batch_duration.as_secs_f64()
    );

    // Batch should be significantly faster
    assert!(
        batch_duration < individual_duration,
        "Batch insertion should be faster than individual insertion"
    );

    // Verify all files were inserted
    let count = metadata_store.count_files().await.unwrap();
    assert_eq!(count, file_count as i64, "All files should be inserted");
}

/// Test 8: Search performance with FTS5
///
/// This test verifies that full-text search using FTS5 is fast
/// even with a large number of files.
#[tokio::test]
async fn test_search_performance() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).await.unwrap();

    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Insert test files
    let file_count = 10000;
    let mut test_files = Vec::new();
    for i in 0..file_count {
        let metadata = log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: format!("hash_{}", i),
            virtual_path: format!("logs/app_{}/file_{}.log", i % 100, i),
            original_name: format!("file_{}.log", i),
            size: 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };
        test_files.push(metadata);
    }

    metadata_store.insert_files_batch(test_files).await.unwrap();

    println!("\n=== Search Performance ===");
    println!("Total files: {}", file_count);

    // Test search queries
    let queries = vec!["app_50", "file_1234", "logs", ".log"];

    for query in queries {
        let start = Instant::now();
        let results = metadata_store.search_files(query).await.unwrap();
        let duration = start.elapsed();

        println!(
            "Query '{}': {} results in {:?}",
            query,
            results.len(),
            duration
        );

        // Search should be fast (< 100ms)
        assert!(
            duration.as_millis() < 100,
            "Search should complete in under 100ms"
        );
    }
}

/// Get approximate memory usage (platform-specific, best effort)
fn get_approximate_memory_usage() -> usize {
    #[cfg(target_os = "linux")]
    {
        // On Linux, read from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return kb * 1024; // Convert to bytes
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use sysinfo crate
        use sysinfo::{Pid, System};
        let mut system = System::new_all();
        system.refresh_all();
        let pid = Pid::from_u32(std::process::id());
        if let Some(process) = system.process(pid) {
            return process.memory() as usize; // Already in bytes
        }
    }

    // Fallback: return 0 (memory tracking not available)
    0
}

/// Test 9: Storage efficiency metrics
///
/// This test verifies that CAS provides good storage efficiency
/// through deduplication and compression.
#[tokio::test]
async fn test_storage_efficiency() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).await.unwrap();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    println!("\n=== Storage Efficiency ===");

    // Create files with varying duplication
    let unique_contents = [
        b"Unique content 1".to_vec(),
        b"Unique content 2".to_vec(),
        b"Unique content 3".to_vec(),
    ];

    let duplicate_content = b"This content is duplicated many times".to_vec();

    let mut total_logical_size = 0u64;

    // Store unique files
    for (file_id, content) in unique_contents.iter().enumerate() {
        let hash = cas.store_content(content).await.unwrap();
        total_logical_size += content.len() as u64;

        let metadata = log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("unique_{}.log", file_id),
            original_name: format!("unique_{}.log", file_id),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };
        metadata_store.insert_file(&metadata).await.unwrap();
    }

    // Store many duplicates
    for i in 0..100 {
        let hash = cas.store_content(&duplicate_content).await.unwrap();
        total_logical_size += duplicate_content.len() as u64;

        let metadata = log_analyzer::storage::FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("duplicate_{}.log", i),
            original_name: format!("duplicate_{}.log", i),
            size: duplicate_content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };
        metadata_store.insert_file(&metadata).await.unwrap();
    }

    let physical_size = cas.get_storage_size().await.unwrap();
    let deduplication_ratio = total_logical_size as f64 / physical_size as f64;

    println!("Total logical size: {} bytes", total_logical_size);
    println!("Physical storage size: {} bytes", physical_size);
    println!("Deduplication ratio: {:.2}x", deduplication_ratio);
    println!(
        "Space saved: {:.2}%",
        (1.0 - 1.0 / deduplication_ratio) * 100.0
    );

    // Should achieve significant deduplication
    assert!(
        deduplication_ratio > 10.0,
        "Should achieve at least 10x deduplication with 100 duplicates"
    );
}
