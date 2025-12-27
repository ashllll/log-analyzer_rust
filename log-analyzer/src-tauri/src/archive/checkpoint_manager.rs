//! Checkpoint Manager for Resumable Extraction
//!
//! This module provides checkpoint support for archive extraction operations,
//! allowing extraction to be paused and resumed without re-extracting files.
//! Checkpoints are written at regular intervals (every 100 files or 1GB) and
//! cleaned up on successful completion.

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// Checkpoint file format version
const CHECKPOINT_VERSION: u32 = 1;

/// Checkpoint file extension
const CHECKPOINT_EXTENSION: &str = ".checkpoint.json";

/// Checkpoint configuration
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Write checkpoint every N files
    pub file_interval: usize,
    /// Write checkpoint every N bytes
    pub byte_interval: u64,
    /// Enable checkpoint support
    pub enabled: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            file_interval: 100,
            byte_interval: 1024 * 1024 * 1024, // 1GB
            enabled: true,
        }
    }
}

/// Accumulated metrics for checkpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CheckpointMetrics {
    /// Total files extracted so far
    pub files_extracted: usize,
    /// Total bytes extracted so far
    pub bytes_extracted: u64,
    /// Maximum depth reached
    pub max_depth_reached: usize,
    /// Number of errors encountered
    pub error_count: usize,
    /// Number of path shortenings applied
    pub path_shortenings: usize,
}

/// Checkpoint data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint format version
    pub version: u32,
    /// Workspace identifier
    pub workspace_id: String,
    /// Archive path being extracted
    pub archive_path: PathBuf,
    /// Target extraction directory
    pub target_dir: PathBuf,
    /// Last successfully extracted file
    pub last_extracted_file: Option<PathBuf>,
    /// Accumulated metrics
    pub metrics: CheckpointMetrics,
    /// Timestamp when checkpoint was created
    pub timestamp: u64,
    /// Set of all extracted files (for duplicate detection)
    pub extracted_files: HashSet<PathBuf>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(workspace_id: String, archive_path: PathBuf, target_dir: PathBuf) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            version: CHECKPOINT_VERSION,
            workspace_id,
            archive_path,
            target_dir,
            last_extracted_file: None,
            metrics: CheckpointMetrics::default(),
            timestamp,
            extracted_files: HashSet::new(),
        }
    }

    /// Update checkpoint with new file extraction
    pub fn update_file(&mut self, file_path: PathBuf, file_size: u64) {
        // Only update metrics if this is a new file (not a duplicate)
        if self.extracted_files.insert(file_path.clone()) {
            self.metrics.files_extracted += 1;
            self.metrics.bytes_extracted += file_size;
        }
        self.last_extracted_file = Some(file_path);
        self.update_timestamp();
    }

    /// Update checkpoint metrics
    pub fn update_metrics(&mut self, metrics: &CheckpointMetrics) {
        self.metrics = metrics.clone();
        self.update_timestamp();
    }

    /// Check if file was already extracted
    pub fn is_file_extracted(&self, file_path: &Path) -> bool {
        self.extracted_files.contains(file_path)
    }

    /// Update timestamp to current time
    fn update_timestamp(&mut self) {
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Checkpoint manager for handling checkpoint operations
pub struct CheckpointManager {
    config: CheckpointConfig,
    checkpoint_dir: PathBuf,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    ///
    /// # Arguments
    ///
    /// * `config` - Checkpoint configuration
    /// * `checkpoint_dir` - Directory where checkpoints are stored
    pub fn new(config: CheckpointConfig, checkpoint_dir: PathBuf) -> Self {
        Self {
            config,
            checkpoint_dir,
        }
    }

    /// Get checkpoint file path for an archive
    fn get_checkpoint_path(&self, workspace_id: &str, archive_path: &Path) -> PathBuf {
        // Create a unique checkpoint filename based on workspace and archive
        let archive_name = archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let checkpoint_name = format!(
            "{}_{}_{}",
            workspace_id,
            archive_name,
            Self::hash_path(archive_path)
        );

        self.checkpoint_dir
            .join(format!("{}{}", checkpoint_name, CHECKPOINT_EXTENSION))
    }

    /// Create a simple hash of a path for uniqueness
    fn hash_path(path: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if a checkpoint exists for an archive
    pub async fn checkpoint_exists(&self, workspace_id: &str, archive_path: &Path) -> Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        let checkpoint_path = self.get_checkpoint_path(workspace_id, archive_path);
        Ok(checkpoint_path.exists())
    }

    /// Load checkpoint from disk
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `archive_path` - Path to the archive
    ///
    /// # Returns
    ///
    /// The loaded checkpoint, or None if no checkpoint exists
    pub async fn load_checkpoint(
        &self,
        workspace_id: &str,
        archive_path: &Path,
    ) -> Result<Option<Checkpoint>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let checkpoint_path = self.get_checkpoint_path(workspace_id, archive_path);

        if !checkpoint_path.exists() {
            debug!("No checkpoint found at {:?}", checkpoint_path);
            return Ok(None);
        }

        info!("Loading checkpoint from {:?}", checkpoint_path);

        // Read checkpoint file
        let mut file = fs::File::open(&checkpoint_path).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to open checkpoint file: {}", e),
                Some(checkpoint_path.clone()),
            )
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to read checkpoint file: {}", e),
                Some(checkpoint_path.clone()),
            )
        })?;

        // Parse JSON
        let checkpoint: Checkpoint = serde_json::from_str(&contents).map_err(|e| {
            AppError::archive_error(
                format!("Failed to parse checkpoint: {}", e),
                Some(checkpoint_path.clone()),
            )
        })?;

        // Validate checkpoint version
        if checkpoint.version != CHECKPOINT_VERSION {
            warn!(
                "Checkpoint version mismatch: expected {}, got {}",
                CHECKPOINT_VERSION, checkpoint.version
            );
            return Ok(None);
        }

        info!(
            "Loaded checkpoint: {} files, {} bytes extracted",
            checkpoint.metrics.files_extracted, checkpoint.metrics.bytes_extracted
        );

        Ok(Some(checkpoint))
    }

    /// Save checkpoint to disk
    ///
    /// # Arguments
    ///
    /// * `checkpoint` - The checkpoint to save
    pub async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Ensure checkpoint directory exists
        fs::create_dir_all(&self.checkpoint_dir)
            .await
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to create checkpoint directory: {}", e),
                    Some(self.checkpoint_dir.clone()),
                )
            })?;

        let checkpoint_path =
            self.get_checkpoint_path(&checkpoint.workspace_id, &checkpoint.archive_path);

        debug!(
            "Saving checkpoint to {:?} ({} files, {} bytes)",
            checkpoint_path, checkpoint.metrics.files_extracted, checkpoint.metrics.bytes_extracted
        );

        // Serialize to JSON
        let json = serde_json::to_string_pretty(checkpoint).map_err(|e| {
            AppError::archive_error(
                format!("Failed to serialize checkpoint: {}", e),
                Some(checkpoint_path.clone()),
            )
        })?;

        // Write to temporary file first
        let temp_path = checkpoint_path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to create checkpoint file: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        file.write_all(json.as_bytes()).await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to write checkpoint: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        file.flush().await.map_err(|e| {
            AppError::archive_error(
                format!("Failed to flush checkpoint: {}", e),
                Some(temp_path.clone()),
            )
        })?;

        // Atomic rename
        fs::rename(&temp_path, &checkpoint_path)
            .await
            .map_err(|e| {
                AppError::archive_error(
                    format!("Failed to rename checkpoint: {}", e),
                    Some(checkpoint_path.clone()),
                )
            })?;

        info!(
            "Checkpoint saved: {} files, {} bytes",
            checkpoint.metrics.files_extracted, checkpoint.metrics.bytes_extracted
        );

        Ok(())
    }

    /// Delete checkpoint from disk
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    /// * `archive_path` - Path to the archive
    pub async fn delete_checkpoint(&self, workspace_id: &str, archive_path: &Path) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let checkpoint_path = self.get_checkpoint_path(workspace_id, archive_path);

        if checkpoint_path.exists() {
            info!("Deleting checkpoint: {:?}", checkpoint_path);
            fs::remove_file(&checkpoint_path).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to delete checkpoint: {}", e),
                    Some(checkpoint_path.clone()),
                )
            })?;
        }

        Ok(())
    }

    /// Check if checkpoint should be written based on progress
    ///
    /// # Arguments
    ///
    /// * `files_since_last` - Number of files extracted since last checkpoint
    /// * `bytes_since_last` - Number of bytes extracted since last checkpoint
    ///
    /// # Returns
    ///
    /// True if checkpoint should be written
    pub fn should_write_checkpoint(&self, files_since_last: usize, bytes_since_last: u64) -> bool {
        if !self.config.enabled {
            return false;
        }

        files_since_last >= self.config.file_interval
            || bytes_since_last >= self.config.byte_interval
    }

    /// Get checkpoint configuration
    pub fn config(&self) -> &CheckpointConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        assert_eq!(checkpoint.version, CHECKPOINT_VERSION);
        assert_eq!(checkpoint.workspace_id, "test_workspace");
        assert_eq!(checkpoint.last_extracted_file, None);
        assert_eq!(checkpoint.metrics.files_extracted, 0);
        assert_eq!(checkpoint.extracted_files.len(), 0);
    }

    #[test]
    fn test_checkpoint_update_file() {
        let mut checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        let file_path = PathBuf::from("/test/output/file1.txt");
        checkpoint.update_file(file_path.clone(), 1024);

        assert_eq!(checkpoint.last_extracted_file, Some(file_path.clone()));
        assert_eq!(checkpoint.metrics.files_extracted, 1);
        assert_eq!(checkpoint.metrics.bytes_extracted, 1024);
        assert_eq!(checkpoint.extracted_files.len(), 1);
        assert!(checkpoint.is_file_extracted(&file_path));
    }

    #[test]
    fn test_checkpoint_is_file_extracted() {
        let mut checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        let file1 = PathBuf::from("/test/output/file1.txt");
        let file2 = PathBuf::from("/test/output/file2.txt");

        checkpoint.update_file(file1.clone(), 1024);

        assert!(checkpoint.is_file_extracted(&file1));
        assert!(!checkpoint.is_file_extracted(&file2));
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.file_interval, 100);
        assert_eq!(config.byte_interval, 1024 * 1024 * 1024);
        assert!(config.enabled);
    }

    #[test]
    fn test_should_write_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let manager =
            CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

        // Should write after 100 files
        assert!(manager.should_write_checkpoint(100, 0));
        assert!(!manager.should_write_checkpoint(99, 0));

        // Should write after 1GB
        assert!(manager.should_write_checkpoint(0, 1024 * 1024 * 1024));
        assert!(!manager.should_write_checkpoint(0, 1024 * 1024 * 1024 - 1));

        // Should write if either threshold is met
        assert!(manager.should_write_checkpoint(100, 1024 * 1024 * 1024));
    }

    #[tokio::test]
    async fn test_checkpoint_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let manager =
            CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

        let mut checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        checkpoint.update_file(PathBuf::from("/test/output/file1.txt"), 1024);
        checkpoint.update_file(PathBuf::from("/test/output/file2.txt"), 2048);

        // Save checkpoint
        manager.save_checkpoint(&checkpoint).await.unwrap();

        // Load checkpoint
        let loaded = manager
            .load_checkpoint("test_workspace", &PathBuf::from("/test/archive.zip"))
            .await
            .unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.workspace_id, checkpoint.workspace_id);
        assert_eq!(loaded.metrics.files_extracted, 2);
        assert_eq!(loaded.metrics.bytes_extracted, 3072);
        assert_eq!(loaded.extracted_files.len(), 2);
    }

    #[tokio::test]
    async fn test_checkpoint_delete() {
        let temp_dir = TempDir::new().unwrap();
        let manager =
            CheckpointManager::new(CheckpointConfig::default(), temp_dir.path().to_path_buf());

        let checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        // Save checkpoint
        manager.save_checkpoint(&checkpoint).await.unwrap();

        // Verify it exists
        assert!(manager
            .checkpoint_exists("test_workspace", &PathBuf::from("/test/archive.zip"))
            .await
            .unwrap());

        // Delete checkpoint
        manager
            .delete_checkpoint("test_workspace", &PathBuf::from("/test/archive.zip"))
            .await
            .unwrap();

        // Verify it's gone
        assert!(!manager
            .checkpoint_exists("test_workspace", &PathBuf::from("/test/archive.zip"))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_checkpoint_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let config = CheckpointConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = CheckpointManager::new(config, temp_dir.path().to_path_buf());

        let checkpoint = Checkpoint::new(
            "test_workspace".to_string(),
            PathBuf::from("/test/archive.zip"),
            PathBuf::from("/test/output"),
        );

        // Save should succeed but do nothing
        manager.save_checkpoint(&checkpoint).await.unwrap();

        // Load should return None
        let loaded = manager
            .load_checkpoint("test_workspace", &PathBuf::from("/test/archive.zip"))
            .await
            .unwrap();
        assert!(loaded.is_none());

        // should_write_checkpoint should always return false
        assert!(!manager.should_write_checkpoint(1000, 10_000_000_000));
    }
}
