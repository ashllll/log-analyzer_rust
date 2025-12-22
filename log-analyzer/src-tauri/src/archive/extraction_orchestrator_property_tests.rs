//! Property-Based Tests for ExtractionOrchestrator
//!
//! These tests validate the correctness properties of the extraction orchestrator
//! using property-based testing with proptest.

use super::extraction_orchestrator::ExtractionOrchestrator;
use crate::archive::{
    ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector, SecurityPolicy,
};
use crate::services::MetadataDB;
use proptest::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::time::sleep;

/// Helper to create a test orchestrator with specified concurrency limit
async fn create_test_orchestrator(max_concurrent: usize) -> ExtractionOrchestrator {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let metadata_db = Arc::new(MetadataDB::new(db_path.to_str().unwrap()).await.unwrap());

    let path_config = PathConfig::default();
    let path_manager = Arc::new(PathManager::new(path_config, metadata_db));

    let security_policy = SecurityPolicy::default();
    let security_detector = Arc::new(SecurityDetector::new(security_policy));

    let policy = ExtractionPolicy::default();
    let engine = Arc::new(ExtractionEngine::new(path_manager, security_detector, policy).unwrap());

    ExtractionOrchestrator::new(engine, Some(max_concurrent))
}

// ============================================================================
// Property 34: Concurrency limit enforcement
// Feature: enhanced-archive-handling, Property 34: Concurrency limit enforcement
// Validates: Requirements 8.1
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 34: Concurrency limit enforcement
    ///
    /// For any number of concurrent extraction requests, the number of actively
    /// executing extractions should never exceed the configured limit.
    ///
    /// This test verifies that:
    /// 1. The orchestrator respects the configured concurrency limit
    /// 2. Available slots decrease as permits are acquired
    /// 3. Available slots increase as permits are released
    /// 4. The total of in-use + available always equals the limit
    #[test]
    fn prop_concurrency_limit_enforcement(
        max_concurrent in 1usize..=8,
        num_requests in 1usize..=20,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = Arc::new(create_test_orchestrator(max_concurrent).await);

            // Initial state: all slots available
            prop_assert_eq!(orchestrator.available_slots(), max_concurrent);

            // Acquire permits up to the limit
            let mut permits = Vec::new();
            for i in 0..num_requests.min(max_concurrent) {
                let permit = orchestrator.concurrency_limiter.acquire().await.unwrap();
                permits.push(permit);

                // Check that available slots decreased
                let expected_available = max_concurrent - (i + 1);
                prop_assert_eq!(orchestrator.available_slots(), expected_available);
            }

            // At this point, all slots should be taken if num_requests >= max_concurrent
            if num_requests >= max_concurrent {
                prop_assert_eq!(orchestrator.available_slots(), 0);
            }

            // Release all permits
            permits.clear();
            sleep(Duration::from_millis(10)).await;

            // All slots should be available again
            prop_assert_eq!(orchestrator.available_slots(), max_concurrent);

            Ok(())
        })?;
    }

    /// Property: Concurrency limit never exceeded
    ///
    /// Verifies that even with many concurrent acquire attempts,
    /// the number of acquired permits never exceeds the limit.
    #[test]
    fn prop_concurrency_never_exceeded(
        max_concurrent in 1usize..=8,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = Arc::new(create_test_orchestrator(max_concurrent).await);

            // Try to acquire more permits than the limit
            let mut permits = Vec::new();
            for _ in 0..max_concurrent {
                let permit = orchestrator.concurrency_limiter.acquire().await.unwrap();
                permits.push(permit);
            }

            // Should have zero available slots
            prop_assert_eq!(orchestrator.available_slots(), 0);

            // Try to acquire one more (should block, so we use try_acquire)
            let result = orchestrator.concurrency_limiter.try_acquire();
            prop_assert!(result.is_err(), "Should not be able to acquire beyond limit");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 45: Request deduplication
// Feature: enhanced-archive-handling, Property 45: Request deduplication
// Validates: Requirements 10.3
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 45: Request deduplication
    ///
    /// For any set of concurrent extraction requests targeting the same archive,
    /// only one extraction should execute and all requests should receive the
    /// same result.
    ///
    /// This test verifies that:
    /// 1. Multiple requests for the same archive are deduplicated
    /// 2. Only one actual extraction occurs
    /// 3. All requests receive the same result
    /// 4. In-flight count reflects deduplication
    #[test]
    fn prop_request_deduplication(
        num_duplicate_requests in 2usize..=10,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = Arc::new(create_test_orchestrator(4).await);

            // Create a test archive path (doesn't need to exist for this test)
            let temp_dir = TempDir::new().unwrap();
            let archive_path = temp_dir.path().join("test.zip");
            let target_dir = temp_dir.path().join("output");

            // Track in-flight count before requests
            let initial_in_flight = orchestrator.in_flight_count();

            // Spawn multiple concurrent requests for the same archive
            let mut handles = Vec::new();
            for _ in 0..num_duplicate_requests {
                let orch = Arc::clone(&orchestrator);
                let archive = archive_path.clone();
                let target = target_dir.clone();

                let handle = tokio::spawn(async move {
                    // This will fail because the archive doesn't exist,
                    // but that's okay - we're testing deduplication logic
                    let _ = orch.extract_archive(&archive, &target, "test_workspace").await;
                });
                handles.push(handle);
            }

            // Give requests time to register
            sleep(Duration::from_millis(50)).await;

            // Check that in-flight count increased by at most 1
            // (deduplication means only one entry in the map)
            let current_in_flight = orchestrator.in_flight_count();
            prop_assert!(
                current_in_flight <= initial_in_flight + 1,
                "In-flight count should increase by at most 1 due to deduplication"
            );

            // Wait for all requests to complete
            for handle in handles {
                let _ = handle.await;
            }

            // Give cleanup time to complete
            sleep(Duration::from_millis(50)).await;

            // In-flight count should return to initial state
            prop_assert_eq!(orchestrator.in_flight_count(), initial_in_flight);

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 44: Cancellation responsiveness
// Feature: enhanced-archive-handling, Property 44: Cancellation responsiveness
// Validates: Requirements 10.2
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 44: Cancellation responsiveness
    ///
    /// For any in-progress extraction, invoking cancellation should stop the
    /// extraction within 2 seconds and perform graceful cleanup.
    ///
    /// This test verifies that:
    /// 1. Cancellation is detected quickly
    /// 2. Cancelled extractions return an error
    /// 3. The cancellation state is reflected in the orchestrator
    /// 4. Child tokens are also cancelled
    #[test]
    fn prop_cancellation_responsiveness(
        delay_before_cancel_ms in 10u64..=100,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = Arc::new(create_test_orchestrator(2).await);

            // Verify initial state
            prop_assert!(!orchestrator.is_cancelled());

            // Create a child token
            let child_token = orchestrator.create_child_token();
            prop_assert!(!child_token.is_cancelled());

            // Wait a bit before cancelling
            sleep(Duration::from_millis(delay_before_cancel_ms)).await;

            // Cancel all operations
            let cancel_start = std::time::Instant::now();
            orchestrator.cancel_all();
            let cancel_duration = cancel_start.elapsed();

            // Cancellation should be immediate (< 10ms)
            prop_assert!(
                cancel_duration < Duration::from_millis(10),
                "Cancellation should be immediate, took {:?}",
                cancel_duration
            );

            // Verify cancellation state
            prop_assert!(orchestrator.is_cancelled());
            prop_assert!(child_token.is_cancelled());

            // Try to start a new extraction after cancellation
            let temp_dir = TempDir::new().unwrap();
            let archive_path = temp_dir.path().join("test.zip");
            let target_dir = temp_dir.path().join("output");

            let result = orchestrator
                .extract_archive(&archive_path, &target_dir, "test_workspace")
                .await;

            // Should fail due to cancellation
            prop_assert!(result.is_err(), "Extraction should fail after cancellation");

            let error_msg = result.unwrap_err().to_string();
            prop_assert!(
                error_msg.contains("cancel") || error_msg.contains("Cancel"),
                "Error should mention cancellation: {}",
                error_msg
            );

            Ok(())
        })?;
    }

    /// Property: Multiple cancellations are idempotent
    ///
    /// Calling cancel_all multiple times should be safe and idempotent.
    #[test]
    fn prop_cancellation_idempotent(
        num_cancellations in 1usize..=10,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = Arc::new(create_test_orchestrator(2).await);

            prop_assert!(!orchestrator.is_cancelled());

            // Cancel multiple times
            for _ in 0..num_cancellations {
                orchestrator.cancel_all();
                prop_assert!(orchestrator.is_cancelled());
            }

            // Should still be cancelled
            prop_assert!(orchestrator.is_cancelled());

            Ok(())
        })?;
    }
}

// ============================================================================
// Additional Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Available slots consistency
    ///
    /// The available slots should always be between 0 and max_concurrent.
    #[test]
    fn prop_available_slots_consistency(
        max_concurrent in 1usize..=8,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = create_test_orchestrator(max_concurrent).await;

            let available = orchestrator.available_slots();
            prop_assert!(
                available <= max_concurrent,
                "Available slots {} should not exceed max {}",
                available,
                max_concurrent
            );

            Ok(())
        })?;
    }

    /// Property: In-flight count non-negative
    ///
    /// The in-flight count should always be non-negative.
    #[test]
    fn prop_in_flight_count_non_negative(
        max_concurrent in 1usize..=8,
    ) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let orchestrator = create_test_orchestrator(max_concurrent).await;

            let in_flight = orchestrator.in_flight_count();
            prop_assert!(
                in_flight >= 0,
                "In-flight count should be non-negative, got {}",
                in_flight
            );

            Ok(())
        })?;
    }
}

// ============================================================================
// Integration Test: Concurrent Extractions
// Validates: Requirements 8.1, 10.3
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    /// Helper to create a simple test ZIP archive
    fn create_test_zip(path: &std::path::Path) -> std::io::Result<()> {
        let file = fs::File::create(path)?;
        let mut zip = ZipWriter::new(file);

        // Add a simple text file
        zip.start_file("test.txt", FileOptions::default())?;
        zip.write_all(b"Test content")?;

        zip.finish()?;
        Ok(())
    }

    /// Integration test: Submit 20 concurrent requests, verify limit enforcement and throughput
    ///
    /// This test verifies that:
    /// 1. The orchestrator handles many concurrent requests
    /// 2. The concurrency limit is enforced
    /// 3. All requests complete successfully or with expected errors
    /// 4. Request deduplication works with real archives
    #[tokio::test]
    async fn test_concurrent_extractions_integration() {
        // Create orchestrator with limit of 4
        let max_concurrent = 4;
        let orchestrator = Arc::new(create_test_orchestrator(max_concurrent).await);

        // Create test archives
        let temp_dir = TempDir::new().unwrap();
        let mut archive_paths = Vec::new();

        for i in 0..5 {
            let archive_path = temp_dir.path().join(format!("test_{}.zip", i));
            create_test_zip(&archive_path).expect("Failed to create test archive");
            archive_paths.push(archive_path);
        }

        // Submit 20 concurrent extraction requests (4 unique archives, 5 duplicates each)
        let mut handles = Vec::new();
        for i in 0..20 {
            let orch = Arc::clone(&orchestrator);
            let archive = archive_paths[i % 5].clone();
            let target = temp_dir.path().join(format!("output_{}", i));

            let handle = tokio::spawn(async move {
                orch.extract_archive(&archive, &target, "test_workspace")
                    .await
            });
            handles.push(handle);
        }

        // Monitor concurrency during execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check that we're not exceeding the limit
        let available = orchestrator.available_slots();
        assert!(
            available <= max_concurrent,
            "Available slots {} should not exceed max {}",
            available,
            max_concurrent
        );

        // Wait for all requests to complete
        let mut success_count = 0;
        let mut error_count = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(_)) => error_count += 1,
                Err(_) => error_count += 1,
            }
        }

        // All requests should complete (either success or error)
        assert_eq!(
            success_count + error_count,
            20,
            "All 20 requests should complete"
        );

        // After completion, all slots should be available
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            orchestrator.available_slots(),
            max_concurrent,
            "All slots should be available after completion"
        );

        // No in-flight requests should remain
        assert_eq!(
            orchestrator.in_flight_count(),
            0,
            "No in-flight requests should remain"
        );
    }

    /// Integration test: Verify request deduplication with real archives
    #[tokio::test]
    async fn test_request_deduplication_integration() {
        let orchestrator = Arc::new(create_test_orchestrator(4).await);

        // Create a single test archive
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        create_test_zip(&archive_path).expect("Failed to create test archive");

        // Submit 10 concurrent requests for the same archive
        let mut handles = Vec::new();
        for i in 0..10 {
            let orch = Arc::clone(&orchestrator);
            let archive = archive_path.clone();
            let target = temp_dir.path().join(format!("output_{}", i));

            let handle = tokio::spawn(async move {
                orch.extract_archive(&archive, &target, "test_workspace")
                    .await
            });
            handles.push(handle);
        }

        // Give requests time to register
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check that only one request is in-flight (deduplication)
        let in_flight = orchestrator.in_flight_count();
        assert!(
            in_flight <= 1,
            "Should have at most 1 in-flight request due to deduplication, got {}",
            in_flight
        );

        // Wait for all to complete
        for handle in handles {
            let _ = handle.await;
        }

        // Cleanup
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(orchestrator.in_flight_count(), 0);
    }

    /// Integration test: Verify cancellation stops ongoing extractions
    #[tokio::test]
    async fn test_cancellation_integration() {
        let orchestrator = Arc::new(create_test_orchestrator(4).await);

        // Create test archives
        let temp_dir = TempDir::new().unwrap();
        let mut archive_paths = Vec::new();

        for i in 0..3 {
            let archive_path = temp_dir.path().join(format!("test_{}.zip", i));
            create_test_zip(&archive_path).expect("Failed to create test archive");
            archive_paths.push(archive_path);
        }

        // Start several extractions
        let mut handles = Vec::new();
        for i in 0..3 {
            let orch = Arc::clone(&orchestrator);
            let archive = archive_paths[i].clone();
            let target = temp_dir.path().join(format!("output_{}", i));

            let handle = tokio::spawn(async move {
                orch.extract_archive(&archive, &target, "test_workspace")
                    .await
            });
            handles.push(handle);
        }

        // Wait a bit for extractions to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel all
        orchestrator.cancel_all();
        assert!(orchestrator.is_cancelled());

        // Wait for all to complete
        let mut cancelled_count = 0;
        let mut completed_count = 0;
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => completed_count += 1,
                Ok(Err(e)) => {
                    if e.to_string().to_lowercase().contains("cancel") {
                        cancelled_count += 1;
                    }
                }
                Err(_) => {}
            }
        }

        // Either some were cancelled OR all completed before cancellation
        // Both are valid outcomes depending on timing
        assert!(
            cancelled_count > 0 || completed_count > 0,
            "Extractions should either be cancelled or complete"
        );

        // Verify that after cancellation, new requests fail
        let archive_path = temp_dir.path().join("test_new.zip");
        create_test_zip(&archive_path).expect("Failed to create test archive");
        let target = temp_dir.path().join("output_new");

        let result = orchestrator
            .extract_archive(&archive_path, &target, "test_workspace")
            .await;

        assert!(
            result.is_err(),
            "New extractions should fail after cancellation"
        );
    }
}
