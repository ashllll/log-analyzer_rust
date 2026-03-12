//! 持久化基础设施模块
//!
//! 提供数据持久化的基础设施实现，包括：
//! - 工作区元数据存储
//! - 文件元数据存储
//! - 搜索历史存储
//! - 配置持久化

use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "ffi")]
use flutter_rust_bridge::frb;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::domain::log_analysis::repositories::{
    KeywordGroup, KeywordGroupRepository, RepositoryResult, SearchHistoryRepository, SearchRecord,
    Workspace, WorkspaceRepository, WorkspaceStatus,
};

/// 持久化配置
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// 应用数据目录
    pub app_data_dir: PathBuf,
    /// 是否启用 WAL 模式
    pub enable_wal: bool,
    /// 连接池大小
    pub pool_size: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            app_data_dir: PathBuf::from("."),
            enable_wal: true,
            pool_size: 5,
        }
    }
}

/// 工作区仓储实现
///
/// 使用 SQLite 进行工作区元数据的持久化
pub struct WorkspaceRepositoryImpl {
    config: PersistenceConfig,
    cache: Arc<RwLock<Vec<Workspace>>>,
}

impl WorkspaceRepositoryImpl {
    /// 创建新的工作区仓储
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 获取工作区元数据文件路径
    fn workspace_meta_path(&self, id: &str) -> PathBuf {
        self.config
            .app_data_dir
            .join("workspaces")
            .join(id)
            .join("workspace.json")
    }

    /// 从文件加载工作区元数据
    fn load_workspace_meta(&self, id: &str) -> RepositoryResult<Option<Workspace>> {
        let path = self.workspace_meta_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取工作区元数据失败: {}", e))?;

        let meta: WorkspaceMetadata =
            serde_json::from_str(&content).map_err(|e| format!("解析工作区元数据失败: {}", e))?;

        Ok(Some(Workspace {
            id: id.to_string(),
            name: meta.name,
            path: meta.path,
            status: WorkspaceStatus::parse(&meta.status),
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            file_count: meta.file_count,
            total_size: meta.total_size,
        }))
    }

    /// 保存工作区元数据到文件
    fn save_workspace_meta(&self, workspace: &Workspace) -> RepositoryResult<()> {
        let path = self.workspace_meta_path(&workspace.id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建工作区目录失败: {}", e))?;
        }

        let meta = WorkspaceMetadata {
            name: workspace.name.clone(),
            path: workspace.path.clone(),
            status: workspace.status.as_str().to_string(),
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
            file_count: workspace.file_count,
            total_size: workspace.total_size,
        };

        let content = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("序列化工作区元数据失败: {}", e))?;

        std::fs::write(&path, content).map_err(|e| format!("写入工作区元数据失败: {}", e))?;

        Ok(())
    }
}

/// 工作区元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceMetadata {
    name: String,
    path: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    file_count: u64,
    total_size: u64,
}

#[async_trait]
impl WorkspaceRepository for WorkspaceRepositoryImpl {
    async fn save(&self, workspace: &Workspace) -> RepositoryResult<()> {
        self.save_workspace_meta(workspace)?;

        // 更新缓存
        let mut cache = self.cache.write();
        if let Some(existing) = cache.iter_mut().find(|w| w.id == workspace.id) {
            *existing = workspace.clone();
        } else {
            cache.push(workspace.clone());
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<Workspace>> {
        // 先检查缓存
        {
            let cache = self.cache.read();
            if let Some(workspace) = cache.iter().find(|w| w.id == id) {
                return Ok(Some(workspace.clone()));
            }
        }

        // 从文件加载
        self.load_workspace_meta(id)
    }

    async fn find_all(&self) -> RepositoryResult<Vec<Workspace>> {
        let workspaces_dir = self.config.app_data_dir.join("workspaces");
        if !workspaces_dir.exists() {
            return Ok(Vec::new());
        }

        let mut workspaces = Vec::new();
        let entries =
            std::fs::read_dir(&workspaces_dir).map_err(|e| format!("读取工作区目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
            if entry.path().is_dir() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(workspace) = self.load_workspace_meta(file_name)? {
                        workspaces.push(workspace);
                    }
                }
            }
        }

        // 更新缓存
        *self.cache.write() = workspaces.clone();

        Ok(workspaces)
    }

    async fn update(&self, workspace: &Workspace) -> RepositoryResult<()> {
        self.save(workspace).await
    }

    async fn delete(&self, id: &str) -> RepositoryResult<()> {
        let workspace_dir = self.config.app_data_dir.join("workspaces").join(id);
        if workspace_dir.exists() {
            std::fs::remove_dir_all(&workspace_dir)
                .map_err(|e| format!("删除工作区目录失败: {}", e))?;
        }

        // 更新缓存
        let mut cache = self.cache.write();
        cache.retain(|w| w.id != id);

        Ok(())
    }

    async fn exists(&self, id: &str) -> RepositoryResult<bool> {
        let path = self.workspace_meta_path(id);
        Ok(path.exists())
    }
}

/// 关键词组仓储实现
pub struct KeywordGroupRepositoryImpl {
    config: PersistenceConfig,
    cache: Arc<RwLock<Vec<KeywordGroup>>>,
}

impl KeywordGroupRepositoryImpl {
    /// 创建新的关键词组仓储
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 获取关键词组存储文件路径
    fn keywords_path(&self) -> PathBuf {
        self.config.app_data_dir.join("keywords.json")
    }

    /// 加载所有关键词组
    fn load_all(&self) -> RepositoryResult<Vec<KeywordGroup>> {
        let path = self.keywords_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取关键词组文件失败: {}", e))?;

        let groups: Vec<KeywordGroupData> =
            serde_json::from_str(&content).map_err(|e| format!("解析关键词组失败: {}", e))?;

        Ok(groups
            .into_iter()
            .map(|g| KeywordGroup {
                id: g.id,
                name: g.name,
                color: g.color,
                patterns: g.patterns,
                enabled: g.enabled,
                created_at: g.created_at,
                updated_at: g.updated_at,
            })
            .collect())
    }

    /// 保存所有关键词组
    fn save_all(&self, groups: &[KeywordGroup]) -> RepositoryResult<()> {
        let path = self.keywords_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let data: Vec<KeywordGroupData> = groups
            .iter()
            .map(|g| KeywordGroupData {
                id: g.id.clone(),
                name: g.name.clone(),
                color: g.color.clone(),
                patterns: g.patterns.clone(),
                enabled: g.enabled,
                created_at: g.created_at,
                updated_at: g.updated_at,
            })
            .collect();

        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("序列化关键词组失败: {}", e))?;

        std::fs::write(&path, content).map_err(|e| format!("写入关键词组文件失败: {}", e))?;

        Ok(())
    }
}

/// 关键词组数据（持久化格式）
///
/// 注意：此类型用于持久化层，包含时间戳字段。
/// FFI 层使用单独的 KeywordGroupData（定义在 ffi/types.rs），
/// 不包含时间戳字段以简化 Dart 绑定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", frb(opaque))]
pub struct KeywordGroupData {
    pub id: String,
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl KeywordGroupRepository for KeywordGroupRepositoryImpl {
    async fn save(&self, group: &KeywordGroup) -> RepositoryResult<()> {
        let mut groups = self.load_all()?;

        if let Some(existing) = groups.iter_mut().find(|g| g.id == group.id) {
            *existing = group.clone();
        } else {
            groups.push(group.clone());
        }

        self.save_all(&groups)?;
        *self.cache.write() = groups;

        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<KeywordGroup>> {
        let groups = self.load_all()?;
        Ok(groups.into_iter().find(|g| g.id == id))
    }

    async fn find_all(&self) -> RepositoryResult<Vec<KeywordGroup>> {
        self.load_all()
    }

    async fn find_enabled(&self) -> RepositoryResult<Vec<KeywordGroup>> {
        let groups = self.load_all()?;
        Ok(groups.into_iter().filter(|g| g.enabled).collect())
    }

    async fn update(&self, group: &KeywordGroup) -> RepositoryResult<()> {
        self.save(group).await
    }

    async fn delete(&self, id: &str) -> RepositoryResult<()> {
        let mut groups = self.load_all()?;
        groups.retain(|g| g.id != id);
        self.save_all(&groups)?;
        *self.cache.write() = groups;
        Ok(())
    }
}

/// 搜索历史仓储实现
pub struct SearchHistoryRepositoryImpl {
    config: PersistenceConfig,
}

impl SearchHistoryRepositoryImpl {
    /// 创建新的搜索历史仓储
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// 获取搜索历史文件路径
    fn history_path(&self) -> PathBuf {
        self.config.app_data_dir.join("search_history.json")
    }
}

#[async_trait]
impl SearchHistoryRepository for SearchHistoryRepositoryImpl {
    async fn save(&self, record: &SearchRecord) -> RepositoryResult<()> {
        let path = self.history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let mut records = if path.exists() {
            let content =
                std::fs::read_to_string(&path).map_err(|e| format!("读取搜索历史失败: {}", e))?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        records.push(SearchRecordData::from(record));
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 保留最近 100 条记录
        records.truncate(100);

        let content = serde_json::to_string_pretty(&records)
            .map_err(|e| format!("序列化搜索历史失败: {}", e))?;

        std::fs::write(&path, content).map_err(|e| format!("写入搜索历史失败: {}", e))?;

        Ok(())
    }

    async fn find_recent(&self, limit: usize) -> RepositoryResult<Vec<SearchRecord>> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取搜索历史失败: {}", e))?;

        let records: Vec<SearchRecordData> =
            serde_json::from_str(&content).map_err(|e| format!("解析搜索历史失败: {}", e))?;

        Ok(records
            .into_iter()
            .take(limit)
            .map(SearchRecord::from)
            .collect())
    }

    async fn find_by_workspace(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> RepositoryResult<Vec<SearchRecord>> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取搜索历史失败: {}", e))?;

        let records: Vec<SearchRecordData> =
            serde_json::from_str(&content).map_err(|e| format!("解析搜索历史失败: {}", e))?;

        Ok(records
            .into_iter()
            .filter(|r| r.workspace_id.as_deref() == Some(workspace_id))
            .take(limit)
            .map(SearchRecord::from)
            .collect())
    }

    async fn delete(&self, id: uuid::Uuid) -> RepositoryResult<()> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(());
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取搜索历史失败: {}", e))?;

        let mut records: Vec<SearchRecordData> =
            serde_json::from_str(&content).map_err(|e| format!("解析搜索历史失败: {}", e))?;

        records.retain(|r| r.id != id);

        let content = serde_json::to_string_pretty(&records)
            .map_err(|e| format!("序列化搜索历史失败: {}", e))?;

        std::fs::write(&path, content).map_err(|e| format!("写入搜索历史失败: {}", e))?;

        Ok(())
    }

    async fn clear(&self) -> RepositoryResult<()> {
        let path = self.history_path();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("清除搜索历史失败: {}", e))?;
        }
        Ok(())
    }
}

/// 搜索记录数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchRecordData {
    id: uuid::Uuid,
    query: String,
    workspace_id: Option<String>,
    result_count: usize,
    duration_ms: u64,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<&SearchRecord> for SearchRecordData {
    fn from(record: &SearchRecord) -> Self {
        Self {
            id: record.id,
            query: record.query.clone(),
            workspace_id: record.workspace_id.clone(),
            result_count: record.result_count,
            duration_ms: record.duration_ms,
            timestamp: record.timestamp,
        }
    }
}

impl From<SearchRecordData> for SearchRecord {
    fn from(data: SearchRecordData) -> Self {
        SearchRecord {
            id: data.id,
            query: data.query,
            workspace_id: data.workspace_id,
            result_count: data.result_count,
            duration_ms: data.duration_ms,
            timestamp: data.timestamp,
        }
    }
}

/// JSON 文件存储通用实现
pub struct JsonFileStorage<T> {
    path: PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> JsonFileStorage<T> {
    /// 创建新的 JSON 文件存储
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 读取数据
    pub fn read(&self) -> RepositoryResult<Option<T>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("读取文件失败 {}: {}", self.path.display(), e))?;

        let data: T = serde_json::from_str(&content)
            .map_err(|e| format!("解析 JSON 失败 {}: {}", self.path.display(), e))?;

        Ok(Some(data))
    }

    /// 写入数据
    pub fn write(&self, data: &T) -> RepositoryResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败 {}: {}", parent.display(), e))?;
        }

        let content =
            serde_json::to_string_pretty(data).map_err(|e| format!("序列化 JSON 失败: {}", e))?;

        std::fs::write(&self.path, content)
            .map_err(|e| format!("写入文件失败 {}: {}", self.path.display(), e))?;

        Ok(())
    }

    /// 删除文件
    pub fn delete(&self) -> RepositoryResult<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)
                .map_err(|e| format!("删除文件失败 {}: {}", self.path.display(), e))?;
        }
        Ok(())
    }
}

/// 持久化工厂
///
/// 创建各种仓储实例
pub struct PersistenceFactory {
    config: PersistenceConfig,
}

impl PersistenceFactory {
    /// 创建新的持久化工厂
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// 创建工作区仓储
    pub fn create_workspace_repository(&self) -> WorkspaceRepositoryImpl {
        WorkspaceRepositoryImpl::new(self.config.clone())
    }

    /// 创建关键词组仓储
    pub fn create_keyword_group_repository(&self) -> KeywordGroupRepositoryImpl {
        KeywordGroupRepositoryImpl::new(self.config.clone())
    }

    /// 创建搜索历史仓储
    pub fn create_search_history_repository(&self) -> SearchHistoryRepositoryImpl {
        SearchHistoryRepositoryImpl::new(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_json_file_storage() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let storage = JsonFileStorage::<Vec<String>>::new(path);

        // 写入数据
        let data = vec!["a".to_string(), "b".to_string()];
        storage.write(&data).unwrap();

        // 读取数据
        let loaded = storage.read().unwrap().unwrap();
        assert_eq!(loaded, data);

        // 删除数据
        storage.delete().unwrap();
        assert!(storage.read().unwrap().is_none());
    }

    #[test]
    fn test_workspace_repository() {
        let dir = tempdir().unwrap();
        let config = PersistenceConfig {
            app_data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };

        let repo = WorkspaceRepositoryImpl::new(config);

        // 创建工作区
        let workspace = Workspace::new(
            "test-id".to_string(),
            "Test Workspace".to_string(),
            "/path/to/logs".to_string(),
        );

        // 保存
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            repo.save(&workspace).await.unwrap();
        });

        // 读取
        rt.block_on(async {
            let loaded = repo.find_by_id("test-id").await.unwrap().unwrap();
            assert_eq!(loaded.name, "Test Workspace");
            assert_eq!(loaded.path, "/path/to/logs");
        });

        // 检查存在
        rt.block_on(async {
            assert!(repo.exists("test-id").await.unwrap());
            assert!(!repo.exists("non-existent").await.unwrap());
        });

        // 删除
        rt.block_on(async {
            repo.delete("test-id").await.unwrap();
            assert!(!repo.exists("test-id").await.unwrap());
        });
    }
}
