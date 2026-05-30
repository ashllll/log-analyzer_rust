//! WorkspaceRepository adapter backed by AppState workspace_services.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::services::file_watcher::WatcherState;
use la_core::domain::{WorkspaceInfo, WorkspaceRepository, WorkspaceStatus};
use la_core::error::{AppError, Result};

/// Runtime workspace repository.
///
/// This adapter keeps the application layer independent from AppState while
/// using the unified workspace_services registry (P6 cleanup).
pub struct RuntimeWorkspaceRepository {
    workspace_services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,
    watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
}

impl RuntimeWorkspaceRepository {
    pub fn new(
        workspace_services: Arc<Mutex<HashMap<String, WorkspaceServiceRef>>>,
        watchers: Arc<Mutex<HashMap<String, WatcherState>>>,
    ) -> Self {
        Self {
            workspace_services,
            watchers,
        }
    }

    async fn build_info(&self, id: &str, path: PathBuf) -> Result<WorkspaceInfo> {
        let status = if self.watchers.lock().contains_key(id) {
            WorkspaceStatus::Watching
        } else {
            WorkspaceStatus::Ready
        };

        let service_clone = { self.workspace_services.lock().get(id).cloned() };
        let file_count = if let Some(service) = service_clone {
            service
                .metadata_store()
                .count_files()
                .await
                .unwrap_or(0)
                .max(0) as usize
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
        let services = { self.workspace_services.lock().clone() };
        let mut workspaces = Vec::with_capacity(services.len());
        for (id, service) in services {
            workspaces.push(
                self.build_info(&id, service.workspace_dir().clone())
                    .await?,
            );
        }
        Ok(workspaces)
    }

    async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>> {
        let service = { self.workspace_services.lock().get(id).cloned() };
        match service {
            Some(svc) => {
                self.build_info(id, svc.workspace_dir().clone())
                    .await
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    async fn delete_workspace(&self, id: &str) -> Result<()> {
        self.stop_watcher(id);

        let workspace_dir = {
            let service = self.workspace_services.lock().remove(id);
            let dir = service.as_ref().map(|s| s.workspace_dir().clone());
            if let Some(svc) = service {
                svc.metadata_store().close().await;
                svc.search_engine().close().await;
            }
            dir
        };

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
