//! WorkspaceRepository adapter backed by AppState runtime maps.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use la_core::domain::{WorkspaceInfo, WorkspaceRepository, WorkspaceStatus};
use la_core::error::{AppError, Result};
use la_search::SearchEngineManager;
use la_storage::{ContentAddressableStorage, MetadataStore};

use crate::services::file_watcher::WatcherState;

/// Runtime workspace repository.
///
/// This adapter keeps the application layer independent from AppState while
/// still using the current in-memory workspace registry.
pub struct RuntimeWorkspaceRepository {
    workspace_dirs: Arc<Mutex<BTreeMap<String, PathBuf>>>,
    metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,
    cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,
    search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,
    watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
}

impl RuntimeWorkspaceRepository {
    pub fn new(
        workspace_dirs: Arc<Mutex<BTreeMap<String, PathBuf>>>,
        metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,
        cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,
        search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,
        watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    ) -> Self {
        Self {
            workspace_dirs,
            metadata_stores,
            cas_instances,
            search_engine_managers,
            watchers,
        }
    }

    async fn build_info(&self, id: &str, path: PathBuf) -> Result<WorkspaceInfo> {
        let status = if self.watchers.lock().contains_key(id) {
            WorkspaceStatus::Watching
        } else {
            WorkspaceStatus::Ready
        };

        let store = { self.metadata_stores.lock().get(id).cloned() };
        let file_count = if let Some(store) = store {
            store.count_files().await.unwrap_or(0).max(0) as usize
        } else {
            0
        };

        Ok(WorkspaceInfo {
            id: id.to_string(),
            name: id.to_string(),
            path: path.display().to_string(),
            status,
            file_count,
        })
    }

    fn stop_watcher(&self, id: &str) {
        let watcher_state = { self.watchers.lock().remove(id) };
        if let Some(mut watcher_state) = watcher_state {
            watcher_state.is_active = false;
            let thread_handle = watcher_state.thread_handle.lock().take();
            let watcher = watcher_state.watcher.lock().take();
            drop(watcher);
            if let Some(handle) = thread_handle {
                let _ = handle.join();
            }
        }
    }
}

#[async_trait]
impl WorkspaceRepository for RuntimeWorkspaceRepository {
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let dirs = { self.workspace_dirs.lock().clone() };
        let mut workspaces = Vec::with_capacity(dirs.len());
        for (id, path) in dirs {
            workspaces.push(self.build_info(&id, path).await?);
        }
        Ok(workspaces)
    }

    async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>> {
        let path = { self.workspace_dirs.lock().get(id).cloned() };
        match path {
            Some(path) => self.build_info(id, path).await.map(Some),
            None => Ok(None),
        }
    }

    async fn delete_workspace(&self, id: &str) -> Result<()> {
        self.stop_watcher(id);

        let store = { self.metadata_stores.lock().remove(id) };
        if let Some(store) = store {
            store.close().await;
        }

        let manager = { self.search_engine_managers.lock().remove(id) };
        if let Some(manager) = manager {
            manager.close().await;
        }

        self.cas_instances.lock().remove(id);
        let workspace_dir = { self.workspace_dirs.lock().remove(id) };

        if let Some(workspace_dir) = workspace_dir {
            if workspace_dir.exists() {
                tokio::fs::remove_dir_all(&workspace_dir)
                    .await
                    .map_err(|e| {
                        AppError::io_error(
                            format!("Failed to delete workspace directory: {e}"),
                            Some(workspace_dir.clone()),
                        )
                    })?;
            }
        }

        Ok(())
    }
}
