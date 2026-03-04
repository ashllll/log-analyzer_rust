//! CQRS 命令定义
//!
//! 定义系统中所有的命令类型
//!
//! 命令表示系统中会改变状态的操作

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 命令基础 trait
///
/// 所有命令都必须实现此 trait
pub trait Command: Send + Sync {
    /// 命令类型名称
    fn command_type(&self) -> &'static str;

    /// 聚合根 ID（可选）
    fn aggregate_id(&self) -> Option<&str> {
        None
    }

    /// 期望版本（用于乐观锁）
    fn expected_version(&self) -> Option<u64> {
        None
    }
}

/// 创建工作区命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceCommand {
    /// 工作区名称
    pub name: String,
    /// 工作区路径
    pub path: PathBuf,
    /// 描述（可选）
    pub description: Option<String>,
}

impl CreateWorkspaceCommand {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

impl Command for CreateWorkspaceCommand {
    fn command_type(&self) -> &'static str {
        "CreateWorkspace"
    }
}

/// 导入文件命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFilesCommand {
    /// 工作区 ID
    pub workspace_id: String,
    /// 文件路径列表
    pub file_paths: Vec<PathBuf>,
    /// 是否递归导入
    pub recursive: bool,
    /// 是否覆盖已存在文件
    pub overwrite: bool,
}

impl ImportFilesCommand {
    pub fn new(workspace_id: String, file_paths: Vec<PathBuf>) -> Self {
        Self {
            workspace_id,
            file_paths,
            recursive: false,
            overwrite: false,
        }
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

impl Command for ImportFilesCommand {
    fn command_type(&self) -> &'static str {
        "ImportFiles"
    }

    fn aggregate_id(&self) -> Option<&str> {
        Some(&self.workspace_id)
    }
}

/// 删除工作区命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteWorkspaceCommand {
    /// 工作区 ID
    pub workspace_id: String,
    /// 是否删除文件
    pub delete_files: bool,
}

impl DeleteWorkspaceCommand {
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            delete_files: false,
        }
    }

    pub fn with_delete_files(mut self, delete: bool) -> Self {
        self.delete_files = delete;
        self
    }
}

impl Command for DeleteWorkspaceCommand {
    fn command_type(&self) -> &'static str {
        "DeleteWorkspace"
    }

    fn aggregate_id(&self) -> Option<&str> {
        Some(&self.workspace_id)
    }
}

/// 保存关键词组命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveKeywordsCommand {
    /// 工作区 ID（可选，全局关键词组时不提供）
    pub workspace_id: Option<String>,
    /// 关键词组名称
    pub name: String,
    /// 关键词列表
    pub keywords: Vec<String>,
    /// 颜色（可选）
    pub color: Option<String>,
    /// 是否覆盖已存在的同名组
    pub overwrite: bool,
}

impl SaveKeywordsCommand {
    pub fn new(name: String, keywords: Vec<String>) -> Self {
        Self {
            workspace_id: None,
            name,
            keywords,
            color: None,
            overwrite: false,
        }
    }

    pub fn for_workspace(mut self, workspace_id: String) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

impl Command for SaveKeywordsCommand {
    fn command_type(&self) -> &'static str {
        "SaveKeywords"
    }

    fn aggregate_id(&self) -> Option<&str> {
        self.workspace_id.as_deref()
    }
}

/// 取消任务命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskCommand {
    /// 任务 ID
    pub task_id: String,
    /// 取消原因
    pub reason: Option<String>,
}

impl CancelTaskCommand {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}

impl Command for CancelTaskCommand {
    fn command_type(&self) -> &'static str {
        "CancelTask"
    }

    fn aggregate_id(&self) -> Option<&str> {
        Some(&self.task_id)
    }
}

/// 导出结果命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResultsCommand {
    /// 搜索会话 ID
    pub session_id: String,
    /// 导出格式
    pub format: String,
    /// 输出路径
    pub output_path: PathBuf,
    /// 最大结果数
    pub max_results: Option<usize>,
}

impl ExportResultsCommand {
    pub fn new(session_id: String, format: String, output_path: PathBuf) -> Self {
        Self {
            session_id,
            format,
            output_path,
            max_results: None,
        }
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max);
        self
    }
}

impl Command for ExportResultsCommand {
    fn command_type(&self) -> &'static str {
        "ExportResults"
    }

    fn aggregate_id(&self) -> Option<&str> {
        Some(&self.session_id)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_workspace_command() {
        let cmd = CreateWorkspaceCommand::new("My Workspace".to_string(), PathBuf::from("/path"))
            .with_description("Test workspace".to_string());

        assert_eq!(cmd.name, "My Workspace");
        assert_eq!(cmd.description, Some("Test workspace".to_string()));
        assert_eq!(cmd.command_type(), "CreateWorkspace");
        assert!(cmd.aggregate_id().is_none());
    }

    #[test]
    fn test_import_files_command() {
        let cmd = ImportFilesCommand::new(
            "ws-1".to_string(),
            vec![PathBuf::from("/file1.log"), PathBuf::from("/file2.log")],
        )
        .with_recursive(true)
        .with_overwrite(true);

        assert_eq!(cmd.workspace_id, "ws-1");
        assert!(cmd.recursive);
        assert!(cmd.overwrite);
        assert_eq!(cmd.command_type(), "ImportFiles");
        assert_eq!(cmd.aggregate_id(), Some("ws-1"));
    }

    #[test]
    fn test_delete_workspace_command() {
        let cmd = DeleteWorkspaceCommand::new("ws-1".to_string()).with_delete_files(true);

        assert_eq!(cmd.workspace_id, "ws-1");
        assert!(cmd.delete_files);
        assert_eq!(cmd.command_type(), "DeleteWorkspace");
    }

    #[test]
    fn test_save_keywords_command() {
        let cmd = SaveKeywordsCommand::new(
            "Errors".to_string(),
            vec!["error".to_string(), "exception".to_string()],
        )
        .for_workspace("ws-1".to_string())
        .with_color("#FF0000".to_string())
        .with_overwrite(true);

        assert_eq!(cmd.name, "Errors");
        assert_eq!(cmd.keywords.len(), 2);
        assert_eq!(cmd.workspace_id, Some("ws-1".to_string()));
        assert!(cmd.overwrite);
    }

    #[test]
    fn test_cancel_task_command() {
        let cmd =
            CancelTaskCommand::new("task-1".to_string()).with_reason("User cancelled".to_string());

        assert_eq!(cmd.task_id, "task-1");
        assert_eq!(cmd.reason, Some("User cancelled".to_string()));
        assert_eq!(cmd.command_type(), "CancelTask");
    }

    #[test]
    fn test_export_results_command() {
        let cmd = ExportResultsCommand::new(
            "session-1".to_string(),
            "json".to_string(),
            PathBuf::from("/tmp/export.json"),
        )
        .with_max_results(100);

        assert_eq!(cmd.session_id, "session-1");
        assert_eq!(cmd.format, "json");
        assert_eq!(cmd.max_results, Some(100));
    }

    #[test]
    fn test_command_serialization() {
        let cmd = CreateWorkspaceCommand::new("Test".to_string(), PathBuf::from("/path"));
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("Test"));

        let deserialized: CreateWorkspaceCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, cmd.name);
    }
}
