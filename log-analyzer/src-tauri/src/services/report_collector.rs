//! 报告收集器
//!
//! 收集和管理处理过程中的报告：
//! - 实时进度推送
//! - 错误聚合和分类
//! - 报告持久化

use crate::models::processing_report::{
    ErrorCategory, ErrorSeverity, ProcessingError, ProcessingReport, ProcessingReportSummary,
    ProcessingStatistics, ProcessingStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 报告收集器
pub struct ReportCollector {
    /// 存储所有报告
    reports: Arc<RwLock<HashMap<String, ProcessingReport>>>,

    /// 进度回调函数
    progress_callbacks: Arc<RwLock<Vec<Box<dyn Fn(ProcessingReportSummary) + Send + Sync>>>>,
}

impl ReportCollector {
    /// 创建新的报告收集器
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            progress_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建新报告
    pub async fn create_report(&self, task_id: String) -> ProcessingReport {
        let report = ProcessingReport::new(task_id.clone());
        let report_id = report.report_id.clone();

        let mut reports = self.reports.write().await;
        reports.insert(report_id, report.clone());

        // 通知进度
        self.notify_progress(report.summary()).await;

        report
    }

    /// 获取报告
    pub async fn get_report(&self, report_id: &str) -> Option<ProcessingReport> {
        let reports = self.reports.read().await;
        reports.get(report_id).cloned()
    }

    /// 获取任务报告（通过任务ID）
    pub async fn get_report_by_task(&self, task_id: &str) -> Option<ProcessingReport> {
        let reports = self.reports.read().await;
        reports.values().find(|r| r.task_id == task_id).cloned()
    }

    /// 更新报告
    pub async fn update_report<F>(&self, report_id: &str, updater: F) -> bool
    where
        F: FnOnce(&mut ProcessingReport),
    {
        let mut reports = self.reports.write().await;
        if let Some(report) = reports.get_mut(report_id) {
            updater(report);
            // 通知进度
            self.notify_progress(report.summary()).await;
            true
        } else {
            false
        }
    }

    /// 添加错误到报告
    pub async fn add_error(&self, report_id: &str, error: ProcessingError) {
        self.update_report(report_id, |report| {
            report.add_error(error);
        })
        .await;
    }

    /// 添加多个错误到报告
    pub async fn add_errors(&self, report_id: &str, errors: Vec<ProcessingError>) {
        self.update_report(report_id, |report| {
            report.add_errors(errors);
        })
        .await;
    }

    /// 更新处理状态
    pub async fn update_status(&self, report_id: &str, status: ProcessingStatus) {
        self.update_report(report_id, |report| {
            report.set_status(status);
        })
        .await;
    }

    /// 更新统计信息
    pub async fn update_statistics<F>(&self, report_id: &str, updater: F)
    where
        F: FnOnce(&mut ProcessingStatistics),
    {
        self.update_report(report_id, |report| {
            updater(&mut report.statistics);
        })
        .await;
    }

    /// 增加已处理文件
    pub async fn increment_processed(&self, report_id: &str, file_size: u64) {
        self.update_statistics(report_id, |stats| {
            stats.increment_processed(file_size);
        })
        .await;
    }

    /// 增加成功文件
    pub async fn increment_successful(&self, report_id: &str) {
        self.update_statistics(report_id, |stats| {
            stats.increment_successful();
        })
        .await;
    }

    /// 增加跳过文件
    pub async fn increment_skipped(&self, report_id: &str) {
        self.update_statistics(report_id, |stats| {
            stats.increment_skipped();
        })
        .await;
    }

    /// 增加失败文件
    pub async fn increment_failed(&self, report_id: &str) {
        self.update_statistics(report_id, |stats| {
            stats.increment_failed();
        })
        .await;
    }

    /// 注册进度回调
    pub async fn register_progress_callback<F>(&self, callback: F)
    where
        F: Fn(ProcessingReportSummary) + Send + Sync + 'static,
    {
        let mut callbacks = self.progress_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// 通知进度更新
    async fn notify_progress(&self, summary: ProcessingReportSummary) {
        let callbacks = self.progress_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(summary.clone());
        }
    }

    /// 获取所有报告摘要
    pub async fn get_all_summaries(&self) -> Vec<ProcessingReportSummary> {
        let reports = self.reports.read().await;
        reports.values().map(|r| r.summary()).collect()
    }

    /// 删除报告
    pub async fn remove_report(&self, report_id: &str) -> bool {
        let mut reports = self.reports.write().await;
        reports.remove(report_id).is_some()
    }

    /// 清理旧报告
    pub async fn cleanup_old_reports(&self, max_age_seconds: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut reports = self.reports.write().await;
        let initial_count = reports.len();

        reports.retain(|_, report| {
            if let Some(end_timestamp) = report.statistics.end_timestamp {
                now - end_timestamp < max_age_seconds
            } else {
                // 保留仍在处理的报告
                true
            }
        });

        initial_count - reports.len()
    }
}

impl Default for ReportCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建便捷的错误
pub fn create_error(
    severity: ErrorSeverity,
    category: ErrorCategory,
    message: impl Into<String>,
) -> ProcessingError {
    ProcessingError::new(severity, category, message.into())
}

/// 创建文件I/O错误
pub fn create_io_error(
    message: impl Into<String>,
    file_path: impl Into<String>,
) -> ProcessingError {
    create_error(ErrorSeverity::Error, ErrorCategory::IoError, message)
        .with_file_path(file_path.into())
}

/// 创建压缩包错误
pub fn create_archive_error(
    message: impl Into<String>,
    file_path: impl Into<String>,
    nesting_depth: usize,
) -> ProcessingError {
    create_error(ErrorSeverity::Error, ErrorCategory::ArchiveError, message)
        .with_file_path(file_path.into())
        .with_nesting_depth(nesting_depth)
}

/// 创建安全风险错误
pub fn create_security_error(message: impl Into<String>) -> ProcessingError {
    create_error(ErrorSeverity::Fatal, ErrorCategory::Security, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_report() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;

        assert_eq!(report.task_id, "task-123");
        assert_eq!(report.status, ProcessingStatus::Pending);

        // 可以通过report_id获取
        let retrieved = collector.get_report(&report.report_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id, "task-123");
    }

    #[tokio::test]
    async fn test_update_report() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;
        let report_id = report.report_id;

        // 更新状态
        collector
            .update_status(&report_id, ProcessingStatus::Processing)
            .await;

        let retrieved = collector.get_report(&report_id).await.unwrap();
        assert_eq!(retrieved.status, ProcessingStatus::Processing);
    }

    #[tokio::test]
    async fn test_add_error() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;
        let report_id = report.report_id;

        // 添加错误（非致命错误会被归类为warnings）
        let error = create_io_error("Failed to read", "/test/path.log");
        collector.add_error(&report_id, error).await;

        let retrieved = collector.get_report(&report_id).await.unwrap();
        // 非致命错误被归类为warnings
        assert!(retrieved.has_warnings());
    }

    #[tokio::test]
    async fn test_increment_statistics() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;
        let report_id = report.report_id;

        // 增加处理计数
        collector.increment_processed(&report_id, 1024).await;
        collector.increment_successful(&report_id).await;

        let retrieved = collector.get_report(&report_id).await.unwrap();
        assert_eq!(retrieved.statistics.processed_files, 1);
        assert_eq!(retrieved.statistics.successful_files, 1);
        assert_eq!(retrieved.statistics.processed_size, 1024);
    }

    #[tokio::test]
    async fn test_progress_callback() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;
        let report_id = report.report_id.clone();

        let callback_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();

        collector
            .register_progress_callback(move |_summary| {
                callback_called_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            })
            .await;

        // 更新状态应该触发回调
        collector
            .update_status(&report_id, ProcessingStatus::Processing)
            .await;

        // 给回调一点时间执行
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(callback_called.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_get_report_by_task() {
        let collector = ReportCollector::new();
        let report = collector.create_report("task-123".to_string()).await;

        let retrieved = collector.get_report_by_task("task-123").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().report_id, report.report_id);
    }

    #[tokio::test]
    async fn test_cleanup_old_reports() {
        let collector = ReportCollector::new();

        // 创建一个已完成的报告
        let report = collector.create_report("task-123".to_string()).await;
        collector
            .update_status(&report.report_id, ProcessingStatus::Completed)
            .await;

        // 手动设置结束时间为很久以前
        let mut reports = collector.reports.write().await;
        if let Some(report) = reports.get_mut(&report.report_id) {
            let old_timestamp = 1000; // 很久以前
            report.statistics.end_timestamp = Some(old_timestamp);
        }
        drop(reports);

        // 清理超过1秒的旧报告
        let removed = collector.cleanup_old_reports(1).await;
        assert_eq!(removed, 1);

        // 报告应该已被删除
        let retrieved = collector.get_report(&report.report_id).await;
        assert!(retrieved.is_none());
    }
}
