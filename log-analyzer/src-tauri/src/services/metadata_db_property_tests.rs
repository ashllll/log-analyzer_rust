/**
 * Property-based tests for MetadataDB
 *
 * These tests verify correctness properties that should hold across all valid inputs
 * using the proptest framework for property-based testing.
 */
use super::metadata_db::MetadataDB;
use proptest::prelude::*;
use std::collections::HashSet;

/// **Feature: enhanced-archive-handling, Property 19: Mapping persistence**
///
/// For any shortened path, the mapping should be retrievable from the SQLite database
/// even after system restart.
///
/// **Validates: Requirements 4.5**
#[cfg(test)]
mod mapping_persistence_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 19: Mapping persistence
        ///
        /// For any valid workspace_id, short_path, and original_path:
        /// 1. Store the mapping in the database
        /// 2. Close and reopen the database connection (simulating restart)
        /// 3. The mapping should still be retrievable
        /// 4. Both forward (short -> original) and reverse (original -> short) lookups should work
        #[test]
        fn prop_mapping_persists_across_restarts(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            short_path in "[a-zA-Z0-9_/-]{1,100}",
            original_path in "[a-zA-Z0-9_/-]{1,500}",
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Use a temporary file for the database to test persistence
                let temp_dir = tempfile::tempdir().unwrap();
                let db_path = temp_dir.path().join("test.db");
                let db_path_str = db_path.to_str().unwrap();

                // Phase 1: Store mapping
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();
                    db.store_mapping(&workspace_id, &short_path, &original_path)
                        .await
                        .unwrap();
                    db.close().await;
                }

                // Phase 2: Reopen database and verify persistence
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();

                    // Forward lookup: short -> original
                    let retrieved_original = db
                        .get_original_path(&workspace_id, &short_path)
                        .await
                        .unwrap();
                    prop_assert_eq!(retrieved_original, Some(original_path.clone()));

                    // Reverse lookup: original -> short
                    let retrieved_short = db
                        .get_short_path(&workspace_id, &original_path)
                        .await
                        .unwrap();
                    prop_assert_eq!(retrieved_short, Some(short_path.clone()));

                    db.close().await;
                }

                Ok(())
            })?;
        }

        /// Property: Multiple mappings persist independently
        ///
        /// For any set of distinct mappings, all should persist independently
        /// and be retrievable after restart.
        #[test]
        fn prop_multiple_mappings_persist_independently(
            mappings in prop::collection::vec(
                (
                    "[a-zA-Z0-9_-]{1,50}",  // workspace_id
                    "[a-zA-Z0-9_/-]{1,100}", // short_path
                    "[a-zA-Z0-9_/-]{1,500}", // original_path
                ),
                1..10
            )
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = tempfile::tempdir().unwrap();
                let db_path = temp_dir.path().join("test.db");
                let db_path_str = db_path.to_str().unwrap();

                // Deduplicate mappings by (workspace_id, short_path) to avoid conflicts
                let mut unique_mappings = Vec::new();
                let mut seen = HashSet::new();
                for (ws, sp, op) in mappings {
                    let key = (ws.clone(), sp.clone());
                    if !seen.contains(&key) {
                        seen.insert(key);
                        unique_mappings.push((ws, sp, op));
                    }
                }

                // Phase 1: Store all mappings
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();
                    for (workspace_id, short_path, original_path) in &unique_mappings {
                        db.store_mapping(workspace_id, short_path, original_path)
                            .await
                            .unwrap();
                    }
                    db.close().await;
                }

                // Phase 2: Verify all mappings persist
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();
                    for (workspace_id, short_path, original_path) in &unique_mappings {
                        let retrieved = db
                            .get_original_path(workspace_id, short_path)
                            .await
                            .unwrap();
                        prop_assert_eq!(retrieved, Some(original_path.clone()));
                    }
                    db.close().await;
                }

                Ok(())
            })?;
        }

        /// Property: Access count persists across restarts
        ///
        /// For any mapping, incrementing the access count should persist
        /// even after database restart.
        #[test]
        fn prop_access_count_persists(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            short_path in "[a-zA-Z0-9_/-]{1,100}",
            original_path in "[a-zA-Z0-9_/-]{1,500}",
            increment_count in 1..20usize,
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = tempfile::tempdir().unwrap();
                let db_path = temp_dir.path().join("test.db");
                let db_path_str = db_path.to_str().unwrap();

                // Phase 1: Store mapping and increment access count
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();
                    db.store_mapping(&workspace_id, &short_path, &original_path)
                        .await
                        .unwrap();

                    for _ in 0..increment_count {
                        db.increment_access_count(&workspace_id, &short_path)
                            .await
                            .unwrap();
                    }
                    db.close().await;
                }

                // Phase 2: Verify access count persists
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();
                    let mappings = db.get_workspace_mappings(&workspace_id).await.unwrap();

                    prop_assert_eq!(mappings.len(), 1);
                    prop_assert_eq!(mappings[0].access_count, increment_count as i64);

                    db.close().await;
                }

                Ok(())
            })?;
        }

        /// Property: Workspace cleanup is persistent
        ///
        /// For any workspace, after cleanup, no mappings should be retrievable
        /// even after database restart.
        #[test]
        fn prop_workspace_cleanup_is_persistent(
            workspace_id in "[a-zA-Z0-9_-]{1,50}",
            mappings in prop::collection::vec(
                (
                    "[a-zA-Z0-9_/-]{1,100}", // short_path
                    "[a-zA-Z0-9_/-]{1,500}", // original_path
                ),
                1..10
            )
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let temp_dir = tempfile::tempdir().unwrap();
                let db_path = temp_dir.path().join("test.db");
                let db_path_str = db_path.to_str().unwrap();

                // Deduplicate by short_path
                let mut unique_mappings = Vec::new();
                let mut seen = HashSet::new();
                for (sp, op) in mappings {
                    if !seen.contains(&sp) {
                        seen.insert(sp.clone());
                        unique_mappings.push((sp, op));
                    }
                }

                // Phase 1: Store mappings and cleanup
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();

                    for (short_path, original_path) in &unique_mappings {
                        db.store_mapping(&workspace_id, short_path, original_path)
                            .await
                            .unwrap();
                    }

                    let deleted = db.cleanup_workspace(&workspace_id).await.unwrap();
                    prop_assert_eq!(deleted, unique_mappings.len());

                    db.close().await;
                }

                // Phase 2: Verify cleanup persists
                {
                    let db = MetadataDB::new(db_path_str).await.unwrap();

                    for (short_path, _) in &unique_mappings {
                        let retrieved = db
                            .get_original_path(&workspace_id, short_path)
                            .await
                            .unwrap();
                        prop_assert_eq!(retrieved, None);
                    }

                    let mappings = db.get_workspace_mappings(&workspace_id).await.unwrap();
                    prop_assert_eq!(mappings.len(), 0);

                    db.close().await;
                }

                Ok(())
            })?;
        }
    }
}
