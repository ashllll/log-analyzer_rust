//! LogSearcher — execute search queries against log content.
//!
//! The searcher operates on raw text content and produces matched log entries.
//! It is intentionally synchronous — heavy search work runs in spawn_blocking
//! at the use case level.

use std::sync::Arc;

use crate::error::Result;
use crate::models::match_detail::MatchDetail;
use crate::models::{LogEntry, SearchFilters, SearchQuery};

/// A compiled plan that can match a single line of text.
///
/// Implementations carry compiled regex engines and matching strategy.
/// The trait is deliberately narrow — one method — so the domain
/// interface stays small while the implementation can be complex.
pub trait MatchPlan: Send + Sync {
    /// Match a single line and return highlight details.
    ///
    /// Returns `None` if the line does not match.
    /// Returns `Some(vec![])` for strategies like `Not` where
    /// the line passes but no terms need highlighting.
    fn match_line(&self, line: &str) -> Option<Vec<MatchDetail>>;
}

/// A compiled and optimized execution plan for a search query.
///
/// Carries a typed handle to the adapter's internal plan data,
/// eliminating both the side-map lookup and the fragile `Any` downcast.
/// The adapter stores its real plan inside `plan` at build time;
/// match_content calls `plan.match_line()` through the trait.
#[derive(Clone)]
pub struct ExecutionPlan {
    /// Opaque plan identifier (hash of the query, for debugging).
    pub id: u64,
    /// Number of engines in this plan.
    pub engine_count: usize,
    /// Estimated steps (for debugging / logging).
    pub steps: Vec<String>,
    /// Typed handle to the adapter's compiled plan.
    /// Set by the adapter in build_plan(), used in match_content().
    pub plan: Option<Arc<dyn MatchPlan>>,
}

impl std::fmt::Debug for ExecutionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionPlan")
            .field("id", &self.id)
            .field("engine_count", &self.engine_count)
            .field("steps", &self.steps)
            .field("plan", &self.plan.as_ref().map(|_| "Some(..)"))
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
