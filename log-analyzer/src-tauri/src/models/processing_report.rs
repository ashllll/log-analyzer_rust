//! 处理报告模型
//!
//! 定义文件处理过程中的报告和错误分类模型：
//! - 可恢复/不可恢复错误
//! - 处理进度实时推送
//! - 详细的错误/警告信息

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    /// 信息（不需要处理）
    Info,
    /// 警告（可继续处理）
    Warning,
    /// 错误（可恢复）
    Error,
    /// 致命错误（不可恢复）
    Fatal,
}

impl ErrorSeverity {
    /// 检查是否为致命错误
    pub fn is_fatal(&self) -> bool {
        *self == ErrorSeverity::Fatal
    }

    /// 检查是否可恢复
    pub fn is_recoverable(&self) -> bool {
        !self.is_fatal()
    }
}

/// 错误分类
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 文件I/O错误
    IoError,
    /// 压缩包处理错误
    ArchiveError,
    /// 内存不足
    OutOfMemory,
    /// 磁盘空间不足
    DiskSpace,
    /// 权限错误
    Permission,
    /// 格式错误
    Format,
    /// 安全风险
    Security,
    /// 超时
    Timeout,
    /// 其他错误
    Other(String),
}

/// 处理错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// 错误ID
    pub error_id: String,

    /// 错误严重程度
    pub severity: ErrorSeverity,

    /// 错误分类
    pub category: ErrorCategory,

    /// 错误消息
    pub message: String,

    /// 文件路径（如果适用）
    pub file_path: Option<String>,

    /// 嵌套深度（如果适用）
    pub nesting_depth: Option<usize>,

    /// 时间戳
    pub timestamp: u64,

    /// 堆栈跟踪（可选）
    pub stack_trace: Option<String>,
}

impl ProcessingError {
    /// 创建新的处理错误
    pub fn new(severity: ErrorSeverity, category: ErrorCategory, message: String) -> Self {
        Self {
            error_id: uuid::Uuid::new_v4().to_string(),
            severity,
            category,
            message,
            file_path: None,
            nesting_depth: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            stack_trace: None,
        }
    }

    /// 设置文件路径
    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    /// 设置嵌套深度
    pub fn with_nesting_depth(mut self, depth: usize) -> Self {
        self.nesting_depth = Some(depth);
        self
    }

    /// 设置堆栈跟踪
    pub fn with_stack_trace(mut self, trace: String) -> Self {
        self.stack_trace = Some(trace);
        self
    }

    /// 检查是否可恢复
    pub fn is_recoverable(&self) -> bool {
        self.severity.is_recoverable()
    }
}

/// 文件处理状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    /// 等待处理
    Pending,
    /// 处理中
    Processing,
    /// 已完成
    Completed,
    /// 已跳过
    Skipped,
    /// 失败
    Failed,
}

/// 文件处理统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingStatistics {
    /// 总文件数
    pub total_files: usize,

    /// 已处理文件数
    pub processed_files: usize,

    /// 成功处理数
    pub successful_files: usize,

    /// 跳过文件数
    pub skipped_files: usize,

    /// 失败文件数
    pub failed_files: usize,

    /// 总大小（字节）
    pub total_size: u64,

    /// 已处理大小（字节）
    pub processed_size: u64,

    /// 开始时间戳
    pub start_timestamp: u64,

    /// 结束时间戳（如果已完成）
    pub end_timestamp: Option<u64>,
}

impl ProcessingStatistics {
    /// 计算处理进度百分比
    pub fn progress_percentage(&self) -> f64 {
        if self.total_files == 0 {
            return 100.0;
        }
        (self.processed_files as f64 / self.total_files as f64) * 100.0
    }

    /// 计算平均处理速度（字节/秒）
    pub fn average_speed(&self) -> f64 {
        let elapsed = self.elapsed_seconds();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.processed_size as f64 / elapsed
    }

    /// 计算已用时间（秒）
    pub fn elapsed_seconds(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64;

        let start = self.start_timestamp as f64;
        now - start
    }

    /// 预估剩余时间（秒）
    pub fn estimated_remaining_seconds(&self) -> Option<f64> {
        if self.processed_files == 0 {
            return None;
        }

        let elapsed = self.elapsed_seconds();
        let files_per_second = self.processed_files as f64 / elapsed;
        let remaining_files = self.total_files - self.processed_files;

        if files_per_second == 0.0 {
            return None;
        }

        Some(remaining_files as f64 / files_per_second)
    }

    /// 增加已处理文件数
    pub fn increment_processed(&mut self, size: u64) {
        self.processed_files += 1;
        self.processed_size += size;
    }

    /// 增加成功文件数
    pub fn increment_successful(&mut self) {
        self.successful_files += 1;
    }

    /// 增加跳过文件数
    pub fn increment_skipped(&mut self) {
        self.skipped_files += 1;
    }

    /// 增加失败文件数
    pub fn increment_failed(&mut self) {
        self.failed_files += 1;
    }

    /// 标记为完成
    pub fn mark_completed(&mut self) {
        self.end_timestamp = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }
}

/// 处理报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingReport {
    /// 报告ID
    pub report_id: String,

    /// 任务ID
    pub task_id: String,

    /// 处理状态
    pub status: ProcessingStatus,

    /// 统计信息
    pub statistics: ProcessingStatistics,

    /// 错误列表（按时间排序）
    pub errors: Vec<ProcessingError>,

    /// 警告列表（按时间排序）
    pub warnings: Vec<ProcessingError>,

    /// 自定义元数据
    pub metadata: HashMap<String, String>,
}

impl ProcessingReport {
    /// 创建新的处理报告
    pub fn new(task_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            report_id: uuid::Uuid::new_v4().to_string(),
            task_id,
            status: ProcessingStatus::Pending,
            statistics: ProcessingStatistics {
                start_timestamp: now,
                ..Default::default()
            },
            errors: Vec::new(),
            warnings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ProcessingError) {
        if error.severity.is_fatal() {
            self.errors.push(error);
        } else {
            self.warnings.push(error);
        }
    }

    /// 添加多个错误
    pub fn add_errors(&mut self, errors: Vec<ProcessingError>) {
        for error in errors {
            self.add_error(error);
        }
    }

    /// 检查是否有致命错误
    pub fn has_fatal_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity.is_fatal())
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 检查是否有警告
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// 获取错误总数
    pub fn total_errors(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// 设置状态
    pub fn set_status(&mut self, status: ProcessingStatus) {
        self.status = status.clone();
        if matches!(
            status,
            ProcessingStatus::Completed | ProcessingStatus::Failed
        ) {
            self.statistics.mark_completed();
        }
    }

    /// 设置元数据
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// 生成摘要
    pub fn summary(&self) -> ProcessingReportSummary {
        ProcessingReportSummary {
            report_id: self.report_id.clone(),
            task_id: self.task_id.clone(),
            status: self.status.clone(),
            total_files: self.statistics.total_files,
            processed_files: self.statistics.processed_files,
            failed_files: self.statistics.failed_files,
            skipped_files: self.statistics.skipped_files,
            total_errors: self.total_errors(),
            fatal_errors: self.errors.len(),
            warnings: self.warnings.len(),
            progress_percentage: self.statistics.progress_percentage(),
        }
    }
}

/// 处理报告摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingReportSummary {
    pub report_id: String,
    pub task_id: String,
    pub status: ProcessingStatus,
    pub total_files: usize,
    pub processed_files: usize,
    pub failed_files: usize,
    pub skipped_files: usize,
    pub total_errors: usize,
    pub fatal_errors: usize,
    pub warnings: usize,
    pub progress_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        assert!(!ErrorSeverity::Info.is_fatal());
        assert!(!ErrorSeverity::Warning.is_fatal());
        assert!(!ErrorSeverity::Error.is_fatal());
        assert!(ErrorSeverity::Fatal.is_fatal());

        assert!(ErrorSeverity::Info.is_recoverable());
        assert!(ErrorSeverity::Warning.is_recoverable());
        assert!(ErrorSeverity::Error.is_recoverable());
        assert!(!ErrorSeverity::Fatal.is_recoverable());
    }

    #[test]
    fn test_processing_error_creation() {
        let error = ProcessingError::new(
            ErrorSeverity::Error,
            ErrorCategory::IoError,
            "Failed to read file".to_string(),
        )
        .with_file_path("/test/path.log".to_string())
        .with_nesting_depth(2);

        assert_eq!(error.severity, ErrorSeverity::Error);
        assert_eq!(error.message, "Failed to read file");
        assert_eq!(error.file_path, Some("/test/path.log".to_string()));
        assert_eq!(error.nesting_depth, Some(2));
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_processing_statistics() {
        let mut stats = ProcessingStatistics {
            total_files: 100,
            start_timestamp: 0,
            ..Default::default()
        };

        stats.increment_processed(1024);
        stats.increment_processed(2048);

        assert_eq!(stats.processed_files, 2);
        assert_eq!(stats.processed_size, 3072);
        assert_eq!(stats.progress_percentage(), 2.0);
    }

    #[test]
    fn test_processing_report() {
        let mut report = ProcessingReport::new("task-123".to_string());

        assert_eq!(report.task_id, "task-123");
        assert_eq!(report.status, ProcessingStatus::Pending);
        assert!(!report.has_errors());
        assert!(!report.has_warnings());

        // 添加错误
        let error = ProcessingError::new(
            ErrorSeverity::Warning,
            ErrorCategory::Format,
            "Invalid format".to_string(),
        );
        report.add_error(error);

        assert!(report.has_warnings());
        assert_eq!(report.total_errors(), 1);

        // 设置完成
        report.set_status(ProcessingStatus::Completed);
        assert_eq!(report.status, ProcessingStatus::Completed);
        assert!(report.statistics.end_timestamp.is_some());
    }

    #[test]
    fn test_report_summary() {
        let mut report = ProcessingReport::new("task-123".to_string());
        report.statistics.total_files = 100;
        report.statistics.processed_files = 50;
        report.statistics.failed_files = 2;
        report.statistics.skipped_files = 1;

        let summary = report.summary();
        assert_eq!(summary.total_files, 100);
        assert_eq!(summary.processed_files, 50);
        assert_eq!(summary.failed_files, 2);
        assert_eq!(summary.skipped_files, 1);
        assert_eq!(summary.progress_percentage, 50.0);
    }

    #[test]
    fn test_average_speed_calculation() {
        let stats = ProcessingStatistics {
            total_files: 10,
            processed_files: 5,
            processed_size: 1024 * 1024, // 1MB
            start_timestamp: 0,
            ..Default::default()
        };

        // 平均速度应该很高，因为开始时间戳为0
        let speed = stats.average_speed();
        assert!(speed > 0.0);
    }

    #[test]
    fn test_estimated_remaining_time() {
        let mut stats = ProcessingStatistics {
            total_files: 100,
            processed_files: 50,
            start_timestamp: 0,
            ..Default::default()
        };

        // 由于开始时间戳为0，实际已用时间很大，预估剩余时间可能不准确
        // 但应该返回Some值
        let remaining = stats.estimated_remaining_seconds();
        assert!(remaining.is_some());

        // 未处理文件时返回None
        stats.processed_files = 0;
        assert!(stats.estimated_remaining_seconds().is_none());
    }
}
