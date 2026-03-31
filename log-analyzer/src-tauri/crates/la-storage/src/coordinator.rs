//! 存储协调器 - 原子操作与事务管理
//!
//! ## 竞态条件防护 (C-H4 修复)
//!
//! 使用数据库事务确保引用检查和文件删除的原子性。

use std::sync::Arc;
use crate::cas::ContentAddressableStorage;
use crate::metadata_store::{FileMetadata, MetadataStore};
use la_core::error::{AppError, Result};
use tracing::{info, warn};

pub struct StorageCoordinator {
    cas: Arc<ContentAddressableStorage>,
    metadata_store: Arc<MetadataStore>,
}

impl StorageCoordinator {
    pub fn new(cas: Arc<ContentAddressableStorage>, metadata_store: Arc<MetadataStore>) -> Self {
        Self { cas, metadata_store }
    }

    pub async fn store_file_atomic(
        &self,
        file_path: &std::path::Path,
        metadata: FileMetadata,
    ) -> Result<(String, i64)> {
        let hash = self.cas.store_file_streaming(file_path).await
            .map_err(|e| AppError::io_error(format!("CAS write failed: {}", e), Some(file_path.to_path_buf())))?;

        let mut metadata = metadata;
        metadata.sha256_hash = hash.clone();

        match self.metadata_store.insert_file(&metadata).await {
            Ok(file_id) => Ok((hash, file_id)),
            Err(e) => {
                self.cleanup_orphan_in_transaction(&hash).await;
                Err(AppError::database_error(format!("Metadata commit failed: {}", e)))
            }
        }
    }

    async fn cleanup_orphan_in_transaction(&self, hash: &str) {
        match self.try_delete_orphan(hash).await {
            Ok(true) => info!(hash = %hash, "Orphan deleted"),
            Ok(false) => info!(hash = %hash, "File has references, keeping"),
            Err(e) => warn!(hash = %hash, error = %e, "Cleanup failed"),
        }
    }

    async fn try_delete_orphan(&self, hash: &str) -> Result<bool> {
        let mut tx = self.metadata_store.begin_transaction().await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE sha256_hash = ?")
            .bind(hash).fetch_one(&mut *tx).await
            .map_err(|e| AppError::database_error(format!("Count failed: {}", e)))?;

        if count > 0 {
            tx.rollback().await.ok();
            return Ok(false);
        }

        let object_path = self.cas.get_object_path(hash);
        match tokio::fs::remove_file(&object_path).await {
            Ok(_) => { tx.commit().await.ok(); self.cas.invalidate_cache_entry(hash); Ok(true) }
            Err(e) => { tx.rollback().await.ok(); Err(AppError::io_error(format!("Remove failed: {}", e), Some(object_path))) }
        }
    }

    pub fn cas(&self) -> &Arc<ContentAddressableStorage> { &self.cas }
    pub fn metadata_store(&self) -> &Arc<MetadataStore> { &self.metadata_store }
}
