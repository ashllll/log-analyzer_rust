//! Extraction Orchestrator for Concurrency Control
//!
//! This module implements the orchestration layer that manages concurrent
//! extraction operations, request deduplication, and cancellation support.
//! It wraps the ExtractionEngine and provides controlled access with
//! resource management.

use crate::extraction_engine::{ExtractionEngine, ExtractionResult};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use la_core::error::{AppError, Result};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Notify, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Shared result for request deduplication
type SharedExtractionResult = Arc<SharedExtractionState>;

struct SharedExtractionState {
    result: Mutex<Option<Arc<Result<ExtractionResult>>>>,
    notify: Notify,
}

impl SharedExtractionState {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            notify: Notify::new(),
        }
    }

    fn clone_result(&self) -> Option<Arc<Result<ExtractionResult>>> {
        self.result.lock().clone()
    }

    fn store_result(&self, result: Arc<Result<ExtractionResult>>) {
        *self.result.lock() = Some(result);
        self.notify.notify_waiters();
    }
}

struct InFlightRequestGuard {
    archive_path: PathBuf,
    in_flight_requests: Arc<DashMap<PathBuf, SharedExtractionResult>>,
    shared_result: SharedExtractionResult,
}

impl InFlightRequestGuard {
    fn new(
        archive_path: PathBuf,
        in_flight_requests: Arc<DashMap<PathBuf, SharedExtractionResult>>,
        shared_result: SharedExtractionResult,
    ) -> Self {
        Self {
            archive_path,
            in_flight_requests,
            shared_result,
        }
    }

    fn finish_with(&self, result: Arc<Result<ExtractionResult>>) {
        self.shared_result.store_result(result);
    }
}

impl Drop for InFlightRequestGuard {
    fn drop(&mut self) {
        if self.shared_result.clone_result().is_none() {
            self.shared_result
                .store_result(Arc::new(Err(AppError::archive_error(
                    "Extraction request ended before producing a result",
                    Some(self.archive_path.clone()),
                ))));
        } else {
            self.shared_result.notify.notify_waiters();
        }

        self.in_flight_requests.remove(&self.archive_path);
    }
}

/// Extraction orchestrator for managing concurrent operations
pub struct ExtractionOrchestrator {
    /// Extraction engine for performing actual extraction
    engine: Arc<ExtractionEngine>,
    /// Semaphore for limiting concurrent extractions
    pub(crate) concurrency_limiter: Arc<Semaphore>,
    /// Map for request deduplication (archive_path -> shared result)
    in_flight_requests: Arc<DashMap<PathBuf, SharedExtractionResult>>,
    /// Global cancellation token
    cancellation_token: CancellationToken,
}

impl ExtractionOrchestrator {
    /// Create a new extraction orchestrator
    ///
    /// # Arguments
    ///
    /// * `engine` - Extraction engine to use for operations
    /// * `max_concurrent_extractions` - Maximum number of concurrent extractions (default: CPU cores / 2)
    ///
    /// # Returns
    ///
    /// A new ExtractionOrchestrator instance
    pub fn new(engine: Arc<ExtractionEngine>, max_concurrent_extractions: Option<usize>) -> Self {
        let max_concurrent = max_concurrent_extractions.unwrap_or_else(|| {
            let cpu_count = num_cpus::get();
            (cpu_count / 2).max(1)
        });

        info!(
            "Initializing ExtractionOrchestrator with max_concurrent={}",
            max_concurrent
        );

        Self {
            engine,
            concurrency_limiter: Arc::new(Semaphore::new(max_concurrent)),
            in_flight_requests: Arc::new(DashMap::new()),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Extract an archive with concurrency control and request deduplication
    ///
    /// This method ensures that:
    /// 1. Only a limited number of extractions run concurrently
    /// 2. Multiple requests for the same archive are deduplicated
    /// 3. Cancellation is supported via the orchestrator's cancellation token
    ///
    /// # Arguments
    ///
    /// * `archive_path` - Path to the archive to extract
    /// * `target_dir` - Directory where files should be extracted
    /// * `workspace_id` - Workspace identifier for path mapping
    ///
    /// # Returns
    ///
    /// ExtractionResult containing statistics and warnings
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails or is cancelled
    pub async fn extract_archive(
        &self,
        archive_path: &Path,
        target_dir: &Path,
        workspace_id: &str,
    ) -> Result<ExtractionResult> {
        let archive_path_buf = archive_path.to_path_buf();

        // Check for cancellation before starting
        if self.cancellation_token.is_cancelled() {
            return Err(AppError::validation_error(
                "Extraction cancelled before starting",
            ));
        }

        let (shared_result, is_owner) =
            match self.in_flight_requests.entry(archive_path_buf.clone()) {
                Entry::Occupied(entry) => (entry.get().clone(), false),
                Entry::Vacant(entry) => {
                    let shared_result = Arc::new(SharedExtractionState::new());
                    entry.insert(shared_result.clone());
                    (shared_result, true)
                }
            };

        // Check if there's already an in-flight request for this archive
        if !is_owner {
            debug!(
                "Request deduplication: waiting for existing extraction of {:?}",
                archive_path
            );

            loop {
                if let Some(result) = shared_result.clone_result() {
                    return Self::resolve_shared_result(&result, &archive_path_buf);
                }

                tokio::select! {
                    _ = shared_result.notify.notified() => {}
                    _ = self.cancellation_token.cancelled() => {
                        return Err(AppError::validation_error("Extraction cancelled while waiting for duplicate request"));
                    }
                }
            }
        }

        let in_flight_guard = InFlightRequestGuard::new(
            archive_path_buf.clone(),
            Arc::clone(&self.in_flight_requests),
            shared_result.clone(),
        );

        // Perform the extraction with concurrency control
        let result = self
            .extract_with_concurrency_control(archive_path, target_dir, workspace_id)
            .await;

        in_flight_guard.finish_with(Arc::new(match &result {
            Ok(extraction_result) => Ok(extraction_result.clone()),
            Err(e) => Err(AppError::archive_error(
                format!("{}", e),
                Some(archive_path_buf.clone()),
            )),
        }));

        result
    }

    fn resolve_shared_result(
        shared_result: &Arc<Result<ExtractionResult>>,
        archive_path: &Path,
    ) -> Result<ExtractionResult> {
        match shared_result.as_ref() {
            Ok(extraction_result) => Ok(extraction_result.clone()),
            Err(error) => Err(AppError::archive_error(
                format!("{}", error),
                Some(archive_path.to_path_buf()),
            )),
        }
    }

    /// Extract with concurrency control using semaphore
    async fn extract_with_concurrency_control(
        &self,
        archive_path: &Path,
        target_dir: &Path,
        workspace_id: &str,
    ) -> Result<ExtractionResult> {
        // Acquire semaphore permit
        let _permit = self.concurrency_limiter.acquire().await.map_err(|e| {
            AppError::archive_error(format!("Failed to acquire permit: {}", e), None)
        })?;

        debug!(
            "Acquired extraction permit for {:?} (available: {})",
            archive_path,
            self.concurrency_limiter.available_permits()
        );

        // Check for cancellation after acquiring permit
        if self.cancellation_token.is_cancelled() {
            return Err(AppError::validation_error("Extraction cancelled"));
        }

        // Perform the actual extraction
        let result = tokio::select! {
            extraction_result = self.engine.extract_archive(archive_path, target_dir, workspace_id) => {
                extraction_result
            }
            _ = self.cancellation_token.cancelled() => {
                warn!("Extraction cancelled for {:?}", archive_path);
                Err(AppError::validation_error("Extraction cancelled"))
            }
        };

        debug!(
            "Released extraction permit for {:?} (available: {})",
            archive_path,
            self.concurrency_limiter.available_permits() + 1
        );

        result
    }

    /// Cancel all in-progress extractions
    ///
    /// This method triggers cancellation for all ongoing extractions.
    /// Extractions will stop at the next cancellation check point and
    /// perform graceful cleanup.
    pub fn cancel_all(&self) {
        info!("Cancelling all in-progress extractions");
        self.cancellation_token.cancel();
    }

    /// Check if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Get the number of in-flight extraction requests
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_requests.len()
    }

    /// Get the number of available extraction slots
    pub fn available_slots(&self) -> usize {
        self.concurrency_limiter.available_permits()
    }

    /// Create a child cancellation token for a specific extraction
    ///
    /// This allows cancelling individual extractions without affecting others.
    pub fn create_child_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::metadata_db::MetadataDB;
    use crate::path_manager::{PathConfig, PathManager};
    use crate::security_detector::{SecurityDetector, SecurityPolicy};
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;

    async fn create_test_orchestrator() -> ExtractionOrchestrator {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let metadata_db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());

        let path_config = PathConfig::default();
        let path_manager = Arc::new(PathManager::new(path_config, metadata_db));

        let security_policy = SecurityPolicy::default();
        let security_detector = Arc::new(SecurityDetector::new(security_policy));

        let policy = crate::extraction_engine::ExtractionPolicy::default();
        let engine =
            Arc::new(ExtractionEngine::new(path_manager, security_detector, policy).unwrap());

        ExtractionOrchestrator::new(engine, Some(2))
    }

    fn sample_extraction_result(workspace_id: &str) -> ExtractionResult {
        ExtractionResult {
            workspace_id: workspace_id.to_string(),
            extracted_files: Vec::new(),
            warnings: Vec::new(),
            max_depth_reached: 1,
            total_files: 2,
            total_bytes: 128,
            path_shortenings_applied: 0,
            depth_limit_skips: 0,
            extraction_duration_secs: 0.5,
            extraction_speed_bytes_per_sec: 256.0,
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = create_test_orchestrator().await;
        assert_eq!(orchestrator.available_slots(), 2);
        assert_eq!(orchestrator.in_flight_count(), 0);
        assert!(!orchestrator.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation() {
        let orchestrator = create_test_orchestrator().await;
        assert!(!orchestrator.is_cancelled());

        orchestrator.cancel_all();
        assert!(orchestrator.is_cancelled());
    }

    #[tokio::test]
    async fn test_child_token() {
        let orchestrator = create_test_orchestrator().await;
        let child_token = orchestrator.create_child_token();

        assert!(!child_token.is_cancelled());

        orchestrator.cancel_all();
        assert!(orchestrator.is_cancelled());
        assert!(child_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_concurrency_limit() {
        let orchestrator = Arc::new(create_test_orchestrator().await);
        assert_eq!(orchestrator.available_slots(), 2);

        // Simulate acquiring permits
        let permit1 = orchestrator.concurrency_limiter.acquire().await.unwrap();
        assert_eq!(orchestrator.available_slots(), 1);

        let permit2 = orchestrator.concurrency_limiter.acquire().await.unwrap();
        assert_eq!(orchestrator.available_slots(), 0);

        // Release permits
        drop(permit1);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(orchestrator.available_slots(), 1);

        drop(permit2);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(orchestrator.available_slots(), 2);
    }

    #[tokio::test]
    async fn test_duplicate_archive_requests_wait_for_same_result() {
        let orchestrator = Arc::new(create_test_orchestrator().await);
        let archive_path = PathBuf::from("/tmp/dedup-test.zip");
        let target_dir = TempDir::new().unwrap();
        let shared_result = Arc::new(SharedExtractionState::new());
        let expected = sample_extraction_result("workspace-1");

        orchestrator
            .in_flight_requests
            .insert(archive_path.clone(), shared_result.clone());

        let orchestrator_for_waiter = Arc::clone(&orchestrator);
        let archive_path_for_waiter = archive_path.clone();
        let target_dir_for_waiter = target_dir.path().to_path_buf();
        let waiter = tokio::spawn(async move {
            orchestrator_for_waiter
                .extract_archive(
                    &archive_path_for_waiter,
                    &target_dir_for_waiter,
                    "workspace-1",
                )
                .await
        });

        sleep(Duration::from_millis(50)).await;
        assert!(
            !waiter.is_finished(),
            "Duplicate request should wait until the shared result is published"
        );

        shared_result.store_result(Arc::new(Ok(expected.clone())));
        orchestrator.in_flight_requests.remove(&archive_path);

        let result = waiter.await.unwrap().unwrap();
        assert_eq!(result.workspace_id, expected.workspace_id);
        assert_eq!(result.total_files, expected.total_files);
        assert_eq!(result.total_bytes, expected.total_bytes);
        assert_eq!(result.max_depth_reached, expected.max_depth_reached);
    }

    #[tokio::test]
    async fn test_inflight_entry_cleaned_up_on_error() {
        let orchestrator = create_test_orchestrator().await;
        let archive_path = PathBuf::from("/tmp/cleanup-test.zip");
        let shared_result = Arc::new(SharedExtractionState::new());

        orchestrator
            .in_flight_requests
            .insert(archive_path.clone(), shared_result.clone());

        {
            let _guard = InFlightRequestGuard::new(
                archive_path.clone(),
                Arc::clone(&orchestrator.in_flight_requests),
                shared_result.clone(),
            );
        }

        assert_eq!(orchestrator.in_flight_count(), 0);
        let stored_result = shared_result.clone_result();
        assert!(stored_result.is_some());
        assert!(stored_result.unwrap().as_ref().is_err());
    }
}
