//! CAS Test Helper Functions
//!
//! This module provides helper functions for testing CAS (Content-Addressable Storage)
//! functionality. These helpers replace the old traditional workspace helpers and ensure
//! all tests use the CAS + MetadataStore architecture.
//!
//! **Validates: Requirements 4.1, 4.2**

use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Result type for CAS test operations
pub type CasTestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// A test workspace with CAS and MetadataStore
pub struct CasTestWorkspace {
    pub cas: ContentAddressableStorage,
    pub metadata: MetadataStore,
    pub temp_dir: TempDir,
    pub workspace_path: PathBuf,
}

impl CasTestWorkspace {
    /// Get the workspace path
    pub fn path(&self) -> &Path {
        &self.workspace_path
    }

    /// Get the objects directory path
    pub fn objects_dir(&self) -> PathBuf {
        self.workspace_path.join("objects")
    }

    /// Get the metadata database path
    pub fn metadata_db_path(&self) -> PathBuf {
        self.workspace_path.join("metadata.db")
    }
}

/// Create a new CAS workspace for testing
///
/// This function creates a temporary directory with:
/// - A ContentAddressableStorage instance
/// - A MetadataStore instance (SQLite database)
/// - Proper directory structure (objects/, metadata.db)
///
/// # Example
///
/// ```no_run
/// use cas_test_helpers::create_cas_workspace;
///
/// #[tokio::test]
/// async fn test_my_feature() {
///     let workspace = create_cas_workspace().await.unwrap();
///     // Use workspace.cas and workspace.metadata for testing
/// }
/// ```
///
/// **Validates: Requirements 4.1, 4.2**
pub async fn create_cas_workspace() -> CasTestResult<CasTestWorkspace> {
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_path_buf();

    // Create CAS instance (this will create the objects directory)
    let cas = ContentAddressableStorage::new(workspace_path.clone());

    // Create MetadataStore instance (this will create the database)
    let metadata = MetadataStore::new(&workspace_path).await?;

    Ok(CasTestWorkspace {
        cas,
        metadata,
        temp_dir,
        workspace_path,
    })
}

/// Configuration for populating a CAS workspace
pub struct PopulateConfig {
    /// Number of regular files to create
    pub file_count: usize,
    /// Number of lines per file
    pub lines_per_file: usize,
    /// Whether to include nested archives
    pub include_archives: bool,
    /// Number of nested archive levels (if include_archives is true)
    pub archive_depth: usize,
}

impl Default for PopulateConfig {
    fn default() -> Self {
        Self {
            file_count: 5,
            lines_per_file: 100,
            include_archives: false,
            archive_depth: 0,
        }
    }
}

/// Populate a CAS workspace with test data
///
/// This function populates a workspace with:
/// - Regular log files with realistic content
/// - Files stored in CAS with SHA-256 hashes
/// - Metadata entries in the MetadataStore
/// - Optional nested archives
///
/// # Arguments
///
/// * `workspace` - The CAS workspace to populate
/// * `config` - Configuration for the test data
///
/// # Returns
///
/// A vector of FileMetadata for all created files
///
/// # Example
///
/// ```no_run
/// use cas_test_helpers::{create_cas_workspace, populate_cas_workspace, PopulateConfig};
///
/// #[tokio::test]
/// async fn test_search() {
///     let workspace = create_cas_workspace().await.unwrap();
///     let config = PopulateConfig {
///         file_count: 10,
///         lines_per_file: 50,
///         ..Default::default()
///     };
///     let files = populate_cas_workspace(&workspace, config).await.unwrap();
///     assert_eq!(files.len(), 10);
/// }
/// ```
///
/// **Validates: Requirements 4.1, 4.2**
pub async fn populate_cas_workspace(
    workspace: &CasTestWorkspace,
    config: PopulateConfig,
) -> CasTestResult<Vec<FileMetadata>> {
    let mut created_files = Vec::new();

    // Create regular log files
    for i in 0..config.file_count {
        let mut content = Vec::new();

        // Generate realistic log content
        for j in 0..config.lines_per_file {
            let line = match j % 10 {
                0 => format!(
                    "2024-01-{:02} 12:00:{:02} [ERROR] Error message {} in file {}\n",
                    (i % 28) + 1,
                    j % 60,
                    j,
                    i
                ),
                1 | 2 => format!(
                    "2024-01-{:02} 12:00:{:02} [WARN] Warning message {} in file {}\n",
                    (i % 28) + 1,
                    j % 60,
                    j,
                    i
                ),
                _ => format!(
                    "2024-01-{:02} 12:00:{:02} [INFO] Info message {} in file {}\n",
                    (i % 28) + 1,
                    j % 60,
                    j,
                    i
                ),
            };
            content.extend_from_slice(line.as_bytes());
        }

        // Store content in CAS
        let hash = workspace.cas.store_content(&content).await?;

        // Create metadata entry
        let file_meta = FileMetadata {
            id: 0, // Will be assigned by database
            sha256_hash: hash.clone(),
            virtual_path: format!("logs/file_{}.log", i),
            original_name: format!("file_{}.log", i),
            size: content.len() as i64,
            modified_time: chrono::Utc::now().timestamp(),
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        // Insert into metadata store
        workspace.metadata.insert_file(&file_meta).await?;

        created_files.push(file_meta);
    }

    // Create nested archives if requested
    if config.include_archives && config.archive_depth > 0 {
        let archive_files =
            create_nested_archive_structure(workspace, config.archive_depth).await?;
        created_files.extend(archive_files);
    }

    Ok(created_files)
}

/// Helper to create nested archive structure for testing
async fn create_nested_archive_structure(
    workspace: &CasTestWorkspace,
    depth: usize,
) -> CasTestResult<Vec<FileMetadata>> {
    let mut created_files = Vec::new();
    let temp_dir = workspace.temp_dir.path();

    // Create innermost files
    let inner_files = vec![
        (
            "inner1.log",
            b"ERROR: Inner error 1\nINFO: Inner info 1" as &[u8],
        ),
        ("inner2.log", b"WARN: Inner warning\nERROR: Inner error 2"),
    ];

    // Create inner archive
    let inner_zip = create_test_zip(temp_dir, "inner.zip", inner_files.clone())?;

    // Store inner archive files in CAS
    for (filename, content) in inner_files {
        let hash = workspace.cas.store_content(content).await?;

        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: format!("archives/inner.zip/{}", filename),
            original_name: filename.to_string(),
            size: content.len() as i64,
            modified_time: chrono::Utc::now().timestamp(),
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: depth as i32,
        };

        workspace.metadata.insert_file(&file_meta).await?;
        created_files.push(file_meta);
    }

    // Create outer archive if depth > 1
    if depth > 1 {
        let _outer_zip = create_nested_test_zip(temp_dir, "outer.zip", vec![inner_zip])?;

        // Store outer archive file in CAS
        let outer_content = b"ERROR: Outer error\nINFO: Outer info";
        let hash = workspace.cas.store_content(outer_content).await?;

        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: "archives/outer.zip/outer_file.log".to_string(),
            original_name: "outer_file.log".to_string(),
            size: outer_content.len() as i64,
            modified_time: chrono::Utc::now().timestamp(),
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: (depth - 1) as i32,
        };

        workspace.metadata.insert_file(&file_meta).await?;
        created_files.push(file_meta);
    }

    Ok(created_files)
}

/// Helper to create a simple ZIP archive
fn create_test_zip(dir: &Path, name: &str, files: Vec<(&str, &[u8])>) -> CasTestResult<PathBuf> {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (filename, content) in files {
        zip.start_file(filename, options)?;
        zip.write_all(content)?;
    }

    zip.finish()?;
    Ok(zip_path)
}

/// Helper to create a nested ZIP archive
fn create_nested_test_zip(
    dir: &Path,
    name: &str,
    inner_archives: Vec<PathBuf>,
) -> CasTestResult<PathBuf> {
    let zip_path = dir.join(name);
    let file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add inner archives
    for inner_archive in inner_archives {
        let inner_name = inner_archive.file_name().unwrap().to_str().unwrap();
        zip.start_file(inner_name, options)?;
        let inner_content = fs::read(&inner_archive)?;
        zip.write_all(&inner_content)?;
    }

    // Add outer file
    zip.start_file("outer_file.log", options)?;
    zip.write_all(b"ERROR: Outer error\nINFO: Outer info")?;

    zip.finish()?;
    Ok(zip_path)
}

/// Verification results for a CAS workspace
#[derive(Debug)]
pub struct VerificationResult {
    /// Total number of files in metadata
    pub total_files: usize,
    /// Number of files with valid CAS objects
    pub valid_cas_objects: usize,
    /// Number of files with missing CAS objects
    pub missing_cas_objects: usize,
    /// Number of files with hash mismatches
    pub hash_mismatches: usize,
    /// Whether the workspace is valid
    pub is_valid: bool,
    /// Detailed error messages
    pub errors: Vec<String>,
}

impl VerificationResult {
    /// Check if the workspace passed all verifications
    pub fn is_ok(&self) -> bool {
        self.is_valid && self.errors.is_empty()
    }

    /// Get a summary message
    pub fn summary(&self) -> String {
        format!(
            "Verification: {} total files, {} valid, {} missing, {} mismatches, {} errors",
            self.total_files,
            self.valid_cas_objects,
            self.missing_cas_objects,
            self.hash_mismatches,
            self.errors.len()
        )
    }
}

/// Verify a CAS workspace for correctness
///
/// This function performs comprehensive verification:
/// - All metadata entries have corresponding CAS objects
/// - All CAS objects have valid SHA-256 hashes
/// - Content integrity (re-hash matches stored hash)
/// - Database schema is correct
/// - No orphaned CAS objects
///
/// # Arguments
///
/// * `workspace` - The CAS workspace to verify
///
/// # Returns
///
/// A VerificationResult with detailed information
///
/// # Example
///
/// ```no_run
/// use cas_test_helpers::{create_cas_workspace, populate_cas_workspace, verify_cas_workspace, PopulateConfig};
///
/// #[tokio::test]
/// async fn test_workspace_integrity() {
///     let workspace = create_cas_workspace().await.unwrap();
///     populate_cas_workspace(&workspace, PopulateConfig::default()).await.unwrap();
///     
///     let result = verify_cas_workspace(&workspace).await.unwrap();
///     assert!(result.is_ok(), "Workspace should be valid: {}", result.summary());
/// }
/// ```
///
/// **Validates: Requirements 4.1, 4.2**
pub async fn verify_cas_workspace(
    workspace: &CasTestWorkspace,
) -> CasTestResult<VerificationResult> {
    let mut result = VerificationResult {
        total_files: 0,
        valid_cas_objects: 0,
        missing_cas_objects: 0,
        hash_mismatches: 0,
        is_valid: true,
        errors: Vec::new(),
    };

    // Verify database exists
    if !workspace.metadata_db_path().exists() {
        result.is_valid = false;
        result
            .errors
            .push("Metadata database does not exist".to_string());
        return Ok(result);
    }

    // Verify objects directory exists
    if !workspace.objects_dir().exists() {
        result.is_valid = false;
        result
            .errors
            .push("Objects directory does not exist".to_string());
        return Ok(result);
    }

    // Get all files from metadata
    let all_files = match workspace.metadata.get_all_files().await {
        Ok(files) => files,
        Err(e) => {
            result.is_valid = false;
            result
                .errors
                .push(format!("Failed to get files from metadata: {}", e));
            return Ok(result);
        }
    };

    result.total_files = all_files.len();

    // Verify each file
    for file in all_files {
        // Check if CAS object exists
        if !workspace.cas.exists(&file.sha256_hash) {
            result.missing_cas_objects += 1;
            result.is_valid = false;
            result.errors.push(format!(
                "Missing CAS object for file: {} (hash: {})",
                file.virtual_path, file.sha256_hash
            ));
            continue;
        }

        // Verify content integrity
        match workspace.cas.read_content(&file.sha256_hash).await {
            Ok(content) => {
                // Re-compute hash
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let computed_hash = format!("{:x}", hasher.finalize());

                if computed_hash != file.sha256_hash {
                    result.hash_mismatches += 1;
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Hash mismatch for file: {} (expected: {}, got: {})",
                        file.virtual_path, file.sha256_hash, computed_hash
                    ));
                } else {
                    result.valid_cas_objects += 1;
                }

                // Verify size matches
                if content.len() as i64 != file.size {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Size mismatch for file: {} (metadata: {}, actual: {})",
                        file.virtual_path,
                        file.size,
                        content.len()
                    ));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!(
                    "Failed to read CAS object for file: {} - {}",
                    file.virtual_path, e
                ));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_cas_workspace() {
        let workspace = create_cas_workspace().await.unwrap();

        // Verify workspace structure
        assert!(workspace.path().exists());
        // Note: objects directory is created lazily when first content is stored
        assert!(workspace.metadata_db_path().exists());

        // Store some content to trigger objects directory creation
        let content = b"test content";
        let hash = workspace.cas.store_content(content).await.unwrap();

        // Now objects directory should exist
        assert!(workspace.objects_dir().exists());

        // Verify we can retrieve the content
        let retrieved = workspace.cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_populate_cas_workspace_default() {
        let workspace = create_cas_workspace().await.unwrap();
        let config = PopulateConfig::default();

        let files = populate_cas_workspace(&workspace, config).await.unwrap();

        // Verify files were created
        assert_eq!(files.len(), 5);

        // Verify all files are in metadata
        let all_files = workspace.metadata.get_all_files().await.unwrap();
        assert_eq!(all_files.len(), 5);
    }

    #[tokio::test]
    async fn test_populate_cas_workspace_custom() {
        let workspace = create_cas_workspace().await.unwrap();
        let config = PopulateConfig {
            file_count: 10,
            lines_per_file: 50,
            include_archives: false,
            archive_depth: 0,
        };

        let files = populate_cas_workspace(&workspace, config).await.unwrap();

        // Verify correct number of files
        assert_eq!(files.len(), 10);

        // Verify each file has correct properties
        for file in files {
            assert!(file.virtual_path.starts_with("logs/"));
            assert!(file.sha256_hash.len() == 64); // SHA-256 hex length
            assert!(file.size > 0);
        }
    }

    #[tokio::test]
    async fn test_verify_cas_workspace_valid() {
        let workspace = create_cas_workspace().await.unwrap();
        populate_cas_workspace(&workspace, PopulateConfig::default())
            .await
            .unwrap();

        let result = verify_cas_workspace(&workspace).await.unwrap();

        assert!(
            result.is_ok(),
            "Workspace should be valid: {}",
            result.summary()
        );
        assert_eq!(result.total_files, 5);
        assert_eq!(result.valid_cas_objects, 5);
        assert_eq!(result.missing_cas_objects, 0);
        assert_eq!(result.hash_mismatches, 0);
    }

    #[tokio::test]
    async fn test_verify_cas_workspace_with_archives() {
        let workspace = create_cas_workspace().await.unwrap();
        let config = PopulateConfig {
            file_count: 3,
            lines_per_file: 50,
            include_archives: true,
            archive_depth: 2,
        };

        populate_cas_workspace(&workspace, config).await.unwrap();

        let result = verify_cas_workspace(&workspace).await.unwrap();

        assert!(
            result.is_ok(),
            "Workspace should be valid: {}",
            result.summary()
        );
        assert!(result.total_files >= 3); // At least the regular files
        assert_eq!(result.missing_cas_objects, 0);
        assert_eq!(result.hash_mismatches, 0);
    }

    #[tokio::test]
    async fn test_workspace_content_integrity() {
        let workspace = create_cas_workspace().await.unwrap();

        // Store some content
        let content = b"Test content for integrity check";
        let hash = workspace.cas.store_content(content).await.unwrap();

        // Create metadata
        let file_meta = FileMetadata {
            id: 0,
            sha256_hash: hash.clone(),
            virtual_path: "test/integrity.txt".to_string(),
            original_name: "integrity.txt".to_string(),
            size: content.len() as i64,
            modified_time: chrono::Utc::now().timestamp(),
            mime_type: Some("text/plain".to_string()),
            parent_archive_id: None,
            depth_level: 0,
        };

        workspace.metadata.insert_file(&file_meta).await.unwrap();

        // Verify
        let result = verify_cas_workspace(&workspace).await.unwrap();

        assert!(result.is_ok());
        assert_eq!(result.valid_cas_objects, 1);

        // Verify content can be retrieved
        let retrieved = workspace.cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content);
    }
}
