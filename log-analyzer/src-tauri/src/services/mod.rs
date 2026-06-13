pub mod file_watcher;
pub mod query_planner;
pub mod query_validator;
pub mod regex_engine;
pub mod search_filters;

#[cfg(test)]
mod error_handling_property_tests;

#[cfg(test)]
mod concurrency_property_tests;

// 仅导出外部使用的类型（P7 修剪：移除 15+ 未使用的 re-export）
pub use file_watcher::{append_to_workspace_index, get_file_metadata, read_file_from_offset};
// parse_log_lines, parse_metadata, TimestampParser: 已提取至 la_core::utils
pub use la_core::utils::{parse_log_lines, parse_metadata, TimestampParser};
// querier/searcher 类型
pub use query_planner::ExecutionPlan;
// regex：仅 export commands/search/query.rs 需要的
pub use regex_engine::looks_like_regex_pattern;
// traits: 已从 la_core::traits 导入，此处移除重复 re-export
