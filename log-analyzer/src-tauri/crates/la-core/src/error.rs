#![allow(non_snake_case)]

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/**
 * 应用错误类型 - 使用 miette 提供用户友好的错误诊断
 *
 * 这个枚举用于用户可见的错误，提供详细的诊断信息
 */
#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[error("IO error: {0}")]
    #[diagnostic(code(app::io_error))]
    Io(#[from] std::io::Error),

    #[error("Search error: {_message}")]
    #[diagnostic(
        code(app::search_error),
        help("Try simplifying your search query or checking the workspace status")
    )]
    Search {
        category: ErrorCategory,
        _message: String,
        #[source]
        _source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Archive error: {_message}")]
    #[diagnostic(
        code(app::archive_error),
        help("Ensure the archive file is not corrupted and is a supported format")
    )]
    Archive {
        category: ErrorCategory,
        _message: String,
        _path: Option<PathBuf>,
    },

    #[error("Validation error: {message}")]
    #[diagnostic(
        code(app::validation_error),
        help("Check that your input meets the required format and constraints")
    )]
    Validation {
        category: ErrorCategory,
        message: String,
    },

    #[error("Security error: {message}")]
    #[diagnostic(code(app::security_error))]
    Security {
        category: ErrorCategory,
        message: String,
    },

    #[error("Not found: {message}")]
    #[diagnostic(code(app::not_found))]
    NotFound {
        category: ErrorCategory,
        message: String,
    },

    #[error("Invalid path: {message}")]
    #[diagnostic(
        code(app::invalid_path),
        help("Ensure the path is valid and accessible")
    )]
    InvalidPath {
        category: ErrorCategory,
        message: String,
    },

    #[error("Encoding error: {message}")]
    #[diagnostic(code(app::encoding_error))]
    Encoding {
        category: ErrorCategory,
        message: String,
    },

    #[error("Query execution error: {message}")]
    #[diagnostic(
        code(app::query_execution_error),
        help("Try simplifying your query or checking the syntax")
    )]
    QueryExecution {
        category: ErrorCategory,
        message: String,
    },

    #[error("File watcher error: {message}")]
    #[diagnostic(code(app::file_watcher_error))]
    FileWatcher {
        category: ErrorCategory,
        message: String,
    },

    #[error("Index error: {message}")]
    #[diagnostic(code(app::index_error))]
    IndexError {
        category: ErrorCategory,
        message: String,
    },

    #[error("Pattern error: {message}")]
    #[diagnostic(code(app::pattern_error), help("Check your regex pattern syntax"))]
    PatternError {
        category: ErrorCategory,
        message: String,
    },

    #[error("Database error: {message}")]
    #[diagnostic(
        code(app::database_error),
        help("Check database connection and schema integrity")
    )]
    DatabaseError {
        category: ErrorCategory,
        message: String,
    },

    #[error("Configuration error: {message}")]
    #[diagnostic(code(app::config_error))]
    Config {
        category: ErrorCategory,
        message: String,
    },

    #[error("Network error: {message}")]
    #[diagnostic(code(app::network_error))]
    Network {
        category: ErrorCategory,
        message: String,
    },

    #[error("Internal error: {message}")]
    #[diagnostic(code(app::internal_error))]
    Internal {
        category: ErrorCategory,
        message: String,
    },

    #[error("Resource cleanup error: {message}")]
    #[diagnostic(code(app::resource_cleanup_error))]
    ResourceCleanup {
        category: ErrorCategory,
        message: String,
    },

    #[error("Concurrency error: {message}")]
    #[diagnostic(code(app::concurrency_error))]
    Concurrency {
        category: ErrorCategory,
        message: String,
    },

    #[error("Parse error: {message}")]
    #[diagnostic(code(app::parse_error))]
    Parse {
        category: ErrorCategory,
        message: String,
    },

    #[error("Timeout error: {message}")]
    #[diagnostic(code(app::timeout_error))]
    Timeout {
        category: ErrorCategory,
        message: String,
    },

    #[error("IO error: {message}")]
    #[diagnostic(code(app::io_error_detailed))]
    IoDetailed {
        category: ErrorCategory,
        message: String,
        path: Option<PathBuf>,
    },
}

/// Structured error categories for programmatic matching.
/// Replaces fragile string-based error classification with a stored field
/// on each AppError variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Path exceeds operating system limits
    PathTooLong,
    /// Archive format not supported
    UnsupportedFormat,
    /// Archive file corrupted
    CorruptedArchive,
    /// Permission denied
    PermissionDenied,
    /// Zip bomb / malicious compression
    ZipBombDetected,
    /// Nesting depth exceeded
    DepthLimitExceeded,
    /// Disk space exhausted
    DiskSpaceExhausted,
    /// Operation cancelled
    CancellationRequested,
    /// Invalid configuration
    InvalidConfiguration,
    /// General internal error
    InternalError,
}

impl AppError {
    /// Classify this error into a structured category for programmatic handling.
    ///
    /// Each variant stores its category at construction time. The Io variant
    /// uses structured ErrorKind matching (no string-based heuristics).
    pub fn category(&self) -> ErrorCategory {
        use std::io::ErrorKind;
        match self {
            // Io uses structured ErrorKind classification (no string matching)
            AppError::Io(e) => match e.kind() {
                ErrorKind::PermissionDenied => ErrorCategory::PermissionDenied,
                ErrorKind::OutOfMemory => ErrorCategory::DiskSpaceExhausted,
                _ => ErrorCategory::InternalError,
            },
            AppError::Search { category, .. } => *category,
            AppError::Archive { category, .. } => *category,
            AppError::Validation { category, .. } => *category,
            AppError::Security { category, .. } => *category,
            AppError::NotFound { category, .. } => *category,
            AppError::InvalidPath { category, .. } => *category,
            AppError::Encoding { category, .. } => *category,
            AppError::QueryExecution { category, .. } => *category,
            AppError::FileWatcher { category, .. } => *category,
            AppError::IndexError { category, .. } => *category,
            AppError::PatternError { category, .. } => *category,
            AppError::DatabaseError { category, .. } => *category,
            AppError::Config { category, .. } => *category,
            AppError::Network { category, .. } => *category,
            AppError::Internal { category, .. } => *category,
            AppError::ResourceCleanup { category, .. } => *category,
            AppError::Concurrency { category, .. } => *category,
            AppError::Parse { category, .. } => *category,
            AppError::Timeout { category, .. } => *category,
            AppError::IoDetailed { category, .. } => *category,
        }
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// 创建搜索错误
    pub fn search_error(message: impl Into<String>) -> Self {
        AppError::Search {
            category: ErrorCategory::InternalError,
            _message: message.into(),
            _source: None,
        }
    }

    /// 创建归档错误
    pub fn archive_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        AppError::Archive {
            category: ErrorCategory::InternalError,
            _message: message.into(),
            _path: path,
        }
    }

    /// 创建验证错误
    pub fn validation_error(message: impl Into<String>) -> Self {
        AppError::Validation {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建验证错误（显式分类）— 少数调用点需要特定 category
    pub fn validation_error_with(message: impl Into<String>, category: ErrorCategory) -> Self {
        AppError::Validation {
            category,
            message: message.into(),
        }
    }

    /// 创建安全问题错误
    pub fn security_error(message: impl Into<String>) -> Self {
        AppError::Security {
            category: ErrorCategory::ZipBombDetected,
            message: message.into(),
        }
    }

    /// 创建未找到错误
    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建无效路径错误
    pub fn invalid_path(message: impl Into<String>) -> Self {
        AppError::InvalidPath {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建编码错误
    pub fn encoding_error(message: impl Into<String>) -> Self {
        AppError::Encoding {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建查询执行错误
    pub fn query_execution_error(message: impl Into<String>) -> Self {
        AppError::QueryExecution {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建文件监控错误
    pub fn file_watcher_error(message: impl Into<String>) -> Self {
        AppError::FileWatcher {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建索引错误
    pub fn index_error(message: impl Into<String>) -> Self {
        AppError::IndexError {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建模式错误
    pub fn pattern_error(message: impl Into<String>) -> Self {
        AppError::PatternError {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建数据库错误
    pub fn database_error(message: impl Into<String>) -> Self {
        AppError::DatabaseError {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建配置错误
    pub fn config_error(message: impl Into<String>) -> Self {
        AppError::Config {
            category: ErrorCategory::InvalidConfiguration,
            message: message.into(),
        }
    }

    /// 创建网络错误
    pub fn network_error(message: impl Into<String>) -> Self {
        AppError::Network {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        AppError::Internal {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建资源清理错误
    pub fn resource_cleanup_error(message: impl Into<String>) -> Self {
        AppError::ResourceCleanup {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建并发错误
    pub fn concurrency_error(message: impl Into<String>) -> Self {
        AppError::Concurrency {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建解析错误
    pub fn parse_error(message: impl Into<String>) -> Self {
        AppError::Parse {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout_error(message: impl Into<String>) -> Self {
        AppError::Timeout {
            category: ErrorCategory::InternalError,
            message: message.into(),
        }
    }

    /// 创建详细的IO错误，自动脱敏路径信息
    ///
    /// 安全考虑：只保留文件名，不暴露完整路径
    pub fn io_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        let message = message.into();
        // 路径脱敏：只保留文件名，避免泄漏系统目录结构
        let sanitized_path = path.map(|p| {
            p.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("[REDACTED]"))
        });

        AppError::IoDetailed {
            category: ErrorCategory::InternalError,
            message,
            path: sanitized_path,
        }
    }
}

/**
 * 统一结果类型 - 使用 thiserror::Error 提供更好的错误处理
 *
 * 对于内部错误处理，使用 AppError
 * 对于用户可见的错误，通过 CommandError 转换
 */
pub type Result<T> = std::result::Result<T, AppError>;

// ============================================================================
// 命令层错误类型
// ============================================================================

/**
 * 命令层错误结构
 *
 * 用于 Tauri 命令的返回值，提供结构化的错误信息
 * 包含错误码、用户友好消息和帮助提示
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    /// 错误码 (对应 AppError 的 diagnostic code)
    pub code: String,

    /// 用户友好的错误消息
    pub message: String,

    /// 帮助提示 (可选)
    pub help: Option<String>,

    /// 错误详情 (可选，用于调试)
    pub details: Option<serde_json::Value>,
}

impl CommandError {
    /// 从 AppError 创建 CommandError
    pub fn from_app_error(err: &AppError) -> Self {
        let code = err.code();
        let message = err.to_string();
        let help = err.help();

        CommandError {
            code,
            message,
            help: help.map(|h| h.to_string()),
            details: None,
        }
    }

    /// 创建简单错误
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        CommandError {
            code: code.into(),
            message: message.into(),
            help: None,
            details: None,
        }
    }

    /// 添加帮助信息
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// 添加详情信息
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

/// 从 AppError 自动转换
impl From<AppError> for CommandError {
    fn from(err: AppError) -> Self {
        CommandError::from_app_error(&err)
    }
}

/// 从字符串自动转换 (向后兼容)
impl From<String> for CommandError {
    fn from(s: String) -> Self {
        CommandError::new("UNKNOWN", s)
    }
}

/// 从 &str 自动转换
impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        CommandError::new("UNKNOWN", s)
    }
}

/**
 * AppError 扩展方法
 */
impl AppError {
    /// 获取错误码
    pub fn code(&self) -> String {
        match self {
            AppError::Io(_) => "IO_ERROR".to_string(),
            AppError::Search { .. } => "SEARCH_ERROR".to_string(),
            AppError::Archive { .. } => "ARCHIVE_ERROR".to_string(),
            AppError::Validation { .. } => "VALIDATION_ERROR".to_string(),
            AppError::Security { .. } => "SECURITY_ERROR".to_string(),
            AppError::NotFound { .. } => "NOT_FOUND".to_string(),
            AppError::InvalidPath { .. } => "INVALID_PATH".to_string(),
            AppError::Encoding { .. } => "ENCODING_ERROR".to_string(),
            AppError::QueryExecution { .. } => "QUERY_EXECUTION_ERROR".to_string(),
            AppError::FileWatcher { .. } => "FILE_WATCHER_ERROR".to_string(),
            AppError::IndexError { .. } => "INDEX_ERROR".to_string(),
            AppError::PatternError { .. } => "PATTERN_ERROR".to_string(),
            AppError::DatabaseError { .. } => "DATABASE_ERROR".to_string(),
            AppError::Config { .. } => "CONFIG_ERROR".to_string(),
            AppError::Network { .. } => "NETWORK_ERROR".to_string(),
            AppError::Internal { .. } => "INTERNAL_ERROR".to_string(),
            AppError::ResourceCleanup { .. } => "RESOURCE_CLEANUP_ERROR".to_string(),
            AppError::Concurrency { .. } => "CONCURRENCY_ERROR".to_string(),
            AppError::Parse { .. } => "PARSE_ERROR".to_string(),
            AppError::Timeout { .. } => "TIMEOUT_ERROR".to_string(),
            AppError::IoDetailed { .. } => "IO_ERROR".to_string(),
        }
    }

    /// 获取帮助提示
    pub fn help(&self) -> Option<&str> {
        match self {
            AppError::Search { .. } => {
                Some("Try simplifying your search query or checking the workspace status")
            }
            AppError::Archive { .. } => {
                Some("Ensure the archive file is not corrupted and is a supported format")
            }
            AppError::Validation { .. } => {
                Some("Check that your input meets the required format and constraints")
            }
            AppError::InvalidPath { .. } => Some("Ensure the path is valid and accessible"),
            AppError::QueryExecution { .. } => {
                Some("Try simplifying your query or checking the syntax")
            }
            AppError::DatabaseError { .. } => {
                Some("Check database connection and schema integrity")
            }
            AppError::PatternError { .. } => Some("Check your regex pattern syntax"),
            _ => None,
        }
    }
}

/**
 * 命令层统一结果类型
 *
 * 使用 CommandResult<T> 作为 Tauri 命令的返回值类型
 * 这样前端可以接收到结构化的错误信息
 */
pub type CommandResult<T> = std::result::Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = AppError::search_error("Query failed");
        assert!(matches!(error, AppError::Search { .. }));

        let error = AppError::validation_error("Invalid input");
        assert!(matches!(error, AppError::Validation { .. }));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let app_error: AppError = io_error.into();

        assert!(matches!(app_error, AppError::Io(_)));
    }

    #[test]
    fn test_error_display() {
        let error = AppError::search_error("Query failed");
        let display = format!("{error}");
        assert!(display.contains("Search error"));
        assert!(display.contains("Query failed"));
    }

    #[test]
    fn test_error_code() {
        let error = AppError::search_error("Query failed");
        assert_eq!(error.code(), "SEARCH_ERROR");

        let error = AppError::validation_error("Invalid input");
        assert_eq!(error.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_error_help() {
        let error = AppError::search_error("Query failed");
        assert!(error.help().is_some());

        let error = AppError::validation_error("Invalid input");
        assert!(error.help().is_some());
    }

    #[test]
    fn test_error_category_is_stored() {
        // Verify that the category is stored at construction time,
        // not derived from display strings.
        let err = AppError::validation_error_with(
            "some random message without any keywords",
            ErrorCategory::UnsupportedFormat,
        );
        assert_eq!(err.category(), ErrorCategory::UnsupportedFormat);

        let err = AppError::validation_error("some random message without any keywords");
        assert_eq!(err.category(), ErrorCategory::InternalError);

        let err = AppError::config_error("bad config");
        assert_eq!(err.category(), ErrorCategory::InvalidConfiguration);

        let err = AppError::security_error("zip bomb detected");
        assert_eq!(err.category(), ErrorCategory::ZipBombDetected);
    }

    #[test]
    fn test_io_error_category_still_uses_errorkind() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let app_error: AppError = io_err.into();
        assert_eq!(app_error.category(), ErrorCategory::PermissionDenied);

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_error: AppError = io_err.into();
        assert_eq!(app_error.category(), ErrorCategory::InternalError);
    }

    #[test]
    fn test_io_error_path_sanitization() {
        // 测试完整路径被脱敏为文件名
        let full_path = PathBuf::from("/home/user/secret/project/file.log");
        let error = AppError::io_error("File not found", Some(full_path));

        if let AppError::IoDetailed { message, path, .. } = error {
            assert_eq!(message, "File not found");
            assert_eq!(path, Some(PathBuf::from("file.log")));
        } else {
            panic!("Expected IoDetailed error");
        }
    }

    #[test]
    fn test_io_error_path_sanitization_windows() {
        // 测试Windows风格路径脱敏（使用模拟路径）
        // 注意：在Unix系统上，反斜杠被视为普通字符
        let full_path = PathBuf::from("C:/Users/Admin/Documents/secret.txt");
        let error = AppError::io_error("Access denied", Some(full_path));

        if let AppError::IoDetailed { path, .. } = error {
            assert_eq!(path, Some(PathBuf::from("secret.txt")));
        } else {
            panic!("Expected IoDetailed error");
        }
    }

    #[test]
    fn test_io_error_no_path() {
        // 测试无路径情况
        let error = AppError::io_error("Generic error", None);

        if let AppError::IoDetailed { message, path, .. } = error {
            assert_eq!(message, "Generic error");
            assert_eq!(path, None);
        } else {
            panic!("Expected IoDetailed error");
        }
    }
}

// ============================================================================
// CommandError 测试
// ============================================================================

#[cfg(test)]
mod command_error_tests {
    use super::*;

    #[test]
    fn test_command_error_from_app_error() {
        let app_error = AppError::search_error("Query failed");
        let cmd_error = CommandError::from_app_error(&app_error);

        assert_eq!(cmd_error.code, "SEARCH_ERROR");
        assert!(cmd_error.message.contains("Query failed"));
        assert!(cmd_error.help.is_some());
    }

    #[test]
    fn test_command_error_new() {
        let error = CommandError::new("CUSTOM_ERROR", "Something went wrong");

        assert_eq!(error.code, "CUSTOM_ERROR");
        assert_eq!(error.message, "Something went wrong");
        assert!(error.help.is_none());
        assert!(error.details.is_none());
    }

    #[test]
    fn test_command_error_with_help() {
        let error =
            CommandError::new("CUSTOM_ERROR", "Something went wrong").with_help("Try again later");

        assert_eq!(error.help, Some("Try again later".to_string()));
    }

    #[test]
    fn test_command_error_with_details() {
        let details = serde_json::json!({"attempt": 3, "max_retries": 5});
        let error =
            CommandError::new("CUSTOM_ERROR", "Something went wrong").with_details(details.clone());

        assert_eq!(error.details, Some(details));
    }

    #[test]
    fn test_command_error_display() {
        let error = CommandError::new("TEST_ERROR", "Test message");
        let display = format!("{error}");

        assert_eq!(display, "[TEST_ERROR] Test message");
    }

    #[test]
    fn test_command_error_from_app_error_auto() {
        let app_error = AppError::validation_error("Invalid input");
        let cmd_error: CommandError = app_error.into();

        assert_eq!(cmd_error.code, "VALIDATION_ERROR");
        assert!(cmd_error.message.contains("Invalid input"));
    }

    #[test]
    fn test_command_error_from_string() {
        let msg = "Simple error message";
        let cmd_error: CommandError = msg.to_string().into();

        assert_eq!(cmd_error.code, "UNKNOWN");
        assert_eq!(cmd_error.message, msg);
    }

    #[test]
    fn test_command_error_from_str() {
        let msg = "Simple error message";
        let cmd_error: CommandError = msg.into();

        assert_eq!(cmd_error.code, "UNKNOWN");
        assert_eq!(cmd_error.message, msg);
    }
}
