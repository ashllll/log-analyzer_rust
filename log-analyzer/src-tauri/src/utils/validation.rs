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
            return Err(format!("Path traversal pattern detected: {}", pattern));
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
/// 对字符串进行 URL 解码，处理 %XX 编码的字符
fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // 尝试读取两个十六进制字符
            let hex1 = chars.next();
            let hex2 = chars.next();

            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                let hex_str = format!("{}{}", h1, h2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    // 将字节转换为字符（假设 UTF-8）
                    if let Some(decoded_char) = char::from_u32(byte as u32) {
                        result.push(decoded_char);
                    } else {
                        // 无效的字符，保留原始序列
                        result.push('%');
                        result.push(h1);
                        result.push(h2);
                    }
                } else {
                    // 无效的十六进制，保留原始序列
                    result.push('%');
                    result.push(h1);
                    result.push(h2);
                }
            } else {
                // 不完整的 % 序列，保留 %
                result.push('%');
                if let Some(h1) = hex1 {
                    result.push(h1);
                }
            }
        } else if c == '+' {
            // URL 编码中 + 表示空格
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// 路径规范化和安全验证
///
/// 将路径转换为规范形式并验证安全性
pub fn canonicalize_and_validate(path: &str) -> Result<std::path::PathBuf, String> {
    // 防止路径遍历
    let safe_path = prevent_path_traversal(path)?;

    // 转换为 PathBuf
    let path_buf = std::path::PathBuf::from(&safe_path);

    // 规范化路径（解析符号链接和相对路径）
    let canonical = dunce::canonicalize(&path_buf)
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

    // 验证规范化后的路径不包含危险模式
    let canonical_str = canonical.to_string_lossy();
    prevent_path_traversal(&canonical_str)?;

    Ok(canonical)
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
        return Err(format!("Reserved filename: {}", name_without_ext));
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
                sanitized = format!("{}_{}", original_sanitized, counter);
            }
            counter += 1;

            if counter > 1000 {
                return Err(format!("Too many duplicate filenames at index {}", index));
            }
        }

        seen.insert(sanitized.clone());
        sanitized_names.push(sanitized);
    }

    Ok(sanitized_names)
}

/// 验证工作区 ID
///
/// 委托给 command_validation 模块的统一实现
pub fn validate_workspace_id(id: &str) -> Result<(), String> {
    crate::utils::command_validation::validate_workspace_id(id)
}

#[cfg(test)]
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

        // 无编码的字符串保持不变
        assert_eq!(url_decode("normal/path"), "normal/path");
        assert_eq!(url_decode(""), "");

        // 不完整的编码序列保留原样
        assert_eq!(url_decode("path%2"), "path%2");
        assert_eq!(url_decode("path%"), "path%");

        // 无效的十六进制保留原样
        assert_eq!(url_decode("path%GG"), "path%GG");
        assert_eq!(url_decode("path%2G"), "path%2G");
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
        // 测试当前目录（应该成功）
        let result = canonicalize_and_validate(".");
        assert!(result.is_ok());

        // 测试路径遍历（应该失败）
        let result = canonicalize_and_validate("../../../etc/passwd");
        assert!(result.is_err());
    }
}

/// 验证路径参数
///
/// 验证路径参数是否有效并返回规范化的绝对路径
pub fn validate_path_param(path: &str, param_name: &str) -> Result<std::path::PathBuf, String> {
    // 先执行基础验证（空值、长度）
    crate::utils::command_validation::validate_path_param(path, param_name)?;

    // 验证路径安全性
    validate_safe_path(path).map_err(|e| format!("Invalid {}: {:?}", param_name, e))?;

    // 转换为 PathBuf
    let path_buf = std::path::PathBuf::from(path);

    // 规范化路径
    let canonical = dunce::canonicalize(&path_buf)
        .map_err(|e| format!("Failed to canonicalize {}: {}", param_name, e))?;

    Ok(canonical)
}

// 包含属性测试模块
#[cfg(test)]
#[path = "validation_property_tests.rs"]
mod validation_property_tests;
