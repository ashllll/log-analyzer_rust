//! 通用工具函数模块
//!
//! 提供路径处理、编码转换、参数验证、重试机制和清理功能等通用工具。

pub mod app_config;
pub mod encoding;
pub mod log_config;
pub mod path;
pub mod retry;
pub mod validation;
pub mod workspace_guard;
pub mod workspace_paths;

// 重新导出常用工具函数
pub use app_config::load_app_config;
pub use log_config::{
    get_debug_log_config, get_log_config, get_production_log_config, load_log_config_from_file,
    reset_log_config, save_log_config_to_file, set_global_log_level, set_module_log_level,
    LogConfig, LogLevel, ModuleLogConfig,
};
pub use path::{canonicalize_path, normalize_path_separator};
