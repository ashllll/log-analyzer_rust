//! Migration Commands
//!
//! Tauri commands for workspace migration from traditional format to CAS format.

use crate::migration::{detect_workspace_format, migrate_workspace_to_cas, needs_migration, MigrationReport, WorkspaceFormat};
use tauri::{command, AppHandle};

/// Detect the format of a workspace
///
/// # Arguments
///
/// * `workspaceId` - The workspace ID to check
///
/// # Returns
///
/// A string representing the workspace format: "traditional", "cas", or "unknown"
#[command]
pub async fn detect_workspace_format_cmd(
    #[allow(non_snake_case)] workspaceId: String,
    app: AppHandle,
) -> Result<String, String> {
    let format = detect_workspace_format(&workspaceId, &app).map_err(|e| e.to_string())?;

    let format_str = match format {
        WorkspaceFormat::Traditional => "traditional",
        WorkspaceFormat::CAS => "cas",
        WorkspaceFormat::Unknown => "unknown",
    };

    Ok(format_str.to_string())
}

/// Check if a workspace needs migration
///
/// # Arguments
///
/// * `workspaceId` - The workspace ID to check
///
/// # Returns
///
/// `true` if the workspace needs migration, `false` otherwise
#[command]
pub async fn needs_migration_cmd(
    #[allow(non_snake_case)] workspaceId: String,
    app: AppHandle,
) -> Result<bool, String> {
    needs_migration(&workspaceId, &app).map_err(|e| e.to_string())
}

/// Migrate a workspace from traditional format to CAS format
///
/// # Arguments
///
/// * `workspaceId` - The workspace ID to migrate
///
/// # Returns
///
/// A migration report with statistics and results
#[command]
pub async fn migrate_workspace_cmd(
    #[allow(non_snake_case)] workspaceId: String,
    app: AppHandle,
) -> Result<MigrationReport, String> {
    migrate_workspace_to_cas(&workspaceId, &app)
        .await
        .map_err(|e| e.to_string())
}
