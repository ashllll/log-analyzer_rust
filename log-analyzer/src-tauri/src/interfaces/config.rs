//! Config command interface adapters.

use std::sync::Arc;

use tauri::AppHandle;

use la_core::models::config::{
    AppConfig, ConfigValidator, FileFilterConfig, SearchConfig, TaskManagerConfig,
};

use crate::adapters::tauri_config::TauriAppConfigProvider;
use crate::application::ConfigUseCase;

fn use_case(app: AppHandle) -> ConfigUseCase<TauriAppConfigProvider> {
    ConfigUseCase::new(Arc::new(TauriAppConfigProvider(app)))
}

#[tauri::command]
pub async fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    tokio::task::spawn_blocking(move || use_case(app).load().map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
pub async fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    tokio::task::spawn_blocking(move || use_case(app).save(&config).map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("Task panicked: {e}"))?
}

#[tauri::command]
pub async fn get_file_filter_config(app: AppHandle) -> Result<FileFilterConfig, String> {
    let config = load_config(app).await?;
    Ok(config.file_filter)
}

#[tauri::command]
pub async fn save_file_filter_config(
    app: AppHandle,
    filter_config: FileFilterConfig,
) -> Result<(), String> {
    let validation = filter_config.validate();
    if !validation.is_valid {
        return Err(format_validation_errors(
            "文件过滤配置验证失败",
            &validation.errors,
        ));
    }

    let mut config = load_config(app.clone()).await?;
    config.file_filter = filter_config;
    save_config(app, config).await
}

#[tauri::command]
pub async fn get_search_config(app: AppHandle) -> Result<SearchConfig, String> {
    let config = load_config(app).await?;
    Ok(config.search)
}

#[tauri::command]
pub async fn save_search_config(app: AppHandle, search_config: SearchConfig) -> Result<(), String> {
    let validation = search_config.validate();
    if !validation.is_valid {
        return Err(format_validation_errors(
            "搜索配置验证失败",
            &validation.errors,
        ));
    }

    let mut config = load_config(app.clone()).await?;
    config.search = search_config;
    save_config(app, config).await
}

#[tauri::command]
pub async fn get_task_manager_config(app: AppHandle) -> Result<TaskManagerConfig, String> {
    let config = load_config(app).await?;
    Ok(config.task_manager)
}

#[tauri::command]
pub async fn save_task_manager_config(
    app: AppHandle,
    task_manager_config: TaskManagerConfig,
) -> Result<(), String> {
    let validation = task_manager_config.validate();
    if !validation.is_valid {
        return Err(format_validation_errors(
            "任务管理器配置验证失败",
            &validation.errors,
        ));
    }

    let mut config = load_config(app.clone()).await?;
    config.task_manager = task_manager_config;
    save_config(app, config).await
}

fn format_validation_errors(
    prefix: &str,
    errors: &[la_core::models::config::FieldValidationError],
) -> String {
    let details = errors
        .iter()
        .map(|error| format!("{}: {}", error.field, error.message))
        .collect::<Vec<_>>()
        .join("; ");
    format!("{prefix}: {details}")
}
