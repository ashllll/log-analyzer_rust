use std::path::PathBuf;
use thiserror::Error;

/**
 * 统一错误类型
 *
 * 使用thiserror提供详细的错误信息和错误链
 */
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Search error: {message}")]
    Search {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Archive error: {message}")]
    Archive {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Query execution error: {0}")]
    QueryExecution(String),

    #[error("File watcher error: {0}")]
    FileWatcher(String),

    #[error("Index error: {0}")]
    IndexError(String),
    
    #[error("Pattern error: {0}")]
    PatternError(String),
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
}

/**
 * 统一结果类型
 */
pub type Result<T> = std::result::Result<T, AppError>;

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
