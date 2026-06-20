//! TauriWorkspacePaths — adapter: wraps Tauri AppHandle for workspace dir resolution.

use std::path::PathBuf;

use la_core::domain::WorkspacePaths;
use tauri::Manager;

use crate::utils::workspace_paths::preferred_workspace_dir_from_root;

/// Adapter that resolves workspace directories via Tauri's path resolver.
pub struct TauriWorkspacePaths {
    app_data_dir: PathBuf,
}

impl TauriWorkspacePaths {
    /// Create a new adapter from a Tauri AppHandle.
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {e}"))?;
        Ok(Self { app_data_dir })
    }
}

impl WorkspacePaths for TauriWorkspacePaths {
    fn workspace_data_dir(&self, workspace_id: &str) -> std::result::Result<PathBuf, String> {
        preferred_workspace_dir_from_root(&self.app_data_dir, workspace_id)
    }
}
