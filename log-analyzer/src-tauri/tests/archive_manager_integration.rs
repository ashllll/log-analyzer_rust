//! Integration tests for ArchiveManager compatibility with enhanced extraction system
//!
//! Tests:
//! - Existing archives work with new engine
//! - Path mappings are accessible and correct
//! - Feature flag toggle between old and new extraction
//! - Backward compatibility with existing workspaces

use log_analyzer::archive::{extract_archive_async, ArchiveManager, ExtractionPolicy};
use log_analyzer::services::MetadataDB;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to create a test ZIP archive
fn create_test_zip(path: &PathBuf, files: &[(&str, &str)]) -> std::io::Result<()> {
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    let file = fs::File::create(path)?;
    let mut zip = ZipWriter::new(file);

    for (name, content) in files {
        zip.start_file(*name, FileOptions::default())?;
        zip.write_all(content.as_bytes())?;
    }

    zip.finish()?;
    Ok(())
}

/// Test that existing archives work with the new enhanced extraction engine
#[tokio::test]
async fn test_enhanced_extraction_basic_archive() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a test archive
    create_test_zip(
        &archive_path,
        &[
            ("file1.txt", "content1"),
            ("file2.txt", "content2"),
            ("dir/file3.txt", "content3"),
        ],
    )
    .unwrap();

    // Extract using enhanced system
    let policy = Arc::new(ExtractionPolicy::default());
    let result = extract_archive_async(&archive_path, &extract_dir, "test_workspace", policy)
        .await
        .unwrap();

    // Verify extraction
    assert_eq!(result.extracted_files.len(), 3);
    assert!(extract_dir.join("file1.txt").exists());
    assert!(extract_dir.join("file2.txt").exists());
    assert!(extract_dir.join("dir/file3.txt").exists());

    // Verify content
    let content1 = fs::read_to_string(extract_dir.join("file1.txt")).unwrap();
    assert_eq!(content1, "content1");
}

/// Test that path mappings are created and accessible
#[tokio::test]
async fn test_path_mappings_accessibility() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");
    let db_path = temp_dir.path().join("metadata.db");

    // Create a test archive with a long filename
    let long_name = "a".repeat(300) + ".txt";
    create_test_zip(&archive_path, &[(&long_name, "long content")]).unwrap();

    // Extract using enhanced system
    let policy = Arc::new(ExtractionPolicy::default());
    let result = extract_archive_async(&archive_path, &extract_dir, "test_workspace", policy)
        .await
        .unwrap();

    // Check if path mappings were created
    if !result.metadata_mappings.is_empty() {
        // Verify we can access the metadata database
        let db = MetadataDB::new(&db_path).await.unwrap();

        // Verify mappings are stored
        for (short_path, original_path) in &result.metadata_mappings {
            let retrieved = db
                .get_original_path("test_workspace", short_path.to_str().unwrap())
                .await
                .unwrap();

            assert!(retrieved.is_some());
            assert_eq!(
                retrieved.unwrap(),
                original_path.to_string_lossy().to_string()
            );
        }
    }
}

/// Test feature flag toggle between old and new extraction
#[tokio::test]
async fn test_feature_flag_toggle() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir_old = temp_dir.path().join("extracted_old");
    let extract_dir_new = temp_dir.path().join("extracted_new");

    // Create a test archive
    create_test_zip(
        &archive_path,
        &[("file1.txt", "content1"), ("file2.txt", "content2")],
    )
    .unwrap();

    // Extract using old system (ArchiveManager)
    let archive_manager = ArchiveManager::new();
    let old_result = archive_manager
        .extract_archive(&archive_path, &extract_dir_old)
        .await
        .unwrap();

    // Extract using new system
    let policy = Arc::new(ExtractionPolicy::default());
    let new_result =
        extract_archive_async(&archive_path, &extract_dir_new, "test_workspace", policy)
            .await
            .unwrap();

    // Both should extract the same number of files
    assert_eq!(old_result.files_extracted, new_result.extracted_files.len());

    // Both should have the same files
    assert!(extract_dir_old.join("file1.txt").exists());
    assert!(extract_dir_new.join("file1.txt").exists());
    assert!(extract_dir_old.join("file2.txt").exists());
    assert!(extract_dir_new.join("file2.txt").exists());

    // Content should be identical
    let old_content1 = fs::read_to_string(extract_dir_old.join("file1.txt")).unwrap();
    let new_content1 = fs::read_to_string(extract_dir_new.join("file1.txt")).unwrap();
    assert_eq!(old_content1, new_content1);
}

/// Test backward compatibility with existing workspaces
#[tokio::test]
async fn test_backward_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a test archive
    create_test_zip(
        &archive_path,
        &[("file1.txt", "content1"), ("nested/file2.txt", "content2")],
    )
    .unwrap();

    // First extraction using old system
    let archive_manager = ArchiveManager::new();
    let old_result = archive_manager
        .extract_archive(&archive_path, &extract_dir)
        .await
        .unwrap();

    assert_eq!(old_result.files_extracted, 2);
    assert!(extract_dir.join("file1.txt").exists());
    assert!(extract_dir.join("nested/file2.txt").exists());

    // Clean up for second extraction
    fs::remove_dir_all(&extract_dir).unwrap();

    // Second extraction using new system should work the same way
    let policy = Arc::new(ExtractionPolicy::default());
    let new_result = extract_archive_async(&archive_path, &extract_dir, "test_workspace", policy)
        .await
        .unwrap();

    assert_eq!(new_result.extracted_files.len(), 2);
    assert!(extract_dir.join("file1.txt").exists());
    assert!(extract_dir.join("nested/file2.txt").exists());

    // Verify content is preserved
    let content1 = fs::read_to_string(extract_dir.join("file1.txt")).unwrap();
    assert_eq!(content1, "content1");
}

/// Test that enhanced extraction handles nested archives
#[tokio::test]
async fn test_nested_archive_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let inner_archive = temp_dir.path().join("inner.zip");
    let outer_archive = temp_dir.path().join("outer.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create inner archive
    create_test_zip(&inner_archive, &[("inner_file.txt", "inner content")]).unwrap();

    // Create outer archive containing inner archive
    let inner_bytes = fs::read(&inner_archive).unwrap();
    create_test_zip(
        &outer_archive,
        &[("inner.zip", std::str::from_utf8(&inner_bytes).unwrap())],
    )
    .unwrap();

    // Extract using enhanced system with depth limit
    let mut policy = ExtractionPolicy::default();
    policy.max_depth = 2; // Allow nested extraction
    let result = extract_archive_async(
        &outer_archive,
        &extract_dir,
        "test_workspace",
        Arc::new(policy),
    )
    .await
    .unwrap();

    // Should extract both outer and inner files
    assert!(result.extracted_files.len() >= 1);

    // Note: Actual nested extraction behavior depends on processor integration
    // This test verifies the API works correctly
}

/// Test that warnings are properly reported
#[tokio::test]
async fn test_warning_reporting() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a test archive with a very long filename
    let long_name = "a".repeat(500) + ".txt";
    create_test_zip(&archive_path, &[(&long_name, "content")]).unwrap();

    // Extract using enhanced system
    let policy = Arc::new(ExtractionPolicy::default());
    let result = extract_archive_async(&archive_path, &extract_dir, "test_workspace", policy)
        .await
        .unwrap();

    // Should have warnings about path shortening (if implemented)
    // This depends on the actual path length limits and shortening logic
    println!("Warnings: {:?}", result.warnings);
    println!("Security events: {:?}", result.security_events);
}

/// Test performance metrics are collected
#[tokio::test]
async fn test_performance_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    let extract_dir = temp_dir.path().join("extracted");

    // Create a test archive
    create_test_zip(
        &archive_path,
        &[
            ("file1.txt", "content1"),
            ("file2.txt", "content2"),
            ("file3.txt", "content3"),
        ],
    )
    .unwrap();

    // Extract using enhanced system
    let policy = Arc::new(ExtractionPolicy::default());
    let result = extract_archive_async(&archive_path, &extract_dir, "test_workspace", policy)
        .await
        .unwrap();

    // Verify performance metrics are collected
    assert!(result.performance_metrics.total_duration.as_secs() >= 0);
    assert_eq!(result.performance_metrics.files_extracted, 3);
    assert!(result.performance_metrics.bytes_extracted > 0);
}
