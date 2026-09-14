//! WatcherRunner — 文件监听事件循环（P7 提取自 WorkspaceServiceImpl）。
//!
//! 将 start_watch 中 ~230 行内联闭包提取为独立结构体，
//! 使 CAS 写入、索引更新、事件发射逻辑集中且可测试。
//! P7-续: FileTailer owns offset/line-count maps; WatcherState Arc removed.

use std::path::Path;
use std::sync::Arc;

use la_core::error::{AppError, Result};
use la_core::traits::{ContentStorage, MetadataStorage};
use tauri::Emitter;
use tokio::runtime::Handle as TokioHandle;
use tracing::warn;

use crate::application::watch::{WatchEvent, WatchEventKind};
use crate::infrastructure::file_tailer::FileTailer;

/// Coalesce bursts while guaranteeing a trailing notification, even if the
/// writer becomes idle. Zero-line changes (truncate) are also notifications.
#[derive(Default)]
struct PendingFileUpdates {
    deadline: Option<std::time::Instant>,
    new_lines: u64,
}

impl PendingFileUpdates {
    fn record(&mut self, lines: u64, now: std::time::Instant) {
        self.deadline
            .get_or_insert(now + std::time::Duration::from_secs(5));
        self.new_lines = self.new_lines.saturating_add(lines);
    }
    fn take_due(&mut self, now: std::time::Instant, force: bool) -> Option<u64> {
        let deadline = self.deadline?;
        if !force && now < deadline {
            return None;
        }
        self.deadline = None;
        Some(std::mem::take(&mut self.new_lines))
    }
}

async fn persist_file_snapshot(
    cas: &dyn ContentStorage,
    metadata: &dyn MetadataStorage,
    file_path: &str,
    virtual_path: &str,
) -> Result<()> {
    let content = tokio::fs::read(file_path).await.map_err(|error| {
        AppError::io_error(
            format!("Failed to read watcher file for CAS storage: {error}"),
            Some(file_path.into()),
        )
    })?;
    // Use the same statistics contract as imports and publish content + READY
    // metadata together. Pending watcher rows otherwise never become searchable
    // with a time/level filter.
    let (content, (min_timestamp, max_timestamp, level_mask)) =
        tokio::task::spawn_blocking(move || {
            let stats = la_core::utils::compute_file_stats(&content);
            (content, stats)
        })
        .await
        .map_err(|error| AppError::internal_error(format!("Watcher statistics failed: {error}")))?;
    let hash = cas.store(&content).await?;
    let file_name = Path::new(file_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    let file_metadata = la_core::storage_types::FileMetadata {
        id: 0,
        sha256_hash: hash,
        virtual_path: virtual_path.to_string(),
        original_name: file_name,
        size: content.len() as i64,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
        min_timestamp,
        max_timestamp,
        level_mask,
        analysis_status: la_core::storage_types::AnalysisStatus::Ready,
    };
    metadata.insert_file(&file_metadata).await?;
    Ok(())
}

/// 文件监听后台运行器。
///
/// 持有后台线程所需的全部共享状态，通过 channels 接收文件事件。
/// `FileTailer` owns offset and line-count maps — no Arc<Mutex<Option>> wrapper.
pub(crate) struct WatcherRunner {
    cas: Arc<la_storage::ContentAddressableStorage>,
    metadata: Arc<dyn MetadataStorage>,
    search_engine: Arc<la_search::SearchEngineManager>,
    /// Tracks per-file offsets and line counts.
    tailer: FileTailer,
    /// Whether the watcher should continue running.
    is_active: bool,
    workspace_id: String,
    runtime: TokioHandle,
    /// Watch 模式广播用 AppHandle（构造时由工厂注入）
    app_handle: tauri::AppHandle,
    /// FilesUpdated 广播 debounce：记录上次广播的时刻
    pending_updates: PendingFileUpdates,
}

impl WatcherRunner {
    pub(crate) fn new(
        cas: Arc<la_storage::ContentAddressableStorage>,
        metadata: Arc<dyn MetadataStorage>,
        search_engine: Arc<la_search::SearchEngineManager>,
        watched_path: std::path::PathBuf,
        workspace_id: String,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            cas,
            metadata,
            search_engine,
            tailer: FileTailer::new(watched_path),
            is_active: true,
            workspace_id,
            runtime: TokioHandle::current(),
            app_handle,
            pending_updates: PendingFileUpdates::default(),
        }
    }

    /// Signal the runner to stop after processing the current event.
    #[allow(dead_code)]
    pub(crate) fn stop(&mut self) {
        self.is_active = false;
    }

    /// 运行事件循环——阻塞当前线程，直到 watcher 被 stop 或出错。
    pub(crate) fn run(mut self, rx: crossbeam::channel::Receiver<WatchEvent>) {
        loop {
            let received = match self.pending_updates.deadline {
                Some(deadline) => {
                    rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                }
                None => rx
                    .recv()
                    .map_err(|_| crossbeam::channel::RecvTimeoutError::Disconnected),
            };
            match received {
                Ok(event) => self.handle_event(&event),
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    self.flush_files_updated(true);
                    break;
                }
            }
            self.flush_files_updated(false);
            if !self.is_active {
                break;
            }
        }
    }

    fn handle_event(&mut self, event: &WatchEvent) {
        let event_type = match event.kind {
            WatchEventKind::Create => "created",
            WatchEventKind::Modify => "modified",
            WatchEventKind::Remove => "deleted",
            WatchEventKind::Other => return,
        };

        for path in &event.paths {
            if path.to_str().is_none() {
                warn!(path = ?path, "Skipping path with non-UTF-8 chars");
                continue;
            }

            match event_type {
                "created" => self.on_create(path),
                "modified" => self.on_modify(path),
                _ => {}
            }
        }
    }

    fn on_create(&mut self, path: &Path) {
        self.tailer.on_create(path);
        self.on_modify(path);
    }

    fn on_modify(&mut self, path: &Path) {
        // Cover both persistence and index commit with the same lifecycle lease.
        let Ok(_activity) = self.runtime.block_on(self.cas.activity_guard()) else {
            return;
        };
        match self.tailer.tail(path) {
            Ok(result) => {
                if result.lines.is_empty() && !result.truncated {
                    return;
                }

                let start_line_number = self.tailer.line_count(path) + 1;
                let new_line_count = result.lines.len();
                let virtual_path = self.tailer.virtual_path(path);

                let new_entries = la_core::utils::parse_log_lines(
                    &result.lines,
                    &virtual_path,
                    &path.to_string_lossy(),
                    0,
                    start_line_number,
                );

                // Persist the complete snapshot before publishing the incremental
                // index update. The tail offset is committed only after both
                // operations succeed, so a later event can retry safely.
                if let Err(error) = self.store_to_cas(&path.to_string_lossy(), &virtual_path) {
                    self.tailer.rollback(path, &result);
                    warn!(
                        error = %error,
                        file = %path.display(),
                        workspace_id = %self.workspace_id,
                        "Failed to persist watcher file snapshot"
                    );
                    return;
                }

                if let Err(error) = self.update_search_index(
                    &new_entries,
                    &virtual_path,
                    result.truncated || self.tailer.line_count(path) == 0,
                ) {
                    self.tailer.rollback(path, &result);
                    warn!(
                        error = %error,
                        count = new_entries.len(),
                        workspace_id = %self.workspace_id,
                        "Failed to update search index for watcher event"
                    );
                    return;
                }

                // Update line count
                self.tailer.add_lines(path, new_line_count);

                // Watch 模式：广播 FilesUpdated（debounce 5 秒，累计行数）
                self.broadcast_files_updated(new_line_count);
            }
            Err(e) => {
                warn!(error = %e, file = %path.display(), "Failed to read file incrementally");
            }
        }
    }

    fn update_search_index(
        &self,
        entries: &[la_core::models::LogEntry],
        virtual_path: &str,
        replace: bool,
    ) -> Result<()> {
        let mut update = self
            .search_engine
            .begin_update()
            .map_err(|error| AppError::index_error(error.to_string()))?;
        if replace {
            update.remove_file(virtual_path);
        }
        update
            .add_documents(entries)
            .map_err(|error| AppError::index_error(error.to_string()))?;
        update
            .commit()
            .map_err(|error| AppError::index_error(error.to_string()))
    }

    fn store_to_cas(&self, file_path: &str, virtual_path: &str) -> Result<()> {
        self.runtime.block_on(async {
            persist_file_snapshot(
                self.cas.as_ref(),
                self.metadata.as_ref(),
                file_path,
                virtual_path,
            )
            .await
        })
    }

    /// Watch 模式广播 FilesUpdated 事件（带 5 秒 debounce）。
    ///
    /// 新内容已写入 Tantivy 索引后调用，告知前端有新日志到达，
    /// 触发 refreshWorkspaces 等静默刷新逻辑。不含日志负载（轻量信号）。
    fn broadcast_files_updated(&mut self, new_lines: usize) {
        self.pending_updates
            .record(new_lines as u64, std::time::Instant::now());
    }

    fn flush_files_updated(&mut self, force: bool) {
        let Some(new_lines) = self
            .pending_updates
            .take_due(std::time::Instant::now(), force)
        else {
            return;
        };
        let event = crate::state_sync::models::WorkspaceEvent::FilesUpdated {
            workspace_id: self.workspace_id.clone(),
            new_lines,
        };
        let app = self.app_handle.clone();
        // 非阻塞：在异步运行时中发射，不阻塞监听事件循环
        self.runtime.spawn(async move {
            // 简单重试（最多 3 次，10ms 间隔）—— 与 TauriEventPublisher 对齐
            for attempt in 0..3 {
                if app.emit("workspace-event", &event).is_ok() {
                    return;
                }
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
            warn!("Failed to emit FilesUpdated after 3 attempts");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use la_core::error::AppError;
    use la_core::storage_types::FileMetadata;

    #[tokio::test]
    async fn watcher_snapshots_are_ready_and_filters_follow_new_content() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let cas = la_storage::ContentAddressableStorage::new(workspace.clone());
        let metadata = la_storage::MetadataStore::new(&workspace).await.unwrap();
        let source = temp.path().join("live.log");
        for text in [
            "2026-01-01 10:00:00 INFO first\n",
            "2026-01-02 12:00:00 ERROR second\n",
        ] {
            std::fs::write(&source, text).unwrap();
            persist_file_snapshot(&cas, &metadata, source.to_str().unwrap(), "live.log")
                .await
                .unwrap();
            let (min, max, mask) = la_core::utils::compute_file_stats(text.as_bytes());
            assert!(min.is_some());
            assert!(mask.is_some());
            let files = metadata
                .get_files_with_pruning(min, max, mask, None)
                .await
                .unwrap();
            assert_eq!(
                files.len(),
                1,
                "new snapshot must be searchable with filters"
            );
            assert_eq!(
                files[0].analysis_status,
                la_core::storage_types::AnalysisStatus::Ready
            );
            assert_eq!(files[0].min_timestamp, min);
            assert_eq!(files[0].level_mask, mask);
        }
        // Replacement without a timestamp must not inherit yesterday's range.
        std::fs::write(&source, "ERROR no timestamp\n").unwrap();
        persist_file_snapshot(&cas, &metadata, source.to_str().unwrap(), "live.log")
            .await
            .unwrap();
        let file = metadata
            .get_file_by_virtual_path("live.log")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(file.min_timestamp, None);
        assert_eq!(file.max_timestamp, None);
    }

    #[tokio::test]
    async fn watched_updates_replace_import_identity_and_empty_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("source");
        std::fs::create_dir(&source_dir).unwrap();
        let path = source_dir.join("app.log");
        std::fs::write(&path, "old ERROR\n").unwrap();
        let workspace = temp.path().join("workspace");
        let cas = la_storage::ContentAddressableStorage::new(workspace.clone());
        let metadata = la_storage::MetadataStore::new(&workspace).await.unwrap();
        // Initial import and subsequent watcher must address the same row.
        persist_file_snapshot(&cas, &metadata, path.to_str().unwrap(), "source/app.log")
            .await
            .unwrap();
        let mut tailer = FileTailer::new(source_dir);
        tailer.tail(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        let result = tailer.tail(&path).unwrap();
        assert!(result.truncated);
        let identity = tailer.virtual_path(&path);
        persist_file_snapshot(&cas, &metadata, path.to_str().unwrap(), &identity)
            .await
            .unwrap();
        let files = metadata.get_all_files().await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].virtual_path, "source/app.log");
        assert_eq!(files[0].size, 0);
        assert_eq!(files[0].level_mask, None);
        assert!(cas
            .read_content(&files[0].sha256_hash)
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn idle_writer_still_delivers_the_coalesced_trailing_update() {
        let mut pending = PendingFileUpdates::default();
        let now = std::time::Instant::now();
        pending.record(2, now);
        pending.record(3, now + std::time::Duration::from_secs(1));
        assert_eq!(pending.take_due(now, false), None);
        assert_eq!(
            pending.take_due(now + std::time::Duration::from_secs(5), false),
            Some(5)
        );
        assert_eq!(
            pending.take_due(now + std::time::Duration::from_secs(10), false),
            None
        );
        pending.record(0, now);
        assert_eq!(pending.take_due(now, true), Some(0));
    }

    struct SuccessfulContentStorage;

    #[async_trait]
    impl ContentStorage for SuccessfulContentStorage {
        async fn store(&self, _content: &[u8]) -> Result<String> {
            Ok("stored-hash".to_string())
        }

        async fn retrieve(&self, _hash: &str) -> Result<Vec<u8>> {
            Ok(Vec::new())
        }

        async fn exists(&self, _hash: &str) -> bool {
            true
        }
    }

    struct FailingMetadataStorage;

    #[async_trait]
    impl MetadataStorage for FailingMetadataStorage {
        async fn insert_file(&self, _metadata: &FileMetadata) -> Result<i64> {
            Err(AppError::database_error(
                "injected watcher metadata failure",
            ))
        }

        async fn get_all_files(&self) -> Result<Vec<FileMetadata>> {
            Ok(Vec::new())
        }

        async fn get_file_by_hash(&self, _hash: &str) -> Result<Option<FileMetadata>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn watcher_snapshot_reports_file_read_failure() {
        let parent = tempfile::TempDir::new().unwrap();
        let missing_file = parent.path().join("missing.log");

        let result = persist_file_snapshot(
            &SuccessfulContentStorage,
            &FailingMetadataStorage,
            missing_file.to_str().unwrap(),
            "missing.log",
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn watcher_snapshot_reports_metadata_failure() {
        let parent = tempfile::TempDir::new().unwrap();
        let file = parent.path().join("app.log");
        std::fs::write(&file, b"one line\n").unwrap();

        let result = persist_file_snapshot(
            &SuccessfulContentStorage,
            &FailingMetadataStorage,
            file.to_str().unwrap(),
            "app.log",
        )
        .await;

        assert!(result.is_err());
    }
}
