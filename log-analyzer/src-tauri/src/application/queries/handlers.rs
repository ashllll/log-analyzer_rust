//! CQRS 查询处理器
//!
//! 定义查询处理器的接口和基础实现

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;

use super::queries::Query;

/// 查询处理器 trait
///
/// 每种查询类型都需要实现对应的处理器
#[async_trait]
pub trait QueryHandler<Q: Query + 'static>: Send + Sync {
    /// 处理查询
    async fn handle(&self, query: Q) -> QueryResult<Box<dyn Any + Send>>;
}

/// 查询结果
pub type QueryResult<T> = Result<T, QueryError>;

/// 查询错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum QueryError {
    #[error("查询执行失败: {0}")]
    ExecutionFailed(String),

    #[error("查询超时")]
    Timeout,

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("无效参数: {0}")]
    InvalidArgument(String),

    #[error("权限不足")]
    Unauthorized,

    #[error("系统繁忙，请稍后重试")]
    TooManyRequests,
}

/// 工作区查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQueryResult {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub file_count: usize,
    pub total_size: u64,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryResult {
    pub results: Vec<SearchResultItem>,
    pub total_count: usize,
    pub has_more: bool,
    pub execution_time_ms: u64,
}

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub line_number: usize,
    pub content: String,
    pub source_file: String,
    pub highlights: Vec<(usize, usize)>,
    pub score: f32,
}

/// 关键词组查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordsQueryResult {
    pub groups: Vec<KeywordGroupItem>,
    pub total_count: usize,
}

/// 关键词组项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordGroupItem {
    pub id: String,
    pub name: String,
    pub keywords: Vec<String>,
    pub color: String,
}

/// 任务状态查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusQueryResult {
    pub tasks: Vec<TaskItem>,
    pub total_count: usize,
}

/// 任务项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub error: Option<String>,
}

// ==================== 基础查询处理器实现 ====================

/// 获取工作区查询处理器
pub struct GetWorkspaceQueryHandler;

impl GetWorkspaceQueryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetWorkspaceQueryHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取关键词组查询处理器
pub struct GetKeywordsQueryHandler;

impl GetKeywordsQueryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetKeywordsQueryHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取任务状态查询处理器
pub struct GetTaskStatusQueryHandler;

impl GetTaskStatusQueryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetTaskStatusQueryHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_error_display() {
        let error = QueryError::NotFound("workspace-123".to_string());
        assert!(error.to_string().contains("workspace-123"));

        let error = QueryError::Timeout;
        assert!(error.to_string().contains("超时"));

        let error = QueryError::Unauthorized;
        assert!(error.to_string().contains("权限不足"));
    }

    #[test]
    fn test_workspace_query_result() {
        let result = WorkspaceQueryResult {
            id: "ws-1".to_string(),
            name: "Test Workspace".to_string(),
            path: "/path/to/workspace".to_string(),
            status: "READY".to_string(),
            file_count: 100,
            total_size: 1024000,
        };

        assert_eq!(result.id, "ws-1");
        assert_eq!(result.file_count, 100);
    }

    #[test]
    fn test_search_query_result() {
        let result = SearchQueryResult {
            results: vec![SearchResultItem {
                line_number: 1,
                content: "error message".to_string(),
                source_file: "app.log".to_string(),
                highlights: vec![(0, 5)],
                score: 95.0,
            }],
            total_count: 1,
            has_more: false,
            execution_time_ms: 50,
        };

        assert_eq!(result.results.len(), 1);
        assert!(!result.has_more);
    }

    #[test]
    fn test_keywords_query_result() {
        let result = KeywordsQueryResult {
            groups: vec![KeywordGroupItem {
                id: "g1".to_string(),
                name: "Errors".to_string(),
                keywords: vec!["error".to_string(), "exception".to_string()],
                color: "#FF0000".to_string(),
            }],
            total_count: 1,
        };

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].keywords.len(), 2);
    }

    #[test]
    fn test_task_status_query_result() {
        let result = TaskStatusQueryResult {
            tasks: vec![TaskItem {
                id: "task-1".to_string(),
                task_type: "import".to_string(),
                status: "running".to_string(),
                progress: 0.5,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: None,
                error: None,
            }],
            total_count: 1,
        };

        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].progress, 0.5);
    }

    #[test]
    fn test_query_handler_creation() {
        let _handler = GetWorkspaceQueryHandler::new();
        let _handler = GetKeywordsQueryHandler::new();
        let _handler = GetTaskStatusQueryHandler::new();
    }
}
