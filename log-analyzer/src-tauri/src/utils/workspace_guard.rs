//! Workspace guard — shared workspace acquisition helper.
//!
//! Consolidates the "validate → resolve → check CAS format → get/create service"
//! preamble that was duplicated across 5 commands. Also eliminates the last
//! remaining bypass of the WorkspaceService seam (virtual_tree creating its own
//! CAS/MetadataStore instances).
//!
//! # Usage
//!
//! ```rust,ignore
//! let (service, workspace_dir) = require_cas_workspace(&app, &state, &workspace_id).await?;
//! // Use service.cas(), service.metadata_store(), service.search_engine(), etc.
//! ```

use std::path::PathBuf;

use tauri::AppHandle;

use crate::application::workspace_service::WorkspaceServiceRef;
use crate::infrastructure::workspace_service_factory::get_or_create_workspace_service;
use crate::models::AppState;
use crate::utils::{validate_workspace_id, workspace_paths::resolve_workspace_dir};
use la_core::error::CommandError;

/// Validate workspace ID, resolve directory, check CAS format, and acquire the service.
///
/// Returns the pre-assembled [`WorkspaceServiceRef`] and its workspace directory.
/// Every command that needs a ready-to-use workspace should call this instead of
/// duplicating the preamble or creating standalone CAS/MetadataStore instances.
pub async fn require_cas_workspace(
    app: &AppHandle,
    state: &AppState,
    workspace_id: &str,
) -> Result<(WorkspaceServiceRef, PathBuf), CommandError> {
    // 1. Validate workspace ID
    validate_workspace_id(workspace_id).map_err(|e| {
        CommandError::new("VALIDATION_ERROR", e).with_help("Workspace ID format is invalid")
    })?;

    // 2. Resolve workspace directory
    let workspace_dir = resolve_workspace_dir(app, workspace_id).map_err(|e| {
        CommandError::new("NOT_FOUND", e).with_help("The workspace may have been deleted")
    })?;

    // 3. Check workspace directory exists
    if !workspace_dir.exists() {
        return Err(CommandError::new("NOT_FOUND", "Workspace not found")
            .with_help("The workspace may have been deleted or moved. Try re-importing"));
    }

    // 4. Check CAS format (metadata.db + objects/)
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");
    if !metadata_db.exists() || !objects_dir.exists() {
        return Err(
            CommandError::new("FORMAT_ERROR", "Workspace is not in CAS format")
                .with_help("Please create a new workspace with the current version"),
        );
    }

    // 5. Get or create the WorkspaceService
    let service = get_or_create_workspace_service(app, state, workspace_id, &workspace_dir)
        .await
        .map_err(|e| {
            CommandError::new(
                "RUNTIME_ERROR",
                format!("Failed to initialize workspace: {e}"),
            )
        })?;

    Ok((service, workspace_dir))
}
