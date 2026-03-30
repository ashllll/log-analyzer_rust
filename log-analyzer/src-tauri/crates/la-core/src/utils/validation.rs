//! 输入验证工具
//!
//! 提供路径安全验证功能（la-core 本地副本）

use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use validator::ValidationError;

/// 验证路径安全性
///
/// 检查路径是否包含路径遍历攻击模式
pub fn validate_safe_path(path: &str) -> Result<(), ValidationError> {
    // 规范化 Unicode
    let normalized: String = path.nfc().collect();

    // 检查路径遍历模式
    if normalized.contains("..") {
        return Err(ValidationError::new("path_traversal"));
    }

    // 检查绝对路径标记 (Unix 和 Windows)
    if normalized.starts_with('/') || normalized.contains(":\\") {
        // 绝对路径是允许的,但需要进一步验证
    }

    // 检查 null 字节
    if normalized.contains('\0') {
        return Err(ValidationError::new("null_byte"));
    }

    // 验证路径可以被解析
    if Path::new(&normalized).components().count() == 0 {
        return Err(ValidationError::new("invalid_path"));
    }

    Ok(())
}
