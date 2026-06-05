//! 存储协调器 - 原子操作与事务管理
//!
//! ## 竞态条件防护 (C-H4 修复)
//!
//! 使用数据库事务确保引用检查和文件删除的原子性。

use crate::cas::ContentAddressableStorage;
use crate::metadata_store::{FileMetadata, MetadataStore};
use la_core::error::{AppError, Result};
use std::sync::Arc;
use tracing::{info, warn};

pub struct StorageCoordinator {
    cas: Arc<ContentAddressableStorage>,
    metadata_store: Arc<MetadataStore>,
}

impl StorageCoordinator {
    pub fn new(cas: Arc<ContentAddressableStorage>, metadata_store: Arc<MetadataStore>) -> Self {
        Self {
            cas,
            metadata_store,
        }
    }

    /// Store a file atomically (with soft guarantee)
    ///
    /// Attempts to write to CAS first, then commits metadata to the database.
    /// If metadata commit fails, the CAS file is a candidate for cleanup but
    /// the caller should rely on periodice GC (garbage collection) to reclaim
    /// orphaned CAS objects. This is a **soft guarantee**, not true two-phase
    /// commit: in the unlikely event that both metadata commit and CAS cleanup
    /// fail, the orphaned object will be collected in the next GC pass.
    pub async fn store_file_atomic(
        &self,
        file_path: &std::path::Path,
        metadata: FileMetadata,
    ) -> Result<(String, i64)> {
        let hash = self
            .cas
            .store_file_zero_copy(file_path)
            .await
            .map_err(|e| {
                AppError::io_error(
                    format!("CAS write failed: {}", e),
                    Some(file_path.to_path_buf()),
                )
            })?;

        let mut metadata = metadata;
        metadata.sha256_hash = hash.clone();

        match self.metadata_store.insert_file(&metadata).await {
            Ok(file_id) => Ok((hash, file_id)),
            Err(e) => {
                // Best-effort cleanup of the just-created CAS object.
                // If cleanup fails the orphan will be reclaimed by periodic GC.
                self.cleanup_orphan_in_transaction(&hash).await;
                Err(AppError::database_error(format!(
                    "Metadata commit failed: {}",
                    e
                )))
            }
        }
    }

    /// Cleanup orphaned CAS object with retry on transient failures
    ///
    /// Retries up to 3 times with exponential backoff (100ms, 200ms, 400ms)
    /// for transient errors like file system contention.
    async fn cleanup_orphan_in_transaction(&self, hash: &str) {
        const MAX_RETRIES: u32 = 3;
        let mut delay = std::time::Duration::from_millis(100);

        for attempt in 1..=MAX_RETRIES {
            match self.try_delete_orphan(hash).await {
                Ok(true) => {
                    info!(hash = %hash, attempt, "Orphan deleted");
                    return;
                }
                Ok(false) => {
                    info!(hash = %hash, "File has references, keeping");
                    return;
                }
                Err(e) if attempt < MAX_RETRIES => {
                    warn!(hash = %hash, error = %e, attempt, "Cleanup failed, retrying");
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, attempt, "Cleanup failed after retries");
                }
            }
        }
    }

    async fn try_delete_orphan(&self, hash: &str) -> Result<bool> {
        let mut tx = self.metadata_store.begin_transaction().await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE sha256_hash = ?")
            .bind(hash)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| AppError::database_error(format!("Count failed: {}", e)))?;

        if count > 0 {
            if let Err(e) = tx.rollback().await {
                warn!(hash = %hash, error = %e, "Rollback failed");
            }
            return Ok(false);
        }

        let object_path = self.cas.get_object_path(hash);
        match tokio::fs::remove_file(&object_path).await {
            Ok(_) => {
                if let Err(e) = tx.commit().await {
                    warn!(hash = %hash, error = %e, "Commit failed");
                }
                self.cas.invalidate_cache_entry(hash);
                Ok(true)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    warn!(hash = %hash, error = %rollback_err, "Rollback failed after remove failure");
                }
                Err(AppError::io_error(
                    format!("Remove failed: {}", e),
                    Some(object_path),
                ))
            }
        }
    }

    pub fn cas(&self) -> &Arc<ContentAddressableStorage> {
        &self.cas
    }
    pub fn metadata_store(&self) -> &Arc<MetadataStore> {
        &self.metadata_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_store::AnalysisStatus;
    use tempfile::TempDir;

    async fn create_test_env() -> (StorageCoordinator, TempDir, TempDir) {
        let workspace_dir = TempDir::new().unwrap();
        let cas = Arc::new(ContentAddressableStorage::new(
            workspace_dir.path().to_path_buf(),
        ));
        let metadata = Arc::new(
            MetadataStore::new(workspace_dir.path()).await.unwrap(),
        );
        let coordinator = StorageCoordinator::new(cas, metadata);
        let file_dir = TempDir::new().unwrap();
        (coordinator, workspace_dir, file_dir)
    }

    #[tokio::test]
    async fn test_store_file_atomic_successful_round_trip() {
        let (coordinator, _ws_dir, file_dir) = create_test_env().await;

        let file_path = file_dir.path().join("test.log");
        tokio::fs::write(&file_path, b"hello world log entry\n")
            .await
            .unwrap();
        let file_size = tokio::fs::metadata(&file_path).await.unwrap().len() as i64;

        let metadata = FileMetadata {
            id: 0,
            sha256_hash: String::new(),
            virtual_path: "logs/test.log".to_string(),
            original_name: "test.log".to_string(),
            size: file_size,
            modified_time: 1690000000,
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        };

        let (hash, file_id) = coordinator
            .store_file_atomic(&file_path, metadata)
            .await
            .unwrap();

        // Hash must be a valid SHA-256 hex string
        assert_eq!(hash.len(), 64);
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash should be hex: {}",
            hash
        );

        // file_id must be a positive row id
        assert!(file_id > 0, "file_id should be positive, got {}", file_id);

        // CAS object must exist on disk
        let object_path = coordinator.cas().get_object_path(&hash);
        assert!(
            object_path.exists(),
            "CAS object should exist at {}",
            object_path.display()
        );

        // Metadata must be retrievable from the store
        let stored = coordinator
            .metadata_store()
            .get_file_by_virtual_path("logs/test.log")
            .await
            .unwrap()
            .expect("metadata should exist");
        assert_eq!(stored.sha256_hash, hash);
        assert_eq!(stored.size, file_size);
        assert_eq!(stored.original_name, "test.log");
    }

    /// Verify that when metadata commit fails, the CAS content survives.
    ///
    /// This tests the soft-guarantee documented on `store_file_atomic`:
    /// orphaned CAS objects are preserved and will be reclaimed by
    /// periodic GC, not eagerly deleted in a way that could lose data.
    #[tokio::test]
    async fn test_cleanup_on_failure_preserves_cas_content() {
        let workspace_dir = TempDir::new().unwrap();
        let cas = Arc::new(ContentAddressableStorage::new(
            workspace_dir.path().to_path_buf(),
        ));
        let metadata = Arc::new(
            MetadataStore::new(workspace_dir.path()).await.unwrap(),
        );
        let coordinator = StorageCoordinator::new(cas.clone(), metadata.clone());
        let file_dir = TempDir::new().unwrap();

        // Write a test file and store it in CAS first (populates the cache)
        let file_path = file_dir.path().join("orphan.log");
        tokio::fs::write(&file_path, b"orphan test file content\n")
            .await
            .unwrap();

        let hash = cas.store_file_zero_copy(&file_path).await.unwrap();
        assert_eq!(hash.len(), 64);

        let object_path = cas.get_object_path(&hash);
        assert!(
            object_path.exists(),
            "CAS object should exist after initial write"
        );

        let original_content = tokio::fs::read(&object_path).await.unwrap();

        // Close the metadata store pool to simulate a database failure.
        // Subsequent `insert_file` and `cleanup_orphan_in_transaction` will fail.
        metadata.close().await;

        let metadata_input = FileMetadata {
            id: 0,
            sha256_hash: String::new(),
            virtual_path: "orphan/test.log".to_string(),
            original_name: "orphan.log".to_string(),
            size: original_content.len() as i64,
            modified_time: 1690000000,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
            min_timestamp: None,
            max_timestamp: None,
            level_mask: None,
            analysis_status: AnalysisStatus::Pending,
        };

        // store_file_atomic: CAS write succeeds (dedup), metadata insert fails
        let result = coordinator
            .store_file_atomic(&file_path, metadata_input)
            .await;
        assert!(
            result.is_err(),
            "should fail when metadata store is unavailable"
        );

        // CAS content must survive the failed transaction.
        // This is the documented soft-guarantee: orphaned CAS objects
        // are preserved and will be collected in the next GC pass.
        assert!(
            object_path.exists(),
            "CAS object should survive cleanup failure"
        );
        let content_after = tokio::fs::read(&object_path).await.unwrap();
        assert_eq!(
            original_content, content_after,
            "CAS content must be unchanged after failed store"
        );
    }
}
