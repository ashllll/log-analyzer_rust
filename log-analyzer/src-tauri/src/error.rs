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
        _message: String,
        _path: Option<PathBuf>,
    },

    #[error("Validation error: {0}")]
    #[diagnostic(
        code(app::validation_error),
        help("Check that your input meets the required format and constraints")
    )]
    Validation(String),

    #[error("Security error: {0}")]
    #[diagnostic(code(app::security_error))]
    Security(String),

    #[error("Not found: {0}")]
    #[diagnostic(code(app::not_found))]
    NotFound(String),

    #[error("Invalid path: {0}")]
    #[diagnostic(
        code(app::invalid_path),
        help("Ensure the path is valid and accessible")
    )]
    InvalidPath(String),

    #[error("Encoding error: {0}")]
    #[diagnostic(code(app::encoding_error))]
    Encoding(String),

    #[error("Query execution error: {0}")]
    #[diagnostic(
        code(app::query_execution_error),
        help("Try simplifying your query or checking the syntax")
    )]
    QueryExecution(String),

    #[error("File watcher error: {0}")]
    #[diagnostic(code(app::file_watcher_error))]
    FileWatcher(String),

    #[error("Index error: {0}")]
    #[diagnostic(code(app::index_error))]
    IndexError(String),

    #[error("Pattern error: {0}")]
    #[diagnostic(code(app::pattern_error), help("Check your regex pattern syntax"))]
    PatternError(String),

    #[error("Database error: {0}")]
    #[diagnostic(
        code(app::database_error),
        help("Check database connection and schema integrity")
    )]
    DatabaseError(String),

    #[error("Configuration error: {0}")]
    #[diagnostic(code(app::config_error))]
    Config(String),

    #[error("Network error: {0}")]
    #[diagnostic(code(app::network_error))]
    Network(String),

    #[error("Internal error: {0}")]
    #[diagnostic(code(app::internal_error))]
    Internal(String),

    #[error("Resource cleanup error: {0}")]
    #[diagnostic(code(app::resource_cleanup_error))]
    ResourceCleanup(String),

    #[error("Concurrency error: {0}")]
    #[diagnostic(code(app::concurrency_error))]
    Concurrency(String),

    #[error("Parse error: {0}")]
    #[diagnostic(code(app::parse_error))]
    Parse(String),

    #[error("Timeout error: {0}")]
    #[diagnostic(code(app::timeout_error))]
    Timeout(String),

    #[error("IO error: {message}")]
    #[diagnostic(code(app::io_error_detailed))]
    IoDetailed {
        message: String,
        path: Option<PathBuf>,
    },
}

impl AppError {
    /**
     * 为错误添加上下文信息
     */
    pub fn with_context(self, context: impl Into<String>) -> Self {
        let context = context.into();
        match self {
            AppError::Search { _message, _source } => AppError::Search {
                _message: format!("{}: {}", context, _message),
                _source,
            },
            AppError::Archive { _message, _path } => AppError::Archive {
                _message: format!("{}: {}", context, _message),
                _path,
            },
            other => other,
        }
    }

    /**
     * 创建搜索错误
     */
    pub fn search_error(message: impl Into<String>) -> Self {
        AppError::Search {
            _message: message.into(),
            _source: None,
        }
    }

    /**
     * 创建归档错误
     */
    pub fn archive_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        AppError::Archive {
            _message: message.into(),
            _path: path,
        }
    }

    /**
     * 创建验证错误
     */
    pub fn validation_error(message: impl Into<String>) -> Self {
        AppError::Validation(message.into())
    }

    /**
     * 创建未找到错误
     */
    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound(message.into())
    }

    /**
     * 创建模式错误
     */
    pub fn pattern_error(message: impl Into<String>) -> Self {
        AppError::PatternError(message.into())
    }

    /**
     * 创建数据库错误
     */
    pub fn database_error(message: impl Into<String>) -> Self {
        AppError::DatabaseError(message.into())
    }

    /**
     * 创建详细的IO错误
     */
    pub fn io_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        AppError::IoDetailed {
            message: message.into(),
            path,
        }
    }
}

/**
 * 统一结果类型 - 使用 eyre::Result 提供更好的错误链
 *
 * 对于内部错误处理，使用 eyre::Result
 * 对于用户可见的错误，转换为 AppError
 */
pub type Result<T> = std::result::Result<T, AppError>;

/**
 * 内部结果类型 - 使用 eyre 进行错误传播
 */
#[allow(dead_code)]
pub type EyreResult<T> = eyre::Result<T>;

/**
 * 将 eyre::Error 转换为 AppError
 */
#[allow(dead_code)]
pub fn eyre_to_app_error(error: eyre::Error) -> AppError {
    AppError::search_error(error.to_string())
}

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
            AppError::Validation(_) => "VALIDATION_ERROR".to_string(),
            AppError::Security(_) => "SECURITY_ERROR".to_string(),
            AppError::NotFound(_) => "NOT_FOUND".to_string(),
            AppError::InvalidPath(_) => "INVALID_PATH".to_string(),
            AppError::Encoding(_) => "ENCODING_ERROR".to_string(),
            AppError::QueryExecution(_) => "QUERY_EXECUTION_ERROR".to_string(),
            AppError::FileWatcher(_) => "FILE_WATCHER_ERROR".to_string(),
            AppError::IndexError(_) => "INDEX_ERROR".to_string(),
            AppError::PatternError(_) => "PATTERN_ERROR".to_string(),
            AppError::DatabaseError(_) => "DATABASE_ERROR".to_string(),
            AppError::Config(_) => "CONFIG_ERROR".to_string(),
            AppError::Network(_) => "NETWORK_ERROR".to_string(),
            AppError::Internal(_) => "INTERNAL_ERROR".to_string(),
            AppError::ResourceCleanup(_) => "RESOURCE_CLEANUP_ERROR".to_string(),
            AppError::Concurrency(_) => "CONCURRENCY_ERROR".to_string(),
            AppError::Parse(_) => "PARSE_ERROR".to_string(),
            AppError::Timeout(_) => "TIMEOUT_ERROR".to_string(),
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
            AppError::Validation(_) => {
                Some("Check that your input meets the required format and constraints")
            }
            AppError::InvalidPath(_) => Some("Ensure the path is valid and accessible"),
            AppError::QueryExecution(_) => {
                Some("Try simplifying your query or checking the syntax")
            }
            AppError::DatabaseError(_) => Some("Check database connection and schema integrity"),
            AppError::PatternError(_) => Some("Check your regex pattern syntax"),
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
        assert!(matches!(error, AppError::Validation(_)));
    }

    #[test]
    fn test_error_with_context() {
        let error = AppError::search_error("Query failed");
        let with_context = error.with_context("Validation");

        if let AppError::Search { _message, .. } = with_context {
            assert!(
                _message.contains("Validation"),
                "Expected context to contain 'Validation', got: {:?}",
                _message
            );
            assert!(
                _message.contains("Query failed"),
                "Expected original error message to be preserved, got: {:?}",
                _message
            );
        } else {
            panic!("Expected Search error variant");
        }
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
        let display = format!("{}", error);
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
        let display = format!("{}", error);

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
