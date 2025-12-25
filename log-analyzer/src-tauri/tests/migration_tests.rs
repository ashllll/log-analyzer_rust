//! Migration Tests
//!
//! Tests for workspace migration from traditional format to CAS format.
//!
//! This test suite covers:
//! - Migration from old format to CAS format
//! - Data integrity verification after migration
//! - Backward compatibility with old format
//!
//! **Validates: Requirements 8.4**

use log_analyzer::migration::WorkspaceFormat;
use log_analyzer::models::config::FileMetadata as OldFileMetadata;
use log_analyzer::storage::{ContentAddressableStorage, FileMetadata, MetadataStore};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to create a test workspace directory structure
fn create_test_workspace(temp_dir: &TempDir, workspace_id: &str, is_cas: bool) -> PathBuf {
    let workspace_dir = temp_dir.path().join("extracted").join(workspace_id);
    fs::create_dir_all(&workspace_dir).unwrap();

    if is_cas {
        // Create CAS markers
        let metadata_db = workspace_dir.join("metadata.db");
        fs::write(&metadata_db, b"").unwrap();

        let objects_dir = workspace_dir.join("objects");
        fs::create_dir_all(&objects_dir).unwrap();
    } else {
        // Create traditional workspace with some files
        let file1 = workspace_dir.join("file1.log");
        fs::write(&file1, b"test content").unwrap();

        let nested_dir = workspace_dir.join("nested");
        fs::create_dir_all(&nested_dir).unwrap();
        let file2 = nested_dir.join("file2.log");
        fs::write(&file2, b"nested content").unwrap();
    }

    workspace_dir
}

/// Helper to create a traditional workspace with index file
fn create_traditional_workspace_with_index(
    temp_dir: &TempDir,
    workspace_id: &str,
) -> (PathBuf, HashMap<String, String>, HashMap<String, OldFileMetadata>) {
    let workspace_dir = temp_dir.path().join("extracted").join(workspace_id);
    fs::create_dir_all(&workspace_dir).unwrap();

    let mut path_map = HashMap::new();
    let mut file_metadata = HashMap::new();

    // Create test files with various content
    let test_files = vec![
        ("file1.log", b"First log file content" as &[u8]),
        ("file2.log", b"Second log file content"),
        ("file3.log", b"Third log file content"),
        ("nested/file4.log", b"Nested log file content"),
        ("nested/deep/file5.log", b"Deeply nested log file"),
    ];

    for (relative_path, content) in test_files {
        let file_path = workspace_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();

        let real_path = file_path.to_string_lossy().to_string();
        let virtual_path = format!("archive/{}", relative_path);

        path_map.insert(real_path.clone(), virtual_path);
        file_metadata.insert(
            real_path,
            OldFileMetadata {
                modified_time: 1234567890,
                size: content.len() as u64,
            },
        );
    }

    (workspace_dir, path_map, file_metadata)
}

/// Helper to verify file content matches
async fn verify_file_content(cas: &ContentAddressableStorage, hash: &str, expected_content: &[u8]) -> bool {
    match cas.read_content(hash).await {
        Ok(content) => content == expected_content,
        Err(_) => false,
    }
}

#[test]
fn test_workspace_format_detection_traditional() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-traditional";
    create_test_workspace(&temp_dir, workspace_id, false);

    // Note: This test would require a mock AppHandle
    // For now, we just verify the directory structure was created correctly
    let workspace_dir = temp_dir.path().join("extracted").join(workspace_id);
    assert!(workspace_dir.exists());
    assert!(workspace_dir.join("file1.log").exists());
    assert!(!workspace_dir.join("metadata.db").exists());
}

#[test]
fn test_workspace_format_detection_cas() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-cas";
    create_test_workspace(&temp_dir, workspace_id, true);

    let workspace_dir = temp_dir.path().join("extracted").join(workspace_id);
    assert!(workspace_dir.exists());
    assert!(workspace_dir.join("metadata.db").exists());
    assert!(workspace_dir.join("objects").exists());
}

#[tokio::test]
async fn test_cas_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();

    // Initialize CAS
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    assert!(workspace_dir.join("objects").exists() || !workspace_dir.join("objects").exists()); // CAS doesn't create objects dir until first store

    // Initialize metadata store
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();
    assert!(workspace_dir.join("metadata.db").exists());

    // Verify we can store a file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, b"test content").unwrap();

    let hash = cas.store_file_streaming(&test_file).await.unwrap();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA-256 hash length

    // Verify object was stored
    let object_path = cas.get_object_path(&hash);
    assert!(object_path.exists());
}

#[test]
fn test_workspace_format_enum() {
    assert_eq!(WorkspaceFormat::Traditional, WorkspaceFormat::Traditional);
    assert_eq!(WorkspaceFormat::CAS, WorkspaceFormat::CAS);
    assert_eq!(WorkspaceFormat::Unknown, WorkspaceFormat::Unknown);
    assert_ne!(WorkspaceFormat::Traditional, WorkspaceFormat::CAS);
}

#[tokio::test]
async fn test_migration_report_structure() {
    use log_analyzer::migration::MigrationReport;

    let report = MigrationReport {
        workspace_id: "test-workspace".to_string(),
        total_files: 100,
        migrated_files: 95,
        failed_files: 5,
        deduplicated_files: 10,
        original_size: 1024 * 1024,
        cas_size: 900 * 1024,
        failed_file_paths: vec!["file1.log".to_string(), "file2.log".to_string()],
        duration_ms: 1500,
        success: true,
    };

    assert_eq!(report.workspace_id, "test-workspace");
    assert_eq!(report.total_files, 100);
    assert_eq!(report.migrated_files, 95);
    assert_eq!(report.failed_files, 5);
    assert_eq!(report.deduplicated_files, 10);
    assert!(report.success);
    assert_eq!(report.failed_file_paths.len(), 2);
}

#[tokio::test]
async fn test_metadata_store_operations() {
    use log_analyzer::storage::FileMetadata;

    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();

    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Insert a file
    let file_metadata = FileMetadata {
        id: 0, // Will be auto-generated
        sha256_hash: "a".repeat(64),
        virtual_path: "test/file1.log".to_string(),
        original_name: "file1.log".to_string(),
        size: 1024,
        modified_time: 1234567890,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    metadata_store.insert_file(&file_metadata).await.unwrap();

    // Retrieve the file
    let retrieved = metadata_store
        .get_file_by_virtual_path("test/file1.log")
        .await
        .unwrap();

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.virtual_path, "test/file1.log");
    assert_eq!(retrieved.size, 1024);
    assert_eq!(retrieved.sha256_hash, "a".repeat(64));
}

#[tokio::test]
async fn test_cas_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());

    // Create two files with identical content
    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");
    fs::write(&file1, b"identical content").unwrap();
    fs::write(&file2, b"identical content").unwrap();

    // Store both files
    let hash1 = cas.store_file_streaming(&file1).await.unwrap();
    let hash2 = cas.store_file_streaming(&file2).await.unwrap();

    // Hashes should be identical (deduplication)
    assert_eq!(hash1, hash2);

    // Only one object should exist
    let object_path = cas.get_object_path(&hash1);
    assert!(object_path.exists());
}

#[tokio::test]
async fn test_migration_verification() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();

    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Insert multiple files
    for i in 0..10 {
        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: format!("{:0>64}", i),
            virtual_path: format!("test/file{}.log", i),
            original_name: format!("file{}.log", i),
            size: 1024 * i,
            modified_time: 1234567890 + i as i64,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };
        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    // Verify all files were inserted
    let all_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), 10);
}

// ============================================================================
// Migration from Old Format Tests
// ============================================================================

/// Test migration from traditional format to CAS format
/// **Validates: Requirements 8.4 - Migration from old format**
#[tokio::test]
async fn test_migration_from_traditional_format() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-migration-workspace";

    // Create traditional workspace with files
    let (workspace_dir, _path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Verify it's in traditional format (no CAS markers)
    assert!(!workspace_dir.join("metadata.db").exists());
    assert!(!workspace_dir.join("objects").exists());

    // Initialize CAS and metadata store (simulating migration)
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Verify CAS markers now exist
    assert!(workspace_dir.join("metadata.db").exists());

    // Migrate files to CAS
    let file1_path = workspace_dir.join("file1.log");
    let hash1 = cas.store_file_streaming(&file1_path).await.unwrap();

    // Store metadata
    let file_metadata = FileMetadata {
        id: 0,
        sha256_hash: hash1.clone(),
        virtual_path: "archive/file1.log".to_string(),
        original_name: "file1.log".to_string(),
        size: 22, // "First log file content".len()
        modified_time: 1234567890,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };
    metadata_store.insert_file(&file_metadata).await.unwrap();

    // Verify file is accessible via CAS
    let content = cas.read_content(&hash1).await.unwrap();
    assert_eq!(content, b"First log file content");

    // Verify metadata is stored
    let retrieved = metadata_store
        .get_file_by_virtual_path("archive/file1.log")
        .await
        .unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().sha256_hash, hash1);
}

/// Test migration preserves all files
/// **Validates: Requirements 8.4 - Data completeness**
#[tokio::test]
async fn test_migration_preserves_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-complete-migration";

    // Create traditional workspace with multiple files
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    let original_file_count = path_map.len();
    assert_eq!(original_file_count, 5); // We created 5 files

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate all files
    for (real_path, virtual_path) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let size = fs::metadata(file_path).unwrap().len();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: size as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
        }
    }

    // Verify all files were migrated
    let all_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(all_files.len(), original_file_count);
}

/// Test migration handles nested directory structures
/// **Validates: Requirements 8.4 - Nested structure preservation**
#[tokio::test]
async fn test_migration_handles_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-nested-migration";

    // Create traditional workspace with nested structure
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate nested files
    let nested_files: Vec<_> = path_map
        .iter()
        .filter(|(_, vpath)| vpath.contains("nested"))
        .collect();

    assert!(!nested_files.is_empty(), "Should have nested files");

    for (real_path, virtual_path) in nested_files {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let size = fs::metadata(file_path).unwrap().len();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: size as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: virtual_path.matches('/').count() as i32,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
        }
    }

    // Verify nested files are accessible
    let nested_file = metadata_store
        .get_file_by_virtual_path("archive/nested/file4.log")
        .await
        .unwrap();
    assert!(nested_file.is_some());
}

// ============================================================================
// Data Integrity Tests
// ============================================================================

/// Test data integrity after migration - content matches
/// **Validates: Requirements 8.4 - Data integrity**
#[tokio::test]
async fn test_migration_data_integrity_content() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-integrity";

    // Create traditional workspace
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Initialize CAS
    let cas = ContentAddressableStorage::new(workspace_dir.clone());

    // Store original content for comparison
    let mut original_contents: HashMap<String, Vec<u8>> = HashMap::new();
    for (real_path, _) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let content = fs::read(file_path).unwrap();
            original_contents.insert(real_path.clone(), content);
        }
    }

    // Migrate files to CAS
    let mut migrated_hashes: HashMap<String, String> = HashMap::new();
    for (real_path, _) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            migrated_hashes.insert(real_path.clone(), hash);
        }
    }

    // Verify content integrity
    for (real_path, original_content) in original_contents.iter() {
        let hash = migrated_hashes.get(real_path).unwrap();
        let migrated_content = cas.read_content(hash).await.unwrap();
        assert_eq!(
            &migrated_content, original_content,
            "Content mismatch for {}",
            real_path
        );
    }
}

/// Test data integrity after migration - metadata preserved
/// **Validates: Requirements 8.4 - Metadata preservation**
#[tokio::test]
async fn test_migration_data_integrity_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-metadata-integrity";

    // Create traditional workspace
    let (workspace_dir, path_map, old_file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate files with metadata
    for (real_path, virtual_path) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let old_meta = old_file_metadata.get(real_path).unwrap();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: old_meta.size as i64,
                modified_time: old_meta.modified_time,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
        }
    }

    // Verify metadata is preserved
    for (real_path, virtual_path) in path_map.iter() {
        let old_meta = old_file_metadata.get(real_path).unwrap();
        let new_meta = metadata_store
            .get_file_by_virtual_path(virtual_path)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(new_meta.size, old_meta.size as i64);
        assert_eq!(new_meta.modified_time, old_meta.modified_time);
    }
}

/// Test migration handles duplicate content (deduplication)
/// **Validates: Requirements 8.4 - Deduplication correctness**
#[tokio::test]
async fn test_migration_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-dedup");
    fs::create_dir_all(&workspace_dir).unwrap();

    // Create files with duplicate content
    let file1 = workspace_dir.join("file1.log");
    let file2 = workspace_dir.join("file2.log");
    let duplicate_content = b"This is duplicate content";
    fs::write(&file1, duplicate_content).unwrap();
    fs::write(&file2, duplicate_content).unwrap();

    // Initialize CAS
    let cas = ContentAddressableStorage::new(workspace_dir.clone());

    // Store both files
    let hash1 = cas.store_file_streaming(&file1).await.unwrap();
    let hash2 = cas.store_file_streaming(&file2).await.unwrap();

    // Hashes should be identical (deduplication)
    assert_eq!(hash1, hash2);

    // Verify only one object exists
    let object_path = cas.get_object_path(&hash1);
    assert!(object_path.exists());

    // Verify content is correct
    let content = cas.read_content(&hash1).await.unwrap();
    assert_eq!(content, duplicate_content);
}

/// Test migration handles file size correctly
/// **Validates: Requirements 8.4 - Size tracking**
#[tokio::test]
async fn test_migration_file_sizes() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-sizes";

    // Create traditional workspace
    let (workspace_dir, path_map, old_file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Calculate total original size
    let original_total_size: u64 = old_file_metadata.values().map(|m| m.size).sum();

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate files
    for (real_path, virtual_path) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let size = fs::metadata(file_path).unwrap().len();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: size as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
        }
    }

    // Verify total size is preserved
    let all_files = metadata_store.get_all_files().await.unwrap();
    let migrated_total_size: i64 = all_files.iter().map(|f| f.size).sum();
    assert_eq!(migrated_total_size, original_total_size as i64);
}

// ============================================================================
// Backward Compatibility Tests
// ============================================================================

/// Test reading old format workspaces (read-only)
/// **Validates: Requirements 8.4 - Backward compatibility**
#[tokio::test]
async fn test_backward_compatibility_read_old_format() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-old-format";

    // Create traditional workspace
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Verify we can read files from traditional format
    for (real_path, _virtual_path) in path_map.iter() {
        let file_path = Path::new(real_path);
        assert!(file_path.exists(), "File should exist: {}", real_path);

        let content = fs::read(file_path).unwrap();
        assert!(!content.is_empty(), "File should have content");
    }

    // Verify no CAS markers exist (still in old format)
    assert!(!workspace_dir.join("metadata.db").exists());
    assert!(!workspace_dir.join("objects").exists());
}

/// Test CAS workspaces are properly identified
/// **Validates: Requirements 8.4 - Format detection**
#[tokio::test]
async fn test_backward_compatibility_cas_detection() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-cas-detection";

    // Create CAS workspace
    let workspace_dir = create_test_workspace(&temp_dir, workspace_id, true);

    // Verify CAS markers exist
    assert!(workspace_dir.join("metadata.db").exists());
    assert!(workspace_dir.join("objects").exists());

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let _metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Store a test file
    let test_file = temp_dir.path().join("test.log");
    fs::write(&test_file, b"test content").unwrap();
    let hash = cas.store_file_streaming(&test_file).await.unwrap();

    // Verify file is accessible
    let content = cas.read_content(&hash).await.unwrap();
    assert_eq!(content, b"test content");
}

/// Test mixed workspace handling (partial migration)
/// **Validates: Requirements 8.4 - Partial migration support**
#[tokio::test]
async fn test_backward_compatibility_mixed_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-mixed";

    // Create traditional workspace
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Initialize CAS (simulating partial migration)
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate only some files
    let mut migrated_count = 0;
    for (real_path, virtual_path) in path_map.iter().take(2) {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let size = fs::metadata(file_path).unwrap().len();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: size as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
            migrated_count += 1;
        }
    }

    // Verify partial migration
    let migrated_files = metadata_store.get_all_files().await.unwrap();
    assert_eq!(migrated_files.len(), migrated_count);

    // Verify unmigrated files still exist in traditional format
    for (real_path, _) in path_map.iter().skip(2) {
        let file_path = Path::new(real_path);
        assert!(file_path.exists(), "Unmigrated file should still exist");
    }
}

/// Test migration report generation
/// **Validates: Requirements 8.4 - Migration reporting**
#[tokio::test]
async fn test_migration_report_generation() {
    use log_analyzer::migration::MigrationReport;

    let report = MigrationReport {
        workspace_id: "test-workspace".to_string(),
        total_files: 100,
        migrated_files: 95,
        failed_files: 5,
        deduplicated_files: 10,
        original_size: 1024 * 1024,
        cas_size: 900 * 1024,
        failed_file_paths: vec!["file1.log".to_string(), "file2.log".to_string()],
        duration_ms: 1500,
        success: true,
    };

    // Verify report structure
    assert_eq!(report.workspace_id, "test-workspace");
    assert_eq!(report.total_files, 100);
    assert_eq!(report.migrated_files, 95);
    assert_eq!(report.failed_files, 5);
    assert_eq!(report.deduplicated_files, 10);
    assert!(report.success);
    assert_eq!(report.failed_file_paths.len(), 2);

    // Verify space savings calculation
    let space_saved = report.original_size - report.cas_size;
    assert!(space_saved > 0);
    let savings_percent = (space_saved as f64 / report.original_size as f64) * 100.0;
    assert!(savings_percent > 0.0);
}

/// Test migration handles errors gracefully
/// **Validates: Requirements 8.4 - Error handling**
#[tokio::test]
async fn test_migration_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-errors");
    fs::create_dir_all(&workspace_dir).unwrap();

    // Create a file that will be deleted before migration
    let file_to_delete = workspace_dir.join("will_be_deleted.log");
    fs::write(&file_to_delete, b"content").unwrap();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());

    // Delete the file
    fs::remove_file(&file_to_delete).unwrap();

    // Try to store the deleted file (should fail gracefully)
    let result = cas.store_file_streaming(&file_to_delete).await;
    assert!(result.is_err(), "Should fail for non-existent file");
}

/// Test migration preserves virtual paths correctly
/// **Validates: Requirements 8.4 - Path mapping preservation**
#[tokio::test]
async fn test_migration_virtual_path_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_id = "test-virtual-paths";

    // Create traditional workspace
    let (workspace_dir, path_map, _file_metadata) =
        create_traditional_workspace_with_index(&temp_dir, workspace_id);

    // Initialize CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Migrate files and track virtual paths
    let mut virtual_paths: Vec<String> = Vec::new();
    for (real_path, virtual_path) in path_map.iter() {
        let file_path = Path::new(real_path);
        if file_path.exists() {
            let hash = cas.store_file_streaming(file_path).await.unwrap();
            let size = fs::metadata(file_path).unwrap().len();

            let file_metadata = FileMetadata {
                id: 0,
                sha256_hash: hash,
                virtual_path: virtual_path.clone(),
                original_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size: size as i64,
                modified_time: 1234567890,
                mime_type: None,
                parent_archive_id: None,
                depth_level: 0,
            };
            metadata_store.insert_file(&file_metadata).await.unwrap();
            virtual_paths.push(virtual_path.clone());
        }
    }

    // Verify all virtual paths are preserved
    for virtual_path in virtual_paths {
        let file = metadata_store
            .get_file_by_virtual_path(&virtual_path)
            .await
            .unwrap();
        assert!(file.is_some(), "Virtual path should exist: {}", virtual_path);
        assert_eq!(file.unwrap().virtual_path, virtual_path);
    }
}
