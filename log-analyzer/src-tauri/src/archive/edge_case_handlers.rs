/**
 * Edge Case Handlers Module
 *
 * Handles various edge cases in archive extraction:
 * - Unicode normalization (NFC form)
 * - Duplicate filename detection
 * - Incomplete extraction detection using checkpoint files
 * - Disk space pre-flight checks
 * - Circular reference detection
 */
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;
use unicode_normalization::UnicodeNormalization;

/// Checkpoint file format for resumable extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionCheckpoint {
    pub workspace_id: String,
    pub archive_path: PathBuf,
    pub last_extracted_file: String,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub timestamp: i64,
}

/// Edge case handler for archive extraction
pub struct EdgeCaseHandler {
    /// Set of canonical paths visited (for circular reference detection)
    visited_paths: HashSet<PathBuf>,
    /// Case-insensitive filename tracker (for Windows duplicate detection)
    extracted_filenames: HashSet<String>,
    /// Whether to use case-insensitive comparison (Windows)
    case_insensitive: bool,
}

impl EdgeCaseHandler {
    /// Create a new edge case handler
    pub fn new() -> Self {
        Self {
            visited_paths: HashSet::new(),
            extracted_filenames: HashSet::new(),
            case_insensitive: cfg!(windows),
        }
    }

    /// Normalize a path to NFC form
    ///
    /// This ensures consistent Unicode representation across different platforms,
    /// particularly important for macOS which uses NFD normalization.
    pub fn normalize_path(&self, path: &str) -> String {
        path.nfc().collect::<String>()
    }

    /// Check if a filename is a duplicate and generate a unique name if needed
    ///
    /// On Windows, uses case-insensitive comparison. Returns the original name
    /// if unique, or appends a numeric suffix (_001, _002, etc.) if duplicate.
    pub fn ensure_unique_filename(&mut self, filename: &str) -> String {
        let normalized = self.normalize_path(filename);
        let comparison_key = if self.case_insensitive {
            normalized.to_lowercase()
        } else {
            normalized.clone()
        };

        if !self.extracted_filenames.contains(&comparison_key) {
            self.extracted_filenames.insert(comparison_key);
            return normalized;
        }

        // Generate unique filename with counter
        let path = Path::new(&normalized);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let mut counter = 1;
        loop {
            let unique_name = if extension.is_empty() {
                format!("{}_{:03}", stem, counter)
            } else {
                format!("{}_{:03}.{}", stem, counter, extension)
            };

            let unique_key = if self.case_insensitive {
                unique_name.to_lowercase()
            } else {
                unique_name.clone()
            };

            if !self.extracted_filenames.contains(&unique_key) {
                self.extracted_filenames.insert(unique_key);
                return unique_name;
            }

            counter += 1;
            if counter > 9999 {
                // Safety limit to prevent infinite loop
                return format!("{}_{}", stem, uuid::Uuid::new_v4());
            }
        }
    }

    /// Check if a path creates a circular reference
    ///
    /// Uses canonical paths to detect cycles in symlinks or nested archives.
    /// Returns true if the path would create a cycle.
    pub fn is_circular_reference(&mut self, path: &Path) -> Result<bool> {
        // Try to canonicalize the path
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If canonicalization fails, the path doesn't exist yet or is invalid
                // This is not a circular reference
                return Ok(false);
            }
        };

        // Check if we've already visited this canonical path
        if self.visited_paths.contains(&canonical) {
            return Ok(true);
        }

        // Add to visited set
        self.visited_paths.insert(canonical);
        Ok(false)
    }

    /// Clear the visited paths set (for starting a new extraction)
    pub fn clear_visited_paths(&mut self) {
        self.visited_paths.clear();
    }

    /// Clear the extracted filenames set (for starting a new extraction)
    pub fn clear_extracted_filenames(&mut self) {
        self.extracted_filenames.clear();
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.clear_visited_paths();
        self.clear_extracted_filenames();
    }
}

impl Default for EdgeCaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Check available disk space before extraction
///
/// Returns an error if insufficient space is available.
/// Estimates required space based on archive size and compression ratio.
pub fn check_disk_space(target_dir: &Path, estimated_size: u64, safety_margin: f64) -> Result<()> {
    let disks = Disks::new_with_refreshed_list();

    // Find the disk containing the target directory
    let target_canonical = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.to_path_buf());

    let mut available_space: Option<u64> = None;

    for disk in disks.list() {
        let mount_point = disk.mount_point();
        if target_canonical.starts_with(mount_point) {
            available_space = Some(disk.available_space());
            break;
        }
    }

    let available = available_space.ok_or_else(|| {
        AppError::archive_error(
            format!("Could not determine disk space for {:?}", target_dir),
            Some(target_dir.to_path_buf()),
        )
    })?;

    let required = (estimated_size as f64 * (1.0 + safety_margin)) as u64;

    if available < required {
        return Err(AppError::archive_error(
            format!(
                "Insufficient disk space: {} bytes available, {} bytes required (including {}% safety margin)",
                available,
                required,
                (safety_margin * 100.0) as u64
            ),
            Some(target_dir.to_path_buf()),
        ));
    }

    Ok(())
}

/// Save extraction checkpoint to disk
pub fn save_checkpoint(checkpoint: &ExtractionCheckpoint, checkpoint_dir: &Path) -> Result<()> {
    fs::create_dir_all(checkpoint_dir).map_err(|e| {
        AppError::archive_error(
            format!("Failed to create checkpoint directory: {}", e),
            Some(checkpoint_dir.to_path_buf()),
        )
    })?;

    let checkpoint_file =
        checkpoint_dir.join(format!("checkpoint_{}.json", checkpoint.workspace_id));

    let json = serde_json::to_string_pretty(checkpoint).map_err(|e| {
        AppError::archive_error(
            format!("Failed to serialize checkpoint: {}", e),
            Some(checkpoint_file.clone()),
        )
    })?;

    fs::write(&checkpoint_file, json).map_err(|e| {
        AppError::archive_error(
            format!("Failed to write checkpoint file: {}", e),
            Some(checkpoint_file.clone()),
        )
    })?;

    Ok(())
}

/// Load extraction checkpoint from disk
pub fn load_checkpoint(
    workspace_id: &str,
    checkpoint_dir: &Path,
) -> Result<Option<ExtractionCheckpoint>> {
    let checkpoint_file = checkpoint_dir.join(format!("checkpoint_{}.json", workspace_id));

    if !checkpoint_file.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&checkpoint_file).map_err(|e| {
        AppError::archive_error(
            format!("Failed to read checkpoint file: {}", e),
            Some(checkpoint_file.clone()),
        )
    })?;

    let checkpoint: ExtractionCheckpoint = serde_json::from_str(&json).map_err(|e| {
        AppError::archive_error(
            format!("Failed to deserialize checkpoint: {}", e),
            Some(checkpoint_file.clone()),
        )
    })?;

    Ok(Some(checkpoint))
}

/// Delete extraction checkpoint from disk
pub fn delete_checkpoint(workspace_id: &str, checkpoint_dir: &Path) -> Result<()> {
    let checkpoint_file = checkpoint_dir.join(format!("checkpoint_{}.json", workspace_id));

    if checkpoint_file.exists() {
        fs::remove_file(&checkpoint_file).map_err(|e| {
            AppError::archive_error(
                format!("Failed to delete checkpoint file: {}", e),
                Some(checkpoint_file.clone()),
            )
        })?;
    }

    Ok(())
}

/// Detect incomplete extraction by checking for checkpoint file
pub fn detect_incomplete_extraction(workspace_id: &str, checkpoint_dir: &Path) -> Result<bool> {
    let checkpoint_file = checkpoint_dir.join(format!("checkpoint_{}.json", workspace_id));
    Ok(checkpoint_file.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_unicode_normalization() {
        let handler = EdgeCaseHandler::new();

        // Test NFC normalization
        let nfd = "e\u{0301}"; // é in NFD form (e + combining acute)
        let nfc = "é"; // é in NFC form (single character)

        let normalized = handler.normalize_path(nfd);
        assert_eq!(normalized, nfc);
    }

    #[test]
    fn test_unique_filename_generation() {
        let mut handler = EdgeCaseHandler::new();

        // First file should be unchanged
        let name1 = handler.ensure_unique_filename("test.txt");
        assert_eq!(name1, "test.txt");

        // Second file with same name should get suffix
        let name2 = handler.ensure_unique_filename("test.txt");
        assert_eq!(name2, "test_001.txt");

        // Third file should increment
        let name3 = handler.ensure_unique_filename("test.txt");
        assert_eq!(name3, "test_002.txt");
    }

    #[test]
    fn test_unique_filename_no_extension() {
        let mut handler = EdgeCaseHandler::new();

        let name1 = handler.ensure_unique_filename("README");
        assert_eq!(name1, "README");

        let name2 = handler.ensure_unique_filename("README");
        assert_eq!(name2, "README_001");
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut handler = EdgeCaseHandler::new();
        let temp_dir = TempDir::new().unwrap();

        let path1 = temp_dir.path().join("file1.txt");
        fs::write(&path1, "test").unwrap();

        // First visit should not be circular
        assert!(!handler.is_circular_reference(&path1).unwrap());

        // Second visit should be circular
        assert!(handler.is_circular_reference(&path1).unwrap());
    }

    #[test]
    fn test_disk_space_check() {
        let temp_dir = TempDir::new().unwrap();

        // Should succeed with small size (or fail with disk detection error)
        let result = check_disk_space(temp_dir.path(), 1024, 0.1);
        if let Err(e) = &result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Could not determine disk space")
                    || error_msg.contains("Insufficient disk space"),
                "Unexpected error: {}",
                error_msg
            );
        }

        // Should fail with impossibly large size (or fail with disk detection error)
        let result = check_disk_space(temp_dir.path(), u64::MAX, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");

        let checkpoint = ExtractionCheckpoint {
            workspace_id: "test_workspace".to_string(),
            archive_path: PathBuf::from("/path/to/archive.zip"),
            last_extracted_file: "file.txt".to_string(),
            files_processed: 42,
            bytes_processed: 1024,
            timestamp: 1234567890,
        };

        // Save checkpoint
        save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();

        // Load checkpoint
        let loaded = load_checkpoint("test_workspace", &checkpoint_dir)
            .unwrap()
            .unwrap();

        assert_eq!(loaded.workspace_id, checkpoint.workspace_id);
        assert_eq!(loaded.archive_path, checkpoint.archive_path);
        assert_eq!(loaded.last_extracted_file, checkpoint.last_extracted_file);
        assert_eq!(loaded.files_processed, checkpoint.files_processed);
        assert_eq!(loaded.bytes_processed, checkpoint.bytes_processed);
    }

    #[test]
    fn test_checkpoint_detection() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");

        // No checkpoint initially
        assert!(!detect_incomplete_extraction("test_workspace", &checkpoint_dir).unwrap());

        // Create checkpoint
        let checkpoint = ExtractionCheckpoint {
            workspace_id: "test_workspace".to_string(),
            archive_path: PathBuf::from("/path/to/archive.zip"),
            last_extracted_file: "file.txt".to_string(),
            files_processed: 42,
            bytes_processed: 1024,
            timestamp: 1234567890,
        };
        save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();

        // Should detect incomplete extraction
        assert!(detect_incomplete_extraction("test_workspace", &checkpoint_dir).unwrap());

        // Delete checkpoint
        delete_checkpoint("test_workspace", &checkpoint_dir).unwrap();

        // Should no longer detect incomplete extraction
        assert!(!detect_incomplete_extraction("test_workspace", &checkpoint_dir).unwrap());
    }

    /// Integration test for interruption recovery
    /// **Validates: Requirements 7.3**
    ///
    /// Simulates an interrupted extraction by:
    /// 1. Creating a checkpoint mid-extraction
    /// 2. Verifying the checkpoint is detected on "restart"
    /// 3. Simulating resume by loading the checkpoint
    /// 4. Verifying cleanup after successful completion
    #[test]
    fn test_interruption_recovery_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");
        let workspace_id = "test_workspace_recovery";

        // Phase 1: Simulate extraction in progress
        let checkpoint = ExtractionCheckpoint {
            workspace_id: workspace_id.to_string(),
            archive_path: PathBuf::from("/test/large_archive.zip"),
            last_extracted_file: "dir/subdir/file_500.txt".to_string(),
            files_processed: 500,
            bytes_processed: 50_000_000, // 50MB processed
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Save checkpoint (simulating periodic checkpoint during extraction)
        save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();

        // Phase 2: Simulate process crash/restart - detect incomplete extraction
        let has_incomplete = detect_incomplete_extraction(workspace_id, &checkpoint_dir).unwrap();
        assert!(
            has_incomplete,
            "Should detect incomplete extraction after restart"
        );

        // Phase 3: Load checkpoint to resume
        let loaded_checkpoint = load_checkpoint(workspace_id, &checkpoint_dir)
            .unwrap()
            .expect("Checkpoint should exist");

        // Verify checkpoint data is intact
        assert_eq!(loaded_checkpoint.workspace_id, workspace_id);
        assert_eq!(loaded_checkpoint.files_processed, 500);
        assert_eq!(loaded_checkpoint.bytes_processed, 50_000_000);
        assert_eq!(
            loaded_checkpoint.last_extracted_file,
            "dir/subdir/file_500.txt"
        );

        // Phase 4: Simulate successful completion - cleanup checkpoint
        delete_checkpoint(workspace_id, &checkpoint_dir).unwrap();

        // Verify checkpoint is removed
        let has_incomplete_after =
            detect_incomplete_extraction(workspace_id, &checkpoint_dir).unwrap();
        assert!(
            !has_incomplete_after,
            "Should not detect incomplete extraction after cleanup"
        );

        // Verify loading returns None after deletion
        let loaded_after_delete = load_checkpoint(workspace_id, &checkpoint_dir).unwrap();
        assert!(
            loaded_after_delete.is_none(),
            "Should return None after checkpoint deletion"
        );
    }

    /// Test multiple concurrent extractions with separate checkpoints
    #[test]
    fn test_multiple_workspace_checkpoints() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");

        let workspaces = vec!["workspace_1", "workspace_2", "workspace_3"];

        // Create checkpoints for multiple workspaces
        for (i, workspace_id) in workspaces.iter().enumerate() {
            let checkpoint = ExtractionCheckpoint {
                workspace_id: workspace_id.to_string(),
                archive_path: PathBuf::from(format!("/test/archive_{}.zip", i)),
                last_extracted_file: format!("file_{}.txt", i * 100),
                files_processed: i * 100,
                bytes_processed: (i as u64) * 10_000_000,
                timestamp: chrono::Utc::now().timestamp(),
            };
            save_checkpoint(&checkpoint, &checkpoint_dir).unwrap();
        }

        // Verify all checkpoints are detected independently
        for workspace_id in &workspaces {
            assert!(
                detect_incomplete_extraction(workspace_id, &checkpoint_dir).unwrap(),
                "Should detect checkpoint for {}",
                workspace_id
            );
        }

        // Load and verify each checkpoint
        for (i, workspace_id) in workspaces.iter().enumerate() {
            let loaded = load_checkpoint(workspace_id, &checkpoint_dir)
                .unwrap()
                .expect("Checkpoint should exist");
            assert_eq!(loaded.files_processed, i * 100);
        }

        // Delete one checkpoint
        delete_checkpoint("workspace_2", &checkpoint_dir).unwrap();

        // Verify only workspace_2 checkpoint is removed
        assert!(detect_incomplete_extraction("workspace_1", &checkpoint_dir).unwrap());
        assert!(!detect_incomplete_extraction("workspace_2", &checkpoint_dir).unwrap());
        assert!(detect_incomplete_extraction("workspace_3", &checkpoint_dir).unwrap());
    }
}
