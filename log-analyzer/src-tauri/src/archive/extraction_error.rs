//! 提取错误类型模块
//!
//! 提供统一的错误枚举用于归档提取操作，使用 thiserror 实现结构化的错误处理。

use std::path::PathBuf;
use thiserror::Error;

/// 提取操作错误类型
///
/// 封装所有可能发生在归档提取过程中的错误，包括限制检查失败、
/// IO错误、格式错误等。
#[derive(Error, Debug, Clone)]
pub enum ExtractionError {
    /// 文件大小超过限制
    #[error("文件大小超过限制: {size} bytes > {limit} bytes")]
    FileTooLarge { size: u64, limit: u64 },

    /// 总大小超过限制
    #[error("总大小超过限制: {total} bytes > {limit} bytes")]
    TotalSizeExceeded { total: u64, limit: u64 },

    /// 文件数量超过限制
    #[error("文件数量超过限制: {count} > {limit}")]
    FileCountExceeded { count: usize, limit: usize },

    /// 嵌套深度超过限制
    #[error("嵌套深度超过限制: {depth} > {max_depth}")]
    DepthExceeded { depth: u32, max_depth: u32 },

    /// 路径安全违规
    #[error("路径安全违规: {path} - {reason}")]
    PathSecurityViolation { path: String, reason: String },

    /// 不支持的压缩格式
    #[error("不支持的压缩格式: {format}")]
    UnsupportedFormat { format: String },

    /// 文件未找到
    #[error("文件未找到: {path}")]
    FileNotFound { path: PathBuf },

    /// 路径不是文件
    #[error("路径不是文件: {path}")]
    NotAFile { path: PathBuf },

    /// 目标目录创建失败
    #[error("无法创建目标目录: {path} - {reason}")]
    DirectoryCreationFailed { path: PathBuf, reason: String },

    /// IO错误
    #[error("IO错误: {operation} - {reason}")]
    IoError { operation: String, reason: String },

    /// 压缩包损坏
    #[error("压缩包损坏: {path} - {reason}")]
    ArchiveCorrupted { path: PathBuf, reason: String },

    /// 密码保护
    #[error("压缩包受密码保护: {path}")]
    PasswordProtected { path: PathBuf },

    /// 提取失败
    #[error("提取失败: {path} - {reason}")]
    ExtractionFailed { path: PathBuf, reason: String },

    /// 编码错误
    #[error("文件名编码错误: {filename}")]
    FilenameEncodingError { filename: String },

    /// 符号链接被禁止
    #[error("符号链接被禁止: {path}")]
    SymlinkNotAllowed { path: PathBuf },

    /// 绝对路径被禁止
    #[error("绝对路径被禁止: {path}")]
    AbsolutePathNotAllowed { path: String },

    /// 父目录遍历被禁止
    #[error("父目录遍历被禁止: {path}")]
    ParentTraversalNotAllowed { path: String },

    /// 文件扩展名被禁止
    #[error("文件扩展名被禁止: {extension}")]
    ExtensionForbidden { extension: String },

    /// 文件扩展名不在白名单
    #[error("文件扩展名不在白名单: {extension}")]
    ExtensionNotAllowed { extension: String },

    /// 路径在黑名单中
    #[error("路径在黑名单中: {path}")]
    PathBlacklisted { path: String },

    /// 配置错误
    #[error("配置错误: {message}")]
    ConfigError { message: String },

    /// 其他错误
    #[error("提取错误: {message}")]
    Other { message: String },
}

impl ExtractionError {
    /// 创建文件过大错误
    pub fn file_too_large(size: u64, limit: u64) -> Self {
        Self::FileTooLarge { size, limit }
    }

    /// 创建总大小超限错误
    pub fn total_size_exceeded(total: u64, limit: u64) -> Self {
        Self::TotalSizeExceeded { total, limit }
    }

    /// 创建文件数量超限错误
    pub fn file_count_exceeded(count: usize, limit: usize) -> Self {
        Self::FileCountExceeded { count, limit }
    }

    /// 创建深度超限错误
    pub fn depth_exceeded(depth: u32, max_depth: u32) -> Self {
        Self::DepthExceeded { depth, max_depth }
    }

    /// 创建路径安全违规错误
    pub fn path_security_violation(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PathSecurityViolation {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// 创建不支持的格式错误
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }

    /// 创建文件未找到错误
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// 创建提取失败错误
    pub fn extraction_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ExtractionFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// 创建IO错误
    pub fn io_error(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::IoError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// 创建配置错误
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// 从IO错误转换
    pub fn from_io_error(operation: impl Into<String>, err: std::io::Error) -> Self {
        Self::IoError {
            operation: operation.into(),
            reason: err.to_string(),
        }
    }

    /// 获取错误分类（用于日志和监控）
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::FileTooLarge { .. } => ErrorCategory::LimitExceeded,
            Self::TotalSizeExceeded { .. } => ErrorCategory::LimitExceeded,
            Self::FileCountExceeded { .. } => ErrorCategory::LimitExceeded,
            Self::DepthExceeded { .. } => ErrorCategory::LimitExceeded,
            Self::PathSecurityViolation { .. } => ErrorCategory::Security,
            Self::SymlinkNotAllowed { .. } => ErrorCategory::Security,
            Self::AbsolutePathNotAllowed { .. } => ErrorCategory::Security,
            Self::ParentTraversalNotAllowed { .. } => ErrorCategory::Security,
            Self::ExtensionForbidden { .. } => ErrorCategory::Security,
            Self::ExtensionNotAllowed { .. } => ErrorCategory::Security,
            Self::PathBlacklisted { .. } => ErrorCategory::Security,
            Self::UnsupportedFormat { .. } => ErrorCategory::Format,
            Self::ArchiveCorrupted { .. } => ErrorCategory::Format,
            Self::PasswordProtected { .. } => ErrorCategory::Security,
            Self::FilenameEncodingError { .. } => ErrorCategory::Format,
            Self::FileNotFound { .. } => ErrorCategory::Io,
            Self::NotAFile { .. } => ErrorCategory::Io,
            Self::DirectoryCreationFailed { .. } => ErrorCategory::Io,
            Self::IoError { .. } => ErrorCategory::Io,
            Self::ExtractionFailed { .. } => ErrorCategory::Extraction,
            Self::ConfigError { .. } => ErrorCategory::Config,
            Self::Other { .. } => ErrorCategory::Other,
        }
    }

    /// 检查错误是否与限制相关
    pub fn is_limit_error(&self) -> bool {
        matches!(
            self,
            Self::FileTooLarge { .. }
                | Self::TotalSizeExceeded { .. }
                | Self::FileCountExceeded { .. }
                | Self::DepthExceeded { .. }
        )
    }

    /// 检查错误是否与安全相关
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Self::PathSecurityViolation { .. }
                | Self::SymlinkNotAllowed { .. }
                | Self::AbsolutePathNotAllowed { .. }
                | Self::ParentTraversalNotAllowed { .. }
                | Self::ExtensionForbidden { .. }
                | Self::ExtensionNotAllowed { .. }
                | Self::PathBlacklisted { .. }
                | Self::PasswordProtected { .. }
        )
    }

    /// 检查错误是否可恢复（跳过当前文件继续处理）
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::FileTooLarge { .. }
                | Self::PathSecurityViolation { .. }
                | Self::FilenameEncodingError { .. }
                | Self::SymlinkNotAllowed { .. }
                | Self::AbsolutePathNotAllowed { .. }
                | Self::ParentTraversalNotAllowed { .. }
                | Self::ExtensionForbidden { .. }
                | Self::ExtensionNotAllowed { .. }
                | Self::PathBlacklisted { .. }
        )
    }
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 限制超限
    LimitExceeded,
    /// 安全问题
    Security,
    /// 格式错误
    Format,
    /// IO错误
    Io,
    /// 提取错误
    Extraction,
    /// 配置错误
    Config,
    /// 其他错误
    Other,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LimitExceeded => write!(f, "LimitExceeded"),
            Self::Security => write!(f, "Security"),
            Self::Format => write!(f, "Format"),
            Self::Io => write!(f, "Io"),
            Self::Extraction => write!(f, "Extraction"),
            Self::Config => write!(f, "Config"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// 提取结果类型别名
pub type ExtractionResult<T> = Result<T, ExtractionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_too_large_error() {
        let err = ExtractionError::file_too_large(200, 100);
        assert_eq!(err.to_string(), "文件大小超过限制: 200 bytes > 100 bytes");
        assert!(err.is_limit_error());
        assert!(err.is_recoverable());
        assert_eq!(err.category(), ErrorCategory::LimitExceeded);
    }

    #[test]
    fn test_total_size_exceeded_error() {
        let err = ExtractionError::total_size_exceeded(2000, 1000);
        assert_eq!(err.to_string(), "总大小超过限制: 2000 bytes > 1000 bytes");
        assert!(err.is_limit_error());
    }

    #[test]
    fn test_file_count_exceeded_error() {
        let err = ExtractionError::file_count_exceeded(150, 100);
        assert_eq!(err.to_string(), "文件数量超过限制: 150 > 100");
        assert!(err.is_limit_error());
    }

    #[test]
    fn test_depth_exceeded_error() {
        let err = ExtractionError::depth_exceeded(10, 5);
        assert_eq!(err.to_string(), "嵌套深度超过限制: 10 > 5");
        assert!(err.is_limit_error());
    }

    #[test]
    fn test_path_security_violation_error() {
        let err = ExtractionError::path_security_violation("../etc/passwd", "路径遍历");
        assert!(err.to_string().contains("路径安全违规"));
        assert!(err.is_security_error());
        assert!(err.is_recoverable());
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_unsupported_format_error() {
        let err = ExtractionError::unsupported_format("unknown");
        assert_eq!(err.to_string(), "不支持的压缩格式: unknown");
        assert_eq!(err.category(), ErrorCategory::Format);
    }

    #[test]
    fn test_io_error() {
        let err = ExtractionError::io_error("read", "permission denied");
        assert!(err.to_string().contains("IO错误"));
        assert_eq!(err.category(), ErrorCategory::Io);
    }

    #[test]
    fn test_security_errors() {
        let symlink_err = ExtractionError::SymlinkNotAllowed {
            path: PathBuf::from("/test/link"),
        };
        assert!(symlink_err.is_security_error());
        assert!(symlink_err.is_recoverable());

        let abs_err = ExtractionError::AbsolutePathNotAllowed {
            path: "/absolute/path".to_string(),
        };
        assert!(abs_err.is_security_error());

        let traversal_err = ExtractionError::ParentTraversalNotAllowed {
            path: "../test".to_string(),
        };
        assert!(traversal_err.is_security_error());
        assert!(traversal_err.is_recoverable());
    }

    #[test]
    fn test_config_error() {
        let err = ExtractionError::config_error("invalid max_depth");
        assert!(err.to_string().contains("配置错误"));
        assert_eq!(err.category(), ErrorCategory::Config);
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_result_type() {
        fn may_fail() -> ExtractionResult<u32> {
            Ok(42)
        }

        fn will_fail() -> ExtractionResult<u32> {
            Err(ExtractionError::config_error("test error"))
        }

        assert_eq!(may_fail().unwrap(), 42);
        assert!(will_fail().is_err());
    }
}
