//! SearchBatch — pure batching logic for search result accumulation.
//!
//! Extracted from SearchExecutor so that the decision of when to flush,
//! truncate, or continue becomes testable pure logic without async or I/O.

use la_core::models::LogEntry;
use std::sync::Arc;

/// Action returned by `SearchBatch::accumulate` after ingesting a chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchAction {
    /// All entries fit; no flush needed yet.
    Continue,
    /// Buffer reached batch size — caller should flush and keep looping.
    Flush,
    /// Max results hit after taking `n` entries — caller should flush and stop.
    Truncate(usize),
}

/// Stateful batch accumulator.
///
/// Holds a buffer of entries and a running total. After each file chunk
/// is processed, `accumulate` decides what the caller should do next.
pub struct SearchBatch {
    buffer: Vec<LogEntry>,
    total: usize,
    batch_size: usize,
}

impl SearchBatch {
    pub fn new(batch_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            total: 0,
            batch_size,
        }
    }

    /// Ingest a chunk of entries and decide the next action.
    ///
    /// # Arguments
    /// * `entries` — new entries from the latest file chunk
    /// * `max_results` — hard cap on total results
    ///
    /// # Returns
    /// * `Continue` — entries added, no flush needed
    /// * `Flush` — buffer full, caller should `take()` and flush
    /// * `Truncate(n)` — only `n` entries were taken (max_results hit),
    ///   caller should `take()` and flush, then stop
    pub fn accumulate(&mut self, entries: Vec<LogEntry>, max_results: usize) -> BatchAction {
        let remaining = max_results.saturating_sub(self.total);

        if remaining == 0 {
            // Already at max — nothing more to take.
            // If buffer has data, signal a final flush; otherwise truncate(0).
            return if self.buffer.is_empty() {
                BatchAction::Truncate(0)
            } else {
                BatchAction::Flush
            };
        }

        if entries.len() > remaining {
            // Only some entries fit before hitting max_results.
            let to_take = remaining;
            self.buffer.extend(entries.into_iter().take(to_take));
            self.total += to_take;
            return BatchAction::Truncate(to_take);
        }

        // All entries fit.
        let added = entries.len();
        self.buffer.extend(entries);
        self.total += added;

        if self.total >= max_results {
            return BatchAction::Flush;
        }

        if self.buffer.len() >= self.batch_size {
            return BatchAction::Flush;
        }

        BatchAction::Continue
    }

    /// Take the current buffer (draining it) for flushing.
    pub fn take(&mut self) -> Vec<LogEntry> {
        std::mem::take(&mut self.buffer)
    }

    /// Total results accumulated so far (including flushed batches).
    pub fn total(&self) -> usize {
        self.total
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::models::LogEntry;

    fn make_entry(id: usize) -> LogEntry {
        LogEntry {
            id,
            timestamp: Arc::from(""),
            level: Arc::from(""),
            file: Arc::from(""),
            line: 1,
            content: Arc::from(""),
            real_path: Arc::from(""),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    fn make_entries(count: usize) -> Vec<LogEntry> {
        (0..count).map(make_entry).collect()
    }

    #[test]
    fn empty_results_continue() {
        let mut batch = SearchBatch::new(2000);
        let action = batch.accumulate(vec![], 1000);
        assert_eq!(action, BatchAction::Continue);
        assert!(batch.is_empty());
        assert_eq!(batch.total(), 0);
    }

    #[test]
    fn batch_boundary_flush() {
        let mut batch = SearchBatch::new(10);
        let action = batch.accumulate(make_entries(10), 1000);
        assert_eq!(action, BatchAction::Flush);
        assert_eq!(batch.total(), 10);
    }

    #[test]
    fn below_boundary_continue() {
        let mut batch = SearchBatch::new(10);
        let action = batch.accumulate(make_entries(5), 1000);
        assert_eq!(action, BatchAction::Continue);
        assert_eq!(batch.total(), 5);
    }

    #[test]
    fn max_results_truncation() {
        let mut batch = SearchBatch::new(2000);
        let action = batch.accumulate(make_entries(15), 10);
        assert_eq!(action, BatchAction::Truncate(10));
        assert_eq!(batch.total(), 10);
        assert_eq!(batch.take().len(), 10);
    }

    #[test]
    fn exact_max_results_flush() {
        let mut batch = SearchBatch::new(2000);
        // total=0, add 10, max=10 → exactly at limit, should Flush
        let action = batch.accumulate(make_entries(10), 10);
        assert_eq!(action, BatchAction::Flush);
        assert_eq!(batch.total(), 10);
    }

    #[test]
    fn accumulate_across_multiple_chunks() {
        let mut batch = SearchBatch::new(10);
        let a = batch.accumulate(make_entries(3), 100);
        assert_eq!(a, BatchAction::Continue);
        let b = batch.accumulate(make_entries(4), 100);
        assert_eq!(b, BatchAction::Continue);
        let c = batch.accumulate(make_entries(5), 100);
        assert_eq!(c, BatchAction::Flush);
        assert_eq!(batch.total(), 12);
        // buffer still holds 12 entries until caller calls take()
        assert!(!batch.is_empty());
        assert_eq!(batch.take().len(), 12);
        assert!(batch.is_empty());
    }

    #[test]
    fn truncation_at_zero_remaining() {
        let mut batch = SearchBatch::new(10);
        batch.accumulate(make_entries(5), 5); // fill to max
        // total=5, remaining=0, empty chunk
        let action = batch.accumulate(vec![], 5);
        assert_eq!(action, BatchAction::Flush);
        assert_eq!(batch.take().len(), 5);
    }
}
