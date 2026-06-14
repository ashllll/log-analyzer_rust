//! WatcherRunner — 文件监听事件循环（P7 提取自 WorkspaceServiceImpl）。
//!
//! 将 start_watch 中 ~230 行内联闭包提取为独立结构体，
//! 使 CAS 写入、索引更新、事件发射逻辑集中且可测试。

use std::path::Path;
use std::sync::Arc;

use la_core::domain::event::EventPublisher;
use la_core::traits::{ContentStorage, MetadataStorage};
use tokio::runtime::Handle as TokioHandle;
use tracing::warn;

use crate::application::watch::{WatchEvent, WatchEventKind};
use crate::services::file_watcher::{self, WatcherState};

/// 文件监听后台运行器。
///
/// 持有后台线程所需的全部共享状态，通过 channels 接收文件事件。
pub(crate) struct WatcherRunner {
    events: Arc<dyn EventPublisher>,
    cas: Arc<dyn ContentStorage>,
    metadata: Arc<dyn MetadataStorage>,
    search_engine: Arc<la_search::SearchEngineManager>,
    watcher_state: Arc<parking_lot::Mutex<Option<WatcherState>>>,
    workspace_id: String,
    runtime: TokioHandle,
}

impl WatcherRunner {
    pub(crate) fn new(
        events: Arc<dyn EventPublisher>,
        cas: Arc<dyn ContentStorage>,
        metadata: Arc<dyn MetadataStorage>,
        search_engine: Arc<la_search::SearchEngineManager>,
        watcher_state: Arc<parking_lot::Mutex<Option<WatcherState>>>,
        workspace_id: String,
    ) -> Self {
        Self {
            events,
            cas,
            metadata,
            search_engine,
            watcher_state,
            workspace_id,
            runtime: TokioHandle::current(),
        }
    }

    /// 运行事件循环——阻塞当前线程，直到 watcher 被 stop 或出错。
    pub(crate) fn run(
        self,
        rx: crossbeam::channel::Receiver<WatchEvent>,
    ) {
        for event in rx {
            self.handle_event(&event);

            // Check is_active to exit
            let is_active = {
                let guard = self.watcher_state.lock();
                guard.as_ref().map(|w| w.is_active).unwrap_or(false)
            };
            if !is_active {
                break;
            }
        }
    }

    fn handle_event(&self, event: &WatchEvent) {
        let event_type = match event.kind {
            WatchEventKind::Create => "created",
            WatchEventKind::Modify => "modified",
            WatchEventKind::Remove => "deleted",
            WatchEventKind::Other => return,
        };

        for path in &event.paths {
            let file_path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => {
                    warn!(path = ?path, "Skipping path with non-UTF-8 chars");
                    continue;
                }
            };

            let timestamp = chrono::Utc::now().timestamp();

            // Emit file-changed event
            self.emit_file_changed(event_type, &file_path_str, timestamp);

            match event_type {
                "created" => self.on_create(&file_path_str),
                "modified" => self.on_modify(&path, &file_path_str),
                _ => {}
            }
        }
    }

    fn emit_file_changed(&self, event_type: &str, file_path: &str, timestamp: i64) {
        let events = self.events.clone();
        let ws = self.workspace_id.clone();
        let et = event_type.to_string();
        let fp = file_path.to_string();
        self.runtime.spawn(async move {
            events.emit_file_changed(&ws, &et, &fp, timestamp).await;
        });
    }

    fn on_create(&self, file_path_str: &str) {
        let mut guard = self.watcher_state.lock();
        if let Some(ref mut w) = *guard {
            w.line_counts.insert(file_path_str.to_string(), 0);
        }
    }

    fn on_modify(&self, path: &Path, file_path_str: &str) {
        let (offset, start_line_number, watch_path_inner) = {
            let guard = self.watcher_state.lock();
            if let Some(ref w) = *guard {
                let offset = *w.file_offsets.get(file_path_str).unwrap_or(&0);
                let start_line = w
                    .line_counts
                    .get(file_path_str)
                    .map_or(1, |&count| count + 1);
                (offset, start_line, w.watched_path.clone())
            } else {
                return;
            }
        };

        match file_watcher::read_file_from_offset(path, offset) {
            Ok((new_lines, new_offset)) => {
                let new_line_count = new_lines.len();
                if new_lines.is_empty() {
                    return;
                }

                let virtual_path = path
                    .strip_prefix(&watch_path_inner)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let new_entries = file_watcher::parse_log_lines(
                    &new_lines,
                    &virtual_path,
                    file_path_str,
                    0,
                    start_line_number,
                );

                // Emit new-logs
                self.emit_new_logs(&new_entries);

                // Update Tantivy search index
                self.update_search_index(&new_entries);

                // Store in CAS + metadata
                self.store_to_cas(file_path_str, &virtual_path);

                // Update offsets
                self.update_offsets(file_path_str, new_offset, new_line_count, start_line_number);
            }
            Err(e) => {
                warn!(error = %e, file = %file_path_str, "Failed to read file incrementally");
            }
        }
    }

    fn emit_new_logs(&self, entries: &[la_core::models::LogEntry]) {
        let events = self.events.clone();
        let ws = self.workspace_id.clone();
        if let Ok(json) = serde_json::to_string(entries) {
            self.runtime.spawn(async move {
                events.emit_new_logs(&ws, &json).await;
            });
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

    fn update_offsets(
        &self,
        file_path: &str,
        new_offset: u64,
        new_line_count: usize,
        start_line_number: usize,
    ) {
        let mut guard = self.watcher_state.lock();
        if let Some(ref mut w) = *guard {
            w.file_offsets.insert(file_path.to_string(), new_offset);
            if new_line_count > 0 {
                w.line_counts.insert(
                    file_path.to_string(),
                    start_line_number - 1 + new_line_count,
                );
            }
        }
    }
}
