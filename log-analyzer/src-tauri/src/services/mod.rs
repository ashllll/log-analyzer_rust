pub mod file_watcher;
pub mod index_store;
pub mod query_executor;
pub mod search_statistics;

// 重新导出所有公共类型和函数
pub use file_watcher::{
    append_to_workspace_index, get_file_metadata, parse_log_lines, parse_metadata,
    read_file_from_offset,
};
pub use index_store::{load_index, save_index};
pub use query_executor::{ExecutionPlan, MatchDetail, QueryExecutor};
pub use search_statistics::calculate_keyword_statistics;
