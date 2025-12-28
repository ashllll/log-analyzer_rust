use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{error, info, warn};

use crate::models::extraction_policy::AuditConfig;

/// Audit event types for extraction operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ExtractionStarted,
    ExtractionCompleted,
    ExtractionFailed,
    SecurityEvent,
    PathShortened,
    DepthLimitReached,
}

/// Security event types for audit logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    ZipBombDetected,
    PathTraversalAttempt,
    ForbiddenExtension,
    ExcessiveCompressionRatio,
    DepthLimitExceeded,
    CircularReferenceDetected,
}

/// Severity levels for security events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Audit log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Timestamp of the event
    pub timestamp: SystemTime,

    /// Event type
    pub event_type: AuditEventType,

    /// User ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Workspace ID
    pub workspace_id: String,

    /// Archive path
    pub archive_path: PathBuf,

    /// Extraction policy applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_policy: Option<String>,

    /// Duration of operation (for completion/failure events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Number of files extracted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_extracted: Option<usize>,

    /// Bytes extracted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_extracted: Option<u64>,

    /// Errors encountered (categorized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors_encountered: Option<HashMap<String, usize>>,

    /// Security flags raised
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_flags_raised: Option<usize>,

    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, String>>,
}

/// Security event log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEventLog {
    /// Timestamp of the event
    pub timestamp: SystemTime,

    /// Security event type
    pub event_type: SecurityEventType,

    /// Severity level
    pub severity: Severity,

    /// Workspace ID
    pub workspace_id: String,

    /// Archive path
    pub archive_path: PathBuf,

    /// File path (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,

    /// Compression ratio (for zip bomb events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_ratio: Option<f64>,

    /// Nesting depth (for depth limit events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nesting_depth: Option<usize>,

    /// Risk score (for security scoring)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f64>,

    /// Additional details
    pub details: HashMap<String, String>,
}

/// Audit logger for extraction operations
pub struct AuditLogger {
    config: Arc<AuditConfig>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: Arc<AuditConfig>) -> Self {
        Self { config }
    }

    /// Log extraction start event
    pub fn log_extraction_start(
        &self,
        workspace_id: &str,
        archive_path: &Path,
        user_id: Option<&str>,
        policy_name: Option<&str>,
    ) {
        if !self.config.enable_audit_logging {
            return;
        }

        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionStarted,
            user_id: user_id.map(|s| s.to_string()),
            workspace_id: workspace_id.to_string(),
            archive_path: archive_path.to_path_buf(),
            extraction_policy: policy_name.map(|s| s.to_string()),
            duration_ms: None,
            files_extracted: None,
            bytes_extracted: None,
            errors_encountered: None,
            security_flags_raised: None,
            context: None,
        };

        self.log_entry(&entry);
    }

    /// Log extraction completion event
    #[allow(clippy::too_many_arguments)]
    pub fn log_extraction_complete(
        &self,
        workspace_id: &str,
        archive_path: &Path,
        duration: Duration,
        files_extracted: usize,
        bytes_extracted: u64,
        errors_by_category: HashMap<String, usize>,
        security_flags: usize,
    ) {
        if !self.config.enable_audit_logging {
            return;
        }

        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionCompleted,
            user_id: None,
            workspace_id: workspace_id.to_string(),
            archive_path: archive_path.to_path_buf(),
            extraction_policy: None,
            duration_ms: Some(duration.as_millis() as u64),
            files_extracted: Some(files_extracted),
            bytes_extracted: Some(bytes_extracted),
            errors_encountered: Some(errors_by_category),
            security_flags_raised: Some(security_flags),
            context: None,
        };

        self.log_entry(&entry);
    }

    /// Log extraction failure event
    pub fn log_extraction_failure(
        &self,
        workspace_id: &str,
        archive_path: &Path,
        duration: Duration,
        error_message: &str,
        files_extracted: usize,
        bytes_extracted: u64,
    ) {
        if !self.config.enable_audit_logging {
            return;
        }

        let mut context = HashMap::new();
        context.insert("error_message".to_string(), error_message.to_string());

        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionFailed,
            user_id: None,
            workspace_id: workspace_id.to_string(),
            archive_path: archive_path.to_path_buf(),
            extraction_policy: None,
            duration_ms: Some(duration.as_millis() as u64),
            files_extracted: Some(files_extracted),
            bytes_extracted: Some(bytes_extracted),
            errors_encountered: None,
            security_flags_raised: None,
            context: Some(context),
        };

        self.log_entry(&entry);
    }

    /// Log security event
    #[allow(clippy::too_many_arguments)]
    pub fn log_security_event(
        &self,
        workspace_id: &str,
        archive_path: &Path,
        event_type: SecurityEventType,
        severity: Severity,
        file_path: Option<&Path>,
        compression_ratio: Option<f64>,
        nesting_depth: Option<usize>,
        risk_score: Option<f64>,
        details: HashMap<String, String>,
    ) {
        if !self.config.enable_audit_logging || !self.config.log_security_events {
            return;
        }

        let event = SecurityEventLog {
            timestamp: SystemTime::now(),
            event_type,
            severity,
            workspace_id: workspace_id.to_string(),
            archive_path: archive_path.to_path_buf(),
            file_path: file_path.map(|p| p.to_path_buf()),
            compression_ratio,
            nesting_depth,
            risk_score,
            details,
        };

        self.log_security_event_entry(&event);
    }

    /// Internal method to log an audit entry
    fn log_entry(&self, entry: &AuditLogEntry) {
        match self.config.log_format.as_str() {
            "json" => {
                // Use structured logging with JSON format
                info!(
                    target: "audit",
                    timestamp = ?entry.timestamp,
                    event_type = ?entry.event_type,
                    user_id = ?entry.user_id,
                    workspace_id = %entry.workspace_id,
                    archive_path = %entry.archive_path.display(),
                    extraction_policy = ?entry.extraction_policy,
                    duration_ms = ?entry.duration_ms,
                    files_extracted = ?entry.files_extracted,
                    bytes_extracted = ?entry.bytes_extracted,
                    errors_encountered = ?entry.errors_encountered,
                    security_flags_raised = ?entry.security_flags_raised,
                    context = ?entry.context,
                    "Audit log entry"
                );
            }
            "text" => {
                // Use human-readable text format
                info!(
                    target: "audit",
                    "[AUDIT] {:?} - workspace: {}, archive: {}, event: {:?}",
                    entry.timestamp,
                    entry.workspace_id,
                    entry.archive_path.display(),
                    entry.event_type
                );
            }
            _ => {
                error!("Invalid log format: {}", self.config.log_format);
            }
        }
    }

    /// Internal method to log a security event
    fn log_security_event_entry(&self, event: &SecurityEventLog) {
        match self.config.log_format.as_str() {
            "json" => {
                // Use structured logging with JSON format at WARN level
                warn!(
                    target: "security",
                    timestamp = ?event.timestamp,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    workspace_id = %event.workspace_id,
                    archive_path = %event.archive_path.display(),
                    file_path = ?event.file_path.as_ref().map(|p| p.display().to_string()),
                    compression_ratio = ?event.compression_ratio,
                    nesting_depth = ?event.nesting_depth,
                    risk_score = ?event.risk_score,
                    details = ?event.details,
                    "Security event detected"
                );
            }
            "text" => {
                // Use human-readable text format at WARN level
                warn!(
                    target: "security",
                    "[SECURITY] {:?} - {:?} - workspace: {}, archive: {}, event: {:?}",
                    event.timestamp,
                    event.severity,
                    event.workspace_id,
                    event.archive_path.display(),
                    event.event_type
                );
            }
            _ => {
                error!("Invalid log format: {}", self.config.log_format);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_creation() {
        let config = Arc::new(AuditConfig {
            enable_audit_logging: true,
            log_format: "json".to_string(),
            log_level: "info".to_string(),
            log_security_events: true,
        });

        let logger = AuditLogger::new(config);
        assert!(logger.config.enable_audit_logging);
    }

    #[test]
    fn test_audit_log_entry_serialization() {
        let entry = AuditLogEntry {
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ExtractionStarted,
            user_id: Some("user123".to_string()),
            workspace_id: "workspace456".to_string(),
            archive_path: PathBuf::from("/path/to/archive.zip"),
            extraction_policy: Some("default".to_string()),
            duration_ms: None,
            files_extracted: None,
            bytes_extracted: None,
            errors_encountered: None,
            security_flags_raised: None,
            context: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("extraction_started"));
        assert!(json.contains("user123"));
        assert!(json.contains("workspace456"));
    }

    #[test]
    fn test_security_event_log_serialization() {
        let mut details = HashMap::new();
        details.insert("reason".to_string(), "High compression ratio".to_string());

        let event = SecurityEventLog {
            timestamp: SystemTime::now(),
            event_type: SecurityEventType::ZipBombDetected,
            severity: Severity::High,
            workspace_id: "workspace789".to_string(),
            archive_path: PathBuf::from("/path/to/suspicious.zip"),
            file_path: Some(PathBuf::from("internal/file.txt")),
            compression_ratio: Some(1000.0),
            nesting_depth: Some(5),
            risk_score: Some(5000.0),
            details,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("zip_bomb_detected"));
        assert!(json.contains("high"));
        assert!(json.contains("1000"));
    }
}
