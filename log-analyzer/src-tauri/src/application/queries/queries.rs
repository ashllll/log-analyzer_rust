//! CQRS 查询定义
//!
//! 定义系统中所有的查询类型

use serde::{Deserialize, Serialize};

/// 查询基础 trait
///
/// 所有查询都必须实现此 trait
pub trait Query: Send + Sync {
    /// 查询类型名称
    fn query_type(&self) -> &'static str;
}

/// 获取工作区查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWorkspaceQuery {
    /// 工作区 ID
    pub workspace_id: String,
}

impl GetWorkspaceQuery {
    pub fn new(workspace_id: String) -> Self {
        Self { workspace_id }
    }
}

impl Query for GetWorkspaceQuery {
    fn query_type(&self) -> &'static str {
        "GetWorkspace"
    }
}

/// 搜索日志查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchLogsQuery {
    /// 工作区 ID
    pub workspace_id: String,
    /// 搜索关键词
    pub keywords: Vec<String>,
    /// 是否大小写敏感
    pub case_sensitive: bool,
    /// 是否使用正则表达式
    pub use_regex: bool,
    /// 最大结果数
    pub max_results: usize,
    /// 偏移量（分页）
    pub offset: usize,
}

impl SearchLogsQuery {
    pub fn new(workspace_id: String, keywords: Vec<String>) -> Self {
        Self {
            workspace_id,
            keywords,
            case_sensitive: false,
            use_regex: false,
            max_results: 1000,
            offset: 0,
        }
    }

    pub fn with_case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    pub fn with_regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }

    pub fn with_pagination(mut self, offset: usize, max_results: usize) -> Self {
        self.offset = offset;
        self.max_results = max_results;
        self
    }
}

impl Query for SearchLogsQuery {
    fn query_type(&self) -> &'static str {
        "SearchLogs"
    }
}

/// 获取关键词组查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetKeywordsQuery {
    /// 工作区 ID（可选）
    pub workspace_id: Option<String>,
    /// 是否包含关键词详情
    pub include_details: bool,
}

impl GetKeywordsQuery {
    pub fn new() -> Self {
        Self {
            workspace_id: None,
            include_details: true,
        }
    }

    pub fn for_workspace(mut self, workspace_id: String) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn with_details(mut self, include: bool) -> Self {
        self.include_details = include;
        self
    }
}

impl Default for GetKeywordsQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl Query for GetKeywordsQuery {
    fn query_type(&self) -> &'static str {
        "GetKeywords"
    }
}

/// 获取任务状态查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskStatusQuery {
    /// 任务 ID（可选，不提供则返回所有任务）
    pub task_id: Option<String>,
    /// 任务类型过滤
    pub task_type: Option<String>,
    /// 状态过滤
    pub status_filter: Option<String>,
    /// 最大返回数量
    pub limit: usize,
}

impl GetTaskStatusQuery {
    pub fn new() -> Self {
        Self {
            task_id: None,
            task_type: None,
            status_filter: None,
            limit: 100,
        }
    }

    pub fn for_task(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn with_type_filter(mut self, task_type: String) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn with_status_filter(mut self, status: String) -> Self {
        self.status_filter = Some(status);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl Default for GetTaskStatusQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl Query for GetTaskStatusQuery {
    fn query_type(&self) -> &'static str {
        "GetTaskStatus"
    }
}

/// 获取性能指标查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPerformanceMetricsQuery {
    /// 工作区 ID（可选）
    pub workspace_id: Option<String>,
    /// 是否包含详细信息
    pub detailed: bool,
}

impl GetPerformanceMetricsQuery {
    pub fn new() -> Self {
        Self {
            workspace_id: None,
            detailed: false,
        }
    }

    pub fn for_workspace(mut self, workspace_id: String) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn with_details(mut self, detailed: bool) -> Self {
        self.detailed = detailed;
        self
    }
}

impl Default for GetPerformanceMetricsQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl Query for GetPerformanceMetricsQuery {
    fn query_type(&self) -> &'static str {
        "GetPerformanceMetrics"
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_workspace_query() {
        let query = GetWorkspaceQuery::new("ws-123".to_string());
        assert_eq!(query.workspace_id, "ws-123");
        assert_eq!(query.query_type(), "GetWorkspace");
    }

    #[test]
    fn test_search_logs_query() {
        let query = SearchLogsQuery::new("ws-1".to_string(), vec!["error".to_string()])
            .with_case_sensitive(true)
            .with_regex(false)
            .with_pagination(10, 100);

        assert_eq!(query.workspace_id, "ws-1");
        assert!(query.case_sensitive);
        assert!(!query.use_regex);
        assert_eq!(query.offset, 10);
        assert_eq!(query.max_results, 100);
        assert_eq!(query.query_type(), "SearchLogs");
    }

    #[test]
    fn test_get_keywords_query() {
        let query = GetKeywordsQuery::new()
            .for_workspace("ws-1".to_string())
            .with_details(false);

        assert_eq!(query.workspace_id, Some("ws-1".to_string()));
        assert!(!query.include_details);
        assert_eq!(query.query_type(), "GetKeywords");
    }

    #[test]
    fn test_get_task_status_query() {
        let query = GetTaskStatusQuery::new()
            .with_type_filter("import".to_string())
            .with_status_filter("running".to_string())
            .with_limit(50);

        assert_eq!(query.task_type, Some("import".to_string()));
        assert_eq!(query.status_filter, Some("running".to_string()));
        assert_eq!(query.limit, 50);
        assert_eq!(query.query_type(), "GetTaskStatus");
    }

    #[test]
    fn test_get_performance_metrics_query() {
        let query = GetPerformanceMetricsQuery::new()
            .for_workspace("ws-1".to_string())
            .with_details(true);

        assert_eq!(query.workspace_id, Some("ws-1".to_string()));
        assert!(query.detailed);
        assert_eq!(query.query_type(), "GetPerformanceMetrics");
    }

    #[test]
    fn test_query_serialization() {
        let query = SearchLogsQuery::new("ws-1".to_string(), vec!["error".to_string()]);
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("ws-1"));
        assert!(json.contains("error"));

        let deserialized: SearchLogsQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.workspace_id, query.workspace_id);
    }
}
