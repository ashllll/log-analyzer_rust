//! Log configuration command interface adapters.

use tauri::AppHandle;

use la_core::error::CommandError;

use crate::utils::log_config::LogConfig;

#[tauri::command]
pub async fn get_current_log_config() -> Result<LogConfig, CommandError> {
    crate::commands::log_config::get_current_log_config().await
}

#[tauri::command]
pub async fn set_log_level(level: String) -> Result<(), CommandError> {
    crate::commands::log_config::set_log_level(level).await
}

#[tauri::command]
pub async fn set_module_level(module: String, level: String) -> Result<(), CommandError> {
    crate::commands::log_config::set_module_level(module, level).await
}

#[tauri::command]
pub async fn reset_log_configuration() -> Result<(), CommandError> {
    crate::commands::log_config::reset_log_configuration().await
}

#[tauri::command]
pub async fn get_recommended_production_config() -> Result<LogConfig, CommandError> {
    crate::commands::log_config::get_recommended_production_config().await
}

#[tauri::command]
pub async fn get_recommended_debug_config() -> Result<LogConfig, CommandError> {
    crate::commands::log_config::get_recommended_debug_config().await
}

#[tauri::command]
pub async fn load_log_config(app: AppHandle, path: String) -> Result<LogConfig, CommandError> {
    crate::commands::log_config::load_log_config(app, path).await
}

#[tauri::command]
pub async fn save_log_config(
    app: AppHandle,
    path: String,
    config: LogConfig,
) -> Result<(), CommandError> {
    crate::commands::log_config::save_log_config(app, path, config).await
}

#[tauri::command]
pub async fn get_available_log_levels() -> Result<Vec<String>, CommandError> {
    crate::commands::log_config::get_available_log_levels().await
}

#[tauri::command]
pub async fn apply_log_preset(preset: String) -> Result<LogConfig, CommandError> {
    crate::commands::log_config::apply_log_preset(preset).await
}
