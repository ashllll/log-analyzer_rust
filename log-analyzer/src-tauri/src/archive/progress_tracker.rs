/**
 * Progress tracking module for archive extraction operations
 *
 * Provides hierarchical progress reporting with thread-safe metrics collection
 */
use crate::archive::extraction_context::ExtractionContext;
use crate::error::{AppError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Error categories for extraction operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    PathTooLong,
    UnsupportedFormat,
    CorruptedArchive,
    PermissionDenied,
    ZipBombDetected,
    DepthLimitExceeded,
    DiskSpaceExhausted,
    CancellationRequested,
    IoError,
    Other,
}

impl ErrorCategory {
    /// Convert from error to category
    pub fn from_error(error: &AppError) -> Self {
        match error {
            AppError::InvalidPath(_) => ErrorCategory::PathTooLong,
            AppError::Archive { _message, .. } => {
                if _message.contains("unsupported") || _message.contains("Unsupported") {
                    ErrorCategory::UnsupportedFormat
                } else if _message.contains("corrupted") || _message.contains("Corrupted") {
                    ErrorCategory::CorruptedArchive
                } else if _message.contains("zip bomb") || _message.contains("Zip bomb") {
                    ErrorCategory::ZipBombDetected
                } else if _message.contains("depth") || _message.contains("Depth") {
                    ErrorCategory::DepthLimitExceeded
                } else if _message.contains("space") || _message.contains("Space") {
                    ErrorCategory::DiskSpaceExhausted
                } else {
                    ErrorCategory::Other
                }
            }
            AppError::Io(io_err) => match io_err.kind() {
                std::io::ErrorKind::PermissionDenied => ErrorCategory::PermissionDenied,
                _ => ErrorCategory::IoError,
            },
            _ => ErrorCategory::Other,
        }
    }
}

/// Progress event emitted during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub workspace_id: String,
    pub current_file: String,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub current_depth: usize,
    pub estimated_remaining_time: Option<Duration>,
    pub hierarchical_path: Vec<String>,
}

/// Thread-safe progress metrics
#[derive(Debug)]
pub struct ProgressMetrics {
    pub files_processed: AtomicUsize,
    pub bytes_processed: AtomicU64,
    pub current_depth: AtomicUsize,
    pub max_depth_reached: AtomicUsize,
    pub errors_by_category: DashMap<ErrorCategory, usize>,
    pub path_shortenings_applied: AtomicUsize,
    pub suspicious_files_detected: AtomicUsize,
}

impl ProgressMetrics {
    pub fn new() -> Self {
        Self {
            files_processed: AtomicUsize::new(0),
            bytes_processed: AtomicU64::new(0),
            current_depth: AtomicUsize::new(0),
            max_depth_reached: AtomicUsize::new(0),
            errors_by_category: DashMap::new(),
            path_shortenings_applied: AtomicUsize::new(0),
            suspicious_files_detected: AtomicUsize::new(0),
        }
    }

    /// Increment files processed counter
    pub fn increment_files(&self) -> usize {
        self.files_processed.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Add bytes processed
    pub fn add_bytes(&self, bytes: u64) -> u64 {
        self.bytes_processed.fetch_add(bytes, Ordering::SeqCst) + bytes
    }

    /// Update current depth
    pub fn set_current_depth(&self, depth: usize) {
        self.current_depth.store(depth, Ordering::SeqCst);

        // Update max depth if needed
        let mut max = self.max_depth_reached.load(Ordering::SeqCst);
        while depth > max {
            match self.max_depth_reached.compare_exchange(
                max,
                depth,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(current) => max = current,
            }
        }
    }

    /// Increment path shortening counter
    pub fn increment_path_shortenings(&self) -> usize {
        self.path_shortenings_applied.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Increment suspicious files counter
    pub fn increment_suspicious_files(&self) -> usize {
        self.suspicious_files_detected
            .fetch_add(1, Ordering::SeqCst)
            + 1
    }

    /// Record an error by category
    pub fn record_error_category(&self, category: ErrorCategory) {
        self.errors_by_category
            .entry(category)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Get current files processed count
    pub fn get_files_processed(&self) -> usize {
        self.files_processed.load(Ordering::SeqCst)
    }

    /// Get current bytes processed count
    pub fn get_bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::SeqCst)
    }

    /// Get current depth
    pub fn get_current_depth(&self) -> usize {
        self.current_depth.load(Ordering::SeqCst)
    }

    /// Get max depth reached
    pub fn get_max_depth_reached(&self) -> usize {
        self.max_depth_reached.load(Ordering::SeqCst)
    }

    /// Get path shortenings applied
    pub fn get_path_shortenings(&self) -> usize {
        self.path_shortenings_applied.load(Ordering::SeqCst)
    }

    /// Get suspicious files detected
    pub fn get_suspicious_files(&self) -> usize {
        self.suspicious_files_detected.load(Ordering::SeqCst)
    }
}

impl Default for ProgressMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Extraction summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub total_files: usize,
    pub total_bytes: u64,
    pub max_depth_reached: usize,
    pub errors_by_category: Vec<(ErrorCategory, usize)>,
    pub path_shortenings_applied: usize,
    pub suspicious_files_detected: usize,
    pub duration: Duration,
}

/// Progress tracker for extraction operations
pub struct ProgressTracker {
    event_sender: broadcast::Sender<ProgressEvent>,
    metrics: Arc<ProgressMetrics>,
    start_time: Instant,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let (sender, _receiver) = broadcast::channel(1000);

        Self {
            event_sender: sender,
            metrics: Arc::new(ProgressMetrics::new()),
            start_time: Instant::now(),
        }
    }

    /// Subscribe to progress events
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.event_sender.subscribe()
    }

    /// Get metrics reference
    pub fn metrics(&self) -> Arc<ProgressMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Emit progress event
    pub async fn emit_progress(
        &self,
        context: &ExtractionContext,
        current_file: &Path,
    ) -> Result<()> {
        // Build hierarchical path from context
        let hierarchical_path = self.build_hierarchical_path(context);

        // Estimate remaining time
        let estimated_remaining_time = self.estimate_remaining_time(context);

        let event = ProgressEvent {
            workspace_id: context.workspace_id.clone(),
            current_file: current_file.display().to_string(),
            files_processed: self.metrics.get_files_processed(),
            bytes_processed: self.metrics.get_bytes_processed(),
            current_depth: context.current_depth,
            estimated_remaining_time,
            hierarchical_path,
        };

        // Send event (ignore if no subscribers)
        match self.event_sender.send(event) {
            Ok(subscriber_count) => {
                debug!(
                    subscriber_count,
                    file = %current_file.display(),
                    "Progress event emitted"
                );
                Ok(())
            }
            Err(_) => {
                // No subscribers, that's okay
                debug!("No subscribers for progress events");
                Ok(())
            }
        }
    }

    /// Record an error with categorization
    pub fn record_error(&self, error: &AppError) {
        let category = ErrorCategory::from_error(error);
        self.metrics.record_error_category(category);

        warn!(
            category = ?category,
            error = %error,
            "Error recorded during extraction"
        );
    }

    /// Generate final summary report
    pub fn generate_summary(&self) -> ExtractionSummary {
        let errors_by_category: Vec<(ErrorCategory, usize)> = self
            .metrics
            .errors_by_category
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();

        let summary = ExtractionSummary {
            total_files: self.metrics.get_files_processed(),
            total_bytes: self.metrics.get_bytes_processed(),
            max_depth_reached: self.metrics.get_max_depth_reached(),
            errors_by_category,
            path_shortenings_applied: self.metrics.get_path_shortenings(),
            suspicious_files_detected: self.metrics.get_suspicious_files(),
            duration: self.start_time.elapsed(),
        };

        info!(
            total_files = summary.total_files,
            total_bytes = summary.total_bytes,
            max_depth = summary.max_depth_reached,
            duration_secs = summary.duration.as_secs(),
            "Extraction summary generated"
        );

        summary
    }

    /// Build hierarchical path from extraction context
    fn build_hierarchical_path(&self, context: &ExtractionContext) -> Vec<String> {
        let mut path = Vec::new();

        // Add parent archives from context
        if let Some(parent) = &context.parent_archive {
            path.push(parent.display().to_string());
        }

        // Add depth indicator
        if context.current_depth > 0 {
            path.push(format!("depth_{}", context.current_depth));
        }

        path
    }

    /// Estimate remaining time based on current progress rate
    fn estimate_remaining_time(&self, context: &ExtractionContext) -> Option<Duration> {
        let elapsed = self.start_time.elapsed();
        let files_processed = self.metrics.get_files_processed();

        // Need at least some progress to estimate
        if files_processed == 0 || elapsed.as_secs() < 1 {
            return None;
        }

        // Calculate processing rate (files per second)
        let rate = files_processed as f64 / elapsed.as_secs_f64();

        // Estimate based on accumulated files (rough estimate)
        // This is a simple heuristic - in practice, we'd need more context
        // about total expected files
        let estimated_remaining_files = context.accumulated_files.saturating_sub(files_processed);

        if estimated_remaining_files == 0 {
            return Some(Duration::from_secs(0));
        }

        let estimated_seconds = (estimated_remaining_files as f64 / rate).ceil() as u64;
        Some(Duration::from_secs(estimated_seconds))
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_progress_metrics_creation() {
        let metrics = ProgressMetrics::new();
        assert_eq!(metrics.get_files_processed(), 0);
        assert_eq!(metrics.get_bytes_processed(), 0);
        assert_eq!(metrics.get_current_depth(), 0);
        assert_eq!(metrics.get_max_depth_reached(), 0);
    }

    #[test]
    fn test_progress_metrics_increment() {
        let metrics = ProgressMetrics::new();

        let count = metrics.increment_files();
        assert_eq!(count, 1);
        assert_eq!(metrics.get_files_processed(), 1);

        let bytes = metrics.add_bytes(1024);
        assert_eq!(bytes, 1024);
        assert_eq!(metrics.get_bytes_processed(), 1024);
    }

    #[test]
    fn test_progress_metrics_depth_tracking() {
        let metrics = ProgressMetrics::new();

        metrics.set_current_depth(5);
        assert_eq!(metrics.get_current_depth(), 5);
        assert_eq!(metrics.get_max_depth_reached(), 5);

        metrics.set_current_depth(3);
        assert_eq!(metrics.get_current_depth(), 3);
        assert_eq!(metrics.get_max_depth_reached(), 5); // Max should remain

        metrics.set_current_depth(10);
        assert_eq!(metrics.get_max_depth_reached(), 10);
    }

    #[test]
    fn test_error_categorization() {
        let metrics = ProgressMetrics::new();

        metrics.record_error_category(ErrorCategory::PathTooLong);
        metrics.record_error_category(ErrorCategory::PathTooLong);
        metrics.record_error_category(ErrorCategory::ZipBombDetected);

        assert_eq!(
            *metrics
                .errors_by_category
                .get(&ErrorCategory::PathTooLong)
                .unwrap(),
            2
        );
        assert_eq!(
            *metrics
                .errors_by_category
                .get(&ErrorCategory::ZipBombDetected)
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_error_category_from_error() {
        let error = AppError::InvalidPath("path too long".to_string());
        assert_eq!(
            ErrorCategory::from_error(&error),
            ErrorCategory::PathTooLong
        );

        let error = AppError::archive_error("unsupported format", None);
        assert_eq!(
            ErrorCategory::from_error(&error),
            ErrorCategory::UnsupportedFormat
        );

        let error = AppError::archive_error("zip bomb detected", None);
        assert_eq!(
            ErrorCategory::from_error(&error),
            ErrorCategory::ZipBombDetected
        );
    }

    #[tokio::test]
    async fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new();
        assert_eq!(tracker.metrics.get_files_processed(), 0);
    }

    #[tokio::test]
    async fn test_progress_event_emission() {
        let tracker = ProgressTracker::new();
        let mut receiver = tracker.subscribe();

        let context = ExtractionContext {
            workspace_id: "test_workspace".to_string(),
            current_depth: 1,
            parent_archive: Some(PathBuf::from("/test/archive.zip")),
            accumulated_size: 1024,
            accumulated_files: 10,
            start_time: Instant::now(),
        };

        let result = tracker
            .emit_progress(&context, Path::new("/test/file.txt"))
            .await;
        assert!(result.is_ok());

        // Try to receive the event
        let event = tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await;

        assert!(event.is_ok());
        let event = event.unwrap().unwrap();
        assert_eq!(event.workspace_id, "test_workspace");
        assert_eq!(event.current_depth, 1);
        assert!(!event.hierarchical_path.is_empty());
    }

    #[test]
    fn test_summary_generation() {
        let tracker = ProgressTracker::new();

        tracker.metrics.increment_files();
        tracker.metrics.increment_files();
        tracker.metrics.add_bytes(2048);
        tracker.metrics.set_current_depth(3);
        tracker
            .metrics
            .record_error_category(ErrorCategory::PathTooLong);

        let summary = tracker.generate_summary();

        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.total_bytes, 2048);
        assert_eq!(summary.max_depth_reached, 3);
        assert_eq!(summary.errors_by_category.len(), 1);
    }
}
