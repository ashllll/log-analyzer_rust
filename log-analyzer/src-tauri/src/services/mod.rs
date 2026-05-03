pub mod file_watcher;
pub mod query_executor;
pub mod query_planner;
pub mod query_validator;
pub mod regex_engine;
pub mod search_events;
pub mod search_filters;
pub mod search_pipeline;
pub mod search_statistics;
pub mod search_strategies;
pub mod traits;

#[cfg(test)]
mod error_handling_property_tests;

#[cfg(test)]
mod concurrency_property_tests;

// 重新导出所有公共类型和函数
pub use file_watcher::{
    append_to_workspace_index, get_file_metadata, parse_log_lines, parse_metadata,
    read_file_from_offset,
};
pub use query_executor::{MatchDetail, QueryExecutor};
pub use query_planner::{ExecutionPlan, QueryPlannerAdapter};
pub use regex_engine::{
    AhoCorasickEngine, EngineError, EngineInfo, EngineMatches, EngineType, MatchResult,
    RegexEngine, StandardEngine,
};
pub use search_statistics::calculate_keyword_statistics;
pub use traits::{
    ContentStorage, MetadataStorage, PlanResult, QueryExecutor as QueryExecutorTrait,
    QueryPlanning, QueryValidation, ValidationResult,
};
