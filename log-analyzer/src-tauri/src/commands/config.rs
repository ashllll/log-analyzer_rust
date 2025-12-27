//! 配置管理命令

use std::fs;

use tauri::{command, AppHandle, Manager};

use crate::models::AppConfig;

#[command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    let path = config_dir.join("config.json");
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");
    if path.exists() {
        let c = fs::read_to_string(path).map_err(|e| e.to_string())?;
        match serde_json::from_str(&c) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("[WARNING] Failed to parse config, using defaults: {}", e);
                Ok(AppConfig {
                    keyword_groups: serde_json::json!([]),
                    workspaces: serde_json::json!([]),
                    advanced_features: Default::default(),
                })
            }
        }
    } else {
        Ok(AppConfig {
            keyword_groups: serde_json::json!([]),
            advanced_features: Default::default(),
            workspaces: serde_json::json!([]),
        })
    }
}
