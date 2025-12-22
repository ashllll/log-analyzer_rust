pub mod config;
pub mod extraction_policy;
pub mod filters;
pub mod log_entry;
pub mod policy_manager;
pub mod search;
pub mod search_statistics;
pub mod state;

// 重新导出核心类型
pub use config::{AppConfig, FileMetadata};
pub use extraction_policy::ExtractionPolicy;
pub use filters::{PerformanceMetrics, SearchFilters};
pub use log_entry::{FileChangeEvent, LogEntry, TaskProgress};
pub use policy_manager::PolicyManager;
pub use search::*;
pub use state::{AppState, SearchCacheKey, WatcherState};
pub mod validated;

// pub use validated::{ValidatedSearchQuery, ValidatedWorkspaceConfig};
