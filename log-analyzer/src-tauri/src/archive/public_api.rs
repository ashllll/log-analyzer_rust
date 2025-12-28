//! Public API for Enhanced Archive Extraction
//!
//! This module provides both synchronous and asynchronous interfaces for
//! archive extraction with comprehensive error handling and result structures.

use crate::archive::extraction_engine::{
    ExtractionEngine, ExtractionPolicy, ExtractionResult as InternalExtractionResult,
    WarningCategory as InternalWarningCategory,
};
use crate::archive::extraction_orchestrator::ExtractionOrchestrator;
use crate::archive::path_manager::{PathConfig, PathManager};
use crate::archive::security_detector::{SecurityDetector, SecurityPolicy};
use crate::error::AppError;
use crate::services::MetadataDB;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::runtime::Runtime;

/// Public extraction result structure with all required fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// List of successfully extracted files
    pub extracted_files: Vec<PathBuf>,
    /// Mapping from shortened paths to original paths
    pub metadata_mappings: HashMap<PathBuf, PathBuf>,
    /// Warnings encountered during extraction
    pub warnings: Vec<ExtractionWarning>,
    /// Performance metrics for the extraction operation
    pub performance_metrics: PerformanceMetrics,
    /// Security events detected during extraction
    pub security_events: Vec<SecurityEvent>,
}

/// Warning encountered during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionWarning {
    /// Warning category
    pub category: WarningCategory,
    /// Warning message
    pub message: String,
    /// File path associated with the warning (if applicable)
    pub file_path: Option<PathBuf>,
    /// Timestamp when the warning occurred
    pub timestamp: SystemTime,
}

/// Categories of extraction warnings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningCategory {
    /// Path was shortened due to length constraints
    PathShortened,
    /// Depth limit was reached
    DepthLimitReached,
    /// High compression ratio detected
    HighCompressionRatio,
    /// Duplicate filename encountered
    DuplicateFilename,
    /// Unicode normalization applied
    UnicodeNormalization,
    /// Insufficient disk space warning
    InsufficientDiskSpace,
    /// Security violation detected
    SecurityViolation,
    /// Extraction error occurred
    ExtractionError,
    /// Path resolution error
    PathError,
}

/// Performance metrics for extraction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total duration of the extraction
    pub total_duration: Duration,
    /// Number of files extracted
    pub files_extracted: usize,
    /// Total bytes extracted
    pub bytes_extracted: u64,
    /// Maximum nesting depth reached
    pub max_depth_reached: usize,
    /// Average extraction speed in MB/s
    pub average_extraction_speed: f64,
    /// Peak memory usage in bytes
    pub peak_memory_usage: usize,
    /// Number of disk I/O operations
    pub disk_io_operations: usize,
}

/// Security event detected during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Type of security event
    pub event_type: SecurityEventType,
    /// Severity level
    pub severity: Severity,
    /// Archive path where the event occurred
    pub archive_path: PathBuf,
    /// Additional details about the event
    pub details: serde_json::Value,
    /// Timestamp when the event occurred
    pub timestamp: SystemTime,
}

/// Types of security events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Zip bomb detected
    ZipBombDetected,
    /// Path traversal attempt detected
    PathTraversalAttempt,
    /// Forbidden file extension detected
    ForbiddenExtension,
    /// Excessive compression ratio detected
    ExcessiveCompressionRatio,
    /// Depth limit exceeded
    DepthLimitExceeded,
}

/// Severity levels for security events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Public extraction error structure with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionError {
    /// Error code for programmatic handling
    pub error_code: ErrorCode,
    /// Human-readable error message
    pub error_message: String,
    /// Path to the file that caused the error (if applicable)
    pub failed_file_path: Option<PathBuf>,
    /// Suggested remediation steps
    pub suggested_remediation: String,
    /// Additional context information
    pub context: HashMap<String, String>,
}

/// Error codes for extraction failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Path exceeds operating system limits
    PathTooLong,
    /// Archive format is not supported
    UnsupportedFormat,
    /// Archive file is corrupted
    CorruptedArchive,
    /// Permission denied for file access
    PermissionDenied,
    /// Zip bomb detected
    ZipBombDetected,
    /// Nesting depth limit exceeded
    DepthLimitExceeded,
    /// Disk space exhausted
    DiskSpaceExhausted,
    /// Operation was cancelled
    CancellationRequested,
    /// Configuration is invalid
    InvalidConfiguration,
    /// Internal error occurred
    InternalError,
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}] {}: {}",
            self.error_code, self.error_message, self.suggested_remediation
        )
    }
}

impl std::error::Error for ExtractionError {}

/// Convert internal AppError to public ExtractionError
impl From<AppError> for ExtractionError {
    fn from(error: AppError) -> Self {
        let error_message = error.to_string();

        // Determine error code based on error message content
        let error_code = if error_message.contains("path") && error_message.contains("long") {
            ErrorCode::PathTooLong
        } else if error_message.contains("unsupported") || error_message.contains("format") {
            ErrorCode::UnsupportedFormat
        } else if error_message.contains("corrupt") {
            ErrorCode::CorruptedArchive
        } else if error_message.contains("permission") || error_message.contains("denied") {
            ErrorCode::PermissionDenied
        } else if error_message.contains("zip bomb") || error_message.contains("compression") {
            ErrorCode::ZipBombDetected
        } else if error_message.contains("depth") {
            ErrorCode::DepthLimitExceeded
        } else if error_message.contains("disk space") || error_message.contains("space") {
            ErrorCode::DiskSpaceExhausted
        } else if error_message.contains("cancel") {
            ErrorCode::CancellationRequested
        } else if error_message.contains("validation") || error_message.contains("invalid") {
            ErrorCode::InvalidConfiguration
        } else {
            ErrorCode::InternalError
        };

        // Generate suggested remediation based on error code
        let suggested_remediation = match error_code {
            ErrorCode::PathTooLong => {
                "Enable long path support in configuration or use path shortening".to_string()
            }
            ErrorCode::UnsupportedFormat => {
                "Verify the archive format is supported (ZIP, RAR, TAR, GZ)".to_string()
            }
            ErrorCode::CorruptedArchive => {
                "Verify the archive file is not corrupted and try re-downloading".to_string()
            }
            ErrorCode::PermissionDenied => {
                "Check file permissions and ensure the application has necessary access rights"
                    .to_string()
            }
            ErrorCode::ZipBombDetected => {
                "Review the archive for malicious content or adjust security thresholds".to_string()
            }
            ErrorCode::DepthLimitExceeded => {
                "Increase max_depth in configuration or extract nested archives manually"
                    .to_string()
            }
            ErrorCode::DiskSpaceExhausted => {
                "Free up disk space or extract to a different location".to_string()
            }
            ErrorCode::CancellationRequested => {
                "Restart the extraction operation if needed".to_string()
            }
            ErrorCode::InvalidConfiguration => {
                "Review and correct the configuration file".to_string()
            }
            ErrorCode::InternalError => {
                "Check logs for details and report the issue if it persists".to_string()
            }
        };

        let mut context = HashMap::new();
        context.insert("original_error".to_string(), error_message.clone());

        ExtractionError {
            error_code,
            error_message,
            failed_file_path: None,
            suggested_remediation,
            context,
        }
    }
}

/// Result type for public API
pub type Result<T> = std::result::Result<T, ExtractionError>;

/// Synchronous extraction API
///
/// Extracts an archive using a blocking runtime
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `target_dir` - Directory where files will be extracted
/// * `workspace_id` - Workspace identifier for tracking
/// * `policy` - Optional extraction policy (uses defaults if None)
///
/// # Returns
///
/// ExtractionResult on success, ExtractionError on failure
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use log_analyzer::archive::public_api::{extract_archive_sync, ExtractionPolicy};
///
/// let result = extract_archive_sync(
///     Path::new("archive.zip"),
///     Path::new("/tmp/output"),
///     "workspace_123",
///     None,
/// )?;
///
/// println!("Extracted {} files", result.extracted_files.len());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[allow(clippy::result_large_err)]
pub fn extract_archive_sync(
    archive_path: &Path,
    target_dir: &Path,
    workspace_id: &str,
    policy: Option<ExtractionPolicy>,
) -> Result<ExtractionResult> {
    // Create a new runtime for synchronous execution
    let runtime = Runtime::new().map_err(|e| ExtractionError {
        error_code: ErrorCode::InternalError,
        error_message: format!("Failed to create runtime: {}", e),
        failed_file_path: None,
        suggested_remediation: "Check system resources and try again".to_string(),
        context: HashMap::new(),
    })?;

    // Block on the async extraction
    runtime.block_on(extract_archive_async(
        archive_path,
        target_dir,
        workspace_id,
        policy,
    ))
}

/// Asynchronous extraction API
///
/// Extracts an archive asynchronously
///
/// # Arguments
///
/// * `archive_path` - Path to the archive file
/// * `target_dir` - Directory where files will be extracted
/// * `workspace_id` - Workspace identifier for tracking
/// * `policy` - Optional extraction policy (uses defaults if None)
///
/// # Returns
///
/// ExtractionResult on success, ExtractionError on failure
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use log_analyzer::archive::public_api::{extract_archive_async, ExtractionPolicy};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let result = extract_archive_async(
///     Path::new("archive.zip"),
///     Path::new("/tmp/output"),
///     "workspace_123",
///     None,
/// ).await?;
///
/// println!("Extracted {} files", result.extracted_files.len());
/// # Ok(())
/// # }
/// ```
pub async fn extract_archive_async(
    archive_path: &Path,
    target_dir: &Path,
    workspace_id: &str,
    policy: Option<ExtractionPolicy>,
) -> Result<ExtractionResult> {
    let start_time = SystemTime::now();

    // Use provided policy or default
    let policy = policy.unwrap_or_default();

    // Validate policy
    policy.validate().map_err(ExtractionError::from)?;

    // Initialize components
    let metadata_db = Arc::new(
        MetadataDB::new(":memory:")
            .await
            .map_err(|e| ExtractionError {
                error_code: ErrorCode::InternalError,
                error_message: format!("Failed to initialize metadata database: {}", e),
                failed_file_path: None,
                suggested_remediation: "Check system resources and try again".to_string(),
                context: HashMap::new(),
            })?,
    );

    let path_manager = Arc::new(PathManager::new(PathConfig::default(), metadata_db.clone()));

    let security_detector = Arc::new(SecurityDetector::new(SecurityPolicy::default()));

    // Create extraction engine
    let engine = ExtractionEngine::new(
        path_manager.clone(),
        security_detector.clone(),
        policy.clone(),
    )
    .map_err(ExtractionError::from)?;

    // Create orchestrator for concurrency control
    let orchestrator = ExtractionOrchestrator::new(Arc::new(engine), Some(num_cpus::get() / 2));

    // Perform extraction
    let internal_result: InternalExtractionResult = orchestrator
        .extract_archive(archive_path, target_dir, workspace_id)
        .await
        .map_err(ExtractionError::from)?;

    // Calculate performance metrics
    let duration = start_time.elapsed().unwrap_or(Duration::from_secs(0));

    // Use the extraction speed from internal result if available
    let speed_mbps = if internal_result.extraction_duration_secs > 0.0 {
        internal_result.speed_mb_per_sec()
    } else if duration.as_secs() > 0 {
        (internal_result.total_bytes as f64 / 1_048_576.0) / duration.as_secs_f64()
    } else {
        0.0
    };

    // Collect metadata mappings
    let metadata_mappings = HashMap::new();
    // Note: In a real implementation, we would query the metadata_db here
    // For now, we'll leave it empty as the actual mapping retrieval
    // would require iterating through all extracted files

    // Convert warnings
    let warnings: Vec<ExtractionWarning> = internal_result
        .warnings
        .iter()
        .map(|w| ExtractionWarning {
            category: match w.category {
                InternalWarningCategory::DepthLimitReached => WarningCategory::DepthLimitReached,
                InternalWarningCategory::PathShortened => WarningCategory::PathShortened,
                InternalWarningCategory::HighCompressionRatio => {
                    WarningCategory::HighCompressionRatio
                }
                InternalWarningCategory::FileSkipped => WarningCategory::DuplicateFilename,
                InternalWarningCategory::SecurityEvent => WarningCategory::SecurityViolation,
                InternalWarningCategory::ArchiveError => WarningCategory::ExtractionError,
                InternalWarningCategory::PathResolutionError => WarningCategory::PathError,
            },
            message: w.message.clone(),
            file_path: w.file_path.clone(),
            timestamp: SystemTime::now(),
        })
        .collect();

    // Build performance metrics
    let performance_metrics = PerformanceMetrics {
        total_duration: duration,
        files_extracted: internal_result.total_files,
        bytes_extracted: internal_result.total_bytes,
        max_depth_reached: internal_result.max_depth_reached,
        average_extraction_speed: speed_mbps,
        peak_memory_usage: 0, // Would need actual memory tracking
        disk_io_operations: internal_result.total_files, // Approximate
    };

    // Build security events (empty for now, would be populated by security detector)
    let security_events = Vec::new();

    Ok(ExtractionResult {
        extracted_files: internal_result.extracted_files,
        metadata_mappings,
        warnings,
        performance_metrics,
        security_events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        let app_error = AppError::validation_error("path too long");
        let extraction_error = ExtractionError::from(app_error);

        assert_eq!(extraction_error.error_code, ErrorCode::PathTooLong);
        assert!(!extraction_error.suggested_remediation.is_empty());
    }

    #[test]
    fn test_extraction_error_display() {
        let error = ExtractionError {
            error_code: ErrorCode::ZipBombDetected,
            error_message: "Suspicious compression ratio detected".to_string(),
            failed_file_path: Some(PathBuf::from("/tmp/malicious.zip")),
            suggested_remediation: "Review the archive".to_string(),
            context: HashMap::new(),
        };

        let display = format!("{}", error);
        assert!(display.contains("ZipBombDetected"));
        assert!(display.contains("Suspicious compression ratio"));
    }

    #[test]
    fn test_error_code_mapping() {
        // Test all error code conversions
        let test_cases = vec![
            ("path too long", ErrorCode::PathTooLong),
            ("unsupported format", ErrorCode::UnsupportedFormat),
            ("corrupted archive", ErrorCode::CorruptedArchive),
            ("permission denied", ErrorCode::PermissionDenied),
            ("zip bomb detected", ErrorCode::ZipBombDetected),
            ("depth limit exceeded", ErrorCode::DepthLimitExceeded),
            ("disk space exhausted", ErrorCode::DiskSpaceExhausted),
            ("operation cancelled", ErrorCode::CancellationRequested),
            ("invalid configuration", ErrorCode::InvalidConfiguration),
            ("unknown error", ErrorCode::InternalError),
        ];

        for (message, expected_code) in test_cases {
            let app_error = AppError::validation_error(message);
            let extraction_error = ExtractionError::from(app_error);
            assert_eq!(extraction_error.error_code, expected_code);
        }
    }

    #[test]
    fn test_error_serialization() {
        let error = ExtractionError {
            error_code: ErrorCode::PathTooLong,
            error_message: "Path exceeds limit".to_string(),
            failed_file_path: Some(PathBuf::from("/very/long/path")),
            suggested_remediation: "Enable long path support".to_string(),
            context: {
                let mut map = HashMap::new();
                map.insert("max_length".to_string(), "260".to_string());
                map
            },
        };

        // Test serialization
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("PathTooLong"));
        assert!(json.contains("Path exceeds limit"));

        // Test deserialization
        let deserialized: ExtractionError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error_code, ErrorCode::PathTooLong);
        assert_eq!(deserialized.error_message, "Path exceeds limit");
        assert_eq!(deserialized.context.get("max_length").unwrap(), "260");
    }

    #[test]
    fn test_result_serialization() {
        let result = ExtractionResult {
            extracted_files: vec![
                PathBuf::from("/tmp/file1.txt"),
                PathBuf::from("/tmp/file2.txt"),
            ],
            metadata_mappings: {
                let mut map = HashMap::new();
                map.insert(
                    PathBuf::from("/tmp/short"),
                    PathBuf::from("/tmp/very/long/original/path"),
                );
                map
            },
            warnings: vec![ExtractionWarning {
                category: WarningCategory::PathShortened,
                message: "Path was shortened".to_string(),
                file_path: Some(PathBuf::from("/tmp/file")),
                timestamp: SystemTime::now(),
            }],
            performance_metrics: PerformanceMetrics {
                total_duration: Duration::from_secs(10),
                files_extracted: 2,
                bytes_extracted: 2048,
                max_depth_reached: 1,
                average_extraction_speed: 204.8,
                peak_memory_usage: 1048576,
                disk_io_operations: 20,
            },
            security_events: vec![],
        };

        // Test serialization
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("file1.txt"));
        assert!(json.contains("file2.txt"));

        // Test deserialization
        let deserialized: ExtractionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.extracted_files.len(), 2);
        assert_eq!(deserialized.warnings.len(), 1);
        assert_eq!(deserialized.performance_metrics.files_extracted, 2);
    }

    #[test]
    fn test_warning_categories() {
        let categories = vec![
            WarningCategory::PathShortened,
            WarningCategory::DepthLimitReached,
            WarningCategory::HighCompressionRatio,
            WarningCategory::DuplicateFilename,
            WarningCategory::UnicodeNormalization,
            WarningCategory::InsufficientDiskSpace,
        ];

        for category in categories {
            let warning = ExtractionWarning {
                category,
                message: "Test warning".to_string(),
                file_path: None,
                timestamp: SystemTime::now(),
            };

            // Verify serialization works
            let json = serde_json::to_string(&warning).unwrap();
            let deserialized: ExtractionWarning = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.category, category);
        }
    }

    #[test]
    fn test_security_event_types() {
        let event_types = vec![
            SecurityEventType::ZipBombDetected,
            SecurityEventType::PathTraversalAttempt,
            SecurityEventType::ForbiddenExtension,
            SecurityEventType::ExcessiveCompressionRatio,
            SecurityEventType::DepthLimitExceeded,
        ];

        for event_type in event_types {
            let event = SecurityEvent {
                event_type,
                severity: Severity::High,
                archive_path: PathBuf::from("/tmp/archive.zip"),
                details: serde_json::json!({"test": "data"}),
                timestamp: SystemTime::now(),
            };

            // Verify serialization works
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: SecurityEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.event_type, event_type);
        }
    }

    #[test]
    fn test_severity_levels() {
        let severities = vec![
            Severity::Low,
            Severity::Medium,
            Severity::High,
            Severity::Critical,
        ];

        for severity in severities {
            let event = SecurityEvent {
                event_type: SecurityEventType::ZipBombDetected,
                severity,
                archive_path: PathBuf::from("/tmp/archive.zip"),
                details: serde_json::json!({}),
                timestamp: SystemTime::now(),
            };

            // Verify serialization works
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: SecurityEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.severity, severity);
        }
    }

    #[test]
    fn test_performance_metrics_calculation() {
        let metrics = PerformanceMetrics {
            total_duration: Duration::from_secs(100),
            files_extracted: 1000,
            bytes_extracted: 104857600, // 100 MB
            max_depth_reached: 5,
            average_extraction_speed: 1.0, // 1 MB/s
            peak_memory_usage: 10485760,   // 10 MB
            disk_io_operations: 1000,
        };

        // Verify metrics are reasonable
        assert_eq!(metrics.files_extracted, 1000);
        assert_eq!(metrics.bytes_extracted, 104857600);
        assert!(metrics.max_depth_reached <= 20);
        assert!(metrics.average_extraction_speed > 0.0);
    }

    #[test]
    fn test_extraction_error_context() {
        let mut context = HashMap::new();
        context.insert("file_size".to_string(), "1000000".to_string());
        context.insert("compression_ratio".to_string(), "1000".to_string());

        let error = ExtractionError {
            error_code: ErrorCode::ZipBombDetected,
            error_message: "Suspicious archive detected".to_string(),
            failed_file_path: Some(PathBuf::from("/tmp/suspicious.zip")),
            suggested_remediation: "Review the archive for malicious content".to_string(),
            context: context.clone(),
        };

        assert_eq!(error.context.get("file_size").unwrap(), "1000000");
        assert_eq!(error.context.get("compression_ratio").unwrap(), "1000");
    }
}
