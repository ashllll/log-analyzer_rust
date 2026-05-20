//! LogFileRepository — access log file metadata and content.
//!
//! Abstracts CAS storage and metadata database behind a single facade.

use async_trait::async_trait;

use crate::error::Result;
use crate::storage_types::FileMetadata;

/// Repository for reading log file metadata and content.
#[async_trait]
pub trait LogFileRepository: Send + Sync {
    /// Get files with server-side filters (time range, level mask, file pattern).
    async fn get_files_with_filters(
        &self,
        workspace_id: &str,
        time_start: Option<i64>,
        time_end: Option<i64>,
        level_mask: Option<u8>,
        file_pattern: Option<&str>,
    ) -> Result<Vec<FileMetadata>>;

    /// Read raw file content by SHA-256 hash (synchronous — called from spawn_blocking).
    fn read_content_sync(&self, hash: &str) -> Result<Vec<u8>>;

    /// Check if a file exists in storage.
    fn file_exists_sync(&self, hash: &str) -> bool;
}
