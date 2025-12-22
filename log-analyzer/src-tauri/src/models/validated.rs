//! 验证数据结构
//!
//! 使用 validator 框架提供结构化验证和错误报告

#![allow(dead_code)]

use chrono;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

// 定义本地的 WORKSPACE_ID_REGEX 用于 validator
#[allow(dead_code)]
static WORKSPACE_ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-_]+$").unwrap());

// Email 验证正则
#[allow(dead_code)]
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

// URL 验证正则
#[allow(dead_code)]
static URL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https?://[a-zA-Z0-9\-._~:/?#\[\]@!$&'()*+,;=]+$").unwrap());

/// 验证的工作区配置
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedWorkspaceConfig {
    /// 工作区 ID - 只允许字母数字和连字符
    #[validate(regex(path = *WORKSPACE_ID_REGEX))]
    #[validate(length(min = 1, max = 50))]
    pub id: String,

    /// 工作区名称
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// 工作区路径 - 必须是有效路径
    #[validate(custom(function = "validate_safe_path_wrapper"))]
    pub path: String,
}

/// 验证的搜索查询
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedSearchQuery {
    /// 查询字符串
    #[validate(length(min = 1, max = 1000))]
    pub query: String,

    /// 最大结果数
    #[validate(range(min = 1, max = 100000))]
    pub max_results: usize,

    /// 工作区 ID
    #[validate(regex(path = *WORKSPACE_ID_REGEX))]
    pub workspace_id: String,
}

/// 验证的用户配置（演示 email 和 URL 验证）
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedUserConfig {
    /// 用户名
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    /// 电子邮件
    #[validate(regex(path = *EMAIL_REGEX))]
    pub email: Option<String>,

    /// 通知 URL（可选）
    #[validate(regex(path = *URL_REGEX))]
    pub notification_url: Option<String>,

    /// 年龄范围验证
    #[validate(range(min = 18, max = 120))]
    pub age: Option<u32>,
}

/// 验证的归档配置（演示嵌套验证）
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedArchiveConfig {
    /// 单个文件大小限制（字节）
    #[validate(range(min = 1, max = 104_857_600))] // 最大 100MB
    pub max_file_size: u64,

    /// 总大小限制（字节）
    #[validate(range(min = 1, max = 1_073_741_824))] // 最大 1GB
    pub max_total_size: u64,

    /// 文件数量限制
    #[validate(range(min = 1, max = 10000))]
    pub max_file_count: usize,

    /// 允许的文件扩展名
    #[validate(length(min = 1))]
    #[validate(custom(function = "validate_extensions"))]
    pub allowed_extensions: Vec<String>,
}

/// 验证的搜索过滤器（演示复杂嵌套验证）
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedSearchFilters {
    /// 关键词列表
    #[validate(length(min = 1, max = 100))]
    pub keywords: Vec<String>,

    /// 日期范围
    #[validate(nested)]
    pub date_range: Option<ValidatedDateRange>,

    /// 文件类型过滤
    #[validate(length(max = 50))]
    pub file_types: Vec<String>,

    /// 大小范围
    #[validate(nested)]
    pub size_range: Option<ValidatedSizeRange>,
}

/// 验证的日期范围
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedDateRange {
    /// 开始日期（ISO 8601 格式）
    #[validate(custom(function = "validate_iso_date"))]
    pub start: String,

    /// 结束日期（ISO 8601 格式）
    #[validate(custom(function = "validate_iso_date"))]
    pub end: String,
}

/// 验证的大小范围
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedSizeRange {
    /// 最小大小（字节）
    #[validate(range(min = 0))]
    pub min: u64,

    /// 最大大小（字节）
    #[validate(range(min = 0))]
    pub max: u64,
}

// 自定义验证函数

/// 验证文件扩展名列表
fn validate_extensions(extensions: &[String]) -> Result<(), ValidationError> {
    for ext in extensions {
        if ext.is_empty() || ext.len() > 10 {
            return Err(ValidationError::new("invalid_extension"));
        }
        // 扩展名应该只包含字母数字
        if !ext.chars().all(|c| c.is_alphanumeric()) {
            return Err(ValidationError::new("invalid_extension_chars"));
        }
    }
    Ok(())
}

/// 验证 ISO 8601 日期格式
fn validate_iso_date(date: &str) -> Result<(), ValidationError> {
    // 简单的 ISO 8601 格式验证
    if chrono::DateTime::parse_from_rfc3339(date).is_err() {
        return Err(ValidationError::new("invalid_iso_date"));
    }
    Ok(())
}

// 包装函数用于 validator
#[allow(dead_code)]
fn validate_safe_path_wrapper(path: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_safe_path(path)
}

/// 验证错误聚合器
///
/// 收集和格式化多个验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorReport {
    /// 错误字段映射
    pub errors: std::collections::HashMap<String, Vec<String>>,
    /// 错误总数
    pub error_count: usize,
}

impl ValidationErrorReport {
    /// 从 validator 错误创建报告
    pub fn from_validation_errors(errors: validator::ValidationErrors) -> Self {
        let mut error_map = std::collections::HashMap::new();
        let mut count = 0;

        for (field, field_errors) in errors.field_errors() {
            let messages: Vec<String> = field_errors
                .iter()
                .map(|e| {
                    e.message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Validation failed for field: {}", field))
                })
                .collect();

            count += messages.len();
            error_map.insert(field.to_string(), messages);
        }

        Self {
            errors: error_map,
            error_count: count,
        }
    }

    /// 转换为用户友好的错误消息
    pub fn to_user_message(&self) -> String {
        let mut messages = Vec::new();
        for (field, errors) in &self.errors {
            for error in errors {
                messages.push(format!("{}: {}", field, error));
            }
        }
        messages.join("; ")
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

/// 验证助手特征
///
/// 为所有可验证类型提供统一的验证接口
pub trait ValidateExt: Validate {
    /// 验证并返回详细的错误报告
    fn validate_with_report(&self) -> Result<(), ValidationErrorReport> {
        self.validate()
            .map_err(ValidationErrorReport::from_validation_errors)
    }
}

// 为所有实现 Validate 的类型自动实现 ValidateExt
impl<T: Validate> ValidateExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_validation() {
        // 有效配置
        let valid_config = ValidatedWorkspaceConfig {
            id: "valid-id-123".to_string(),
            name: "Test Workspace".to_string(),
            path: "/valid/path".to_string(),
        };
        assert!(valid_config.validate().is_ok());

        // 无效 ID (包含特殊字符)
        let invalid_id = ValidatedWorkspaceConfig {
            id: "invalid@id!".to_string(),
            name: "Test".to_string(),
            path: "/path".to_string(),
        };
        assert!(invalid_id.validate().is_err());

        // 空名称
        let empty_name = ValidatedWorkspaceConfig {
            id: "valid-id".to_string(),
            name: "".to_string(),
            path: "/path".to_string(),
        };
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_search_query_validation() {
        // 有效查询
        let valid_query = ValidatedSearchQuery {
            query: "test query".to_string(),
            max_results: 1000,
            workspace_id: "workspace-1".to_string(),
        };
        assert!(valid_query.validate().is_ok());

        // 空查询
        let empty_query = ValidatedSearchQuery {
            query: "".to_string(),
            max_results: 1000,
            workspace_id: "workspace-1".to_string(),
        };
        assert!(empty_query.validate().is_err());

        // 超出范围的结果数
        let invalid_max = ValidatedSearchQuery {
            query: "test".to_string(),
            max_results: 200000,
            workspace_id: "workspace-1".to_string(),
        };
        assert!(invalid_max.validate().is_err());
    }

    #[test]
    fn test_user_config_validation() {
        // 有效用户配置
        let valid_user = ValidatedUserConfig {
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            notification_url: Some("https://example.com/notify".to_string()),
            age: Some(25),
        };
        assert!(valid_user.validate().is_ok());

        // 无效 email
        let invalid_email = ValidatedUserConfig {
            username: "testuser".to_string(),
            email: Some("invalid-email".to_string()),
            notification_url: None,
            age: Some(25),
        };
        assert!(invalid_email.validate().is_err());

        // 无效 URL
        let invalid_url = ValidatedUserConfig {
            username: "testuser".to_string(),
            email: None,
            notification_url: Some("not-a-url".to_string()),
            age: Some(25),
        };
        assert!(invalid_url.validate().is_err());

        // 年龄超出范围
        let invalid_age = ValidatedUserConfig {
            username: "testuser".to_string(),
            email: None,
            notification_url: None,
            age: Some(150),
        };
        assert!(invalid_age.validate().is_err());
    }

    #[test]
    fn test_archive_config_validation() {
        // 有效配置
        let valid_config = ValidatedArchiveConfig {
            max_file_size: 10_485_760,   // 10MB
            max_total_size: 104_857_600, // 100MB
            max_file_count: 1000,
            allowed_extensions: vec!["log".to_string(), "txt".to_string()],
        };
        assert!(valid_config.validate().is_ok());

        // 文件大小超出限制
        let invalid_size = ValidatedArchiveConfig {
            max_file_size: 200_000_000, // 200MB
            max_total_size: 104_857_600,
            max_file_count: 1000,
            allowed_extensions: vec!["log".to_string()],
        };
        assert!(invalid_size.validate().is_err());

        // 无效扩展名
        let invalid_ext = ValidatedArchiveConfig {
            max_file_size: 10_485_760,
            max_total_size: 104_857_600,
            max_file_count: 1000,
            allowed_extensions: vec!["log@invalid".to_string()],
        };
        assert!(invalid_ext.validate().is_err());
    }

    #[test]
    fn test_nested_validation() {
        // 有效的嵌套结构
        let valid_filters = ValidatedSearchFilters {
            keywords: vec!["error".to_string(), "warning".to_string()],
            date_range: Some(ValidatedDateRange {
                start: "2024-01-01T00:00:00Z".to_string(),
                end: "2024-12-31T23:59:59Z".to_string(),
            }),
            file_types: vec!["log".to_string()],
            size_range: Some(ValidatedSizeRange {
                min: 0,
                max: 1_000_000,
            }),
        };
        assert!(valid_filters.validate().is_ok());

        // 无效的日期格式
        let invalid_date = ValidatedSearchFilters {
            keywords: vec!["error".to_string()],
            date_range: Some(ValidatedDateRange {
                start: "invalid-date".to_string(),
                end: "2024-12-31T23:59:59Z".to_string(),
            }),
            file_types: vec![],
            size_range: None,
        };
        assert!(invalid_date.validate().is_err());
    }

    #[test]
    fn test_validation_error_report() {
        // 创建一个有多个错误的配置
        let invalid_config = ValidatedWorkspaceConfig {
            id: "".to_string(),   // 太短
            name: "".to_string(), // 空名称
            path: "/path".to_string(),
        };

        let result = invalid_config.validate_with_report();
        assert!(result.is_err());

        if let Err(report) = result {
            assert!(report.has_errors());
            assert!(report.error_count >= 2); // 至少两个错误
            assert!(report.errors.contains_key("id"));
            assert!(report.errors.contains_key("name"));

            let message = report.to_user_message();
            assert!(!message.is_empty());
        }
    }

    #[test]
    fn test_validate_ext_trait() {
        let config = ValidatedWorkspaceConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            path: "/path".to_string(),
        };

        // 使用 ValidateExt trait
        let result = config.validate_with_report();
        assert!(result.is_ok());
    }
}
