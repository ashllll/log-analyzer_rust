//! 文件监听服务
//!
//! P7 修剪后仅保留：
//! - `WatcherState`：单个工作区监听线程的生命周期句柄（由 WorkspaceServiceImpl 持有）
//! - TimestampParser 回归测试（实现位于 la_core::utils）
//!
//! 历史职责已迁移或删除：
//! - 增量读取 / 索引追加 / CAS 写入 → `infrastructure::FileTailer` + `WatcherRunner`
//!   （旧实现 `read_file_from_offset` / `append_to_workspace_index` /
//!   `get_file_metadata` 零调用方，已删除）
//! - 日志行解析 → `la_core::utils`（调用方均直接使用 la_core 路径）

use std::sync::Arc;

/// 文件监听器状态
///
/// 仅保留生命周期管理字段：file_offsets / line_counts 由 FileTailer 持有，
/// workspace_id / watched_path 构造后从未被读取，均已移除。
#[derive(Debug, Clone)]
pub struct WatcherState {
    pub is_active: bool,
    /// 监听线程的 JoinHandle，用于确保正确退出并清理资源
    /// 使用 parking_lot::Mutex 避免 poison 问题（B-M3）
    pub thread_handle: Arc<parking_lot::Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// 底层文件监听器，存放在这里确保其生命周期与状态同步
    /// 使用 parking_lot::Mutex 避免 poison 问题（B-M3）
    pub watcher: Arc<parking_lot::Mutex<Option<notify::RecommendedWatcher>>>,
}

#[cfg(test)]
mod tests {
    use la_core::utils::TimestampParser;

    #[test]
    fn test_timestamp_parser_iso8601() {
        let line = "2024-01-15T10:30:45.123 [INFO] Application started";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15T10:30:45.123");
    }

    #[test]
    fn test_timestamp_parser_common() {
        let line = "2024-01-15 10:30:45 [ERROR] Database connection failed";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "2024-01-15 10:30:45");
    }

    #[test]
    fn test_timestamp_parser_us() {
        let line = "01/15/2024 10:30:45.456 [WARN] Low memory warning";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), "01/15/2024 10:30:45.456");
    }

    #[test]
    fn test_timestamp_parser_no_match() {
        let line = "This is a log line without timestamp";
        let timestamp = TimestampParser::parse_timestamp(line);
        assert!(timestamp.is_none());
    }

    #[test]
    fn test_parse_naive_datetime_supports_datetime_local() {
        let timestamp = TimestampParser::parse_naive_datetime("2024-01-15T10:30");
        assert!(timestamp.is_some());
        assert_eq!(
            timestamp.unwrap().format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-01-15 10:30:00"
        );
    }
}
