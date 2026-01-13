//! 应用状态管理 - 简化版本

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// 简化应用状态
#[derive(Debug)]
pub struct AppState {
    pub workspace_dirs: Arc<Mutex<HashMap<String, std::path::PathBuf>>>,
    pub cas_instances: Arc<Mutex<HashMap<String, Arc<()>>>>,
    pub metadata_stores: Arc<Mutex<HashMap<String, Arc<()>>>>,
    pub task_manager: Arc<Mutex<Option<()>>>,
    pub search_cancellation_tokens: Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    pub total_searches: Arc<Mutex<u64>>,
    pub cache_hits: Arc<Mutex<u64>>,
    pub last_search_duration: Arc<Mutex<std::time::Duration>>,
    pub watchers: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    pub cleanup_queue: Arc<Mutex<Vec<String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace_dirs: Arc::new(Mutex::new(HashMap::new())),
            cas_instances: Arc::new(Mutex::new(HashMap::new())),
            metadata_stores: Arc::new(Mutex::new(HashMap::new())),
            task_manager: Arc::new(Mutex::new(None)),
            search_cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            total_searches: Arc::new(Mutex::new(0)),
            cache_hits: Arc::new(Mutex::new(0)),
            last_search_duration: Arc::new(Mutex::new(std::time::Duration::from_secs(0))),
            watchers: Arc::new(Mutex::new(HashMap::new())),
            cleanup_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_workspace_dir(&self, workspace_id: &str) -> Option<std::path::PathBuf> {
        let dirs = self.workspace_dirs.lock();
        dirs.get(workspace_id).cloned()
    }
    
    pub fn set_workspace_dir(&self, workspace_id: String, path: std::path::PathBuf) {
        let mut dirs = self.workspace_dirs.lock();
        dirs.insert(workspace_id, path);
    }
}