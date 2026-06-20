//! WorkspacePaths — resolve workspace data directories.
//!
//! Abstracts Tauri's `app_handle.path().app_data_dir()` behind a trait so
//! infrastructure code doesn't depend on the Tauri framework directly.

use std::path::PathBuf;

/// Resolves workspace storage directories from the application data root.
pub trait WorkspacePaths: Send + Sync {
    /// Returns the data directory for a given workspace.
    fn workspace_data_dir(&self, workspace_id: &str) -> std::result::Result<PathBuf, String>;
}
