//! 输入验证工具
//!
//! 提供路径安全、工作区 ID 等验证功能

use once_cell::sync::Lazy;
use regex::Regex;
use sanitize_filename::sanitize;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use validator::ValidationError;

/// 工作区 ID 正则表达式 - 只允许字母数字、连字符和下划线
/// 使用 Lazy<Regex> 避免启动时 panic，使用 unwrap 因为正则表达式在编译时已知是有效的
pub static WORKSPACE_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-_]+$").unwrap());

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

/// 全面的路径遍历攻击防护
///
/// 检测各种路径遍历攻击模式，包括：
/// - 基本的 .. 模式
/// - URL 编码的路径遍历（如 %2e%2e）
/// - 双重 URL 编码（如 %252e%252e）
/// - Unicode 规范化绕过
pub fn prevent_path_traversal(path: &str) -> Result<String, String> {
    // Unicode NFC 规范化 - 防止使用不同 Unicode 表示的相同字符绕过检查
    let normalized: String = path.nfc().collect();

    // URL 解码 - 防止编码后的路径遍历攻击
    let decoded = url_decode(&normalized);

    // 再次 NFC 规范化（解码后可能产生新的组合字符）
    let decoded_normalized: String = decoded.nfc().collect();

    // 检查常见的路径遍历模式（在解码后的路径上检查）
    let dangerous_patterns = [
        "..",
        "/../",
        "\\..\\",
        "%2e%2e",
        "%252e%252e",
        "..%2f",
        "..%5c",
        "%2e%2e/",
        "%2e%2e\\",
    ];

    // 在原始路径、解码后的路径上都进行检查
    for pattern in &dangerous_patterns {
        if normalized.to_lowercase().contains(pattern)
            || decoded_normalized.to_lowercase().contains(pattern)
        {
            return Err(format!("Path traversal pattern detected: {pattern}"));
        }
    }

    // 检查解码后的路径中的 .. 模式（捕获任何 URL 编码变体）
    if decoded_normalized.contains("..") {
        return Err("Path traversal pattern detected: decoded ..".to_string());
    }

    // 检查 null 字节注入
    if normalized.contains('\0') || decoded_normalized.contains('\0') {
        return Err("Null byte injection detected".to_string());
    }

    // 检查控制字符
    if normalized
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err("Control characters detected in path".to_string());
    }

    // 返回解码并规范化后的路径
    Ok(decoded_normalized)
}

/// URL 解码辅助函数
///
/// 对字符串进行 URL 解码，正确处理 UTF-8 多字节序列（如 %E4%B8%AD → 中）
fn url_decode(input: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if matches!(chars.peek(), Some('u' | 'U')) {
                let prefix = chars.next().unwrap_or('u');
                let mut hex = String::with_capacity(4);
                for _ in 0..4 {
                    if let Some(h) = chars.next() {
                        hex.push(h);
                    } else {
                        bytes.push(b'%');
                        bytes.push(prefix as u8);
                        bytes.extend_from_slice(hex.as_bytes());
                        return String::from_utf8_lossy(&bytes).into_owned();
                    }
                }

                if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                    if let Some(decoded) = char::from_u32(code_point) {
                        let mut buf = [0u8; 4];
                        bytes.extend_from_slice(decoded.encode_utf8(&mut buf).as_bytes());
                        continue;
                    }
                }

                bytes.push(b'%');
                bytes.push(prefix as u8);
                bytes.extend_from_slice(hex.as_bytes());
                continue;
            }

            let hex1 = chars.next();
            let hex2 = chars.next();

            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                let hex_str = format!("{h1}{h2}");
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    bytes.push(byte);
                } else {
                    // 无效的十六进制，保留原始序列
                    bytes.push(b'%');
                    bytes.push(h1 as u8);
                    bytes.push(h2 as u8);
                }
            } else {
                // 不完整的 % 序列，保留 %
                bytes.push(b'%');
                if let Some(h1) = hex1 {
                    bytes.push(h1 as u8);
                }
            }
        } else if c == '+' {
            // URL 编码中 + 表示空格
            bytes.push(b' ');
        } else {
            // 对非 ASCII 字符直接转为 UTF-8 字节
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }

    // 将字节序列解码为 UTF-8，替换无效序列为替换字符
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 路径规范化和安全验证（Base Directory Jail 模式）
///
/// 将路径转换为规范形式并验证安全性。采用 Base Directory Jail 模式：
/// 1. 先规范化基础目录
/// 2. 在基础目录内拼接目标路径
/// 3. 规范化目标路径（解析符号链接和相对路径）
/// 4. 验证目标路径仍以基础目录为前缀，防止符号链接逃逸
///
/// # FIX(CR-05)
/// 此前的实现直接调用 `dunce::canonicalize()` 解析符号链接，
/// 导致目标路径可能通过符号链接逃逸出预期目录。
/// 现在强制要求提供 `base_dir`，并在规范化后做前缀校验。
pub fn canonicalize_and_validate(path: &str, base_dir: &str) -> Result<std::path::PathBuf, String> {
    // 防止路径遍历（原始输入）
    let safe_path = prevent_path_traversal(path)?;

    // 规范化基础目录
    let canonical_base = dunce::canonicalize(base_dir)
        .map_err(|e| format!("Failed to canonicalize base directory: {e}"))?;

    // 在基础目录内拼接目标路径（禁止直接拼接未经规范化的绝对路径）
    let target_path = canonical_base.join(&safe_path);

    // 规范化目标路径（解析符号链接和相对路径）
    let canonical_target = dunce::canonicalize(&target_path)
        .map_err(|e| format!("Failed to canonicalize path: {e}"))?;

    // FIX(CR-05): 验证规范化后的路径仍在基础目录内，防止符号链接逃逸
    if !canonical_target.starts_with(&canonical_base) {
        return Err("Path escapes base directory after canonicalization".to_string());
    }

    // 再次验证规范化后的路径不包含危险模式
    let canonical_str = canonical_target.to_string_lossy();
    prevent_path_traversal(&canonical_str)?;

    Ok(canonical_target)
}

/// 清理文件名
///
/// 移除不安全字符并规范化 Unicode
#[allow(dead_code)]
pub fn sanitize_file_name(name: &str) -> String {
    // Unicode 规范化
    let normalized: String = name.nfc().collect();

    // 使用 sanitize-filename 清理
    sanitize(&normalized)
}

/// 高级文件名清理
///
/// 提供更严格的文件名清理，包括长度限制和字符白名单
pub fn sanitize_file_name_strict(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    // Unicode 规范化
    let normalized: String = name.nfc().collect();

    // 使用 sanitize-filename 清理
    let sanitized = sanitize(&normalized);

    if sanitized.is_empty() {
        return Err("Filename contains only invalid characters".to_string());
    }

    // 限制文件名长度（大多数文件系统限制为 255 字节）
    if sanitized.len() > 255 {
        return Err("Filename too long (max 255 characters)".to_string());
    }

    // 检查保留名称（Windows）
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let name_upper = sanitized.to_uppercase();
    let name_without_ext = name_upper.split('.').next().unwrap_or("");

    if reserved_names.contains(&name_without_ext) {
        return Err(format!("Reserved filename: {name_without_ext}"));
    }

    Ok(sanitized)
}

/// 批量清理文件名
///
/// 清理多个文件名并确保没有重复
pub fn sanitize_file_names_batch(names: &[String]) -> Result<Vec<String>, String> {
    let mut sanitized_names = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (index, name) in names.iter().enumerate() {
        let mut sanitized = sanitize_file_name_strict(name)?;

        // 处理重复名称
        let mut counter = 1;
        let original_sanitized = sanitized.clone();
        while seen.contains(&sanitized) {
            // 添加数字后缀
            let parts: Vec<&str> = original_sanitized.rsplitn(2, '.').collect();
            if parts.len() == 2 {
                sanitized = format!("{}_{}.{}", parts[1], counter, parts[0]);
            } else {
                sanitized = format!("{original_sanitized}_{counter}");
            }
            counter += 1;

            if counter > 1000 {
                return Err(format!("Too many duplicate filenames at index {index}"));
            }
        }

        seen.insert(sanitized.clone());
        sanitized_names.push(sanitized);
    }

    Ok(sanitized_names)
}

/// 搜索查询最大长度
pub const MAX_SEARCH_QUERY_LENGTH: usize = 1000;

/// 工作区ID最大长度
pub const MAX_WORKSPACE_ID_LENGTH: usize = 50;

/// 路径最大长度
pub const MAX_PATH_LENGTH: usize = 500;

/// 验证工作区 ID
///
/// 检查ID是否为空、长度是否超限、是否包含非法字符
pub fn validate_workspace_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("工作区ID不能为空".to_string());
    }
    if id.len() > MAX_WORKSPACE_ID_LENGTH {
        return Err(format!("工作区ID过长（最大{MAX_WORKSPACE_ID_LENGTH}字符）"));
    }
    if !WORKSPACE_ID_REGEX.is_match(id) {
        return Err("工作区ID只能包含字母数字、连字符和下划线".to_string());
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safe_path() {
        // 有效路径
        assert!(validate_safe_path("/valid/path").is_ok());
        assert!(validate_safe_path("relative/path").is_ok());

        // 路径遍历攻击
        assert!(validate_safe_path("../etc/passwd").is_err());
        assert!(validate_safe_path("/path/../../../etc").is_err());

        // Null 字节
        assert!(validate_safe_path("/path\0/file").is_err());
    }

    #[test]
    fn test_prevent_path_traversal() {
        // 有效路径
        assert!(prevent_path_traversal("/valid/path").is_ok());
        assert!(prevent_path_traversal("relative/path").is_ok());

        // 各种路径遍历攻击模式
        assert!(prevent_path_traversal("../etc/passwd").is_err());
        assert!(prevent_path_traversal("/path/../../../etc").is_err());
        assert!(prevent_path_traversal("path/..%2f..%2fetc").is_err());
        assert!(prevent_path_traversal("path%2e%2e/etc").is_err());

        // Null 字节注入
        assert!(prevent_path_traversal("/path\0/file").is_err());

        // 控制字符
        assert!(prevent_path_traversal("/path/\x01file").is_err());

        // URL 编码的路径遍历攻击 - 新增测试
        assert!(prevent_path_traversal("%2e%2e/%2e%2e/etc/passwd").is_err());
        assert!(prevent_path_traversal("%252e%252e/%252e%252e/etc").is_err());
        assert!(prevent_path_traversal("path/%2e%2e%2f%2e%2e%2fetc").is_err());
        assert!(prevent_path_traversal("..%2f..%2fsecret.txt").is_err());
        assert!(prevent_path_traversal("..%5c..%5cwindows%5csystem32").is_err());

        // 双重编码攻击
        assert!(prevent_path_traversal("%252e%252e/%252e%252e/etc/passwd").is_err());

        // 混合编码攻击
        assert!(prevent_path_traversal("%2e.%2f/etc/passwd").is_err());
        assert!(prevent_path_traversal("%u002e%u002e/%u002e%u002e/etc").is_err());
    }

    #[test]
    fn test_url_decode() {
        // 基本解码
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("path%2fto%2ffile"), "path/to/file");
        assert_eq!(url_decode("path%5cfile"), r"path\file");

        // + 号解码为空格
        assert_eq!(url_decode("hello+world"), "hello world");

        // 路径遍历编码解码
        assert_eq!(url_decode("%2e%2e"), "..");
        assert_eq!(url_decode("%2e%2e%2f"), "../");
        assert_eq!(url_decode("%252e%252e"), "%2e%2e");
        assert_eq!(url_decode("%u4E2D%u6587"), "中文");
        assert_eq!(url_decode("%u002e%u002e"), "..");

        // 无编码的字符串保持不变
        assert_eq!(url_decode("normal/path"), "normal/path");
        assert_eq!(url_decode(""), "");

        // 不完整的编码序列保留原样
        assert_eq!(url_decode("path%2"), "path%2");
        assert_eq!(url_decode("path%"), "path%");

        // 无效的十六进制保留原样
        assert_eq!(url_decode("path%GG"), "path%GG");
        assert_eq!(url_decode("path%2G"), "path%2G");
        assert_eq!(url_decode("path%u00GG"), "path%u00GG");
    }

    #[test]
    fn test_sanitize_file_name() {
        assert_eq!(sanitize_file_name("normal.txt"), "normal.txt");
        assert_eq!(sanitize_file_name("file:with:colons"), "filewithcolons");
        // sanitize-filename 的行为可能因版本而异，只检查结果不包含危险字符
        let result = sanitize_file_name("../../../etc/passwd");
        assert!(!result.contains('/'));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_sanitize_file_name_strict() {
        // 有效文件名
        assert!(sanitize_file_name_strict("normal.txt").is_ok());
        assert!(sanitize_file_name_strict("file-name_123.log").is_ok());

        // 空文件名
        assert!(sanitize_file_name_strict("").is_err());

        // 保留名称（Windows）
        assert!(sanitize_file_name_strict("CON").is_err());
        assert!(sanitize_file_name_strict("con.txt").is_err());
        assert!(sanitize_file_name_strict("PRN").is_err());
        assert!(sanitize_file_name_strict("AUX.log").is_err());

        // 特殊字符应该被清理
        let result = sanitize_file_name_strict("file:with:colons.txt").unwrap();
        assert!(!result.contains(':'));
    }

    #[test]
    fn test_sanitize_file_names_batch() {
        // 无重复
        let names = vec![
            "file1.txt".to_string(),
            "file2.txt".to_string(),
            "file3.txt".to_string(),
        ];
        let result = sanitize_file_names_batch(&names).unwrap();
        assert_eq!(result.len(), 3);

        // 有重复
        let names_dup = vec![
            "file.txt".to_string(),
            "file.txt".to_string(),
            "file.txt".to_string(),
        ];
        let result_dup = sanitize_file_names_batch(&names_dup).unwrap();
        assert_eq!(result_dup.len(), 3);
        assert_ne!(result_dup[0], result_dup[1]);
        assert_ne!(result_dup[1], result_dup[2]);
    }

    #[test]
    fn test_validate_workspace_id() {
        // 有效 ID
        assert!(validate_workspace_id("valid-id-123").is_ok());
        assert!(validate_workspace_id("workspace_1").is_ok());

        // 无效 ID
        assert!(validate_workspace_id("").is_err());
        assert!(validate_workspace_id("invalid@id!").is_err());
        assert!(validate_workspace_id("id with spaces").is_err());
    }

    #[test]
    fn test_canonicalize_and_validate() {
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.to_string_lossy();

        // 测试当前目录（应该成功）
        let result = canonicalize_and_validate(".", &base);
        assert!(result.is_ok());

        // 测试路径遍历（应该失败）
        let result = canonicalize_and_validate("../../../etc/passwd", &base);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_search_query() {
        assert!(validate_search_query("test").is_ok());
        assert!(validate_search_query("hello world").is_ok());
        assert!(validate_search_query("").is_err());
        let long_query = "a".repeat(MAX_SEARCH_QUERY_LENGTH + 1);
        assert!(validate_search_query(&long_query).is_err());
    }

    #[test]
    fn test_validate_export_path() {
        assert!(validate_export_path("file.txt").is_ok());
        assert!(validate_export_path("path/to/file").is_ok());
        assert!(validate_export_path("/path/to/file").is_err());
        assert!(validate_export_path("../file.txt").is_err());
        assert!(validate_export_path("path/../file.txt").is_err());
        assert!(validate_export_path("file\0.txt").is_err());
        assert!(validate_export_path("%2e%2e/file.txt").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_validate_export_path_windows() {
        assert!(validate_export_path("C:\\windows\\file.txt").is_err());
    }

    #[test]
    fn test_validate_port() {
        assert!(validate_port(1, "port").is_ok());
        assert!(validate_port(8080, "port").is_ok());
        assert!(validate_port(65535, "port").is_ok());
        assert!(validate_port(0, "port").is_err());
        assert!(validate_port(65536, "port").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(50u64, 1u64, 100u64, "value").is_ok());
        assert!(validate_range(1u64, 1u64, 100u64, "value").is_ok());
        assert!(validate_range(100u64, 1u64, 100u64, "value").is_ok());
        assert!(validate_range(0u64, 1u64, 100u64, "value").is_err());
        assert!(validate_range(101u64, 1u64, 100u64, "value").is_err());
    }

    #[test]
    fn test_validate_log_level() {
        assert!(validate_log_level("trace").is_ok());
        assert!(validate_log_level("DEBUG").is_ok());
        assert!(validate_log_level("Info").is_ok());
        assert!(validate_log_level("invalid").is_err());
        assert!(validate_log_level("").is_err());
    }

    #[test]
    fn test_validate_websocket_url() {
        assert!(validate_websocket_url("ws://localhost:8080").is_ok());
        assert!(validate_websocket_url("wss://secure.example.com").is_ok());
        assert!(validate_websocket_url("http://localhost:8080").is_err());
        assert!(validate_websocket_url("https://example.com").is_err());
    }

    #[test]
    fn test_validate_api_key() {
        assert!(validate_api_key("", 16).is_ok());
        assert!(validate_api_key(&"a".repeat(16), 16).is_ok());
        assert!(validate_api_key(&"a".repeat(15), 16).is_err());
    }
}

/// 验证路径参数
///
/// 验证路径参数是否有效并返回规范化的绝对路径
pub fn validate_path_param(path: &str, param_name: &str) -> Result<std::path::PathBuf, String> {
    // 先执行基础验证（空值、长度）
    if path.is_empty() {
        return Err(format!("{param_name}不能为空"));
    }
    if path.len() > MAX_PATH_LENGTH {
        return Err(format!("{param_name}过长（最大{MAX_PATH_LENGTH}字符）"));
    }

    // 验证路径安全性
    validate_safe_path(path).map_err(|e| format!("Invalid {param_name}: {e:?}"))?;

    // 转换为 PathBuf
    let path_buf = std::path::PathBuf::from(path);

    // 规范化路径
    let canonical = dunce::canonicalize(&path_buf)
        .map_err(|e| format!("Failed to canonicalize {param_name}: {e}"))?;

    Ok(canonical)
}

/// 验证搜索查询
///
/// 检查查询是否为空、长度是否超限
pub fn validate_search_query(query: &str) -> Result<(), String> {
    if query.is_empty() {
        return Err("搜索查询不能为空".to_string());
    }
    if query.len() > MAX_SEARCH_QUERY_LENGTH {
        return Err(format!("搜索查询过长（最大{MAX_SEARCH_QUERY_LENGTH}字符）"));
    }
    Ok(())
}

/// 验证导出路径安全性
///
/// FIX(HI-10): 复用 `prevent_path_traversal()` 进行全面的路径安全检查，
/// 覆盖 Null 字节、URL 编码绕过、Unicode 规范化绕过等攻击向量。
/// 同时限制只能使用相对路径，禁止绝对路径。
pub fn validate_export_path(path: &str) -> Result<(), String> {
    // 全面路径遍历检测（包含 URL 解码、Unicode NFC、null 字节、控制字符）
    prevent_path_traversal(path)?;

    // 禁止绝对路径（导出只能使用相对路径）
    let path_obj = std::path::Path::new(path);
    if path_obj.is_absolute() {
        return Err("导出路径必须是相对路径".to_string());
    }
    // 额外拒绝以 / 或 \ 开头的路径（跨平台一致性，Windows 下 /path 被视为相对路径）
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("导出路径必须是相对路径".to_string());
    }

    // 额外的 ParentDir 检查（防御深层路径遍历）
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
        return Err(format!("{field_name}必须在 1-65535 之间"));
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
        return Err(format!("{field_name}必须在 {min}-{max} 之间"));
    }
    Ok(())
}

/// 验证日志级别
///
/// 检查日志级别是否为有效值
pub fn validate_log_level(level: &str) -> Result<(), String> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Err(format!("无效的日志级别，必须是以下之一: {valid_levels:?}"));
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
        return Err(format!("API密钥长度至少为 {min_length} 个字符"));
    }
    Ok(())
}

/// 验证导入源路径。
///
/// 导入源允许用户选择普通日志目录，但拒绝文件系统根目录和常见系统敏感目录。
/// 这避免恶意 IPC 调用把整块系统目录作为工作区导入源。
pub fn validate_import_source_path(
    path: &str,
    param_name: &str,
) -> Result<std::path::PathBuf, String> {
    let canonical = validate_path_param(path, param_name)?;

    if !canonical.exists() {
        return Err(format!("{param_name} does not exist"));
    }
    if !canonical.is_dir() {
        return Err(format!("{param_name} must be a directory"));
    }
    if is_filesystem_root(&canonical) || is_sensitive_system_path(&canonical) {
        return Err(format!(
            "{} points to a protected system location: {}",
            param_name,
            canonical.display()
        ));
    }

    Ok(canonical)
}

fn is_filesystem_root(path: &std::path::Path) -> bool {
    path.parent().is_none()
}

fn is_sensitive_system_path(path: &std::path::Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        let protected_roots = [
            std::env::var_os("WINDIR").map(std::path::PathBuf::from),
            std::env::var_os("SystemRoot").map(std::path::PathBuf::from),
            std::env::var_os("ProgramFiles").map(std::path::PathBuf::from),
            std::env::var_os("ProgramFiles(x86)").map(std::path::PathBuf::from),
            std::env::var_os("ProgramData").map(std::path::PathBuf::from),
        ];

        protected_roots
            .into_iter()
            .flatten()
            .filter_map(|root| dunce::canonicalize(root).ok())
            .any(|root| path == root || path.starts_with(&root))
    }

    #[cfg(not(target_os = "windows"))]
    {
        const PROTECTED_ROOTS: &[&str] = &[
            "/bin",
            "/boot",
            "/dev",
            "/etc",
            "/lib",
            "/lib64",
            "/proc",
            "/root",
            "/run",
            "/sbin",
            "/sys",
            "/usr/bin",
            "/usr/sbin",
        ];

        PROTECTED_ROOTS.iter().any(|root| {
            let root = std::path::Path::new(root);
            path == root || path.starts_with(root)
        })
    }
}

// 属性测试模块已移至 validation/mod.rs
