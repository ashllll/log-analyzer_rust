//! WatchUseCase — application-layer file watch orchestration.
//!
//! Encapsulates the file watching flow: start watching a workspace directory
//! for file changes and stop when no longer needed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use la_core::domain::event::EventPublisher;
use la_core::error::{AppError, Result};

use la_core::traits::{ContentStorage, MetadataStorage};
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tracing::{error, warn};

use crate::services::file_watcher::{self, WatcherState};

/// Result type for start-watch operations.
#[derive(Debug, Clone)]
pub struct WatchStartResult {
    pub workspace_id: String,
    pub watched_path: String,
}

/// Result type for stop-watch operations.
#[derive(Debug, Clone)]
pub struct WatchStopResult {
    pub workspace_id: String,
}

/// Application use case for workspace file watching.
///
/// Owns the watcher state map and orchestrates the full watch lifecycle:
/// - Creating and managing `notify` file system watchers
/// - Incremental file reading and log parsing
/// - CAS content storage and metadata updates
/// - Frontend event emission
pub struct WatchUseCase<E, C, M>
where
    E: EventPublisher + 'static,
    C: ContentStorage + 'static,
    M: MetadataStorage + 'static,
{
    events: Arc<E>,
    cas: Arc<C>,
    metadata: Arc<M>,
    watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
}

impl<E, C, M> WatchUseCase<E, C, M>
where
    E: EventPublisher,
    C: ContentStorage,
    M: MetadataStorage,
{
    /// Create a new WatchUseCase.
    ///
    /// The `watchers` map is shared with the caller (typically `AppState`)
    /// so watcher state persists across command invocations.
    pub fn new(
        events: Arc<E>,
        cas: Arc<C>,
        metadata: Arc<M>,
        watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    ) -> Self {
        Self {
            events,
            cas,
            metadata,
            watchers,
        }
    }

    /// Start watching a workspace directory for file changes.
    ///
    /// The watch flow:
    /// 1. Validate workspace and path
    /// 2. Create notify watcher with event channel
    /// 3. Spawn background thread to process events
    /// 4. Track file offsets and line counts for incremental reads
    /// 5. Store new log entries in CAS and update metadata
    /// 6. Emit file-changed and new-logs events to frontend
    ///
    /// Note: search index updates are handled by the Tauri command layer
    /// via the `emit_new_logs` event.
    pub async fn start(&self, workspace_id: &str, path: &str) -> Result<WatchStartResult> {
        // ── 1. Validate ──
        let watch_path = PathBuf::from(path);
        if !watch_path.exists() {
            return Err(AppError::validation_error(format!(
                "Path does not exist: {}",
                path
            )));
        }

        // ── 2. Check not already watching ──
        {
            let watchers = self.watchers.lock();
            if watchers.contains_key(workspace_id) {
                return Err(AppError::validation_error(
                    "Workspace is already being watched".to_string(),
                ));
            }
        }

        // ── 3. Create notify watcher ──
        let (tx, rx) = crossbeam::channel::unbounded::<std::result::Result<Event, notify::Error>>();

        let mut watcher = recommended_watcher(tx).map_err(|e| {
            AppError::io_error(format!("Failed to create file watcher: {}", e), None)
        })?;

        watcher.watch(&watch_path, RecursiveMode::Recursive).map_err(|e| {
            AppError::io_error(format!("Failed to start watching path: {}", e), None)
        })?;

        // ── 4. Create WatcherState ──
        let watcher_state = WatcherState {
            workspace_id: workspace_id.to_string(),
            watched_path: watch_path.clone(),
            file_offsets: HashMap::new(),
            line_counts: HashMap::new(),
            is_active: true,
            thread_handle: Arc::new(parking_lot::Mutex::new(None)),
            watcher: Arc::new(parking_lot::Mutex::new(Some(watcher))),
        };

        let thread_handle_arc = Arc::clone(&watcher_state.thread_handle);

        // ── 5. Insert into watchers map ──
        {
            let mut watchers = self.watchers.lock();
            watchers.insert(workspace_id.to_string(), watcher_state);
        }

        // ── 6. Clone Arcs for background thread ──
        let events = Arc::clone(&self.events);
        let cas = Arc::clone(&self.cas);
        let metadata = Arc::clone(&self.metadata);
        let watchers_arc = Arc::clone(&self.watchers);
        let workspace_id_clone = workspace_id.to_string();
        let _watch_path_clone = watch_path.clone();
        let runtime_handle = tokio::runtime::Handle::current();

        // ── 7. Spawn background thread ──
        let handle = thread::spawn(move || {
            for res in rx {
                match res {
                    Ok(event) => {
                        let event_type = match event.kind {
                            EventKind::Create(_) => "created",
                            EventKind::Modify(_) => "modified",
                            EventKind::Remove(_) => "deleted",
                            _ => continue,
                        };

                        for path in event.paths {
                            let file_path_str = match path.to_str() {
                                Some(s) => s.to_string(),
                                None => {
                                    warn!(path = ?path, "Skipping path with non-UTF-8 chars");
                                    continue;
                                }
                            };

                            let timestamp = chrono::Utc::now().timestamp();

                            // Emit file-changed event via domain trait
                            let events_clone = events.clone();
                            let ws = workspace_id_clone.clone();
                            let fp = file_path_str.clone();
                            let et = event_type.to_string();
                            runtime_handle.spawn(async move {
                                events_clone
                                    .emit_file_changed(&ws, &et, &fp, timestamp)
                                    .await;
                            });

                            // On Create: init line_counts
                            if event_type == "created" && path.is_file() {
                                let mut watchers = watchers_arc.lock();
                                if let Some(w) = watchers.get_mut(&workspace_id_clone) {
                                    w.line_counts.insert(file_path_str.clone(), 0);
                                }
                            }

                            // On Modify: read new content, parse, store
                            if event_type == "modified" && path.is_file() {
                                let (offset, start_line_number, watcher_watched_path) = {
                                    let watchers = watchers_arc.lock();
                                    if let Some(w) = watchers.get(&workspace_id_clone) {
                                        let offset = *w.file_offsets.get(&file_path_str).unwrap_or(&0);
                                        let start_line = if let Some(&count) =
                                            w.line_counts.get(&file_path_str)
                                        {
                                            count + 1
                                        } else if offset > 0 {
                                            1
                                        } else {
                                            1
                                        };
                                        (offset, start_line, w.watched_path.clone())
                                    } else {
                                        continue;
                                    }
                                };

                                match file_watcher::read_file_from_offset(&path, offset) {
                                    Ok((new_lines, new_offset)) => {
                                        let new_line_count = new_lines.len();
                                        if !new_lines.is_empty() {
                                            let virtual_path_buf = path
                                                .strip_prefix(&watcher_watched_path)
                                                .unwrap_or(&path);
                                            let virtual_path = virtual_path_buf.to_string_lossy();

                                            let new_entries = file_watcher::parse_log_lines(
                                                &new_lines,
                                                &virtual_path,
                                                &file_path_str,
                                                0,
                                                start_line_number,
                                            );

                                            // Emit new-logs via domain trait
                                            let events2 = events.clone();
                                            let ws2 = workspace_id_clone.clone();
                                            if let Ok(json) =
                                                serde_json::to_string(&new_entries)
                                            {
                                                runtime_handle.spawn(async move {
                                                    events2
                                                        .emit_new_logs(&ws2, &json)
                                                        .await;
                                                });
                                            }

                                            // Store content in CAS + update metadata
                                            let cas2 = Arc::clone(&cas);
                                            let meta2 = Arc::clone(&metadata);
                                            let _ws3 = workspace_id_clone.clone();
                                            let fp3 = file_path_str.clone();
                                            let vp3 = virtual_path.to_string();
                                            runtime_handle.spawn(async move {
                                                // Read full file content for CAS storage
                                                match tokio::fs::read(&fp3).await {
                                                    Ok(content) => {
                                                        match cas2.store(&content).await {
                                                            Ok(hash) => {
                                                                let file_size =
                                                                    content.len() as i64;
                                                                let file_name = Path::new(&fp3)
                                                                    .file_name()
                                                                    .map(|n| {
                                                                        n.to_string_lossy()
                                                                            .to_string()
                                                                    })
                                                                    .unwrap_or_else(|| {
                                                                        fp3.clone()
                                                                    });
                                                                let file_meta =
                                                                    la_core::storage_types::FileMetadata {
                                                                        id: 0,
                                                                        sha256_hash: hash,
                                                                        virtual_path: vp3,
                                                                        original_name: file_name,
                                                                        size: file_size,
                                                                        modified_time: 0,
                                                                        mime_type: None,
                                                                        parent_archive_id: None,
                                                                        depth_level: 0,
                                                                        min_timestamp: None,
                                                                        max_timestamp: None,
                                                                        level_mask: None,
                                                                        analysis_status:
                                                                            la_core::storage_types::AnalysisStatus::Pending,
                                                                    };
                                                                if let Err(e) =
                                                                    meta2.insert_file(&file_meta).await
                                                                {
                                                                    warn!(
                                                                        error = %e,
                                                                        file = %fp3,
                                                                        "Failed to insert watcher file metadata"
                                                                    );
                                                                }
                                                            }
                                                            Err(e) => {
                                                                warn!(
                                                                    error = %e,
                                                                    file = %fp3,
                                                                    "Failed to store watcher file content in CAS"
                                                                );
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            error = %e,
                                                            file = %fp3,
                                                            "Failed to read watcher file for CAS storage"
                                                        );
                                                    }
                                                }
                                            });
                                        }

                                        // Update offsets
                                        {
                                            let mut watchers = watchers_arc.lock();
                                            if let Some(w) =
                                                watchers.get_mut(&workspace_id_clone)
                                            {
                                                w.file_offsets
                                                    .insert(file_path_str.clone(), new_offset);
                                                if new_line_count > 0 {
                                                    w.line_counts.insert(
                                                        file_path_str.clone(),
                                                        start_line_number - 1 + new_line_count,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            error = %e,
                                            file = %file_path_str,
                                            "Failed to read file incrementally"
                                        );
                                    }
                                }
                            }
                        }

                        // Check is_active
                        let is_active = {
                            let watchers = watchers_arc.lock();
                            watchers
                                .get(&workspace_id_clone)
                                .map(|w| w.is_active)
                                .unwrap_or(false)
                        };

                        if !is_active {
                            break;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Watch error");
                    }
                }
            }
        });

        // Store thread handle for later join
        *thread_handle_arc.lock() = Some(handle);

        Ok(WatchStartResult {
            workspace_id: workspace_id.to_string(),
            watched_path: path.to_string(),
        })
    }

    /// Stop watching a workspace.
    ///
    /// Sets the active flag to false, removes the watcher from the map,
    /// and joins the background thread. The notify watcher is dropped
    /// which closes the event channel and unblocks the thread.
    pub async fn stop(&self, workspace_id: &str) -> Result<WatchStopResult> {
        let mut watchers = self.watchers.lock();

        let (thread_handle, watcher_opt) =
            if let Some(watcher_state) = watchers.get_mut(workspace_id) {
                watcher_state.is_active = false;
                let h = watcher_state.thread_handle.lock().take();
                let w = watcher_state.watcher.lock().take();
                (h, w)
            } else {
                return Err(AppError::validation_error(
                    "No active watcher found for this workspace".to_string(),
                ));
            };

        // Remove from map
        watchers.remove(workspace_id);

        // Release the lock so the background thread can complete its final loop iteration
        drop(watchers);

        // Drop the watcher — this closes the tx channel, causing rx iteration to end
        drop(watcher_opt);

        // Join the background thread outside the lock to avoid deadlock
        if let Some(handle) = thread_handle {
            if handle.join().is_err() {
                error!("Failed to join watcher thread");
            }
        }

        Ok(WatchStopResult {
            workspace_id: workspace_id.to_string(),
        })
    }

    /// Check if a workspace is currently being watched.
    pub fn is_watching(&self, workspace_id: &str) -> bool {
        self.watchers.lock().contains_key(workspace_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;

    struct StubEvents {
        last_event: Mutex<String>,
    }

    #[async_trait]
    impl EventPublisher for StubEvents {
        async fn emit_search_start(&self, id: &str) {
            *self.last_event.lock() = format!("search_start:{}", id);
        }
        async fn emit_search_progress(&self, _id: &str, _c: usize) {}
        async fn emit_search_complete(
            &self,
            _id: &str,
            _s: la_core::domain::event::SearchSummary,
        ) {
        }
        async fn emit_search_error(&self, _id: &str, _e: &str) {}
        async fn emit_search_cancelled(&self, _id: &str) {}
        async fn emit_search_timeout(&self, _id: &str) {}
        async fn emit_file_changed(
            &self,
            _workspace_id: &str,
            _event_type: &str,
            _file_path: &str,
            _timestamp: i64,
        ) {
        }
        async fn emit_new_logs(&self, _workspace_id: &str, _entries_json: &str) {}
    }

    struct StubCas;
    #[async_trait]
    impl ContentStorage for StubCas {
        async fn store(&self, _content: &[u8]) -> Result<String> {
            Ok("stub-hash".into())
        }
        async fn retrieve(&self, _hash: &str) -> Result<Vec<u8>> {
            Ok(vec![])
        }
        async fn exists(&self, _hash: &str) -> bool {
            true
        }
    }

    struct StubMetadata;
    #[async_trait]
    impl MetadataStorage for StubMetadata {
        async fn insert_file(&self, _metadata: &la_core::storage_types::FileMetadata) -> Result<i64> {
            Ok(1)
        }
        async fn get_all_files(&self) -> Result<Vec<la_core::storage_types::FileMetadata>> {
            Ok(vec![])
        }
        async fn get_file_by_hash(
            &self,
            _hash: &str,
        ) -> Result<Option<la_core::storage_types::FileMetadata>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn watch_use_case_start_stop() {
        let temp_dir = tempfile::tempdir().unwrap();
        let watch_path = temp_dir.path().to_string_lossy().to_string();

        let events = Arc::new(StubEvents {
            last_event: Mutex::new(String::new()),
        });
        let cas = Arc::new(StubCas);
        let metadata = Arc::new(StubMetadata);
        let watchers = Arc::new(Mutex::new(HashMap::new()));
        let use_case = WatchUseCase::new(events, cas, metadata, watchers);

        let result = use_case.start("ws-1", &watch_path).await.unwrap();
        assert_eq!(result.workspace_id, "ws-1");
        assert_eq!(result.watched_path, watch_path);

        // Small delay to let the background thread start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let result = use_case.stop("ws-1").await.unwrap();
        assert_eq!(result.workspace_id, "ws-1");

        assert!(!use_case.is_watching("ws-1"));
    }

    #[tokio::test]
    async fn watch_use_case_rejects_nonexistent_path() {
        let events = Arc::new(StubEvents {
            last_event: Mutex::new(String::new()),
        });
        let cas = Arc::new(StubCas);
        let metadata = Arc::new(StubMetadata);
        let watchers = Arc::new(Mutex::new(HashMap::new()));
        let use_case = WatchUseCase::new(events, cas, metadata, watchers);

        let err = use_case
            .start("ws-1", "/nonexistent/path/12345")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn watch_use_case_rejects_duplicate_start() {
        let temp_dir = tempfile::tempdir().unwrap();
        let watch_path = temp_dir.path().to_string_lossy().to_string();

        let events = Arc::new(StubEvents {
            last_event: Mutex::new(String::new()),
        });
        let cas = Arc::new(StubCas);
        let metadata = Arc::new(StubMetadata);
        let watchers = Arc::new(Mutex::new(HashMap::new()));
        let use_case = WatchUseCase::new(events, cas, metadata, watchers);

        // First start should succeed
        use_case.start("ws-1", &watch_path).await.unwrap();
        assert!(use_case.is_watching("ws-1"));

        // Second start should fail
        let err = use_case.start("ws-1", &watch_path).await.unwrap_err();
        assert!(err.to_string().contains("already being watched"));

        // Cleanup
        use_case.stop("ws-1").await.unwrap();
    }

    #[tokio::test]
    async fn watch_use_case_stop_nonexistent_errors() {
        let events = Arc::new(StubEvents {
            last_event: Mutex::new(String::new()),
        });
        let cas = Arc::new(StubCas);
        let metadata = Arc::new(StubMetadata);
        let watchers = Arc::new(Mutex::new(HashMap::new()));
        let use_case = WatchUseCase::new(events, cas, metadata, watchers);

        let err = use_case.stop("no-such-workspace").await.unwrap_err();
        assert!(err.to_string().contains("No active watcher"));
    }
}
