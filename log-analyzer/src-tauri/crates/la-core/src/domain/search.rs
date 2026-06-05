//! LogSearcher — execute search queries against log content.
//!
//! The searcher operates on raw text content and produces matched log entries.
//! It is intentionally synchronous — heavy search work runs in spawn_blocking
//! at the use case level.

use std::any::Any;
use std::sync::Arc;

use crate::error::Result;
use crate::models::{LogEntry, SearchFilters, SearchQuery};

/// Per-match detail for frontend highlighting.
#[derive(Debug, Clone)]
pub struct MatchDetail {
    pub term_value: String,
    pub byte_offset: usize,
    pub char_offset: usize,
    pub length: usize,
}

/// A compiled and optimized execution plan for a search query.
///
/// Carries an opaque handle to the adapter's internal plan data,
/// eliminating the need for a side-map lookup in match_content.
/// The adapter stores its real plan inside `opaque` at build time;
/// match_content extracts it without any shared mutable state.
#[derive(Clone)]
pub struct ExecutionPlan {
    /// Opaque plan identifier (hash of the query, for debugging).
    pub id: u64,
    /// Number of engines in this plan.
    pub engine_count: usize,
    /// Estimated steps (for debugging / logging).
    pub steps: Vec<String>,
    /// Opaque handle to the adapter's compiled plan data.
    /// Set by the adapter in build_plan(), read in match_content().
    /// Eliminates the Mutex<HashMap<u64, ServicePlan>> side-map.
    pub opaque: Option<Arc<dyn Any + Send + Sync>>,
}

impl std::fmt::Debug for ExecutionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionPlan")
            .field("id", &self.id)
            .field("engine_count", &self.engine_count)
            .field("steps", &self.steps)
            .field("opaque", &self.opaque.as_ref().map(|_| "Some(..)"))
            .finish()
    }
}

/// Engine for searching log content — operates on raw bytes/text.
pub trait LogSearcher: Send + Sync {
    /// Build an execution plan from a search query.
    ///
    /// The plan compiles regex patterns, selects optimal engines
    /// (Aho-Corasick / Standard / Memchr), and is reusable across
    /// multiple files with the same query.
    fn build_plan(&self, query: &SearchQuery) -> Result<ExecutionPlan>;

    /// Match content against the plan and return log entries.
    ///
    /// `content` is the full text content of one log file.
    /// `virtual_path` is the display path shown in search results.
    /// `global_offset` is the starting line number offset for correct
    /// line numbering in results.
    fn match_content(
        &self,
        content: &str,
        virtual_path: &str,
        plan: &ExecutionPlan,
        filters: &SearchFilters,
        global_offset: usize,
    ) -> Vec<LogEntry>;
}
