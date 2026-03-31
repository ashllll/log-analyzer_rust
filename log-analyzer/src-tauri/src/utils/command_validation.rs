//! 命令参数验证工具
//!
//! 提供commands/目录下各命令共享的验证逻辑，避免重复代码。

use once_cell::sync::Lazy;
use regex::Regex;

/// 工作区ID正则表达式
/// 只允许字母数字、连字符和下划线
pub static WORKSPACE_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-_]+$").unwrap());

/// 搜索查询最大长度
pub const MAX_SEARCH_QUERY_LENGTH: usize = 1000;

/// 工作区ID最大长度
pub const MAX_WORKSPACE_ID_LENGTH: usize = 100;

/// 路径最大长度
pub const MAX_PATH_LENGTH: usize = 500;

/// 验证搜索查询
///
/// 检查查询是否为空、长度是否超限
pub fn validate_search_query(query: &str) -> Result<(), String> {
    if query.is_empty() {
        return Err("搜索查询不能为空".to_string());
    }
    if query.len() > MAX_SEARCH_QUERY_LENGTH {
        return Err(format!(
            "搜索查询过长（最大{}字符）",
            MAX_SEARCH_QUERY_LENGTH
        ));
    }
    Ok(())
}

/// 验证工作区ID格式
///
/// 检查ID是否为空、长度是否超限、是否包含非法字符
pub fn validate_workspace_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("工作区ID不能为空".to_string());
    }
    if id.len() > MAX_WORKSPACE_ID_LENGTH {
        return Err(format!(
            "工作区ID过长（最大{}字符）",
            MAX_WORKSPACE_ID_LENGTH
        ));
    }
    if !WORKSPACE_ID_REGEX.is_match(id) {
        return Err(
            "工作区ID只能包含字母数字、连字符和下划线".to_string(),
        );
    }
    Ok(())
}

/// 验证路径参数
///
/// 检查路径是否为空、长度是否超限
pub fn validate_path_param(path: &str, param_name: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err(format!("{}不能为空", param_name));
    }
    if path.len() > MAX_PATH_LENGTH {
        return Err(format!(
            "{}过长（最大{}字符）",
            param_name, MAX_PATH_LENGTH
        ));
    }
    Ok(())
}

/// 验证导出路径安全性
///
/// 检查路径是否包含路径遍历模式
pub fn validate_export_path(path: &str) -> Result<(), String> {
    let path_obj = std::path::Path::new(path);
    if path_obj
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err("导出路径包含非法路径遍历 (..)".to_string());
    }
    Ok(())
}

/// 验证端口号范围
///
/// 检查端口是否在有效范围内（1-65535）
pub fn validate_port(port: u64, field_name: &str) -> Result<(), String> {
    if port == 0 || port > 65535 {
        return Err(format!("{}必须在 1-65535 之间", field_name));
    }
    Ok(())
}

/// 验证数值范围
///
/// 检查数值是否在指定范围内
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    field_name: &str,
) -> Result<(), String> {
    if value < min || value > max {
        return Err(format!(
            "{}必须在 {}-{} 之间",
            field_name, min, max
        ));
    }
    Ok(())
}

/// 验证日志级别
///
/// 检查日志级别是否为有效值
pub fn validate_log_level(level: &str) -> Result<(), String> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Err(format!(
            "无效的日志级别，必须是以下之一: {:?}",
            valid_levels
        ));
    }
    Ok(())
}

/// 验证WebSocket URL
///
/// 检查URL是否以ws://或wss://开头
pub fn validate_websocket_url(url: &str) -> Result<(), String> {
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err("WebSocket URL 必须以 ws:// 或 wss:// 开头".to_string());
    }
    Ok(())
}

/// 验证API密钥长度
///
/// 检查API密钥长度是否满足最小要求
pub fn validate_api_key(key: &str, min_length: usize) -> Result<(), String> {
    if !key.is_empty() && key.len() < min_length {
        return Err(format!("API密钥长度至少为 {} 个字符", min_length));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_search_query() {
        // 有效查询
        assert!(validate_search_query("test").is_ok());
        assert!(validate_search_query("hello world").is_ok());

        // 空查询
        assert!(validate_search_query("").is_err());

        // 过长查询
        let long_query = "a".repeat(MAX_SEARCH_QUERY_LENGTH + 1);
        assert!(validate_search_query(&long_query).is_err());
    }

    #[test]
    fn test_validate_workspace_id() {
        // 有效ID
        assert!(validate_workspace_id("workspace-123").is_ok());
        assert!(validate_workspace_id("test_id").is_ok());

        // 空ID
        assert!(validate_workspace_id("").is_err());

        // 非法字符
        assert!(validate_workspace_id("test@id").is_err());
        assert!(validate_workspace_id("test id").is_err());

        // 过长ID
        let long_id = "a".repeat(MAX_WORKSPACE_ID_LENGTH + 1);
        assert!(validate_workspace_id(&long_id).is_err());
    }

    #[test]
    fn test_validate_path_param() {
        // 有效路径
        assert!(validate_path_param("/path/to/file", "path").is_ok());

        // 空路径
        assert!(validate_path_param("", "path").is_err());

        // 过长路径
        let long_path = "a".repeat(MAX_PATH_LENGTH + 1);
        assert!(validate_path_param(&long_path, "path").is_err());
    }

    #[test]
    fn test_validate_export_path() {
        // 有效路径
        assert!(validate_export_path("/path/to/file").is_ok());
        assert!(validate_export_path("file.txt").is_ok());

        // 包含路径遍历
        assert!(validate_export_path("../file.txt").is_err());
        assert!(validate_export_path("/path/../file.txt").is_err());
    }

    #[test]
    fn test_validate_port() {
        // 有效端口
        assert!(validate_port(1, "port").is_ok());
        assert!(validate_port(8080, "port").is_ok());
        assert!(validate_port(65535, "port").is_ok());

        // 无效端口
        assert!(validate_port(0, "port").is_err());
        assert!(validate_port(65536, "port").is_err());
    }

    #[test]
    fn test_validate_range() {
        // 有效值
        assert!(validate_range(50u64, 1u64, 100u64, "value").is_ok());
        assert!(validate_range(1u64, 1u64, 100u64, "value").is_ok());
        assert!(validate_range(100u64, 1u64, 100u64, "value").is_ok());

        // 无效值
        assert!(validate_range(0u64, 1u64, 100u64, "value").is_err());
        assert!(validate_range(101u64, 1u64, 100u64, "value").is_err());
    }

    #[test]
    fn test_validate_log_level() {
        // 有效级别
        assert!(validate_log_level("trace").is_ok());
        assert!(validate_log_level("DEBUG").is_ok());
        assert!(validate_log_level("Info").is_ok());

        // 无效级别
        assert!(validate_log_level("invalid").is_err());
        assert!(validate_log_level("").is_err());
    }

    #[test]
    fn test_validate_websocket_url() {
        // 有效URL
        assert!(validate_websocket_url("ws://localhost:8080").is_ok());
        assert!(validate_websocket_url("wss://secure.example.com").is_ok());

        // 无效URL
        assert!(validate_websocket_url("http://localhost:8080").is_err());
        assert!(validate_websocket_url("https://example.com").is_err());
    }

    #[test]
    fn test_validate_api_key() {
        // 空密钥（允许）
        assert!(validate_api_key("", 16).is_ok());

        // 有效密钥
        assert!(validate_api_key(&"a".repeat(16), 16).is_ok());

        // 过短密钥
        assert!(validate_api_key(&"a".repeat(15), 16).is_err());
    }
}
