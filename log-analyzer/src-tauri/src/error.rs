use miette::Diagnostic;
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

    #[error("Search error: {message}")]
    #[diagnostic(
        code(app::search_error),
        help("Try simplifying your search query or checking the workspace status")
    )]
    Search {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Archive error: {message}")]
    #[diagnostic(
        code(app::archive_error),
        help("Ensure the archive file is not corrupted and is a supported format")
    )]
    Archive {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Validation error: {0}")]
    #[diagnostic(
        code(app::validation_error),
        help("Check that your input meets the required format and constraints")
    )]
    Validation(String),

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
            AppError::Search { message, source } => AppError::Search {
                message: format!("{}: {}", context, message),
                source,
            },
            AppError::Archive { message, path } => AppError::Archive {
                message: format!("{}: {}", context, message),
                path,
            },
            other => other,
        }
    }

    /**
     * 创建搜索错误
     */
    pub fn search_error(message: impl Into<String>) -> Self {
        AppError::Search {
            message: message.into(),
            source: None,
        }
    }

    /**
     * 创建归档错误
     */
    pub fn archive_error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        AppError::Archive {
            message: message.into(),
            path,
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

        match with_context {
            AppError::Search { message, .. } => {
                assert!(message.contains("Validation"));
                assert!(message.contains("Query failed"));
            }
            _ => panic!("Expected Search error"),
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
}
