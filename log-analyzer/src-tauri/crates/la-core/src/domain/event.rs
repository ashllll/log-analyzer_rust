//! EventPublisher — emit search and task events to the frontend.
//!
//! Abstracts Tauri's `app_handle.emit()` behind a trait so application
//! logic doesn't depend on the Tauri framework directly.

use async_trait::async_trait;

/// Summary statistics emitted when a search completes.
#[derive(Debug, Clone)]
pub struct SearchSummary {
    pub total_count: usize,
    pub duration_ms: u64,
    pub was_truncated: bool,
}

/// Publisher for application events consumed by the frontend.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Emitted when a new search starts.
    async fn emit_search_start(&self, search_id: &str);

    /// Emitted periodically during search execution.
    async fn emit_search_progress(&self, search_id: &str, count: usize);

    /// Emitted when a search completes successfully.
    async fn emit_search_complete(&self, search_id: &str, summary: SearchSummary);

    /// Emitted when a search encounters an error.
    async fn emit_search_error(&self, search_id: &str, error: &str);

    /// Emitted when a search is cancelled.
    async fn emit_search_cancelled(&self, search_id: &str);

    /// Emitted when a search times out.
    async fn emit_search_timeout(&self, search_id: &str);
}
