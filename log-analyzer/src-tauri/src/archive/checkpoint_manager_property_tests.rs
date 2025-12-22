//! Property-based tests for checkpoint manager
//!
//! **Feature: enhanced-archive-handling, Property 23: Resumption checkpoint consistency**
//! **Validates: Requirements 5.4**
//!
//! For any extraction that is paused and resumed, the system should continue from
//! the last successfully extracted file without re-extracting previous files.

#[cfg(test)]
mod tests {
    use crate::archive::checkpoint_manager::{
        Checkpoint, CheckpointConfig, CheckpointManager, CheckpointMetrics,
    };
    use proptest::prelude::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Generate arbitrary checkpoint metrics
    fn arb_checkpoint_metrics() -> impl Strategy<Value = CheckpointMetrics> {
        (
            0usize..10000,       // files_extracted
            0u64..1_000_000_000, // bytes_extracted
            0usize..20,          // max_depth_reached
            0usize..100,         // error_count
            0usize..100,         // path_shortenings
        )
            .prop_map(
                |(files, bytes, depth, errors, shortenings)| CheckpointMetrics {
                    files_extracted: files,
                    bytes_extracted: bytes,
                    max_depth_reached: depth,
                    error_count: errors,
                    path_shortenings: shortenings,
                },
            )
    }

    /// Generate arbitrary file paths
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5).prop_map(|components| {
            let mut path = PathBuf::from("/test/output");
            for component in components {
                path.push(component);
            }
            path.with_extension("txt")
        })
    }

    /// Generate a list of UNIQUE file paths with sizes
    fn arb_file_list() -> impl Strategy<Value = Vec<(PathBuf, u64)>> {
        prop::collection::vec((arb_file_path(), 1u64..1_000_000), 1..100)
            .prop_map(|files| {
                // Deduplicate by path, keeping first occurrence
                let mut seen = std::collections::HashSet::new();
                files
                    .into_iter()
                    .filter(|(path, _)| seen.insert(path.clone()))
                    .collect::<Vec<(PathBuf, u64)>>()
            })
            .prop_filter(
                "Must have at least one file",
                |files: &Vec<(PathBuf, u64)>| !files.is_empty(),
            )
    }

    proptest! {
        /// **Property 23: Resumption checkpoint consistency**
        ///
        /// For any extraction that is paused and resumed, the checkpoint should:
        /// 1. Accurately track all extracted files
        /// 2. Correctly identify already-extracted files
        /// 3. Maintain consistent metrics across save/load cycles
        /// 4. Prevent duplicate extraction of files
        #[test]
        fn prop_checkpoint_resumption_consistency(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            archive_name in "[a-zA-Z0-9_-]{5,20}",
            file_list in arb_file_list(),
            pause_point in 0usize..100,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let manager = CheckpointManager::new(
                    CheckpointConfig::default(),
                    temp_dir.path().to_path_buf(),
                );

                let archive_path = PathBuf::from(format!("/test/{}.zip", archive_name));
                let target_dir = PathBuf::from("/test/output");

                // Create initial checkpoint
                let mut checkpoint = Checkpoint::new(
                    workspace_id.clone(),
                    archive_path.clone(),
                    target_dir.clone(),
                );

                // Simulate extraction up to pause point
                let pause_index = pause_point.min(file_list.len());
                let mut total_bytes = 0u64;

                for (file_path, file_size) in file_list.iter().take(pause_index) {
                    checkpoint.update_file(file_path.clone(), *file_size);
                    total_bytes += file_size;
                }

                // Save checkpoint (simulating pause)
                manager.save_checkpoint(&checkpoint).await.unwrap();

                // Load checkpoint (simulating resume)
                let loaded = manager
                    .load_checkpoint(&workspace_id, &archive_path)
                    .await
                    .unwrap()
                    .expect("Checkpoint should exist");

                // Verify checkpoint consistency
                prop_assert_eq!(&loaded.workspace_id, &checkpoint.workspace_id);
                prop_assert_eq!(&loaded.archive_path, &checkpoint.archive_path);
                prop_assert_eq!(&loaded.target_dir, &checkpoint.target_dir);
                prop_assert_eq!(
                    loaded.metrics.files_extracted,
                    checkpoint.metrics.files_extracted
                );
                prop_assert_eq!(
                    loaded.metrics.bytes_extracted,
                    checkpoint.metrics.bytes_extracted
                );
                prop_assert_eq!(loaded.extracted_files.len(), pause_index);

                // Verify all extracted files are tracked
                for (file_path, _) in file_list.iter().take(pause_index) {
                    prop_assert!(
                        loaded.is_file_extracted(file_path),
                        "File {:?} should be marked as extracted",
                        file_path
                    );
                }

                // Verify files after pause point are not tracked
                for (file_path, _) in file_list.iter().skip(pause_index) {
                    prop_assert!(
                        !loaded.is_file_extracted(file_path),
                        "File {:?} should not be marked as extracted",
                        file_path
                    );
                }

                // Verify metrics match
                prop_assert_eq!(loaded.metrics.bytes_extracted, total_bytes);
                prop_assert_eq!(loaded.metrics.files_extracted, pause_index);

                // Simulate resuming extraction
                let mut resumed_checkpoint = loaded;
                let mut resumed_bytes = total_bytes;

                for (file_path, file_size) in file_list.iter().skip(pause_index) {
                    // Should not re-extract already extracted files
                    if !resumed_checkpoint.is_file_extracted(file_path) {
                        resumed_checkpoint.update_file(file_path.clone(), *file_size);
                        resumed_bytes += file_size;
                    }
                }

                // Verify final state
                prop_assert_eq!(
                    resumed_checkpoint.metrics.files_extracted,
                    file_list.len()
                );
                prop_assert_eq!(
                    resumed_checkpoint.metrics.bytes_extracted,
                    file_list.iter().map(|(_, size)| size).sum::<u64>()
                );

                // All files should now be tracked
                for (file_path, _) in &file_list {
                    prop_assert!(
                        resumed_checkpoint.is_file_extracted(file_path),
                        "File {:?} should be marked as extracted after resume",
                        file_path
                    );
                }

                // Cleanup
                manager.delete_checkpoint(&workspace_id, &archive_path).await.unwrap();

                Ok(())
            })?
        }

        /// Property: Checkpoint metrics are preserved across save/load cycles
        #[test]
        fn prop_checkpoint_metrics_preservation(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            metrics in arb_checkpoint_metrics(),
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let manager = CheckpointManager::new(
                    CheckpointConfig::default(),
                    temp_dir.path().to_path_buf(),
                );

                let archive_path = PathBuf::from("/test/archive.zip");
                let target_dir = PathBuf::from("/test/output");

                // Create checkpoint with metrics
                let mut checkpoint = Checkpoint::new(
                    workspace_id.clone(),
                    archive_path.clone(),
                    target_dir.clone(),
                );
                checkpoint.update_metrics(&metrics);

                // Save and load
                manager.save_checkpoint(&checkpoint).await.unwrap();
                let loaded = manager
                    .load_checkpoint(&workspace_id, &archive_path)
                    .await
                    .unwrap()
                    .expect("Checkpoint should exist");

                // Verify metrics are preserved
                prop_assert_eq!(loaded.metrics, metrics);

                // Cleanup
                manager.delete_checkpoint(&workspace_id, &archive_path).await.unwrap();

                Ok(())
            })?
        }

        /// Property: Checkpoint prevents duplicate file extraction
        #[test]
        fn prop_checkpoint_prevents_duplicates(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            file_list in arb_file_list(),
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let manager = CheckpointManager::new(
                    CheckpointConfig::default(),
                    temp_dir.path().to_path_buf(),
                );

                let archive_path = PathBuf::from("/test/archive.zip");
                let target_dir = PathBuf::from("/test/output");

                // Create checkpoint and extract all files
                let mut checkpoint = Checkpoint::new(
                    workspace_id.clone(),
                    archive_path.clone(),
                    target_dir.clone(),
                );

                for (file_path, file_size) in &file_list {
                    checkpoint.update_file(file_path.clone(), *file_size);
                }

                // Save checkpoint
                manager.save_checkpoint(&checkpoint).await.unwrap();

                // Load checkpoint
                let loaded = manager
                    .load_checkpoint(&workspace_id, &archive_path)
                    .await
                    .unwrap()
                    .expect("Checkpoint should exist");

                // Attempt to "re-extract" files (should be skipped)
                let mut duplicate_count = 0;
                for (file_path, _) in &file_list {
                    if loaded.is_file_extracted(file_path) {
                        duplicate_count += 1;
                    }
                }

                // All files should be marked as already extracted
                prop_assert_eq!(duplicate_count, file_list.len());

                // Cleanup
                manager.delete_checkpoint(&workspace_id, &archive_path).await.unwrap();

                Ok(())
            })?
        }

        /// Property: Checkpoint write intervals are respected
        #[test]
        fn prop_checkpoint_write_intervals(
            file_count in 0usize..500,
            byte_count in 0u64..5_000_000_000,
        ) {
            let temp_dir = TempDir::new().unwrap();
            let config = CheckpointConfig {
                file_interval: 100,
                byte_interval: 1024 * 1024 * 1024, // 1GB
                enabled: true,
            };
            let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

            let should_write = manager.should_write_checkpoint(file_count, byte_count);

            // Should write if either threshold is met
            let expected = file_count >= 100 || byte_count >= 1024 * 1024 * 1024;
            prop_assert_eq!(should_write, expected);
        }

        /// Property: Disabled checkpoints don't write or load
        #[test]
        fn prop_checkpoint_disabled_behavior(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let config = CheckpointConfig {
                    enabled: false,
                    ..Default::default()
                };
                let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

                let archive_path = PathBuf::from("/test/archive.zip");
                let target_dir = PathBuf::from("/test/output");

                // Create checkpoint
                let checkpoint = Checkpoint::new(
                    workspace_id.clone(),
                    archive_path.clone(),
                    target_dir.clone(),
                );

                // Save should succeed but not write
                manager.save_checkpoint(&checkpoint).await.unwrap();

                // Load should return None
                let loaded = manager
                    .load_checkpoint(&workspace_id, &archive_path)
                    .await
                    .unwrap();
                prop_assert!(loaded.is_none());

                // should_write_checkpoint should always return false
                prop_assert!(!manager.should_write_checkpoint(1000, 10_000_000_000));

                Ok(())
            })?
        }
    }

    #[tokio::test]
    async fn test_checkpoint_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let manager =
            CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

        let archive_path = PathBuf::from("/test/archive.zip");
        let mut checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            archive_path.clone(),
            PathBuf::from("/test/output"),
        );

        // Corrupt the version
        checkpoint.version = 999;

        // Save corrupted checkpoint
        manager.save_checkpoint(&checkpoint).await.unwrap();

        // Load should return None due to version mismatch
        let loaded = manager
            .load_checkpoint("test_workspace", &archive_path)
            .await
            .unwrap();
        assert!(loaded.is_none());
    }
}
