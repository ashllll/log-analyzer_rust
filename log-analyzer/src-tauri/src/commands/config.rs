//! 配置管理命令
//!
//! 使用行业标准的 `config` crate 实现配置管理：
//! - 支持多层配置：默认值 → 配置文件 → 环境变量
//! - 环境变量前缀：`LOG_ANALYZER_`
//! - 保持向后兼容 JSON 配置文件
//! - 配置验证支持

use std::fs;

use la_core::error::AppError;
use tauri::{command, AppHandle, Manager};

use la_core::models::config::{
    AppConfig, AppConfigLoader, CacheConfig, ConfigValidator, FileFilterConfig, SearchConfig,
    TaskManagerConfig,
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
            fs::create_dir_all(&config_dir).map_err(|e| {
                AppError::io_error(e.to_string(), Some(config_dir.clone())).to_string()
            })?;
        }
        let path = config_dir.join("config.json");
        let tmp_path = config_dir.join("config.json.tmp");
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        // 先写临时文件，再原子重命名，防止进程崩溃导致配置损坏
        fs::write(&tmp_path, &json)
            .map_err(|e| AppError::io_error(e.to_string(), Some(tmp_path.clone())).to_string())?;
        fs::rename(&tmp_path, &path)
            .map_err(|e| AppError::io_error(e.to_string(), Some(path.clone())).to_string())?;
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
