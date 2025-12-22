//! Extraction Orchestrator for Concurrency Control
//!
//! This module implements the orchestration layer that manages concurrent
//! extraction operations, request deduplication, and cancellation support.
//! It wraps the ExtractionEngine and provides controlled access with
//! resource management.

use crate::archive::{ExtractionEngine, ExtractionResult};
use crate::error::{AppError, Result};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Shared result for request deduplication
type SharedExtractionResult = Arc<Mutex<Option<Arc<Result<ExtractionResult>>>>>;

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

        // Check if there's already an in-flight request for this archive
        if let Some(existing_result) = self.in_flight_requests.get(&archive_path_buf) {
            debug!(
                "Request deduplication: waiting for existing extraction of {:?}",
                archive_path
            );

            // Wait for the existing extraction to complete
            let result = {
                let guard = existing_result.lock().await;
                guard.clone()
            };

            return match result {
                Some(arc_result) => match arc_result.as_ref() {
                    Ok(extraction_result) => Ok(extraction_result.clone()),
                    Err(e) => Err(AppError::archive_error(
                        format!("Deduplicated request failed: {}", e),
                        Some(archive_path_buf.clone()),
                    )),
                },
                None => Err(AppError::archive_error(
                    "Deduplicated request completed but result was not available",
                    Some(archive_path_buf.clone()),
                )),
            };
        }

        // Create a shared result for this extraction
        let shared_result: SharedExtractionResult = Arc::new(Mutex::new(None));
        self.in_flight_requests
            .insert(archive_path_buf.clone(), shared_result.clone());

        // Perform the extraction with concurrency control
        let result = self
            .extract_with_concurrency_control(archive_path, target_dir, workspace_id)
            .await;

        // Store the result in the shared result
        {
            let mut guard = shared_result.lock().await;
            *guard = Some(Arc::new(match &result {
                Ok(extraction_result) => Ok(extraction_result.clone()),
                Err(e) => Err(AppError::archive_error(
                    format!("{}", e),
                    Some(archive_path_buf.clone()),
                )),
            }));
        }

        // Remove from in-flight requests
        self.in_flight_requests.remove(&archive_path_buf);

        result
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
    use crate::archive::{PathConfig, PathManager, SecurityDetector, SecurityPolicy};
    use crate::services::MetadataDB;
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

        let policy = crate::archive::ExtractionPolicy::default();
        let engine =
            Arc::new(ExtractionEngine::new(path_manager, security_detector, policy).unwrap());

        ExtractionOrchestrator::new(engine, Some(2))
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
}
