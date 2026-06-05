//! TaskScheduler adapter — wraps the actor-based `TaskManager`.
//!
//! Delegates task lifecycle operations (create / update / complete / fail /
//! cancel) to the existing `TaskManager` actor.

use std::sync::Arc;

use async_trait::async_trait;

use la_core::domain::{TaskHandle, TaskScheduler};
use la_core::error::{AppError, Result};

use crate::task_manager::{TaskManager, TaskManagerError, TaskStatus};

/// Adapter that wraps `TaskManager` behind the `TaskScheduler` trait.
///
/// Constructor-injected with `Arc<TaskManager>` — the caller (interfaces layer)
/// is responsible for extracting the manager from `AppState`.
pub struct TaskManagerAdapter {
    manager: Arc<TaskManager>,
}

impl TaskManagerAdapter {
    /// Create a new adapter.
    pub fn new(manager: Arc<TaskManager>) -> Self {
        Self { manager }
    }
}

/// Map `TaskManagerError` to a domain-level `AppError`.
fn map_error(e: TaskManagerError) -> AppError {
    AppError::internal_error(format!("Task manager error: {e}"))
}

#[async_trait]
impl TaskScheduler for TaskManagerAdapter {
    async fn create(
        &self,
        id: &str,
        task_type: &str,
        target: &str,
        workspace_id: Option<&str>,
    ) -> Result<TaskHandle> {
        self.manager
            .create_task_async(
                id.to_string(),
                task_type.to_string(),
                target.to_string(),
                workspace_id.map(|s| s.to_string()),
            )
            .await
            .map_err(map_error)?;

        Ok(TaskHandle::new(id))
    }

    async fn update(&self, handle: &TaskHandle, progress: u8, message: &str) -> Result<()> {
        self.manager
            .update_task_async(
                handle.id(),
                progress,
                message.to_string(),
                TaskStatus::Running,
            )
            .await
            .map_err(map_error)?;

        Ok(())
    }

    async fn complete(&self, handle: &TaskHandle) -> Result<()> {
        self.manager
            .update_task_async(handle.id(), 100, "Done".to_string(), TaskStatus::Completed)
            .await
            .map_err(map_error)?;

        Ok(())
    }

    async fn fail(&self, handle: &TaskHandle, error: &str) -> Result<()> {
        self.manager
            .update_task_async(
                handle.id(),
                0,
                format!("Error: {error}"),
                TaskStatus::Failed,
            )
            .await
            .map_err(map_error)?;

        Ok(())
    }

    async fn cancel(&self, handle: &TaskHandle) -> Result<()> {
        self.manager
            .update_task_async(handle.id(), 0, "Cancelled".to_string(), TaskStatus::Stopped)
            .await
            .map_err(map_error)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_handle_display() {
        let handle = TaskHandle::new("import-1");
        assert_eq!(handle.id(), "import-1");
        assert_eq!(handle.to_string(), "TaskHandle(import-1)");
    }
}
