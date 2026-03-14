pub mod cache_state;
pub mod config;
pub mod extraction_policy;
pub mod filters;
pub mod import_decision;
pub mod log_entry;
pub mod metrics_state;
pub mod policy_manager;
pub mod processing_report;
pub mod search;
pub mod search_state;
pub mod search_statistics;
pub mod state;
pub mod workspace_state;

// 重新导出核心类型
pub use cache_state::CacheState;
pub use config::{AppConfig, FileFilterConfig, FilterMode};
pub use extraction_policy::ExtractionPolicy;
pub use filters::{PerformanceMetrics, SearchFilters};
pub use import_decision::{FileTypeInfo, ImportDecision, ImportDecisionDetails, RejectionReason};
pub use log_entry::{FileChangeEvent, LogEntry, TaskProgress};
pub use metrics_state::MetricsState;
pub use policy_manager::PolicyManager;
pub use processing_report::{
    ErrorCategory, ErrorSeverity, ProcessingError, ProcessingReport, ProcessingReportSummary,
    ProcessingStatistics, ProcessingStatus,
};
pub use search::SearchCacheKey;
pub use search::*;
pub use search_state::SearchState;
pub use search_statistics::{KeywordStatistics, SearchResultSummary};
pub use state::AppState;
pub use workspace_state::WorkspaceState;
pub mod validated;
