use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub const PRIMARY_WORKSPACE_DIR_NAME: &str = "workspaces";

pub fn preferred_workspace_dir(app: &AppHandle, workspace_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    Ok(preferred_workspace_dir_from_root(
        &app_data_dir,
        workspace_id,
    ))
}

pub fn resolve_workspace_dir(app: &AppHandle, workspace_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    Ok(resolve_workspace_dir_from_root(&app_data_dir, workspace_id))
}

fn preferred_workspace_dir_from_root(app_data_dir: &Path, workspace_id: &str) -> PathBuf {
    app_data_dir
        .join(PRIMARY_WORKSPACE_DIR_NAME)
        .join(workspace_id)
}

fn resolve_workspace_dir_from_root(app_data_dir: &Path, workspace_id: &str) -> PathBuf {
    preferred_workspace_dir_from_root(app_data_dir, workspace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_current_workspace_directory_layout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let current = temp_dir
            .path()
            .join(PRIMARY_WORKSPACE_DIR_NAME)
            .join("ws-1");
        std::fs::create_dir_all(&current).unwrap();

        let resolved = resolve_workspace_dir_from_root(temp_dir.path(), "ws-1");

        assert_eq!(resolved, current);
    }

    #[test]
    fn returns_current_layout_when_workspace_not_created_yet() {
        let temp_dir = tempfile::tempdir().unwrap();

        let resolved = resolve_workspace_dir_from_root(temp_dir.path(), "ws-1");

        assert_eq!(
            resolved,
            temp_dir
                .path()
                .join(PRIMARY_WORKSPACE_DIR_NAME)
                .join("ws-1")
        );
    }
}
