//! Integration tests for error handling consistency
//!
//! **Property 2: Error Type Consistency**
//! **Validates: Requirements 1.2, 2.1**
//!
//! Tests that validation functions return the correct Result type with appropriate error variants

use log_analyzer::models::validated::{validate_search_query, validate_workspace_config};
use std::collections::HashMap;
use tempfile::TempDir;

// Import the validation functions we need to test
use log_analyzer::models::{ValidatedSearchQuery, ValidatedWorkspaceConfig, ValidationResult};

/// Test helper to create a temporary directory for testing
fn create_test_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

/// Test helper to create a valid workspace config for testing
fn create_valid_workspace_config(temp_dir: &TempDir) -> ValidatedWorkspaceConfig {
    ValidatedWorkspaceConfig {
        workspace_id: "test-workspace-123".to_string(),
        name: "Test Workspace".to_string(),
        description: Some("A test workspace for integration testing".to_string()),
        path: temp_dir.path().to_string_lossy().to_string(),
        max_file_size: 1_000_000, // 1MB
        max_file_count: 1000,
        enable_watch: false,
        tags: vec!["test".to_string(), "integration".to_string()],
        contact_email: Some("test@example.com".to_string()),
        project_url: Some("https://example.com".to_string()),
        metadata: HashMap::new(),
    }
}

/// Test helper to create a valid search query for testing
fn create_valid_search_query() -> ValidatedSearchQuery {
    ValidatedSearchQuery {
        query: "test search query".to_string(),
        workspace_id: "test-workspace-123".to_string(),
        max_results: Some(100),
        priority: Some(1),
        timeout_seconds: Some(30),
        case_sensitive: false,
        use_regex: false,
        file_pattern: Some("*.log".to_string()),
        time_start: None,
        time_end: None,
        log_levels: vec!["INFO".to_string(), "ERROR".to_string()],
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use log_analyzer::models::validated::{validate_search_query, validate_workspace_config};

    /// **Property 2: Error Type Consistency**
    /// For any validation function call with invalid input,
    /// the function should return the correct Result type with appropriate error variant
    #[test]
    fn test_workspace_config_validation_error_consistency() {
        // Test case 1: Invalid workspace ID with special characters
        let mut invalid_config = ValidatedWorkspaceConfig {
            workspace_id: "invalid/workspace/../id".to_string(), // Contains path traversal
            name: "Test".to_string(),
            description: None,
            path: "/nonexistent/path".to_string(),
            max_file_size: 1000,
            max_file_count: 10,
            enable_watch: false,
            tags: vec![],
            contact_email: Some("test@example.com".to_string()),
            project_url: Some("https://example.com".to_string()),
            metadata: HashMap::new(),
        };

        let result = validate_workspace_config(&invalid_config);

        // Verify the result is a ValidationResult<()> type
        assert!(
            !result.is_valid(),
            "Expected validation to fail for invalid workspace ID"
        );
        assert!(
            !result.errors.is_empty(),
            "Expected validation errors to be present"
        );

        // Verify error message contains information about workspace ID format
        let has_workspace_id_error = result.errors.iter().any(|error| {
            error.contains("workspace_id") || error.contains("Invalid workspace ID format")
        });
        assert!(
            has_workspace_id_error,
            "Expected workspace ID validation error, got: {:?}",
            result.errors
        );

        // Test case 2: Empty workspace ID
        invalid_config.workspace_id = "".to_string();
        let result = validate_workspace_config(&invalid_config);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for empty workspace ID"
        );
        let has_length_error = result.errors.iter().any(|error| {
            error.contains("workspace_id")
                && (error.contains("length") || error.contains("1-100 characters"))
        });
        assert!(
            has_length_error,
            "Expected workspace ID length validation error, got: {:?}",
            result.errors
        );

        // Test case 3: Workspace ID too long
        invalid_config.workspace_id = "a".repeat(101); // Exceeds 100 character limit
        let result = validate_workspace_config(&invalid_config);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for overly long workspace ID"
        );
        let has_length_error = result.errors.iter().any(|error| {
            error.contains("workspace_id")
                && (error.contains("length") || error.contains("1-100 characters"))
        });
        assert!(
            has_length_error,
            "Expected workspace ID length validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_workspace_config_path_validation_error_consistency() {
        let mut invalid_config = ValidatedWorkspaceConfig {
            workspace_id: "valid-workspace".to_string(),
            name: "Test".to_string(),
            description: None,
            path: "../../../etc/passwd".to_string(), // Path traversal attack
            max_file_size: 1000,
            max_file_count: 10,
            enable_watch: false,
            tags: vec![],
            contact_email: Some("test@example.com".to_string()),
            project_url: Some("https://example.com".to_string()),
            metadata: HashMap::new(),
        };

        let result = validate_workspace_config(&invalid_config);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for path traversal"
        );
        let has_path_error = result.errors.iter().any(|error| {
            error.to_lowercase().contains("path")
                && (error.to_lowercase().contains("invalid")
                    || error.to_lowercase().contains("sequences")
                    || error.to_lowercase().contains("value"))
        });
        assert!(
            has_path_error,
            "Expected path validation error, got: {:?}",
            result.errors
        );

        // Test case: Path too long
        invalid_config.path = "a".repeat(501); // Exceeds 500 character limit
        let result = validate_workspace_config(&invalid_config);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for overly long path"
        );
        let has_path_length_error = result.errors.iter().any(|error| {
            error.to_lowercase().contains("path")
                && (error.to_lowercase().contains("too long")
                    || error.to_lowercase().contains("invalid")
                    || error.to_lowercase().contains("value"))
        });
        assert!(
            has_path_length_error,
            "Expected path length validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_workspace_config_numeric_validation_error_consistency() {
        let mut invalid_config = ValidatedWorkspaceConfig {
            workspace_id: "valid-workspace".to_string(),
            name: "Test".to_string(),
            description: None,
            path: "/valid/path".to_string(),
            max_file_size: 0,  // Invalid: below minimum
            max_file_count: 0, // Invalid: below minimum
            enable_watch: false,
            tags: vec![],
            contact_email: Some("test@example.com".to_string()),
            project_url: Some("https://example.com".to_string()),
            metadata: HashMap::new(),
        };

        let result = validate_workspace_config(&invalid_config);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for invalid numeric values"
        );

        // Check for max_file_size validation error
        let has_file_size_error = result
            .errors
            .iter()
            .any(|error| error.contains("max_file_size") && error.contains("1B-1GB"));
        assert!(
            has_file_size_error,
            "Expected max_file_size validation error, got: {:?}",
            result.errors
        );

        // Check for max_file_count validation error
        let has_file_count_error = result
            .errors
            .iter()
            .any(|error| error.contains("max_file_count") && error.contains("1-100000"));
        assert!(
            has_file_count_error,
            "Expected max_file_count validation error, got: {:?}",
            result.errors
        );

        // Test upper bounds
        invalid_config.max_file_size = 1_073_741_825; // Exceeds 1GB limit
        invalid_config.max_file_count = 100_001; // Exceeds limit

        let result = validate_workspace_config(&invalid_config);
        assert!(
            !result.is_valid(),
            "Expected validation to fail for values exceeding limits"
        );
    }

    #[test]
    fn test_search_query_validation_error_consistency() {
        // Test case 1: Empty query
        let mut invalid_query = ValidatedSearchQuery {
            query: "".to_string(), // Invalid: empty
            workspace_id: "valid-workspace".to_string(),
            max_results: Some(100),
            priority: Some(1),
            timeout_seconds: Some(30),
            case_sensitive: false,
            use_regex: false,
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec![],
        };

        let result = validate_search_query(&invalid_query);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for empty query"
        );
        let has_query_error = result.errors.iter().any(|error| {
            error.contains("query")
                && (error.contains("length") || error.contains("1-1000 characters"))
        });
        assert!(
            has_query_error,
            "Expected query length validation error, got: {:?}",
            result.errors
        );

        // Test case 2: Query too long
        invalid_query.query = "a".repeat(1001); // Exceeds 1000 character limit
        let result = validate_search_query(&invalid_query);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for overly long query"
        );
        let has_query_length_error = result.errors.iter().any(|error| {
            error.contains("query")
                && (error.contains("length") || error.contains("1-1000 characters"))
        });
        assert!(
            has_query_length_error,
            "Expected query length validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_search_query_regex_validation_error_consistency() {
        let invalid_regex_query = ValidatedSearchQuery {
            query: "[invalid regex pattern".to_string(), // Invalid regex
            workspace_id: "valid-workspace".to_string(),
            max_results: Some(100),
            priority: Some(1),
            timeout_seconds: Some(30),
            case_sensitive: false,
            use_regex: true, // Enable regex validation
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec![],
        };

        let result = validate_search_query(&invalid_regex_query);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for invalid regex"
        );
        let has_regex_error = result
            .errors
            .iter()
            .any(|error| error.contains("regular expression") || error.contains("regex"));
        assert!(
            has_regex_error,
            "Expected regex validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_search_query_workspace_id_validation_error_consistency() {
        let invalid_workspace_query = ValidatedSearchQuery {
            query: "valid query".to_string(),
            workspace_id: "invalid/workspace/../id".to_string(), // Invalid workspace ID
            max_results: Some(100),
            priority: Some(1),
            timeout_seconds: Some(30),
            case_sensitive: false,
            use_regex: false,
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec![],
        };

        let result = validate_search_query(&invalid_workspace_query);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for invalid workspace ID"
        );
        let has_workspace_error = result.errors.iter().any(|error| {
            error.contains("workspace_id")
                && (error.contains("Invalid workspace ID format") || error.contains("format"))
        });
        assert!(
            has_workspace_error,
            "Expected workspace ID validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_search_query_max_results_validation_error_consistency() {
        let invalid_max_results_query = ValidatedSearchQuery {
            query: "valid query".to_string(),
            workspace_id: "valid-workspace".to_string(),
            max_results: Some(100_001), // Exceeds limit
            priority: Some(1),
            timeout_seconds: Some(30),
            case_sensitive: false,
            use_regex: false,
            file_pattern: None,
            time_start: None,
            time_end: None,
            log_levels: vec![],
        };

        let result = validate_search_query(&invalid_max_results_query);

        assert!(
            !result.is_valid(),
            "Expected validation to fail for invalid max_results"
        );
        let has_max_results_error = result
            .errors
            .iter()
            .any(|error| error.contains("max_results") && error.contains("1-100000"));
        assert!(
            has_max_results_error,
            "Expected max_results validation error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_valid_configurations_return_success() {
        let temp_dir = create_test_temp_dir();
        let valid_config = create_valid_workspace_config(&temp_dir);

        let result = validate_workspace_config(&valid_config);

        // Valid configuration should pass validation
        assert!(
            result.is_valid(),
            "Expected validation to succeed for valid config, got errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.is_empty(),
            "Expected no validation errors for valid config"
        );

        // Test valid search query
        let valid_query = create_valid_search_query();
        let result = validate_search_query(&valid_query);

        assert!(
            result.is_valid(),
            "Expected validation to succeed for valid query, got errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.is_empty(),
            "Expected no validation errors for valid query"
        );
    }

    #[test]
    fn test_validation_result_type_consistency() {
        let temp_dir = create_test_temp_dir();
        let config = create_valid_workspace_config(&temp_dir);

        // Test that the return type is consistently ValidationResult<()>
        let result: ValidationResult<()> = validate_workspace_config(&config);

        // Verify the structure of ValidationResult
        assert!(
            result.data == (),
            "Expected ValidationResult data to be unit type"
        );
        assert!(
            result.errors.is_empty() || !result.errors.is_empty(),
            "ValidationResult should have errors field"
        );
        assert!(
            result.warnings.is_empty() || !result.warnings.is_empty(),
            "ValidationResult should have warnings field"
        );

        // Test methods are available
        assert_eq!(result.is_valid(), result.errors.is_empty());
        assert_eq!(result.has_warnings(), !result.warnings.is_empty());
    }

    #[test]
    fn test_validation_warnings_consistency() {
        let temp_dir = create_test_temp_dir();
        let mut config = create_valid_workspace_config(&temp_dir);

        // Create a configuration that should generate warnings
        config.max_file_size = 100_000_000; // Large file size
        config.max_file_count = 50_000; // Large file count
        config.tags = (0..25).map(|i| format!("tag{}", i)).collect(); // Many tags

        let result = validate_workspace_config(&config);

        // Should be valid but have warnings
        assert!(
            result.is_valid(),
            "Expected validation to succeed despite warnings"
        );
        assert!(
            result.has_warnings(),
            "Expected warnings for large limits and many tags"
        );

        // Check for performance warning
        let has_performance_warning = result
            .warnings
            .iter()
            .any(|warning| warning.contains("performance") || warning.contains("impact"));
        assert!(
            has_performance_warning,
            "Expected performance warning, got: {:?}",
            result.warnings
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test for the complete validation workflow
    #[test]
    fn test_end_to_end_validation_workflow() {
        let temp_dir = create_test_temp_dir();

        // Test the complete workflow from creation to validation
        let config = ValidatedWorkspaceConfig {
            workspace_id: "integration-test-workspace".to_string(),
            name: "Integration Test Workspace".to_string(),
            description: Some("Testing end-to-end validation workflow".to_string()),
            path: temp_dir.path().to_string_lossy().to_string(),
            max_file_size: 5_000_000, // 5MB
            max_file_count: 5000,
            enable_watch: true,
            tags: vec![
                "integration".to_string(),
                "test".to_string(),
                "e2e".to_string(),
            ],
            contact_email: Some("test@example.com".to_string()),
            project_url: Some("https://example.com".to_string()),
            metadata: {
                let mut map = HashMap::new();
                map.insert("test_type".to_string(), "integration".to_string());
                map.insert("created_by".to_string(), "test_suite".to_string());
                map
            },
        };

        let result = validate_workspace_config(&config);

        assert!(
            result.is_valid(),
            "End-to-end validation should succeed for valid configuration"
        );
        assert!(
            result.errors.is_empty(),
            "No errors expected for valid configuration"
        );

        // Test search query validation in the same workflow
        let query = ValidatedSearchQuery {
            query: "ERROR|WARN".to_string(),
            workspace_id: "integration-test-workspace".to_string(),
            max_results: Some(1000),
            priority: Some(1),
            timeout_seconds: Some(30),
            case_sensitive: true,
            use_regex: true, // Test regex validation
            file_pattern: Some("*.log".to_string()),
            time_start: Some("2024-01-01T00:00:00Z".to_string()),
            time_end: Some("2024-12-31T23:59:59Z".to_string()),
            log_levels: vec!["ERROR".to_string(), "WARN".to_string(), "INFO".to_string()],
        };

        let query_result = validate_search_query(&query);

        assert!(
            query_result.is_valid(),
            "Search query validation should succeed for valid regex query"
        );
        assert!(
            query_result.errors.is_empty(),
            "No errors expected for valid search query"
        );
    }

    /// Test error propagation and consistency across multiple validation calls
    #[test]
    fn test_multiple_validation_error_consistency() {
        // Test multiple invalid configurations to ensure consistent error handling
        let invalid_configs = vec![
            ValidatedWorkspaceConfig {
                workspace_id: "".to_string(), // Empty ID
                name: "Test".to_string(),
                description: None,
                path: "/valid/path".to_string(),
                max_file_size: 1000,
                max_file_count: 10,
                enable_watch: false,
                tags: vec![],
                contact_email: Some("test@example.com".to_string()),
                project_url: Some("https://example.com".to_string()),
                metadata: HashMap::new(),
            },
            ValidatedWorkspaceConfig {
                workspace_id: "valid-id".to_string(),
                name: "".to_string(), // Empty name
                description: None,
                path: "/valid/path".to_string(),
                max_file_size: 1000,
                max_file_count: 10,
                enable_watch: false,
                tags: vec![],
                contact_email: Some("test@example.com".to_string()),
                project_url: Some("https://example.com".to_string()),
                metadata: HashMap::new(),
            },
            ValidatedWorkspaceConfig {
                workspace_id: "valid-id".to_string(),
                name: "Valid Name".to_string(),
                description: None,
                path: "../invalid/path".to_string(), // Path traversal
                max_file_size: 1000,
                max_file_count: 10,
                enable_watch: false,
                tags: vec![],
                contact_email: Some("test@example.com".to_string()),
                project_url: Some("https://example.com".to_string()),
                metadata: HashMap::new(),
            },
        ];

        for (index, config) in invalid_configs.iter().enumerate() {
            let result = validate_workspace_config(config);

            assert!(
                !result.is_valid(),
                "Configuration {} should fail validation",
                index
            );
            assert!(
                !result.errors.is_empty(),
                "Configuration {} should have validation errors",
                index
            );

            // Ensure the result type is consistent
            let _: ValidationResult<()> = result;
        }
    }
}
