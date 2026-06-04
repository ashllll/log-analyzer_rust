pub mod file_watcher;
pub mod query_executor;
pub mod query_planner;
pub mod query_validator;
pub mod regex_engine;
pub mod traits;

#[cfg(test)]
mod error_handling_property_tests;

#[cfg(test)]
mod concurrency_property_tests;

// 重新导出所有公共类型和函数
pub use file_watcher::{
    append_to_workspace_index, get_file_metadata, read_file_from_offset,
};
// parse_log_lines, parse_metadata, TimestampParser: 已提取至 la_core::utils，
// 保留 re-export 向后兼容
pub use la_core::utils::{parse_log_lines, parse_metadata, TimestampParser};
pub use query_executor::{MatchDetail, QueryPlanBuilder};
pub use query_planner::{ExecutionPlan, QueryPlannerAdapter};
pub use regex_engine::{
    looks_like_regex_pattern, AhoCorasickEngine, EngineError, EngineInfo, EngineMatches,
    EngineType, MatchResult, RegexEngine, StandardEngine,
};
pub use traits::{
    ContentStorage, MetadataStorage, PlanResult, QueryExecutor as QueryExecutorTrait,
    QueryPlanning, QueryValidation, ValidationResult,
};
