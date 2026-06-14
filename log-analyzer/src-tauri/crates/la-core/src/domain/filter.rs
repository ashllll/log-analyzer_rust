//! Filter trait — abstracts file and line-level search filtering.
//!
//! Defined in the domain layer so that application/infrastructure code
//! depends on the trait rather than the concrete `CompiledSearchFilters`.
//!
//! # Adapters
//!
//! - `CompiledSearchFilters` — production adapter (compiles from `SearchFilters`).
//! - Mock implementations for unit testing `QueryEngineLogSearcher`.

/// Metadata extracted from a single log line, used by filters to decide
/// whether the line passes time-range and log-level constraints.
#[derive(Debug, Clone)]
pub struct LineMetadata {
    /// The raw timestamp string from the log line (e.g. "2024-01-15T10:30:45").
    pub timestamp: String,
    /// The canonical log level name (e.g. "ERROR", "WARN").
    pub level: &'static str,
    /// The lowercased log level for set-lookup (e.g. "error").
    pub level_normalized: &'static str,
    /// Parsed datetime, if timestamp was successfully parsed.
    pub datetime: Option<chrono::NaiveDateTime>,
    /// Bitmask computed from the level (0 if unknown).
    pub level_mask: u8,
}

/// Trait for filtering search results by file path and line metadata.
///
/// Implementations decide which files and lines to include based on
/// criteria such as time range, log level mask, and file-pattern matching.
pub trait Filter: Send + Sync {
    /// Returns `true` if the file (identified by its virtual or real path)
    /// passes the file-pattern filter.
    fn matches_file(&self, virtual_path: &str, real_path: Option<&str>) -> bool;

    /// Returns `true` if the parsed line metadata passes time-range and
    /// log-level filters.
    fn matches_line(&self, metadata: &LineMetadata) -> bool;

    /// Whether the filter includes a time-range constraint (used to skip
    /// expensive timestamp parsing when not needed).
    fn has_time_filter(&self) -> bool;
}
