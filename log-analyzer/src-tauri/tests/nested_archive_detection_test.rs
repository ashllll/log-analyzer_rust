//! Integration tests for nested archive detection and processing
//!
//! These tests verify that the extraction engine correctly:
//! - Detects nested archives within extracted files
//! - Adds nested archives to the extraction stack
//! - Respects depth limits
//! - Logs appropriate warnings when depth limits are reached

use log_analyzer::archive::{
    ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector,
};
use log_analyzer::services::MetadataDB;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use zip::write::FileOptions;
use zip::ZipWriter;

/// Helper function to create a test extraction engine
async fn create_test_engine(policy: ExtractionPolicy) -> (ExtractionEngine, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());
    let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
    let security_detector = Arc::new(SecurityDetector::default());

    let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();

    (engine, temp_dir)
}

/// Helper function to create a ZIP archive with specified files
fn create_zip_archive(path: &Path, files: &[(&str, &[u8])]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (name, content) in files {
        zip.start_file(*name, options)?;
        zip.write_all(content)?;
    }

    zip.finish()?;
    Ok(())
}

/// Helper function to create a nested archive structure
/// Creates: outer.zip containing inner.zip containing file.txt
fn create_nested_archive(outer_path: &Path, inner_path: &Path) -> std::io::Result<()> {
    // Create inner archive
    create_zip_archive(inner_path, &[("file.txt", b"nested content")])?;

    // Read inner archive content
    let inner_content = std::fs::read(inner_path)?;

    // Create outer archive containing inner archive
    create_zip_archive(outer_path, &[("inner.zip", &inner_content)])?;

    Ok(())
}

#[tokio::test]
async fn test_detect_single_nested_archive() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create nested archive structure
    let inner_path = temp_dir.path().join("inner.zip");
    let outer_path = temp_dir.path().join("outer.zip");
    create_nested_archive(&outer_path, &inner_path).unwrap();

    // Extract outer archive
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&outer_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Verify that nested archive was detected and processed
    // The outer archive contains 1 file (inner.zip)
    // The inner archive contains 1 file (file.txt)
    // Total should be 2 files
    assert_eq!(
        result.total_files, 2,
        "Should extract 2 files total (1 from outer + 1 from inner)"
    );

    // Verify max depth reached is 1 (outer=0, inner=1)
    assert_eq!(
        result.max_depth_reached, 1,
        "Should reach depth 1 for nested archive"
    );

    // Verify the nested file exists
    let nested_file = extract_dir.join("inner/file.txt");
    assert!(
        nested_file.exists(),
        "Nested file should exist: {:?}",
        nested_file
    );

    // Verify content
    let content = tokio::fs::read_to_string(&nested_file).await.unwrap();
    assert_eq!(
        content, "nested content",
        "Nested file content should match"
    );
}

#[tokio::test]
async fn test_detect_multiple_nested_archives() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create multiple nested archives
    let inner1_path = temp_dir.path().join("inner1.zip");
    let inner2_path = temp_dir.path().join("inner2.zip");
    create_zip_archive(&inner1_path, &[("file1.txt", b"content1")]).unwrap();
    create_zip_archive(&inner2_path, &[("file2.txt", b"content2")]).unwrap();

    // Read inner archives
    let inner1_content = std::fs::read(&inner1_path).unwrap();
    let inner2_content = std::fs::read(&inner2_path).unwrap();

    // Create outer archive with multiple nested archives
    let outer_path = temp_dir.path().join("outer.zip");
    create_zip_archive(
        &outer_path,
        &[
            ("inner1.zip", &inner1_content),
            ("inner2.zip", &inner2_content),
            ("regular.txt", b"regular content"),
        ],
    )
    .unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&outer_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should extract:
    // - 3 files from outer (inner1.zip, inner2.zip, regular.txt)
    // - 1 file from inner1 (file1.txt)
    // - 1 file from inner2 (file2.txt)
    // Total: 5 files
    assert_eq!(
        result.total_files, 5,
        "Should extract 5 files total (3 from outer + 1 from inner1 + 1 from inner2)"
    );

    // Verify all files exist
    assert!(extract_dir.join("regular.txt").exists());
    assert!(extract_dir.join("inner1/file1.txt").exists());
    assert!(extract_dir.join("inner2/file2.txt").exists());
}

#[tokio::test]
async fn test_depth_limit_enforcement() {
    // Set a low depth limit
    let policy = ExtractionPolicy {
        max_depth: 2,
        ..Default::default()
    };

    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create deeply nested structure: level0.zip -> level1.zip -> level2.zip -> file.txt
    let level2_path = temp_dir.path().join("level2.zip");
    create_zip_archive(&level2_path, &[("file.txt", b"deep content")]).unwrap();

    let level2_content = std::fs::read(&level2_path).unwrap();
    let level1_path = temp_dir.path().join("level1.zip");
    create_zip_archive(&level1_path, &[("level2.zip", &level2_content)]).unwrap();

    let level1_content = std::fs::read(&level1_path).unwrap();
    let level0_path = temp_dir.path().join("level0.zip");
    create_zip_archive(&level0_path, &[("level1.zip", &level1_content)]).unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&level0_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should extract:
    // - level0 (depth 0): level1.zip
    // - level1 (depth 1): level2.zip
    // - level2 (depth 2): SKIPPED due to depth limit
    // Total: 2 files (level1.zip and level2.zip)
    assert_eq!(
        result.total_files, 2,
        "Should extract only 2 files due to depth limit"
    );

    // Verify max depth reached
    assert_eq!(
        result.max_depth_reached, 1,
        "Should reach depth 1 before hitting limit"
    );

    // Verify depth limit skip was recorded
    assert_eq!(
        result.depth_limit_skips, 1,
        "Should record 1 depth limit skip"
    );

    // Verify warning was generated
    assert!(
        !result.warnings.is_empty(),
        "Should have warnings about depth limit"
    );

    let depth_warnings: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains("Depth limit"))
        .collect();
    assert_eq!(
        depth_warnings.len(),
        1,
        "Should have exactly 1 depth limit warning"
    );

    // Verify level2.zip exists but was not extracted
    assert!(extract_dir.join("level1/level2.zip").exists());
    // Verify file.txt does NOT exist (because level2.zip was not extracted)
    assert!(!extract_dir.join("level1/level2/file.txt").exists());
}

#[tokio::test]
async fn test_nested_archive_with_regular_files() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create inner archive
    let inner_path = temp_dir.path().join("inner.zip");
    create_zip_archive(
        &inner_path,
        &[("nested1.txt", b"nested1"), ("nested2.txt", b"nested2")],
    )
    .unwrap();

    let inner_content = std::fs::read(&inner_path).unwrap();

    // Create outer archive with nested archive and regular files
    let outer_path = temp_dir.path().join("outer.zip");
    create_zip_archive(
        &outer_path,
        &[
            ("file1.txt", b"content1"),
            ("inner.zip", &inner_content),
            ("file2.txt", b"content2"),
        ],
    )
    .unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&outer_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should extract:
    // - 3 files from outer (file1.txt, inner.zip, file2.txt)
    // - 2 files from inner (nested1.txt, nested2.txt)
    // Total: 5 files
    assert_eq!(result.total_files, 5, "Should extract 5 files total");

    // Verify all files exist
    assert!(extract_dir.join("file1.txt").exists());
    assert!(extract_dir.join("file2.txt").exists());
    assert!(extract_dir.join("inner/nested1.txt").exists());
    assert!(extract_dir.join("inner/nested2.txt").exists());
}

#[tokio::test]
async fn test_archive_format_detection() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create archives with different extensions
    let zip_path = temp_dir.path().join("archive.zip");
    create_zip_archive(&zip_path, &[("file.txt", b"content")]).unwrap();

    let zip_content = std::fs::read(&zip_path).unwrap();

    // Create outer archive containing nested archives with different names
    let outer_path = temp_dir.path().join("outer.zip");
    create_zip_archive(
        &outer_path,
        &[
            ("nested.zip", &zip_content),
            ("nested.ZIP", &zip_content), // Test case insensitivity
            ("regular.txt", b"not an archive"),
        ],
    )
    .unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&outer_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should detect both .zip and .ZIP as archives
    // - 3 files from outer (nested.zip, nested.ZIP, regular.txt)
    // - 1 file from nested.zip (file.txt)
    // - 1 file from nested.ZIP (file.txt)
    // Total: 5 files
    assert_eq!(
        result.total_files, 5,
        "Should detect archives regardless of case"
    );

    // Verify nested extractions
    assert!(extract_dir.join("nested/file.txt").exists());
    assert!(extract_dir.join("nested/file.txt").exists());
}

#[tokio::test]
async fn test_empty_nested_archive() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create empty inner archive
    let inner_path = temp_dir.path().join("empty.zip");
    create_zip_archive(&inner_path, &[]).unwrap();

    let inner_content = std::fs::read(&inner_path).unwrap();

    // Create outer archive
    let outer_path = temp_dir.path().join("outer.zip");
    create_zip_archive(&outer_path, &[("empty.zip", &inner_content)]).unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&outer_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should extract:
    // - 1 file from outer (empty.zip)
    // - 0 files from empty.zip
    // Total: 1 file
    assert_eq!(
        result.total_files, 1,
        "Should extract 1 file (the empty archive itself)"
    );

    // Verify max depth reached is 1 (even though inner archive is empty)
    assert_eq!(
        result.max_depth_reached, 1,
        "Should still process empty nested archive"
    );
}

#[tokio::test]
async fn test_three_level_nesting() {
    let policy = ExtractionPolicy::default();
    let (engine, temp_dir) = create_test_engine(policy).await;

    // Create level 3 (deepest)
    let level3_path = temp_dir.path().join("level3.zip");
    create_zip_archive(&level3_path, &[("deep.txt", b"very deep content")]).unwrap();

    // Create level 2
    let level3_content = std::fs::read(&level3_path).unwrap();
    let level2_path = temp_dir.path().join("level2.zip");
    create_zip_archive(&level2_path, &[("level3.zip", &level3_content)]).unwrap();

    // Create level 1
    let level2_content = std::fs::read(&level2_path).unwrap();
    let level1_path = temp_dir.path().join("level1.zip");
    create_zip_archive(&level1_path, &[("level2.zip", &level2_content)]).unwrap();

    // Create level 0 (top)
    let level1_content = std::fs::read(&level1_path).unwrap();
    let level0_path = temp_dir.path().join("level0.zip");
    create_zip_archive(&level0_path, &[("level1.zip", &level1_content)]).unwrap();

    // Extract
    let extract_dir = temp_dir.path().join("extracted");
    let result = engine
        .extract_archive(&level0_path, &extract_dir, "test_workspace")
        .await
        .unwrap();

    // Should extract all levels:
    // - level0: level1.zip
    // - level1: level2.zip
    // - level2: level3.zip
    // - level3: deep.txt
    // Total: 4 files
    assert_eq!(result.total_files, 4, "Should extract all 4 files");

    // Verify max depth
    assert_eq!(
        result.max_depth_reached, 3,
        "Should reach depth 3 for three-level nesting"
    );

    // Verify the deepest file exists
    let deep_file = extract_dir.join("level1/level2/level3/deep.txt");
    assert!(
        deep_file.exists(),
        "Deep file should exist: {:?}",
        deep_file
    );

    let content = tokio::fs::read_to_string(&deep_file).await.unwrap();
    assert_eq!(content, "very deep content");
}
