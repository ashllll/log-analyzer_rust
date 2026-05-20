//! WorkspaceRepository — workspace lifecycle management.
//!
//! Abstracts workspace creation, listing, and deletion.

use async_trait::async_trait;

use crate::error::Result;

/// Summary info about a workspace, decoupled from storage details.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: WorkspaceStatus,
    pub file_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceStatus {
    Ready,
    Processing,
    Watching,
    Failed,
}

/// Repository for workspace persistence.
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    /// List all known workspaces.
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>>;

    /// Get a single workspace by id.
    async fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceInfo>>;

    /// Delete a workspace and all associated data.
    async fn delete_workspace(&self, id: &str) -> Result<()>;
}
