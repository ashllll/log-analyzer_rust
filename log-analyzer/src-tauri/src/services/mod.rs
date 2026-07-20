pub mod file_watcher;
pub mod query_planner;
pub mod regex_engine;
pub mod search_filters;

#[cfg(test)]
mod error_handling_property_tests;

#[cfg(test)]
mod concurrency_property_tests;

// 仅导出外部使用的类型（P7 修剪：移除 15+ 未使用的 re-export）
// file_watcher 仅保留 WatcherState（由 workspace_service_impl 直接按路径引用）；
// parse_log_lines / parse_metadata / TimestampParser 已提取至 la_core::utils，
// 调用方均直接使用 la_core 路径，此处不再重复 re-export。
// querier/searcher 类型
pub use query_planner::ExecutionPlan;
// query_planner: export standalone validation for frontend type-ahead
pub use query_planner::compute_query_cache_key;
pub use query_planner::QueryPlanner;
// regex：仅 export commands/search/query.rs 需要的
pub use regex_engine::looks_like_regex_pattern;
// regex_engine types for independent use and testing
pub use regex_engine::{
    EngineError, EngineInfo, EngineMatches, EngineType, MatchResult, RegexEngine,
};
// traits: 已从 la_core::traits 导入，此处移除重复 re-export
