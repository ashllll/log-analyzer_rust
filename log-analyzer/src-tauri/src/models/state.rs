//! 应用状态 — 扁平化的运行时状态持有者
//!
//! P8: 折叠 WorkspaceContext / SearchContext / TaskContext / SyncContext → AppState。
//! P11: 将 5 个扁平字段提取为 4 个 typed registries。
//! 每个 registry 内部管理自己的锁，提供小接口。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use crate::application::search_session::SearchSessionManager;
use crate::application::workspace_service::WorkspaceServiceRef;
use crate::infrastructure::TaskManagerAdapter;
use crate::state_sync::StateSync;
use crate::task_manager::TaskManager;
use la_core::domain::TaskScheduler;
use la_search::DiskResultStore;

// ============================================================================
// Typed Registries
// ============================================================================

#[derive(Default)]
pub struct WorkspaceRegistry {
    services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,
}

impl WorkspaceRegistry {
    pub fn get(&self, id: &str) -> Option<WorkspaceServiceRef> {
        self.services.lock().get(id).cloned()
    }
    pub fn register(&self, id: String, svc: WorkspaceServiceRef) {
        self.services.lock().insert(id, svc);
    }
    pub fn remove(&self, id: &str) {
        self.services.lock().remove(id);
    }
    pub fn all(&self) -> Vec<WorkspaceServiceRef> {
        self.services.lock().values().cloned().collect()
    }
    pub fn ids(&self) -> Vec<String> {
        self.services.lock().keys().cloned().collect()
    }
}

pub struct SearchRegistry {
    disk_result_store: RwLock<Option<Arc<DiskResultStore>>>,
    search_session_manager: RwLock<Option<SearchSessionManager>>,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl Default for SearchRegistry {
    fn default() -> Self {
        Self {
            disk_result_store: RwLock::new(None),
            search_session_manager: RwLock::new(None),
            thread_pool: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(num_cpus::get().max(2))
                    .build()
                    .expect("Failed to create search thread pool"),
            ),
        }
    }
}

impl SearchRegistry {
    pub fn init_disk_result_store_at(&self, base_path: PathBuf) -> Result<(), String> {
        let cache_dir = base_path.join("search-cache");
        match DiskResultStore::new(cache_dir.clone(), 50) {
            Ok(store) => {
                let store = Arc::new(store);
                let manager = SearchSessionManager::new(Arc::clone(&store));
                *self.disk_result_store.write() = Some(store);
                *self.search_session_manager.write() = Some(manager);
                tracing::info!(path = %cache_dir.display(), "DiskResultStore initialized");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %cache_dir.display(), "DiskResultStore init failed");
                Err(format!("DiskResultStore init failed at {}: {e}", cache_dir.display()))
            }
        }
    }
    pub fn get_disk_result_store(&self) -> Option<Arc<DiskResultStore>> {
        self.disk_result_store.read().clone()
    }
    pub fn get_search_session_manager(&self) -> Option<SearchSessionManager> {
        self.search_session_manager.read().clone()
    }
    pub fn get_thread_pool(&self) -> Arc<rayon::ThreadPool> {
        Arc::clone(&self.thread_pool)
    }
    pub fn cleanup_disk_result_store(&self) {
        if let Some(store) = self.disk_result_store.read().as_ref() {
            store.cleanup_all();
        }
    }
}

#[derive(Default)]
pub struct TaskRegistry {
    manager: Arc<Mutex<Option<TaskManager>>>,
}

impl TaskRegistry {
    pub fn init(&self, tm: TaskManager) {
        *self.manager.lock() = Some(tm);
    }
    pub fn take(&self) -> Option<TaskManager> {
        self.manager.lock().take()
    }
    pub fn clone_manager(&self) -> Option<TaskManager> {
        self.manager.lock().clone()
    }
    pub fn scheduler(&self) -> Option<Arc<dyn TaskScheduler>> {
        self.manager.lock().as_ref().map(|tm| {
            Arc::new(TaskManagerAdapter::new(Arc::new(tm.clone()))) as Arc<dyn TaskScheduler>
        })
    }
}

#[derive(Default)]
pub struct SyncRegistry {
    sync: Arc<Mutex<Option<StateSync>>>,
}

impl SyncRegistry {
    pub fn init(&self, s: StateSync) {
        *self.sync.lock() = Some(s);
    }
    pub fn get(&self) -> Option<StateSync> {
        self.sync.lock().clone()
    }
    pub fn arc(&self) -> Arc<Mutex<Option<StateSync>>> {
        Arc::clone(&self.sync)
    }
}

// ============================================================================
// AppState — thin registry holder
// ============================================================================

pub struct AppState {
    pub workspace: WorkspaceRegistry,
    pub search: SearchRegistry,
    pub task: TaskRegistry,
    pub sync: SyncRegistry,
}

#[allow(clippy::derivable_impls)]
impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: WorkspaceRegistry::default(),
            search: SearchRegistry::default(),
            task: TaskRegistry::default(),
            sync: SyncRegistry::default(),
        }
    }
}

// ── Backward-compat methods (delegate to registries) ──

impl AppState {
    pub fn get_workspace_service(&self, workspace_id: &str) -> Option<WorkspaceServiceRef> {
        self.workspace.get(workspace_id)
    }
    pub fn set_workspace_service(&self, workspace_id: String, service: WorkspaceServiceRef) {
        self.workspace.register(workspace_id, service);
    }
    pub fn remove_workspace_service(&self, workspace_id: &str) {
        self.workspace.remove(workspace_id);
    }
    pub fn all_workspace_services(&self) -> Vec<WorkspaceServiceRef> {
        self.workspace.all()
    }
    pub fn workspace_ids(&self) -> Vec<String> {
        self.workspace.ids()
    }

    pub fn init_disk_result_store_at(&self, base_path: PathBuf) -> Result<(), String> {
        self.search.init_disk_result_store_at(base_path)
    }
    pub fn get_disk_result_store(&self) -> Option<Arc<DiskResultStore>> {
        self.search.get_disk_result_store()
    }
    pub fn get_search_session_manager(&self) -> Option<SearchSessionManager> {
        self.search.get_search_session_manager()
    }
    pub fn get_search_thread_pool(&self) -> Arc<rayon::ThreadPool> {
        self.search.get_thread_pool()
    }
    pub fn cleanup_disk_result_store(&self) {
        self.search.cleanup_disk_result_store();
    }

    pub fn init_task_manager(&self, task_manager: TaskManager) {
        self.task.init(task_manager);
    }
    pub fn take_task_manager(&self) -> Option<TaskManager> {
        self.task.take()
    }
    pub fn get_task_manager_clone(&self) -> Option<TaskManager> {
        self.task.clone_manager()
    }
    pub fn get_task_scheduler(&self) -> Option<Arc<dyn TaskScheduler>> {
        self.task.scheduler()
    }

    pub fn init_state_sync(&self, sync: StateSync) {
        self.sync.init(sync);
    }
    pub fn get_state_sync(&self) -> Option<StateSync> {
        self.sync.get()
    }
    pub fn state_sync_arc(&self) -> Arc<Mutex<Option<StateSync>>> {
        self.sync.arc()
    }
}
