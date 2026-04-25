// 留在 la-tauri 的有状态模块
pub mod cache_state;
pub mod search_state;
pub mod state;
pub mod workspace_state;

// 从 la-core re-export 的纯数据模块
pub mod config;
pub mod extraction_policy;
pub mod filters;
pub mod import_decision;
pub mod log_entry;
pub mod policy_manager;
pub mod processing_report;
pub mod search;
pub mod search_statistics;
pub mod validated;

// 重新导出核心类型
pub use cache_state::CacheState;
pub use config::{AppConfig, FileFilterConfig, FilterMode};
pub use extraction_policy::ExtractionPolicy;
pub use filters::{PerformanceMetrics, SearchFilters};
pub use import_decision::{FileTypeInfo, ImportDecision, ImportDecisionDetails, RejectionReason};
pub use log_entry::{FileChangeEvent, LogEntry, TaskProgress};
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
