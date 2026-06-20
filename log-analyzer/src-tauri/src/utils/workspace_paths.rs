use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub const PRIMARY_WORKSPACE_DIR_NAME: &str = "workspaces";

/// Generate a workspace ID from a human-readable name.
///
/// Slugifies the name (lowercase, special chars → '-'), appends a short UUID suffix,
/// and enforces a max total length of 50 chars. Format: `ws-{slug}-{8-char-uuid}`.
pub fn build_workspace_id(name: &str) -> String {
    let mut slug = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "workspace" } else { slug };
    let suffix = uuid::Uuid::new_v4().to_string();
    let suffix = &suffix[..8];
    let max_slug_len = 50usize.saturating_sub("ws-".len() + "-".len() + suffix.len());
    let mut slug = slug.chars().take(max_slug_len).collect::<String>();
    slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        slug = "workspace".to_string();
    }

    format!("ws-{slug}-{suffix}")
}

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
pub fn preferred_workspace_dir_from_root(
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

    #[test]
    fn build_workspace_id_slugifies_name() {
        let id = build_workspace_id("My Project");
        assert!(id.starts_with("ws-my-project-"));
        // ID format: "ws-{slug}-{8-char-uuid}"
        assert!(id.len() >= 21, "ID too short: {id}");
        assert!(id.len() <= 50, "ID too long: {id}");
    }

    #[test]
    fn build_workspace_id_handles_empty_name() {
        let id = build_workspace_id("");
        assert!(id.starts_with("ws-workspace-"));
        assert!(id.len() >= 21, "ID too short: {id}");
    }

    #[test]
    fn build_workspace_id_handles_special_chars() {
        let id = build_workspace_id("Hello! World?");
        // '!' → '-', '?' → '-', spaces → '-', then consecutive dashes collapsed
        assert!(id.starts_with("ws-hello-world-") || id.starts_with("ws-hello--world-"));
        assert!(!id.contains('!'));
        assert!(!id.contains('?'));
    }

    #[test]
    fn build_workspace_id_handles_very_long_name() {
        let id = build_workspace_id(
            "a very long workspace name that exceeds fifty characters total for the slug portion",
        );
        assert!(id.starts_with("ws-"));
        assert!(id.len() <= 50, "ID should not exceed 50 chars: {id}");
    }
}
