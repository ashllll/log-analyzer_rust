//! Legacy Format Detection Commands
//!
//! Provides Tauri commands for detecting and reporting legacy workspace formats.

use tauri::{command, AppHandle, Manager};
use tracing::{info, warn};

use crate::utils::legacy_detection::{
    generate_legacy_message, scan_legacy_workspaces, LegacyWorkspaceInfo,
};

/// Response for legacy workspace detection
#[derive(Debug, Clone, serde::Serialize)]
pub struct LegacyDetectionResponse {
    /// Whether any legacy workspaces were found
    pub has_legacy_workspaces: bool,
    /// Number of legacy workspaces detected
    pub count: usize,
    /// User-friendly message about the legacy workspaces
    pub message: String,
    /// List of legacy workspace IDs
    pub workspace_ids: Vec<String>,
}

/// Scan for legacy workspace formats
///
/// This command checks the indices directory for old `.idx.gz` and `.idx` files
/// that indicate workspaces using the deprecated format.
///
/// # Returns
///
/// A response containing information about detected legacy workspaces
#[command]
pub fn scan_legacy_formats(app: AppHandle) -> Result<LegacyDetectionResponse, String> {
    info!("Scanning for legacy workspace formats");

    // Get indices directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let indices_dir = app_data_dir.join("indices");

    // Scan for legacy workspaces
    let legacy_workspaces = scan_legacy_workspaces(&indices_dir);

    let has_legacy = !legacy_workspaces.is_empty();
    let count = legacy_workspaces.len();

    if has_legacy {
        warn!("Detected {} legacy workspace(s) with old format", count);
    } else {
        info!("No legacy workspaces detected");
    }

    // Generate user-friendly message
    let message = generate_legacy_message(&legacy_workspaces);

    // Extract workspace IDs
    let workspace_ids: Vec<String> = legacy_workspaces
        .iter()
        .map(|w| w.workspace_id.clone())
        .collect();

    Ok(LegacyDetectionResponse {
        has_legacy_workspaces: has_legacy,
        count,
        message,
        workspace_ids,
    })
}

/// Get detailed information about a specific legacy workspace
///
/// # Arguments
///
/// * `workspace_id` - The workspace ID to check
///
/// # Returns
///
/// Optional information about the legacy workspace
#[command]
pub fn get_legacy_workspace_info(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
) -> Result<Option<LegacyWorkspaceInfo>, String> {
    info!("Checking legacy format for workspace: {}", workspaceId);

    // Get indices directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let indices_dir = app_data_dir.join("indices");

    // Check for legacy format
    let legacy_info =
        crate::utils::legacy_detection::check_workspace_legacy_format(&workspaceId, &indices_dir);

    if legacy_info.is_some() {
        warn!("Workspace {} uses legacy format", workspaceId);
    }

    Ok(legacy_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_detection_response_serialization() {
        let response = LegacyDetectionResponse {
            has_legacy_workspaces: true,
            count: 2,
            message: "Test message".to_string(),
            workspace_ids: vec!["workspace1".to_string(), "workspace2".to_string()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("has_legacy_workspaces"));
        assert!(json.contains("workspace1"));
    }
}
