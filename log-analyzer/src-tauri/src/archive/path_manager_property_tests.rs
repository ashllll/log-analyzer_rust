/**
 * Property-Based Tests for PathManager
 *
 * Tests correctness properties using proptest framework.
 */
use super::path_manager::{HashAlgorithm, PathConfig, PathManager};
use crate::services::MetadataDB;
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Create a test PathManager with in-memory database
async fn create_test_manager(config: PathConfig) -> PathManager {
    let db = Arc::new(MetadataDB::new(":memory:").await.unwrap());
    PathManager::new(config, db)
}

/// Strategy for generating valid path components
fn path_component_strategy() -> impl Strategy<Value = String> {
    // Generate strings with 256-500 characters (exceeding typical limits)
    prop::string::string_regex("[a-zA-Z0-9_-]{256,500}").unwrap()
}

/// Strategy for generating workspace IDs
fn workspace_id_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z0-9_]{8,32}").unwrap()
}

/// **Feature: enhanced-archive-handling, Property 4: Path mapping round-trip**
/// **Validates: Requirements 1.4**
///
/// For any path that undergoes shortening, retrieving the original path from
/// the shortened path should return the exact original path (bidirectional mapping integrity).
#[cfg(test)]
mod property_4_path_mapping_round_trip {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_path_mapping_round_trip(
            original_component in path_component_strategy(),
            workspace_id in workspace_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // Create config that will trigger shortening
                let config = PathConfig {
                    max_path_length: 100,
                    shortening_threshold: 0.8,
                    enable_long_paths: false,
                    hash_algorithm: HashAlgorithm::SHA256,
                    hash_length: 16,
                };

                let manager = create_test_manager(config).await;

                // Create a path that will need shortening
                let original_path = PathBuf::from(&original_component);

                // Shorten the path
                let short_path = manager.resolve_extraction_path(
                    &workspace_id,
                    &original_path
                ).await.unwrap();

                // Retrieve the original path
                let retrieved = manager.resolve_original_path(
                    &workspace_id,
                    &short_path
                ).await.unwrap();

                // Property: Round-trip should preserve the original path
                let retrieved_str = retrieved.to_string_lossy().to_string();
                let original_str = original_path.to_string_lossy().to_string();
                prop_assert_eq!(
                    retrieved_str,
                    original_str,
                    "Round-trip failed: original={}, retrieved={}",
                    original_path.display(),
                    retrieved.display()
                );

                Ok(())
            });
            result.unwrap();
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 2: Windows UNC prefix application**
/// **Validates: Requirements 1.2**
///
/// For any path on Windows exceeding 260 characters, the system should automatically
/// prepend the UNC prefix (\\?\) to enable long path support.
#[cfg(test)]
#[cfg(target_os = "windows")]
mod property_2_windows_unc_prefix {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_windows_unc_prefix_application(
            path_length in 261usize..500usize,
            workspace_id in workspace_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // Create config with long path support enabled
                let config = PathConfig {
                    max_path_length: 32767,
                    shortening_threshold: 0.8,
                    enable_long_paths: true,
                    hash_algorithm: HashAlgorithm::SHA256,
                    hash_length: 16,
                };

                let manager = create_test_manager(config).await;

                // Create a path exceeding 260 characters
                let long_component = "a".repeat(path_length);
                let original_path = PathBuf::from(format!("C:\\{}", long_component));

                // Resolve the path
                let resolved = manager.resolve_extraction_path(
                    &workspace_id,
                    &original_path
                ).await.unwrap();

                let resolved_str = resolved.to_string_lossy();

                // Property: Path exceeding 260 chars should have UNC prefix
                prop_assert!(
                    resolved_str.starts_with(r"\\?\"),
                    "Path exceeding 260 characters should have UNC prefix: {}",
                    resolved_str
                );

                Ok(())
            });
            result.unwrap();
        }

        #[test]
        fn prop_short_paths_no_unc_prefix(
            path_length in 10usize..250usize,
            workspace_id in workspace_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // Create config with long path support enabled
                let config = PathConfig {
                    max_path_length: 32767,
                    shortening_threshold: 0.8,
                    enable_long_paths: true,
                    hash_algorithm: HashAlgorithm::SHA256,
                    hash_length: 16,
                };

                let manager = create_test_manager(config).await;

                // Create a path under 260 characters (including "C:\")
                let short_component = "a".repeat(path_length);
                let original_path = PathBuf::from(format!("C:\\{}", short_component));

                // Resolve the path
                let resolved = manager.resolve_extraction_path(
                    &workspace_id,
                    &original_path
                ).await.unwrap();

                let resolved_str = resolved.to_string_lossy();

                // Property: Path under 260 chars should NOT have UNC prefix
                prop_assert!(
                    !resolved_str.starts_with(r"\\?\"),
                    "Path under 260 characters should not have UNC prefix: {}",
                    resolved_str
                );

                Ok(())
            });
            result.unwrap();
        }
    }
}

/// **Feature: enhanced-archive-handling, Property 3: Path shortening consistency**
/// **Validates: Requirements 1.3**
///
/// For any filename exceeding the OS limit, applying the path shortening strategy
/// twice should produce the same shortened path (idempotent hashing).
#[cfg(test)]
mod property_3_path_shortening_idempotence {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_path_shortening_idempotence(
            original_component in path_component_strategy(),
            workspace_id in workspace_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // Create config that will trigger shortening
                let config = PathConfig {
                    max_path_length: 100,
                    shortening_threshold: 0.8,
                    enable_long_paths: false,
                    hash_algorithm: HashAlgorithm::SHA256,
                    hash_length: 16,
                };

                let manager = create_test_manager(config).await;

                // Create a path that will need shortening
                let original_path = PathBuf::from(&original_component);

                // Apply shortening twice
                let short_path_1 = manager.resolve_extraction_path(
                    &workspace_id,
                    &original_path
                ).await.unwrap();

                let short_path_2 = manager.resolve_extraction_path(
                    &workspace_id,
                    &original_path
                ).await.unwrap();

                // Property: Shortening should be idempotent
                let short_str_1 = short_path_1.to_string_lossy().to_string();
                let short_str_2 = short_path_2.to_string_lossy().to_string();
                prop_assert_eq!(
                    short_str_1,
                    short_str_2,
                    "Path shortening is not idempotent: first={}, second={}",
                    short_path_1.display(),
                    short_path_2.display()
                );

                Ok(())
            });
            result.unwrap();
        }

        #[test]
        fn prop_hash_determinism(
            component in prop::string::string_regex("[a-zA-Z0-9_-]{100,200}").unwrap()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                let config = PathConfig {
                    max_path_length: 100,
                    shortening_threshold: 0.8,
                    enable_long_paths: false,
                    hash_algorithm: HashAlgorithm::SHA256,
                    hash_length: 16,
                };

                let manager = create_test_manager(config).await;

                // Hash the same component multiple times
                let hash1 = manager.hash_path_component(&component);
                let hash2 = manager.hash_path_component(&component);
                let hash3 = manager.hash_path_component(&component);

                // Property: Hash should be deterministic
                prop_assert_eq!(&hash1, &hash2, "Hash is not deterministic");
                prop_assert_eq!(&hash2, &hash3, "Hash is not deterministic");

                // Property: Hash should be the configured length
                prop_assert_eq!(
                    hash1.len(),
                    16,
                    "Hash length should be 16 characters, got {}",
                    hash1.len()
                );

                Ok(())
            });
            result.unwrap();
        }
    }
}

#[cfg(test)]
mod additional_properties {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_path_length_prediction_accuracy(
            base_len in 10usize..50usize,
            archive_len in 5usize..20usize,
            internal_len in 10usize..100usize,
            depth in 0usize..10usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                let config = PathConfig::os_default();
                let manager = create_test_manager(config).await;

                let base_path = PathBuf::from("a".repeat(base_len));
                let archive_name = "a".repeat(archive_len);
                let internal_path = "a".repeat(internal_len);

                let predicted = manager.predict_path_length(
                    &base_path,
                    &archive_name,
                    &internal_path,
                    depth
                );

                // Calculate actual length
                let actual_path = format!(
                    "{}/{}/{}",
                    base_path.display(),
                    archive_name,
                    internal_path
                );
                let actual_len = actual_path.len() + depth;

                // Property: Prediction should be within ±5 characters
                let diff = if predicted > actual_len {
                    predicted - actual_len
                } else {
                    actual_len - predicted
                };

                prop_assert!(
                    diff <= 5,
                    "Path length prediction off by {} characters (predicted={}, actual={})",
                    diff,
                    predicted,
                    actual_len
                );

                Ok(())
            });
            result.unwrap();
        }
    }
}
