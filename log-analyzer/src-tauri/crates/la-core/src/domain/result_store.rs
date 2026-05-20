//! SearchResultRepository — write and page through search results.
//!
//! Abstracts DiskResultStore so application logic doesn't depend on
//! NDJSON file format or disk layout.

use crate::error::Result;
use crate::models::LogEntry;

/// A page of search results with pagination metadata.
#[derive(Debug, Clone)]
pub struct SearchResultPage {
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub is_complete: bool,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

/// Repository for writing and reading search result pages.
/// Methods are synchronous — the underlying storage is file-backed.
pub trait SearchResultRepository: Send + Sync {
    /// Create a new search result session.
    fn create_session(&self, search_id: &str) -> Result<()>;

    /// Append a batch of log entries.
    fn append_entries(&self, search_id: &str, entries: &[LogEntry]) -> Result<()>;

    /// Read a page starting at the given offset.
    fn read_page(&self, search_id: &str, offset: usize, limit: usize) -> Result<SearchResultPage>;

    /// Mark the session complete.
    fn complete_session(&self, search_id: &str) -> Result<()>;

    /// Remove a session.
    fn remove_session(&self, search_id: &str);

    /// Check if a session exists.
    fn has_session(&self, search_id: &str) -> bool;
}
