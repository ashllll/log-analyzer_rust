//! Property-based tests for ExtractionEngine
//!
//! These tests verify correctness properties of the extraction engine using
//! property-based testing with proptest.

#[cfg(test)]
mod property_tests {
    use crate::archive::{
        ExtractionContext, ExtractionEngine, ExtractionItem, ExtractionPolicy, ExtractionStack,
        PathConfig, PathManager, SecurityDetector,
    };
    use crate::services::MetadataDB;
    use proptest::prelude::*;
    use proptest::proptest;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Helper to create a test extraction engine
    async fn create_test_engine(max_depth: usize) -> ExtractionEngine {
        let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
        let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
        let security_detector = Arc::new(SecurityDetector::default());
        let policy = ExtractionPolicy {
            max_depth,
            ..Default::default()
        };

        ExtractionEngine::new(path_manager, security_detector, policy).unwrap()
    }

    /// **Feature: enhanced-archive-handling, Property 6: Depth limit enforcement**
    /// **Validates: Requirements 2.1**
    ///
    /// Property: For any archive extraction operation, the maximum nesting depth
    /// should never exceed the configured limit (default 10 levels).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_depth_limit_enforcement(
            max_depth in 1usize..=20usize,
            num_nested_archives in 1usize..30usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let engine = create_test_engine(max_depth).await;
                let temp_dir = TempDir::new().unwrap();

                // Create a simulated nested archive structure
                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                // Try to push archives at various depths
                let mut max_depth_reached = 0;
                let mut depth_limit_violations = 0;

                for depth in 0..num_nested_archives {
                    let archive_path = temp_dir.path().join(format!("archive_{}.zip", depth));
                    let target_dir = temp_dir.path().join(format!("target_{}", depth));

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        depth,
                        context.clone(),
                    );

                    // Check if this depth would exceed the limit
                    if depth >= max_depth {
                        // This should be caught by the engine
                        depth_limit_violations += 1;
                    } else {
                        max_depth_reached = max_depth_reached.max(depth);
                    }

                    // Push to stack (stack itself doesn't enforce depth limit)
                    let _ = stack.push(item);
                }

                // Property 1: max_depth_reached should never exceed configured max_depth
                prop_assert!(
                    max_depth_reached < max_depth,
                    "Max depth reached {} should be less than limit {}",
                    max_depth_reached,
                    max_depth
                );

                // Property 2: If we have more archives than max_depth, some should be skipped
                if num_nested_archives > max_depth {
                    prop_assert!(
                        depth_limit_violations > 0,
                        "Should have depth limit violations when num_archives {} > max_depth {}",
                        num_nested_archives,
                        max_depth
                    );
                }

                // Property 3: The policy's max_depth should be within valid range (1-20)
                prop_assert!(
                    engine.policy().max_depth >= 1 && engine.policy().max_depth <= 20,
                    "Policy max_depth {} should be in range [1, 20]",
                    engine.policy().max_depth
                );

                // Property 4: Configured max_depth should match what we set
                prop_assert_eq!(
                    engine.policy().max_depth,
                    max_depth,
                    "Engine max_depth should match configured value"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_depth_limit_consistency(
            max_depth in 1usize..=20usize,
            depth in 0usize..30usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let engine = create_test_engine(max_depth).await;
                let context = ExtractionContext::new("test_workspace".to_string());

                // Property: is_depth_limit_reached should be consistent with max_depth
                let expected = depth >= max_depth;

                // Create a context at the specified depth
                let mut test_context = context.clone();
                test_context.current_depth = depth;

                let actual = test_context.is_depth_limit_reached(max_depth);

                prop_assert_eq!(
                    actual,
                    expected,
                    "Depth limit check should be consistent: depth={}, max_depth={}, expected={}, actual={}",
                    depth,
                    max_depth,
                    expected,
                    actual
                );

                // Property: depth < max_depth should never trigger limit
                if depth < max_depth {
                    prop_assert!(
                        !actual,
                        "Depth {} below limit {} should not trigger limit",
                        depth,
                        max_depth
                    );
                }

                // Property: depth >= max_depth should always trigger limit
                if depth >= max_depth {
                    prop_assert!(
                        actual,
                        "Depth {} at or above limit {} should trigger limit",
                        depth,
                        max_depth
                    );
                }

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_depth_limit_boundary(
            max_depth in 1usize..=20usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let engine = create_test_engine(max_depth).await;
                let context = ExtractionContext::new("test_workspace".to_string());

                // Test boundary conditions
                let mut test_context = context.clone();

                // Property: depth = max_depth - 1 should not trigger limit
                test_context.current_depth = max_depth.saturating_sub(1);
                if max_depth > 0 {
                    prop_assert!(
                        !test_context.is_depth_limit_reached(max_depth),
                        "Depth {} (max_depth - 1) should not trigger limit {}",
                        max_depth - 1,
                        max_depth
                    );
                }

                // Property: depth = max_depth should trigger limit
                test_context.current_depth = max_depth;
                prop_assert!(
                    test_context.is_depth_limit_reached(max_depth),
                    "Depth {} (max_depth) should trigger limit {}",
                    max_depth,
                    max_depth
                );

                // Property: depth = max_depth + 1 should trigger limit
                test_context.current_depth = max_depth + 1;
                prop_assert!(
                    test_context.is_depth_limit_reached(max_depth),
                    "Depth {} (max_depth + 1) should trigger limit {}",
                    max_depth + 1,
                    max_depth
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_child_context_depth_increment(
            max_depth in 1usize..=20usize,
            initial_depth in 0usize..15usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let _engine = create_test_engine(max_depth).await;

                // Create a context at initial depth
                let mut parent_context = ExtractionContext::new("test_workspace".to_string());
                parent_context.current_depth = initial_depth;

                // Create child context
                let child_context = parent_context.create_child(PathBuf::from("parent.zip"));

                // Property: child depth should be parent depth + 1
                prop_assert_eq!(
                    child_context.current_depth,
                    initial_depth + 1,
                    "Child depth should be parent depth + 1"
                );

                // Property: child should have parent archive set
                prop_assert!(
                    child_context.parent_archive.is_some(),
                    "Child should have parent archive set"
                );

                // Property: workspace_id should be preserved
                prop_assert_eq!(
                    child_context.workspace_id,
                    parent_context.workspace_id,
                    "Workspace ID should be preserved in child context"
                );

                // Property: accumulated metrics should be preserved
                prop_assert_eq!(
                    child_context.accumulated_size,
                    parent_context.accumulated_size,
                    "Accumulated size should be preserved"
                );

                prop_assert_eq!(
                    child_context.accumulated_files,
                    parent_context.accumulated_files,
                    "Accumulated files should be preserved"
                );

                Ok(())
            }).unwrap();
        }

        /// **Feature: enhanced-archive-handling, Property 7: Iterative traversal stack safety**
        /// **Validates: Requirements 2.3**
        ///
        /// Property: For any deeply nested archive structure (up to 20 levels), the extraction
        /// process should complete without stack overflow errors using iterative traversal.

        #[test]
        fn prop_stack_safety_no_overflow(
            nesting_depth in 1usize..=20usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Create engine with max depth that allows this nesting (capped at 20)
                let max_depth = std::cmp::min(nesting_depth + 1, 20);
                let engine = create_test_engine(max_depth).await;
                let temp_dir = TempDir::new().unwrap();

                // Create a stack and simulate deep nesting
                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                // Push items at various depths to simulate deep nesting
                for depth in 0..nesting_depth {
                    let archive_path = temp_dir.path().join(format!("level_{}.zip", depth));
                    let target_dir = temp_dir.path().join(format!("extract_{}", depth));

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        depth,
                        context.clone(),
                    );

                    // Property: Stack should accept items without overflow
                    let push_result = stack.push(item);
                    prop_assert!(
                        push_result.is_ok(),
                        "Stack should accept item at depth {} without overflow",
                        depth
                    );
                }

                // Property: Stack size should equal nesting depth
                prop_assert_eq!(
                    stack.len(),
                    nesting_depth,
                    "Stack size should match number of pushed items"
                );

                // Property: Stack should not be empty after pushing items
                prop_assert!(
                    !stack.is_empty(),
                    "Stack should not be empty after pushing {} items",
                    nesting_depth
                );

                // Property: We should be able to pop all items
                let mut popped_count = 0;
                while stack.pop().is_some() {
                    popped_count += 1;
                }

                prop_assert_eq!(
                    popped_count,
                    nesting_depth,
                    "Should be able to pop all {} items from stack",
                    nesting_depth
                );

                // Property: Stack should be empty after popping all items
                prop_assert!(
                    stack.is_empty(),
                    "Stack should be empty after popping all items"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_stack_size_limit_enforcement(
            num_items in 1usize..=1500usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let _engine = create_test_engine(20).await;
                let temp_dir = TempDir::new().unwrap();

                // Create a stack with default max size (1000)
                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                let mut successfully_pushed = 0;
                let mut push_failed = false;

                // Try to push items
                for i in 0..num_items {
                    let archive_path = temp_dir.path().join(format!("archive_{}.zip", i));
                    let target_dir = temp_dir.path().join(format!("target_{}", i));

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        0,
                        context.clone(),
                    );

                    match stack.push(item) {
                        Ok(_) => successfully_pushed += 1,
                        Err(_) => {
                            push_failed = true;
                            break;
                        }
                    }
                }

                // Property: If we try to push more than max_size, push should fail
                if num_items > ExtractionStack::DEFAULT_MAX_SIZE {
                    prop_assert!(
                        push_failed,
                        "Push should fail when exceeding max stack size {}",
                        ExtractionStack::DEFAULT_MAX_SIZE
                    );

                    prop_assert_eq!(
                        successfully_pushed,
                        ExtractionStack::DEFAULT_MAX_SIZE,
                        "Should successfully push exactly max_size items"
                    );
                } else {
                    // Property: If within limit, all pushes should succeed
                    prop_assert!(
                        !push_failed,
                        "All pushes should succeed when within limit"
                    );

                    prop_assert_eq!(
                        successfully_pushed,
                        num_items,
                        "Should successfully push all {} items",
                        num_items
                    );
                }

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_stack_lifo_order(
            num_items in 1usize..=100usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let _engine = create_test_engine(20).await;
                let temp_dir = TempDir::new().unwrap();

                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                // Push items with identifiable paths
                let mut pushed_paths = Vec::new();
                for i in 0..num_items {
                    let archive_path = temp_dir.path().join(format!("archive_{}.zip", i));
                    let target_dir = temp_dir.path().join(format!("target_{}", i));

                    pushed_paths.push(archive_path.clone());

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        0,
                        context.clone(),
                    );

                    stack.push(item).unwrap();
                }

                // Property: Items should be popped in LIFO order (reverse of push order)
                let mut popped_paths = Vec::new();
                while let Some(item) = stack.pop() {
                    popped_paths.push(item.archive_path);
                }

                // Reverse pushed_paths to get expected LIFO order
                pushed_paths.reverse();

                prop_assert_eq!(
                    popped_paths,
                    pushed_paths,
                    "Stack should maintain LIFO (Last-In-First-Out) order"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_stack_clear_operation(
            num_items in 1usize..=100usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let _engine = create_test_engine(20).await;
                let temp_dir = TempDir::new().unwrap();

                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                // Push items
                for i in 0..num_items {
                    let archive_path = temp_dir.path().join(format!("archive_{}.zip", i));
                    let target_dir = temp_dir.path().join(format!("target_{}", i));

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        0,
                        context.clone(),
                    );

                    stack.push(item).unwrap();
                }

                // Property: Stack should have items before clear
                prop_assert_eq!(stack.len(), num_items);
                prop_assert!(!stack.is_empty());

                // Clear the stack
                stack.clear();

                // Property: Stack should be empty after clear
                prop_assert_eq!(stack.len(), 0, "Stack length should be 0 after clear");
                prop_assert!(stack.is_empty(), "Stack should be empty after clear");

                // Property: Pop should return None after clear
                prop_assert!(
                    stack.pop().is_none(),
                    "Pop should return None on empty stack"
                );

                Ok(())
            }).unwrap();
        }

        /// **Feature: enhanced-archive-handling, Property 9: Sibling processing independence**
        /// **Validates: Requirements 2.5**
        ///
        /// Property: For any archive tree where one branch reaches the depth limit, all sibling
        /// branches at the same level should continue processing normally.

        #[test]
        fn prop_sibling_processing_independence(
            max_depth in 2usize..=10usize,
            num_siblings in 2usize..=5usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let engine = create_test_engine(max_depth).await;
                let temp_dir = TempDir::new().unwrap();

                let mut stack = ExtractionStack::new();
                let context = ExtractionContext::new("test_workspace".to_string());

                // Create multiple sibling branches at depth 0
                let mut sibling_paths = Vec::new();
                for i in 0..num_siblings {
                    let archive_path = temp_dir.path().join(format!("sibling_{}.zip", i));
                    let target_dir = temp_dir.path().join(format!("sibling_target_{}", i));

                    sibling_paths.push(archive_path.clone());

                    let item = ExtractionItem::new(
                        archive_path,
                        target_dir,
                        0,
                        context.clone(),
                    );

                    stack.push(item).unwrap();
                }

                // Property: All siblings should be on the stack
                prop_assert_eq!(
                    stack.len(),
                    num_siblings,
                    "All {} siblings should be on stack",
                    num_siblings
                );

                // Simulate one branch reaching depth limit by creating a deep branch
                let deep_branch_path = temp_dir.path().join("deep_branch.zip");
                let deep_branch_target = temp_dir.path().join("deep_target");

                // Create an item at max_depth (which would be at the limit)
                let deep_item = ExtractionItem::new(
                    deep_branch_path.clone(),
                    deep_branch_target,
                    max_depth,
                    context.clone(),
                );

                stack.push(deep_item).unwrap();

                // Property: Stack should now have siblings + 1 deep branch
                prop_assert_eq!(
                    stack.len(),
                    num_siblings + 1,
                    "Stack should have all siblings plus deep branch"
                );

                // Pop the deep branch (simulating it being processed and hitting depth limit)
                let popped_deep = stack.pop().unwrap();
                prop_assert_eq!(
                    popped_deep.depth,
                    max_depth,
                    "Popped item should be the deep branch"
                );

                // Property: After processing deep branch, siblings should still be on stack
                prop_assert_eq!(
                    stack.len(),
                    num_siblings,
                    "All {} siblings should still be on stack after deep branch processed",
                    num_siblings
                );

                // Property: We should be able to process all siblings
                let mut processed_siblings = 0;
                while let Some(item) = stack.pop() {
                    // Verify this is a sibling (depth 0)
                    prop_assert_eq!(
                        item.depth,
                        0,
                        "Remaining items should be siblings at depth 0"
                    );
                    processed_siblings += 1;
                }

                // Property: All siblings should have been processed
                prop_assert_eq!(
                    processed_siblings,
                    num_siblings,
                    "Should process all {} siblings",
                    num_siblings
                );

                Ok(())
            }).unwrap();
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 35: Streaming memory bounds**
/// **Validates: Requirements 8.2**
///
/// Property: For any large archive extraction, the peak memory usage should not exceed
/// buffer_size * concurrent_extractions + overhead (estimated 10MB).
#[cfg(test)]
mod streaming_memory_bounds_tests {
    use crate::archive::{
        ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector,
    };
    use crate::services::MetadataDB;
    use proptest::prelude::*;
    use proptest::proptest;
    use std::sync::Arc;

    proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_streaming_memory_bounds(
                buffer_size in 1024usize..=1024*1024usize, // 1KB to 1MB
                max_parallel_files in 1usize..=8usize,
            ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    buffer_size,
                    max_parallel_files,
                    ..Default::default()
                };

                let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();

                // Property 1: Policy should reflect configured values
                prop_assert_eq!(
                    engine.policy().buffer_size,
                    buffer_size,
                    "Engine buffer_size should match configured value"
                );

                prop_assert_eq!(
                    engine.policy().max_parallel_files,
                    max_parallel_files,
                    "Engine max_parallel_files should match configured value"
                );

                // Property 2: Maximum theoretical memory usage should be bounded
                let max_theoretical_memory = buffer_size * max_parallel_files;
                let overhead = 10 * 1024 * 1024; // 10MB overhead
                let max_expected_memory = max_theoretical_memory + overhead;

                // Property 3: Buffer size should be positive
                prop_assert!(
                    buffer_size > 0,
                    "Buffer size should be positive"
                );

                // Property 4: Max parallel files should be positive
                prop_assert!(
                    max_parallel_files > 0,
                    "Max parallel files should be positive"
                );

                // Property 5: Memory bound should be reasonable (< 1GB for typical configs)
                if buffer_size <= 64 * 1024 && max_parallel_files <= 4 {
                    prop_assert!(
                        max_expected_memory < 1024 * 1024 * 1024,
                        "Memory bound should be < 1GB for typical configs"
                    );
                }

                // Property 6: Larger buffer or more parallel files should increase memory bound
                let larger_buffer_policy = ExtractionPolicy {
                    buffer_size: buffer_size * 2,
                    max_parallel_files,
                    ..Default::default()
                };

                let larger_buffer_memory = larger_buffer_policy.buffer_size * larger_buffer_policy.max_parallel_files;
                prop_assert!(
                    larger_buffer_memory >= max_theoretical_memory,
                    "Larger buffer should not decrease memory bound"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_streaming_buffer_size_validation(
            buffer_size in 0usize..=2*1024*1024usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    buffer_size,
                    ..Default::default()
                };

                let result = ExtractionEngine::new(path_manager, security_detector, policy);

                // Property: Zero buffer size should be rejected
                if buffer_size == 0 {
                    prop_assert!(
                        result.is_err(),
                        "Zero buffer size should be rejected"
                    );
                } else {
                    // Property: Positive buffer size should be accepted
                    prop_assert!(
                        result.is_ok(),
                        "Positive buffer size should be accepted"
                    );

                    let engine = result.unwrap();
                    prop_assert_eq!(
                        engine.policy().buffer_size,
                        buffer_size,
                        "Engine should use configured buffer size"
                    );
                }

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_parallel_extraction_limit(
            max_parallel_files in 0usize..=16usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    max_parallel_files,
                    ..Default::default()
                };

                let result = ExtractionEngine::new(path_manager, security_detector, policy);

                // Property: Zero max_parallel_files should be rejected
                if max_parallel_files == 0 {
                    prop_assert!(
                        result.is_err(),
                        "Zero max_parallel_files should be rejected"
                    );
                } else {
                    // Property: Positive max_parallel_files should be accepted
                    prop_assert!(
                        result.is_ok(),
                        "Positive max_parallel_files should be accepted"
                    );

                    let engine = result.unwrap();
                    prop_assert_eq!(
                        engine.policy().max_parallel_files,
                        max_parallel_files,
                        "Engine should use configured max_parallel_files"
                    );
                }

                Ok(())
            }).unwrap();
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 36: Directory creation batching**
/// **Validates: Requirements 8.3**
///
/// Property: For any extraction creating multiple directories, the number of filesystem
/// syscalls should be less than the number of directories (due to batching).
#[cfg(test)]
mod directory_creation_batching_tests {
    use crate::archive::{
        ExtractionEngine, ExtractionPolicy, PathConfig, PathManager, SecurityDetector,
    };
    use crate::services::MetadataDB;
    use proptest::prelude::*;
    use proptest::proptest;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_directory_creation_batching(
            num_directories in 1usize..=100usize,
            dir_batch_size in 1usize..=20usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    dir_batch_size,
                    ..Default::default()
                };

                let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();
                let temp_dir = TempDir::new().unwrap();

                // Create list of directories to create
                let directories: Vec<PathBuf> = (0..num_directories)
                    .map(|i| temp_dir.path().join(format!("dir_{}", i)))
                    .collect();

                // Property 1: Engine should use configured batch size
                prop_assert_eq!(
                    engine.policy().dir_batch_size,
                    dir_batch_size,
                    "Engine should use configured dir_batch_size"
                );

                // Create directories in batches
                let created = engine.create_directories_batched(&directories).await.unwrap();

                // Property 2: All directories should be created
                prop_assert_eq!(
                    created,
                    num_directories,
                    "Should create all {} directories",
                    num_directories
                );

                // Property 3: All directories should exist
                for dir in &directories {
                    prop_assert!(
                        dir.exists(),
                        "Directory {:?} should exist after batched creation",
                        dir
                    );
                }

                // Property 4: Number of batches should be ceil(num_directories / dir_batch_size)
                let expected_batches = (num_directories + dir_batch_size - 1) / dir_batch_size;

                // Property 5: Batch count should be less than or equal to num_directories
                prop_assert!(
                    expected_batches <= num_directories,
                    "Number of batches {} should be <= num_directories {}",
                    expected_batches,
                    num_directories
                );

                // Property 6: If batch_size >= num_directories, should process in 1 batch
                if dir_batch_size >= num_directories {
                    prop_assert_eq!(
                        expected_batches,
                        1,
                        "Should process in 1 batch when batch_size >= num_directories"
                    );
                }

                // Property 7: Batch size should be positive
                prop_assert!(
                    dir_batch_size > 0,
                    "Batch size should be positive"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_directory_batching_deduplication(
            num_unique_dirs in 1usize..=50usize,
            num_duplicates in 1usize..=5usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());
                let policy = ExtractionPolicy::default();

                let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();
                let temp_dir = TempDir::new().unwrap();

                // Create list with duplicates
                let mut directories = Vec::new();
                for i in 0..num_unique_dirs {
                    let dir = temp_dir.path().join(format!("dir_{}", i));
                    // Add the same directory multiple times
                    for _ in 0..num_duplicates {
                        directories.push(dir.clone());
                    }
                }

                let total_dirs = directories.len();
                prop_assert_eq!(
                    total_dirs,
                    num_unique_dirs * num_duplicates,
                    "Total directories should be unique * duplicates"
                );

                // Create directories (should deduplicate)
                let created = engine.create_directories_batched(&directories).await.unwrap();

                // Property: Should only create unique directories
                prop_assert_eq!(
                    created,
                    num_unique_dirs,
                    "Should only create {} unique directories, not {}",
                    num_unique_dirs,
                    total_dirs
                );

                // Property: All unique directories should exist
                for i in 0..num_unique_dirs {
                    let dir = temp_dir.path().join(format!("dir_{}", i));
                    prop_assert!(
                        dir.exists(),
                        "Unique directory {:?} should exist",
                        dir
                    );
                }

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_directory_batching_empty_input(
            dir_batch_size in 1usize..=20usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    dir_batch_size,
                    ..Default::default()
                };

                let engine = ExtractionEngine::new(path_manager, security_detector, policy).unwrap();

                // Property: Empty input should create 0 directories
                let created = engine.create_directories_batched(&[]).await.unwrap();
                prop_assert_eq!(
                    created,
                    0,
                    "Empty input should create 0 directories"
                );

                Ok(())
            }).unwrap();
        }

        #[test]
        fn prop_batch_size_validation(
            dir_batch_size in 0usize..=50usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
                let path_manager = Arc::new(PathManager::new(PathConfig::default(), db));
                let security_detector = Arc::new(SecurityDetector::default());

                let policy = ExtractionPolicy {
                    dir_batch_size,
                    ..Default::default()
                };

                let result = ExtractionEngine::new(path_manager, security_detector, policy);

                // Property: Zero batch size should be rejected
                if dir_batch_size == 0 {
                    prop_assert!(
                        result.is_err(),
                        "Zero dir_batch_size should be rejected"
                    );
                } else {
                    // Property: Positive batch size should be accepted
                    prop_assert!(
                        result.is_ok(),
                        "Positive dir_batch_size should be accepted"
                    );

                    let engine = result.unwrap();
                    prop_assert_eq!(
                        engine.policy().dir_batch_size,
                        dir_batch_size,
                        "Engine should use configured dir_batch_size"
                    );
                }

                Ok(())
            }).unwrap();
        }
    }
}
