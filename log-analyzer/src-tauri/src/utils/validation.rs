//! 参数验证工具
//!
//! 提供路径、工作区ID等参数的验证功能。

use std::path::PathBuf;

/// 验证路径参数
///
/// 检查路径是否非空、存在、安全且有效。
///
/// # 参数
///
/// - `path` - 要验证的路径字符串
/// - `param_name` - 参数名称（用于错误消息）
///
/// # 返回值
///
/// - `Ok(PathBuf)` - 验证通过，返回路径
/// - `Err(String)` - 验证失败，返回错误信息
///
/// # 示例
///
/// ```ignore
/// let path = validate_path_param("/path/to/file", "input_path")?;
/// ```
pub fn validate_path_param(path: &str, param_name: &str) -> Result<PathBuf, String> {
    // 1. 检查路径是否为空
    if path.trim().is_empty() {
        return Err(format!("Parameter '{}' cannot be empty", param_name));
    }

    // 2. 检查路径长度
    if path.len() > 1024 {
        return Err(format!(
            "Parameter '{}' path too long (max 1024 characters)",
            param_name
        ));
    }

    // 3. 检查路径是否包含非法字符（防止路径遍历攻击）
    if path.contains("../")
        || path.contains("..\\")
        || path.starts_with("../")
        || path.starts_with("..\\")
    {
        return Err(format!(
            "Parameter '{}' contains invalid path traversal sequences",
            param_name
        ));
    }

    // 4. 检查路径是否存在
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    Ok(path_buf)
}

/// 验证工作区ID参数
///
/// 检查工作区ID是否非空且不包含非法字符（防止路径穿越攻击）。
///
/// # 参数
///
/// - `workspace_id` - 要验证的工作区ID
///
/// # 返回值
///
/// - `Ok(())` - 验证通过
/// - `Err(String)` - 验证失败，返回错误信息
///
/// # 示例
///
/// ```ignore
/// validate_workspace_id("workspace-123")?;
/// ```
pub fn validate_workspace_id(workspace_id: &str) -> Result<(), String> {
    if workspace_id.trim().is_empty() {
        return Err("Workspace ID cannot be empty".to_string());
    }

    // 检查是否包含非法字符（避免路径穿越）
    if workspace_id.contains("..") || workspace_id.contains('/') || workspace_id.contains('\\') {
        return Err("Workspace ID contains invalid characters".to_string());
    }

    Ok(())
}

/// TODO: 需要添加符号链接解析和验证
/// 当前函数仅检查路径字符串，未处理符号链接。
/// 建议添加以下功能：
/// 1. 解析符号链接并获取最终路径
/// 2. 验证最终路径是否在允许的根目录内
/// 3. 添加配置允许的根目录列表
///
/// 示例实现：
/// ```rust
/// pub fn validate_path_with_symlinks(path: &str, allowed_roots: &[&str]) -> Result<PathBuf, String> {
///     let path_buf = PathBuf::from(path);
///
///     // 解析符号链接
///     let canonical_path = path_buf.canonicalize().map_err(|e| {
///         format!("Failed to canonicalize path {}: {}", path, e)
///     })?;
///
///     // 检查是否在允许的根目录内
///     let is_allowed = allowed_roots.iter().any(|root| {
///         canonical_path.starts_with(root)
///     });
///
///     if !is_allowed {
///         return Err(format!(
///             "Path '{}' is outside allowed directories",
///             path
///         ));
///     }
///
///     Ok(canonical_path)
/// }
/// ```
