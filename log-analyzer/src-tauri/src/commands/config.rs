//! 配置管理命令
//!
//! 暂时保留在lib.rs中，阶段5整合时迁移

// TODO: 从lib.rs迁移 save_config 和 load_config

/*
use crate::models::config::AppConfig;
use std::fs;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> { ... }

#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<AppConfig, String> { ... }
*/
