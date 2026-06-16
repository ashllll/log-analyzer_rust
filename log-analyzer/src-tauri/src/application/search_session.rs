//! SearchSessionManager — owns the backend lifecycle of a search session.
//!
//! A search session has two runtime resources:
//! - A `DiskResultStore` session where paginated results are written/read.
//! - A `CancellationToken` used to abort an in-flight search.
//!
//! This module centralises all `search_id`-based operations so that commands
//! no longer need to iterate workspaces (for cancellation) or read through a
//! workspace service (for paging). The manager is owned by the backend app
//! state and shared with each `WorkspaceServiceImpl`.

use std::collections::HashMap;
use std::sync::Arc;

use la_core::error::{AppError, Result};
use la_search::{DiskResultStore, SearchPageResult};
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;

// ============================================================================
// SearchSessionManager
// ============================================================================

/// Central owner of search session runtime state.
#[derive(Clone)]
pub struct SearchSessionManager {
    disk_result_store: Arc<DiskResultStore>,
    sessions: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl SearchSessionManager {
    /// Create a new manager backed by the given global disk result store.
    pub fn new(disk_result_store: Arc<DiskResultStore>) -> Self {
        Self {
            disk_result_store,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create the `DiskResultStore` session for a new search.
    ///
    /// Called before returning `search_id` so the frontend can safely request
    /// page 0 immediately.
    pub fn create_session(&self, search_id: &str) -> Result<()> {
        self.disk_result_store
            .create_session(search_id)
            .map_err(|e| AppError::io_error(format!("Failed to create search session: {e}"), None))
    }

    /// Register a cancellation token for an active search session.
    pub fn register_token(&self, search_id: &str, token: CancellationToken) {
        self.sessions.lock().insert(search_id.to_string(), token);
    }

    /// Cancel the search identified by `search_id`, if it is still active.
    pub fn cancel_search(&self, search_id: &str) -> Result<()> {
        let token = self.sessions.lock().get(search_id).cloned();
        match token {
            Some(t) => {
                t.cancel();
                Ok(())
            }
            None => Err(AppError::not_found(format!(
                "Search session '{search_id}' not found"
            ))),
        }
    }

    /// Read a page of results for the given search session.
    pub fn fetch_search_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<SearchPageResult> {
        let limit = limit.min(10_000);

        if !self.disk_result_store.has_session(search_id) {
            return Err(AppError::not_found(format!(
                "Search session '{search_id}' not found"
            )));
        }

        self.disk_result_store
            .read_page(search_id, offset, limit)
            .map_err(|e| AppError::io_error(format!("Failed to read search page: {e}"), None))
    }

    /// Remove the cancellation token after a search finishes.
    ///
    /// The `DiskResultStore` session is deliberately kept alive so the frontend
    /// can continue paginating through results.
    pub fn cleanup_token(&self, search_id: &str) {
        self.sessions.lock().remove(search_id);
    }

    /// Returns the number of active cancellation tokens.
    #[cfg(test)]
    pub(crate) fn active_token_count(&self) -> usize {
        self.sessions.lock().len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use la_core::models::LogEntry;
    use std::sync::Arc as StdArc;

    fn make_entry(id: usize, content: &str) -> LogEntry {
        LogEntry {
            id,
            timestamp: StdArc::from("2026-01-01T00:00:00Z"),
            level: StdArc::from("INFO"),
            file: StdArc::from("test.log"),
            real_path: StdArc::from("/tmp/test.log"),
            line: id,
            content: StdArc::from(content),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    fn make_manager() -> (SearchSessionManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();
        (SearchSessionManager::new(Arc::new(store)), dir)
    }

    #[test]
    fn create_session_then_fetch_page() {
        let (mgr, _dir) = make_manager();
        let search_id = "session-1";

        mgr.create_session(search_id).unwrap();

        let entries: Vec<LogEntry> = (0..5).map(|i| make_entry(i, "hello")).collect();
        // Write directly through the underlying store; the manager only exposes
        // creation/paging for result sessions.
        mgr.disk_result_store
            .append_entries(search_id, &entries)
            .unwrap();
        mgr.disk_result_store.complete_session(search_id).unwrap();

        let page = mgr.fetch_search_page(search_id, 0, 2).unwrap();
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.total_count, 5);
        assert!(page.is_complete);
        assert!(page.has_more);
        assert_eq!(page.next_offset, Some(2));
    }

    #[test]
    fn fetch_page_for_unknown_session_fails() {
        let (mgr, _dir) = make_manager();
        let err = mgr.fetch_search_page("missing", 0, 10).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn register_and_cancel_token() {
        let (mgr, _dir) = make_manager();
        let search_id = "session-2";
        let token = CancellationToken::new();

        mgr.register_token(search_id, token.clone());
        assert_eq!(mgr.active_token_count(), 1);

        mgr.cancel_search(search_id).unwrap();
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_unknown_search_fails() {
        let (mgr, _dir) = make_manager();
        let err = mgr.cancel_search("missing").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn cleanup_token_removes_only_token() {
        let (mgr, _dir) = make_manager();
        let search_id = "session-3";

        mgr.create_session(search_id).unwrap();
        mgr.register_token(search_id, CancellationToken::new());

        mgr.cleanup_token(search_id);
        assert_eq!(mgr.active_token_count(), 0);

        // Result session must remain so pagination still works.
        assert!(mgr.disk_result_store.has_session(search_id));
    }
}
