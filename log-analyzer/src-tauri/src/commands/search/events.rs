//! 搜索事件类型与推送函数

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use la_core::models::search_statistics::SearchResultSummary;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchIdEvent {
    pub search_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchProgressEvent {
    pub search_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchSummaryEvent {
    pub search_id: String,
    pub summary: SearchResultSummary,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchCompleteEvent {
    pub search_id: String,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchErrorEvent {
    pub search_id: String,
    pub error: String,
}

/// Streaming result batch — carries actual LogEntry data for real-time frontend display.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchResultBatchEvent {
    pub search_id: String,
    pub entries: Vec<la_core::models::LogEntry>,
    pub offset: usize,
    pub is_final: bool,
}

pub(crate) fn emit_search_id_event(app_handle: &AppHandle, event_name: &str, search_id: &str) {
    let _ = app_handle.emit(event_name, SearchIdEvent { search_id: search_id.to_string() });
}

pub(crate) fn emit_search_error(app_handle: &AppHandle, search_id: &str, error: impl Into<String>) {
    let _ = app_handle.emit("search-error", SearchErrorEvent { search_id: search_id.to_string(), error: error.into() });
}
