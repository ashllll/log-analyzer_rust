//! Property-Based Tests for Resource Manager
//!
//! These tests validate the correctness properties of resource management
//! using property-based testing with proptest.

use super::resource_manager::{ManagedBuffer, ResourceManager};
use crate::services::MetadataDB;
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use tokio::fs;

/// Helper to create a test resource manager
async fn create_test_manager() -> (ResourceManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let metadata_db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());

    let temp_base = temp_dir.path().join("temp");
    fs::create_dir_all(&temp_base).await.unwrap();

    let manager = ResourceManager::new(metadata_db, temp_base, Some(Duration::from_secs(1)));

    (manager, temp_dir)
}

/// **Feature: enhanced-archive-handling, Property 38: Resource release timing**
///
/// *For any* extraction operation, all file handles and memory buffers should be
/// released within 5 seconds of completion.
///
/// **Validates: Requirements 8.5**
#[cfg(test)]
mod property_38_resource_release_timing {
    use super::*;

    proptest! {
        #[test]
        fn prop_resource_release_within_time_limit(
            handle_count in 1usize..100,
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Register multiple file handles
                for i in 0..handle_count {
                    let path = PathBuf::from(format!("/test/file_{}.txt", i));
                    manager.register_file_handle(path, workspace_id.clone()).await;
                }

                // Verify handles are registered
                prop_assert_eq!(manager.active_handle_count().await, handle_count);

                // Measure release time
                let start = SystemTime::now();
                let released = manager.release_workspace_handles(&workspace_id).await.unwrap();
                let elapsed = start.elapsed().unwrap();

                // Verify all handles were released
                prop_assert_eq!(released, handle_count);
                prop_assert_eq!(manager.active_handle_count().await, 0);

                // Verify release time is within 5 seconds
                prop_assert!(
                    elapsed < Duration::from_secs(5),
                    "Resource release took {:?}, exceeding 5 second limit",
                    elapsed
                );

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_workspace_cleanup_completes_within_time_limit(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            file_count in 1usize..50,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Register file handles
                for i in 0..file_count {
                    let path = PathBuf::from(format!("/test/file_{}.txt", i));
                    manager.register_file_handle(path, workspace_id.clone()).await;
                }

                // Create path mappings
                for i in 0..file_count {
                    manager.metadata_db
                        .store_mapping(
                            &workspace_id,
                            &format!("short_{}", i),
                            &format!("original_{}", i)
                        )
                        .await
                        .unwrap();
                }

                // Create temp files
                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);
                fs::create_dir_all(&workspace_temp).await.unwrap();
                for i in 0..file_count {
                    fs::write(workspace_temp.join(format!("temp_{}.txt", i)), b"test")
                        .await
                        .unwrap();
                }

                // Perform cleanup and measure time
                let start = SystemTime::now();
                let stats = manager.cleanup_workspace(&workspace_id).await.unwrap();
                let elapsed = start.elapsed().unwrap();

                // Verify cleanup completed
                prop_assert_eq!(stats.handles_released, file_count);
                prop_assert_eq!(stats.mappings_removed, file_count);
                prop_assert_eq!(stats.temp_files_removed, file_count);

                // Verify cleanup time is within 5 seconds
                prop_assert!(
                    elapsed < Duration::from_secs(5),
                    "Workspace cleanup took {:?}, exceeding 5 second limit",
                    elapsed
                );

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_managed_buffer_cleanup_is_immediate(
            buffer_size in 1024usize..1024*1024, // 1KB to 1MB
            buffer_name in "[a-zA-Z0-9_-]{5,20}",
        ) {
            // Create and drop buffer, measuring time
            let start = SystemTime::now();
            {
                let _buffer = ManagedBuffer::new(buffer_size, buffer_name);
                // Buffer will be dropped at end of scope
            }
            let elapsed = start.elapsed().unwrap();

            // Verify buffer cleanup is immediate (< 100ms)
            prop_assert!(
                elapsed < Duration::from_millis(100),
                "Buffer cleanup took {:?}, should be immediate",
                elapsed
            );
        }
    }

    proptest! {
        #[test]
        fn prop_concurrent_handle_release_is_fast(
            workspace_count in 2usize..10,
            handles_per_workspace in 5usize..20,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;
                let manager = Arc::new(manager);

                // Register handles for multiple workspaces
                for ws_idx in 0..workspace_count {
                    let workspace_id = format!("workspace_{}", ws_idx);
                    for handle_idx in 0..handles_per_workspace {
                        let path = PathBuf::from(format!("/test/ws{}_file{}.txt", ws_idx, handle_idx));
                        manager.register_file_handle(path, workspace_id.clone()).await;
                    }
                }

                // Verify total handles
                let total_handles = workspace_count * handles_per_workspace;
                prop_assert_eq!(manager.active_handle_count().await, total_handles);

                // Release all workspaces concurrently
                let start = SystemTime::now();
                let mut tasks = Vec::new();
                for ws_idx in 0..workspace_count {
                    let workspace_id = format!("workspace_{}", ws_idx);
                    let manager_clone = Arc::clone(&manager);
                    tasks.push(tokio::spawn(async move {
                        manager_clone.release_workspace_handles(&workspace_id).await
                    }));
                }

                // Wait for all releases
                for task in tasks {
                    task.await.unwrap().unwrap();
                }
                let elapsed = start.elapsed().unwrap();

                // Verify all handles released
                prop_assert_eq!(manager.active_handle_count().await, 0);

                // Verify concurrent release is fast (< 5 seconds)
                prop_assert!(
                    elapsed < Duration::from_secs(5),
                    "Concurrent handle release took {:?}, exceeding 5 second limit",
                    elapsed
                );

                Ok(())
            })?;
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 37: Temporary file cleanup**
///
/// *For any* extraction operation, all temporary files should be located in the
/// dedicated temp directory and cleaned up within the configured TTL (default 24 hours).
///
/// **Validates: Requirements 8.4**
#[cfg(test)]
mod property_37_temporary_file_cleanup {
    use super::*;
    use tokio::time::sleep;

    proptest! {
        #[test]
        fn prop_temp_files_cleaned_after_ttl(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            file_count in 1usize..20,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Create temp files
                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);
                fs::create_dir_all(&workspace_temp).await.unwrap();

                for i in 0..file_count {
                    let file_path = workspace_temp.join(format!("temp_{}.txt", i));
                    fs::write(&file_path, b"test data").await.unwrap();
                }

                // Verify files exist
                for i in 0..file_count {
                    let file_path = workspace_temp.join(format!("temp_{}.txt", i));
                    prop_assert!(file_path.exists());
                }

                // Files are new, should not be cleaned up yet
                let cleaned = manager.cleanup_temp_files(Some(&workspace_id)).await.unwrap();
                prop_assert_eq!(cleaned, 0);

                // Wait for TTL to expire (1 second in test config)
                sleep(Duration::from_secs(2)).await;

                // Now files should be cleaned up
                let cleaned = manager.cleanup_temp_files(Some(&workspace_id)).await.unwrap();
                prop_assert_eq!(cleaned, file_count);

                // Verify files are deleted
                for i in 0..file_count {
                    let file_path = workspace_temp.join(format!("temp_{}.txt", i));
                    prop_assert!(!file_path.exists());
                }

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_temp_files_in_dedicated_directory(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Get workspace temp directory
                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);

                // Verify it's under the base temp directory
                prop_assert!(workspace_temp.starts_with(&manager.temp_base_dir));

                // Verify it contains the workspace ID
                prop_assert!(workspace_temp.to_string_lossy().contains(&workspace_id));

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_nested_temp_files_cleaned_recursively(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            depth in 1usize..5,
            files_per_level in 1usize..5,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);
                let mut total_files = 0;

                // Create nested directory structure with files
                let mut current_dir = workspace_temp.clone();
                for level in 0..depth {
                    fs::create_dir_all(&current_dir).await.unwrap();

                    // Create files at this level
                    for i in 0..files_per_level {
                        let file_path = current_dir.join(format!("file_{}_{}.txt", level, i));
                        fs::write(&file_path, b"test").await.unwrap();
                        total_files += 1;
                    }

                    // Go deeper
                    current_dir = current_dir.join(format!("level_{}", level + 1));
                }

                // Wait for TTL
                sleep(Duration::from_secs(2)).await;

                // Cleanup should remove all files recursively
                let cleaned = manager.cleanup_temp_files(Some(&workspace_id)).await.unwrap();
                prop_assert_eq!(cleaned, total_files);

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_workspace_cleanup_removes_all_temp_files(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            file_count in 1usize..30,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Create temp files
                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);
                fs::create_dir_all(&workspace_temp).await.unwrap();

                for i in 0..file_count {
                    fs::write(workspace_temp.join(format!("temp_{}.txt", i)), b"test")
                        .await
                        .unwrap();
                }

                // Perform workspace cleanup (ignores TTL)
                let stats = manager.cleanup_workspace(&workspace_id).await.unwrap();

                // Verify all temp files removed
                prop_assert_eq!(stats.temp_files_removed, file_count);

                // Verify temp directory is deleted
                prop_assert!(!workspace_temp.exists());

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_cleanup_preserves_other_workspaces(
            workspace1_id in "[a-zA-Z0-9_-]{5,20}",
            workspace2_id in "[a-zA-Z0-9_-]{5,20}",
            file_count in 1usize..10,
        ) {
            // Ensure different workspace IDs
            prop_assume!(workspace1_id != workspace2_id);

            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Create temp files for both workspaces
                let ws1_temp = manager.get_workspace_temp_dir(&workspace1_id);
                let ws2_temp = manager.get_workspace_temp_dir(&workspace2_id);

                fs::create_dir_all(&ws1_temp).await.unwrap();
                fs::create_dir_all(&ws2_temp).await.unwrap();

                for i in 0..file_count {
                    fs::write(ws1_temp.join(format!("temp_{}.txt", i)), b"ws1")
                        .await
                        .unwrap();
                    fs::write(ws2_temp.join(format!("temp_{}.txt", i)), b"ws2")
                        .await
                        .unwrap();
                }

                // Cleanup workspace1
                let stats = manager.cleanup_workspace(&workspace1_id).await.unwrap();
                prop_assert_eq!(stats.temp_files_removed, file_count);

                // Verify workspace1 temp is deleted
                prop_assert!(!ws1_temp.exists());

                // Verify workspace2 temp is preserved
                prop_assert!(ws2_temp.exists());
                for i in 0..file_count {
                    let file_path = ws2_temp.join(format!("temp_{}.txt", i));
                    prop_assert!(file_path.exists());
                }

                Ok(())
            })?;
        }
    }
}

/// Additional property tests for resource management
#[cfg(test)]
mod additional_properties {
    use super::*;

    proptest! {
        #[test]
        fn prop_managed_buffer_size_matches_allocation(
            buffer_size in 1024usize..1024*1024,
            buffer_name in "[a-zA-Z0-9_-]{5,20}",
        ) {
            let buffer = ManagedBuffer::new(buffer_size, buffer_name);
            prop_assert_eq!(buffer.len(), buffer_size);
            prop_assert!(!buffer.is_empty());
        }
    }

    proptest! {
        #[test]
        fn prop_managed_buffer_is_writable(
            buffer_size in 1024usize..10240,
            buffer_name in "[a-zA-Z0-9_-]{5,20}",
            test_byte in 0u8..=255,
        ) {
            let mut buffer = ManagedBuffer::new(buffer_size, buffer_name);

            // Write to buffer
            buffer.as_mut_slice()[0] = test_byte;

            // Verify write
            prop_assert_eq!(buffer.as_slice()[0], test_byte);
        }
    }

    proptest! {
        #[test]
        fn prop_handle_registration_is_idempotent(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            file_path in "[a-zA-Z0-9_/-]{10,50}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                let path = PathBuf::from(file_path);

                // Register same handle multiple times
                manager.register_file_handle(path.clone(), workspace_id.clone()).await;
                manager.register_file_handle(path.clone(), workspace_id.clone()).await;
                manager.register_file_handle(path.clone(), workspace_id.clone()).await;

                // Each registration adds a new entry (not idempotent by design)
                // This is intentional to track multiple opens of the same file
                prop_assert_eq!(manager.active_handle_count().await, 3);

                Ok(())
            })?;
        }
    }

    proptest! {
        #[test]
        fn prop_cleanup_stats_are_accurate(
            workspace_id in "[a-zA-Z0-9_-]{5,20}",
            handle_count in 1usize..20,
            mapping_count in 1usize..20,
            file_count in 1usize..20,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let (manager, _temp_dir) = create_test_manager().await;

                // Register handles
                for i in 0..handle_count {
                    let path = PathBuf::from(format!("/test/file_{}.txt", i));
                    manager.register_file_handle(path, workspace_id.clone()).await;
                }

                // Create mappings
                for i in 0..mapping_count {
                    manager.metadata_db
                        .store_mapping(
                            &workspace_id,
                            &format!("short_{}", i),
                            &format!("original_{}", i)
                        )
                        .await
                        .unwrap();
                }

                // Create temp files
                let workspace_temp = manager.get_workspace_temp_dir(&workspace_id);
                fs::create_dir_all(&workspace_temp).await.unwrap();
                for i in 0..file_count {
                    fs::write(workspace_temp.join(format!("temp_{}.txt", i)), b"test")
                        .await
                        .unwrap();
                }

                // Perform cleanup
                let stats = manager.cleanup_workspace(&workspace_id).await.unwrap();

                // Verify stats accuracy
                prop_assert_eq!(stats.handles_released, handle_count);
                prop_assert_eq!(stats.mappings_removed, mapping_count);
                prop_assert_eq!(stats.temp_files_removed, file_count);
                prop_assert!(stats.cleanup_duration < Duration::from_secs(5));

                Ok(())
            })?;
        }
    }
}
