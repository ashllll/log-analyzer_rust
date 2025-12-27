//! Legacy Format Detection
//!
//! This module provides utilities for detecting and handling legacy workspace formats
//! that are no longer supported. It helps guide users to migrate to the new CAS-based format.

use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Legacy workspace format information
#[derive(Debug, Clone, serde::Serialize)]
pub struct LegacyWorkspaceInfo {
    /// Workspace ID
    pub workspace_id: String,
    /// Path to the legacy index file
    pub index_path: PathBuf,
    /// Type of legacy format detected
    pub format_type: LegacyFormatType,
}

/// Types of legacy formats that may be detected
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum LegacyFormatType {
    /// Compressed index file (.idx.gz)
    CompressedIndex,
    /// Uncompressed index file (.idx)
    UncompressedIndex,
}

impl LegacyFormatType {
    /// Get a human-readable description of the format type
    pub fn description(&self) -> &str {
        match self {
            LegacyFormatType::CompressedIndex => "Compressed index file (.idx.gz)",
            LegacyFormatType::UncompressedIndex => "Uncompressed index file (.idx)",
        }
    }
}

/// Scan for legacy workspace formats in the indices directory
///
/// This function checks for old `.idx.gz` and `.idx` files that indicate
/// workspaces using the deprecated format.
///
/// # Arguments
///
/// * `indices_dir` - Path to the indices directory
///
/// # Returns
///
/// A vector of detected legacy workspaces
pub fn scan_legacy_workspaces(indices_dir: &Path) -> Vec<LegacyWorkspaceInfo> {
    let mut legacy_workspaces = Vec::new();

    // Check if indices directory exists
    if !indices_dir.exists() {
        info!("Indices directory does not exist, no legacy workspaces to scan");
        return legacy_workspaces;
    }

    // Read directory entries
    let entries = match std::fs::read_dir(indices_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read indices directory: {}", e);
            return legacy_workspaces;
        }
    };

    // Scan for legacy index files
    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Check for .idx.gz files
        if file_name.ends_with(".idx.gz") {
            let workspace_id = file_name.trim_end_matches(".idx.gz").to_string();
            legacy_workspaces.push(LegacyWorkspaceInfo {
                workspace_id,
                index_path: path,
                format_type: LegacyFormatType::CompressedIndex,
            });
        }
        // Check for .idx files (uncompressed)
        else if file_name.ends_with(".idx") {
            let workspace_id = file_name.trim_end_matches(".idx").to_string();
            legacy_workspaces.push(LegacyWorkspaceInfo {
                workspace_id,
                index_path: path,
                format_type: LegacyFormatType::UncompressedIndex,
            });
        }
    }

    if !legacy_workspaces.is_empty() {
        info!(
            "Detected {} legacy workspace(s) with old format",
            legacy_workspaces.len()
        );
    }

    legacy_workspaces
}

/// Generate a user-friendly message about legacy workspaces
///
/// This creates a helpful message that explains the situation and guides
/// users on how to proceed.
///
/// # Arguments
///
/// * `legacy_workspaces` - List of detected legacy workspaces
///
/// # Returns
///
/// A formatted message string
pub fn generate_legacy_message(legacy_workspaces: &[LegacyWorkspaceInfo]) -> String {
    if legacy_workspaces.is_empty() {
        return String::new();
    }

    let count = legacy_workspaces.len();
    let workspace_list: Vec<String> = legacy_workspaces
        .iter()
        .map(|w| format!("  - {} ({})", w.workspace_id, w.format_type.description()))
        .collect();

    format!(
        "⚠️  Legacy Workspace Format Detected\n\
        \n\
        We found {} workspace(s) using an old format that is no longer supported:\n\
        \n\
        {}\n\
        \n\
        📋 What this means:\n\
        The application has migrated to a new Content-Addressable Storage (CAS) architecture \
        that provides better performance, reliability, and deduplication.\n\
        \n\
        🔧 What you need to do:\n\
        1. Create a new workspace using the current version\n\
        2. Re-import your log files or archives\n\
        3. The old workspace data will be automatically cleaned up\n\
        \n\
        ✨ Benefits of the new format:\n\
        - Automatic deduplication saves storage space\n\
        - Faster search with SQLite FTS5\n\
        - Better handling of nested archives\n\
        - More reliable data integrity\n\
        \n\
        The legacy index files will be removed during cleanup to free up space.",
        count,
        workspace_list.join("\n")
    )
}

/// Check if a specific workspace uses legacy format
///
/// # Arguments
///
/// * `workspace_id` - The workspace ID to check
/// * `indices_dir` - Path to the indices directory
///
/// # Returns
///
/// `Some(LegacyWorkspaceInfo)` if legacy format is detected, `None` otherwise
pub fn check_workspace_legacy_format(
    workspace_id: &str,
    indices_dir: &Path,
) -> Option<LegacyWorkspaceInfo> {
    // Check for compressed index
    let compressed_path = indices_dir.join(format!("{}.idx.gz", workspace_id));
    if compressed_path.exists() {
        return Some(LegacyWorkspaceInfo {
            workspace_id: workspace_id.to_string(),
            index_path: compressed_path,
            format_type: LegacyFormatType::CompressedIndex,
        });
    }

    // Check for uncompressed index
    let uncompressed_path = indices_dir.join(format!("{}.idx", workspace_id));
    if uncompressed_path.exists() {
        return Some(LegacyWorkspaceInfo {
            workspace_id: workspace_id.to_string(),
            index_path: uncompressed_path,
            format_type: LegacyFormatType::UncompressedIndex,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_workspaces = scan_legacy_workspaces(temp_dir.path());
        assert!(legacy_workspaces.is_empty());
    }

    #[test]
    fn test_scan_with_legacy_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some legacy index files
        fs::write(temp_dir.path().join("workspace1.idx.gz"), b"dummy").unwrap();
        fs::write(temp_dir.path().join("workspace2.idx"), b"dummy").unwrap();
        fs::write(temp_dir.path().join("not-an-index.txt"), b"dummy").unwrap();

        let legacy_workspaces = scan_legacy_workspaces(temp_dir.path());

        assert_eq!(legacy_workspaces.len(), 2);
        assert!(legacy_workspaces
            .iter()
            .any(|w| w.workspace_id == "workspace1"));
        assert!(legacy_workspaces
            .iter()
            .any(|w| w.workspace_id == "workspace2"));
    }

    #[test]
    fn test_check_workspace_legacy_format() {
        let temp_dir = TempDir::new().unwrap();

        // Create a legacy index file
        fs::write(temp_dir.path().join("test-workspace.idx.gz"), b"dummy").unwrap();

        let result = check_workspace_legacy_format("test-workspace", temp_dir.path());
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.workspace_id, "test-workspace");
        assert_eq!(info.format_type, LegacyFormatType::CompressedIndex);
    }

    #[test]
    fn test_check_workspace_no_legacy() {
        let temp_dir = TempDir::new().unwrap();

        let result = check_workspace_legacy_format("nonexistent", temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_legacy_message() {
        let legacy_workspaces = vec![LegacyWorkspaceInfo {
            workspace_id: "workspace1".to_string(),
            index_path: PathBuf::from("/path/to/workspace1.idx.gz"),
            format_type: LegacyFormatType::CompressedIndex,
        }];

        let message = generate_legacy_message(&legacy_workspaces);
        assert!(message.contains("Legacy Workspace Format Detected"));
        assert!(message.contains("workspace1"));
        assert!(message.contains("Content-Addressable Storage"));
    }

    #[test]
    fn test_generate_legacy_message_empty() {
        let message = generate_legacy_message(&[]);
        assert!(message.is_empty());
    }
}
