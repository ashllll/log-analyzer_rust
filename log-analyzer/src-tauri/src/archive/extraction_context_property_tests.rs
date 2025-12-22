//! Property-based tests for extraction context consistency
//!
//! These tests verify that ExtractionContext maintains consistency
//! across various operations and state transitions.

#[cfg(test)]
mod tests {
    use crate::archive::extraction_context::{ExtractionContext, ExtractionItem, ExtractionStack};
    use proptest::prelude::*;
    use std::path::PathBuf;

    /// **Feature: enhanced-archive-handling, Property 8: Extraction context consistency**
    /// **Validates: Requirements 2.4**
    ///
    /// Property: For any file being extracted, the ExtractionContext should accurately
    /// reflect the current depth, parent archive, and accumulated metrics.
    ///
    /// This property verifies that:
    /// 1. Child contexts correctly increment depth
    /// 2. Parent archive references are maintained
    /// 3. Accumulated metrics are preserved across context creation
    /// 4. Workspace ID remains consistent throughout the hierarchy
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_context_child_depth_increments(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            parent_path in "[a-zA-Z0-9_/-]{1,100}\\.zip",
            initial_size in 0u64..1_000_000_000u64,
            initial_files in 0usize..10_000usize,
        ) {
            // Create parent context with some accumulated metrics
            let mut parent_ctx = ExtractionContext::new(workspace_id.clone());
            parent_ctx.update_metrics(initial_size, initial_files);

            let parent_archive = PathBuf::from(&parent_path);

            // Create child context
            let child_ctx = parent_ctx.create_child(parent_archive.clone());

            // Verify depth increments by exactly 1
            prop_assert_eq!(child_ctx.current_depth, parent_ctx.current_depth + 1);

            // Verify parent archive is set correctly
            prop_assert_eq!(child_ctx.parent_archive, Some(parent_archive));

            // Verify workspace ID is preserved
            prop_assert_eq!(child_ctx.workspace_id, workspace_id);

            // Verify accumulated metrics are preserved
            prop_assert_eq!(child_ctx.accumulated_size, initial_size);
            prop_assert_eq!(child_ctx.accumulated_files, initial_files);
        }

        #[test]
        fn prop_context_metrics_accumulate_correctly(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            updates in prop::collection::vec((0u64..1_000_000u64, 0usize..100usize), 1..20),
        ) {
            let mut ctx = ExtractionContext::new(workspace_id);

            let mut expected_size = 0u64;
            let mut expected_files = 0usize;

            for (bytes, files) in updates {
                ctx.update_metrics(bytes, files);
                expected_size = expected_size.saturating_add(bytes);
                expected_files = expected_files.saturating_add(files);

                // Verify metrics match expected values after each update
                prop_assert_eq!(ctx.accumulated_size, expected_size);
                prop_assert_eq!(ctx.accumulated_files, expected_files);
            }
        }

        #[test]
        fn prop_context_depth_limit_check_is_consistent(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            current_depth in 0usize..30usize,
            max_depth in 1usize..20usize,
        ) {
            let mut ctx = ExtractionContext::new(workspace_id);
            ctx.current_depth = current_depth;

            let is_limit_reached = ctx.is_depth_limit_reached(max_depth);
            let expected = current_depth >= max_depth;

            prop_assert_eq!(is_limit_reached, expected);
        }

        #[test]
        fn prop_context_hierarchy_maintains_consistency(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            depth_levels in 1usize..10usize,
        ) {
            let mut ctx = ExtractionContext::new(workspace_id.clone());

            // Create a hierarchy of contexts
            for level in 0..depth_levels {
                let parent_archive = PathBuf::from(format!("/archive/level_{}.zip", level));
                let expected_depth = level + 1;

                ctx = ctx.create_child(parent_archive.clone());

                // Verify depth matches the level
                prop_assert_eq!(ctx.current_depth, expected_depth);

                // Verify parent archive is set
                prop_assert_eq!(ctx.parent_archive.as_ref(), Some(&parent_archive));

                // Verify workspace ID is preserved
                prop_assert_eq!(&ctx.workspace_id, &workspace_id);
            }

            // Final depth should equal depth_levels
            prop_assert_eq!(ctx.current_depth, depth_levels);
        }

        #[test]
        fn prop_extraction_item_preserves_context(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            archive_path in "[a-zA-Z0-9_/-]{1,100}\\.zip",
            target_dir in "[a-zA-Z0-9_/-]{1,100}",
            depth in 0usize..20usize,
            accumulated_size in 0u64..1_000_000_000u64,
            accumulated_files in 0usize..10_000usize,
        ) {
            let mut ctx = ExtractionContext::new(workspace_id.clone());
            ctx.current_depth = depth;
            ctx.update_metrics(accumulated_size, accumulated_files);

            let item = ExtractionItem::new(
                PathBuf::from(&archive_path),
                PathBuf::from(&target_dir),
                depth,
                ctx.clone(),
            );

            // Verify item preserves all context information
            prop_assert_eq!(item.depth, depth);
            prop_assert_eq!(item.archive_path, PathBuf::from(&archive_path));
            prop_assert_eq!(item.target_dir, PathBuf::from(&target_dir));
            prop_assert_eq!(item.parent_context.workspace_id, workspace_id);
            prop_assert_eq!(item.parent_context.current_depth, depth);
            prop_assert_eq!(item.parent_context.accumulated_size, accumulated_size);
            prop_assert_eq!(item.parent_context.accumulated_files, accumulated_files);
        }

        #[test]
        fn prop_stack_operations_maintain_lifo_order(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            items_count in 1usize..50usize,
        ) {
            let mut stack = ExtractionStack::new();
            let ctx = ExtractionContext::new(workspace_id);

            let mut expected_order = Vec::new();

            // Push items onto stack
            for i in 0..items_count {
                let item = ExtractionItem::new(
                    PathBuf::from(format!("/archive_{}.zip", i)),
                    PathBuf::from(format!("/target_{}", i)),
                    i,
                    ctx.clone(),
                );
                expected_order.push(i);
                stack.push(item).unwrap();
            }

            // Pop items and verify LIFO order
            expected_order.reverse();
            for expected_index in expected_order {
                let item = stack.pop().unwrap();
                prop_assert_eq!(item.depth, expected_index);
            }

            // Stack should be empty
            prop_assert!(stack.is_empty());
        }

        #[test]
        fn prop_stack_respects_size_limit(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            max_size in 1usize..100usize,
            push_count in 1usize..200usize,
        ) {
            let mut stack = ExtractionStack::with_max_size(max_size);
            let ctx = ExtractionContext::new(workspace_id);

            let mut successful_pushes = 0;

            for i in 0..push_count {
                let item = ExtractionItem::new(
                    PathBuf::from(format!("/archive_{}.zip", i)),
                    PathBuf::from(format!("/target_{}", i)),
                    i,
                    ctx.clone(),
                );

                let result = stack.push(item);

                if i < max_size {
                    // Should succeed within limit
                    prop_assert!(result.is_ok());
                    successful_pushes += 1;
                } else {
                    // Should fail beyond limit
                    prop_assert!(result.is_err());
                }
            }

            // Verify stack size equals successful pushes
            prop_assert_eq!(stack.len(), successful_pushes);
            prop_assert_eq!(stack.len(), max_size.min(push_count));
        }

        #[test]
        fn prop_context_elapsed_time_is_monotonic(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
        ) {
            let ctx = ExtractionContext::new(workspace_id);

            let elapsed1 = ctx.elapsed();
            std::thread::sleep(std::time::Duration::from_millis(10));
            let elapsed2 = ctx.elapsed();

            // Second measurement should be greater than or equal to first
            prop_assert!(elapsed2 >= elapsed1);
        }

        #[test]
        fn prop_context_metrics_never_overflow(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            // Use large values near u64::MAX to test saturation
            size1 in (u64::MAX - 1_000_000)..u64::MAX,
            size2 in 1u64..1_000_000u64,
            files1 in (usize::MAX - 1_000)..usize::MAX,
            files2 in 1usize..1_000usize,
        ) {
            let mut ctx = ExtractionContext::new(workspace_id);

            // Update with large values
            ctx.update_metrics(size1, files1);

            // Update again (should saturate, not overflow)
            ctx.update_metrics(size2, files2);

            // Verify no panic occurred and values are at or near max
            prop_assert!(ctx.accumulated_size >= size1);
            prop_assert!(ctx.accumulated_files >= files1);
        }
    }
}
