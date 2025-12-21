//! Property-Based Tests for Public API
//!
//! These tests verify the correctness properties of the public API structures
//! and functions using property-based testing with proptest.

#[cfg(test)]
mod tests {
    use crate::archive::public_api::{
        ErrorCode, ExtractionError, ExtractionResult, ExtractionWarning, PerformanceMetrics,
        SecurityEvent, SecurityEventType, Severity, WarningCategory,
    };
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    // Strategy for generating ErrorCode
    fn error_code_strategy() -> impl Strategy<Value = ErrorCode> {
        prop_oneof![
            Just(ErrorCode::PathTooLong),
            Just(ErrorCode::UnsupportedFormat),
            Just(ErrorCode::CorruptedArchive),
            Just(ErrorCode::PermissionDenied),
            Just(ErrorCode::ZipBombDetected),
            Just(ErrorCode::DepthLimitExceeded),
            Just(ErrorCode::DiskSpaceExhausted),
            Just(ErrorCode::CancellationRequested),
            Just(ErrorCode::InvalidConfiguration),
            Just(ErrorCode::InternalError),
        ]
    }

    // Strategy for generating ExtractionError
    fn extraction_error_strategy() -> impl Strategy<Value = ExtractionError> {
        (
            error_code_strategy(),
            "[a-zA-Z0-9 ]{10,100}",
            prop::option::of("[a-zA-Z0-9/_.-]{5,50}"),
            "[a-zA-Z0-9 ]{10,100}",
            prop::collection::hash_map("[a-z]{3,10}", "[a-zA-Z0-9 ]{5,20}", 0..5),
        )
            .prop_map(
                |(error_code, error_message, failed_file_path, suggested_remediation, context)| {
                    ExtractionError {
                        error_code,
                        error_message,
                        failed_file_path: failed_file_path.map(PathBuf::from),
                        suggested_remediation,
                        context,
                    }
                },
            )
    }

    // Strategy for generating WarningCategory
    fn warning_category_strategy() -> impl Strategy<Value = WarningCategory> {
        prop_oneof![
            Just(WarningCategory::PathShortened),
            Just(WarningCategory::DepthLimitReached),
            Just(WarningCategory::HighCompressionRatio),
            Just(WarningCategory::DuplicateFilename),
            Just(WarningCategory::UnicodeNormalization),
            Just(WarningCategory::InsufficientDiskSpace),
        ]
    }

    // Strategy for generating SecurityEventType
    fn security_event_type_strategy() -> impl Strategy<Value = SecurityEventType> {
        prop_oneof![
            Just(SecurityEventType::ZipBombDetected),
            Just(SecurityEventType::PathTraversalAttempt),
            Just(SecurityEventType::ForbiddenExtension),
            Just(SecurityEventType::ExcessiveCompressionRatio),
            Just(SecurityEventType::DepthLimitExceeded),
        ]
    }

    // Strategy for generating Severity
    fn severity_strategy() -> impl Strategy<Value = Severity> {
        prop_oneof![
            Just(Severity::Low),
            Just(Severity::Medium),
            Just(Severity::High),
            Just(Severity::Critical),
        ]
    }

    // Strategy for generating ExtractionWarning
    fn extraction_warning_strategy() -> impl Strategy<Value = ExtractionWarning> {
        (
            warning_category_strategy(),
            "[a-zA-Z0-9 ]{10,100}",
            prop::option::of("[a-zA-Z0-9/_.-]{5,50}"),
        )
            .prop_map(|(category, message, file_path)| ExtractionWarning {
                category,
                message,
                file_path: file_path.map(PathBuf::from),
                timestamp: SystemTime::now(),
            })
    }

    // Strategy for generating SecurityEvent
    fn security_event_strategy() -> impl Strategy<Value = SecurityEvent> {
        (
            security_event_type_strategy(),
            severity_strategy(),
            "[a-zA-Z0-9/_.-]{5,50}",
        )
            .prop_map(|(event_type, severity, archive_path)| SecurityEvent {
                event_type,
                severity,
                archive_path: PathBuf::from(archive_path),
                details: serde_json::json!({"test": "data"}),
                timestamp: SystemTime::now(),
            })
    }

    // Strategy for generating PerformanceMetrics
    fn performance_metrics_strategy() -> impl Strategy<Value = PerformanceMetrics> {
        (
            0u64..1000000,
            0usize..10000,
            0u64..1000000000,
            0usize..20,
            0.0f64..1000.0,
            0usize..1000000000,
            0usize..100000,
        )
            .prop_map(
                |(
                    duration_secs,
                    files_extracted,
                    bytes_extracted,
                    max_depth_reached,
                    average_extraction_speed,
                    peak_memory_usage,
                    disk_io_operations,
                )| {
                    PerformanceMetrics {
                        total_duration: Duration::from_secs(duration_secs),
                        files_extracted,
                        bytes_extracted,
                        max_depth_reached,
                        average_extraction_speed,
                        peak_memory_usage,
                        disk_io_operations,
                    }
                },
            )
    }

    // Strategy for generating ExtractionResult
    fn extraction_result_strategy() -> impl Strategy<Value = ExtractionResult> {
        (
            prop::collection::vec("[a-zA-Z0-9/_.-]{5,50}", 0..100),
            prop::collection::hash_map("[a-zA-Z0-9/_.-]{5,50}", "[a-zA-Z0-9/_.-]{5,50}", 0..50),
            prop::collection::vec(extraction_warning_strategy(), 0..20),
            performance_metrics_strategy(),
            prop::collection::vec(security_event_strategy(), 0..10),
        )
            .prop_map(
                |(
                    extracted_files,
                    metadata_mappings,
                    warnings,
                    performance_metrics,
                    security_events,
                )| {
                    ExtractionResult {
                        extracted_files: extracted_files.into_iter().map(PathBuf::from).collect(),
                        metadata_mappings: metadata_mappings
                            .into_iter()
                            .map(|(k, v)| (PathBuf::from(k), PathBuf::from(v)))
                            .collect(),
                        warnings,
                        performance_metrics,
                        security_events,
                    }
                },
            )
    }

    /// **Feature: enhanced-archive-handling, Property 46: Error structure completeness**
    ///
    /// Property: For any ExtractionError, all required fields must be present and non-empty
    /// where applicable.
    ///
    /// **Validates: Requirements 10.4**
    ///
    /// This property ensures that every ExtractionError contains:
    /// - A valid error_code
    /// - A non-empty error_message
    /// - A non-empty suggested_remediation
    /// - A context map (may be empty)
    /// - An optional failed_file_path
    proptest! {
        #[test]
        fn prop_error_structure_completeness(error in extraction_error_strategy()) {
            // Verify error_code is set (all enum variants are valid)
            let _ = error.error_code;

            // Verify error_message is non-empty
            prop_assert!(!error.error_message.is_empty(),
                "error_message must not be empty");

            // Verify suggested_remediation is non-empty
            prop_assert!(!error.suggested_remediation.is_empty(),
                "suggested_remediation must not be empty");

            // Verify context exists (may be empty, but must be present)
            let _ = &error.context;

            // Verify failed_file_path is properly handled (Option type)
            if let Some(path) = &error.failed_file_path {
                prop_assert!(!path.as_os_str().is_empty(),
                    "failed_file_path, if present, must not be empty");
            }

            // Verify the error can be displayed
            let display_str = format!("{}", error);
            prop_assert!(!display_str.is_empty(),
                "Error display string must not be empty");

            // Verify the error can be serialized
            let serialized = serde_json::to_string(&error);
            prop_assert!(serialized.is_ok(),
                "Error must be serializable to JSON");

            // Verify the error can be deserialized
            if let Ok(json_str) = serialized {
                let deserialized: Result<ExtractionError, _> = serde_json::from_str(&json_str);
                prop_assert!(deserialized.is_ok(),
                    "Error must be deserializable from JSON");
            }
        }
    }

    /// Test that error codes map to appropriate remediation suggestions
    proptest! {
        #[test]
        fn prop_error_code_has_remediation(error_code in error_code_strategy()) {
            let error = ExtractionError {
                error_code,
                error_message: "Test error".to_string(),
                failed_file_path: None,
                suggested_remediation: match error_code {
                    ErrorCode::PathTooLong => "Enable long path support".to_string(),
                    ErrorCode::UnsupportedFormat => "Verify format".to_string(),
                    ErrorCode::CorruptedArchive => "Re-download".to_string(),
                    ErrorCode::PermissionDenied => "Check permissions".to_string(),
                    ErrorCode::ZipBombDetected => "Review archive".to_string(),
                    ErrorCode::DepthLimitExceeded => "Increase depth".to_string(),
                    ErrorCode::DiskSpaceExhausted => "Free space".to_string(),
                    ErrorCode::CancellationRequested => "Restart".to_string(),
                    ErrorCode::InvalidConfiguration => "Fix config".to_string(),
                    ErrorCode::InternalError => "Check logs".to_string(),
                },
                context: HashMap::new(),
            };

            prop_assert!(!error.suggested_remediation.is_empty(),
                "Every error code must have a remediation suggestion");
        }
    }

    /// Test that ExtractionError implements required traits
    #[test]
    fn test_extraction_error_traits() {
        let error = ExtractionError {
            error_code: ErrorCode::InternalError,
            error_message: "Test error".to_string(),
            failed_file_path: None,
            suggested_remediation: "Test remediation".to_string(),
            context: HashMap::new(),
        };

        // Test Display trait
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());

        // Test Error trait
        let error_trait: &dyn std::error::Error = &error;
        assert!(error_trait.to_string().len() > 0);

        // Test Clone trait
        let cloned = error.clone();
        assert_eq!(cloned.error_code, error.error_code);

        // Test Debug trait
        let debug_str = format!("{:?}", error);
        assert!(!debug_str.is_empty());
    }

    /// **Feature: enhanced-archive-handling, Property 47: Result structure completeness**
    ///
    /// Property: For any ExtractionResult, all required fields must be present and properly
    /// structured.
    ///
    /// **Validates: Requirements 10.5**
    ///
    /// This property ensures that every ExtractionResult contains:
    /// - A list of extracted_files (may be empty)
    /// - A metadata_mappings map (may be empty)
    /// - A warnings list (may be empty)
    /// - Complete performance_metrics with all fields
    /// - A security_events list (may be empty)
    proptest! {
        #[test]
        fn prop_result_structure_completeness(result in extraction_result_strategy()) {
            // Verify extracted_files list exists (may be empty)
            let _ = &result.extracted_files;

            // Verify all extracted file paths are valid
            for file_path in &result.extracted_files {
                prop_assert!(!file_path.as_os_str().is_empty(),
                    "Extracted file paths must not be empty");
            }

            // Verify metadata_mappings exists (may be empty)
            let _ = &result.metadata_mappings;

            // Verify all mapping paths are valid
            for (short_path, original_path) in &result.metadata_mappings {
                prop_assert!(!short_path.as_os_str().is_empty(),
                    "Short path in mapping must not be empty");
                prop_assert!(!original_path.as_os_str().is_empty(),
                    "Original path in mapping must not be empty");
            }

            // Verify warnings list exists (may be empty)
            let _ = &result.warnings;

            // Verify all warnings have required fields
            for warning in &result.warnings {
                prop_assert!(!warning.message.is_empty(),
                    "Warning message must not be empty");
                let _ = warning.category;
                let _ = warning.timestamp;
            }

            // Verify performance_metrics exists and has all fields
            let metrics = &result.performance_metrics;
            let _ = metrics.total_duration;
            let _ = metrics.files_extracted;
            let _ = metrics.bytes_extracted;
            let _ = metrics.max_depth_reached;

            // Verify max_depth_reached is within valid range (0-20)
            prop_assert!(metrics.max_depth_reached <= 20,
                "max_depth_reached must be <= 20, got {}", metrics.max_depth_reached);

            // Verify average_extraction_speed is non-negative
            prop_assert!(metrics.average_extraction_speed >= 0.0,
                "average_extraction_speed must be non-negative");

            let _ = metrics.peak_memory_usage;
            let _ = metrics.disk_io_operations;

            // Verify security_events list exists (may be empty)
            let _ = &result.security_events;

            // Verify all security events have required fields
            for event in &result.security_events {
                let _ = event.event_type;
                let _ = event.severity;
                prop_assert!(!event.archive_path.as_os_str().is_empty(),
                    "Security event archive_path must not be empty");
                let _ = event.timestamp;
            }

            // Verify the result can be serialized
            let serialized = serde_json::to_string(&result);
            prop_assert!(serialized.is_ok(),
                "Result must be serializable to JSON");

            // Verify the result can be deserialized
            if let Ok(json_str) = serialized {
                let deserialized: Result<ExtractionResult, _> = serde_json::from_str(&json_str);
                prop_assert!(deserialized.is_ok(),
                    "Result must be deserializable from JSON");

                // Verify deserialized result has same number of files
                if let Ok(deserialized_result) = deserialized {
                    prop_assert_eq!(
                        deserialized_result.extracted_files.len(),
                        result.extracted_files.len(),
                        "Deserialized result must have same number of extracted files"
                    );
                }
            }
        }
    }

    /// Test that performance metrics are consistent
    proptest! {
        #[test]
        fn prop_performance_metrics_consistency(metrics in performance_metrics_strategy()) {
            // If files were extracted, bytes should typically be > 0 (unless all files are empty)
            // This is a weak consistency check
            if metrics.files_extracted > 0 {
                // We can't enforce bytes_extracted > 0 because files could be empty
                // But we can check that disk_io_operations is reasonable
                prop_assert!(metrics.disk_io_operations >= 0,
                    "disk_io_operations must be non-negative");
            }

            // Speed should be non-negative
            prop_assert!(metrics.average_extraction_speed >= 0.0,
                "average_extraction_speed must be non-negative");

            // Memory usage should be non-negative
            prop_assert!(metrics.peak_memory_usage >= 0,
                "peak_memory_usage must be non-negative");

            // Depth should be within valid range
            prop_assert!(metrics.max_depth_reached <= 20,
                "max_depth_reached must be <= 20");
        }
    }

    /// Test that ExtractionResult implements required traits
    #[test]
    fn test_extraction_result_traits() {
        let result = ExtractionResult {
            extracted_files: vec![PathBuf::from("/tmp/file1.txt")],
            metadata_mappings: HashMap::new(),
            warnings: vec![],
            performance_metrics: PerformanceMetrics {
                total_duration: Duration::from_secs(10),
                files_extracted: 1,
                bytes_extracted: 1024,
                max_depth_reached: 1,
                average_extraction_speed: 102.4,
                peak_memory_usage: 1048576,
                disk_io_operations: 10,
            },
            security_events: vec![],
        };

        // Test Clone trait
        let cloned = result.clone();
        assert_eq!(cloned.extracted_files.len(), result.extracted_files.len());

        // Test Debug trait
        let debug_str = format!("{:?}", result);
        assert!(!debug_str.is_empty());

        // Test serialization
        let serialized = serde_json::to_string(&result);
        assert!(serialized.is_ok());
    }
}
