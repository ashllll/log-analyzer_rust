//! 日志配置管理命令
//!
//! 提供运行时调整日志级别的 Tauri 命令接口

use tauri::command;

use crate::utils::log_config::{
    get_debug_log_config, get_log_config, get_production_log_config, load_log_config_from_file,
    reset_log_config, save_log_config_to_file, set_global_log_level, set_module_log_level,
    LogConfig, LogLevel,
};

/// 获取当前日志配置
#[command]
pub async fn get_current_log_config() -> Result<LogConfig, String> {
    Ok(get_log_config())
}

/// 设置全局日志级别
///
/// # 参数
/// - `level`: 日志级别字符串 (trace, debug, info, warn, error)
#[command]
pub async fn set_log_level(level: String) -> Result<(), String> {
    let log_level =
        LogLevel::from_string(&level).ok_or_else(|| format!("无效的日志级别: {}", level))?;

    set_global_log_level(log_level).map_err(|e| format!("设置日志级别失败: {}", e))?;

    Ok(())
}

/// 为特定模块设置日志级别
///
/// # 参数
/// - `module`: 模块名称（如 "log_analyzer::task_manager"）
/// - `level`: 日志级别字符串
#[command]
pub async fn set_module_level(module: String, level: String) -> Result<(), String> {
    let log_level =
        LogLevel::from_string(&level).ok_or_else(|| format!("无效的日志级别: {}", level))?;

    set_module_log_level(&module, log_level);
    Ok(())
}

/// 重置日志配置为默认值
#[command]
pub async fn reset_log_configuration() -> Result<(), String> {
    reset_log_config();
    Ok(())
}

/// 获取生产环境推荐配置
#[command]
pub async fn get_recommended_production_config() -> Result<LogConfig, String> {
    Ok(get_production_log_config())
}

/// 获取调试模式配置
#[command]
pub async fn get_recommended_debug_config() -> Result<LogConfig, String> {
    Ok(get_debug_log_config())
}

/// 从文件加载日志配置
///
/// # 参数
/// - `path`: 配置文件路径
#[command]
pub async fn load_log_config(path: String) -> Result<LogConfig, String> {
    let path = std::path::Path::new(&path);
    load_log_config_from_file(path)
}

/// 保存日志配置到文件
///
/// # 参数
/// - `path`: 配置文件路径
/// - `config`: 要保存的配置
#[command]
pub async fn save_log_config(path: String, config: LogConfig) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    save_log_config_to_file(path, &config)
}

/// 获取所有支持的日志级别
#[command]
pub async fn get_available_log_levels() -> Result<Vec<String>, String> {
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
#[command]
pub async fn apply_log_preset(preset: String) -> Result<LogConfig, String> {
    let config = match preset.as_str() {
        "production" => {
            let config = get_production_log_config();
            // 应用生产环境配置
            for module_config in &config.modules {
                set_module_log_level(&module_config.module, module_config.level);
            }
            config
        }
        "debug" => {
            let config = get_debug_log_config();
            // 应用调试配置
            for module_config in &config.modules {
                set_module_log_level(&module_config.module, module_config.level);
            }
            config
        }
        "default" => {
            reset_log_config();
            get_log_config()
        }
        _ => return Err(format!("未知的预设: {}", preset)),
    };

    Ok(config)
}
