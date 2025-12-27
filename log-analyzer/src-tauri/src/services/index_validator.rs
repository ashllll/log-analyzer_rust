//! Index Validator Service
//!
//! Validates workspace metadata integrity and generates validation reports.
//! Ensures all hashes in the metadata store exist in CAS.
//!
//! ## Features
//!
//! - Validate all file hashes exist in CAS
//! - Generate detailed validation reports
//! - Identify missing or corrupted objects
//! - Support for batch validation

use crate::error::Result;
use crate::storage::{ContentAddressableStorage, MetadataStore};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Validation report for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Total number of files checked
    pub total_files: usize,
    /// Number of valid files (hash exists in CAS)
    pub valid_files: usize,
    /// Number of invalid files (hash missing from CAS)
    pub invalid_files: usize,
    /// Details of invalid files
    pub invalid_file_details: Vec<InvalidFileInfo>,
    /// Warnings encountered during validation
    pub warnings: Vec<String>,
    /// Timestamp of validation
    pub timestamp: SystemTime,
    /// Total size of valid files (bytes)
    pub total_valid_size: u64,
    /// Total size of invalid files (bytes)
    pub total_invalid_size: u64,
}

/// Information about an invalid file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidFileInfo {
    /// SHA-256 hash that's missing
    pub hash: String,
    /// Virtual path of the file
    pub virtual_path: String,
    /// Original filename
    pub original_name: String,
    /// File size
    pub size: i64,
    /// Reason for invalidity
    pub reason: String,
}

/// Index validator for workspace metadata
pub struct IndexValidator {
    metadata_store: MetadataStore,
    cas: ContentAddressableStorage,
}

impl IndexValidator {
    /// Create a new index validator
    ///
    /// # Arguments
    ///
    /// * `metadata_store` - Metadata store to validate
    /// * `cas` - Content-addressable storage to check against
    pub fn new(metadata_store: MetadataStore, cas: ContentAddressableStorage) -> Self {
        Self {
            metadata_store,
            cas,
        }
    }

    /// Validate all metadata entries against CAS
    ///
    /// Checks that every file hash in the metadata store has a corresponding
    /// object in the CAS. This ensures data integrity.
    ///
    /// # Returns
    ///
    /// Validation report with details of any issues found
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use log_analyzer::services::IndexValidator;
    /// # use log_analyzer::storage::{MetadataStore, ContentAddressableStorage};
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let metadata = MetadataStore::new(&PathBuf::from("./workspace")).await.unwrap();
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let validator = IndexValidator::new(metadata, cas);
    ///
    /// let report = validator.validate_metadata().await.unwrap();
    /// println!("Valid files: {}/{}", report.valid_files, report.total_files);
    /// # })
    /// ```
    pub async fn validate_metadata(&self) -> Result<ValidationReport> {
        info!("Starting metadata validation");

        let start_time = SystemTime::now();
        let mut valid_files = 0;
        let mut invalid_files = 0;
        let mut invalid_file_details = Vec::new();
        let mut warnings = Vec::new();
        let mut total_valid_size = 0u64;
        let mut total_invalid_size = 0u64;

        // Get all files from metadata store
        let files = self.metadata_store.get_all_files().await?;
        let total_files = files.len();

        info!(total_files = total_files, "Validating file hashes");

        for file in files {
            debug!(
                hash = %file.sha256_hash,
                virtual_path = %file.virtual_path,
                "Checking file"
            );

            // Check if hash exists in CAS
            if self.cas.exists(&file.sha256_hash) {
                valid_files += 1;
                total_valid_size += file.size as u64;
                debug!(hash = %file.sha256_hash, "Hash exists in CAS");
            } else {
                invalid_files += 1;
                total_invalid_size += file.size as u64;

                warn!(
                    hash = %file.sha256_hash,
                    virtual_path = %file.virtual_path,
                    "Hash missing from CAS"
                );

                invalid_file_details.push(InvalidFileInfo {
                    hash: file.sha256_hash.clone(),
                    virtual_path: file.virtual_path.clone(),
                    original_name: file.original_name.clone(),
                    size: file.size,
                    reason: "Object not found in CAS".to_string(),
                });
            }
        }

        // Add summary warnings
        if invalid_files > 0 {
            warnings.push(format!(
                "{} files have missing objects in CAS ({}% of total)",
                invalid_files,
                (invalid_files * 100) / total_files.max(1)
            ));
        }

        let report = ValidationReport {
            total_files,
            valid_files,
            invalid_files,
            invalid_file_details,
            warnings,
            timestamp: start_time,
            total_valid_size,
            total_invalid_size,
        };

        info!(
            total = total_files,
            valid = valid_files,
            invalid = invalid_files,
            "Metadata validation complete"
        );

        Ok(report)
    }

    /// Validate a specific file by hash
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash to validate
    ///
    /// # Returns
    ///
    /// `true` if the hash exists in CAS, `false` otherwise
    pub fn validate_hash(&self, hash: &str) -> bool {
        self.cas.exists(hash)
    }

    /// Validate multiple hashes in batch
    ///
    /// More efficient than calling `validate_hash` multiple times.
    ///
    /// # Arguments
    ///
    /// * `hashes` - List of SHA-256 hashes to validate
    ///
    /// # Returns
    ///
    /// Vector of booleans indicating validity of each hash
    pub fn validate_hashes_batch(&self, hashes: &[String]) -> Vec<bool> {
        hashes.iter().map(|hash| self.cas.exists(hash)).collect()
    }

    /// Get validation statistics without full details
    ///
    /// Faster than `validate_metadata` as it doesn't collect detailed info.
    ///
    /// # Returns
    ///
    /// Tuple of (total_files, valid_files, invalid_files)
    pub async fn get_validation_stats(&self) -> Result<(usize, usize, usize)> {
        let files = self.metadata_store.get_all_files().await?;
        let total_files = files.len();

        let mut valid_files = 0;
        let mut invalid_files = 0;

        for file in files {
            if self.cas.exists(&file.sha256_hash) {
                valid_files += 1;
            } else {
                invalid_files += 1;
            }
        }

        Ok((total_files, valid_files, invalid_files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::metadata_store::FileMetadata;
    use crate::storage::{ContentAddressableStorage, MetadataStore};
    use tempfile::TempDir;

    // Helper function to create FileMetadata
    fn create_file_metadata(
        hash: &str,
        virtual_path: &str,
        original_name: &str,
        size: i64,
        depth_level: i32,
    ) -> FileMetadata {
        FileMetadata {
            id: 0, // Will be auto-generated
            sha256_hash: hash.to_string(),
            virtual_path: virtual_path.to_string(),
            original_name: original_name.to_string(),
            size,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level,
        }
    }

    #[tokio::test]
    async fn test_validate_empty_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());
        let validator = IndexValidator::new(metadata, cas);

        let report = validator.validate_metadata().await.unwrap();

        assert_eq!(report.total_files, 0);
        assert_eq!(report.valid_files, 0);
        assert_eq!(report.invalid_files, 0);
        assert!(report.invalid_file_details.is_empty());
    }

    #[tokio::test]
    async fn test_validate_all_valid() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Store content in CAS
        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();

        // Insert file metadata
        let file_meta =
            create_file_metadata(&hash, "test/file.log", "file.log", content.len() as i64, 0);
        metadata.insert_file(&file_meta).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);
        let report = validator.validate_metadata().await.unwrap();

        assert_eq!(report.total_files, 1);
        assert_eq!(report.valid_files, 1);
        assert_eq!(report.invalid_files, 0);
        assert!(report.invalid_file_details.is_empty());
    }

    #[tokio::test]
    async fn test_validate_missing_object() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Insert file metadata WITHOUT storing in CAS
        let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let file_meta = create_file_metadata(fake_hash, "test/file.log", "file.log", 100, 0);
        metadata.insert_file(&file_meta).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);
        let report = validator.validate_metadata().await.unwrap();

        assert_eq!(report.total_files, 1);
        assert_eq!(report.valid_files, 0);
        assert_eq!(report.invalid_files, 1);
        assert_eq!(report.invalid_file_details.len(), 1);
        assert_eq!(report.invalid_file_details[0].hash, fake_hash);
    }

    #[tokio::test]
    async fn test_validate_mixed() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Store one file in CAS
        let content1 = b"valid content";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(
            &hash1,
            "test/valid.log",
            "valid.log",
            content1.len() as i64,
            0,
        );
        metadata.insert_file(&file_meta1).await.unwrap();

        // Insert metadata without CAS object
        let fake_hash = "1111111111111111111111111111111111111111111111111111111111111111";
        let file_meta2 = create_file_metadata(fake_hash, "test/invalid.log", "invalid.log", 200, 0);
        metadata.insert_file(&file_meta2).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);
        let report = validator.validate_metadata().await.unwrap();

        assert_eq!(report.total_files, 2);
        assert_eq!(report.valid_files, 1);
        assert_eq!(report.invalid_files, 1);
        assert_eq!(report.invalid_file_details.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_hash() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);

        assert!(validator.validate_hash(&hash));
        assert!(!validator.validate_hash("nonexistent_hash"));
    }

    #[tokio::test]
    async fn test_validate_hashes_batch() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content1 = b"content 1";
        let content2 = b"content 2";
        let hash1 = cas.store_content(content1).await.unwrap();
        let hash2 = cas.store_content(content2).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);

        let hashes = vec![
            hash1.clone(),
            "nonexistent".to_string(),
            hash2.clone(),
            "another_nonexistent".to_string(),
        ];

        let results = validator.validate_hashes_batch(&hashes);

        assert_eq!(results.len(), 4);
        assert!(results[0]); // hash1 exists
        assert!(!results[1]); // nonexistent
        assert!(results[2]); // hash2 exists
        assert!(!results[3]); // another_nonexistent
    }

    #[tokio::test]
    async fn test_get_validation_stats() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Add 2 valid files
        let content1 = b"content 1";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(&hash1, "test/1.log", "1.log", 100, 0);
        metadata.insert_file(&file_meta1).await.unwrap();

        let content2 = b"content 2";
        let hash2 = cas.store_content(content2).await.unwrap();
        let file_meta2 = create_file_metadata(&hash2, "test/2.log", "2.log", 200, 0);
        metadata.insert_file(&file_meta2).await.unwrap();

        // Add 1 invalid file
        let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let file_meta3 = create_file_metadata(fake_hash, "test/3.log", "3.log", 300, 0);
        metadata.insert_file(&file_meta3).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);
        let (total, valid, invalid) = validator.get_validation_stats().await.unwrap();

        assert_eq!(total, 3);
        assert_eq!(valid, 2);
        assert_eq!(invalid, 1);
    }

    #[tokio::test]
    async fn test_validation_report_sizes() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Add valid file
        let content1 = b"valid content";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(
            &hash1,
            "test/valid.log",
            "valid.log",
            content1.len() as i64,
            0,
        );
        metadata.insert_file(&file_meta1).await.unwrap();

        // Add invalid file
        let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let file_meta2 = create_file_metadata(fake_hash, "test/invalid.log", "invalid.log", 500, 0);
        metadata.insert_file(&file_meta2).await.unwrap();

        let validator = IndexValidator::new(metadata, cas);
        let report = validator.validate_metadata().await.unwrap();

        assert_eq!(report.total_valid_size, content1.len() as u64);
        assert_eq!(report.total_invalid_size, 500);
    }
}
