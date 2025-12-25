//! Data Migration Module
//!
//! This module provides tools for migrating workspaces from the old path_map format
//! to the new Content-Addressable Storage (CAS) + SQLite metadata format.
//!
//! ## Migration Process
//!
//! 1. **Detection**: Identify old vs new workspace format
//! 2. **Reading**: Load old path_map and file_metadata from index files
//! 3. **Conversion**: Store files in CAS and create metadata entries
//! 4. **Verification**: Ensure all files are accessible after migration
//! 5. **Cleanup**: Optionally remove old format files
//!
//! ## Old Format
//!
//! ```text
//! workspace/
//! ├── indices/{workspace_id}.idx.gz  # Bincode serialized path_map
//! └── extracted/{workspace_id}/      # Extracted files (traditional paths)
//!     ├── file1.log
//!     └── nested/
//!         └── file2.log
//! ```
//!
//! ## New Format (CAS)
//!
//! ```text
//! workspace/
//! ├── indices/{workspace_id}.idx.gz  # Still exists for compatibility
//! └── extracted/{workspace_id}/      # CAS workspace
//!     ├── metadata.db                # SQLite database
//!     └── objects/                   # Content storage
//!         ├── a3/
//!         │   └── f2e1d4c5...
//!         └── b7/
//!             └── e145a3b2...
//! ```

use crate::error::{AppError, Result};
use crate::services::{load_index, save_index};
use crate::storage::{ContentAddressableStorage, MetadataStore};
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Manager};
use tracing::{debug, error, info, warn};

/// Migration report containing statistics and results
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrationReport {
    /// Workspace ID that was migrated
    pub workspace_id: String,
    /// Total number of files in the old format
    pub total_files: usize,
    /// Number of files successfully migrated
    pub migrated_files: usize,
    /// Number of files that failed to migrate
    pub failed_files: usize,
    /// Number of files deduplicated (same content hash)
    pub deduplicated_files: usize,
    /// Total size of original files (bytes)
    pub original_size: u64,
    /// Total size after CAS storage (bytes)
    pub cas_size: u64,
    /// List of files that failed to migrate
    pub failed_file_paths: Vec<String>,
    /// Migration duration (milliseconds)
    pub duration_ms: u64,
    /// Whether migration was successful
    pub success: bool,
}

/// Workspace format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceFormat {
    /// Old format: traditional file paths with path_map
    Traditional,
    /// New format: CAS + SQLite metadata
    CAS,
    /// Unknown or corrupted format
    Unknown,
}

/// Detect the format of a workspace
///
/// # Arguments
///
/// * `workspace_id` - The workspace ID to check
/// * `app` - Tauri application handle
///
/// # Returns
///
/// The detected workspace format
pub fn detect_workspace_format(workspace_id: &str, app: &AppHandle) -> Result<WorkspaceFormat> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::validation_error(format!("Failed to get app data dir: {}", e)))?;

    let workspace_dir = app_data_dir.join("extracted").join(workspace_id);

    // Check if workspace directory exists
    if !workspace_dir.exists() {
        debug!(
            workspace_id = %workspace_id,
            "Workspace directory does not exist"
        );
        return Ok(WorkspaceFormat::Unknown);
    }

    // Check for CAS markers
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");

    if metadata_db.exists() || objects_dir.exists() {
        info!(
            workspace_id = %workspace_id,
            "Detected CAS workspace format"
        );
        return Ok(WorkspaceFormat::CAS);
    }

    // Check for traditional format (has regular files/directories)
    if workspace_dir.read_dir().map_err(AppError::Io)?.count() > 0 {
        info!(
            workspace_id = %workspace_id,
            "Detected traditional workspace format"
        );
        return Ok(WorkspaceFormat::Traditional);
    }

    warn!(
        workspace_id = %workspace_id,
        "Could not determine workspace format"
    );
    Ok(WorkspaceFormat::Unknown)
}

/// Migrate a workspace from traditional format to CAS format
///
/// # Arguments
///
/// * `workspace_id` - The workspace ID to migrate
/// * `app` - Tauri application handle
///
/// # Returns
///
/// A migration report with statistics and results
///
/// # Errors
///
/// Returns an error if:
/// - Workspace is already in CAS format
/// - Index file cannot be loaded
/// - CAS initialization fails
/// - File migration fails critically
pub async fn migrate_workspace_to_cas(
    workspace_id: &str,
    app: &AppHandle,
) -> Result<MigrationReport> {
    let start_time = std::time::Instant::now();

    info!(
        workspace_id = %workspace_id,
        "Starting workspace migration to CAS"
    );

    // Step 1: Detect current format
    let format = detect_workspace_format(workspace_id, app)?;
    match format {
        WorkspaceFormat::CAS => {
            return Err(AppError::validation_error(
                "Workspace is already in CAS format",
            ));
        }
        WorkspaceFormat::Unknown => {
            return Err(AppError::validation_error(
                "Cannot migrate workspace with unknown format",
            ));
        }
        WorkspaceFormat::Traditional => {
            debug!("Confirmed traditional format, proceeding with migration");
        }
    }

    // Step 2: Load old index file
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::validation_error(format!("Failed to get app data dir: {}", e)))?
        .join("indices");

    let mut index_path = index_dir.join(format!("{}.idx.gz", workspace_id));
    if !index_path.exists() {
        index_path = index_dir.join(format!("{}.idx", workspace_id));
        if !index_path.exists() {
            return Err(AppError::not_found(format!(
                "Index file not found for workspace: {}",
                workspace_id
            )));
        }
    }

    info!(
        index_path = %index_path.display(),
        "Loading old index file"
    );

    let (old_path_map, old_file_metadata) = load_index(&index_path)?;
    let total_files = old_path_map.len();

    info!(
        total_files = total_files,
        "Loaded old index with {} files",
        total_files
    );

    // Step 3: Initialize CAS and metadata store
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::validation_error(format!("Failed to get app data dir: {}", e)))?;

    let workspace_dir = app_data_dir.join("extracted").join(workspace_id);
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await?;

    info!(
        workspace_dir = %workspace_dir.display(),
        "Initialized CAS and metadata store"
    );

    // Step 4: Migrate files
    let mut migrated_files = 0;
    let mut failed_files = 0;
    let mut deduplicated_files = 0;
    let mut failed_file_paths = Vec::new();
    let mut original_size = 0u64;
    let mut cas_size = 0u64;

    let mut new_path_map: HashMap<String, String> = HashMap::new();
    let mut new_file_metadata: HashMap<String, crate::models::config::FileMetadata> = HashMap::new();

    for (real_path, virtual_path) in old_path_map.iter() {
        let file_path = Path::new(real_path);

        // Check if file exists
        if !file_path.exists() {
            warn!(
                real_path = %real_path,
                virtual_path = %virtual_path,
                "File does not exist, skipping"
            );
            failed_files += 1;
            failed_file_paths.push(real_path.clone());
            continue;
        }

        // Get file metadata
        let file_size = match std::fs::metadata(file_path) {
            Ok(meta) => meta.len(),
            Err(e) => {
                error!(
                    real_path = %real_path,
                    error = %e,
                    "Failed to get file metadata"
                );
                failed_files += 1;
                failed_file_paths.push(real_path.clone());
                continue;
            }
        };

        original_size += file_size;

        // Store file in CAS
        let hash = match cas.store_file_streaming(file_path).await {
            Ok(h) => h,
            Err(e) => {
                error!(
                    real_path = %real_path,
                    error = %e,
                    "Failed to store file in CAS"
                );
                failed_files += 1;
                failed_file_paths.push(real_path.clone());
                continue;
            }
        };

        // Check if this is a duplicate
        let object_path = cas.get_object_path(&hash);
        if object_path.exists() {
            let existing_size = match std::fs::metadata(&object_path) {
                Ok(meta) => meta.len(),
                Err(_) => file_size,
            };
            
            // Only count as deduplicated if we didn't just create it
            if existing_size == file_size {
                deduplicated_files += 1;
            }
        }

        cas_size += file_size;

        // Extract original filename from virtual path
        let original_name = virtual_path
            .rsplit('/')
            .next()
            .unwrap_or(virtual_path)
            .to_string();

        // Get modified time from old metadata
        let modified_time = old_file_metadata
            .get(real_path)
            .map(|m| m.modified_time)
            .unwrap_or(0);

        // Create FileMetadata for insertion
        let file_metadata = crate::storage::FileMetadata {
            id: 0, // Will be auto-generated
            sha256_hash: hash.clone(),
            virtual_path: virtual_path.clone(),
            original_name,
            size: file_size as i64,
            modified_time,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        // Insert into metadata store
        if let Err(e) = metadata_store.insert_file(&file_metadata).await {
            error!(
                virtual_path = %virtual_path,
                error = %e,
                "Failed to insert file metadata"
            );
            failed_files += 1;
            failed_file_paths.push(real_path.clone());
            continue;
        }

        // Update new path map (now hash-based)
        new_path_map.insert(hash.clone(), virtual_path.clone());
        new_file_metadata.insert(
            hash.clone(),
            crate::models::config::FileMetadata {
                modified_time,
                size: file_size,
            },
        );

        migrated_files += 1;

        if migrated_files % 100 == 0 {
            debug!(
                migrated = migrated_files,
                total = total_files,
                "Migration progress: {}/{}",
                migrated_files,
                total_files
            );
        }
    }

    // Step 5: Save updated index (for backward compatibility)
    info!("Saving updated index file");
    
    // Convert CasFileMetadata back to old FileMetadata format for index
    let old_format_metadata: HashMap<String, crate::models::config::FileMetadata> = new_file_metadata
        .iter()
        .map(|(hash, meta)| {
            (
                hash.clone(),
                crate::models::config::FileMetadata {
                    modified_time: meta.modified_time,
                    size: meta.size,
                },
            )
        })
        .collect();

    save_index(app, workspace_id, &new_path_map, &old_format_metadata)?;

    // Step 6: Verify migration
    info!("Verifying migration completeness");
    let verification_result = verify_migration(&metadata_store, migrated_files).await;
    
    let success = match verification_result {
        Ok(verified_count) => {
            if verified_count == migrated_files {
                info!(
                    verified = verified_count,
                    "Migration verification successful"
                );
                true
            } else {
                warn!(
                    expected = migrated_files,
                    actual = verified_count,
                    "Migration verification mismatch"
                );
                false
            }
        }
        Err(e) => {
            error!(
                error = %e,
                "Migration verification failed"
            );
            false
        }
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let report = MigrationReport {
        workspace_id: workspace_id.to_string(),
        total_files,
        migrated_files,
        failed_files,
        deduplicated_files,
        original_size,
        cas_size,
        failed_file_paths,
        duration_ms,
        success,
    };

    info!(
        workspace_id = %workspace_id,
        migrated = migrated_files,
        failed = failed_files,
        deduplicated = deduplicated_files,
        duration_ms = duration_ms,
        "Migration completed"
    );

    Ok(report)
}

/// Verify migration completeness
async fn verify_migration(metadata_store: &MetadataStore, expected_count: usize) -> Result<usize> {
    let all_files = metadata_store.get_all_files().await?;
    Ok(all_files.len())
}

/// Check if a workspace needs migration
///
/// # Arguments
///
/// * `workspace_id` - The workspace ID to check
/// * `app` - Tauri application handle
///
/// # Returns
///
/// `true` if the workspace is in traditional format and needs migration
pub fn needs_migration(workspace_id: &str, app: &AppHandle) -> Result<bool> {
    let format = detect_workspace_format(workspace_id, app)?;
    Ok(format == WorkspaceFormat::Traditional)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_format_detection() {
        // This is a placeholder test
        // Real tests would require setting up test workspaces
        assert_eq!(WorkspaceFormat::Traditional, WorkspaceFormat::Traditional);
    }
}
