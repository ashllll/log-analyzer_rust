//! Virtual File Tree Commands
//!
//! Provides commands for accessing the virtual file tree structure
//! and retrieving file content by hash from the Content-Addressable Storage.
//!
//! # P7 Consolidation
//!
//! Both commands now go through [`require_cas_workspace`], using the
//! pre-assembled WorkspaceService instead of creating standalone CAS /
//! MetadataStore instances. This closes the last remaining bypass of the
//! WorkspaceService seam in the command layer.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tracing::{debug, error, info};

use crate::application::virtual_tree::{build_tree_structure, VirtualTreeNode};
use crate::models::AppState;

/// File content response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub content: String,
    pub hash: String,
    pub size: usize,
}

fn validate_file_hash(hash: &str) -> Result<(), String> {
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid file hash format".to_string());
    }
    Ok(())
}

/// Read file content by SHA-256 hash.
///
/// Uses the workspace's pre-assembled CAS instance (via WorkspaceService),
/// rather than creating a standalone ContentAddressableStorage.
#[tauri::command]
pub async fn read_file_by_hash(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    hash: String,
    state: State<'_, AppState>,
) -> Result<FileContentResponse, String> {
    validate_file_hash(&hash)?;

    info!(
        workspace_id = %workspaceId,
        hash = %hash,
        "Reading file by hash"
    );

    // ── Acquire workspace service (validates ID, resolves dir, checks CAS format) ──
    let (service, _workspace_dir) =
        crate::utils::workspace_guard::require_cas_workspace(&app, &state, &workspaceId)
            .await
            .map_err(|e| e.to_string())?;

    let cas = service.cas();

    if !cas.exists(&hash) {
        error!(hash = %hash, "File not found in CAS");
        return Err(format!("File not found: {}", hash));
    }

    let content_bytes = cas
        .read_content(&hash)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let size = content_bytes.len();
    let content = String::from_utf8(content_bytes)
        .map_err(|e| format!("File content is not valid UTF-8: {}", e))?;

    debug!(
        hash = %hash,
        size,
        "Successfully read file content"
    );

    Ok(FileContentResponse {
        content,
        hash,
        size,
    })
}

/// Get virtual file tree structure.
///
/// Uses the workspace's pre-assembled MetadataStore (via WorkspaceService),
/// rather than opening a new standalone connection.
#[tauri::command]
pub async fn get_virtual_file_tree(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<Vec<VirtualTreeNode>, String> {
    info!(
        workspace_id = %workspaceId,
        "Getting virtual file tree"
    );

    // ── Acquire workspace service ──
    let (service, _workspace_dir) =
        crate::utils::workspace_guard::require_cas_workspace(&app, &state, &workspaceId)
            .await
            .map_err(|e| e.to_string())?;

    let metadata_store = service.metadata_store();

    // Get all archives and files
    let archives = metadata_store
        .get_all_archives()
        .await
        .map_err(|e| format!("Failed to get archives: {}", e))?;

    let all_files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to get files: {}", e))?;

    // Build tree structure
    let tree = build_tree_structure(&archives, &all_files, metadata_store.as_ref()).await?;

    info!(
        workspace_id = %workspaceId,
        node_count = tree.len(),
        "Successfully built virtual file tree"
    );

    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_hash_accepts_sha256() {
        let hash = "a3".repeat(32);
        assert!(validate_file_hash(&hash).is_ok());
    }

    #[test]
    fn test_validate_file_hash_rejects_path_like_values() {
        assert!(validate_file_hash("../etc/passwd").is_err());
        assert!(validate_file_hash("aa/tmp/escaped").is_err());
        assert!(validate_file_hash("short").is_err());
    }
}
