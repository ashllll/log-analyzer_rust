//! 验证命令
//!
//! 提供数据验证功能

use crate::models::validated::{
    ValidatedArchiveConfig, ValidatedSearchQuery, ValidatedWorkspaceConfig, ValidationErrorReport,
};
use tauri::command;
use validator::Validate;

/// 验证工作区配置
#[command]
pub async fn validate_workspace_config_cmd(
    config: ValidatedWorkspaceConfig,
) -> Result<ValidationErrorReport, String> {
    match config.validate() {
        Ok(_) => Ok(ValidationErrorReport {
            errors: std::collections::HashMap::new(),
            error_count: 0,
        }),
        Err(e) => Ok(ValidationErrorReport::from_validation_errors(e)),
    }
}

/// 验证搜索查询
#[command]
pub async fn validate_search_query_cmd(
    query: ValidatedSearchQuery,
) -> Result<ValidationErrorReport, String> {
    match query.validate() {
        Ok(_) => Ok(ValidationErrorReport {
            errors: std::collections::HashMap::new(),
            error_count: 0,
        }),
        Err(e) => Ok(ValidationErrorReport::from_validation_errors(e)),
    }
}

/// 验证归档配置（替代原来的 ValidatedImportConfig）
#[command]
pub async fn validate_archive_config_cmd(
    config: ValidatedArchiveConfig,
) -> Result<ValidationErrorReport, String> {
    use validator::Validate;

    let mut result = ValidationErrorReport {
        errors: std::collections::HashMap::new(),
        error_count: 0,
    };

    match config.validate() {
        Ok(_) => {
            // 额外的业务逻辑验证
            if config.max_file_size > 5_000_000_000 {
                // 5GB
                result.errors.insert(
                    "max_file_size".to_string(),
                    vec!["Large import size may take significant time".to_string()],
                );
                result.error_count += 1;
            }

            if config.allowed_extensions.is_empty() {
                result.errors.insert(
                    "allowed_extensions".to_string(),
                    vec!["No file extensions specified".to_string()],
                );
                result.error_count += 1;
            }
        }
        Err(errors) => {
            result = ValidationErrorReport::from_validation_errors(errors);
        }
    }

    Ok(result)
}

/// 批量验证工作区配置
#[command]
pub async fn batch_validate_workspace_configs(
    configs: Vec<ValidatedWorkspaceConfig>,
) -> Result<Vec<ValidationErrorReport>, String> {
    let results: Vec<ValidationErrorReport> = configs
        .iter()
        .map(|config| match config.validate() {
            Ok(_) => ValidationErrorReport {
                errors: std::collections::HashMap::new(),
                error_count: 0,
            },
            Err(e) => ValidationErrorReport::from_validation_errors(e),
        })
        .collect();

    Ok(results)
}

/// 验证工作区ID格式
#[command]
pub async fn validate_workspace_id_format(workspace_id: String) -> Result<bool, String> {
    use once_cell::sync::Lazy;

    static WORKSPACE_ID_REGEX: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

    if workspace_id.is_empty() || workspace_id.len() > 100 {
        return Ok(false);
    }

    Ok(WORKSPACE_ID_REGEX.is_match(&workspace_id))
}

/// 验证文件路径安全性
#[command]
pub async fn validate_path_security(path: String) -> Result<ValidationErrorReport, String> {
    use std::path::Path;

    let mut result = ValidationErrorReport {
        errors: std::collections::HashMap::new(),
        error_count: 0,
    };

    if path.is_empty() {
        result
            .errors
            .insert("path".to_string(), vec!["Path cannot be empty".to_string()]);
        result.error_count = 1;
        return Ok(result);
    }

    if path.len() > 500 {
        result.errors.insert(
            "path".to_string(),
            vec!["Path too long (max 500 characters)".to_string()],
        );
        result.error_count += 1;
    }

    // 检查路径遍历攻击
    if path.contains("..") || path.contains("~") {
        result.errors.insert(
            "path".to_string(),
            vec!["Path contains dangerous sequences".to_string()],
        );
        result.error_count += 1;
    }

    // 检查Windows保留字符
    if cfg!(target_os = "windows") {
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if path.chars().any(|c| invalid_chars.contains(&c)) {
            result.errors.insert(
                "path".to_string(),
                vec!["Path contains invalid characters for Windows".to_string()],
            );
            result.error_count += 1;
        }
    }

    // 检查路径组件
    let path_buf = Path::new(&path);
    if path_buf
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        result.errors.insert(
            "path".to_string(),
            vec!["Path contains parent directory references".to_string()],
        );
        result.error_count += 1;
    }

    // 检查Windows保留名称
    if cfg!(target_os = "windows") {
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        for component in path_buf.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    let name_upper = name_str.to_uppercase();
                    if reserved_names.contains(&name_upper.as_str()) {
                        result.errors.insert(
                            "path".to_string(),
                            vec![format!("Path contains Windows reserved name: {}", name_str)],
                        );
                        result.error_count += 1;
                    }
                }
            }
        }
    }

    Ok(result)
}
