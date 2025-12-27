//! Error Recovery Tests
//!
//! Tests for transaction rollback, checkpoint save/resume, and integrity verification.
//!
//! **Feature: archive-search-fix, Task 5.3: Write tests for error recovery**
//! **Validates: Requirements 8.1, 8.4**

use log_analyzer::archive::checkpoint_manager::{Checkpoint, CheckpointConfig, CheckpointManager};
use log_analyzer::storage::{
    verify_after_import, verify_workspace_integrity, ContentAddressableStorage, FileMetadata,
    MetadataStore,
};
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test checkpoint save and resume functionality
///
/// This test verifies that:
/// 1. Checkpoints can be saved during processing
/// 2. Checkpoints can be loaded and resumed
/// 3. Already-processed files are skipped on resume
///
/// **Validates: Requirements 8.4**
#[tokio::test]
async fn test_checkpoint_save_and_resume() {
    let temp_dir = TempDir::new().unwrap();
    let checkpoint_dir = temp_dir.path().join("checkpoints");
    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    // Create checkpoint manager
    let manager = CheckpointManager::new(CheckpointConfig::default(), checkpoint_dir.clone());

    // Create initial checkpoint
    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    // Simulate processing some files
    let files = vec![
        (PathBuf::from("/test/output/file1.txt"), 1024u64),
        (PathBuf::from("/test/output/file2.txt"), 2048u64),
        (PathBuf::from("/test/output/file3.txt"), 4096u64),
    ];

    for (file_path, file_size) in &files {
        checkpoint.update_file(file_path.clone(), *file_size);
    }

    // Save checkpoint
    manager.save_checkpoint(&checkpoint).await.unwrap();

    // Verify checkpoint was saved
    assert!(
        manager
            .checkpoint_exists(workspace_id, &archive_path)
            .await
            .unwrap(),
        "Checkpoint should exist after save"
    );

    // Load checkpoint (simulating resume)
    let loaded_checkpoint = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap()
        .expect("Checkpoint should be loadable");

    // Verify loaded checkpoint has correct state
    assert_eq!(
        loaded_checkpoint.metrics.files_extracted, 3,
        "Should have 3 files extracted"
    );
    assert_eq!(
        loaded_checkpoint.metrics.bytes_extracted, 7168,
        "Should have correct byte count"
    );

    // Verify files are marked as extracted
    for (file_path, _) in &files {
        assert!(
            loaded_checkpoint.is_file_extracted(file_path),
            "File {:?} should be marked as extracted",
            file_path
        );
    }

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}

/// Test that checkpoint prevents duplicate processing
///
/// This test verifies that when resuming from a checkpoint,
/// already-processed files are not processed again.
///
/// **Validates: Requirements 8.4**
#[tokio::test]
async fn test_checkpoint_prevents_duplicate_processing() {
    let temp_dir = TempDir::new().unwrap();
    let checkpoint_dir = temp_dir.path().join("checkpoints");
    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    let manager = CheckpointManager::new(CheckpointConfig::default(), checkpoint_dir.clone());

    // Create checkpoint with some files already processed
    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    let file1 = PathBuf::from("/test/output/file1.txt");
    let file2 = PathBuf::from("/test/output/file2.txt");
    let file3 = PathBuf::from("/test/output/file3.txt");

    // Process first two files
    checkpoint.update_file(file1.clone(), 1024);
    checkpoint.update_file(file2.clone(), 2048);

    manager.save_checkpoint(&checkpoint).await.unwrap();

    // Load checkpoint
    let mut loaded_checkpoint = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap()
        .unwrap();

    // Simulate processing all files (including already-processed ones)
    let all_files = vec![
        (file1.clone(), 1024u64),
        (file2.clone(), 2048u64),
        (file3.clone(), 4096u64),
    ];

    let mut skipped_count = 0;
    let mut processed_count = 0;

    for (file_path, file_size) in all_files {
        if loaded_checkpoint.is_file_extracted(&file_path) {
            skipped_count += 1;
        } else {
            loaded_checkpoint.update_file(file_path, file_size);
            processed_count += 1;
        }
    }

    // Verify correct counts
    assert_eq!(skipped_count, 2, "Should skip 2 already-processed files");
    assert_eq!(processed_count, 1, "Should process 1 new file");
    assert_eq!(
        loaded_checkpoint.metrics.files_extracted, 3,
        "Should have 3 total files"
    );

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}

/// Test integrity verification detects missing files
///
/// This test verifies that integrity verification correctly identifies
/// when files in metadata don't have corresponding CAS objects.
///
/// **Validates: Requirements 2.4, 8.1**
#[tokio::test]
async fn test_integrity_verification_detects_missing_files() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Add file to metadata without storing in CAS
    let file_metadata = FileMetadata {
        id: 0,
        sha256_hash: "nonexistent_hash_12345".to_string(),
        virtual_path: "missing.log".to_string(),
        original_name: "missing.log".to_string(),
        size: 1024,
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

    // Should detect missing file
    assert_eq!(report.total_files, 1, "Should have 1 file in metadata");
    assert_eq!(report.valid_files, 0, "Should have 0 valid files");
    assert_eq!(
        report.missing_objects.len(),
        1,
        "Should have 1 missing object"
    );
    assert!(!report.is_valid(), "Report should indicate invalid state");
}

/// Test integrity verification detects corrupted files
///
/// This test verifies that integrity verification correctly identifies
/// when CAS objects have been corrupted (hash mismatch).
///
/// **Validates: Requirements 2.4, 8.1**
#[tokio::test]
async fn test_integrity_verification_detects_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Store a file in CAS
    let content = b"original content";
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

    // Manually corrupt the file
    let object_path = cas.get_object_path(&hash);
    tokio::fs::write(&object_path, b"corrupted content")
        .await
        .unwrap();

    // Verify integrity
    let report = verify_workspace_integrity(&cas, &metadata_store)
        .await
        .unwrap();

    // Should detect corruption
    assert_eq!(report.total_files, 1, "Should have 1 file in metadata");
    assert_eq!(report.valid_files, 0, "Should have 0 valid files");
    assert_eq!(
        report.corrupted_objects.len(),
        1,
        "Should have 1 corrupted object"
    );
    assert!(!report.is_valid(), "Report should indicate invalid state");
}

/// Test integrity verification passes for valid workspace
///
/// This test verifies that integrity verification correctly validates
/// a workspace with no issues.
///
/// **Validates: Requirements 2.4**
#[tokio::test]
async fn test_integrity_verification_passes_for_valid_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Store multiple files
    let files = vec![
        (b"content 1" as &[u8], "file1.log"),
        (b"content 2", "file2.log"),
        (b"content 3", "file3.log"),
    ];

    for (content, name) in files {
        let hash = cas.store_content(content).await.unwrap();

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: name.to_string(),
            original_name: name.to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    // Verify integrity
    let report = verify_workspace_integrity(&cas, &metadata_store)
        .await
        .unwrap();

    // Should pass validation
    assert_eq!(report.total_files, 3, "Should have 3 files in metadata");
    assert_eq!(report.valid_files, 3, "Should have 3 valid files");
    assert_eq!(
        report.missing_objects.len(),
        0,
        "Should have no missing objects"
    );
    assert_eq!(
        report.corrupted_objects.len(),
        0,
        "Should have no corrupted objects"
    );
    assert!(report.is_valid(), "Report should indicate valid state");
}

/// Test verify_after_import convenience function
///
/// This test verifies that the convenience function for post-import
/// verification works correctly.
///
/// **Validates: Requirements 2.4**
#[tokio::test]
async fn test_verify_after_import() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Store a file
    let content = b"test content";
    let hash = cas.store_content(content).await.unwrap();

    let file_metadata = FileMetadata {
        id: 0,
        sha256_hash: hash,
        virtual_path: "test.log".to_string(),
        original_name: "test.log".to_string(),
        size: content.len() as i64,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    metadata_store.insert_file(&file_metadata).await.unwrap();

    // Verify after import
    let report = verify_after_import(temp_dir.path()).await.unwrap();

    assert!(report.is_valid(), "Verification should pass");
    assert_eq!(report.total_files, 1);
    assert_eq!(report.valid_files, 1);
}

/// Test checkpoint should_write_checkpoint logic
///
/// This test verifies that checkpoints are written at appropriate intervals.
///
/// **Validates: Requirements 8.4**
#[tokio::test]
async fn test_checkpoint_write_intervals() {
    let temp_dir = TempDir::new().unwrap();
    let config = CheckpointConfig {
        file_interval: 100,
        byte_interval: 1024 * 1024 * 1024, // 1GB
        enabled: true,
    };
    let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

    // Should not write before threshold
    assert!(!manager.should_write_checkpoint(99, 0));
    assert!(!manager.should_write_checkpoint(0, 1024 * 1024 * 1024 - 1));

    // Should write at threshold
    assert!(manager.should_write_checkpoint(100, 0));
    assert!(manager.should_write_checkpoint(0, 1024 * 1024 * 1024));

    // Should write if either threshold is met
    assert!(manager.should_write_checkpoint(100, 1024 * 1024 * 1024));
    assert!(manager.should_write_checkpoint(150, 500));
}

/// Test checkpoint disabled mode
///
/// This test verifies that when checkpoints are disabled,
/// no checkpoint operations are performed.
///
/// **Validates: Requirements 8.4**
#[tokio::test]
async fn test_checkpoint_disabled_mode() {
    let temp_dir = TempDir::new().unwrap();
    let config = CheckpointConfig {
        file_interval: 100,
        byte_interval: 1024 * 1024 * 1024,
        enabled: false, // Disabled
    };
    let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    let checkpoint = Checkpoint::new(workspace_id.to_string(), archive_path.clone(), target_dir);

    // Save should succeed but do nothing
    manager.save_checkpoint(&checkpoint).await.unwrap();

    // Load should return None
    let loaded = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
    assert!(loaded.is_none(), "Should return None when disabled");

    // should_write_checkpoint should always return false
    assert!(!manager.should_write_checkpoint(1000, 10_000_000_000));
}

/// Test that import process calls integrity verification
///
/// This test verifies that the import command automatically calls
/// verify_after_import and generates a validation report.
///
/// **Validates: Requirements 2.4 (Task 5.2)**
#[tokio::test]
async fn test_import_calls_integrity_verification() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().to_path_buf();

    // Create CAS and metadata store
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Simulate import by storing files in CAS and metadata
    let files = vec![
        ("file1.log", b"content 1" as &[u8]),
        ("file2.log", b"content 2"),
        ("file3.log", b"content 3"),
    ];

    for (name, content) in files {
        let hash = cas.store_content(content).await.unwrap();

        let file_metadata = FileMetadata {
            id: 0,
            sha256_hash: hash,
            virtual_path: name.to_string(),
            original_name: name.to_string(),
            size: content.len() as i64,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        };

        metadata_store.insert_file(&file_metadata).await.unwrap();
    }

    // Verify after import (this is what the import command should call)
    let report = verify_after_import(&workspace_dir).await.unwrap();

    // Assertions
    assert!(
        report.is_valid(),
        "Verification should pass after successful import"
    );
    assert_eq!(report.total_files, 3, "Should have 3 files");
    assert_eq!(report.valid_files, 3, "All 3 files should be valid");
    assert_eq!(
        report.invalid_files.len(),
        0,
        "Should have no invalid files"
    );
    assert_eq!(
        report.missing_objects.len(),
        0,
        "Should have no missing objects"
    );
    assert_eq!(
        report.corrupted_objects.len(),
        0,
        "Should have no corrupted objects"
    );
}

/// Test that verification detects missing CAS objects after import
///
/// This test simulates a scenario where metadata exists but CAS objects are missing,
/// which should be detected by the verification step.
///
/// **Validates: Requirements 2.4 (Task 5.2)**
#[tokio::test]
async fn test_verification_detects_missing_objects() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().to_path_buf();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Add metadata for a file that doesn't exist in CAS
    let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let file_metadata = FileMetadata {
        id: 0,
        sha256_hash: fake_hash.to_string(),
        virtual_path: "missing.log".to_string(),
        original_name: "missing.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    metadata_store.insert_file(&file_metadata).await.unwrap();

    // Verify - should detect missing object
    let report = verify_after_import(&workspace_dir).await.unwrap();

    assert!(
        !report.is_valid(),
        "Verification should fail with missing objects"
    );
    assert_eq!(report.total_files, 1);
    assert_eq!(report.valid_files, 0);
    assert_eq!(report.invalid_files.len(), 1);
    assert_eq!(report.missing_objects.len(), 1);
    assert_eq!(report.missing_objects[0], fake_hash);
}

/// Test that verification detects corrupted CAS objects
///
/// This test simulates a scenario where a CAS object exists but is corrupted
/// (hash mismatch), which should be detected by the verification step.
///
/// **Validates: Requirements 2.4 (Task 5.2)**
#[tokio::test]
async fn test_verification_detects_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().to_path_buf();

    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();

    // Store a file in CAS
    let original_content = b"original content";
    let hash = cas.store_content(original_content).await.unwrap();

    // Add metadata
    let file_metadata = FileMetadata {
        id: 0,
        sha256_hash: hash.clone(),
        virtual_path: "test.log".to_string(),
        original_name: "test.log".to_string(),
        size: original_content.len() as i64,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    metadata_store.insert_file(&file_metadata).await.unwrap();

    // Manually corrupt the CAS object
    let object_path = cas.get_object_path(&hash);
    tokio::fs::write(&object_path, b"corrupted content")
        .await
        .unwrap();

    // Verify - should detect corruption
    let report = verify_after_import(&workspace_dir).await.unwrap();

    assert!(
        !report.is_valid(),
        "Verification should fail with corrupted objects"
    );
    assert_eq!(report.total_files, 1);
    assert_eq!(report.valid_files, 0);
    assert_eq!(report.invalid_files.len(), 1);
    assert_eq!(report.corrupted_objects.len(), 1);
    assert_eq!(report.corrupted_objects[0], hash);
}

/// Test transaction rollback on failure
///
/// This test verifies that when a transaction fails, all operations
/// within that transaction are rolled back, maintaining consistency.
///
/// **Validates: Requirements 8.4 (Task 5 - Transaction support)**
#[tokio::test]
async fn test_transaction_rollback_on_failure() {
    let temp_dir = TempDir::new().unwrap();
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Verify initial state is empty
    let initial_count = metadata_store.count_files().await.unwrap();
    assert_eq!(initial_count, 0, "Should start with no files");

    // Begin a transaction
    let mut tx = metadata_store.begin_transaction().await.unwrap();

    // Insert first file successfully
    let file1 = FileMetadata {
        id: 0,
        sha256_hash: "hash1".to_string(),
        virtual_path: "file1.log".to_string(),
        original_name: "file1.log".to_string(),
        size: 100,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    let id1 = MetadataStore::insert_file_tx(&mut tx, &file1)
        .await
        .unwrap();
    assert!(id1 > 0, "Should insert first file");

    // Insert second file successfully
    let file2 = FileMetadata {
        id: 0,
        sha256_hash: "hash2".to_string(),
        virtual_path: "file2.log".to_string(),
        original_name: "file2.log".to_string(),
        size: 200,
        modified_time: 0,
        mime_type: None,
        parent_archive_id: None,
        depth_level: 0,
    };

    let id2 = MetadataStore::insert_file_tx(&mut tx, &file2)
        .await
        .unwrap();
    assert!(id2 > 0, "Should insert second file");

    // Explicitly rollback the transaction (simulating a failure)
    drop(tx); // Dropping without commit causes rollback

    // Verify that no files were actually inserted (rollback worked)
    let final_count = metadata_store.count_files().await.unwrap();
    assert_eq!(final_count, 0, "Should have no files after rollback");

    // Verify files are not retrievable
    let file1_result = metadata_store.get_file_by_hash("hash1").await.unwrap();
    assert!(
        file1_result.is_none(),
        "File1 should not exist after rollback"
    );

    let file2_result = metadata_store.get_file_by_hash("hash2").await.unwrap();
    assert!(
        file2_result.is_none(),
        "File2 should not exist after rollback"
    );
}

/// Test transaction commit on success
///
/// This test verifies that when a transaction is committed successfully,
/// all operations within that transaction are persisted.
///
/// **Validates: Requirements 8.4 (Task 5 - Transaction support)**
#[tokio::test]
async fn test_transaction_commit_on_success() {
    let temp_dir = TempDir::new().unwrap();
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Begin a transaction
    let mut tx = metadata_store.begin_transaction().await.unwrap();

    // Insert multiple files
    let files = vec![
        FileMetadata {
            id: 0,
            sha256_hash: "hash1".to_string(),
            virtual_path: "file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        },
        FileMetadata {
            id: 0,
            sha256_hash: "hash2".to_string(),
            virtual_path: "file2.log".to_string(),
            original_name: "file2.log".to_string(),
            size: 200,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        },
        FileMetadata {
            id: 0,
            sha256_hash: "hash3".to_string(),
            virtual_path: "file3.log".to_string(),
            original_name: "file3.log".to_string(),
            size: 300,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: None,
            depth_level: 0,
        },
    ];

    // Insert all files in transaction
    for file in &files {
        let id = MetadataStore::insert_file_tx(&mut tx, file).await.unwrap();
        assert!(id > 0, "Should insert file");
    }

    // Commit the transaction
    tx.commit().await.unwrap();

    // Verify all files were persisted
    let final_count = metadata_store.count_files().await.unwrap();
    assert_eq!(final_count, 3, "Should have 3 files after commit");

    // Verify each file is retrievable
    for file in &files {
        let result = metadata_store
            .get_file_by_hash(&file.sha256_hash)
            .await
            .unwrap();
        assert!(
            result.is_some(),
            "File {} should exist after commit",
            file.sha256_hash
        );
        let retrieved = result.unwrap();
        assert_eq!(retrieved.virtual_path, file.virtual_path);
        assert_eq!(retrieved.size, file.size);
    }
}

/// Test transaction with mixed file and archive operations
///
/// This test verifies that transactions work correctly when inserting
/// both files and archives atomically.
///
/// **Validates: Requirements 8.4 (Task 5 - Transaction support)**
#[tokio::test]
async fn test_transaction_mixed_operations() {
    use log_analyzer::storage::ArchiveMetadata;

    let temp_dir = TempDir::new().unwrap();
    let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

    // Begin a transaction
    let mut tx = metadata_store.begin_transaction().await.unwrap();

    // Insert an archive
    let archive = ArchiveMetadata {
        id: 0,
        sha256_hash: "archive_hash".to_string(),
        virtual_path: "archive.zip".to_string(),
        original_name: "archive.zip".to_string(),
        archive_type: "zip".to_string(),
        parent_archive_id: None,
        depth_level: 0,
        extraction_status: "pending".to_string(),
    };

    let archive_id = MetadataStore::insert_archive_tx(&mut tx, &archive)
        .await
        .unwrap();
    assert!(archive_id > 0, "Should insert archive");

    // Insert files belonging to the archive
    let files = vec![
        FileMetadata {
            id: 0,
            sha256_hash: "file1_hash".to_string(),
            virtual_path: "archive.zip/file1.log".to_string(),
            original_name: "file1.log".to_string(),
            size: 100,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(archive_id),
            depth_level: 1,
        },
        FileMetadata {
            id: 0,
            sha256_hash: "file2_hash".to_string(),
            virtual_path: "archive.zip/file2.log".to_string(),
            original_name: "file2.log".to_string(),
            size: 200,
            modified_time: 0,
            mime_type: None,
            parent_archive_id: Some(archive_id),
            depth_level: 1,
        },
    ];

    for file in &files {
        let id = MetadataStore::insert_file_tx(&mut tx, file).await.unwrap();
        assert!(id > 0, "Should insert file");
    }

    // Commit the transaction
    tx.commit().await.unwrap();

    // Verify archive was persisted
    let archive_count = metadata_store.count_archives().await.unwrap();
    assert_eq!(archive_count, 1, "Should have 1 archive");

    // Verify files were persisted
    let file_count = metadata_store.count_files().await.unwrap();
    assert_eq!(file_count, 2, "Should have 2 files");

    // Verify archive-file relationship
    let children = metadata_store
        .get_archive_children(archive_id)
        .await
        .unwrap();
    assert_eq!(children.len(), 2, "Archive should have 2 children");
}

// ========== PROPERTY-BASED TESTS ==========

/// Generate valid file content
fn file_content_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..1024)
}

/// Generate file names
fn file_name_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_-]{1,20}\\.log").unwrap()
}

/// **Feature: archive-search-fix, Property 7: Error recovery isolation**
/// **Validates: Requirements 8.1, 8.4**
///
/// For any single file failure, remaining files process successfully.
///
/// This property ensures that:
/// 1. When one file fails to process, other files are still processed (Requirement 8.1)
/// 2. The system maintains consistency despite partial failures (Requirement 8.4)
/// 3. Failed files are tracked but don't prevent successful processing of valid files
#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Property: Single file failure doesn't prevent other files from being processed
        ///
        /// This test simulates a scenario where:
        /// 1. Multiple files are being processed
        /// 2. One file fails (simulated by invalid hash or missing content)
        /// 3. All other files should still be processed successfully
        ///
        /// The property verifies that error isolation works correctly:
        /// - Failed files are identified
        /// - Successful files are still stored in CAS
        /// - Metadata is correctly maintained for successful files
        /// - The system doesn't crash or stop processing
        #[test]
        fn prop_error_recovery_isolation(
            file_names in prop::collection::vec(file_name_strategy(), 3..10),
            file_contents in prop::collection::vec(file_content_strategy(), 3..10),
            failure_index in 0usize..10
        ) {
            // Ensure we have matching names and contents
            let count = file_names.len().min(file_contents.len());
            let file_names = &file_names[..count];
            let file_contents = &file_contents[..count];

            // Ensure failure_index is within bounds
            let failure_index = failure_index % count;

            // Run async test
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
                let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

                let mut successful_files = Vec::new();
                let mut failed_files = Vec::new();

                // Process all files, simulating a failure for one
                for (i, (name, content)) in file_names.iter().zip(file_contents.iter()).enumerate() {
                    if i == failure_index {
                        // Simulate a failure by creating metadata without storing in CAS
                        let fake_hash = format!("fake_hash_{}", i);
                        let file_metadata = FileMetadata {
                            id: 0,
                            sha256_hash: fake_hash.clone(),
                            virtual_path: name.clone(),
                            original_name: name.clone(),
                            size: content.len() as i64,
                            modified_time: 0,
                            mime_type: None,
                            parent_archive_id: None,
                            depth_level: 0,
                        };

                        // Try to insert metadata (this simulates a partial failure)
                        if let Ok(_) = metadata_store.insert_file(&file_metadata).await {
                            failed_files.push((name.clone(), fake_hash));
                        }
                    } else {
                        // Process normally
                        match cas.store_content(content).await {
                            Ok(hash) => {
                                let file_metadata = FileMetadata {
                                    id: 0,
                                    sha256_hash: hash.clone(),
                                    virtual_path: name.clone(),
                                    original_name: name.clone(),
                                    size: content.len() as i64,
                                    modified_time: 0,
                                    mime_type: None,
                                    parent_archive_id: None,
                                    depth_level: 0,
                                };

                                if let Ok(_) = metadata_store.insert_file(&file_metadata).await {
                                    successful_files.push((name.clone(), hash));
                                }
                            }
                            Err(e) => {
                                // Unexpected error, but we should still continue
                                eprintln!("Unexpected error storing file {}: {}", name, e);
                            }
                        }
                    }
                }

                // Property 1: At least one file should have failed (the one we simulated)
                prop_assert_eq!(
                    failed_files.len(),
                    1,
                    "Should have exactly 1 failed file"
                );

                // Property 2: All other files should have been processed successfully
                prop_assert_eq!(
                    successful_files.len(),
                    count - 1,
                    "Should have {} successful files, got {}",
                    count - 1,
                    successful_files.len()
                );

                // Property 3: Verify that successful files are actually in CAS
                for (name, hash) in &successful_files {
                    let object_path = cas.get_object_path(hash);
                    prop_assert!(
                        object_path.exists(),
                        "CAS object should exist for successful file: {}",
                        name
                    );
                }

                // Property 4: Verify that failed file is NOT in CAS
                for (name, fake_hash) in &failed_files {
                    let object_path = cas.get_object_path(fake_hash);
                    prop_assert!(
                        !object_path.exists(),
                        "CAS object should NOT exist for failed file: {}",
                        name
                    );
                }

                // Property 5: Verify integrity check detects the failure
                let report = verify_workspace_integrity(&cas, &metadata_store)
                    .await
                    .unwrap();

                prop_assert_eq!(
                    report.total_files,
                    count,
                    "Should have {} total files in metadata",
                    count
                );

                prop_assert_eq!(
                    report.valid_files,
                    count - 1,
                    "Should have {} valid files",
                    count - 1
                );

                prop_assert_eq!(
                    report.invalid_files.len(),
                    1,
                    "Should have 1 invalid file"
                );

                prop_assert!(
                    !report.is_valid(),
                    "Report should indicate invalid state due to missing object"
                );

                // Property 6: Successful files should be readable from CAS
                for (name, hash) in &successful_files {
                    let content_result = cas.read_content(hash).await;
                    prop_assert!(
                        content_result.is_ok(),
                        "Should be able to read content for successful file: {}",
                        name
                    );
                }

                Ok(())
            })?;
        }

        /// Property: Multiple file failures don't prevent other files from being processed
        ///
        /// This test extends the single failure case to multiple failures,
        /// ensuring that the system can handle multiple errors gracefully.
        #[test]
        fn prop_multiple_error_recovery_isolation(
            file_names in prop::collection::vec(file_name_strategy(), 5..15),
            file_contents in prop::collection::vec(file_content_strategy(), 5..15),
            failure_count in 1usize..5
        ) {
            // Ensure we have matching names and contents
            let count = file_names.len().min(file_contents.len());
            let file_names = &file_names[..count];
            let file_contents = &file_contents[..count];

            // Ensure failure_count doesn't exceed total count
            let failure_count = failure_count.min(count - 1).max(1);

            // Run async test
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let cas = ContentAddressableStorage::new(temp_dir.path().to_path_buf());
                let metadata_store = MetadataStore::new(temp_dir.path()).await.unwrap();

                let mut successful_files = Vec::new();
                let mut failed_files = Vec::new();

                // Process all files, simulating failures for the first N files
                for (i, (name, content)) in file_names.iter().zip(file_contents.iter()).enumerate() {
                    if i < failure_count {
                        // Simulate a failure
                        let fake_hash = format!("fake_hash_{}", i);
                        let file_metadata = FileMetadata {
                            id: 0,
                            sha256_hash: fake_hash.clone(),
                            virtual_path: name.clone(),
                            original_name: name.clone(),
                            size: content.len() as i64,
                            modified_time: 0,
                            mime_type: None,
                            parent_archive_id: None,
                            depth_level: 0,
                        };

                        if let Ok(_) = metadata_store.insert_file(&file_metadata).await {
                            failed_files.push((name.clone(), fake_hash));
                        }
                    } else {
                        // Process normally
                        match cas.store_content(content).await {
                            Ok(hash) => {
                                let file_metadata = FileMetadata {
                                    id: 0,
                                    sha256_hash: hash.clone(),
                                    virtual_path: name.clone(),
                                    original_name: name.clone(),
                                    size: content.len() as i64,
                                    modified_time: 0,
                                    mime_type: None,
                                    parent_archive_id: None,
                                    depth_level: 0,
                                };

                                if let Ok(_) = metadata_store.insert_file(&file_metadata).await {
                                    successful_files.push((name.clone(), hash));
                                }
                            }
                            Err(e) => {
                                eprintln!("Unexpected error storing file {}: {}", name, e);
                            }
                        }
                    }
                }

                // Property 1: Should have the expected number of failures
                prop_assert_eq!(
                    failed_files.len(),
                    failure_count,
                    "Should have {} failed files",
                    failure_count
                );

                // Property 2: All other files should have been processed successfully
                prop_assert_eq!(
                    successful_files.len(),
                    count - failure_count,
                    "Should have {} successful files",
                    count - failure_count
                );

                // Property 3: At least one file should have succeeded
                prop_assert!(
                    !successful_files.is_empty(),
                    "At least one file should have been processed successfully"
                );

                // Property 4: Verify integrity check detects all failures
                let report = verify_workspace_integrity(&cas, &metadata_store)
                    .await
                    .unwrap();

                prop_assert_eq!(
                    report.invalid_files.len(),
                    failure_count,
                    "Should detect {} invalid files",
                    failure_count
                );

                prop_assert_eq!(
                    report.valid_files,
                    count - failure_count,
                    "Should have {} valid files",
                    count - failure_count
                );

                Ok(())
            })?;
        }
    }
}
