//! Example usage of CAS test helpers
//!
//! This file demonstrates how to use the CAS test helper functions
//! in integration tests. These examples show the recommended patterns
//! for testing CAS-based functionality.
//!
//! **Validates: Requirements 4.1, 4.2**

mod cas_test_helpers;

use cas_test_helpers::{
    create_cas_workspace, populate_cas_workspace, verify_cas_workspace, PopulateConfig,
};

/// Example: Basic workspace creation and verification
#[tokio::test]
async fn example_basic_workspace_test() {
    // Create a new CAS workspace
    let workspace = create_cas_workspace().await.unwrap();

    // Populate with default configuration (5 files, 100 lines each)
    let files = populate_cas_workspace(&workspace, PopulateConfig::default())
        .await
        .unwrap();

    // Verify the workspace is valid
    let result = verify_cas_workspace(&workspace).await.unwrap();
    assert!(result.is_ok(), "Workspace should be valid");

    // Use the files for testing
    assert_eq!(files.len(), 5);
    for file in files {
        assert!(file.sha256_hash.len() == 64);
        assert!(file.size > 0);
    }
}

/// Example: Custom workspace configuration
#[tokio::test]
async fn example_custom_workspace_test() {
    let workspace = create_cas_workspace().await.unwrap();

    // Create a custom configuration
    let config = PopulateConfig {
        file_count: 20,
        lines_per_file: 200,
        include_archives: false,
        archive_depth: 0,
    };

    let files = populate_cas_workspace(&workspace, config).await.unwrap();

    assert_eq!(files.len(), 20);

    // Verify all files are accessible
    for file in &files {
        let content = workspace.cas.read_content(&file.sha256_hash).await.unwrap();
        assert!(!content.is_empty());
    }
}

/// Example: Testing with nested archives
#[tokio::test]
async fn example_nested_archives_test() {
    let workspace = create_cas_workspace().await.unwrap();

    let config = PopulateConfig {
        file_count: 5,
        lines_per_file: 50,
        include_archives: true,
        archive_depth: 2,
    };

    let files = populate_cas_workspace(&workspace, config).await.unwrap();

    // Should have regular files plus archive files
    assert!(files.len() >= 5);

    // Verify files at different depths
    let depth_0_files: Vec<_> = files.iter().filter(|f| f.depth_level == 0).collect();
    let depth_2_files: Vec<_> = files.iter().filter(|f| f.depth_level == 2).collect();

    assert!(!depth_0_files.is_empty(), "Should have depth 0 files");
    assert!(!depth_2_files.is_empty(), "Should have depth 2 files");
}

/// Example: Testing search functionality with CAS
#[tokio::test]
async fn example_search_test() {
    let workspace = create_cas_workspace().await.unwrap();

    // Populate with test data
    populate_cas_workspace(&workspace, PopulateConfig::default())
        .await
        .unwrap();

    // Get all files from metadata
    let all_files = workspace.metadata.get_all_files().await.unwrap();

    // Search for "ERROR" in all files
    let mut error_count = 0;
    for file in all_files {
        let content = workspace.cas.read_content(&file.sha256_hash).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();

        for line in content_str.lines() {
            if line.contains("ERROR") {
                error_count += 1;
            }
        }
    }

    // Each file has 10 ERROR lines (every 10th line)
    assert_eq!(error_count, 50); // 5 files * 10 errors each
}

/// Example: Testing deduplication
#[tokio::test]
async fn example_deduplication_test() {
    let workspace = create_cas_workspace().await.unwrap();

    // Store the same content twice
    let content = b"Duplicate content for testing";
    let hash1 = workspace.cas.store_content(content).await.unwrap();
    let hash2 = workspace.cas.store_content(content).await.unwrap();

    // Hashes should be identical (deduplication)
    assert_eq!(hash1, hash2);

    // Content should be retrievable
    let retrieved = workspace.cas.read_content(&hash1).await.unwrap();
    assert_eq!(retrieved, content);
}

/// Example: Testing workspace integrity
#[tokio::test]
async fn example_integrity_test() {
    let workspace = create_cas_workspace().await.unwrap();

    // Populate workspace
    populate_cas_workspace(&workspace, PopulateConfig::default())
        .await
        .unwrap();

    // Verify integrity
    let result = verify_cas_workspace(&workspace).await.unwrap();

    // Check verification results
    assert!(result.is_ok(), "Workspace should pass verification");
    assert_eq!(result.missing_cas_objects, 0);
    assert_eq!(result.hash_mismatches, 0);
    assert_eq!(result.valid_cas_objects, result.total_files);

    println!("Verification: {}", result.summary());
}

/// Example: Testing with metadata queries
#[tokio::test]
async fn example_metadata_query_test() {
    let workspace = create_cas_workspace().await.unwrap();

    populate_cas_workspace(&workspace, PopulateConfig::default())
        .await
        .unwrap();

    // Query files by virtual path pattern (using LIKE instead of FTS search)
    // Note: search_files uses FTS5 which has special syntax requirements
    let all_files = workspace.metadata.get_all_files().await.unwrap();
    let log_files: Vec<_> = all_files
        .iter()
        .filter(|f| f.virtual_path.contains("logs/"))
        .collect();

    assert!(!log_files.is_empty());

    // All files should have "logs/" in their virtual path
    for file in log_files {
        assert!(file.virtual_path.contains("logs/"));
    }
}

/// Example: Testing file metadata
#[tokio::test]
async fn example_metadata_test() {
    let workspace = create_cas_workspace().await.unwrap();

    let files = populate_cas_workspace(&workspace, PopulateConfig::default())
        .await
        .unwrap();

    // Verify metadata properties
    for file in files {
        // Check hash format (SHA-256 is 64 hex characters)
        assert_eq!(file.sha256_hash.len(), 64);
        assert!(file.sha256_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Check size is positive
        assert!(file.size > 0);

        // Check virtual path format
        assert!(file.virtual_path.starts_with("logs/"));
        assert!(file.virtual_path.ends_with(".log"));

        // Check mime type
        assert_eq!(file.mime_type, Some("text/plain".to_string()));
    }
}

/// Example: Performance testing pattern
#[tokio::test]
async fn example_performance_test() {
    use std::time::Instant;

    let workspace = create_cas_workspace().await.unwrap();

    // Measure population time
    let start = Instant::now();
    let config = PopulateConfig {
        file_count: 50,
        lines_per_file: 100,
        include_archives: false,
        archive_depth: 0,
    };
    populate_cas_workspace(&workspace, config).await.unwrap();
    let populate_duration = start.elapsed();

    println!("Population took: {:?}", populate_duration);

    // Measure search time
    let all_files = workspace.metadata.get_all_files().await.unwrap();
    let start = Instant::now();

    let mut total_lines = 0;
    for file in all_files {
        let content = workspace.cas.read_content(&file.sha256_hash).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        total_lines += content_str.lines().count();
    }

    let search_duration = start.elapsed();

    println!("Search took: {:?}", search_duration);
    println!("Total lines processed: {}", total_lines);

    // Performance assertions
    assert!(
        populate_duration.as_secs() < 10,
        "Population should be fast"
    );
    assert!(search_duration.as_secs() < 5, "Search should be fast");
}
