//! 通用工具函数模块
//!
//! 提供路径处理、编码转换、参数验证、重试机制和清理功能等通用工具。

pub mod async_resource_manager;
pub mod app_config;
pub mod command_validation;
pub mod encoding;
pub mod log_config;
pub mod path;
pub mod retry;
pub mod validation;
pub mod workspace_guard;
pub mod workspace_paths;

// 重新导出常用工具函数
pub use app_config::load_app_config;
pub use async_resource_manager::{AsyncResourceManager, OperationType};
pub use log_config::{
    get_debug_log_config, get_log_config, get_production_log_config, load_log_config_from_file,
    reset_log_config, save_log_config_to_file, set_global_log_level, set_module_log_level,
    LogConfig, LogLevel, ModuleLogConfig,
};
pub use path::{canonicalize_path, normalize_path_separator};
pub use validation::{validate_path_param, validate_workspace_id};

// 重新导出命令验证函数
pub use command_validation::{
    validate_api_key, validate_export_path, validate_log_level, validate_port, validate_range,
    validate_search_query, validate_websocket_url, MAX_PATH_LENGTH, MAX_SEARCH_QUERY_LENGTH,
    MAX_WORKSPACE_ID_LENGTH,
};
