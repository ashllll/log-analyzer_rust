//! EventPublisher adapter — wraps Tauri AppHandle.

use async_trait::async_trait;
use tauri::Emitter;

use la_core::domain::event::{EventPublisher, SearchSummary};

/// Adapter that delegates to Tauri's event system.
#[derive(Clone)]
pub struct TauriEventPublisher {
    pub app_handle: tauri::AppHandle,
}

#[async_trait]
impl EventPublisher for TauriEventPublisher {
    async fn emit_search_start(&self, search_id: &str) {
        let _ = self.app_handle.emit(
            "search-start",
            serde_json::json!({ "search_id": search_id }),
        );
    }

    async fn emit_search_progress(&self, search_id: &str, count: usize) {
        let _ = self.app_handle.emit(
            "search-progress",
            serde_json::json!({ "search_id": search_id, "count": count }),
        );
    }

    async fn emit_search_complete(&self, search_id: &str, summary: SearchSummary) {
        let _ = self.app_handle.emit(
            "search-complete",
            serde_json::json!({
                "search_id": search_id,
                "total_count": summary.total_count,
            }),
        );
        let _ = self.app_handle.emit(
            "search-summary",
            serde_json::json!({
                "search_id": search_id,
                "duration_ms": summary.duration_ms,
                "was_truncated": summary.was_truncated,
            }),
        );
    }

    async fn emit_search_error(&self, search_id: &str, error: &str) {
        let _ = self.app_handle.emit(
            "search-error",
            serde_json::json!({ "search_id": search_id, "error": error }),
        );
    }

    async fn emit_search_cancelled(&self, search_id: &str) {
        let _ = self.app_handle.emit(
            "search-cancelled",
            serde_json::json!({ "search_id": search_id }),
        );
    }

    async fn emit_search_timeout(&self, search_id: &str) {
        let _ = self.app_handle.emit(
            "search-timeout",
            serde_json::json!({ "search_id": search_id }),
        );
    }

    async fn emit_file_changed(
        &self,
        workspace_id: &str,
        event_type: &str,
        file_path: &str,
        timestamp: i64,
    ) {
        let _ = self.app_handle.emit(
            "file-changed",
            serde_json::json!({
                "event_type": event_type,
                "file_path": file_path,
                "workspace_id": workspace_id,
                "timestamp": timestamp,
            }),
        );
    }

    async fn emit_new_logs(&self, _workspace_id: &str, entries_json: &str) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(entries_json) {
            let _ = self.app_handle.emit("new-logs", value);
        }
    }
}

impl TauriEventPublisher {
    /// Emit a workspace event with retry (3 attempts, 10ms backoff).
    ///
    /// Moved from StateSync so retry-on-failure is centralized in the
    /// event publisher adapter rather than living in the state-sync layer.
    pub async fn emit_workspace_event_with_retry(
        &self,
        event: &crate::state_sync::WorkspaceEvent,
    ) -> Result<(), String> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 10;
        let mut last_error: Option<String> = None;

        for attempt in 0..MAX_RETRIES {
            match self.app_handle.emit("workspace-event", event) {
                Ok(()) => {
                    tracing::debug!(
                        event_type = ?event,
                        attempt = attempt + 1,
                        "Broadcasted workspace event"
                    );
                    return Ok(());
                }
                Err(e) => {
                    let msg = format!("Failed to emit event: {e}");
                    last_error = Some(msg.clone());
                    if attempt + 1 < MAX_RETRIES {
                        tracing::warn!(
                            error = %e,
                            attempt = attempt + 1,
                            max_retries = MAX_RETRIES,
                            "Emit failed, retrying..."
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS,
                        )).await;
                    }
                }
            }
        }

        let error_msg = last_error.unwrap_or_else(|| "Unknown emit failure".to_string());
        tracing::error!(
            error = %error_msg,
            event_type = ?event,
            max_retries = MAX_RETRIES,
            "Failed to emit workspace event after all retries"
        );
        Err(error_msg)
    }
}
