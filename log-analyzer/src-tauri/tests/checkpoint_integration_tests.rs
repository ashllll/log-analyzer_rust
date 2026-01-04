//! Integration tests for checkpoint-based resumption
//!
//! **Feature: enhanced-archive-handling, Task 15.2: Pause and resume integration test**
//! **Validates: Requirements 5.4**
//!
//! Tests that extraction can be paused and resumed without re-extracting files.

use log_analyzer::archive::{Checkpoint, CheckpointConfig, CheckpointManager};
use std::path::PathBuf;
use tempfile::TempDir;

/// Test that pause and resume works correctly
///
/// This test simulates:
/// 1. Starting an extraction
/// 2. Extracting some files and saving a checkpoint
/// 3. "Pausing" the extraction
/// 4. "Resuming" from the checkpoint
/// 5. Verifying no files are re-extracted
#[tokio::test]
async fn test_pause_and_resume_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let manager =
        CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    // Simulate initial extraction phase
    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    // Simulate extracting first batch of files
    let first_batch = vec![
        (PathBuf::from("/test/output/file1.txt"), 1024u64),
        (PathBuf::from("/test/output/file2.txt"), 2048u64),
        (PathBuf::from("/test/output/file3.txt"), 3072u64),
    ];

    for (file_path, file_size) in &first_batch {
        checkpoint.update_file(file_path.clone(), *file_size);
    }

    // Save checkpoint (simulating pause)
    manager.save_checkpoint(&checkpoint).await.unwrap();

    // Verify checkpoint was saved
    assert!(manager
        .checkpoint_exists(workspace_id, &archive_path)
        .await
        .unwrap());

    // Load checkpoint (simulating resume)
    let loaded_checkpoint = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap()
        .expect("Checkpoint should exist");

    // Verify loaded checkpoint has correct state
    assert_eq!(loaded_checkpoint.metrics.files_extracted, 3);
    assert_eq!(loaded_checkpoint.metrics.bytes_extracted, 6144);

    // Simulate second batch of files (resume phase)
    let second_batch = [
        (PathBuf::from("/test/output/file4.txt"), 4096u64),
        (PathBuf::from("/test/output/file5.txt"), 5120u64),
    ];

    let mut resumed_checkpoint = loaded_checkpoint;
    let mut skipped_count = 0;
    let mut extracted_count = 0;

    // Process all files, skipping already extracted ones
    for (file_path, file_size) in first_batch.iter().chain(second_batch.iter()) {
        if resumed_checkpoint.is_file_extracted(file_path) {
            // File was already extracted, skip it
            skipped_count += 1;
        } else {
            // File needs to be extracted
            resumed_checkpoint.update_file(file_path.clone(), *file_size);
            extracted_count += 1;
        }
    }

    // Verify that first batch files were skipped
    assert_eq!(skipped_count, 3, "Should skip 3 already-extracted files");
    assert_eq!(extracted_count, 2, "Should extract 2 new files");

    // Verify final state
    assert_eq!(resumed_checkpoint.metrics.files_extracted, 5);
    assert_eq!(resumed_checkpoint.metrics.bytes_extracted, 15360);

    // All files should now be marked as extracted
    for (file_path, _) in first_batch.iter().chain(second_batch.iter()) {
        assert!(
            resumed_checkpoint.is_file_extracted(file_path),
            "File {:?} should be marked as extracted",
            file_path
        );
    }

    // Save final checkpoint
    manager.save_checkpoint(&resumed_checkpoint).await.unwrap();

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}

/// Test that checkpoint prevents duplicate extraction across multiple resume cycles
#[tokio::test]
async fn test_multiple_pause_resume_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let manager =
        CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    // Create initial checkpoint
    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    // Simulate multiple pause/resume cycles
    let batches = vec![
        vec![
            (PathBuf::from("/test/output/batch1_file1.txt"), 1000u64),
            (PathBuf::from("/test/output/batch1_file2.txt"), 2000u64),
        ],
        vec![
            (PathBuf::from("/test/output/batch2_file1.txt"), 3000u64),
            (PathBuf::from("/test/output/batch2_file2.txt"), 4000u64),
        ],
        vec![
            (PathBuf::from("/test/output/batch3_file1.txt"), 5000u64),
            (PathBuf::from("/test/output/batch3_file2.txt"), 6000u64),
        ],
    ];

    let mut total_extracted = 0;
    let mut total_bytes = 0;

    for (batch_num, batch) in batches.iter().enumerate() {
        // Extract files in this batch
        for (file_path, file_size) in batch {
            if !checkpoint.is_file_extracted(file_path) {
                checkpoint.update_file(file_path.clone(), *file_size);
                total_extracted += 1;
                total_bytes += file_size;
            }
        }

        // Save checkpoint after each batch (simulating pause)
        manager.save_checkpoint(&checkpoint).await.unwrap();

        // Load checkpoint (simulating resume)
        checkpoint = manager
            .load_checkpoint(workspace_id, &archive_path)
            .await
            .unwrap()
            .expect("Checkpoint should exist");

        // Verify state after each cycle
        assert_eq!(
            checkpoint.metrics.files_extracted,
            (batch_num + 1) * 2,
            "Should have extracted {} files after batch {}",
            (batch_num + 1) * 2,
            batch_num + 1
        );
    }

    // Verify final state
    assert_eq!(total_extracted, 6);
    assert_eq!(total_bytes, 21000);
    assert_eq!(checkpoint.metrics.files_extracted, 6);
    assert_eq!(checkpoint.metrics.bytes_extracted, 21000);

    // Verify no duplicates if we try to "re-extract" everything
    let mut duplicate_count = 0;
    for batch in &batches {
        for (file_path, _) in batch {
            if checkpoint.is_file_extracted(file_path) {
                duplicate_count += 1;
            }
        }
    }
    assert_eq!(
        duplicate_count, 6,
        "All files should be marked as extracted"
    );

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}

/// Test checkpoint behavior when extraction is interrupted mid-file
#[tokio::test]
async fn test_interruption_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let manager =
        CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    // Create checkpoint and extract some files
    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    // Extract first two files successfully
    checkpoint.update_file(PathBuf::from("/test/output/file1.txt"), 1024);
    checkpoint.update_file(PathBuf::from("/test/output/file2.txt"), 2048);

    // Save checkpoint
    manager.save_checkpoint(&checkpoint).await.unwrap();

    // Simulate interruption: third file starts but doesn't complete
    // (In real scenario, process would crash here)

    // Resume: load checkpoint
    let resumed = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap()
        .expect("Checkpoint should exist");

    // Verify only completed files are in checkpoint
    assert_eq!(resumed.metrics.files_extracted, 2);
    assert_eq!(resumed.metrics.bytes_extracted, 3072);

    // File 3 should not be marked as extracted
    assert!(!resumed.is_file_extracted(&PathBuf::from("/test/output/file3.txt")));

    // Can safely re-attempt file 3
    let mut final_checkpoint = resumed;
    final_checkpoint.update_file(PathBuf::from("/test/output/file3.txt"), 3072);

    assert_eq!(final_checkpoint.metrics.files_extracted, 3);
    assert_eq!(final_checkpoint.metrics.bytes_extracted, 6144);

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}

/// Test that checkpoint write intervals are respected
#[tokio::test]
async fn test_checkpoint_write_intervals() {
    let temp_dir = TempDir::new().unwrap();
    let config = CheckpointConfig {
        file_interval: 100,
        byte_interval: 1024 * 1024 * 1024, // 1GB
        enabled: true,
    };
    let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

    let workspace_id = "test_workspace";
    let archive_path = PathBuf::from("/test/archive.zip");
    let target_dir = PathBuf::from("/test/output");

    let mut checkpoint = Checkpoint::new(
        workspace_id.to_string(),
        archive_path.clone(),
        target_dir.clone(),
    );

    let mut files_since_checkpoint = 0;
    let mut bytes_since_checkpoint = 0u64;

    // Simulate extracting files
    for i in 0..250 {
        let file_path = PathBuf::from(format!("/test/output/file{}.txt", i));
        let file_size = 1024u64;

        checkpoint.update_file(file_path, file_size);
        files_since_checkpoint += 1;
        bytes_since_checkpoint += file_size;

        // Check if we should write checkpoint
        if manager.should_write_checkpoint(files_since_checkpoint, bytes_since_checkpoint) {
            manager.save_checkpoint(&checkpoint).await.unwrap();
            files_since_checkpoint = 0;
            bytes_since_checkpoint = 0;
        }
    }

    // Verify checkpoint was written (should have been written at 100 and 200 files)
    let loaded = manager
        .load_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    // Should have at least 200 files (2 checkpoints at 100-file intervals)
    assert!(loaded.metrics.files_extracted >= 200);

    // Cleanup
    manager
        .delete_checkpoint(workspace_id, &archive_path)
        .await
        .unwrap();
}
