use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub const PRIMARY_WORKSPACE_DIR_NAME: &str = "workspaces";

pub fn preferred_workspace_dir(app: &AppHandle, workspace_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    preferred_workspace_dir_from_root(&app_data_dir, workspace_id)
}

pub fn resolve_workspace_dir(app: &AppHandle, workspace_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    resolve_workspace_dir_from_root(&app_data_dir, workspace_id)
}

/// FIX(HI-11): 拼接前验证 workspace_id，防止路径遍历
fn preferred_workspace_dir_from_root(
    app_data_dir: &Path,
    workspace_id: &str,
) -> Result<PathBuf, String> {
    crate::utils::validation::validate_workspace_id(workspace_id)?;
    Ok(app_data_dir
        .join(PRIMARY_WORKSPACE_DIR_NAME)
        .join(workspace_id))
}

fn resolve_workspace_dir_from_root(
    app_data_dir: &Path,
    workspace_id: &str,
) -> Result<PathBuf, String> {
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

        let resolved = resolve_workspace_dir_from_root(temp_dir.path(), "ws-1").unwrap();

        assert_eq!(resolved, current);
    }

    #[test]
    fn returns_current_layout_when_workspace_not_created_yet() {
        let temp_dir = tempfile::tempdir().unwrap();

        let resolved = resolve_workspace_dir_from_root(temp_dir.path(), "ws-1").unwrap();

        assert_eq!(
            resolved,
            temp_dir
                .path()
                .join(PRIMARY_WORKSPACE_DIR_NAME)
                .join("ws-1")
        );
    }

    #[test]
    fn rejects_path_traversal_in_workspace_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(resolve_workspace_dir_from_root(temp_dir.path(), "../etc/passwd").is_err());
        assert!(resolve_workspace_dir_from_root(temp_dir.path(), "ws-1/../../secret").is_err());
    }
}
