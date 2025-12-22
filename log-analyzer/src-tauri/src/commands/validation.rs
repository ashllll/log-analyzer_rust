//! 验证命令
//!
//! 提供数据验证功能

use crate::models::validated::{validate_search_query, validate_workspace_config};
use crate::models::{
    ValidatedImportConfig, ValidatedSearchQuery, ValidatedWorkspaceConfig, ValidationResult,
};
use tauri::command;

/// 验证工作区配置
#[command]
pub async fn validate_workspace_config_cmd(
    config: ValidatedWorkspaceConfig,
) -> Result<ValidationResult<()>, String> {
    Ok(validate_workspace_config(&config))
}

/// 验证搜索查询
#[command]
pub async fn validate_search_query_cmd(
    query: ValidatedSearchQuery,
) -> Result<ValidationResult<()>, String> {
    Ok(validate_search_query(&query))
}

/// 验证导入配置
#[command]
pub async fn validate_import_config_cmd(
    config: ValidatedImportConfig,
) -> Result<ValidationResult<()>, String> {
    use validator::Validate;

    let mut result = ValidationResult::new(());

    match config.validate() {
        Ok(_) => {
            // 额外的业务逻辑验证
            if config.max_import_size > 5_000_000_000 {
                // 5GB
                result
                    .warnings
                    .push("Large import size may take significant time".to_string());
            }

            if config.allowed_extensions.is_empty() && !config.recursive {
                result
                    .warnings
                    .push("No file extensions specified for non-recursive import".to_string());
            }
        }
        Err(errors) => {
            result.errors = errors
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |e| {
                        format!(
                            "{}: {}",
                            field,
                            e.message.as_ref().unwrap_or(&"Invalid value".into())
                        )
                    })
                })
                .collect();
        }
    }

    Ok(result)
}

/// 批量验证工作区配置
#[command]
pub async fn batch_validate_workspace_configs(
    configs: Vec<ValidatedWorkspaceConfig>,
) -> Result<Vec<ValidationResult<()>>, String> {
    let results = configs
        .iter()
        .map(|config| validate_workspace_config(config))
        .collect();

    Ok(results)
}

/// 验证工作区ID格式
#[command]
pub async fn validate_workspace_id_format(workspace_id: String) -> Result<bool, String> {
    use regex::Regex;

    lazy_static::lazy_static! {
        static ref WORKSPACE_ID_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    }

    if workspace_id.is_empty() || workspace_id.len() > 100 {
        return Ok(false);
    }

    Ok(WORKSPACE_ID_REGEX.is_match(&workspace_id))
}

/// 验证文件路径安全性
#[command]
pub async fn validate_path_security(path: String) -> Result<ValidationResult<()>, String> {
    use std::path::Path;

    let mut result = ValidationResult::new(());

    if path.is_empty() {
        result.errors.push("Path cannot be empty".to_string());
        return Ok(result);
    }

    if path.len() > 500 {
        result
            .errors
            .push("Path too long (max 500 characters)".to_string());
        return Ok(result);
    }

    // 检查路径遍历攻击
    if path.contains("..") || path.contains("~") {
        result
            .errors
            .push("Path contains dangerous sequences".to_string());
        return Ok(result);
    }

    // 检查Windows保留字符
    if cfg!(target_os = "windows") {
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if path.chars().any(|c| invalid_chars.contains(&c)) {
            result
                .errors
                .push("Path contains invalid characters for Windows".to_string());
            return Ok(result);
        }
    }

    // 检查路径组件
    let path_buf = Path::new(&path);
    if path_buf
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        result
            .errors
            .push("Path contains parent directory references".to_string());
        return Ok(result);
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
                        result
                            .errors
                            .push(format!("Path contains Windows reserved name: {}", name_str));
                        return Ok(result);
                    }
                }
            }
        }
    }

    Ok(result)
}
