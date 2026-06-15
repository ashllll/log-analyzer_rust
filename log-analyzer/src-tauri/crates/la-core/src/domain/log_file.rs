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

    /// Read line chunks by SHA-256 hash (synchronous — called from spawn_blocking).
    ///
    /// The default implementation falls back to `read_content_sync` so tests and
    /// non-CAS adapters keep working. Production CAS overrides this to avoid
    /// loading very large files into memory before the first search results can
    /// be flushed.
    fn read_line_chunks_sync(
        &self,
        hash: &str,
        chunk_size: usize,
        visitor: &mut dyn FnMut(Vec<String>, usize) -> Result<bool>,
    ) -> Result<()> {
        let content = self.read_content_sync(hash)?;
        let text = String::from_utf8_lossy(&content);
        let mut lines = Vec::with_capacity(chunk_size);
        let mut chunk_start_line = 1usize;
        let mut next_line = 1usize;

        for line in text.lines() {
            lines.push(line.to_string());
            if lines.len() >= chunk_size {
                if !visitor(std::mem::take(&mut lines), chunk_start_line)? {
                    return Ok(());
                }
                next_line += chunk_size;
                chunk_start_line = next_line;
            }
        }

        if !lines.is_empty() {
            visitor(lines, chunk_start_line)?;
        }

        Ok(())
    }

    /// Check if a file exists in storage.
    fn file_exists_sync(&self, hash: &str) -> bool;
}
