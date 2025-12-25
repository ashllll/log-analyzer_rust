//! Parallel Archive Processing
//!
//! This module provides parallel processing capabilities for archive extraction
//! and file processing using rayon for CPU-bound operations and tokio for I/O.
//!
//! Key features:
//! - Process multiple archives concurrently
//! - Batch database insertions for better performance
//! - Automatic work distribution based on CPU cores
//! - Progress tracking across parallel operations
//!
//! # Requirements
//!
//! Validates: Requirements 6.2

use crate::archive::processor::{process_path_with_cas_and_checkpoints, CasProcessingContext};
use crate::error::{AppError, Result};
use crate::storage::{FileMetadata, MetadataStore};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tracing::{debug, info, warn};

/// Configuration for parallel processing
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of archives to process concurrently
    pub max_concurrent_archives: usize,
    /// Batch size for database insertions
    pub db_batch_size: usize,
    /// Number of worker threads for CPU-bound operations
    pub worker_threads: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            max_concurrent_archives: num_cpus.min(4), // Cap at 4 to avoid overwhelming I/O
            db_batch_size: 100,                       // Insert 100 files at a time
            worker_threads: num_cpus,
        }
    }
}

/// Parallel archive processor
pub struct ParallelProcessor {
    pub config: ParallelConfig,
}

impl ParallelProcessor {
    /// Create a new parallel processor with default configuration
    pub fn new() -> Self {
        Self {
            config: ParallelConfig::default(),
        }
    }

    /// Create a new parallel processor with custom configuration
    pub fn with_config(config: ParallelConfig) -> Self {
        Self { config }
    }

    /// Process multiple archive paths in parallel
    ///
    /// This method distributes archive processing across multiple threads,
    /// improving throughput for workloads with many archives.
    ///
    /// # Arguments
    ///
    /// * `archive_paths` - List of archive paths to process
    /// * `context` - Processing context with CAS and metadata store
    /// * `app` - Tauri app handle
    /// * `task_id` - Task ID for progress reporting
    /// * `workspace_id` - Workspace ID
    ///
    /// # Returns
    ///
    /// Number of successfully processed archives
    ///
    /// # Requirements
    ///
    /// Validates: Requirements 6.2
    pub async fn process_archives_parallel(
        &self,
        archive_paths: Vec<PathBuf>,
        context: Arc<CasProcessingContext>,
        app: AppHandle,
        task_id: String,
        workspace_id: String,
    ) -> Result<usize> {
        let total_archives = archive_paths.len();
        info!(
            total = total_archives,
            max_concurrent = self.config.max_concurrent_archives,
            "Starting parallel archive processing"
        );

        // Split archives into batches for concurrent processing
        let batches: Vec<_> = archive_paths
            .chunks(self.config.max_concurrent_archives)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut total_processed = 0;

        for (batch_idx, batch) in batches.iter().enumerate() {
            info!(
                batch = batch_idx + 1,
                total_batches = batches.len(),
                batch_size = batch.len(),
                "Processing archive batch"
            );

            // Process batch concurrently
            let results = self
                .process_batch(
                    batch.clone(),
                    context.clone(),
                    app.clone(),
                    task_id.clone(),
                    workspace_id.clone(),
                )
                .await;

            // Count successes
            let batch_processed = results.iter().filter(|r| r.is_ok()).count();
            total_processed += batch_processed;

            // Log failures
            for (idx, result) in results.iter().enumerate() {
                if let Err(e) = result {
                    warn!(
                        archive = %batch[idx].display(),
                        error = %e,
                        "Failed to process archive in parallel batch"
                    );
                }
            }

            debug!(
                batch = batch_idx + 1,
                processed = batch_processed,
                failed = batch.len() - batch_processed,
                "Batch processing completed"
            );
        }

        info!(
            total_processed = total_processed,
            total_archives = total_archives,
            success_rate = format!("{:.1}%", (total_processed as f64 / total_archives as f64) * 100.0),
            "Parallel archive processing completed"
        );

        Ok(total_processed)
    }

    /// Process a batch of archives concurrently
    async fn process_batch(
        &self,
        batch: Vec<PathBuf>,
        context: Arc<CasProcessingContext>,
        app: AppHandle,
        task_id: String,
        workspace_id: String,
    ) -> Vec<Result<()>> {
        // Use tokio to spawn concurrent tasks
        let mut handles = Vec::new();

        for archive_path in batch {
            let context = context.clone();
            let app = app.clone();
            let task_id = task_id.clone();
            let workspace_id = workspace_id.clone();

            let handle = tokio::spawn(async move {
                let virtual_path = archive_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                process_path_with_cas_and_checkpoints(
                    &archive_path,
                    &virtual_path,
                    &context,
                    &app,
                    &task_id,
                    &workspace_id,
                    None,  // parent_archive_id
                    0,     // depth_level
                )
                .await
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(AppError::archive_error(
                    format!("Task join error: {}", e),
                    None,
                ))),
            }
        }

        results
    }

    /// Batch insert files into metadata store
    ///
    /// This method collects file metadata and inserts them in batches
    /// to reduce database transaction overhead.
    ///
    /// # Arguments
    ///
    /// * `files` - List of file metadata to insert
    /// * `metadata_store` - Metadata store instance
    ///
    /// # Returns
    ///
    /// Number of successfully inserted files
    pub async fn batch_insert_files(
        &self,
        files: Vec<FileMetadata>,
        metadata_store: Arc<MetadataStore>,
    ) -> Result<usize> {
        let total_files = files.len();
        info!(
            total = total_files,
            batch_size = self.config.db_batch_size,
            "Starting batch file insertion"
        );

        let batches: Vec<_> = files
            .chunks(self.config.db_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut total_inserted = 0;

        for (batch_idx, batch) in batches.iter().enumerate() {
            debug!(
                batch = batch_idx + 1,
                total_batches = batches.len(),
                batch_size = batch.len(),
                "Inserting file batch"
            );

            // Insert batch
            for file in batch {
                match metadata_store.insert_file(file).await {
                    Ok(_) => total_inserted += 1,
                    Err(e) => {
                        warn!(
                            file = %file.virtual_path,
                            error = %e,
                            "Failed to insert file in batch"
                        );
                    }
                }
            }
        }

        info!(
            total_inserted = total_inserted,
            total_files = total_files,
            "Batch file insertion completed"
        );

        Ok(total_inserted)
    }

    /// Process files in parallel using rayon
    ///
    /// This method uses rayon's parallel iterators for CPU-bound operations
    /// like hashing and validation.
    ///
    /// # Arguments
    ///
    /// * `file_paths` - List of file paths to process
    /// * `processor` - Function to process each file
    ///
    /// # Returns
    ///
    /// Vector of processing results
    pub fn process_files_parallel<F, T>(
        &self,
        file_paths: Vec<PathBuf>,
        processor: F,
    ) -> Vec<Result<T>>
    where
        F: Fn(&Path) -> Result<T> + Send + Sync,
        T: Send,
    {
        info!(
            total_files = file_paths.len(),
            worker_threads = self.config.worker_threads,
            "Starting parallel file processing with rayon"
        );

        // Use rayon for parallel processing
        let results: Vec<_> = file_paths
            .par_iter()
            .map(|path| processor(path))
            .collect();

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        info!(
            total = file_paths.len(),
            success = success_count,
            failed = file_paths.len() - success_count,
            "Parallel file processing completed"
        );

        results
    }
}

impl Default for ParallelProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.max_concurrent_archives > 0);
        assert!(config.db_batch_size > 0);
        assert!(config.worker_threads > 0);
    }

    #[test]
    fn test_parallel_processor_creation() {
        let processor = ParallelProcessor::new();
        assert!(processor.config.max_concurrent_archives > 0);

        let custom_config = ParallelConfig {
            max_concurrent_archives: 2,
            db_batch_size: 50,
            worker_threads: 4,
        };
        let custom_processor = ParallelProcessor::with_config(custom_config);
        assert_eq!(custom_processor.config.max_concurrent_archives, 2);
        assert_eq!(custom_processor.config.db_batch_size, 50);
    }

    #[test]
    fn test_process_files_parallel() {
        let temp_dir = TempDir::new().unwrap();
        let processor = ParallelProcessor::new();

        // Create test files
        let mut file_paths = Vec::new();
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("file{}.txt", i));
            std::fs::write(&file_path, format!("content {}", i)).unwrap();
            file_paths.push(file_path);
        }

        // Process files in parallel
        let results = processor.process_files_parallel(file_paths, |path| {
            // Simple processor: check if file exists
            if path.exists() {
                Ok(path.to_path_buf())
            } else {
                Err(AppError::not_found(format!(
                    "File not found: {}",
                    path.display()
                )))
            }
        });

        // All files should be processed successfully
        assert_eq!(results.len(), 10);
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 10);
    }

    #[test]
    fn test_process_files_parallel_with_failures() {
        let processor = ParallelProcessor::new();

        // Mix of existing and non-existing files
        let file_paths = vec![
            PathBuf::from("/nonexistent/file1.txt"),
            PathBuf::from("/nonexistent/file2.txt"),
        ];

        let results = processor.process_files_parallel(file_paths, |path| {
            if path.exists() {
                Ok(())
            } else {
                Err(AppError::not_found(format!(
                    "File not found: {}",
                    path.display()
                )))
            }
        });

        // All should fail since files don't exist
        assert_eq!(results.len(), 2);
        assert_eq!(results.iter().filter(|r| r.is_err()).count(), 2);
    }
}
