//! Workspace Metrics Service
//!
//! Collects and reports metrics for CAS-based workspaces.
//!
//! ## Metrics Tracked
//!
//! - Deduplication ratio (space saved by deduplication)
//! - Storage efficiency (actual vs theoretical storage)
//! - Maximum nesting depth of archives
//! - Total file count and size
//! - Archive count and distribution

use crate::error::{AppError, Result};
use crate::storage::{ContentAddressableStorage, MetadataStore};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Workspace metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetrics {
    /// Total number of files in workspace
    pub total_files: usize,
    /// Total number of archives
    pub total_archives: usize,
    /// Total logical size (sum of all file sizes)
    pub total_logical_size: u64,
    /// Actual storage size (CAS objects)
    pub actual_storage_size: u64,
    /// Space saved by deduplication (bytes)
    pub space_saved: u64,
    /// Deduplication ratio (0.0 to 1.0, higher is better)
    pub deduplication_ratio: f64,
    /// Storage efficiency (0.0 to 1.0, higher is better)
    pub storage_efficiency: f64,
    /// Maximum nesting depth of archives
    pub max_nesting_depth: i32,
    /// Average nesting depth
    pub avg_nesting_depth: f64,
    /// Number of unique content hashes
    pub unique_hashes: usize,
    /// Distribution of files by depth level
    pub depth_distribution: Vec<DepthDistribution>,
}

/// Distribution of files at a specific depth level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthDistribution {
    /// Depth level (0 = root)
    pub depth: i32,
    /// Number of files at this depth
    pub file_count: usize,
    /// Total size of files at this depth
    pub total_size: u64,
}

/// Workspace metrics collector
pub struct WorkspaceMetricsCollector {
    metadata_store: MetadataStore,
    cas: ContentAddressableStorage,
}

impl WorkspaceMetricsCollector {
    /// Create a new metrics collector
    ///
    /// # Arguments
    ///
    /// * `metadata_store` - Metadata store to collect from
    /// * `cas` - Content-addressable storage to measure
    pub fn new(metadata_store: MetadataStore, cas: ContentAddressableStorage) -> Self {
        Self {
            metadata_store,
            cas,
        }
    }

    /// Collect all workspace metrics
    ///
    /// Gathers comprehensive metrics about the workspace including
    /// deduplication ratio, storage efficiency, and nesting depth.
    ///
    /// # Returns
    ///
    /// Complete metrics report
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use log_analyzer::services::WorkspaceMetricsCollector;
    /// # use log_analyzer::storage::{MetadataStore, ContentAddressableStorage};
    /// # use std::path::PathBuf;
    /// # tokio_test::block_on(async {
    /// let metadata = MetadataStore::new(&PathBuf::from("./workspace")).await.unwrap();
    /// let cas = ContentAddressableStorage::new(PathBuf::from("./workspace"));
    /// let collector = WorkspaceMetricsCollector::new(metadata, cas);
    /// 
    /// let metrics = collector.collect_metrics().await.unwrap();
    /// println!("Deduplication ratio: {:.2}%", metrics.deduplication_ratio * 100.0);
    /// # })
    /// ```
    pub async fn collect_metrics(&self) -> Result<WorkspaceMetrics> {
        info!("Collecting workspace metrics");

        // Get all files from metadata
        let files = self.metadata_store.get_all_files().await?;
        let total_files = files.len();

        // Get all archives
        let archives = self.metadata_store.get_all_archives().await?;
        let total_archives = archives.len();

        // Calculate logical size (sum of all file sizes)
        let total_logical_size: u64 = files.iter().map(|f| f.size as u64).sum();

        // Get actual storage size from CAS
        let actual_storage_size = self.cas.get_storage_size().await?;

        // Calculate space saved and deduplication ratio
        let space_saved = if total_logical_size > actual_storage_size {
            total_logical_size - actual_storage_size
        } else {
            0
        };

        let deduplication_ratio = if total_logical_size > 0 {
            space_saved as f64 / total_logical_size as f64
        } else {
            0.0
        };

        // Storage efficiency (inverse of overhead)
        let storage_efficiency = if total_logical_size > 0 {
            actual_storage_size as f64 / total_logical_size as f64
        } else {
            1.0
        };

        // Find maximum nesting depth
        let max_nesting_depth = files
            .iter()
            .map(|f| f.depth_level)
            .max()
            .unwrap_or(0)
            .max(
                archives
                    .iter()
                    .map(|a| a.depth_level)
                    .max()
                    .unwrap_or(0),
            );

        // Calculate average nesting depth
        let avg_nesting_depth = if total_files > 0 {
            files.iter().map(|f| f.depth_level as f64).sum::<f64>() / total_files as f64
        } else {
            0.0
        };

        // Count unique hashes
        let unique_hashes: std::collections::HashSet<_> =
            files.iter().map(|f| &f.sha256_hash).collect();
        let unique_hashes_count = unique_hashes.len();

        // Calculate depth distribution
        let mut depth_map: std::collections::HashMap<i32, (usize, u64)> =
            std::collections::HashMap::new();

        for file in &files {
            let entry = depth_map.entry(file.depth_level).or_insert((0, 0));
            entry.0 += 1; // file count
            entry.1 += file.size as u64; // total size
        }

        let mut depth_distribution: Vec<DepthDistribution> = depth_map
            .into_iter()
            .map(|(depth, (file_count, total_size))| DepthDistribution {
                depth,
                file_count,
                total_size,
            })
            .collect();

        depth_distribution.sort_by_key(|d| d.depth);

        let metrics = WorkspaceMetrics {
            total_files,
            total_archives,
            total_logical_size,
            actual_storage_size,
            space_saved,
            deduplication_ratio,
            storage_efficiency,
            max_nesting_depth,
            avg_nesting_depth,
            unique_hashes: unique_hashes_count,
            depth_distribution,
        };

        info!(
            total_files = total_files,
            total_archives = total_archives,
            dedup_ratio = format!("{:.2}%", deduplication_ratio * 100.0),
            max_depth = max_nesting_depth,
            "Workspace metrics collected"
        );

        debug!(
            logical_size = total_logical_size,
            actual_size = actual_storage_size,
            space_saved = space_saved,
            unique_hashes = unique_hashes_count,
            "Detailed metrics"
        );

        Ok(metrics)
    }

    /// Get quick metrics summary (faster than full collection)
    ///
    /// Returns basic metrics without detailed analysis.
    ///
    /// # Returns
    ///
    /// Tuple of (total_files, total_archives, max_depth)
    pub async fn get_quick_summary(&self) -> Result<(usize, usize, i32)> {
        let files = self.metadata_store.get_all_files().await?;
        let archives = self.metadata_store.get_all_archives().await?;

        let max_depth = files
            .iter()
            .map(|f| f.depth_level)
            .max()
            .unwrap_or(0)
            .max(
                archives
                    .iter()
                    .map(|a| a.depth_level)
                    .max()
                    .unwrap_or(0),
            );

        Ok((files.len(), archives.len(), max_depth))
    }

    /// Calculate deduplication ratio only
    ///
    /// Faster than full metrics collection when only deduplication
    /// ratio is needed.
    ///
    /// # Returns
    ///
    /// Deduplication ratio (0.0 to 1.0)
    pub async fn get_deduplication_ratio(&self) -> Result<f64> {
        let files = self.metadata_store.get_all_files().await?;
        let total_logical_size: u64 = files.iter().map(|f| f.size as u64).sum();
        let actual_storage_size = self.cas.get_storage_size().await?;

        let space_saved = if total_logical_size > actual_storage_size {
            total_logical_size - actual_storage_size
        } else {
            0
        };

        let ratio = if total_logical_size > 0 {
            space_saved as f64 / total_logical_size as f64
        } else {
            0.0
        };

        Ok(ratio)
    }

    /// Get storage efficiency
    ///
    /// Returns the ratio of actual storage to logical size.
    /// Lower is better (more efficient).
    ///
    /// # Returns
    ///
    /// Storage efficiency ratio
    pub async fn get_storage_efficiency(&self) -> Result<f64> {
        let files = self.metadata_store.get_all_files().await?;
        let total_logical_size: u64 = files.iter().map(|f| f.size as u64).sum();
        let actual_storage_size = self.cas.get_storage_size().await?;

        let efficiency = if total_logical_size > 0 {
            actual_storage_size as f64 / total_logical_size as f64
        } else {
            1.0
        };

        Ok(efficiency)
    }

    /// Get maximum nesting depth
    ///
    /// # Returns
    ///
    /// Maximum depth level found in the workspace
    pub async fn get_max_nesting_depth(&self) -> Result<i32> {
        let files = self.metadata_store.get_all_files().await?;
        let archives = self.metadata_store.get_all_archives().await?;

        let max_depth = files
            .iter()
            .map(|f| f.depth_level)
            .max()
            .unwrap_or(0)
            .max(
                archives
                    .iter()
                    .map(|a| a.depth_level)
                    .max()
                    .unwrap_or(0),
            );

        Ok(max_depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{ContentAddressableStorage, MetadataStore};
    use crate::storage::metadata_store::FileMetadata;
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
    async fn test_collect_metrics_empty() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());
        let collector = WorkspaceMetricsCollector::new(metadata, cas);

        let metrics = collector.collect_metrics().await.unwrap();

        assert_eq!(metrics.total_files, 0);
        assert_eq!(metrics.total_archives, 0);
        assert_eq!(metrics.total_logical_size, 0);
        assert_eq!(metrics.actual_storage_size, 0);
        assert_eq!(metrics.space_saved, 0);
        assert_eq!(metrics.deduplication_ratio, 0.0);
        assert_eq!(metrics.max_nesting_depth, 0);
    }

    #[tokio::test]
    async fn test_collect_metrics_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();
        let file_meta = create_file_metadata(&hash, "test/file.log", "file.log", content.len() as i64, 0);
        metadata.insert_file(&file_meta).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let metrics = collector.collect_metrics().await.unwrap();

        assert_eq!(metrics.total_files, 1);
        assert_eq!(metrics.total_archives, 0);
        assert_eq!(metrics.total_logical_size, content.len() as u64);
        assert!(metrics.actual_storage_size >= content.len() as u64);
        assert_eq!(metrics.unique_hashes, 1);
        assert_eq!(metrics.max_nesting_depth, 0);
    }

    #[tokio::test]
    async fn test_deduplication_ratio() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Store different content (database has UNIQUE constraint on hash)
        let content1 = b"content 1";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(&hash1, "test/file1.log", "file1.log", 100, 0);
        metadata.insert_file(&file_meta1).await.unwrap();

        let content2 = b"content 2";
        let hash2 = cas.store_content(content2).await.unwrap();
        let file_meta2 = create_file_metadata(&hash2, "test/file2.log", "file2.log", 100, 0);
        metadata.insert_file(&file_meta2).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let metrics = collector.collect_metrics().await.unwrap();

        assert_eq!(metrics.total_files, 2);
        assert_eq!(metrics.unique_hashes, 2);
        assert!(metrics.deduplication_ratio >= 0.0);
    }

    #[tokio::test]
    async fn test_nesting_depth() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Add files at different depths
        let content1 = b"depth 0";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(&hash1, "file0.log", "file0.log", 100, 0);
        metadata.insert_file(&file_meta1).await.unwrap();

        let content2 = b"depth 2";
        let hash2 = cas.store_content(content2).await.unwrap();
        let file_meta2 = create_file_metadata(&hash2, "archive/file2.log", "file2.log", 200, 2);
        metadata.insert_file(&file_meta2).await.unwrap();

        let content3 = b"depth 5";
        let hash3 = cas.store_content(content3).await.unwrap();
        let file_meta3 = create_file_metadata(&hash3, "archive/nested/file5.log", "file5.log", 300, 5);
        metadata.insert_file(&file_meta3).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let metrics = collector.collect_metrics().await.unwrap();

        assert_eq!(metrics.max_nesting_depth, 5);
        assert!(metrics.avg_nesting_depth > 0.0);
        assert_eq!(metrics.depth_distribution.len(), 3); // 3 different depths
    }

    #[tokio::test]
    async fn test_depth_distribution() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Add 2 files at depth 0
        for i in 0..2 {
            let content = format!("depth 0 file {}", i);
            let hash = cas.store_content(content.as_bytes()).await.unwrap();
            let file_meta = create_file_metadata(
                &hash,
                &format!("file{}.log", i),
                &format!("file{}.log", i),
                content.len() as i64,
                0,
            );
            metadata.insert_file(&file_meta).await.unwrap();
        }

        // Add 3 files at depth 1
        for i in 0..3 {
            let content = format!("depth 1 file {}", i);
            let hash = cas.store_content(content.as_bytes()).await.unwrap();
            let file_meta = create_file_metadata(
                &hash,
                &format!("archive/file{}.log", i),
                &format!("file{}.log", i),
                content.len() as i64,
                1,
            );
            metadata.insert_file(&file_meta).await.unwrap();
        }

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let metrics = collector.collect_metrics().await.unwrap();

        assert_eq!(metrics.depth_distribution.len(), 2);

        let depth0 = metrics
            .depth_distribution
            .iter()
            .find(|d| d.depth == 0)
            .unwrap();
        assert_eq!(depth0.file_count, 2);

        let depth1 = metrics
            .depth_distribution
            .iter()
            .find(|d| d.depth == 1)
            .unwrap();
        assert_eq!(depth1.file_count, 3);
    }

    #[tokio::test]
    async fn test_quick_summary() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        // Add some files
        let content = b"test";
        let hash = cas.store_content(content).await.unwrap();
        let file_meta = create_file_metadata(&hash, "file.log", "file.log", 100, 2);
        metadata.insert_file(&file_meta).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let (files, archives, max_depth) = collector.get_quick_summary().await.unwrap();

        assert_eq!(files, 1);
        assert_eq!(archives, 0);
        assert_eq!(max_depth, 2);
    }

    #[tokio::test]
    async fn test_get_deduplication_ratio() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content1 = b"duplicate 1";
        let hash1 = cas.store_content(content1).await.unwrap();
        let file_meta1 = create_file_metadata(&hash1, "file1.log", "file1.log", 100, 0);
        metadata.insert_file(&file_meta1).await.unwrap();
        
        let content2 = b"duplicate 2";
        let hash2 = cas.store_content(content2).await.unwrap();
        let file_meta2 = create_file_metadata(&hash2, "file2.log", "file2.log", 100, 0);
        metadata.insert_file(&file_meta2).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let ratio = collector.get_deduplication_ratio().await.unwrap();

        assert!(ratio >= 0.0);
        assert!(ratio <= 1.0);
    }

    #[tokio::test]
    async fn test_get_storage_efficiency() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content = b"test content";
        let hash = cas.store_content(content).await.unwrap();
        let file_meta = create_file_metadata(&hash, "file.log", "file.log", content.len() as i64, 0);
        metadata.insert_file(&file_meta).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let efficiency = collector.get_storage_efficiency().await.unwrap();

        assert!(efficiency > 0.0);
        assert!(efficiency <= 2.0); // Should be close to 1.0
    }

    #[tokio::test]
    async fn test_get_max_nesting_depth() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");

        let metadata = MetadataStore::new(&workspace_dir).await.unwrap();
        let cas = ContentAddressableStorage::new(workspace_dir.clone());

        let content = b"nested";
        let hash = cas.store_content(content).await.unwrap();
        let file_meta = create_file_metadata(&hash, "deep/file.log", "file.log", 100, 10);
        metadata.insert_file(&file_meta).await.unwrap();

        let collector = WorkspaceMetricsCollector::new(metadata, cas);
        let max_depth = collector.get_max_nesting_depth().await.unwrap();

        assert_eq!(max_depth, 10);
    }
}
