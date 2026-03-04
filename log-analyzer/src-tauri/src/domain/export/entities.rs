//! 导出领域实体
//!
//! 定义导出相关的实体对象

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

use super::value_objects::{ExportFormat, ExportOptions};

/// 导出任务实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTask {
    /// 任务 ID
    pub id: String,
    /// 导出选项
    pub options: ExportOptions,
    /// 关联的搜索会话 ID
    pub search_session_id: Option<String>,
    /// 任务状态
    pub status: ExportTaskStatus,
    /// 导出的结果数量
    pub result_count: usize,
    /// 输出文件大小（字节）
    pub output_size: u64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 完成时间
    pub finished_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
}

impl ExportTask {
    /// 创建新的导出任务
    pub fn new(options: ExportOptions) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            options,
            search_session_id: None,
            status: ExportTaskStatus::Pending,
            result_count: 0,
            output_size: 0,
            created_at: Utc::now(),
            finished_at: None,
            error: None,
        }
    }

    /// 关联搜索会话
    pub fn with_search_session(mut self, session_id: String) -> Self {
        self.search_session_id = Some(session_id);
        self
    }

    /// 开始导出
    pub fn start(&mut self) {
        self.status = ExportTaskStatus::Running;
    }

    /// 更新进度
    pub fn update_progress(&mut self, result_count: usize) {
        self.result_count = result_count;
    }

    /// 完成导出
    pub fn complete(&mut self, result_count: usize, output_size: u64) {
        self.status = ExportTaskStatus::Completed;
        self.result_count = result_count;
        self.output_size = output_size;
        self.finished_at = Some(Utc::now());
    }

    /// 导出失败
    pub fn fail(&mut self, error: String) {
        self.status = ExportTaskStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
    }

    /// 取消导出
    pub fn cancel(&mut self) {
        self.status = ExportTaskStatus::Cancelled;
        self.finished_at = Some(Utc::now());
    }

    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ExportTaskStatus::Completed | ExportTaskStatus::Failed | ExportTaskStatus::Cancelled
        )
    }

    /// 获取耗时（毫秒）
    pub fn duration_ms(&self) -> i64 {
        let end = self.finished_at.unwrap_or_else(Utc::now);
        (end - self.created_at).num_milliseconds()
    }

    /// 获取输出路径
    pub fn output_path(&self) -> Option<&PathBuf> {
        self.options.output_path.as_ref()
    }

    /// 获取导出格式
    pub fn format(&self) -> ExportFormat {
        self.options.format
    }
}

impl fmt::Display for ExportTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExportTask[id={}, format={}, status={}, results={}]",
            &self.id[..8],
            self.options.format,
            self.status,
            self.result_count
        )
    }
}

/// 导出任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportTaskStatus {
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

impl fmt::Display for ExportTaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportTaskStatus::Pending => write!(f, "pending"),
            ExportTaskStatus::Running => write!(f, "running"),
            ExportTaskStatus::Completed => write!(f, "completed"),
            ExportTaskStatus::Failed => write!(f, "failed"),
            ExportTaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 导出结果
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// 任务 ID
    pub task_id: String,
    /// 输出文件路径
    pub output_path: PathBuf,
    /// 文件大小
    pub size: u64,
    /// 导出的记录数
    pub record_count: usize,
    /// 导出耗时（毫秒）
    pub duration_ms: i64,
}

impl ExportResult {
    /// 创建导出结果
    pub fn new(
        task_id: String,
        output_path: PathBuf,
        size: u64,
        record_count: usize,
        duration_ms: i64,
    ) -> Self {
        Self {
            task_id,
            output_path,
            size,
            record_count,
            duration_ms,
        }
    }

    /// 获取格式化的文件大小
    pub fn formatted_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }

    /// 获取吞吐量（记录/秒）
    pub fn throughput(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        (self.record_count as f64) / (self.duration_ms as f64 / 1000.0)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_task_creation() {
        let options = ExportOptions::json();
        let task = ExportTask::new(options);

        assert_eq!(task.status, ExportTaskStatus::Pending);
        assert_eq!(task.result_count, 0);
        assert!(!task.is_finished());
    }

    #[test]
    fn test_export_task_lifecycle() {
        let options = ExportOptions::csv();
        let mut task = ExportTask::new(options);

        task.start();
        assert_eq!(task.status, ExportTaskStatus::Running);

        task.update_progress(50);
        assert_eq!(task.result_count, 50);

        task.complete(100, 1024);
        assert_eq!(task.status, ExportTaskStatus::Completed);
        assert_eq!(task.result_count, 100);
        assert_eq!(task.output_size, 1024);
        assert!(task.is_finished());
    }

    #[test]
    fn test_export_task_failure() {
        let options = ExportOptions::html();
        let mut task = ExportTask::new(options);

        task.start();
        task.fail("Write error".to_string());

        assert_eq!(task.status, ExportTaskStatus::Failed);
        assert_eq!(task.error, Some("Write error".to_string()));
        assert!(task.is_finished());
    }

    #[test]
    fn test_export_task_with_search_session() {
        let options = ExportOptions::json();
        let task = ExportTask::new(options).with_search_session("session-123".to_string());

        assert_eq!(task.search_session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_export_result_formatted_size() {
        let result = ExportResult::new(
            "task-1".to_string(),
            PathBuf::from("/tmp/export.json"),
            500,
            100,
            1000,
        );
        assert_eq!(result.formatted_size(), "500 B");

        let result = ExportResult::new(
            "task-1".to_string(),
            PathBuf::from("/tmp/export.json"),
            1536,
            100,
            1000,
        );
        assert!(result.formatted_size().contains("KB"));

        let result = ExportResult::new(
            "task-1".to_string(),
            PathBuf::from("/tmp/export.json"),
            1572864,
            100,
            1000,
        );
        assert!(result.formatted_size().contains("MB"));
    }

    #[test]
    fn test_export_result_throughput() {
        let result = ExportResult::new(
            "task-1".to_string(),
            PathBuf::from("/tmp/export.json"),
            1024,
            1000,
            1000,
        );

        // 1000 条记录 / 1 秒 = 1000 条/秒
        assert_eq!(result.throughput(), 1000.0);
    }

    #[test]
    fn test_export_task_format() {
        let options = ExportOptions::csv();
        let task = ExportTask::new(options);

        assert_eq!(task.format(), ExportFormat::Csv);
    }

    #[test]
    fn test_export_task_duration() {
        let options = ExportOptions::json();
        let mut task = ExportTask::new(options);

        task.start();
        task.complete(10, 100);

        assert!(task.duration_ms() >= 0);
    }
}
