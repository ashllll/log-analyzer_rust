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
        min_timestamp: None,
        max_timestamp: None,
        level_mask: None,
        analysis_status: la_core::storage_types::AnalysisStatus::Pending,
    };
    metadata.insert_file(&file_metadata).await?;
    Ok(())
}

/// 文件监听后台运行器。
///
/// 持有后台线程所需的全部共享状态，通过 channels 接收文件事件。
/// `FileTailer` owns offset and line-count maps — no Arc<Mutex<Option>> wrapper.
pub(crate) struct WatcherRunner {
    cas: Arc<dyn ContentStorage>,
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
    last_broadcast: std::time::Instant,
}

impl WatcherRunner {
    pub(crate) fn new(
        cas: Arc<dyn ContentStorage>,
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
            last_broadcast: std::time::Instant::now(),
        }
    }

    /// Signal the runner to stop after processing the current event.
    #[allow(dead_code)]
    pub(crate) fn stop(&mut self) {
        self.is_active = false;
    }

    /// 运行事件循环——阻塞当前线程，直到 watcher 被 stop 或出错。
    pub(crate) fn run(mut self, rx: crossbeam::channel::Receiver<WatchEvent>) {
        for event in rx {
            self.handle_event(&event);

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
        match self.tailer.tail(path) {
            Ok(result) => {
                if result.lines.is_empty() {
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

                if let Err(error) = self.update_search_index(&new_entries) {
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

    fn update_search_index(&self, entries: &[la_core::models::LogEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut update = self
            .search_engine
            .begin_update()
            .map_err(|error| AppError::index_error(error.to_string()))?;
        update
            .add_documents(entries)
            .map_err(|error| AppError::index_error(error.to_string()))?;
        update
            .commit()
            .map_err(|error| AppError::index_error(error.to_string()))
    }

    fn store_to_cas(&self, file_path: &str, virtual_path: &str) -> Result<()> {
        self.runtime.block_on(persist_file_snapshot(
            self.cas.as_ref(),
            self.metadata.as_ref(),
            file_path,
            virtual_path,
        ))
    }

    /// Watch 模式广播 FilesUpdated 事件（带 5 秒 debounce）。
    ///
    /// 新内容已写入 Tantivy 索引后调用，告知前端有新日志到达，
    /// 触发 refreshWorkspaces 等静默刷新逻辑。不含日志负载（轻量信号）。
    fn broadcast_files_updated(&mut self, new_lines: usize) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_broadcast).as_secs() < 5 {
            return;
        }
        self.last_broadcast = now;

        let event = crate::state_sync::models::WorkspaceEvent::FilesUpdated {
            workspace_id: self.workspace_id.clone(),
            new_lines: new_lines as u64,
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
