//! 动态日志级别控制模块
//!
//! 提供运行时调整日志级别的能力，支持通过配置或命令调整日志输出级别。
//! 用于优化性能，减少不必要的 DEBUG 日志输出。

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::reload::Handle;

/// 日志级别配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 最详细的日志，用于高频循环内部
    Trace,
    /// 详细调试信息
    Debug,
    /// 一般信息
    Info,
    /// 警告信息
    Warn,
    /// 错误信息
    Error,
}

impl LogLevel {
    /// 转换为 tracing LevelFilter
    pub fn to_level_filter(self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
        }
    }

    /// 从字符串解析
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LogLevel::from_string(s).ok_or_else(|| format!("Invalid log level: {}", s))
    }
}

#[allow(clippy::derivable_impls)]
impl Default for LogLevel {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            LogLevel::Debug
        }
        #[cfg(not(debug_assertions))]
        {
            LogLevel::Info
        }
    }
}

/// 模块特定的日志级别配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLogConfig {
    /// 模块名称（支持通配符，如 "task_manager*"）
    pub module: String,
    /// 日志级别
    pub level: LogLevel,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 全局默认日志级别
    pub default_level: LogLevel,
    /// 模块特定配置
    pub modules: Vec<ModuleLogConfig>,
    /// 是否启用文件日志
    pub enable_file_log: bool,
    /// 文件日志路径（相对路径或绝对路径）
    pub file_log_path: Option<String>,
    /// 是否启用 JSON 格式
    pub json_format: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            default_level: LogLevel::default(),
            modules: vec![
                // 高频循环模块默认使用 Info 级别
                ModuleLogConfig {
                    module: "log_analyzer::task_manager".to_string(),
                    level: LogLevel::Info,
                },
                ModuleLogConfig {
                    module: "log_analyzer::search_engine".to_string(),
                    level: LogLevel::Info,
                },
                ModuleLogConfig {
                    module: "log_analyzer::cache_manager".to_string(),
                    level: LogLevel::Info,
                },
            ],
            enable_file_log: false,
            file_log_path: None,
            json_format: false,
        }
    }
}

/// 日志级别重载句柄类型
pub type ReloadHandle = Handle<LevelFilter, tracing_subscriber::Registry>;

/// 全局日志配置状态
static LOG_CONFIG: Lazy<RwLock<LogConfig>> = Lazy::new(|| RwLock::new(LogConfig::default()));

/// 全局重载句柄（在初始化时设置）
static RELOAD_HANDLE: Lazy<RwLock<Option<ReloadHandle>>> = Lazy::new(|| RwLock::new(None));

/// 初始化日志系统
///
/// # 参数
/// - `config`: 可选的自定义配置，为 None 时使用默认配置
///
/// # 示例
/// ```rust
/// use log_analyzer::utils::log_config::{init_logging, LogConfig, LogLevel};
///
/// let config = LogConfig {
///     default_level: LogLevel::Info,
///     ..Default::default()
/// };
/// init_logging(Some(config));
/// ```
pub fn init_logging(config: Option<LogConfig>) {
    let config = config.unwrap_or_default();

    // 保存配置
    *LOG_CONFIG.write() = config.clone();

    // 构建过滤器
    let filter = build_env_filter(&config);

    // 初始化订阅器
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");

    tracing::info!("日志系统初始化完成，默认级别: {:?}", config.default_level);
}

/// 构建环境过滤器
fn build_env_filter(config: &LogConfig) -> tracing_subscriber::EnvFilter {
    let mut filter = tracing_subscriber::EnvFilter::new(
        config.default_level.to_level_filter().to_string()
    );

    // 添加模块特定配置
    for module_config in &config.modules {
        let directive = format!(
            "{}={}",
            module_config.module,
            module_config.level.to_level_filter()
        );
        filter = filter.add_directive(directive.parse().unwrap());
    }

    filter
}

/// 更新全局日志级别
///
/// # 参数
/// - `level`: 新的日志级别
///
/// # 返回
/// - `Ok(())`: 更新成功
/// - `Err(String)`: 更新失败（日志系统未初始化）
pub fn set_global_log_level(level: LogLevel) -> Result<(), String> {
    // 更新配置
    {
        let mut config = LOG_CONFIG.write();
        config.default_level = level;
    }

    // 尝试通过重载句柄更新（如果可用）
    if let Some(handle) = RELOAD_HANDLE.read().as_ref() {
        handle
            .reload(level.to_level_filter())
            .map_err(|e| format!("Failed to reload log level: {}", e))?;
    }

    tracing::info!("日志级别已更新为: {:?}", level);
    Ok(())
}

/// 获取当前日志配置
pub fn get_log_config() -> LogConfig {
    LOG_CONFIG.read().clone()
}

/// 为特定模块设置日志级别
///
/// # 参数
/// - `module`: 模块名称（如 "log_analyzer::task_manager"）
/// - `level`: 日志级别
pub fn set_module_log_level(module: &str, level: LogLevel) {
    let mut config = LOG_CONFIG.write();

    // 查找并更新或添加模块配置
    if let Some(existing) = config.modules.iter_mut().find(|m| m.module == module) {
        existing.level = level;
    } else {
        config.modules.push(ModuleLogConfig {
            module: module.to_string(),
            level,
        });
    }

    tracing::info!("模块 {} 的日志级别已设置为 {:?}", module, level);
}

/// 重置为默认日志配置
pub fn reset_log_config() {
    let default_config = LogConfig::default();
    *LOG_CONFIG.write() = default_config.clone();

    // 尝试更新运行时日志级别
    if let Some(handle) = RELOAD_HANDLE.read().as_ref() {
        let _ = handle.reload(default_config.default_level.to_level_filter());
    }

    tracing::info!("日志配置已重置为默认值");
}

/// 从配置文件加载日志配置
///
/// # 参数
/// - `path`: 配置文件路径
///
/// # 返回
/// - `Ok(LogConfig)`: 加载成功
/// - `Err(String)`: 加载失败
pub fn load_log_config_from_file(path: &std::path::Path) -> Result<LogConfig, String> {
    if !path.exists() {
        return Err(format!("配置文件不存在: {}", path.display()));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    let config: LogConfig = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;

    Ok(config)
}

/// 保存日志配置到文件
///
/// # 参数
/// - `path`: 配置文件路径
/// - `config`: 要保存的配置
///
/// # 返回
/// - `Ok(())`: 保存成功
/// - `Err(String)`: 保存失败
pub fn save_log_config_to_file(
    path: &std::path::Path,
    config: &LogConfig,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    Ok(())
}

/// 获取推荐的发布模式日志配置
///
/// 用于生产环境，减少日志输出以提高性能
pub fn get_production_log_config() -> LogConfig {
    LogConfig {
        default_level: LogLevel::Info,
        modules: vec![
            // 高频模块使用 Warn 级别
            ModuleLogConfig {
                module: "log_analyzer::task_manager".to_string(),
                level: LogLevel::Warn,
            },
            ModuleLogConfig {
                module: "log_analyzer::task_manager::actor".to_string(),
                level: LogLevel::Warn,
            },
            ModuleLogConfig {
                module: "log_analyzer::search_engine".to_string(),
                level: LogLevel::Warn,
            },
            ModuleLogConfig {
                module: "log_analyzer::cache_manager".to_string(),
                level: LogLevel::Warn,
            },
            // 关键路径保持 Info
            ModuleLogConfig {
                module: "log_analyzer::commands".to_string(),
                level: LogLevel::Info,
            },
        ],
        enable_file_log: true,
        file_log_path: Some("logs/app.log".to_string()),
        json_format: false,
    }
}

/// 获取调试模式日志配置
///
/// 用于开发和故障排查，输出详细日志
pub fn get_debug_log_config() -> LogConfig {
    LogConfig {
        default_level: LogLevel::Debug,
        modules: vec![
            ModuleLogConfig {
                module: "log_analyzer::task_manager".to_string(),
                level: LogLevel::Debug,
            },
            ModuleLogConfig {
                module: "log_analyzer::search_engine".to_string(),
                level: LogLevel::Debug,
            },
        ],
        enable_file_log: true,
        file_log_path: Some("logs/debug.log".to_string()),
        json_format: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_string() {
        assert_eq!(LogLevel::from_string("trace"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_string("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_string("Info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_string("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_string("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_string("invalid"), None);
    }

    #[test]
    fn test_log_level_to_filter() {
        assert_eq!(LogLevel::Trace.to_level_filter(), LevelFilter::TRACE);
        assert_eq!(LogLevel::Debug.to_level_filter(), LevelFilter::DEBUG);
        assert_eq!(LogLevel::Info.to_level_filter(), LevelFilter::INFO);
        assert_eq!(LogLevel::Warn.to_level_filter(), LevelFilter::WARN);
        assert_eq!(LogLevel::Error.to_level_filter(), LevelFilter::ERROR);
    }

    #[test]
    fn test_default_config() {
        let config = LogConfig::default();
        // 高频模块应该默认使用 Info 级别
        let task_manager_config = config
            .modules
            .iter()
            .find(|m| m.module == "log_analyzer::task_manager");
        assert!(task_manager_config.is_some());
        assert_eq!(task_manager_config.unwrap().level, LogLevel::Info);
    }

    #[test]
    fn test_production_config() {
        let config = get_production_log_config();
        assert_eq!(config.default_level, LogLevel::Info);

        // 高频模块应该使用 Warn 级别
        let task_manager_config = config
            .modules
            .iter()
            .find(|m| m.module == "log_analyzer::task_manager");
        assert_eq!(task_manager_config.unwrap().level, LogLevel::Warn);
    }

    #[test]
    fn test_set_module_log_level() {
        set_module_log_level("test_module", LogLevel::Error);

        let config = get_log_config();
        let module_config = config
            .modules
            .iter()
            .find(|m| m.module == "test_module");
        assert!(module_config.is_some());
        assert_eq!(module_config.unwrap().level, LogLevel::Error);
    }
}
