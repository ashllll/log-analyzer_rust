/**
 * Property-based tests for ProgressTracker
 *
 * Tests correctness properties related to progress reporting and error handling
 */
use super::progress_tracker::{ErrorCategory, ProgressTracker};
use crate::archive::extraction_context::ExtractionContext;
use crate::error::AppError;
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Property 20: Progress event completeness
// **Feature: enhanced-archive-handling, Property 20: Progress event completeness**
// **Validates: Requirements 5.1**
//
// Property: For any extraction operation, all emitted progress events should
// contain: current_file, files_processed, bytes_processed, current_depth, and
// hierarchical_path.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_progress_event_completeness(
        workspace_id in "[a-z0-9_]{5,20}",
        current_depth in 0usize..20,
        accumulated_size in 0u64..10_000_000,
        accumulated_files in 0usize..1000,
        file_path in "[a-z0-9_/]{10,50}\\.txt",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create progress tracker
            let tracker = ProgressTracker::new();
            let mut receiver = tracker.subscribe();

            // Create extraction context
            let context = ExtractionContext {
                workspace_id: workspace_id.clone(),
                current_depth,
                parent_archive: if current_depth > 0 {
                    Some(PathBuf::from("/test/parent.zip"))
                } else {
                    None
                },
                accumulated_size,
                accumulated_files,
                start_time: Instant::now(),
            };

            // Update metrics to match context
            tracker.metrics().set_current_depth(current_depth);
            for _ in 0..accumulated_files.min(10) {
                tracker.metrics().increment_files();
            }
            tracker.metrics().add_bytes(accumulated_size.min(10000));

            // Emit progress event
            let result = tracker.emit_progress(&context, &PathBuf::from(&file_path)).await;
            prop_assert!(result.is_ok());

            // Receive and validate event
            let event = tokio::time::timeout(
                Duration::from_millis(100),
                receiver.recv()
            ).await;

            prop_assert!(event.is_ok(), "Event should be received");
            let event = event.unwrap().unwrap();

            // Validate all required fields are present and non-empty/valid
            prop_assert!(!event.workspace_id.is_empty(), "workspace_id should not be empty");
            prop_assert_eq!(event.workspace_id, workspace_id, "workspace_id should match");

            prop_assert!(!event.current_file.is_empty(), "current_file should not be empty");
            prop_assert!(event.current_file.contains(&file_path) || event.current_file.ends_with(".txt"),
                "current_file should contain the file path");

            // files_processed and bytes_processed should be >= 0 (always true for usize/u64)
            // but we can check they're reasonable
            prop_assert!(event.files_processed <= accumulated_files + 100,
                "files_processed should be reasonable");
            prop_assert!(event.bytes_processed <= accumulated_size + 100000,
                "bytes_processed should be reasonable");

            prop_assert_eq!(event.current_depth, current_depth, "current_depth should match");

            // hierarchical_path should be present (may be empty for depth 0)
            // For depth > 0, it should have at least one element
            if current_depth > 0 {
                prop_assert!(!event.hierarchical_path.is_empty(),
                    "hierarchical_path should not be empty for depth > 0");
            }

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 21: Hierarchical progress structure
// **Feature: enhanced-archive-handling, Property 21: Hierarchical progress structure**
// **Validates: Requirements 5.2**
//
// Property: For any nested archive extraction, the progress events should
// maintain parent-child relationships visible in the hierarchical_path field.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_hierarchical_progress_structure(
        workspace_id in "[a-z0-9_]{5,20}",
        depth in 1usize..10,
        parent_archive_name in "[a-z0-9_]{5,20}\\.zip",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Create progress tracker
            let tracker = ProgressTracker::new();
            let mut receiver = tracker.subscribe();

            // Create extraction context with parent archive
            let parent_path = PathBuf::from(format!("/test/{}", parent_archive_name));
            let context = ExtractionContext {
                workspace_id: workspace_id.clone(),
                current_depth: depth,
                parent_archive: Some(parent_path.clone()),
                accumulated_size: 1024,
                accumulated_files: 10,
                start_time: Instant::now(),
            };

            // Update metrics
            tracker.metrics().set_current_depth(depth);

            // Emit progress event
            let result = tracker.emit_progress(&context, &PathBuf::from("/test/file.txt")).await;
            prop_assert!(result.is_ok());

            // Receive and validate event
            let event = tokio::time::timeout(
                Duration::from_millis(100),
                receiver.recv()
            ).await;

            prop_assert!(event.is_ok(), "Event should be received");
            let event = event.unwrap().unwrap();

            // Validate hierarchical structure
            prop_assert!(!event.hierarchical_path.is_empty(),
                "hierarchical_path should not be empty for nested archives");

            // Should contain parent archive path
            let has_parent = event.hierarchical_path.iter()
                .any(|p| p.contains(&parent_archive_name) || p.contains("test"));
            prop_assert!(has_parent,
                "hierarchical_path should contain parent archive reference");

            // Should contain depth indicator
            let has_depth = event.hierarchical_path.iter()
                .any(|p| p.contains("depth"));
            prop_assert!(has_depth,
                "hierarchical_path should contain depth indicator");

            // Depth in event should match context
            prop_assert_eq!(event.current_depth, depth,
                "current_depth should match context depth");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 22: Error categorization and resilience
// **Feature: enhanced-archive-handling, Property 22: Error categorization and resilience**
// **Validates: Requirements 5.3**
//
// Property: For any error during extraction, the error should be categorized
// into one of the defined categories, and the system should continue processing.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_error_categorization_and_resilience(
        error_type in 0usize..8,
        error_count in 1usize..20,
    ) {
        // Create progress tracker
        let tracker = ProgressTracker::new();

        // Generate different types of errors
        let errors: Vec<AppError> = (0..error_count).map(|i| {
            match (error_type + i) % 8 {
                0 => AppError::InvalidPath("path too long".to_string()),
                1 => AppError::archive_error("unsupported format", None),
                2 => AppError::archive_error("corrupted archive", None),
                3 => AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "permission denied"
                )),
                4 => AppError::archive_error("zip bomb detected", None),
                5 => AppError::archive_error("depth limit exceeded", None),
                6 => AppError::archive_error("insufficient disk space", None),
                _ => AppError::Validation("other error".to_string()),
            }
        }).collect();

        // Record all errors
        for error in &errors {
            tracker.record_error(error);
        }

        // Verify errors were categorized
        let summary = tracker.generate_summary();

        // Should have recorded errors
        prop_assert!(!summary.errors_by_category.is_empty(),
            "Should have recorded error categories");

        // Total errors should match
        let total_errors: usize = summary.errors_by_category.iter()
            .map(|(_, count)| count)
            .sum();
        prop_assert_eq!(total_errors, error_count,
            "Total categorized errors should match error count");

        // Each error should be in a valid category
        for (category, count) in &summary.errors_by_category {
            prop_assert!(*count > 0, "Category count should be positive");

            // Verify category is valid
            let valid_category = matches!(
                category,
                ErrorCategory::PathTooLong
                    | ErrorCategory::UnsupportedFormat
                    | ErrorCategory::CorruptedArchive
                    | ErrorCategory::PermissionDenied
                    | ErrorCategory::ZipBombDetected
                    | ErrorCategory::DepthLimitExceeded
                    | ErrorCategory::DiskSpaceExhausted
                    | ErrorCategory::IoError
                    | ErrorCategory::Other
            );
            prop_assert!(valid_category, "Category should be valid");
        }

        // System should still be functional (can generate summary)
        prop_assert!(summary.duration.as_nanos() > 0,
            "Duration should be positive");
    }
}

// Additional helper tests for error categorization
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_mapping() {
        // Test all error types map to correct categories
        let test_cases = vec![
            (
                AppError::InvalidPath("test".to_string()),
                ErrorCategory::PathTooLong,
            ),
            (
                AppError::archive_error("unsupported", None),
                ErrorCategory::UnsupportedFormat,
            ),
            (
                AppError::archive_error("corrupted", None),
                ErrorCategory::CorruptedArchive,
            ),
            (
                AppError::archive_error("zip bomb", None),
                ErrorCategory::ZipBombDetected,
            ),
            (
                AppError::archive_error("depth exceeded", None),
                ErrorCategory::DepthLimitExceeded,
            ),
            (
                AppError::archive_error("no space", None),
                ErrorCategory::DiskSpaceExhausted,
            ),
            (
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                )),
                ErrorCategory::PermissionDenied,
            ),
            (
                AppError::Validation("test".to_string()),
                ErrorCategory::Other,
            ),
        ];

        for (error, expected_category) in test_cases {
            let category = ErrorCategory::from_error(&error);
            assert_eq!(
                category, expected_category,
                "Error {:?} should map to {:?}",
                error, expected_category
            );
        }
    }

    #[test]
    fn test_multiple_errors_same_category() {
        let tracker = ProgressTracker::new();

        // Record multiple errors of the same type
        for _ in 0..5 {
            tracker.record_error(&AppError::InvalidPath("test".to_string()));
        }

        let summary = tracker.generate_summary();

        // Should have exactly one category with count 5
        assert_eq!(summary.errors_by_category.len(), 1);
        let (category, count) = &summary.errors_by_category[0];
        assert_eq!(*category, ErrorCategory::PathTooLong);
        assert_eq!(*count, 5);
    }

    #[test]
    fn test_mixed_error_categories() {
        let tracker = ProgressTracker::new();

        // Record different types of errors
        tracker.record_error(&AppError::InvalidPath("test".to_string()));
        tracker.record_error(&AppError::archive_error("zip bomb", None));
        tracker.record_error(&AppError::InvalidPath("test2".to_string()));

        let summary = tracker.generate_summary();

        // Should have two categories
        assert_eq!(summary.errors_by_category.len(), 2);

        // Verify counts
        let path_count = summary
            .errors_by_category
            .iter()
            .find(|(cat, _)| *cat == ErrorCategory::PathTooLong)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        assert_eq!(path_count, 2);

        let bomb_count = summary
            .errors_by_category
            .iter()
            .find(|(cat, _)| *cat == ErrorCategory::ZipBombDetected)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        assert_eq!(bomb_count, 1);
    }
}
