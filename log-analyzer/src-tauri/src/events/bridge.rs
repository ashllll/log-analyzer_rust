//! Bridge between the new event system and Tauri's frontend communication

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::{get_event_bus, AppEvent, BroadcastResult};

/// Bridge that forwards events from the internal event bus to Tauri's frontend
pub struct TauriBridge {
    app_handle: AppHandle,
    receiver: broadcast::Receiver<AppEvent>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl TauriBridge {
    /// Create a new Tauri bridge
    pub fn new(app_handle: AppHandle) -> Self {
        let receiver = get_event_bus().subscribe("tauri_bridge".to_string());

        Self {
            app_handle,
            receiver,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the bridge (forwards events to Tauri frontend)
    pub async fn start(&mut self) {
        use std::sync::atomic::Ordering;

        if self.is_running.load(Ordering::Relaxed) {
            warn!("Tauri bridge is already running");
            return;
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!("Starting Tauri event bridge");

        while self.is_running.load(Ordering::Relaxed) {
            match self.receiver.recv().await {
                Ok(event) => {
                    if let Err(e) = self.forward_event_to_tauri(event).await {
                        error!(error = %e, "Failed to forward event to Tauri frontend");
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Event bus closed, stopping Tauri bridge");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "Tauri bridge lagged behind, some events were skipped"
                    );
                    // Continue processing
                }
            }
        }

        info!("Tauri event bridge stopped");
    }

    /// Stop the bridge
    pub fn stop(&self) {
        use std::sync::atomic::Ordering;
        self.is_running.store(false, Ordering::Relaxed);
        info!("Stopping Tauri event bridge");
    }

    /// Forward an event to the Tauri frontend using the appropriate event name
    async fn forward_event_to_tauri(&self, event: AppEvent) -> Result<(), tauri::Error> {
        match event {
            // Search events
            AppEvent::SearchStart { message } => {
                self.app_handle.emit("search-start", message)?;
            }
            AppEvent::SearchProgress { progress } => {
                self.app_handle.emit("search-progress", progress)?;
            }
            AppEvent::SearchResults { results } => {
                self.app_handle.emit("search-results", results)?;
            }
            AppEvent::SearchSummary { summary } => {
                self.app_handle.emit("search-summary", summary)?;
            }
            AppEvent::SearchComplete { count } => {
                self.app_handle.emit("search-complete", count)?;
            }
            AppEvent::SearchError { error } => {
                self.app_handle.emit("search-error", error)?;
            }

            // Async search events
            AppEvent::AsyncSearchStart { search_id } => {
                self.app_handle.emit("async-search-start", search_id)?;
            }
            AppEvent::AsyncSearchProgress {
                search_id,
                progress,
            } => {
                self.app_handle
                    .emit("async-search-progress", (search_id, progress))?;
            }
            AppEvent::AsyncSearchResults { results } => {
                self.app_handle.emit("async-search-results", results)?;
            }
            AppEvent::AsyncSearchComplete { search_id, count } => {
                self.app_handle
                    .emit("async-search-complete", (search_id, count))?;
            }
            AppEvent::AsyncSearchError { search_id, error } => {
                self.app_handle
                    .emit("async-search-error", (search_id, error))?;
            }

            // Task events
            AppEvent::TaskUpdate { progress } => {
                self.app_handle.emit("task-update", progress)?;
            }
            AppEvent::ImportComplete { task_id } => {
                self.app_handle.emit("import-complete", task_id)?;
            }

            // File watcher events
            AppEvent::FileChanged { event } => {
                self.app_handle.emit("file-changed", event)?;
            }
            AppEvent::NewLogs { entries } => {
                self.app_handle.emit("new-logs", entries)?;
            }

            // System events (these might not need to be forwarded to frontend)
            AppEvent::SystemError { error, context } => {
                debug!(error = %error, context = ?context, "System error event (not forwarded to frontend)");
            }
            AppEvent::SystemWarning { warning, context } => {
                debug!(warning = %warning, context = ?context, "System warning event (not forwarded to frontend)");
            }
            AppEvent::SystemInfo { info, context } => {
                debug!(info = %info, context = ?context, "System info event (not forwarded to frontend)");
            }
        }

        Ok(())
    }
}

/// Initialize and start the Tauri bridge
pub async fn init_tauri_bridge(app_handle: AppHandle) -> TauriBridge {
    let bridge = TauriBridge::new(app_handle);

    // Start the bridge in a background task
    let _bridge_handle = bridge.is_running.clone();
    let mut bridge_clone = TauriBridge::new(bridge.app_handle.clone());

    tauri::async_runtime::spawn(async move {
        bridge_clone.start().await;
    });

    bridge
}

/// Convenience functions for emitting events using the new system
pub mod emit {
    use super::*;
    use crate::events::emit_event;
    use crate::models::{FileChangeEvent, LogEntry, SearchResultSummary, TaskProgress};

    /// Emit a search start event
    pub fn search_start(message: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchStart {
            message: message.into(),
        })
    }

    /// Emit a search progress event
    pub fn search_progress(progress: i32) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchProgress { progress })
    }

    /// Emit search results
    pub fn search_results(results: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchResults { results })
    }

    /// Emit search summary
    pub fn search_summary(summary: SearchResultSummary) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchSummary { summary })
    }

    /// Emit search complete event
    pub fn search_complete(count: usize) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchComplete { count })
    }

    /// Emit search error
    pub fn search_error(error: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchError {
            error: error.into(),
        })
    }

    /// Emit async search start event
    pub fn async_search_start(search_id: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchStart {
            search_id: search_id.into(),
        })
    }

    /// Emit async search progress
    pub fn async_search_progress(
        search_id: impl Into<String>,
        progress: u32,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchProgress {
            search_id: search_id.into(),
            progress,
        })
    }

    /// Emit async search results
    pub fn async_search_results(results: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchResults { results })
    }

    /// Emit async search complete
    pub fn async_search_complete(
        search_id: impl Into<String>,
        count: usize,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchComplete {
            search_id: search_id.into(),
            count,
        })
    }

    /// Emit async search error
    pub fn async_search_error(
        search_id: impl Into<String>,
        error: impl Into<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchError {
            search_id: search_id.into(),
            error: error.into(),
        })
    }

    /// Emit task update
    pub fn task_update(progress: TaskProgress) -> BroadcastResult<usize> {
        emit_event(AppEvent::TaskUpdate { progress })
    }

    /// Emit import complete
    pub fn import_complete(task_id: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::ImportComplete {
            task_id: task_id.into(),
        })
    }

    /// Emit file changed event
    pub fn file_changed(event: FileChangeEvent) -> BroadcastResult<usize> {
        emit_event(AppEvent::FileChanged { event })
    }

    /// Emit new logs
    pub fn new_logs(entries: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::NewLogs { entries })
    }

    /// Emit system error
    pub fn system_error(
        error: impl Into<String>,
        context: Option<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemError {
            error: error.into(),
            context,
        })
    }

    /// Emit system warning
    pub fn system_warning(
        warning: impl Into<String>,
        context: Option<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemWarning {
            warning: warning.into(),
            context,
        })
    }

    /// Emit system info
    pub fn system_info(info: impl Into<String>, context: Option<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemInfo {
            info: info.into(),
            context,
        })
    }
}
