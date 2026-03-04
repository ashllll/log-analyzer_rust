//! CQRS 命令处理器
//!
//! 定义命令处理器的接口和基础实现

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;

use super::commands::Command;

/// 命令处理器 trait
///
/// 每种命令类型都需要实现对应的处理器
#[async_trait]
pub trait CommandHandler<C: Command + 'static>: Send + Sync {
    /// 处理命令
    async fn handle(&self, command: C) -> CommandResult<Box<dyn Any + Send>>;
}

/// 命令结果
pub type CommandResult<T> = Result<T, CommandError>;

/// 命令错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum CommandError {
    #[error("命令执行失败: {0}")]
    ExecutionFailed(String),

    #[error("验证失败: {0}")]
    ValidationFailed(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("冲突: {0}")]
    Conflict(String),

    #[error("权限不足")]
    Unauthorized,

    #[error("并发冲突")]
    ConcurrencyConflict,

    #[error("操作超时")]
    Timeout,
}

/// 命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 聚合根 ID
    pub aggregate_id: String,
    /// 新版本号
    pub version: u64,
    /// 产生的事件数量
    pub events_count: usize,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 消息
    pub message: Option<String>,
}

impl CommandExecutionResult {
    pub fn success(aggregate_id: String, version: u64) -> Self {
        Self {
            success: true,
            aggregate_id,
            version,
            events_count: 0,
            execution_time_ms: 0,
            message: None,
        }
    }

    pub fn with_events(mut self, count: usize) -> Self {
        self.events_count = count;
        self
    }

    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// 创建工作区结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceResult {
    pub workspace_id: String,
    pub name: String,
    pub path: String,
}

/// 导入文件结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFilesResult {
    pub task_id: String,
    pub files_queued: usize,
    pub estimated_size: u64,
}

/// 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub task_id: String,
    pub output_path: String,
    pub file_size: u64,
    pub record_count: usize,
}

// ==================== 基础命令处理器实现 ====================

/// 创建工作区命令处理器
pub struct CreateWorkspaceCommandHandler;

impl CreateWorkspaceCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CreateWorkspaceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 删除工作区命令处理器
pub struct DeleteWorkspaceCommandHandler;

impl DeleteWorkspaceCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeleteWorkspaceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 保存关键词组命令处理器
pub struct SaveKeywordsCommandHandler;

impl SaveKeywordsCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SaveKeywordsCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 取消任务命令处理器
pub struct CancelTaskCommandHandler;

impl CancelTaskCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CancelTaskCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_display() {
        let error = CommandError::NotFound("workspace-123".to_string());
        assert!(error.to_string().contains("workspace-123"));

        let error = CommandError::ValidationFailed("名称不能为空".to_string());
        assert!(error.to_string().contains("名称不能为空"));

        let error = CommandError::ConcurrencyConflict;
        assert!(error.to_string().contains("并发冲突"));
    }

    #[test]
    fn test_command_execution_result() {
        let result = CommandExecutionResult::success("ws-1".to_string(), 1)
            .with_events(2)
            .with_execution_time(50)
            .with_message("工作区创建成功".to_string());

        assert!(result.success);
        assert_eq!(result.aggregate_id, "ws-1");
        assert_eq!(result.version, 1);
        assert_eq!(result.events_count, 2);
        assert_eq!(result.execution_time_ms, 50);
    }

    #[test]
    fn test_create_workspace_result() {
        let result = CreateWorkspaceResult {
            workspace_id: "ws-1".to_string(),
            name: "Test Workspace".to_string(),
            path: "/path/to/workspace".to_string(),
        };

        assert_eq!(result.workspace_id, "ws-1");
    }

    #[test]
    fn test_import_files_result() {
        let result = ImportFilesResult {
            task_id: "task-1".to_string(),
            files_queued: 10,
            estimated_size: 1024000,
        };

        assert_eq!(result.files_queued, 10);
    }

    #[test]
    fn test_export_result() {
        let result = ExportResult {
            task_id: "task-1".to_string(),
            output_path: "/tmp/export.json".to_string(),
            file_size: 5000,
            record_count: 100,
        };

        assert_eq!(result.record_count, 100);
    }

    #[test]
    fn test_command_handler_creation() {
        let _handler = CreateWorkspaceCommandHandler::new();
        let _handler = DeleteWorkspaceCommandHandler::new();
        let _handler = SaveKeywordsCommandHandler::new();
        let _handler = CancelTaskCommandHandler::new();
    }
}
