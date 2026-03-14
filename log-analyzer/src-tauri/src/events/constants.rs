//! 事件名称常量定义
//! 
//! 统一使用 snake_case 命名规范，确保前后端事件名称一致性

// ============================================================================
// 搜索相关事件
// ============================================================================

/// 搜索开始
pub const EVENT_SEARCH_START: &str = "search-start";
/// 搜索进度更新
pub const EVENT_SEARCH_PROGRESS: &str = "search-progress";
/// 搜索结果数据
pub const EVENT_SEARCH_RESULTS: &str = "search-results";
/// 搜索摘要统计
pub const EVENT_SEARCH_SUMMARY: &str = "search-summary";
/// 搜索完成
pub const EVENT_SEARCH_COMPLETE: &str = "search-complete";
/// 搜索错误
pub const EVENT_SEARCH_ERROR: &str = "search-error";
/// 搜索取消
pub const EVENT_SEARCH_CANCELLED: &str = "search-cancelled";

// ============================================================================
// 异步搜索相关事件
// ============================================================================

/// 异步搜索开始
pub const EVENT_ASYNC_SEARCH_START: &str = "async-search-start";
/// 异步搜索进度更新
pub const EVENT_ASYNC_SEARCH_PROGRESS: &str = "async-search-progress";
/// 异步搜索结果数据
pub const EVENT_ASYNC_SEARCH_RESULTS: &str = "async-search-results";
/// 异步搜索完成
pub const EVENT_ASYNC_SEARCH_COMPLETE: &str = "async-search-complete";
/// 异步搜索错误
pub const EVENT_ASYNC_SEARCH_ERROR: &str = "async-search-error";

// ============================================================================
// 任务相关事件
// ============================================================================

/// 任务进度更新
pub const EVENT_TASK_UPDATE: &str = "task-update";
/// 导入完成
pub const EVENT_IMPORT_COMPLETE: &str = "import-complete";

// ============================================================================
// 文件监控相关事件
// ============================================================================

/// 文件变化通知
pub const EVENT_FILE_CHANGED: &str = "file-changed";
/// 新日志条目通知
pub const EVENT_NEW_LOGS: &str = "new-logs";

// ============================================================================
// 系统相关事件（通常不转发到前端）
// ============================================================================

/// 系统错误
pub const EVENT_SYSTEM_ERROR: &str = "system-error";
/// 系统警告
pub const EVENT_SYSTEM_WARNING: &str = "system-warning";
/// 系统信息
pub const EVENT_SYSTEM_INFO: &str = "system-info";

// ============================================================================
// 分页搜索相关事件
// ============================================================================

/// 分页搜索结果
pub const EVENT_PAGED_SEARCH_RESULTS: &str = "paged-search-results";
/// 分页搜索元数据
pub const EVENT_PAGED_SEARCH_META: &str = "paged-search-meta";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_names_are_snake_case() {
        // 验证所有事件名称使用 kebab-case（前端常用）
        let events = [
            EVENT_SEARCH_START,
            EVENT_SEARCH_PROGRESS,
            EVENT_SEARCH_RESULTS,
            EVENT_SEARCH_SUMMARY,
            EVENT_SEARCH_COMPLETE,
            EVENT_SEARCH_ERROR,
            EVENT_SEARCH_CANCELLED,
            EVENT_ASYNC_SEARCH_START,
            EVENT_ASYNC_SEARCH_PROGRESS,
            EVENT_ASYNC_SEARCH_RESULTS,
            EVENT_ASYNC_SEARCH_COMPLETE,
            EVENT_ASYNC_SEARCH_ERROR,
            EVENT_TASK_UPDATE,
            EVENT_IMPORT_COMPLETE,
            EVENT_FILE_CHANGED,
            EVENT_NEW_LOGS,
            EVENT_SYSTEM_ERROR,
            EVENT_SYSTEM_WARNING,
            EVENT_SYSTEM_INFO,
            EVENT_PAGED_SEARCH_RESULTS,
            EVENT_PAGED_SEARCH_META,
        ];

        for event in &events {
            // 验证使用 kebab-case（小写字母和连字符）
            assert!(
                event.chars().all(|c| c.is_lowercase() || c == '-'),
                "Event name '{}' should be kebab-case",
                event
            );
            // 验证不包含下划线
            assert!(
                !event.contains('_'),
                "Event name '{}' should use hyphens, not underscores",
                event
            );
        }
    }

    #[test]
    fn test_event_name_prefix_consistency() {
        // 搜索事件前缀一致性
        assert!(EVENT_SEARCH_START.starts_with("search-"));
        assert!(EVENT_SEARCH_PROGRESS.starts_with("search-"));
        assert!(EVENT_SEARCH_RESULTS.starts_with("search-"));
        
        // 异步搜索事件前缀一致性
        assert!(EVENT_ASYNC_SEARCH_START.starts_with("async-search-"));
        assert!(EVENT_ASYNC_SEARCH_PROGRESS.starts_with("async-search-"));
        
        // 系统事件前缀一致性
        assert!(EVENT_SYSTEM_ERROR.starts_with("system-"));
        assert!(EVENT_SYSTEM_WARNING.starts_with("system-"));
        assert!(EVENT_SYSTEM_INFO.starts_with("system-"));
    }
}
