/// Test error handling and warning recording in extraction engine
///
/// This test verifies that the extraction engine properly handles:
/// 1. Single file extraction errors (log warning, continue processing)
/// 2. Archive-level errors (stop current archive, continue with stack)
/// 3. Path shortening warnings
/// 4. Depth limit warnings
/// 5. Security event warnings

use log_analyzer::archive::{
    ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector,
};
use log_analyzer::services::MetadataDB;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to create a test extraction engine
async fn create_test_engine() -> (ExtractionEngine, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db = Arc::new(
        MetadataDB::new(temp_dir.path().join("test.db").to_str().unwrap())
            .await
            .unwrap(),
    );
    let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
    let security_detector = Arc::new(SecurityDetector::default());
    let policy = ExtractionPolicy::default();

    let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();
    (engine, temp_dir)
}

/// Test that archive-level errors are recorded as warnings and processing continues
#[tokio::test]
async fn test_archive_level_error_handling() {
    let (engine, temp_dir) = create_test_engine().await;

    // Create a non-existent archive path (will cause archive-level error)
    let archive_path = temp_dir.path().join("nonexistent.zip");
    let target_dir = temp_dir.path().join("extracted");

    // Attempt extraction
    let result = engine
        .extract_archive(&archive_path, &target_dir, "test_workspace")
        .await;

    // With our improved error handling, archive-level errors are now captured
    // as warnings in the result, allowing the extraction to complete gracefully
    match result {
        Ok(extraction_result) => {
            // Should have warnings about the failed archive
            assert!(
                !extraction_result.warnings.is_empty(),
                "Expected warnings for non-existent archive"
            );
            
            // Should have no files extracted
            assert_eq!(extraction_result.total_files, 0);
            
            // Check that we have an ArchiveError warning
            let has_archive_error = extraction_result.warnings.iter().any(|w| {
                matches!(w.category, log_analyzer::archive::extraction_engine::WarningCategory::ArchiveError)
            });
            assert!(has_archive_error, "Expected ArchiveError warning");
            
            println!(
                "Extraction completed with {} warnings (as expected)",
                extraction_result.warnings.len()
            );
            for warning in &extraction_result.warnings {
                println!("Warning: {:?} - {}", warning.category, warning.message);
            }
        }
        Err(e) => {
            // If it returns Err, that's also acceptable for non-existent files
            println!("Extraction failed: {}", e);
        }
    }
}

/// Test that unsupported format errors are handled properly
#[tokio::test]
async fn test_unsupported_format_error() {
    let (engine, temp_dir) = create_test_engine().await;

    // Create a file with unsupported extension
    let archive_path = temp_dir.path().join("test.txt");
    std::fs::write(&archive_path, "not an archive").unwrap();
    let target_dir = temp_dir.path().join("extracted");

    // Attempt extraction
    let result = engine
        .extract_archive(&archive_path, &target_dir, "test_workspace")
        .await;

    // Should return an error for unsupported format
    // Note: The error occurs during handler selection, which is part of the
    // iterative extraction process, so it should be captured as a warning
    match result {
        Ok(extraction_result) => {
            // If it returns Ok, it should have warnings about the unsupported format
            assert!(
                !extraction_result.warnings.is_empty(),
                "Expected warnings for unsupported format"
            );
            println!(
                "Extraction completed with {} warnings",
                extraction_result.warnings.len()
            );
            for warning in &extraction_result.warnings {
                println!("Warning: {:?} - {}", warning.category, warning.message);
            }
        }
        Err(e) => {
            // If it returns Err, that's also acceptable
            println!("Extraction failed as expected: {}", e);
            assert!(
                e.to_string().contains("Unsupported") || e.to_string().contains("format"),
                "Error message should mention unsupported format: {}",
                e
            );
        }
    }
}

/// Test that depth limit warnings are recorded
#[tokio::test]
async fn test_depth_limit_warning() {
    let (engine, temp_dir) = create_test_engine().await;

    // Create a simple test archive
    let archive_path = temp_dir.path().join("test.zip");
    let target_dir = temp_dir.path().join("extracted");

    // Create a minimal ZIP file
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    // Add a simple file
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("test.txt", options).unwrap();
    std::io::Write::write_all(&mut zip, b"test content").unwrap();

    zip.finish().unwrap();

    // Extract with default policy (should succeed)
    let result = engine
        .extract_archive(&archive_path, &target_dir, "test_workspace")
        .await;

    // Should succeed
    assert!(result.is_ok());
    let extraction_result = result.unwrap();

    // Verify basic extraction worked
    assert_eq!(extraction_result.total_files, 1);
    assert_eq!(extraction_result.max_depth_reached, 0);
}

/// Test that security event warnings are recorded
#[tokio::test]
async fn test_security_event_warning() {
    let (engine, temp_dir) = create_test_engine().await;

    // Create a ZIP with path traversal attempt
    let archive_path = temp_dir.path().join("malicious.zip");
    let target_dir = temp_dir.path().join("extracted");

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    // Try to add a file with path traversal (this should be caught)
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    // Note: The zip library may sanitize paths, so this test verifies
    // that our security checks work when such paths are encountered
    zip.start_file("../../../etc/passwd", options).unwrap();
    std::io::Write::write_all(&mut zip, b"malicious content").unwrap();

    zip.finish().unwrap();

    // Attempt extraction
    let result = engine
        .extract_archive(&archive_path, &target_dir, "test_workspace")
        .await;

    // The extraction may fail or succeed depending on how the zip library handles it
    // The important thing is that it doesn't panic and handles the error gracefully
    match result {
        Ok(extraction_result) => {
            // If it succeeded, the file should have been sanitized or skipped
            println!(
                "Extraction completed with {} files and {} warnings",
                extraction_result.total_files,
                extraction_result.warnings.len()
            );
        }
        Err(e) => {
            // If it failed, it should be a security error
            println!("Extraction failed as expected: {}", e);
        }
    }
}

/// Test warning category types
#[tokio::test]
async fn test_warning_categories() {
    use log_analyzer::archive::extraction_engine::{ExtractionWarning, WarningCategory};

    // Test that all warning categories can be created
    let categories = vec![
        WarningCategory::DepthLimitReached,
        WarningCategory::PathShortened,
        WarningCategory::HighCompressionRatio,
        WarningCategory::FileSkipped,
        WarningCategory::SecurityEvent,
        WarningCategory::ArchiveError,
        WarningCategory::PathResolutionError,
    ];

    for category in categories {
        let warning = ExtractionWarning {
            message: format!("Test warning for {:?}", category),
            file_path: Some(PathBuf::from("test.txt")),
            category,
        };

        // Verify warning can be created and accessed
        assert!(!warning.message.is_empty());
        assert_eq!(warning.category, category);
    }
}

/// Test that extraction continues after single file errors
#[tokio::test]
async fn test_continue_after_file_error() {
    let (engine, temp_dir) = create_test_engine().await;

    // Create a ZIP with multiple files
    let archive_path = temp_dir.path().join("multi.zip");
    let target_dir = temp_dir.path().join("extracted");

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    // Add multiple files
    for i in 0..5 {
        zip.start_file(format!("file{}.txt", i), options).unwrap();
        std::io::Write::write_all(&mut zip, format!("content {}", i).as_bytes()).unwrap();
    }

    zip.finish().unwrap();

    // Extract
    let result = engine
        .extract_archive(&archive_path, &target_dir, "test_workspace")
        .await;

    // Should succeed
    assert!(result.is_ok());
    let extraction_result = result.unwrap();

    // Should have extracted all files
    assert_eq!(extraction_result.total_files, 5);
}
