//! LogFileRepository adapter — wraps MetadataStore + CAS behind the trait.

use std::io::{BufRead, Read};
use std::sync::Arc;

use async_trait::async_trait;

use la_core::domain::LogFileRepository;
use la_core::error::Result;
use la_core::storage_types::FileMetadata;
use la_storage::{ContentAddressableStorage, MetadataStore};

use crate::utils::encoding::decode_log_content;

/// Adapter that delegates to MetadataStore (for queries) and CAS (for content).
pub struct CasLogFileRepository {
    metadata: Arc<MetadataStore>,
    cas: Arc<ContentAddressableStorage>,
    // Owned by the repository, not the async search monitor: blocking workers
    // keep this alive even after a timeout has emitted its terminal event.
    _activity: tokio::sync::OwnedRwLockReadGuard<()>,
}

impl CasLogFileRepository {
    pub async fn new(
        metadata: Arc<MetadataStore>,
        cas: Arc<ContentAddressableStorage>,
    ) -> Result<Self> {
        let activity = cas.activity_guard().await?;
        Ok(Self {
            metadata,
            cas,
            _activity: activity,
        })
    }
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

        // P2#8 修复：单行读取上限。无换行的巨型“行”（单行 JSON、损坏
        // 文件）此前会让 read_until 无界累积直至整文件进内存。超限时
        // 截断该行（保留前 8MB 供搜索）并丢弃剩余字节直到换行。
        const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

        loop {
            line_bytes.clear();
            let mut limited = (&mut reader).take((MAX_LINE_BYTES + 1) as u64);
            let bytes_read = limited.read_until(b'\n', &mut line_bytes).map_err(|e| {
                la_core::error::AppError::io_error(
                    format!("Failed to read CAS line for hash {hash}: {e}"),
                    Some(object_path.clone()),
                )
            })?;
            if bytes_read == 0 {
                break;
            }

            let has_newline = line_bytes.last() == Some(&b'\n');
            if !has_newline && bytes_read == MAX_LINE_BYTES + 1 {
                // 超长行：丢弃剩余字节直到换行或 EOF，避免下一轮把
                // 行尾误当新行。
                discard_line_tail(&mut reader).map_err(|e| {
                    la_core::error::AppError::io_error(
                        format!("Failed to drain oversized CAS line for hash {hash}: {e}"),
                        Some(object_path.clone()),
                    )
                })?;
                line_bytes.truncate(MAX_LINE_BYTES);
                tracing::warn!(
                    hash = %hash,
                    max_bytes = MAX_LINE_BYTES,
                    "CAS line exceeded size limit, truncated for streaming search"
                );
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

/// Consume through the next newline using only the reader's fixed buffer.
fn discard_line_tail(reader: &mut impl BufRead) -> std::io::Result<()> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(());
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(buffer.len(), |index| index + 1);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn oversized_tail_uses_bounded_reader_and_preserves_next_line() {
        // Generate 40 MiB without allocating a 40 MiB input Vec.
        let input = std::io::repeat(b'x')
            .take(40 * 1024 * 1024)
            .chain(std::io::Cursor::new(b"\nafter\n"));
        let mut reader = std::io::BufReader::with_capacity(4096, input);
        discard_line_tail(&mut reader).unwrap();
        assert_eq!(reader.capacity(), 4096);
        let mut next = String::new();
        reader.read_line(&mut next).unwrap();
        assert_eq!(next, "after\n");
        // EOF without a newline must also terminate.
        let mut eof = std::io::BufReader::with_capacity(4096, std::io::repeat(b'x').take(100_000));
        discard_line_tail(&mut eof).unwrap();
        assert!(eof.fill_buf().unwrap().is_empty());
    }

    #[tokio::test]
    async fn gc_keeps_snapshot_alive_until_last_worker_reference_is_dropped() {
        let temp = TempDir::new().unwrap();
        let cas = Arc::new(ContentAddressableStorage::new(temp.path().to_path_buf()));
        let metadata = Arc::new(MetadataStore::new(temp.path()).await.unwrap());
        let hash = cas.store_content(b"old snapshot\n").await.unwrap();
        let monitor = Arc::new(
            CasLogFileRepository::new(metadata.clone(), cas.clone())
                .await
                .unwrap(),
        );
        let worker = monitor.clone();
        // Models timeout: the async owner goes away while the worker still
        // owns the old hash snapshot, which is no longer referenced in SQLite.
        drop(monitor);
        assert_eq!(
            la_storage::gc::gc_orphaned_objects(&cas, &metadata)
                .await
                .unwrap(),
            0
        );
        assert_eq!(worker.read_content_sync(&hash).unwrap(), b"old snapshot\n");
        drop(worker);
        assert_eq!(
            la_storage::gc::gc_orphaned_objects(&cas, &metadata)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn shutdown_waits_for_the_last_search_worker_and_rejects_stale_handles() {
        use futures::FutureExt;
        let temp = TempDir::new().unwrap();
        let cas = Arc::new(ContentAddressableStorage::new(temp.path().to_path_buf()));
        let metadata = Arc::new(MetadataStore::new(temp.path()).await.unwrap());
        let repo = Arc::new(
            CasLogFileRepository::new(metadata.clone(), cas.clone())
                .await
                .unwrap(),
        );
        let worker = repo.clone();
        cas.stop_accepting_activity();
        let drain = cas.drain_activity();
        tokio::pin!(drain);
        assert!(drain.as_mut().now_or_never().is_none());
        assert!(CasLogFileRepository::new(metadata, cas.clone())
            .await
            .is_err());
        drop(repo);
        assert!(drain.as_mut().now_or_never().is_none());
        drop(worker);
        let _exclusive = drain.await;
        assert!(cas.activity_guard().await.is_err());
    }

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
        let repo = CasLogFileRepository::new(metadata, cas).await.unwrap();

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
    // P2#8 regression: oversized lines must not load the whole file

    #[tokio::test]
    async fn oversized_single_line_is_truncated_and_following_lines_survive() {
        let temp = TempDir::new().unwrap();
        let workspace_dir = temp.path().join("workspace");
        let cas = Arc::new(ContentAddressableStorage::new(workspace_dir.clone()));
        let metadata = Arc::new(MetadataStore::new(&workspace_dir).await.unwrap());

        let mut content = vec![b'a'; 8 * 1024 * 1024 + 1];
        content.extend_from_slice(b"\nafter-one\nafter-two\n");
        let hash = cas.store_content(&content).await.unwrap();
        let repo = CasLogFileRepository::new(metadata, cas).await.unwrap();

        let mut received = Vec::new();
        repo.read_line_chunks_sync(&hash, 10, &mut |lines, _start| {
            received.extend(lines);
            Ok(true)
        })
        .unwrap();

        assert_eq!(received.len(), 3, "truncated line + two normal lines");
        assert_eq!(received[0].len(), 8 * 1024 * 1024);
        assert_eq!(received[1], "after-one");
        assert_eq!(received[2], "after-two");
    }
}
