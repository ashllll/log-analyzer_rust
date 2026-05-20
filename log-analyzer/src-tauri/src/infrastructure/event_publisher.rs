//! EventPublisher adapter — wraps Tauri AppHandle.

use async_trait::async_trait;
use tauri::Emitter;

use la_core::domain::event::{EventPublisher, SearchSummary};

/// Adapter that delegates to Tauri's event system.
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
}
