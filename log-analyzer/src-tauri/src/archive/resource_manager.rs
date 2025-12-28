//! Resource Management for Archive Extraction
//!
//! This module implements resource management and cleanup for archive extraction operations.
//! It handles:
//! - Temporary directory cleanup with TTL (default 24 hours)
//! - File handle release within 5 seconds of completion
//! - Memory buffer cleanup using Drop trait
//! - Workspace cleanup on deletion (remove path mappings, temp files)

use crate::error::{AppError, Result};
use crate::services::MetadataDB;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Default TTL for temporary files (24 hours)
const DEFAULT_TEMP_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Maximum time to release resources after completion (5 seconds)
const MAX_RESOURCE_RELEASE_TIME: Duration = Duration::from_secs(5);

/// Resource manager for handling cleanup and resource lifecycle
pub struct ResourceManager {
    /// Metadata database for path mappings
    pub(crate) metadata_db: Arc<MetadataDB>,
    /// Base temporary directory
    pub(crate) temp_base_dir: PathBuf,
    /// TTL for temporary files
    temp_ttl: Duration,
    /// Active file handles (tracked for cleanup)
    active_handles: Arc<RwLock<Vec<FileHandle>>>,
}

/// Represents a tracked file handle
#[derive(Debug, Clone)]
struct FileHandle {
    path: PathBuf,
    #[allow(dead_code)]
    opened_at: SystemTime,
    workspace_id: String,
}

impl ResourceManager {
    /// Create a new resource manager
    ///
    /// # Arguments
    ///
    /// * `metadata_db` - Metadata database for path mappings
    /// * `temp_base_dir` - Base directory for temporary files
    /// * `temp_ttl` - Optional TTL for temporary files (default: 24 hours)
    ///
    /// # Returns
    ///
    /// A new ResourceManager instance
    pub fn new(
        metadata_db: Arc<MetadataDB>,
        temp_base_dir: PathBuf,
        temp_ttl: Option<Duration>,
    ) -> Self {
        let ttl = temp_ttl.unwrap_or(DEFAULT_TEMP_TTL);

        info!(
            "Initializing ResourceManager with temp_dir={:?}, ttl={:?}",
            temp_base_dir, ttl
        );

        Self {
            metadata_db,
            temp_base_dir,
            temp_ttl: ttl,
            active_handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the temporary directory for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    ///
    /// # Returns
    ///
    /// Path to the workspace's temporary directory
    pub fn get_workspace_temp_dir(&self, workspace_id: &str) -> PathBuf {
        self.temp_base_dir.join(workspace_id).join("temp")
    }

    /// Register a file handle for tracking
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `workspace_id` - Workspace identifier
    pub async fn register_file_handle(&self, path: PathBuf, workspace_id: String) {
        let handle = FileHandle {
            path: path.clone(),
            opened_at: SystemTime::now(),
            workspace_id,
        };

        let mut handles = self.active_handles.write().await;
        handles.push(handle);

        debug!("Registered file handle: {:?}", path);
    }

    /// Unregister a file handle
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    pub async fn unregister_file_handle(&self, path: &Path) {
        let mut handles = self.active_handles.write().await;
        handles.retain(|h| h.path != path);

        debug!("Unregistered file handle: {:?}", path);
    }

    /// Release all file handles for a workspace
    ///
    /// This method ensures all file handles are released within 5 seconds.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    ///
    /// # Returns
    ///
    /// Number of handles released
    pub async fn release_workspace_handles(&self, workspace_id: &str) -> Result<usize> {
        let start = SystemTime::now();

        let mut handles = self.active_handles.write().await;
        let initial_count = handles.len();

        // Remove handles for this workspace
        handles.retain(|h| h.workspace_id != workspace_id);

        let released_count = initial_count - handles.len();

        let elapsed = start.elapsed().unwrap_or(Duration::ZERO);
        if elapsed > MAX_RESOURCE_RELEASE_TIME {
            warn!(
                "Resource release took {:?}, exceeding limit of {:?}",
                elapsed, MAX_RESOURCE_RELEASE_TIME
            );
        }

        info!(
            "Released {} file handles for workspace {} in {:?}",
            released_count, workspace_id, elapsed
        );

        Ok(released_count)
    }

    /// Clean up temporary files older than TTL
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Optional workspace identifier (if None, cleans all workspaces)
    ///
    /// # Returns
    ///
    /// Number of files cleaned up
    pub async fn cleanup_temp_files(&self, workspace_id: Option<&str>) -> Result<usize> {
        let now = SystemTime::now();
        let mut cleaned_count = 0;

        let base_dir = match workspace_id {
            Some(ws_id) => self.get_workspace_temp_dir(ws_id),
            None => self.temp_base_dir.clone(),
        };

        if !base_dir.exists() {
            return Ok(0);
        }

        cleaned_count += self.cleanup_directory_recursive(&base_dir, now).await?;

        info!(
            "Cleaned up {} temporary files from {:?}",
            cleaned_count, base_dir
        );

        Ok(cleaned_count)
    }

    /// Recursively clean up a directory
    fn cleanup_directory_recursive<'a>(
        &'a self,
        dir: &'a Path,
        now: SystemTime,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize>> + Send + 'a>> {
        Box::pin(async move {
            let mut cleaned_count = 0;

            let mut entries = fs::read_dir(dir).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to read directory: {}", e),
                    Some(dir.to_path_buf()),
                )
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to read directory entry: {}", e),
                    Some(dir.to_path_buf()),
                )
            })? {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to read metadata: {}", e),
                        Some(path.clone()),
                    )
                })?;

                if metadata.is_dir() {
                    // Recursively clean subdirectories
                    cleaned_count += self.cleanup_directory_recursive(&path, now).await?;

                    // Try to remove empty directory
                    if let Ok(mut sub_entries) = fs::read_dir(&path).await {
                        if sub_entries.next_entry().await.ok().flatten().is_none() {
                            if let Err(e) = fs::remove_dir(&path).await {
                                debug!("Failed to remove empty directory {:?}: {}", path, e);
                            } else {
                                debug!("Removed empty directory: {:?}", path);
                            }
                        }
                    }
                } else if metadata.is_file() {
                    // Check file age
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age > self.temp_ttl {
                                // File is older than TTL, remove it
                                match fs::remove_file(&path).await {
                                    Ok(_) => {
                                        debug!(
                                            "Removed old temp file: {:?} (age: {:?})",
                                            path, age
                                        );
                                        cleaned_count += 1;
                                    }
                                    Err(e) => {
                                        warn!("Failed to remove temp file {:?}: {}", path, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(cleaned_count)
        })
    }

    /// Clean up all resources for a workspace
    ///
    /// This method performs complete cleanup including:
    /// - Path mappings from database
    /// - Temporary files
    /// - File handles
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace identifier
    ///
    /// # Returns
    ///
    /// Cleanup statistics
    pub async fn cleanup_workspace(&self, workspace_id: &str) -> Result<CleanupStats> {
        info!("Starting workspace cleanup for: {}", workspace_id);

        let start = SystemTime::now();

        // Release file handles
        let handles_released = self.release_workspace_handles(workspace_id).await?;

        // Clean up path mappings
        let mappings_removed = self
            .metadata_db
            .cleanup_workspace(workspace_id)
            .await
            .map_err(|e| {
                AppError::archive_error(format!("Failed to cleanup path mappings: {}", e), None)
            })?;

        // Clean up temporary files (force cleanup regardless of TTL)
        let temp_dir = self.get_workspace_temp_dir(workspace_id);
        let temp_files_removed = if temp_dir.exists() {
            let removed = self.remove_directory_recursive(&temp_dir).await?;

            // Remove the workspace temp directory itself
            if let Err(e) = fs::remove_dir_all(&temp_dir).await {
                warn!(
                    "Failed to remove workspace temp directory {:?}: {}",
                    temp_dir, e
                );
            }

            removed
        } else {
            0
        };

        let elapsed = start.elapsed().unwrap_or(Duration::ZERO);

        let stats = CleanupStats {
            handles_released,
            mappings_removed,
            temp_files_removed,
            cleanup_duration: elapsed,
        };

        info!(
            "Workspace cleanup completed for {}: {:?} in {:?}",
            workspace_id, stats, elapsed
        );

        Ok(stats)
    }

    /// Remove a directory and all its contents recursively
    #[allow(clippy::only_used_in_recursion)]
    fn remove_directory_recursive<'a>(
        &'a self,
        dir: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize>> + Send + 'a>> {
        Box::pin(async move {
            let mut removed_count = 0;

            if !dir.exists() {
                return Ok(0);
            }

            let mut entries = fs::read_dir(dir).await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to read directory: {}", e),
                    Some(dir.to_path_buf()),
                )
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                AppError::archive_error(
                    format!("Failed to read directory entry: {}", e),
                    Some(dir.to_path_buf()),
                )
            })? {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| {
                    AppError::archive_error(
                        format!("Failed to read metadata: {}", e),
                        Some(path.clone()),
                    )
                })?;

                if metadata.is_dir() {
                    removed_count += self.remove_directory_recursive(&path).await?;
                    if let Err(e) = fs::remove_dir(&path).await {
                        warn!("Failed to remove directory {:?}: {}", path, e);
                    }
                } else {
                    match fs::remove_file(&path).await {
                        Ok(_) => {
                            removed_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to remove file {:?}: {}", path, e);
                        }
                    }
                }
            }

            Ok(removed_count)
        })
    }

    /// Get the configured TTL for temporary files
    pub fn temp_ttl(&self) -> Duration {
        self.temp_ttl
    }

    /// Get the number of active file handles
    pub async fn active_handle_count(&self) -> usize {
        let handles = self.active_handles.read().await;
        handles.len()
    }
}

/// Statistics from a cleanup operation
#[derive(Debug, Default, Clone)]
pub struct CleanupStats {
    /// Number of file handles released
    pub handles_released: usize,
    /// Number of path mappings removed
    pub mappings_removed: usize,
    /// Number of temporary files removed
    pub temp_files_removed: usize,
    /// Duration of cleanup operation
    pub cleanup_duration: Duration,
}

/// RAII wrapper for memory buffers that ensures cleanup on drop
///
/// This struct implements the Drop trait to ensure memory buffers are
/// properly cleaned up when they go out of scope.
pub struct ManagedBuffer {
    buffer: Vec<u8>,
    name: String,
}

impl ManagedBuffer {
    /// Create a new managed buffer
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the buffer in bytes
    /// * `name` - Name for debugging/logging
    ///
    /// # Returns
    ///
    /// A new ManagedBuffer instance
    pub fn new(size: usize, name: String) -> Self {
        debug!("Allocating managed buffer '{}' of size {}", name, size);

        Self {
            buffer: vec![0u8; size],
            name,
        }
    }

    /// Get a mutable reference to the buffer
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// Get an immutable reference to the buffer
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the size of the buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Drop for ManagedBuffer {
    fn drop(&mut self) {
        debug!(
            "Dropping managed buffer '{}' of size {}",
            self.name,
            self.buffer.len()
        );

        // Explicitly clear the buffer to ensure memory is released
        self.buffer.clear();
        self.buffer.shrink_to_fit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::sleep;

    async fn create_test_resource_manager() -> (ResourceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let metadata_db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());

        let temp_base = temp_dir.path().join("temp");
        fs::create_dir_all(&temp_base).await.unwrap();

        let manager = ResourceManager::new(metadata_db, temp_base, Some(Duration::from_secs(1)));

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_resource_manager_creation() {
        let (manager, _temp_dir) = create_test_resource_manager().await;
        assert_eq!(manager.temp_ttl(), Duration::from_secs(1));
        assert_eq!(manager.active_handle_count().await, 0);
    }

    #[tokio::test]
    async fn test_file_handle_registration() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        let path = PathBuf::from("/test/file.txt");
        manager
            .register_file_handle(path.clone(), "workspace1".to_string())
            .await;

        assert_eq!(manager.active_handle_count().await, 1);

        manager.unregister_file_handle(&path).await;
        assert_eq!(manager.active_handle_count().await, 0);
    }

    #[tokio::test]
    async fn test_release_workspace_handles() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        // Register handles for multiple workspaces
        manager
            .register_file_handle(PathBuf::from("/test/file1.txt"), "workspace1".to_string())
            .await;
        manager
            .register_file_handle(PathBuf::from("/test/file2.txt"), "workspace1".to_string())
            .await;
        manager
            .register_file_handle(PathBuf::from("/test/file3.txt"), "workspace2".to_string())
            .await;

        assert_eq!(manager.active_handle_count().await, 3);

        // Release handles for workspace1
        let released = manager
            .release_workspace_handles("workspace1")
            .await
            .unwrap();
        assert_eq!(released, 2);
        assert_eq!(manager.active_handle_count().await, 1);

        // Release handles for workspace2
        let released = manager
            .release_workspace_handles("workspace2")
            .await
            .unwrap();
        assert_eq!(released, 1);
        assert_eq!(manager.active_handle_count().await, 0);
    }

    #[tokio::test]
    async fn test_managed_buffer() {
        let buffer_size = 1024;
        let mut buffer = ManagedBuffer::new(buffer_size, "test_buffer".to_string());

        assert_eq!(buffer.len(), buffer_size);
        assert!(!buffer.is_empty());

        // Write to buffer
        buffer.as_mut_slice()[0] = 42;
        assert_eq!(buffer.as_slice()[0], 42);

        // Buffer will be dropped at end of scope
    }

    #[tokio::test]
    async fn test_cleanup_temp_files_with_ttl() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        let workspace_id = "test_workspace";
        let workspace_temp = manager.get_workspace_temp_dir(workspace_id);
        fs::create_dir_all(&workspace_temp).await.unwrap();

        // Create some test files
        let file1 = workspace_temp.join("file1.txt");
        let file2 = workspace_temp.join("file2.txt");
        fs::write(&file1, b"test1").await.unwrap();
        fs::write(&file2, b"test2").await.unwrap();

        // Files are new, should not be cleaned up
        let cleaned = manager
            .cleanup_temp_files(Some(workspace_id))
            .await
            .unwrap();
        assert_eq!(cleaned, 0);
        assert!(file1.exists());
        assert!(file2.exists());

        // Wait for TTL to expire (1 second)
        sleep(Duration::from_secs(2)).await;

        // Now files should be cleaned up
        let cleaned = manager
            .cleanup_temp_files(Some(workspace_id))
            .await
            .unwrap();
        assert_eq!(cleaned, 2);
        assert!(!file1.exists());
        assert!(!file2.exists());
    }

    #[tokio::test]
    async fn test_workspace_cleanup() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        let workspace_id = "cleanup_workspace";

        // Register file handles
        manager
            .register_file_handle(PathBuf::from("/test/file1.txt"), workspace_id.to_string())
            .await;
        manager
            .register_file_handle(PathBuf::from("/test/file2.txt"), workspace_id.to_string())
            .await;

        // Create path mappings
        manager
            .metadata_db
            .store_mapping(workspace_id, "short1", "original1")
            .await
            .unwrap();
        manager
            .metadata_db
            .store_mapping(workspace_id, "short2", "original2")
            .await
            .unwrap();

        // Create temp files
        let workspace_temp = manager.get_workspace_temp_dir(workspace_id);
        fs::create_dir_all(&workspace_temp).await.unwrap();
        fs::write(workspace_temp.join("temp1.txt"), b"test1")
            .await
            .unwrap();
        fs::write(workspace_temp.join("temp2.txt"), b"test2")
            .await
            .unwrap();

        // Perform cleanup
        let stats = manager.cleanup_workspace(workspace_id).await.unwrap();

        assert_eq!(stats.handles_released, 2);
        assert_eq!(stats.mappings_removed, 2);
        assert_eq!(stats.temp_files_removed, 2);
        assert!(stats.cleanup_duration < Duration::from_secs(5));

        // Verify cleanup
        assert_eq!(manager.active_handle_count().await, 0);
        assert!(!workspace_temp.exists());

        let mappings = manager
            .metadata_db
            .get_workspace_mappings(workspace_id)
            .await
            .unwrap();
        assert_eq!(mappings.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_nonexistent_workspace() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        let stats = manager.cleanup_workspace("nonexistent").await.unwrap();

        assert_eq!(stats.handles_released, 0);
        assert_eq!(stats.mappings_removed, 0);
        assert_eq!(stats.temp_files_removed, 0);
    }

    #[tokio::test]
    async fn test_cleanup_nested_directories() {
        let (manager, _temp_dir) = create_test_resource_manager().await;

        let workspace_id = "nested_workspace";
        let workspace_temp = manager.get_workspace_temp_dir(workspace_id);

        // Create nested directory structure
        let nested_dir = workspace_temp.join("level1").join("level2");
        fs::create_dir_all(&nested_dir).await.unwrap();

        // Create files at different levels
        fs::write(workspace_temp.join("root.txt"), b"root")
            .await
            .unwrap();
        fs::write(workspace_temp.join("level1").join("level1.txt"), b"level1")
            .await
            .unwrap();
        fs::write(nested_dir.join("level2.txt"), b"level2")
            .await
            .unwrap();

        // Perform cleanup
        let stats = manager.cleanup_workspace(workspace_id).await.unwrap();

        assert_eq!(stats.temp_files_removed, 3);
        assert!(!workspace_temp.exists());
    }
}
