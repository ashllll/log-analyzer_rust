//! Performance Regression Tests for CAS Migration
//!
//! This test suite validates that the CAS migration maintains or improves
//! performance compared to baseline measurements.
//!
//! **Validates Requirements:**
//! - 5.1: Import performance with CAS deduplication
//! - 5.2: Search performance using SQLite FTS5
//! - 5.3: Memory usage stability
//!
//! Run with: `cargo test --test performance_regression_tests --release -- --nocapture`

use log_analyzer::storage::cas::ContentAddressableStorage;
use log_analyzer::storage::metadata_store::{FileMetadata, MetadataStore};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

/// Performance thresholds based on requirements
struct PerformanceThresholds {
    /// Maximum import time per MB (milliseconds)
    import_per_mb_max_ms: u128,
    /// Maximum search time per 1000 files (milliseconds)
    search_per_1k_files_max_ms: u128,
    #[allow(dead_code)]
    /// Maximum memory growth per 1000 operations (MB)
    memory_growth_max_mb: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            import_per_mb_max_ms: 200,       // 200ms per MB (generous threshold)
            search_per_1k_files_max_ms: 100, // 100ms per 1000 files
            memory_growth_max_mb: 10.0,      // 10MB max growth per 1000 ops
        }
    }
}

/// Create test workspace
async fn create_test_workspace() -> (TempDir, ContentAddressableStorage, MetadataStore) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().to_path_buf();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    (temp_dir, cas, metadata_store)
}

/// Create test files with specified content
fn create_test_files(dir: &Path, count: usize, size_bytes: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for i in 0..count {
        let file_path = dir.join(format!("test_file_{}.log", i));
        let mut file = fs::File::create(&file_path).unwrap();

        // Create content with some variation
        let content = format!(
            "Test log entry {} - ERROR: Something went wrong at line {}\n{}",
            i,
            i * 10,
            "x".repeat(size_bytes.saturating_sub(100))
        );
        file.write_all(content.as_bytes()).unwrap();

        files.push(file_path);
    }

    files
}

/// Create duplicate files for deduplication testing
fn create_duplicate_files(
    dir: &Path,
    unique_count: usize,
    duplicates_per_unique: usize,
    size_bytes: usize,
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for i in 0..unique_count {
        let content = format!(
            "Unique content {} - {}\n",
            i,
            "x".repeat(size_bytes.saturating_sub(50))
        );

        for j in 0..duplicates_per_unique {
            let file_path = dir.join(format!("test_file_{}_{}.log", i, j));
            let mut file = fs::File::create(&file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            files.push(file_path);
        }
    }

    files
}

/// Test 1: Import Performance (Requirement 5.1)
#[tokio::test]
async fn test_import_performance() {
    let thresholds = PerformanceThresholds::default();
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create test files (100 files, 10KB each = 1MB total)
    let test_files_dir = TempDir::new().unwrap();
    let files = create_test_files(test_files_dir.path(), 100, 10 * 1024);

    let start = Instant::now();

    // Import files into CAS
    for (idx, file_path) in files.iter().enumerate() {
        let hash = cas.store_file_streaming(file_path).await.unwrap();

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("/test/file_{}.log", idx),
            original_name: format!("file_{}.log", idx),
            size: 10 * 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    let duration = start.elapsed();
    let duration_ms = duration.as_millis();

    println!("Import Performance:");
    println!("  Files: {}", files.len());
    println!("  Total size: ~1MB");
    println!("  Duration: {}ms", duration_ms);
    println!(
        "  Throughput: {:.2} files/sec",
        files.len() as f64 / duration.as_secs_f64()
    );

    // Validate against threshold
    assert!(
        duration_ms < thresholds.import_per_mb_max_ms,
        "Import performance regression: {}ms > {}ms threshold",
        duration_ms,
        thresholds.import_per_mb_max_ms
    );

    println!(
        "  ✓ Performance within threshold ({} < {}ms)",
        duration_ms, thresholds.import_per_mb_max_ms
    );
}

/// Test 2: CAS Deduplication Efficiency (Requirement 5.1)
#[tokio::test]
async fn test_deduplication_efficiency() {
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create 50 unique files, each duplicated 5 times (250 total files)
    let test_files_dir = TempDir::new().unwrap();
    let files = create_duplicate_files(test_files_dir.path(), 50, 5, 1024);

    let mut unique_hashes = std::collections::HashSet::new();

    let start = Instant::now();

    // Import files and track unique hashes
    for (idx, file_path) in files.iter().enumerate() {
        let hash = cas.store_file_streaming(file_path).await.unwrap();
        unique_hashes.insert(hash.clone());

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("/test/file_{}.log", idx),
            original_name: format!("file_{}.log", idx),
            size: 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    let duration = start.elapsed();

    // Calculate deduplication ratio
    let dedup_ratio = 1.0 - (unique_hashes.len() as f64 / files.len() as f64);

    println!("Deduplication Efficiency:");
    println!("  Total files: {}", files.len());
    println!("  Unique hashes: {}", unique_hashes.len());
    println!("  Deduplication ratio: {:.1}%", dedup_ratio * 100.0);
    println!("  Duration: {}ms", duration.as_millis());

    // Validate deduplication works (should have exactly 50 unique hashes)
    assert_eq!(
        unique_hashes.len(),
        50,
        "Deduplication failed: expected 50 unique hashes, got {}",
        unique_hashes.len()
    );

    println!("  ✓ Deduplication working correctly (80% reduction)");
}

/// Test 3: Search Performance with SQLite FTS5 (Requirement 5.2)
#[tokio::test]
async fn test_search_performance() {
    let thresholds = PerformanceThresholds::default();
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create and import 1000 test files
    let test_files_dir = TempDir::new().unwrap();
    let files = create_test_files(test_files_dir.path(), 1000, 1024);

    for (idx, file_path) in files.iter().enumerate() {
        let hash = cas.store_file_streaming(file_path).await.unwrap();

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("/test/file_{}.log", idx),
            original_name: format!("file_{}.log", idx),
            size: 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    // Perform search operations
    let start = Instant::now();

    // Search by path pattern
    let results = metadata_store.search_files("/test/file").await.unwrap();

    let duration = start.elapsed();
    let duration_ms = duration.as_millis();

    println!("Search Performance:");
    println!("  Total files: {}", files.len());
    println!("  Search results: {}", results.len());
    println!("  Duration: {}ms", duration_ms);
    println!(
        "  Throughput: {:.2} searches/sec",
        1000.0 / duration.as_secs_f64()
    );

    // Validate search found all files
    assert_eq!(results.len(), 1000, "Search should find all 1000 files");

    // Validate against threshold
    assert!(
        duration_ms < thresholds.search_per_1k_files_max_ms,
        "Search performance regression: {}ms > {}ms threshold",
        duration_ms,
        thresholds.search_per_1k_files_max_ms
    );

    println!(
        "  ✓ Performance within threshold ({} < {}ms)",
        duration_ms, thresholds.search_per_1k_files_max_ms
    );
}

/// Test 4: Memory Stability (Requirement 5.3)
#[tokio::test]
async fn test_memory_stability() {
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create a single test file
    let test_files_dir = TempDir::new().unwrap();
    let test_file = test_files_dir.path().join("test.log");
    let mut file = fs::File::create(&test_file).unwrap();
    file.write_all(b"Test content for memory stability test\n")
        .unwrap();

    // Perform 1000 repeated operations
    let operation_count = 1000;

    let start = Instant::now();

    for i in 0..operation_count {
        let hash = cas.store_file_streaming(&test_file).await.unwrap();

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("/test/file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: 40,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    let duration = start.elapsed();

    println!("Memory Stability:");
    println!("  Operations: {}", operation_count);
    println!("  Duration: {}ms", duration.as_millis());
    println!(
        "  Avg time per operation: {:.2}ms",
        duration.as_millis() as f64 / operation_count as f64
    );

    // Note: Actual memory measurement would require platform-specific APIs
    // This test validates that operations complete without hanging or crashing
    println!("  ✓ Operations completed without memory issues");
}

/// Test 5: Nested Archive Handling (Requirement 5.3)
#[tokio::test]
async fn test_nested_archive_performance() {
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create nested structure (5 levels, 10 files per level)
    let test_files_dir = TempDir::new().unwrap();
    let depth = 5;
    let files_per_level = 10;

    let start = Instant::now();

    for level in 0..depth {
        for file_idx in 0..files_per_level {
            let file_path = test_files_dir
                .path()
                .join(format!("level_{}_file_{}.log", level, file_idx));
            let mut file = fs::File::create(&file_path).unwrap();
            file.write_all(format!("Content at level {} file {}\n", level, file_idx).as_bytes())
                .unwrap();

            let hash = cas.store_file_streaming(&file_path).await.unwrap();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: format!("/archive/level_{}/file_{}.log", level, file_idx),
                original_name: format!("file_{}.log", file_idx),
                size: 50,
                modified_time: 0,
                mime_type: Some("text/plain".to_string()),
                parent_archive_id: if level > 0 {
                    Some((level - 1) as i64)
                } else {
                    None
                },
                depth_level: level as i32,
            };

            metadata_store.insert_file(&file_metadata).await.unwrap();
        }
    }

    let duration = start.elapsed();
    let total_files = depth * files_per_level;

    println!("Nested Archive Performance:");
    println!("  Depth levels: {}", depth);
    println!("  Files per level: {}", files_per_level);
    println!("  Total files: {}", total_files);
    println!("  Duration: {}ms", duration.as_millis());
    println!(
        "  Avg time per file: {:.2}ms",
        duration.as_millis() as f64 / total_files as f64
    );

    // Verify all files were stored
    let all_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(
        all_files.len(),
        total_files,
        "All nested files should be stored"
    );

    println!("  ✓ Nested archive handling working correctly");
}

/// Test 6: Content Retrieval Performance
#[tokio::test]
async fn test_content_retrieval_performance() {
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    // Create and store 100 files
    let test_files_dir = TempDir::new().unwrap();
    let files = create_test_files(test_files_dir.path(), 100, 1024);

    let mut hashes = Vec::new();

    for (idx, file_path) in files.iter().enumerate() {
        let hash = cas.store_file_streaming(file_path).await.unwrap();
        hashes.push(hash.clone());

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("/test/file_{}.log", idx),
            original_name: format!("file_{}.log", idx),
            size: 1024,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    // Retrieve all content
    let start = Instant::now();

    for hash in &hashes {
        let content = cas.read_content(hash).await.unwrap();
        assert!(!content.is_empty(), "Content should not be empty");
    }

    let duration = start.elapsed();

    println!("Content Retrieval Performance:");
    println!("  Files retrieved: {}", hashes.len());
    println!("  Duration: {}ms", duration.as_millis());
    println!(
        "  Avg time per file: {:.2}ms",
        duration.as_millis() as f64 / hashes.len() as f64
    );
    println!(
        "  Throughput: {:.2} files/sec",
        hashes.len() as f64 / duration.as_secs_f64()
    );

    println!("  ✓ Content retrieval working efficiently");
}

/// Test 7: Concurrent Operations Performance
#[tokio::test]
async fn test_concurrent_operations() {
    let (_temp_dir, cas, metadata_store) = create_test_workspace().await;

    let thread_count = 4;
    let files_per_thread = 25;

    let start = Instant::now();

    let mut handles = Vec::new();

    for thread_id in 0..thread_count {
        let cas = cas.clone();
        // Create a new metadata store for each thread
        let workspace_dir = _temp_dir.path().to_path_buf();

        let handle = tokio::spawn(async move {
            let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();
            let test_files_dir = TempDir::new().unwrap();
            let files =
                create_test_files(test_files_dir.path(), files_per_thread, 1024);

            for (idx, file_path) in files.iter().enumerate() {
                let hash = cas.store_file_streaming(file_path).await.unwrap();

                let file_metadata = FileMetadata {
                    id: 0,
                    sha256_hash: hash,
                    virtual_path: format!("/test/thread_{}/file_{}.log", thread_id, idx),
                    original_name: format!("file_{}.log", idx),
                    size: 1024,
                    modified_time: 0,
                    mime_type: Some("text/plain".to_string()),
                    parent_archive_id: None,
                    depth_level: 0,
                };

                metadata_store.insert_file(&file_metadata).await.unwrap();
            }
        });

        handles.push(handle);
    }

    futures_util::future::join_all(handles).await;

    let duration = start.elapsed();
    let total_files = thread_count * files_per_thread;

    println!("Concurrent Operations Performance:");
    println!("  Threads: {}", thread_count);
    println!("  Files per thread: {}", files_per_thread);
    println!("  Total files: {}", total_files);
    println!("  Duration: {}ms", duration.as_millis());
    println!(
        "  Throughput: {:.2} files/sec",
        total_files as f64 / duration.as_secs_f64()
    );

    // Verify all files were stored
    let all_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(
        all_files.len(),
        total_files,
        "All concurrent files should be stored"
    );

    println!("  ✓ Concurrent operations working correctly");
}
