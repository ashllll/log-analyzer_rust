//! 日志分析仓储接口
//!
//! 定义数据访问的抽象接口，遵循 DDD 的依赖倒置原则。
//! 基础设施层负责实现这些接口。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::entities::{LogEntry, LogFile, LogFormat};
use super::value_objects::LogLevel;

/// 仓储操作结果类型
pub type RepositoryResult<T> = std::result::Result<T, String>;

/// 日志条目仓储接口
///
/// 定义日志条目的持久化操作
#[async_trait]
pub trait LogEntryRepository: Send + Sync {
    /// 保存日志条目
    async fn save(&self, entry: &LogEntry) -> RepositoryResult<()>;

    /// 批量保存日志条目
    async fn save_batch(&self, entries: &[LogEntry]) -> RepositoryResult<usize>;

    /// 根据 ID 查找日志条目
    async fn find_by_id(&self, id: Uuid) -> RepositoryResult<Option<LogEntry>>;

    /// 根据文件 ID 查找日志条目
    async fn find_by_file_id(
        &self,
        file_id: Uuid,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<LogEntry>>;

    /// 根据时间范围查找日志条目
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<LogEntry>>;

    /// 根据级别查找日志条目
    async fn find_by_level(
        &self,
        level: &LogLevel,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<LogEntry>>;

    /// 搜索日志条目
    async fn search(&self, query: &str, limit: Option<usize>) -> RepositoryResult<Vec<LogEntry>>;

    /// 统计日志条目数量
    async fn count(&self) -> RepositoryResult<u64>;

    /// 根据文件 ID 删除日志条目
    async fn delete_by_file_id(&self, file_id: Uuid) -> RepositoryResult<usize>;
}

/// 日志文件仓储接口
///
/// 定义日志文件的持久化操作
#[async_trait]
pub trait LogFileRepository: Send + Sync {
    /// 保存日志文件元数据
    async fn save(&self, file: &LogFile) -> RepositoryResult<()>;

    /// 根据 ID 查找日志文件
    async fn find_by_id(&self, id: Uuid) -> RepositoryResult<Option<LogFile>>;

    /// 根据路径查找日志文件
    async fn find_by_path(&self, path: &str) -> RepositoryResult<Option<LogFile>>;

    /// 查找所有日志文件
    async fn find_all(&self) -> RepositoryResult<Vec<LogFile>>;

    /// 根据格式查找日志文件
    async fn find_by_format(&self, format: &LogFormat) -> RepositoryResult<Vec<LogFile>>;

    /// 更新日志文件
    async fn update(&self, file: &LogFile) -> RepositoryResult<()>;

    /// 删除日志文件
    async fn delete(&self, id: Uuid) -> RepositoryResult<()>;

    /// 统计日志文件数量
    async fn count(&self) -> RepositoryResult<u64>;

    /// 计算总大小
    async fn total_size(&self) -> RepositoryResult<u64>;
}

/// 工作区仓储接口
///
/// 定义工作区的持久化操作
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    /// 保存工作区
    async fn save(&self, workspace: &Workspace) -> RepositoryResult<()>;

    /// 根据 ID 查找工作区
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<Workspace>>;

    /// 查找所有工作区
    async fn find_all(&self) -> RepositoryResult<Vec<Workspace>>;

    /// 更新工作区
    async fn update(&self, workspace: &Workspace) -> RepositoryResult<()>;

    /// 删除工作区
    async fn delete(&self, id: &str) -> RepositoryResult<()>;

    /// 检查工作区是否存在
    async fn exists(&self, id: &str) -> RepositoryResult<bool>;
}

/// 工作区实体
///
/// 表示一个日志分析工作区
#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub file_count: u64,
    pub total_size: u64,
}

impl Workspace {
    /// 创建新工作区
    pub fn new(id: String, name: String, path: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            path,
            status: WorkspaceStatus::Created,
            created_at: now,
            updated_at: now,
            file_count: 0,
            total_size: 0,
        }
    }

    /// 更新工作区状态
    pub fn set_status(&mut self, status: WorkspaceStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// 更新文件统计
    pub fn update_statistics(&mut self, file_count: u64, total_size: u64) {
        self.file_count = file_count;
        self.total_size = total_size;
        self.updated_at = Utc::now();
    }

    /// 检查是否为活跃状态
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkspaceStatus::Ready | WorkspaceStatus::Scanning
        )
    }
}

/// 工作区状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceStatus {
    Created,
    Scanning,
    Ready,
    Error,
    Offline,
}

impl WorkspaceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceStatus::Created => "CREATED",
            WorkspaceStatus::Scanning => "SCANNING",
            WorkspaceStatus::Ready => "READY",
            WorkspaceStatus::Error => "ERROR",
            WorkspaceStatus::Offline => "OFFLINE",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CREATED" => WorkspaceStatus::Created,
            "SCANNING" => WorkspaceStatus::Scanning,
            "READY" => WorkspaceStatus::Ready,
            "ERROR" => WorkspaceStatus::Error,
            "OFFLINE" => WorkspaceStatus::Offline,
            _ => WorkspaceStatus::Created,
        }
    }
}

/// 搜索历史仓储接口
///
/// 定义搜索历史的持久化操作
#[async_trait]
pub trait SearchHistoryRepository: Send + Sync {
    /// 保存搜索记录
    async fn save(&self, record: &SearchRecord) -> RepositoryResult<()>;

    /// 查找最近的搜索记录
    async fn find_recent(&self, limit: usize) -> RepositoryResult<Vec<SearchRecord>>;

    /// 根据工作区查找搜索记录
    async fn find_by_workspace(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> RepositoryResult<Vec<SearchRecord>>;

    /// 删除搜索记录
    async fn delete(&self, id: Uuid) -> RepositoryResult<()>;

    /// 清空搜索历史
    async fn clear(&self) -> RepositoryResult<()>;
}

/// 搜索记录实体
#[derive(Debug, Clone)]
pub struct SearchRecord {
    pub id: Uuid,
    pub query: String,
    pub workspace_id: Option<String>,
    pub result_count: usize,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl SearchRecord {
    pub fn new(query: String, workspace_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            query,
            workspace_id,
            result_count: 0,
            duration_ms: 0,
            timestamp: Utc::now(),
        }
    }

    pub fn with_results(mut self, result_count: usize, duration_ms: u64) -> Self {
        self.result_count = result_count;
        self.duration_ms = duration_ms;
        self
    }
}

/// 关键词组仓储接口
///
/// 定义关键词组的持久化操作
#[async_trait]
pub trait KeywordGroupRepository: Send + Sync {
    /// 保存关键词组
    async fn save(&self, group: &KeywordGroup) -> RepositoryResult<()>;

    /// 根据 ID 查找关键词组
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<KeywordGroup>>;

    /// 查找所有关键词组
    async fn find_all(&self) -> RepositoryResult<Vec<KeywordGroup>>;

    /// 查找启用的关键词组
    async fn find_enabled(&self) -> RepositoryResult<Vec<KeywordGroup>>;

    /// 更新关键词组
    async fn update(&self, group: &KeywordGroup) -> RepositoryResult<()>;

    /// 删除关键词组
    async fn delete(&self, id: &str) -> RepositoryResult<()>;
}

/// 关键词组实体
#[derive(Debug, Clone)]
pub struct KeywordGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl KeywordGroup {
    pub fn new(id: String, name: String, color: String, patterns: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            color,
            patterns,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.updated_at = Utc::now();
    }

    pub fn add_pattern(&mut self, pattern: String) {
        if !self.patterns.contains(&pattern) {
            self.patterns.push(pattern);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_pattern(&mut self, pattern: &str) {
        self.patterns.retain(|p| p != pattern);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let workspace = Workspace::new(
            "ws-1".to_string(),
            "Test Workspace".to_string(),
            "/path/to/logs".to_string(),
        );

        assert_eq!(workspace.id, "ws-1");
        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.path, "/path/to/logs");
        assert_eq!(workspace.status, WorkspaceStatus::Created);
        assert!(workspace.file_count == 0);
    }

    #[test]
    fn test_workspace_status_transitions() {
        let mut workspace =
            Workspace::new("ws-1".to_string(), "Test".to_string(), "/path".to_string());

        workspace.set_status(WorkspaceStatus::Scanning);
        assert_eq!(workspace.status, WorkspaceStatus::Scanning);
        assert!(workspace.is_active());

        workspace.set_status(WorkspaceStatus::Ready);
        assert_eq!(workspace.status, WorkspaceStatus::Ready);
        assert!(workspace.is_active());

        workspace.set_status(WorkspaceStatus::Error);
        assert_eq!(workspace.status, WorkspaceStatus::Error);
        assert!(!workspace.is_active());
    }

    #[test]
    fn test_keyword_group() {
        let mut group = KeywordGroup::new(
            "kg-1".to_string(),
            "Errors".to_string(),
            "#ff0000".to_string(),
            vec!["error".to_string(), "exception".to_string()],
        );

        assert!(group.enabled);
        assert_eq!(group.patterns.len(), 2);

        group.add_pattern("fatal".to_string());
        assert_eq!(group.patterns.len(), 3);

        group.remove_pattern("error");
        assert_eq!(group.patterns.len(), 2);

        group.toggle();
        assert!(!group.enabled);
    }

    #[test]
    fn test_search_record() {
        let record = SearchRecord::new("error OR exception".to_string(), Some("ws-1".to_string()))
            .with_results(42, 150);

        assert_eq!(record.query, "error OR exception");
        assert_eq!(record.workspace_id, Some("ws-1".to_string()));
        assert_eq!(record.result_count, 42);
        assert_eq!(record.duration_ms, 150);
    }

    #[test]
    fn test_workspace_status_from_str() {
        assert_eq!(WorkspaceStatus::parse("READY"), WorkspaceStatus::Ready);
        assert_eq!(
            WorkspaceStatus::parse("scanning"),
            WorkspaceStatus::Scanning
        );
        assert_eq!(WorkspaceStatus::parse("unknown"), WorkspaceStatus::Created);
    }
}
