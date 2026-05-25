//! 日志配置管理命令
//!
//! 提供运行时调整日志级别的 Tauri 命令接口。
//! 所有公开命令统一使用 `CommandError` 作为错误类型。

use la_core::error::CommandError;
use tauri::{AppHandle, Manager};

use crate::utils::log_config::{
    get_debug_log_config, get_log_config, get_production_log_config, load_log_config_from_file,
    reset_log_config, save_log_config_to_file, set_global_log_level, set_module_log_level,
    LogConfig, LogLevel,
};

/// FIX(CR-03): 将用户传入的路径解析为应用配置目录下的安全路径
fn resolve_log_config_path(
    app: &AppHandle,
    path: &str,
) -> Result<std::path::PathBuf, CommandError> {
    let safe_name = crate::utils::validation::prevent_path_traversal(path)
        .map_err(|e| CommandError::new("CONFIG_PATH_UNSAFE", format!("配置路径不安全: {}", e)))?;
    let path_obj = std::path::Path::new(&safe_name);

    // 拒绝绝对路径
    for component in path_obj.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(CommandError::new(
                    "ABSOLUTE_PATH_REJECTED",
                    "配置路径必须是相对路径".to_string(),
                ));
            }
            std::path::Component::ParentDir => {
                return Err(CommandError::new(
                    "PATH_TRAVERSAL_DETECTED",
                    "配置路径包含非法路径遍历".to_string(),
                ));
            }
        }
    }

    let config_dir = app.path().app_config_dir().map_err(|e| {
        CommandError::new(
            "APP_CONFIG_DIR_UNAVAILABLE",
            format!("无法获取应用配置目录: {}", e),
        )
    })?;
    Ok(config_dir.join(path_obj))
}

/// 获取当前日志配置
#[tauri::command]
pub async fn get_current_log_config() -> Result<LogConfig, CommandError> {
    Ok(get_log_config())
}

/// 设置全局日志级别
///
/// # 参数
/// - `level`: 日志级别字符串 (trace, debug, info, warn, error)
#[tauri::command]
pub async fn set_log_level(level: String) -> Result<(), CommandError> {
    let log_level = LogLevel::from_string(&level).ok_or_else(|| {
        CommandError::new("INVALID_LOG_LEVEL", format!("无效的日志级别: {}", level))
    })?;

    set_global_log_level(log_level).map_err(|e| {
        CommandError::new("LOG_LEVEL_SET_FAILED", format!("设置日志级别失败: {}", e))
    })?;

    Ok(())
}

/// 为特定模块设置日志级别
///
/// # 参数
/// - `module`: 模块名称（如 "log_analyzer::task_manager"）
/// - `level`: 日志级别字符串
#[tauri::command]
pub async fn set_module_level(module: String, level: String) -> Result<(), CommandError> {
    let log_level = LogLevel::from_string(&level).ok_or_else(|| {
        CommandError::new("INVALID_LOG_LEVEL", format!("无效的日志级别: {}", level))
    })?;

    set_module_log_level(&module, log_level);
    Ok(())
}

/// 重置日志配置为默认值
#[tauri::command]
pub async fn reset_log_configuration() -> Result<(), CommandError> {
    reset_log_config();
    Ok(())
}

/// 获取生产环境推荐配置
#[tauri::command]
pub async fn get_recommended_production_config() -> Result<LogConfig, CommandError> {
    Ok(get_production_log_config())
}

/// 获取调试模式配置
#[tauri::command]
pub async fn get_recommended_debug_config() -> Result<LogConfig, CommandError> {
    Ok(get_debug_log_config())
}

/// 从文件加载日志配置
///
/// # 参数
/// - `path`: 配置文件路径
#[tauri::command]
pub async fn load_log_config(app: AppHandle, path: String) -> Result<LogConfig, CommandError> {
    let final_path = resolve_log_config_path(&app, &path)?;
    load_log_config_from_file(&final_path)
        .map_err(|e| CommandError::new("LOG_CONFIG_LOAD_FAILED", e))
}

/// 保存日志配置到文件
///
/// # 参数
/// - `path`: 配置文件路径
/// - `config`: 要保存的配置
#[tauri::command]
pub async fn save_log_config(
    app: AppHandle,
    path: String,
    config: LogConfig,
) -> Result<(), CommandError> {
    let final_path = resolve_log_config_path(&app, &path)?;
    save_log_config_to_file(&final_path, &config)
        .map_err(|e| CommandError::new("LOG_CONFIG_SAVE_FAILED", e))
}

/// 获取所有支持的日志级别
#[tauri::command]
pub async fn get_available_log_levels() -> Result<Vec<String>, CommandError> {
    Ok(vec![
        "trace".to_string(),
        "debug".to_string(),
        "info".to_string(),
        "warn".to_string(),
        "error".to_string(),
    ])
}

/// 应用预设的日志配置
///
/// # 参数
/// - `preset`: 预设名称 ("production", "debug", "default")
#[tauri::command]
pub async fn apply_log_preset(preset: String) -> Result<LogConfig, CommandError> {
    let config = match preset.as_str() {
        "production" => {
            let config = get_production_log_config();
            for module_config in &config.modules {
                set_module_log_level(&module_config.module, module_config.level);
            }
            config
        }
        "debug" => {
            let config = get_debug_log_config();
            for module_config in &config.modules {
                set_module_log_level(&module_config.module, module_config.level);
            }
            config
        }
        "default" => {
            reset_log_config();
            get_log_config()
        }
        _ => {
            return Err(CommandError::new(
                "UNKNOWN_LOG_PRESET",
                format!("未知的预设: {}", preset),
            ))
        }
    };

    Ok(config)
}
