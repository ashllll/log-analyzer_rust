//! Property-Based Tests for PolicyManager
//!
//! These tests use proptest to verify universal properties that should hold
//! across all valid and invalid configurations.

use super::extraction_policy::*;
use super::policy_manager::PolicyManager;
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Generate valid extraction configurations
fn valid_extraction_config() -> impl Strategy<Value = ExtractionConfig> {
    (
        1usize..=20,            // max_depth: 1-20
        1u64..=1_000_000_000,   // max_file_size: 1B to 1GB
        1u64..=100_000_000_000, // max_total_size: 1B to 100GB
        1u64..=500_000_000_000, // max_workspace_size: 1B to 500GB
        0usize..=16,            // concurrent_extractions: 0-16
        1024usize..=1_048_576,  // buffer_size: 1KB to 1MB
    )
        .prop_map(
            |(
                max_depth,
                max_file_size,
                max_total_size,
                max_workspace_size,
                concurrent_extractions,
                buffer_size,
            )| {
                ExtractionConfig {
                    max_depth,
                    max_file_size,
                    max_total_size,
                    max_workspace_size,
                    concurrent_extractions,
                    buffer_size,
                }
            },
        )
}

/// Generate valid security configurations
fn valid_security_config() -> impl Strategy<Value = SecurityConfig> {
    (
        1.0f64..=10000.0,       // compression_ratio_threshold
        1.0f64..=100_000_000.0, // exponential_backoff_threshold
        any::<bool>(),          // enable_zip_bomb_detection
    )
        .prop_map(
            |(
                compression_ratio_threshold,
                exponential_backoff_threshold,
                enable_zip_bomb_detection,
            )| {
                SecurityConfig {
                    compression_ratio_threshold,
                    exponential_backoff_threshold,
                    enable_zip_bomb_detection,
                }
            },
        )
}

/// Generate valid paths configurations
fn valid_paths_config() -> impl Strategy<Value = PathsConfig> {
    (
        any::<bool>(), // enable_long_paths
        0.1f32..=1.0,  // shortening_threshold: 0.1-1.0
        prop::sample::select(vec!["SHA256".to_string(), "SHA512".to_string()]),
        8usize..=32, // hash_length: 8-32
    )
        .prop_map(
            |(enable_long_paths, shortening_threshold, hash_algorithm, hash_length)| PathsConfig {
                enable_long_paths,
                shortening_threshold,
                hash_algorithm,
                hash_length,
            },
        )
}

/// Generate valid performance configurations
fn valid_performance_config() -> impl Strategy<Value = PerformanceConfig> {
    (
        1u64..=168,    // temp_dir_ttl_hours: 1-168 (1 week)
        1usize..=365,  // log_retention_days: 1-365
        any::<bool>(), // enable_streaming
        1usize..=100,  // directory_batch_size: 1-100
        1usize..=8,    // parallel_files_per_archive: 1-8
    )
        .prop_map(
            |(
                temp_dir_ttl_hours,
                log_retention_days,
                enable_streaming,
                directory_batch_size,
                parallel_files_per_archive,
            )| {
                PerformanceConfig {
                    temp_dir_ttl_hours,
                    log_retention_days,
                    enable_streaming,
                    directory_batch_size,
                    parallel_files_per_archive,
                }
            },
        )
}

/// Generate valid audit configurations
fn valid_audit_config() -> impl Strategy<Value = AuditConfig> {
    (
        any::<bool>(), // enable_audit_logging
        prop::sample::select(vec!["json".to_string(), "text".to_string()]),
        prop::sample::select(vec![
            "trace".to_string(),
            "debug".to_string(),
            "info".to_string(),
            "warn".to_string(),
            "error".to_string(),
        ]),
        any::<bool>(), // log_security_events
    )
        .prop_map(
            |(enable_audit_logging, log_format, log_level, log_security_events)| AuditConfig {
                enable_audit_logging,
                log_format,
                log_level,
                log_security_events,
            },
        )
}

/// Generate valid extraction policies
fn valid_extraction_policy() -> impl Strategy<Value = ExtractionPolicy> {
    (
        valid_extraction_config(),
        valid_security_config(),
        valid_paths_config(),
        valid_performance_config(),
        valid_audit_config(),
    )
        .prop_map(
            |(extraction, security, paths, performance, audit)| ExtractionPolicy {
                extraction,
                security,
                paths,
                performance,
                audit,
            },
        )
}

/// Generate invalid extraction configurations (violating constraints)
fn invalid_extraction_config() -> impl Strategy<Value = ExtractionPolicy> {
    prop_oneof![
        // Invalid max_depth (0 or > 20)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.max_depth = 0;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.max_depth = 21;
            policy
        }),
        // Invalid max_file_size (0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.max_file_size = 0;
            policy
        }),
        // Invalid max_total_size (0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.max_total_size = 0;
            policy
        }),
        // Invalid max_workspace_size (0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.max_workspace_size = 0;
            policy
        }),
        // Invalid buffer_size (0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.extraction.buffer_size = 0;
            policy
        }),
    ]
}

/// Generate invalid paths configurations
fn invalid_paths_config() -> impl Strategy<Value = ExtractionPolicy> {
    prop_oneof![
        // Invalid shortening_threshold (<= 0.0 or > 1.0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.paths.shortening_threshold = 0.0;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.paths.shortening_threshold = 1.5;
            policy
        }),
        // Invalid hash_length (< 8 or > 32)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.paths.hash_length = 7;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.paths.hash_length = 33;
            policy
        }),
        // Invalid hash_algorithm
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.paths.hash_algorithm = "MD5".to_string();
            policy
        }),
    ]
}

/// Generate invalid audit configurations
fn invalid_audit_config() -> impl Strategy<Value = ExtractionPolicy> {
    prop_oneof![
        // Invalid log_format
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.audit.log_format = "xml".to_string();
            policy
        }),
        // Invalid log_level
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.audit.log_level = "critical".to_string();
            policy
        }),
    ]
}

/// Generate invalid performance configurations
fn invalid_performance_config() -> impl Strategy<Value = ExtractionPolicy> {
    prop_oneof![
        // Invalid directory_batch_size (0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.performance.directory_batch_size = 0;
            policy
        }),
        // Invalid parallel_files_per_archive (0 or > 8)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.performance.parallel_files_per_archive = 0;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.performance.parallel_files_per_archive = 9;
            policy
        }),
    ]
}

/// Generate invalid security configurations
fn invalid_security_config() -> impl Strategy<Value = ExtractionPolicy> {
    prop_oneof![
        // Invalid compression_ratio_threshold (<= 0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.security.compression_ratio_threshold = 0.0;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.security.compression_ratio_threshold = -1.0;
            policy
        }),
        // Invalid exponential_backoff_threshold (<= 0)
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.security.exponential_backoff_threshold = 0.0;
            policy
        }),
        Just({
            let mut policy = ExtractionPolicy::default();
            policy.security.exponential_backoff_threshold = -1.0;
            policy
        }),
    ]
}

proptest! {
    /// **Feature: enhanced-archive-handling, Property 26: Configuration validation enforcement**
    /// **Validates: Requirements 6.3**
    ///
    /// Property: For any valid configuration, the PolicyManager should accept it
    /// and validation should succeed.
    ///
    /// This property ensures that:
    /// 1. All valid configurations pass validation
    /// 2. Valid configurations can be loaded and applied
    /// 3. The validation logic correctly identifies valid configurations
    #[test]
    fn prop_valid_configurations_pass_validation(
        policy in valid_extraction_policy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Validation should succeed for valid policies
            let validation_result = manager.validate_policy(&policy);
            prop_assert!(
                validation_result.is_ok(),
                "Valid policy should pass validation: {:?}",
                validation_result.err()
            );

            // Should be able to update with valid policy
            let update_result = manager.update_policy(policy.clone()).await;
            prop_assert!(
                update_result.is_ok(),
                "Should be able to update with valid policy: {:?}",
                update_result.err()
            );

            // Retrieved policy should match
            let retrieved = manager.get_policy().await;
            prop_assert_eq!(retrieved.extraction.max_depth, policy.extraction.max_depth);
            prop_assert_eq!(retrieved.extraction.max_file_size, policy.extraction.max_file_size);

            Ok(())
        })?;
    }

    /// **Feature: enhanced-archive-handling, Property 27: Invalid configuration rejection**
    /// **Validates: Requirements 6.4**
    ///
    /// Property: For any invalid configuration, the PolicyManager should reject it
    /// and the current policy should remain unchanged.
    ///
    /// This property ensures that:
    /// 1. Invalid configurations are detected and rejected
    /// 2. The current policy is not modified when validation fails
    /// 3. Error messages are provided for invalid configurations
    #[test]
    fn prop_invalid_extraction_config_rejected(
        invalid_policy in invalid_extraction_config()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Store initial policy
            let initial_policy = manager.get_policy().await;
            let initial_depth = initial_policy.extraction.max_depth;

            // Validation should fail
            let validation_result = manager.validate_policy(&invalid_policy);
            prop_assert!(
                validation_result.is_err(),
                "Invalid extraction config should fail validation"
            );

            // Update should fail
            let update_result = manager.update_policy(invalid_policy).await;
            prop_assert!(
                update_result.is_err(),
                "Should not be able to update with invalid policy"
            );

            // Current policy should remain unchanged
            let current_policy = manager.get_policy().await;
            prop_assert_eq!(
                current_policy.extraction.max_depth,
                initial_depth,
                "Policy should remain unchanged after failed update"
            );

            Ok(())
        })?;
    }

    #[test]
    fn prop_invalid_paths_config_rejected(
        invalid_policy in invalid_paths_config()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Store initial policy
            let initial_policy = manager.get_policy().await;

            // Validation should fail
            let validation_result = manager.validate_policy(&invalid_policy);
            prop_assert!(
                validation_result.is_err(),
                "Invalid paths config should fail validation"
            );

            // Update should fail
            let update_result = manager.update_policy(invalid_policy).await;
            prop_assert!(
                update_result.is_err(),
                "Should not be able to update with invalid policy"
            );

            // Current policy should remain unchanged
            let current_policy = manager.get_policy().await;
            prop_assert_eq!(
                current_policy.paths.hash_length,
                initial_policy.paths.hash_length,
                "Policy should remain unchanged after failed update"
            );

            Ok(())
        })?;
    }

    #[test]
    fn prop_invalid_audit_config_rejected(
        invalid_policy in invalid_audit_config()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Store initial policy
            let initial_policy = manager.get_policy().await;

            // Validation should fail
            let validation_result = manager.validate_policy(&invalid_policy);
            prop_assert!(
                validation_result.is_err(),
                "Invalid audit config should fail validation"
            );

            // Update should fail
            let update_result = manager.update_policy(invalid_policy).await;
            prop_assert!(
                update_result.is_err(),
                "Should not be able to update with invalid policy"
            );

            // Current policy should remain unchanged
            let current_policy = manager.get_policy().await;
            prop_assert_eq!(
                current_policy.audit.log_format,
                initial_policy.audit.log_format,
                "Policy should remain unchanged after failed update"
            );

            Ok(())
        })?;
    }

    #[test]
    fn prop_invalid_performance_config_rejected(
        invalid_policy in invalid_performance_config()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Store initial policy
            let initial_policy = manager.get_policy().await;

            // Validation should fail
            let validation_result = manager.validate_policy(&invalid_policy);
            prop_assert!(
                validation_result.is_err(),
                "Invalid performance config should fail validation"
            );

            // Update should fail
            let update_result = manager.update_policy(invalid_policy).await;
            prop_assert!(
                update_result.is_err(),
                "Should not be able to update with invalid policy"
            );

            // Current policy should remain unchanged
            let current_policy = manager.get_policy().await;
            prop_assert_eq!(
                current_policy.performance.directory_batch_size,
                initial_policy.performance.directory_batch_size,
                "Policy should remain unchanged after failed update"
            );

            Ok(())
        })?;
    }

    #[test]
    fn prop_invalid_security_config_rejected(
        invalid_policy in invalid_security_config()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = PolicyManager::new(PathBuf::from("test_config.toml"));

            // Store initial policy
            let initial_policy = manager.get_policy().await;

            // Validation should fail
            let validation_result = manager.validate_policy(&invalid_policy);
            prop_assert!(
                validation_result.is_err(),
                "Invalid security config should fail validation"
            );

            // Update should fail
            let update_result = manager.update_policy(invalid_policy).await;
            prop_assert!(
                update_result.is_err(),
                "Should not be able to update with invalid policy"
            );

            // Current policy should remain unchanged
            let current_policy = manager.get_policy().await;
            prop_assert_eq!(
                current_policy.security.compression_ratio_threshold,
                initial_policy.security.compression_ratio_threshold,
                "Policy should remain unchanged after failed update"
            );

            Ok(())
        })?;
    }
}

/// Additional property tests for TOML parsing (task 8.3)
#[cfg(test)]
mod toml_parsing_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_valid_toml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("valid_config.toml");

        let toml_content = r#"
            [extraction]
            max_depth = 15
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            
            [security]
            compression_ratio_threshold = 100.0
            exponential_backoff_threshold = 1000000.0
            enable_zip_bomb_detection = true
            
            [paths]
            enable_long_paths = true
            shortening_threshold = 0.8
            hash_algorithm = "SHA256"
            hash_length = 16
            
            [performance]
            temp_dir_ttl_hours = 24
            log_retention_days = 90
            enable_streaming = true
            directory_batch_size = 10
            parallel_files_per_archive = 4
            
            [audit]
            enable_audit_logging = true
            log_format = "json"
            log_level = "info"
            log_security_events = true
        "#;

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        let policy = manager.load_policy().await.unwrap();

        assert_eq!(policy.extraction.max_depth, 15);
        assert_eq!(policy.extraction.concurrent_extractions, 4);
        assert_eq!(policy.security.compression_ratio_threshold, 100.0);
        assert_eq!(policy.paths.hash_algorithm, "SHA256");
        assert_eq!(policy.performance.directory_batch_size, 10);
        assert_eq!(policy.audit.log_format, "json");
    }

    #[tokio::test]
    async fn test_parse_invalid_toml_format() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_format.toml");

        // Invalid TOML syntax
        fs::write(&config_path, "this is not valid toml {{{")
            .await
            .unwrap();

        let manager = PolicyManager::new(config_path);
        let result = manager.load_policy().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_missing_required_fields() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("missing_fields.toml");

        // Missing required fields
        let toml_content = r#"
            [extraction]
            max_depth = 15
            # Missing other required fields
        "#;

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        let result = manager.load_policy().await;

        // Should fail due to missing fields
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_type_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("type_mismatch.toml");

        // Type mismatch: max_depth should be integer, not string
        let toml_content = r#"
            [extraction]
            max_depth = "fifteen"
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            
            [security]
            compression_ratio_threshold = 100.0
            exponential_backoff_threshold = 1000000.0
            enable_zip_bomb_detection = true
            
            [paths]
            enable_long_paths = true
            shortening_threshold = 0.8
            hash_algorithm = "SHA256"
            hash_length = 16
            
            [performance]
            temp_dir_ttl_hours = 24
            log_retention_days = 90
            enable_streaming = true
            directory_batch_size = 10
            parallel_files_per_archive = 4
            
            [audit]
            enable_audit_logging = true
            log_format = "json"
            log_level = "info"
            log_security_events = true
        "#;

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        let result = manager.load_policy().await;

        // Should fail due to type mismatch
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_with_extra_fields() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("extra_fields.toml");

        // Valid config with extra fields (should be ignored)
        let toml_content = r#"
            [extraction]
            max_depth = 15
            max_file_size = 104857600
            max_total_size = 10737418240
            max_workspace_size = 53687091200
            concurrent_extractions = 4
            buffer_size = 65536
            extra_field = "ignored"
            
            [security]
            compression_ratio_threshold = 100.0
            exponential_backoff_threshold = 1000000.0
            enable_zip_bomb_detection = true
            
            [paths]
            enable_long_paths = true
            shortening_threshold = 0.8
            hash_algorithm = "SHA256"
            hash_length = 16
            
            [performance]
            temp_dir_ttl_hours = 24
            log_retention_days = 90
            enable_streaming = true
            directory_batch_size = 10
            parallel_files_per_archive = 4
            
            [audit]
            enable_audit_logging = true
            log_format = "json"
            log_level = "info"
            log_security_events = true
        "#;

        fs::write(&config_path, toml_content).await.unwrap();

        let manager = PolicyManager::new(config_path);
        let policy = manager.load_policy().await.unwrap();

        // Should parse successfully, ignoring extra fields
        assert_eq!(policy.extraction.max_depth, 15);
    }
}
