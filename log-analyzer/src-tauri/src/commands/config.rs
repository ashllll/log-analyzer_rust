//! 配置管理命令
//!
//! 使用行业标准的 `config` crate 实现配置管理：
//! - 支持多层配置：默认值 → 配置文件 → 环境变量
//! - 环境变量前缀：`LOG_ANALYZER_`
//! - 保持向后兼容 JSON 配置文件

use std::fs;

use tauri::{command, AppHandle, Manager};

use crate::models::config::{AppConfig, AppConfigLoader, FileFilterConfig};

/// 加载配置（使用新的 ConfigLoader 系统）
fn load_config_internal(app: &AppHandle) -> Result<AppConfig, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;

    let config_path = config_dir.join("config.json");

    if config_path.exists() {
        AppConfigLoader::load(Some(config_path))
            .map(|loader| loader.get_config().clone())
            .map_err(|e| e.to_string())
    } else {
        // 返回默认配置
        Ok(AppConfig::default())
    }
}

#[command]
pub async fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        }
        let path = config_dir.join("config.json");
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))?
}

#[command]
pub async fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    tokio::task::spawn_blocking(move || load_config_internal(&app))
        .await
        .map_err(|e| format!("Task panicked: {}", e))?
}

#[command]
pub async fn get_file_filter_config(app: AppHandle) -> Result<FileFilterConfig, String> {
    let config = load_config(app).await?;
    Ok(config.file_filter)
}

#[command]
pub async fn save_file_filter_config(
    app: AppHandle,
    filter_config: FileFilterConfig,
) -> Result<(), String> {
    let mut config = load_config(app.clone()).await?;
    config.file_filter = filter_config;
    save_config(app, config).await?;
    Ok(())
}
