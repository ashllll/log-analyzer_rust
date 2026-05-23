//! WorkspaceUseCase — application-layer workspace lifecycle orchestration.

use std::sync::Arc;

use la_core::domain::{WorkspaceInfo, WorkspaceRepository};
use la_core::error::Result;

/// Application use case for workspace queries and lifecycle operations.
pub struct WorkspaceUseCase<R>
where
    R: WorkspaceRepository + 'static,
{
    workspaces: Arc<R>,
}

impl<R> WorkspaceUseCase<R>
where
    R: WorkspaceRepository,
{
    pub fn new(workspaces: Arc<R>) -> Self {
        Self { workspaces }
    }

    pub async fn list(&self) -> Result<Vec<WorkspaceInfo>> {
        self.workspaces.list_workspaces().await
    }

    pub async fn get(&self, id: &str) -> Result<Option<WorkspaceInfo>> {
        self.workspaces.get_workspace(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        self.workspaces.delete_workspace(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use la_core::domain::WorkspaceStatus;
    use parking_lot::Mutex;

    struct StubWorkspaceRepository {
        deleted: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl WorkspaceRepository for StubWorkspaceRepository {
        async fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
            Ok(vec![WorkspaceInfo {
                id: "ws-1".to_string(),
                name: "Workspace 1".to_string(),
                path: "/tmp/ws-1".to_string(),
                status: WorkspaceStatus::Ready,
                file_count: 3,
            }])
        }

        async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>> {
            if id == "ws-1" {
                Ok(Some(WorkspaceInfo {
                    id: id.to_string(),
                    name: "Workspace 1".to_string(),
                    path: "/tmp/ws-1".to_string(),
                    status: WorkspaceStatus::Ready,
                    file_count: 3,
                }))
            } else {
                Ok(None)
            }
        }

        async fn delete_workspace(&self, id: &str) -> Result<()> {
            self.deleted.lock().push(id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn workspace_use_case_delegates_to_repository() {
        let repo = Arc::new(StubWorkspaceRepository {
            deleted: Mutex::new(Vec::new()),
        });
        let use_case = WorkspaceUseCase::new(Arc::clone(&repo));

        let workspaces = use_case.list().await.unwrap();
        assert_eq!(workspaces.len(), 1);

        let workspace = use_case.get("ws-1").await.unwrap().unwrap();
        assert_eq!(workspace.file_count, 3);

        use_case.delete("ws-1").await.unwrap();
        assert_eq!(repo.deleted.lock().as_slice(), ["ws-1"]);
    }
}
