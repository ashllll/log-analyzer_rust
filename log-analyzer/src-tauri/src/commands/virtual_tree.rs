//! Virtual File Tree Commands
//!
//! Provides commands for accessing the virtual file tree structure
//! and retrieving file content by hash from the Content-Addressable Storage.
//!
//! # Commands
//!
//! - `read_file_by_hash`: Read file content using SHA-256 hash
//! - `get_virtual_file_tree`: Get hierarchical file tree structure
//!
//! # Requirements
//!
//! Validates: Requirements 1.4, 4.2
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use la_storage::{ContentAddressableStorage, MetadataStore};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tracing::{debug, error, info};

use crate::application::virtual_tree::{build_tree_structure, VirtualTreeNode};
use crate::utils::validation::validate_workspace_id;
use crate::utils::workspace_paths::resolve_workspace_dir;

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

/// Read file content by SHA-256 hash
///
/// This command retrieves file content from the Content-Addressable Storage
/// using the file's SHA-256 hash. This is the primary method for accessing
/// files in the CAS-based system.
///
/// # Arguments
///
/// * `workspace_id` - ID of the workspace containing the file
/// * `hash` - SHA-256 hash of the file to read
///
/// # Returns
///
/// File content as a UTF-8 string along with metadata
///
/// # Errors
///
/// Returns error if:
/// - Workspace directory cannot be determined
/// - File hash doesn't exist in CAS
/// - File cannot be read
/// - Content is not valid UTF-8
///
/// # Requirements
///
/// Validates: Requirements 1.4
///
/// # Example
///
/// ```typescript
/// const content = await invoke('read_file_by_hash', {
///   workspaceId: 'workspace_123',
///   hash: 'a3f2e1d4c5b6a7...'
/// });
/// ```
#[tauri::command]
pub async fn read_file_by_hash(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    hash: String,
) -> Result<FileContentResponse, String> {
    validate_workspace_id(&workspaceId)?;
    validate_file_hash(&hash)?;

    info!(
        workspace_id = %workspaceId,
        hash = %hash,
        "Reading file by hash"
    );

    // Get workspace directory
    let workspace_dir = resolve_workspace_dir(&app, &workspaceId)?;

    if !workspace_dir.exists() {
        error!(workspace_id = %workspaceId, "Workspace directory not found");
        return Err(format!("Workspace not found: {}", workspaceId));
    }

    // Initialize CAS
    let cas = ContentAddressableStorage::new(workspace_dir);

    // Check if file exists
    if !cas.exists(&hash) {
        error!(hash = %hash, "File not found in CAS");
        return Err(format!("File not found: {}", hash));
    }

    // Read content
    let content_bytes = cas
        .read_content(&hash)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // 先记录大小，再消费 content_bytes 避免完整克隆
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

/// Get virtual file tree structure
///
/// This command queries the metadata store to build a hierarchical tree
/// structure representing all files and nested archives in the workspace.
///
/// # Arguments
///
/// * `workspace_id` - ID of the workspace
///
/// # Returns
///
/// Root-level tree nodes (files and archives)
///
/// # Errors
///
/// Returns error if:
/// - Workspace directory cannot be determined
/// - Metadata store cannot be opened
/// - Database query fails
///
/// # Requirements
///
/// Validates: Requirements 4.2
///
/// # Example
///
/// ```typescript
/// const tree = await invoke('get_virtual_file_tree', {
///   workspaceId: 'workspace_123'
/// });
/// ```
#[tauri::command]
pub async fn get_virtual_file_tree(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
) -> Result<Vec<VirtualTreeNode>, String> {
    info!(workspace_id = %workspaceId, "Getting virtual file tree");

    // Get workspace directory
    let workspace_dir = resolve_workspace_dir(&app, &workspaceId)?;

    if !workspace_dir.exists() {
        error!(workspace_id = %workspaceId, "Workspace directory not found");
        return Err(format!("Workspace not found: {}", workspaceId));
    }

    // Open metadata store
    let metadata_store = MetadataStore::new(&workspace_dir)
        .await
        .map_err(|e| format!("Failed to open metadata store: {}", e))?;

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
    let tree = build_tree_structure(&archives, &all_files, &metadata_store).await?;

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
