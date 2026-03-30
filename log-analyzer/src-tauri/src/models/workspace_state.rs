//! 工作区状态管理
//!
//! 使用 DashMap 替代 Arc<Mutex<HashMap<...>>> 实现无锁并发访问

use crate::services::file_watcher::WatcherState;
use dashmap::DashMap;
use la_search::SearchEngineManager;
use la_storage::{ContentAddressableStorage, MetadataStore};
use std::path::PathBuf;
use std::sync::Arc;

/// 工作区状态 - 管理工作区相关的所有资源
pub struct WorkspaceState {
    /// 工作区目录映射 (workspace_id -> path)
    pub workspace_dirs: DashMap<String, PathBuf>,
    /// CAS 实例映射 (workspace_id -> CAS)
    pub cas_instances: DashMap<String, Arc<ContentAddressableStorage>>,
    /// 元数据存储映射 (workspace_id -> MetadataStore)
    pub metadata_stores: DashMap<String, Arc<MetadataStore>>,
    /// 搜索引擎管理器映射 (workspace_id -> SearchEngineManager)
    pub search_engine_managers: DashMap<String, Arc<SearchEngineManager>>,
    /// 文件监听器状态映射 (workspace_id -> WatcherState)
    pub watchers: DashMap<String, WatcherState>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            workspace_dirs: DashMap::new(),
            cas_instances: DashMap::new(),
            metadata_stores: DashMap::new(),
            search_engine_managers: DashMap::new(),
            watchers: DashMap::new(),
        }
    }
}

impl WorkspaceState {
    /// 获取工作区目录
    pub fn get_workspace_dir(&self, workspace_id: &str) -> Option<PathBuf> {
        self.workspace_dirs
            .get(workspace_id)
            .map(|entry| entry.clone())
    }

    /// 设置工作区目录
    pub fn set_workspace_dir(&self, workspace_id: String, path: PathBuf) {
        self.workspace_dirs.insert(workspace_id, path);
    }

    /// 移除工作区及其所有关联资源
    pub fn remove_workspace(&self, workspace_id: &str) {
        self.workspace_dirs.remove(workspace_id);
        self.cas_instances.remove(workspace_id);
        self.metadata_stores.remove(workspace_id);
        self.search_engine_managers.remove(workspace_id);
        self.watchers.remove(workspace_id);
    }

    /// 检查工作区是否存在
    pub fn has_workspace(&self, workspace_id: &str) -> bool {
        self.workspace_dirs.contains_key(workspace_id)
    }
}
