//! SearchResultRepository adapter — wraps DiskResultStore.

use std::sync::Arc;

use la_core::domain::{SearchResultPage, SearchResultRepository};
use la_core::error::Result;
use la_search::DiskResultStore;

/// Adapter that delegates to the existing DiskResultStore.
pub struct DiskResultStoreRepo {
    pub store: Arc<DiskResultStore>,
}

impl SearchResultRepository for DiskResultStoreRepo {
    fn create_session(&self, search_id: &str) -> Result<()> {
        self.store.create_session(search_id).map_err(|e| {
            la_core::error::AppError::io_error(
                format!("Failed to create search session: {e}"),
                None,
            )
        })
    }

    fn append_entries(&self, search_id: &str, entries: &[la_core::models::LogEntry]) -> Result<()> {
        self.store
            .append_entries(search_id, entries)
            .map(|_| ())
            .map_err(|e| {
                la_core::error::AppError::io_error(format!("Failed to append entries: {e}"), None)
            })
    }

    fn read_page(&self, search_id: &str, offset: usize, limit: usize) -> Result<SearchResultPage> {
        let page = self
            .store
            .read_page(search_id, offset, limit)
            .map_err(|e| {
                la_core::error::AppError::io_error(
                    format!("Failed to read search page: {e}"),
                    None,
                )
            })?;

        Ok(SearchResultPage {
            entries: page.entries,
            total_count: page.total_count,
            is_complete: page.is_complete,
            has_more: page.has_more,
            next_offset: page.next_offset,
        })
    }

    fn complete_session(&self, search_id: &str) -> Result<()> {
        self.store.complete_session(search_id).map_err(|e| {
            la_core::error::AppError::io_error(format!("Failed to complete session: {e}"), None)
        })
    }

    fn remove_session(&self, search_id: &str) {
        self.store.remove_session(search_id);
    }

    fn has_session(&self, search_id: &str) -> bool {
        self.store.has_session(search_id)
    }
}
