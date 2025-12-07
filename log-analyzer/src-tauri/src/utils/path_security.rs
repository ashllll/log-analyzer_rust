//! 路径安全验证模块
//!
//! 提供全面的路径安全检查功能,防止路径穿越、文件系统错误和恶意压缩包攻击。
//! 支持Windows保留字符过滤、保留文件名检测、路径长度限制等安全措施。

use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// 路径组件验证结果
#[derive(Debug, Clone, PartialEq)]
pub enum PathValidationResult {
    /// 路径安全,返回清理后的字符串
    Valid(String),
    /// 路径不安全,返回原因
    Unsafe(String),
    /// 需要清理,返回(原始路径, 清理后路径)
    RequiresSanitization(String, String),
}

/// 安全检查配置
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// 单个路径组件最大长度(默认255)
    pub max_component_length: usize,
    /// 最大路径深度(默认100)
    pub max_path_depth: usize,
    /// 是否允许Unicode字符
    #[allow(dead_code)]
    pub allow_unicode: bool,
    /// 是否强制Windows兼容
    pub windows_compatible: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_component_length: 255,
            max_path_depth: 100,
            allow_unicode: true,
            windows_compatible: true,
        }
    }
}

/// Windows保留文件名列表
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Windows保留字符列表
const WINDOWS_RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

/// 验证并清理路径组件
///
/// # Arguments
///
/// * `component` - 要验证的路径组件(单个文件名或目录名)
/// * `config` - 安全检查配置
///
/// # Returns
///
/// 返回验证结果,包含清理后的路径或错误原因
///
/// # Examples
///
/// ```
/// use crate::utils::path_security::{validate_and_sanitize_path, SecurityConfig};
///
/// let config = SecurityConfig::default();
/// let result = validate_and_sanitize_path("normal_file.log", &config);
/// ```
pub fn validate_and_sanitize_path(
    component: &str,
    config: &SecurityConfig,
) -> PathValidationResult {
    // 检查空字符串
    if component.is_empty() {
        return PathValidationResult::Unsafe("路径组件为空".to_string());
    }

    // 检查只有空格
    if component.trim().is_empty() {
        return PathValidationResult::Unsafe("路径组件只包含空格".to_string());
    }

    // 检查路径穿越
    if component.contains("..") {
        return PathValidationResult::Unsafe("包含路径穿越(..)".to_string());
    }

    // 检查当前目录
    if component == "." {
        return PathValidationResult::Unsafe("路径组件为当前目录(.)".to_string());
    }

    // 检查绝对路径
    if component.starts_with('/') || component.starts_with('\\') {
        return PathValidationResult::Unsafe("路径不能以/或\\开头".to_string());
    }

    // Windows: 检查驱动器字母
    if config.windows_compatible && component.len() >= 2 {
        let chars: Vec<char> = component.chars().collect();
        if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_alphabetic() {
            return PathValidationResult::Unsafe("路径不能包含驱动器字母".to_string());
        }
    }

    // Unicode规范化
    let normalized = component.nfc().collect::<String>();

    // 移除控制字符
    let mut sanitized = String::new();
    let mut has_control_chars = false;
    for ch in normalized.chars() {
        if ch == '\0' || ('\x01'..='\x1F').contains(&ch) {
            has_control_chars = true;
            continue; // 跳过控制字符
        }
        sanitized.push(ch);
    }

    // Windows: 替换保留字符
    let mut has_reserved_chars = false;
    if config.windows_compatible {
        let temp = sanitized.clone();
        sanitized.clear();
        for ch in temp.chars() {
            if WINDOWS_RESERVED_CHARS.contains(&ch) {
                has_reserved_chars = true;
                sanitized.push('_');
            } else {
                sanitized.push(ch);
            }
        }
    }

    // Windows: 检查保留文件名
    let needs_prefix = if config.windows_compatible {
        is_windows_reserved_name(&sanitized)
    } else {
        false
    };

    if needs_prefix {
        sanitized = format!("_{}", sanitized);
    }

    // 检查长度并截断
    let needs_truncation = if sanitized.len() > config.max_component_length {
        let truncated = truncate_long_component(&sanitized, config.max_component_length);
        sanitized = truncated;
        true
    } else {
        false
    };

    // 判断是否需要清理
    if has_control_chars || has_reserved_chars || needs_prefix || needs_truncation {
        PathValidationResult::RequiresSanitization(component.to_string(), sanitized)
    } else if sanitized == component {
        PathValidationResult::Valid(sanitized)
    } else {
        PathValidationResult::RequiresSanitization(component.to_string(), sanitized)
    }
}

/// 检查是否为Windows保留文件名
///
/// # Arguments
///
/// * `name` - 要检查的文件名(不含路径)
///
/// # Returns
///
/// 如果是保留文件名返回true,否则返回false
pub fn is_windows_reserved_name(name: &str) -> bool {
    // 移除扩展名
    let name_without_ext = if let Some(pos) = name.rfind('.') {
        &name[..pos]
    } else {
        name
    };

    // 大小写不敏感比较
    let upper_name = name_without_ext.to_uppercase();
    WINDOWS_RESERVED_NAMES.contains(&upper_name.as_str())
}

/// 截断超长路径组件
///
/// # Arguments
///
/// * `component` - 要截断的路径组件
/// * `max_len` - 最大长度
///
/// # Returns
///
/// 截断后的路径组件,包含hash后缀保证唯一性
fn truncate_long_component(component: &str, max_len: usize) -> String {
    if component.len() <= max_len {
        return component.to_string();
    }

    // 分离文件名和扩展名
    let (name, ext) = if let Some(pos) = component.rfind('.') {
        let name = &component[..pos];
        let ext = &component[pos..]; // 包含点号
        (name, ext)
    } else {
        (component, "")
    };

    // 计算hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    component.hash(&mut hasher);
    let hash = hasher.finish();
    let hash_suffix = format!("_{:08x}", hash & 0xFFFFFFFF);

    // 计算可用长度
    let available_len = max_len.saturating_sub(hash_suffix.len() + ext.len());

    // 截断文件名
    let mut truncated_name = String::new();
    let mut current_len = 0;
    for ch in name.chars() {
        let ch_len = ch.len_utf8();
        if current_len + ch_len > available_len {
            break;
        }
        truncated_name.push(ch);
        current_len += ch_len;
    }

    // 组合结果
    format!("{}{}{}", truncated_name, hash_suffix, ext)
}

/// 检查路径深度
///
/// # Arguments
///
/// * `path` - 要检查的路径
/// * `max_depth` - 最大深度
///
/// # Returns
///
/// 如果深度超过限制返回Err,否则返回Ok
pub fn check_path_depth(path: &Path, max_depth: usize) -> Result<(), String> {
    let depth = path.components().count();
    if depth > max_depth {
        Err(format!("路径深度{}超过最大限制{}", depth, max_depth))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_normal_path() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("normal_file.log", &config);
        assert_eq!(
            result,
            PathValidationResult::Valid("normal_file.log".to_string())
        );
    }

    #[test]
    fn test_validate_path_traversal() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("../etc/passwd", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));
    }

    #[test]
    fn test_validate_windows_reserved_chars() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("file<name>.log", &config);
        if let PathValidationResult::RequiresSanitization(_, sanitized) = result {
            assert_eq!(sanitized, "file_name_.log");
        } else {
            panic!("Expected RequiresSanitization");
        }
    }

    #[test]
    fn test_validate_windows_reserved_name() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("CON", &config);
        if let PathValidationResult::RequiresSanitization(_, sanitized) = result {
            assert_eq!(sanitized, "_CON");
        } else {
            panic!("Expected RequiresSanitization");
        }
    }

    #[test]
    fn test_validate_control_characters() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("file\x00name.log", &config);
        if let PathValidationResult::RequiresSanitization(_, sanitized) = result {
            assert_eq!(sanitized, "filename.log");
        } else {
            panic!("Expected RequiresSanitization");
        }
    }

    #[test]
    fn test_truncate_long_component() {
        let long_name = "a".repeat(300);
        let result = truncate_long_component(&long_name, 255);
        assert!(result.len() <= 255);
        assert!(result.contains('_')); // 应该包含hash后缀
    }

    #[test]
    fn test_truncate_with_extension() {
        let long_name = format!("{}.log", "a".repeat(300));
        let result = truncate_long_component(&long_name, 255);
        assert!(result.len() <= 255);
        assert!(result.ends_with(".log"));
    }

    #[test]
    fn test_is_windows_reserved_name() {
        assert!(is_windows_reserved_name("CON"));
        assert!(is_windows_reserved_name("con"));
        assert!(is_windows_reserved_name("CON.txt"));
        assert!(is_windows_reserved_name("COM1"));
        assert!(!is_windows_reserved_name("CONFIG"));
        assert!(!is_windows_reserved_name("normal"));
    }

    #[test]
    fn test_check_path_depth() {
        let shallow_path = Path::new("a/b/c");
        assert!(check_path_depth(shallow_path, 100).is_ok());

        let deep_path_str = (0..150)
            .map(|i| format!("dir{}", i))
            .collect::<Vec<_>>()
            .join("/");
        let deep_path = Path::new(&deep_path_str);
        assert!(check_path_depth(deep_path, 100).is_err());
    }

    #[test]
    fn test_empty_and_whitespace() {
        let config = SecurityConfig::default();

        let result = validate_and_sanitize_path("", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));

        let result = validate_and_sanitize_path("   ", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));
    }

    #[test]
    fn test_absolute_paths() {
        let config = SecurityConfig::default();

        let result = validate_and_sanitize_path("/etc/passwd", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));

        let result = validate_and_sanitize_path("\\Windows\\System32", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));
    }

    #[test]
    fn test_drive_letter() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("C:\\Users", &config);
        assert!(matches!(result, PathValidationResult::Unsafe(_)));
    }

    #[test]
    fn test_unicode_normalization() {
        let config = SecurityConfig::default();
        // 测试组合字符
        let result = validate_and_sanitize_path("café", &config);
        assert!(matches!(
            result,
            PathValidationResult::Valid(_) | PathValidationResult::RequiresSanitization(_, _)
        ));
    }
}
