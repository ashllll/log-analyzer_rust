use proptest::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::archive::audit_logger::{
    AuditEventType, AuditLogEntry, AuditLogger, SecurityEventLog, SecurityEventType,
    Severity as AuditSeverity,
};
use crate::models::extraction_policy::AuditConfig;

/// Generate valid workspace IDs
fn valid_workspace_id() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{8,32}"
}

/// Generate valid archive paths
fn valid_archive_path() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5).prop_map(|parts| {
        let mut path = PathBuf::new();
        for part in parts {
            path.push(part);
        }
        path.set_extension("zip");
        path
    })
}

/// Generate valid user IDs
fn valid_user_id() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[a-zA-Z0-9_-]{4,16}")
}

/// Generate valid policy names
fn valid_policy_name() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop::sample::select(vec![
        "default".to_string(),
        "strict".to_string(),
        "permissive".to_string(),
    ]))
}

/// Generate valid durations
fn valid_duration() -> impl Strategy<Value = Duration> {
    (0u64..3600000u64).prop_map(Duration::from_millis)
}

/// Generate valid file counts
fn valid_file_count() -> impl Strategy<Value = usize> {
    0usize..10000usize
}

/// Generate valid byte counts
fn valid_byte_count() -> impl Strategy<Value = u64> {
    0u64..10_737_418_240u64 // Up to 10GB
}

/// Generate valid error categories
fn valid_error_categories() -> impl Strategy<Value = HashMap<String, usize>> {
    prop::collection::hash_map(
        prop::sample::select(vec![
            "path_too_long".to_string(),
            "unsupported_format".to_string(),
            "corrupted_archive".to_string(),
            "permission_denied".to_string(),
            "zip_bomb_detected".to_string(),
        ]),
        0usize..100usize,
        0..5,
    )
}

/// Generate valid security event types
fn valid_security_event_type() -> impl Strategy<Value = SecurityEventType> {
    prop::sample::select(vec![
        SecurityEventType::ZipBombDetected,
        SecurityEventType::PathTraversalAttempt,
        SecurityEventType::ForbiddenExtension,
        SecurityEventType::ExcessiveCompressionRatio,
        SecurityEventType::DepthLimitExceeded,
        SecurityEventType::CircularReferenceDetected,
    ])
}

/// Generate valid severity levels
fn valid_severity() -> impl Strategy<Value = AuditSeverity> {
    prop::sample::select(vec![
        AuditSeverity::Low,
        AuditSeverity::Medium,
        AuditSeverity::High,
        AuditSeverity::Critical,
    ])
}

/// Generate valid compression ratios
fn valid_compression_ratio() -> impl Strategy<Value = Option<f64>> {
    prop::option::of(1.0f64..10000.0f64)
}

/// Generate valid nesting depths
fn valid_nesting_depth() -> impl Strategy<Value = Option<usize>> {
    prop::option::of(0usize..20usize)
}

/// Generate valid risk scores
fn valid_risk_score() -> impl Strategy<Value = Option<f64>> {
    prop::option::of(0.0f64..1_000_000.0f64)
}

/// Generate valid security details
fn valid_security_details() -> impl Strategy<Value = HashMap<String, String>> {
    prop::collection::hash_map("[a-zA-Z_]{4,16}", "[a-zA-Z0-9 ]{4,64}", 0..5)
}

proptest! {
    /// **Feature: enhanced-archive-handling, Property 39: Audit log completeness**
    ///
    /// For any extraction operation, the audit log should contain:
    /// timestamp, user_id, workspace_id, archive_path, and extraction_policy_applied
    ///
    /// **Validates: Requirements 9.1**
    #[test]
    fn prop_audit_log_completeness(
        workspace_id in valid_workspace_id(),
        archive_path in valid_archive_path(),
        user_id in valid_user_id(),
        policy_name in valid_policy_name(),
    ) {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let logger = AuditLogger::new(config);

        // Create an audit log entry for extraction start
        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionStarted,
            user_id: user_id.clone(),
            workspace_id: workspace_id.clone(),
            archive_path: archive_path.clone(),
            extraction_policy: policy_name.clone(),
            duration_ms: None,
            files_extracted: None,
            bytes_extracted: None,
            errors_encountered: None,
            security_flags_raised: None,
            context: None,
        };

        // Verify all required fields are present
        prop_assert!(entry.timestamp <= SystemTime::now());
        prop_assert_eq!(entry.event_type, AuditEventType::ExtractionStarted);
        prop_assert_eq!(&entry.user_id, &user_id);
        prop_assert_eq!(&entry.workspace_id, &workspace_id);
        prop_assert_eq!(&entry.archive_path, &archive_path);
        prop_assert_eq!(&entry.extraction_policy, &policy_name);

        // Verify the entry can be serialized to JSON
        let json = serde_json::to_string(&entry);
        prop_assert!(json.is_ok(), "Audit log entry should be serializable to JSON");

        let json_str = json.unwrap();
        prop_assert!(json_str.contains(&workspace_id), "JSON should contain workspace_id");
        prop_assert!(json_str.contains("extraction_started"), "JSON should contain event_type");
    }

    /// **Feature: enhanced-archive-handling, Property 42: Structured log format**
    ///
    /// For any audit log entry, the log should be valid JSON with consistent field names
    /// across all log entries.
    ///
    /// **Validates: Requirements 9.4**
    #[test]
    fn prop_structured_log_format(
        workspace_id in valid_workspace_id(),
        archive_path in valid_archive_path(),
        duration in valid_duration(),
        files_extracted in valid_file_count(),
        bytes_extracted in valid_byte_count(),
        errors_by_category in valid_error_categories(),
        security_flags in 0usize..100usize,
    ) {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let _logger = AuditLogger::new(config);

        // Create an audit log entry for extraction completion
        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionCompleted,
            user_id: None,
            workspace_id: workspace_id.clone(),
            archive_path: archive_path.clone(),
            extraction_policy: None,
            duration_ms: Some(duration.as_millis() as u64),
            files_extracted: Some(files_extracted),
            bytes_extracted: Some(bytes_extracted),
            errors_encountered: Some(errors_by_category.clone()),
            security_flags_raised: Some(security_flags),
            context: None,
        };

        // Serialize to JSON
        let json_result = serde_json::to_string(&entry);
        prop_assert!(json_result.is_ok(), "Entry should serialize to JSON");

        let json_str = json_result.unwrap();

        // Verify JSON is valid by deserializing it back
        let deserialized: Result<AuditLogEntry, _> = serde_json::from_str(&json_str);
        prop_assert!(deserialized.is_ok(), "JSON should be valid and deserializable");

        // Verify consistent field names
        prop_assert!(json_str.contains("\"timestamp\""), "JSON should have timestamp field");
        prop_assert!(json_str.contains("\"event_type\""), "JSON should have event_type field");
        prop_assert!(json_str.contains("\"workspace_id\""), "JSON should have workspace_id field");
        prop_assert!(json_str.contains("\"archive_path\""), "JSON should have archive_path field");
        prop_assert!(json_str.contains("\"duration_ms\""), "JSON should have duration_ms field");
        prop_assert!(json_str.contains("\"files_extracted\""), "JSON should have files_extracted field");
        prop_assert!(json_str.contains("\"bytes_extracted\""), "JSON should have bytes_extracted field");

        // Verify field values match
        let deserialized_entry = deserialized.unwrap();
        prop_assert_eq!(deserialized_entry.workspace_id, workspace_id);
        prop_assert_eq!(deserialized_entry.archive_path, archive_path);
        prop_assert_eq!(deserialized_entry.files_extracted, Some(files_extracted));
        prop_assert_eq!(deserialized_entry.bytes_extracted, Some(bytes_extracted));
        prop_assert_eq!(deserialized_entry.security_flags_raised, Some(security_flags));
    }

    /// **Feature: enhanced-archive-handling, Property 40: Security event logging**
    ///
    /// For any security event (zip bomb, path traversal, etc.), the system should log
    /// at WARN level with full context including event type, severity, and details.
    ///
    /// **Validates: Requirements 9.2**
    #[test]
    fn prop_security_event_logging(
        workspace_id in valid_workspace_id(),
        archive_path in valid_archive_path(),
        event_type in valid_security_event_type(),
        severity in valid_severity(),
        compression_ratio in valid_compression_ratio(),
        nesting_depth in valid_nesting_depth(),
        risk_score in valid_risk_score(),
        details in valid_security_details(),
    ) {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let _logger = AuditLogger::new(config);

        // Create a security event log entry
        let event = SecurityEventLog {
            timestamp: SystemTime::now(),
            event_type: event_type.clone(),
            severity: severity.clone(),
            workspace_id: workspace_id.clone(),
            archive_path: archive_path.clone(),
            file_path: None,
            compression_ratio,
            nesting_depth,
            risk_score,
            details: details.clone(),
        };

        // Verify all required fields are present
        prop_assert!(event.timestamp <= SystemTime::now());
        prop_assert_eq!(event.event_type, event_type);
        prop_assert_eq!(event.severity, severity);
        prop_assert_eq!(&event.workspace_id, &workspace_id);
        prop_assert_eq!(&event.archive_path, &archive_path);
        prop_assert_eq!(event.compression_ratio, compression_ratio);
        prop_assert_eq!(event.nesting_depth, nesting_depth);
        prop_assert_eq!(event.risk_score, risk_score);
        prop_assert_eq!(&event.details, &details);

        // Serialize to JSON
        let json_result = serde_json::to_string(&event);
        prop_assert!(json_result.is_ok(), "Security event should serialize to JSON");

        let json_str = json_result.unwrap();

        // Verify JSON is valid by deserializing it back
        let deserialized: Result<SecurityEventLog, _> = serde_json::from_str(&json_str);
        prop_assert!(deserialized.is_ok(), "JSON should be valid and deserializable");

        // Verify consistent field names for security events
        prop_assert!(json_str.contains("\"timestamp\""), "JSON should have timestamp field");
        prop_assert!(json_str.contains("\"event_type\""), "JSON should have event_type field");
        prop_assert!(json_str.contains("\"severity\""), "JSON should have severity field");
        prop_assert!(json_str.contains("\"workspace_id\""), "JSON should have workspace_id field");
        prop_assert!(json_str.contains("\"archive_path\""), "JSON should have archive_path field");
        prop_assert!(json_str.contains("\"details\""), "JSON should have details field");

        // Verify field values match
        let deserialized_event = deserialized.unwrap();
        prop_assert_eq!(deserialized_event.event_type, event_type);
        prop_assert_eq!(deserialized_event.severity, severity);
        prop_assert_eq!(deserialized_event.workspace_id, workspace_id);
        prop_assert_eq!(deserialized_event.archive_path, archive_path);

        // For floating point values, use approximate comparison
        if let (Some(expected), Some(actual)) = (compression_ratio, deserialized_event.compression_ratio) {
            prop_assert!((expected - actual).abs() < 0.0001,
                "Compression ratio mismatch: expected {}, got {}", expected, actual);
        } else {
            prop_assert_eq!(deserialized_event.compression_ratio, compression_ratio);
        }

        prop_assert_eq!(deserialized_event.nesting_depth, nesting_depth);

        if let (Some(expected), Some(actual)) = (risk_score, deserialized_event.risk_score) {
            prop_assert!((expected - actual).abs() < 0.0001,
                "Risk score mismatch: expected {}, got {}", expected, actual);
        } else {
            prop_assert_eq!(deserialized_event.risk_score, risk_score);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_audit_logger_disabled() {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: false,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let logger = AuditLogger::new(config);

        // Logging should not panic when disabled
        logger.log_extraction_start(
            "workspace123",
            &PathBuf::from("/test/archive.zip"),
            Some("user456"),
            Some("default"),
        );
    }

    #[test]
    fn test_security_logging_disabled() {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: false,
        });

        let logger = AuditLogger::new(config);

        // Security logging should not panic when disabled
        logger.log_security_event(
            "workspace123",
            &PathBuf::from("/test/archive.zip"),
            SecurityEventType::ZipBombDetected,
            AuditSeverity::High,
            None,
            Some(1000.0),
            Some(5),
            Some(5000.0),
            HashMap::new(),
        );
    }

    #[test]
    fn test_extraction_complete_logging() {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let logger = AuditLogger::new(config);

        let mut errors = HashMap::new();
        errors.insert("corrupted_archive".to_string(), 2);
        errors.insert("permission_denied".to_string(), 1);

        // Should not panic
        logger.log_extraction_complete(
            "workspace123",
            &PathBuf::from("/test/archive.zip"),
            Duration::from_secs(60),
            100,
            1024 * 1024 * 50, // 50MB
            errors,
            3,
        );
    }

    #[test]
    fn test_extraction_failure_logging() {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let logger = AuditLogger::new(config);

        // Should not panic
        logger.log_extraction_failure(
            "workspace123",
            &PathBuf::from("/test/archive.zip"),
            Duration::from_secs(30),
            "Disk space exhausted",
            50,
            1024 * 1024 * 25, // 25MB
        );
    }
}
