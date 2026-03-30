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

use crate::{SearchEngineManager, SearchError, SearchResult};
use la_core::models::LogEntry;
use once_cell::sync::Lazy;
use regex::Regex;

/// 从日志行中提取时间戳和日志级别
///
/// 返回元组：(时间戳, 日志级别)
///
/// # 提取规则
///
/// - **时间戳**：查找常见时间戳格式（ISO 8601、Unix 时间戳等）
/// - **日志级别**：按优先级匹配 ERROR > WARN > INFO > DEBUG（默认）
fn parse_metadata(line: &str) -> (String, String) {
    static LOG_LEVEL_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\b(ERROR|WARN|INFO|DEBUG)\b").unwrap());

    let level = LOG_LEVEL_REGEX
        .find(line)
        .map(|m| m.as_str())
        .unwrap_or("DEBUG");

    // 简单时间戳提取：尝试匹配常见格式
    static TIMESTAMP_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:\d{4}[-/]\d{2}[-/]\d{2}[\sT]\d{2}:\d{2}:\d{2}(?:\.\d+)?|\d{10,13})")
            .unwrap()
    });

    let timestamp = TIMESTAMP_REGEX
        .find(line)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    (timestamp, level.to_string())
}

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

        // 不在此处重置取消令牌：若调用方在调用前已发出取消信号，重置会丢失该信号。
        // 调用方应在调用前通过 new() 或手动重置令牌来确保令牌处于未取消状态。

        // Estimate total lines first (without holding lock)
        let total_lines_estimate = self.estimate_total_lines(&log_files).await?;

        // Initialize progress
        {
            let mut progress = self.progress.lock();
            progress.files_processed = 0;
            progress.total_files = log_files.len() as u64;
            progress.lines_processed = 0;
            progress.total_lines_estimate = total_lines_estimate;
            progress.elapsed_time = Duration::ZERO;
        }

        // Clear existing index（spawn_blocking 隔离同步 Mutex，避免阻塞 tokio worker）
        // 检查取消令牌
        if self.cancellation_token.load(Ordering::Relaxed) {
            warn!("Index building cancelled before clearing index");
            return Err(SearchError::IndexError("Indexing cancelled".to_string()));
        }

        let cancellation_token = Arc::clone(&self.cancellation_token);
        {
            let m = Arc::clone(&self.search_manager);
            tokio::task::spawn_blocking(move || {
                // 定期检查取消令牌
                if cancellation_token.load(Ordering::Relaxed) {
                    return Err(SearchError::IndexError("Indexing cancelled".to_string()));
                }
                m.clear_index()
            })
            .await
            .map_err(|e| SearchError::IndexError(format!("spawn_blocking panicked: {e}")))??;
        }

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

            // Commit periodically（spawn_blocking 隔离同步 Mutex + 磁盘 IO，避免阻塞 tokio worker）
            if batch_idx % 10 == 0 {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    warn!("Index building cancelled before commit");
                    return Err(SearchError::IndexError("Indexing cancelled".to_string()));
                }
                let cancellation_token = Arc::clone(&self.cancellation_token);
                let m = Arc::clone(&self.search_manager);
                tokio::task::spawn_blocking(move || {
                    if cancellation_token.load(Ordering::Relaxed) {
                        return Err(SearchError::IndexError("Indexing cancelled".to_string()));
                    }
                    m.commit()
                })
                .await
                .map_err(|e| SearchError::IndexError(format!("spawn_blocking panicked: {e}")))??;
                debug!(batch = batch_idx, "Committed batch to index");
            }
        }

        // Final commit（spawn_blocking 隔离同步 Mutex + 磁盘 IO，避免阻塞 tokio worker）
        if self.cancellation_token.load(Ordering::Relaxed) {
            warn!("Index building cancelled before final commit");
            return Err(SearchError::IndexError("Indexing cancelled".to_string()));
        }

        let cancellation_token = Arc::clone(&self.cancellation_token);
        {
            let m = Arc::clone(&self.search_manager);
            tokio::task::spawn_blocking(move || {
                if cancellation_token.load(Ordering::Relaxed) {
                    return Err(SearchError::IndexError("Indexing cancelled".to_string()));
                }
                m.commit()
            })
            .await
            .map_err(|e| SearchError::IndexError(format!("spawn_blocking panicked: {e}")))??;
        }

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

            // 将整批次 add_document 移入 spawn_blocking，避免在 async 循环中逐条持有 Mutex
            let entries_len = batch.entries.len() as u64;
            let batch_file_path = batch.file_path.clone();
            let entries = batch.entries;
            let manager_for_batch = Arc::clone(&search_manager);

            let batch_errors = tokio::task::spawn_blocking(move || -> u64 {
                let mut errors = 0u64;
                for entry in &entries {
                    if let Err(e) = manager_for_batch.add_document(entry) {
                        error!(error = %e, "Failed to add document to index");
                        errors += 1;
                    }
                }
                errors
            })
            .await
            .map_err(|e| SearchError::IndexError(format!("spawn_blocking panicked: {e}")))?;

            stats.error_count += batch_errors;
            stats.lines_processed += entries_len;
            lines_processed.fetch_add(entries_len, Ordering::Relaxed);

            // Update progress
            if let Some(callback) = progress_callback {
                self.update_progress(callback, &batch_file_path);
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
        let global_offset = batch_idx
            .saturating_mul(1000000)
            .saturating_add(file_idx.saturating_mul(100000));

        // 预构建文件路径 Arc<str>，避免每行重复 to_string_lossy()
        let file_path_str: Arc<str> = file_path.to_string_lossy().into();

        for line_result in reader.lines() {
            // Check for cancellation
            if cancellation_token.load(Ordering::Relaxed) {
                break;
            }

            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    error!(file = %file_path.display(), error = %e, "Error reading line from file");
                    break; // Stop processing this file on IO error
                }
            };
            line_number += 1;

            // Parse log entry
            let (timestamp, level) = parse_metadata(&line);
            let log_entry = LogEntry {
                id: global_offset + line_number,
                timestamp: timestamp.into(),
                level: level.into(),
                file: Arc::clone(&file_path_str),
                real_path: Arc::clone(&file_path_str),
                line: line_number,
                content: line.into(),
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

            if tx.send(processed_batch).await.is_err() {
                tracing::debug!("流式构建器：最后一批数据发送时接收端已关闭");
            }
        }

        Ok(())
    }

    /// Estimate total lines across all files for progress tracking
    async fn estimate_total_lines(&self, files: &[PathBuf]) -> SearchResult<u64> {
        let sample_size = (files.len() / 10).clamp(1, 10); // Sample 10% or max 10 files

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
    use crate::manager::SearchConfig;
    use crate::SearchEngineManager;
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

        // 修复后 build_index_streaming 不再重置取消令牌。
        // 已处于取消状态时，函数应在 clear_index 阶段检测到取消信号并提前返回错误。
        let result = builder.build_index_streaming(vec![], None).await;

        // 已取消状态下，即使文件列表为空，也应该返回取消错误
        assert!(
            result.is_err(),
            "Should return error when cancelled before build"
        );
    }
}
