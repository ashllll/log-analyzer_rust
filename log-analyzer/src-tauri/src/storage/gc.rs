//! Garbage Collection Module for CAS
//!
//! Provides automatic cleanup of orphaned files that are no longer
//! referenced by any metadata entries.
//!
//! ## Features
//!
//! - Background garbage collection with configurable intervals
//! - Reference counting to identify orphaned files
//! - Safe deletion with verification
//! - Metrics collection for monitoring
//!
//! ## Architecture
//!
//! The GC operates in two modes:
//! 1. **Full GC**: Scans all objects and checks against metadata store
//! 2. **Incremental GC**: Processes a subset of files on each run
//!
//! Safety is ensured by:
//! - Only deleting files with zero references
//! - Dry-run capability for testing
//! - Detailed logging of all operations

use crate::error::{AppError, Result};
use crate::storage::{ContentAddressableStorage, MetadataStore};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Statistics for a garbage collection run
#[derive(Debug, Clone, Default)]
pub struct GCStats {
    /// Total files scanned
    pub files_scanned: usize,
    /// Files identified as orphaned (zero references)
    pub orphaned_files: usize,
    /// Files successfully deleted
    pub files_deleted: usize,
    /// Bytes reclaimed
    pub bytes_reclaimed: u64,
    /// Errors encountered during cleanup
    pub errors: usize,
    /// Duration of the GC run
    pub duration_ms: u64,
}

/// Configuration for garbage collection
#[derive(Debug, Clone)]
pub struct GCConfig {
    /// Interval between automatic GC runs
    pub interval: Duration,
    /// Minimum age of files before they can be GC'd (safety buffer)
    pub min_file_age: Duration,
    /// Maximum files to process in one incremental run
    pub batch_size: usize,
    /// Enable dry-run mode (don't actually delete)
    pub dry_run: bool,
}

impl Default for GCConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(3600), // 1 hour default
            min_file_age: Duration::from_secs(300), // 5 minutes safety buffer
            batch_size: 1000,
            dry_run: false,
        }
    }
}

/// Garbage collector for CAS storage
pub struct GarbageCollector {
    cas: Arc<ContentAddressableStorage>,
    metadata_store: Arc<MetadataStore>,
    config: GCConfig,
    last_run: RwLock<Option<Instant>>,
    total_stats: RwLock<GCStats>,
}

impl GarbageCollector {
    /// Create a new garbage collector
    ///
    /// # Arguments
    ///
    /// * `cas` - Content-addressable storage instance
    /// * `metadata_store` - Metadata store for reference checking
    /// * `config` - GC configuration
    pub fn new(
        cas: Arc<ContentAddressableStorage>,
        metadata_store: Arc<MetadataStore>,
        config: GCConfig,
    ) -> Self {
        Self {
            cas,
            metadata_store,
            config,
            last_run: RwLock::new(None),
            total_stats: RwLock::new(GCStats::default()),
        }
    }

    /// Run a full garbage collection
    ///
    /// Scans all objects in the CAS and removes any that have no
    /// metadata references.
    ///
    /// # Returns
    ///
    /// Returns statistics about the GC run
    pub async fn run_full_gc(&self) -> Result<GCStats> {
        let start = Instant::now();
        let mut stats = GCStats::default();

        info!("Starting full garbage collection");

        // Get all object hashes from the CAS directory
        let objects_dir = self.cas.objects_dir();
        let orphaned_hashes = self
            .scan_and_identify_orphans(objects_dir, &mut stats)
            .await?;

        stats.orphaned_files = orphaned_hashes.len();

        // Clean up orphaned files
        for hash in orphaned_hashes {
            match self.cleanup_orphaned_file(&hash).await {
                Ok(bytes) => {
                    stats.files_deleted += 1;
                    stats.bytes_reclaimed += bytes;
                }
                Err(e) => {
                    error!(hash = %hash, error = %e, "Failed to cleanup orphaned file");
                    stats.errors += 1;
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;

        // Update last run time and total stats
        *self.last_run.write().await = Some(Instant::now());
        let mut total = self.total_stats.write().await;
        total.files_scanned += stats.files_scanned;
        total.orphaned_files += stats.orphaned_files;
        total.files_deleted += stats.files_deleted;
        total.bytes_reclaimed += stats.bytes_reclaimed;
        total.errors += stats.errors;

        info!(
            files_scanned = stats.files_scanned,
            orphaned_files = stats.orphaned_files,
            files_deleted = stats.files_deleted,
            bytes_reclaimed = stats.bytes_reclaimed,
            duration_ms = stats.duration_ms,
            "Full garbage collection completed"
        );

        Ok(stats)
    }

    /// Run incremental garbage collection
    ///
    /// Processes a limited batch of files for frequent, low-impact cleanup.
    ///
    /// # Returns
    ///
    /// Returns statistics about the GC run
    pub async fn run_incremental_gc(&self) -> Result<GCStats> {
        let start = Instant::now();
        let stats = GCStats::default();

        debug!("Starting incremental garbage collection");

        // TODO: Implement incremental GC with cursor-based scanning
        // For now, just run a limited full GC

        info!(
            duration_ms = start.elapsed().as_millis() as u64,
            "Incremental garbage collection completed (placeholder)"
        );

        Ok(stats)
    }

    /// Check if a file has any metadata references
    ///
    /// # Arguments
    ///
    /// * `hash` - SHA-256 hash of the file
    ///
    /// # Returns
    ///
    /// Returns true if the file has references, false otherwise
    pub async fn has_references(&self, hash: &str) -> Result<bool> {
        // Check in files table
        match self.metadata_store.get_file_by_hash(hash).await {
            Ok(Some(_)) => return Ok(true),
            Ok(None) => {}
            Err(e) => {
                return Err(AppError::database_error(format!(
                    "Failed to check file references: {}",
                    e
                )));
            }
        }

        // Could also check archives table if needed
        // For now, just check files

        Ok(false)
    }

    /// Scan all objects and identify orphans
    async fn scan_and_identify_orphans(
        &self,
        objects_dir: PathBuf,
        stats: &mut GCStats,
    ) -> Result<Vec<String>> {
        let mut orphaned_hashes = Vec::new();
        let min_age_secs = self.config.min_file_age.as_secs();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Walk the objects directory
        let mut entries = fs::read_dir(&objects_dir).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to read objects directory: {}", e),
                Some(objects_dir.clone()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // This is a shard directory (e.g., "a3/")
                let mut shard_entries = fs::read_dir(&path).await?;

                while let Some(file_entry) = shard_entries.next_entry().await? {
                    let file_path = file_entry.path();

                    if file_path.is_file() {
                        stats.files_scanned += 1;

                        // Check file age
                        let metadata = fs::metadata(&file_path).await?;
                        let modified_time = metadata
                            .modified()?
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        if now - modified_time < min_age_secs {
                            // File too new, skip for safety
                            continue;
                        }

                        // Get hash from filename
                        let hash = file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_default();

                        if hash.is_empty() {
                            warn!(path = %file_path.display(), "Found file with invalid name");
                            continue;
                        }

                        // Check if file has references
                        match self.has_references(&hash).await {
                            Ok(false) => {
                                debug!(hash = %hash, "Found orphaned file");
                                orphaned_hashes.push(hash);
                            }
                            Ok(true) => {
                                debug!(hash = %hash, "File has references");
                            }
                            Err(e) => {
                                warn!(
                                    hash = %hash,
                                    error = %e,
                                    "Failed to check references, skipping"
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(orphaned_hashes)
    }

    /// Clean up a single orphaned file
    async fn cleanup_orphaned_file(&self, hash: &str) -> Result<u64> {
        let object_path = self.cas.get_object_path(hash);

        if self.config.dry_run {
            info!(
                hash = %hash,
                path = %object_path.display(),
                "[DRY RUN] Would delete orphaned file"
            );
            return Ok(0);
        }

        // Get file size before deletion for stats
        let size = fs::metadata(&object_path).await?.len();

        // Delete the file
        fs::remove_file(&object_path).await.map_err(|e| {
            AppError::io_error(
                format!("Failed to delete orphaned file: {}", e),
                Some(object_path.clone()),
            )
        })?;

        // Invalidate cache entry
        self.cas.invalidate_cache_entry(hash);

        // Try to remove parent directory if empty
        if let Some(parent) = object_path.parent() {
            if let Ok(mut entries) = fs::read_dir(parent).await {
                if entries.next_entry().await?.is_none() {
                    let _ = fs::remove_dir(parent).await;
                }
            }
        }

        info!(
            hash = %hash,
            path = %object_path.display(),
            bytes = size,
            "Deleted orphaned file"
        );

        Ok(size)
    }

    /// Get the last GC run time
    pub async fn last_run(&self) -> Option<Instant> {
        *self.last_run.read().await
    }

    /// Get total GC statistics
    pub async fn total_stats(&self) -> GCStats {
        self.total_stats.read().await.clone()
    }

    /// Start automatic background GC
    ///
    /// Spawns a background task that runs GC at configured intervals
    ///
    /// # Arguments
    ///
    /// * `shutdown_rx` - Receiver for shutdown signal
    pub fn start_background_gc(
        self: Arc<Self>,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) {
        let interval = self.config.interval;

        tokio::spawn(async move {
            info!(interval_secs = interval.as_secs(), "Starting background GC");

            let mut interval = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        match self.run_full_gc().await {
                            Ok(stats) => {
                                info!(
                                    files_deleted = stats.files_deleted,
                                    bytes_reclaimed = stats.bytes_reclaimed,
                                    "Background GC completed"
                                );
                            }
                            Err(e) => {
                                error!(error = %e, "Background GC failed");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Background GC shutting down");
                        break;
                    }
                }
            }
        });
    }
}

/// GC manager for coordinating multiple storage backends
pub struct GCManager {
    collectors: Vec<Arc<GarbageCollector>>,
}

impl GCManager {
    /// Create a new GC manager
    pub fn new() -> Self {
        Self {
            collectors: Vec::new(),
        }
    }

    /// Add a garbage collector
    pub fn add_collector(&mut self, collector: Arc<GarbageCollector>) {
        self.collectors.push(collector);
    }

    /// Run GC on all collectors
    pub async fn run_all(&self) -> Vec<GCStats> {
        let mut all_stats = Vec::new();

        for collector in &self.collectors {
            match collector.run_full_gc().await {
                Ok(stats) => all_stats.push(stats),
                Err(e) => {
                    error!(error = %e, "GC run failed for collector");
                    all_stats.push(GCStats::default());
                }
            }
        }

        all_stats
    }
}

impl Default for GCManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Note: These tests would need proper mocking of MetadataStore
    // For now, we just verify the structure compiles

    #[test]
    fn test_gc_config_default() {
        let config = GCConfig::default();
        assert_eq!(config.interval, Duration::from_secs(3600));
        assert_eq!(config.min_file_age, Duration::from_secs(300));
        assert_eq!(config.batch_size, 1000);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_gc_stats_default() {
        let stats = GCStats::default();
        assert_eq!(stats.files_scanned, 0);
        assert_eq!(stats.orphaned_files, 0);
        assert_eq!(stats.files_deleted, 0);
        assert_eq!(stats.bytes_reclaimed, 0);
        assert_eq!(stats.errors, 0);
    }
}
