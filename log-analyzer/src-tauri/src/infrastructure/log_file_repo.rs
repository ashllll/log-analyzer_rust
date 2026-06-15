//! LogFileRepository adapter — wraps MetadataStore + CAS behind the trait.

use std::io::BufRead;
use std::sync::Arc;

use async_trait::async_trait;

use la_core::domain::LogFileRepository;
use la_core::error::Result;
use la_core::storage_types::FileMetadata;
use la_storage::{ContentAddressableStorage, MetadataStore};

use crate::utils::encoding::decode_log_content;

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

    fn read_line_chunks_sync(
        &self,
        hash: &str,
        chunk_size: usize,
        visitor: &mut dyn FnMut(Vec<String>, usize) -> Result<bool>,
    ) -> Result<()> {
        let object_path = self.cas.get_object_path(hash);
        let file = std::fs::File::open(&object_path).map_err(|e| {
            la_core::error::AppError::io_error(
                format!("Failed to open CAS content for hash {hash}: {e}"),
                Some(object_path.clone()),
            )
        })?;
        let mut reader = std::io::BufReader::with_capacity(256 * 1024, file);
        let mut line_bytes = Vec::with_capacity(1024);
        let mut lines = Vec::with_capacity(chunk_size);
        let mut chunk_start_line = 1usize;
        let mut next_line = 1usize;

        loop {
            line_bytes.clear();
            let bytes_read = reader.read_until(b'\n', &mut line_bytes).map_err(|e| {
                la_core::error::AppError::io_error(
                    format!("Failed to read CAS line for hash {hash}: {e}"),
                    Some(object_path.clone()),
                )
            })?;
            if bytes_read == 0 {
                break;
            }

            while matches!(line_bytes.last(), Some(b'\n' | b'\r')) {
                line_bytes.pop();
            }

            let (line, _) = decode_log_content(&line_bytes);
            lines.push(line);

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

    fn file_exists_sync(&self, hash: &str) -> bool {
        self.cas.exists(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn read_line_chunks_sync_streams_from_cas_object() {
        let temp = TempDir::new().unwrap();
        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());
        let hash = cas
            .store_content(b"one\ntwo\nthree\nfour\nfive\n")
            .await
            .unwrap();
        let repo = CasLogFileRepository { metadata, cas };

        let mut chunks = Vec::new();
        repo.read_line_chunks_sync(&hash, 2, &mut |lines, start_line| {
            chunks.push((start_line, lines));
            Ok(true)
        })
        .unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], (1, vec!["one".to_string(), "two".to_string()]));
        assert_eq!(
            chunks[1],
            (3, vec!["three".to_string(), "four".to_string()])
        );
        assert_eq!(chunks[2], (5, vec!["five".to_string()]));
    }
}
