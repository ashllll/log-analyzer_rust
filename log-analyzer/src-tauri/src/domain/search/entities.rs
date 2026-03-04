//! 搜索领域实体
//!
//! 定义搜索相关的实体对象

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::value_objects::SearchQuery;

/// 搜索结果实体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    /// 结果 ID
    pub id: String,
    /// 匹配的行号
    pub line_number: usize,
    /// 匹配的文本内容
    pub content: String,
    /// 高亮范围列表 (start, end)
    pub highlights: Vec<(usize, usize)>,
    /// 来源文件路径
    pub source_file: String,
    /// 匹配分数 (0-100)
    pub score: f32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl SearchResult {
    /// 创建新的搜索结果
    pub fn new(line_number: usize, content: String, source_file: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            line_number,
            content,
            highlights: Vec::new(),
            source_file,
            score: 100.0,
            created_at: Utc::now(),
        }
    }

    /// 添加高亮范围
    pub fn add_highlight(&mut self, start: usize, end: usize) {
        if start < end && end <= self.content.len() {
            self.highlights.push((start, end));
        }
    }

    /// 设置匹配分数
    pub fn set_score(&mut self, score: f32) {
        self.score = score.clamp(0.0, 100.0);
    }

    /// 获取高亮后的内容
    pub fn highlighted_content(&self) -> String {
        if self.highlights.is_empty() {
            return self.content.clone();
        }

        let mut result = String::new();
        let mut last_end = 0;

        // 按起始位置排序
        let mut sorted_highlights = self.highlights.clone();
        sorted_highlights.sort_by_key(|h| h.0);

        for (start, end) in sorted_highlights {
            if start > last_end {
                result.push_str(&self.content[last_end..start]);
            }
            result.push_str("**");
            result.push_str(&self.content[start..end]);
            result.push_str("**");
            last_end = end;
        }

        if last_end < self.content.len() {
            result.push_str(&self.content[last_end..]);
        }

        result
    }

    /// 检查是否包含高亮
    pub fn has_highlights(&self) -> bool {
        !self.highlights.is_empty()
    }
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} - {}",
            self.source_file,
            self.line_number,
            if self.content.len() > 100 {
                &self.content[..100]
            } else {
                &self.content
            }
        )
    }
}

/// 搜索会话实体
///
/// 跟踪一次完整搜索过程的状态
#[derive(Debug, Clone)]
pub struct SearchSession {
    /// 会话 ID
    pub id: String,
    /// 搜索查询
    pub query: SearchQuery,
    /// 工作区 ID
    pub workspace_id: String,
    /// 会话状态
    pub status: SearchSessionStatus,
    /// 结果数量
    pub result_count: usize,
    /// 已处理文件数
    pub files_processed: usize,
    /// 总文件数
    pub total_files: usize,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 结束时间
    pub finished_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
}

impl SearchSession {
    /// 创建新的搜索会话
    pub fn new(query: SearchQuery, workspace_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query,
            workspace_id,
            status: SearchSessionStatus::Pending,
            result_count: 0,
            files_processed: 0,
            total_files: 0,
            started_at: Utc::now(),
            finished_at: None,
            error: None,
        }
    }

    /// 开始搜索
    pub fn start(&mut self, total_files: usize) {
        self.status = SearchSessionStatus::Running;
        self.total_files = total_files;
    }

    /// 更新进度
    pub fn update_progress(&mut self, files_processed: usize, result_count: usize) {
        self.files_processed = files_processed;
        self.result_count = result_count;
    }

    /// 完成搜索
    pub fn complete(&mut self, result_count: usize) {
        self.status = SearchSessionStatus::Completed;
        self.result_count = result_count;
        self.finished_at = Some(Utc::now());
    }

    /// 搜索失败
    pub fn fail(&mut self, error: String) {
        self.status = SearchSessionStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
    }

    /// 取消搜索
    pub fn cancel(&mut self) {
        self.status = SearchSessionStatus::Cancelled;
        self.finished_at = Some(Utc::now());
    }

    /// 获取进度百分比
    pub fn progress_percent(&self) -> f32 {
        if self.total_files == 0 {
            return 0.0;
        }
        (self.files_processed as f32 / self.total_files as f32 * 100.0).min(100.0)
    }

    /// 获取耗时（毫秒）
    pub fn duration_ms(&self) -> i64 {
        let end = self.finished_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_milliseconds()
    }

    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            SearchSessionStatus::Completed
                | SearchSessionStatus::Failed
                | SearchSessionStatus::Cancelled
        )
    }
}

/// 搜索会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSessionStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl fmt::Display for SearchSessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchSessionStatus::Pending => write!(f, "pending"),
            SearchSessionStatus::Running => write!(f, "running"),
            SearchSessionStatus::Completed => write!(f, "completed"),
            SearchSessionStatus::Failed => write!(f, "failed"),
            SearchSessionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult::new(
            42,
            "This is an error message".to_string(),
            "app.log".to_string(),
        );

        assert_eq!(result.line_number, 42);
        assert_eq!(result.source_file, "app.log");
        assert_eq!(result.score, 100.0);
        assert!(!result.has_highlights());
    }

    #[test]
    fn test_search_result_highlights() {
        let mut result = SearchResult::new(
            1,
            "error: something went wrong".to_string(),
            "test.log".to_string(),
        );

        result.add_highlight(0, 5); // "error"
        assert!(result.has_highlights());
        assert_eq!(result.highlights.len(), 1);
    }

    #[test]
    fn test_search_result_highlighted_content() {
        let mut result = SearchResult::new(1, "error message".to_string(), "test.log".to_string());

        result.add_highlight(0, 5);
        let highlighted = result.highlighted_content();
        assert_eq!(highlighted, "**error** message");
    }

    #[test]
    fn test_search_result_score_clamping() {
        let mut result = SearchResult::new(1, "test".to_string(), "test.log".to_string());

        result.set_score(150.0);
        assert_eq!(result.score, 100.0);

        result.set_score(-10.0);
        assert_eq!(result.score, 0.0);

        result.set_score(50.0);
        assert_eq!(result.score, 50.0);
    }

    #[test]
    fn test_search_session_creation() {
        let query = SearchQuery::new("error".to_string());
        let session = SearchSession::new(query, "workspace-1".to_string());

        assert_eq!(session.status, SearchSessionStatus::Pending);
        assert_eq!(session.result_count, 0);
        assert!(!session.is_finished());
    }

    #[test]
    fn test_search_session_lifecycle() {
        let query = SearchQuery::new("error".to_string());
        let mut session = SearchSession::new(query, "workspace-1".to_string());

        // 开始
        session.start(100);
        assert_eq!(session.status, SearchSessionStatus::Running);
        assert_eq!(session.total_files, 100);

        // 更新进度
        session.update_progress(50, 25);
        assert_eq!(session.files_processed, 50);
        assert_eq!(session.result_count, 25);
        assert_eq!(session.progress_percent(), 50.0);

        // 完成
        session.complete(42);
        assert_eq!(session.status, SearchSessionStatus::Completed);
        assert_eq!(session.result_count, 42);
        assert!(session.is_finished());
        assert!(session.finished_at.is_some());
    }

    #[test]
    fn test_search_session_failure() {
        let query = SearchQuery::new("error".to_string());
        let mut session = SearchSession::new(query, "workspace-1".to_string());

        session.start(100);
        session.fail("IO error".to_string());

        assert_eq!(session.status, SearchSessionStatus::Failed);
        assert_eq!(session.error, Some("IO error".to_string()));
        assert!(session.is_finished());
    }

    #[test]
    fn test_search_session_cancellation() {
        let query = SearchQuery::new("error".to_string());
        let mut session = SearchSession::new(query, "workspace-1".to_string());

        session.start(100);
        session.cancel();

        assert_eq!(session.status, SearchSessionStatus::Cancelled);
        assert!(session.is_finished());
    }

    #[test]
    fn test_search_session_duration() {
        let query = SearchQuery::new("error".to_string());
        let mut session = SearchSession::new(query, "workspace-1".to_string());

        session.start(10);
        session.update_progress(10, 5);
        session.complete(5);

        assert!(session.duration_ms() >= 0);
    }
}
