//! 参数验证工具
//!
//! 提供路径、工作区ID等参数的验证功能。

use std::path::PathBuf;

/// 验证路径参数
///
/// 检查路径是否非空且存在。
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
    if path.trim().is_empty() {
        return Err(format!("Parameter '{}' cannot be empty", param_name));
    }

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
