//! EventPublisher adapter — wraps Tauri AppHandle.

use async_trait::async_trait;
use tauri::Emitter;

use la_core::domain::event::EventPublisher;
use la_core::models::search_statistics::SearchResultSummary;

/// Adapter that delegates to Tauri's event system.
#[derive(Clone)]
pub struct TauriEventPublisher {
    pub app_handle: tauri::AppHandle,
}

#[async_trait]
impl EventPublisher for TauriEventPublisher {
    async fn emit_search_start(&self, search_id: &str) {
        if let Err(e) = self.app_handle.emit(
            "search-start",
            serde_json::json!({ "search_id": search_id }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-start event");
        }
    }

    async fn emit_search_progress(&self, search_id: &str, count: usize) {
        if let Err(e) = self.app_handle.emit(
            "search-progress",
            serde_json::json!({ "search_id": search_id, "count": count }),
        ) {
            tracing::warn!(error = %e, search_id, count, "Failed to emit search-progress event");
        }
    }

    async fn emit_search_complete(&self, search_id: &str, summary: SearchResultSummary) {
        if let Err(e) = self.app_handle.emit(
            "search-complete",
            serde_json::json!({
                "search_id": search_id,
                "total_count": summary.total_matches,
            }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-complete event");
        }
        if let Err(e) = self.app_handle.emit(
            "search-summary",
            serde_json::json!({
                "search_id": search_id,
                "summary": summary,
            }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-summary event");
        }
    }

    async fn emit_search_error(&self, search_id: &str, error: &str) {
        if let Err(e) = self.app_handle.emit(
            "search-error",
            serde_json::json!({ "search_id": search_id, "error": error }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-error event");
        }
    }

    async fn emit_search_cancelled(&self, search_id: &str) {
        if let Err(e) = self.app_handle.emit(
            "search-cancelled",
            serde_json::json!({ "search_id": search_id }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-cancelled event");
        }
    }

    async fn emit_search_timeout(&self, search_id: &str) {
        if let Err(e) = self.app_handle.emit(
            "search-timeout",
            serde_json::json!({ "search_id": search_id }),
        ) {
            tracing::warn!(error = %e, search_id, "Failed to emit search-timeout event");
        }
    }

    async fn emit_import_complete(&self, task_id: &str) {
        if let Err(e) = self.app_handle.emit(
            "import-complete",
            serde_json::json!({
                "task_id": task_id,
            }),
        ) {
            tracing::warn!(error = %e, task_id, "Failed to emit import-complete event");
        }
    }

    async fn emit_import_error(&self, error: &str) {
        if let Err(e) = self.app_handle.emit("import-error", error) {
            tracing::warn!(error = %e, "Failed to emit import-error event");
        }
    }

    async fn emit_validation_report(&self, workspace_id: &str, report_json: &str) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(report_json) {
            if let Err(e) = self.app_handle.emit("validation-report", value) {
                tracing::warn!(error = %e, workspace_id, "Failed to emit validation-report event");
            }
        } else {
            // Fallback: emit as raw JSON string if parsing fails
            if let Err(e) = self.app_handle.emit(
                "validation-report",
                serde_json::json!({ "workspace_id": workspace_id, "raw": report_json }),
            ) {
                tracing::warn!(error = %e, workspace_id, "Failed to emit validation-report fallback event");
            }
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
                        tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
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
