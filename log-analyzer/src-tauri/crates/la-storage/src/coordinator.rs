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
