//! TaskScheduler — domain trait for task lifecycle management.
//!
//! Abstracts long-running task operations (import, export, watch) behind
//! a simple interface. Infrastructure adapters wrap the actor-based
//! `TaskManager` for production use.

use std::fmt;

use async_trait::async_trait;

use crate::error::Result;

/// Opaque handle to a scheduled task.
///
/// Returned by [`TaskScheduler::create`] and passed to subsequent
/// `update` / `complete` / `fail` / `cancel` calls.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub(crate) id: String,
}

impl TaskHandle {
    /// Create a new task handle from an id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Return the task id.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for TaskHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TaskHandle({})", self.id)
    }
}

/// Domain trait for scheduling and tracking long-running tasks.
///
/// # Implementors
///
/// - `TaskManagerAdapter` (production) — wraps the actor-based `TaskManager`.
/// - Mock implementations for unit testing `ImportUseCase`.
#[async_trait]
pub trait TaskScheduler: Send + Sync {
    /// Create a new task and return its handle.
    async fn create(
        &self,
        id: &str,
        task_type: &str,
        target: &str,
        workspace_id: Option<&str>,
    ) -> Result<TaskHandle>;

    /// Update task progress (0-100) with a status message.
    async fn update(&self, handle: &TaskHandle, progress: u8, message: &str) -> Result<()>;

    /// Mark the task as successfully completed.
    async fn complete(&self, handle: &TaskHandle) -> Result<()>;

    /// Mark the task as failed with an error message.
    async fn fail(&self, handle: &TaskHandle, error: &str) -> Result<()>;

    /// Cancel a running task.
    async fn cancel(&self, handle: &TaskHandle) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock scheduler for unit testing.
    struct MockTaskScheduler;

    #[async_trait]
    impl TaskScheduler for MockTaskScheduler {
        async fn create(
            &self,
            id: &str,
            _task_type: &str,
            _target: &str,
            _workspace_id: Option<&str>,
        ) -> Result<TaskHandle> {
            Ok(TaskHandle::new(id))
        }

        async fn update(&self, _handle: &TaskHandle, _progress: u8, _message: &str) -> Result<()> {
            Ok(())
        }

        async fn complete(&self, _handle: &TaskHandle) -> Result<()> {
            Ok(())
        }

        async fn fail(&self, _handle: &TaskHandle, _error: &str) -> Result<()> {
            Ok(())
        }

        async fn cancel(&self, _handle: &TaskHandle) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_create_and_complete() {
        let scheduler = MockTaskScheduler;
        let handle = scheduler
            .create("task-1", "Import", "test.zip", Some("ws-1"))
            .await
            .unwrap();
        assert_eq!(handle.id(), "task-1");

        scheduler.update(&handle, 50, "halfway").await.unwrap();
        scheduler.complete(&handle).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_fail_and_cancel() {
        let scheduler = MockTaskScheduler;
        let handle = scheduler
            .create("task-2", "Export", "results.csv", None)
            .await
            .unwrap();

        scheduler.fail(&handle, "disk full").await.unwrap();
        scheduler.cancel(&handle).await.unwrap();
    }
}
