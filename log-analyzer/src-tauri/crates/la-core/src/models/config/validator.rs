//! 配置验证基础设施
//!
//! 包含配置错误类型、验证结果、ConfigValidator trait 和验证辅助函数。

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============ 配置错误类型 ============

#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigError {
    #[error("配置加载失败: {0}")]
    LoadError(String),

    #[error("配置验证失败: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("多字段验证失败: {0}")]
    ValidationErrors(String),

    #[error("配置文件不存在: {0}")]
    FileNotFound(String),

    #[error("配置字段 {field} 超出范围: 期望 {expected}, 实际 {actual}")]
    OutOfRange {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("配置字段 {field} 格式无效: {message}")]
    InvalidFormat { field: String, message: String },
}

/// 字段级验证错误
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<FieldValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    pub fn add_error(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) {
        self.errors.push(FieldValidationError {
            field: field.into(),
            message: message.into(),
            code: code.into(),
        });
        self.is_valid = false;
    }

    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
            self.errors.extend(other.errors);
        }
    }

    pub fn to_config_error(&self) -> Option<ConfigError> {
        if self.is_valid {
            return None;
        }

        if self.errors.len() == 1 {
            let err = &self.errors[0];
            return Some(ConfigError::ValidationError {
                field: err.field.clone(),
                message: err.message.clone(),
            });
        }

        let messages: Vec<String> = self
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect();
        Some(ConfigError::ValidationErrors(messages.join("; ")))
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 配置验证 trait
///
/// 为所有配置类型提供统一的验证接口
pub trait ConfigValidator {
    /// 验证配置有效性
    ///
    /// 返回 ValidationResult 包含所有验证错误
    fn validate(&self) -> ValidationResult;

    /// 验证并返回 Result
    ///
    /// 验证失败时返回 ConfigError
    fn validate_result(&self) -> Result<(), ConfigError> {
        let result = self.validate();
        match result.to_config_error() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// 验证并返回默认值修复后的配置
    ///
    /// 对于无效字段使用默认值替换
    fn validate_with_defaults(&self) -> (ValidationResult, bool);
}

// ============ 验证辅助函数 ============

/// 验证端口范围
pub(crate) fn validate_port(port: u16) -> Option<FieldValidationError> {
    if port == 0 {
        return Some(FieldValidationError {
            field: "port".to_string(),
            message: "端口号不能为 0".to_string(),
            code: "invalid_port".to_string(),
        });
    }
    None
}

/// 验证主机名
pub(crate) fn validate_host(host: &str) -> Option<FieldValidationError> {
    if host.is_empty() {
        return Some(FieldValidationError {
            field: "host".to_string(),
            message: "主机名不能为空".to_string(),
            code: "empty_host".to_string(),
        });
    }

    // 检查是否包含非法字符
    if host.contains('\0') || host.contains('\n') || host.contains('\r') {
        return Some(FieldValidationError {
            field: "host".to_string(),
            message: "主机名包含非法字符".to_string(),
            code: "invalid_host_chars".to_string(),
        });
    }

    None
}

/// 验证数值范围
pub(crate) fn validate_range<T: PartialOrd + std::fmt::Display>(
    field: &str,
    value: T,
    min: T,
    max: T,
) -> Option<FieldValidationError> {
    if value < min || value > max {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: format!("值必须在 {min} 到 {max} 之间, 实际为 {value}"),
            code: "out_of_range".to_string(),
        });
    }
    None
}

/// 验证非空字符串
#[allow(dead_code)]
pub(crate) fn validate_non_empty(
    field: &str,
    value: &str,
    max_len: usize,
) -> Option<FieldValidationError> {
    if value.is_empty() {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: "值不能为空".to_string(),
            code: "empty_value".to_string(),
        });
    }

    if value.len() > max_len {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: format!("值长度不能超过 {max_len} 字符"),
            code: "too_long".to_string(),
        });
    }

    None
}

/// 验证日志级别
pub(crate) fn validate_log_level(level: &str) -> Option<FieldValidationError> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Some(FieldValidationError {
            field: "log_level".to_string(),
            message: format!("无效的日志级别: {level}, 必须是以下之一: {valid_levels:?}"),
            code: "invalid_log_level".to_string(),
        });
    }
    None
}

/// 验证文件扩展名
pub(crate) fn validate_extension(ext: &str) -> Option<FieldValidationError> {
    if ext.is_empty() {
        return Some(FieldValidationError {
            field: "extension".to_string(),
            message: "扩展名不能为空".to_string(),
            code: "empty_extension".to_string(),
        });
    }

    if ext.len() > 20 {
        return Some(FieldValidationError {
            field: "extension".to_string(),
            message: "扩展名不能超过 20 个字符".to_string(),
            code: "extension_too_long".to_string(),
        });
    }

    // 扩展名应该只包含字母数字和少量特殊字符
    if !ext
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
    {
        return Some(FieldValidationError {
            field: "extension".to_string(),
            message: "扩展名只能包含字母、数字、点和连字符".to_string(),
            code: "invalid_extension_chars".to_string(),
        });
    }

    None
}

/// 验证路径
pub(crate) fn validate_path(field: &str, path: &str) -> Option<FieldValidationError> {
    if path.is_empty() {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: "路径不能为空".to_string(),
            code: "empty_path".to_string(),
        });
    }

    // 检查路径遍历攻击
    if path.contains("..") {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: "路径包含非法的目录遍历序列".to_string(),
            code: "path_traversal".to_string(),
        });
    }

    // 检查 null 字节
    if path.contains('\0') {
        return Some(FieldValidationError {
            field: field.to_string(),
            message: "路径包含空字节".to_string(),
            code: "null_byte".to_string(),
        });
    }

    None
}

/// 验证正则表达式模式
pub(crate) fn validate_regex_pattern(pattern: &str) -> Option<FieldValidationError> {
    if pattern.is_empty() {
        return None; // 空模式在某些场景下是允许的
    }

    match regex::Regex::new(pattern) {
        Ok(_) => None,
        Err(e) => Some(FieldValidationError {
            field: "pattern".to_string(),
            message: format!("无效的正则表达式: {e}"),
            code: "invalid_regex".to_string(),
        }),
    }
}
