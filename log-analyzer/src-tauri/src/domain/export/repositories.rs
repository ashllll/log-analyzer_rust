//! 导出仓储接口
//!
//! 定义导出相关数据的持久化接口

use async_trait::async_trait;
use std::path::Path;

use super::entities::{ExportResult, ExportTask};
use super::value_objects::ExportFormat;

/// 导出仓储接口
#[async_trait]
pub trait ExportRepository: Send + Sync {
    /// 保存导出任务
    async fn save_task(&self, task: &ExportTask) -> Result<(), ExportRepositoryError>;

    /// 获取导出任务
    async fn get_task(&self, id: &str) -> Result<Option<ExportTask>, ExportRepositoryError>;

    /// 获取用户的导出历史
    async fn get_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<ExportTask>, ExportRepositoryError>;

    /// 保存导出文件
    async fn save_file(
        &self,
        task_id: &str,
        data: &[u8],
        path: &Path,
    ) -> Result<ExportResult, ExportRepositoryError>;

    /// 删除导出任务及文件
    async fn delete_task(&self, id: &str) -> Result<(), ExportRepositoryError>;

    /// 清理过期的导出文件
    async fn cleanup_expired(&self, max_age_days: u64) -> Result<usize, ExportRepositoryError>;

    /// 获取存储统计
    async fn get_storage_stats(&self) -> Result<ExportStorageStats, ExportRepositoryError>;
}

/// 导出仓储错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExportRepositoryError {
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("任务未找到: {0}")]
    TaskNotFound(String),

    #[error("文件写入错误: {0}")]
    FileWriteError(String),

    #[error("存储空间不足")]
    StorageFull,

    #[error("权限错误: {0}")]
    PermissionError(String),
}

/// 导出存储统计
#[derive(Debug, Clone)]
pub struct ExportStorageStats {
    /// 总任务数
    pub total_tasks: usize,
    /// 总文件大小（字节）
    pub total_size: u64,
    /// 按格式分组的数量
    pub by_format: std::collections::HashMap<ExportFormat, usize>,
    /// 最旧任务日期
    pub oldest_task_date: Option<chrono::DateTime<chrono::Utc>>,
}

impl ExportStorageStats {
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            total_size: 0,
            by_format: std::collections::HashMap::new(),
            oldest_task_date: None,
        }
    }

    /// 格式化总大小
    pub fn formatted_total_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.total_size >= GB {
            format!("{:.2} GB", self.total_size as f64 / GB as f64)
        } else if self.total_size >= MB {
            format!("{:.2} MB", self.total_size as f64 / MB as f64)
        } else if self.total_size >= KB {
            format!("{:.2} KB", self.total_size as f64 / KB as f64)
        } else {
            format!("{} B", self.total_size)
        }
    }
}

impl Default for ExportStorageStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存导出仓储（用于测试）
pub struct InMemoryExportRepository {
    tasks: std::collections::HashMap<String, ExportTask>,
}

impl InMemoryExportRepository {
    pub fn new() -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
        }
    }
}

impl Default for InMemoryExportRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExportRepository for InMemoryExportRepository {
    async fn save_task(&self, _task: &ExportTask) -> Result<(), ExportRepositoryError> {
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Result<Option<ExportTask>, ExportRepositoryError> {
        Ok(self.tasks.get(id).cloned())
    }

    async fn get_history(
        &self,
        _user_id: &str,
        limit: usize,
    ) -> Result<Vec<ExportTask>, ExportRepositoryError> {
        Ok(self.tasks.values().take(limit).cloned().collect())
    }

    async fn save_file(
        &self,
        task_id: &str,
        data: &[u8],
        path: &Path,
    ) -> Result<ExportResult, ExportRepositoryError> {
        Ok(ExportResult::new(
            task_id.to_string(),
            path.to_path_buf(),
            data.len() as u64,
            0,
            0,
        ))
    }

    async fn delete_task(&self, _id: &str) -> Result<(), ExportRepositoryError> {
        Ok(())
    }

    async fn cleanup_expired(&self, _max_age_days: u64) -> Result<usize, ExportRepositoryError> {
        Ok(0)
    }

    async fn get_storage_stats(&self) -> Result<ExportStorageStats, ExportRepositoryError> {
        Ok(ExportStorageStats::new())
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_storage_stats_new() {
        let stats = ExportStorageStats::new();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_export_storage_stats_formatted_size() {
        let mut stats = ExportStorageStats::new();

        stats.total_size = 500;
        assert_eq!(stats.formatted_total_size(), "500 B");

        stats.total_size = 1536;
        assert!(stats.formatted_total_size().contains("KB"));

        stats.total_size = 1572864;
        assert!(stats.formatted_total_size().contains("MB"));

        stats.total_size = 1610612736;
        assert!(stats.formatted_total_size().contains("GB"));
    }

    #[tokio::test]
    async fn test_in_memory_repository_creation() {
        let repo = InMemoryExportRepository::new();
        let stats = repo.get_storage_stats().await.unwrap();
        assert_eq!(stats.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_in_memory_repository_get_task_empty() {
        let repo = InMemoryExportRepository::new();
        let task = repo.get_task("non-existent").await.unwrap();
        assert!(task.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_get_history() {
        let repo = InMemoryExportRepository::new();
        let history = repo.get_history("user-1", 10).await.unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_export_repository_error_display() {
        let error = ExportRepositoryError::TaskNotFound("test-id".to_string());
        assert!(error.to_string().contains("test-id"));

        let error = ExportRepositoryError::StorageFull;
        assert!(error.to_string().contains("存储空间不足"));
    }
}
