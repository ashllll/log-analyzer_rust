//! Virtual file tree command interface adapters.

use tauri::AppHandle;

use crate::commands::virtual_tree::{FileContentResponse, VirtualTreeNode};

#[tauri::command]
pub async fn read_file_by_hash(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    hash: String,
) -> Result<FileContentResponse, String> {
    crate::commands::virtual_tree::read_file_by_hash(app, workspaceId, hash).await
}

#[tauri::command]
pub async fn get_virtual_file_tree(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
) -> Result<Vec<VirtualTreeNode>, String> {
    crate::commands::virtual_tree::get_virtual_file_tree(app, workspaceId).await
}
