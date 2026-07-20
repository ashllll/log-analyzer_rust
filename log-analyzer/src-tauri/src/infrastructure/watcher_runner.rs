//! WatcherRunner — 文件监听事件循环（P7 提取自 WorkspaceServiceImpl）。
//!
//! 将 start_watch 中 ~230 行内联闭包提取为独立结构体，
//! 使 CAS 写入、索引更新、事件发射逻辑集中且可测试。
//! P7-续: FileTailer owns offset/line-count maps; WatcherState Arc removed.

use std::path::Path;
use std::sync::Arc;

use la_core::traits::{ContentStorage, MetadataStorage};
use tauri::Emitter;
use tokio::runtime::Handle as TokioHandle;
use tracing::warn;

use crate::application::watch::{WatchEvent, WatchEventKind};
use crate::infrastructure::file_tailer::FileTailer;

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
    }

    fn on_modify(&mut self, path: &Path) {
        let start_line_number = self.tailer.line_count(path) + 1;

        match self.tailer.tail(path) {
            Ok(result) => {
                if result.lines.is_empty() {
                    return;
                }

                let new_line_count = result.lines.len();
                let virtual_path = self.tailer.virtual_path(path);

                let new_entries = la_core::utils::parse_log_lines(
                    &result.lines,
                    &virtual_path,
                    &path.to_string_lossy(),
                    0,
                    start_line_number,
                );

                // 更新搜索索引与存储（前端通过 workspace-event 通道获知变更）
                self.update_search_index(&new_entries);

                // Store in CAS + metadata
                self.store_to_cas(&path.to_string_lossy(), &virtual_path);

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

    fn update_search_index(&self, entries: &[la_core::models::LogEntry]) {
        if entries.is_empty() {
            return;
        }
        if let Err(e) = self.search_engine.add_documents(entries) {
            warn!(
                error = %e, count = entries.len(),
                workspace_id = %self.workspace_id,
                "Failed to add watch documents to search index"
            );
        }
        if let Err(e) = self.search_engine.commit() {
            warn!(
                error = %e, workspace_id = %self.workspace_id,
                "Failed to commit search index after watch update"
            );
        }
    }

    fn store_to_cas(&self, file_path: &str, virtual_path: &str) {
        let cas = Arc::clone(&self.cas);
        let metadata = Arc::clone(&self.metadata);
        let fp = file_path.to_string();
        let vp = virtual_path.to_string();
        self.runtime.spawn(async move {
            match tokio::fs::read(&fp).await {
                Ok(content) => match cas.store(&content).await {
                    Ok(hash) => {
                        let file_name = Path::new(&fp)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| fp.clone());
                        let file_meta = la_core::storage_types::FileMetadata {
                            id: 0,
                            sha256_hash: hash,
                            virtual_path: vp,
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
                        if let Err(e) = metadata.insert_file(&file_meta).await {
                            warn!(error = %e, file = %fp, "Failed to insert watcher file metadata");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, file = %fp, "Failed to store watcher file content in CAS");
                    }
                },
                Err(e) => {
                    warn!(error = %e, file = %fp, "Failed to read watcher file for CAS storage");
                }
            }
        });
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
