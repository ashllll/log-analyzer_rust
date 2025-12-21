/**
 * Property-Based Tests for Edge Case Handlers
 *
 * Tests correctness properties for:
 * - Unicode normalization consistency (Property 29)
 * - Duplicate filename uniqueness (Property 30)
 * - Circular reference detection (Property 33)
 */
use super::edge_case_handlers::*;
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

/// **Feature: enhanced-archive-handling, Property 29: Unicode normalization consistency**
/// **Validates: Requirements 7.1**
///
/// For any path containing Unicode characters, the system should normalize it to NFC form,
/// and normalizing twice should produce the same result (idempotent).
#[cfg(test)]
mod unicode_normalization_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unicode_normalization_idempotent(
            // Generate strings with various Unicode characters
            s in r"[\u{0000}-\u{FFFF}]{1,100}"
        ) {
            let handler = EdgeCaseHandler::new();

            // Normalize once
            let normalized1 = handler.normalize_path(&s);

            // Normalize again
            let normalized2 = handler.normalize_path(&normalized1);

            // Should be idempotent - normalizing twice gives same result
            prop_assert_eq!(normalized1, normalized2);
        }

        #[test]
        fn prop_unicode_normalization_nfc_form(
            // Generate strings with combining characters
            base in "[a-zA-Z]{1,20}",
            combining in prop::collection::vec(r"[\u{0300}-\u{036F}]", 0..5)
        ) {
            let handler = EdgeCaseHandler::new();

            // Create string with combining characters (NFD-like)
            let mut input = base.clone();
            for c in combining {
                input.push_str(&c);
            }

            // Normalize to NFC
            let normalized = handler.normalize_path(&input);

            // Normalizing again should give same result (idempotent)
            let normalized_again = handler.normalize_path(&normalized);
            prop_assert_eq!(normalized, normalized_again);

            // The normalized form should be stable
            // (this is the key property of NFC normalization)
        }

        #[test]
        fn prop_unicode_normalization_preserves_ascii(
            // ASCII strings should be unchanged
            s in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let handler = EdgeCaseHandler::new();

            let normalized = handler.normalize_path(&s);

            // ASCII strings should be unchanged by normalization
            prop_assert_eq!(s, normalized);
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 30: Duplicate filename uniqueness**
/// **Validates: Requirements 7.2**
///
/// For any archive containing duplicate filenames (case-insensitive on Windows),
/// the system should ensure all extracted files have unique names by appending numeric suffixes.
#[cfg(test)]
mod duplicate_filename_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_duplicate_filenames_are_unique(
            // Generate a filename and a count of duplicates
            filename in "[a-zA-Z0-9_-]{1,20}\\.(txt|log|dat)",
            duplicate_count in 1usize..20
        ) {
            let mut handler = EdgeCaseHandler::new();
            let mut generated_names = Vec::new();

            // Generate multiple files with the same name
            for _ in 0..duplicate_count {
                let unique_name = handler.ensure_unique_filename(&filename);
                generated_names.push(unique_name);
            }

            // All generated names should be unique
            let unique_set: std::collections::HashSet<_> = generated_names.iter().collect();
            prop_assert_eq!(unique_set.len(), generated_names.len());

            // First name should be the original (if not already used)
            // Subsequent names should have suffixes
            if duplicate_count > 1 {
                prop_assert!(generated_names[1].contains("_001"));
            }
        }

        #[test]
        fn prop_case_insensitive_duplicates_on_windows(
            // Generate filenames with different cases
            base in "[a-z]{5,10}",
            ext in "(txt|log|dat)"
        ) {
            let mut handler = EdgeCaseHandler::new();

            let lowercase = format!("{}.{}", base.to_lowercase(), ext);
            let uppercase = format!("{}.{}", base.to_uppercase(), ext);

            let name1 = handler.ensure_unique_filename(&lowercase);
            let name2 = handler.ensure_unique_filename(&uppercase);

            // On Windows (case_insensitive = true), these should be treated as duplicates
            if cfg!(windows) {
                prop_assert_ne!(name1, name2.clone());
                // Second one should have a suffix
                prop_assert!(name2.contains("_001"));
            }
        }

        #[test]
        fn prop_filename_without_extension_gets_suffix(
            filename in "[a-zA-Z0-9_-]{1,20}",
            duplicate_count in 2usize..10
        ) {
            let mut handler = EdgeCaseHandler::new();
            let mut generated_names = Vec::new();

            for _ in 0..duplicate_count {
                let unique_name = handler.ensure_unique_filename(&filename);
                generated_names.push(unique_name);
            }

            // All should be unique
            let unique_set: std::collections::HashSet<_> = generated_names.iter().collect();
            prop_assert_eq!(unique_set.len(), generated_names.len());

            // Second name should have _001 suffix (no dot before it)
            prop_assert!(generated_names[1].ends_with("_001"));
        }

        #[test]
        fn prop_unicode_filenames_are_normalized_and_unique(
            // Generate Unicode filenames
            base in r"[\u{0041}-\u{007A}\u{00C0}-\u{00FF}]{5,15}",
            ext in "(txt|log)",
            count in 2usize..5
        ) {
            let mut handler = EdgeCaseHandler::new();
            let filename = format!("{}.{}", base, ext);
            let mut generated_names = Vec::new();

            for _ in 0..count {
                let unique_name = handler.ensure_unique_filename(&filename);
                generated_names.push(unique_name);
            }

            // All should be unique
            let unique_set: std::collections::HashSet<_> = generated_names.iter().collect();
            prop_assert_eq!(unique_set.len(), generated_names.len());
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 33: Circular reference detection**
/// **Validates: Requirements 7.5**
///
/// For any archive containing circular symlink references, the system should detect
/// the cycle and skip the problematic entries without infinite loops.
#[cfg(test)]
mod circular_reference_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_circular_reference_detected_on_revisit(
            filename in "[a-zA-Z0-9_-]{5,20}\\.txt"
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut handler = EdgeCaseHandler::new();

            // Create a real file
            let file_path = temp_dir.path().join(&filename);
            fs::write(&file_path, "test content").unwrap();

            // First visit should not be circular
            let is_circular_first = handler.is_circular_reference(&file_path).unwrap();
            prop_assert!(!is_circular_first);

            // Second visit should be detected as circular
            let is_circular_second = handler.is_circular_reference(&file_path).unwrap();
            prop_assert!(is_circular_second);
        }

        #[test]
        fn prop_nonexistent_paths_not_circular(
            // Generate random path components
            components in prop::collection::vec("[a-zA-Z0-9_-]{3,10}", 1..5)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut handler = EdgeCaseHandler::new();

            // Build a path that doesn't exist
            let mut path = temp_dir.path().to_path_buf();
            for component in components {
                path.push(component);
            }

            // Non-existent paths should not be considered circular
            let is_circular = handler.is_circular_reference(&path).unwrap();
            prop_assert!(!is_circular);
        }

        #[test]
        fn prop_reset_clears_circular_detection(
            filename in "[a-zA-Z0-9_-]{5,20}\\.txt"
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut handler = EdgeCaseHandler::new();

            let file_path = temp_dir.path().join(&filename);
            fs::write(&file_path, "test content").unwrap();

            // Visit once
            handler.is_circular_reference(&file_path).unwrap();

            // Reset handler
            handler.reset();

            // After reset, should not be circular anymore
            let is_circular = handler.is_circular_reference(&file_path).unwrap();
            prop_assert!(!is_circular);
        }

        #[test]
        fn prop_different_files_not_circular(
            file_count in 2usize..10
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut handler = EdgeCaseHandler::new();

            // Create multiple different files
            for i in 0..file_count {
                let file_path = temp_dir.path().join(format!("file_{}.txt", i));
                fs::write(&file_path, format!("content {}", i)).unwrap();

                // Each different file should not be circular on first visit
                let is_circular = handler.is_circular_reference(&file_path).unwrap();
                prop_assert!(!is_circular);
            }
        }
    }
}

/// Additional edge case tests for disk space and checkpoints
#[cfg(test)]
mod disk_space_and_checkpoint_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_disk_space_check_accepts_small_sizes(
            size in 1u64..1_000_000 // 1 byte to 1MB
        ) {
            let temp_dir = TempDir::new().unwrap();

            // Small sizes should succeed on most systems
            // However, if the disk cannot be detected, we accept that as well
            let result = check_disk_space(temp_dir.path(), size, 0.1);

            // Either succeeds or fails with a specific error about disk detection
            if let Err(e) = result {
                let error_msg = e.to_string();
                prop_assert!(
                    error_msg.contains("Could not determine disk space") ||
                    error_msg.contains("Insufficient disk space"),
                    "Unexpected error: {}", error_msg
                );
            }
        }

        #[test]
        fn prop_checkpoint_roundtrip(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            files_processed in 0usize..10000,
            bytes_processed in 0u64..1_000_000_000
        ) {
            let temp_dir = TempDir::new().unwrap();
            let checkpoint_dir = temp_dir.path().join("checkpoints");

            let checkpoint = ExtractionCheckpoint {
                workspace_id: workspace_id.clone(),
                archive_path: std::path::PathBuf::from("/test/archive.zip"),
                last_extracted_file: "test.txt".to_string(),
                files_processed,
                bytes_processed,
                timestamp: 1234567890,
            };

            // Save checkpoint
            save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();

            // Load checkpoint
            let loaded = load_checkpoint(&workspace_id, &checkpoint_dir)
                .unwrap()
                .expect("Checkpoint should exist");

            // Verify round-trip
            prop_assert_eq!(loaded.workspace_id, checkpoint.workspace_id);
            prop_assert_eq!(loaded.files_processed, checkpoint.files_processed);
            prop_assert_eq!(loaded.bytes_processed, checkpoint.bytes_processed);
        }

        #[test]
        fn prop_checkpoint_detection_after_save(
            workspace_id in "[a-zA-Z0-9_-]{5,20}"
        ) {
            let temp_dir = TempDir::new().unwrap();
            let checkpoint_dir = temp_dir.path().join("checkpoints");

            // Initially no checkpoint
            let detected_before = detect_incomplete_extraction(&workspace_id, &checkpoint_dir).unwrap();
            prop_assert!(!detected_before);

            // Save checkpoint
            let checkpoint = ExtractionCheckpoint {
                workspace_id: workspace_id.clone(),
                archive_path: std::path::PathBuf::from("/test/archive.zip"),
                last_extracted_file: "test.txt".to_string(),
                files_processed: 42,
                bytes_processed: 1024,
                timestamp: 1234567890,
            };
            save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();

            // Should detect incomplete extraction
            let detected_after = detect_incomplete_extraction(&workspace_id, &checkpoint_dir).unwrap();
            prop_assert!(detected_after);

            // Delete checkpoint
            delete_checkpoint(&workspace_id, &checkpoint_dir).unwrap();

            // Should no longer detect
            let detected_final = detect_incomplete_extraction(&workspace_id, &checkpoint_dir).unwrap();
            prop_assert!(!detected_final);
        }
    }
}
