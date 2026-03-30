//! 路径安全验证模块
//!
//! 提供全面的路径安全检查功能,防止路径穿越、文件系统错误和恶意压缩包攻击。
//! 支持Windows保留字符过滤、保留文件名检测、路径长度限制等安全措施。

use tracing::warn;
use unicode_normalization::UnicodeNormalization;

#[cfg(test)]
use std::path::Path;

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
    /// 是否强制Windows兼容
    pub windows_compatible: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_component_length: 255,
            max_path_depth: 100,
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
/// ```rust,ignore
/// use log_analyzer::utils::path_security::{validate_and_sanitize_path, SecurityConfig};
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
    if component.contains("..")
        || component.contains('/')
        || component.contains('\\')
        || component.contains(':')
    {
        return PathValidationResult::Unsafe("包含非法路径字符(.. , / , \\ , :)".to_string());
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

/// 验证并清理完整的归档内部路径(支持多级目录)
///
/// # Arguments
///
/// * `full_path` - 归档内的完整路径
/// * `config` - 安全检查配置
///
/// # Returns
///
/// 返回验证结果,包含清理后的路径或错误原因
pub fn validate_and_sanitize_archive_path(
    full_path: &str,
    config: &SecurityConfig,
) -> PathValidationResult {
    if full_path.is_empty() {
        return PathValidationResult::Unsafe("路径为空".to_string());
    }

    // 预检：如果包含驱动器字母且是Windows兼容模式，直接拒绝
    if config.windows_compatible && full_path.contains(':') {
        let chars: Vec<char> = full_path.chars().collect();
        for i in 0..chars.len().saturating_sub(1) {
            if chars[i].is_ascii_alphabetic() && chars[i + 1] == ':' {
                return PathValidationResult::Unsafe("路径包含驱动器字母".to_string());
            }
        }
    }

    let mut sanitized_components = Vec::new();
    let mut needs_sanitization = false;

    // 统一使用 / 和 \ 作为分隔符进行拆分
    let components = full_path.split(['/', '\\']);

    for component in components {
        if component.is_empty() || component == "." {
            // 跳过空组件(如 //)或当前目录组件(.)
            if !full_path.is_empty() {
                needs_sanitization = true;
            }
            continue;
        }

        match validate_and_sanitize_path(component, config) {
            PathValidationResult::Valid(s) => sanitized_components.push(s),
            PathValidationResult::RequiresSanitization(_, s) => {
                sanitized_components.push(s);
                needs_sanitization = true;
            }
            PathValidationResult::Unsafe(reason) => {
                return PathValidationResult::Unsafe(format!(
                    "路径组件 '{}' 不安全: {}",
                    component, reason
                ));
            }
        }
    }

    if sanitized_components.is_empty() {
        return PathValidationResult::Unsafe("路径不包含有效的文件名组件".to_string());
    }

    // 内部统一使用正斜杠
    let sanitized_path = sanitized_components.join("/");

    if needs_sanitization || sanitized_path != full_path.replace('\\', "/") {
        PathValidationResult::RequiresSanitization(full_path.to_string(), sanitized_path)
    } else {
        PathValidationResult::Valid(sanitized_path)
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
    let result = format!("{}{}{}", truncated_name, hash_suffix, ext);

    // 截断日志：通知运维/开发者文件名已被修改，方便追溯
    // 中文场景：每个汉字占 3 字节，85 汉字 = 255 字节触发此分支
    warn!(
        original_name = %component,
        truncated_name = %result,
        original_bytes = component.len(),
        truncated_bytes = result.len(),
        max_bytes = max_len,
        "文件名超过系统路径组件长度上限（{} 字节），已截断并追加哈希后缀以保证唯一性",
        max_len
    );

    result
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
#[cfg(test)]
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
    use std::path::Path;

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
        match result {
            PathValidationResult::RequiresSanitization(_, sanitized) => {
                assert_eq!(sanitized, "file_name_.log");
            }
            other => panic!("Expected RequiresSanitization, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_windows_reserved_name() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("CON", &config);
        match result {
            PathValidationResult::RequiresSanitization(_, sanitized) => {
                assert_eq!(sanitized, "_CON");
            }
            other => panic!("Expected RequiresSanitization, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_control_characters() {
        let config = SecurityConfig::default();
        let result = validate_and_sanitize_path("file\x00name.log", &config);
        match result {
            PathValidationResult::RequiresSanitization(_, sanitized) => {
                assert_eq!(sanitized, "filename.log");
            }
            other => panic!("Expected RequiresSanitization, got: {:?}", other),
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

    // --- 长路径系统性修复测试 ---

    /// 中文文件名：86 个汉字 = 258 字节，超过 255 字节上限应被截断
    #[test]
    fn test_truncate_chinese_filename_86_chars() {
        // 每个汉字占 3 字节 UTF-8，86 × 3 = 258 字节 > 255
        let chinese_name = "中".repeat(86);
        assert_eq!(chinese_name.len(), 258, "前置条件：86 汉字 = 258 字节");

        let result = truncate_long_component(&chinese_name, 255);

        assert!(result.len() <= 255, "截断后仍超限：{} 字节", result.len());
        assert_ne!(result, chinese_name, "超限文件名应被截断");
        // 截断结果应含哈希后缀（格式：_{8位十六进制}）
        assert!(result.contains('_'), "截断后应含哈希后缀以保证唯一性");
    }

    /// 中文文件名带扩展名：截断后扩展名应保留
    #[test]
    fn test_truncate_chinese_filename_with_extension() {
        let long_chinese = format!("{}.log", "日".repeat(86));
        assert!(long_chinese.len() > 255);

        let result = truncate_long_component(&long_chinese, 255);

        assert!(result.len() <= 255, "截断后仍超限：{} 字节", result.len());
        assert!(
            result.ends_with(".log"),
            "截断后扩展名应保留，实际为：{}",
            result
        );
    }

    /// 85 个汉字 = 255 字节，恰好在上限内，不应截断
    #[test]
    fn test_chinese_85_chars_not_truncated() {
        let chinese_name = "中".repeat(85);
        assert_eq!(chinese_name.len(), 255, "前置条件：85 汉字 = 255 字节");

        // 不应触发截断（== max_len，无需截断）
        let result = truncate_long_component(&chinese_name, 255);
        assert_eq!(result, chinese_name, "255 字节不超限，不应截断");
    }

    /// 归档路径验证：含中文长文件名的完整路径应能成功处理
    #[test]
    fn test_archive_path_with_long_chinese_component() {
        let config = SecurityConfig::default();
        // 构造包含超长中文组件的归档内部路径
        let long_component = "中".repeat(90); // 270 字节，超限
        let archive_path = format!("subdir/{}.log", long_component);

        let result = validate_and_sanitize_archive_path(&archive_path, &config);

        // 应返回 RequiresSanitization（不是 Unsafe），截断后路径合法
        match result {
            PathValidationResult::RequiresSanitization(original, sanitized) => {
                assert_eq!(original, archive_path);
                // 验证各组件长度合规
                for component in sanitized.split('/') {
                    assert!(
                        component.len() <= 255,
                        "组件仍超限：{} ({} 字节)",
                        component,
                        component.len()
                    );
                }
            }
            PathValidationResult::Valid(sanitized) => {
                // 也可能直接为 Valid（若截断后与原始路径规范化结果一致）
                for component in sanitized.split('/') {
                    assert!(component.len() <= 255);
                }
            }
            PathValidationResult::Unsafe(reason) => {
                panic!("含长中文文件名的路径不应被判为 Unsafe，原因：{}", reason);
            }
        }
    }
}
