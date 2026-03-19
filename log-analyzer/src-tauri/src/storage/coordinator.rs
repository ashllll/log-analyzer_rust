//! Storage Coordinator Module
//!
//! Implements Saga compensation transaction pattern to ensure atomicity
//! between ContentAddressableStorage (CAS) and MetadataStore operations.
//!
//! ## Problem
//!
//! CAS (filesystem) and MetadataStore (SQLite) are two separate storage engines
//! with no atomic guarantee for cross-engine operations. A failure between
//! the two writes can lead to data inconsistency:
//! - Metadata exists but file doesn't (orphaned metadata)
//! - File exists but metadata doesn't (orphaned file)
//!
//! ## Solution: Saga Pattern
//!
//! The coordinator implements a Saga-like pattern where:
//! 1. Start with MetadataStore transaction (BEGIN)
//! 2. Insert metadata record (but don't commit yet)
//! 3. Write to CAS (filesystem)
//! 4. If CAS succeeds, commit the metadata transaction
//! 5. If CAS fails, rollback the metadata transaction (compensation)
//!
//! This ensures that metadata is only persisted when the file is successfully
//! stored, maintaining consistency between both storage engines.
//!
//! ## Usage
//!
//! ```ignore
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # use log_analyzer::storage::{ContentAddressableStorage, MetadataStore};
//! # use std::sync::Arc;
//! # use log_analyzer::storage::coordinator::StorageCoordinator;
//! # let cas = Arc::new(ContentAddressableStorage::new("/tmp/cas").await?);
//! # let metadata_store = Arc::new(MetadataStore::new("/tmp/meta").await?);
//! let coordinator = StorageCoordinator::new(cas, metadata_store);
//! # let path = std::path::Path::new("/tmp/test.log");
//! # let metadata = log_analyzer::storage::FileMetadata::default();
//! let (hash, file_id) = coordinator.store_file_atomic(path, metadata).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Storage coordinator implementing Saga compensation pattern
///
/// Provides atomic operations across CAS and MetadataStore by using
/// transaction coordination and compensation on failure.
pub struct StorageCoordinator {
    cas: Arc<ContentAddressableStorage>,
    metadata_store: Arc<MetadataStore>,
}

impl StorageCoordinator {
    /// Create a new storage coordinator
    ///
    /// # Arguments
    ///
    /// * `cas` - Content-addressable storage instance
    /// * `metadata_store` - Metadata store instance
    pub fn new(cas: Arc<ContentAddressableStorage>, metadata_store: Arc<MetadataStore>) -> Self {
        Self {
            cas,
            metadata_store,
        }
    }

    /// Atomically store a file with its metadata
    ///
    /// This method ensures consistency between CAS and MetadataStore by:
    /// 1. Writing file to CAS first (to get the hash)
    /// 2. Starting a metadata transaction
    /// 3. Inserting metadata record with the correct hash
    /// 4. Committing the transaction
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to store
    /// * `file_metadata` - Metadata for the file
    ///
    /// # Returns
    ///
    /// Returns a tuple of (sha256_hash, file_id) on success
    ///
    /// # Errors
    ///
    /// Returns an error if either CAS write or metadata insert fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let coordinator = StorageCoordinator::new(cas, metadata_store);
    /// let metadata = FileMetadata {
    ///     id: 0, // auto-generated
    ///     sha256_hash: "...".to_string(),
    ///     virtual_path: "/logs/app.log".to_string(),
    ///     original_name: "app.log".to_string(),
    ///     size: 1024,
    ///     modified_time: 1234567890,
    ///     mime_type: Some("text/plain".to_string()),
    ///     parent_archive_id: None,
    ///     depth_level: 0,
    /// };
    /// let (hash, file_id) = coordinator.store_file_atomic(path, metadata).await?;
    /// ```
    pub async fn store_file_atomic(
        &self,
        path: &Path,
        mut file_metadata: FileMetadata,
    ) -> Result<(String, i64)> {
        info!(
            path = %path.display(),
            virtual_path = %file_metadata.virtual_path,
            "Starting atomic file storage"
        );

        // Step 1: Write to CAS (filesystem) first to get the hash
        // This ensures we have the correct content hash before storing metadata
        debug!("Writing file to CAS");
        let hash = match self.cas.store_file_streaming(path).await {
            Ok(h) => {
                debug!(hash = %h, "File stored to CAS successfully");
                h
            }
            Err(e) => {
                error!(error = %e, "CAS write failed");
                return Err(e);
            }
        };

        // Step 2: Update metadata with the correct hash
        file_metadata.sha256_hash = hash.clone();

        // Step 3: Begin MetadataStore transaction
        debug!("Beginning metadata transaction");
        let mut tx = self.metadata_store.begin_transaction().await?;

        // Step 4: Insert metadata record within transaction
        debug!("Inserting metadata record in transaction");
        let file_id = match MetadataStore::insert_file_tx(&mut tx, &file_metadata).await {
            Ok(id) => {
                debug!(file_id = id, "Metadata record inserted successfully");
                id
            }
            Err(e) => {
                error!(error = %e, "Failed to insert metadata record, rolling back");
                // Attempt rollback (best effort)
                if let Err(rollback_err) = tx.rollback().await {
                    warn!(error = %rollback_err, "Rollback after metadata insert failure failed");
                }
                return Err(e);
            }
        };

        // Step 5: Commit the metadata transaction
        debug!("Committing metadata transaction");
        match tx.commit().await {
            Ok(_) => {
                info!(
                    hash = %hash,
                    file_id = file_id,
                    "Atomic file storage completed successfully"
                );
                Ok((hash, file_id))
            }
            Err(e) => {
                // CAS 写入成功但元数据事务提交失败，文件内容已落盘但无元数据记录
                // 记录 warn 级别日志并包含 cas_hash，供后续 GC 任务扫描孤儿文件
                warn!(
                    cas_hash = %hash,
                    file_id = file_id,
                    error = %e,
                    "CAS 写入成功但元数据提交失败，可能产生孤儿文件，hash 已记录供 GC 使用"
                );
                Err(crate::error::AppError::database_error(format!(
                    "Metadata commit failed after CAS write. Hash: {}, File ID: {}. Error: {}",
                    hash, file_id, e
                )))
            }
        }
    }

    /// Get reference to CAS
    pub fn cas(&self) -> &Arc<ContentAddressableStorage> {
        &self.cas
    }

    /// Get reference to MetadataStore
    pub fn metadata_store(&self) -> &Arc<MetadataStore> {
        &self.metadata_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_coordinator() -> (StorageCoordinator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("workspace");

        let cas = Arc::new(ContentAddressableStorage::new(workspace_path.clone()));
        let metadata_store = Arc::new(MetadataStore::new(&workspace_path).await.unwrap());

        let coordinator = StorageCoordinator::new(cas, metadata_store);
        (coordinator, temp_dir)
    }

    fn create_test_metadata(virtual_path: &str) -> FileMetadata {
        FileMetadata {
            id: 0,
            sha256_hash: String::new(), // Will be filled by CAS
            virtual_path: virtual_path.to_string(),
            original_name: "test.log".to_string(),
            size: 100,
            modified_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        }
    }

    #[tokio::test]
    async fn test_store_file_atomic_success() {
        let (coordinator, temp_dir) = create_test_coordinator().await;

        // Create test file
        let test_file = temp_dir.path().join("test.log");
        fs::write(&test_file, b"test content for atomic storage")
            .await
            .unwrap();

        let metadata = create_test_metadata("/test/file.log");

        // Execute atomic storage
        let result = coordinator.store_file_atomic(&test_file, metadata).await;
        assert!(result.is_ok(), "Atomic storage should succeed");

        let (hash, file_id) = result.unwrap();
        assert!(!hash.is_empty(), "Hash should not be empty");
        assert!(file_id > 0, "File ID should be positive");

        // Verify file exists in CAS
        assert!(coordinator.cas.exists(&hash), "File should exist in CAS");

        // Verify metadata exists
        let retrieved = coordinator
            .metadata_store
            .get_file_by_hash(&hash)
            .await
            .unwrap();
        assert!(retrieved.is_some(), "Metadata should exist");
    }

    #[tokio::test]
    async fn test_store_file_atomic_cas_failure_rollback() {
        let (coordinator, temp_dir) = create_test_coordinator().await;

        // Use a non-existent file path to trigger CAS failure
        let non_existent = temp_dir.path().join("does_not_exist.log");

        let metadata = create_test_metadata("/test/rollback_test.log");

        // This should fail due to missing file
        let result = coordinator.store_file_atomic(&non_existent, metadata).await;
        assert!(result.is_err(), "Should fail for non-existent file");

        // Verify no orphaned metadata exists
        let all_files = coordinator.metadata_store.get_all_files().await.unwrap();
        assert!(
            all_files.is_empty(),
            "No metadata should exist after rollback"
        );
    }

    #[tokio::test]
    async fn test_store_file_atomic_deduplication() {
        let (coordinator, temp_dir) = create_test_coordinator().await;

        // Create test file
        let test_file = temp_dir.path().join("duplicate.log");
        fs::write(&test_file, b"duplicate content").await.unwrap();

        let metadata1 = create_test_metadata("/test/file1.log");
        let metadata2 = create_test_metadata("/test/file2.log");

        // Store first file
        let (hash1, id1) = coordinator
            .store_file_atomic(&test_file, metadata1)
            .await
            .unwrap();

        // Store same file again with different metadata
        let (hash2, id2) = coordinator
            .store_file_atomic(&test_file, metadata2)
            .await
            .unwrap();

        // Same content should produce same hash (deduplication)
        assert_eq!(hash1, hash2, "Same content should have same hash");

        // Same content should return the same file_id (CAS deduplication)
        // The database has UNIQUE constraint on sha256_hash, so same content
        // shares the same metadata entry
        assert_eq!(
            id1, id2,
            "Same content should share the same file_id (CAS deduplication)"
        );
    }
}
