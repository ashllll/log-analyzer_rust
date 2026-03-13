//! FFI 错误处理模块
//!
//! 提供跨 FFI 边界的安全错误传递机制，遵循 flutter_rust_bridge 2.x 最佳实践。
//!
//! ## 设计原则
//!
//! 1. **绝不 Panic**: FFI 边界永不主动触发 panic，所有错误通过 Result 类型传递
//! 2. **错误分类**: 区分业务错误、系统错误和 FFI 特定错误
//! 3. **错误上下文**: 使用 anyhow/miette 提供丰富的错误上下文
//! 4. **跨语言兼容**: 错误类型可安全转换为 Dart 异常
//!
//! ## 参考实现
//!
//! - [flutter_rust_bridge Error Handling](https://cjycode.com/flutter_rust_bridge/guides/miscellaneous/errors)
//! - [Rust FFI Guidelines - Error Handling](https://rust-lang.github.io/rust-bindgen/expectations.html)
//! - [PyO3 Exception Handling](https://pyo3.rs/main/doc/exception)

use std::fmt;
use std::panic::Location;

use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// FFI 错误代码枚举
///
/// 提供结构化的错误分类，便于 Flutter 端进行特定处理
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[frb(dart_metadata = ("immutable"))]
pub enum FfiErrorCode {
    /// 通用未知错误
    Unknown,
    /// 初始化失败
    InitializationFailed,
    /// 无效参数
    InvalidArgument,
    /// 资源未找到
    NotFound,
    /// 资源已存在
    AlreadyExists,
    /// 权限不足
    PermissionDenied,
    /// IO 错误
    IoError,
    /// 数据库错误
    DatabaseError,
    /// 序列化/反序列化错误
    SerializationError,
    /// 运行时错误（Tokio 等）
    RuntimeError,
    /// 任务被取消
    TaskCancelled,
    /// 超时
    Timeout,
    /// FFI 特定错误
    FfiError,
    /// 会话不存在或已过期
    SessionExpired,
    /// 无效的状态转换
    InvalidStateTransition,
    /// 工作区错误
    Workspace,
    /// 搜索错误
    Search,
    /// 验证错误
    Validation,
    /// 并发错误
    Concurrency,
    /// 内部错误
    Internal,
}

impl fmt::Display for FfiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiErrorCode::Unknown => write!(f, "UNKNOWN"),
            FfiErrorCode::InitializationFailed => write!(f, "INITIALIZATION_FAILED"),
            FfiErrorCode::InvalidArgument => write!(f, "INVALID_ARGUMENT"),
            FfiErrorCode::NotFound => write!(f, "NOT_FOUND"),
            FfiErrorCode::AlreadyExists => write!(f, "ALREADY_EXISTS"),
            FfiErrorCode::PermissionDenied => write!(f, "PERMISSION_DENIED"),
            FfiErrorCode::IoError => write!(f, "IO_ERROR"),
            FfiErrorCode::DatabaseError => write!(f, "DATABASE_ERROR"),
            FfiErrorCode::SerializationError => write!(f, "SERIALIZATION_ERROR"),
            FfiErrorCode::RuntimeError => write!(f, "RUNTIME_ERROR"),
            FfiErrorCode::TaskCancelled => write!(f, "TASK_CANCELLED"),
            FfiErrorCode::Timeout => write!(f, "TIMEOUT"),
            FfiErrorCode::FfiError => write!(f, "FFI_ERROR"),
            FfiErrorCode::SessionExpired => write!(f, "SESSION_EXPIRED"),
            FfiErrorCode::InvalidStateTransition => write!(f, "INVALID_STATE_TRANSITION"),
            FfiErrorCode::Workspace => write!(f, "WORKSPACE_ERROR"),
            FfiErrorCode::Search => write!(f, "SEARCH_ERROR"),
            FfiErrorCode::Validation => write!(f, "VALIDATION_ERROR"),
            FfiErrorCode::Concurrency => write!(f, "CONCURRENCY_ERROR"),
            FfiErrorCode::Internal => write!(f, "INTERNAL_ERROR"),
        }
    }
}

/// FFI 错误类型 - Enum 形式
///
/// 跨 FFI 边界的安全错误类型，包含详细的错误变体和上下文信息。
/// 设计参考：Node.js N-API 的错误处理、PyO3 的 PyErr
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata = ("immutable"))]
pub enum FfiError {
    #[error("Not initialized")]
    NotInitialized,

    #[error("IO error: {message}")]
    Io {
        message: String,
        path: Option<String>,
    },

    #[error("Workspace error: {context}")]
    Workspace {
        context: String,
        #[source]
        source: Option<String>,
    },

    #[error("Search error: {message}")]
    Search { message: String },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("Not found: {resource} - {id}")]
    NotFound { resource: String, id: String },

    #[error("Concurrency error: {message}")]
    Concurrency { message: String },

    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Invalid argument: {name} - {reason}")]
    InvalidArgument { name: String, reason: String },

    #[error("Runtime error: {operation} - {details}")]
    RuntimeError { operation: String, details: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Session expired: {session_id}")]
    SessionExpired { session_id: String },

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    #[error("Already exists: {resource} - {id}")]
    AlreadyExists { resource: String, id: String },

    #[error("Task cancelled: {task_id}")]
    TaskCancelled { task_id: String },

    #[error("Initialization failed: {message}")]
    InitializationFailed { message: String },
}

/// FFI 结果类型别名
pub type FfiResult<T> = std::result::Result<T, FfiError>;

impl FfiError {
    /// 创建新的 FFI 错误（从旧版结构体风格兼容）
    #[track_caller]
    pub fn new(code: FfiErrorCode, message: impl Into<String>) -> Self {
        let message = message.into();
        match code {
            FfiErrorCode::NotFound => FfiError::NotFound {
                resource: "unknown".to_string(),
                id: message,
            },
            FfiErrorCode::IoError => FfiError::Io {
                message,
                path: None,
            },
            FfiErrorCode::InvalidArgument => FfiError::InvalidArgument {
                name: "unknown".to_string(),
                reason: message,
            },
            FfiErrorCode::DatabaseError => FfiError::DatabaseError { message },
            FfiErrorCode::SerializationError => FfiError::SerializationError { message },
            FfiErrorCode::Timeout => FfiError::Timeout { duration_ms: 0 },
            FfiErrorCode::RuntimeError => FfiError::RuntimeError {
                operation: "unknown".to_string(),
                details: message,
            },
            FfiErrorCode::SessionExpired => FfiError::SessionExpired {
                session_id: message,
            },
            FfiErrorCode::PermissionDenied => FfiError::PermissionDenied { operation: message },
            FfiErrorCode::AlreadyExists => FfiError::AlreadyExists {
                resource: "unknown".to_string(),
                id: message,
            },
            FfiErrorCode::TaskCancelled => FfiError::TaskCancelled { task_id: message },
            FfiErrorCode::InitializationFailed => FfiError::InitializationFailed { message },
            FfiErrorCode::Workspace => FfiError::Workspace {
                context: message,
                source: None,
            },
            FfiErrorCode::Search => FfiError::Search { message },
            FfiErrorCode::Validation => FfiError::Validation {
                field: "unknown".to_string(),
                message,
            },
            FfiErrorCode::Concurrency => FfiError::Concurrency { message },
            FfiErrorCode::Internal | _ => FfiError::Internal { message },
        }
    }

    /// 创建带详细信息的错误（兼容旧版 API）
    #[track_caller]
    pub fn with_details(
        code: FfiErrorCode,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        let message = format!("{} | Details: {}", message.into(), details.into());
        Self::new(code, message)
    }

    /// 添加上下文（兼容旧版 API）
    pub fn with_context(self, context: impl Into<String>) -> Self {
        let context = context.into();
        match self {
            FfiError::Workspace { context: _, source } => FfiError::Workspace {
                context,
                source,
            },
            FfiError::Search { message } => FfiError::Search {
                message: format!("{}: {}", context, message),
            },
            FfiError::Internal { message } => FfiError::Internal {
                message: format!("{}: {}", context, message),
            },
            FfiError::Io { message, path } => FfiError::Io {
                message: format!("{}: {}", context, message),
                path,
            },
            FfiError::Validation { field, message } => FfiError::Validation {
                field,
                message: format!("{}: {}", context, message),
            },
            FfiError::NotFound { resource, id } => FfiError::NotFound {
                resource,
                id: format!("{}: {}", context, id),
            },
            FfiError::Concurrency { message } => FfiError::Concurrency {
                message: format!("{}: {}", context, message),
            },
            FfiError::Timeout { duration_ms } => FfiError::Timeout { duration_ms },
            FfiError::InvalidArgument { name, reason } => FfiError::InvalidArgument {
                name,
                reason: format!("{}: {}", context, reason),
            },
            FfiError::RuntimeError { operation, details } => FfiError::RuntimeError {
                operation,
                details: format!("{}: {}", context, details),
            },
            FfiError::DatabaseError { message } => FfiError::DatabaseError {
                message: format!("{}: {}", context, message),
            },
            FfiError::SerializationError { message } => FfiError::SerializationError {
                message: format!("{}: {}", context, message),
            },
            FfiError::SessionExpired { session_id } => FfiError::SessionExpired { session_id },
            FfiError::PermissionDenied { operation } => FfiError::PermissionDenied {
                operation: format!("{}: {}", context, operation),
            },
            FfiError::AlreadyExists { resource, id } => FfiError::AlreadyExists {
                resource,
                id: format!("{}: {}", context, id),
            },
            FfiError::TaskCancelled { task_id } => FfiError::TaskCancelled { task_id },
            FfiError::InitializationFailed { message } => FfiError::InitializationFailed {
                message: format!("{}: {}", context, message),
            },
            FfiError::NotInitialized => FfiError::Internal {
                message: format!("{}: Not initialized", context),
            },
        }
    }

    /// 获取错误代码（兼容旧版 API）
    pub fn code(&self) -> FfiErrorCode {
        match self {
            FfiError::NotInitialized => FfiErrorCode::InitializationFailed,
            FfiError::Io { .. } => FfiErrorCode::IoError,
            FfiError::Workspace { .. } => FfiErrorCode::Workspace,
            FfiError::Search { .. } => FfiErrorCode::Search,
            FfiError::Validation { .. } => FfiErrorCode::Validation,
            FfiError::NotFound { .. } => FfiErrorCode::NotFound,
            FfiError::Concurrency { .. } => FfiErrorCode::Concurrency,
            FfiError::Timeout { .. } => FfiErrorCode::Timeout,
            FfiError::Internal { .. } => FfiErrorCode::Internal,
            FfiError::InvalidArgument { .. } => FfiErrorCode::InvalidArgument,
            FfiError::RuntimeError { .. } => FfiErrorCode::RuntimeError,
            FfiError::DatabaseError { .. } => FfiErrorCode::DatabaseError,
            FfiError::SerializationError { .. } => FfiErrorCode::SerializationError,
            FfiError::SessionExpired { .. } => FfiErrorCode::SessionExpired,
            FfiError::PermissionDenied { .. } => FfiErrorCode::PermissionDenied,
            FfiError::AlreadyExists { .. } => FfiErrorCode::AlreadyExists,
            FfiError::TaskCancelled { .. } => FfiErrorCode::TaskCancelled,
            FfiError::InitializationFailed { .. } => FfiErrorCode::InitializationFailed,
        }
    }

    /// 创建未知错误
    #[track_caller]
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// 创建初始化失败错误
    #[track_caller]
    pub fn initialization_failed(message: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: message.into(),
        }
    }

    /// 创建无效参数错误
    #[track_caller]
    pub fn invalid_argument(name: &str, reason: impl Into<String>) -> Self {
        Self::InvalidArgument {
            name: name.to_string(),
            reason: reason.into(),
        }
    }

    /// 创建资源未找到错误
    #[track_caller]
    pub fn not_found(resource: &str, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.to_string(),
            id: id.into(),
        }
    }

    /// 创建 IO 错误
    #[track_caller]
    pub fn io_error(operation: &str, error: impl fmt::Display) -> Self {
        Self::Io {
            message: format!("IO 操作失败: {}", operation),
            path: None,
        }
    }

    /// 创建运行时错误
    #[track_caller]
    pub fn runtime_error(operation: &str, error: impl fmt::Display) -> Self {
        Self::RuntimeError {
            operation: operation.to_string(),
            details: error.to_string(),
        }
    }

    /// 创建会话过期错误
    #[track_caller]
    pub fn session_expired(session_id: impl Into<String>) -> Self {
        Self::SessionExpired {
            session_id: session_id.into(),
        }
    }

    /// 转换为 Dart 友好的错误结构
    pub fn into_dart_exception(self) -> String {
        format!("[{:?}] {}", self.code(), self)
    }
}

// ==================== 从标准错误类型转换 ====================

impl From<std::io::Error> for FfiError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            path: None,
        }
    }
}

impl From<serde_json::Error> for FfiError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            message: err.to_string(),
        }
    }
}

impl From<sqlx::Error> for FfiError {
    #[track_caller]
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl From<uuid::Error> for FfiError {
    #[track_caller]
    fn from(err: uuid::Error) -> Self {
        Self::invalid_argument("uuid", err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for FfiError {
    #[track_caller]
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::SerializationError {
            message: format!("UTF-8 解码失败: {}", err),
        }
    }
}

impl From<std::str::Utf8Error> for FfiError {
    #[track_caller]
    fn from(err: std::str::Utf8Error) -> Self {
        Self::SerializationError {
            message: format!("UTF-8 解码失败: {}", err),
        }
    }
}

impl From<regex::Error> for FfiError {
    #[track_caller]
    fn from(err: regex::Error) -> Self {
        Self::invalid_argument("regex pattern", err.to_string())
    }
}

impl From<anyhow::Error> for FfiError {
    #[track_caller]
    fn from(err: anyhow::Error) -> Self {
        Self::Internal {
            message: format!("{:?}", err),
        }
    }
}

// ==================== 从 AppError 转换 ====================

impl From<crate::error::AppError> for FfiError {
    fn from(err: crate::error::AppError) -> Self {
        use crate::error::AppError;

        match err {
            AppError::Io(e) => FfiError::Io {
                message: e.to_string(),
                path: None,
            },
            AppError::Search { _message, .. } => FfiError::Search { message: _message },
            AppError::NotFound(msg) => FfiError::NotFound {
                resource: "unknown".to_string(),
                id: msg,
            },
            AppError::Validation(msg) => FfiError::Validation {
                field: "unknown".to_string(),
                message: msg,
            },
            AppError::Archive { _message, .. } => FfiError::Internal { message: _message },
            AppError::Security(msg) => FfiError::PermissionDenied { operation: msg },
            AppError::InvalidPath(msg) => FfiError::Io {
                message: msg.clone(),
                path: Some(msg),
            },
            AppError::Encoding(msg) => FfiError::SerializationError { message: msg },
            AppError::QueryExecution(msg) => FfiError::Search { message: msg },
            AppError::FileWatcher(msg) => FfiError::Internal { message: msg },
            AppError::IndexError(msg) => FfiError::Internal { message: msg },
            AppError::PatternError(msg) => FfiError::Validation {
                field: "pattern".to_string(),
                message: msg,
            },
            AppError::DatabaseError(msg) => FfiError::DatabaseError { message: msg },
            AppError::Config(msg) => FfiError::Internal { message: msg },
            AppError::Network(msg) => FfiError::Io {
                message: msg,
                path: None,
            },
            AppError::Internal(msg) => FfiError::Internal { message: msg },
            AppError::ResourceCleanup(msg) => FfiError::Internal { message: msg },
            AppError::Concurrency(msg) => FfiError::Concurrency { message: msg },
            AppError::Parse(msg) => FfiError::SerializationError { message: msg },
            AppError::Timeout(msg) => FfiError::Timeout {
                duration_ms: msg.parse().unwrap_or(0),
            },
            AppError::IoDetailed { message, path } => FfiError::Io {
                message,
                path: path.map(|p| p.to_string_lossy().to_string()),
            },
        }
    }
}

// ==================== 旧版 API 兼容：结构体形式的 FfiError ====================

/// FFI 结果包装类型
///
/// 用于需要明确返回成功/失败状态的 FFI 函数
#[derive(Debug, Clone)]
#[frb(dart_metadata = ("immutable"))]
pub struct FfiResultWrapper<T> {
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    pub error: Option<FfiError>,
    /// 数据（成功时）
    pub data: Option<T>,
}

impl<T> FfiResultWrapper<T> {
    /// 创建成功结果
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            error: None,
            data: Some(data),
        }
    }

    /// 创建失败结果
    pub fn err(error: FfiError) -> Self {
        Self {
            success: false,
            error: Some(error),
            data: None,
        }
    }

    /// 从 Result 转换
    pub fn from_result(result: FfiResult<T>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(e) => Self::err(e),
        }
    }
}

impl<T: Default> FfiResultWrapper<T> {
    /// 创建带默认数据的成功结果
    pub fn ok_default() -> Self {
        Self::ok(T::default())
    }
}

// ==================== 便捷宏 ====================

/// 便捷宏：返回 FFI 错误
#[macro_export]
macro_rules! ffi_err {
    ($code:expr, $msg:expr) => {
        return Err($crate::ffi::error::FfiError::new($code, $msg))
    };
    ($code:expr, $fmt:expr, $($arg:tt)*) => {
        return Err($crate::ffi::error::FfiError::new($code, format!($fmt, $($arg)*)))
    };
}

/// 便捷宏：包装结果为 FFI 结果
#[macro_export]
macro_rules! ffi_wrap {
    ($expr:expr, $code:expr, $context:expr) => {
        $expr.map_err(|e| $crate::ffi::error::FfiError::with_details(
            $code,
            $context,
            e.to_string(),
        ))
    };
}

/// 便捷宏：确保条件满足，否则返回错误
#[macro_export]
macro_rules! ffi_ensure {
    ($cond:expr, $code:expr, $msg:expr) => {
        if !$cond {
            ffi_err!($code, $msg);
        }
    };
    ($cond:expr, $code:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            ffi_err!($code, format!($fmt, $($arg)*));
        }
    };
}

// ==================== Panic 处理 ====================

/// FFI 安全的 panic 钩子
///
/// 将 panic 转换为可恢复的错误，而不是终止进程
pub fn setup_ffi_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()));
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        tracing::error!(
            location = ?location,
            message = %message,
            "FFI 边界捕获到 panic"
        );
    }));
}

/// 捕获 panic 并转换为 FFI 错误
pub fn catch_panic_as_ffi_error<F, T>(f: F) -> FfiResult<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(result) => Ok(result),
        Err(panic_info) => {
            let message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            Err(FfiError::Internal {
                message: format!("FFI 调用中发生 panic: {}", message),
            })
        }
    }
}

/// 捕获 panic 并返回默认值
pub fn catch_panic_with_default<F, T>(default: T, f: F) -> T
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(result) => result,
        Err(_) => {
            tracing::error!("FFI 调用中发生 panic，返回默认值");
            default
        }
    }
}

/// 将 Result<T, String> 转换为 FfiResult<T>
pub fn map_error<T>(result: Result<T, String>, context: &str) -> FfiResult<T> {
    result.map_err(|e| FfiError::Internal {
        message: format!("{}: {}", context, e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_error_creation() {
        let err = FfiError::new(FfiErrorCode::NotFound, "test not found");
        assert!(matches!(err, FfiError::NotFound { .. }));
    }

    #[test]
    fn test_ffi_error_with_details() {
        let err =
            FfiError::with_details(FfiErrorCode::IoError, "read failed", "permission denied");
        assert!(matches!(err, FfiError::Io { .. }));
    }

    #[test]
    fn test_ffi_error_helpers() {
        let err = FfiError::not_found("user", "123");
        assert!(matches!(err, FfiError::NotFound { .. }));

        let err = FfiError::invalid_argument("age", "must be positive");
        assert!(matches!(err, FfiError::InvalidArgument { .. }));
    }

    #[test]
    fn test_result_wrapper() {
        let ok_result: FfiResult<i32> = Ok(42);
        let wrapper = FfiResultWrapper::from_result(ok_result);
        assert!(wrapper.success);
        assert_eq!(wrapper.data, Some(42));

        let err_result: FfiResult<i32> = Err(FfiError::unknown("test error"));
        let wrapper = FfiResultWrapper::from_result(err_result);
        assert!(!wrapper.success);
        assert!(wrapper.error.is_some());
    }

    #[test]
    fn test_catch_panic() {
        let result = catch_panic_as_ffi_error(|| {
            panic!("test panic");
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test panic"));
    }

    #[test]
    fn test_catch_panic_success() {
        let result = catch_panic_as_ffi_error(|| 42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_map_error() {
        let ok: Result<i32, String> = Ok(42);
        let result = map_error(ok, "context");
        assert_eq!(result.unwrap(), 42);

        let err: Result<i32, String> = Err("error message".to_string());
        let result = map_error(err, "context");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("context"));
        assert!(err_msg.contains("error message"));
    }

    #[test]
    fn test_from_app_error() {
        use crate::error::AppError;

        let app_err = AppError::NotFound("test".to_string());
        let ffi_err: FfiError = app_err.into();
        assert!(matches!(ffi_err, FfiError::NotFound { .. }));

        let app_err = AppError::Search {
            _message: "search failed".to_string(),
            _source: None,
        };
        let ffi_err: FfiError = app_err.into();
        assert!(matches!(ffi_err, FfiError::Search { .. }));
    }
}
