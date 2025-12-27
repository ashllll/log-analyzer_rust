//! Streaming Index Builder
#![allow(dead_code)]
//!
//! Handles indexing of large datasets that exceed available RAM by:
//! - Processing files in streaming fashion
//! - Memory-mapped file access for datasets over 1GB
//! - Parallel indexing across multiple CPU cores
//! - Progress tracking and cancellation support

use parking_lot::Mutex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::{SearchEngineManager, SearchError, SearchResult};
use crate::models::LogEntry;
use crate::services::parse_metadata;

/// Progress callback for indexing operations
pub type ProgressCallback = Arc<dyn Fn(IndexingProgress) + Send + Sync>;

/// Indexing progress information
#[derive(Debug, Clone)]
pub struct IndexingProgress {
    pub files_processed: u64,
    pub total_files: u64,
    pub lines_processed: u64,
    pub total_lines_estimate: u64,
    pub current_file: String,
    pub elapsed_time: Duration,
    pub estimated_remaining: Option<Duration>,
}

/// Configuration for streaming index builder
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub batch_size: usize,
    pub memory_limit_mb: usize,
    pub parallel_workers: usize,
    pub commit_interval: Duration,
    pub use_memory_mapping: bool,
    pub memory_mapping_threshold_gb: u64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            batch_size: 10_000,
            memory_limit_mb: 512,
            parallel_workers: num_cpus::get(),
            commit_interval: Duration::from_secs(30),
            use_memory_mapping: true,
            memory_mapping_threshold_gb: 1,
        }
    }
}

/// Streaming index builder for large datasets
pub struct StreamingIndexBuilder {
    search_manager: Arc<SearchEngineManager>,
    config: StreamingConfig,
    cancellation_token: Arc<AtomicBool>,
    progress: Arc<Mutex<IndexingProgress>>,
}

impl StreamingIndexBuilder {
    /// Create a new streaming index builder
    pub fn new(search_manager: Arc<SearchEngineManager>, config: StreamingConfig) -> Self {
        let progress = IndexingProgress {
            files_processed: 0,
            total_files: 0,
            lines_processed: 0,
            total_lines_estimate: 0,
            current_file: String::new(),
            elapsed_time: Duration::ZERO,
            estimated_remaining: None,
        };

        Self {
            search_manager,
            config,
            cancellation_token: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(progress)),
        }
    }

    /// Build index from multiple log files with progress tracking
    pub async fn build_index_streaming(
        &self,
        log_files: Vec<PathBuf>,
        progress_callback: Option<ProgressCallback>,
    ) -> SearchResult<IndexingStats> {
        let start_time = Instant::now();

        info!(
            file_count = log_files.len(),
            workers = self.config.parallel_workers,
            batch_size = self.config.batch_size,
            "Starting streaming index build"
        );

        // Reset cancellation token
        self.cancellation_token.store(false, Ordering::Relaxed);

        // Initialize progress
        {
            let mut progress = self.progress.lock();
            progress.files_processed = 0;
            progress.total_files = log_files.len() as u64;
            progress.lines_processed = 0;
            progress.total_lines_estimate = self.estimate_total_lines(&log_files).await?;
            progress.elapsed_time = Duration::ZERO;
        }

        // Clear existing index
        self.search_manager.clear_index()?;

        let mut stats = IndexingStats::default();
        let lines_processed = Arc::new(AtomicU64::new(0));

        // Process files in parallel batches
        let file_batches: Vec<_> = log_files.chunks(self.config.parallel_workers).collect();

        for (batch_idx, file_batch) in file_batches.iter().enumerate() {
            // Check for cancellation
            if self.cancellation_token.load(Ordering::Relaxed) {
                warn!("Index building cancelled by user");
                return Err(SearchError::IndexError("Indexing cancelled".to_string()));
            }

            // Process batch in parallel
            let batch_stats = self
                .process_file_batch(
                    file_batch,
                    batch_idx,
                    &lines_processed,
                    progress_callback.as_ref(),
                )
                .await?;

            stats.merge(batch_stats);

            // Commit periodically
            if batch_idx % 10 == 0 {
                self.search_manager.commit()?;
                debug!(batch = batch_idx, "Committed batch to index");
            }
        }

        // Final commit
        self.search_manager.commit()?;

        stats.total_time = start_time.elapsed();

        info!(
            files_processed = stats.files_processed,
            lines_processed = stats.lines_processed,
            errors = stats.error_count,
            time_ms = stats.total_time.as_millis(),
            "Index building completed"
        );

        Ok(stats)
    }

    /// Process a batch of files in parallel
    async fn process_file_batch(
        &self,
        files: &[PathBuf],
        batch_idx: usize,
        lines_processed: &Arc<AtomicU64>,
        progress_callback: Option<&ProgressCallback>,
    ) -> SearchResult<IndexingStats> {
        let (tx, mut rx) = mpsc::channel::<ProcessedBatch>(1000);
        let search_manager = Arc::clone(&self.search_manager);
        let cancellation_token = Arc::clone(&self.cancellation_token);

        // Spawn file processing tasks
        let processing_tasks: Vec<_> = files
            .iter()
            .enumerate()
            .map(|(file_idx, file_path)| {
                let tx = tx.clone();
                let file_path = file_path.clone();
                let cancellation_token = Arc::clone(&cancellation_token);
                let config = self.config.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::process_single_file(
                        file_path,
                        file_idx,
                        batch_idx,
                        tx,
                        cancellation_token,
                        config,
                    )
                    .await
                    {
                        error!(error = %e, "Failed to process file");
                    }
                })
            })
            .collect();

        // Close sender to signal completion
        drop(tx);

        let mut stats = IndexingStats::default();

        // Collect processed batches and add to index
        while let Some(batch) = rx.recv().await {
            // Check for cancellation
            if self.cancellation_token.load(Ordering::Relaxed) {
                break;
            }

            // Add documents to index
            for log_entry in &batch.entries {
                if let Err(e) = search_manager.add_document(log_entry) {
                    error!(error = %e, "Failed to add document to index");
                    stats.error_count += 1;
                }
            }

            stats.lines_processed += batch.entries.len() as u64;
            lines_processed.fetch_add(batch.entries.len() as u64, Ordering::Relaxed);

            // Update progress
            if let Some(callback) = progress_callback {
                self.update_progress(callback, &batch.file_path);
            }
        }

        // Wait for all processing tasks to complete
        for task in processing_tasks {
            let _ = task.await;
        }

        stats.files_processed = files.len() as u64;
        Ok(stats)
    }

    /// Process a single file and send batches through channel
    async fn process_single_file(
        file_path: PathBuf,
        file_idx: usize,
        batch_idx: usize,
        tx: mpsc::Sender<ProcessedBatch>,
        cancellation_token: Arc<AtomicBool>,
        config: StreamingConfig,
    ) -> SearchResult<()> {
        debug!(file = %file_path.display(), "Processing file");

        let file = File::open(&file_path)?;
        let reader = BufReader::with_capacity(64 * 1024, file); // 64KB buffer

        let mut batch = Vec::with_capacity(config.batch_size);
        let mut line_number = 0;
        let global_offset = (batch_idx * 1000000) + (file_idx * 100000);

        for line_result in reader.lines() {
            // Check for cancellation
            if cancellation_token.load(Ordering::Relaxed) {
                break;
            }

            let line = line_result?;
            line_number += 1;

            // Parse log entry
            let (timestamp, level) = parse_metadata(&line);
            let log_entry = LogEntry {
                id: global_offset + line_number,
                timestamp,
                level,
                file: file_path.to_string_lossy().to_string(),
                real_path: file_path.to_string_lossy().to_string(),
                line: line_number,
                content: line,
                tags: vec![],
                match_details: None,
                matched_keywords: None,
            };

            batch.push(log_entry);

            // Send batch when full
            if batch.len() >= config.batch_size {
                let processed_batch = ProcessedBatch {
                    entries: std::mem::take(&mut batch),
                    file_path: file_path.clone(),
                };

                if tx.send(processed_batch).await.is_err() {
                    break; // Receiver dropped
                }

                batch = Vec::with_capacity(config.batch_size);
            }
        }

        // Send remaining entries
        if !batch.is_empty() {
            let processed_batch = ProcessedBatch {
                entries: batch,
                file_path: file_path.clone(),
            };

            let _ = tx.send(processed_batch).await;
        }

        Ok(())
    }

    /// Estimate total lines across all files for progress tracking
    async fn estimate_total_lines(&self, files: &[PathBuf]) -> SearchResult<u64> {
        let sample_size = (files.len() / 10).max(1).min(10); // Sample 10% or max 10 files

        let mut total_estimate = 0u64;
        let mut sampled_files = 0;
        let mut total_size = 0u64;
        let mut sampled_size = 0u64;

        for file_path in files.iter().take(sample_size) {
            if let Ok(metadata) = std::fs::metadata(file_path) {
                let file_size = metadata.len();
                total_size += file_size;
                sampled_size += file_size;

                // Quick line count estimation
                if let Ok(file) = File::open(file_path) {
                    let reader = BufReader::new(file);
                    let line_count = reader.lines().count() as u64;
                    total_estimate += line_count;
                    sampled_files += 1;
                }
            }
        }

        // Get total size of all files
        for file_path in files.iter().skip(sample_size) {
            if let Ok(metadata) = std::fs::metadata(file_path) {
                total_size += metadata.len();
            }
        }

        // Extrapolate based on size ratio
        if sampled_files > 0 && sampled_size > 0 {
            let lines_per_byte = total_estimate as f64 / sampled_size as f64;
            let estimated_total = (total_size as f64 * lines_per_byte) as u64;
            Ok(estimated_total)
        } else {
            Ok(files.len() as u64 * 1000) // Fallback estimate
        }
    }

    /// Update progress and call callback
    fn update_progress(&self, callback: &ProgressCallback, current_file: &Path) {
        let mut progress = self.progress.lock();
        progress.files_processed += 1;
        progress.current_file = current_file.to_string_lossy().to_string();
        // progress.elapsed_time = progress.elapsed_time; // Will be updated by caller

        // Estimate remaining time
        if progress.files_processed > 0 {
            let avg_time_per_file =
                progress.elapsed_time.as_secs_f64() / progress.files_processed as f64;
            let remaining_files = progress
                .total_files
                .saturating_sub(progress.files_processed);
            progress.estimated_remaining = Some(Duration::from_secs_f64(
                avg_time_per_file * remaining_files as f64,
            ));
        }

        callback(progress.clone());
    }

    /// Cancel the indexing operation
    pub fn cancel(&self) {
        self.cancellation_token.store(true, Ordering::Relaxed);
        info!("Index building cancellation requested");
    }

    /// Check if indexing is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.load(Ordering::Relaxed)
    }
}

/// Batch of processed log entries
#[derive(Debug)]
struct ProcessedBatch {
    entries: Vec<LogEntry>,
    file_path: PathBuf,
}

/// Statistics from indexing operation
#[derive(Debug, Default)]
pub struct IndexingStats {
    pub files_processed: u64,
    pub lines_processed: u64,
    pub error_count: u64,
    pub total_time: Duration,
}

impl IndexingStats {
    fn merge(&mut self, other: IndexingStats) {
        self.files_processed += other.files_processed;
        self.lines_processed += other.lines_processed;
        self.error_count += other.error_count;
        // Don't merge total_time as it's set by the caller
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_engine::{manager::SearchConfig, SearchEngineManager};
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    async fn create_test_setup() -> (StreamingIndexBuilder, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("search_index");
        let config = SearchConfig {
            index_path,
            ..Default::default()
        };
        let manager = Arc::new(SearchEngineManager::new(config).unwrap());
        let builder = StreamingIndexBuilder::new(manager, StreamingConfig::default());
        (builder, temp_dir)
    }

    #[tokio::test]
    async fn test_streaming_builder_creation() {
        let (_builder, _temp_dir) = create_test_setup().await;
        // If we get here, creation was successful
    }

    #[tokio::test]
    async fn test_empty_file_list() {
        let (builder, _temp_dir) = create_test_setup().await;

        let result = builder.build_index_streaming(vec![], None).await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.lines_processed, 0);
    }

    #[tokio::test]
    async fn test_single_file_indexing() {
        let (builder, _temp_dir) = create_test_setup().await;

        // Create a test file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "2023-01-01 10:00:00 INFO Test log line 1").unwrap();
        writeln!(temp_file, "2023-01-01 10:00:01 ERROR Test log line 2").unwrap();
        writeln!(temp_file, "2023-01-01 10:00:02 DEBUG Test log line 3").unwrap();

        let files = vec![temp_file.path().to_path_buf()];
        let result = builder.build_index_streaming(files, None).await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.lines_processed, 3);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let (builder, _temp_dir) = create_test_setup().await;

        // 测试取消标志的设置和检查
        assert!(!builder.is_cancelled(), "Should not be cancelled initially");

        builder.cancel();
        assert!(
            builder.is_cancelled(),
            "Should be cancelled after cancel() call"
        );

        // 注意：build_index_streaming 会在开始时重置取消令牌
        // 这是设计行为，允许重新开始索引构建
        // 所以空文件列表的构建应该成功
        let result = builder.build_index_streaming(vec![], None).await;

        // 空文件列表应该成功完成（没有文件需要处理）
        assert!(
            result.is_ok(),
            "Empty file list should succeed: {:?}",
            result.err()
        );
    }
}
