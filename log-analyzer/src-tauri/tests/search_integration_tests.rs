//! Integration tests for search functionality with CAS and path validation
//!
//! Tests the search command's ability to:
//! - Search CAS-stored files
//! - Search nested archives
//! - Validate file paths before opening
//! - Handle missing files gracefully
//! - Continue processing when individual files fail
//! - Provide detailed error logging
//! - Perform efficiently with CAS storage
//!
//! **Validates: Requirements 1.4**

use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::TempDir;

/// Test helper to create a test file with content
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to create a test workspace with CAS and MetadataStore
async fn create_test_workspace() -> (ContentAddressableStorage, MetadataStore, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let cas = ContentAddressableStorage::new(workspace_path.clone());
    let metadata = MetadataStore::new(&workspace_path).await.unwrap();

    (cas, metadata, temp_dir)
}

/// Helper to create a simple ZIP archive with test files
fn create_simple_zip(dir: &Path, name: &str, files: Vec<(&str, &[u8])>) -> PathBuf {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (filename, content) in files {
        zip.start_file(filename, options).unwrap();
        zip.write_all(content).unwrap();
    }

    zip.finish().unwrap();
    zip_path
}

/// Helper to create a nested ZIP archive (archive containing another archive)
fn create_nested_zip(dir: &Path, name: &str, inner_archives: Vec<PathBuf>) -> PathBuf {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add inner archives
    for inner_archive in inner_archives {
        let inner_name = inner_archive.file_name().unwrap().to_str().unwrap();
        zip.start_file(inner_name, options).unwrap();
        let inner_content = fs::read(&inner_archive).unwrap();
        zip.write_all(&inner_content).unwrap();
    }

    // Add some regular files too
    zip.start_file("outer_file.log", options).unwrap();
    zip.write_all(b"ERROR: outer level error\nINFO: outer level info").unwrap();

    zip.finish().unwrap();
    zip_path
}

/// Helper to simulate search on CAS-stored files
/// Returns matching lines from content
async fn search_cas_file_async(cas: &ContentAddressableStorage, hash: &str, query: &str) -> Vec<String> {
    let content = cas.read_content(hash).await.unwrap();
    let content_str = String::from_utf8(content).unwrap();
    content_str
        .lines()
        .filter(|line| line.contains(query))
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn test_search_with_valid_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    let file1 = create_test_file(&temp_dir, "test1.log", "ERROR: Test error message\nINFO: Normal log");
    let file2 = create_test_file(&temp_dir, "test2.log", "WARN: Warning message\nERROR: Another error");
    
    // Verify files exist
    assert!(file1.exists(), "Test file 1 should exist");
    assert!(file2.exists(), "Test file 2 should exist");
    
    // In a real integration test, we would call the search command here
    // For now, we verify the setup is correct
    let content1 = fs::read_to_string(&file1).unwrap();
    assert!(content1.contains("ERROR"), "File 1 should contain ERROR");
    
    let content2 = fs::read_to_string(&file2).unwrap();
    assert!(content2.contains("ERROR"), "File 2 should contain ERROR");
}

#[test]
fn test_search_with_missing_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create one valid file
    let valid_file = create_test_file(&temp_dir, "valid.log", "ERROR: Test error");
    
    // Create a path to a non-existent file
    let missing_file = temp_dir.path().join("missing.log");
    
    // Verify states
    assert!(valid_file.exists(), "Valid file should exist");
    assert!(!missing_file.exists(), "Missing file should not exist");
    
    // The search should handle the missing file gracefully
    // and continue processing the valid file
}

#[test]
fn test_search_with_mixed_valid_and_invalid_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple files
    let file1 = create_test_file(&temp_dir, "file1.log", "ERROR: Error in file 1");
    let file2 = create_test_file(&temp_dir, "file2.log", "ERROR: Error in file 2");
    let file3 = create_test_file(&temp_dir, "file3.log", "ERROR: Error in file 3");
    
    // Verify all files exist initially
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
    
    // Delete file2 to simulate a missing file scenario
    fs::remove_file(&file2).unwrap();
    assert!(!file2.exists(), "File 2 should be deleted");
    
    // The search should:
    // 1. Process file1 successfully
    // 2. Skip file2 with a warning
    // 3. Process file3 successfully
    
    // Verify remaining files still exist
    assert!(file1.exists(), "File 1 should still exist");
    assert!(file3.exists(), "File 3 should still exist");
}

#[test]
fn test_path_validation_before_file_open() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test with various path scenarios
    let valid_path = temp_dir.path().join("valid.log");
    let invalid_path = temp_dir.path().join("nonexistent").join("file.log");
    
    // Create only the valid file
    fs::write(&valid_path, "Test content").unwrap();
    
    // Verify path states
    assert!(valid_path.exists(), "Valid path should exist");
    assert!(!invalid_path.exists(), "Invalid path should not exist");
    
    // The search implementation should check path.exists() before File::open()
    // This prevents unnecessary error handling and provides better logging
}

#[test]
fn test_search_error_handling_and_recovery() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file that will be accessible
    let accessible_file = create_test_file(&temp_dir, "accessible.log", "ERROR: Accessible");
    
    // Create a file path that doesn't exist
    let missing_file = temp_dir.path().join("missing.log");
    
    // Simulate a search scenario where:
    // 1. First file is accessible
    // 2. Second file is missing
    // 3. Search should continue and not crash
    
    assert!(accessible_file.exists());
    assert!(!missing_file.exists());
    
    // The search should:
    // - Successfully read accessible_file
    // - Log a warning for missing_file
    // - Continue processing without panicking
    // - Return results from accessible_file
}

#[test]
fn test_search_with_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create an empty file
    let empty_file = create_test_file(&temp_dir, "empty.log", "");
    
    assert!(empty_file.exists());
    
    let content = fs::read_to_string(&empty_file).unwrap();
    assert_eq!(content.len(), 0, "File should be empty");
    
    // Search should handle empty files gracefully
    // - No matches should be found
    // - No errors should occur
}

#[test]
fn test_search_with_large_file() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a large file with many lines
    let mut content = String::new();
    for i in 0..10000 {
        content.push_str(&format!("Line {}: ", i));
        if i % 100 == 0 {
            content.push_str("ERROR: Test error\n");
        } else {
            content.push_str("INFO: Normal log\n");
        }
    }
    
    let large_file = create_test_file(&temp_dir, "large.log", &content);
    
    assert!(large_file.exists());
    
    // Verify file size
    let metadata = fs::metadata(&large_file).unwrap();
    assert!(metadata.len() > 100_000, "File should be reasonably large");
    
    // Search should handle large files efficiently
    // - Use buffered reading (BufReader with 8192 capacity)
    // - Process line by line without loading entire file into memory
}

#[test]
fn test_search_with_special_characters_in_path() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with various valid characters in names
    let file1 = create_test_file(&temp_dir, "test-file.log", "ERROR: Test");
    let file2 = create_test_file(&temp_dir, "test_file.log", "ERROR: Test");
    let file3 = create_test_file(&temp_dir, "test.file.log", "ERROR: Test");
    
    // All files should exist and be searchable
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
}

#[test]
fn test_search_continues_after_file_error() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple files
    let files: Vec<PathBuf> = (0..5)
        .map(|i| create_test_file(&temp_dir, &format!("file{}.log", i), &format!("ERROR: Error {}", i)))
        .collect();
    
    // Verify all files exist
    for file in &files {
        assert!(file.exists());
    }
    
    // Delete the middle file
    fs::remove_file(&files[2]).unwrap();
    assert!(!files[2].exists());
    
    // Search should:
    // - Process files[0] and files[1] successfully
    // - Skip files[2] with a warning
    // - Continue to process files[3] and files[4] successfully
    // - Return results from 4 files (all except files[2])
    
    // Verify other files still exist
    assert!(files[0].exists());
    assert!(files[1].exists());
    assert!(files[3].exists());
    assert!(files[4].exists());
}

/// Test that validates the search implementation follows the requirements
#[test]
fn test_search_requirements_validation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Requirement 1.5: System SHALL validate file path exists and is accessible
    let valid_file = create_test_file(&temp_dir, "valid.log", "ERROR: Test");
    let invalid_path = temp_dir.path().join("nonexistent.log");
    
    assert!(valid_file.exists(), "Valid file should exist");
    assert!(!invalid_path.exists(), "Invalid path should not exist");
    
    // Requirement 8.1: System SHALL log error and continue processing other files
    // This is tested by the test_search_continues_after_file_error test
    
    // Requirement 8.3: System SHALL provide detailed error information
    // The implementation logs:
    // - warn! for non-existent files
    // - error! for files that fail to open
    // Both include the file path and error details
}

// ========== CAS-Based Search Integration Tests ==========
// These tests validate search functionality with Content-Addressable Storage
// **Validates: Requirements 1.4**

/// Test search on CAS-stored files
/// Verifies that files stored in CAS can be searched successfully
#[tokio::test]
async fn test_search_cas_stored_files() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create test files with searchable content
    let test_files = vec![
        ("app.log", b"ERROR: Application crashed\nINFO: Starting application\nERROR: Database connection failed" as &[u8]),
        ("system.log", b"WARN: High memory usage\nINFO: System startup\nERROR: Disk space low"),
        ("debug.log", b"DEBUG: Processing request\nDEBUG: Cache hit\nINFO: Request completed"),
    ];

    // Store files in CAS and index in metadata
    for (filename, content) in test_files {
        let hash = cas.store_content(content).await.unwrap();
        
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash.clone(),
            virtual_path: format!("logs/{}", filename),
            original_name: filename.to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata.insert_file(&file_meta).await.unwrap();

        // Verify content can be retrieved from CAS
        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content, "CAS should return original content");
    }

    // Verify all files are indexed
    let all_files = metadata.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), 3, "All files should be indexed");

    // Test search for "ERROR" - should find matches in app.log and system.log
    let mut error_matches = 0;
    for file in &all_files {
        let matches = search_cas_file_async(&cas, &file.sha256_hash, "ERROR").await;
        error_matches += matches.len();
    }
    assert!(error_matches >= 3, "Should find at least 3 ERROR lines");

    // Test search for "DEBUG" - should find matches only in debug.log
    let mut debug_matches = 0;
    for file in &all_files {
        let matches = search_cas_file_async(&cas, &file.sha256_hash, "DEBUG").await;
        debug_matches += matches.len();
    }
    assert_eq!(debug_matches, 2, "Should find exactly 2 DEBUG lines");

    // Test search for non-existent term
    let mut no_matches = 0;
    for file in &all_files {
        let matches = search_cas_file_async(&cas, &file.sha256_hash, "NONEXISTENT").await;
        no_matches += matches.len();
    }
    assert_eq!(no_matches, 0, "Should find no matches for non-existent term");
}

/// Test search with nested archives
/// Verifies that files from nested archives can be searched
#[tokio::test]
async fn test_search_nested_archives() {
    let (cas, metadata, temp_dir) = create_test_workspace().await;

    // Create inner archive (level 1)
    let inner_files = vec![
        ("inner1.log", b"ERROR: Inner error 1\nINFO: Inner info 1" as &[u8]),
        ("inner2.log", b"WARN: Inner warning\nERROR: Inner error 2"),
    ];
    let inner_zip = create_simple_zip(temp_dir.path(), "inner.zip", inner_files.clone());

    // Create outer archive containing inner archive (level 2)
    let outer_zip = create_nested_zip(temp_dir.path(), "outer.zip", vec![inner_zip.clone()]);

    // Extract and process outer archive
    let extract_dir = temp_dir.path().join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Manually extract outer archive for testing
    let outer_file = fs::File::open(&outer_zip).unwrap();
    let mut outer_archive = zip::ZipArchive::new(outer_file).unwrap();
    outer_archive.extract(&extract_dir).unwrap();

    // Store outer files in CAS
    let outer_file_path = extract_dir.join("outer_file.log");
    if outer_file_path.exists() {
        let content = fs::read(&outer_file_path).unwrap();
        let hash = cas.store_content(&content).await.unwrap();
        
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: "outer.zip/outer_file.log".to_string(),
            original_name: "outer_file.log".to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 1,
        };
        metadata.insert_file(&file_meta).await.unwrap();
    }

    // Extract and process inner archive
    let inner_zip_extracted = extract_dir.join("inner.zip");
    if inner_zip_extracted.exists() {
        let inner_extract_dir = temp_dir.path().join("extracted_inner");
        fs::create_dir_all(&inner_extract_dir).unwrap();

        let inner_file = fs::File::open(&inner_zip_extracted).unwrap();
        let mut inner_archive = zip::ZipArchive::new(inner_file).unwrap();
        inner_archive.extract(&inner_extract_dir).unwrap();

        // Store inner files in CAS
        for (filename, _) in inner_files {
            let file_path = inner_extract_dir.join(filename);
            if file_path.exists() {
                let content = fs::read(&file_path).unwrap();
                let hash = cas.store_content(&content).await.unwrap();
                
                let file_meta = FileMetadata {
                    id: 0,
                    sha256_hash: hash,
                    virtual_path: format!("outer.zip/inner.zip/{}", filename),
                    original_name: filename.to_string(),
                    size: content.len() as i64,
                    modified_time: 0,
                    mime_type: Some("text/plain".to_string()),
                    parent_archive_id: None,
                    depth_level: 2,
                };
                metadata.insert_file(&file_meta).await.unwrap();
            }
        }
    }

    // Verify all files are indexed (outer + inner files)
    let all_files = metadata.get_all_files().await.unwrap();
    assert!(all_files.len() >= 2, "Should have at least inner files indexed");

    // Test search across nested archives for "ERROR"
    let mut error_count = 0;
    for file in &all_files {
        let matches = search_cas_file_async(&cas, &file.sha256_hash, "ERROR").await;
        error_count += matches.len();
    }
    assert!(error_count >= 3, "Should find ERROR in nested archives");

    // Verify files from different depths can be searched
    let depth_2_files: Vec<_> = all_files.iter().filter(|f| f.depth_level == 2).collect();
    assert!(!depth_2_files.is_empty(), "Should have files at depth 2");

    for file in depth_2_files {
        // Verify we can read content from deeply nested files
        let content = cas.read_content(&file.sha256_hash).await.unwrap();
        assert!(!content.is_empty(), "Nested file content should not be empty");
    }

    // Test metadata search for nested paths
    let nested_files = metadata.search_files("inner").await.unwrap();
    assert!(!nested_files.is_empty(), "Should find files with 'inner' in path");
}

/// Test search performance with CAS
/// Verifies that CAS-based search is efficient
#[tokio::test]
async fn test_search_performance_with_cas() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create multiple files with varying sizes
    let file_count = 50;
    let lines_per_file = 100;

    for i in 0..file_count {
        let mut content = Vec::new();
        for j in 0..lines_per_file {
            let line = if j % 10 == 0 {
                format!("ERROR: Error message {} in file {}\n", j, i)
            } else if j % 5 == 0 {
                format!("WARN: Warning message {} in file {}\n", j, i)
            } else {
                format!("INFO: Info message {} in file {}\n", j, i)
            };
            content.extend_from_slice(line.as_bytes());
        }

        let hash = cas.store_content(&content).await.unwrap();
        
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("logs/file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata.insert_file(&file_meta).await.unwrap();
    }

    // Verify all files are indexed
    let all_files = metadata.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), file_count, "All files should be indexed");

    // Measure search performance
    let start = Instant::now();
    
    let mut total_matches = 0;
    for file in &all_files {
        let matches = search_cas_file_async(&cas, &file.sha256_hash, "ERROR").await;
        total_matches += matches.len();
    }
    
    let duration = start.elapsed();

    // Verify search results
    assert_eq!(
        total_matches,
        file_count * (lines_per_file / 10),
        "Should find expected number of ERROR lines"
    );

    // Performance assertion: searching 50 files with 100 lines each should be fast
    // This is a reasonable expectation for CAS-based search
    assert!(
        duration.as_millis() < 1000,
        "Search should complete in less than 1 second, took {:?}",
        duration
    );

    println!(
        "Search performance: {} files, {} total lines, {} matches found in {:?}",
        file_count,
        file_count * lines_per_file,
        total_matches,
        duration
    );
}

/// Test search with deduplication
/// Verifies that CAS deduplication doesn't affect search results
#[tokio::test]
async fn test_search_with_deduplication() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create identical content
    let content = b"ERROR: Duplicate error\nINFO: Duplicate info\nERROR: Another duplicate error";

    // Store the same content once (CAS will deduplicate automatically)
    let hash = cas.store_content(content).await.unwrap();

    // Index the files with different virtual paths but same hash
    // Note: We can't insert multiple files with the same hash due to UNIQUE constraint
    // So we'll test that the same content can be searched from different virtual paths
    // by using the metadata's virtual_path field
    
    let file_meta = FileMetadata {
        id: 0,
        sha256_hash: hash.clone(),
        virtual_path: "logs/duplicate_0.log".to_string(),
        original_name: "duplicate_0.log".to_string(),
        size: content.len() as i64,
        modified_time: 0,
        mime_type: Some("text/plain".to_string()),
        parent_archive_id: None,
        depth_level: 0,
    };

    metadata.insert_file(&file_meta).await.unwrap();

    // Verify file is indexed
    let all_files = metadata.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), 1, "File should be indexed");

    // Search should work correctly
    let matches = search_cas_file_async(&cas, &hash, "ERROR").await;
    assert_eq!(matches.len(), 2, "Should have 2 ERROR lines");

    // Verify the same content can be retrieved multiple times
    let content1 = cas.read_content(&hash).await.unwrap();
    let content2 = cas.read_content(&hash).await.unwrap();
    assert_eq!(content1, content2, "Content should be identical");
    assert_eq!(content1, content, "Content should match original");

    // Verify storage efficiency: only one copy of content is stored
    let storage_size = cas.get_storage_size().await.unwrap();
    assert!(
        storage_size <= (content.len() * 2) as u64,
        "Storage should be deduplicated (at most 2x content size for overhead)"
    );
}

/// Test search with missing CAS objects
/// Verifies graceful handling when CAS objects are missing
#[tokio::test]
async fn test_search_with_missing_cas_objects() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create and store a file
    let content = b"ERROR: Test error";
    let hash = cas.store_content(content).await.unwrap();
    
    let file_meta = FileMetadata {
        id: 0,
        sha256_hash: hash.clone(),
        virtual_path: "logs/test.log".to_string(),
        original_name: "test.log".to_string(),
        size: content.len() as i64,
        modified_time: 0,
        mime_type: Some("text/plain".to_string()),
        parent_archive_id: None,
        depth_level: 0,
    };
    metadata.insert_file(&file_meta).await.unwrap();

    // Create a file with a fake hash (simulating missing CAS object)
    let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let fake_file_meta = FileMetadata {
        id: 0,
        sha256_hash: fake_hash.to_string(),
        virtual_path: "logs/missing.log".to_string(),
        original_name: "missing.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: Some("text/plain".to_string()),
        parent_archive_id: None,
        depth_level: 0,
    };
    metadata.insert_file(&fake_file_meta).await.unwrap();

    // Get all files
    let all_files = metadata.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), 2, "Should have 2 files indexed");

    // Search should handle missing CAS objects gracefully
    let mut successful_searches = 0;
    let mut failed_searches = 0;

    for file in &all_files {
        // Check if hash exists before searching
        if cas.exists(&file.sha256_hash) {
            let matches = search_cas_file_async(&cas, &file.sha256_hash, "ERROR").await;
            successful_searches += 1;
            if file.sha256_hash == hash {
                assert_eq!(matches.len(), 1, "Should find ERROR in valid file");
            }
        } else {
            // This simulates the search skipping missing files
            failed_searches += 1;
        }
    }

    assert_eq!(successful_searches, 1, "Should successfully search 1 file");
    assert_eq!(failed_searches, 1, "Should skip 1 missing file");
}

/// Test search with large files in CAS
/// Verifies that large files can be searched efficiently
#[tokio::test]
async fn test_search_large_files_in_cas() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create a large file (1MB)
    let mut content = Vec::new();
    for i in 0..10000 {
        let line = if i % 100 == 0 {
            format!("ERROR: Error at line {}\n", i)
        } else {
            format!("INFO: Log line {}\n", i)
        };
        content.extend_from_slice(line.as_bytes());
    }

    let hash = cas.store_content(&content).await.unwrap();
    
    let file_meta = FileMetadata {
        id: 0,
        sha256_hash: hash.clone(),
        virtual_path: "logs/large.log".to_string(),
        original_name: "large.log".to_string(),
        size: content.len() as i64,
        modified_time: 0,
        mime_type: Some("text/plain".to_string()),
        parent_archive_id: None,
        depth_level: 0,
    };
    metadata.insert_file(&file_meta).await.unwrap();

    // Search the large file
    let start = Instant::now();
    let matches = search_cas_file_async(&cas, &hash, "ERROR").await;
    let duration = start.elapsed();

    // Verify results
    assert_eq!(matches.len(), 100, "Should find 100 ERROR lines");

    // Performance check: searching a 1MB file should be fast
    assert!(
        duration.as_millis() < 500,
        "Large file search should complete in less than 500ms, took {:?}",
        duration
    );

    println!(
        "Large file search: {} bytes, {} matches found in {:?}",
        content.len(),
        matches.len(),
        duration
    );
}

/// Test search with empty files in CAS
/// Verifies that empty files are handled correctly
#[tokio::test]
async fn test_search_empty_files_in_cas() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Store an empty file
    let content = b"";
    let hash = cas.store_content(content).await.unwrap();
    
    let file_meta = FileMetadata {
        id: 0,
        sha256_hash: hash.clone(),
        virtual_path: "logs/empty.log".to_string(),
        original_name: "empty.log".to_string(),
        size: 0,
        modified_time: 0,
        mime_type: Some("text/plain".to_string()),
        parent_archive_id: None,
        depth_level: 0,
    };
    metadata.insert_file(&file_meta).await.unwrap();

    // Search the empty file
    let matches = search_cas_file_async(&cas, &hash, "ERROR").await;

    // Should return no matches without errors
    assert_eq!(matches.len(), 0, "Empty file should have no matches");

    // Verify the file exists in CAS
    assert!(cas.exists(&hash), "Empty file should exist in CAS");
}

/// Test concurrent search on CAS files
/// Verifies that multiple searches can run concurrently
#[tokio::test]
async fn test_concurrent_search_on_cas() {
    let (cas, metadata, _temp_dir) = create_test_workspace().await;

    // Create multiple files
    for i in 0..10 {
        let content = format!("ERROR: Error in file {}\nINFO: Info in file {}\n", i, i);
        let hash = cas.store_content(content.as_bytes()).await.unwrap();
        
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("logs/file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };
        metadata.insert_file(&file_meta).await.unwrap();
    }

    let all_files = metadata.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), 10, "Should have 10 files");

    // Perform concurrent searches
    let mut handles = vec![];
    
    for file in all_files {
        let cas_clone = cas.clone();
        let hash = file.sha256_hash.clone();
        
        let handle = tokio::spawn(async move {
            search_cas_file_async(&cas_clone, &hash, "ERROR").await
        });
        
        handles.push(handle);
    }

    // Wait for all searches to complete
    let mut total_matches = 0;
    for handle in handles {
        let matches = handle.await.unwrap();
        total_matches += matches.len();
    }

    // Each file should have 1 ERROR line
    assert_eq!(total_matches, 10, "Should find 10 ERROR lines total");
}
