//! LogFileRepository adapter — wraps MetadataStore + CAS behind the trait.

use std::sync::Arc;

use async_trait::async_trait;

use la_core::domain::LogFileRepository;
use la_core::error::Result;
use la_core::storage_types::FileMetadata;
use la_storage::{ContentAddressableStorage, MetadataStore};

/// Adapter that delegates to MetadataStore (for queries) and CAS (for content).
pub struct CasLogFileRepository {
    pub metadata: Arc<MetadataStore>,
    pub cas: Arc<ContentAddressableStorage>,
}

#[async_trait]
impl LogFileRepository for CasLogFileRepository {
    async fn get_files_with_filters(
        &self,
        workspace_id: &str,
        time_start: Option<i64>,
        time_end: Option<i64>,
        level_mask: Option<u8>,
        file_pattern: Option<&str>,
    ) -> Result<Vec<FileMetadata>> {
        // MetadataStore has its own error type; map it
        self.metadata
            .get_files_with_pruning(time_start, time_end, level_mask, file_pattern)
            .await
            .map_err(|e| {
                la_core::error::AppError::database_error(format!(
                    "Failed to get files for workspace {workspace_id}: {e}"
                ))
            })
    }

    fn read_content_sync(&self, hash: &str) -> Result<Vec<u8>> {
        self.cas.read_content_sync(hash).map_err(|e| {
            la_core::error::AppError::io_error(
                format!("Failed to read CAS content for hash {hash}: {e}"),
                None,
            )
        })
    }

    fn file_exists_sync(&self, hash: &str) -> bool {
        self.cas.exists(hash)
    }
}
