//! Integrity Verification for CAS and Metadata
//!
//! This module provides integrity verification for the Content-Addressable Storage
//! and metadata store. It ensures that:
//! - All files in metadata have corresponding objects in CAS
//! - All CAS objects have valid hashes
//! - No corruption has occurred
//!
//! # Requirements
//!
//! Validates: Requirements 2.4

use crate::error::Result;
use crate::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

/// Validation report for integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Total files checked
    pub total_files: usize,
    /// Files with valid integrity
    pub valid_files: usize,
    /// Files with integrity issues
    pub invalid_files: Vec<InvalidFileInfo>,
    /// Missing CAS objects
    pub missing_objects: Vec<String>,
    /// Corrupted CAS objects
    pub corrupted_objects: Vec<String>,
    /// Warnings encountered
    pub warnings: Vec<String>,
    /// Timestamp of validation
    pub timestamp: i64,
}

/// Information about an invalid file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidFileInfo {
    /// File ID in metadata
    pub file_id: i64,
    /// Virtual path
    pub virtual_path: String,
    /// SHA-256 hash
    pub hash: String,
    /// Reason for invalidity
    pub reason: String,
}

impl ValidationReport {
    /// Create a new empty validation report
    pub fn new() -> Self {
        Self {
            total_files: 0,
            valid_files: 0,
            invalid_files: Vec::new(),
            missing_objects: Vec::new(),
            corrupted_objects: Vec::new(),
            warnings: Vec::new(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        self.invalid_files.is_empty()
            && self.missing_objects.is_empty()
            && self.corrupted_objects.is_empty()
    }

    /// Get total number of errors
    pub fn error_count(&self) -> usize {
        self.invalid_files.len() + self.missing_objects.len() + self.corrupted_objects.len()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify file integrity using hash
///
/// Reads the file content from CAS and verifies that the computed hash
/// matches the expected hash. This detects corruption.
///
/// # Arguments
///
/// * `cas` - Content-Addressable Storage instance
/// * `file_metadata` - File metadata containing the expected hash
///
/// # Returns
///
/// `true` if integrity check passes, `false` if corrupted or missing
///
/// # Requirements
///
/// Validates: Requirements 2.4
pub async fn verify_file_integrity(
    cas: &ContentAddressableStorage,
    file_metadata: &FileMetadata,
) -> Result<bool> {
    let hash = &file_metadata.sha256_hash;

    // Check if object exists
    if !cas.exists(hash) {
        debug!(
            hash = %hash,
            virtual_path = %file_metadata.virtual_path,
            "CAS object does not exist"
        );
        return Ok(false);
    }

    // Verify integrity by recomputing hash
    match cas.verify_integrity(hash).await {
        Ok(is_valid) => {
            if !is_valid {
                warn!(
                    hash = %hash,
                    virtual_path = %file_metadata.virtual_path,
                    "CAS object is corrupted (hash mismatch)"
                );
            }
            Ok(is_valid)
        }
        Err(e) => {
            warn!(
                hash = %hash,
                virtual_path = %file_metadata.virtual_path,
                error = %e,
                "Failed to verify integrity"
            );
            Ok(false)
        }
    }
}

/// Verify integrity of all files in the workspace
///
/// This function checks:
/// 1. All files in metadata have corresponding CAS objects
/// 2. All CAS objects have valid hashes (no corruption)
/// 3. Generates a detailed validation report
///
/// # Arguments
///
/// * `cas` - Content-Addressable Storage instance
/// * `metadata_store` - Metadata store instance
///
/// # Returns
///
/// A validation report with details of any issues found
///
/// # Requirements
///
/// Validates: Requirements 2.4
pub async fn verify_workspace_integrity(
    cas: &ContentAddressableStorage,
    metadata_store: &MetadataStore,
) -> Result<ValidationReport> {
    info!("Starting workspace integrity verification");

    let mut report = ValidationReport::new();

    // Get all files from metadata
    let files = metadata_store.get_all_files().await?;
    report.total_files = files.len();

    info!(total_files = files.len(), "Verifying file integrity");

    // Verify each file
    for file in files {
        let hash = &file.sha256_hash;

        // Check if CAS object exists
        if !cas.exists(hash) {
            report.missing_objects.push(hash.clone());
            report.invalid_files.push(InvalidFileInfo {
                file_id: file.id,
                virtual_path: file.virtual_path.clone(),
                hash: hash.clone(),
                reason: "CAS object does not exist".to_string(),
            });
            continue;
        }

        // Verify integrity
        match verify_file_integrity(cas, &file).await {
            Ok(true) => {
                report.valid_files += 1;
            }
            Ok(false) => {
                report.corrupted_objects.push(hash.clone());
                report.invalid_files.push(InvalidFileInfo {
                    file_id: file.id,
                    virtual_path: file.virtual_path.clone(),
                    hash: hash.clone(),
                    reason: "Hash mismatch (corrupted)".to_string(),
                });
            }
            Err(e) => {
                report.warnings.push(format!(
                    "Failed to verify file {}: {}",
                    file.virtual_path, e
                ));
            }
        }
    }

    info!(
        total = report.total_files,
        valid = report.valid_files,
        invalid = report.invalid_files.len(),
        missing = report.missing_objects.len(),
        corrupted = report.corrupted_objects.len(),
        "Integrity verification completed"
    );

    Ok(report)
}

/// Verify integrity after import
///
/// This is a convenience function that verifies integrity immediately after
/// an import operation completes. It's useful for catching issues early.
///
/// # Arguments
///
/// * `workspace_dir` - Workspace directory
///
/// # Returns
///
/// A validation report
///
/// # Requirements
///
/// Validates: Requirements 2.4
pub async fn verify_after_import(workspace_dir: &Path) -> Result<ValidationReport> {
    info!(
        workspace = %workspace_dir.display(),
        "Verifying integrity after import"
    );

    // Create CAS and metadata store instances
    let cas = ContentAddressableStorage::new(workspace_dir.to_path_buf());
    let metadata_store = MetadataStore::new(workspace_dir).await?;

    // Verify integrity
    let report = verify_workspace_integrity(&cas, &metadata_store).await?;

    if report.is_valid() {
        info!("Integrity verification passed");
    } else {
        warn!(
            errors = report.error_count(),
            "Integrity verification found issues"
        );
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_file_integrity_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        // Store a file
        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();

        // Create file metadata
        let file_metadata = FileMetadata {
            id: 1,
            sha256_hash: hash.clone(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        // Verify integrity
        let is_valid = verify_file_integrity(&cas, &file_metadata).await.unwrap();
        assert!(is_valid, "Integrity check should pass for valid file");
    }

    #[tokio::test]
    async fn test_verify_file_integrity_missing() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());

        // Create file metadata for non-existent file
        let file_metadata = FileMetadata {
            id: 1,
            sha256_hash: "nonexistent_hash".to_string(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        // Verify integrity
        let is_valid = verify_file_integrity(&cas, &file_metadata).await.unwrap();
        assert!(!is_valid, "Integrity check should fail for missing file");
    }

    #[tokio::test]
    async fn test_verify_workspace_integrity_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
        let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

        let report = verify_workspace_integrity(&cas, &metadata_store)
            .await
            .unwrap();

        assert_eq!(report.total_files, 0);
        assert_eq!(report.valid_files, 0);
        assert!(report.is_valid());
    }

    #[tokio::test]
    async fn test_verify_workspace_integrity_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
        let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

        // Store a file in CAS
        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();

        // Add file to metadata
        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash.clone(),
            virtual_path: "test.log".to_string(),
            original_name: "test.log".to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };
        metadata_store.insert_file(&file_metadata).await.unwrap();

        // Verify integrity
        let report = verify_workspace_integrity(&cas, &metadata_store)
            .await
            .unwrap();

        assert_eq!(report.total_files, 1);
        assert_eq!(report.valid_files, 1);
        assert!(report.is_valid());
    }

    #[tokio::test]
    async fn test_validation_report_is_valid() {
        let mut report = ValidationReport::new();
        assert!(report.is_valid());

        report.invalid_files.push(InvalidFileInfo {
            file_id: 1,
            virtual_path: "test.log".to_string(),
            hash: "hash123".to_string(),
            reason: "Test".to_string(),
        });
        assert!(!report.is_valid());
    }

    #[tokio::test]
    async fn test_validation_report_error_count() {
        let mut report = ValidationReport::new();
        assert_eq!(report.error_count(), 0);

        report.invalid_files.push(InvalidFileInfo {
            file_id: 1,
            virtual_path: "test.log".to_string(),
            hash: "hash123".to_string(),
            reason: "Test".to_string(),
        });
        report.missing_objects.push("hash456".to_string());
        report.corrupted_objects.push("hash789".to_string());

        assert_eq!(report.error_count(), 3);
    }
}
