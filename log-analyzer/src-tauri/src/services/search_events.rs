//! 搜索事件总线
//!
//! 封装 Tauri 事件发送，统一搜索生命周期事件（开始、进度、完成、错误、摘要）。

use tauri::{AppHandle, Emitter};

use la_core::models::search_statistics::SearchResultSummary;

/// 搜索事件总线
#[derive(Clone)]
pub struct SearchEventBus {
    app_handle: AppHandle,
}

impl SearchEventBus {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn emit_start(&self, _search_id: &str) {
        let _ = self.app_handle.emit("search-start", ());
    }

    pub fn emit_progress(&self, _search_id: &str, count: usize) {
        let _ = self.app_handle.emit("search-progress", count);
    }

    pub fn emit_complete(&self, _search_id: &str, total: usize) {
        let _ = self.app_handle.emit("search-complete", total);
    }

    pub fn emit_error(&self, _search_id: &str, error: &str) {
        let _ = self.app_handle.emit("search-error", error);
    }

    pub fn emit_timeout(&self, search_id: &str) {
        let _ = self.app_handle.emit("search-timeout", search_id);
    }

    pub fn emit_summary(&self, _search_id: &str, summary: &SearchResultSummary) {
        let _ = self.app_handle.emit("search-summary", summary);
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }
}
