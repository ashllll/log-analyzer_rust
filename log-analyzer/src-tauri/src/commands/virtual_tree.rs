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

use crate::storage::{ContentAddressableStorage, MetadataStore};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager};
use tracing::{debug, error, info};

/// File content response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub content: String,
    pub hash: String,
    pub size: usize,
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
#[command]
pub async fn read_file_by_hash(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    hash: String,
) -> Result<FileContentResponse, String> {
    info!(
        workspace_id = %workspaceId,
        hash = %hash,
        "Reading file by hash"
    );

    // Get workspace directory
    let workspace_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("workspaces")
        .join(&workspaceId);

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

    // Convert to UTF-8 string
    let content = String::from_utf8(content_bytes.clone())
        .map_err(|e| format!("File content is not valid UTF-8: {}", e))?;

    debug!(
        hash = %hash,
        size = content_bytes.len(),
        "Successfully read file content"
    );

    Ok(FileContentResponse {
        content,
        hash,
        size: content_bytes.len(),
    })
}

/// Virtual file tree node
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum VirtualTreeNode {
    #[serde(rename = "file")]
    File {
        name: String,
        path: String,
        hash: String,
        size: i64,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
    #[serde(rename = "archive")]
    Archive {
        name: String,
        path: String,
        hash: String,
        #[serde(rename = "archiveType")]
        archive_type: String,
        children: Vec<VirtualTreeNode>,
    },
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
#[command]
pub async fn get_virtual_file_tree(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
) -> Result<Vec<VirtualTreeNode>, String> {
    info!(workspace_id = %workspaceId, "Getting virtual file tree");

    // Get workspace directory
    let workspace_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("workspaces")
        .join(&workspaceId);

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

/// Build hierarchical tree structure from flat data
async fn build_tree_structure(
    archives: &[crate::storage::ArchiveMetadata],
    files: &[crate::storage::FileMetadata],
    metadata_store: &MetadataStore,
) -> Result<Vec<VirtualTreeNode>, String> {
    let mut tree = Vec::new();

    // Find root-level archives (no parent)
    let root_archives: Vec<_> = archives
        .iter()
        .filter(|a| a.parent_archive_id.is_none())
        .collect();

    // Find root-level files (no parent archive)
    let root_files: Vec<_> = files
        .iter()
        .filter(|f| f.parent_archive_id.is_none())
        .collect();

    // Add root archives with their children
    for archive in root_archives {
        let node = build_archive_node(archive, archives, files, metadata_store).await?;
        tree.push(node);
    }

    // Add root files
    for file in root_files {
        tree.push(VirtualTreeNode::File {
            name: file.original_name.clone(),
            path: file.virtual_path.clone(),
            hash: file.sha256_hash.clone(),
            size: file.size,
            mime_type: file.mime_type.clone(),
        });
    }

    Ok(tree)
}

/// Build archive node with its children recursively
#[allow(clippy::only_used_in_recursion)]
fn build_archive_node<'a>(
    archive: &'a crate::storage::ArchiveMetadata,
    all_archives: &'a [crate::storage::ArchiveMetadata],
    all_files: &'a [crate::storage::FileMetadata],
    metadata_store: &'a MetadataStore,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<VirtualTreeNode, String>> + Send + 'a>>
{
    Box::pin(async move {
        let mut children = Vec::new();

        // Find child archives
        let child_archives: Vec<_> = all_archives
            .iter()
            .filter(|a| a.parent_archive_id == Some(archive.id))
            .collect();

        // Find child files
        let child_files: Vec<_> = all_files
            .iter()
            .filter(|f| f.parent_archive_id == Some(archive.id))
            .collect();

        // Recursively build child archive nodes
        for child_archive in child_archives {
            let child_node =
                build_archive_node(child_archive, all_archives, all_files, metadata_store).await?;
            children.push(child_node);
        }

        // Add child files
        for file in child_files {
            children.push(VirtualTreeNode::File {
                name: file.original_name.clone(),
                path: file.virtual_path.clone(),
                hash: file.sha256_hash.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
            });
        }

        Ok(VirtualTreeNode::Archive {
            name: archive.original_name.clone(),
            path: archive.virtual_path.clone(),
            hash: archive.sha256_hash.clone(),
            archive_type: archive.archive_type.clone(),
            children,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_tree_node_serialization() {
        let file_node = VirtualTreeNode::File {
            name: "test.log".to_string(),
            path: "archive.zip/test.log".to_string(),
            hash: "abc123".to_string(),
            size: 1024,
            mime_type: Some("text/plain".to_string()),
        };

        let json = serde_json::to_string(&file_node).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"name\":\"test.log\""));
    }

    #[test]
    fn test_archive_node_serialization() {
        let archive_node = VirtualTreeNode::Archive {
            name: "archive.zip".to_string(),
            path: "archive.zip".to_string(),
            hash: "def456".to_string(),
            archive_type: "zip".to_string(),
            children: vec![],
        };

        let json = serde_json::to_string(&archive_node).unwrap();
        assert!(json.contains("\"type\":\"archive\""));
        assert!(json.contains("\"archiveType\":\"zip\""));
    }
}
