//! 配置管理命令
//!
//! 使用行业标准的 `config` crate 实现配置管理：
//! - 支持多层配置：默认值 → 配置文件 → 环境变量
//! - 环境变量前缀：`LOG_ANALYZER_`
//! - 保持向后兼容 JSON 配置文件
//! - 配置验证支持

use std::fs;

use tauri::{command, AppHandle, Manager};

use la_core::models::config::{
    AppConfig, AppConfigLoader, CacheConfig, ConfigValidator, FieldValidationError,
    FileFilterConfig, SearchConfig, TaskManagerConfig, ValidationResult,
};

/// 加载配置（使用新的 ConfigLoader 系统）
///
/// 加载时会自动验证配置，无效配置会记录警告并使用默认值
fn load_config_internal(app: &AppHandle) -> Result<AppConfig, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e: tauri::Error| e.to_string())?;

    let config_path = config_dir.join("config.json");

    if config_path.exists() {
        match AppConfigLoader::load(Some(config_path.clone())) {
            Ok(loader) => {
                // 检查验证结果
                if let Some(validation) = loader.get_validation_result() {
                    if !validation.is_valid {
                        tracing::warn!(
                            "配置文件包含无效值，将使用默认值。错误: {:?}",
                            validation.errors
                        );
                    }
                }
                Ok(loader.get_config().clone())
            }
            Err(e) => {
                // 配置解析失败时降级到默认配置，避免应用无法启动
                tracing::warn!(
                    path = %config_path.display(),
                    error = %e,
                    "配置文件解析失败，使用默认配置"
                );
                Ok(AppConfig::default())
            }
        }
    } else {
        // 返回默认配置
        Ok(AppConfig::default())
    }
}

/// 保存配置
///
/// 保存前会进行验证，确保配置有效性
#[command]
pub async fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    // 先验证配置
    let validation = config.validate();
    if !validation.is_valid {
        let error_msg = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("配置验证失败: {}", error_msg));
    }

    tokio::task::spawn_blocking(move || {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|e: tauri::Error| e.to_string())?;
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        }
        let path = config_dir.join("config.json");
        let tmp_path = config_dir.join("config.json.tmp");
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        // 先写临时文件，再原子重命名，防止进程崩溃导致配置损坏
        fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
        fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))?
}

/// 加载配置
///
/// 返回当前配置，配置会自动验证
#[command]
pub async fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    tokio::task::spawn_blocking(move || load_config_internal(&app))
        .await
        .map_err(|e| format!("Task panicked: {}", e))?
}

/// 验证配置
///
/// 验证配置的有效性并返回详细的验证结果
/// 前端可以在保存前调用此命令预览验证错误
#[command]
pub async fn validate_config(config: AppConfig) -> Result<ValidationResult, String> {
    Ok(config.validate())
}

/// 验证配置字段
///
/// 验证单个配置字段的有效性
#[command]
pub async fn validate_config_field(
    config_section: String,
    field_name: String,
    field_value: serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    // 根据配置节和字段名进行验证
    match config_section.as_str() {
        "server" => validate_server_field(&field_name, &field_value),
        "search" => validate_search_field(&field_name, &field_value),
        "monitoring" => validate_monitoring_field(&field_name, &field_value),
        "security" => validate_security_field(&field_name, &field_value),
        "cache" => validate_cache_field(&field_name, &field_value),
        "frontend" => validate_frontend_field(&field_name, &field_value),
        _ => Ok(None),
    }
}

/// 验证服务器配置字段
fn validate_server_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "port" => {
            if let Some(port) = field_value.as_u64() {
                if port == 0 || port > 65535 {
                    return Ok(Some(FieldValidationError {
                        field: "port".to_string(),
                        message: "端口号必须在 1-65535 之间".to_string(),
                        code: "invalid_port".to_string(),
                    }));
                }
            }
        }
        "max_connections" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 10000 {
                    return Ok(Some(FieldValidationError {
                        field: "max_connections".to_string(),
                        message: "最大连接数必须在 1-10000 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        "timeout_seconds" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 3600 {
                    return Ok(Some(FieldValidationError {
                        field: "timeout_seconds".to_string(),
                        message: "超时时间必须在 1-3600 秒之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 验证搜索配置字段
fn validate_search_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "max_results" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 1_000_000 {
                    return Ok(Some(FieldValidationError {
                        field: "max_results".to_string(),
                        message: "最大结果数必须在 1-1000000 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        "timeout_seconds" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 3600 {
                    return Ok(Some(FieldValidationError {
                        field: "timeout_seconds".to_string(),
                        message: "超时时间必须在 1-3600 秒之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        "max_concurrent_searches" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 100 {
                    return Ok(Some(FieldValidationError {
                        field: "max_concurrent_searches".to_string(),
                        message: "并发搜索数必须在 1-100 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 验证监控配置字段
fn validate_monitoring_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "log_level" => {
            if let Some(level) = field_value.as_str() {
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_levels.contains(&level.to_lowercase().as_str()) {
                    return Ok(Some(FieldValidationError {
                        field: "log_level".to_string(),
                        message: format!("无效的日志级别，必须是以下之一: {:?}", valid_levels),
                        code: "invalid_log_level".to_string(),
                    }));
                }
            }
        }
        "max_log_files" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 100 {
                    return Ok(Some(FieldValidationError {
                        field: "max_log_files".to_string(),
                        message: "最大日志文件数必须在 1-100 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 验证安全配置字段
fn validate_security_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "api_key" => {
            if let Some(key) = field_value.as_str() {
                if !key.is_empty() && key.len() < 16 {
                    return Ok(Some(FieldValidationError {
                        field: "api_key".to_string(),
                        message: "API 密钥长度至少为 16 个字符".to_string(),
                        code: "api_key_too_short".to_string(),
                    }));
                }
            }
        }
        "rate_limit_per_minute" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 10000 {
                    return Ok(Some(FieldValidationError {
                        field: "rate_limit_per_minute".to_string(),
                        message: "速率限制必须在 1-10000 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 验证缓存配置字段
fn validate_cache_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "regex_cache_size" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 100000 {
                    return Ok(Some(FieldValidationError {
                        field: "regex_cache_size".to_string(),
                        message: "正则缓存大小必须在 1-100000 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        "min_hit_rate_threshold" => {
            if let Some(val) = field_value.as_f64() {
                if !(0.0..=1.0).contains(&val) {
                    return Ok(Some(FieldValidationError {
                        field: "min_hit_rate_threshold".to_string(),
                        message: "命中率阈值必须在 0.0-1.0 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 验证前端配置字段
fn validate_frontend_field(
    field_name: &str,
    field_value: &serde_json::Value,
) -> Result<Option<FieldValidationError>, String> {
    match field_name {
        "vite_dev_server_port" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 65535 {
                    return Ok(Some(FieldValidationError {
                        field: "vite_dev_server_port".to_string(),
                        message: "端口号必须在 1-65535 之间".to_string(),
                        code: "invalid_port".to_string(),
                    }));
                }
            }
        }
        "websocket_url" => {
            if let Some(url) = field_value.as_str() {
                if !url.starts_with("ws://") && !url.starts_with("wss://") {
                    return Ok(Some(FieldValidationError {
                        field: "websocket_url".to_string(),
                        message: "WebSocket URL 必须以 ws:// 或 wss:// 开头".to_string(),
                        code: "invalid_websocket_url".to_string(),
                    }));
                }
            }
        }
        "log_truncate_threshold" => {
            if let Some(val) = field_value.as_u64() {
                if val == 0 || val > 100000 {
                    return Ok(Some(FieldValidationError {
                        field: "log_truncate_threshold".to_string(),
                        message: "日志截断阈值必须在 1-100000 之间".to_string(),
                        code: "out_of_range".to_string(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

/// 获取文件过滤配置
#[command]
pub async fn get_file_filter_config(app: AppHandle) -> Result<FileFilterConfig, String> {
    let config = load_config(app).await?;
    Ok(config.file_filter)
}

/// 保存文件过滤配置
#[command]
pub async fn save_file_filter_config(
    app: AppHandle,
    filter_config: FileFilterConfig,
) -> Result<(), String> {
    // 验证配置
    let validation = filter_config.validate();
    if !validation.is_valid {
        let error_msg = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("文件过滤配置验证失败: {}", error_msg));
    }

    let mut config = load_config(app.clone()).await?;
    config.file_filter = filter_config;
    save_config(app, config).await?;
    Ok(())
}

// ============ 缓存配置命令 ============

/// 获取缓存配置
#[command]
pub async fn get_cache_config(app: AppHandle) -> Result<CacheConfig, String> {
    let config = load_config(app).await?;
    Ok(config.cache)
}

/// 保存缓存配置
#[command]
pub async fn save_cache_config(app: AppHandle, cache_config: CacheConfig) -> Result<(), String> {
    // 验证配置
    let validation = cache_config.validate();
    if !validation.is_valid {
        let error_msg = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("缓存配置验证失败: {}", error_msg));
    }

    let mut config = load_config(app.clone()).await?;
    config.cache = cache_config;
    save_config(app, config).await?;
    Ok(())
}

// ============ 搜索配置命令 ============

/// 获取搜索配置
#[command]
pub async fn get_search_config(app: AppHandle) -> Result<SearchConfig, String> {
    let config = load_config(app).await?;
    Ok(config.search)
}

/// 保存搜索配置
#[command]
pub async fn save_search_config(app: AppHandle, search_config: SearchConfig) -> Result<(), String> {
    // 验证配置
    let validation = search_config.validate();
    if !validation.is_valid {
        let error_msg = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("搜索配置验证失败: {}", error_msg));
    }

    let mut config = load_config(app.clone()).await?;
    config.search = search_config;
    save_config(app, config).await?;
    Ok(())
}

// ============ 任务管理器配置命令 ============

/// 获取任务管理器配置
#[command]
pub async fn get_task_manager_config(app: AppHandle) -> Result<TaskManagerConfig, String> {
    let config = load_config(app).await?;
    Ok(config.task_manager)
}

/// 保存任务管理器配置
#[command]
pub async fn save_task_manager_config(
    app: AppHandle,
    task_manager_config: TaskManagerConfig,
) -> Result<(), String> {
    // 验证配置
    let validation = task_manager_config.validate();
    if !validation.is_valid {
        let error_msg = validation
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("任务管理器配置验证失败: {}", error_msg));
    }

    let mut config = load_config(app.clone()).await?;
    config.task_manager = task_manager_config;
    save_config(app, config).await?;
    Ok(())
}
